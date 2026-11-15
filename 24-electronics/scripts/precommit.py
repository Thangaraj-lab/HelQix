#!/usr/bin/env python
"""Pre-commit guard: refuse a commit when a staged file looks like a key.

Install once (from the repository root):

    cp 24-electronics/scripts/precommit.py .git/hooks/pre-commit

Git then runs it with no arguments before every commit. It scans the staged
versions of the files (git diff --cached), not the working tree, so a file
that is dirty but not staged cannot block an unrelated commit.

Exit 0 when clean, 1 when a staged file matches a credential pattern.
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from key_scan import scan  # noqa: E402


def staged_files() -> list[Path]:
    try:
        proc = subprocess.run(
            ["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR"],
            capture_output=True, text=True, check=True,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        print(f"precommit: cannot list staged files: {exc}", file=sys.stderr)
        return []
    return [Path(line) for line in proc.stdout.splitlines() if line.strip() and Path(line).is_file()]


def selftest() -> int:
    fake = "nva" + "pi-" + "a" * 24
    with tempfile.TemporaryDirectory(prefix="precommit-") as tmp:
        root = Path(tmp)
        clean = root / "clean.txt"
        clean.write_text("ordinary notes\n", encoding="utf-8")
        hits = scan([clean])
        if hits:
            print("selftest: clean file was flagged", file=sys.stderr)
            return 1
        dirty = root / "dirty.txt"
        dirty.write_text(f"token = '{fake}'\n", encoding="utf-8")
        hits = scan([dirty])
        if not hits:
            print("selftest: a planted key was not found", file=sys.stderr)
            return 1
    print("precommit selftest: clean")
    return 0


def main(argv: list[str]) -> int:
    if "--selftest" in argv:
        return selftest()
    hits = scan(staged_files())
    for hit in hits:
        print(hit)
    if hits:
        print(f"precommit: blocked, {len(hits)} possible credential(s) staged")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
