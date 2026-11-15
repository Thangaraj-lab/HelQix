"""E5 interface tests: python -m helqix_sensors run CASE --out DIR."""

import contextlib
import io
import json
import subprocess
import sys
from pathlib import Path
from types import SimpleNamespace

import pytest

from helqix_sensors import cli

REPO = Path(__file__).resolve().parents[2]
EXAMPLES = REPO / "examples" / "sensors"


def model(**over):
    body = {
        "id": "s1", "kind": "voltage", "unit": "V",
        "range": {"min": -5.0, "max": 5.0},
        "resolution": {"bits": 20},
        "noise_density": 0.0, "bias": 0.0, "drift_per_hour": 0.0,
        "bandwidth_hz": 100.0, "latency_s": 0.0, "sample_rate_hz": 1000.0,
        "saturation": {"mode": "clip"}, "source": "generic",
    }
    body.update(over)
    return body


def case(**extra):
    body = {
        "version": "case/0.1",
        "model": model(),
        "signal": {"synthesize": {"type": "dc", "value": 0.25, "duration_s": 0.1}},
    }
    body.update(extra)
    return body


def write_case(tmp_path, body) -> Path:
    case_path = tmp_path / "case.json"
    case_path.write_text(json.dumps(body, indent=2), encoding="utf-8")
    return case_path


def run(tmp_path, body, out_name="out"):
    case_path = write_case(tmp_path, body)
    out = tmp_path / out_name
    buffer = io.StringIO()
    with contextlib.redirect_stderr(buffer), contextlib.redirect_stdout(io.StringIO()):
        code = cli.main(["run", str(case_path), "--out", str(out)])
    return SimpleNamespace(returncode=code, stderr=buffer.getvalue(), stdout=""), out


def run_module(tmp_path, body, out_name="out"):
    case_path = write_case(tmp_path, body)
    out = tmp_path / out_name
    proc = subprocess.run(
        [sys.executable, "-m", "helqix_sensors", "run", str(case_path), "--out", str(out)],
        cwd=str(REPO),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=150,
    )
    return proc, out


def results(out):
    return json.loads((out / "results.json").read_text(encoding="utf-8"))


def value(payload, name):
    for entry in payload["values"]:
        if entry["name"] == name:
            return entry
    raise AssertionError(f"{name} not in values: {[e['name'] for e in payload['values']]}")


def test_sensor_case_writes_the_contract_folder(tmp_path):
    proc, out = run_module(tmp_path, case(seed=7))
    assert proc.returncode == 0, proc.stderr
    payload = results(out)
    assert payload["status"] == "ok"
    assert payload["analysis"] == "sensor"
    assert payload["validation_label"] == "CHECKED"
    assert payload["solver"] == {"name": "helqix_sensors", "version": "0.1.0"}
    assert payload["model"]["id"] == "s1" and payload["model"]["unit"] == "V"
    gap = value(payload, "reality_gap_rms")
    assert gap["unit"] == "V" and gap["value"] >= 0.0
    assert value(payload, "cutoff_measured_hz")["unit"] == "Hz"
    time = value(payload, "sample_count")
    assert time["unit"] == "1" and time["value"] == 100
    ids = [p["id"] for p in payload["probes"]]
    assert ids == ["truth", "analog", "reported", "clipped"]
    assert all(len(p["x"]) <= 2000 for p in payload["probes"])
    rows = (out / "series.csv").read_text(encoding="utf-8").splitlines()
    assert rows[0] == "time_s,truth,analog,reported,clipped"
    assert len(rows) - 1 == 100
    assert sorted(p.name for p in tmp_path.iterdir()) == ["case.json", "out"]


def test_reference_can_validate_a_case(tmp_path):
    # span 4 with 20 bits gives a step of 3.8e-6 V, so 0.25 V is exactly
    # representable and the quiet model reports the truth bit for bit
    quiet = model(
        noise_density=0.0, bias=0.0, drift_per_hour=0.0, latency_s=0.0,
        range={"min": -2.0, "max": 2.0},
    )
    body = case(model=quiet, seed=1, reference={
        "quantity": "reported_mean", "name": "reported", "value": 0.25, "relative_tolerance": 1e-9,
    })
    proc, out = run(tmp_path, body)
    assert proc.returncode == 0, proc.stderr
    payload = results(out)
    assert payload["validation_label"] == "VALIDATED"
    assert payload["reference"]["matched"] is True


def test_mismatched_reference_is_unvalidated(tmp_path):
    body = case(reference={
        "quantity": "reported_mean", "name": "reported", "value": 10.0, "relative_tolerance": 1e-6,
    })
    proc, out = run(tmp_path, body)
    assert proc.returncode == 0
    payload = results(out)
    assert payload["validation_label"] == "UNVALIDATED"
    assert payload["reference"]["matched"] is False


def test_probe_document_input_is_read(tmp_path):
    probes = {
        "status": "ok", "analysis": "dc", "values": [],
        "probes": [
            {"id": "x", "unit": "V", "time_s": [0.0, 0.01, 0.02], "value": [1.0, 2.0, 3.0]},
            {"id": "z", "unit": "V", "time_s": [0.0, 0.01, 0.02], "value": [0.0, 0.5, 1.0]},
        ],
    }
    proc, out = run(tmp_path, case(signal=probes, probe="z"))
    assert proc.returncode == 0, proc.stderr
    payload = results(out)
    assert payload["probe"] == "z"
    truth = [p for p in payload["probes"] if p["id"] == "truth"][0]
    assert truth["value"][0] == 0.0 and abs(truth["value"][-1] - 1.0) <= 1e-12


def test_probe_unit_mismatch_exits_2(tmp_path):
    probes = {"probes": [{"id": "p", "unit": "degC", "time_s": [0.0, 0.1], "value": [0.0, 1.0]}]}
    proc, _ = run(tmp_path, case(signal=probes))
    assert proc.returncode == 2
    assert "degC" in proc.stderr and "V" in proc.stderr
    assert "Traceback" not in proc.stderr


def test_bad_input_exits_2(tmp_path):
    proc, _ = run(tmp_path, case(model=model(kind="magnetism")))
    assert proc.returncode == 2 and "kind" in proc.stderr
    body = case()
    del body["signal"]
    proc, _ = run(tmp_path, body, out_name="out2")
    assert proc.returncode == 2 and "signal" in proc.stderr
    proc, _ = run(tmp_path, case(version="case/9.9"), out_name="out3")
    assert proc.returncode == 2 and "version" in proc.stderr


def test_unknown_case_key_is_refused(tmp_path):
    body = case()
    body["seeed"] = 3
    proc, _ = run(tmp_path, body)
    assert proc.returncode == 2
    assert "seeed" in proc.stderr


def test_unknown_model_field_is_refused(tmp_path):
    bad = model()
    bad["noise_densiy"] = 0.01
    proc, _ = run(tmp_path, case(model=bad))
    assert proc.returncode == 2
    assert "noise_densiy" in proc.stderr


def test_missing_optional_model_fields_use_documented_defaults(tmp_path):
    keep = {"id", "kind", "unit", "range", "resolution", "bandwidth_hz", "sample_rate_hz"}
    lean = {key: value for key, value in model().items() if key in keep}
    proc, out = run(tmp_path, case(model=lean))
    assert proc.returncode == 0, proc.stderr
    payload = results(out)
    assert payload["model"]["source"] == "generic"
    assert value(payload, "noise_sigma_predicted")["value"] == 0.0


def test_sensor_limits_accept_only_timeout(tmp_path):
    proc, _ = run(tmp_path, case(limits={"max_steps": 10}))
    assert proc.returncode == 2
    assert "timeout_s" in proc.stderr


def test_timeout_is_enforced(tmp_path):
    model_body = model(sample_rate_hz=500000.0, bandwidth_hz=100000.0)
    signal = {"synthesize": {"type": "dc", "value": 0.5, "duration_s": 1.0}}
    proc, out = run(tmp_path, case(model=model_body, signal=signal, limits={"timeout_s": 0.001}))
    assert proc.returncode == 3
    assert "timeout" in results(out)["message"].lower()


def test_same_seed_gives_identical_results_across_runs(tmp_path):
    noisy = model(noise_density=0.01)
    proc, out1 = run(tmp_path, case(model=noisy, seed=11), out_name="a")
    assert proc.returncode == 0, proc.stderr
    proc, out2 = run(tmp_path, case(model=noisy, seed=11), out_name="b")
    assert proc.returncode == 0, proc.stderr
    reported1 = [p for p in results(out1)["probes"] if p["id"] == "reported"][0]["value"]
    reported2 = [p for p in results(out2)["probes"] if p["id"] == "reported"][0]["value"]
    assert reported1 == reported2


def test_out_folder_must_be_fresh(tmp_path):
    out = tmp_path / "out"
    out.mkdir()
    (out / "keep.txt").write_text("x", encoding="utf-8")
    proc, _ = run(tmp_path, case())
    assert proc.returncode == 2
    assert "fresh" in proc.stderr.lower()
    assert (out / "keep.txt").is_file()


# ---------------------------------------------------------------- examples
def example_paths():
    return sorted(EXAMPLES.glob("*.json"))


def test_there_are_ten_sensor_examples():
    assert len(example_paths()) == 10


@pytest.mark.parametrize("path", example_paths(), ids=lambda p: p.stem)
def test_every_sensor_example_runs(tmp_path, path):
    body = json.loads(path.read_text(encoding="utf-8"))
    snapshot = json.dumps(body, sort_keys=True)
    proc, out = run(tmp_path, body)
    assert proc.returncode == 0, proc.stderr
    payload = results(out)
    assert payload["status"] == "ok"
    assert payload["validation_label"] in ("VALIDATED", "CHECKED", "UNVALIDATED")
    for probe in payload["probes"]:
        assert len(probe["x"]) <= 2000
    json.dumps(payload, allow_nan=False)
    assert json.dumps(json.loads(path.read_text(encoding="utf-8")), sort_keys=True) == snapshot
