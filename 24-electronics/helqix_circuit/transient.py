"""Transient analysis with backward Euler or trapezoidal integration."""

from __future__ import annotations

import math
from dataclasses import dataclass

import numpy as np

from .core import (
    State,
    assemble,
    check_topology,
    component_currents,
    node_voltage_map,
    powers,
    solve_system,
)
from .errors import InputError, SolverError
from .netlist import Circuit


@dataclass
class TransientSolution:
    times: np.ndarray
    node_voltages: dict[str, np.ndarray]
    branch_currents: dict[str, np.ndarray]
    report: dict
    method: str

    def value_at(self, node: str, time_s: float) -> float:
        if node not in self.node_voltages:
            raise InputError(f"node {node!r} is not in the transient result")
        return float(np.interp(time_s, self.times, self.node_voltages[node]))


def _method(name: str) -> str:
    key = name.lower().replace("-", "_")
    if key in {"trap", "trapezoidal"}:
        return "trapezoidal"
    if key in {"be", "backward_euler"}:
        return "backward_euler"
    raise InputError(f"unknown transient method {name!r}; use 'trapezoidal' or 'backward_euler'")


def _initial_operating_point(circuit: Circuit):
    check_topology(circuit, "ic")
    system = assemble(circuit, "ic", time_s=0.0)
    x, report = solve_system(system)
    voltages = node_voltage_map(x, system)
    currents = component_currents(circuit, x, system)
    state = State()
    for component in circuit.components:
        if component.kind == "capacitor":
            state.cap_v[component.id] = component.initial or 0.0
            state.cap_i[component.id] = currents.get(component.id, 0.0)
        elif component.kind == "inductor":
            state.ind_i[component.id] = component.initial or 0.0
            state.ind_v[component.id] = float(np.real(voltages.get(component.nodes[0], 0.0) - voltages.get(component.nodes[1], 0.0)))
    return state, _transient_start_vector(circuit, state, voltages), voltages, currents, report


def _transient_start_vector(circuit: Circuit, state: State, voltages: dict) -> np.ndarray:
    transient_system = assemble(circuit, "tran", state=state, time_s=0.0, step_s=1.0, method="trapezoidal")
    x0 = np.zeros(transient_system.size)
    for name, index in transient_system.node_index.items():
        x0[index] = float(np.real(voltages.get(name, 0.0)))
    for component in circuit.components:
        if component.kind == "inductor":
            x0[transient_system.branch_index[component.id] + len(transient_system.node_index)] = state.ind_i.get(
                component.id, component.initial or 0.0
            )
    return x0


def _restart_at_breakpoint(circuit: Circuit, state: State, time_s: float):
    """Restart the integration state after a source jump.

    Capacitor voltages and inductor currents cannot jump, so they are held from
    the previous state; the algebraic unknowns are resolved with the waveform
    value just after the breakpoint.
    """
    check_topology(circuit, "ic")
    system = assemble(circuit, "ic", state=state, time_s=time_s)
    x, report = solve_system(system)
    voltages = node_voltage_map(x, system)
    currents = component_currents(circuit, x, system)
    new_state = State()
    for component in circuit.components:
        if component.kind == "capacitor":
            new_state.cap_v[component.id] = state.cap_v.get(component.id, component.initial or 0.0)
            new_state.cap_i[component.id] = currents.get(component.id, 0.0)
        elif component.kind == "inductor":
            new_state.ind_i[component.id] = state.ind_i.get(component.id, component.initial or 0.0)
            new_state.ind_v[component.id] = float(np.real(voltages.get(component.nodes[0], 0.0) - voltages.get(component.nodes[1], 0.0)))
    return new_state, _transient_start_vector(circuit, new_state, voltages), voltages, currents, report


def _breakpoints(circuit: Circuit, stop_s: float) -> list[float]:
    """Times inside (0, stop_s) where a source waveform jumps or kinks."""
    points: list[float] = []
    for component in circuit.components:
        waveform = component.waveform
        if not waveform:
            continue
        if waveform["type"] == "step":
            edge = waveform["delay_s"]
            if 0.0 < edge < stop_s:
                points.append(edge)
        elif waveform["type"] == "pulse":
            delay = waveform["delay_s"]
            period = waveform["period_s"]
            duty = waveform["duty"]
            index = 0
            while delay + index * period < stop_s:
                for edge in (delay + index * period, delay + (index + duty) * period):
                    if 0.0 < edge < stop_s:
                        points.append(edge)
                index += 1
                if len(points) > 20000:
                    raise InputError("waveform has more than 20000 edges before stop_s; shorten stop_s or lower the pulse rate")
    return sorted(set(points))


def _take_step(circuit: Circuit, state: State, t0: float, h: float, x0: np.ndarray, method: str, max_newton: int,
               source_time: float | None = None):
    system = assemble(circuit, "tran", state=state, time_s=source_time if source_time is not None else t0 + h,
                      method=method, step_s=h)
    x, report = solve_system(system, x0, max_iterations=max_newton)
    voltages = node_voltage_map(x, system)
    new_state = state.copy()
    for component in circuit.components:
        if component.kind == "capacitor":
            v_new = float(np.real(voltages.get(component.nodes[0], 0.0) - voltages.get(component.nodes[1], 0.0)))
            v_old = state.cap_v.get(component.id, component.initial or 0.0)
            if method == "backward_euler":
                i_new = component.value * (v_new - v_old) / h
            else:
                i_old = state.cap_i.get(component.id, 0.0)
                i_new = 2.0 * component.value * (v_new - v_old) / h - i_old
            new_state.cap_v[component.id] = v_new
            new_state.cap_i[component.id] = i_new
        elif component.kind == "inductor":
            i_new = float(np.real(x[system.branch_index[component.id] + len(system.node_index)]))
            v_new = float(np.real(voltages.get(component.nodes[0], 0.0) - voltages.get(component.nodes[1], 0.0)))
            new_state.ind_i[component.id] = i_new
            new_state.ind_v[component.id] = v_new
    currents = component_currents(circuit, x, system, new_state)
    return x, new_state, voltages, currents, report


def _error(a: State, b: State, va: dict, vb: dict, rtol: float) -> float:
    worst = 0.0
    for mapping_a, mapping_b in ((a.cap_v, b.cap_v), (a.ind_i, b.ind_i)):
        for key in mapping_a:
            left, right = mapping_a[key], mapping_b.get(key, 0.0)
            scale = 1e-12 + rtol * max(abs(left), abs(right))
            worst = max(worst, abs(left - right) / scale)
    for key in va:
        left, right = float(np.real(va[key])), float(np.real(vb.get(key, 0.0)))
        scale = 1e-12 + rtol * max(abs(left), abs(right))
        worst = max(worst, abs(left - right) / scale)
    return worst


def transient(
    circuit: Circuit,
    *,
    stop_s: float | None = None,
    step_s: float | None = None,
    method: str | None = None,
    adaptive: bool | None = None,
    rtol: float | None = None,
    max_step_s: float | None = None,
    min_step_s: float | None = None,
    max_steps: int | None = None,
    max_newton_iterations: int = 60,
) -> TransientSolution:
    analysis = circuit.analysis
    stop = float(analysis.get("stop_s", 0.0) if stop_s is None else stop_s)
    step = float(analysis.get("step_s", stop / 100.0 if stop > 0 else 0.0) if step_s is None else step_s)
    method = _method(str(analysis.get("method", "trapezoidal") if method is None else method))
    adaptive = bool(analysis.get("adaptive", True) if adaptive is None else adaptive)
    rtol = float(analysis.get("rtol", 1e-4) if rtol is None else rtol)
    max_step = float(analysis.get("max_step_s", step) if max_step_s is None else max_step_s)
    min_step = float(analysis.get("min_step_s", max(step * 1e-6, 1e-15)) if min_step_s is None else min_step_s)
    max_steps_raw = analysis.get("max_steps", 200000) if max_steps is None else max_steps
    if not all(math.isfinite(value) for value in (stop, step, rtol, max_step, min_step)):
        raise InputError("transient stop_s, step_s, rtol, min_step_s and max_step_s must be finite")
    if stop <= 0 or step <= 0:
        raise InputError("transient analysis needs stop_s > 0 and step_s > 0")
    if max_step <= 0 or min_step <= 0 or min_step > max_step:
        raise InputError("transient needs 0 < min_step_s <= max_step_s")
    if not 0 < rtol < 1:
        raise InputError("transient rtol must be between 0 and 1")
    if isinstance(max_steps_raw, bool) or not isinstance(max_steps_raw, (int, float)):
        raise InputError("transient max_steps must be an integer greater than zero")
    if not math.isfinite(float(max_steps_raw)) or float(max_steps_raw) < 1 or float(max_steps_raw) != int(float(max_steps_raw)):
        raise InputError("transient max_steps must be an integer greater than zero")
    max_steps = int(max_steps_raw)

    state, x, voltages, currents, _ = _initial_operating_point(circuit)
    external = list(circuit.node_names)
    times = [0.0]
    node_series = {name: [float(np.real(voltages.get(name, 0.0)))] for name in external}
    current_series = {component.id: [currents.get(component.id, 0.0)] for component in circuit.components}
    breakpoints = _breakpoints(circuit, stop)
    breakpoint_index = 0

    t = 0.0
    h = min(step, max_step)
    accepted = 0
    rejected = 0
    attempts = 0
    newton_total = 0
    minimum_used = h
    maximum_used = h

    def record(t_new, voltages_new, currents_new):
        times.append(t_new)
        for name in external:
            node_series[name].append(float(np.real(voltages_new.get(name, 0.0))))
        for component in circuit.components:
            current_series[component.id].append(currents_new.get(component.id, 0.0))

    while t < stop - 1e-15:
        if attempts > max_steps * 20:
            raise SolverError(f"transient exceeded {max_steps * 20} attempts; reduce rtol or raise max_steps", code="max_steps")
        if accepted >= max_steps:
            raise SolverError(f"transient reached max_steps={max_steps} before stop_s", code="max_steps")
        h_use = min(h, stop - t)
        landing = False
        if breakpoint_index < len(breakpoints):
            distance = breakpoints[breakpoint_index] - t
            if distance <= h_use:
                h_use = distance
                landing = True
        if h_use <= 1e-15:
            breakpoint_index += 1
            continue
        left_limit = math.nextafter(t + h_use, -math.inf) if landing else None
        if adaptive and h_use / 2.0 >= min_step:
            attempts += 1
            x1, s1, v1, c1, r1 = _take_step(circuit, state, t, h_use, x, method, max_newton_iterations, left_limit)
            x_a, s_a, v_a, c_a, r_a = _take_step(circuit, state, t, h_use / 2.0, x, method, max_newton_iterations)
            x_b, s_b, v_b, c_b, r_b = _take_step(circuit, s_a, t + h_use / 2.0, h_use / 2.0, x_a, method,
                                                  max_newton_iterations, left_limit)
            newton_total += r1.iterations + r_a.iterations + r_b.iterations
            error = _error(s1, s_b, v1, v_b, rtol)
            if error <= 1.0:
                t_new = breakpoints[breakpoint_index] if landing else t + h_use
                if landing:
                    state, x, v_post, c_post, restart_report = _restart_at_breakpoint(circuit, s_b, t_new)
                    newton_total += restart_report.iterations
                    record(t_new, v_post, c_post)
                    breakpoint_index += 1
                else:
                    state, x = s_b, x_b
                    record(t_new, v_b, c_b)
                t = t_new
                accepted += 1
                minimum_used = min(minimum_used, h_use)
                maximum_used = max(maximum_used, h_use)
                if error < 0.3 and h < max_step:
                    h = min(2.0 * h, max_step)
            else:
                rejected += 1
                h = max(h_use * 0.5, min_step)
                if h <= min_step and h_use <= min_step * 1.0000001:
                    raise SolverError("transient step underflow: rtol too tight for min_step_s", code="step_underflow")
        else:
            attempts += 1
            x_new, s_new, v_new, c_new, report = _take_step(circuit, state, t, h_use, x, method, max_newton_iterations,
                                                            left_limit)
            newton_total += report.iterations
            t_new = breakpoints[breakpoint_index] if landing else t + h_use
            if landing:
                state, x, v_post, c_post, restart_report = _restart_at_breakpoint(circuit, s_new, t_new)
                newton_total += restart_report.iterations
                record(t_new, v_post, c_post)
                breakpoint_index += 1
            else:
                state, x = s_new, x_new
                record(t_new, v_new, c_new)
            t = t_new
            accepted += 1
            minimum_used = min(minimum_used, h_use)
            maximum_used = max(maximum_used, h_use)

    return TransientSolution(
        times=np.asarray(times),
        node_voltages={name: np.asarray(values) for name, values in node_series.items()},
        branch_currents={name: np.asarray(values) for name, values in current_series.items()},
        report={
            "converged": True,
            "method": method,
            "adaptive": adaptive,
            "accepted_steps": accepted,
            "rejected_steps": rejected,
            "attempts": attempts,
            "newton_iterations": newton_total,
            "rtol": rtol,
            "min_step_s": minimum_used,
            "max_step_s": maximum_used,
            "final_time_s": t,
        },
        method=method,
    )
