"""Empirical uncertainty of the validated quantity on held-out data.

Method: per held-out case, RMSE and the nearest-rank 95th percentile of |V_sim - V_meas|; across
cases, the minimum, maximum and mean RMSE. No error distribution is assumed. With only a few
cells this is a description of observed spread, not a statistical confidence interval.
"""
from __future__ import annotations

from . import metrics

METHOD = ("Per case: RMSE and nearest-rank 95th percentile of absolute voltage error over all samples "
          "(discharge and rest). Across cases: min, max and mean RMSE. No distribution assumed; with few cells "
          "this describes observed spread and is not a confidence interval. Instrument voltage accuracy is not "
          "stated in the data source and is not included.")


def summarise(held_out: dict) -> dict:
    """held_out: {case_id: (simulated, measured)}."""
    if not held_out:
        raise ValueError("no held-out cases to summarise")
    per_case = {}
    for cid, (sim, meas) in sorted(held_out.items()):
        per_case[cid] = {"rmse_v": metrics.rmse(sim, meas), "p95_abs_error_v": metrics.percentile_abs_error(sim, meas, 95.0),
                         "max_abs_error_v": metrics.peak_abs_error(sim, meas), "samples": len(meas)}
    rmses = [v["rmse_v"] for v in per_case.values()]
    return {"method": METHOD, "per_case": per_case,
            "across_cases": {"n_cases": len(rmses), "rmse_min_v": min(rmses), "rmse_max_v": max(rmses), "rmse_mean_v": sum(rmses) / len(rmses),
                             "p95_abs_error_max_v": max(v["p95_abs_error_v"] for v in per_case.values())}}
