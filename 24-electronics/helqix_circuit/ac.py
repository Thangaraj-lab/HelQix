"""Small-signal AC sweep and frequency metrics."""

from __future__ import annotations

import cmath
import math
import time
from dataclasses import dataclass

import numpy as np
from scipy.optimize import minimize_scalar

from .core import assemble, check_topology, node_voltage_map, solve_system
from .devices import source_phasor
from .errors import InputError, SolverError
from .netlist import Circuit


@dataclass
class AcSolution:
    frequencies: np.ndarray
    node_voltages: dict[str, np.ndarray]
    source_phasors: dict[str, complex]

    def transfer(self, source_id: str, output_node: str) -> np.ndarray:
        return self.node_voltages[output_node] / self.source_phasors[source_id]


def _has_nonlinear(circuit: Circuit) -> bool:
    return any(component.kind in {"diode", "mosfet"} for component in circuit.components)


class AcAnalyzer:
    def __init__(self, circuit: Circuit):
        self.circuit = circuit
        self.operating_point = None
        if _has_nonlinear(circuit):
            from .mna import solve_dc

            solution = solve_dc(circuit)
            self.operating_point = {name: complex(value, 0.0) for name, value in solution.node_voltages.items()}
            for name, value in (solution.internal_node_voltages or {}).items():
                self.operating_point[name] = complex(value, 0.0)

    def solve(self, frequency_hz: float) -> dict[str, complex]:
        if frequency_hz <= 0:
            raise InputError("AC frequency must be greater than zero")
        system = assemble(
            self.circuit,
            "ac",
            frequency_hz=frequency_hz,
            operating_point=self.operating_point,
        )
        x, _ = solve_system(system)
        return node_voltage_map(x, system)

    def transfer(self, source_id: str, output_node: str, frequency_hz: float, reference_node: str = "0") -> complex:
        component = self.circuit.component_map.get(source_id)
        if component is None or component.kind != "vsource":
            raise InputError(f"AC transfer source {source_id!r} must be a voltage source")
        if output_node not in self.circuit.node_names:
            raise InputError(f"output node {output_node!r} is not in the circuit")
        if reference_node not in self.circuit.node_names and reference_node != "0":
            raise InputError(f"reference node {reference_node!r} is not in the circuit")
        phasor = source_phasor(component)
        if abs(phasor) == 0:
            raise InputError(f"source {source_id!r} has zero AC amplitude")
        voltages = self.solve(frequency_hz)
        return (voltages.get(output_node, 0.0) - voltages.get(reference_node, 0.0)) / phasor


def _sweep_frequencies(circuit: Circuit) -> np.ndarray:
    analysis = circuit.analysis
    start = float(analysis.get("start_hz", 0.0))
    stop = float(analysis.get("stop_hz", 0.0))
    if not (math.isfinite(start) and math.isfinite(stop)) or start <= 0 or stop < start:
        raise InputError("AC analysis needs finite 0 < start_hz <= stop_hz")
    if "points" in analysis:
        points = int(analysis["points"])
        if points < 2:
            raise InputError("AC analysis needs at least 2 points")
        return np.logspace(math.log10(start), math.log10(stop), points)
    per_decade = int(analysis.get("points_per_decade", 50))
    if per_decade < 1:
        raise InputError("AC points_per_decade must be at least 1")
    decades = math.log10(stop / start)
    count = max(2, int(round(decades * per_decade)) + 1)
    return np.logspace(math.log10(start), math.log10(stop), count)


def ac_sweep(circuit: Circuit, frequencies=None, deadline: float | None = None) -> AcSolution:
    if str(circuit.analysis.get("type", "ac")).lower() != "ac":
        raise InputError("circuit analysis type is not 'ac'")
    check_topology(circuit, "ac")
    grid = np.asarray(_sweep_frequencies(circuit) if frequencies is None else frequencies, dtype=float)
    if grid.ndim != 1 or grid.size == 0 or np.any(grid <= 0) or not np.all(np.isfinite(grid)):
        raise InputError("AC frequencies must be a non-empty list of finite positive numbers")
    analyzer = AcAnalyzer(circuit)
    node_names = list(circuit.node_names)
    series = {name: [] for name in node_names}
    for frequency in grid:
        if deadline is not None and time.perf_counter() > deadline:
            raise SolverError("AC sweep exceeded its timeout; raise timeout_s or reduce the case", code="timeout")
        voltages = analyzer.solve(float(frequency))
        for name in node_names:
            series[name].append(voltages.get(name, 0.0))
    phasors = {
        component.id: source_phasor(component)
        for component in circuit.components
        if component.kind == "vsource"
    }
    return AcSolution(grid, {name: np.asarray(values, dtype=complex) for name, values in series.items()}, phasors)


def cutoff_frequency(
    circuit: Circuit,
    source_id: str,
    output_node: str,
    target_ratio: float = 1.0 / math.sqrt(2.0),
    f_min: float = 1e-3,
    f_max: float = 1e12,
    points: int = 200,
) -> float:
    if not 0 < target_ratio < 1:
        raise InputError("cutoff target_ratio must be between 0 and 1")
    analyzer = AcAnalyzer(circuit)
    grid = np.logspace(math.log10(f_min), math.log10(f_max), points)
    values = np.array([abs(analyzer.transfer(source_id, output_node, float(f))) for f in grid])
    crossing = None
    for index in range(1, len(grid)):
        if values[index - 1] >= target_ratio > values[index]:
            crossing = (grid[index - 1], grid[index])
            break
    if crossing is None:
        raise SolverError("no -3 dB like crossing found in the searched frequency range", code="no_crossover")
    low, high = crossing

    def difference(frequency: float) -> float:
        return abs(analyzer.transfer(source_id, output_node, float(frequency))) - target_ratio

    if difference(low) < 0:
        low, high = high, low
    for _ in range(80):
        middle = math.sqrt(low * high)
        if difference(middle) > 0:
            low = middle
        else:
            high = middle
    return math.sqrt(low * high)


def resonance_metrics(
    circuit: Circuit,
    source_id: str,
    output_node: str,
    f_min: float,
    f_max: float,
    points_per_decade: int = 200,
    reference_node: str = "0",
) -> dict:
    if f_min <= 0 or f_max <= f_min:
        raise InputError("resonance search needs 0 < f_min < f_max")
    analyzer = AcAnalyzer(circuit)
    decades = math.log10(f_max / f_min)
    count = max(3, int(round(decades * points_per_decade)) + 1)
    grid = np.logspace(math.log10(f_min), math.log10(f_max), count)

    def magnitude(frequency: float) -> float:
        return abs(analyzer.transfer(source_id, output_node, frequency, reference_node))

    magnitudes = np.array([magnitude(float(f)) for f in grid])
    peak_index = int(np.argmax(magnitudes))
    lower = grid[max(0, peak_index - 1)]
    upper = grid[min(len(grid) - 1, peak_index + 1)]

    def negative_magnitude(log_frequency: float) -> float:
        return -magnitude(float(10.0 ** log_frequency))

    optimum = minimize_scalar(
        negative_magnitude,
        bounds=(math.log10(lower), math.log10(upper)),
        method="bounded",
        options={"xatol": 1e-11},
    )
    f_peak = float(10.0 ** optimum.x)
    peak = magnitude(f_peak)
    target = peak / math.sqrt(2.0)
    if magnitude(f_min) >= target or magnitude(f_max) >= target:
        raise SolverError("resonance search range does not contain both half-power points", code="no_crossover")

    def bisect(low: float, high: float) -> float:
        # low is below the half-power target, high is above it
        for _ in range(80):
            middle = math.sqrt(low * high)
            if magnitude(middle) > target:
                high = middle
            else:
                low = middle
        return math.sqrt(low * high)

    f_low = bisect(f_min, f_peak)
    f_high = bisect(f_max, f_peak)
    return {
        "f_peak_hz": f_peak,
        "magnitude": peak,
        "f_low_hz": f_low,
        "f_high_hz": f_high,
        "bandwidth_hz": f_high - f_low,
        "q": f_peak / (f_high - f_low),
    }
