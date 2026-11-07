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
| 4 | E10 finish: Studio sign-in flow, key refresh, a real Keycloak run | Every later feature assumes a user | LOCAL 2026-11-06: E10.1 JWKS URL refresh (6 tests, 6 mutations), E10.2 Studio sign-in with PKCE (16 tests, 7 mutations), E10.3 real Keycloak 26.8.2 in WSL: two real users cannot see each other's projects, builds or revisions, a token for another audience or with a damaged signature is refused (2 tests; 2 of 3 mutations caught, the unchecked-issuer one is covered by a unit test only). CI job `keycloak-accounts` added, **not run on GitHub yet** BROWSER WALKTHROUGH 2026-11-06 (real Keycloak, Studio, dev API): alice signed in, made a project and a run and listed exactly her run; after sign-out bob signed in and listed 0 projects and 0 runs. Run ids stay capability links (documented in AUTH.md). Review of the E10 diff: 4 findings fixed (rotation race, limiter, transient refresh failure, gate follows a lost session), 4 skipped as non-blocking (list efficiency, non-atomic owner insert, pre-migration runs, periodic refresh result). **E10 ACCEPTED by CI 2026-11-07 (run 42355480082, 16 of 16 jobs green, including 'Two real users on a real Keycloak (E10)').** |
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
| E4.1 | Schema: `part.setBehavior` op in the contracts (Rust, JSON schema, fixtures, TS), rules in the model build (hash-named FMU, named unique outputs, registered units), TS fold | DONE 2026-11-05: contracts 5, modelbuild 6+15, API 5 (PostgreSQL), UI 64 tests pass; 4 mutations caught; review fixes: an FMU model is refused by the lumped run until E4.3 |
| E4.2 | Translator in `helqix-modelbuild`: a committed model becomes an FMU run plan (fmu hash, parameters in SI, outputs, times), or the parts and reasons that block it | DONE 2026-11-05: `helqix_modelbuild::fmu::fmu_plan` and `FmuPlan::adapter_input`; 8 tests; 6 mutations caught; no behaviour gives `Ok(None)` so the lumped body is still used. The runner (E4.3) must check each FMU variable's declared unit against the plan's dimension, because FMPy takes parameters in the FMU's own units |
| E4.3 | FMU library and `POST .../revisions/{n}/run` runs the plan through the FMI adapter, stores frames, links the run to the revision | DONE 2026-11-05 locally (WSL, PostgreSQL, FMPy 0.3.32): FMI adapter 11, python unit 7, API 12+5 FMU tests, modelbuild 14+15; 12 mutations caught. **Design decision: no FMU upload.** An FMU is native code, so the library is a directory the operator fills (`<sha256>.fmu`), re-hashed on every use. E4.4 therefore lets the user choose from the library, not upload. The CI job is E4.5 |
| E4.4 | Studio: attach an FMU to a part (choose from the library, parameters, outputs), see the run, Run my model uses it | DONE 2026-11-05: `GET /v1/fmus/{sha}` (the FMU describes itself), Behaviour panel in Define, FMU run form in Simulate; UI 87 tests, FMI 13, FMU API 6; walkthrough in [E4_WALKTHROUGH.md](E4_WALKTHROUGH.md) |
| E4.5 | Acceptance: a Studio-built model runs through FMPy, frames in the store, tied to its revision, in CI | Job `fmu-model-run` added to `.github/workflows/ci.yml` 2026-11-05: Python unit checks with Pint, then the API test through PostgreSQL, FMPy and the Reference FMUs, which **fails if any test was skipped**; its commands were dry-run locally in WSL, with and without `HELQIX_PYTHON` (the guard fails the step without it). **ACCEPTED 2026-11-05: GitHub Actions run [42042975508](https://github.com/Thangaraj-lab/HelQix/actions/runs/42042975508) on `ef467fc9`, 14 of 14 jobs green, including `fmu-model-run` with its no-skip step.** Scope of the proof: one Reference FMU (BouncingBall); other FMUs are untested |

Unit note found in E4.1: the unit registry spells dimensionless as `one`; FMUs and the FMI adapter use `1`. E4.2 maps between them and tests it.

## E5 sub-steps (locked 2026-11-05)

Design: a second kind of part behaviour, the **battery cell** of the domain pack (`18-libraries/python/helqix_battery`, PyBaMM SPM with the Chen2020 parameters, library entry `18-libraries/battery/lgm50_spm_chen2020.library-entry.json`). The user states the cell's open-circuit voltage, its contact resistance and a constant discharge current, with units; the engine runs the pack through the existing PyBaMM adapter and stores terminal voltage, discharge capacity and current as frames, tied to the revision. The pack's own label travels with every run: **NotVerified**, with the published held-out error (71.1 to 77.3 mV against the 46 mV target). The Studio must never show it as verified.

| Step | Work | Done when |
|---|---|---|
| E5.1 | `battery_cell` behaviour in the schema (Rust, JSON schema, fixtures, TS), rules in the model build (known pack, voltage / resistance / current dimensions, ranges), one behaviour per model across kinds, translator to a battery run plan | DONE 2026-11-05: contracts 5 (+ Python schema 11), modelbuild 27+19, UI 88; 7 mutations caught; `run_plan` covers point mass, FMU, battery cell and refuses two behaviours. **Rule change to confirm: plan rule 2 (no domain words in product code) now excludes `src/domainpacks`, the one place a domain pack's words may live; every other product file is still scanned.** Found by the tests: `fmu_plan` panicked when the one behaviour was not an FMU |
| E5.2 | API: run the plan through the PyBaMM adapter, frames and the pack's label and published error in the answer; config from the environment, all or none | DONE 2026-11-05 locally (WSL, PostgreSQL, PyBaMM 26.10.1.1 on Python 3.12 via uv; installed size 648 MB, of which casadi 262 MB): 5 API tests pass, the Studio-built model equals a direct run of the pack (1e-9), charge conservation 1e-6, voltage falls inside 2.5 to 4.25 V; a run is never served without the pack's label (missing entry, label or published error gives 503); 5 mutations caught. The calibrated resistance is offered by `GET /v1/packs`, never applied for the user |
| E5.3 | Studio: Battery cell panel in Define, run form and the NotVerified banner in Simulate, published error shown in Verify | DONE 2026-11-05: Battery cell panel in Define (the label first; the calibrated resistance is offered, not applied), run form in Simulate with the NotVerified notice above the button, the notice in Verify; the pack list route is domain-neutral (`GET /v1/packs`); UI 99 tests; walkthrough with the real API and PyBaMM: a 5 A discharge over 600 s gave 21 frames, discharge capacity 0 to 0.83 A.h (5 A x 600 s / 3600 = 0.833), Verify replay 21 of 21 |
| E5.4 | CI: extend the battery job with the API test (no skip) | Step added to the `battery-pybamm` job (PostgreSQL service, no-skip guard; dry-run locally: passes with PyBaMM, fails without). **ACCEPTED 2026-11-06: GitHub Actions run [42176030611](https://github.com/Thangaraj-lab/HelQix/actions/runs/42176030611) on `033d9266`, 14 of 14 jobs green, including the battery API step with its no-skip guard.** Scope: one pack (LG M50, SPM Chen2020), constant current; the pack stays NotVerified |

## E8 sub-steps (locked 2026-11-07)

Today the Evidence drawer only hashes what the browser holds, shows "Not signed", and has a disabled Export button. The signing builder (`helqix-evidence-pack`) and the independent offline verifier (`helqix-evidence-verify`) already exist and are CI-checked on sample packages (K5). E8 connects a stored run to them, so a user can download a signed package and anyone can verify it offline.

| Step | What | Done when |
|---|---|---|
| E8.1 | Package builder for a stored built-in-engine run (Bitwise): re-run from the stored parameters, refuse if the stored chain differs, freeze at the first failure of a declared requirement or at the last sample when it holds; files: freeze, state chain, requirement outcome, verdict (label NotValidated, no causal claim), the run's frames. The verifier learns a requirement that holds (failure null, freeze at the last sample) without weakening the failing case | Round trip: built package passes the offline verifier for a failing and a holding requirement; mutations: tampered file, wrong chain, requirement flipped **DONE 2026-11-07 locally (WSL):** `run_files` in `helqix-evidence-pack`, verifier accepts a holding requirement; 7 new tests (failing and holding packages pass the offline verifier, a stored run a re-run does not reproduce gets no evidence, changed frames, a signer lying in the outcome or in the freeze position are refused); 4 mutations caught (one survived until a test isolated the outcome check); both K5 sample packages still PASS the changed verifier. Note: the statement's `failure_index` is now optional (null = holds); existing signed fixtures are unchanged |
| E8.2 | API: `POST /v1/runs/{id}/evidence` (declared requirement in the body) returns a zip of the package, its in-toto attestation and the public key; signing key from the environment (test key until F4, key id and public key shown, never the private key); `GET /v1/evidence/key` | API test: unzip, offline verifier PASS; a user cannot export a run for a project they do not own; no key configured gives 503 **DONE 2026-11-07 locally (WSL, PostgreSQL):** `POST /v1/runs/{id}/evidence` and `GET /v1/evidence/key`, own stored-method ZIP writer (no new archive dependency), key from `HELQIX_EVIDENCE_KEY_FILE` (see `docs/EVIDENCE_EXPORT.md`). 4 API tests + 5 unit tests; the real `helqix-evidence-verify` binary with no network says PASS on the downloaded package and FAIL after one changed byte, and Python's `zipfile` accepts the archive; 4 mutations caught (user may export any run, flipped comparator, wrong trusted key in the archive, export without a key). Honest limits: built-in-engine runs only (FMU and battery are E8.5), test key until F4 |
| E8.3 | Studio: Export evidence button downloads the zip, the drawer shows the signature (key id, root hash, test-key notice) and the verifier command | UI tests; walkthrough: download, unzip, verifier PASS |
| E8.4 | CI: build through the API, unzip, run the verifier with no network, tamper must fail | New CI step green |
| E8.5 | (after E8.4) Tolerance packages for FMU and battery runs (invocation record and tool output roles) | Own plan before starting |

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
