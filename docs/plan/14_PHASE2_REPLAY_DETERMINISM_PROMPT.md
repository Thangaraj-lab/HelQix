# 14. Phase 2 Prompt: Freeze, Exact Replay, Determinism Levels

Prerequisites: Gate 0 passed. Gate 1 passed or explicitly waived by the founder in writing. D5 (authoritative runtime owner) resolved in docs/RUNTIME_TRACE.md. Paste 13 first.

Branch: `phase-2-replay`.

## Goal

A failed or flagged run can be frozen at an exact state and replayed, producing the same result (bitwise for the core runtime), with the determinism level declared and recorded for every run.

## Background to verify in code (do not assume)

Per the audit, 07-time has snapshot and replay code (replay.rs defines a `replay` function; snapshot.rs; 99 tests pass), and 04-state has snapshots and diffs. Whether the end-to-end run path (gateway or app) uses them is recorded in RUNTIME_TRACE. If the run path does not use them, wiring is part of this phase.

## Tasks

1. Determinism level type: add `Bitwise | Tolerance(tol) | Statistical` to the run and execution record (find the right existing record type; do not create a parallel one). Every run result carries it. Core runtime runs are Bitwise; external or non-deterministic backends are Tolerance or Statistical and must declare so.
2. Freeze: a freeze captures exactly what is needed to reproduce: model revision identifier and hash, parameters, environment, scenario, initial state, solver identity and version, step configuration, random seeds, logical time, and the state itself. Freeze can be triggered by a failure event, a requirement violation, a manual command, or a time marker. A freeze is immutable and content-addressed (hash).
3. Replay: re-execute from a freeze (or from the start) using only the frozen content; produce a state sequence and a hash chain over states. Replay must not read mutable global state, wall-clock time, environment variables, thread scheduling order, or hash-map iteration order. Audit for these and fix or document each source of nondeterminism.
4. Replay verification: compare original and replay state-hash sequences; report first divergence (time, variable) when they differ.
5. Scope of bitwise claim: state exactly what is guaranteed (same platform, same toolchain, same build, same inputs). Do not claim cross-platform bitwise reproducibility unless you tested it. Floating-point caveats (fused multiply-add, reduction order, different libm) must be listed in docs.
6. External solvers: do not claim exact replay. Record tool, version, configuration, input hashes and output hashes; replay means re-running and comparing within tolerance, level Tolerance. Note: FMI state save and restore is optional and can be subtly wrong in practice; do not rely on it.
7. Run-twice command: a single documented command runs a reference scenario twice (and a replay once) and prints PASS or FAIL with hashes.

## Required tests

- Same inputs twice produce identical state-hash sequences (property test over several random but seeded inputs).
- Replay from a freeze equals the original continuation.
- Negative: change one input bit or one parameter; replay or comparison must detect the divergence.
- Negative: tamper with a freeze file; load must fail the hash check.
- Different thread counts or ordering (if any parallelism exists) still give the same result, or the code documents and tests that it is single-threaded.
- Freeze at failure time of a reference failure scenario, then replay reproduces the failure at the same logical time.
- Mutation sanity check as in the preamble.

## Gate 2

Raw output shows: the run-twice command passes; replay is bit-exact for the core runtime on the reference scenarios; negative tests detect divergence; docs state the exact scope of the guarantee; STATUS updated. If replay cannot be made reproducible, apply the kill trigger: drop the determinism claim, report the sources of nondeterminism found, and stop for a decision.

## Out of scope

Causality, branching UI, signed evidence (key signing is Phase 5), adapters.
