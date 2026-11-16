"""link-harness: per-wire currents, losses and deterministic fault scenarios."""

import contextlib
import io
import json
from pathlib import Path
from types import SimpleNamespace

import pytest

from helqix_circuit import InputError, SolverError, cli, load_circuit
from helqix_circuit.harness_doc import load_harness
from helqix_circuit.link import INTERMITTENT_TRIALS, link_harness, parse_fault

REPO = Path(__file__).resolve().parents[2]


def circuit_body():
    return {
        "version": "circuit/0.1",
        "components": [
            {"id": "V1", "kind": "vsource", "nodes": ["p", "0"], "value": {"value": 12.0, "unit": "V"}},
            {"id": "R1", "kind": "resistor", "nodes": ["a", "b"], "value": {"value": 10.0, "unit": "ohm"}},
        ],
        "analysis": {"type": "dc"},
    }


def wire(wid, from_node, to_node, from_pin, to_pin, colour):
    return {
        "id": wid,
        "from": from_node,
        "to": to_node,
        "from_pin": from_pin,
        "to_pin": to_pin,
        "diameter_mm": 1.0,
        "material": "copper",
        "insulation": {"colour": colour, "rating_c": 105},
        "length_mm": 10000.0,
        "slack_mm": 100.0,
        "bend_radius_mm": 6.0,
        "bundle": "B1",
    }


def harness_body(extra_wires=()):
    wires = [
        wire("W1", "p", "a", "J1.1", "J2.1", "red"),
        wire("W2", "b", "0", "J2.2", "J1.2", "black"),
    ]
    wires.extend(extra_wires)
    return {
        "version": "harness/0.1",
        "connectors": [
            {"id": "J1", "at_mm": [0.0, 0.0], "pins": {"1": [0.0, 0.0], "2": [0.0, 20.0], "3": [0.0, 40.0]}},
            {"id": "J2", "at_mm": [100.0, 0.0], "pins": {"1": [100.0, 0.0], "2": [100.0, 20.0], "3": [100.0, 40.0]}},
        ],
        "wires": wires,
        "bundles": [{"id": "B1", "diameter_mm": 6.0}],
        "obstacles": [],
    }


def linked(*faults, seed=11, circuit=None, harness=None):
    return link_harness(
        load_circuit(circuit or circuit_body()),
        load_harness(harness or harness_body()),
        faults=faults,
        seed=seed,
    )


def wire_entry(entries, wid):
    for entry in entries:
        if entry["id"] == wid:
            return entry
    raise AssertionError(f"wire {wid} is not in the table")


def test_as_built_link_table_matches_the_hand_calculation():
    payload = linked()
    assert payload["status"] == "ok"
    assert payload["supply"] == {"value": 12.0, "unit": "V", "label": "GIVEN", "source": "V1"}
    w1 = wire_entry(payload["wires"], "W1")
    assert w1["current_a"]["value"] == pytest.approx(1.149534, abs=1e-6)
    assert w1["drop_v"]["value"] == pytest.approx(0.252330, abs=1e-6)
    assert w1["drop_percent"]["value"] == pytest.approx(2.102751, abs=1e-5)
    assert w1["power_w"]["value"] == pytest.approx(0.290062, abs=1e-6)
    assert w1["verdict"] == "PASS"
    assert w1["colour_hint"] == "OK"
    assert w1["route"]["cut_length_mm"]["value"] == pytest.approx(100.0)
    assert w1["temperature_rise_c"]["value"] is None
    assert w1["temperature_rise_c"]["label"] == "UNKNOWN"
    assert w1["ampacity_a"]["label"] == "UNKNOWN"
    assert payload["verdict"] == "PASS"
    assert payload["scenarios"][0]["id"] == "as_built"
    assert payload["fault_verdict"] is None


def test_joint_raises_the_wire_verdict():
    payload = linked(parse_fault("joint:W1:0.5", {"W1", "W2"}))
    scenario = payload["scenarios"][1]
    assert scenario["fault"] == "joint:W1:0.5"
    w1 = wire_entry(scenario["wires"], "W1")
    assert w1["joint_ohm"]["value"] == 0.5
    assert w1["current_a"]["value"] == pytest.approx(1.096991, abs=1e-6)
    assert w1["drop_v"]["value"] == pytest.approx(0.789292, abs=1e-6)
    assert w1["drop_percent"]["value"] == pytest.approx(6.577435, abs=1e-5)
    assert w1["power_w"]["value"] == pytest.approx(0.865847, abs=1e-6)
    assert w1["verdict"] == "FAIL"
    assert w1["colour_hint"] == "FAIL"
    assert "joint" in w1["reason"]
    w2 = wire_entry(scenario["wires"], "W2")
    assert w2["drop_percent"]["value"] == pytest.approx(2.006639, abs=1e-5)
    assert w2["verdict"] == "PASS"
    assert scenario["verdict"] == "FAIL"
    assert payload["fault_verdict"] == "FAIL"


def test_open_wire_is_reported_and_the_rest_still_solves():
    payload = linked(parse_fault("open:W2", {"W1", "W2"}))
    scenario = payload["scenarios"][1]
    assert scenario["status"] == "ok"
    opened = wire_entry(scenario["wires"], "W2")
    assert opened["current_a"]["value"] == 0.0
    assert opened["verdict"] == "FAIL"
    assert "open" in opened["reason"]
    assert wire_entry(scenario["wires"], "W1")["current_a"]["value"] == pytest.approx(0.0, abs=1e-12)
    assert scenario["verdict"] == "FAIL"


def test_open_wire_that_leaves_a_node_floating_reports_no_solution():
    body = {
        "version": "circuit/0.1",
        "components": [
            {"id": "V1", "kind": "vsource", "nodes": ["p", "0"], "value": {"value": 12.0, "unit": "V"}},
            {"id": "R1", "kind": "resistor", "nodes": ["a", "b"], "value": {"value": 10.0, "unit": "ohm"}},
            {"id": "I1", "kind": "isource", "nodes": ["0", "x"], "value": {"value": 0.5, "unit": "A"}},
        ],
        "analysis": {"type": "dc"},
    }
    extra = [wire("W3", "x", "p", "J2.3", "J1.3", "blue")]
    payload = linked(
        parse_fault("open:W3", {"W1", "W2", "W3"}),
        circuit=body,
        harness=harness_body(extra),
    )
    scenario = payload["scenarios"][1]
    assert scenario["status"] == "no_solution"
    assert "no DC path" in scenario["message"]
    assert scenario["verdict"] == "FAIL"
    assert payload["status"] == "ok"


def test_short_between_wires_matches_the_hand_calculation():
    payload = linked(parse_fault("short:W1,W2", {"W1", "W2"}))
    scenario = payload["scenarios"][1]
    w1 = wire_entry(scenario["wires"], "W1")
    w2 = wire_entry(scenario["wires"], "W2")
    assert w1["current_a"]["value"] == pytest.approx(52.288329, abs=1e-4)
    assert w1["drop_percent"]["value"] == pytest.approx(95.646899, abs=1e-4)
    assert w1["verdict"] == "FAIL"
    assert w2["current_a"]["value"] == pytest.approx(0.051115, abs=1e-6)
    assert w2["verdict"] == "PASS"
    assert scenario["fault_current_a"]["value"] == pytest.approx(52.237213, abs=1e-4)
    assert "to' node" in scenario["model"]


def test_intermittent_is_seeded_and_reports_the_opened_count():
    ids = {"W1", "W2"}
    fault = parse_fault("intermittent:W1:0.5", ids)
    payload = linked(fault, seed=7)
    scenario = payload["scenarios"][1]
    assert scenario["checks"] == {
        "trials": INTERMITTENT_TRIALS,
        "seed": 7,
        "opened": 13,
        "solved": 20,
        "no_solution": 0,
    }
    w1 = wire_entry(scenario["wires"], "W1")
    assert w1["verdict"] == "FAIL"
    assert w1["reason"] == "opens in 13 of 20 seeded checks"
    assert w1["current_min_a"]["value"] == pytest.approx(0.0, abs=1e-12)
    assert w1["current_a"]["value"] == pytest.approx(1.149534, abs=1e-6)
    assert linked(fault, seed=7) == payload
    assert linked(fault, seed=8)["scenarios"][1]["checks"]["opened"] != 13


def test_wire_nodes_must_exist_and_faults_are_parsed_strictly():
    bad = harness_body()
    bad["wires"][0]["from"] = "z"
    with pytest.raises(InputError, match="not in the circuit"):
        linked(harness=bad)
    ids = {"W1", "W2"}
    with pytest.raises(InputError, match="unknown kind"):
        parse_fault("melt:W1", ids)
    with pytest.raises(InputError, match="unknown wire"):
        parse_fault("open:W9", ids)
    with pytest.raises(InputError, match="joint"):
        parse_fault("joint:W1:0", ids)
    with pytest.raises(InputError, match="two"):
        parse_fault("short:W1", ids)
    with pytest.raises(InputError, match="different"):
        parse_fault("short:W1,W1", ids)
    with pytest.raises(InputError, match="probability"):
        parse_fault("intermittent:W1:1.5", ids)


def test_base_solve_failure_is_raised():
    body = circuit_body()
    body["components"].append(
        {"id": "R2", "kind": "resistor", "nodes": ["c", "d"], "value": {"value": 5.0, "unit": "ohm"}}
    )
    with pytest.raises(SolverError):
        linked(circuit=body)


def run_cli(tmp_path, case, harness, *args, out_name="out"):
    case_path = tmp_path / "case.json"
    harness_path = tmp_path / "harness.json"
    case_path.write_text(json.dumps(case), encoding="utf-8")
    harness_path.write_text(json.dumps(harness), encoding="utf-8")
    out = tmp_path / out_name
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr), contextlib.redirect_stdout(io.StringIO()):
        code = cli.main(["link-harness", str(case_path), str(harness_path), "--out", str(out), *args])
    return SimpleNamespace(returncode=code, stderr=stderr.getvalue()), out


def case_wrapper():
    return {"version": "case/0.1", "circuit": circuit_body()}


def test_cli_writes_the_link_report_and_uses_exit_codes(tmp_path):
    proc, out = run_cli(tmp_path, case_wrapper(), harness_body(), "--fault", "joint:W1:0.5", "--seed", "3")
    assert proc.returncode == 0, proc.stderr
    payload = json.loads((out / "results.json").read_text(encoding="utf-8"))
    assert payload["command"] == "link-harness"
    assert payload["status"] == "ok"
    assert [scenario["id"] for scenario in payload["scenarios"]] == ["as_built", "fault-1"]

    proc, out = run_cli(tmp_path, case_wrapper(), harness_body(), "--fault", "melt:W1", out_name="bad_fault")
    assert proc.returncode == 2
    assert "error:" in proc.stderr
    assert not (out / "results.json").exists()

    body = case_wrapper()
    body["wires"] = [{"id": "W9"}]
    proc, _ = run_cli(tmp_path, body, harness_body(), out_name="case_wires")
    assert proc.returncode == 2
    assert "harness document" in proc.stderr


def test_cli_writes_a_solver_error_when_the_base_has_no_solution(tmp_path):
    body = case_wrapper()
    body["circuit"]["components"].append(
        {"id": "R2", "kind": "resistor", "nodes": ["c", "d"], "value": {"value": 5.0, "unit": "ohm"}}
    )
    proc, out = run_cli(tmp_path, body, harness_body())
    assert proc.returncode == 3
    payload = json.loads((out / "results.json").read_text(encoding="utf-8"))
    assert payload["status"] == "solver_error"
    assert payload["code"] == "floating_node"


def test_shipped_harness_example_links_to_its_paired_case(tmp_path):
    case_path = REPO / "examples" / "harness" / "cases" / "feed_case.json"
    harness_path = REPO / "examples" / "harness" / "divider_harness.json"
    out = tmp_path / "example_out"
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr), contextlib.redirect_stdout(io.StringIO()):
        code = cli.main(
            [
                "link-harness", str(case_path), str(harness_path), "--out", str(out),
                "--fault", "joint:W1:1.0",
            ]
        )
    assert code == 0, stderr.getvalue()
    payload = json.loads((out / "results.json").read_text(encoding="utf-8"))
    assert payload["verdict"] == "PASS"
    assert payload["fault_verdict"] == "FAIL"
    assert wire_entry(payload["wires"], "W2")["route"]["cut_length_mm"]["value"] == pytest.approx(726.8496, abs=1e-4)
    w1 = wire_entry(payload["wires"], "W1")
    assert w1["current_a"]["value"] == pytest.approx(1.193718, abs=1e-6)
    assert w1["drop_percent"]["value"] == pytest.approx(0.251803, abs=1e-5)
    assert w1["colour_hint"] == "OK"
