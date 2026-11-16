#!/usr/bin/env python
"""Bundle one package result folder as electronics-result/0.1.

Usage:
    python scripts/result_bundle.py SRC_DIR OUT_DIR

SRC_DIR holds a results.json written by helqix_circuit or helqix_sensors
(plus values.csv / series.csv when the result has tables). OUT_DIR receives
the copied payload, manifest.json with a SHA-256 per file, and README.md.
Exit 0 on success, 2 on a bad source or a non-fresh OUT_DIR.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from helqix_io.bundle import BundleError, build_bundle


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Bundle one HELQIX result folder.")
    parser.add_argument("source", help="folder holding results.json")
    parser.add_argument("out", help="fresh folder for the bundle")
    args = parser.parse_args(argv)
    try:
        manifest = build_bundle(args.source, args.out)
    except BundleError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    names = ", ".join(entry["name"] for entry in manifest["files"])
    print(f"electronics-result/0.1 bundle at {args.out}: {names}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
