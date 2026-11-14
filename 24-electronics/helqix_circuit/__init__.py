"""helqix_circuit — offline DC, transient and small-signal circuit analysis."""

from .errors import HelqixError, InputError, SolverError
from .mna import DcSolution, solve_dc
from .netlist import Circuit, Component, load_circuit

PACKAGE_NAME = "helqix_circuit"
__version__ = "0.1.0"

__all__ = [
    "Circuit",
    "Component",
    "DcSolution",
    "HelqixError",
    "InputError",
    "SolverError",
    "load_circuit",
    "solve_dc",
]
