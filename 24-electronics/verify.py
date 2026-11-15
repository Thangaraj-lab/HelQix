#!/usr/bin/env python
"""Run every electronics suite and print one table.

Usage:
    python 24-electronics/verify.py            # fast suites (nightly skipped)
    python 24-electronics/verify.py --nightly  # include nightly-marked tests
    python 24-electronics/verify.py --no-mutations --no-keyscan

Exit code 0 only when every suite is green, every registered mutation was
caught, and the key scan is clean.
"""

from __future__ import annotations

import argparse
import importlib.util
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parent
SUITES = ["helqix_circuit", "helqix_sensors", "helqix_io"]


def ensure_runtime() -> None:
    """Re-run under the local venv when the current interpreter lacks pytest."""
    if importlib.util.find_spec("pytest") is not None:
        return
    venv_python = ROOT / ".venv" / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
    if venv_python.exists() and Path(sys.executable).resolve() != venv_python.resolve():
        os.execv(str(venv_python), [str(venv_python), str(Path(__file__).resolve()), *sys.argv[1:]])
    raise SystemExit("pytest is not installed; create 24-electronics/.venv and install numpy scipy pytest")


SUMMARY = re.compile(
    r"(?:(?P<failed>\d+) failed)?(?:, )?(?:(?P<passed>\d+) passed)?"
    r"(?:, )?(?:(?P<skipped>\d+) skipped)?(?:, )?(?:(?P<errors>\d+) error)?(?:, )?(?:(?P<warnings>\d+) warning)?"
)


def parse_counts(output: str) -> dict[str, int]:
    counts = {"passed": 0, "failed": 0, "skipped": 0, "errors": 0}
    for line in reversed(output.splitlines()):
        line = line.strip()
        if " in " not in line:
            continue
        match = SUMMARY.search(line.split(" in ")[0])
        if match and any(v is not None for v in match.groupdict().values()):
            for key in counts:
                counts[key] = int(match.group(key) or 0)
            return counts
    return counts


def run_suite(name: str, nightly: bool) -> dict[str, object]:
    tests_dir = ROOT / name / "tests"
    if not tests_dir.exists():
        return {"name": name, "status": "NO-TESTS", "seconds": 0.0, **{"passed": 0, "failed": 0, "skipped": 0, "errors": 0}}
    cmd = [sys.executable, "-m", "pytest", str(tests_dir), "-q", "--tb=short", "-p", "no:cacheprovider"]
    if not nightly:
        cmd += ["-m", "not nightly"]
    env = dict(os.environ)
    env["PYTHONPATH"] = str(ROOT) + os.pathsep + env.get("PYTHONPATH", "")
    started = time.perf_counter()
    proc = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True, env=env)
    seconds = time.perf_counter() - started
    counts = parse_counts(proc.stdout + "\n" + proc.stderr)
    ok = proc.returncode == 0
    status = "PASS" if ok else "FAIL"
    result = {"name": name, "status": status, "seconds": seconds, **counts}
    result["output"] = proc.stdout if ok else (proc.stdout + "\n" + proc.stderr)
    return result


def run_mutations() -> tuple[bool, str]:
    module_path = ROOT / "mutation_checks.py"
    if not module_path.exists():
        return True, "none registered"
    sys.path.insert(0, str(ROOT))
    try:
        import mutation_checks

        return mutation_checks.run()
    finally:
        sys.path.remove(str(ROOT))


def run_key_scan() -> tuple[bool, str]:
    scanner = ROOT / "scripts" / "key_scan.py"
    if not scanner.exists():
        return True, "scanner absent"
    proc = subprocess.run([sys.executable, str(scanner), str(ROOT)], capture_output=True, text=True)
    text = (proc.stdout + proc.stderr).strip()
    return proc.returncode == 0, text or "clean"


def run_config_check() -> tuple[bool, str]:
    checker = ROOT / "scripts" / "check_ci.py"
    if not checker.exists():
        return False, "checker absent"
    proc = subprocess.run([sys.executable, str(checker)], capture_output=True, text=True)
    text = (proc.stdout + proc.stderr).strip()
    return proc.returncode == 0, text or "clean"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--nightly", action="store_true", help="include tests marked nightly")
    parser.add_argument("--no-mutations", action="store_true")
    parser.add_argument("--no-keyscan", action="store_true")
    args = parser.parse_args()

    ensure_runtime()
    started = time.perf_counter()
    results = []
    for suite in SUITES:
        result = run_suite(suite, args.nightly)
        results.append(result)
        if result["status"] == "FAIL":
            print(f"\n----- {suite} failure output -----")
            print(result["output"])
            print(f"----- end {suite} -----\n")

    mutation_ok, mutation_text = (True, "skipped") if args.no_mutations else run_mutations()
    key_ok, key_text = (True, "skipped") if args.no_keyscan else run_key_scan()
    config_ok, config_text = run_config_check()
    total = time.perf_counter() - started

    print("=" * 78)
    print(f"HELQIX electronics verification  ({'nightly' if args.nightly else 'fast'} mode)")
    print("=" * 78)
    header = f"{'SUITE':<20}{'TESTS':>7}{'PASS':>7}{'FAIL':>7}{'SKIP':>7}{'SECONDS':>10}  STATUS"
    print(header)
    print("-" * len(header))
    all_green = True
    for r in results:
        tests = r["passed"] + r["failed"] + r["skipped"] + r["errors"]
        if r["status"] != "PASS":
            all_green = False
        print(f"{r['name']:<20}{tests:>7}{r['passed']:>7}{r['failed']:>7}{r['skipped']:>7}{r['seconds']:>10.2f}  {r['status']}")
    print("-" * len(header))
    print(f"{'MUTATIONS':<20}{'':>7}{'':>7}{'':>7}{'':>7}{'':>10}  {'PASS' if mutation_ok else 'FAIL'}  {mutation_text}")
    print(f"{'KEY SCAN':<20}{'':>7}{'':>7}{'':>7}{'':>7}{'':>10}  {'PASS' if key_ok else 'FAIL'}  {key_text}")
    print(f"{'CONFIG':<20}{'':>7}{'':>7}{'':>7}{'':>7}{'':>10}  {'PASS' if config_ok else 'FAIL'}  {config_text}")
    print(f"TOTAL {total:.2f} s")
    print("OVERALL", "PASS" if (all_green and mutation_ok and key_ok and config_ok) else "FAIL")
    return 0 if (all_green and mutation_ok and key_ok and config_ok) else 1


if __name__ == "__main__":
    raise SystemExit(main())
