"""helqix_sensors — deterministic models of what a real sensor would report."""

from .errors import HelqixError, InputError
from .sensor import (
    KINDS,
    SensorModel,
    SensorTrace,
    load_model,
    simulate,
    synthesize_signal,
)

PACKAGE_NAME = "helqix_sensors"
__version__ = "0.1.0"

__all__ = [
    "HelqixError",
    "InputError",
    "KINDS",
    "SensorModel",
    "SensorTrace",
    "load_model",
    "simulate",
    "synthesize_signal",
]
