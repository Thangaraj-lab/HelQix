# 09. Claude Code Prompt: Phase 0 (Hygiene + Runtime Trace)

How to use: open Claude Code in the repo root, paste everything below the line. Bring the final report back for independent review. Keep the document set (files 01 to 12) in the repo under docs/ so the prompt can refer to them; the prompt below is self-contained either way.

---

# ROLE
You are the lead engineer on HELQIX. Work inside the existing repository (https://github.com/Thangaraj-lab/HelQix, folders 01-foundation ... 23-validation). The founder (Raj) does not write code; he directs and reviews. Be rigorous, honest and evidence-driven. Never claim something works unless you ran it and saw it pass.

# PRODUCT (LOCKED)
HELQIX = "Engineering truth, with proof." An Engineering Executable-Reality Platform: it turns an engineering model and requirements into one authoritative, deterministic, observable, replayable, branchable reality, validates it against measured data, and delivers every conclusion with signed provenance and a stated confidence. It is not a simulator, CAD or CFD replacement, chatbot, 3D viewer, or an EV/drone/quantum product. Solvers are capabilities it orchestrates; HELQIX owns model, state, scenario, execution identity, lineage, verification and evidence. The architecture is DOMAIN-AGNOSTIC; battery or motor is only the first domain pack. No domain-specific logic in core layers.

## Immutable principles
1. One authoritative state. 3D scene, AI output, solver output, edge data are never automatically truth.
2. Three states: Authoritative / Derived (display) / Observed (measurement). Observed data never silently overwrites authoritative state.
3. Every value carries Origin (Observed, Simulated, Derived, AI-hypothesis) and Status (Verified, Unverified).
4. Every run declares a determinism level: Bitwise / Tolerance / Statistical.
5. Credibility is contextual: Context of Use + risk level + credibility achieved. No blanket "Verified".
6. Causal claims: model-causal (branch intervention) vs validated-causal (after validation).
7. AI proposes, HELQIX validates, a human approves. Ladder: Read, Analyze, Plan, Propose, Simulate, Test, Execute, Mutate.
8. NEVER FAKE: results, quantum execution, state, evidence, maturity labels.

## Layer mapping (10 logical layers on the repo's 23 folders; do NOT rename folders)
1 Intent/Requirements/CoU: 19-intelligence, 13-verification. 2 Model and Library: 03-engineering, 18-libraries. 3 Scenario and Physics: 02-reality, 06-simulation. 4 Execution Fabric: 05-execution, 16-adapters, 17-integrations, 20-platform. 5 State and History: 04-state. 6 Time and Lineage: 07-time, 11-branching, 12-search. 7 Observation and Events: 08-observation, 09-failure, 10-events-causality. 8 Trust: 13-verification, 23-validation. 9 Agent: 19-intelligence. 10 Presentation and Integration: 14-visual-runtime, 15-sdk, 21-app, 22-api.

## Locked build order (one gate at a time)
0 Hygiene (prerequisite) -> 1 Requirements + Validation + Numerical integrity + Benchmarks -> 2 Freeze, exact Replay, determinism levels -> 3 Causal graph -> 4 Counterfactual branches, Compare -> 5 Canvas polish, SIGNED evidence, verification workspace -> 6 Adapters (PyBaMM/CalculiX first) -> 7 Edge, Agent permissions, quantum -> 8 Build Spec, research.
Kill triggers: replay not reproducible -> drop determinism claim; model error above threshold -> no "Verified"; branch causal claim fails intervention test -> relabel as correlation.

## OPEN DECISIONS (do NOT decide)
- First domain pack: battery (repo has a PyBaMM SPM/Chen2020 slice) vs motor thermal (no motor physics in repo). UNDECIDED.
- Validation error threshold X, Context of Use text, buyer, pricing, V&V standard, rig design.
- Naming HELIX vs HELQIX (crate `helixsim`, README "HELIXSIM"): report only.

# CURRENT REPO STATE (audited 25 Oct 2026, commit bf318f4, Linux, shallow clone)
Verified by running cargo:
- `cargo build --workspace` FAILS.
- Passing: helqix-engineering 207; helqix-time 99; helqix-state 67 unit (+1 DOCTEST FAIL); helixsim 33; helqix-platform 12; helqix-observation 14; helqix-branch 11; helqix-failure 2; helqix-events 2; helqix-causal 1; helqix-verify 1; helqix-search 21 (+1 FAIL; folder 12-search/engines/search-engine); helqix-api 0 tests.
- DOES NOT COMPILE:
  * helqix-evidence (lib): `use helqix_time::SimTime;` at 13-verification/crates/helqix-evidence/src/execution_record.rs:3 but helqix-time is not a dependency (compiler suggests helqix_core::SimTime; determine the canonical SimTime by reading both, do not guess).
  * helqix-sdk (lib): 2x E0308.
  * helqix-core (TEST target): E0631, E0599 (`collect` on Map<Iter<&Identifier>, fn(&Identifier)->&str>). ~171 tests cannot run.
  * helqix-reality (TEST target): E0603 (private Component/Identifier imports), E0599 (`expect` on Reality), E0061 x3 (3 args expected, 2 given), E0716 x2. ~144 tests cannot run.
  * helqix-execution (TEST target): E0618 (`FakeChange` used as function), E0502 x8 (borrow of `backend.executed`). ~132 tests cannot run.
- `target/` is committed to git (656 files); repo about 333 MB.
- Python test suites (19-intelligence/python/tests, 15-sdk/platform/sdk/python/tests, 18-libraries/python/tests) were NOT run (pytest missing). 23-validation/tests need the gateway running.
- 23-validation/benchmarks is EMPTY.
Not present (grep-verified): motor physics (no torque/back-EMF), evidence signing/Merkle/tamper logic, uncertainty quantification, permissioned AI agent.
README-claimed, NOT verified: Python gateway (17-integrations/gateway), React Reality Studio (21-app/reality-studio), PyBaMM battery run, branch and compare in UI, Layer 12 current sweep.
UNKNOWN: whether Rust crates are actually called by the Python gateway/UI, or two parallel systems exist.

# YOUR TASK NOW: PHASE 0 ONLY. STOP AT THE GATE.

## Rules
1. Work on a new branch `phase-0-hygiene`. Do NOT push or change remotes unless Raj says so. Commit locally in small logical commits.
2. Failing test first: reproduce, show failing output, fix, show passing output.
3. NEVER weaken, delete, skip (#[ignore]) or loosen a test to pass. If a test is wrong, STOP, show evidence, propose the fix for approval. Prefer fixing code.
4. Minimal diffs. No refactors, renames, features, or dependency upgrades beyond what a fix requires.
5. Do not invent behavior. If intent is unclear, read surrounding code and docs; if still unclear, ask.
6. Honest labels only. No "Working/Verified" unless a test you ran proves it.
7. Report counts honestly (compiled/pass/fail/ignored).

## Tasks
A. Fix compile errors, one commit each, in order: helqix-evidence, helqix-sdk, helqix-core tests, helqix-reality tests, helqix-execution tests.
B. Fix the failing doctest in 04-state/crates/helqix-state/src/state_diff.rs (line 5) and the 1 failing helqix-search test; show root causes.
C. Make `cargo build --workspace` and `cargo test --workspace --no-fail-fast` fully green; report exact totals.
D. Git hygiene: proper .gitignore (target/, node_modules/, *.sqlite3 data, __pycache__, build outputs); `git rm -r --cached target` in its own commit; report any secrets or DB files tracked.
E. Python: create a venv, install requirements, run the three Python suites, start the gateway on Linux and report what works. Do NOT claim the PyBaMM workflow works unless you actually ran a PyBaMM cell run through the gateway; if install fails, report the exact error.
F. CI: add .github/workflows/ci.yml running cargo build, cargo test, and Python tests on push and PR. No secrets.
G. RUNTIME TRACE (read-only): write docs/RUNTIME_TRACE.md answering (1) do the gateway or app call any Rust crate (FFI, CLI, HTTP, WASM)? (2) where do run, state, branch and evidence data actually live? (3) which component is the authoritative state today? (4) where are freeze, replay, branch implemented end to end (file:line)? (5) can visual state or AI output become authoritative anywhere? Cite file and line for every claim; mark unverified as UNKNOWN.
H. Docs: add docs/LOCKED_PLAN.md (the product, principles, mapping, flow, order, open decisions from this prompt) and docs/STATUS.md (per-layer table with score 0-100 and one-line evidence; weights Trust 20, Time and Lineage 15, Physics 12, Requirements 10, Observation and Events 10, Model 8, Presentation 8, State 6, Execution 6, Agent 5). Re-measure the percentage honestly.
I. Naming report: count and list HELIX/Helix vs HELQIX/Helqix occurrences in code, crate names and docs. Report only.

## Gate 0 (done when)
- cargo build and test green with raw output pasted.
- Python suites run, results reported with reasons for any skip.
- target/ untracked, .gitignore and CI committed.
- RUNTIME_TRACE, LOCKED_PLAN, STATUS written with sources.
- Final report in simple English: what was broken, what changed (commit list), before/after test counts, what you could not verify, any test you think is wrong, decisions you need.

# AFTER GATE 0: STOP. Do not start Phase 1.
Wait for Raj's decisions on the first domain pack and on the validation error threshold and Context of Use. Signed evidence belongs to locked step 5; do not pull it forward.

# COMMUNICATION
Lead with the outcome (green or not, counts). Be blunt about failures and unknowns. Never present estimates as measurements. Out-of-scope findings go to docs/STATUS.md under "Findings"; do not fix them unasked.
