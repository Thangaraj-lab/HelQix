# 29. User Workflow per Screen (one user, one scenario, 16 screens)

Purpose: the product story from the user's side, screen by screen, so design, UI build and engine gates all agree. Screens follow files 24 and 25. Engine dependency says which phase makes the screen real; until then it carries PREVIEW or MOCK.

## The user (example persona, HYPOTHESIS to test in Track B interviews)

Ananya, thermal and reliability engineer at a small EV-component company. Her team's drive unit overheated in a bench test. Her manager asks: "Why did it fail, will the fix work, and can you prove it to the customer?" She does not trust black-box tools; she needs reproducible, defensible evidence. Domain stays neutral here (motor or battery); the first domain pack is decision D1.

## Master flow (matches locked flow v3)

Intent and requirements -> Model -> Scenario -> Run -> Observe -> Failure -> Understand (causality) -> Branch (what-if) -> Compare -> Verify -> Evidence -> Export/Deploy. Back-edges: failed verification returns to Model or Scenario; poor validation returns to calibration.

## Screen-by-screen

### 01 Landing / Entry
- Goal: understand what HELQIX is, sign in.
- Ananya does: reads "Engineering truth, with proof.", clicks Get Started, logs in.
- Example prompt: none (auth only).
- System: shows login or signup; remembers last project.
- Next: 02. Engine: none. Real now (LandingPage, LoginPage exist; not run).

### 02 Home / Overview
- Goal: find her project and see trust status at a glance.
- Does: opens project "Drive Unit Thermal"; sees recent runs, requirements passing (for example 18 of 22), last evidence package date.
- Example prompt (search/command box): "open drive unit thermal failure from last Thursday".
- System: resolves to the project and the failed run; no invented data.
- Next: 03 or 08. Engine: project and run listing (API). PREVIEW parts: Trust Summary until Phase 1.

### 03 Engineering Workspace / Reality Canvas
- Goal: see the system as one authoritative reality with live values.
- Does: orbits the model, toggles layers (Structure, Physics, State, Flow, Environment), reads the run header (model revision, run id, determinism level).
- Example prompt: "show me only the thermal layer and highlight the stator".
- System: filters layers and highlights the component; values carry Origin x Status badges.
- Next: 04, 06, or 08. Engine: state and model read (Phase 0 fixes). Mode tabs: Reality, Time, Causality, Branch, Verification.

### 04 Model Editor
- Goal: inspect or change the model.
- Does: opens Component Properties for the rotor; checks material, mass, parameters; sees units.
- Example prompt: "increase cooling flow on the stator housing by 20 percent".
- System: this is a PROPOSAL, not an edit. Shows a structured diff and creates a new model revision only after Ananya confirms; nothing changes silently.
- Next: 10 or 05. Engine: model revisions (exists, library level). Free-text intent: deterministic extraction claimed; agent is Phase 7B, so until then she uses structured fields.

### 05 Simulation Setup
- Goal: choose scenario, solver, duration, precision, determinism.
- Does: picks scenario "Bench test, continuous load", selects Deterministic run configuration, sets time step and duration.
- Example prompt: "run the same bench load profile for 10 seconds with seed 8421".
- System: fills fields, displays the determinism level (Bitwise for the core runtime, Tolerance or Statistical for external solvers) and the exact solver and version.
- Next: 06. Engine: Phase 2 for determinism levels; earlier it is Claimed.

### 06 Simulation Run
- Goal: run and watch live.
- Does: starts the run, sees live metrics and run status; clicks Freeze at the moment temperature crosses the limit.
- Example prompt: "freeze when winding temperature exceeds the limit".
- System: a requirement-linked trigger creates a content-addressed freeze (hash shown). 
- Next: 07 or 08. Engine: Phase 2 (freeze). Before that, Freeze is PREVIEW.

### 07 Simulation Results
- Goal: read outcomes by run, metric, distribution.
- Does: compares Run 01 to Run 04, reads key metrics, state timeline (Init, Stable, Transition, Critical, Recovered).
- Example prompt: "which run had the highest winding temperature and when?"
- System: answers from stored data with units, run id and time; each number shows Origin: Simulated and Status.
- Next: 08. Engine: results and observations (observation layer partial).

### 08 Failure Investigation / Causality
- Goal: find why it failed.
- Does: opens the failure at 8.42 s; reads the Candidate chain (structural + temporal): Load up, Current up, Copper loss up, Temperature up, Thermal failure; clicks each node to see equation, previous value, value at failure, observation, snapshot link.
- Example prompt: "why did the rotor overheat at 8.42 seconds?"
- System: returns the ranked candidate chains with method and grade; never says "root cause" until an intervention test supports it. Button: Run Counterfactual Simulation.
- Next: 09 or 10. Engine: Phase 3 (candidate chains), Phase 4 (intervention support). PREVIEW until Phase 3.

### 09 Compare
- Goal: see what changed between original and fix.
- Does: selects Scenario A (original) and Scenario B (branch with more cooling); watches both, reads metric deltas and the first-divergence time.
- Example prompt: "compare branch B against the original and show where they first differ".
- System: aligned curves, delta table, requirement status per branch, determinism level of both.
- Next: 10 or 14. Engine: Phase 4. PREVIEW until then.

### 10 Scenario Builder
- Goal: define conditions and counterfactuals precisely.
- Does: builds scenario chain (Initial State, Environment, Inputs, Constraints, Events); uses presets (Normal, Extreme, Cold, Hot); forks from the freeze instead of from time zero.
- Example prompt: "fork from the freeze at 8.0 s and reduce load by 10 percent".
- System: creates a branch from the exact frozen state, records the intervention as a structured diff, parent remains unchanged.
- Next: 06 (run branch) or 09. Engine: Phase 4 (branching exists at library level, 11 tests).

### 11 Timeline / Replay
- Goal: reproduce and inspect the exact moment.
- Does: scrubs the timeline, replays from the freeze, reads the event details.
- Example prompt: "replay from the freeze and confirm it matches the original".
- System: replays and shows "Replay matches original: PASS" with state-hash comparison; if not identical, shows first divergence (time, variable) and the determinism level.
- Next: 08 or 14. Engine: Phase 2. PREVIEW until Gate 2.

### 12 Build / Optimization
- Goal: search for a better design within constraints.
- Does: sets goals (efficiency, weight, stability), picks an algorithm, starts optimization, reads proposals.
- Example prompt: "find the cooling flow that keeps winding temperature under the limit with minimum added weight".
- System: runs bounded sweeps via branches; results labeled Proposed change (Unverified) with a button "Validate by branch simulation". Search is bounded today (2 to 5 point sweep), so say so.
- Next: 09 then 14. Engine: 12-search (partial), Phase 4.

### 13 Compute / Resources
- Goal: see where work runs and what it costs.
- Does: checks CPU, GPU, edge, cloud, quantum (simulator) availability; reads routing reasons.
- Example prompt: "why did this run go to GPU?"
- System: shows the recorded routing decision and reason. Quantum shown as PREVIEW, simulator only.
- Next: back to 05/06. Engine: 20-platform primitives exist; routing is Phase 6/7C.

### 14 Verify / Validation
- Goal: decide whether the result can be trusted for this use.
- Does: reviews Context of Use, requirement outcomes, model error against the threshold, uncertainty, credibility level; sees any failing criteria.
- Example prompt: "is this model credible for deciding the cooling change?"
- System: answers by Context of Use and risk: states which criteria pass and which fail; marks "Verified" only if all Trust criteria pass, else lists the exact missing criteria. Never a blanket Verified.
- Next: 15, or back to 04/05 if validation fails. Engine: Phase 1 (validation) and Phase 5. PREVIEW until Gate 1.

### 15 Deploy / Export
- Goal: hand proof to manager and customer.
- Does: exports the Evidence Package (signed, manifest, root hash), also CSV/JSON of results; copies the verifier command for the customer.
- Example prompt: "export evidence for the cooling fix and show me how the customer can check it".
- System: builds the package, signs it (Phase 5), shows the signature status and the exact independent-verifier command; states what signing proves (origin and integrity, not physics correctness).
- Next: done, or 16 for edge deployment. Engine: Phase 5. PREVIEW until Gate 5.

### 16 Edge / Real-time Monitoring
- Goal: compare reality with the model on real hardware.
- Does: connects the sensor rig, watches live Observed data against the simulation, sees residuals and anomaly events; feeds data back to validation.
- Example prompt: "overlay the rig temperature on the simulated winding temperature and flag deviations above the tolerance".
- System: Observed values appear with Origin: Observed; they never overwrite authoritative state; deviations create events and validation metrics.
- Next: 14 (re-validate). Engine: Phase 7A (needs real rig data). PREVIEW until Gate 7A.

## Ananya's 10-minute story (the golden demo)

1 Open project (02) -> 2 view failed run (03) -> 3 open failure (08) and read the candidate chain -> 4 replay from the freeze and see PASS (11) -> 5 fork with 20 percent more cooling (10) -> 6 run branch (06) -> 7 compare (09) -> 8 check credibility (14) -> 9 export signed evidence (15) -> 10 customer runs the verifier and gets PASS. This is the demo that proves the product; no screen outside this path should be polished before it works end to end on real data.

## Rules for every screen

1. Prompts shown are examples of how a user may ask; deterministic intent extraction exists (claimed, not run) and the AI agent arrives in Phase 7B. Until then these map to forms and buttons; do not fake an AI answer.
2. Anything the user asks that changes the model is a proposal first and needs confirmation.
3. Every answer shows units, run id, time and Origin x Status.
4. If the data does not exist, say so (empty or error state); never invent.
5. Mark PREVIEW or MOCK on anything whose engine gate is not passed.
