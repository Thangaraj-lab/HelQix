"""E2 AC targets: RLC resonance, RC cutoff and op-amp gain."""

import cmath
import math

import pytest

from helqix_circuit import AcAnalyzer, ac_sweep, cutoff_frequency, load_circuit, resonance_metrics


def r(cid, a, b, ohms):
    return {"id": cid, "kind": "resistor", "nodes": [a, b], "value": {"value": ohms, "unit": "ohm"}}


def v(cid, a, b, volts):
    return {"id": cid, "kind": "vsource", "nodes": [a, b], "value": {"value": volts, "unit": "V"}}


def ac_analysis(start=1000.0, stop=1000.0, points=2):
    return {"type": "ac", "start_hz": start, "stop_hz": stop, "points": points}


def test_rlc_resonance_frequency_and_q_within_one_percent():
    circuit = load_circuit(
        {
            "version": "circuit/0.1",
            "components": [
                v("V1", "1", "0", 1.0),
                r("R1", "1", "2", 10.0),
                {"id": "L1", "kind": "inductor", "nodes": ["2", "3"], "value": {"value": 1e-3, "unit": "H"}},
                {"id": "C1", "kind": "capacitor", "nodes": ["3", "0"], "value": {"value": 1e-6, "unit": "F"}},
            ],
            "analysis": ac_analysis(100.0, 100000.0),
        }
    )
    # measure the voltage across R (node 1 minus node 2): it peaks at f0
    metrics = resonance_metrics(circuit, "V1", "1", 100.0, 100000.0, reference_node="2")
    expected_f0 = 1.0 / (2.0 * math.pi * math.sqrt(1e-3 * 1e-6))
    expected_q = (1.0 / 10.0) * math.sqrt(1e-3 / 1e-6)
    assert abs(metrics["f_peak_hz"] - expected_f0) / expected_f0 <= 0.01
    assert abs(metrics["q"] - expected_q) / expected_q <= 0.01


def test_rc_lowpass_cutoff_within_one_percent():
    circuit = load_circuit(
        {
            "version": "circuit/0.1",
            "components": [
                v("V1", "1", "0", 1.0),
                r("R1", "1", "2", 1000.0),
                {"id": "C1", "kind": "capacitor", "nodes": ["2", "0"], "value": {"value": 100e-9, "unit": "F"}},
            ],
            "analysis": {"type": "ac", "start_hz": 1.0, "stop_hz": 1e6, "points_per_decade": 50},
        }
    )
    expected = 1.0 / (2.0 * math.pi * 1000.0 * 100e-9)
    measured = cutoff_frequency(circuit, "V1", "2")
    assert abs(measured - expected) / expected <= 0.01


def test_ac_sweep_returns_complex_transfer():
    circuit = load_circuit(
        {
            "version": "circuit/0.1",
            "components": [
                v("V1", "1", "0", 1.0),
                r("R1", "1", "2", 1000.0),
                {"id": "C1", "kind": "capacitor", "nodes": ["2", "0"], "value": {"value": 100e-9, "unit": "F"}},
            ],
            "analysis": {"type": "ac", "start_hz": 10.0, "stop_hz": 100000.0, "points_per_decade": 2},
        }
    )
    solution = ac_sweep(circuit)
    assert solution.frequencies.size == 9  # 4 decades * 2 points/decade + 1
    assert solution.node_voltages["2"].dtype.kind == "c"
    magnitude = abs(solution.transfer("V1", "2")[0])
    expected = 1.0 / math.sqrt(1.0 + (2.0 * math.pi * 10.0 * 1000.0 * 100e-9) ** 2)
    assert magnitude == pytest.approx(expected, rel=1e-6)


def inverting_amp(gain_params):
    return load_circuit(
        {
            "version": "circuit/0.1",
            "components": [
                v("V1", "1", "0", 1.0),
                r("R1", "1", "2", 1000.0),
                r("RF", "2", "3", 10000.0),
                {"id": "U1", "kind": "opamp", "nodes": ["3", "0", "2", "0"], "parameters": gain_params},
            ],
            "analysis": ac_analysis(),
        }
    )


def test_ideal_inverting_amplifier_gain_within_point_one_percent():
    analyzer = AcAnalyzer(inverting_amp({"ideal": True}))
    gain = analyzer.transfer("V1", "3", 1000.0)
    expected = -10000.0 / 1000.0
    assert abs(gain - expected) / abs(expected) <= 0.001
    assert abs(cmath.phase(gain) - math.pi) <= 1e-9


def test_finite_gain_inverting_amplifier_within_point_one_percent():
    open_loop = 1e5
    analyzer = AcAnalyzer(inverting_amp({"ideal": False, "gain": open_loop}))
    gain = analyzer.transfer("V1", "3", 1000.0)
    ratio = 10000.0 / 1000.0
    expected = -ratio / (1.0 + (1.0 + ratio) / open_loop)
    assert abs(gain - expected) / abs(expected) <= 0.001
    assert abs(cmath.phase(gain) - math.pi) <= 1e-3
