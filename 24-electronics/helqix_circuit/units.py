"""Explicit unit conversion. No value is accepted without a known unit."""

from __future__ import annotations

import math

from .errors import InputError

# Scale to SI. Keys are the canonical spelling; aliases are normalised first.
_SCALE = {
    "1": 1.0,
    "ohm": 1.0,
    "kohm": 1e3,
    "megohm": 1e6,
    "mohm": 1e-3,
    "v": 1.0,
    "mv": 1e-3,
    "kv": 1e3,
    "a": 1.0,
    "ma": 1e-3,
    "ua": 1e-6,
    "na": 1e-9,
    "s": 1.0,
    "ms": 1e-3,
    "us": 1e-6,
    "ns": 1e-9,
    "f": 1.0,
    "mf": 1e-3,
    "uf": 1e-6,
    "nf": 1e-9,
    "pf": 1e-12,
    "h": 1.0,
    "mh": 1e-3,
    "uh": 1e-6,
    "hz": 1.0,
    "khz": 1e3,
    "mhz": 1e6,
    "degc": 1.0,
    "c": 1.0,
    "m": 1.0,
    "mm": 1e-3,
    "mm2": 1e-6,
    "w": 1.0,
    "mw": 1e-3,
    "w/m2k": 1.0,
}

_ALIASES = {
    "V": "v",
    "mV": "mv",
    "kV": "kv",
    "A": "a",
    "mA": "ma",
    "uA": "ua",
    "nA": "na",
    "S": "s",
    "mS": "ms",
    "uS": "us",
    "F": "f",
    "mF": "mf",
    "uF": "uf",
    "nF": "nf",
    "pF": "pf",
    "H": "h",
    "mH": "mh",
    "uH": "uh",
    "Hz": "hz",
    "kHz": "khz",
    "MHz": "mhz",
    "Ohm": "ohm",
    "ohms": "ohm",
    "kOhm": "kohm",
    "MOhm": "megohm",
    "mOhm": "mohm",
    "degC": "degc",
    "°C": "degc",
    "W": "w",
    "mW": "mw",
}

# A component kind may only use units that describe its own quantity.
COMPATIBLE = {
    "resistor": {"ohm", "kohm", "megohm", "mohm"},
    "vsource": {"v", "mv", "kv"},
    "isource": {"a", "ma", "ua", "na"},
    "vcvs": {"1"},
    "vccs": {"s", "ms", "us"},
    "capacitor": {"f", "mf", "uf", "nf", "pf"},
    "inductor": {"h", "mh", "uh"},
}


def normalise(unit: str) -> str:
    if not isinstance(unit, str) or not unit.strip():
        raise InputError("value is missing a unit")
    raw = unit.strip()
    raw = raw.replace("Ω", "ohm").replace("µ", "u").replace("μ", "u")
    if raw in _ALIASES:
        raw = _ALIASES[raw]
    key = raw.lower()
    if key not in _SCALE:
        raise InputError(f"unknown unit {unit!r}; accepted units are {sorted(_SCALE)}")
    return key


def to_si(value, unit: str, kind: str | None = None) -> tuple[float, str]:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise InputError(f"value must be a number, got {value!r}")
    number = float(value)
    if not math.isfinite(number):
        raise InputError("value must be finite")
    key = normalise(unit)
    if kind is not None and kind in COMPATIBLE and key not in COMPATIBLE[kind]:
        raise InputError(f"{kind} cannot use unit {unit!r}; expected one of {sorted(COMPATIBLE[kind])}")
    return number * _SCALE[key], key
