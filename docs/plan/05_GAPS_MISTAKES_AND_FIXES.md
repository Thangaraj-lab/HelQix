# 05. Gaps, Mistakes, and How to Fix Each

Three kinds of problems: (A) mistakes in the earlier spec or discussion, (B) defects in the repo, (C) missing layers.

## A. Mistakes in the spec and in the discussion (corrected)

| # | Mistake | Correction |
|---|---|---|
| A1 | Spec lists "coupled motor physics" as a current capability | Not in the repo. Move to Planned. |
| A2 | Spec marks 20 capabilities as CURRENT | Labels must be test-based (03). Most are library-level, Partial, Stub or Broken. |
| A3 | Spec claims 1 working quantum pathway as a prototype requirement | Not needed for the golden demo. Quantum is an optional module; classical baseline first. |
| A4 | "No one has freeze, branch and evidence" (said earlier in discussion) | Overclaim. FMI already has optional state save and restore; SPDM tools (Ansys Minerva with Aras) already give traceability, version control and audit trails. Real wedge is the integration: freeze plus deterministic replay plus branch lineage plus causal attribution plus signed evidence in one runtime. HYPOTHESIS until a patent and competitor search is done. |
| A5 | "Branch proves causality" | Only model-causal. Validated-causal needs a validated model. |
| A6 | "Rig costs 500 to 1500 rupees" | Unverified and likely low. INA219 modules measure up to 26 V and about 3.2 A with the stock shunt (VERIFIED, see 11). Total rig cost is UNKNOWN until a bill of materials exists. |
| A7 | Phone as the edge node | Phone gives IMU, vibration, timing, not current or temperature. Tier 1 = phone, Tier 2 = microcontroller with current sensor and thermocouple or thermistor. |
| A8 | "Verify" used for two things | Split: verification (solved right) vs validation (matches reality). |
| A9 | Calibrate and validate shown as one step | Must use a held-out split, else circular. |
| A10 | Standards mapping: "ASME V&V 40 for aerospace/auto" | V&V 40 was written for medical devices; its subcommittee considers it general enough for other physics-based disciplines (VERIFIED). It applies to physics models, not statistical or AI models. NASA-STD-7009 is the aerospace models-and-simulation standard. ISO 26262 and DO-178C tool-qualification routes are from memory: UNVERIFIED. |
| A11 | 5D scenario check marked everything green | Convenient. In a scenario where AI surrogates replace solvers, bitwise determinism breaks; incumbents already have digital-thread products and can copy snapshot or branch features. |
| A12 | Numeric scores (8/10, 6/10) given in discussion | No rubric. Ignore them. |
| A13 | Wedge "motor/battery" left open | Repo has a real solver only for battery. Decision needed (08). |
| A14 | "Three.js WebGPU is the foundation" | WebGPURenderer falls back to WebGL 2 but is described as experimental; WebGLRenderer is still the stable choice (VERIFIED, 11). |

## B. Repo defects and fixes (Phase 0 scope)

| # | Defect | Fix approach |
|---|---|---|
| B1 | helqix-evidence lib does not compile (missing helqix_time dependency) | Determine canonical SimTime (helqix_core vs helqix_time), fix the dependency or import. Do not guess. |
| B2 | helqix-sdk lib 2 x E0308 | Read the mismatch, fix types. |
| B3 | helqix-core test target does not compile (E0631, E0599) | Fix iterator or closure type in tests. Do not delete tests. |
| B4 | helqix-reality test target: private imports, missing method, wrong arity, temporaries | Tests are probably out of date against the API. Decide per case whether the test or the API is right; show evidence. |
| B5 | helqix-execution test target: FakeChange used as function, 8 borrow errors | Fix test fixture. |
| B6 | helqix-state doctest fails | Fix example. |
| B7 | helqix-search 1 failing test | Find root cause before touching the test. |
| B8 | `target/` committed; no proper .gitignore | .gitignore, `git rm -r --cached target`. |
| B9 | No CI | GitHub Actions: cargo build, cargo test, Python tests. |
| B10 | HELIX vs HELQIX naming (crate `helixsim`, "HELIXSIM" in README) | Report first; rename only with approval. |
| B11 | Authoritative state owner unknown (Rust vs Python gateway) | Write RUNTIME_TRACE with file and line citations. |
| B12 | Spec doc contradicts repo | Replace with this document set. |
| B13 | helqix-api has 0 tests | Add after Phase 0, tied to Trust work. |
| B14 | benchmarks folder empty | Phase 1. |

## C. Missing layers and their priority

| Priority | Missing | Why it matters |
|---|---|---|
| 1 | Validation and calibration (measured data, parameter estimation, held-out split) | Without it, "Verified" is empty. |
| 2 | Requirements and Context of Use (machine-checkable, risk level) | Verify needs pass or fail criteria. |
| 3 | Uncertainty and sensitivity | Decisions need bounds ("92 C plus or minus 4"); sensitivity says which parameters matter. |
| 4 | Numerical integrity (stiffness, step size, convergence, error estimate) | A deterministic wrong answer is still wrong. |
| 5 | Benchmark and regression suite with closed-form cases | Lets the platform verify itself. |
| 6 | Validated model library with credibility metadata | The real moat. |
| 7 | Determinism levels for external tools | Core is bitwise; solvers, GPU and quantum are not. |
| 8 | Interoperability: FMI/FMU, STEP, SysML v2, PLM and SPDM connectors | Customers already have SPDM; HELQIX sits above or beside it. |
| 9 | Collaboration and sign-off: roles, identity, e-signature | Certification needs human accountability. |
| 10 | State storage at scale: delta checkpoints, copy-on-write, content-addressed storage, branch pruning | Field data is large; branch explosion in optimization. |
| 11 | Deployment and tenancy: on-prem, air-gapped | Aerospace and defense will not upload models. |
| 12 | Control and firmware loop (SIL/HIL) | Real systems have controllers. |
| 13 | Metering and cost | Needed for routing and billing. |
| 14 | Failure knowledge base (privacy-controlled) | Data flywheel. |

## D. Overlapping concepts (merge in language, keep features)

Freeze, Replay, Branch, Compare = Time and Lineage. Observation, Failure, Causality = Observation and Events. Compute Fabric, Quantum, External adapters = Execution Fabric. Physics, Simulation, Solver = three words for one stack. Environment is a scenario input. Cognition, AI Agent, Build Intelligence = one Agent layer. Verify, Validate, Evidence = Trust. Optimization = automated branching. Edge = an observation adapter. Result: 27+ names become 10 layers; no feature is removed.

## E. Things that stay (verified strengths)

Verification vs validation split; determinism does not equal correctness; three-state separation; authoritative-state rule; Origin and Status badges; the "never fake" rule; AI proposal-only governance; branch as intervention; the counterfactual canvas.

## F. Safety gap in the earlier plan

A rechargeable lithium cell discharge rig is a fire hazard if built wrongly. The earlier plan did not mention it. Required before any cell is bought: protection circuit and cut-off, fuse, supervised testing, a fire-safe test area, written test limits. See 08.
