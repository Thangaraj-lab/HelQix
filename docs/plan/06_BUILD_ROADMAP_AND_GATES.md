# 06. Build Roadmap, Gates, Kill Criteria (LOCKED order)

## Working rules

1. One gate at a time. No next step before the current gate passes.
2. Failing test first: reproduce, then fix, then show passing output.
3. Raw test output reported, not summaries.
4. CI green or no merge.
5. Labels test-based (03). No "Working" without a passing test and a demo.
6. Never weaken or delete a test to make it pass; if a test is wrong, show evidence and ask.
7. Minimal diffs. No push unless the founder says so.

## Locked build order

| Step | Build | Exit gate |
|---|---|---|
| 0 (prerequisite, added) | Hygiene: workspace builds, tests honest, target/ out of git, CI, runtime trace, docs | cargo build and test green, raw output pasted; RUNTIME_TRACE, LOCKED_PLAN, STATUS written |
| 1 | Requirements with Context of Use and risk; benchmarks with closed-form cases; numerical integrity; uncertainty and sensitivity; calibrate and validate with held-out split; validated library metadata | A published model error bound against measured data |
| 2 | Freeze, exact replay, determinism levels | Replay is bit-exact for the core runtime |
| 3 | Causal graph, failure investigation | Click-through from failure to root cause works with equations and values |
| 4 | Counterfactual branches, Compare | Intervention tests pass; incompatible comparisons disclosed, no invented winner |
| 5 | Canvas polish, signed evidence, verification workspace | Evidence package independently re-verifiable by a separate script |
| 6 | Adapters (PyBaMM, CalculiX first) | Consistency checks pass against the core |
| 7 | Edge (sensor rig first, phone second), agent permissions, quantum module | Measured data flows into the observation layer; permission ladder enforced |
| 8 | Build Spec, then research items | Only after steps 1 to 5 are proven |

Signed evidence belongs to step 5. Only the compile repair of the evidence crate belongs to step 0.

## Phase 0 scope (details in 09)

Fix compile errors (evidence, sdk, core tests, reality tests, execution tests); fix the state doctest and the one search test; get cargo build and test green; gitignore and untrack target/; Python venv and tests; CI workflow; runtime trace; locked-plan and status docs; naming report.

## Kill and relabel triggers

- Replay not reproducible: drop the determinism claim.
- Model error above threshold X (to be defined, see 08): no "Verified" label.
- Zero of 3 pilots commit to an evidence report: re-evaluate the wedge.
- Branch causal claim fails intervention tests: relabel causality as correlation.

## Two parallel tracks

Track A (engineering, Claude Code): phases 0, 1 (code), 2, 3, 4, 5.
Track B (founder and team, no code): validation rig and data; 5 engineer interviews; patent and competitor search; choose V&V standard; pilot outreach; code review by a senior engineer.

Without Track B, the work produces "a nice demo", not a business.

## Time estimates (ESTIMATE, depends on rig and data)

Step 0: 1 to 2 days. Step 1: 2 to 3 weeks of code, plus 2 to 4 weeks for rig data. Step 2: 1 to 2 weeks. Step 3: 2 to 3 weeks. Step 4: 1 to 2 weeks. Wedge MVP overall about 3 to 4 months if nothing blocks.

## Cadence

Monday: define goal and gate. Mid-week: status board update and blockers. Friday: demo, re-measure completion (12), kill-criteria check.

## How a non-coder verifies the work

1. CI badge is green.
2. Test report per phase: count, pass, fail (raw output).
3. A single command runs the demo twice and gives the same result.
4. Status board with evidence and date.
5. After step 1, a senior engineer or professor reviews code and tests for 1 to 2 hours. This reduces the risk of AI-written tests that pass trivially.

## Scheduled later (locked layers not in the first gates)

Branch storage at scale; interoperability (FMI/FMU, STEP, SysML v2); collaboration and sign-off; tenancy and on-prem; metering; control loop; failure knowledge base.

## Golden demonstration (must be redefined once the domain is chosen)

Spec demo is motor and load in 14 steps: build, configure, execute, observe, trigger failure, explain causal chain, freeze, replay, branch, compare, optimize, compute, verify, generate evidence. Definition of done: the whole chain works; a visually impressive demo without the chain is not a HELQIX prototype. The battery version of this script is not written yet.
