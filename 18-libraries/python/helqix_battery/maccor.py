"""Reader for Maccor cycler CSV exports (as published in the Chen2020 dataset)."""
from __future__ import annotations

import csv
import math
from dataclasses import dataclass, field
from pathlib import Path

REQUIRED = ("Step", "Test Time [s]", "Current [A]", "Voltage [V]", "Md")


class MaccorFormatError(ValueError):
    """The file does not have the expected Maccor layout or contains unparseable values."""


@dataclass
class Step:
    """A run of consecutive samples sharing one step number and one mode (R rest, C charge, D discharge)."""
    number: int
    mode: str
    time_s: list = field(default_factory=list)
    current_a: list = field(default_factory=list)
    voltage_v: list = field(default_factory=list)


def _number(text: str, row: int, column: str) -> float:
    try:
        value = float(text)
    except ValueError:
        raise MaccorFormatError(f"row {row}: {column} is not a number: {text!r}") from None
    if not math.isfinite(value):
        raise MaccorFormatError(f"row {row}: {column} is not finite")
    return value


def read_maccor_csv(path: Path) -> list[Step]:
    with open(path, newline="", encoding="utf-8", errors="strict") as handle:
        rows = list(csv.reader(handle))
    header_index = next((i for i, r in enumerate(rows) if r and r[0] == "Rec"), None)
    if header_index is None:
        raise MaccorFormatError("no 'Rec' header row found")
    header = rows[header_index]
    missing = [name for name in REQUIRED if name not in header]
    if missing:
        raise MaccorFormatError(f"missing columns: {', '.join(missing)}")
    col = {name: header.index(name) for name in REQUIRED}
    steps: list[Step] = []
    for offset, row in enumerate(rows[header_index + 1:], start=header_index + 2):
        if not row or not row[0].strip():
            continue
        if len(row) < len(header):
            raise MaccorFormatError(f"row {offset}: expected {len(header)} columns, found {len(row)}")
        number = int(_number(row[col["Step"]], offset, "Step"))
        mode = row[col["Md"]].strip()
        if not steps or steps[-1].number != number or steps[-1].mode != mode:
            steps.append(Step(number, mode))
        step = steps[-1]
        step.time_s.append(_number(row[col["Test Time [s]"]], offset, "Test Time [s]"))
        step.current_a.append(_number(row[col["Current [A]"]], offset, "Current [A]"))
        step.voltage_v.append(_number(row[col["Voltage [V]"]], offset, "Voltage [V]"))
    return steps
