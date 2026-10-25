"""K6 asset pipeline for one asset folder (examples/assets/<name>/ with asset.json and LICENSES.json):

  1. licence register gate (check_licenses.py)
  2. Blender build (separate process): import, PBR material, UV, Cycles AO bake, GLB export  -> raw.glb
  3. glTF-Transform optimize: meshopt compression, no simplification (geometry is engineering data);
     KTX2 textures when `toktx` (KTX-Software) is on PATH, required with --require-ktx2        -> <name>.glb
  4. Khronos glTF-Validator gate: zero errors
  5. size budget from asset.json (default 2 MB)
  6. Cycles render with Open Image Denoise (thumbnail; hero with --hero) from raw.glb, because
     Blender cannot import meshopt or KTX2; the geometry is identical. Label: Rendered preview.

Outputs go to <out>/<name>/<key>/, where <key> is a SHA-256 over the source files, the scripts and the
tool versions; an existing complete output is reused (`cached: true`). Writes report.json there.
Usage: python -I tools/assets/pipeline.py <asset-dir> [--out .cache/assets] [--blender PATH] [--require-ktx2] [--hero]
"""
from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))
from check_licenses import check_asset  # noqa: E402

GLTF_TRANSFORM = ["node", str(HERE / "node_modules" / "@gltf-transform" / "cli" / "bin" / "cli.js")]


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace", **kw)


def default_blender() -> str:
    for c in (os.environ.get("HELQIX_BLENDER"), shutil.which("blender"), r"C:\Program Files\Blender Foundation\Blender 5.2\blender.exe"):
        if c and Path(c).exists():
            return c
    raise SystemExit("Blender not found: set HELQIX_BLENDER or pass --blender")


def marker(output: str, name: str) -> dict:
    for line in output.splitlines():
        if line.startswith(name + "="):
            return json.loads(line[len(name) + 1:])
    raise RuntimeError(f"{name} not found in output:\n{output[-2000:]}")


def cache_key(asset_dir: Path, versions: dict, flags: dict) -> str:
    h = hashlib.sha256()
    for f in sorted(p for p in asset_dir.rglob("*") if p.is_file()):
        h.update(f.relative_to(asset_dir).as_posix().encode() + b"\0" + f.read_bytes() + b"\0")
    for script in (HERE / "build_asset.py", HERE / "pipeline.py", HERE / "validate.mjs", ROOT / "tools" / "render" / "render.py", HERE / "package-lock.json"):
        h.update(script.read_bytes())
    h.update(json.dumps([versions, flags], sort_keys=True).encode())
    return h.hexdigest()


def main() -> int:
    argv = sys.argv[1:]
    if not argv:
        print(__doc__)
        return 1
    asset_dir = Path(argv[0]).resolve()
    out_root = Path(argv[argv.index("--out") + 1]) if "--out" in argv else ROOT / ".cache" / "assets"
    blender = argv[argv.index("--blender") + 1] if "--blender" in argv else default_blender()
    require_ktx2, hero = "--require-ktx2" in argv, "--hero" in argv
    spec = json.loads((asset_dir / "asset.json").read_text(encoding="utf-8"))
    name = spec["name"]

    problems = check_asset(asset_dir)
    if problems:
        print("licence register FAILED:\n  " + "\n  ".join(problems))
        return 3

    toktx = shutil.which("toktx")
    if require_ktx2 and not toktx:
        print("KTX2 required but toktx (KTX-Software) is not on PATH")
        return 4
    versions = {
        "blender": run([blender, "--version"]).stdout.splitlines()[0].strip(),
        "gltf_transform": run(GLTF_TRANSFORM + ["--version"]).stdout.strip().splitlines()[-1],
        "toktx": run([toktx, "--version"]).stdout.strip() if toktx else None,
    }
    flags = {"ktx2": bool(toktx), "hero": hero}
    key = cache_key(asset_dir, versions, flags)
    out = out_root / name / key[:16]
    report_path = out / "report.json"
    if report_path.is_file():
        report = json.loads(report_path.read_text(encoding="utf-8"))
        report["cached"] = True
        print(json.dumps(report, indent=2))
        return 0 if report.get("pass") else 2
    out.mkdir(parents=True, exist_ok=True)
    t0 = time.time()

    raw = out / "raw.glb"
    b = run([blender, "-b", "--factory-startup", "--python", str(HERE / "build_asset.py"), "--", str(asset_dir / "asset.json"), str(raw), str(out / "work")])
    if b.returncode != 0 or not raw.exists():
        print("Blender build failed:\n" + (b.stdout + b.stderr)[-3000:])
        return 5
    build = marker(b.stdout, "HELQIX_BUILD")

    final = out / f"{name}.glb"
    opt = GLTF_TRANSFORM + ["optimize", str(raw), str(final), "--compress", "meshopt", "--simplify", "false",
                            "--texture-compress", "ktx2" if toktx else "false"]
    t = time.time()
    o = run(opt)
    if o.returncode != 0:
        print("glTF-Transform failed:\n" + (o.stdout + o.stderr)[-3000:])
        return 6
    optimize_seconds = round(time.time() - t, 3)

    v = run(["node", str(HERE / "validate.mjs"), str(final)], cwd=HERE)
    validation = json.loads(v.stdout.strip().splitlines()[-1]) if v.stdout.strip() else {"errors": None, "messages": [v.stderr[-1000:]]}
    ktx2_ok = (not require_ktx2) or (validation.get("info", {}).get("textures") and all(t == "image/ktx2" for t in validation["info"]["textures"]))
    size_ok = final.stat().st_size <= int(spec.get("budget_bytes", 2_000_000))

    renders = {}
    for mode in (["thumbnail", "hero"] if hero else ["thumbnail"]):
        png = out / f"{name}.{mode}.png"
        r = run([blender, "-b", "--factory-startup", "--python", str(ROOT / "tools" / "render" / "render.py"), "--", str(raw), str(png), mode])
        if r.returncode != 0 or not png.exists():
            print(f"render {mode} failed:\n" + (r.stdout + r.stderr)[-3000:])
            return 7
        renders[mode] = marker(r.stdout, "HELQIX_RENDER") | {"file": png.name}

    shutil.copyfile(asset_dir / "LICENSES.json", out / "LICENSES.json")
    passed = v.returncode == 0 and validation.get("errors") == 0 and size_ok and bool(ktx2_ok)
    report = {
        "schema": "helqix.asset-report.v1", "asset": name, "label": spec.get("label", "Example"), "key": key, "cached": False, "pass": passed,
        "versions": versions, "build": build, "optimize_seconds": optimize_seconds,
        "bytes": {"raw_glb": raw.stat().st_size, "final_glb": final.stat().st_size, "budget": int(spec.get("budget_bytes", 2_000_000))},
        "size_within_budget": size_ok, "textures_ktx2": bool(toktx), "ktx2_required": require_ktx2,
        "validation": validation, "renders": renders, "seconds_total": round(time.time() - t0, 3),
        "note": "Renders are Rendered previews (Derived), not engineering evidence. Geometry is not simplified.",
    }
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if passed else 2


if __name__ == "__main__":
    raise SystemExit(main())
