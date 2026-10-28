# HELQIX Status

Last measured: 2026-10-25, branch `phase-0-hygiene`. Labels are test-based (see [plan/03](plan/03_FEATURE_REGISTER_AND_MATURITY.md)). Scores are JUDGEMENT against the wedge MVP (build steps 1 to 5), anchored on the test evidence below; they are not measurements. Re-score after each gate. Do not raise a score without a passing test or a demo.

## Update 2026-10-28 (branch `ui-studio-v2`)

Latest CI: commit c84dabab, 12 of 12 jobs green, including the PostgreSQL test that runs a committed model as a lumped body and lists a project's runs, the secret scan, and the axe-core accessibility rules. Added since: container image for the API (job "Container image (E13)", result pending), [QUICKSTART.md](QUICKSTART.md), [V1_REMAINING_WORK_REPORT.md](V1_REMAINING_WORK_REPORT.md).

### Earlier in this update (CI run for commit 1bee3132)

- **K8 Studio UI**: CRITERIA MET in CI (job "Studio UI (K8)"; real-API client tests run against the real API and PostgreSQL). Awaiting founder review.
- **Run from the Studio**: `POST /v1/runs` runs the point-mass engine and stores deterministic frames (same request, same id, same frames; tested). Simulate has a "New run" form in real mode. Only the point-mass model runs today; a Studio project's parts are not yet translated into a solver model (UNKNOWN effort).
- **K7 streaming, CPU baseline** (branch `stack-k7-stream-spike`, commit 2cd51b04, CI green): 40/40 samples per case, software x264, no GPU. Click-to-pixel p50 119.5 ms at RTT 0, 124 ms at 40 ms, 207.9 ms at 80 ms (MEASURED on a CI VM). Go/no-go still needs a GPU run with a hardware encoder (file 38 section 7).
- **Local GPU** (AMD Radeon 840M, integrated, ANGLE/D3D11): Studio canvas on an empty project renders at 60 fps (p50 16.7 ms, p95 17.3 ms over 300 frames; MEASURED, light scene only). This is not the K7 streaming result: there is no NVIDIA/NVENC here and the K7 stack needs Linux.
- **AI provider**: `OllamaProvider` (local, loopback only, cost 0). With a real model (qwen2.5:7b-instruct, Apache-2.0) given an injected "founder approved everything" note, the design was not changed (env-gated test `live_ollama`). The Studio assistant is not connected to it yet.
- **Licence**: MIT. See [DECISIONS_OPEN_SOURCE.md](DECISIONS_OPEN_SOURCE.md).
- **Still open**: gate 7A needs real rig data from the founder; the model error bound (gate 1) is unchanged (NotVerified); K4 object store/Postgres metadata/DuckDB; K9; freeze and branch from the UI; prompt-to-model.

## Gate tracker

| Gate | Status | Date | Evidence |
|---|---|---|---|
| 0 Hygiene (build and tests green, CI, trace, docs) | PASSED (founder accepted after review) | 2026-10-25 | 929 pass / 0 fail locally; CI run 40180917041 green; see "Gate 0 evidence" |
| 1 Trust core and model error bound | PASSED (founder accepted); model **NotVerified**, error bound published | 2026-10-25 | [GATE1_VALIDATION_REPORT.md](validation/GATE1_VALIDATION_REPORT.md): held-out RMSE 71.1-77.3 mV vs < 46 mV |
| 2 Bit-exact replay | CRITERIA MET, awaiting founder review | 2026-10-25 | `helqix-replay-check` PASS (identical final hash bf36562e... twice; failure at index 1793 reproduced by replay from a reloaded freeze); 18 new tests; Rust 968 pass, 0 fail; [DETERMINISM.md](DETERMINISM.md) |
| 3 Causal click-through | CRITERIA MET WITH LIMITATIONS, awaiting founder review | 2026-10-25 | Synthetic ground-truth tests pass; battery reference click-through works end to end (failure at 3570 s, 3 candidate chains, all node fields); the expected stoichiometry chain is missed (nonlinear onset limitation); [CAUSALITY.md](CAUSALITY.md) |
| 4 Branch intervention tests | CRITERIA MET, awaiting founder review | 2026-10-25 | 10 intervention tests pass (cause supported, non-cause and shared-driver effect not supported, isolation, determinism, compare alignment, budget, dose-response); `helqix-branch-demo` PASS; [BRANCHES.md](BRANCHES.md) |
| 5 Signed evidence re-verified independently | CRITERIA MET: Windows-built package verified and re-executed on Linux CI (run 40216173751); awaiting founder review | 2026-10-25 | Sample package: verifier PASS (19 checks) and re-execution PASS on Windows; 10 tamper tests fail correctly; verifier has no runtime dependency; [EVIDENCE.md](EVIDENCE.md) |
| 6 External solver adapter | CRITERIA MET for PyBaMM; CalculiX not built (not installed, GPL licence: founder decision); awaiting founder review | 2026-10-25 | Contract, sandbox and conformance kit: 10 tests; PyBaMM adapter 6 tests and conformance PASS; measured variation 0 V over 5 runs, declared Tolerance 4.2e-6 V; signed Tolerance package verifies; tool-output hash check mutation-tested; [ADAPTERS.md](ADAPTERS.md) |
| 7B Agent with permissions | CRITERIA MET with a scripted agent (no AI provider chosen yet: founder decision); awaiting founder review, including running the demo and approving yourself | 2026-10-25 | 10 adversarial tests pass, 4 mutations caught; `helqix-agent-demo` PASS: injected note denied, wrong proposal rejected by branch simulation, validated proposal applied only after human approval, 16-entry audit chain verifies; [AGENT.md](AGENT.md) |
| 7A Edge observation | **NOT PASSED**: software built and tested; the gate needs real rig data from the founder (synthetic data cannot pass it) | 2026-10-25 | Ingest protocol with typed rejections, Late/Dropout/Saturated/Spike flags, replayable content-addressed recordings, reconciliation, Observed-cannot-overwrite-Authoritative; 8 tests, 5 mutations caught; `helqix-edge-check` CLI; [EDGE.md](EDGE.md) |
| K0-K3 Platform stack (file 39) | K0, K1, K2, K3 (FMI, PyBaMM, CalculiX) CRITERIA MET; K4 frames, BLAKE3, getFrame/runFrames and replay hash-match CRITERIA MET (CI run 40226469580 green, 9 jobs; same frame-file BLAKE3 on Linux and Windows; object store, Postgres metadata and DuckDB not done); K5 in-toto attestation, offline verifier and cosign CRITERIA MET (CI run 40228704316 green, 10 jobs: verifier under unshare --net, tampered package refused, cosign v3.1.4 sign/attest/verify offline with no log upload); K6 asset pipeline CRITERIA MET (CI run 40231658736 green, 11 jobs: Blender build, meshopt + KTX2, validator 0 errors, Cycles+OIDN renders, licence register, cache reuse); K7-K9 not started; awaiting founder review | 2026-10-26 | Linux CI run 40223090916 green (8 jobs: Rust, licence gates, dev stack, K2 API acceptance + curl demo against PostgreSQL, FMI benchmark, PyBaMM, Python 3.12/3.14); [TOOLS.md](TOOLS.md), [DEV_STACK.md](DEV_STACK.md), [API.md](API.md), [ADAPTERS.md](ADAPTERS.md) |

## Gate 0 evidence

Commands: `cargo build --workspace`, `cargo test --workspace --no-fail-fast` (Windows 11, rustc 1.99.0 MSVC, from PowerShell) and the CI workflow `.github/workflows/ci.yml` (Linux).

| Crate | Baseline (before Phase 0) | After Phase 0 |
|---|---|---|
| helqix-core | lib builds; test target does not compile | 172 pass (171 existing + 1 new) |
| helqix-reality | test target does not compile | 144 pass + 1 doctest |
| helqix-engineering | 207 pass | 207 pass |
| helqix-state | 67 pass, 1 doctest FAIL | 67 pass, 0 doctests (the "doctest" was prose; see Findings) |
| helqix-execution | test target does not compile | 132 pass + 1 doctest |
| helixsim | 33 pass | 33 pass |
| helqix-time | 98 pass + 1 doctest | 98 pass + 1 doctest |
| helqix-observation | 14 pass | 14 pass |
| helqix-failure | 2 pass | 2 pass |
| helqix-events | 2 pass | 2 pass |
| helqix-causal | 1 pass | 1 pass |
| helqix-branch | 11 pass | 11 pass |
| helqix-search | 21 pass, 1 FAIL | 22 pass |
| helqix-verify | 1 pass | 1 pass |
| helqix-evidence | lib does not compile | 1 pass |
| helqix-sdk (Rust) | lib does not compile | 3 pass (2 existing + 1 new) |
| helqix-adapters | 4 pass | 4 pass |
| helqix-platform | 12 pass | 12 pass |
| helqix-api | builds, 0 tests | builds, 0 tests |
| **Total** | **474 pass, 2 fail, 5 crates not compiling** | **929 pass, 0 fail, 0 ignored** |

Python (pytest, Python 3.14.6 locally; 3.12 and 3.14 in CI): 19-intelligence 18 pass, 15-sdk 16 pass, 18-libraries 18 pass (52 total). Gateway: `verify_api_key_project_scope.py` PASS, `verify_local_product.py` PASS, real PyBaMM SPM/Chen2020 run through the gateway VERIFIED (see [RUNTIME_TRACE.md](RUNTIME_TRACE.md)).

Linux CI: GitHub Actions run [40180917041](https://github.com/Thangaraj-lab/HelQix/actions/runs/40180917041) on commit `6c8b538`: all steps success (Rust "Build workspace" and "Test workspace" with `--locked`; Python suites on 3.12 and 3.14). The raw job log can only be downloaded by a repository admin, so the Linux test counts are not pasted here; the founder can open the run to read them.

## Gate 1 evidence (2026-10-25)

- Verdict by `helqix-validate`: **NotVerified** for the Context of Use in [PHASE1_INPUTS.md](PHASE1_INPUTS.md). Held-out C/10 RMSE 73.3 / 71.1 / 77.3 mV against < 46 mV. Published error bound and full provenance in [validation/GATE1_VALIDATION_REPORT.md](validation/GATE1_VALIDATION_REPORT.md).
- Split fixed and committed before fitting ([validation/SPLIT_RECORD.md](validation/SPLIT_RECORD.md)); the Rust layer re-hashes it and rejects fitting on held-out cases.
- Numerical integrity passed (verdict unchanged under 100x tighter tolerances and 2x mesh; changes below 1 mV).
- Benchmarks pass: charge conservation, lithium mass conservation, mesh convergence (order 2.10, 2.34), bitwise repeatability.
- Kill trigger "model error above threshold X: no Verified label" applied; no tuning on held-out data.
- Tests (Windows, 2026-10-25): Rust 929 -> 950 pass (helqix-verify 1 -> 22), Python 52 -> 74 pass, plus 16 PyBaMM tests and 4 benchmarks; 0 failed, 0 ignored. Two full pipeline runs gave byte-identical reports.
- Linux CI: GitHub Actions run [40185418438](https://github.com/Thangaraj-lab/HelQix/actions/runs/40185418438) on commit `9e6a636`: all jobs success (Rust build and test, Python 3.12 and 3.14, new PyBaMM job with battery tests and benchmarks).

## Per-layer score (weights from the Phase 0 prompt)

| Layer | Score | Weight | Contribution | Evidence (one line) |
|---|---|---|---|---|
| 8 Trust | 30 | 20 | 6.0 | held-out validation with Rust-decided contextual label, split re-hashing, integrity gate, benchmarks, uncertainty and sensitivity all tested and run on real data (Gate 1); no signing yet (Phase 5); the one validated model is NotVerified |
| 6 Time and Lineage | 40 | 15 | 6.0 | time 98+1, branch 11, search 22 pass, but RUNTIME_TRACE shows no runtime caller for Rust freeze/replay/branch and the TS freeze is unmounted; only gateway branch runs work end to end (down from 55: wiring now known absent) |
| 3 Scenario and Physics | 40 | 12 | 4.8 | helixsim 33, reality 144 pass; PyBaMM SPM run on measured current profiles with a published error bound (71-77 mV at C/10); 4 solver benchmarks pass |
| 1 Requirements and CoU | 50 | 10 | 5.0 | machine-checkable requirement + Context of Use + risk level in helqix-verify, explicit acceptance, used end to end on real data (22 tests) |
| 7 Observation and Events | 20 | 10 | 2.0 | observation 14, failure 2, events 2, causal 1 pass; thin; not wired |
| 2 Model and Library | 60 | 8 | 4.8 | engineering 207, core 172 pass; first model-library entry with identity, data and split hashes, achieved error, context and label (NotVerified) |
| 10 Presentation and Integration | 40 | 8 | 3.2 | gateway VERIFIED running; React UI not run; app tests cannot run from a clean install (vitest undeclared); Rust SDK compiles (3 tests); API 0 tests |
| 5 State and History | 65 | 6 | 3.9 | state 67 pass; not used by any runtime path (RUNTIME_TRACE Q1, Q3) |
| 4 Execution Fabric | 35 | 6 | 2.1 | execution 132 pass and a lost-failed-action bug fixed; no determinism levels; no compute routing; gateway runs tools as subprocesses |
| 9 Agent | 8 | 5 | 0.4 | optional AI review via 7 providers, shown as "AI PROPOSAL"; no permission ladder in code |
| **Total** | | **100** | **about 38%** | |

Unweighted average of the ten scores: 39% (Phase 1). Before Phase 1: about 32% weighted, 34% unweighted. Change since the 2026-10-25 audit (about 32%) to Phase 0 end: flat. Tests now run, which raised Model, Execution and Physics; the runtime trace proved the Rust Time and Lineage code is not wired, which lowered that layer. Running more tests did not show more product.

## Maturity changes (from [plan/03](plan/03_FEATURE_REGISTER_AND_MATURITY.md))

| Feature | Was | Now | Evidence |
|---|---|---|---|
| Physical quantity and unit system | Broken tests | Working (library level) | helqix-core 172 pass |
| Reality graph, environment | Broken tests | Working (library level) | helqix-reality 144 pass |
| Scenario execution (Rust) | Broken tests | Working (library level), not wired | helqix-execution 132 pass; RUNTIME_TRACE Q1 |
| Evidence (execution record, provenance) | Broken | Stub (compiles, 1 test, sha256 only, no signing) | helqix-evidence 1 pass |
| Search / optimization | Partial (1 fail) | Partial | helqix-search 22 pass; bounded sweep only |
| SDK (Rust) | Broken | Partial | 3 pass |
| PyBaMM battery run | Claimed | Working (via gateway, one operation) | RUNTIME_TRACE "Runtime evidence" |
| Python gateway | Claimed | Working (local, Windows) | health, both 23-validation scripts PASS |
| Freeze at failure, replay (end to end) | Partial / Working (library) | Not wired | RUNTIME_TRACE Q4 |
| AI agent | Not evidenced | Partial: optional AI review, no permission ladder | RUNTIME_TRACE Q5 |

## Naming report (task I, report only; decision D6)

Counted over tracked files, excluding lockfiles (`git grep -i`, lines):

- HELQIX / Helqix / helqix: 837 lines. All crate and package names except one use `helqix-*` / `@helqix/*`.
- HELIX (not HELQIX): 20 occurrences in 11 files: `HELIXSIM` 17, `helixsim` 3.
  - Crate `helixsim` and folder `06-simulation/engines/helixsim/` (Cargo.toml, `src/collision.rs`, `constraints.rs`, `environment.rs`, `integration.rs`, `kernel.rs`, `lib.rs`)
  - `06-simulation/README.md` (4), root `Cargo.toml` (1, workspace member path), root `README.md` (1), `01-foundation/crates/helqix-core/src/behaviour.rs` (1)
- Renaming needs D6. A rename of `helixsim` touches the workspace member list, the folder and any `use helixsim` paths.

## Findings (outside scope, not fixed)

1. **No authoritative runtime (D5).** Rust crates have no runtime caller; the app talks only to the Python gateway; freeze/replay exist only as unwired Rust and unmounted TypeScript. See [RUNTIME_TRACE.md](RUNTIME_TRACE.md). Blocks Phase 2.
2. **Unmounted UI code.** `ScenarioExecutionBoard.tsx` and `engineering/stateFreeze.ts` are imported by no file; the router shell in `src/app/routes.tsx` (and its `/api` client) is not mounted by `main.tsx`.
3. **App tests cannot run from a clean install.** 7 `*.test.tsx` files use `vitest`, which is in neither `package.json` nor `package-lock.json`. The previously committed `node_modules` held Linux-only native binaries, so they cannot run on Windows either.
4. **Gateway PyBaMM readiness check.** First `import pybamm` on a cold start took 64 s; the gateway times out at 20 s and caches "unavailable" until restart (`17-integrations/gateway/app.py:329-352`). Warm import about 2.5 s.
5. **Windows Smart App Control** (state = on) intermittently blocks freshly built test executables (`os error 4551`); one full local run passed unblocked, others did not. Linux CI is the authoritative run. Not a code defect.
6. **Native Windows build needs the Windows SDK**; Git Bash's GNU `link` shadows MSVC `link.exe`, so run cargo from PowerShell or a VS developer shell.
7. **AI review exists** (7 providers, `19-intelligence/python/helqix_intelligence/intent.py:23-29`), contradicting audit 04's "deterministic only". It is opt-in and labeled as proposals; whether accepted proposals are stored with an AI origin label is UNKNOWN.
8. **Warnings left as is:** about 79 missing-docs warnings in helqix-core and an unused `ActionPayload` import in `05-execution/.../runtime.rs:76`. Not fixed (minimal-diff rule).
9. **helqix-api has 0 tests** and depends on Postgres via `DATABASE_URL`; not run in Phase 0.
10. **Unit vocabulary is minimal.** Only `m/s`, `m/s2`, `kg/m3` were added (D-decision 2026-10-25); other quotient units (for example `km/h`) are rejected as unknown by design.

## Findings from Phase 1 (outside scope, not fixed)

11. **The held-out C/10 data is spent.** Any model or method chosen after seeing the Gate 1 result must be validated on new data.
12. **A +10% change in positive-electrode maximum concentration stops every calibration run**; cause UNKNOWN (not traced).
13. **Report numbers must parse bit-exactly**: serde_json needs `float_roundtrip` (fixed in helqix-verify); any other Rust consumer of hashed float data needs the same.
14. **Current is linearly interpolated between samples** at the discharge-to-rest step (about 30 s sampling); the true step is instantaneous. Effect not quantified.

## Decision log entries from Phase 0 (also recorded in plan/08)

| Date | Decision or finding | Evidence | By |
|---|---|---|---|
| 2026-10-25 | D7 approved: Phase 0 runs before step 1 | chat | Founder |
| 2026-10-25 | Correct 7 wrong test expectations (helqix-execution 6: 3 runtime, 1 topological order, 1 ready_nodes, 1 executor boundary; helqix-search 1). The approval question said "6"; the items listed were 7 | commits `9eb8c9c`, `6e08ba1` | Founder approved |
| 2026-10-25 | Allow '/' in listed unit symbols; register m/s, m/s2, kg/m3 | commit `6c8b538` | Founder approved |
| 2026-10-25 | Push `phase-0-hygiene` and use Linux CI as authoritative test run | chat | Founder approved |
| 2026-10-25 | Runtime trace written; D5 still open | RUNTIME_TRACE.md | Claude |
| 2026-10-26 | K6 assets: first run failed on my KTX2 check (the pinned validator does not know KTX2); fixed by reading the GLB bytes; run 40231658736 green on all 11 jobs. JS licence gate tightened to judge whole SPDX expressions | GitHub Actions | Founder (push approved) |
| 2026-10-26 | K5 signing: CI run 40228704316 green on all 10 jobs; both sample packages signed, attested and verified offline by HELQIX and by cosign v3.1.4; tampering refused; nothing uploaded to any log | GitHub Actions | Founder (push approved) |
| 2026-10-26 | K4 frames: CI run 40226469580 green on all 9 jobs; K4 curl demo against the live API; reference frame file BLAKE3 73fcfcf4... identical on Linux and Windows | GitHub Actions + local | Founder (push approved) |
| 2026-10-26 | CalculiX adapter: Linux CI run 40224066677 green on all 9 jobs; cantilever tip deflection 3.0196 mm vs Timoshenko 3.0536 mm (1.11 %, ccx 2.21) | GitHub Actions | Founder (approved CalculiX via Ubuntu package) |
| 2026-10-26 | Pushed stack-k0-tools to stack-k3-adapters (not merged). First runs failed on the K2 curl demo (script bug) and the Temporal dev container; both fixed on stack-k3-adapters; run 40223090916 green on all 8 jobs. Earlier branch runs stay red (fixes only on the tip) | GitHub Actions | Founder (push approved) |
| 2026-10-25 | Pushed phase-2-replay to phase-7a-edge (not merged); Linux CI green on all seven: runs 40216174944, 40216176119, 40216175725, 40216173751, 40216174979, 40216175075, 40216174260 | GitHub Actions | Founder (push approved) |
| 2026-10-25 | Gate 1 PASSED with the model NotVerified (held-out C/10 RMSE 71.1-77.3 mV vs < 46 mV); error bound published; search for a second LG M50 dataset for a fair re-test; Phase 2 not started | validation/GATE1_VALIDATION_REPORT.md; CI run 40185418438 | Founder |
| 2026-10-25 | Gate 0 PASSED after review of the Phase 0 report; Phase 1 not started (needs D1, D2, D4) | Gate 0 evidence above; CI run 40180917041 | Founder |
