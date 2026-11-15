#!/usr/bin/env python
"""Check that the automation artifacts exist and say what they must.

This is a text check, not a YAML parser: the goal is to catch a workflow that
silently stopped running the nightly property tests or lost one platform.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parent
WORKFLOW = REPO / ".github" / "workflows" / "priya-electronics.yml"
README = ROOT / "README.md"
PRECOMMIT = ROOT / "scripts" / "precommit.py"
PROPERTIES = [ROOT / "helqix_circuit" / "tests" / "test_properties.py", ROOT / "helqix_sensors" / "tests" / "test_properties.py"]


def check() -> list[str]:
    problems: list[str] = []
    if not WORKFLOW.is_file():
        problems.append(f"missing {WORKFLOW}")
    else:
        text = WORKFLOW.read_text(encoding="utf-8")
        for token, why in [
            ("push:", "no push trigger"),
            ("pull_request:", "no pull_request trigger"),
            ("schedule:", "no nightly schedule"),
            ("cron:", "schedule has no cron line"),
            ("windows-latest", "windows-latest is not in the matrix"),
            ("ubuntu-latest", "ubuntu-latest is not in the matrix"),
            ("verify.py", "the workflow never runs verify.py"),
            ("--nightly", "the nightly job does not pass --nightly"),
        ]:
            if token not in text:
                problems.append(f"workflow: {why}")
    if not README.is_file():
        problems.append(f"missing {README}")
    else:
        text = README.read_text(encoding="utf-8")
        for token in ("venv", "helqix_circuit run", "verify.py"):
            if token not in text:
                problems.append(f"README does not mention {token!r}")
    for path in PROPERTIES:
        if not path.is_file():
            problems.append(f"missing {path}")
            continue
        text = path.read_text(encoding="utf-8")
        if "nightly" not in text:
            problems.append(f"{path.name} is not marked nightly")
        if "SEEDS = list(range(20))" not in text:
            problems.append(f"{path} does not declare twenty seeds")
    if not PRECOMMIT.is_file():
        problems.append(f"missing {PRECOMMIT}")
    else:
        proc = subprocess.run([sys.executable, str(PRECOMMIT), "--selftest"], capture_output=True, text=True)
        if proc.returncode != 0:
            problems.append("precommit selftest failed: " + (proc.stdout + proc.stderr).strip()[:200])
    return problems


def main() -> int:
    problems = check()
    for problem in problems:
        print(problem)
    if problems:
        print(f"config: {len(problems)} problem(s)")
        return 1
    print("config: clean")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
