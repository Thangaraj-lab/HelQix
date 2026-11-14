"""Parse and validate the versioned JSON netlist."""

from __future__ import annotations

import copy
import json
import math
from dataclasses import dataclass, field
from pathlib import Path

from .errors import InputError
from .units import to_si

VERSION = "circuit/0.1"
GROUND = "0"

KIND_NODES = {
    "resistor": 2,
    "vsource": 2,
    "isource": 2,
    "vcvs": 4,
    "vccs": 4,
    "switch": 2,
}

POSITIVE_KINDS = {"resistor", "capacitor", "inductor"}


@dataclass(frozen=True)
class Component:
    id: str
    kind: str
    nodes: tuple[str, ...]
    value: float | None
    unit: str | None
    state: str | None = None
    params: dict = field(default_factory=dict)

    def voltage(self, node_voltages: dict[str, float]) -> float:
        return node_voltages.get(self.nodes[0], 0.0) - node_voltages.get(self.nodes[1], 0.0)


@dataclass(frozen=True)
class Circuit:
    version: str
    title: str
    components: tuple[Component, ...]
    analysis: dict
    raw: dict
    ground: str = GROUND

    @property
    def component_map(self) -> dict[str, Component]:
        return {component.id: component for component in self.components}

    @property
    def node_names(self) -> list[str]:
        seen: list[str] = []
        for component in self.components:
            for node in component.nodes:
                if node != self.ground and node not in seen:
                    seen.append(node)
        return seen


def _read(source) -> dict:
    if isinstance(source, Circuit):
        return source.raw
    if isinstance(source, dict):
        return copy.deepcopy(source)
    if isinstance(source, (str, Path)):
        path = Path(source)
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as exc:
            raise InputError(f"cannot read netlist {path}: {exc}") from exc
        try:
            return json.loads(text)
        except json.JSONDecodeError as exc:
            raise InputError(f"netlist {path} is not valid JSON: {exc}") from exc
    raise InputError(f"netlist must be a dict, path or Circuit, got {type(source).__name__}")


def _number(value, what: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise InputError(f"{what} must be a number")
    number = float(value)
    if not math.isfinite(number):
        raise InputError(f"{what} must be finite")
    return number


def _component(index: int, body: dict) -> Component:
    where = f"component #{index + 1}"
    if not isinstance(body, dict):
        raise InputError(f"{where} must be an object")
    cid = body.get("id")
    if not isinstance(cid, str) or not cid.strip():
        raise InputError(f"{where} needs a non-empty string id")
    where = f"component {cid!r}"
    kind = body.get("kind")
    if kind not in KIND_NODES:
        raise InputError(f"{where} has unknown kind {kind!r}; supported kinds are {sorted(KIND_NODES)}")
    nodes = body.get("nodes")
    if not isinstance(nodes, list) or len(nodes) != KIND_NODES[kind]:
        raise InputError(f"{where} needs {KIND_NODES[kind]} nodes, got {nodes!r}")
    if not all(isinstance(node, str) and node for node in nodes):
        raise InputError(f"{where} node names must be non-empty strings")
    if kind in {"vcvs", "vccs"}:
        if nodes[0] == nodes[1] or nodes[2] == nodes[3]:
            raise InputError(f"{where} connects a terminal pair to itself")
    elif len(set(nodes)) != len(nodes):
        raise InputError(f"{where} connects a node to itself")
    if kind == "switch":
        state = body.get("state", "open")
        if state not in {"open", "closed"}:
            raise InputError(f"{where} state must be 'open' or 'closed', got {state!r}")
        return Component(cid, kind, tuple(nodes), None, None, state=state)
    value_body = body.get("value")
    if not isinstance(value_body, dict):
        raise InputError(f"{where} needs value {{'value': number, 'unit': string}}")
    if "value" not in value_body or "unit" not in value_body:
        raise InputError(f"{where} value needs both 'value' and 'unit'")
    value, unit = to_si(value_body["value"], value_body["unit"], kind)
    if kind == "resistor" and value <= 0:
        raise InputError(f"{where} resistance must be greater than zero")
    if kind in POSITIVE_KINDS and value <= 0:
        raise InputError(f"{where} {kind} value must be greater than zero")
    params = body.get("parameters", {})
    if params is None:
        params = {}
    if not isinstance(params, dict):
        raise InputError(f"{where} parameters must be an object")
    return Component(cid, kind, tuple(nodes), value, unit, params=copy.deepcopy(params))


def load_circuit(source) -> Circuit:
    body = _read(source)
    if not isinstance(body, dict):
        raise InputError("netlist root must be an object")
    version = body.get("version")
    if version != VERSION:
        raise InputError(f"netlist version must be {VERSION!r}, got {version!r}")
    if "ground" in body and body["ground"] != GROUND:
        raise InputError(f"ground node must be named '0', got {body['ground']!r}")
    components_body = body.get("components")
    if not isinstance(components_body, list):
        raise InputError("netlist needs a components list")
    components: list[Component] = []
    seen: set[str] = set()
    for index, item in enumerate(components_body):
        component = _component(index, item)
        if component.id in seen:
            raise InputError(f"duplicate component id {component.id!r}")
        seen.add(component.id)
        components.append(component)
    analysis = body.get("analysis", {"type": "dc"})
    if not isinstance(analysis, dict) or not isinstance(analysis.get("type", "dc"), str):
        raise InputError("analysis must be an object with a type")
    title = body.get("title", "")
    if not isinstance(title, str):
        raise InputError("title must be a string")
    return Circuit(version, title, tuple(components), copy.deepcopy(analysis), body)
