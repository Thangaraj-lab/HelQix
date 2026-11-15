"""helqix_circuit — offline DC, transient and small-signal circuit analysis."""

from .ac import AcAnalyzer, AcSolution, ac_sweep, cutoff_frequency, resonance_metrics
from .errors import HelqixError, InputError, SolverError
from .harness import HarnessReport, analyze_harness, awg_diameter_mm
from .harness_doc import HarnessDoc, HarnessWire, colour_entry, load_harness
from .link import Fault, link_harness, parse_fault
from .mna import DcSolution, solve_dc
from .netlist import Circuit, Component, load_circuit
from .routing import Box, Route, route
from .transient import TransientSolution, transient

PACKAGE_NAME = "helqix_circuit"
__version__ = "0.1.0"

__all__ = [
    "AcAnalyzer",
    "AcSolution",
    "Box",
    "Circuit",
    "Component",
    "DcSolution",
    "Fault",
    "HarnessDoc",
    "HarnessReport",
    "HarnessWire",
    "HelqixError",
    "InputError",
    "Route",
    "SolverError",
    "TransientSolution",
    "ac_sweep",
    "analyze_harness",
    "awg_diameter_mm",
    "colour_entry",
    "cutoff_frequency",
    "link_harness",
    "load_circuit",
    "load_harness",
    "parse_fault",
    "resonance_metrics",
    "route",
    "solve_dc",
    "transient",
]
