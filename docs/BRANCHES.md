# Counterfactual branches and intervention tests (Phase 4)

Date 2026-10-25. Module `helqix_branch::intervention` (built on the Phase 2 freeze), demo `helqix-branch-demo`.

- **Fork** from a freeze with exactly one structured intervention: a model parameter (mass, drag coefficient, reference area, air density) or one state value. The branch records its lineage: parent freeze SHA-256, fork index and time. Invalid values (non-finite, negative, zero mass, unknown state variable) are rejected.
- **Isolation**: the parent run is only read; a test shows the parent hash chain is unchanged after branching.
- **Determinism**: branches run on the same core runtime (Bitwise); the same intervention twice gives identical hash chains.
- **Compare**: samples aligned only at identical (bit-equal) times, no interpolation; unmatched samples and missing variables are counted; per-variable maximum difference, first divergence time, requirement status per run (violated at t, or satisfied with margin).
- **Intervention test**: supported when the failure is removed or delayed by more than the stated tolerance, not supported when unchanged within it, inconclusive when there is no parent failure or the failure moved earlier. Supported is graded **model-causal** only; **validated-causal is never granted here** (needs Phase 1 validation plus real data).
- **Dose-response**: at least 3 levels; reports whether the failure time moves monotonically with the level order.
- **Budget**: maximum branches and total integration steps per request; exceeding either is rejected with a message.
- **No merge**: adopting a change would be a human-approved new model revision (design only).

## Demonstration (VERIFIED, `cargo run -p helqix-branch --bin helqix-branch-demo`)

Body dropped from 20 m (drag), frozen at t = 0.5 s; parent impact 2.061 s.

| Intervention at the freeze | Branch impact | Verdict |
|---|---|---|
| vertical velocity set to 0 (Phase 3 candidate `velocity_y -> height`) | 2.495 s | InterventionSupported, model-causal |
| z position moved 5 m (not a cause) | 2.061 s, height difference 0 | NotSupported |
| drag coefficient 0.47 -> 1.2 | 2.122 s | InterventionSupported, model-causal |

Dose-response, drag 0.6 / 1.0 / 1.4: impact 2.072 / 2.105 / 2.138 s, monotonic.

## Limitations

- The intervention module works on the core runtime's point-mass runs. Branching the battery pack (external PyBaMM solver, Tolerance level) needs the Phase 6 adapter contract.
- `helqix-branch` now holds two branch notions: the earlier component-state `BranchTree` (Layer 5 `helqix-state` states) and these numeric run branches. Unifying them is part of the D5 runtime work (finding, not done).
- Kill trigger "branch causal claim fails intervention test: relabel as correlation" is implemented by the NotSupported grade; no Phase 3 candidate in the demo failed its test.
