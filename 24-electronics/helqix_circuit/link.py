"""Link a harness/0.1 document to a DC circuit and inject wiring faults.

Every wire from the document becomes a series resistor between the circuit
nodes it names; the DC operating point gives each wire's current, voltage
drop and loss.  Temperature rise is UNKNOWN because no thermal method is
cited, and ampacity comes only from the transcribed IEC 60364-5-52 row, so a
size without a row keeps an UNKNOWN ampacity and its verdict comes from the
drop alone.  Faults are one scenario each and deterministic: an open wire,
a series joint, a short between two wire ends, or a wire that opens on a
seeded share of 20 checks.
"""

from __future__ import annotations

import copy
import math
import random
from dataclasses import dataclass

from .ampacity_table import TABLE_SOURCE, copper_ampacity
from .errors import InputError, SolverError
from .harness import (
    AMPERAGE_DESIGN_FRACTION,
    DROP_LIMIT_PERCENT,
    DROP_WARN_PERCENT,
    REFERENCE_TEMP_C,
    WireSpec,
    _ampacity_verdict,
    _drop_verdict,
    _supply_reference,
    _worst_verdict,
)
from .mna import solve_dc
from .netlist import load_circuit

SHORT_OHM = 0.01
INTERMITTENT_TRIALS = 20
DEFAULT_SEED = 11
TEMPERATURE_REASON = (
    "no cited thermal method for this harness; a bare convection coefficient "
    "would be an assumption, so the rise is not reported"
)
SHORT_MODEL = (
    "a short joins the 'to' node of the first wire to the 'to' node of the "
    f"second through {SHORT_OHM:g} ohm"
)
COLOUR_HINTS = {"PASS": "OK", "WARN": "WARN", "FAIL": "FAIL", "UNKNOWN": "UNKNOWN"}
FAULT_KINDS = ("open", "joint", "short", "intermittent")
LABELS = {
    "supply": "GIVEN",
    "resistance": "CALCULATED",
    "current": "CALCULATED",
    "drop": "CALCULATED",
    "power": "CALCULATED",
    "temperature": "UNKNOWN",
    "ampacity": "TABLE",
    "checks": "MEASURED",
}


@dataclass(frozen=True)
class Fault:
    kind: str
    wires: tuple[str, ...]
    value: float | None
    text: str


def _number(value, what: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise InputError(f"{what} must be a number")
    number = float(value)
    if not math.isfinite(number):
        raise InputError(f"{what} must be finite")
    return number


def _decimal(text: str, what: str) -> float:
    try:
        return _number(float(text), what)
    except ValueError:
        raise InputError(f"{what} must be a number") from None


def parse_fault(text, wire_ids) -> Fault:
    """Parse open:W, joint:W:OHM, short:W1,W2 or intermittent:W:P."""
    if not isinstance(text, str) or not text.strip():
        raise InputError("fault must be a non-empty string")
    text = text.strip()
    head, _, rest = text.partition(":")
    kind = head.strip().lower()
    if kind not in FAULT_KINDS:
        raise InputError(f"fault {text!r} has an unknown kind {head!r}; use one of {list(FAULT_KINDS)}")
    if kind == "short":
        names = [name.strip() for name in rest.split(",")]
        if len(names) != 2:
            raise InputError(f"fault {text!r} needs exactly two wires, like short:W1,W2")
        for name in names:
            if name not in wire_ids:
                raise InputError(f"fault {text!r} names an unknown wire {name!r}")
        if names[0] == names[1]:
            raise InputError(f"fault {text!r} needs two different wires")
        return Fault(kind, tuple(names), None, text)

    wire, _, tail = rest.partition(":")
    wire = wire.strip()
    if wire not in wire_ids:
        raise InputError(f"fault {text!r} names an unknown wire {wire!r}")
    if kind == "open":
        if tail:
            raise InputError(f"fault {text!r} takes no value; use open:{wire}")
        return Fault(kind, (wire,), None, text)
    if not tail.strip():
        raise InputError(f"fault {text!r} needs a value")
    value = _decimal(tail.strip(), f"fault {text!r} value")
    if kind == "joint":
        if value <= 0:
            raise InputError(f"fault {text!r} joint resistance must be greater than zero")
        return Fault(kind, (wire,), value, text)
    if not 0.0 < value < 1.0:
        raise InputError(f"fault {text!r} probability must be between 0 and 1, exclusive")
    return Fault(kind, (wire,), value, text)


def _wire_spec(wire) -> WireSpec:
    return WireSpec(
        id=wire.id,
        from_node=wire.from_node,
        to_node=wire.to_node,
        length_m=wire.length_mm / 1000.0,
        area_m2=wire.area_mm2 * 1e-6,
        diameter_m=wire.diameter_mm / 1000.0,
        material=wire.material,
        insulation_rating_c=wire.insulation_rating_c,
        method="C",
    )


def _bits(value: float, unit: str, label: str) -> dict:
    return {"value": value, "unit": unit, "label": label}


def _route_bits(route) -> dict:
    cut = (
        {"value": None, "unit": "mm", "label": "UNKNOWN"}
        if route.blocked
        else {"value": route.cut_length_mm, "unit": "mm", "label": "CALCULATED"}
    )
    return {"blocked": route.blocked, "cut_length_mm": cut, "reason": route.reason}


def _judge(spec, current: float, supply: float, *, open_wire=False, joint_ohm=0.0) -> dict:
    r_wire = spec.resistance(REFERENCE_TEMP_C)
    r_total = r_wire + joint_ohm
    drop = current * r_total
    percent = 100.0 * drop / supply
    power = current * current * r_total
    ampacity = None
    if not open_wire and spec.material == "copper":
        ampacity, _ = copper_ampacity(spec.area_m2, spec.method)
    ampacity_verdict = _ampacity_verdict(current, ampacity) if ampacity is not None else "UNKNOWN"
    if open_wire:
        drop_verdict = "FAIL"
        verdict = "FAIL"
        reason = "the wire is open as injected; no current can pass through it"
    else:
        drop_verdict = _drop_verdict(percent, DROP_LIMIT_PERCENT, DROP_WARN_PERCENT)
        verdict = drop_verdict if ampacity_verdict == "UNKNOWN" else _worst_verdict(drop_verdict, ampacity_verdict)
        drop_text = (
            f"{percent:.2f} % drop of the {supply:.6g} V supply is {drop_verdict} "
            f"(PASS to {DROP_LIMIT_PERCENT:g} %, WARN to {DROP_WARN_PERCENT:g} %, FAIL above)"
        )
        if ampacity is None:
            amp_text = "ampacity is UNKNOWN: no cited row for this size and method in Table B.52.2"
        else:
            amp_text = (
                f"{current:.1f} A against the cited {ampacity:.1f} A row is {ampacity_verdict} "
                f"(PASS to {AMPERAGE_DESIGN_FRACTION * ampacity:.1f} A, WARN up to the row)"
            )
        reason = f"{verdict}: {drop_text}; {amp_text}"
        if joint_ohm:
            reason += f"; an injected {joint_ohm:g} ohm joint is in series"
    return {
        "resistance": r_wire,
        "current": current,
        "drop": drop,
        "percent": percent,
        "power": power,
        "ampacity": ampacity,
        "drop_verdict": drop_verdict,
        "verdict": verdict,
        "reason": reason,
    }


def _entry(wire, numbers: dict, joint_ohm: float = 0.0, label: str = "CALCULATED") -> dict:
    """Assemble the JSON entry from a judged wire; label names the current's origin."""
    return {
        "id": wire.id,
        "from": wire.from_node,
        "to": wire.to_node,
        "resistance_ohm": _bits(numbers["resistance"], "ohm", LABELS["resistance"]),
        "joint_ohm": (_bits(joint_ohm, "ohm", "GIVEN") if joint_ohm else None),
        "current_a": _bits(numbers["current"], "A", label),
        "drop_v": _bits(numbers["drop"], "V", "CALCULATED" if label == "CALCULATED" else LABELS["checks"]),
        "drop_percent": _bits(numbers["percent"], "%", "CALCULATED" if label == "CALCULATED" else LABELS["checks"]),
        "power_w": _bits(numbers["power"], "W", "CALCULATED" if label == "CALCULATED" else LABELS["checks"]),
        "temperature_rise_c": {"value": None, "unit": "K", "label": LABELS["temperature"], "reason": TEMPERATURE_REASON},
        "ampacity_a": (
            {"value": numbers["ampacity"], "unit": "A", "label": LABELS["ampacity"], "source": TABLE_SOURCE}
            if numbers["ampacity"] is not None
            else {"value": None, "unit": "A", "label": "UNKNOWN", "reason": "no cited ampacity row"}
        ),
        "drop_verdict": numbers["drop_verdict"],
        "verdict": numbers["verdict"],
        "colour_hint": COLOUR_HINTS[numbers["verdict"]],
        "reason": numbers["reason"],
        "insulation": {"colour": wire.colour, "rating_c": wire.insulation_rating_c},
        "route": _route_bits(wire.route),
    }


def _solve(body: dict):
    try:
        return solve_dc(load_circuit(body))
    except InputError as exc:
        if exc.code == "floating_node":
            raise SolverError(str(exc), code="floating_node") from exc
        raise


def _solve_scenario(circuit, doc, specs, supply, *, open_ids=(), joints=None, short=None) -> dict:
    body = copy.deepcopy(circuit.raw)
    body["analysis"] = {"type": "dc"}
    joints = joints or {}
    opened = set(open_ids)
    for wire, spec in zip(doc.wires, specs):
        if wire.id in opened:
            continue
        resistance = spec.resistance(REFERENCE_TEMP_C) + joints.get(wire.id, 0.0)
        body["components"].append(
            {
                "id": f"__wire__{wire.id}",
                "kind": "resistor",
                "nodes": [wire.from_node, wire.to_node],
                "value": {"value": resistance, "unit": "ohm"},
            }
        )
    if short is not None:
        body["components"].append(
            {
                "id": "__short__",
                "kind": "resistor",
                "nodes": [short[0].to_node, short[1].to_node],
                "value": {"value": SHORT_OHM, "unit": "ohm"},
            }
        )
    solution = _solve(body)
    entries = []
    for wire, spec in zip(doc.wires, specs):
        if wire.id in opened:
            current = 0.0
        else:
            v_from = solution.node_voltages.get(wire.from_node, 0.0)
            v_to = solution.node_voltages.get(wire.to_node, 0.0)
            current = abs(v_from - v_to) / (spec.resistance(REFERENCE_TEMP_C) + joints.get(wire.id, 0.0))
        numbers = _judge(spec, current, supply, open_wire=wire.id in opened, joint_ohm=joints.get(wire.id, 0.0))
        entries.append(_entry(wire, numbers, joint_ohm=joints.get(wire.id, 0.0)))
    result = {
        "status": "ok",
        "verdict": _worst_verdict(*(entry["verdict"] for entry in entries)),
        "wires": entries,
    }
    if short is not None:
        v_a = solution.node_voltages.get(short[0].to_node, 0.0)
        v_b = solution.node_voltages.get(short[1].to_node, 0.0)
        result["fault_current_a"] = _bits(abs(v_a - v_b) / SHORT_OHM, "A", LABELS["current"])
        result["model"] = SHORT_MODEL
    return result


def _describe(fault: Fault) -> str:
    wire = fault.wires[0]
    if fault.kind == "open":
        return f"{wire} is open, so no current can pass through it"
    if fault.kind == "joint":
        return f"{wire} has an injected high-resistance joint of {fault.value:g} ohm in series"
    if fault.kind == "short":
        return f"an injected short of {SHORT_OHM:g} ohm joins the far ends of {fault.wires[0]} and {fault.wires[1]}"
    return f"{wire} opens on a seeded share of {fault.value:g} of {INTERMITTENT_TRIALS} checks"


def _intermittent(circuit, doc, specs, supply, fault: Fault, seed: int) -> dict:
    rng = random.Random(seed)
    opened = no_solution = solved = 0
    lows: dict[str, float] = {}
    highs: dict[str, float] = {}
    for _ in range(INTERMITTENT_TRIALS):
        is_open = rng.random() < fault.value
        opened += int(is_open)
        try:
            scenario = _solve_scenario(
                circuit, doc, specs, supply, open_ids=(fault.wires[0],) if is_open else ()
            )
        except SolverError:
            no_solution += 1
            continue
        solved += 1
        for entry in scenario["wires"]:
            current = entry["current_a"]["value"]
            lows[entry["id"]] = min(lows.get(entry["id"], current), current)
            highs[entry["id"]] = max(highs.get(entry["id"], current), current)

    entries = []
    for wire, spec in zip(doc.wires, specs):
        if wire.id not in highs:
            entries.append(
                {
                    "id": wire.id,
                    "verdict": "UNKNOWN",
                    "colour_hint": "UNKNOWN",
                    "reason": "no seeded check reached a DC solution",
                }
            )
            continue
        numbers = _judge(spec, highs[wire.id], supply)
        entry = _entry(wire, numbers, label=LABELS["checks"])
        entry["current_min_a"] = _bits(lows[wire.id], "A", LABELS["checks"])
        if wire.id == fault.wires[0]:
            entry["verdict"] = "FAIL"
            entry["colour_hint"] = "FAIL"
            entry["reason"] = f"opens in {opened} of {INTERMITTENT_TRIALS} seeded checks"
        entries.append(entry)
    return {
        "status": "ok",
        "verdict": _worst_verdict(*(entry["verdict"] for entry in entries)),
        "checks": {
            "trials": INTERMITTENT_TRIALS,
            "seed": seed,
            "opened": opened,
            "solved": solved,
            "no_solution": no_solution,
        },
        "wires": entries,
    }


def link_harness(circuit, doc, *, faults=(), seed: int = DEFAULT_SEED) -> dict:
    """Solve the harness as built and once per fault; return the link report."""
    circuit = load_circuit(circuit)
    available = set(circuit.node_names) | {"0"}
    for wire in doc.wires:
        for node in (wire.from_node, wire.to_node):
            if node not in available:
                raise InputError(f"wire {wire.id!r} connects node {node!r}, which is not in the circuit")
    supply, supply_source = _supply_reference(circuit, None)
    specs = [_wire_spec(wire) for wire in doc.wires]
    wire_ids = {wire.id for wire in doc.wires}
    for fault in faults:
        for name in fault.wires:
            if name not in wire_ids:
                raise InputError(f"fault {fault.text!r} names an unknown wire {name!r}")

    base = _solve_scenario(circuit, doc, specs, supply)
    base["id"] = "as_built"
    base["fault"] = None
    base["description"] = "all wires fitted; the base DC operating point"
    scenarios = [base]
    fault_verdicts = []
    for index, fault in enumerate(faults):
        if fault.kind == "intermittent":
            scenario = _intermittent(circuit, doc, specs, supply, fault, seed)
        else:
            joints = {fault.wires[0]: fault.value} if fault.kind == "joint" else {}
            opened = fault.wires if fault.kind == "open" else ()
            short = (
                tuple(next(w for w in doc.wires if w.id == name) for name in fault.wires)
                if fault.kind == "short"
                else None
            )
            try:
                scenario = _solve_scenario(
                    circuit, doc, specs, supply, open_ids=opened, joints=joints, short=short
                )
            except SolverError as exc:
                scenario = {
                    "status": "no_solution",
                    "verdict": "FAIL",
                    "message": str(exc),
                    "wires": [],
                }
        scenario["id"] = f"fault-{index + 1}"
        scenario["fault"] = fault.text
        scenario["description"] = _describe(fault)
        scenarios.append(scenario)
        fault_verdicts.append(scenario["verdict"])

    return {
        "status": "ok",
        "command": "link-harness",
        "analysis": "dc",
        "supply": {"value": supply, "unit": "V", "label": LABELS["supply"], "source": supply_source},
        "labels": dict(LABELS),
        "notes": {
            "temperature": TEMPERATURE_REASON,
            "ampacity": "PASS up to 80 % of the cited row, WARN up to the row, FAIL above",
            "short_model": SHORT_MODEL,
            "verdict": "an UNKNOWN ampacity does not block a verdict; the drop decides",
        },
        "wires": base["wires"],
        "verdict": base["verdict"],
        "fault_verdict": _worst_verdict(*fault_verdicts) if fault_verdicts else None,
        "scenarios": scenarios,
    }
