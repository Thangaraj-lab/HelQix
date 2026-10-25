# 08. Open Decisions, Hypotheses, Decision Log

## Open decisions (founder must decide; nobody else)

| # | Decision | Status | Needed by |
|---|---|---|---|
| D1 | First domain pack: battery cell, motor thermal, or another | DECIDED 2026-10-25: battery cell (LG M50, PyBaMM SPM/Chen2020); see docs/PHASE1_INPUTS.md | Done |
| D2 | Context of Use text and validation error threshold X | DECIDED 2026-10-25: terminal-voltage RMSE < 46 mV on held-out C/10, per cell; Context of Use and risk Low in docs/PHASE1_INPUTS.md | Done |
| D3 | V&V standard family for the first vertical (ASME V&V 10/20, V&V 40, NASA-STD-7009) | OPEN | Before pilot reports |
| D4 | Validation rig design, data plan, held-out split protocol | DECIDED 2026-10-25 for Phase 1: public data (Zenodo 4032561), calibrate on C/2-1.5C, hold out C/10. Own rig deferred to Phase 7A | Rig: Phase 7A |
| D5 | Authoritative runtime owner (Rust crates vs Python gateway vs TypeScript app) | DECIDED 2026-10-25: Rust core is authoritative; Python gateway calls it | Done |
| D6 | Naming cleanup HELIX to HELQIX in code | OPEN; naming report in docs/STATUS.md | After Phase 0 report |
| D7 | Phase 0 "hygiene" step inserted before locked step 1 | APPROVED 2026-10-25; Gate 0 PASSED 2026-10-25 | Done |
| D8 | Which external adapter first (PyBaMM, CalculiX, OpenFOAM) | Deferred to step 6 | Later |
| D9 | Requirements language format | OPEN | Phase 1 |
| D10 | Pricing, buyer, unit economics | After pilots | Later |
| D11 | Funding and monthly budget | OPEN | Before buying hardware |
| D12 | Who reviews code (senior engineer or professor) | OPEN | After Phase 1 |

## Unproven hypotheses

| # | Hypothesis | How to test |
|---|---|---|
| H1 | Trust and verification become the scarce resource in the AI era | 5 engineer interviews |
| H2 | The integrated freeze, replay, branch, causal and signed-evidence runtime is novel and wanted | Patent search, competitor feature check, pilots |
| H3 | Battery SPM is enough for thermal-failure root cause | UNKNOWN. Check which PyBaMM models and options include thermal behavior; run a case. |
| H4 | A cheap rig can validate a single-cell model | Rig BOM and one measured discharge compared to the model |
| H5 | Customers will pay for an evidence report | 3 pilots |
| H6 | Public battery datasets can substitute for own data | Not verified; search and check cell chemistry match |

## Choosing the first domain: criteria

Score 1 to 5 per criterion. These scores are my JUDGEMENT, not evidence; the founder should re-score after Phase 0 and interviews.

| Criterion | Battery cell | Motor thermal |
|---|---|---|
| 1. Real solver or code already exists | 4 (PyBaMM slice in repo, run unverified) | 1 (no motor code; but the ODEs are simple to write) |
| 2. Cheap to validate | 3 (cell and sensor rig within INA219 range; fire risk and cell cost) | 4 (small motor, current and temperature sensing) |
| 3. Failure and causal chain clear | 3 (depends on thermal model, H3) | 5 (load, current, copper loss, temperature, limit) |
| 4. Buyers exist | 4 (EV and battery R&D; ASSUMPTION) | 3 (ASSUMPTION) |
| 5. Safe to test | 2 | 5 |
| Total (equal weights) | 16 | 18 |

Sensitivity: with equal weights motor is slightly ahead; if reuse of existing code is weighted at least double, battery leads. The result is close, so a decision should be made on strategy (reuse of the repo vs speed to a clean demo), not on this table alone. Do not treat 16 vs 18 as a finding.

Other domains (pump, drone, fan, structural) were not scored.

## Validation rig (draft requirements, not a design)

- Purpose: produce measured data to validate one model, not to claim digital-twin accuracy.
- Cell rig (if battery): INA219-class sensing is limited to about 26 V and about 3.2 A with the stock shunt (VERIFIED, 11). Temperature sensing needed. Safety below is mandatory.
- Motor rig (if motor): small DC motor, controllable load, current and voltage sensing, temperature sensing.
- Data protocol: record raw data with timestamps; split into calibration set and held-out validation set before fitting; never tune on the validation set; store everything with hashes.
- Bill of materials and total cost: UNKNOWN (not yet made).

## Safety (mandatory before buying any lithium cell)

Protection and cut-off circuit, fuse, current and temperature limits written down, supervised tests, fire-safe test area, no unattended charging, proper storage. Prefer a lab or workshop with supervision.

## IP, legal, naming checks (not yet done)

- Trademark and domain check for HELQIX; resolve HELIX vs HELQIX in code and brand.
- Repo license review: crates declare MIT; the LICENSE file content and third-party licenses (PyBaMM, Blender) not checked.
- Patent search around: simulation state snapshot and branching, deterministic replay, provenance and evidence signing for engineering simulation.
- Compliance claims (ISO 26262, DO-178C tool qualification): UNVERIFIED. Do not market until reviewed.

## Decision log

| Date | Decision or finding | Evidence | By |
|---|---|---|---|
| 2026-10-25 | Definition v1.0 locked (files 01, 02, 06) | Discussion and adversarial review | Founder |
| 2026-10-25 | Repo audit completed; spec contradicted by code on motor physics, evidence compile, tests | 04 | Claude |
| 2026-10-25 | Proposal: insert Phase 0 hygiene | 04, 05 | Claude, pending approval |
| 2026-10-25 | Proposal: battery first as wedge | 04; see scoring above, close | Claude, pending approval |
| 2026-10-25 | D7 approved: Phase 0 hygiene runs before locked step 1 | chat | Founder |
| 2026-10-25 | Correct 7 wrong test expectations (helqix-execution 6, helqix-search 1) | commits 9eb8c9c, 6e08ba1; docs/STATUS.md | Founder approved |
| 2026-10-25 | Allow '/' in listed unit symbols; register m/s, m/s2, kg/m3 in helqix-core | commit 6c8b538 | Founder approved |
| 2026-10-25 | Push phase-0-hygiene and use Linux CI as the authoritative test run | CI run 40180917041 | Founder approved |
| 2026-10-25 | Runtime trace: Rust crates have no runtime caller; app uses the Python gateway; freeze/replay not wired. D5 still open | docs/RUNTIME_TRACE.md | Claude |
| 2026-10-25 | Gate 0 PASSED (929 Rust tests pass, 0 fail; 52 Python pass; CI green). Phase 1 not started: needs D1, D2, D4 | docs/STATUS.md "Gate 0 evidence" | Founder |
| 2026-10-25 | D1 battery cell; D4 public data first; D5 Rust core authoritative; D2 quantity terminal voltage | chat | Founder |
| 2026-10-25 | Threshold 46 mV RMSE (anchored to Chen et al. 2020 Table VIII), Context of Use, risk Low, dataset and C/10 hold-out split | docs/PHASE1_INPUTS.md | Claude, delegated by founder |
| 2026-10-25 | Finding: Chen2020 parameters were tuned on the C/2, 1C, 1.5C discharges of the same dataset, so only C/10 is held out | Chen et al. 2020, validation section and Table IX | Claude |
| 2026-10-25 | Gate 1 result: model NotVerified (held-out C/10 RMSE 71.1-77.3 mV vs < 46 mV); kill trigger applied, no tuning on held-out data | docs/validation/GATE1_VALIDATION_REPORT.md | helqix-validate |
| 2026-10-25 | Gate 1 PASSED with the model NotVerified (exit criterion: published error bound against measured data). C/10 data is spent as held-out evidence; look for a second LG M50 dataset for a fair re-test. Phase 2 not started | docs/STATUS.md; CI run 40185418438 | Founder |
| (next) | | | |
