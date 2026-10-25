"""Calibration / held-out split, fixed and hashed before any fitting.

The protocol (docs/PHASE1_INPUTS.md): C/2, 1C and 1.5C discharges calibrate (the published
Chen2020 parameters were already tuned on them); C/10 discharges are held out. Hashing follows
the same specification as helqix-verify (Rust) so the trust layer can re-check the split.
Fitting code must take a CalibrationSet, which can only be built from calibration cases.
"""
from __future__ import annotations

import hashlib
import struct
from dataclasses import dataclass

PROTOCOL_VERSION = "helqix.battery-split.v1"
CALIBRATION_RATES = ("c2", "1c", "1c5")
VALIDATION_RATES = ("c10",)
_TOKEN = object()


class SplitViolation(RuntimeError):
    """Held-out data was about to be used for fitting, or a case does not fit the protocol."""


def _f64s(values) -> bytes:
    return b"".join(struct.pack("<d", float(v)) for v in values)


def data_sha256(time_s, current_a, voltage_v) -> str:
    payload = b"helqix.case-data.v1\n" + b"t" + _f64s(time_s) + b"i" + _f64s(current_a) + b"v" + _f64s(voltage_v)
    return hashlib.sha256(payload).hexdigest()


def split_sha256(entries) -> str:
    """entries: iterable of (case_id, role, data_sha256)."""
    text = "helqix.split.v1\n" + "".join(f"{cid}\t{role}\t{digest}\n" for cid, role, digest in sorted(entries))
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def case_digest(case) -> str:
    return data_sha256(case.time_s, case.current_a, case.voltage_v)


@dataclass(frozen=True)
class Split:
    calibration: tuple
    validation: tuple
    sha256: str

    def role_of(self, case_id: str) -> str:
        if any(c.id == case_id for c in self.calibration):
            return "calibration"
        if any(c.id == case_id for c in self.validation):
            return "validation"
        raise KeyError(case_id)

    def calibration_set(self) -> "CalibrationSet":
        return CalibrationSet(self, list(self.calibration), token=_TOKEN)


class CalibrationSet:
    """The only thing fitting code may receive. Cannot contain a held-out case."""

    def __init__(self, split: Split, cases, token):
        if token is not _TOKEN:
            raise SplitViolation("a CalibrationSet can only be created by Split.calibration_set()")
        held_out = {c.id for c in split.validation}
        if any(c.id in held_out for c in cases):
            raise SplitViolation("held-out cases cannot be used for calibration")
        self.cases = tuple(cases)
        self.split_sha256 = split.sha256


def require_calibration_set(candidate) -> CalibrationSet:
    if not isinstance(candidate, CalibrationSet):
        raise SplitViolation("fitting accepts only a CalibrationSet built from the fixed split")
    return candidate


def make_split(cases) -> Split:
    calibration, validation = [], []
    for case in sorted(cases, key=lambda c: c.id):
        if case.rate in VALIDATION_RATES:
            validation.append(case)
        elif case.rate in CALIBRATION_RATES:
            calibration.append(case)
        else:
            raise SplitViolation(f"case {case.id} has rate {case.rate!r}, which the protocol does not assign")
    entries = [(c.id, "calibration", case_digest(c)) for c in calibration] + [(c.id, "validation", case_digest(c)) for c in validation]
    return Split(tuple(calibration), tuple(validation), split_sha256(entries))
