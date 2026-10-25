# 38. Final Open-Source Stack, Render Ladder, and How We Serve Every Type of User

Decision (file 35 style): build now with high-quality open-source tools only; serve every device from one product through an **Adaptive Render Ladder**; high-end visuals reach low-end laptops through our own cloud GPU stream; v1 runs on free tiers, credits and Raj's own RTX 4050 laptop as the first render server.
Labels: VERIFIED (web check today), CALCULATED, ASSUMPTION, HYPOTHESIS, UNKNOWN. Every license below is **VERIFY at adoption** unless stated; a license scanner (cargo-deny, license-checker) runs in CI.

## 1. The principle
One scene description (glTF + stored simulation frames + Bindings). Several renderers consume it. Rendered pixels are Derived state only. Simulation truth, replay hashes and evidence never depend on which tier drew the picture.

## 2. Render ladder

| Tier | Where it renders | Target user | Technique |
|---|---|---|---|
| T0 Lite | User browser, WebGL2 | Low-end laptop, tablet, integrated GPU | Baked lighting and AO, LOD (meshoptimizer), KTX2 textures, simplified PBR, instancing |
| T1 Standard | User browser, WebGPU or WebGL2 | Mid and high-end laptop | Full PBR, HDRI, soft shadows, SSAO, filmic tone mapping |
| T1+ Local converge | User browser | High-end GPU | three-gpu-pathtracer progressive refinement when the camera is idle |
| T2 Cloud Realtime | Our GPU, streamed by WebRTC | Any device, including low-end | Same three.js app in headless GPU Chromium, higher settings, path-trace converge, H.264 or AV1 stream |
| T3 Cloud Cinematic | Our GPU, async job | Everyone | Blender Cycles + Intel Open Image Denoise; thumbnails, hero plates, replay videos; cached |

Rules: (1) UI panels are local DOM, only the canvas is streamed. (2) Gizmo, selection outline and picking run locally on an invisible low-poly proxy scene so latency is hidden. (3) Auto-select the tier at startup, remember per device, user can override. (4) Engineering view (neutral studio) is the default for Model and Verify at every tier.

Auto-select inputs: WebGPU availability, GPU class (detect-gpu), a 2 second frame-time benchmark, memory, and a network probe (RTT, bandwidth) for T2. ASSUMPTION: T2 is comfortable at RTT under 80 ms.

## 3. Every user type

| User | Default | Also offered |
|---|---|---|
| Low-end laptop or Chromebook | T0 | T2 on demand (credits) |
| Mid laptop | T1 | T2 optional, T3 jobs |
| High-end workstation | T1+ | T3 jobs |
| Phone or tablet | Read-only viewer: results, replay, evidence (T0), or T2 view-only stream | Studio editing needs 1280 x 800 minimum |
| Enterprise, on-prem or air-gapped | Same T2 container on their own GPU server | No cloud needed |
| External reviewer or verifier | No 3D needed: Evidence sheet + verifier command | Static viewer |
| Raj and the team (v1 demo) | T1 on RTX 4050; T2 served from the RTX 4050 laptop (Docker/WSL2) | Phone (Narzo 70 Turbo) as the low-end test client |

## 4. Final tool list (all open source; license VERIFY)

**Client**: three.js (WebGPU + WebGL2), meshoptimizer, glTF-Transform, Draco, KTX2/Basis Universal, three-gpu-pathtracer, detect-gpu, uPlot, WebRTC + WebCodecs, Radix UI or React Aria, Vitest, Playwright, axe.
**Cloud render (T2)**: headless Chromium with GPU (Playwright), GStreamer with NVENC (SVT-AV1 or VA-API fallback), LiveKit or mediasoup (WebRTC), coturn (TURN), Selkies-GStreamer (reference design), Docker + NVIDIA Container Toolkit, Kubernetes + KEDA or Nomad, Temporal (session lifecycle), Garage or SeaweedFS (object cache), OpenTelemetry.
**Cinematic (T3)**: Blender Cycles (run as a separate CLI process, GPL boundary), Intel Open Image Denoise, FFmpeg, glTF-Validator (CI gate).
**Authoring**: Blender, Material Maker, Poly Haven and ambientCG (CC0, license per asset), CesiumJS and MapLibre (terrain).
**Physics and solvers**: FMI/FMPy + OpenModelica, PyBaMM, CalculiX (subprocess), SUNDIALS and SciPy, Rapier deterministic build and Jolt (visual and mechanism), FEniCSx and MOOSE (research), Pint (units).
**Trust**: Sigstore (cosign, Rekor v2) , in-toto, SLSA, BLAKE3, SALib, OpenTURNS, Hypothesis, proptest.
**Platform**: Rust (axum, tokio, serde, sqlx), PostgreSQL + pgvector, NATS, Arrow + Parquet + DuckDB + Zarr, Keycloak or Ory.
**Quantum (Phase 7C, never faked)**: Qiskit + Aer, PennyLane, cloud QPU through IBM Quantum open plan or AWS Braket; UI shows Available, Queued, Unsupported.
**Not used in the core**: UE5 (source-available, fees), Omniverse Kit (proprietary), MinIO (AGPL), Houdini and Substance (proprietary).

## 5. Why headless three.js for T2 (CALCULATED from judgement scores, ASSUMPTION)
Weights: photoreal .25, same code path (bindings, replay) .25, open license .20, team feasibility .20, GPU cost .10.
Headless three.js 4.65; Godot headless 3.38; O3DE 3.33; Blender EEVEE stream 3.13; UE5 Pixel Streaming 3.05.
Weakness: live global illumination ceiling is below AAA engines. Mitigation: converge path tracing when still, T3 for hero imagery. Research track for v2: O3DE.

## 6. Free-tier reality for v1
- Real-time GPU streaming has no true free tier (Colab and Kaggle terms and quotas change; do not serve from them; use them only for batch renders; VERIFY).
- v1 path: T0 and T1 local; T3 plates pre-rendered and cached; T2 only on click for a 15 minute trial from (a) the RTX 4050 laptop, (b) spot GPU on credits (GitHub Student Pack, Azure for Students, Google Cloud, AWS Activate, Microsoft Founders Hub, NVIDIA Inception, Modal or Lightning credits; amounts and eligibility VERIFY).
- Cost model (ASSUMPTIONS): stream 8 Mbps = 3.6 GB per hour; egress $0.09 per GB = $0.32 per hour; GPU spot $0.50 per hour; about $0.82 per session hour; 17 session hours per month = about $14 per active user per month. T2 must be metered and priced; it cannot be free forever.
- Guards: hard session cap, idle auto-kill, budget alarm, one session per free user.

## 7. Build order now
1. A0 audit, contracts, EngineClient, tokens, shell (files 36, 37).
2. T0 and T1 renderer with quality settings, detect-gpu, tier switch, asset pipeline with glTF-Validator gate.
3. Prompt screen and live build (A2, A3).
4. Simulate with bound motion (A4); measure fps.
5. T2 spike (prompt B below), then T3 pipeline (prompt C).
6. Cloud scheduler, pricing and credits only after the spike passes.

Pass criteria (ASSUMPTIONS to confirm by measuring): T0 holds 30 fps with 200 parts on a throttled/integrated-GPU profile; T1 holds 60 fps on the RTX 4050; T2 input-to-photon under 100 ms at RTT 40 ms; the phone client plays the stream smoothly.

## 8. Risks
GPU credit denial; WSL2 NVENC support (VERIFY); stream stutter on mobile data; cost leaks from idle sessions; photoreal expectations versus three.js ceiling (judge on screenshots); GPL boundary for Blender and CalculiX; asset license mistakes. Weakest assumption: that converged three.js stills reach the quality in your reference images; decide with side-by-side screenshots at the T2 spike.

---

## PROMPT A (Antigravity addendum to file 37): Render ladder in the frontend
Add this to the Antigravity prompt as milestone **A4b**, after Simulate.
- Implement quality tiers T0, T1, T1+ in the existing three.js renderer: T0 (WebGL2, baked lighting, LOD via meshoptimizer, KTX2, instancing), T1 (PBR, HDRI, shadows, SSAO), T1+ (three-gpu-pathtracer converge when idle).
- Add `detect-gpu` + 2 second frame-time benchmark + memory + network probe; auto-select tier, show the tier chip in the header, allow manual override, remember per device.
- Add `RenderBackend` interface: `LocalBackend` (T0, T1, T1+) and `StreamBackend` (T2, stub that reads a WebRTC stream; shows "Cloud render unavailable" until a server exists). Local proxy scene for picking, gizmo and selection when streaming.
- Never claim T2 is available unless a stream is connected. Label tiers honestly. Report measured fps on (a) CPU/GPU-throttled profile and (b) RTX 4050 laptop, with screenshots per tier.
- Tests: tier selection logic, fallback when WebGPU missing, picking works with the proxy scene, visual regression per tier.

## PROMPT B (Claude Code): T2 cloud realtime spike (paste file 13 first)
Role: build a minimal proof that the existing `21-app/reality-studio` three.js scene can be rendered on a GPU machine and streamed to a browser with interaction.
Scope (spike only, no product code changes except a `streaming/` folder and a `?render=stream` flag):
1. Docker image with NVIDIA Container Toolkit running Chromium via Playwright with GPU enabled; load the Studio scene in a fixed 1920 x 1080 viewport.
2. Capture frames and encode with GStreamer NVENC (H.264; AV1 optional); send over WebRTC using LiveKit (or mediasoup) with coturn for TURN.
3. Browser client page receives the stream; forwards mouse, wheel and key events to the headless page (orbit, zoom, select).
4. Measure and report: input-to-photon latency at RTT 0, 40 and 80 ms (use tc netem), bitrate, CPU and GPU usage, frames dropped, encode time. Test clients: desktop Chrome and an Android phone.
5. Run on the RTX 4050 laptop (Docker or WSL2). State exactly whether NVENC works in that environment; if not, report the fallback (software x264) and its cost. Do not claim results you did not measure.
6. Include idle auto-shutdown and a hard session time limit.
Deliver: a README with run commands, measured numbers table, screenshots, risks, and a go/no-go recommendation against the pass criteria in section 7 of file 38. No pushes or deploys without permission. License-check every dependency and list the licenses.

## PROMPT C (Claude Code): Blender T3 pipeline and asset QA (paste file 13 first)
- `tools/assets/`: a headless Blender (CLI/bpy) script that imports a source model, applies PBR materials and baked AO, exports glTF (Draco or meshopt + KTX2), then runs glTF-Transform optimize and the Khronos glTF-Validator; fail on errors.
- `tools/render/`: Cycles batch render with Open Image Denoise for (a) project thumbnails from a model revision and (b) hero plates from a scene file, outputs cached by content hash.
- `LICENSES.json` per asset (source URL, license, date checked); CI fails if an asset lacks a license record.
- Blender runs as a separate process only; no linking. Everything open source; list versions used.
- Report the render time, file sizes (target: textures KTX2, each part under 2 MB; ASSUMPTION), and sample outputs. Honesty: generated thumbnails are labeled Rendered preview; parts with no model remain PLACEHOLDER MODEL.
