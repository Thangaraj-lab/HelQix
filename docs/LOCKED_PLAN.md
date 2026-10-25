# HELQIX Locked Plan (v1.0, 2026-10-25)

Single-page summary of what is locked. Full text and reasoning: [docs/plan/](plan/00_README_INDEX.md) (files 01, 02 and 06 are the locked sources). Locked sections change only with evidence (a failing test, pilot feedback, or a verified source), and every change is logged in the decision log in [plan/08](plan/08_OPEN_DECISIONS_AND_HYPOTHESES.md).

## Product

HELQIX = "Engineering truth, with proof." An Engineering Executable-Reality Platform: it turns an engineering model and its requirements into one authoritative, deterministic, observable, replayable and branchable reality, validates it against measured data, and delivers every conclusion with signed provenance and a stated confidence.

- It owns model, state, scenario, execution identity, lineage, verification and evidence. Solvers and tools are capabilities it orchestrates.
- It is not a simulator, CAD or CFD replacement, chatbot, 3D viewer, dashboard, or an EV/drone/quantum product. It is not a digital twin until a model is validated against measured data.
- Domain-agnostic core (System, Component, Property, Constraint, Environment, Physics, Scenario, State, Observation, Failure). Battery or motor is only the first domain pack. No domain logic in core layers.
- Paying moment: "Failure reproduced, fix proven in a branch, signed evidence in hand."

## The 8 immutable principles

1. One authoritative engineering state. 3D scene, AI output, solver output and edge data are never automatically truth.
2. Three states: Authoritative (computational truth), Derived (display only), Observed (measurement). Observed data never silently overwrites authoritative state.
3. Every value carries Origin (Observed, Simulated, Derived, AI-hypothesis) x Status (Verified, Unverified).
4. Every run declares a determinism level: Bitwise (core), Tolerance (external solvers), Statistical (stochastic or quantum).
5. Credibility is contextual: Context of Use + risk level + credibility achieved. No blanket "Verified".
6. Causal claims are graded: model-causal (branch intervention) vs validated-causal (after validation against measurement).
7. AI proposes, HELQIX validates, a human approves. Permission ladder: Read, Analyze, Plan, Propose, Simulate, Test, Execute, Mutate.
8. Never fake: results, quantum execution, state, evidence, maturity labels.

Also locked: verification (solved right) is kept separate from validation (matches reality); calibration and validation use different data; determinism is not correctness.

## Layer mapping (10 logical layers on 23 folders; folders are not renamed)

| Logical layer | Repo folders |
|---|---|
| 1 Intent, Requirements, Context of Use | 19-intelligence, 13-verification |
| 2 Model and Validated Library | 03-engineering, 18-libraries |
| 3 Scenario and Physics | 02-reality, 06-simulation |
| 4 Execution Fabric | 05-execution, 16-adapters, 17-integrations, 20-platform |
| 5 State and History | 04-state |
| 6 Time and Lineage | 07-time, 11-branching, 12-search |
| 7 Observation and Events | 08-observation, 09-failure, 10-events-causality |
| 8 Trust | 13-verification, 23-validation |
| 9 Agent | 19-intelligence |
| 10 Presentation and Integration | 14-visual-runtime, 15-sdk, 21-app, 22-api |

## Locked flow (v3)

Intent -> Requirements + Context of Use + Risk -> Model -> Code and Solution Verification -> Simulate (deterministic, level declared) -> Observe -> Failure -> Causality -> Freeze -> Replay -> Branch -> Compare -> Calibrate (training data) -> Validate (held-out data) -> Optimize (carrying validation status) -> Signed Evidence -> Report path, or Build Spec path -> Real product -> Edge observation -> new data -> back to Model.

Back-edges: Failure -> Model; Verification fail -> Requirements or Model; Validation gap -> Calibrate; new measurements -> Model.

## Locked build order (one gate at a time)

| Step | Build | Exit gate |
|---|---|---|
| 0 | Hygiene: workspace builds, tests honest, target/ out of git, CI, runtime trace, docs | cargo build and test green with raw output; RUNTIME_TRACE, LOCKED_PLAN, STATUS written |
| 1 | Requirements + Context of Use + risk; benchmarks; numerical integrity; uncertainty and sensitivity; calibrate/validate with held-out split; validated library metadata | A published model error bound against measured data |
| 2 | Freeze, exact replay, determinism levels | Replay is bit-exact for the core runtime |
| 3 | Causal graph, failure investigation | Click-through from failure to candidate root cause with equations and values |
| 4 | Counterfactual branches, Compare | Intervention tests pass; incompatible comparisons disclosed |
| 5 | Canvas polish, signed evidence, verification workspace | Evidence package re-verified by a separate script |
| 6 | Adapters (PyBaMM, CalculiX first) | Consistency checks pass against the core |
| 7 | Edge (rig first, phone second), agent permissions, quantum | Measured data flows into observation; permission ladder enforced |
| 8 | Build Spec, research | Only after steps 1 to 5 are proven |

Signed evidence belongs to step 5. Step 0 only repairs compilation of the evidence crate.

## Kill and relabel triggers

- Replay not reproducible -> drop the determinism claim.
- Model error above threshold X -> no "Verified" label.
- Branch causal claim fails an intervention test -> relabel as correlation.
- Zero of 3 pilots commit to an evidence report -> re-evaluate the wedge.

## Open decisions (not decided here; founder only)

| # | Decision | Status |
|---|---|---|
| D1 | First domain pack (battery cell vs motor thermal vs other) | DECIDED: battery cell ([PHASE1_INPUTS.md](PHASE1_INPUTS.md)) |
| D2 | Context of Use text and validation error threshold X | DECIDED: voltage RMSE < 46 mV on held-out C/10 ([PHASE1_INPUTS.md](PHASE1_INPUTS.md)) |
| D3 | V&V standard family (ASME V&V 10/20, V&V 40, NASA-STD-7009) | OPEN, before pilot reports |
| D4 | Validation rig design, data plan, held-out split protocol | DECIDED for Phase 1: public data, C/10 held out; rig deferred to Phase 7A |
| D5 | Authoritative runtime owner (Rust crates vs Python gateway vs TypeScript app) | DECIDED: Rust core authoritative, gateway calls it |
| D6 | Naming cleanup HELIX -> HELQIX in code | OPEN; report in [STATUS.md](STATUS.md) |
| D7 | Phase 0 hygiene step before locked step 1 | APPROVED by founder 2026-10-25 |
| D8 | First external adapter | Deferred to step 6 |
| D9 | Requirements language format | OPEN, Phase 1 |
| D10 | Pricing, buyer, unit economics | After pilots |
| D11 | Funding and monthly budget | OPEN, before buying hardware |
| D12 | Code reviewer (senior engineer or professor) | OPEN, after Phase 1 |
