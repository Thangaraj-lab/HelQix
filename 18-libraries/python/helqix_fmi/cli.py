"""FMU runner for the HELQIX FMI adapter.

stdin  : {"schema": "helqix.fmi-input.v1", "fmu_path": "<absolute .fmu>", "outputs": ["x", ...],
          "start_time": 0, "stop_time": 10, "step_size": 0.1, "parameters": {"k": 1.0}}
          (start/stop/step default to the FMU's DefaultExperiment)
stdout : {"tool_version": "fmpy <v>", "fmi_version": "2.0", "model": "<name>", "step_size": h,
          "outputs": [{"name": "time", "unit": "s", "values": [...]}, {"name": "x", "unit": "1", "values": [...]}]}
exit   : 0 ok, 2 invalid input, 3 simulation failed.

Units come from the model description (variable unit or declared type unit) and are normalized to
SI base units with Pint; a variable with no unit is reported as "1" (dimensionless). An FMU
contains native code: running one executes that code. Only run FMUs from trusted sources.
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

SCHEMA = "helqix.fmi-input.v1"


def fail(code: int, message: str) -> None:
    sys.stderr.write(message[:2000] + "\n")
    raise SystemExit(code)


def unit_of(model_description, variable) -> str:
    unit = getattr(variable, "unit", None)
    declared = getattr(variable, "declaredType", None)
    if not unit and declared is not None:
        unit = getattr(declared, "unit", None)
    return unit or "1"


def normalize(values: list[float], unit: str):
    """Converts values to SI base units with Pint. Returns (values, unit symbol)."""
    if unit in ("1", ""):
        return values, "1"
    import pint

    ureg = pint.UnitRegistry()
    try:
        q = ureg.Quantity(1.0, unit)
    except Exception:  # unknown unit: report it unchanged so the adapter's unit check rejects it
        return values, unit
    base = q.to_base_units()
    factor = float(base.magnitude)
    symbol = f"{base.units:~}".replace(" ", "").replace("**", "^") or "1"
    return [v * factor for v in values], symbol


def main() -> None:
    try:
        doc = json.loads(sys.stdin.read())
    except Exception as e:
        fail(2, f"input is not JSON: {e}")
    if not isinstance(doc, dict) or doc.get("schema") != SCHEMA:
        fail(2, f"schema must be {SCHEMA}")
    path = Path(str(doc.get("fmu_path", "")))
    if not path.is_absolute() or path.suffix != ".fmu" or not path.is_file():
        fail(2, "fmu_path must be an absolute path to an existing .fmu file")
    outputs = doc.get("outputs")
    if not isinstance(outputs, list) or not outputs or not all(isinstance(o, str) for o in outputs):
        fail(2, "outputs must be a non-empty list of variable names")
    params = doc.get("parameters", {})
    if not isinstance(params, dict) or not all(isinstance(v, (int, float)) and math.isfinite(v) for v in params.values()):
        fail(2, "parameters must map names to finite numbers")

    import fmpy
    from fmpy import read_model_description, simulate_fmu

    md = read_model_description(str(path))
    by_name = {v.name: v for v in md.modelVariables}
    for name in list(outputs) + list(params):
        if name not in by_name:
            fail(2, f"unknown variable {name}")
    exp = md.defaultExperiment
    start = float(doc.get("start_time", getattr(exp, "startTime", None) or 0.0))
    stop = float(doc.get("stop_time", getattr(exp, "stopTime", None) or 1.0))
    step = doc.get("step_size", getattr(exp, "stepSize", None))
    if step is None or not (float(step) > 0) or not (stop > start):
        fail(2, "need step_size > 0 and stop_time > start_time")
    step = float(step)
    try:
        result = simulate_fmu(str(path), fmi_type="CoSimulation", start_time=start, stop_time=stop, output_interval=step,
                              step_size=step, start_values=params, output=outputs)
    except Exception as e:
        fail(3, f"simulation failed: {type(e).__name__}: {e}")
    out = [{"name": "time", "unit": "s", "values": [float(t) for t in result["time"]]}]
    for name in outputs:
        values, unit = normalize([float(x) for x in result[name]], unit_of(md, by_name[name]))
        out.append({"name": name, "unit": unit, "values": values})
    sys.stdout.write(json.dumps({"tool_version": f"fmpy {fmpy.__version__}", "fmi_version": md.fmiVersion, "model": md.modelName,
                                 "step_size": step, "outputs": out}))


if __name__ == "__main__":
    main()
