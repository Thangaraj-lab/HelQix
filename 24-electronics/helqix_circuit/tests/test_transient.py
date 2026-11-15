"""E2 transient targets. Expected values are analytic formulas written here."""

import math

import pytest

from helqix_circuit import load_circuit, transient


def r(cid, a, b, ohms):
    return {"id": cid, "kind": "resistor", "nodes": [a, b], "value": {"value": ohms, "unit": "ohm"}}


def v(cid, a, b, volts):
    return {"id": cid, "kind": "vsource", "nodes": [a, b], "value": {"value": volts, "unit": "V"}}


def c(cid, a, b, farads, initial=0.0):
    return {"id": cid, "kind": "capacitor", "nodes": [a, b], "value": {"value": farads, "unit": "F"}, "initial": {"value": initial, "unit": "V"}}


def rc_case(method, step_divisor=200, stop_s=5e-3):
    tau = 1000.0 * 1e-6
    return load_circuit(
        {
            "version": "circuit/0.1",
            "components": [v("V1", "1", "0", 5.0), r("R1", "1", "2", 1000.0), c("C1", "2", "0", 1e-6)],
            "analysis": {
                "type": "tran",
                "stop_s": stop_s,
                "step_s": tau / step_divisor,
                "method": method,
                "adaptive": False,
            },
        }
    )


def test_rc_charge_trapezoidal_within_half_percent():
    tau = 1000.0 * 1e-6
    circuit = rc_case("trapezoidal")
    solution = transient(circuit)
    worst = 0.0
    for fraction in (0.5, 1.0, 2.0, 3.0, 5.0):
        t = fraction * tau
        expected = 5.0 * (1.0 - math.exp(-t / tau))
        measured = solution.value_at("2", t)
        relative = abs(measured - expected) / expected
        worst = max(worst, relative)
        assert relative <= 0.005
    # trapezoidal should be far better than the target at this step
    assert worst <= 1e-3
    assert solution.report["converged"] is True


def test_rc_charge_backward_euler_within_half_percent():
    tau = 1000.0 * 1e-6
    solution = transient(rc_case("backward_euler"))
    for fraction in (0.5, 1.0, 2.0, 3.0, 5.0):
        t = fraction * tau
        expected = 5.0 * (1.0 - math.exp(-t / tau))
        assert abs(solution.value_at("2", t) - expected) / expected <= 0.005


def test_halving_the_step_changes_rc_by_less_than_one_percent():
    fine = transient(rc_case("trapezoidal", step_divisor=400))
    coarse = transient(rc_case("trapezoidal", step_divisor=200))
    tau = 1000.0 * 1e-6
    for fraction in (0.5, 1.0, 2.0, 3.0, 5.0):
        t = fraction * tau
        a = fine.value_at("2", t)
        b = coarse.value_at("2", t)
        assert abs(a - b) / max(abs(a), 1e-12) <= 0.01


def test_adaptive_step_converges_and_matches_analytic():
    tau = 1000.0 * 1e-6
    circuit = load_circuit(
        {
            "version": "circuit/0.1",
            "components": [v("V1", "1", "0", 5.0), r("R1", "1", "2", 1000.0), c("C1", "2", "0", 1e-6)],
            "analysis": {
                "type": "tran",
                "stop_s": 5e-3,
                "step_s": tau / 20.0,
                "max_step_s": tau,
                "rtol": 1e-4,
                "method": "trapezoidal",
                "adaptive": True,
            },
        }
    )
    solution = transient(circuit)
    assert solution.report["converged"] is True
    assert solution.report["accepted_steps"] > 0
    assert abs(solution.report["final_time_s"] - 5e-3) <= 1e-12
    for fraction in (0.5, 1.0, 2.0, 3.0, 5.0):
        t = fraction * tau
        expected = 5.0 * (1.0 - math.exp(-t / tau))
        assert abs(solution.value_at("2", t) - expected) / expected <= 0.005


def test_transient_report_includes_newton_counts():
    solution = transient(rc_case("trapezoidal"))
    report = solution.report
    assert report["accepted_steps"] == 1000
    assert report["rejected_steps"] == 0
    assert report["newton_iterations"] >= 1000
    assert report["method"] == "trapezoidal"


def test_rlc_ring_uses_inductor_state():
    # A series RLC driven by a step should ring; energy must move through L.
    circuit = load_circuit(
        {
            "version": "circuit/0.1",
            "components": [
                {"id": "V1", "kind": "vsource", "nodes": ["1", "0"], "value": {"value": 1.0, "unit": "V"},
                 "waveform": {"type": "step", "delay_s": 0.0}},
                r("R1", "1", "2", 10.0),
                {"id": "L1", "kind": "inductor", "nodes": ["2", "3"], "value": {"value": 1e-3, "unit": "H"}},
                c("C1", "3", "0", 1e-6),
            ],
            "analysis": {"type": "tran", "stop_s": 1e-3, "step_s": 1e-6, "method": "trapezoidal", "adaptive": False},
        }
    )
    solution = transient(circuit)
    peak = max(solution.node_voltages["3"])
    # series RLC step response: the capacitor voltage peaks above the 1 V input
    assert peak > 1.0
    assert "L1" in solution.branch_currents
