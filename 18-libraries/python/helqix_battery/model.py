"""PyBaMM SPM (Chen2020) driven by a measured current profile.

Output is Simulated/Unverified data. Determinism level declared for this external solver:
Tolerance (results depend on solver tolerances and mesh; see integrity.py).
"""
from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field

PARAMETER_SET = "Chen2020"
MODEL_NAME = "lithium_ion.SPM"
MODEL_OPTIONS = {"contact resistance": "true"}
LOWER_CUTOFF_V = 2.0  # below the 2.5 V test cut-off, so the model follows the measured profile
BASE_MESH = {"x_n": 20, "x_s": 20, "x_p": 20, "r_n": 20, "r_p": 20}
DETERMINISM_LEVEL = "Tolerance"


class SimulationIncomplete(RuntimeError):
    """The solver stopped before the last requested time; no partial result is returned."""


@dataclass(frozen=True)
class ModelConfig:
    contact_resistance_ohm: float = 0.0
    rtol: float = 1e-6
    atol: float = 1e-8
    mesh_scale: int = 1
    scaled_parameters: tuple = field(default_factory=tuple)  # ((name, factor), ...) for sensitivity runs


def _pybamm():
    import pybamm  # imported lazily so the data modules work without PyBaMM
    return pybamm


def identity(config: ModelConfig) -> dict:
    pybamm = _pybamm()
    ident = {
        "name": f"PyBaMM {MODEL_NAME} + {PARAMETER_SET}",
        "solver_package": "pybamm", "solver_version": pybamm.__version__,
        "model": MODEL_NAME, "options": MODEL_OPTIONS, "parameter_set": PARAMETER_SET,
        "contact_resistance_ohm": float(config.contact_resistance_ohm).hex(),
        "lower_cutoff_v": float(LOWER_CUTOFF_V).hex(),
        "rtol": float(config.rtol).hex(), "atol": float(config.atol).hex(),
        "mesh": {k: v * config.mesh_scale for k, v in BASE_MESH.items()},
        "scaled_parameters": [[n, float(f).hex()] for n, f in config.scaled_parameters],
        "current_interpolation": "linear", "thermal": "isothermal",
        "initial_state": "from measured open-circuit voltage before discharge",
        "determinism_level": DETERMINISM_LEVEL,
    }
    canonical = json.dumps(ident, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return {**ident, "sha256": hashlib.sha256(canonical).hexdigest()}


def solve(time_s, current_a, initial_ocv_v: float, config: ModelConfig):
    """Returns the PyBaMM solution evaluated at exactly the given times."""
    pybamm = _pybamm()
    import numpy as np
    t = np.asarray(time_s, dtype=float)
    i = np.asarray(current_a, dtype=float)
    values = pybamm.ParameterValues(PARAMETER_SET)
    for name, factor in config.scaled_parameters:
        values.update({name: values[name] * factor})
    values.update({
        "Current function [A]": pybamm.Interpolant(t, i, pybamm.t, interpolator="linear"),
        "Contact resistance [Ohm]": float(config.contact_resistance_ohm),
        "Lower voltage cut-off [V]": LOWER_CUTOFF_V,
    }, check_already_exists=False)
    var_pts = {getattr(pybamm.standard_spatial_vars, k): v * config.mesh_scale for k, v in BASE_MESH.items()}
    simulation = pybamm.Simulation(pybamm.lithium_ion.SPM(dict(MODEL_OPTIONS)), parameter_values=values, var_pts=var_pts,
                                   solver=pybamm.IDAKLUSolver(rtol=config.rtol, atol=config.atol))
    try:
        solution = simulation.solve(t_eval=[float(t[0]), float(t[-1])], t_interp=t, initial_soc=f"{float(initial_ocv_v)} V")
    except pybamm.SolverError as error:
        raise SimulationIncomplete(f"solver failed: {error}") from error
    if solution.t[-1] < t[-1] or len(solution["Voltage [V]"].entries) != len(t):
        raise SimulationIncomplete(f"solver stopped at {solution.t[-1]:.1f} s of {t[-1]:.1f} s ({solution.termination})")
    return solution


def simulate(time_s, current_a, initial_ocv_v: float, config: ModelConfig) -> list[float]:
    voltage = solve(time_s, current_a, initial_ocv_v, config)["Voltage [V]"].entries
    return [float(v) for v in voltage]
