"""CalculiX tool pieces that need no ccx: input validation, mesh generation, .dat parsing.
The solve itself is tested by the Rust adapter tests in CI (Ubuntu calculix-ccx)."""

import pytest

from helqix_calculix import cli

GOOD = {"schema": cli.SCHEMA, "length_m": 1.0, "width_m": 0.05, "height_m": 0.05, "youngs_modulus_pa": 2.1e11,
        "poisson_ratio": 0.3, "tip_load_n": 1000.0, "elements": [4, 2, 2]}

@pytest.mark.parametrize("change", [{"schema": "x"}, {"length_m": 0}, {"width_m": -1.0}, {"youngs_modulus_pa": "stiff"},
                                    {"poisson_ratio": 0.5}, {"tip_load_n": float("nan")}, {"elements": [0, 2, 2]},
                                    {"elements": [200, 200, 1]}, {"elements": [2, 2]}, {"height_m": True}])
def test_invalid_inputs_exit_2(change):
    with pytest.raises(SystemExit) as e:
        cli.parse_input({**GOOD, **change})
    assert e.value.code == 2

def read_mesh(path):
    nodes, elements, section = {}, [], None
    for line in open(path, encoding="ascii"):
        line = line.strip()
        if line.startswith("*"):
            section = line.split(",")[0].upper()
            continue
        if section == "*NODE":
            i, x, y, z = line.split(",")
            nodes[int(i)] = (float(x), float(y), float(z))
        elif section == "*ELEMENT":
            elements.append([int(v) for v in line.split(",")[1:]])
    return nodes, elements

def test_mesh_counts_sets_and_positive_element_volume(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    p = cli.parse_input(GOOD)
    mesh = cli.write_inp(p)
    assert mesh == {"elements": 16, "nodes": 45, "element_type": "C3D8I", "tip_nodes": 9}
    nodes, elements = read_mesh(tmp_path / "cantilever.inp")
    assert len(nodes) == 45 and len(elements) == 16
    for conn in elements:
        p0, p1, p3, p4 = (nodes[conn[i]] for i in (0, 1, 3, 4))
        a, b, c = ([q[k] - p0[k] for k in range(3)] for q in (p1, p3, p4))
        volume = (a[1] * b[2] - a[2] * b[1]) * c[0] + (a[2] * b[0] - a[0] * b[2]) * c[1] + (a[0] * b[1] - a[1] * b[0]) * c[2]
        assert volume > 0, f"element {conn} is inverted"
    text = (tmp_path / "cantilever.inp").read_text()
    loads = [float(l.split(",")[2]) for l in text.split("*CLOAD")[1].split("*NODE PRINT")[0].strip().splitlines()]
    assert len(loads) == 9 and abs(sum(loads) + 1000.0) < 1e-9, "the full tip load, split over the end face, pointing -z"
    assert "FIXED, 1, 3, 0.0" in text

DAT = """
 displacements (vx,vy,vz) for set TIP and time  0.1000000E+01

        41  1.000000E-05  2.000000E-08 -3.050000E-03
        42  1.000000E-05 -2.000000E-08 -3.040000E-03

"""

def test_dat_parser_reads_uz_and_checks_the_count():
    assert cli.parse_dat(DAT, 2) == [-3.05e-3, -3.04e-3]
    with pytest.raises(ValueError):
        cli.parse_dat(DAT, 3)
    with pytest.raises(ValueError):
        cli.parse_dat("no results here", 2)
