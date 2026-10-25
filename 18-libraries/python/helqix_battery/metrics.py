"""Error metrics between a simulated and a measured series sampled at the same times."""
from __future__ import annotations

import math


def _checked(simulated, measured):
    if len(simulated) != len(measured) or not measured:
        raise ValueError("series must be non-empty and the same length")
    if not all(math.isfinite(x) for x in list(simulated) + list(measured)):
        raise ValueError("series contain a non-finite value")
    return [s - m for s, m in zip(simulated, measured)]


def rmse(simulated, measured) -> float:
    diffs = _checked(simulated, measured)
    return math.sqrt(sum(d * d for d in diffs) / len(diffs))


def peak_abs_error(simulated, measured) -> float:
    return max(abs(d) for d in _checked(simulated, measured))


def percentile_abs_error(simulated, measured, percentile: float) -> float:
    """Nearest-rank percentile of |error| (no interpolation, no distribution assumed)."""
    errors = sorted(abs(d) for d in _checked(simulated, measured))
    rank = max(1, math.ceil(percentile / 100.0 * len(errors)))
    return errors[rank - 1]
