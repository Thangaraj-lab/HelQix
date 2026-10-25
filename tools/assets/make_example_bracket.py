"""Writes the Example source model examples/assets/example-bracket/source.obj: an L-shaped bracket
(two plates, metres), authored here and released as CC0-1.0. Deterministic: same output every run.
Usage: python -I tools/assets/make_example_bracket.py <repo-root>
"""
from __future__ import annotations

import sys
from pathlib import Path


def box(x0, y0, z0, x1, y1, z1):
    v = [(x, y, z) for x in (x0, x1) for y in (y0, y1) for z in (z0, z1)]
    # Faces as quads, counter-clockwise seen from outside (vertex index = 4*ix + 2*iy + iz).
    f = [(0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)]
    return v, f


def main() -> int:
    out = Path(sys.argv[1]) / "examples" / "assets" / "example-bracket" / "source.obj"
    out.parent.mkdir(parents=True, exist_ok=True)
    parts = [box(0.0, 0.0, 0.0, 0.10, 0.06, 0.006), box(0.0, 0.0, 0.006, 0.006, 0.06, 0.08)]  # starts on top of the base plate: no overlapping faces
    lines = ["# HELQIX Example asset: L bracket, 100 x 60 x 80 mm, 6 mm plate. CC0-1.0. Units: metres.", "o example_bracket"]
    offset = 1
    for verts, faces in parts:
        lines += [f"v {x:.6f} {y:.6f} {z:.6f}" for x, y, z in verts]
        lines += ["f " + " ".join(str(offset + i) for i in face) for face in faces]
        offset += len(verts)
    out.write_text("\n".join(lines) + "\n", encoding="ascii")
    print(out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
