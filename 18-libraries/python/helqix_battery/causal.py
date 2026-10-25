"""Battery reference scenario for failure investigation (Phase 3).

Exports PyBaMM SPM signals for a constant-current discharge and the SPM's *declared* structure
(which quantity enters which equation) as `helqix.investigation-input.v1` for `helqix-investigate`.
The relations list every input of each SPM equation; it is the model's structure, not the
expected answer. The requirement is the 2.5 V lower cut-off of the test protocol in the
Chen et al. 2020 dataset (docs/PHASE1_INPUTS.md).
"""
from __future__ import annotations

from . import model

CUTOFF_V = 2.5  # test protocol lower cut-off (dataset description / LG M50 datasheet)

SIGNALS = {
    "current": "Current [A]",
    "neg_surface_stoichiometry": "X-averaged negative particle surface stoichiometry",
    "pos_surface_stoichiometry": "X-averaged positive particle surface stoichiometry",
    "neg_open_circuit_potential": "X-averaged negative electrode open-circuit potential [V]",
    "pos_open_circuit_potential": "X-averaged positive electrode open-circuit potential [V]",
    "neg_reaction_overpotential": "X-averaged negative electrode reaction overpotential [V]",
    "pos_reaction_overpotential": "X-averaged positive electrode reaction overpotential [V]",
    "voltage": "Voltage [V]",
}

# (cause, effect, relation id, parameters read from the parameter set)
RELATIONS = [
    ("current", "neg_surface_stoichiometry", "spm_negative_particle_diffusion", ["Negative particle diffusivity [m2.s-1]", "Negative particle radius [m]"]),
    ("current", "pos_surface_stoichiometry", "spm_positive_particle_diffusion", ["Positive particle diffusivity [m2.s-1]", "Positive particle radius [m]"]),
    ("neg_surface_stoichiometry", "neg_open_circuit_potential", "negative_open_circuit_potential_curve", []),
    ("pos_surface_stoichiometry", "pos_open_circuit_potential", "positive_open_circuit_potential_curve", []),
    ("current", "neg_reaction_overpotential", "negative_butler_volmer", []),
    ("current", "pos_reaction_overpotential", "positive_butler_volmer", []),
    ("neg_surface_stoichiometry", "neg_reaction_overpotential", "negative_exchange_current_density", []),
    ("pos_surface_stoichiometry", "pos_reaction_overpotential", "positive_exchange_current_density", []),
    ("pos_open_circuit_potential", "voltage", "spm_terminal_voltage", []),
    ("neg_open_circuit_potential", "voltage", "spm_terminal_voltage", []),
    ("pos_reaction_overpotential", "voltage", "spm_terminal_voltage", []),
    ("neg_reaction_overpotential", "voltage", "spm_terminal_voltage", []),
    ("current", "voltage", "contact_resistance_drop", ["Contact resistance [Ohm]"]),
]

METHOD = {"min_relative_change": 0.05, "onset_fraction": 0.1, "path_limit": 256}


def _parameter_values(config: model.ModelConfig) -> dict:
    import pybamm
    values = pybamm.ParameterValues(model.PARAMETER_SET)
    values.update({"Contact resistance [Ohm]": config.contact_resistance_ohm}, check_already_exists=False)
    return values


def reference_input(current_a: float, seconds: int, step_s: int, initial_ocv_v: float, config: model.ModelConfig) -> dict:
    times = [float(t) for t in range(0, seconds + 1, step_s)]
    solution = model.solve(times, [current_a] * len(times), initial_ocv_v, config)
    nodes = [{"id": node, "values": [float(v) for v in solution[var].entries]} for node, var in SIGNALS.items()]
    values = _parameter_values(config)
    relations = []
    for cause, effect, relation, parameters in RELATIONS:
        params = {}
        for name in parameters:
            value = values[name]
            if isinstance(value, (int, float)):
                params[name] = float(value)
        relations.append({"cause": cause, "effect": effect, "relation": relation, "parameters": params})
    return {
        "schema": "helqix.investigation-input.v1",
        "times_seconds": times,
        "nodes": nodes,
        "relations": relations,
        "requirement": {"id": "discharge_cutoff", "node": "voltage", "comparator": "at_least", "threshold": CUTOFF_V},
        "method": METHOD,
        "state_reference": None,
        "scenario": {"current_a": current_a, "initial_ocv_v": initial_ocv_v, "model": model.identity(config)},
    }
