# 08. Open Decisions, Hypotheses, Decision Log

## Open decisions (founder must decide; nobody else)

| # | Decision | Status | Needed by |
|---|---|---|---|
| D1 | First domain pack: battery cell, motor thermal, or another | OPEN | Before Phase 1 |
| D2 | Context of Use text and validation error threshold X | OPEN | Before Phase 1 |
| D3 | V&V standard family for the first vertical (ASME V&V 10/20, V&V 40, NASA-STD-7009) | OPEN | Before pilot reports |
| D4 | Validation rig design, data plan, held-out split protocol | OPEN | Before Phase 1 exit |
| D5 | Authoritative runtime owner (Rust crates vs Python gateway) | Needs trace (Phase 0) | After Phase 0 |
| D6 | Naming cleanup HELIX to HELQIX in code | OPEN | After Phase 0 report |
| D7 | Phase 0 "hygiene" step inserted before locked step 1 | Recommended, not yet approved | Now |
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
| (next) | | | |
