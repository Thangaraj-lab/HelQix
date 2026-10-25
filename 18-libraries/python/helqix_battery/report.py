"""Builds a helqix.validation-report.v1 document for the Rust trust layer (helqix-validate).

The report carries raw series and provenance but no verdict: the label is decided by
helqix-verify, which recomputes every error and re-hashes the split.
"""
from __future__ import annotations

from . import dataset
from .split import SplitViolation

REQUIREMENT = {
    "id": "voltage_rmse_limit",
    "statement": "Terminal-voltage RMSE on each held-out C/10 discharge plus its 2 h rest is below 46 mV "
                 "(anchor: Chen et al. 2020 Table VIII, largest published RMSE of the tuned reference model).",
    "metric": "voltage_rmse",
    "comparison": "LessThan",
    "threshold": {"value": 46.0, "unit": "mV"},
    "accepted": True,
    "accepted_by": "founder delegation, 2026-10-25 (docs/PHASE1_INPUTS.md)",
}

CONTEXT_OF_USE = {
    "statement": "Predict the terminal voltage of an LG M50 21700 cell during constant-current discharge at 25 C "
                 "(cells like those in the dataset), to reproduce when the cell reaches its 2.5 V cut-off under a "
                 "given constant load. Valid only for constant current C/10 at 25 C; any other condition is not validated.",
    "risk": "Low",
    "risk_consequence": "A wrong research or demonstration conclusion; no safety, certification or purchase decision depends on it.",
}


def build(split_, simulated: dict, fitted_on, model_identity: dict, dataset_files: dict, integrity: dict, extras: dict, date: str) -> dict:
    calibration_ids = {c.id for c in split_.calibration}
    for cid in fitted_on:
        if cid not in calibration_ids:
            raise SplitViolation(f"fitted_on names {cid}, which is not a calibration case")
    entries = []
    for role, group in (("calibration", split_.calibration), ("validation", split_.validation)):
        for c in group:
            if c.id not in simulated:
                raise ValueError(f"no simulation for case {c.id}")
            if len(simulated[c.id]) != len(c.time_s):
                raise ValueError(f"simulation for {c.id} has {len(simulated[c.id])} samples, expected {len(c.time_s)}")
            entries.append({"id": c.id, "role": role, "cell": c.cell, "rate": c.rate, "initial_rest_voltage_v": c.initial_rest_voltage_v,
                            "dropped_samples": c.dropped_samples, "time_s": list(c.time_s), "input": list(c.current_a),
                            "measured": list(c.voltage_v), "simulated": list(simulated[c.id])})
    return {
        "schema": "helqix.validation-report.v1",
        "date": date,
        "model": model_identity,
        "dataset": {"doi": dataset.DATASET["doi"], "licence": dataset.DATASET["licence"], "reference": dataset.DATASET["reference"], "files": dataset_files},
        "requirement": REQUIREMENT,
        "context_of_use": CONTEXT_OF_USE,
        "output_unit": "V",
        "input_unit": "A",
        "split": {"sha256": split_.sha256, "fitted_on": list(fitted_on), "protocol": "helqix.battery-split.v1"},
        "numerical_integrity": integrity,
        "value_labels": {
            "measured": {"origin": "Observed", "status": "Unverified"},
            "simulated": {"origin": "Simulated", "status": "Unverified until assessed by helqix-validate for the stated context of use"},
        },
        "cases": entries,
        **extras,
    }
