"""FMU runner for the HELQIX FMI adapter.

stdin  : {"schema": "helqix.fmi-input.v1", "fmu_path": "<absolute .fmu>", "outputs": ["x", ...],
          "start_time": 0, "stop_time": 10, "step_size": 0.1, "parameters": {"k": 1.0},
          "parameters_si": {"g": {"value_si": -9.81, "dimension": [1, 0, -2, 0, 0, 0, 0]}}}
          (start/stop/step default to the FMU's DefaultExperiment)
          `parameters` are in the FMU's own units. `parameters_si` are coherent SI values with the exponents of their
          dimension (length, mass, time, current, temperature, substance, luminous intensity): the tool reads the unit the
          FMU declares for that variable, refuses (exit 2) when it measures another dimension, and converts to it.
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


DIMENSIONS = ("[length]", "[mass]", "[time]", "[current]", "[temperature]", "[substance]", "[luminosity]")


def unit_factor_and_dimension(unit: str, md=None):
    """(factor to coherent SI, exponents of the dimension) of a unit the FMU declares. Offset units (degC) are refused.

    The FMU's own UnitDefinitions are authoritative (FMI writes `m/s2`, which Pint cannot parse); Pint is the fallback
    for a plain unit the FMU does not define. Angles (rad) are dimensionless, as in the HELQIX unit registry."""
    if unit in ("1", ""):
        return 1.0, [0] * 7
    for u in (getattr(md, "unitDefinitions", None) or []):
        if u.name == unit and u.baseUnit is not None:
            b = u.baseUnit
            if (b.offset or 0.0) != 0.0:
                raise ValueError(f"the FMU declares the offset unit '{unit}'; only multiplicative units can be converted")
            return float(b.factor or 1.0), [int(b.m), int(b.kg), int(b.s), int(b.A), int(b.K), int(b.mol), int(b.cd)]
    try:
        import pint
    except ImportError:
        raise ValueError(f"the FMU does not define unit '{unit}' and Pint is not installed to read it")

    ureg = pint.UnitRegistry()
    try:
        q = ureg.Quantity(1.0, unit)
        zero = ureg.Quantity(0.0, unit).to_base_units().magnitude
    except Exception as e:
        raise ValueError(f"the FMU declares unit '{unit}', which is not a known unit ({type(e).__name__})")
    if zero != 0:
        raise ValueError(f"the FMU declares the offset unit '{unit}'; only multiplicative units can be converted")
    base = q.to_base_units()
    dims = base.dimensionality
    unknown = [k for k in dims if k not in DIMENSIONS]
    if unknown:
        raise ValueError(f"unit '{unit}' has an unsupported dimension {unknown}")
    return float(base.magnitude), [int(dims.get(k, 0)) for k in DIMENSIONS]


def si_start_values(spec, by_name, md) -> dict:
    """Converts `parameters_si` to start values in the FMU's units, or fails with exit 2."""
    if not isinstance(spec, dict):
        fail(2, "parameters_si must be an object")
    out = {}
    for name, item in spec.items():
        if (not isinstance(item, dict) or set(item) != {"value_si", "dimension"}
                or not isinstance(item["value_si"], (int, float)) or isinstance(item["value_si"], bool) or not math.isfinite(item["value_si"])
                or not isinstance(item["dimension"], list) or len(item["dimension"]) != 7
                or not all(isinstance(d, int) and not isinstance(d, bool) for d in item["dimension"])):
            fail(2, f"parameters_si.{name} must be {{value_si: finite number, dimension: 7 integers}}")
        if name not in by_name:
            fail(2, f"unknown variable {name}")
        unit = unit_of(md, by_name[name])
        try:
            factor, dimension = unit_factor_and_dimension(unit, md)
        except ValueError as e:
            fail(2, f"parameter {name}: {e}")
        if dimension != item["dimension"]:
            fail(2, f"parameter {name}: the FMU declares unit '{unit}' (dimension {dimension}) but the model gave dimension {item['dimension']}")
        out[name] = float(item["value_si"]) / factor
    return out


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

    si = doc.get("parameters_si", {})

    import fmpy
    from fmpy import read_model_description, simulate_fmu

    md = read_model_description(str(path))
    by_name = {v.name: v for v in md.modelVariables}
    for name in list(outputs) + list(params):
        if name not in by_name:
            fail(2, f"unknown variable {name}")
    converted = si_start_values(si, by_name, md)
    clash = sorted(set(converted) & set(params))
    if clash:
        fail(2, f"parameters and parameters_si both set {clash}")
    params = {**params, **converted}
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
