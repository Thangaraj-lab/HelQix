"""Cantilever benchmark through CalculiX for the HELQIX CalculiX adapter.

stdin  : {"schema": "helqix.calculix-input.v1", "length_m": 1.0, "width_m": 0.05, "height_m": 0.05,
          "youngs_modulus_pa": 2.1e11, "poisson_ratio": 0.3, "tip_load_n": 1000.0,
          "elements": [20, 2, 2]}
stdout : {"tool_version": "ccx <version>", "outputs": [
            {"name": "tip_deflection", "unit": "m", "values": [...]},
            {"name": "euler_bernoulli_deflection", "unit": "m", "values": [...]},
            {"name": "timoshenko_deflection", "unit": "m", "values": [...]}], "mesh": {...}}
exit   : 0 ok, 2 invalid input, 3 ccx failed or produced no result.

The beam is clamped at x = 0 (all displacements fixed) and loaded downward (-z) at x = L, the load
split evenly over the end-face nodes. Elements are C3D8I (8-node bricks with incompatible modes,
which bend without shear locking). The reported tip deflection is the mean -uz of the end-face
nodes. The closed forms are CALCULATED for comparison, not tool output:
  Euler-Bernoulli  d = F L^3 / (3 E I),   I = b h^3 / 12
  Timoshenko       d + F L / (k G A),     k = 5/6, G = E / (2 (1 + nu)), A = b h
ccx is the program path given as the launcher's second argument (else HELQIX_CCX, else `ccx` on PATH) and runs single-threaded in the current directory.
"""
from __future__ import annotations

import json
import math
import os
import re
import subprocess
import sys

SCHEMA = "helqix.calculix-input.v1"
JOB = "cantilever"
# Hard cap so ccx cannot outlive the adapter (which kills this process at its own timeout).
CCX_TIMEOUT_S = 300


def fail(code: int, message: str) -> None:
    sys.stderr.write(message[:2000] + "\n")
    raise SystemExit(code)


def parse_input(doc) -> dict:
    if not isinstance(doc, dict) or doc.get("schema") != SCHEMA:
        fail(2, f"schema must be {SCHEMA}")
    out = {}
    for key in ("length_m", "width_m", "height_m", "youngs_modulus_pa", "tip_load_n"):
        v = doc.get(key)
        if not isinstance(v, (int, float)) or isinstance(v, bool) or not math.isfinite(v) or v <= 0:
            fail(2, f"{key} must be a positive finite number")
        out[key] = float(v)
    nu = doc.get("poisson_ratio")
    if not isinstance(nu, (int, float)) or isinstance(nu, bool) or not (0.0 <= nu < 0.5):
        fail(2, "poisson_ratio must be in [0, 0.5)")
    out["poisson_ratio"] = float(nu)
    el = doc.get("elements", [20, 2, 2])
    if not (isinstance(el, list) and len(el) == 3 and all(isinstance(n, int) and not isinstance(n, bool) and 1 <= n <= 200 for n in el)):
        fail(2, "elements must be three integers in 1..200 (along length, width, height)")
    if el[0] * el[1] * el[2] > 20000:
        fail(2, "mesh too large (more than 20000 elements)")
    out["elements"] = el
    return out


def node_id(i: int, j: int, k: int, ny: int, nz: int) -> int:
    return 1 + i * (ny + 1) * (nz + 1) + j * (nz + 1) + k


def write_inp(p: dict) -> dict:
    """Writes cantilever.inp in the current directory; returns mesh facts."""
    nx, ny, nz = p["elements"]
    L, b, h = p["length_m"], p["width_m"], p["height_m"]
    lines = ["*HEADING", "HELQIX cantilever benchmark", "*NODE, NSET=NALL"]
    for i in range(nx + 1):
        for j in range(ny + 1):
            for k in range(nz + 1):
                lines.append(f"{node_id(i, j, k, ny, nz)}, {L * i / nx:.12g}, {b * j / ny:.12g}, {h * k / nz:.12g}")
    lines.append("*ELEMENT, TYPE=C3D8I, ELSET=EALL")
    e = 1
    for i in range(nx):
        for j in range(ny):
            for k in range(nz):
                n = lambda di, dj, dk: node_id(i + di, j + dj, k + dk, ny, nz)
                # CalculiX/Abaqus brick order: bottom face (k) counter-clockwise, then top face (k+1).
                conn = [n(0, 0, 0), n(1, 0, 0), n(1, 1, 0), n(0, 1, 0), n(0, 0, 1), n(1, 0, 1), n(1, 1, 1), n(0, 1, 1)]
                lines.append(f"{e}, " + ", ".join(map(str, conn)))
                e += 1
    fixed = [node_id(0, j, k, ny, nz) for j in range(ny + 1) for k in range(nz + 1)]
    tip = [node_id(nx, j, k, ny, nz) for j in range(ny + 1) for k in range(nz + 1)]
    lines.append("*NSET, NSET=FIXED")
    lines += [", ".join(map(str, fixed[i:i + 8])) for i in range(0, len(fixed), 8)]
    lines.append("*NSET, NSET=TIP")
    lines += [", ".join(map(str, tip[i:i + 8])) for i in range(0, len(tip), 8)]
    lines += ["*MATERIAL, NAME=ELASTIC", "*ELASTIC", f"{p['youngs_modulus_pa']:.12g}, {p['poisson_ratio']:.12g}",
              "*SOLID SECTION, ELSET=EALL, MATERIAL=ELASTIC",
              "*BOUNDARY", "FIXED, 1, 3, 0.0",
              "*STEP", "*STATIC", "*CLOAD"]
    per_node = -p["tip_load_n"] / len(tip)
    lines += [f"{nid}, 3, {per_node:.12g}" for nid in tip]
    lines += ["*NODE PRINT, NSET=TIP", "U", "*END STEP"]
    with open(f"{JOB}.inp", "w", encoding="ascii") as f:
        f.write("\n".join(lines) + "\n")
    return {"elements": nx * ny * nz, "nodes": (nx + 1) * (ny + 1) * (nz + 1), "element_type": "C3D8I", "tip_nodes": len(tip)}


def parse_dat(text: str, expected_nodes: int) -> list[float]:
    """Returns uz of every node printed after the displacement header in a ccx .dat file."""
    lines = text.splitlines()
    start = next((i for i, l in enumerate(lines) if "displacements" in l.lower()), None)
    if start is None:
        raise ValueError("no displacement block in the .dat file")
    uz = []
    for l in lines[start + 1:]:
        parts = l.split()
        if not parts:
            if uz:
                break
            continue
        if not re.fullmatch(r"\d+", parts[0]) or len(parts) < 4:
            break
        uz.append(float(parts[3].replace("D", "E")))
    if len(uz) != expected_nodes:
        raise ValueError(f"expected {expected_nodes} tip nodes in the .dat file, found {len(uz)}")
    return uz


def ccx_version(ccx: str, env: dict) -> str:
    try:
        r = subprocess.run([ccx, "-v"], capture_output=True, text=True, timeout=30, env=env)
    except FileNotFoundError:
        fail(3, f"ccx not found: {ccx}")
    m = re.search(r"Version\s+([0-9][0-9.]*)", r.stdout + r.stderr)
    return f"ccx {m.group(1)}" if m else "ccx (version not reported)"


def main() -> None:
    try:
        doc = json.loads(sys.stdin.read())
    except Exception as e:
        fail(2, f"input is not JSON: {e}")
    p = parse_input(doc)
    mesh = write_inp(p)
    # The adapter passes the ccx path as an argument; requests never choose the program.
    ccx = (sys.argv[2] if len(sys.argv) > 2 and sys.argv[2] else None) or os.environ.get("HELQIX_CCX") or "ccx"
    env = {k: v for k, v in os.environ.items() if k in ("PATH", "SYSTEMROOT", "TEMP", "TMP", "HOME", "LANG")}
    env["OMP_NUM_THREADS"] = "1"
    env["CCX_NPROC_EQUATION_SOLVER"] = "1"
    version = ccx_version(ccx, env)
    try:
        r = subprocess.run([ccx, "-i", JOB], capture_output=True, text=True, env=env, timeout=CCX_TIMEOUT_S)
    except FileNotFoundError:
        fail(3, f"ccx not found: {ccx}")
    except subprocess.TimeoutExpired:
        fail(3, f"ccx exceeded {CCX_TIMEOUT_S} s")
    if r.returncode != 0 or not os.path.exists(f"{JOB}.dat"):
        fail(3, f"ccx failed (exit {r.returncode}): {(r.stdout + r.stderr)[-1500:]}")
    try:
        uz = parse_dat(open(f"{JOB}.dat", encoding="ascii", errors="replace").read(), mesh["tip_nodes"])
    except ValueError as e:
        fail(3, str(e))
    tip = -sum(uz) / len(uz)
    E, nu, F, L, b, h = p["youngs_modulus_pa"], p["poisson_ratio"], p["tip_load_n"], p["length_m"], p["width_m"], p["height_m"]
    I = b * h ** 3 / 12.0
    eb = F * L ** 3 / (3.0 * E * I)
    G = E / (2.0 * (1.0 + nu))
    timo = eb + F * L / ((5.0 / 6.0) * G * b * h)
    sys.stdout.write(json.dumps({"tool_version": version, "outputs": [
        {"name": "tip_deflection", "unit": "m", "values": [tip]},
        {"name": "euler_bernoulli_deflection", "unit": "m", "values": [eb]},
        {"name": "timoshenko_deflection", "unit": "m", "values": [timo]},
    ], "mesh": mesh}))


if __name__ == "__main__":
    main()
