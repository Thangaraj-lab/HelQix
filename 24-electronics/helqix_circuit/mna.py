"""Modified nodal analysis for linear DC circuits."""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
from scipy.sparse import coo_matrix
from scipy.sparse.linalg import splu

from .errors import InputError, SolverError
from .netlist import Circuit, Component

SOLVER_NAME = "helqix_circuit.mna"
SOLVER_VERSION = "0.1.0"


@dataclass
class DcSolution:
    node_voltages: dict[str, float]
    branch_currents: dict[str, float]
    powers: dict[str, float]
    residual: float
    circuit: Circuit

    def to_dict(self) -> dict:
        return {
            "status": "ok",
            "analysis": "dc",
            "validation_label": "CALCULATED",
            "node_voltages": {name: {"value": value, "unit": "V"} for name, value in self.node_voltages.items()},
            "branch_currents": {name: {"value": value, "unit": "A"} for name, value in self.branch_currents.items()},
            "component_powers": {name: {"value": value, "unit": "W"} for name, value in self.powers.items()},
            "residual": self.residual,
            "solver": {"name": SOLVER_NAME, "version": SOLVER_VERSION, "method": "modified nodal analysis, sparse LU"},
        }


def _conductive_edges(circuit: Circuit) -> list[tuple[str, str, str]]:
    edges = []
    for component in circuit.components:
        if component.kind == "resistor":
            edges.append((component.nodes[0], component.nodes[1], component.id))
        elif component.kind in {"vsource", "vcvs"}:
            edges.append((component.nodes[0], component.nodes[1], component.id))
        elif component.kind == "switch" and component.state == "closed":
            edges.append((component.nodes[0], component.nodes[1], component.id))
        elif component.kind == "inductor":
            edges.append((component.nodes[0], component.nodes[1], component.id))
    return edges


def _check_floating(circuit: Circuit) -> None:
    nodes = {"0"}
    for component in circuit.components:
        nodes.update(component.nodes)
    adjacency: dict[str, set[str]] = {node: set() for node in nodes}
    for a, b, _ in _conductive_edges(circuit):
        adjacency[a].add(b)
        adjacency[b].add(a)
    seen = {"0"}
    stack = ["0"]
    while stack:
        node = stack.pop()
        for neighbour in adjacency[node]:
            if neighbour not in seen:
                seen.add(neighbour)
                stack.append(neighbour)
    floating = sorted(nodes - seen)
    if floating:
        raise InputError(
            "floating node(s) " + ", ".join(repr(node) for node in floating)
            + ": no DC path to ground through resistors, voltage sources, closed switches or inductors. "
            "Current sources, open switches and control terminals do not provide a path.",
            code="floating_node",
        )


class _UnionFind:
    def __init__(self):
        self.parent: dict[str, str] = {}
        self.members: dict[str, list[str]] = {}

    def find(self, node: str) -> str:
        self.parent.setdefault(node, node)
        while self.parent[node] != node:
            self.parent[node] = self.parent[self.parent[node]]
            node = self.parent[node]
        return node

    def union(self, a: str, b: str, label: str) -> list[str] | None:
        ra, rb = self.find(a), self.find(b)
        if ra == rb:
            return self.members.setdefault(ra, []) + [label]
        self.parent[rb] = ra
        merged = self.members.get(ra, []) + self.members.get(rb, []) + [label]
        self.members[ra] = merged
        self.members.pop(rb, None)
        return None


def _check_voltage_loops(circuit: Circuit) -> None:
    union = _UnionFind()
    for component in circuit.components:
        if component.kind == "vsource" or (component.kind == "switch" and component.state == "closed"):
            loop = union.union(component.nodes[0], component.nodes[1], component.id)
            if loop is not None:
                names = ", ".join(dict.fromkeys(loop))
                raise InputError(
                    f"voltage-source loop: {names} form a closed loop of ideal voltage sources "
                    "(including closed switches and inductors). Such a loop has no unique solution.",
                    code="voltage_source_loop",
                )


def _assemble(circuit: Circuit) -> tuple[coo_matrix, np.ndarray, dict[str, int], dict[str, int]]:
    node_index = {name: i for i, name in enumerate(circuit.node_names)}
    branch_components = [
        component
        for component in circuit.components
        if component.kind in {"vsource", "vcvs"} or (component.kind == "switch" and component.state == "closed")
    ]
    branch_index = {component.id: i for i, component in enumerate(branch_components)}
    size = len(node_index) + len(branch_components)
    rows: list[int] = []
    cols: list[int] = []
    data: list[float] = []
    rhs = np.zeros(size)

    def add(row: int | None, col: int | None, value: float) -> None:
        if row is None or col is None or value == 0.0:
            return
        rows.append(row)
        cols.append(col)
        data.append(value)

    def node(name: str) -> int | None:
        return node_index.get(name)

    def branch(cid: str) -> int | None:
        index = branch_index.get(cid)
        return None if index is None else len(node_index) + index

    for component in circuit.components:
        a, b = node(component.nodes[0]), node(component.nodes[1])
        if component.kind == "resistor":
            g = 1.0 / component.value
            add(a, a, g)
            add(b, b, g)
            add(a, b, -g)
            add(b, a, -g)
        elif component.kind == "vsource":
            k = branch(component.id)
            add(a, k, 1.0)
            add(b, k, -1.0)
            add(k, a, 1.0)
            add(k, b, -1.0)
            rhs[k] = component.value
        elif component.kind == "isource":
            # positive current flows inside the source from nodes[0] to nodes[1]
            if a is not None:
                rhs[a] -= component.value
            if b is not None:
                rhs[b] += component.value
        elif component.kind == "vcvs":
            k = branch(component.id)
            c, d = node(component.nodes[2]), node(component.nodes[3])
            add(a, k, 1.0)
            add(b, k, -1.0)
            add(k, a, 1.0)
            add(k, b, -1.0)
            add(k, c, -component.value)
            add(k, d, component.value)
        elif component.kind == "vccs":
            c, d = node(component.nodes[2]), node(component.nodes[3])
            gm = component.value
            add(a, c, gm)
            add(a, d, -gm)
            add(b, c, -gm)
            add(b, d, gm)
        elif component.kind == "switch" and component.state == "closed":
            k = branch(component.id)
            add(a, k, 1.0)
            add(b, k, -1.0)
            add(k, a, 1.0)
            add(k, b, -1.0)

    matrix = coo_matrix((data, (rows, cols)), shape=(size, size)).tocsc()
    matrix.sum_duplicates()
    return matrix, rhs, node_index, branch_index


def _factor(matrix):
    if matrix.shape[0] == 0:
        return None
    try:
        return splu(matrix)
    except RuntimeError as exc:
        raise SolverError(
            "singular circuit: the equations have no unique solution. "
            "Conflicting ideal voltage constraints or an unreferenced subnetwork are the usual causes. "
            f"(factorisation said: {exc})",
            code="singular",
        ) from exc
    except ValueError as exc:
        raise SolverError(f"singular circuit: matrix factorisation failed: {exc}", code="singular") from exc


def _currents(circuit: Circuit, x: np.ndarray, node_index: dict[str, int], branch_index: dict[str, int]) -> tuple[dict[str, float], dict[str, float], dict[str, float]]:
    voltages = {"0": 0.0}
    for name, index in node_index.items():
        voltages[name] = float(x[index])

    def v(name: str) -> float:
        return voltages.get(name, 0.0)

    currents: dict[str, float] = {}
    for component in circuit.components:
        if component.kind == "resistor":
            currents[component.id] = (v(component.nodes[0]) - v(component.nodes[1])) / component.value
        elif component.kind == "isource":
            currents[component.id] = component.value
        elif component.kind == "vccs":
            currents[component.id] = component.value * (v(component.nodes[2]) - v(component.nodes[3]))
        elif component.kind == "switch" and component.state == "open":
            currents[component.id] = 0.0
        else:
            index = branch_index.get(component.id)
            currents[component.id] = float(x[len(node_index) + index]) if index is not None else 0.0

    powers = {
        component.id: (v(component.nodes[0]) - v(component.nodes[1])) * currents[component.id]
        for component in circuit.components
    }
    return voltages, currents, powers


def solve_dc(circuit: Circuit) -> DcSolution:
    analysis_type = str(circuit.analysis.get("type", "dc")).lower()
    if analysis_type != "dc":
        raise InputError(f"solve_dc cannot run analysis type {analysis_type!r}")
    _check_floating(circuit)
    _check_voltage_loops(circuit)
    matrix, rhs, node_index, branch_index = _assemble(circuit)
    lu = _factor(matrix)
    if lu is None:
        x = np.zeros(0)
    else:
        x = lu.solve(rhs)
    if not np.all(np.isfinite(x)):
        raise SolverError("singular circuit: the solution contains non-finite values", code="singular")
    scale = max(float(np.max(np.abs(rhs))) if rhs.size else 0.0, 1.0)
    residual = float(np.max(np.abs(matrix @ x - rhs))) if matrix.shape[0] else 0.0
    relative = residual / scale
    if relative > 1e-6:
        raise SolverError(
            f"singular circuit: the solution does not satisfy the equations (relative residual {relative:.3e})",
            code="singular",
        )
    voltages, currents, powers = _currents(circuit, x, node_index, branch_index)
    return DcSolution(voltages, currents, powers, relative, circuit)
