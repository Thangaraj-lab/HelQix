"""Runs the Phase 1 battery validation pipeline and writes report.json for helqix-validate.

Usage (from 18-libraries/python):
    python -m helqix_battery.validate --data <dir> --out <dir> --date YYYY-MM-DD

Order is fixed: verify data -> extract cases -> split (must equal the committed record) ->
calibrate on calibration cases only -> simulate every case with the calibrated candidate ->
numerical integrity on held-out cases -> sensitivity (calibration only) -> uncertainty -> report.
This script assigns no credibility label; helqix-validate (Rust) does.
"""
from __future__ import annotations

import argparse
import json
from dataclasses import asdict, replace
from pathlib import Path

from . import calibrate, cases, dataset, integrity, maccor, metrics, model, report, sensitivity, split, uncertainty

EXPECTED_SPLIT_SHA256 = "9858fefda7fc8f779601bad7f25b0567ca4d0e676d8a69e8ac617fb9f28031db"  # docs/validation/SPLIT_RECORD.md
THRESHOLD_V = 0.046


class SplitMismatch(RuntimeError):
    """The data produced a split different from the one fixed before fitting."""


def _cell_id(file_name: str) -> str:
    return file_name.split("_")[1].split(".")[0]


def run(data_dir: Path, out_dir: Path, date: str, files=None, expected_split_sha256=EXPECTED_SPLIT_SHA256, sensitivity_step=0.10) -> dict:
    data_dir, out_dir = Path(data_dir), Path(out_dir)
    files = dict(dataset.DATASET["files"] if files is None else files)
    if all((data_dir / name).is_file() for name in files):
        for name, md5 in files.items():
            dataset.verify_file(data_dir / name, md5)
        paths = {name: data_dir / name for name in files}
    else:
        paths = dataset.fetch(data_dir, files=files)

    all_cases, excluded = [], []
    for name in sorted(paths):
        extracted, dropped = cases.extract_cases(_cell_id(name), maccor.read_maccor_csv(paths[name]))
        all_cases += extracted
        excluded += [{"where": w, "reason": r} for w, r in dropped]
    fixed = split.make_split(all_cases)
    if expected_split_sha256 is not None and fixed.sha256 != expected_split_sha256:
        raise SplitMismatch(f"split {fixed.sha256} differs from the recorded {expected_split_sha256}")

    base = model.ModelConfig()
    fit = calibrate.calibrate(fixed.calibration_set(), base)  # sees calibration cases only
    candidate = replace(base, contact_resistance_ohm=fit.contact_resistance_ohm)

    simulated = {c.id: model.simulate(c.time_s, c.current_a, c.initial_rest_voltage_v, candidate) for c in all_cases}
    reference = {c.id: metrics.rmse(model.simulate(c.time_s, c.current_a, c.initial_rest_voltage_v, base), c.voltage_v) for c in all_cases}
    numerical = integrity.check(list(fixed.validation), candidate, THRESHOLD_V)
    sens = sensitivity.one_at_a_time(fixed.calibration_set(), candidate, step=sensitivity_step)
    unc = uncertainty.summarise({c.id: (simulated[c.id], c.voltage_v) for c in fixed.validation})

    doc = report.build(
        split_=fixed, simulated=simulated, fitted_on=list(fit.fitted_on), model_identity=model.identity(candidate),
        dataset_files={name: {"md5": dataset.file_md5(p), "sha256": dataset.file_sha256(p)} for name, p in sorted(paths.items())},
        integrity=numerical, date=date,
        extras={"calibration": {**asdict(fit), "candidate_declared": "SPM + Chen2020 + fitted contact resistance (docs/validation/CALIBRATION_METHOD.md)"},
                "reference_unfitted_rmse_v": reference, "sensitivity": sens, "uncertainty": unc, "excluded_steps": excluded},
    )
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "report.json").write_text(json.dumps(doc, indent=1) + "\n", encoding="utf-8")
    (out_dir / "summary.md").write_text(_summary(doc, fixed), encoding="utf-8")
    return {"split_sha256": fixed.sha256, "contact_resistance_ohm": fit.contact_resistance_ohm, "integrity_passed": numerical["passed"]}


def _summary(doc: dict, fixed) -> str:
    def mv(v):
        return f"{1000 * v:.1f}"
    rows = []
    for c in doc["cases"]:
        rows.append(f"| {c['id']} | {c['role']} | {mv(metrics.rmse(c['simulated'], c['measured']))} | {mv(doc['reference_unfitted_rmse_v'][c['id']])} | {mv(metrics.peak_abs_error(c['simulated'], c['measured']))} |")
    cal = doc["calibration"]
    return "\n".join([
        f"# Battery validation run ({doc['date']})", "",
        f"Model: {doc['model']['name']} (sha256 `{doc['model']['sha256']}`)",
        f"Split: `{fixed.sha256}`; fitted on {len(cal['fitted_on'])} calibration cases only.",
        f"Fitted contact resistance: {cal['contact_resistance_ohm']:.6f} Ohm (at bound: {cal['at_bound']}, {cal['evaluations']} evaluations).",
        f"Numerical integrity passed: {doc['numerical_integrity']['passed']}.", "",
        "| Case | Role | RMSE candidate (mV) | RMSE unfitted (mV) | Peak error candidate (mV) |", "|---|---|---|---|---|", *rows, "",
        "No credibility label is assigned here. Run helqix-validate on report.json; only its output decides VerifiedForContext or NotVerified.", ""])


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--data", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--date", required=True)
    args = parser.parse_args(argv)
    result = run(args.data, args.out, args.date)
    print(json.dumps(result, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
