"""E2 nonlinear targets: Shockley diode and level-1 MOSFET."""

import math

import pytest

from helqix_circuit import load_circuit, solve_dc

BOLTZMANN = 1.380649e-23  # CODATA 2018 exact
CHARGE = 1.602176634e-19  # CODATA 2018 exact
T_C = 27.0


def r(cid, a, b, ohms):
    return {"id": cid, "kind": "resistor", "nodes": [a, b], "value": {"value": ohms, "unit": "ohm"}}


def v(cid, a, b, volts):
    return {"id": cid, "kind": "vsource", "nodes": [a, b], "value": {"value": volts, "unit": "V"}}


def diode(cid, a, b, is_a=1e-12, n=1.0, rs_ohm=0.0):
    return {
        "id": cid,
        "kind": "diode",
        "nodes": [a, b],
        "parameters": {"is_a": is_a, "n": n, "rs_ohm": rs_ohm, "t_deg_c": T_C},
    }


def diode_reference_current(vs, series_ohms, is_a, n):
    """Bisection on Vd: Is(exp(Vd/(nVt))-1) = (Vs - Vd)/(R+Rs)."""
    vt = BOLTZMANN * (T_C + 273.15) / CHARGE
    low, high = 0.0, 40.0 * n * vt  # exp argument capped at 40

    def residual(vd):
        return is_a * (math.exp(vd / (n * vt)) - 1.0) - (vs - vd) / series_ohms

    for _ in range(200):
        middle = 0.5 * (low + high)
        if residual(middle) < 0.0:
            low = middle
        else:
            high = middle
    vd = 0.5 * (low + high)
    return (vs - vd) / series_ohms, vd


def test_diode_matches_shockley_within_one_percent():
    circuit = load_circuit(
        {
            "version": "circuit/0.1",
            "components": [v("V1", "1", "0", 5.0), r("R1", "1", "2", 1000.0), diode("D1", "2", "0")],
        }
    )
    solution = solve_dc(circuit)
    expected, expected_vd = diode_reference_current(5.0, 1000.0, 1e-12, 1.0)
    measured = solution.branch_currents["D1"]
    assert abs(measured - expected) / expected <= 0.01
    assert abs(solution.node_voltages["2"] - expected_vd) / expected_vd <= 0.01
    assert solution.report.converged is True
    assert solution.report.iterations >= 1


def test_diode_with_series_resistance_matches_shockley():
    circuit = load_circuit(
        {
            "version": "circuit/0.1",
            "components": [v("V1", "1", "0", 5.0), r("R1", "1", "2", 1000.0), diode("D1", "2", "0", rs_ohm=10.0)],
        }
    )
    solution = solve_dc(circuit)
    expected, _ = diode_reference_current(5.0, 1010.0, 1e-12, 1.0)
    measured = solution.branch_currents["D1"]
    assert abs(measured - expected) / expected <= 0.01


def mosfet(cid, d, g, s, b, channel, vth, k, lam=0.0):
    return {
        "id": cid,
        "kind": "mosfet",
        "nodes": [d, g, s, b],
        "parameters": {"channel": channel, "vth": vth, "k": k, "lambda": lam},
    }


def test_nmos_saturation_current_matches_level_one():
    circuit = load_circuit(
        {
            "version": "circuit/0.1",
            "components": [
                v("VDD", "vdd", "0", 10.0),
                r("RD", "vdd", "d", 1000.0),
                v("VG", "g", "0", 3.0),
                mosfet("M1", "d", "g", "0", "0", "n", 1.0, 1e-3),
            ],
        }
    )
    solution = solve_dc(circuit)
    expected_id = 0.5 * 1e-3 * (3.0 - 1.0) ** 2  # saturation, 2 mA
    expected_vd = 10.0 - expected_id * 1000.0
    # check the assumed region first: Vds = 8 V > Vgs - Vth = 2 V
    assert expected_vd > 3.0 - 1.0
    assert abs(solution.branch_currents["M1"] - expected_id) / expected_id <= 0.01
    assert abs(solution.node_voltages["d"] - expected_vd) / expected_vd <= 0.01


def test_nmos_triode_current_matches_level_one():
    circuit = load_circuit(
        {
            "version": "circuit/0.1",
            "components": [
                v("VDD", "vdd", "0", 10.0),
                r("RD", "vdd", "d", 10000.0),
                v("VG", "g", "0", 3.0),
                mosfet("M1", "d", "g", "0", "0", "n", 1.0, 1e-3),
            ],
        }
    )
    # K((Vgs-Vth)Vds - Vds^2/2) = (10 - Vds)/RD
    # 5 Vds^2 - 21 Vds + 10 = 0, triode root = (21 - sqrt(441 - 200))/10
    expected_vds = (21.0 - math.sqrt(441.0 - 200.0)) / 10.0
    expected_id = (10.0 - expected_vds) / 10000.0
    assert expected_vds < 2.0  # triode condition
    solution = solve_dc(circuit)
    assert abs(solution.branch_currents["M1"] - expected_id) / expected_id <= 0.01
    assert abs(solution.node_voltages["d"] - expected_vds) / expected_vds <= 0.01


def test_pmos_saturation_current_matches_level_one():
    circuit = load_circuit(
        {
            "version": "circuit/0.1",
            "components": [
                v("VDD", "vdd", "0", 10.0),
                r("RD", "d", "0", 1000.0),
                v("VG", "g", "0", 7.0),
                mosfet("M1", "d", "g", "vdd", "vdd", "p", 1.0, 1e-3),
            ],
        }
    )
    solution = solve_dc(circuit)
    # Vsg = 3 V, |Vth| = 1 V, |Id| = 2 mA; current flows source -> drain
    expected_id = -0.5 * 1e-3 * (3.0 - 1.0) ** 2
    expected_vd = -expected_id * 1000.0
    assert abs(solution.branch_currents["M1"] - expected_id) / abs(expected_id) <= 0.01
    assert abs(solution.node_voltages["d"] - expected_vd) / expected_vd <= 0.01


def test_capacitor_open_and_inductor_short_in_dc():
    capacitor = load_circuit(
        {
            "version": "circuit/0.1",
            "components": [
                v("V1", "1", "0", 10.0),
                r("R1", "1", "2", 1000.0),
                {"id": "C1", "kind": "capacitor", "nodes": ["2", "0"], "value": {"value": 1e-6, "unit": "F"}},
            ],
        }
    )
    cap_solution = solve_dc(capacitor)
    assert cap_solution.node_voltages["2"] == pytest.approx(10.0, rel=1e-12)
    assert cap_solution.branch_currents["C1"] == 0.0

    inductor = load_circuit(
        {
            "version": "circuit/0.1",
            "components": [
                v("V1", "1", "0", 10.0),
                r("R1", "1", "2", 1000.0),
                {"id": "L1", "kind": "inductor", "nodes": ["2", "0"], "value": {"value": 1e-3, "unit": "H"}},
            ],
        }
    )
    ind_solution = solve_dc(inductor)
    assert ind_solution.node_voltages["2"] == pytest.approx(0.0, abs=1e-12)
    assert ind_solution.branch_currents["L1"] == pytest.approx(10.0 / 1000.0, rel=1e-12)
