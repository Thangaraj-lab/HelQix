"""Random-seeded properties, run in the nightly verification only."""

from __future__ import annotations

import random

import pytest

from helqix_circuit import load_circuit, solve_dc

SEEDS = list(range(20))


def _ladder(seed: int):
    rng = random.Random(seed)
    supply = rng.uniform(-20.0, 20.0)
    resistors = [10.0 ** rng.uniform(1.0, 5.0) for _ in range(3)]
    nodes = ["1", "2", "3"]
    components = [
        {"id": "V1", "kind": "vsource", "nodes": ["1", "0"], "value": {"value": supply, "unit": "V"}},
    ]
    for index, ohms in enumerate(resistors):
        components.append(
            {"id": f"R{index + 1}", "kind": "resistor", "nodes": [nodes[index], nodes[index + 1] if index < 2 else "0"],
             "value": {"value": ohms, "unit": "ohm"}}
        )
    body = {"version": "circuit/0.1", "components": components, "analysis": {"type": "dc"}}
    return body, supply, resistors


@pytest.mark.nightly
@pytest.mark.parametrize("seed", SEEDS, ids=[f"seed{s}" for s in SEEDS])
def test_random_series_ladder_matches_the_hand_formula(seed):
    body, supply, resistors = _ladder(seed)
    solution = solve_dc(load_circuit(body))
    total = sum(resistors)
    for index in range(len(resistors) - 1):
        expected = supply * sum(resistors[index + 1:]) / total
        node = str(index + 2)
        assert solution.node_voltages[node] == pytest.approx(expected, rel=1e-9, abs=1e-9)
    for index, ohms in enumerate(resistors):
        left, right = str(index + 1), str(index + 2) if index < 2 else "0"
        drop = solution.node_voltages[left] - solution.node_voltages[right]
        assert solution.branch_currents[f"R{index + 1}"] == pytest.approx(drop / ohms, rel=1e-9, abs=1e-12)

    # The same network with every resistor scaled by k has the same voltages.
    scale = 10.0 ** random.Random(seed).uniform(-2.0, 2.0)
    for component in body["components"]:
        if component["kind"] == "resistor":
            component["value"]["value"] *= scale
    scaled = solve_dc(load_circuit(body))
    for node in ("1", "2", "3"):
        assert scaled.node_voltages[node] == pytest.approx(solution.node_voltages[node], rel=1e-8, abs=1e-10)
