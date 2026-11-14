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
    "capacitor": 2,
    "inductor": 2,
    "diode": 2,
    "mosfet": 4,
    "opamp": 4,
}

LINEAR_DC_KINDS = {"resistor", "vsource", "isource", "vcvs", "vccs", "switch"}
SOURCE_KINDS = {"vsource", "isource"}
POSITIVE_KINDS = {"resistor", "capacitor", "inductor"}

WAVEFORM_TYPES = {"dc", "step", "sine", "pulse"}


@dataclass(frozen=True)
class Component:
    id: str
    kind: str
    nodes: tuple[str, ...]
    value: float | None
    unit: str | None
    state: str | None = None
    params: dict = field(default_factory=dict)
    waveform: dict | None = None
    initial: float | None = None

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
    elif kind == "mosfet":
        if nodes[0] == nodes[2]:
            raise InputError(f"{where} drain and source must be different nodes")
    elif kind == "opamp":
        if nodes[0] == nodes[3]:
            raise InputError(f"{where} output and reference must be different nodes")
    elif len(set(nodes)) != len(nodes):
        raise InputError(f"{where} connects a node to itself")

    if kind == "switch":
        state = body.get("state", "open")
        if state not in {"open", "closed"}:
            raise InputError(f"{where} state must be 'open' or 'closed', got {state!r}")
        return Component(cid, kind, tuple(nodes), None, None, state=state)

    if kind in {"diode", "mosfet", "opamp"}:
        params = _device_parameters(cid, kind, body.get("parameters"))
        return Component(cid, kind, tuple(nodes), None, None, params=params)

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

    initial = None
    if kind in {"capacitor", "inductor"}:
        initial = _initial_value(where, kind, body.get("initial"))

    waveform = None
    if kind in SOURCE_KINDS:
        waveform = _waveform(where, body.get("waveform"))

    params = body.get("parameters", {})
    if params is None:
        params = {}
    if not isinstance(params, dict):
        raise InputError(f"{where} parameters must be an object")
    return Component(cid, kind, tuple(nodes), value, unit, params=copy.deepcopy(params), waveform=waveform, initial=initial)


def _initial_value(where: str, kind: str, body) -> float:
    if body is None:
        return 0.0
    if not isinstance(body, dict) or "value" not in body or "unit" not in body:
        raise InputError(f"{where} initial needs {{'value': number, 'unit': string}}")
    value, unit = to_si(body["value"], body["unit"])
    allowed = {"v", "mv"} if kind == "capacitor" else {"a", "ma", "ua"}
    if unit not in allowed:
        raise InputError(f"{where} initial unit must be one of {sorted(allowed)}")
    return value


def _number_field(where: str, params: dict, key: str, default=None):
    if key not in params:
        if default is None:
            raise InputError(f"{where} parameter {key!r} is required")
        return default
    return _number(params[key], f"{where} parameter {key!r}")


def _device_parameters(cid: str, kind: str, body) -> dict:
    where = f"component {cid!r}"
    if body is None:
        body = {}
    if not isinstance(body, dict):
        raise InputError(f"{where} parameters must be an object")
    params = copy.deepcopy(body)
    if kind == "diode":
        result = {
            "is_a": _number_field(where, params, "is_a", 1e-12),
            "n": _number_field(where, params, "n", 1.0),
            "rs_ohm": _number_field(where, params, "rs_ohm", 0.0),
            "t_deg_c": _number_field(where, params, "t_deg_c", 27.0),
        }
        if result["is_a"] <= 0 or result["n"] <= 0 or result["rs_ohm"] < 0:
            raise InputError(f"{where} diode needs is_a > 0, n > 0, rs_ohm >= 0")
        return result
    if kind == "mosfet":
        channel = params.get("channel")
        if channel not in {"n", "p"}:
            raise InputError(f"{where} parameter 'channel' must be 'n' or 'p'")
        result = {
            "channel": channel,
            "vth": _number_field(where, params, "vth"),
            "k": _number_field(where, params, "k"),
            "lambda": _number_field(where, params, "lambda", 0.0),
        }
        if result["k"] <= 0 or result["lambda"] < 0:
            raise InputError(f"{where} mosfet needs k > 0 and lambda >= 0")
        return result
    ideal = params.get("ideal", False)
    if not isinstance(ideal, bool):
        raise InputError(f"{where} parameter 'ideal' must be true or false")
    result = {"ideal": ideal}
    if not ideal:
        result["gain"] = _number_field(where, params, "gain")
        if result["gain"] <= 0:
            raise InputError(f"{where} op-amp gain must be greater than zero")
    return result


def _waveform(where: str, body) -> dict | None:
    if body is None:
        return None
    if not isinstance(body, dict):
        raise InputError(f"{where} waveform must be an object")
    kind = body.get("type", "dc")
    if kind not in WAVEFORM_TYPES:
        raise InputError(f"{where} waveform type must be one of {sorted(WAVEFORM_TYPES)}")
    result = {"type": kind}
    if kind == "step":
        result["delay_s"] = _number(body.get("delay_s", 0.0), f"{where} waveform delay_s")
    elif kind == "sine":
        result["amplitude"] = _number(body.get("amplitude"), f"{where} waveform amplitude")
        result["frequency_hz"] = _number(body.get("frequency_hz"), f"{where} waveform frequency_hz")
        result["offset"] = _number(body.get("offset", 0.0), f"{where} waveform offset")
        result["phase_deg"] = _number(body.get("phase_deg", 0.0), f"{where} waveform phase_deg")
    elif kind == "pulse":
        result["amplitude"] = _number(body.get("amplitude"), f"{where} waveform amplitude")
        result["period_s"] = _number(body.get("period_s"), f"{where} waveform period_s")
        result["duty"] = _number(body.get("duty", 0.5), f"{where} waveform duty")
        result["delay_s"] = _number(body.get("delay_s", 0.0), f"{where} waveform delay_s")
        if result["period_s"] <= 0 or not 0.0 < result["duty"] < 1.0:
            raise InputError(f"{where} pulse needs period_s > 0 and 0 < duty < 1")
    return result


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
