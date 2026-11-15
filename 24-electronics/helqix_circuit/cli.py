"""The platform contracts: ``run CASE --out DIR`` and
``link-harness CASE HARNESS --out DIR``.

Exit 0 on a result, 2 on bad input, 3 on solver failure. Only the output
folder is written. No network, no clock-dependent results.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
import time
from pathlib import Path

import numpy as np

from . import (
    InputError,
    SolverError,
    __version__,
    ac_sweep,
    analyze_harness,
    cutoff_frequency,
    load_circuit,
    resonance_metrics,
    solve_dc,
    transient,
)
from .harness_doc import load_harness
from .link import DEFAULT_SEED, link_harness, parse_fault

CASE_VERSION = "case/0.1"
CASE_KEYS = ("circuit", "limits", "reference", "version", "wires")
MAX_PROBE_POINTS = 2000
MAX_REQUESTED_STEPS = 2_000_000
DEFAULT_LIMITS = {"timeout_s": 30.0, "max_nodes": 10000, "max_steps": 200000}
OUTPUT_FILES = ("results.json", "series.csv", "values.csv")
REFERENCE_QUANTITIES = ("node_voltage", "branch_current", "probe_final", "metric")
LABEL = "CALCULATED"


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(
        prog="python -m helqix_circuit",
        description="Run one circuit case offline and write results to a folder.",
    )
    sub = parser.add_subparsers(dest="command")
    run_parser = sub.add_parser("run", help="run one case JSON file")
    run_parser.add_argument("case", help="path to the case JSON file")
    run_parser.add_argument("--out", required=True, help="fresh folder for results and CSVs")
    link_parser = sub.add_parser(
        "link-harness",
        help="link a harness/0.1 document to a DC circuit and check every wire",
    )
    link_parser.add_argument("case", help="path to the case JSON file; it must not carry a wires list")
    link_parser.add_argument("harness", help="path to the harness/0.1 JSON document")
    link_parser.add_argument("--out", required=True, help="fresh folder for the link report")
    link_parser.add_argument(
        "--fault",
        action="append",
        default=[],
        metavar="TEXT",
        help="inject a fault (repeatable): open:W, joint:W:OHM, short:W1,W2 or intermittent:W:P",
    )
    link_parser.add_argument(
        "--seed",
        type=int,
        default=DEFAULT_SEED,
        help=f"seed for intermittent faults (default {DEFAULT_SEED})",
    )
    args = parser.parse_args(argv)
    try:
        if args.command == "run":
            return _run(args.case, args.out)
        if args.command == "link-harness":
            return _link(args.case, args.harness, args.out, args.fault, args.seed)
    except InputError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    parser.print_help()
    return 2


def _check_step_budget(circuit, kind: str) -> None:
    if kind != "transient":
        return
    analysis = circuit.analysis
    try:
        stop = float(analysis.get("stop_s", 0.0))
        step = float(analysis.get("step_s", stop / 100.0 if stop > 0 else 0.0))
    except (TypeError, ValueError):
        return
    if not (math.isfinite(stop) and math.isfinite(step)) or stop <= 0 or step <= 0:
        return
    requested = stop / step
    if requested > MAX_REQUESTED_STEPS:
        raise InputError(
            f"transient requests {requested:,.0f} steps from stop_s / step_s; the cap is "
            f"{MAX_REQUESTED_STEPS:,}. Increase step_s or shorten stop_s."
        )


def _run(case_path: str, out_text: str) -> int:
    body = _load_case(case_path)
    limits = _limits(body.get("limits"))
    reference = _reference(body.get("reference"))
    wires = _wires(body.get("wires"))
    circuit = load_circuit(body["circuit"])
    kind = _analysis_kind(circuit)
    _check_step_budget(circuit, kind)
    node_count = len([name for name in circuit.node_names if name != "0"])
    if node_count > limits["max_nodes"]:
        raise InputError(f"case needs {node_count} nodes; max_nodes is {limits['max_nodes']}")
    if wires is not None and kind != "dc":
        raise InputError("wire checks use a DC analysis; set analysis type 'dc' or remove the wires list")

    out = _prepare_out(out_text)
    started = time.perf_counter()
    deadline = started + limits["timeout_s"]
    try:
        payload = _analyze(circuit, kind, limits, wires, deadline)
    except SolverError as exc:
        _write_json(
            out / "results.json",
            {
                "status": "solver_error",
                "message": str(exc),
                "code": getattr(exc, "code", None),
                "analysis": kind,
                "solver": {"name": "helqix_circuit", "version": __version__},
                "seconds": _seconds(started),
            },
        )
        return 3

    label = "CHECKED"
    reference_report = None
    if reference is not None:
        payload["reference"] = _find_reference_value(reference, payload)
        reference_report = _judge(reference, payload["reference"])
        label = "VALIDATED" if reference_report["matched"] else "UNVALIDATED"
    result = {
        "status": "ok",
        "analysis": kind,
        "validation_label": label,
        "solver": {"name": "helqix_circuit", "version": __version__},
        "seconds": _seconds(started),
        "limits": limits,
        "values": payload["values"],
        "probes": payload["probes"],
        "series_csv": "series.csv" if payload["probes"] else None,
    }
    if reference_report is not None:
        result["reference"] = reference_report
    if wires is not None:
        result["wires"] = payload["wires"]
        result["wire_verdict"] = payload["wire_verdict"]
    if payload.get("report") is not None:
        result["report"] = payload["report"]
    _write_json(out / "results.json", result)
    _write_values_csv(out / "values.csv", result["values"])
    if payload["probes"]:
        _write_series_csv(out / "series.csv", payload["probes"])
    return 0


def _seconds(started: float) -> float:
    return round(time.perf_counter() - started, 6)


def _read_harness(path_text: str):
    try:
        text = Path(path_text).read_text(encoding="utf-8")
    except OSError as exc:
        raise InputError(f"cannot read harness file {path_text}: {exc.strerror or exc}") from exc
    try:
        body = json.loads(text)
    except json.JSONDecodeError as exc:
        raise InputError(f"harness file is not valid JSON: {exc}") from exc
    return load_harness(body)


def _link(case_path: str, harness_path: str, out_text: str, fault_texts, seed: int) -> int:
    body = _load_case(case_path)
    if body.get("wires") is not None:
        raise InputError("the harness document supplies the wires; remove the 'wires' list from the case")
    circuit = load_circuit(body["circuit"])
    if _analysis_kind(circuit) != "dc":
        raise InputError("link-harness uses a DC analysis; set the circuit analysis type to 'dc'")
    limits = _limits(body.get("limits"))
    node_count = len([name for name in circuit.node_names if name != "0"])
    if node_count > limits["max_nodes"]:
        raise InputError(f"case needs {node_count} nodes; max_nodes is {limits['max_nodes']}")
    doc = _read_harness(harness_path)
    fault_ids = {wire.id for wire in doc.wires}
    faults = [parse_fault(text, fault_ids) for text in fault_texts]

    out = _prepare_out(out_text)
    started = time.perf_counter()
    try:
        payload = link_harness(circuit, doc, faults=faults, seed=seed)
    except SolverError as exc:
        _write_json(
            out / "results.json",
            {
                "status": "solver_error",
                "command": "link-harness",
                "message": str(exc),
                "code": getattr(exc, "code", None),
                "analysis": "dc",
                "solver": {"name": "helqix_circuit", "version": __version__},
                "seconds": _seconds(started),
            },
        )
        return 3
    payload["solver"] = {"name": "helqix_circuit", "version": __version__}
    payload["seconds"] = _seconds(started)
    _write_json(out / "results.json", payload)
    return 0


def _load_case(path_text: str) -> dict:
    try:
        text = Path(path_text).read_text(encoding="utf-8")
    except OSError as exc:
        raise InputError(f"cannot read case file {path_text}: {exc.strerror or exc}") from exc
    try:
        body = json.loads(text)
    except json.JSONDecodeError as exc:
        raise InputError(f"case file is not valid JSON: {exc}") from exc
    if not isinstance(body, dict):
        raise InputError("case must be a JSON object")
    if body.get("version") != CASE_VERSION:
        raise InputError(f"case version must be {CASE_VERSION!r}, got {body.get('version')!r}")
    unknown = sorted(set(body) - set(CASE_KEYS))
    if unknown:
        raise InputError(f"unknown case key(s) {unknown}; allowed keys: {list(CASE_KEYS)}")
    if not isinstance(body.get("circuit"), dict):
        raise InputError("circuit case needs a 'circuit' netlist object")
    return body


def _finite(value, what: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise InputError(f"{what} must be a number")
    number = float(value)
    if not math.isfinite(number):
        raise InputError(f"{what} must be finite")
    return number


def _positive_int(value, what: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise InputError(f"{what} must be an integer")
    if value < 1:
        raise InputError(f"{what} must be at least 1")
    return value


def _limits(raw) -> dict:
    if raw is None:
        raw = {}
    if not isinstance(raw, dict):
        raise InputError("limits must be an object")
    unknown = sorted(set(raw) - set(DEFAULT_LIMITS))
    if unknown:
        raise InputError(f"unknown limit(s) {unknown}; known limits: {sorted(DEFAULT_LIMITS)}")
    limits = dict(DEFAULT_LIMITS)
    limits.update(raw)
    timeout = _finite(limits["timeout_s"], "limits.timeout_s")
    if timeout <= 0:
        raise InputError("limits.timeout_s must be greater than zero")
    return {
        "timeout_s": timeout,
        "max_nodes": _positive_int(limits["max_nodes"], "limits.max_nodes"),
        "max_steps": _positive_int(limits["max_steps"], "limits.max_steps"),
    }


def _reference(raw):
    if raw is None:
        return None
    if not isinstance(raw, dict):
        raise InputError("reference must be an object")
    quantity = raw.get("quantity")
    if quantity not in REFERENCE_QUANTITIES:
        raise InputError(f"reference quantity must be one of {list(REFERENCE_QUANTITIES)}, got {quantity!r}")
    name = raw.get("name")
    if not isinstance(name, str) or not name:
        raise InputError("reference name must be a non-empty string")
    value = _finite(raw.get("value"), "reference value")
    tolerance = _finite(raw.get("relative_tolerance", 1e-6), "reference relative_tolerance")
    if tolerance <= 0:
        raise InputError("reference relative_tolerance must be greater than zero")
    return {"quantity": quantity, "name": name, "value": value, "relative_tolerance": tolerance}


def _wires(raw):
    if raw is None:
        return None
    if not isinstance(raw, list) or not raw:
        raise InputError("wires must be a non-empty list")
    return raw


def _analysis_kind(circuit) -> str:
    raw = str(circuit.analysis.get("type", "dc")).lower()
    if raw in ("dc",):
        return "dc"
    if raw in ("tran", "transient"):
        return "transient"
    if raw == "ac":
        return "ac"
    raise InputError(f"unsupported analysis type {raw!r}; use 'dc', 'tran' or 'ac'")


def _prepare_out(text: str) -> Path:
    out = Path(text)
    if out.exists():
        if not out.is_dir():
            raise InputError(f"out path {out} is not a folder")
        foreign = sorted(p.name for p in out.iterdir() if p.name not in OUTPUT_FILES)
        if foreign:
            raise InputError(
                f"out folder {out} is not fresh; it already contains {foreign}; choose an empty folder"
            )
        for name in OUTPUT_FILES:
            path = out / name
            if path.exists():
                path.unlink()
    else:
        try:
            out.mkdir(parents=True)
        except OSError as exc:
            raise InputError(f"cannot create out folder {out}: {exc.strerror or exc}") from exc
    return out


def _run_with_timeout(fn, timeout_s: float):
    box = {}

    def target():
        try:
            box["value"] = fn()
        except BaseException as exc:  # noqa: BLE001 - re-raised on the caller
            box["error"] = exc

    thread = threading.Thread(target=target, daemon=True)
    thread.start()
    thread.join(timeout_s)
    if thread.is_alive():
        raise SolverError(f"analysis exceeded timeout_s={timeout_s:g}", code="timeout")
    if "error" in box:
        raise box["error"]
    return box["value"]


def _check_time(deadline: float, timeout_s: float) -> None:
    if time.perf_counter() > deadline:
        raise SolverError(f"analysis exceeded timeout_s={timeout_s:g}", code="timeout")


def _analyze(circuit, kind: str, limits: dict, wires, deadline: float) -> dict:
    _check_time(deadline, limits["timeout_s"])
    if kind == "dc":
        solution = solve_dc(circuit)
        _check_time(deadline, limits["timeout_s"])
        values = [
            _entry(f"V({name})", "node_voltage", volts, "V")
            for name, volts in solution.node_voltages.items()
        ]
        values += [
            _entry(f"I({cid})", "branch_current", amps, "A")
            for cid, amps in solution.branch_currents.items()
        ]
        result = {"values": values, "probes": []}
        if wires is not None:
            _check_time(deadline, limits["timeout_s"])
            report = analyze_harness(circuit, wires).to_dict()
            _check_time(deadline, limits["timeout_s"])
            result["wires"] = list(report["wires"].values())
            result["wire_verdict"] = report["verdict"]
        return result

    if kind == "transient":
        solution = transient(circuit, max_steps=limits["max_steps"], deadline=deadline)
        index = _downsample_index(solution.times.size)
        probes = [
            _probe(f"V({name})", "V", "time_s", solution.times, volts, index)
            for name, volts in solution.node_voltages.items()
        ]
        probes += [
            _probe(f"I({cid})", "A", "time_s", solution.times, amps, index)
            for cid, amps in solution.branch_currents.items()
        ]
        values = [
            _entry(probe["id"], "probe_final", float(probe["value"][-1]), probe["unit"])
            for probe in probes
        ]
        return {"values": values, "probes": probes, "report": _json_safe(solution.report)}

    solution = ac_sweep(circuit, deadline=deadline)
    index = _downsample_index(solution.frequencies.size)
    probes = [
        _probe(f"V({name})", "V", "frequency_hz", solution.frequencies, np.abs(volts), index)
        for name, volts in solution.node_voltages.items()
    ]
    values = [
        _entry(probe["id"], "probe_final", float(probe["value"][-1]), probe["unit"])
        for probe in probes
    ]
    metrics = _ac_metrics(circuit)
    values += metrics
    _check_time(deadline, limits["timeout_s"])
    report = {
        "start_hz": float(solution.frequencies[0]),
        "stop_hz": float(solution.frequencies[-1]),
        "points": int(solution.frequencies.size),
    }
    return {"values": values, "probes": probes, "report": report}


def _ac_metrics(circuit) -> list[dict]:
    analysis = circuit.analysis
    values = []
    cutoff = analysis.get("cutoff")
    if cutoff is not None:
        if not isinstance(cutoff, dict) or not isinstance(cutoff.get("source"), str) or not isinstance(cutoff.get("node"), str):
            raise InputError("analysis.cutoff needs a source id and a node")
        measured = cutoff_frequency(circuit, cutoff["source"], cutoff["node"])
        values.append(_entry("cutoff_hz", "metric", measured, "Hz"))
    resonance = analysis.get("resonance")
    if resonance is not None:
        if not isinstance(resonance, dict) or not isinstance(resonance.get("source"), str) or not isinstance(resonance.get("node"), str):
            raise InputError("analysis.resonance needs a source id and a node")
        measured = resonance_metrics(
            circuit,
            resonance["source"],
            resonance["node"],
            _finite(resonance.get("f_min_hz", 1.0), "resonance f_min_hz"),
            _finite(resonance.get("f_max_hz", 1e9), "resonance f_max_hz"),
            points_per_decade=_positive_int(resonance.get("points_per_decade", 200), "resonance points_per_decade"),
            reference_node=str(resonance.get("reference_node", "0")),
        )
        for name, number in measured.items():
            unit = "Hz" if name.endswith("_hz") else "1"
            values.append(_entry(name, "metric", float(number), unit))
    return values


def _downsample_index(size: int) -> np.ndarray:
    if size <= MAX_PROBE_POINTS:
        return np.arange(size)
    return np.unique(np.rint(np.linspace(0.0, size - 1.0, MAX_PROBE_POINTS)).astype(int))


def _probe(probe_id: str, unit: str, x_name: str, x: np.ndarray, y: np.ndarray, index: np.ndarray) -> dict:
    return {
        "id": probe_id,
        "unit": unit,
        "x_name": x_name,
        x_name: [float(value) for value in x[index]],
        "value": [float(value) for value in y[index]],
        "samples_total": int(x.size),
        "samples_written": int(index.size),
        "label": LABEL,
    }


def _entry(name: str, quantity: str, value, unit: str) -> dict:
    return {"name": name, "quantity": quantity, "value": float(value), "unit": unit, "label": LABEL}


def _find_reference_value(reference: dict, payload: dict) -> float:
    quantity = reference["quantity"]
    name = reference["name"]
    if quantity == "probe_final":
        for probe in payload["probes"]:
            if probe["id"] == name:
                return float(probe["value"][-1])
        raise InputError(f"reference probe {name!r} is not in the result")
    for entry in payload["values"]:
        if entry["quantity"] == quantity and entry["name"] == name:
            return float(entry["value"])
    raise InputError(f"reference {quantity} {name!r} is not in the result")


def _judge(reference: dict, actual: float) -> dict:
    expected = reference["value"]
    scale = max(abs(expected), 1e-300)
    error = abs(actual - expected) / scale
    tolerance = reference["relative_tolerance"]
    return {
        "quantity": reference["quantity"],
        "name": reference["name"],
        "expected": expected,
        "actual": actual,
        "relative_error": error,
        "relative_tolerance": tolerance,
        "matched": error <= tolerance,
    }


def _json_safe(value):
    if isinstance(value, dict):
        return {str(key): _json_safe(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [_json_safe(item) for item in value]
    if isinstance(value, np.generic):
        return value.item()
    return value


def _write_json(path: Path, payload: dict) -> None:
    path.write_text(json.dumps(_json_safe(payload), indent=2, allow_nan=False) + "\n", encoding="utf-8")


def _write_values_csv(path: Path, values: list[dict]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["name", "quantity", "value", "unit", "label"])
        for entry in values:
            writer.writerow([entry["name"], entry["quantity"], repr(entry["value"]), entry["unit"], entry["label"]])


def _write_series_csv(path: Path, probes: list[dict]) -> None:
    x_key = probes[0]["x_name"]
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow([x_key] + [probe["id"] for probe in probes])
        for row in range(len(probes[0][x_key])):
            writer.writerow(
                [repr(probes[0][x_key][row])] + [repr(probe["value"][row]) for probe in probes]
            )
