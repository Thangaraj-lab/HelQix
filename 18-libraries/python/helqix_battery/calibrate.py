"""Fits the lumped series resistance on calibration cases only (method: docs/validation/CALIBRATION_METHOD.md)."""
from __future__ import annotations

from dataclasses import dataclass, replace

from . import metrics, model
from .split import require_calibration_set

BOUNDS_OHM = (0.0, 0.1)
XATOL_OHM = 1e-5
AT_BOUND_MARGIN_OHM = 1e-4


@dataclass(frozen=True)
class CalibrationResult:
    contact_resistance_ohm: float
    objective_mean_rmse_v: float
    case_rmse_v: dict
    fitted_on: tuple
    split_sha256: str
    evaluations: int
    at_bound: bool
    method: str = "scipy.optimize.minimize_scalar(method='bounded') on mean RMSE"


def _mean_rmse(cases, config):
    per_case = {c.id: metrics.rmse(model.simulate(c.time_s, c.current_a, c.initial_rest_voltage_v, config), c.voltage_v) for c in cases}
    return sum(per_case.values()) / len(per_case), per_case


def calibrate(calibration_set, base: model.ModelConfig) -> CalibrationResult:
    calibration_set = require_calibration_set(calibration_set)
    from scipy.optimize import minimize_scalar
    cases = calibration_set.cases
    found = minimize_scalar(lambda r: _mean_rmse(cases, replace(base, contact_resistance_ohm=float(r)))[0],
                            bounds=BOUNDS_OHM, method="bounded", options={"xatol": XATOL_OHM})
    resistance = float(found.x)
    objective, per_case = _mean_rmse(cases, replace(base, contact_resistance_ohm=resistance))
    at_bound = resistance - BOUNDS_OHM[0] < AT_BOUND_MARGIN_OHM or BOUNDS_OHM[1] - resistance < AT_BOUND_MARGIN_OHM
    return CalibrationResult(resistance, objective, per_case, tuple(c.id for c in cases), calibration_set.split_sha256, int(found.nfev), at_bound)
