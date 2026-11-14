"""E4 sensor targets. Every expected value is a hand formula written here."""

import math

import numpy as np
import pytest

from helqix_sensors import InputError, load_model, simulate, synthesize_signal


def model(**overrides):
    body = {
        "id": "S1",
        "kind": "voltage",
        "unit": "V",
        "range": {"min": -1.0, "max": 1.0},
        "resolution": {"bits": 12},
        "noise_density": 1e-3,
        "bias": 0.0,
        "drift_per_hour": 0.0,
        "bandwidth_hz": 100.0,
        "latency_s": 0.0,
        "sample_rate_hz": 10000.0,
        "saturation": {"mode": "clip"},
        "source": "generic",
    }
    body.update(overrides)
    return body


def series(values, fs):
    values = np.asarray(values, dtype=float)
    return {"time_s": np.arange(values.size) / fs, "value": values}


# ---------------------------------------------------------------- noise
def test_noise_sigma_within_three_percent_over_100k_samples():
    fs = 1000.0
    fc = 10.0
    density = 1e-3
    n = 100000
    trace = simulate(
        load_model(model(sample_rate_hz=fs, bandwidth_hz=fc, noise_density=density)),
        series(np.zeros(n), fs),
        seed=7,
    )
    # first-order analog ENBW = (pi/2) * fc; digital ENBW is within 0.01 % here
    predicted = density * math.sqrt((math.pi / 2.0) * fc)
    measured = trace.report["noise_sigma_measured"]
    assert abs(measured - predicted) / predicted <= 0.03
    assert abs(trace.report["noise_sigma_predicted"] - predicted) / predicted <= 1e-3


# ---------------------------------------------------------------- quantization
def test_quantization_step_is_exact():
    fs = 1000.0
    step = 2.0 / 2**12  # (max - min) / 2**bits
    trace = simulate(
        load_model(model(sample_rate_hz=fs, bandwidth_hz=100.0, noise_density=0.0)),
        series(np.full(100, 0.3), fs),
    )
    assert trace.report["quantization_step"] == step
    expected = step * round(0.3 / step)
    assert np.all(np.abs(trace.reported - expected) <= 1e-12)
    # -0.3 ignores the positive bias direction too
    trace_neg = simulate(
        load_model(model(sample_rate_hz=fs, bandwidth_hz=100.0, noise_density=0.0)),
        series(np.full(100, -0.3), fs),
    )
    expected_neg = step * round(-0.3 / step)
    assert np.all(np.abs(trace_neg.reported - expected_neg) <= 1e-12)


def test_explicit_step_resolution_is_used():
    trace = simulate(
        load_model(model(resolution={"step": 0.05}, noise_density=0.0, bandwidth_hz=100.0),),
        series(np.full(50, 0.31), 10000.0),
    )
    assert trace.report["quantization_step"] == 0.05
    assert np.all(np.abs(trace.reported - 0.30) <= 1e-12)


# ---------------------------------------------------------------- bandwidth
@pytest.mark.parametrize("fc", [10.0, 100.0, 1000.0, 4000.0])
def test_minus_three_db_point_within_two_percent(fc):
    trace = simulate(
        load_model(model(sample_rate_hz=10000.0, bandwidth_hz=fc, noise_density=0.0)),
        series(np.zeros(100), 10000.0),
    )
    measured = trace.report["cutoff_measured_hz"]
    assert abs(measured - fc) / fc <= 0.02


# ---------------------------------------------------------------- saturation
def test_saturation_is_exact_and_flagged():
    fs = 1000.0
    trace = simulate(
        load_model(model(sample_rate_hz=fs, bandwidth_hz=100.0, noise_density=0.0)),
        series(np.full(200, 5.0), fs),
    )
    assert np.all(trace.reported == 1.0)
    assert trace.report["clipped_samples"] == 200
    assert np.all(trace.clipped)

    low = simulate(
        load_model(model(sample_rate_hz=fs, bandwidth_hz=100.0, noise_density=0.0)),
        series(np.full(200, -5.0), fs),
    )
    assert np.all(low.reported == -1.0)
    assert low.report["clipped_samples"] == 200


# ---------------------------------------------------------------- determinism
def test_same_seed_gives_identical_bytes():
    spec = load_model(model())
    signal = series(np.zeros(2000), 10000.0)
    first = simulate(spec, signal, seed=42)
    second = simulate(spec, signal, seed=42)
    assert first.reported.tobytes() == second.reported.tobytes()
    assert first.analog.tobytes() == second.analog.tobytes()
    assert first.report["reality_gap_rms"] == second.report["reality_gap_rms"]
    other = simulate(spec, signal, seed=43)
    assert other.analog.tobytes() != first.analog.tobytes()


# ---------------------------------------------------------------- reality gap
def test_reality_gap_reports_noise_dominating_quantization():
    fs = 10000.0
    fc = 100.0
    density = 5e-5
    step = 2.0 / 2**12
    trace = simulate(
        load_model(model(sample_rate_hz=fs, bandwidth_hz=fc, noise_density=density)),
        series(np.full(200000, 0.25), fs),
        seed=11,
    )
    sigma = density * math.sqrt((math.pi / 2.0) * fc)
    quant_rms = step / math.sqrt(12.0)
    expected_gap = math.sqrt(sigma * sigma + quant_rms * quant_rms)
    gap = trace.report["reality_gap_rms"]
    assert abs(gap - expected_gap) / expected_gap <= 0.05
    assert abs(trace.report["quantization_error_rms"] - quant_rms) / quant_rms <= 0.05
    assert trace.report["noise_error_rms"] > 2.0 * trace.report["quantization_error_rms"]


# ---------------------------------------------------------------- bias / drift
def test_bias_and_drift_are_applied():
    trace = simulate(
        load_model(
            model(
                bias=0.1,
                drift_per_hour=0.2,
                noise_density=0.0,
                bandwidth_hz=4.5,
                sample_rate_hz=10.0,
                resolution={"bits": 24},
                range={"min": -2.0, "max": 2.0},
            )
        ),
        series(np.zeros(36001), 10.0),
    )
    assert abs(trace.reported[0] - 0.1) <= 1e-6
    # a drifting signal is delayed by the first-order filter by a fraction of
    # a sample, so 1e-5 covers the honest lag and the 24-bit step
    assert abs(trace.reported[-1] - 0.3) <= 1e-5


# ---------------------------------------------------------------- latency
def test_latency_shifts_the_reading_by_whole_samples():
    fs = 1000.0
    values = np.concatenate((np.zeros(500), np.ones(500)))

    def edge(latency_s):
        trace = simulate(
            load_model(
                model(
                    sample_rate_hz=fs,
                    bandwidth_hz=400.0,
                    noise_density=0.0,
                    latency_s=latency_s,
                    resolution={"bits": 16},
                    range={"min": -2.0, "max": 2.0},
                )
            ),
            series(values, fs),
        )
        return int(np.argmax(trace.reported > 0.5))

    assert edge(0.15) - edge(0.05) == 100


# ---------------------------------------------------------------- probes input
def probe_document():
    times = np.arange(0.0, 0.101, 0.001)
    return {
        "status": "this key must not be read",
        "analysis": {"nonsense": [1, 2, 3]},
        "probes": [
            {"id": "out", "unit": "V", "time_s": times, "value": times * 2.0},
            {"id": "other", "unit": "V", "time_s": times, "value": np.zeros_like(times)},
        ],
    }


def test_probes_from_results_json_are_read_and_resampled():
    spec = load_model(
        model(sample_rate_hz=10000.0, bandwidth_hz=4000.0, noise_density=0.0, resolution={"bits": 16})
    )
    trace = simulate(spec, probe_document(), probe="out")
    assert trace.truth[0] == pytest.approx(0.0, abs=1e-15)
    assert trace.truth[100] == pytest.approx(0.02, abs=1e-12)
    assert trace.truth[-1] == pytest.approx(0.2, abs=1e-12)
    assert trace.report["sample_count"] == 1001  # 0.1 s at 10 kHz


def test_probe_must_be_chosen_when_several_exist():
    spec = load_model(model())
    with pytest.raises(InputError):
        simulate(spec, probe_document())
    trace = simulate(spec, probe_document(), probe=1)
    assert np.allclose(trace.truth, 0.0)


def test_document_without_probes_is_refused():
    with pytest.raises(InputError):
        simulate(load_model(model()), {"status": "ok", "values": []})


def test_probe_unit_must_match_the_model():
    document = probe_document()
    document["probes"][0]["unit"] = "degC"  # the model reports volts
    with pytest.raises(InputError):
        simulate(load_model(model()), document, probe="out")


def test_absurd_sample_count_is_refused():
    document = {"probes": [{"id": "out", "time_s": [0.0, 1e7], "value": [0.0, 1.0]}]}
    with pytest.raises(InputError):
        simulate(load_model(model()), document)


def test_negative_time_is_refused():
    with pytest.raises(InputError):
        simulate(load_model(model()), {"time_s": [-0.1, 0.0, 0.1], "value": [0.0, 0.0, 0.0]})


# ---------------------------------------------------------------- signals
def test_signal_generators_match_hand_formulas():
    fs = 1000.0
    dc = synthesize_signal({"type": "dc", "value": 1.5, "duration_s": 0.01}, fs)
    assert np.allclose(dc["value"], 1.5)
    assert dc["time_s"][-1] == pytest.approx(0.009, abs=1e-12)

    sine = synthesize_signal(
        {"type": "sine", "amplitude": 2.0, "frequency_hz": 100.0, "offset": 0.5, "duration_s": 0.01},
        fs,
    )
    t = sine["time_s"]
    assert np.allclose(sine["value"], 0.5 + 2.0 * np.sin(2.0 * math.pi * 100.0 * t))

    step = synthesize_signal(
        {"type": "step", "amplitude": 3.0, "delay_s": 0.002, "duration_s": 0.01},
        fs,
    )
    assert step["value"][1] == 0.0
    assert step["value"][2] == 3.0


# ---------------------------------------------------------------- validation
@pytest.mark.parametrize(
    "overrides",
    [
        {"kind": "magnetism"},
        {"unit": ""},
        {"range": {"min": 1.0, "max": -1.0}},
        {"resolution": {"bits": 0}},
        {"resolution": {"step": -0.1}},
        {"resolution": {"bits": 12, "step": 0.1}},
        {"resolution": {}},
        {"noise_density": -1.0},
        {"bandwidth_hz": 0.0},
        {"bandwidth_hz": 5000.0},
        {"latency_s": -0.1},
        {"sample_rate_hz": 0.0},
        {"saturation": {"mode": "fold"}},
        {"id": ""},
    ],
)
def test_bad_models_are_refused(overrides):
    with pytest.raises(InputError):
        load_model(model(**overrides))


def test_bad_signal_series_are_refused():
    spec = load_model(model())
    with pytest.raises(InputError):
        simulate(spec, {"time_s": [0.0, 0.0, 0.1], "value": [1.0, 2.0, 3.0]})
    with pytest.raises(InputError):
        simulate(spec, {"time_s": [0.0, 0.1], "value": [1.0, float("nan")]})
    with pytest.raises(InputError):
        simulate(spec, {"time_s": [0.0, 0.1], "value": [1.0]})
    with pytest.raises(InputError):
        simulate(spec, {"time_s": [], "value": []})
