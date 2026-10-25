# Failure investigation and candidate causes (Phase 3)

Date 2026-10-25. Crates: `helqix-causal` (candidate chains), `helqix-events` (`EventLog`), `helqix-investigation` (failure event, click-through, `helqix-investigate` CLI). Battery reference scenario: `18-libraries/python/helqix_battery/causal.py`.

## What it does (VERIFIED by tests)

1. **Failure** = first sample violating a requirement (node, comparator, threshold). Never hard-coded.
2. **Event log**: onset events for every node on a candidate chain, then the failure event; append-only, time-ordered, no duplicate ids.
3. **Structure**: only declared relations (equation or relation id, with parameters) count. The causal graph is acyclic by construction; mutual coupling cannot be represented (see limitations).
4. **Candidate chains**: paths of declared relations ending at the failed node, kept only if every node changed by at least `min_relative_change` of its scale and onsets (first reaching `onset_fraction` of the node's largest change) never go backwards along the chain. Ranked by root onset, then length. No probabilities.
5. **Grade**: every chain and node is `candidate (structural + temporal)`. The UI word must be "candidate root cause", never "root cause". Only an intervention (Phase 4) can upgrade a candidate, and only to model-causal.
6. **Click-through** (`NodeDetail`): relation id, relation parameters with values, baseline value, previous value, value at failure, signal, frozen-state reference (Phase 2 freeze hash + grid index), related events, branch experiments (empty until Phase 4).

Tests: 7 synthetic ground-truth tests in `helqix-causal` (known chain ranked first, common driver, late mediator, unchanged upstream, no failure, simultaneous failures, post-failure changes), 2 in `helqix-events`, 8 in `helqix-investigation` (including a replay-linked reference on the core runtime whose state reference opens the exact saved freeze, and JSON round trip), 3 in the battery pack.

## Battery reference scenario (CALCULATED, simulation only)

PyBaMM SPM + Chen2020, constant 5 A (1C) from 4.18 V, 10 s samples, requirement "voltage at least 2.5 V" (test-protocol cut-off). Eight signals; thirteen declared SPM relations (every input of each equation, not only the expected one).

Result from `helqix-investigate`:
- Failure: voltage 2.4787 V at t = 3570 s (index 357).
- Candidate chains (ranked): `neg_reaction_overpotential -> voltage` (onset 20 s), `pos_open_circuit_potential -> voltage` (onset 50 s), `pos_reaction_overpotential -> voltage` (onset 120 s).

## Limitations found (documented, not hidden)

1. **Nonlinear links break the onset rule.** The physically expected chain `pos_surface_stoichiometry -> pos_open_circuit_potential -> voltage` is not found: the open-circuit potential reaches 10% of its change before the stoichiometry does, because the potential curve is strongly nonlinear, so the link is rejected as "effect before cause".
2. **Integrated quantities break the onset rule.** A body launched upward: height changes at once because velocity is already non-zero, while velocity only starts changing later; no chain is found (test `known_limitation_launched_body_yields_no_candidate_chain`).
3. **Ranking by earliest onset favours fast transients.** In the battery case a start-up overpotential transient ranks first, though it is not what drives the cell to cut-off.
4. **Acyclic structure only.** Two-way coupling (for example velocity components through quadratic drag) must be declared one way.
5. **Common drivers** are handled only as far as the declared structure allows; a wrongly declared link can produce a wrong candidate.

Consequence: Phase 3 produces honest candidates with complete click-through, not reliable root-cause ranking. Phase 4 intervention tests are where cause evidence must come from. A better temporal rule (for example deviation from a baseline run, or onset on a normalised monotone transform) is a design decision for the founder, not done here.

## Reproduce

```
cd 18-libraries/python
python -c "import json,sys; sys.path.insert(0,'.'); from helqix_battery import causal, model; json.dump(causal.reference_input(5.0, 3600, 10, 4.18, model.ModelConfig()), open('../../.data/battery_investigation.json','w'))"
cd ../..
cargo run -p helqix-investigation --bin helqix-investigate -- .data/battery_investigation.json
```
