# HELQIX v1: complete analysis and remaining work

Written 2026-10-28 on branch `ui-studio-v2` (CI green at `da61bd0b`, 12 of 12 jobs). Nothing here is merged to `main`.

Labels follow the plan: **VERIFIED** (a test or a measurement I ran or read), **CALCULATED**, **ASSUMPTION**, **HYPOTHESIS**, **UNKNOWN**. Effort figures are **ESTIMATE** only and depend on decisions and data that do not exist yet. Where I did not check something, it says so.

---

## 1. Verdict in one page

| Question | Answer | Label |
|---|---|---|
| Is the engine core real and tested? | Yes: deterministic point-mass runtime, freeze, bit-exact replay, branch with intervention tests, signed evidence with an offline verifier, external solver adapters, and a permissioned agent. All have passing tests and CI. | VERIFIED |
| Is the model scientifically trustworthy? | No. The battery model is **NotVerified**: held-out RMSE 71.1 to 77.3 mV against the 46 mV target the founder set (gate 1). The error bound is published, the claim "Verified" is not allowed. | VERIFIED |
| Is there a product a user can use end to end? | Partly. The Studio runs on the mock engine fully and on the real engine for: projects, ops-based model building, revisions, stored-run playback, starting a point-mass run, freeze, branch, and AI-proposed parts. A Studio project's parts are **not** translated into a solver model except as one lumped body. | VERIFIED |
| Has the 7A gate (real measured data) passed? | **No.** It needs rig data only the founder can produce. | VERIFIED |
| Is the streaming (K7) decision made? | **No.** Only a CPU baseline exists. Go/no-go needs a hardware-encoder GPU run. | VERIFIED |
| Is v1 shippable today? | No. The remaining work below is roughly: 6 founder-only items, about 25 engineering work packages, and a release phase. | ASSESSMENT |

What is strong: trust machinery (replay, signing, tamper tests, permission ladder) and honesty discipline (labels, mutation checks, CI as the authority). What is weak: validated science, a real solver behind the Studio, accounts and operations, and everything that depends on a GPU or on measured data.

---

## 2. Evidence base (what this report stands on)

- Gate tracker and evidence table in [STATUS.md](STATUS.md); plan files `docs/plan/00` to `39`.
- CI on `ui-studio-v2` at `da61bd0b`: Rust build and test (Linux), Contracts and staged-build API (K2), Studio UI (K8), Licence gates (K1), Dev stack (K1), FMI, CalculiX, PyBaMM, Evidence signing (K5), Asset pipeline (K6), Python 3.12 and 3.14. All success.
- CI on `stack-k7-stream-spike` at `2cd51b04`: 12 of 12 jobs success including "Cloud render spike, CPU baseline (K7)".
- Local runs this session: UI 46 tests pass, 6 skipped (real-API tests run only in CI), type check and build clean; `helqix-agent` 6 unit + 10 permission tests pass; one env-gated test with a real local model passed.
- Local limits (VERIFIED): Windows Smart App Control blocks freshly built test programs (os error 4551); `cargo build --workspace --locked` cannot find `sqlx` locally. Linux CI is therefore the authority for Rust API tests.
- Local hardware (VERIFIED): AMD Radeon 840M integrated GPU, no NVIDIA, no Docker, no WSL, no GStreamer.

---

## 3. Gate by gate (locked order, plan file 06)

| Gate | Status today | What remains to close it | Owner | Effort (ESTIMATE) |
|---|---|---|---|---|
| 0 Hygiene | PASSED | Naming cleanup HELIX to HELQIX in code (decision D6 open); doc-comment warnings across crates | Engineer | 1 to 2 days |
| 1 Trust core, model error bound | PASSED as a process; **model NotVerified** (71 to 77 mV vs under 46 mV) | Either improve the model (parameters, SPM vs DFN, thermal terms) and re-run held-out, or keep the claim at "NotVerified" and say so in every report. Also review by a senior engineer or professor (D12) | Engineer + Founder | 1 to 3 weeks if the model is improved; decision only if not |
| 2 Bit-exact replay | Criteria met, awaiting founder review | Founder review: run `helqix-replay-check` yourself | Founder | 1 hour |
| 3 Causal click-through | Criteria met with a limitation: the expected stoichiometry chain is missed (nonlinear onset) | Fix or document: a nonlinear-onset detector, or label causal claims as correlation where it fails (kill trigger in file 06) | Engineer | 1 to 2 weeks |
| 4 Branch intervention tests | Criteria met, awaiting review. The Studio now freezes and branches a point-mass run through the API | Review; extend branching to non-point-mass models once they exist | Founder / Engineer | review 1 hour |
| 5 Signed evidence | Criteria met, re-verified on Linux CI. Offline verifier works; cosign sign, attest, verify offline all in CI | A **real** signing key held by you outside the repo; publish the public key; decide on transparency log (Rekor v2) | Founder | key: 1 hour; Rekor: later |
| 6 Adapters | PyBaMM, FMI/FMPy, CalculiX meet criteria. Others (OpenFOAM, FreeCAD, ROS, Gazebo, PX4) not started | CalculiX is GPL: confirm it stays a separate process (it is) and record the founder decision; add adapters only if v1 needs them | Founder decision | decision only |
| 7A Edge observation | **NOT PASSED.** Software done (ingest, rejection types, flags, content-addressed recordings, reconciliation) | **Real rig data.** Rig BOM and cost are UNKNOWN. Safety checklist (file 08) must be met before any lithium cell is bought | Founder | rig: 2 to 4 weeks; no code can substitute |
| 7B Agent with permissions | Criteria met with a scripted agent. Now also: a real local model (Qwen2.5-7B via Ollama) was injected and still could not change the design | Review by founder (run demo, approve yourself); connect the Studio assistant chat to the provider | Founder + Engineer | review 1 hour; wiring 2 to 4 days |
| 8 Build Spec, research | Not started | Only after 1 to 5 are accepted | Later | n/a |

---

## 4. Platform stack K0 to K9 (plan file 39)

| Item | Status | Evidence | Remaining |
|---|---|---|---|
| K0 Tool register and licence audit | Done (69 tools, all licences VERIFIED) | [TOOLS.md](TOOLS.md) | Versions and dates UNKNOWN for Temporal, SeaweedFS, Keycloak and others |
| K1 Dev stack, licence gates | Done | CI | Docker-based stack cannot run on this PC |
| K2 Contracts and staged-build API | Done | CI, curl demo against PostgreSQL | Authentication is a single API key (see section 7) |
| K3 Solver adapters | FMI, PyBaMM, CalculiX done with benchmarks | CI | CalculiX/GPL decision is the founder's |
| K4 Frames store | Parquet frames, BLAKE3, getFrame, runFrames, replay hash-match done | CI | **Object store (SeaweedFS), Postgres metadata for runs, DuckDB queries not done.** Frame store is a local directory |
| K5 Signed evidence | Done | CI: offline verifier under no-network, cosign v3.1.4 | Real key; Rekor v2 later |
| K6 Asset pipeline | Done (Blender build, meshopt, KTX2, validator, renders, licence register) | CI | A shipped part library does not exist; assets are fetched locally and git-ignored |
| K7 Stream spike | **CPU baseline only**: 40 of 40 samples per case, click-to-pixel p50 119.5 ms (RTT 0), 124 ms (40 ms), 207.9 ms (80 ms) on a CI VM with software x264 | CI, branch `stack-k7-stream-spike` | **GPU run with NVENC or equivalent (founder hardware), WebRTC server choice (LiveKit recommended), TURN, session lifecycle at scale** |
| K8 Studio UI | CI job green; mock and real engine; real-API client tests | CI | See section 5 |
| K9 Quantum | Not started | n/a | Decide whether it is in v1 (recommend: no) |

The local GPU is an integrated AMD part. A render-only measurement was taken: the Studio canvas on an empty project ran at 60 fps (p50 16.7 ms, p95 17.3 ms over 300 frames). That is a light scene and not a streaming result.

---

## 5. The Studio product, screen by screen

### 5.1 What works on the real engine (VERIFIED by tests unless noted)

- Projects: create, list, rename. Staged builds with an operation log: add part, place, port, connection, binding, requirement, undo, redo, commit, revisions.
- Simulate: play a **stored** run; start a point-mass run from typed numbers (`POST /v1/runs`, same input gives the same run id and frames); freeze at a time; branch with one changed parameter; play the branch.
- Describe a system: `POST /v1/interpret` asks a local open-source model for parts; the API validates (shape, size, position, count, plain-text names) and the Studio applies them labelled as AI hypotheses with placeholder maturity.
- Run your own model: parts plus a stated density give one lumped body (mass from volume times density, frontal area from the bounding box). A part without a density makes the model unrunnable and is named; nothing is invented.
- Assets: glTF import and HDRI upload with structure checks and a recorded licence.

### 5.2 Gaps by stage

| Stage | Gap | Impact | Fix |
|---|---|---|---|
| Define | The model is geometry plus ports; the engine knows nothing about electrical, thermal or mechanical meaning of connections | High: this is why only a lumped body can be simulated | A model schema with physical roles and a translator to FMI/Modelica or domain packs; start with the battery domain pack |
| Define | Interpretation proposes parts only, not connections, ports, bindings or requirements | Medium | Extend the proposal schema and validator step by step |
| Model | 3D scene is correct but there is no part library and no automatic mass or material from catalog | Medium | Shipped CC0 library with density and licence records |
| Simulate | Runs are point-mass only; stored runs are not tied to a model | High | Run a model through a solver adapter and store its frames with the model revision |
| Understand | Causal chain is shown from stored data; engine causal graph is limited (nonlinear onset) | Medium | Gate 3 work |
| Improve | Real-engine comparison is by switching runs; no side-by-side verdict panel from the engine's intervention tests | Medium | Wire intervention-test verdicts (gate 4) into Improve |
| Verify | Evidence sheet hashes and replay check exist; export of a signed package from the UI is not wired | Medium | An API endpoint that builds and signs a package server-side |
| Observe | PREVIEW only; needs 7A data | High | Founder data |
| Build | PREVIEW; Build Spec is gate 8 | Low for v1 | Defer |
| Assistant | CORRECTION 2026-10-28: the Studio assistant is a rule-based suggestion bar computed from project state, not a chat, so there is nothing to wire; AI enters through `/v1/interpret` | None | Optional: a real chat later, under the permission ladder |
| All | I did not click through the newest real-engine screens in a browser (density field, "Use my model's mass and area", Freeze and Branch). Tests cover the logic only | Medium | A browser walkthrough with screenshots |

### 5.3 Quality items not yet done (UNKNOWN unless stated)

- Accessibility audit (axe-core is in the register, not run on every stage).
- Keyboard navigation and focus order on drawers and the 3D canvas.
- Mobile and tablet layouts were checked only for earlier screens.
- Performance budgets for large models (hundreds of parts, large glTF) not measured.
- Error and empty states on API failure for every screen: partly done.
- Internationalisation: not started.

---

## 6. Tool integration matrix (69 tools in `tools/TOOLS.lock.json`)

| Group | Tools | State |
|---|---|---|
| Platform core | axum, tokio, serde, sqlx, PostgreSQL, BLAKE3, Arrow, Parquet | Integrated and in CI |
| Solvers | FMPy, Modelica Reference FMUs, PyBaMM, CalculiX (CLI), SciPy, Pint | Integrated (benchmarks in CI). SUNDIALS not checked. OpenModelica excluded (AGPL/custom licence) |
| Trust | cosign, in-toto, offline verifier | Integrated. SLSA generator and Rekor v2 not done |
| Assets | Blender CLI, glTF-Transform, meshoptimizer, Basis Universal, glTF-Validator, Open Image Denoise | Integrated (K6) |
| Client rendering | three.js, glTF loaders | Integrated. three-gpu-pathtracer and detect-gpu: present in register, wiring not verified |
| Stream | Playwright, Chromium, GStreamer, x264, TURN | Spike works on CPU. LiveKit, mediasoup, Selkies, Docker, NVIDIA Container Toolkit not integrated |
| Data services | DuckDB, Zarr, pgvector, NATS, Temporal, SeaweedFS | **Not integrated** |
| Identity and ops | Keycloak or Ory Kratos, OpenTelemetry Collector | **Not integrated** |
| Physics engines | Rapier, Jolt | Not integrated |
| Numerics (heavy) | FEniCSx, MOOSE | Not integrated |
| Uncertainty | SALib, OpenTURNS, Hypothesis, proptest | Register only for SALib and OpenTURNS: use unconfirmed |
| Quantum | Qiskit, Qiskit Aer, PennyLane | Not started |
| AI | Ollama (MIT), Qwen2.5-7B-Instruct (Apache-2.0) | Integrated in agent and `/v1/interpret`; not in the chat |

---

## 7. Engineering quality, security and operations

| Area | Today | Needed for v1 |
|---|---|---|
| Authentication | One shared API key sent as a bearer token; the Studio dev proxy adds it server-side | Real accounts and sessions (Keycloak or Kratos), per-user ownership of projects, roles |
| Authorisation | Not per user | Row-level ownership, audit of who approved what |
| Secrets | Keys in env and git-ignored `.env.local`; nothing in git (VERIFIED by licence and policy gates only; no dedicated secret scan seen) | Add a secret scanner to CI |
| Multi-tenancy | None | Tenant id on every record, quota and rate limits |
| Storage | Local directory for frames and specs; PostgreSQL for projects and builds | Object store with versioning, backup and restore tested |
| Backups and disaster recovery | Not done | Backup plan, restore drill |
| Observability | `tracing` logs only | OpenTelemetry traces and metrics, error reporting, health dashboards |
| Rate limiting and abuse | Body size and timeout layers; `/v1/interpret` can take about 30 to 40 s on CPU | Per-user rate limits, queue for model calls |
| Reproducible builds | `Cargo.lock` now committed (a missed file broke CI once) | Lock files for Python and npm checked in CI |
| Packaging | None | Release build, container images, installer for the desktop case |
| Local developer experience | Windows blocks fresh test programs; sqlx missing locally | Document the Linux-CI-first workflow or provide a dev container |
| Git access | The credential dialog does not work from non-interactive runs; pushes need your sign-in | Use a token with the credential manager, or push from your terminal |
| GitHub API polling | Unauthenticated limit of 60 per hour was hit twice | Use an authenticated token for CI reads |
| Code review | No senior review yet (D12) | Independent review of tests, especially numerical ones |
| Technical debt | Many "missing documentation" warnings; HELIX/HELQIX naming mixed | Cleanup passes |

---

## 8. Science and validation (the part that decides trust)

1. **Model error bound.** Target RMSE under 46 mV on held-out C/10; achieved 71.1 to 77.3 mV. Options: (a) improve the model, (b) widen the published bound and keep "NotVerified", (c) change the Context of Use with a recorded decision. Option (c) must come from the founder with a stated reason.
2. **Own measured data (7A).** Public data (Zenodo 4032561) was used for Phase 1. Whether a public dataset can substitute for own data is HYPOTHESIS H6. A single-cell rig is H4.
3. **Thermal failure root cause.** Whether the single-particle model is enough (H3) is UNKNOWN; the causal chain for the thermal case was not checked.
4. **Uncertainty and sensitivity.** SALib and OpenTURNS are registered; propagation of parameter uncertainty into reported results is not demonstrated in the Studio.
5. **V&V standard (D3).** Open. Needed before any pilot report is presented as a verification document.
6. **Compliance claims** (ISO 26262, DO-178C tool qualification) are UNVERIFIED and must not be marketed.

---

## 9. Legal, IP and business (founder-only unless noted)

| Item | State |
|---|---|
| Licence of the repository | MIT, root `LICENSE` and all manifests. Holder name "HELQIX contributors" is a placeholder for you to confirm |
| Third-party licences | Audited in K0; AGPL and source-available excluded; GPL tools only as separate processes |
| Trademark and domain for HELQIX | Not done |
| Patent search | Not done |
| 5 engineer interviews (H1) | Not done |
| 3 pilots (H5) and pricing (D10) | Not started |
| Funding and budget (D11) | Open; hardware purchases depend on it |
| Security and safety for lithium cells | Written requirements exist; no cell may be bought before they are met |

---

## 10. Work breakdown (ordered, with acceptance criteria)

Priority: **P0** blocks v1, **P1** needed for a credible v1, **P2** can ship after.

### Founder-only

| ID | Work | Acceptance | Priority |
|---|---|---|---|
| F1 | Rig data for 7A (design, safety, record, hash, split) | Raw recordings in the ingest format; `helqix-edge-check` passes on real data | P0 |
| F2 | GPU streaming run (hardware encoder) | K7 table with p50 and p95 latency on your GPU; go/no-go written | P0 |
| F3 | Review gates 2 to 7B and sign off in STATUS.md | Dated acceptance lines | P0 |
| F4 | Create and keep the cosign key; publish the public key | Evidence package verifies with the committed public key | P0 |
| F5 | Decisions D3, D6, D9, D11, D12 and the CalculiX/GPL stance | Entries in the decision log | P0 |
| F6 | Trademark, patent search, interviews, pilots | Reports | P1 |
| F7 | Confirm MIT holder name | `LICENSE` updated | P1 |

### Engineering

| ID | Work | Depends on | Acceptance | Priority | Effort |
|---|---|---|---|---|---|
| E1 | Browser walkthrough of new real-engine screens with screenshots; fix findings | none | Written check list, zero blocking defects | P0 | 1 day |
| E2 | ~~Wire Studio assistant chat to Ollama~~ NOT NEEDED (the assistant is a rule-based bar; AI is in `/v1/interpret`) | n/a | n/a | closed | 0 |
| E3 | Interpretation v2: connections, ports, requirements | E2 | Validated proposals for a pump, a battery pack and a gearbox | P1 | 1 week |
| E4 | Model schema with physical roles; translator to a solver adapter (FMU first). First step DONE: lumped-body translation in Rust (`helqix-modelbuild::lumped`) | none | A Studio model runs through FMPy and the frames store, tied to its revision | P0 | 3 to 5 weeks |
| E5 | Battery domain pack in the Studio (PyBaMM adapter behind a Studio model) | E4 | A cell model defined in the Studio reproduces the reference run | P0 | 2 to 3 weeks |
| E6 | Link runs to model revisions; real `listRuns` | E4 | DONE locally 2026-10-28 (commit 05ce5f0a): runs record project and revision; `GET /v1/projects/{id}/runs`; CI result pending (push blocked on sign-in) | P0 | done |
| E7 | Improve stage on engine intervention verdicts | E4 | Verdict from `intervention_test` shown, incompatible comparison disclosed | P1 | 1 week |
| E8 | Evidence export from the UI (server builds and signs) | F4 | Downloaded package passes the offline verifier | P0 | 1 week |
| E9 | K4 completion: object store (SeaweedFS), run metadata in Postgres, DuckDB queries | none | Runs survive a restart and a second machine; BLAKE3 unchanged | P1 | 2 weeks |
| E10 | Accounts and ownership (Keycloak or Kratos), per-user data | none | Two users cannot see each other's projects; tested | P0 | 2 to 3 weeks |
| E11 | Observability (OpenTelemetry), error reporting, dashboards | none | Traces for every API call; alert examples | P1 | 1 week |
| E12 | Rate limits and a queue for model calls | E10 | PARTLY DONE: one model call at a time, 429 busy, tested. Per-user limits wait for accounts | P1 | partly done |
| E13 | Packaging: container images, release build, install guide | E10 | Clean-machine install in under 30 minutes | P0 | 1 to 2 weeks |
| E14 | Accessibility and keyboard pass; mobile layouts | none | PARTLY DONE: axe-core structural rules pass on Hero, Home, New project (one real defect fixed); Studio stages, keyboard walk and contrast still to do | P1 | partly done |
| E15 | Performance budgets for big models and imports | none | Frame time and memory budgets enforced in tests | P1 | 1 week |
| E16 | Causal limitation (gate 3): nonlinear onset | none | The missed chain is found, or the claim is relabelled with a test | P1 | 1 to 2 weeks |
| E17 | Model accuracy work (gate 1) | F5 | Held-out RMSE under the agreed threshold, or an approved wider bound | P0 | 1 to 3 weeks |
| E18 | Uncertainty and sensitivity in results (SALib) | E4 | Reports show intervals with their method | P1 | 1 to 2 weeks |
| E19 | K7 production design: LiveKit/WebRTC, TURN, session limits | F2 | Decision record plus a 10-session soak | P1 | 2 weeks |
| E20 | Secret scanning and dependency updates in CI | none | DONE locally: `scripts/secret_scan.py` (0 findings on the repo; tests plant secrets); wired into the licence job; dependency updates not done | P1 | partly done |
| E21 | Naming cleanup HELIX to HELQIX, documentation warnings | F5 | Zero mixed names; warning count reduced | P2 | 2 days |
| E22 | Part library with density and licence records | none | 20 CC0 parts checked by the importer | P2 | 1 week |
| E23 | Rapier or Jolt for rigid-body scenes | E4 | Only if v1 scope needs it | P2 | 2 weeks |
| E24 | K9 quantum module | none | Only if v1 scope needs it | P2 | 2 weeks |
| E25 | User docs, quick start, sample project end to end | E4 to E8 | PARTLY DONE: [QUICKSTART.md](QUICKSTART.md); sample project and golden demo from the docs not done | P0 | partly done |

### Release

| ID | Work | Acceptance |
|---|---|---|
| R1 | Full CI on a release branch, all gates green, licence audit refreshed | Green run recorded |
| R2 | Status board and STATUS.md re-scored with evidence | Every label backed by a test or demo |
| R3 | Founder merge decision and tag | Your decision; I will not merge |

---

## 11. Critical path

1. E4 (model to solver) is the longest chain and unlocks E5, E6, E7, E18 and E25.
2. F1 (rig data) is independent of code but gates the 7A claim and the "measured" part of the story. Start it now because it has the longest waiting time.
3. F2 and F4 are short for you and unblock E19 and E8.
4. E10 (accounts) and E13 (packaging) can run in parallel with E4.

ESTIMATE, one engineer working alone: about 14 to 20 weeks for all P0 and P1 items, assuming no blocker from F1, F2, F5. P2 items add 4 to 6 weeks.

---

## 12. Proposed v1 definition (for your approval)

Must have: one domain (battery cell) from description to signed evidence: describe or build a cell model, run it through a real solver, freeze and replay it, branch with an intervention test, see the causal chain, export a signed package that an independent script verifies, plus accounts, packaging and docs, and an honest model error bound.

Should have: measured-data ingest from your rig (7A), streamed high-quality rendering (K7 decision), uncertainty intervals.

Not in v1: quantum (K9), Build Spec (gate 8), rigid-body engines, FEniCSx/MOOSE, interoperability beyond FMI, collaboration features.

---

## 13. Risks and kill criteria

| Risk | Trigger | Response |
|---|---|---|
| Model never meets the threshold | E17 fails twice | Keep "NotVerified", publish the bound, change the pitch to evidence and replay rather than prediction |
| No GPU path | F2 fails | Ship CPU streaming for small scenes or stay client-rendered |
| Rig data unavailable | F1 slips more than 8 weeks | Label Observe as preview in v1 |
| One-engineer capacity | E4 overruns | Cut to a single FMU path and defer E5 |
| AI proposal quality | Proposals mislead | They are labelled hypotheses and validated; keep human approval mandatory |
| Compliance overclaim | Marketing text | No ISO or DO-178C language until reviewed |
| Replay not reproducible on another machine | Cross-platform hash mismatch | Drop the determinism claim for that component (kill trigger in file 06) |

---

## 14. Honest unknowns in this report

- The exact test counts per crate after Phase 0 were taken from STATUS.md; I did not rerun the whole Rust suite locally (blocked by Smart App Control).
- Whether SALib, OpenTURNS, SUNDIALS, three-gpu-pathtracer and detect-gpu are used anywhere in code was not checked.
- Effort figures are guesses from the size of similar work in this repository, not commitments.
- K9 and gate 8 scope were not re-read in full for this report.
