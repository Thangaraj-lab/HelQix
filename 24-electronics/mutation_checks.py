"""Break formulas on purpose; a named test must fail for each mutation.

Each entry is a source replacement applied to a throwaway copy of
24-electronics/. A mutation that no test catches is reported as a survivor.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
IGNORE = shutil.ignore_patterns(".venv", "__pycache__", ".pytest_cache", "*.pyc", "examples", "docs", "schemas")

MUTATIONS = [
    {
        "phase": "E1",
        "name": "resistor conductance halved",
        "file": "helqix_circuit/mna.py",
        "old": "g = 1.0 / component.value",
        "new": "g = 1.0 / (2.0 * component.value)",
        "test": "helqix_circuit/tests/test_dc.py::test_voltage_divider_matches_hand_formula",
    },
    {
        "phase": "E1",
        "name": "ohm scale drifted by 1e-7",
        "file": "helqix_circuit/units.py",
        "old": '"ohm": 1.0,',
        "new": '"ohm": 1.0000001,',
        "test": "helqix_circuit/tests/test_dc.py::test_voltage_divider_matches_hand_formula",
    },
    {
        "phase": "E1",
        "name": "current source injection sign flipped",
        "file": "helqix_circuit/mna.py",
        "old": "rhs[a] -= component.value",
        "new": "rhs[a] += component.value",
        "test": "helqix_circuit/tests/test_dc.py::test_current_source_sign_follows_spice_convention",
    },
    {
        "phase": "E1",
        "name": "VCCS output-row sign flipped",
        "file": "helqix_circuit/mna.py",
        "old": "add(a, c, gm)",
        "new": "add(a, c, -gm)",
        "test": "helqix_circuit/tests/test_dc.py::test_vcvs_and_vccs_stamps",
    },
    {
        "phase": "E1",
        "name": "voltage-source loop check disabled",
        "file": "helqix_circuit/mna.py",
        "old": "def _check_voltage_loops(circuit: Circuit) -> None:\n    union = _UnionFind()",
        "new": "def _check_voltage_loops(circuit: Circuit) -> None:\n    return\n    union = _UnionFind()",
        "test": "helqix_circuit/tests/test_dc.py::test_voltage_source_loop_is_refused_with_names",
    },
]


def _copy_tree(destination: Path) -> None:
    shutil.copytree(ROOT, destination, ignore=IGNORE)


def _run_test(root: Path, node: str) -> tuple[int, str]:
    env = dict(os.environ)
    env["PYTHONPATH"] = str(root) + os.pathsep + env.get("PYTHONPATH", "")
    proc = subprocess.run(
        [sys.executable, "-m", "pytest", node, "-q", "--tb=no", "-p", "no:cacheprovider"],
        cwd=str(root),
        capture_output=True,
        text=True,
        env=env,
        timeout=120,
    )
    return proc.returncode, (proc.stdout + proc.stderr).strip().splitlines()[-1] if (proc.stdout or proc.stderr) else ""


def run() -> tuple[bool, str]:
    caught = 0
    survivors = []
    for mutation in MUTATIONS:
        with tempfile.TemporaryDirectory(prefix="helqix-mut-") as tmp:
            root = Path(tmp) / "24-electronics"
            _copy_tree(root)
            target = root / mutation["file"]
            text = target.read_text(encoding="utf-8")
            if mutation["old"] not in text:
                survivors.append(f"{mutation['phase']} {mutation['name']}: pattern not found")
                continue
            target.write_text(text.replace(mutation["old"], mutation["new"], 1), encoding="utf-8")
            code, summary = _run_test(root, mutation["test"])
            if code == 0:
                survivors.append(f"{mutation['phase']} {mutation['name']}: survived ({summary})")
            else:
                caught += 1
    total = len(MUTATIONS)
    if survivors:
        return False, f"{caught}/{total} caught; survivors: " + "; ".join(survivors)
    return True, f"{caught}/{total} caught"


if __name__ == "__main__":
    ok, text = run()
    print(text)
    raise SystemExit(0 if ok else 1)
