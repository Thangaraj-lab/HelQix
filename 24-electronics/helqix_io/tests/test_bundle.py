"""electronics-result/0.1 bundles: copied payload, hashes, labels, determinism."""

import hashlib
import json
from pathlib import Path

import pytest

from helqix_io import BundleError, build_bundle


def results_ok():
    return {
        "status": "ok",
        "analysis": "dc",
        "validation_label": "CHECKED",
        "solver": {"name": "helqix_circuit", "version": "0.1.0"},
        "seconds": 0.001,
        "limits": {"timeout_s": 30.0, "max_nodes": 10000, "max_steps": 200000},
        "values": [
            {"name": "V(2)", "quantity": "node_voltage", "value": 6.666666666666667, "unit": "V", "label": "CALCULATED"},
            {"name": "I(V1)", "quantity": "branch_current", "value": -0.0033333333333333335, "unit": "A", "label": "CALCULATED"},
        ],
        "probes": [],
        "series_csv": None,
        "wires": [
            {
                "id": "W1",
                "drop_v": {"value": 0.252330, "unit": "V", "label": "CALCULATED"},
                "ampacity_a": {"value": None, "unit": "A", "label": "UNKNOWN"},
                "verdict": "PASS",
            }
        ],
    }


def source_dir(tmp_path, body=None, *, series=False, name="source"):
    src = tmp_path / name
    src.mkdir()
    body = results_ok() if body is None else body
    if isinstance(body, str):
        (src / "results.json").write_text(body, encoding="utf-8")
        return src
    if series:
        body = dict(body, probes=[{"id": "V(2)", "unit": "V", "x_name": "time_s", "time_s": [0.0, 1.0], "value": [0.0, 1.0], "label": "CALCULATED"}], series_csv="series.csv")
        (src / "series.csv").write_text("time_s,V(2)\n0.0,0.0\n1.0,1.0\n", encoding="utf-8")
    (src / "results.json").write_text(json.dumps(body, indent=2), encoding="utf-8")
    if body["status"] == "ok":
        (src / "values.csv").write_text(
            "name,quantity,value,unit,label\nV(2),node_voltage,6.666666666666667,V,CALCULATED\n",
            encoding="utf-8",
        )
    return src


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def test_bundle_copies_the_payload_and_records_hashes(tmp_path):
    src = source_dir(tmp_path)
    out = tmp_path / "bundle"
    manifest = build_bundle(src, out)
    assert manifest["version"] == "electronics-result/0.1"
    assert manifest["status"] == "ok"
    assert manifest["analysis"] == "dc"
    assert manifest["validation_label"] == "CHECKED"
    assert manifest["source"] == {"package": "helqix_circuit", "version": "0.1.0"}
    names = [entry["name"] for entry in manifest["files"]]
    assert names == ["results.json", "values.csv"]
    for entry in manifest["files"]:
        path = out / entry["name"]
        assert entry["bytes"] == path.stat().st_size
        assert entry["sha256"] == digest(path)
    assert (out / "results.json").read_bytes() == (src / "results.json").read_bytes()
    assert sorted(p.name for p in out.iterdir()) == ["README.md", "manifest.json", "results.json", "values.csv"]
    assert manifest["units"] == ["A", "V"]
    assert manifest["labels"] == ["CALCULATED", "UNKNOWN"]
    readme = (out / "README.md").read_text(encoding="utf-8")
    assert readme.startswith("# electronics-result/0.1 bundle")
    assert "helqix_circuit 0.1.0" in readme
    assert "CALCULATED" in readme and "UNKNOWN" in readme


def test_series_is_copied_when_the_result_names_it(tmp_path):
    src = source_dir(tmp_path, series=True)
    manifest = build_bundle(src, tmp_path / "bundle")
    assert [entry["name"] for entry in manifest["files"]] == ["results.json", "series.csv", "values.csv"]
    assert (tmp_path / "bundle" / "series.csv").is_file()


def test_same_input_gives_byte_identical_bundles(tmp_path):
    src = source_dir(tmp_path)
    build_bundle(src, tmp_path / "one")
    build_bundle(src, tmp_path / "two")
    for name in ("manifest.json", "README.md", "results.json", "values.csv"):
        assert (tmp_path / "one" / name).read_bytes() == (tmp_path / "two" / name).read_bytes()
    build_bundle(src, tmp_path / "one")
    for name in ("manifest.json", "README.md", "results.json", "values.csv"):
        assert (tmp_path / "one" / name).read_bytes() == (tmp_path / "two" / name).read_bytes()


def test_solver_error_bundle_has_no_values_table(tmp_path):
    body = {
        "status": "solver_error",
        "message": "floating node(s) 'c': no DC path to ground",
        "code": "floating_node",
        "analysis": "dc",
        "solver": {"name": "helqix_circuit", "version": "0.1.0"},
        "seconds": 0.001,
    }
    manifest = build_bundle(source_dir(tmp_path, body), tmp_path / "bundle")
    assert manifest["status"] == "solver_error"
    assert manifest["units"] == [] and manifest["labels"] == []
    assert [entry["name"] for entry in manifest["files"]] == ["results.json"]
    assert not (tmp_path / "bundle" / "values.csv").exists()
    assert "floating node" in (tmp_path / "bundle" / "README.md").read_text(encoding="utf-8")


def test_bad_sources_are_refused(tmp_path):
    with pytest.raises(BundleError, match="does not exist"):
        build_bundle(tmp_path / "missing", tmp_path / "bundle")
    (tmp_path / "empty").mkdir()
    with pytest.raises(BundleError, match="results.json"):
        build_bundle(tmp_path / "empty", tmp_path / "bundle")
    src = tmp_path / "bad_json"
    src.mkdir()
    (src / "results.json").write_text("{not json", encoding="utf-8")
    with pytest.raises(BundleError, match="JSON"):
        build_bundle(src, tmp_path / "bundle")
    src = source_dir(tmp_path, dict(results_ok(), status="half_done"), name="bad_status")
    with pytest.raises(BundleError, match="status"):
        build_bundle(src, tmp_path / "bundle")
    src = source_dir(tmp_path, dict(results_ok(), solver={"name": "helqix_circuit"}), name="bad_solver")
    with pytest.raises(BundleError, match="solver"):
        build_bundle(src, tmp_path / "bundle")
    src = source_dir(tmp_path, "[NaN]", name="bad_nan")
    (src / "results.json").write_text(json.dumps(results_ok()).replace("6.666666666666667", "NaN"), encoding="utf-8")
    with pytest.raises(BundleError, match="finite"):
        build_bundle(src, tmp_path / "bundle")


def test_missing_payload_files_are_refused(tmp_path):
    src = source_dir(tmp_path)
    (src / "values.csv").unlink()
    with pytest.raises(BundleError, match="values.csv"):
        build_bundle(src, tmp_path / "bundle")
    src = source_dir(tmp_path, series=True, name="src2")
    (src / "series.csv").unlink()
    with pytest.raises(BundleError, match="series.csv"):
        build_bundle(src, tmp_path / "bundle")


def test_out_folder_must_be_fresh_and_separate_from_the_source(tmp_path):
    src = source_dir(tmp_path)
    build_bundle(src, tmp_path / "bundle")
    (tmp_path / "bundle" / "notes.txt").write_text("keep", encoding="utf-8")
    with pytest.raises(BundleError, match="fresh"):
        build_bundle(src, tmp_path / "bundle")
    assert (tmp_path / "bundle" / "notes.txt").read_text(encoding="utf-8") == "keep"
    with pytest.raises(BundleError, match="different"):
        build_bundle(src, src)


SCRIPT = Path(__file__).resolve().parents[2] / "scripts" / "result_bundle.py"


def test_script_wraps_the_builder(tmp_path):
    import os
    import subprocess
    import sys

    src = source_dir(tmp_path)
    out = tmp_path / "bundle"
    env = dict(os.environ)
    env["PYTHONPATH"] = str(SCRIPT.parents[1]) + os.pathsep + env.get("PYTHONPATH", "")
    proc = subprocess.run(
        [sys.executable, str(SCRIPT), str(src), str(out)],
        capture_output=True,
        text=True,
        env=env,
        timeout=60,
    )
    assert proc.returncode == 0, proc.stderr
    assert "electronics-result/0.1" in proc.stdout
    assert (out / "manifest.json").is_file()


def test_a_real_circuit_result_bundles(tmp_path):
    import os
    import subprocess
    import sys

    repo = Path(__file__).resolve().parents[2]
    out = tmp_path / "run"
    env = dict(os.environ)
    env["PYTHONPATH"] = str(repo) + os.pathsep + env.get("PYTHONPATH", "")
    proc = subprocess.run(
        [
            sys.executable, "-m", "helqix_circuit", "run",
            str(repo / "examples" / "circuit" / "dc_divider.json"), "--out", str(out),
        ],
        cwd=str(repo),
        capture_output=True,
        text=True,
        env=env,
        timeout=120,
    )
    assert proc.returncode == 0, proc.stderr
    manifest = build_bundle(out, tmp_path / "bundle")
    assert manifest["source"] == {"package": "helqix_circuit", "version": "0.1.0"}
    assert manifest["analysis"] == "dc"
    assert manifest["labels"] == ["CALCULATED"]
    assert manifest["units"] == ["A", "V"]
    assert [entry["name"] for entry in manifest["files"]] == ["results.json", "values.csv"]


def test_manifest_schema_matches_this_version(tmp_path):
    schema_path = Path(__file__).resolve().parents[3] / "docs" / "priya" / "schemas" / "bundle_manifest.schema.json"
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    assert schema["properties"]["version"]["const"] == "electronics-result/0.1"
    manifest = build_bundle(source_dir(tmp_path), tmp_path / "bundle")
    assert set(schema["required"]) <= set(manifest)
    assert set(manifest["files"][0]) == set(schema["properties"]["files"]["items"]["required"])
