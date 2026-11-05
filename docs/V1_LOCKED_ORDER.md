# HELQIX v1: locked order of work

Locked 2026-11-05 on branch `ui-studio-v2` at the founder's instruction ("v1 full work correct plan, lock it, work in order"). This file does not replace [V1_REMAINING_WORK_REPORT.md](V1_REMAINING_WORK_REPORT.md): it fixes the **order** and the **v1 definition** from that report (section 12, now approved by the instruction above). Work IDs (E, F, R) are the report's. No second gate numbering.

Rules (unchanged): one item at a time; test first; mutation check where a test guards logic; labels VERIFIED / CALCULATED / ASSUMPTION / HYPOTHESIS / UNKNOWN; Linux CI is the authority for Rust API tests; nothing is pushed or merged without the founder.

## v1 definition (locked)

Must have: one domain (battery cell) from description to signed evidence. Describe or build a cell model in the Studio, run it through a real solver, freeze and replay it, branch with an intervention test, see the causal chain, export a signed package that an independent script verifies; plus accounts, packaging, docs, and an honest model error bound.

Should have: measured-data ingest from the founder's rig (7A), streamed rendering decision (K7), uncertainty intervals.

Not in v1: quantum (K9), Build Spec (gate 8), rigid-body engines, FEniCSx/MOOSE, interoperability beyond FMI, collaboration features. Changing this list needs a dated founder decision.

## Order of engineering work

| Step | Item | Why here | Done when |
|---|---|---|---|
| 1 | E1 Browser walkthrough of the real-engine Studio screens | Nobody has clicked the newest screens; findings change later steps | Written checklist with screenshots, zero blocking defects |
| 2 | E4 Model schema with physical roles; translator to a solver adapter (FMU first) | Longest chain; unlocks E5, E7, E18, E25 | A Studio model runs through FMPy and the frames store, tied to its revision |
| 3 | E5 Battery domain pack (PyBaMM behind a Studio model) | The v1 domain | A cell defined in the Studio reproduces the reference run |
| 4 | E10 finish: Studio sign-in flow, key refresh, a real Keycloak run | Every later feature assumes a user | Two users cannot see each other's data, end to end |
| 5 | E8 Evidence export from the UI (server builds and signs) | Core v1 promise | Downloaded package passes the offline verifier (test key until F4) |
| 6 | E17 Model accuracy (needs F5 decision) | The honest-claim question | Held-out RMSE under the agreed threshold, or an approved wider bound with "NotVerified" |
| 7 | E13 finish: Studio image, compose file for the whole stack, release build | Needed to deploy | One command starts the stack; install time measured |
| 8 | E25 Sample project and golden demo from the docs | Users need a path | A new person finishes the demo from QUICKSTART alone |
| 9 | P1 batch: E7, E16, E18, E11, E12, E14, E15, E20 | Credible v1 | Each item's own acceptance line in the report |
| 10 | P2 only if time: E21, E22 (E23, E24 are out of v1) | Cleanup | n/a |
| 11 | R1 to R3 Release: full CI, re-score STATUS.md, founder merge decision | Last | Founder's call; the assistant never merges |

E19 (K7 production design) waits for F2. E3 (interpretation v2) is P1 and moves into step 9 if E4 finishes early.

## E4 sub-steps (locked 2026-11-05; each one finishes with tests, a built-in review and an update)

Design: a part gets a **behaviour**. The first kind is an FMU (FMI 2.0 co-simulation), named by the SHA-256 of its file (never a path), with unit-carrying parameters and the output variables to record. The model stays the single source of truth; the solver adapter that already exists (FMPy through `helqix_fmi`) runs it.

| Step | Work | Done when |
|---|---|---|
| E4.1 | Schema: `part.setBehavior` op in the contracts (Rust, JSON schema, fixtures, TS), rules in the model build (hash-named FMU, named unique outputs, registered units), TS fold | DONE 2026-11-05: contracts 5, modelbuild 13, API 5 (PostgreSQL), UI 64 tests pass; 4 mutations caught |
| E4.2 | Translator in `helqix-modelbuild`: a committed model becomes an FMU run plan (fmu hash, parameters in SI, outputs, times), or the parts and reasons that block it | Pure-function tests; a model with no behaviour keeps using the lumped body |
| E4.3 | FMU store (content-addressed, size cap, licence note) and `POST .../revisions/{n}/run` runs the plan through the FMI adapter, stores frames, links the run to the revision | API test with the Reference FMUs against PostgreSQL in CI; replay hash matches |
| E4.4 | Studio: attach an FMU to a part (upload, parameters, outputs), see the run, Run my model uses it | Browser walkthrough with screenshots; UI tests |
| E4.5 | Acceptance: a Studio-built model runs through FMPy, frames in the store, tied to its revision, in CI | One CI job, green |

Unit note found in E4.1: the unit registry spells dimensionless as `one`; FMUs and the FMI adapter use `1`. E4.2 maps between them and tests it.

## Founder items, in parallel (the assistant cannot do these)

| ID | Item | Needed by step |
|---|---|---|
| F1 | Rig data for 7A (longest waiting time: start now) | before release; gates the "measured" claim |
| F2 | GPU streaming run | E19 |
| F4 | Create and keep the cosign key, publish the public key | step 5 (final), test key before |
| F5 | Decisions D3, D6, D9, D11, D12, CalculiX/GPL stance | step 6 |
| F3 | Review and sign off gates 2 to 7B in STATUS.md | before R2 |
| F6, F7 | Trademark, patent search, interviews, pilots; confirm MIT holder name | P1 |

## Change rule

If a step fails twice, the assistant stops, writes what failed and what was measured, and asks. Order changes only by a dated line added here.
