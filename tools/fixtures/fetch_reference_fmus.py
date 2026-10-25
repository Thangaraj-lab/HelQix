"""Downloads the Modelica Association Reference FMUs (BSD-2-Clause) at a pinned release, checks the
archive SHA-256, and extracts the FMI 2.0 FMUs and their shipped reference outputs into
.cache/reference-fmus/<version>/ (git-ignored). Example data for tests and benchmarks only.

Usage: python -I tools/fixtures/fetch_reference_fmus.py <repo-root>
Prints the directory. Re-running with a matching archive does not download again.
"""
from __future__ import annotations

import hashlib
import io
import sys
import urllib.request
import zipfile
from pathlib import Path

VERSION = "v0.0.42"
URL = f"https://github.com/modelica/Reference-FMUs/releases/download/{VERSION}/Reference-FMUs.zip"
SHA256 = "b6b4a195b9c0f6bdd3459cd949cbb35fc21ee2049d6dab4cb7a3ba7da63b1e97"
REF_DIR = "extra/org.fmi-standard.fmi-ls-ref/"


def main() -> int:
    out = Path(sys.argv[1]) / ".cache" / "reference-fmus" / VERSION
    archive = out / "Reference-FMUs.zip"
    out.mkdir(parents=True, exist_ok=True)
    if not archive.exists() or hashlib.sha256(archive.read_bytes()).hexdigest() != SHA256:
        req = urllib.request.Request(URL, headers={"User-Agent": "helqix-fixtures"})
        with urllib.request.urlopen(req, timeout=120) as r:
            data = r.read()
        digest = hashlib.sha256(data).hexdigest()
        if digest != SHA256:
            print(f"SHA-256 mismatch: expected {SHA256}, got {digest}", file=sys.stderr)
            return 1
        archive.write_bytes(data)
    with zipfile.ZipFile(archive) as z:
        for name in z.namelist():
            if name.startswith("2.0/") and name.endswith(".fmu"):
                fmu_bytes = z.read(name)
                (out / "fmus").mkdir(exist_ok=True)
                (out / "fmus" / Path(name).name).write_bytes(fmu_bytes)
                with zipfile.ZipFile(io.BytesIO(fmu_bytes)) as fmu:
                    for inner in fmu.namelist():
                        if inner.startswith(REF_DIR) and inner.endswith(".csv"):
                            (out / "refs").mkdir(exist_ok=True)
                            (out / "refs" / Path(inner).name).write_bytes(fmu.read(inner))
        (out / "LICENSE.txt").write_bytes(z.read("LICENSE.txt"))
    print(out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
