"""General assembly: DC operating point, transient companions, AC stamping."""

from __future__ import annotations

import cmath
from dataclasses import dataclass, field

import numpy as np
from scipy.sparse import coo_matrix
from scipy.sparse.linalg import splu

from .devices import diode_eval, mosfet_eval, source_dc, source_phasor, source_value
from .errors import InputError, SolverError
from .netlist import Circuit, Component

INTERNAL_PREFIX = "__helqix_internal__:"


@dataclass
class NewtonReport:
    converged: bool
    iterations: int
    residual: float
    max_update: float
    damping_steps: int
    message: str = ""

    def to_dict(self) -> dict:
        return {
            "converged": self.converged,
            "iterations": self.iterations,
            "residual": self.residual,
            "max_update": self.max_update,
            "damping_steps": self.damping_steps,
            "message": self.message,
        }


@dataclass
class State:
    cap_v: dict[str, float] = field(default_factory=dict)
    cap_i: dict[str, float] = field(default_factory=dict)
    ind_i: dict[str, float] = field(default_factory=dict)
    ind_v: dict[str, float] = field(default_factory=dict)

    def copy(self) -> "State":
        return State(dict(self.cap_v), dict(self.cap_i), dict(self.ind_i), dict(self.ind_v))


@dataclass
class System:
    matrix: object
    rhs: np.ndarray
    node_index: dict[str, int]
    branch_index: dict[str, int]
    external_nodes: list[str]
    nonlinear: list[tuple[str, Component, str | None]]
    mode: str
    size: int


def internal_node(component: Component) -> str:
    return INTERNAL_PREFIX + component.id


def node_layout(circuit: Circuit) -> tuple[list[str], dict[str, int]]:
    names = list(circuit.node_names)
    for component in circuit.components:
        if component.kind == "diode" and component.params["rs_ohm"] > 0.0:
            names.append(internal_node(component))
    return names, {name: i for i, name in enumerate(names)}


def branch_components(circuit: Circuit, mode: str) -> list[Component]:
    result = []
    for component in circuit.components:
        if component.kind in {"vsource", "vcvs", "opamp"}:
            result.append(component)
        elif component.kind == "switch" and component.state == "closed":
            result.append(component)
        elif component.kind == "inductor" and mode in {"dc", "tran"}:
            result.append(component)
        elif component.kind == "capacitor" and mode == "ic":
            result.append(component)
    return result


def _edges(circuit: Circuit, mode: str) -> list[tuple[str, str]]:
    edges = []
    for component in circuit.components:
        kind = component.kind
        if kind in {"resistor", "vsource", "vcvs"}:
            edges.append((component.nodes[0], component.nodes[1]))
        elif kind == "diode":
            if component.params["rs_ohm"] > 0.0:
                edges.append((component.nodes[0], internal_node(component)))
                edges.append((internal_node(component), component.nodes[1]))
            else:
                edges.append((component.nodes[0], component.nodes[1]))
        elif kind == "switch" and component.state == "closed":
            edges.append((component.nodes[0], component.nodes[1]))
        elif kind == "inductor" and mode != "ic":
            edges.append((component.nodes[0], component.nodes[1]))
        elif kind == "capacitor" and mode in {"ic", "tran", "ac"}:
            edges.append((component.nodes[0], component.nodes[1]))
        elif kind == "mosfet":
            edges.append((component.nodes[0], component.nodes[2]))
        elif kind == "opamp":
            edges.append((component.nodes[0], component.nodes[3]))
    return edges


def check_topology(circuit: Circuit, mode: str) -> None:
    nodes = {"0"}
    for component in circuit.components:
        nodes.update(component.nodes)
    if mode in {"dc", "tran", "ac", "ic"}:
        for component in circuit.components:
            if component.kind == "diode" and component.params["rs_ohm"] > 0.0:
                nodes.add(internal_node(component))
    adjacency: dict[str, set[str]] = {name: set() for name in nodes}
    for a, b in _edges(circuit, mode):
        adjacency[a].add(b)
        adjacency[b].add(a)
    seen = {"0"}
    stack = ["0"]
    while stack:
        name = stack.pop()
        for neighbour in adjacency[name]:
            if neighbour not in seen:
                seen.add(neighbour)
                stack.append(neighbour)
    floating = sorted(nodes - seen)
    if floating:
        raise InputError(
            "floating node(s) " + ", ".join(repr(name) for name in floating)
            + f": no {mode.upper()} path to ground. Current sources, open switches, op-amp inputs "
            "and (in DC) capacitors do not provide a path.",
            code="floating_node",
        )

    fixed: list[tuple[str, str, str]] = []
    for component in circuit.components:
        if component.kind == "vsource":
            fixed.append((component.nodes[0], component.nodes[1], component.id))
        elif component.kind == "switch" and component.state == "closed":
            fixed.append((component.nodes[0], component.nodes[1], component.id))
        elif component.kind == "inductor" and mode == "dc":
            fixed.append((component.nodes[0], component.nodes[1], component.id))
    parent: dict[str, str] = {}
    members: dict[str, list[str]] = {}

    def find(name: str) -> str:
        parent.setdefault(name, name)
        while parent[name] != name:
            parent[name] = parent[parent[name]]
            name = parent[name]
        return name

    for a, b, label in fixed:
        ra, rb = find(a), find(b)
        if ra == rb:
            names = ", ".join(dict.fromkeys(members.get(ra, []) + [label]))
            raise InputError(
                f"voltage-source loop: {names} form a closed loop of ideal voltage sources "
                "(including closed switches and, in DC, inductors). Such a loop has no unique solution.",
                code="voltage_source_loop",
            )
        parent[rb] = ra
        members[ra] = members.get(ra, []) + members.get(rb, []) + [label]
        members.pop(rb, None)


class _Builder:
    def __init__(self, size: int, complex_mode: bool):
        self.size = size
        self.rows: list[int] = []
        self.cols: list[int] = []
        self.data: list[complex | float] = []
        dtype = complex if complex_mode else float
        self.rhs = np.zeros(size, dtype=dtype)

    def add(self, row: int | None, col: int | None, value) -> None:
        if row is None or col is None or value == 0:
            return
        self.rows.append(row)
        self.cols.append(col)
        self.data.append(value)

    def matrix(self):
        matrix = coo_matrix((self.data, (self.rows, self.cols)), shape=(self.size, self.size)).tocsc()
        matrix.sum_duplicates()
        return matrix


def assemble(
    circuit: Circuit,
    mode: str,
    *,
    state: State | None = None,
    time_s: float = 0.0,
    method: str = "trapezoidal",
    step_s: float | None = None,
    frequency_hz: float | None = None,
    operating_point: dict[str, complex] | None = None,
) -> System:
    node_names, node_index = node_layout(circuit)
    branches = branch_components(circuit, mode)
    branch_index = {component.id: i for i, component in enumerate(branches)}
    size = len(node_names) + len(branches)
    complex_mode = mode == "ac"
    builder = _Builder(size, complex_mode)
    nonlinear: list[tuple[str, Component, str | None]] = []

    def node(name: str) -> int | None:
        return node_index.get(name)

    def branch(name: str) -> int | None:
        index = branch_index.get(name)
        return None if index is None else len(node_names) + index

    def stamp_source(component: Component, value) -> None:
        a, b = node(component.nodes[0]), node(component.nodes[1])
        if component.kind == "vsource":
            k = branch(component.id)
            builder.add(a, k, 1.0)
            builder.add(b, k, -1.0)
            builder.add(k, a, 1.0)
            builder.add(k, b, -1.0)
            builder.rhs[k] = value
        else:
            if a is not None:
                builder.rhs[a] -= value
            if b is not None:
                builder.rhs[b] += value

    def stamp_conductance(a, b, value) -> None:
        builder.add(a, a, value)
        builder.add(b, b, value)
        builder.add(a, b, -value)
        builder.add(b, a, -value)

    for component in circuit.components:
        a, b = node(component.nodes[0]), node(component.nodes[1])
        kind = component.kind
        if kind == "resistor":
            stamp_conductance(a, b, 1.0 / component.value)
        elif kind in {"vsource", "isource"}:
            if mode == "ac":
                stamp_source(component, source_phasor(component))
            elif mode in {"tran", "ic"}:
                stamp_source(component, source_value(component, time_s))
            else:
                stamp_source(component, source_dc(component))
        elif kind == "vcvs":
            k = branch(component.id)
            c, d = node(component.nodes[2]), node(component.nodes[3])
            builder.add(a, k, 1.0)
            builder.add(b, k, -1.0)
            builder.add(k, a, 1.0)
            builder.add(k, b, -1.0)
            builder.add(k, c, -component.value)
            builder.add(k, d, component.value)
        elif kind == "vccs":
            c, d = node(component.nodes[2]), node(component.nodes[3])
            gm = component.value
            builder.add(a, c, gm)
            builder.add(a, d, -gm)
            builder.add(b, c, -gm)
            builder.add(b, d, gm)
        elif kind == "switch":
            if component.state == "closed":
                k = branch(component.id)
                builder.add(a, k, 1.0)
                builder.add(b, k, -1.0)
                builder.add(k, a, 1.0)
                builder.add(k, b, -1.0)
        elif kind == "capacitor":
            capacitance = component.value
            if mode == "dc":
                continue
            if mode == "ac":
                if frequency_hz is None or frequency_hz <= 0:
                    raise InputError("AC analysis needs a frequency greater than zero")
                stamp_conductance(a, b, 1j * 2.0 * np.pi * frequency_hz * capacitance)
            elif mode == "ic":
                k = branch(component.id)
                builder.add(a, k, 1.0)
                builder.add(b, k, -1.0)
                builder.add(k, a, 1.0)
                builder.add(k, b, -1.0)
                held = (state.cap_v if state else {}).get(component.id)
                builder.rhs[k] = held if held is not None else (component.initial if component.initial is not None else 0.0)
            else:
                if step_s is None or step_s <= 0:
                    raise InputError("transient analysis needs a step greater than zero")
                previous_v = (state.cap_v if state else {}).get(component.id, component.initial or 0.0)
                previous_i = (state.cap_i if state else {}).get(component.id, 0.0)
                if method == "backward_euler":
                    g = capacitance / step_s
                    offset = g * previous_v
                elif method == "trapezoidal":
                    g = 2.0 * capacitance / step_s
                    offset = g * previous_v + previous_i
                else:
                    raise InputError(f"unknown transient method {method!r}")
                stamp_conductance(a, b, g)
                if a is not None:
                    builder.rhs[a] += offset
                if b is not None:
                    builder.rhs[b] -= offset
        elif kind == "inductor":
            inductance = component.value
            if mode == "ac":
                if frequency_hz is None or frequency_hz <= 0:
                    raise InputError("AC analysis needs a frequency greater than zero")
                stamp_conductance(a, b, 1.0 / (1j * 2.0 * np.pi * frequency_hz * inductance))
            elif mode == "ic":
                held = (state.ind_i if state else {}).get(component.id)
                value = held if held is not None else (component.initial if component.initial is not None else 0.0)
                if a is not None:
                    builder.rhs[a] -= value
                if b is not None:
                    builder.rhs[b] += value
            else:
                k = branch(component.id)
                builder.add(a, k, 1.0)
                builder.add(b, k, -1.0)
                builder.add(k, a, 1.0)
                builder.add(k, b, -1.0)
                if mode == "dc":
                    pass
                else:
                    if step_s is None or step_s <= 0:
                        raise InputError("transient analysis needs a step greater than zero")
                    previous_i = (state.ind_i if state else {}).get(component.id, component.initial or 0.0)
                    previous_v = (state.ind_v if state else {}).get(component.id, 0.0)
                    if method == "backward_euler":
                        builder.add(k, k, -inductance / step_s)
                        builder.rhs[k] = -(inductance / step_s) * previous_i
                    elif method == "trapezoidal":
                        g = 2.0 * inductance / step_s
                        builder.add(k, k, -g)
                        builder.rhs[k] = -g * previous_i - previous_v
                    else:
                        raise InputError(f"unknown transient method {method!r}")
        elif kind == "diode":
            junction = component.nodes[1]
            if component.params["rs_ohm"] > 0.0:
                junction = internal_node(component)
                stamp_conductance(node(component.nodes[1]), node(junction), 1.0 / component.params["rs_ohm"])
            if mode == "ac":
                if operating_point is None:
                    raise InputError("AC analysis of a diode needs a DC operating point")
                vd = operating_point.get(component.nodes[0], 0.0) - operating_point.get(junction, 0.0)
                _, g = diode_eval(float(np.real(vd)), component.params)
                stamp_conductance(a, node(junction), g)
            else:
                nonlinear.append(("diode", component, junction))
        elif kind == "mosfet":
            if mode == "ac":
                if operating_point is None:
                    raise InputError("AC analysis of a MOSFET needs a DC operating point")
                d, g, s = (node(name) for name in component.nodes[:3])
                vd = float(np.real(operating_point.get(component.nodes[0], 0.0)))
                vg = float(np.real(operating_point.get(component.nodes[1], 0.0)))
                vs = float(np.real(operating_point.get(component.nodes[2], 0.0)))
                _, deriv = mosfet_eval(vd, vg, vs, component.params)
                builder.add(d, g, deriv["g"])
                builder.add(d, d, deriv["d"])
                builder.add(d, s, deriv["s"])
                builder.add(s, g, -deriv["g"])
                builder.add(s, d, -deriv["d"])
                builder.add(s, s, -deriv["s"])
            else:
                nonlinear.append(("mosfet", component, None))
        elif kind == "opamp":
            k = branch(component.id)
            out, inp, inn, ref = (node(name) for name in component.nodes)
            builder.add(out, k, 1.0)
            builder.add(ref, k, -1.0)
            if component.params["ideal"]:
                builder.add(k, inp, 1.0)
                builder.add(k, inn, -1.0)
            else:
                gain = component.params["gain"]
                builder.add(k, out, 1.0)
                builder.add(k, ref, -1.0)
                builder.add(k, inp, -gain)
                builder.add(k, inn, gain)
        else:
            raise InputError(f"unsupported component kind {kind!r}")

    matrix = builder.matrix()
    if complex_mode:
        matrix = matrix.astype(complex)
    return System(matrix, builder.rhs, node_index, branch_index, list(circuit.node_names), nonlinear, mode, size)


def _nonlinear_terms(system: System, x: np.ndarray) -> tuple[np.ndarray, coo_matrix]:
    extra = np.zeros(system.size)
    rows: list[int] = []
    cols: list[int] = []
    data: list[float] = []

    def add(row, col, value):
        if row is None or col is None or value == 0.0:
            return
        rows.append(row)
        cols.append(col)
        data.append(value)

    def voltage(name: str) -> float:
        index = system.node_index.get(name)
        return 0.0 if index is None else float(x[index])

    def index(name: str) -> int | None:
        return system.node_index.get(name)

    for kind, component, junction in system.nonlinear:
        if kind == "diode":
            vd = voltage(component.nodes[0]) - voltage(junction)
            current, conductance = diode_eval(vd, component.params)
            a, k = index(component.nodes[0]), index(junction)
            extra[a] += current
            if k is not None:
                extra[k] -= current
            add(a, a, conductance)
            add(a, k, -conductance)
            add(k, a, -conductance)
            add(k, k, conductance)
        else:
            d_name, g_name, s_name = component.nodes[:3]
            current, deriv = mosfet_eval(voltage(d_name), voltage(g_name), voltage(s_name), component.params)
            d, g, s = index(d_name), index(g_name), index(s_name)
            extra[d] += current
            if s is not None:
                extra[s] -= current
            add(d, g, deriv["g"])
            add(d, d, deriv["d"])
            add(d, s, deriv["s"])
            add(s, g, -deriv["g"])
            add(s, d, -deriv["d"])
            add(s, s, -deriv["s"])
    jacobian = coo_matrix((data, (rows, cols)), shape=(system.size, system.size)).tocsc()
    jacobian.sum_duplicates()
    return extra, jacobian


def _residual(system: System, x: np.ndarray) -> tuple[np.ndarray, coo_matrix]:
    extra, jacobian = _nonlinear_terms(system, x)
    residual = system.matrix @ x - system.rhs + extra
    return np.asarray(residual).reshape(-1), jacobian


def _factor(matrix):
    try:
        return splu(matrix.tocsc())
    except (RuntimeError, ValueError) as exc:
        raise SolverError(f"singular circuit: matrix factorisation failed ({exc})", code="singular") from exc


def solve_system(system: System, x0: np.ndarray | None = None, rtol: float = 1e-9, atol: float = 1e-12, max_iterations: int = 100) -> tuple[np.ndarray, NewtonReport]:
    if system.matrix.dtype.kind == "c" or not system.nonlinear:
        lu = _factor(system.matrix)
        x = np.asarray(lu.solve(system.rhs))
        if not np.all(np.isfinite(x)):
            raise SolverError("singular circuit: non-finite solution", code="singular")
        residual = float(np.max(np.abs(system.matrix @ x - system.rhs))) if system.size else 0.0
        return x, NewtonReport(True, 1, residual, 0.0, 0, "linear solve")

    x = np.zeros(system.size) if x0 is None else np.array(x0, dtype=float).copy()
    residual, jacobian = _residual(system, x)
    norm = float(np.max(np.abs(residual))) if residual.size else 0.0
    scale = max(1.0, float(np.max(np.abs(system.rhs))) if system.rhs.size else 0.0)
    damping_steps = 0
    max_update = 0.0
    for iteration in range(1, max_iterations + 1):
        lu = _factor(system.matrix + jacobian)
        dx = np.asarray(lu.solve(-residual))
        if not np.all(np.isfinite(dx)):
            raise SolverError("singular circuit: Newton update is not finite", code="singular")
        alpha = 1.0
        accepted = False
        trial_x = x
        trial_residual = residual
        trial_norm = norm
        for _ in range(14):
            candidate = x + alpha * dx
            candidate_residual, _ = _residual(system, candidate)
            candidate_norm = float(np.max(np.abs(candidate_residual))) if candidate_residual.size else 0.0
            if candidate_norm < trial_norm or candidate_norm <= 1e-12 * scale:
                trial_x, trial_residual, trial_norm = candidate, candidate_residual, candidate_norm
                accepted = True
                break
            alpha *= 0.5
        if not accepted:
            trial_x = x + alpha * dx
            trial_residual, _ = _residual(system, trial_x)
            trial_norm = float(np.max(np.abs(trial_residual))) if trial_residual.size else 0.0
        if alpha < 1.0:
            damping_steps += 1
        max_update = float(np.max(np.abs(trial_x - x))) if x.size else 0.0
        x, residual, norm = trial_x, trial_residual, trial_norm
        _, jacobian = _residual(system, x)
        tolerance = atol + rtol * max(1.0, float(np.max(np.abs(x))) if x.size else 0.0)
        if max_update <= tolerance and norm <= 1e-6 * scale:
            return x, NewtonReport(True, iteration, norm, max_update, damping_steps, "converged")
    raise SolverError(
        f"Newton did not converge after {max_iterations} iterations (residual {norm:.3e}); "
        "check device parameters and source values",
        code="newton_failed",
    )


def node_voltage_map(x: np.ndarray, system: System) -> dict[str, complex]:
    return {name: complex(x[index]) for name, index in system.node_index.items()}


def component_currents(circuit: Circuit, x: np.ndarray, system: System, state: State | None = None) -> dict[str, float]:
    voltages = node_voltage_map(x, system)
    currents: dict[str, float] = {}
    for component in circuit.components:
        kind = component.kind
        a = component.nodes[0]
        b = component.nodes[1]
        if kind == "resistor":
            currents[component.id] = float(np.real((voltages.get(a, 0) - voltages.get(b, 0)) / component.value))
        elif kind == "isource":
            currents[component.id] = float(np.real(source_value(component, 0.0) if system.mode == "tran" else source_dc(component)))
        elif kind == "vccs":
            currents[component.id] = float(np.real(component.value * (voltages.get(component.nodes[2], 0) - voltages.get(component.nodes[3], 0))))
        elif kind == "switch":
            if component.state == "open":
                currents[component.id] = 0.0
            else:
                currents[component.id] = float(np.real(x[system.branch_index[component.id] + len(system.node_index)]))
        elif kind == "capacitor":
            if system.mode == "tran" and state is not None:
                currents[component.id] = float(state.cap_i.get(component.id, 0.0))
            elif system.mode == "ic":
                currents[component.id] = float(np.real(x[system.branch_index[component.id] + len(system.node_index)]))
            else:
                currents[component.id] = 0.0
        elif kind == "inductor":
            if system.mode in {"dc", "tran"}:
                currents[component.id] = float(np.real(x[system.branch_index[component.id] + len(system.node_index)]))
            elif system.mode == "ic":
                currents[component.id] = component.initial or 0.0
            else:
                currents[component.id] = 0.0
        elif kind == "diode":
            junction = internal_node(component) if component.params["rs_ohm"] > 0.0 else b
            vd = float(np.real(voltages.get(a, 0) - voltages.get(junction, 0)))
            currents[component.id] = float(diode_eval(vd, component.params)[0])
        elif kind == "mosfet":
            d, g, s = component.nodes[:3]
            currents[component.id] = float(mosfet_eval(float(np.real(voltages.get(d, 0))), float(np.real(voltages.get(g, 0))), float(np.real(voltages.get(s, 0))), component.params)[0])
        elif kind in {"vsource", "vcvs", "opamp"}:
            currents[component.id] = float(np.real(x[system.branch_index[component.id] + len(system.node_index)]))
        else:
            currents[component.id] = 0.0
    return currents


def powers(circuit: Circuit, voltages: dict[str, complex], currents: dict[str, float]) -> dict[str, float]:
    result = {}
    for component in circuit.components:
        v = voltages.get(component.nodes[0], 0.0) - voltages.get(component.nodes[1], 0.0)
        result[component.id] = float(np.real(v * currents.get(component.id, 0.0)))
    return result
