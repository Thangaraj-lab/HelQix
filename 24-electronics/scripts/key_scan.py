#!/usr/bin/env python
"""Fail when an owned file looks like it contains a credential.

Patterns are assembled at run time so this scanner does not match itself.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

SKIP_DIRS = {".venv", "__pycache__", ".pytest_cache", "node_modules", ".git"}

PATTERNS = [
    re.compile("nva" + r"pi-[A-Za-z0-9_\-]{8,}"),
    re.compile("sk" + r"-(?:ant|proj|live)-[A-Za-z0-9_\-]{8,}"),
    re.compile("AK" + r"IA[0-9A-Z]{16}"),
    re.compile("gh" + r"p_[A-Za-z0-9]{20,}"),
    re.compile("github" + r"_pat_[A-Za-z0-9_]{20,}"),
    re.compile("xo" + r"xb-[A-Za-z0-9\-]{10,}"),
    re.compile("-----BEGIN" + r" [A-Z ]*PRIVATE KEY-----"),
    re.compile(r"(?i)(api[_-]?key|secret|password|token)\s*[:=]\s*[\"'][A-Za-z0-9_\-/+]{16,}[\"']"),
]


def scan(paths: list[Path]) -> list[str]:
    hits: list[str] = []
    for base in paths:
        files = [base] if base.is_file() else sorted(base.rglob("*"))
        for path in files:
            if not path.is_file() or any(part in SKIP_DIRS for part in path.parts):
                continue
            if path.suffix.lower() in {".png", ".jpg", ".jpeg", ".gif", ".pdf", ".npy", ".zip"}:
                continue
            try:
                text = path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            for lineno, line in enumerate(text.splitlines(), 1):
                for pattern in PATTERNS:
                    if pattern.search(line):
                        hits.append(f"{path}:{lineno}: possible credential ({pattern.pattern[:24]}...)")
    return hits


def main(argv: list[str]) -> int:
    targets = [Path(arg) for arg in argv[1:]] or [Path(".")]
    hits = scan(targets)
    for hit in hits:
        print(hit)
    if hits:
        print(f"key scan: {len(hits)} match(es)")
        return 1
    print("key scan: clean")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
