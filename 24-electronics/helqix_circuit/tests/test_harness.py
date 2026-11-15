"""E3 harness tests. Every expected value is a hand formula written here.

Ampacity rows come from IEC 60364-5-52 Annex B, Table B.52.2 (PVC, two
loaded conductors, copper, conductor 70 degC, ambient 30 degC).  The values
used below are the ones transcribed into helqix_circuit/ampacity_table.py
from a published copy of that table.
"""

import math

import pytest

from helqix_circuit import InputError, load_circuit
from helqix_circuit.harness import (
    AMBIENT_C,
    H_W_M2K,
    analyze_harness,
    awg_diameter_mm,
)

RHO_COPPER = 1.724e-8
ALPHA_COPPER = 0.00393
RHO_ALUMINIUM = 2.826e-8
ALPHA_ALUMINIUM = 0.00403


def circuit_with(*components):
    return load_circuit(
        {
            "version": "circuit/0.1",
            "components": list(components),
            "analysis": {"type": "dc"},
        }
    )


def isource(cid, a, b, amps):
    return {"id": cid, "kind": "isource", "nodes": [a, b], "value": {"value": amps, "unit": "A"}}


def wire(cid, a, b, length_mm, material="copper", rating=70.0, gauge=None, diameter=None, method="C"):
    body = {
        "id": cid,
        "from": a,
        "to": b,
        "length_mm": length_mm,
        "material": material,
        "insulation_rating_c": rating,
        "installation_method": method,
    }
    if gauge is not None:
        body["gauge_awg"] = gauge
    if diameter is not None:
        body["diameter_mm"] = diameter
    return body


def area_m2(d_mm):
    return math.pi * (d_mm / 2000.0) ** 2


# ---------------------------------------------------------------- geometry
def test_awg_diameter_matches_the_definition():
    # d_mm = 0.127 * 92 ** ((36 - n) / 39)
    assert awg_diameter_mm(12) == pytest.approx(0.127 * 92.0 ** (24.0 / 39.0), rel=1e-15)
    assert awg_diameter_mm(24) == pytest.approx(0.127 * 92.0 ** (12.0 / 39.0), rel=1e-15)
    # published 12 AWG diameter is about 2.053 mm
    assert abs(awg_diameter_mm(12) - 2.053) < 0.001


# ---------------------------------------------------------------- resistance
def test_wire_resistance_matches_rho_l_over_a():
    # 10 m of 2.5 mm^2 copper at 20 degC: R = rho * L / A
    expected = RHO_COPPER * 10.0 / 2.5e-6
    report = analyze_harness(
        circuit_with(isource("I1", "1", "0", 1.0)),
        [wire("W1", "1", "0", 10000.0, diameter=math.sqrt(4.0 * 2.5e-6 / math.pi) * 1000.0)],
        supply_voltage_v=12.0,
    )
    measured = report.wires["W1"].resistance_ohm
    assert abs(measured - expected) / expected <= 1e-6


def test_temperature_correction_uses_alpha():
    # R(T) = R(20) * (1 + alpha * (T - 20))
    diameter_mm = math.sqrt(4.0 * 2.5e-6 / math.pi) * 1000.0
    report = analyze_harness(
        circuit_with(isource("I1", "1", "0", 1.0)),
        [wire("W1", "1", "0", 10000.0, diameter=diameter_mm)],
        supply_voltage_v=12.0,
        conductor_temp_c=50.0,
    )
    expected = RHO_COPPER * 10.0 / 2.5e-6 * (1.0 + ALPHA_COPPER * 30.0)
    measured = report.wires["W1"].resistance_ohm
    assert abs(measured - expected) / expected <= 1e-6


def test_aluminium_resistivity_is_used_for_aluminium():
    diameter_mm = math.sqrt(4.0 * 2.5e-6 / math.pi) * 1000.0
    report = analyze_harness(
        circuit_with(isource("I1", "1", "0", 1.0)),
        [wire("W1", "1", "0", 10000.0, material="aluminium", diameter=diameter_mm)],
        supply_voltage_v=12.0,
        conductor_temp_c=50.0,
    )
    expected = RHO_ALUMINIUM * 10.0 / 2.5e-6 * (1.0 + ALPHA_ALUMINIUM * 30.0)
    measured = report.wires["W1"].resistance_ohm
    assert abs(measured - expected) / expected <= 1e-6


# ---------------------------------------------------------------- drop / loss
def test_wire_in_series_with_a_load_drop_matches_hand_formula():
    # 12 V source, 10 ohm load, 2 m of 0.5 mm^2 copper.
    # R_w = rho * L / A; I = 12 / (10 + R_w); drop = I * R_w
    diameter_mm = math.sqrt(4.0 * 0.5e-6 / math.pi) * 1000.0
    r_w = RHO_COPPER * 2.0 / 0.5e-6
    current = 12.0 / (10.0 + r_w)
    expected_drop = current * r_w
    expected_percent = 100.0 * expected_drop / 12.0

    circuit = circuit_with(
        {"id": "V1", "kind": "vsource", "nodes": ["1", "0"], "value": {"value": 12.0, "unit": "V"}},
        {"id": "R1", "kind": "resistor", "nodes": ["2", "0"], "value": {"value": 10.0, "unit": "ohm"}},
    )
    report = analyze_harness(circuit, [wire("W1", "1", "2", 2000.0, diameter=diameter_mm)])
    result = report.wires["W1"]
    assert abs(result.current_a - current) / current <= 1e-6
    assert abs(result.drop_v - expected_drop) / expected_drop <= 1e-6
    assert abs(result.drop_percent - expected_percent) / expected_percent <= 1e-6
    assert abs(result.power_w - current * current * r_w) / (current * current * r_w) <= 1e-6


def test_temperature_rise_uses_the_documented_convection_assumption():
    # dT = P / (h * perimeter * L), h = 10 W/(m^2 K) is an ASSUMPTION
    diameter_mm = math.sqrt(4.0 * 1.5e-6 / math.pi) * 1000.0
    perimeter_m = math.pi * diameter_mm / 1000.0
    r_w = RHO_COPPER * 3.0 / 1.5e-6
    current = 5.0
    expected_rise = current * current * r_w / (H_W_M2K * perimeter_m * 3.0)
    report = analyze_harness(
        circuit_with(isource("I1", "1", "0", current)),
        [wire("W1", "1", "0", 3000.0, diameter=diameter_mm)],
        supply_voltage_v=12.0,
    )
    result = report.wires["W1"]
    assert abs(result.temperature_rise_c - expected_rise) / expected_rise <= 1e-6
    assert abs(result.conductor_temp_c - (AMBIENT_C + expected_rise)) <= 1e-9
    assert result.labels["temperature_rise_c"] == "ASSUMPTION"
    assert result.labels["resistance_ohm"] == "CALCULATED"


# ---------------------------------------------------------------- ampacity
def test_ampacity_row_is_cited_and_not_invented():
    # IEC 60364-5-52 Table B.52.2, PVC, 2 loaded conductors, copper,
    # conductor 70 degC, ambient 30 degC, method C, 2.5 mm^2 -> 27 A
    diameter_mm = math.sqrt(4.0 * 2.5e-6 / math.pi) * 1000.0
    report = analyze_harness(
        circuit_with(isource("I1", "1", "0", 10.0)),
        [wire("W1", "1", "0", 1000.0, diameter=diameter_mm, method="C")],
        supply_voltage_v=12.0,
    )
    result = report.wires["W1"]
    assert result.ampacity_a == pytest.approx(27.0)
    assert "IEC 60364-5-52" in result.ampacity_source
    assert "B.52.2" in result.ampacity_source
    assert "method C" in result.ampacity_source
    assert result.labels["ampacity_a"] == "TABLE"


def test_ampacity_verdict_on_both_sides_of_each_limit():
    # limit is the table value; 80 % of it is the continuous-load design rule
    diameter_mm = math.sqrt(4.0 * 2.5e-6 / math.pi) * 1000.0

    def verdict_at(current_a):
        report = analyze_harness(
            circuit_with(isource("I1", "1", "0", current_a)),
            [wire("W1", "1", "0", 1000.0, diameter=diameter_mm, method="C")],
            supply_voltage_v=12.0,
        )
        return report.wires["W1"].ampacity_verdict

    assert verdict_at(27.0 * 0.8 - 0.01) == "PASS"
    assert verdict_at(27.0 * 0.8) == "WARN"
    assert verdict_at(27.0 * 0.8 + 0.01) == "WARN"
    assert verdict_at(27.0) == "WARN"
    assert verdict_at(27.0 + 0.01) == "FAIL"


def test_drop_verdict_on_both_sides_of_each_limit():
    # 1 A through 2.5 mm^2 copper; choose length to hit the percentage.
    # L = R * A / rho, R = percent/100 * 10 V / 1 A
    area = 2.5e-6

    def verdict_at(percent):
        resistance = percent / 100.0 * 10.0 / 1.0
        length_mm = resistance * area / RHO_COPPER * 1000.0
        diameter_mm = math.sqrt(4.0 * area / math.pi) * 1000.0
        report = analyze_harness(
            circuit_with(isource("I1", "1", "0", 1.0)),
            [wire("W1", "1", "0", length_mm, diameter=diameter_mm)],
            supply_voltage_v=10.0,
        )
        return report.wires["W1"].drop_verdict

    assert verdict_at(2.99) == "PASS"
    assert verdict_at(3.0) == "PASS"
    assert verdict_at(3.01) == "WARN"
    assert verdict_at(5.0) == "WARN"
    assert verdict_at(5.01) == "FAIL"


def test_untabulated_size_gets_unknown_and_no_verdict():
    # 0.75 mm^2 is not a row of the cited table.
    diameter_mm = math.sqrt(4.0 * 0.75e-6 / math.pi) * 1000.0
    report = analyze_harness(
        circuit_with(isource("I1", "1", "0", 1.0)),
        [wire("W1", "1", "0", 1000.0, diameter=diameter_mm)],
        supply_voltage_v=12.0,
    )
    result = report.wires["W1"]
    assert result.ampacity_a is None
    assert result.ampacity_verdict == "UNKNOWN"
    assert report.verdict == "UNKNOWN"
    assert "no cited ampacity row" in result.reason


def test_aluminium_ampacity_is_unknown_because_no_row_was_transcribed():
    diameter_mm = math.sqrt(4.0 * 2.5e-6 / math.pi) * 1000.0
    report = analyze_harness(
        circuit_with(isource("I1", "1", "0", 1.0)),
        [wire("W1", "1", "0", 1000.0, material="aluminium", diameter=diameter_mm)],
        supply_voltage_v=12.0,
    )
    result = report.wires["W1"]
    assert result.ampacity_a is None
    assert result.ampacity_verdict == "UNKNOWN"
    assert "aluminium" in result.reason


def test_overload_is_never_reported_as_pass():
    # 1.5 mm^2 method C is 19.5 A in the cited table; 100 A must be FAIL
    diameter_mm = math.sqrt(4.0 * 1.5e-6 / math.pi) * 1000.0
    report = analyze_harness(
        circuit_with(isource("I1", "1", "0", 100.0)),
        [wire("W1", "1", "0", 1000.0, diameter=diameter_mm)],
        supply_voltage_v=12.0,
    )
    result = report.wires["W1"]
    assert result.ampacity_verdict == "FAIL"
    assert result.verdict == "FAIL"
    assert report.verdict == "FAIL"
    assert result.reason.startswith("FAIL")
    assert "100.0 A" in result.reason


def test_overall_verdict_is_the_worst_wire():
    good = wire("W1", "1", "0", 1000.0, diameter=math.sqrt(4.0 * 2.5e-6 / math.pi) * 1000.0)
    bad = wire("W2", "2", "0", 1000.0, diameter=math.sqrt(4.0 * 1.5e-6 / math.pi) * 1000.0)
    report = analyze_harness(
        circuit_with(isource("I1", "1", "0", 5.0), isource("I2", "2", "0", 50.0)),
        [good, bad],
        supply_voltage_v=12.0,
    )
    assert report.wires["W1"].verdict == "PASS"
    assert report.wires["W2"].verdict == "FAIL"
    assert report.verdict == "FAIL"


# ---------------------------------------------------------------- validation
@pytest.mark.parametrize(
    "body",
    [
        {"length_mm": 0.0},
        {"length_mm": -1.0},
        {"length_mm": float("nan")},
        {"material": "unobtainium"},
        {"insulation_rating_c": 0.0},
        {"gauge_awg": 24},
        {"diameter_mm": None},
        {"installation_method": "Z"},
    ],
)
def test_bad_wire_descriptions_are_refused(body):
    base = wire("W1", "1", "0", 1000.0, diameter=0.5)
    base.update(body)
    with pytest.raises(InputError):
        analyze_harness(circuit_with(isource("I1", "1", "0", 1.0)), [base], supply_voltage_v=12.0)


def test_wire_must_connect_nodes_that_exist():
    with pytest.raises(InputError):
        analyze_harness(
            circuit_with(isource("I1", "1", "0", 1.0)),
            [wire("W1", "1", "9", 1000.0, gauge=20)],
            supply_voltage_v=12.0,
        )


def test_duplicate_wire_ids_are_refused():
    w = wire("W1", "1", "0", 1000.0, gauge=20)
    with pytest.raises(InputError):
        analyze_harness(circuit_with(isource("I1", "1", "0", 1.0)), [w, dict(w)], supply_voltage_v=12.0)


def test_bad_supply_voltage_and_ambient_are_refused():
    w = [wire("W1", "1", "0", 1000.0, gauge=20)]
    circuit = circuit_with(isource("I1", "1", "0", 1.0))
    for kwargs in ({"supply_voltage_v": 0.0}, {"supply_voltage_v": float("nan")}, {"ambient_c": -300.0}):
        with pytest.raises(InputError):
            analyze_harness(circuit, w, **kwargs)
