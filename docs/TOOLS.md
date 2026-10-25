# Tool register and licence audit (K0)

Machine-readable register: [`tools/TOOLS.lock.json`](../tools/TOOLS.lock.json) (70 tools). It is produced by two scripts that read primary sources only:

```bash
python -I tools/register/fetch_tools.py .         # licence, version, release date from crates.io, PyPI, npm, GitHub, Forgejo
python -I tools/register/read_license_files.py .  # licence text read and classified where the registry gave no clear id
```

Fetched 2026-10-25. Every licence in the register is **VERIFIED** (registry metadata or the licence file itself, with the file URL recorded). Version and release date are **UNKNOWN** for Temporal, SeaweedFS, Keycloak, Ory Kratos, OpenTelemetry Collector and MinIO: GitHub's unauthenticated API limit (60 requests per hour) was hit. Rerun `fetch_tools.py` after the limit resets.

Licence rule (file 39): MIT, Apache-2.0, BSD, ISC, MPL-2.0, PostgreSQL, CC0 allowed; LGPL for dynamic or separate use; GPL only as a separate process; AGPL and source-available forbidden.

## Findings that change the plan

| # | Finding | Label | Effect |
|---|---|---|---|
| 1 | **Garage is AGPL-3.0** (licence file at git.deuxfleurs.fr). File 39 lists Garage while forbidding AGPL. | VERIFIED | Use **SeaweedFS** (Apache-2.0) as the S3 store. SeaweedFS also sells an Enterprise edition; use the open-source `weed` binary only. |
| 2 | **OpenModelica is "AGPL-3.0 or OSMC-PL 1.8"** (OSMC-License.txt). AGPL is forbidden; OSMC-PL is a custom licence. | VERIFIED | **Founder decision.** Recommended: do not use OpenModelica now. The FMI adapter uses **FMPy (BSD-3) + Modelica Reference FMUs (BSD-2)**, which needs no Modelica compiler. |
| 3 | **This PC has no NVIDIA GPU**: AMD Radeon 840M integrated graphics; no `nvidia-smi`; no WSL; no Docker. Files 38 and 39 assume an RTX 4050 laptop. | VERIFIED | NVENC cannot be tested here. The K7 stream spike needs the RTX 4050 machine (if it exists) or a cloud GPU. The K1 dev stack needs Docker installed (founder action). |
| 4 | OpenTURNS is LGPL-3.0 (not BSD, not AGPL). | VERIFIED | Allowed as a Python library used dynamically. |
| 5 | FFmpeg is LGPL-2.1 with optional GPL parts (x264 is GPL-2.0). | VERIFIED | Use an LGPL build as a separate process; x264 only as a separate process. |
| 6 | Blender GPL-3.0, CalculiX GPL-2.0. | VERIFIED | Separate process via CLI only (already the design). |
| 7 | MinIO is AGPL-3.0. | VERIFIED | Confirms the plan's exclusion. |

## The nine questions in file 39

| Question | Answer | Label |
|---|---|---|
| FMPy maintenance | Releases 0.3.28 (2026-03-02) to 0.3.32 (2026-09-09): five in seven months. BSD-3. Maintained. | VERIFIED (PyPI) |
| PyBaMM licence and cadence | BSD-3-Clause. Monthly releases (26.6.2 on 2026-06-16 to 26.10.1.1 on 2026-10-20, eight releases). Pin the minor version. | VERIFIED (PyPI, licence file) |
| LiveKit vs mediasoup | LiveKit server Apache-2.0 (v1.13.11); mediasoup ISC (3.31.0). Both allowed. **Recommend LiveKit**: a complete server with TURN support, client SDKs and GStreamer-friendly ingest; mediasoup is a Node library we would have to build a server around. | Licences VERIFIED; fit is ASSUMPTION until the K7 spike |
| Selkies-GStreamer licence | MPL-2.0 (selkies-project/selkies 2.1.0). Allowed; file-level copyleft only if we modify its files. | VERIFIED |
| three-gpu-pathtracer licence | MIT (0.0.27). Pre-1.0: expect API changes. | VERIFIED |
| NVENC under WSL2 on an RTX 4050 | Not answerable from this PC (no NVIDIA GPU). NVIDIA's WSL user guide page did not return text about NVENC to a plain fetch. | **UNKNOWN** |
| Garage vs SeaweedFS | Garage is AGPL-3.0, so forbidden. SeaweedFS Apache-2.0, single binary, S3 API. **SeaweedFS.** | VERIFIED |
| Temporal self-host | Temporal server MIT. The README's dev path is `temporal server start-dev` (single binary). Production persistence requirements were not read. | Licence and dev command VERIFIED; production requirements UNKNOWN |
| Rekor v2 for self-hosted signing | rekor-tiles Apache-2.0 (v2.3.0). Its README says separate binaries per storage backend (GCP, AWS) and that the public-good Rekor v2 URL was planned for distribution "end of 2025/early 2026". **Recommend:** sign with cosign using a self-held key and keep our own Ed25519 verifier (Phase 5) as the offline check; add transparency-log upload later. | VERIFIED (README); public-instance status today UNKNOWN |

## Recommended tool per slot

| Slot | Choice | Licence | Boundary |
|---|---|---|---|
| Client renderer | three.js 0.186, three-gpu-pathtracer, detect-gpu, meshoptimizer | MIT | in-process |
| Stream (T2) | Playwright + Chromium, GStreamer (LGPL, separate process), LiveKit, coturn, NVIDIA Container Toolkit | Apache/BSD/LGPL | GStreamer and encoders as separate processes |
| Assets (T3) | Blender CLI, Open Image Denoise, glTF-Transform, glTF-Validator, Draco, Basis Universal, FFmpeg (LGPL build) | GPL (Blender) / Apache / MIT / LGPL | Blender and FFmpeg as separate processes |
| Solvers | FMPy + Reference FMUs, PyBaMM, CalculiX (ccx subprocess), SUNDIALS, SciPy, Pint | BSD / GPL (CalculiX) | CalculiX separate process |
| Trust | cosign, in-toto, BLAKE3, SALib, Hypothesis, proptest; Rekor v2 later | Apache / MIT / MPL / CC0 | — |
| Platform | axum, tokio, serde, schemars, sqlx, PostgreSQL + pgvector, NATS, Temporal, Arrow/Parquet/DuckDB, **SeaweedFS**, Keycloak, OpenTelemetry Collector | MIT / Apache / PostgreSQL | services in containers |
| Quantum (K9 only) | Qiskit, Qiskit Aer, PennyLane | Apache-2.0 | not started |
| Gates | cargo-deny, license-checker-rseidelsohn, Vitest, Playwright, axe-core | MIT / Apache / BSD / MPL | CI |

## Forbidden or blocked

| Tool | Why |
|---|---|
| MinIO, Garage | AGPL-3.0 |
| OpenModelica | AGPL-3.0 or custom OSMC-PL (founder decision) |
| UE5, Omniverse Kit | source-available / proprietary (plan) |
