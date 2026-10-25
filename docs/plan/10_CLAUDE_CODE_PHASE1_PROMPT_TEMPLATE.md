# 10. Claude Code Prompt Template: Phase 1 (Trust Core) — BLOCKED

Do not use until Gate 0 has passed and decisions D1, D2 and D4 (08) are made. Fill every <PLACEHOLDER>. If a placeholder is empty, do not start.

## Fill-in values (founder)

- <DOMAIN_PACK>: battery cell / motor thermal / other. (D1)
- <CONTEXT_OF_USE>: one paragraph. Example form: "The model is used to predict <quantity> of <system> under <conditions> to decide <decision>." (D2)
- <RISK_LEVEL>: low / medium / high, with a sentence on the consequence of a wrong prediction.
- <ERROR_THRESHOLD_X>: quantity, unit, and number. Example form: "cell terminal voltage RMS error below <X> mV on held-out discharge cycles". (D2)
- <DATASET>: path or source of measured data; who collected it; conditions. (D4)
- <SPLIT_PROTOCOL>: how calibration and validation sets are separated, decided before fitting.
- <RIG_SAFETY_CONFIRMED>: yes or no (see safety section in 08).

## Prompt body (paste after filling)

You are continuing HELQIX on branch `phase-1-trust-core` created from the merged Phase 0 work. Read docs/LOCKED_PLAN.md, docs/STATUS.md and docs/RUNTIME_TRACE.md first. All rules from the Phase 0 prompt still apply (failing test first, raw output, never weaken tests, minimal diffs, honest labels, no push without permission, domain logic stays out of core layers).

Scope: locked build step 1 only, for <DOMAIN_PACK>.

Tasks:
1. Requirements model: machine-checkable requirement (quantity, comparator, threshold, duration, conditions) plus Context of Use and risk level stored with each requirement and carried into results. Context of Use: <CONTEXT_OF_USE>. Risk: <RISK_LEVEL>.
2. Benchmark and regression suite in 23-validation/benchmarks: closed-form reference cases for the solver path used by <DOMAIN_PACK> (derive the analytic answer in the test and compare with tolerance; document the derivation). Include at least: a case with a known analytic solution, a convergence-order check by halving the step size, and a determinism check (same inputs, same output, declared level).
3. Numerical integrity: step-size or convergence error estimate attached to each run result; a run that exceeds the tolerance is flagged, not silently accepted.
4. Calibrate and validate: implement parameter estimation on the calibration subset only; validation on the held-out subset only, per <SPLIT_PROTOCOL>; the split is recorded and hashed before fitting; code must make it impossible to fit on the validation set.
5. Uncertainty and sensitivity: report parameter sensitivity (which parameters move the output) and an uncertainty bound on the validated quantity; state the method used and its assumptions.
6. Validated model library metadata: model identity and version, parameter set, dataset hash, split hash, error achieved, Context of Use, risk level, date. No model gets a Verified status unless the error is below <ERROR_THRESHOLD_X> on held-out data from <DATASET>.
7. Two-axis labels (Origin x Status) attached to outputs where the data structures allow; do not redesign unrelated types.

Gate 1 (done when): all new tests pass with raw output pasted; a validation report states the held-out error against <ERROR_THRESHOLD_X> with the number, the dataset hash, and the uncertainty bound; the report says "Verified for Context of Use X" or "Not verified" honestly. If the error exceeds the threshold, report it and stop; do not tune on the validation set.

Out of scope: signed evidence (locked step 5), replay, causality, UI, adapters, quantum, agent.

Final report: plain English, outcome first, raw numbers, what you could not verify, decisions needed.
