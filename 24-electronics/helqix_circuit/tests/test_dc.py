"""E1 DC tests. Every expected value is a hand formula written here."""

import copy
import math

import pytest

from helqix_circuit import InputError, SolverError, load_circuit, solve_dc


def assert_rel(actual, expected, rel=1e-9):
    assert math.isclose(actual, expected, rel_tol=rel, abs_tol=1e-15), (actual, expected, abs(actual - expected) / max(abs(expected), 1e-300))


def circ(*components, **extra):
    body = {"version": "circuit/0.1", "components": list(components), "analysis": {"type": "dc"}}
    body.update(extra)
    return body


def r(cid, a, b, ohms):
    return {"id": cid, "kind": "resistor", "nodes": [a, b], "value": {"value": ohms, "unit": "ohm"}}


def v(cid, a, b, volts):
    return {"id": cid, "kind": "vsource", "nodes": [a, b], "value": {"value": volts, "unit": "V"}}


def i(cid, a, b, amps):
    return {"id": cid, "kind": "isource", "nodes": [a, b], "value": {"value": amps, "unit": "A"}}


# ---------------------------------------------------------------- divider
def test_voltage_divider_matches_hand_formula():
    circuit = load_circuit(circ(v("V1", "1", "0", 10.0), r("R1", "1", "2", 1000.0), r("R2", "2", "0", 2000.0)))
    sol = solve_dc(circuit)
    # V2 = Vs * R2 / (R1 + R2) = 10 * 2000 / 3000
    assert_rel(sol.node_voltages["2"], 10.0 * 2000.0 / (1000.0 + 2000.0))
    assert_rel(sol.branch_currents["V1"], -10.0 / 3000.0)
    assert_rel(sol.branch_currents["R1"], 10.0 / 3000.0)


# ---------------------------------------------------------------- bridge
def test_wheatstone_bridge_matches_simultaneous_equations():
    circuit = load_circuit(
        circ(
            v("V1", "1", "0", 5.0),
            r("R1", "1", "2", 1000.0),
            r("R2", "2", "0", 1000.0),
            r("R3", "1", "3", 2000.0),
            r("R4", "3", "0", 3000.0),
            r("R5", "2", "3", 1000.0),
        )
    )
    sol = solve_dc(circuit)
    # KCL node 2: (V2-5)/1000 + V2/1000 + (V2-V3)/1000 = 0  ->  3 V2 - V3 = 5
    # KCL node 3: (V3-5)/2000 + V3/3000 + (V3-V2)/1000 = 0  -> -6 V2 + 11 V3 = 15
    # Cramer: det = 3*11 - (-1)*(-6) = 27
    #         V2 = (5*11 - (-1)*15)/27 = 70/27
    #         V3 = (3*15 - 5*(-6))/27  = 75/27 = 25/9
    assert_rel(sol.node_voltages["2"], 70.0 / 27.0)
    assert_rel(sol.node_voltages["3"], 25.0 / 9.0)


# ---------------------------------------------------------------- thevenin / norton
def test_thevenin_equivalent_matches_hand_calculation():
    # open circuit: V_A = 12 * 6/(4+6) = 7.2 V
    # R_th = 4||6 = 2.4 ohm
    # with 1.6 ohm load: V = 7.2 * 1.6/(2.4+1.6) = 2.88 V
    circuit = load_circuit(
        circ(
            v("V1", "1", "0", 12.0),
            r("R1", "1", "A", 4.0),
            r("R2", "A", "0", 6.0),
            r("RL", "A", "0", 1.6),
        )
    )
    assert_rel(solve_dc(circuit).node_voltages["A"], 7.2 * 1.6 / (2.4 + 1.6))

    open_circuit = load_circuit(circ(v("V1", "1", "0", 12.0), r("R1", "1", "A", 4.0), r("R2", "A", "0", 6.0)))
    open_sol = solve_dc(open_circuit)
    v_oc = open_sol.node_voltages["A"]
    assert_rel(v_oc, 12.0 * 6.0 / (4.0 + 6.0))
    # short circuit current through a 0 V source from A to 0
    short = load_circuit(
        circ(
            v("V1", "1", "0", 12.0),
            r("R1", "1", "A", 4.0),
            r("R2", "A", "0", 6.0),
            v("SHORT", "A", "0", 0.0),
        )
    )
    i_sc = abs(solve_dc(short).branch_currents["SHORT"])
    # R_th = V_oc / I_sc = 7.2 / 3.0 = 2.4
    assert_rel(v_oc / i_sc, 4.0 * 6.0 / (4.0 + 6.0))


def test_norton_equivalent_gives_same_load_voltage():
    # Norton: I_N = 12/4 = 3 A into node A, R_N = 4||6 = 2.4 ohm
    # load 1.6: V = 3 * (2.4*1.6)/(2.4+1.6) = 3 * 0.96 = 2.88 V
    circuit = load_circuit(
        circ(
            i("I1", "0", "A", 3.0),
            r("RN", "A", "0", 2.4),
            r("RL", "A", "0", 1.6),
        )
    )
    assert_rel(solve_dc(circuit).node_voltages["A"], 3.0 * (2.4 * 1.6) / (2.4 + 1.6))


# ---------------------------------------------------------------- superposition
def test_superposition_sum_equals_full_solution():
    full = load_circuit(
        circ(
            v("V1", "S1", "0", 10.0),
            r("R1", "S1", "A", 1000.0),
            v("V2", "S2", "0", 5.0),
            r("R2", "S2", "A", 2000.0),
            r("R3", "A", "0", 1000.0),
        )
    )
    v_full = solve_dc(full).node_voltages["A"]

    only_v1 = load_circuit(
        circ(
            v("V1", "S1", "0", 10.0),
            r("R1", "S1", "A", 1000.0),
            v("V2", "S2", "0", 0.0),
            r("R2", "S2", "A", 2000.0),
            r("R3", "A", "0", 1000.0),
        )
    )
    only_v2 = load_circuit(
        circ(
            v("V1", "S1", "0", 0.0),
            r("R1", "S1", "A", 1000.0),
            v("V2", "S2", "0", 5.0),
            r("R2", "S2", "A", 2000.0),
            r("R3", "A", "0", 1000.0),
        )
    )
    part1 = solve_dc(only_v1).node_voltages["A"]
    part2 = solve_dc(only_v2).node_voltages["A"]
    # part1 = 10 * (1k || 2k) / (1k + 1k||2k) = 4.0
    assert_rel(part1, 10.0 * (1000.0 * 2000.0 / 3000.0) / (1000.0 + 1000.0 * 2000.0 / 3000.0))
    # part2 = 5 * (1k || 1k) / (2k + 1k||1k) = 1.0
    assert_rel(part2, 5.0 * (1000.0 * 1000.0 / 2000.0) / (2000.0 + 1000.0 * 1000.0 / 2000.0))
    assert_rel(v_full, part1 + part2)
    # direct nodal solution: (Va-10)/1000 + (Va-5)/2000 + Va/1000 = 0 -> 5 Va = 25
    assert_rel(v_full, 25.0 / 5.0)


# ---------------------------------------------------------------- 10-node ladder
def test_ten_node_ladder_matches_closed_form():
    n = 10
    rser = 1000.0
    vs = 10.0
    components = [v("V1", "s", "0", vs), r("Rs", "s", "1", rser)]
    for k in range(1, n + 1):
        if k < n:
            components.append(r(f"R{k}_{k+1}", str(k), str(k + 1), rser))
        components.append(r(f"R{k}g", str(k), "0", rser))
    circuit = load_circuit(circ(*components))
    sol = solve_dc(circuit)

    # closed form: eq[n] = R; eq[k] = R || (R + eq[k+1])
    eq = [None] * (n + 1)
    eq[n] = rser
    for k in range(n - 1, 0, -1):
        eq[k] = (rser * (rser + eq[k + 1])) / (2.0 * rser + eq[k + 1])
    expected = [0.0] * (n + 1)
    expected[1] = vs * eq[1] / (rser + eq[1])
    for k in range(2, n + 1):
        expected[k] = expected[k - 1] * eq[k] / (rser + eq[k])
    for k in range(1, n + 1):
        assert_rel(sol.node_voltages[str(k)], expected[k])


# ---------------------------------------------------------------- source signs
def test_current_source_sign_follows_spice_convention():
    # current flows inside the source from nodes[0] to nodes[1]; node 1 loses 1 mA
    circuit = load_circuit(circ(i("I1", "1", "0", 1e-3), r("R1", "1", "0", 1000.0)))
    assert_rel(solve_dc(circuit).node_voltages["1"], -1e-3 * 1000.0)

    reversed_source = load_circuit(circ(i("I1", "0", "1", 1e-3), r("R1", "1", "0", 1000.0)))
    assert_rel(solve_dc(reversed_source).node_voltages["1"], 1e-3 * 1000.0)


def test_vcvs_and_vccs_stamps():
    vcvs = load_circuit(
        circ(
            v("V1", "1", "0", 2.0),
            {"id": "E1", "kind": "vcvs", "nodes": ["2", "0", "1", "0"], "value": {"value": 3.0, "unit": "1"}},
            r("RL", "2", "0", 1000.0),
        )
    )
    sol = solve_dc(vcvs)
    assert_rel(sol.node_voltages["2"], 3.0 * 2.0)
    # resistor current 6 mA leaves node 2; the source supplies it, so its
    # branch current (node 2 to 0 through the source) is -6 mA
    assert_rel(sol.branch_currents["E1"], -6.0 / 1000.0)

    vccs = load_circuit(
        circ(
            v("V1", "1", "0", 2.0),
            {"id": "G1", "kind": "vccs", "nodes": ["2", "0", "1", "0"], "value": {"value": 1e-3, "unit": "S"}},
            r("R1", "2", "0", 1000.0),
        )
    )
    # 2 mA leaves node 2 through G1; the resistor must supply it: V2/1k + 2mA = 0
    assert_rel(solve_dc(vccs).node_voltages["2"], -(1e-3 * 2.0) * 1000.0)


def test_ideal_switch_closed_and_open():
    closed = load_circuit(
        circ(
            v("V1", "1", "0", 10.0),
            r("R1", "1", "2", 1000.0),
            {"id": "S1", "kind": "switch", "nodes": ["2", "3"], "state": "closed"},
            r("R2", "3", "0", 1000.0),
        )
    )
    sol = solve_dc(closed)
    assert_rel(sol.node_voltages["2"], 5.0)
    assert_rel(sol.node_voltages["3"], 5.0)

    opened = load_circuit(
        circ(
            v("V1", "1", "0", 10.0),
            r("R1", "1", "2", 1000.0),
            {"id": "S1", "kind": "switch", "nodes": ["2", "3"], "state": "open"},
            r("R2", "3", "0", 1000.0),
        )
    )
    open_sol = solve_dc(opened)
    assert_rel(open_sol.node_voltages["2"], 10.0)
    assert_rel(open_sol.node_voltages["3"], 0.0)


# ---------------------------------------------------------------- refusals
def test_floating_node_is_refused_with_name():
    circuit = load_circuit(circ(v("V1", "1", "0", 5.0), r("R1", "1", "2", 1000.0), i("I1", "3", "0", 1e-3)))
    with pytest.raises(InputError) as exc:
        solve_dc(circuit)
    assert "3" in str(exc.value)
    assert "floating" in str(exc.value).lower()


def test_voltage_source_loop_is_refused_with_names():
    circuit = load_circuit(circ(v("V1", "1", "0", 5.0), v("V2", "1", "0", 3.0)))
    with pytest.raises(InputError) as exc:
        solve_dc(circuit)
    message = str(exc.value)
    assert "voltage-source loop" in message.lower()
    assert "V1" in message and "V2" in message


def test_singular_circuit_is_refused_with_plain_message():
    # independent V2 fixes node 2 at 3 V while E1 forces it to 2*V1 = 10 V
    circuit = load_circuit(
        circ(
            v("V1", "1", "0", 5.0),
            {"id": "E1", "kind": "vcvs", "nodes": ["2", "0", "1", "0"], "value": {"value": 2.0, "unit": "1"}},
            v("V2", "2", "0", 3.0),
        )
    )
    with pytest.raises(SolverError) as exc:
        solve_dc(circuit)
    assert "singular" in str(exc.value).lower()


def test_bad_inputs_are_rejected():
    with pytest.raises(InputError):
        load_circuit({"version": "circuit/9.9", "components": []})
    with pytest.raises(InputError):
        load_circuit(circ(r("R1", "1", "0", 1000.0), r("R1", "1", "0", 2000.0)))
    with pytest.raises(InputError):
        load_circuit(circ({"id": "R1", "kind": "resistor", "nodes": ["1", "0"], "value": {"value": 1000.0, "unit": "V"}}))
    with pytest.raises(InputError):
        load_circuit(circ({"id": "X1", "kind": "mystery", "nodes": ["1", "0"], "value": {"value": 1.0, "unit": "1"}}))
    with pytest.raises(InputError):
        load_circuit(circ({"id": "R1", "kind": "resistor", "nodes": ["1"], "value": {"value": 1.0, "unit": "ohm"}}))
    with pytest.raises(InputError):
        load_circuit(circ(r("R1", "1", "0", 0.0)))


def test_result_carries_units_and_validation_label():
    circuit = load_circuit(circ(v("V1", "1", "0", 10.0), r("R1", "1", "0", 1000.0)))
    payload = solve_dc(circuit).to_dict()
    assert payload["validation_label"] == "CALCULATED"
    assert payload["node_voltages"]["1"]["unit"] == "V"
    assert payload["branch_currents"]["V1"]["unit"] == "A"


def test_load_circuit_does_not_mutate_input():
    body = circ(v("V1", "1", "0", 10.0), r("R1", "1", "0", 1000.0))
    snapshot = copy.deepcopy(body)
    load_circuit(body)
    assert body == snapshot
