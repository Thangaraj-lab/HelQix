"""One-at-a-time local sensitivity on calibration data only.

Method: each listed parameter is scaled by (1 - step) and (1 + step) with all others fixed; the
change in RMSE against the measured calibration voltage and the largest voltage change are
reported. Local and one-at-a-time: it does not capture interactions or global behaviour.
"""
from __future__ import annotations

from dataclasses import replace

from . import metrics, model
from .split import require_calibration_set

PARAMETERS = (
    "Negative particle diffusivity [m2.s-1]",
    "Positive particle diffusivity [m2.s-1]",
    "Negative particle radius [m]",
    "Positive particle radius [m]",
    "Maximum concentration in negative electrode [mol.m-3]",
    "Maximum concentration in positive electrode [mol.m-3]",
    "Contact resistance [Ohm]",
)
RESISTANCE = "Contact resistance [Ohm]"


def _config(base: model.ModelConfig, name: str, factor: float) -> model.ModelConfig:
    if name == RESISTANCE:
        return replace(base, contact_resistance_ohm=base.contact_resistance_ohm * factor)
    return replace(base, scaled_parameters=base.scaled_parameters + ((name, factor),))


def one_at_a_time(calibration_set, base: model.ModelConfig, step: float = 0.10, parameters=PARAMETERS) -> dict:
    cases = require_calibration_set(calibration_set).cases
    baseline = {c.id: model.simulate(c.time_s, c.current_a, c.initial_rest_voltage_v, base) for c in cases}
    base_rmse = {c.id: metrics.rmse(baseline[c.id], c.voltage_v) for c in cases}
    entries = []
    for name in parameters:
        for factor in (1.0 - step, 1.0 + step):
            config = _config(base, name, factor)
            d_rmse, max_dv = [], 0.0
            for c in cases:
                v = model.simulate(c.time_s, c.current_a, c.initial_rest_voltage_v, config)
                d_rmse.append(metrics.rmse(v, c.voltage_v) - base_rmse[c.id])
                max_dv = max(max_dv, metrics.peak_abs_error(v, baseline[c.id]))
            entries.append({"parameter": name, "factor": factor, "mean_delta_rmse_v": sum(d_rmse) / len(d_rmse), "max_abs_dv_v": max_dv})
    ranking = sorted({e["parameter"] for e in entries}, key=lambda n: -max(e["max_abs_dv_v"] for e in entries if e["parameter"] == n))
    return {"method": f"one-at-a-time, each parameter scaled by 1 -/+ {step}, others fixed; local only, no interactions",
            "cases": [c.id for c in cases], "baseline_rmse_v": base_rmse, "entries": entries, "ranking_by_max_abs_dv": ranking}
