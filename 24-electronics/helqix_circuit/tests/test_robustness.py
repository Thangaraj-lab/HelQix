"""Review regressions: source breakpoints, input validation, honest refusals.

These came from an adversarial pass over the public API: a step source at t=0
made the adaptive controller crawl, non-finite analysis fields slipped through,
and a diode could be asked for a temperature below absolute zero.
"""

import math

import pytest

from helqix_circuit import InputError, load_circuit, solve_dc, transient

TAU = 1000.0 * 1e-6


def r(cid, a, b, ohms):
    return {"id": cid, "kind": "resistor", "nodes": [a, b], "value": {"value": ohms, "unit": "ohm"}}


def v(cid, a, b, volts, waveform=None):
    body = {"id": cid, "kind": "vsource", "nodes": [a, b], "value": {"value": volts, "unit": "V"}}
    if waveform is not None:
        body["waveform"] = waveform
    return body


def c(cid, a, b, farads, initial=0.0):
    return {"id": cid, "kind": "capacitor", "nodes": [a, b],
            "value": {"value": farads, "unit": "F"}, "initial": {"value": initial, "unit": "V"}}


def rc(step_delay_s=0.0, waveform="step", stop_s=5e-3, step_s=TAU / 10.0):
    source = v("V1", "1", "0", 1.0)
    if waveform == "step":
        source["waveform"] = {"type": "step", "delay_s": step_delay_s}
    return load_circuit({
        "version": "circuit/0.1",
        "components": [source, r("R1", "1", "2", 1000.0), c("C1", "2", "0", 1e-6)],
        "analysis": {"type": "tran", "stop_s": stop_s, "step_s": step_s},
    })


def test_adaptive_transient_handles_step_source_at_zero():
    # The operating point at t=0+ must use the source value at t=0, otherwise
    # the first step starts from the wrong capacitor current and never settles.
    solution = transient(rc(0.0), max_steps=5000)
    expected = 1.0 - math.exp(-5.0)
    assert abs(solution.value_at("2", 5.0 * TAU) - expected) / expected <= 0.005
    assert solution.report["final_time_s"] == pytest.approx(5e-3, abs=1e-12)
    assert solution.report["accepted_steps"] < 1000


def test_adaptive_transient_lands_on_delayed_step_breakpoint():
    solution = transient(rc(1e-3), max_steps=5000)
    early = 1.0 - math.exp(-0.5)
    late = 1.0 - math.exp(-4.0)
    assert abs(solution.value_at("2", 1.5e-3) - early) / early <= 0.005
    assert abs(solution.value_at("2", 5e-3) - late) / late <= 0.005
    assert solution.report["final_time_s"] == pytest.approx(5e-3, abs=1e-12)
    assert solution.report["accepted_steps"] < 1000


def test_adaptive_transient_handles_pulse_edges_without_stalling():
    circuit = load_circuit({
        "version": "circuit/0.1",
        "components": [
            v("V1", "1", "0", 1.0, {"type": "pulse", "amplitude": 1.0, "period_s": 1e-3,
                                    "duty": 0.5, "delay_s": 0.0}),
            r("R1", "1", "2", 1000.0),
            c("C1", "2", "0", 1e-6),
        ],
        "analysis": {"type": "tran", "stop_s": 5e-3, "step_s": TAU / 10.0},
    })
    solution = transient(circuit, max_steps=20000)
    assert solution.report["final_time_s"] == pytest.approx(5e-3, abs=1e-12)
    assert solution.report["accepted_steps"] < 20000
    values = solution.node_voltages["2"]
    assert all(math.isfinite(float(value)) for value in values)


@pytest.mark.parametrize("field", ["stop_s", "step_s", "max_step_s", "min_step_s"])
def test_transient_rejects_non_finite_analysis_fields(field):
    wrong = float("nan") if field != "step_s" else float("inf")
    with pytest.raises(InputError):
        transient(rc(), **{field: wrong})


def test_transient_rejects_non_positive_max_steps():
    with pytest.raises(InputError):
        transient(rc(), max_steps=0)


def test_diode_temperature_must_be_above_absolute_zero():
    for temperature in (-300.0, -273.15):
        with pytest.raises(InputError):
            load_circuit({
                "version": "circuit/0.1",
                "components": [
                    {"id": "V1", "kind": "vsource", "nodes": ["1", "0"],
                     "value": {"value": 0.6, "unit": "V"}},
                    {"id": "D1", "kind": "diode", "nodes": ["1", "0"],
                     "parameters": {"t_deg_c": temperature}},
                ],
            })


@pytest.mark.parametrize("frequency", [float("nan"), float("inf")])
def test_ac_sweep_rejects_non_finite_frequency(frequency):
    from helqix_circuit import ac_sweep

    circuit = load_circuit({
        "version": "circuit/0.1",
        "components": [
            v("V1", "1", "0", 1.0, {"type": "sine", "amplitude": 1.0, "frequency_hz": 1000.0}),
            r("R1", "1", "2", 1000.0),
            c("C1", "2", "0", 1e-6),
        ],
        "analysis": {"type": "ac", "start_hz": 10.0, "stop_hz": 1e5, "points_per_decade": 2},
    })
    with pytest.raises(InputError):
        ac_sweep(circuit, frequencies=[frequency])
    with pytest.raises(InputError):
        ac_sweep(circuit, frequencies=[1000.0, frequency])


def test_ac_analysis_bounds_must_be_finite():
    from helqix_circuit import ac_sweep

    circuit = load_circuit({
        "version": "circuit/0.1",
        "components": [
            v("V1", "1", "0", 1.0, {"type": "sine", "amplitude": 1.0, "frequency_hz": 1000.0}),
            r("R1", "1", "2", 1000.0),
            c("C1", "2", "0", 1e-6),
        ],
        "analysis": {"type": "ac", "start_hz": float("nan"), "stop_hz": 1e5, "points_per_decade": 2},
    })
    with pytest.raises(InputError):
        ac_sweep(circuit)
