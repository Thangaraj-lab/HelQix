# Asset and render pipeline (K6)

Rendered pixels are **Derived**: previews, never engineering evidence. Geometry is never simplified by the pipeline.

## Run

```bash
cd tools/assets && npm ci && cd ../..
python -I tools/assets/pipeline.py examples/assets/<asset> [--hero] [--require-ktx2] [--blender <path>]
python -I tools/assets/check_licenses.py examples/assets
```

An asset folder holds the source model, `asset.json` (name, label, source file, units, PBR material, AO bake settings, size budget) and `LICENSES.json`.

## Steps

| Step | Tool (licence) | What it does |
|---|---|---|
| 1. Licence register | `check_licenses.py` | Every file in the asset folder needs an entry (source, URL, licence, date checked) with an allowed licence (CC0, CC-BY-4.0, MIT, Apache-2.0, BSD). Otherwise the pipeline stops. |
| 2. Build | Blender 5.2.1 (GPL), **separate process** via `build_asset.py` | Import OBJ/glTF/STL (metres, Z up), join, outward normals, PBR material from the spec, UV unwrap, **Cycles ambient-occlusion bake** wired to glTF's occlusion slot, GLB export |
| 3. Optimise | glTF-Transform 4.5.1 (MIT) | `optimize --compress meshopt --simplify false`; **KTX2** textures (`--texture-compress ktx2`) when KTX-Software `toktx` 4.4.2 (Apache-2.0) is installed, required with `--require-ktx2` |
| 4. Validate (gate) | Khronos glTF-Validator 2.0.0-dev.3.10 (Apache-2.0) via `validate.mjs` | Fails on any error |
| 4b. KTX2 check | `pipeline.py` reads the GLB itself | With `--require-ktx2`, every image must declare `image/ktx2` **and** start with the 12-byte KTX2 signature. The pinned validator predates KTX2 support: it reports `image/ktx2` as two warnings ("not in list", "format not recognized"), so the KTX2 decision cannot come from it. |
| 5. Budget | `asset.json` | Fails if the final GLB is over budget (default 2 MB) |
| 6. Render | Blender Cycles + **Intel Open Image Denoise**, `tools/render/render.py` | Thumbnail 256×256 (32 samples); hero 1280×720 (128 samples). A `.glb` gets a neutral studio and an auto-framed camera; a `.blend` scene renders with its own camera. Rendered from the pre-compression GLB, because Blender cannot import meshopt or KTX2 (same geometry). |

**Cache:** outputs go to `.cache/assets/<asset>/<key>/` (git-ignored). The key is a SHA-256 over the asset folder's files, the pipeline scripts, the npm lockfile, the tool versions and the flags. A complete earlier result is reused (`"cached": true`). The output folder holds the final GLB, the renders, a copy of `LICENSES.json` and `report.json` (versions, timings, sizes, validator summary, labels).

## Example asset

`examples/assets/example-bracket/`: an L bracket, 100 × 60 × 80 mm, 6 mm plate, written by `tools/assets/make_example_bracket.py` and released CC0-1.0. Labelled **Example**. The first version overlapped the two plates and rendered a dark artifact at the corner, so the upright plate now starts on top of the base plate.

## Measured

Founder's PC (AMD laptop CPU, Blender 5.2.1, no KTX2 tool installed; MEASURED):

| Item | Value |
|---|---|
| Triangles | 24 |
| Build (import, material, UV, AO bake 512 px at 32 samples, export) | 2.1 s (AO bake 0.63 s) |
| Raw GLB / final GLB (meshopt + quantization, PNG occlusion texture) | 64 564 B / 64 528 B (the 60 KB AO texture dominates; KTX2 is what shrinks it) |
| Validator | 0 errors, 0 warnings, 1 info |
| Thumbnail render 256×256, 32 samples, OIDN | 1.05 s |
| Hero render 1280×720, 128 samples, OIDN | 8.6 s |
| Cached re-run | 2 s (tool version probes only) |
| Cycles built with OIDN (`_cycles.with_openimagedenoise`) | true |

CI (Linux, KTX2 required) publishes its own numbers as a notice.

## Licence gates

- `check_licenses.py`: tested (the committed example passes; a file without a record, a disallowed licence, an incomplete record, a missing register and a listed-but-missing file all fail).
- The JS licence gate now judges **whole SPDX expressions** (`scripts/js_license_eval.py`): `AND` needs every part allowed, `OR` needs one. The K1 version accepted any expression that merely contained an allowed name, so "MIT AND GPL-3.0" would have passed; that is fixed and tested. LGPL parts pass only for listed, dynamically loaded libraries. Today that is `@img/sharp-*` (libvips, loaded by glTF-Transform's image tools at build time, not shipped).

## Not done

Hero plates from a hand-made `.blend` scene are supported but no scene is committed. No HDRI environment lighting, so metal looks flat. No asset with textures from an outside source yet (the register is ready for it). Parts without a model stay PLACEHOLDER MODEL in the UI (UI track).
