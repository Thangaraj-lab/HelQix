"""E7 live probe: seeded sensor readings through both transports into a consumer.

Every scenario injects one fault and compares the observed counters with the
documented fault model; the combined scenario conserves every reading.
"""

import importlib.util
import json
from pathlib import Path

import pytest

from helqix_io import IOTimeoutError

PROBE_PATH = Path(__file__).resolve().parents[2] / "scripts" / "live_probe.py"


def load_probe():
    spec = importlib.util.spec_from_file_location("helqix_live_probe", PROBE_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


probe = load_probe()
SAMPLES = 120


def voltage_model(**overrides):
    body = {
        "id": "V9",
        "kind": "voltage",
        "unit": "V",
        "range": {"min": -1.0, "max": 1.0},
        "resolution": {"bits": 12},
        "noise_density": 1e-3,
        "bias": 0.0,
        "drift_per_hour": 0.0,
        "bandwidth_hz": 50.0,
        "latency_s": 0.0,
        "sample_rate_hz": 1000.0,
        "saturation": {"mode": "clip"},
        "source": "test",
    }
    body.update(overrides)
    return body


def temperature_model():
    return voltage_model(
        id="T9", kind="temperature", unit="degC",
        range={"min": 0.0, "max": 100.0}, resolution={"bits": 10},
        noise_density=0.0, bandwidth_hz=10.0,
    )


def signal(duration_s=0.12, amplitude=0.5, offset=0.0, frequency_hz=5.0):
    return {
        "type": "sine", "duration_s": duration_s, "amplitude": amplitude,
        "frequency_hz": frequency_hz, "offset": offset, "phase_deg": 0.0,
    }


@pytest.fixture(scope="module")
def probe_report():
    return probe.run_probe(voltage_model(), signal(), seed=11)


def test_probe_delivers_every_reading_cleanly_for_two_sensor_kinds():
    cases = (
        (voltage_model(), signal()),
        (temperature_model(), signal(amplitude=20.0, offset=50.0, frequency_hz=2.0)),
    )
    for model, waveform in cases:
        report = probe.run_probe(model, waveform, seed=3)
        assert report["outcome"] == "ok"
        assert report["sensor"]["samples"] == SAMPLES
        for channel in ("serial", "bus"):
            clean = report["channels"][channel]["clean"]
            assert clean["order"] == list(range(SAMPLES))
            assert clean["unique"] == SAMPLES
            assert clean["stats"]["received"] == SAMPLES
            assert clean["stats"]["dropped"] == 0
            assert clean["stats"]["corrupted"] == 0
            assert clean["stats"]["duplicated"] == 0
            assert clean["stats"]["reordered"] == 0


def test_probe_same_seed_is_identical_and_another_seed_differs():
    first = probe.run_probe(voltage_model(), signal(), seed=11)
    second = probe.run_probe(voltage_model(), signal(), seed=11)
    assert json.dumps(first, sort_keys=True) == json.dumps(second, sort_keys=True)
    other = probe.run_probe(voltage_model(), signal(), seed=29)
    assert other["sensor"]["values_sha"] != first["sensor"]["values_sha"]
    assert other["channels"]["serial"]["drop"]["order"] != first["channels"]["serial"]["drop"]["order"]


def test_probe_drop_matches_an_independent_prediction(probe_report):
    delivered, _, dropped, _ = probe.predict_delivery(SAMPLES, seed=11, drop=0.5)
    for channel in ("serial", "bus"):
        scenario = probe_report["channels"][channel]["drop"]
        assert scenario["order"] == delivered
        assert scenario["delivered"] == len(delivered)
        assert scenario["stats"]["dropped"] == dropped
        assert scenario["delivered"] + dropped == SAMPLES


def test_probe_corrupt_never_delivers_a_faulty_reading(probe_report):
    delivered, _, _, corrupted = probe.predict_delivery(SAMPLES, seed=11, corrupt=0.5)
    for channel in ("serial", "bus"):
        scenario = probe_report["channels"][channel]["corrupt"]
        assert scenario["order"] == delivered
        assert scenario["stats"]["corrupted"] == corrupted
        assert scenario["delivered"] + corrupted == SAMPLES


def test_probe_reorder_puts_the_first_reading_last(probe_report):
    for channel in ("serial", "bus"):
        scenario = probe_report["channels"][channel]["reorder"]
        assert scenario["order"] == list(range(1, SAMPLES)) + [0]
        assert scenario["stats"]["reordered"] == SAMPLES - 1
        assert scenario["out_of_order"] == 1


def test_probe_duplicate_repeats_every_reading_once(probe_report):
    for channel in ("serial", "bus"):
        scenario = probe_report["channels"][channel]["duplicate"]
        assert scenario["order"] == [index for index in range(SAMPLES) for _ in (0, 1)]
        assert scenario["stats"]["duplicated"] == SAMPLES
        assert scenario["stats"]["received"] == 2 * SAMPLES


def test_probe_delay_holds_every_reading_behind_the_timeout(probe_report):
    for channel in ("serial", "bus"):
        scenario = probe_report["channels"][channel]["delay"]
        assert scenario["delay_probe_timed_out"] is True
        assert scenario["order"] == list(range(SAMPLES))
        assert scenario["stats"]["received"] == SAMPLES


def test_probe_combined_scenario_conserves_every_reading(probe_report):
    for channel in ("serial", "bus"):
        scenario = probe_report["channels"][channel]["combined"]
        stats = scenario["stats"]
        assert scenario["unique"] + stats["dropped"] + stats["corrupted"] == SAMPLES
        assert stats["received"] == len(scenario["order"]) == scenario["delivered"]
        assert stats["received"] == scenario["unique"] + scenario["duplicates"]
        assert stats["duplicated"] == scenario["duplicates"]
        assert stats["dropped"] + stats["corrupted"] > 0


def test_probe_consumer_rejects_a_reading_that_differs_from_the_sensor():
    pending = [probe.encode_reading("V9", 0, 0.5)]

    def fetch(timeout_s):
        if pending:
            return pending.pop()
        raise IOTimeoutError("drained")

    with pytest.raises(probe.ProbeError, match="the sensor reported"):
        probe.consume(fetch, [0.25], "V9")
