"""Evaluates npm licence expressions from `license-checker-rseidelsohn --json` (stdin) against the
allowlist. SPDX semantics: `A OR B` passes if either side passes; `A AND B` only if both do.
LGPL parts pass only for packages listed in LGPL_DYNAMIC (dynamically loaded native libraries, used
at build time). Missing or unparsable licences fail. Exit 1 listing every failure.
Usage: npx license-checker-rseidelsohn --production --json | python -I scripts/js_license_eval.py
"""
from __future__ import annotations

import json
import re
import sys

ALLOW = {"MIT", "MIT-0", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "MPL-2.0", "0BSD", "CC0-1.0", "Zlib",
         "BlueOak-1.0.0", "Python-2.0", "Unlicense", "CC-BY-4.0"}
LGPL = {"LGPL-2.1", "LGPL-2.1-only", "LGPL-2.1-or-later", "LGPL-3.0", "LGPL-3.0-only", "LGPL-3.0-or-later"}
# Package-name prefixes whose LGPL part is accepted, with the reason (file 39 rule 3: LGPL, dynamic or separate use).
LGPL_DYNAMIC = {
    "@img/sharp-": "prebuilt libvips loaded at run time by sharp (glTF-Transform texture tools); build-time only, not shipped",
}
# SIL Open Font License 1.1: allowed only for font packages. It governs the font files and does not
# reach the application that uses them. Any other package under OFL is refused.
OFL_FONTS = {
    "@fontsource/": "self-hosted IBM Plex and Sora fonts for the Studio UI (file 37 section 3)",
}


def tokens(expr: str) -> list[str]:
    return re.findall(r"\(|\)|[^\s()]+", expr)


def allowed(expr: str, package: str) -> bool:
    toks = tokens(expr)
    pos = 0

    def atom() -> bool:
        nonlocal pos
        if pos >= len(toks):
            raise ValueError("unexpected end")
        t = toks[pos]
        pos += 1
        if t == "(":
            v = disj()
            if pos >= len(toks) or toks[pos] != ")":
                raise ValueError("missing )")
            pos += 1
            return v
        lic = t.rstrip("*")  # license-checker marks guessed licences with '*'; the guess is still judged
        if lic in ALLOW:
            return True
        if lic in LGPL:
            return any(package.startswith(p) for p in LGPL_DYNAMIC)
        if lic == "OFL-1.1":
            return any(package.startswith(p) for p in OFL_FONTS)
        return False

    def conj() -> bool:
        nonlocal pos
        v = atom()
        while pos < len(toks) and toks[pos].upper() == "AND":
            pos += 1
            v = atom() and v
        return v

    def disj() -> bool:
        nonlocal pos
        v = conj()
        while pos < len(toks) and toks[pos].upper() == "OR":
            pos += 1
            v = conj() or v
        return v

    result = disj()
    if pos != len(toks):
        raise ValueError(f"trailing tokens in {expr!r}")
    return result


def evaluate(packages: dict) -> list[str]:
    failures = []
    for name, info in sorted(packages.items()):
        if name.startswith("@helqix/"):
            continue  # HELQIX's own packages: licence is a founder decision (docs/TOOLS.md)
        lic = info.get("licenses")
        if isinstance(lic, list):
            lic = " AND ".join(lic)
        if not lic or str(lic).upper() in ("UNKNOWN", "UNLICENSED", "CUSTOM"):
            failures.append(f"{name}: no usable licence ({lic})")
            continue
        try:
            ok = allowed(str(lic), name)
        except ValueError as e:
            failures.append(f"{name}: cannot parse licence {lic!r} ({e})")
            continue
        if not ok:
            failures.append(f"{name}: licence {lic!r} is not allowed")
    return failures


def main() -> int:
    packages = json.load(sys.stdin)
    failures = evaluate(packages)
    for f in failures:
        print("FAIL", f)
    lgpl = [n for n, i in packages.items() if any(x in str(i.get("licenses")) for x in ("LGPL",))]
    for n in lgpl:
        reason = next((r for p, r in LGPL_DYNAMIC.items() if n.startswith(p)), None)
        if reason:
            print(f"LGPL accepted: {n} ({reason})")
    print(f"{len(packages)} packages checked, {len(failures)} failures")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
