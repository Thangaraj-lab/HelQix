"""Harness checks: wire resistance, voltage drop, loss and ampacity.

The wires are inserted into a copy of the circuit as series resistors, the
DC operating point is solved, and each wire is judged on its own drop and on
a published ampacity row.  Every number carries a validation label; numbers
that depend on the convection assumption are labelled ASSUMPTION and are
never used to claim safety.
"""

from __future__ import annotations

import copy
import math
from dataclasses import dataclass, field

from .ampacity_table import METHODS, TABLE_SOURCE, copper_ampacity, nominal_size_mm2
from .errors import InputError
from .mna import solve_dc
from .netlist import Circuit, load_circuit

AMBIENT_C = 30.0
H_W_M2K = 10.0
REFERENCE_TEMP_C = 20.0
DROP_LIMIT_PERCENT = 3.0
DROP_WARN_PERCENT = 5.0
AMPERAGE_DESIGN_FRACTION = 0.8

MATERIALS = {
    "copper": (1.724e-8, 0.00393),
    "aluminium": (2.826e-8, 0.00403),
}

RESISTIVITY_SOURCE = (
    "resistivity at 20 degC and temperature coefficient from published "
    "resistivity tables (copper 1.724e-8 ohm m, 0.00393 /K; aluminium "
    "2.826e-8 ohm m, 0.00403 /K)"
)

LABELS = {
    "resistance_ohm": "CALCULATED",
    "current_a": "CALCULATED",
    "drop_v": "CALCULATED",
    "drop_percent": "CALCULATED",
    "power_w": "CALCULATED",
    "temperature_rise_c": "ASSUMPTION",
    "conductor_temp_c": "ASSUMPTION",
    "ampacity_a": "TABLE",
}


def awg_diameter_mm(gauge: float) -> float:
    # d_mm = 0.127 * 92 ** ((36 - n) / 39), n may be negative for 00, 000, 0000
    return 0.127 * 92.0 ** ((36.0 - gauge) / 39.0)


def _number(value, what: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise InputError(f"{what} must be a number")
    number = float(value)
    if not math.isfinite(number):
        raise InputError(f"{what} must be finite")
    return number


@dataclass(frozen=True)
class WireSpec:
    id: str
    from_node: str
    to_node: str
    length_m: float
    area_m2: float
    diameter_m: float
    material: str
    insulation_rating_c: float
    method: str

    @property
    def area_mm2(self) -> float:
        return self.area_m2 * 1e6

    @property
    def diameter_mm(self) -> float:
        return self.diameter_m * 1000.0

    def resistance(self, conductor_temp_c: float = REFERENCE_TEMP_C) -> float:
        rho20, alpha = MATERIALS[self.material]
        rho = rho20 * (1.0 + alpha * (conductor_temp_c - REFERENCE_TEMP_C))
        return rho * self.length_m / self.area_m2


def _wire(index: int, body) -> WireSpec:
    where = f"wire #{index + 1}"
    if not isinstance(body, dict):
        raise InputError(f"{where} must be an object")
    wid = body.get("id")
    if not isinstance(wid, str) or not wid.strip():
        raise InputError(f"{where} needs a non-empty string id")
    where = f"wire {wid!r}"
    from_node = body.get("from")
    to_node = body.get("to")
    for name, node in (("from", from_node), ("to", to_node)):
        if not isinstance(node, str) or not node:
            raise InputError(f"{where} needs a non-empty string {name!r} node")
    length_mm = _number(body.get("length_mm"), f"{where} length_mm")
    if length_mm <= 0:
        raise InputError(f"{where} length_mm must be greater than zero")

    gauge_body = body.get("gauge_awg")
    diameter_body = body.get("diameter_mm")
    has_gauge = gauge_body is not None
    has_diameter = diameter_body is not None
    if has_gauge == has_diameter:
        raise InputError(f"{where} needs exactly one of gauge_awg or diameter_mm")
    if has_gauge:
        gauge = _number(gauge_body, f"{where} gauge_awg")
        if not -20.0 < gauge < 60.0:
            raise InputError(f"{where} gauge_awg must be between -20 and 60")
        diameter_mm = awg_diameter_mm(gauge)
    else:
        diameter_mm = _number(diameter_body, f"{where} diameter_mm")
        if diameter_mm <= 0:
            raise InputError(f"{where} diameter_mm must be greater than zero")

    material = body.get("material")
    if material not in MATERIALS:
        raise InputError(f"{where} material must be one of {sorted(MATERIALS)}, got {material!r}")
    rating = _number(body.get("insulation_rating_c"), f"{where} insulation_rating_c")
    if rating <= 0:
        raise InputError(f"{where} insulation_rating_c must be greater than zero")
    method = body.get("installation_method", "C")
    if not isinstance(method, str):
        raise InputError(f"{where} installation_method must be a string")
    method = method.upper()
    if method not in METHODS:
        raise InputError(f"{where} installation_method must be one of {list(METHODS)}, got {method!r}")

    diameter_m = diameter_mm / 1000.0
    area_m2 = math.pi * (diameter_m / 2.0) ** 2
    return WireSpec(wid, from_node, to_node, length_mm / 1000.0, area_m2, diameter_m, material, rating, method)


@dataclass
class WireResult:
    id: str
    from_node: str
    to_node: str
    length_m: float
    area_mm2: float
    diameter_mm: float
    material: str
    method: str
    insulation_rating_c: float
    resistance_ohm: float
    current_a: float
    drop_v: float
    drop_percent: float
    power_w: float
    temperature_rise_c: float
    conductor_temp_c: float
    ampacity_a: float | None
    ampacity_size_mm2: float | None
    ampacity_source: str | None
    drop_verdict: str
    ampacity_verdict: str
    verdict: str
    reason: str
    labels: dict = field(default_factory=dict)
    sources: dict = field(default_factory=dict)

    def to_dict(self) -> dict:
        return {
            "id": self.id,
            "from": self.from_node,
            "to": self.to_node,
            "length_mm": {"value": self.length_m * 1000.0, "unit": "mm", "label": "CALCULATED"},
            "area_mm2": {"value": self.area_mm2, "unit": "mm^2", "label": "CALCULATED"},
            "diameter_mm": {"value": self.diameter_mm, "unit": "mm", "label": "CALCULATED"},
            "material": self.material,
            "installation_method": self.method,
            "insulation_rating_c": {"value": self.insulation_rating_c, "unit": "degC", "label": "GIVEN"},
            "resistance_ohm": {"value": self.resistance_ohm, "unit": "ohm", "label": LABELS["resistance_ohm"]},
            "current_a": {"value": self.current_a, "unit": "A", "label": LABELS["current_a"]},
            "drop_v": {"value": self.drop_v, "unit": "V", "label": LABELS["drop_v"]},
            "drop_percent": {"value": self.drop_percent, "unit": "%", "label": LABELS["drop_percent"]},
            "power_w": {"value": self.power_w, "unit": "W", "label": LABELS["power_w"]},
            "temperature_rise_c": {"value": self.temperature_rise_c, "unit": "K", "label": LABELS["temperature_rise_c"]},
            "conductor_temp_c": {"value": self.conductor_temp_c, "unit": "degC", "label": LABELS["conductor_temp_c"]},
            "ampacity_a": (
                {"value": self.ampacity_a, "unit": "A", "label": LABELS["ampacity_a"], "source": self.ampacity_source}
                if self.ampacity_a is not None
                else {"value": None, "unit": "A", "label": "UNKNOWN", "reason": "no cited ampacity row"}
            ),
            "drop_verdict": self.drop_verdict,
            "ampacity_verdict": self.ampacity_verdict,
            "verdict": self.verdict,
            "reason": self.reason,
        }


@dataclass
class HarnessReport:
    supply_voltage_v: float
    supply_source: str
    ambient_c: float
    h_w_m2k: float
    wires: dict[str, WireResult]
    verdict: str

    def to_dict(self) -> dict:
        return {
            "status": "ok",
            "analysis": "dc",
            "validation_label": "CALCULATED",
            "supply_voltage_v": {"value": self.supply_voltage_v, "unit": "V"},
            "supply_source": self.supply_source,
            "ambient_c": {"value": self.ambient_c, "unit": "degC", "label": "ASSUMPTION"},
            "h_w_m2k": {"value": self.h_w_m2k, "unit": "W/(m^2 K)", "label": "ASSUMPTION"},
            "temperature_note": "steady-state temperature rise is informational only and is not a safety claim",
            "verdict": self.verdict,
            "wires": {wid: result.to_dict() for wid, result in self.wires.items()},
            "ampacity_source": TABLE_SOURCE,
            "resistance_source": RESISTIVITY_SOURCE,
            "solver": {"name": "helqix_circuit.harness", "version": "0.1.0", "method": "DC MNA with wires as series resistors"},
        }


def _worst_verdict(*verdicts: str) -> str:
    for level in ("FAIL", "WARN", "UNKNOWN"):
        if level in verdicts:
            return level
    return "PASS"


def _drop_verdict(percent: float, limit: float, warn: float) -> str:
    if percent > warn + 1e-9:
        return "FAIL"
    if percent > limit + 1e-9:
        return "WARN"
    return "PASS"


def _ampacity_verdict(current: float, ampacity: float) -> str:
    if current > ampacity + 1e-9:
        return "FAIL"
    if current >= AMPERAGE_DESIGN_FRACTION * ampacity - 1e-9:
        return "WARN"
    return "PASS"


def _supply_reference(circuit: Circuit, supply_voltage_v) -> tuple[float, str]:
    if supply_voltage_v is not None:
        value = _number(supply_voltage_v, "supply_voltage_v")
        if value <= 0:
            raise InputError("supply_voltage_v must be greater than zero")
        return value, "argument"
    best: tuple[float, str] | None = None
    for component in circuit.components:
        if component.kind != "vsource":
            continue
        value = abs(float(component.value))
        if best is None or value > best[0]:
            best = (value, component.id)
    if best is None or best[0] == 0.0:
        raise InputError("supply_voltage_v is required when the circuit has no non-zero voltage source")
    return best


def analyze_harness(
    circuit,
    wires,
    supply_voltage_v=None,
    ambient_c: float = AMBIENT_C,
    conductor_temp_c: float = REFERENCE_TEMP_C,
    h_w_m2k: float = H_W_M2K,
    drop_limit_percent: float = DROP_LIMIT_PERCENT,
    drop_warn_percent: float = DROP_WARN_PERCENT,
) -> HarnessReport:
    circuit = load_circuit(circuit)
    ambient = _number(ambient_c, "ambient_c")
    if ambient <= -273.15:
        raise InputError("ambient_c must be above absolute zero (-273.15 degC)")
    conductor_temp = _number(conductor_temp_c, "conductor_temp_c")
    if conductor_temp <= -273.15:
        raise InputError("conductor_temp_c must be above absolute zero (-273.15 degC)")
    h = _number(h_w_m2k, "h_w_m2k")
    if h <= 0:
        raise InputError("h_w_m2k must be greater than zero")
    limit = _number(drop_limit_percent, "drop_limit_percent")
    warn = _number(drop_warn_percent, "drop_warn_percent")
    if not 0.0 < limit < warn:
        raise InputError("drop limits need 0 < drop_limit_percent < drop_warn_percent")

    supply, supply_source = _supply_reference(circuit, supply_voltage_v)

    if not isinstance(wires, (list, tuple)) or not wires:
        raise InputError("wires must be a non-empty list")
    specs: list[WireSpec] = []
    seen: set[str] = set()
    available = set(circuit.node_names) | {"0"}
    for index, body in enumerate(wires):
        spec = _wire(index, body)
        if spec.id in seen:
            raise InputError(f"duplicate wire id {spec.id!r}")
        seen.add(spec.id)
        for node in (spec.from_node, spec.to_node):
            if node not in available:
                raise InputError(f"wire {spec.id!r} connects node {node!r}, which is not in the circuit")
        specs.append(spec)

    body = copy.deepcopy(circuit.raw)
    body["analysis"] = {"type": "dc"}
    for spec in specs:
        body["components"].append(
            {
                "id": f"__wire__{spec.id}",
                "kind": "resistor",
                "nodes": [spec.from_node, spec.to_node],
                "value": {"value": spec.resistance(conductor_temp), "unit": "ohm"},
            }
        )
    loaded = load_circuit(body)
    solution = solve_dc(loaded)

    results: dict[str, WireResult] = {}
    for spec in specs:
        resistance = spec.resistance(conductor_temp)
        v_from = solution.node_voltages.get(spec.from_node, 0.0)
        v_to = solution.node_voltages.get(spec.to_node, 0.0)
        current = abs((v_from - v_to) / resistance)
        drop = current * resistance
        percent = 100.0 * drop / supply
        power = current * current * resistance
        perimeter = math.pi * spec.diameter_m
        rise = power / (h * perimeter * spec.length_m)
        conductor_c = ambient + rise

        drop_verdict = _drop_verdict(percent, limit, warn)
        ampacity, size = (None, None)
        if spec.material == "copper":
            ampacity, size = copper_ampacity(spec.area_m2, spec.method)
        if ampacity is None:
            ampacity_verdict = "UNKNOWN"
            if spec.material == "copper":
                ampacity_text = (
                    f"ampacity is UNKNOWN: no cited ampacity row for {spec.area_mm2:.2f} mm^2 copper "
                    f"in Table B.52.2"
                )
            else:
                ampacity_text = (
                    "ampacity is UNKNOWN: no cited ampacity row for aluminium in the transcribed "
                    "Table B.52.2 (copper rows only)"
                )
            ampacity_source = None
        else:
            ampacity_verdict = _ampacity_verdict(current, ampacity)
            ampacity_text = (
                f"{current:.1f} A is {ampacity_verdict}: PASS up to {AMPERAGE_DESIGN_FRACTION * ampacity:.1f} A, "
                f"WARN up to the {ampacity:.1f} A row for {size:g} mm^2 copper method {spec.method}"
            )
            ampacity_source = TABLE_SOURCE

        verdict = _worst_verdict(drop_verdict, ampacity_verdict)
        drop_text = (
            f"voltage drop {percent:.2f} % of the {supply:.6g} V supply is {drop_verdict} "
            f"(PASS up to {limit:g} %, WARN up to {warn:g} %, FAIL above)"
        )
        reason = f"{verdict}: " + "; ".join((drop_text, ampacity_text))

        results[spec.id] = WireResult(
            id=spec.id,
            from_node=spec.from_node,
            to_node=spec.to_node,
            length_m=spec.length_m,
            area_mm2=spec.area_mm2,
            diameter_mm=spec.diameter_mm,
            material=spec.material,
            method=spec.method,
            insulation_rating_c=spec.insulation_rating_c,
            resistance_ohm=resistance,
            current_a=current,
            drop_v=drop,
            drop_percent=percent,
            power_w=power,
            temperature_rise_c=rise,
            conductor_temp_c=conductor_c,
            ampacity_a=ampacity,
            ampacity_size_mm2=size,
            ampacity_source=ampacity_source,
            drop_verdict=drop_verdict,
            ampacity_verdict=ampacity_verdict,
            verdict=verdict,
            reason=reason,
            labels=dict(LABELS),
            sources={"resistance": RESISTIVITY_SOURCE, "ampacity": TABLE_SOURCE},
        )

    overall = _worst_verdict(*(result.verdict for result in results.values()))
    return HarnessReport(supply, supply_source, ambient, h, results, overall)
