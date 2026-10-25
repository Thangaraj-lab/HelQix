"""Turns cycler steps into discharge cases: constant-current discharge plus the following rest.

A discharge counts only if it starts from a known state: a rest that follows a charge. Rate
labels come from the dataset description (C/10, C/2, 1C, 1.5C), which corresponds to measured
currents of 0.5, 2.5, 5 and 7.5 A, i.e. a 5 Ah reference capacity.
"""
from __future__ import annotations

from dataclasses import dataclass

REFERENCE_CAPACITY_AH = 5.0
RATE_LABELS = {0.1: "c10", 0.5: "c2", 1.0: "1c", 1.5: "1c5"}
RATE_TOLERANCE = 0.02  # relative; the cycler holds current far tighter than this
REST_MODES = {"R", "SBr"}


@dataclass
class Case:
    id: str
    cell: str
    rate: str
    time_s: list
    current_a: list
    voltage_v: list
    discharge_samples: int
    initial_rest_voltage_v: float
    dropped_samples: int = 0


def _rate_label(current_a: float) -> str | None:
    c_rate = current_a / REFERENCE_CAPACITY_AH
    for nominal, label in RATE_LABELS.items():
        if abs(c_rate - nominal) <= RATE_TOLERANCE * nominal:
            return label
    return None


def extract_cases(cell: str, steps) -> tuple[list[Case], list[tuple[str, str]]]:
    extracted, excluded = [], []
    for index, step in enumerate(steps):
        if step.mode != "D":
            continue
        where = f"{cell} step {step.number}"
        previous = steps[index - 1] if index >= 1 else None
        before = steps[index - 2] if index >= 2 else None
        if previous is None or previous.mode not in REST_MODES or before is None or before.mode != "C":
            excluded.append((where, "discharge is not preceded by a charge then a rest, so its starting state is unknown"))
            continue
        mean_current = sum(step.current_a) / len(step.current_a)
        label = _rate_label(mean_current)
        if label is None:
            excluded.append((where, f"mean current {mean_current:.3g} A matches no documented rate"))
            continue
        rest = []
        for following in steps[index + 1:]:
            if following.mode not in REST_MODES:
                break
            rest.append(following)
        t0 = step.time_s[0]
        time_s = [t - t0 for t in step.time_s]
        current = list(step.current_a)
        voltage = list(step.voltage_v)
        for segment in rest:
            time_s += [t - t0 for t in segment.time_s]
            current += segment.current_a
            voltage += segment.voltage_v
        kept_t, kept_i, kept_v, dropped = [], [], [], 0
        for t, i, v in zip(time_s, current, voltage):
            if kept_t and t <= kept_t[-1]:
                dropped += 1
                continue
            kept_t.append(t); kept_i.append(i); kept_v.append(v)
        extracted.append(Case(id=f"{cell}_{label}", cell=cell, rate=label, time_s=kept_t, current_a=kept_i, voltage_v=kept_v,
                              discharge_samples=len(step.time_s), initial_rest_voltage_v=previous.voltage_v[-1], dropped_samples=dropped))
    return extracted, excluded
