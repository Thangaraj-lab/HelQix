"""Random-seeded sensor properties, run in the nightly verification only."""

from __future__ import annotations

import random

import numpy as np
import pytest

from helqix_sensors import simulate, synthesize_signal

SEEDS = list(range(20))


@pytest.mark.nightly
@pytest.mark.parametrize("seed", SEEDS, ids=[f"seed{s}" for s in SEEDS])
def test_random_sensor_model_keeps_its_promises(seed):
    rng = random.Random(seed)
    fs = 1000.0
    low, high = -rng.uniform(1.0, 10.0), rng.uniform(1.0, 10.0)
    bits = rng.randint(8, 16)
    span = high - low
    model = {
        "id": f"p{seed}", "kind": "voltage", "unit": "V", "range": {"min": low, "max": high},
        "resolution": {"bits": bits},
        "noise_density": rng.uniform(0.0, 0.01), "bias": rng.uniform(-0.1, 0.1),
        "drift_per_hour": rng.uniform(-1.0, 1.0),
        "bandwidth_hz": rng.uniform(10.0, 300.0), "latency_s": rng.randint(0, 5) / fs,
        "sample_rate_hz": fs, "saturation": {"mode": "clip"}, "source": "generic",
    }
    spec = {
        "type": "sine", "amplitude": rng.uniform(0.2, 0.9) * span / 2.0,
        "frequency_hz": rng.uniform(1.0, 20.0), "offset": rng.uniform(-0.1, 0.1) * span,
        "phase_deg": rng.uniform(0.0, 360.0), "duration_s": 0.5,
    }
    signal = synthesize_signal(spec, fs)
    trace = simulate(model, signal, seed=seed)
    assert trace.time_s.size == trace.truth.size == trace.analog.size == trace.reported.size
    assert trace.reported.size == 500
    assert np.all(np.isfinite(trace.reported))
    assert trace.reported.min() >= low - 1e-12 and trace.reported.max() <= high + 1e-12

    step = span / 2**bits
    clipped = np.asarray(trace.clipped, dtype=bool)
    assert np.array_equal(clipped, (trace.analog < low) | (trace.analog > high))
    grid = trace.reported[~clipped] / step
    assert np.all(np.abs(grid - np.round(grid)) <= 1e-9)

    again = simulate(model, signal, seed=seed)
    assert np.array_equal(trace.reported, again.reported)

    gap = float(np.sqrt(np.mean((trace.reported - trace.truth) ** 2)))
    assert trace.report["reality_gap_rms"] == pytest.approx(gap, rel=1e-12, abs=1e-15)
    assert trace.report["sample_count"] == trace.reported.size
