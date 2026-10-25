"""Second K0 pass: for entries whose registry gave no clear SPDX id, read the licence file itself
(raw file hosts, not rate-limited APIs) and classify it from its text. Updates tools/TOOLS.lock.json
in place and records which file was read. Text that matches no rule stays UNKNOWN.

Usage: python -I tools/register/read_license_files.py <repo-root>
"""
from __future__ import annotations

import json
import re
import sys
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from fetch_tools import verdict  # noqa: E402

UA = {"User-Agent": "helqix-tool-register/0.1 (license audit)"}
NAMES = ["COPYING.LESSER", "LICENSE", "COPYRIGHT", "LICENSE.txt", "LICENSE.md", "COPYING", "COPYING.txt", "LICENSE-APACHE", "COPYING.LESSER", "LICENSE.LGPL"]
# Where the registry source is not a GitHub repo, the repo or page that holds the licence text.
EXTRA = {
    "FMPy": "github:CATIA-Systems/FMPy",
    "PyBaMM": "github:pybamm-team/PyBaMM",
    "SciPy": "github:scipy/scipy",
    "OpenTURNS": "github:openturns/openturns",
    "Qiskit Aer": "github:Qiskit/qiskit-aer",
    "Garage": "url:https://git.deuxfleurs.fr/Deuxfleurs/garage/raw/branch/main/LICENSE",
    "CalculiX": "url:http://www.dhondt.de/",
    "Blender": "url:https://raw.githubusercontent.com/blender/blender/HEAD/doc/license/GPL3-license.txt",
    "OpenModelica": "url:https://raw.githubusercontent.com/OpenModelica/OpenModelica/HEAD/OSMC-License.txt",
    "Modelica Reference FMUs": "url:https://raw.githubusercontent.com/modelica/Reference-FMUs/HEAD/LICENSE.txt",
}

# Licence-family titles are matched only near the top of the file: the GPL-3 text itself mentions
# the Affero licence in section 13, and LGPL texts mention the GPL.
TITLE_CHARS = 600
RULES = [
    (r"AGPL VERSION 3 LICENSE OR\s+\*?\s*THIS OSMC PUBLIC LICENSE", "AGPL-3.0 OR OSMC-PL-1.8"),
    (r"GNU AFFERO GENERAL PUBLIC LICENSE", "AGPL-3.0"),
    (r"Server Side Public License", "SSPL-1.0"),
    (r"GNU LESSER GENERAL PUBLIC LICENSE\s+Version 3", "LGPL-3.0"),
    (r"GNU LESSER GENERAL PUBLIC LICENSE\s+Version 2\.1", "LGPL-2.1"),
    (r"GNU (LIBRARY|LESSER) GENERAL PUBLIC", "LGPL-2.1"),
    (r"OSMC PUBLIC LICENSE|Open Source Modelica Consortium Public License", "OSMC-PL (GPL-3.0 option)"),
    (r"GNU GENERAL PUBLIC LICENSE\s+Version 3", "GPL-3.0"),
    (r"GNU GENERAL PUBLIC LICENSE\s+Version\s+2", "GPL-2.0"),
    (r"released under the 2-Clause BSD license", "BSD-2-Clause"),
    (r"Mozilla Public License,? [Vv]ersion 2\.0", "MPL-2.0"),
    (r"Apache License,?\s+Version 2\.0", "Apache-2.0"),
    (r"PostgreSQL (Global Development Group|License)[\s\S]{0,600}Permission to use, copy, modify, and distribute", "PostgreSQL"),
    (r"Permission is hereby granted, free of charge", "MIT"),
    (r"Redistribution and use in source and binary forms[\s\S]{0,900}Neither the name", "BSD-3-Clause"),
    (r"Redistribution and use in source and binary forms", "BSD-2-Clause"),
]


def read(url: str) -> str | None:
    try:
        with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=30) as r:
            return r.read(200_000).decode("utf-8", "replace")
    except Exception:
        return None


def hits(text: str, page: bool = False) -> list[str]:
    # A web page (not a licence file) states its licence anywhere in the body.
    head = text if page else text[:TITLE_CHARS]
    found = []
    for pattern, spdx in RULES:
        family = "GENERAL PUBLIC" in pattern or "AFFERO" in pattern
        if re.search(pattern, head if family and "OSMC" not in pattern else text, re.IGNORECASE) and spdx not in found:
            found.append(spdx)
    if "BSD-3-Clause" in found and "BSD-2-Clause" in found:
        found.remove("BSD-2-Clause")
    if "AGPL-3.0 OR OSMC-PL-1.8" in found:
        found = ["AGPL-3.0 OR OSMC-PL-1.8"]
    return found


def classify(text: str, page: bool = False) -> str | None:
    h = hits(text, page)
    return h[0] if h else None


def main() -> int:
    root = Path(sys.argv[1])
    p = root / "tools" / "TOOLS.lock.json"
    doc = json.loads(p.read_text(encoding="utf-8"))
    for t in doc["tools"]:
        lic = t.get("license")
        clear = not t.get("license_source") and lic and lic not in ("NOASSERTION", "Other") and not lic.startswith("BSD License") and lic not in ("LGPL", "Apache 2.0")
        if clear:
            continue
        src = EXTRA.get(t["name"]) or (f"github:{t['id']}" if t["source"] == "github" else None)
        if not src:
            continue
        candidates = [src[4:]] if src.startswith("url:") else [f"https://raw.githubusercontent.com/{src[7:]}/HEAD/{n}" for n in NAMES]
        for url in candidates:
            text = read(url)
            page = url.endswith('/')
            spdx = classify(text, page) if text else None
            if spdx:
                # FFmpeg and similar ship several licences: record every rule that matched.
                all_hits = hits(text, page)
                t.update(license=spdx if len(all_hits) == 1 else " / ".join(all_hits), license_source=url, status="VERIFIED", license_method="licence text read and classified")
                t.pop("error", None)
                break
        t["verdict"] = verdict(t.get("license"), t.get("boundary"))
    p.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    for t in doc["tools"]:
        if t.get("license_source") or t["status"] != "VERIFIED" or t["verdict"].startswith("unknown"):
            print(f"{t['status']:8} {t['name'][:28]:28} {str(t.get('license'))[:44]:44} {t['verdict']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
