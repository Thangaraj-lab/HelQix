"""Numerical integrity: is the solution accurate enough that it cannot change the verdict?

Each case is re-solved with 100x tighter solver tolerances and with a mesh refined by 2 in
every dimension. The check passes only if every run completes and the pass/fail verdict
against the threshold is the same in all three runs. The changes are reported, not hidden.
"""
from __future__ import annotations

from dataclasses import replace

from . import metrics, model

TOLERANCE_FACTOR = 100.0
MESH_FACTOR = 2


def check(cases, config: model.ModelConfig, threshold_v: float) -> dict:
    tight = replace(config, rtol=config.rtol / TOLERANCE_FACTOR, atol=config.atol / TOLERANCE_FACTOR)
    refined = replace(config, mesh_scale=config.mesh_scale * MESH_FACTOR)
    entries, passed = [], True
    for c in cases:
        try:
            runs = {name: model.simulate(c.time_s, c.current_a, c.initial_rest_voltage_v, cfg) for name, cfg in (("baseline", config), ("tight", tight), ("refined", refined))}
        except model.SimulationIncomplete as error:
            entries.append({"case": c.id, "error": str(error)})
            passed = False
            continue
        rmse = {name: metrics.rmse(v, c.voltage_v) for name, v in runs.items()}
        verdicts = {name: value < threshold_v for name, value in rmse.items()}
        stable = len(set(verdicts.values())) == 1
        passed = passed and stable
        entries.append({
            "case": c.id,
            "rmse_baseline_v": rmse["baseline"], "rmse_tight_tolerance_v": rmse["tight"], "rmse_refined_mesh_v": rmse["refined"],
            "max_abs_dv_tight_v": metrics.peak_abs_error(runs["tight"], runs["baseline"]),
            "max_abs_dv_refined_v": metrics.peak_abs_error(runs["refined"], runs["baseline"]),
            "verdict_stable": stable,
        })
    return {"passed": passed, "rule": "verdict unchanged under 100x tighter tolerances and 2x mesh refinement; all runs complete",
            "tolerance_factor": TOLERANCE_FACTOR, "mesh_factor": MESH_FACTOR, "threshold_v": threshold_v, "cases": entries}
