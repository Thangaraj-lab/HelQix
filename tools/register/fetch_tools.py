"""K0 tool register: reads license, latest version and release date from each tool's primary
registry (crates.io, PyPI, npm, GitHub or Forgejo API) and writes tools/TOOLS.lock.json.

Usage: python -I tools/register/fetch_tools.py <repo-root>
Network read-only. A field that could not be fetched is recorded as UNKNOWN, never guessed.
"""
from __future__ import annotations

import json
import sys
import time
import urllib.request
from pathlib import Path

UA = {"User-Agent": "helqix-tool-register/0.1 (license audit)"}

# (slot, name, role, source, id, separate_process_only_note)
TOOLS = [
    # Render / stream (T2)
    ("render-stream", "Playwright", "headless Chromium driver", "npm", "playwright", None),
    ("render-stream", "Chromium", "headless GPU browser", "github", "chromium/chromium", None),
    ("render-stream", "GStreamer", "capture and encode pipeline", "github", "GStreamer/gstreamer", None),
    ("render-stream", "x264", "software H.264 fallback", "github", "mirror/x264", None),
    ("render-stream", "LiveKit server", "WebRTC SFU", "github", "livekit/livekit", None),
    ("render-stream", "mediasoup", "WebRTC SFU (alternative)", "npm", "mediasoup", None),
    ("render-stream", "coturn", "TURN server", "github", "coturn/coturn", None),
    ("render-stream", "Selkies", "GStreamer WebRTC reference design", "github", "selkies-project/selkies", None),
    ("render-stream", "Docker Engine (Moby)", "containers", "github", "moby/moby", None),
    ("render-stream", "NVIDIA Container Toolkit", "GPU in containers", "github", "NVIDIA/nvidia-container-toolkit", None),
    # Cinematic / assets (T3)
    ("assets", "Blender", "asset build and Cycles renders (CLI only)", "github", "blender/blender", "GPL: separate process via CLI only"),
    ("assets", "Intel Open Image Denoise", "render denoiser", "github", "RenderKit/oidn", None),
    ("assets", "FFmpeg", "video encode of replays", "github", "FFmpeg/FFmpeg", "LGPL build only, as a separate process; GPL parts (x264) only as a separate process"),
    ("assets", "glTF-Transform", "glTF optimize", "npm", "@gltf-transform/core", None),
    ("assets", "meshoptimizer", "mesh LOD and compression", "github", "zeux/meshoptimizer", None),
    ("assets", "Draco", "geometry compression", "github", "google/draco", None),
    ("assets", "Basis Universal", "KTX2 textures", "github", "BinomialLLC/basis_universal", None),
    ("assets", "Khronos glTF-Validator", "asset QA gate", "github", "KhronosGroup/glTF-Validator", None),
    # Client renderer
    ("client", "three.js", "renderer", "npm", "three", None),
    ("client", "three-gpu-pathtracer", "T1+ progressive path tracing", "npm", "three-gpu-pathtracer", None),
    ("client", "detect-gpu", "GPU tier detection", "npm", "detect-gpu", None),
    # Solvers
    ("solver", "FMPy", "FMI/FMU runner", "pypi", "fmpy", None),
    ("solver", "OpenModelica", "Modelica compiler (FMU export)", "github", "OpenModelica/OpenModelica", "OSMC-PL licence: verify the GPL option; separate process only"),
    ("solver", "Modelica Reference FMUs", "test FMUs for the FMI adapter", "github", "modelica/Reference-FMUs", None),
    ("solver", "PyBaMM", "battery models", "pypi", "pybamm", None),
    ("solver", "CalculiX", "FEA (ccx)", "manual", "http://www.dhondt.de/", "GPL: separate process only"),
    ("solver", "SUNDIALS", "ODE/DAE integrators", "github", "LLNL/sundials", None),
    ("solver", "SciPy", "numerics", "pypi", "scipy", None),
    ("solver", "Pint", "units", "pypi", "pint", None),
    ("solver-research", "FEniCSx (DOLFINx)", "FEM research", "github", "FEniCS/dolfinx", None),
    ("solver-research", "MOOSE", "multiphysics research", "github", "idaholab/moose", None),
    ("solver-research", "Rapier", "deterministic physics", "crates", "rapier3d", None),
    ("solver-research", "Jolt Physics", "physics", "github", "jrouwe/JoltPhysics", None),
    # Trust
    ("trust", "cosign", "artifact signing", "github", "sigstore/cosign", None),
    ("trust", "Rekor v2 (rekor-tiles)", "transparency log", "github", "sigstore/rekor-tiles", None),
    ("trust", "in-toto", "supply-chain attestations", "pypi", "in-toto", None),
    ("trust", "SLSA GitHub generator", "build provenance", "github", "slsa-framework/slsa-github-generator", None),
    ("trust", "BLAKE3", "content addressing", "crates", "blake3", None),
    ("trust", "SALib", "sensitivity analysis", "pypi", "SALib", None),
    ("trust", "OpenTURNS", "uncertainty quantification", "pypi", "openturns", None),
    ("trust", "Hypothesis", "property tests (Python)", "pypi", "hypothesis", None),
    ("trust", "proptest", "property tests (Rust)", "crates", "proptest", None),
    # Platform
    ("platform", "axum", "HTTP API", "crates", "axum", None),
    ("platform", "tokio", "async runtime", "crates", "tokio", None),
    ("platform", "serde", "serialization", "crates", "serde", None),
    ("platform", "schemars", "JSON schema generation", "crates", "schemars", None),
    ("platform", "sqlx", "Postgres access", "crates", "sqlx", None),
    ("platform", "PostgreSQL", "metadata store", "github", "postgres/postgres", None),
    ("platform", "pgvector", "vector search", "github", "pgvector/pgvector", None),
    ("platform", "NATS server", "messaging", "github", "nats-io/nats-server", None),
    ("platform", "Temporal", "workflows (render sessions)", "github", "temporalio/temporal", None),
    ("platform", "Apache Arrow (Rust)", "columnar frames", "crates", "arrow", None),
    ("platform", "Parquet (Rust)", "frame storage", "crates", "parquet", None),
    ("platform", "DuckDB (Rust)", "analysis queries", "crates", "duckdb", None),
    ("platform", "Zarr", "field arrays", "pypi", "zarr", None),
    ("platform", "Garage", "S3-compatible object store", "forgejo", "git.deuxfleurs.fr/Deuxfleurs/garage", None),
    ("platform", "SeaweedFS", "S3-compatible object store (alternative)", "github", "seaweedfs/seaweedfs", None),
    ("platform", "Keycloak", "identity", "github", "keycloak/keycloak", None),
    ("platform", "Ory Kratos", "identity (alternative)", "github", "ory/kratos", None),
    ("platform", "OpenTelemetry Collector", "observability", "github", "open-telemetry/opentelemetry-collector", None),
    # Quantum (K9, later)
    ("quantum", "Qiskit", "quantum SDK", "pypi", "qiskit", None),
    ("quantum", "Qiskit Aer", "quantum simulator", "pypi", "qiskit-aer", None),
    ("quantum", "PennyLane", "quantum SDK", "pypi", "pennylane", None),
    # Quality gates
    ("gates", "cargo-deny", "Rust license/advisory gate", "crates", "cargo-deny", None),
    ("gates", "license-checker-rseidelsohn", "JS license gate", "npm", "license-checker-rseidelsohn", None),
    ("gates", "Vitest", "JS unit tests", "npm", "vitest", None),
    ("gates", "@playwright/test", "end-to-end tests", "npm", "@playwright/test", None),
    ("gates", "axe-core", "accessibility tests", "npm", "axe-core", None),
    # Named as forbidden by the plan; checked so the verdict is evidence-based
    ("forbidden-check", "MinIO", "object store (named forbidden)", "github", "minio/minio", None),
]

ALLOWED = {"MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "MPL-2.0", "Zlib", "CC0-1.0", "PostgreSQL", "BSL-1.0", "Unlicense", "0BSD"}
WEAK_COPYLEFT = {"LGPL-2.1", "LGPL-2.1-only", "LGPL-2.1-or-later", "LGPL-3.0", "LGPL-3.0-only", "LGPL-3.0-or-later"}
GPL = {"GPL-2.0", "GPL-2.0-only", "GPL-2.0-or-later", "GPL-3.0", "GPL-3.0-only", "GPL-3.0-or-later"}
FORBIDDEN = {"AGPL-3.0", "AGPL-3.0-only", "AGPL-3.0-or-later", "SSPL-1.0", "BUSL-1.1", "Elastic-2.0"}


def get(url: str):
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.loads(r.read().decode("utf-8"))


def fetch(source: str, ident: str) -> dict:
    if source == "crates":
        d = get(f"https://crates.io/api/v1/crates/{ident}")
        v = d["crate"]["max_stable_version"] or d["crate"]["max_version"]
        ver = next((x for x in d["versions"] if x["num"] == v), d["versions"][0])
        return {"license": ver.get("license"), "version": v, "released": ver["created_at"][:10], "url": f"https://crates.io/crates/{ident}"}
    if source == "pypi":
        d = get(f"https://pypi.org/pypi/{ident}/json")
        info = d["info"]
        files = d["releases"].get(info["version"], [])
        lic = info.get("license_expression") or (info.get("license") if info.get("license") and len(info.get("license")) < 60 else None)
        if not lic:
            lic = "; ".join(c.split(" :: ")[-1] for c in info.get("classifiers", []) if c.startswith("License ::")) or None
        return {"license": lic, "version": info["version"], "released": files[0]["upload_time"][:10] if files else None, "url": f"https://pypi.org/project/{ident}/"}
    if source == "npm":
        d = get(f"https://registry.npmjs.org/{ident.replace('/', '%2F')}")
        v = d["dist-tags"]["latest"]
        lic = d["versions"][v].get("license")
        if isinstance(lic, dict):
            lic = lic.get("type")
        return {"license": lic, "version": v, "released": d["time"][v][:10], "url": f"https://www.npmjs.com/package/{ident}"}
    if source == "github":
        d = get(f"https://api.github.com/repos/{ident}")
        out = {"license": (d.get("license") or {}).get("spdx_id"), "last_commit": d.get("pushed_at", "")[:10], "url": d["html_url"], "archived": d.get("archived")}
        try:
            rel = get(f"https://api.github.com/repos/{ident}/releases/latest")
            out.update(version=rel.get("tag_name"), released=(rel.get("published_at") or "")[:10])
        except Exception:
            out.update(version=None, released=None)
        return out
    if source == "forgejo":
        host, repo = ident.split("/", 1)
        d = get(f"https://{host}/api/v1/repos/{repo}")
        rel = get(f"https://{host}/api/v1/repos/{repo}/releases?limit=1")
        lic = None
        try:
            lic = ", ".join(d.get("licenses") or []) or None
        except Exception:
            pass
        return {"license": lic, "last_commit": (d.get("updated_at") or "")[:10], "version": rel[0]["tag_name"] if rel else None,
                "released": (rel[0].get("published_at") or "")[:10] if rel else None, "url": d.get("html_url")}
    raise ValueError(source)


def verdict(lic: str | None, note: str | None) -> str:
    if not lic or lic in ("NOASSERTION", "UNKNOWN", "Other"):
        return "unknown: check manually"
    parts = [p.strip() for p in lic.replace("(", " ").replace(")", " ").replace(" OR ", "|").replace(" or ", "|").replace("/", "|").split("|") if p.strip()]
    norm = lambda p: p.replace(" License", "").replace("MIT", "MIT").strip()
    names = {norm(p) for p in parts}
    if "OSMC-PL-1.8" in lic:
        return "forbidden unless the founder accepts OSMC-PL 1.8 (the alternative is AGPL-3.0, forbidden)"
    if names & FORBIDDEN and not names & (ALLOWED | WEAK_COPYLEFT):
        return "forbidden"
    if names & ALLOWED:
        return "allowed"
    if names & WEAK_COPYLEFT:
        return "allowed (LGPL: dynamic or separate use only)"
    if names & GPL:
        return "separate-process-only"
    return "unknown: check manually"


def main() -> int:
    root = Path(sys.argv[1])
    out = []
    for slot, name, role, source, ident, note in TOOLS:
        rec = {"slot": slot, "name": name, "role": role, "source": source, "id": ident}
        if source == "manual":
            rec.update(license=None, version=None, released=None, url=ident, status="UNKNOWN", verdict="unknown: check manually")
        else:
            try:
                rec.update(fetch(source, ident))
                rec["status"] = "VERIFIED" if rec.get("license") else "UNKNOWN"
            except Exception as e:  # recorded, never filled from memory
                rec.update(license=None, version=None, released=None, status="UNKNOWN", error=str(e)[:160])
            rec["verdict"] = verdict(rec.get("license"), note)
        if note:
            rec["boundary"] = note
        out.append(rec)
        time.sleep(0.3)
    doc = {"schema": "helqix.tools-lock.v1", "fetched_on": time.strftime("%Y-%m-%d"), "method": "primary registry metadata (crates.io, PyPI, npm, GitHub/Forgejo APIs)", "tools": out}
    (root / "tools" / "TOOLS.lock.json").write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    for r in out:
        print(f"{r['status']:8} {r['name'][:28]:28} {str(r.get('license'))[:30]:30} {str(r.get('version'))[:16]:16} {str(r.get('released') or r.get('last_commit'))[:10]:10} {r['verdict']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
