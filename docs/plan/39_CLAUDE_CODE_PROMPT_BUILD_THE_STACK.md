# 39. Claude Code Prompt: Build the Final Stack (tools, render server, pipelines, platform)

Use: paste file 13 (COMMON_PREAMBLE) first, then everything under "PROMPT". Put the HELQIX docs folder into the repo at `docs/helqix/` (ask me first) so you can read files 02, 06, 13, 17, 18, 35, 36, 38. One milestone per session; stop and report after each.

Division of work (so two tools do not edit the same code):
- **Claude Code** (this prompt): repo infrastructure, tool register and license gate, dev stack, API and engine contracts, solver adapters, frames and storage, signing, asset and render pipelines, cloud render server.
- **Antigravity** (file 37): all UI under `21-app/reality-studio/src` (screens, Studio, renderer tiers on the client). Claude Code must not restyle UI. It may only add the `streaming/` folder and the contracts package.

---

## PROMPT

### Role
Senior platform engineer on HELQIX, a domain-agnostic Engineering Executable-Reality Platform ("Engineering truth, with proof"). The founder does not code; he reviews your reports and the running system. Follow the COMMON PREAMBLE. Do not rename the 23 numbered folders. Never push, open PRs, deploy, or delete files on disk without my explicit permission.

### Locked rules (override everything)
1. One authoritative state owned by the engine. Rendered pixels, charts and 3D scenes are Derived and never truth.
2. No domain parts in product code (no motor, battery, pump... outside `examples/` and test fixtures, labeled Example).
3. Everything open source. Allowed licenses: MIT, Apache-2.0, BSD, MPL-2.0, LGPL (dynamic/separate use). **GPL tools (Blender, CalculiX, Gmsh, OpenFOAM) run only as separate processes** via CLI or adapter. **AGPL and source-available/proprietary tools (MinIO, UE5, Omniverse Kit) are forbidden.** Anything unclear: stop and ask.
4. Never fake. Anything not real is tagged MOCK, DEMO or PREVIEW. Quantum is never simulated as if real hardware: show Available, Queued, Unsupported honestly.
5. Every claim in your reports carries a label: VERIFIED (you checked a primary source or ran it), CALCULATED, ASSUMPTION, HYPOTHESIS, UNKNOWN. If the network allowlist blocks a primary source, mark the fact UNKNOWN, do not fill from memory.
6. Determinism levels (bitwise, tolerance, statistical) are declared per run; replay hash match is a test, not a claim.

### The stack to establish
**Render/stream**: headless Chromium (Playwright) with GPU, GStreamer + NVENC (software x264 fallback), LiveKit or mediasoup (WebRTC), coturn, Selkies-GStreamer (reference), Docker + NVIDIA Container Toolkit.
**Cinematic/assets**: Blender (CLI, separate process), Intel Open Image Denoise, FFmpeg, glTF-Transform, meshoptimizer, Draco, KTX2/Basis Universal, Khronos glTF-Validator.
**Solvers**: FMI via FMPy + OpenModelica, PyBaMM, CalculiX (subprocess), SUNDIALS and SciPy, Pint (units). Research only: FEniCSx, MOOSE, Rapier deterministic, Jolt.
**Trust**: Sigstore (cosign, Rekor v2), in-toto, SLSA, BLAKE3, SALib, OpenTURNS, Hypothesis, proptest.
**Platform**: Rust (axum, tokio, serde, schemars, sqlx), PostgreSQL + pgvector, NATS, Temporal, Arrow + Parquet + DuckDB + Zarr, Garage (S3-compatible), Keycloak or Ory, OpenTelemetry.
**Quantum (later)**: Qiskit + Aer, PennyLane, cloud QPU via IBM Quantum open plan or AWS Braket.
**Quality gates**: cargo-deny, JS license checker, Vitest, Playwright, axe, glTF-Validator.

### Milestones

**K0. Tool register and license audit (no feature code)**
- Create `tools/TOOLS.lock.json` and a readable `docs/TOOLS.md`: for every tool above record name, role, version proposed, license (read from the repo or package metadata), license compatibility verdict (allowed, separate-process-only, forbidden), maintenance signal (last release or commit date), install method, and status (VERIFIED/UNKNOWN). Check primary sources (GitHub, crates.io, PyPI, npm, vendor docs).
- Specifically verify and report: FMPy maintenance (last release), PyBaMM license and release cadence, LiveKit vs mediasoup license and fit, Selkies-GStreamer license, three-gpu-pathtracer license, NVENC under WSL2 on an RTX 4050 laptop, Garage vs SeaweedFS fitness, Temporal self-host requirements, Rekor v2 usage for self-hosted signing.
- Recommend one tool per slot. Flag every GPL/AGPL/unknown item and the boundary design.
- STOP. Report. No installs yet except read-only checks.

**K1. Repo infrastructure and gates**
- Execute file 36 **M0** (hygiene, untrack node_modules/dist/target, test runner, D5 trace, domain-word CI test) if not already done.
- Add `cargo-deny` (licenses, advisories, bans) and a JS license check to CI; make them fail on forbidden licenses and unknown licenses.
- Add `docker compose` dev stack: PostgreSQL + pgvector, NATS, Garage, Temporal (dev), OpenTelemetry collector. One command up/down. Document ports and volumes. No secrets in git; use `.env.example`.
- Add a `make check` (or equivalent) that runs build, lint, tests, license gates.
- STOP. Report.

**K2. Contracts and minimal real API**
- Execute file 36 **M1** contracts: `ModelOp`, `BuildEvent`, `Value`, `Binding`, `ModelRevision`, `RunFrame`, `EngineClient`, JSON schema generation (schemars), schema validation tests.
- In `22-api/helqix-api` implement real minimal routes (replace the `.gitkeep` placeholders only for what you implement): `POST /projects`, `POST /projects/:id/builds`, `GET /builds/:id/events` (SSE), `POST /builds/:id/ops`, `POST /builds/:id/undo|redo`, `POST /builds/:id/commit`, `GET /sources`. Staged revisions live in Postgres. Per D5 outcome, authoritative state is the Rust engine, not the browser. Keep the MockEngineClient working beside it.
- Tests: op log order equals events; undo removes one op; refresh restores a staged build; commit returns revision r1.
- STOP. Report with the exact curl commands and outputs.

**K3. Solver adapters (follow file 18 Phase 6 and file 13 rules)**
Order: (1) FMI/FMU adapter via FMPy with a tiny **test FMU** from the Modelica Standard Library or FMI reference FMUs (labeled Example); (2) PyBaMM adapter; (3) CalculiX subprocess adapter with a published benchmark case. Adapter contract: Capabilities, Validate, Prepare, Execute, Status, Cancel, Collect, Normalize, Provenance. Each adapter declares its determinism level; units normalized with Pint. Each adapter ships a regression benchmark with a reference result and tolerance. Failure and timeout paths are tested. GPL tool = separate process, documented boundary.
- STOP after each adapter with a report. Do not start the next without my go.

**K4. Frames, storage and replay**
- Store run state frames in Arrow/Parquet (fields in Zarr if needed), object data in Garage, metadata in Postgres; analysis through DuckDB.
- Content-address artifacts with BLAKE3. Implement freeze and exact replay per file 14 with a hash-match test; scrubbing reads frames, never mutates.
- `getFrame(runId, t)` and `runFrames` endpoints; deterministic interpolation rule documented and tested.
- STOP. Report with a replay-match demonstration and benchmarks (frame size, read latency).

**K5. Signed evidence (follow file 17)**
- Evidence package builder; sign with Sigstore (cosign) using a self-hosted or public-good flow as the K0 audit recommends; record in a transparency log if allowed; in-toto attestation for the build/run chain; independent verifier CLI that runs with no HELQIX services.
- Honest copy: "Signing proves origin and integrity, not correct physics."
- Tests: tampered package fails verification; verifier runs offline.
- STOP. Report.

**K6. Asset and cinematic pipeline** — execute **Prompt C in file 38** (Blender CLI + glTF-Transform + glTF-Validator gate + Cycles with Open Image Denoise + FFmpeg + `LICENSES.json` register). STOP. Report.

**K7. Cloud realtime spike** — execute **Prompt B in file 38** (headless GPU Chromium + GStreamer NVENC + WebRTC with LiveKit/mediasoup + coturn; measure latency at RTT 0/40/80 ms; test desktop and Android phone; run on the RTX 4050 laptop). Output a go/no-go. STOP. Report.

**K8. Render sessions and metering (only if K7 is go, and only with my written approval)**
- Session scheduler on Temporal: request, allocate GPU worker, health check, hard time cap, idle auto-kill, cleanup. Per-session metering (GPU seconds, egress bytes) stored in Postgres. Budget alarm and one-free-trial rule.
- On-prem mode: same container runs on a customer GPU host without cloud dependencies.
- Observability with OpenTelemetry. Cost report using measured numbers, not assumptions.
- STOP. Report.

**K9 (do NOT start without my written go-ahead): quantum layer**, Qiskit + Aer local simulator first, then cloud QPU adapter with honest queue/availability states, only after the solver and evidence gates pass.

### Non-goals
UI redesign (Antigravity owns it); a custom rendering engine; UE5/Omniverse; MinIO; any claim of text-to-3D generation; multi-user editing; production deploy.

### Engineering rules
- Small commits on a branch, never pushed without permission. Do not force-push. Say before deleting anything on disk.
- Every module has tests; run build, lint, tests and license gates before reporting; show real output including failures.
- No secrets in the repo. Pin versions in lock files. Record exact tool versions in `tools/TOOLS.lock.json`.
- Stop and ask when: a license is unclear, a decision (D1 to D12, N1 to N5) is needed, a dependency adds a forbidden license, the engine contradicts the contracts, a measured result misses the pass criteria.

### Report format after every milestone (plain English)
1. What now works, with exact commands to run it.
2. What is MOCK, PREVIEW, or UNKNOWN.
3. Test and gate results (counts and failures).
4. Tools added with version and license, and any license risk.
5. Measurements (latency, size, cost) clearly labeled measured or assumed.
6. Contradictions with the docs.
7. Decisions I must make next, with your recommendation.
8. What you did not verify.

### Acceptance for the whole task
`make check` passes including license gates; contracts validate; a staged build commits r1 through the real API; at least one real adapter (FMI) runs a benchmark with a regression tolerance; a run replays with a matching hash; an evidence package verifies offline and a tampered one fails; Blender pipeline outputs validated glTF with a license register; the T2 spike reports measured latency and a go/no-go.
