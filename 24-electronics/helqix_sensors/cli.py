"""The platform contract: python -m helqix_sensors run CASE --out DIR.

Exit 0 on a result, 2 on bad input, 3 on solver failure (including a timeout).
Only the output folder is written. No network.
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
    load_model,
    simulate,
    synthesize_signal,
)

CASE_VERSION = "case/0.1"
CASE_KEYS = ("limits", "model", "probe", "reference", "seed", "signal", "version")
MAX_PROBE_POINTS = 2000
DEFAULT_LIMITS = {"timeout_s": 30.0}
OUTPUT_FILES = ("results.json", "series.csv", "values.csv")
REFERENCE_QUANTITIES = ("reported_mean", "reality_gap_rms", "clipped_samples", "metric")
SERIES_IDS = ("truth", "analog", "reported", "clipped")
REPORT_UNITS = {
    "sample_count": "1",
    "reality_gap_rms": None,
    "quantization_step": None,
    "quantization_rms_theory": None,
    "quantization_error_rms": None,
    "noise_sigma_predicted": None,
    "noise_sigma_measured": None,
    "noise_error_rms": None,
    "delay_error_rms": None,
    "cutoff_requested_hz": "Hz",
    "cutoff_measured_hz": "Hz",
    "enbw_hz": "Hz",
    "group_delay_s": "s",
    "clipped_samples": "1",
}
REPORT_ORDER = tuple(REPORT_UNITS)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(
        prog="python -m helqix_sensors",
        description="Run one sensor reality case offline and write results to a folder.",
    )
    sub = parser.add_subparsers(dest="command")
    run_parser = sub.add_parser("run", help="run one case JSON file")
    run_parser.add_argument("case", help="path to the case JSON file")
    run_parser.add_argument("--out", required=True, help="fresh folder for results and CSVs")
    args = parser.parse_args(argv)
    if args.command != "run":
        parser.print_help()
        return 2
    try:
        return _run(args.case, args.out)
    except InputError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2


def _run(case_path: str, out_text: str) -> int:
    body = _load_case(case_path)
    limits = _limits(body.get("limits"))
    reference = _reference(body.get("reference"))
    model = load_model(body["model"])
    signal = body["signal"]
    if isinstance(signal, dict) and "synthesize" in signal:
        if set(signal) != {"synthesize"}:
            raise InputError("signal with 'synthesize' must not carry other keys")
        signal = synthesize_signal(signal["synthesize"], model.sample_rate_hz)
    probe = body.get("probe")
    seed = body.get("seed", 0)

    out = _prepare_out(out_text)
    started = time.perf_counter()
    try:
        _check_time(deadline=started + limits["timeout_s"], timeout_s=limits["timeout_s"])
        trace = simulate(model, signal, seed=seed, probe=probe)
        _check_time(deadline=started + limits["timeout_s"], timeout_s=limits["timeout_s"])
    except SolverError as exc:
        _write_json(
            out / "results.json",
            {
                "status": "solver_error",
                "message": str(exc),
                "code": getattr(exc, "code", None),
                "analysis": "sensor",
                "solver": {"name": "helqix_sensors", "version": __version__},
                "seconds": _seconds(started),
            },
        )
        return 3

    payload = _series_payload(trace)
    label = "CHECKED"
    reference_report = None
    if reference is not None:
        actual = _reference_actual(reference, trace)
        reference_report = _judge(reference, actual)
        label = "VALIDATED" if reference_report["matched"] else "UNVALIDATED"
    result = {
        "status": "ok",
        "analysis": "sensor",
        "validation_label": label,
        "solver": {"name": "helqix_sensors", "version": __version__},
        "seconds": _seconds(started),
        "limits": limits,
        "model": {"id": trace.model.id, "kind": trace.model.kind, "unit": trace.model.unit, "source": trace.model.source},
        "seed": seed,
        "probe": probe,
        "values": payload["values"],
        "probes": payload["probes"],
        "series_csv": "series.csv",
        "report": trace.report,
    }
    if reference_report is not None:
        result["reference"] = reference_report
    _write_json(out / "results.json", result)
    _write_values_csv(out / "values.csv", result["values"])
    _write_series_csv(out / "series.csv", payload["probes"])
    return 0


def _seconds(started: float) -> float:
    return round(time.perf_counter() - started, 6)


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
    if "model" not in body:
        raise InputError("sensor case needs a 'model' object or path")
    if "signal" not in body:
        raise InputError("sensor case needs a 'signal' (series, probe document or synthesize block)")
    return body


def _finite(value, what: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise InputError(f"{what} must be a number")
    number = float(value)
    if not math.isfinite(number):
        raise InputError(f"{what} must be finite")
    return number


def _limits(raw) -> dict:
    if raw is None:
        raw = {}
    if not isinstance(raw, dict):
        raise InputError("limits must be an object")
    unknown = sorted(set(raw) - set(DEFAULT_LIMITS))
    if unknown:
        raise InputError(f"unknown limit(s) {unknown}; sensor cases honor only timeout_s")
    timeout = _finite(raw.get("timeout_s", DEFAULT_LIMITS["timeout_s"]), "limits.timeout_s")
    if timeout <= 0:
        raise InputError("limits.timeout_s must be greater than zero")
    return {"timeout_s": timeout}


def _reference(raw):
    if raw is None:
        return None
    if not isinstance(raw, dict):
        raise InputError("reference must be an object")
    quantity = raw.get("quantity")
    if quantity not in REFERENCE_QUANTITIES:
        raise InputError(f"reference quantity must be one of {list(REFERENCE_QUANTITIES)}, got {quantity!r}")
    name = raw.get("name", "")
    if not isinstance(name, str):
        raise InputError("reference name must be a string")
    value = _finite(raw.get("value"), "reference value")
    tolerance = _finite(raw.get("relative_tolerance", 1e-6), "reference relative_tolerance")
    if tolerance <= 0:
        raise InputError("reference relative_tolerance must be greater than zero")
    return {"quantity": quantity, "name": name, "value": value, "relative_tolerance": tolerance}


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


def _series_payload(trace) -> dict:
    index = _downsample_index(trace.time_s.size)
    unit = trace.model.unit
    series = {
        "truth": trace.truth,
        "analog": trace.analog,
        "reported": trace.reported,
        "clipped": np.asarray(trace.clipped, dtype=float),
    }
    probes = [
        {
            "id": probe_id,
            "unit": unit if probe_id != "clipped" else "1",
            "x_name": "time_s",
            "x": [float(value) for value in trace.time_s[index]],
            "value": [float(value) for value in series[probe_id][index]],
            "samples_total": int(trace.time_s.size),
            "samples_written": int(index.size),
            "label": "CALCULATED" if probe_id != "clipped" else "MEASURED",
        }
        for probe_id in SERIES_IDS
    ]
    values = []
    for name in REPORT_ORDER:
        if name in trace.report:
            unit_or_none = REPORT_UNITS[name]
            values.append(
                {
                    "name": name,
                    "quantity": "sensor_metric",
                    "value": float(trace.report[name]),
                    "unit": unit if unit_or_none is None else unit_or_none,
                    "label": trace.report.get("labels", {}).get(name, "CALCULATED"),
                }
            )
    return {"probes": probes, "values": values}


def _downsample_index(size: int) -> np.ndarray:
    if size <= MAX_PROBE_POINTS:
        return np.arange(size)
    return np.unique(np.rint(np.linspace(0.0, size - 1.0, MAX_PROBE_POINTS)).astype(int))


def _reference_actual(reference: dict, trace) -> float:
    quantity = reference["quantity"]
    if quantity == "reported_mean":
        return float(np.mean(trace.reported))
    if quantity == "clipped_samples":
        return float(trace.report["clipped_samples"])
    if quantity == "reality_gap_rms":
        return float(trace.report["reality_gap_rms"])
    name = reference["name"]
    if name not in trace.report:
        raise InputError(f"reference metric {name!r} is not in the sensor report")
    return float(trace.report[name])


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
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow([probes[0]["x_name"]] + [probe["id"] for probe in probes])
        for row in range(len(probes[0]["x"])):
            writer.writerow(
                [repr(probes[0]["x"][row])] + [repr(probe["value"][row]) for probe in probes]
            )
