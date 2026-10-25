# 15. Phase 3 Prompt: Causal Graph and Failure Investigation

Prerequisites: Gate 2 passed. Paste 13 first. Branch: `phase-3-causality`.

## Goal

From a failure, an engineer can click through a causal chain to the root cause, and at each node see the equation, parameter, previous and current value, observation, evidence and linked state. Causal claims are honestly graded: until intervention tests exist (Phase 4), a chain is a "candidate cause (structural and temporal)", not a proven cause.

## Background to verify

Per the audit the layers are thin: 09-failure (about 318 lines, 2 tests), 10-events-causality (events 2 tests, causal 1 test, about 335 lines), 08-observation (14 tests). Read them fully before designing. Extend the existing types; do not create parallel ones.

## Tasks

1. Failure definition: failures come from requirement violations or physical limit crossings, defined in the model or requirements (not hard-coded in core). A failure event records requirement id, quantity, threshold, observed value, logical time, and the frozen state reference (from Phase 2).
2. Event log: signals, anomalies, state changes and failures as events with logical time and state references. Events are append-only and ordered.
3. Structural dependency graph: derived from the model (which quantities depend on which parameters and equations). The model must expose equation or relation identifiers; if not, add them in the domain pack and note that.
4. Candidate causal chain: combine structural dependency (a quantity can influence another) with temporal precedence (change occurred earlier) and magnitude (change significant relative to a stated threshold). Output a ranked list of chains with a score and the method. Label each: `candidate (structural + temporal)`.
5. Node detail contract: for each node return equation or relation id, parameters involved with values, previous value, value at failure, the observation or signal, the state snapshot reference, related events, and links to branch experiments if they exist.
6. Click-through API and, if the UI exists, a minimal Causality-mode view. The UI shows the grade label and never says "root cause" for an unproven candidate; use "candidate root cause".
7. Confounding and false cause: handle shared drivers (two effects of one cause) and coincidence; the algorithm must not report one effect as the cause of another effect when a common driver explains both, within the limits of structural information. Document limitations.

## Required tests (synthetic ground truth first)

- Known chain A -> B -> C: the algorithm recovers it and ranks it first.
- Common driver: A drives B and C; B must not be reported as the cause of C.
- No-failure run produces no failure chain.
- Multiple simultaneous anomalies: chains are separated and each linked to its own failure.
- Click-through returns all required fields for every node on a reference chain.
- Replay-linked: the state reference opens the exact frozen state (uses Phase 2).
- Reference domain scenario: the expected chain (for example load, current, loss, temperature, limit) is found. Use the chosen domain pack; do not hard-code the chain.
- Mutation sanity check as in the preamble.

## Gate 3

Raw outputs show synthetic ground-truth tests pass, a reference scenario click-through works end to end from failure to candidate root cause with all node fields, labels say "candidate" for unproven chains, STATUS updated. Limitations documented.

## Out of scope

Intervention proof (Phase 4), signed evidence, canvas polish, adapters.
