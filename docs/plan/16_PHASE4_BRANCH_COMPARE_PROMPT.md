# 16. Phase 4 Prompt: Counterfactual Branches, Intervention Tests, Compare

Prerequisites: Gate 3 passed. Paste 13 first. Branch: `phase-4-branches`.

## Goal

An engineer can fork reality from a frozen failure state, change one thing, run it, and compare against the original. A branch is an experiment: it can upgrade a causal claim from "candidate" to "intervention-supported" only when the intervention actually removes or changes the failure as predicted.

## Background to verify

helqix-branch has 11 passing tests (fork, fork_at, isolation). Compare is only README-claimed (battery curves). Read 11-branching and the app's compare code before designing. Extend existing types.

## Tasks

1. Fork from a freeze (Phase 2) with a single declared intervention (parameter, component, scenario, or environment change). Intervention is recorded as a structured diff, not free text. Branches inherit parent lineage (parent id, fork time, parent state hash).
2. Isolation: a branch can never mutate the parent or siblings. Add a test that tries to and fails.
3. Run the branch with the same determinism machinery (level declared, state-hash chain).
4. Compare: aligned time series and scalar metrics, parent vs branch (and branch vs branch), with deltas, first-divergence time, and requirement status per branch (violated / satisfied / margin). Output machine-readable and renderable.
5. Intervention test for causality: given a candidate chain from Phase 3, the system proposes interventions on the candidate cause node; running them yields a verdict: `intervention-supported` (failure removed or shifted as predicted, beyond a stated tolerance), `not-supported` (no effect), or `inconclusive` (needs more runs). Label stays "model-causal" because it is shown inside the model; "validated-causal" requires Phase 1 validation evidence plus real-world data and is NOT granted here.
6. Dose-response check where applicable: vary the intervention across 3 or more levels and check monotonic effect; report if non-monotonic.
7. Branch cost control: cap on number of branches and total runtime per request; reject beyond limit with a clear message. Search (12-search) uses the same branch machinery; do not duplicate.
8. Merge is NOT in scope. Branches are experiments; adopting a change into the main model is a human-approved action recorded as a new model revision (design only, no auto-merge).

## Required tests

- Fork-and-diff: parent unchanged after branch run (hash equality before/after).
- Same intervention twice gives identical branch results (determinism).
- Known synthetic system where the true cause is known: intervening on it removes the failure; intervening on a non-cause does not. Verdicts must be correct.
- Common-driver synthetic system: intervening on an effect node must not be credited as removing the cause.
- Compare handles different lengths, different sample times, and missing variables without crashing or silently misaligning (explicit alignment rule documented).
- Branch limit enforcement.
- Mutation sanity check as in the preamble.

## Gate 4

Raw output shows intervention tests give correct verdicts on synthetic ground truth, a reference-domain demonstration (fork from failure, apply one change, show failure removed or not, Compare output), isolation proven, STATUS updated. Kill trigger: if a branch causal claim fails an intervention, relabel as correlation and report.

## Out of scope

Signed evidence, new canvas modes, adapters, auto-merge.
