"""Asset licence register gate (K6). Every file in examples/assets/<asset>/ other than LICENSES.json
must have an entry in that folder's LICENSES.json with source, url, license and date_checked, and the
licence must be on the allowlist. Exit 1 with every problem listed.
Usage: python -I tools/assets/check_licenses.py <assets-root>   (default examples/assets)
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ALLOWED = {"CC0-1.0", "CC-BY-4.0", "MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause"}
REQUIRED = ("file", "source", "url", "license", "date_checked")


def check_asset(folder: Path) -> list[str]:
    problems = []
    reg = folder / "LICENSES.json"
    if not reg.is_file():
        return [f"{folder.name}: no LICENSES.json"]
    try:
        doc = json.loads(reg.read_text(encoding="utf-8"))
    except Exception as e:
        return [f"{folder.name}: LICENSES.json is not JSON ({e})"]
    entries = {}
    for i, e in enumerate(doc.get("files", [])):
        missing = [k for k in REQUIRED if not str(e.get(k, "")).strip()]
        if missing:
            problems.append(f"{folder.name}: entry {i} lacks {missing}")
            continue
        if e["license"] not in ALLOWED:
            problems.append(f"{folder.name}/{e['file']}: licence {e['license']} is not allowed")
        if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", e["date_checked"]):
            problems.append(f"{folder.name}/{e['file']}: date_checked must be YYYY-MM-DD")
        entries[e["file"]] = e
    for f in sorted(p for p in folder.rglob("*") if p.is_file() and p.name != "LICENSES.json"):
        rel = f.relative_to(folder).as_posix()
        if rel not in entries:
            problems.append(f"{folder.name}/{rel}: no licence record")
    for name in entries:
        if not (folder / name).is_file():
            problems.append(f"{folder.name}/{name}: listed but missing")
    return problems


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else "examples/assets")
    folders = sorted(p for p in root.iterdir() if p.is_dir()) if root.is_dir() else []
    problems = [p for f in folders for p in check_asset(f)]
    for p in problems:
        print("FAIL", p)
    print(f"{len(folders)} assets checked, {len(problems)} problems")
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
