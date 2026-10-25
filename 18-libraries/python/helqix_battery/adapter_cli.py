"""PyBaMM tool entry point for the Rust solver adapter (Phase 6).

Reads one JSON document (`helqix.pybamm-input.v1`) on stdin and writes one JSON document
(`helqix.pybamm-output.v1`) on stdout. The Rust side treats everything here as untrusted.
Exit codes: 0 ok, 2 invalid input, 3 simulation did not complete.
"""
from __future__ import annotations

import json
import math
import sys

from . import model


def _fail(code: int, message: str) -> None:
    sys.stderr.write(message[:2000] + "\n")
    raise SystemExit(code)


def _series(doc: dict, key: str) -> list:
    values = doc.get(key)
    if not isinstance(values, list) or not values or not all(isinstance(v, (int, float)) and math.isfinite(v) for v in values):
        _fail(2, f"{key} must be a non-empty list of finite numbers")
    return [float(v) for v in values]


def run(doc: dict) -> dict:
    if doc.get("schema") != "helqix.pybamm-input.v1":
        _fail(2, "schema must be helqix.pybamm-input.v1")
    time_s = _series(doc, "time_s")
    current_a = _series(doc, "current_a")
    if len(time_s) != len(current_a) or any(b <= a for a, b in zip(time_s, time_s[1:])):
        _fail(2, "time_s must be strictly increasing and as long as current_a")
    ocv = doc.get("initial_ocv_v")
    resistance = doc.get("contact_resistance_ohm", 0.0)
    if not isinstance(ocv, (int, float)) or not (2.5 <= ocv <= 4.25):
        _fail(2, "initial_ocv_v must be a number in [2.5, 4.25] V")
    if not isinstance(resistance, (int, float)) or not (0.0 <= resistance <= 1.0):
        _fail(2, "contact_resistance_ohm must be in [0, 1] Ohm")
    config = model.ModelConfig(contact_resistance_ohm=float(resistance))
    try:
        solution = model.solve(time_s, current_a, float(ocv), config)
    except model.SimulationIncomplete as error:
        _fail(3, f"simulation did not complete: {error}")
    import pybamm
    return {
        "schema": "helqix.pybamm-output.v1",
        "tool": "pybamm",
        "tool_version": pybamm.__version__,
        "model_identity_sha256": model.identity(config)["sha256"],
        "outputs": [
            {"name": "terminal_voltage", "unit": "V", "values": [float(v) for v in solution["Voltage [V]"].entries]},
            {"name": "discharge_capacity", "unit": "A.h", "values": [float(v) for v in solution["Discharge capacity [A.h]"].entries]},
        ],
    }


def main() -> None:
    try:
        doc = json.loads(sys.stdin.read())
    except json.JSONDecodeError as error:
        _fail(2, f"input is not JSON: {error}")
    if not isinstance(doc, dict):
        _fail(2, "input must be a JSON object")
    sys.stdout.write(json.dumps(run(doc)))
    sys.stdout.flush()


if __name__ == "__main__":
    main()
