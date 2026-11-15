"""E5 interface tests: python -m helqix_circuit run CASE --out DIR."""

import contextlib
import copy
import io
import json
import subprocess
import sys
from pathlib import Path
from types import SimpleNamespace

import pytest

from helqix_circuit import cli

REPO = Path(__file__).resolve().parents[2]
EXAMPLES = REPO / "examples" / "circuit"


def r(cid, a, b, ohms):
    return {"id": cid, "kind": "resistor", "nodes": [a, b], "value": {"value": ohms, "unit": "ohm"}}


def v(cid, a, b, volts):
    return {"id": cid, "kind": "vsource", "nodes": [a, b], "value": {"value": volts, "unit": "V"}}


def c(cid, a, b, farads):
    return {"id": cid, "kind": "capacitor", "nodes": [a, b], "value": {"value": farads, "unit": "F"}}


def divider():
    return {
        "version": "circuit/0.1",
        "components": [v("V1", "1", "0", 10.0), r("R1", "1", "2", 1000.0), r("R2", "2", "0", 2000.0)],
        "analysis": {"type": "dc"},
    }


def rc_transient(**analysis):
    body = {"type": "tran", "stop_s": 0.01, "step_s": 1e-6, "method": "trapezoidal", "adaptive": False}
    body.update(analysis)
    return {
        "version": "circuit/0.1",
        "components": [v("V1", "1", "0", 5.0), r("R1", "1", "2", 1000.0), c("C1", "2", "0", 1e-6)],
        "analysis": body,
    }


def case(circuit, **extra):
    body = {"version": "case/0.1", "circuit": circuit}
    body.update(extra)
    return body


def write_case(tmp_path, body) -> Path:
    case_path = tmp_path / "case.json"
    if isinstance(body, str):
        case_path.write_text(body, encoding="utf-8")
    else:
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
        [sys.executable, "-m", "helqix_circuit", "run", str(case_path), "--out", str(out)],
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


# ---------------------------------------------------------------- result shape
def test_divider_case_writes_the_contract_folder(tmp_path):
    body = case(divider(), reference={
        "quantity": "node_voltage", "name": "V(2)", "value": 20.0 / 3.0, "relative_tolerance": 1e-9,
    })
    proc, out = run_module(tmp_path, body)
    assert proc.returncode == 0, proc.stderr
    payload = results(out)
    assert payload["status"] == "ok"
    assert payload["analysis"] == "dc"
    assert payload["validation_label"] == "VALIDATED"
    assert payload["solver"] == {"name": "helqix_circuit", "version": "0.1.0"}
    assert isinstance(payload["seconds"], float) and payload["seconds"] >= 0.0
    entry = value(payload, "V(2)")
    assert entry["quantity"] == "node_voltage" and entry["unit"] == "V"
    assert abs(entry["value"] - 20.0 / 3.0) <= 1e-9
    assert payload["reference"]["matched"] is True
    assert (out / "values.csv").is_file()
    first_line = (out / "values.csv").read_text(encoding="utf-8").splitlines()[0]
    assert first_line == "name,quantity,value,unit,label"
    assert not (out / "series.csv").exists()
    # nothing outside the output folder was written
    assert sorted(p.name for p in tmp_path.iterdir()) == ["case.json", "out"]
    assert sorted(p.name for p in out.iterdir()) == ["results.json", "values.csv"]


def test_solver_failure_exits_3_and_reports_status(tmp_path):
    # E1 fixes node 2 at 2*V1 = 10 V while V2 fixes it at 3 V: no unique solution
    singular = {
        "version": "circuit/0.1",
        "components": [
            v("V1", "1", "0", 5.0),
            {"id": "E1", "kind": "vcvs", "nodes": ["2", "0", "1", "0"], "value": {"value": 2.0, "unit": "1"}},
            v("V2", "2", "0", 3.0),
        ],
        "analysis": {"type": "dc"},
    }
    proc, out = run(tmp_path, case(singular))
    assert proc.returncode == 3, proc.stderr
    payload = results(out)
    assert payload["status"] == "solver_error"
    assert "singular" in payload["message"].lower()
    assert isinstance(payload["seconds"], float)


def test_bad_input_exits_2_with_a_plain_message(tmp_path):
    proc, out = run(tmp_path, {"version": "case/0.1"})
    assert proc.returncode == 2
    assert "circuit" in proc.stderr.lower()
    assert "Traceback" not in proc.stderr
    assert not (out / "results.json").exists()
    proc, out = run(tmp_path, "{not json", out_name="out2")
    assert proc.returncode == 2 and "json" in proc.stderr.lower()
    proc, out = run(tmp_path, case(divider(), version="case/9.9"), out_name="out3")
    assert proc.returncode == 2 and "version" in proc.stderr.lower()


def test_unknown_case_key_is_refused(tmp_path):
    body = case(divider())
    body["limts"] = {"timeout_s": 1}
    proc, _ = run(tmp_path, body)
    assert proc.returncode == 2
    assert "limts" in proc.stderr


def test_unknown_fields_inside_the_circuit_are_refused(tmp_path):
    body = case(divider())
    body["circuit"]["analysys"] = {"type": "tran"}
    proc, _ = run(tmp_path, body)
    assert proc.returncode == 2 and "analysys" in proc.stderr
    body = case(divider())
    body["circuit"]["components"][0]["wafeform"] = {"type": "dc"}
    proc, _ = run(tmp_path, body, out_name="out2")
    assert proc.returncode == 2 and "wafeform" in proc.stderr
    body = case(divider())
    body["circuit"]["analysis"]["stopp_s"] = 1.0
    proc, _ = run(tmp_path, body, out_name="out3")
    assert proc.returncode == 2 and "stopp_s" in proc.stderr


def test_missing_analysis_defaults_to_dc(tmp_path):
    body = case(divider())
    del body["circuit"]["analysis"]
    proc, out = run(tmp_path, body)
    assert proc.returncode == 0, proc.stderr
    assert results(out)["analysis"] == "dc"


def test_limits_max_nodes_is_enforced(tmp_path):
    proc, out = run(tmp_path, case(divider(), limits={"max_nodes": 1}))
    assert proc.returncode == 2
    assert "max_nodes" in proc.stderr


def test_limits_max_steps_is_enforced(tmp_path):
    body = case(rc_transient(), limits={"max_steps": 3})
    proc, out = run(tmp_path, body)
    assert proc.returncode == 3
    assert "max_steps" in results(out)["message"]


def test_timeout_is_enforced(tmp_path):
    # max_steps is small so a build with the deadline check disabled fails in
    # milliseconds instead of running until the test harness kills it.
    body = case(rc_transient(stop_s=0.05), limits={"timeout_s": 0.0005, "max_steps": 2000})
    proc, out = run(tmp_path, body)
    assert proc.returncode == 3
    assert "timeout" in results(out)["message"].lower()


def test_probes_are_downsampled_to_2000_points(tmp_path):
    proc, out = run(tmp_path, case(rc_transient()))
    assert proc.returncode == 0, proc.stderr
    payload = results(out)
    assert payload["probes"], "transient results must carry probes"
    for probe in payload["probes"]:
        assert 2 <= len(probe["time_s"]) <= 2000
        assert probe["samples_total"] == 10001
        assert probe["time_s"][0] == 0.0 and abs(probe["time_s"][-1] - 0.01) <= 1e-12
        assert all(b > a for a, b in zip(probe["time_s"], probe["time_s"][1:]))
    voltage = [p for p in payload["probes"] if p["id"] == "V(2)"][0]
    assert voltage["unit"] == "V" and voltage["x_name"] == "time_s"
    rows = (out / "series.csv").read_text(encoding="utf-8").splitlines()
    assert len(rows) - 1 == len(voltage["time_s"])
    assert rows[0].split(",")[0] == "time_s"


def test_ac_case_reports_cutoff_and_sweep(tmp_path):
    netlist = {
        "version": "circuit/0.1",
        "components": [v("V1", "1", "0", 1.0), r("R1", "1", "2", 1000.0), c("C1", "2", "0", 100e-9)],
        "analysis": {
            "type": "ac", "start_hz": 1.0, "stop_hz": 1e6, "points_per_decade": 50,
            "cutoff": {"source": "V1", "node": "2"},
        },
    }
    proc, out = run(tmp_path, case(netlist))
    assert proc.returncode == 0, proc.stderr
    payload = results(out)
    assert payload["analysis"] == "ac"
    expected = 1.0 / (2.0 * 3.141592653589793 * 1000.0 * 100e-9)
    cutoff = value(payload, "cutoff_hz")
    assert cutoff["unit"] == "Hz"
    assert abs(cutoff["value"] - expected) / expected <= 0.01
    assert all(p["x_name"] == "frequency_hz" for p in payload["probes"])
    assert payload["series_csv"] == "series.csv"


def test_wire_verdicts_are_in_the_result(tmp_path):
    wire = {
        "id": "W1", "from": "1", "to": "2", "length_mm": 2000.0, "diameter_mm": 1.784,
        "material": "copper", "insulation_rating_c": 70.0, "installation_method": "C",
    }
    proc, out = run(tmp_path, case(divider(), wires=[wire]))
    assert proc.returncode == 0, proc.stderr
    payload = results(out)
    assert payload["wire_verdict"] == "PASS"
    assert [w["id"] for w in payload["wires"]] == ["W1"]
    assert payload["wires"][0]["verdict"] == "PASS"


def test_wires_with_a_non_dc_analysis_are_refused(tmp_path):
    wire = {
        "id": "W1", "from": "1", "to": "2", "length_mm": 2000.0, "gauge_awg": 20,
        "material": "copper", "insulation_rating_c": 70.0,
    }
    proc, _ = run(tmp_path, case(rc_transient(), wires=[wire]))
    assert proc.returncode == 2
    assert "dc" in proc.stderr.lower()


def test_empty_circuit_is_refused(tmp_path):
    body = {"version": "case/0.1", "circuit": {"version": "circuit/0.1", "components": []}}
    proc, out = run(tmp_path, body)
    assert proc.returncode == 2
    assert "no components" in proc.stderr
    assert not out.exists()


def test_transient_step_count_cap_is_refused(tmp_path):
    body = case(rc_transient(stop_s=10.0, step_s=1e-7), limits={"timeout_s": 0.2})  # 100,000,000 steps
    proc, out = run(tmp_path, body)
    assert proc.returncode == 2
    assert "2,000,000" in proc.stderr and "cap" in proc.stderr
    assert not out.exists()


def test_wire_insulation_rating_is_required(tmp_path):
    wire = {
        "id": "W1", "from": "1", "to": "2", "length_mm": 2000.0, "diameter_mm": 1.784,
        "material": "copper", "installation_method": "C",
    }
    proc, out = run(tmp_path, case(divider(), wires=[wire]))
    assert proc.returncode == 2
    assert "insulation_rating_c" in proc.stderr
    assert not out.exists() or not any(out.iterdir())


def test_mismatched_reference_is_unvalidated_but_still_a_result(tmp_path):
    body = case(divider(), reference={
        "quantity": "node_voltage", "name": "V(2)", "value": 5.0, "relative_tolerance": 1e-9,
    })
    proc, out = run(tmp_path, body)
    assert proc.returncode == 0
    payload = results(out)
    assert payload["validation_label"] == "UNVALIDATED"
    assert payload["reference"]["matched"] is False
    assert payload["reference"]["relative_error"] > 0.3


def test_out_folder_must_be_fresh(tmp_path):
    out = tmp_path / "out"
    out.mkdir()
    (out / "my_notes.txt").write_text("keep me", encoding="utf-8")
    proc, _ = run(tmp_path, case(divider()))
    assert proc.returncode == 2
    assert "fresh" in proc.stderr.lower()
    assert (out / "my_notes.txt").read_text(encoding="utf-8") == "keep me"


def test_rerun_into_our_own_outputs_is_allowed(tmp_path):
    proc, out = run(tmp_path, case(divider()))
    assert proc.returncode == 0
    proc, out = run(tmp_path, case(divider()))
    assert proc.returncode == 0, proc.stderr
    assert results(out)["status"] == "ok"


def test_case_file_is_not_modified(tmp_path):
    body = case(divider())
    proc, _ = run(tmp_path, body)
    assert proc.returncode == 0
    assert json.loads((tmp_path / "case.json").read_text(encoding="utf-8")) == body


# ---------------------------------------------------------------- examples
def example_paths():
    return sorted(EXAMPLES.glob("*.json"))


def test_there_are_ten_circuit_examples():
    assert len(example_paths()) == 10


@pytest.mark.parametrize("path", example_paths(), ids=lambda p: p.stem)
def test_every_circuit_example_runs(tmp_path, path):
    body = json.loads(path.read_text(encoding="utf-8"))
    snapshot = copy.deepcopy(body)
    proc, out = run(tmp_path, body)
    assert proc.returncode == 0, proc.stderr
    payload = results(out)
    assert payload["status"] == "ok"
    assert payload["validation_label"] in ("VALIDATED", "CHECKED", "UNVALIDATED")
    for probe in payload["probes"]:
        assert len(probe[probe["x_name"]]) <= 2000
    json.dumps(payload, allow_nan=False)
    assert json.loads(path.read_text(encoding="utf-8")) == snapshot
