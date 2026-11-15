"""helqix_circuit — offline DC, transient and small-signal circuit analysis."""

from .ac import AcAnalyzer, AcSolution, ac_sweep, cutoff_frequency, resonance_metrics
from .errors import HelqixError, InputError, SolverError
from .harness import HarnessReport, analyze_harness, awg_diameter_mm
from .mna import DcSolution, solve_dc
from .netlist import Circuit, Component, load_circuit
from .transient import TransientSolution, transient

PACKAGE_NAME = "helqix_circuit"
__version__ = "0.1.0"

__all__ = [
    "AcAnalyzer",
    "AcSolution",
    "Circuit",
    "Component",
    "DcSolution",
    "HarnessReport",
    "HelqixError",
    "InputError",
    "SolverError",
    "TransientSolution",
    "ac_sweep",
    "analyze_harness",
    "awg_diameter_mm",
    "cutoff_frequency",
    "load_circuit",
    "resonance_metrics",
    "solve_dc",
    "transient",
]
