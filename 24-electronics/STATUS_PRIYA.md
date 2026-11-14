# Priya — HELQIX electronics status

Five lines per phase. Full evidence and round-by-round measurements are kept
in the phase sections below the five-line reports.

## E0 — Setup

- Done: inventory, device profile, branch `priya/e1-circuit`, venv, skeleton, conventions, unified verifier.
- Tests: 3 smoke suites, 3 passed, 0 failed, 0 skipped (`verify.py`, 2.12 s total).
- FAILs: none. UNKNOWNs: none.
- Files changed: new `24-electronics/` files only.
- Needed: nothing; continued to E1.

## E2 — Transient, AC and nonlinear

- Done: capacitor, inductor, Shockley diode with series R, level-1 n/p MOSFET, ideal and finite-gain op-amp, BE + trapezoidal transient with adaptive step and damped Newton, AC sweep and frequency metrics.
- Tests: 33 passed, 0 failed, 0 skipped in `helqix_circuit` (17 new E2 tests), 11/11 mutations caught, full verify 20.29 s; targets MEASURED: RC trap 1.61e-6, BE 0.192% (target 0.5%), RLC f0/Q 1.70e-8 (1%), cutoff 1.43e-16 (1%), finite op-amp 0.0 (0.1%).
- FAILs: none. UNKNOWN: ngspice cross-check (not installed; no admin or user-level install available in this session).
- Files changed: `helqix_circuit/{core,devices,transient,ac}.py`, `mna.py`, `netlist.py`, `units.py`, `tests/test_{transient,nonlinear,ac}.py`, `mutation_checks.py`.
- Needed: nothing; continued to E3.

## E1 — DC analysis

- Done: versioned JSON netlist, sparse modified nodal analysis, R/V/I/VCVS/VCCS/ideal-switch, floating/loop/singular refusals with named causes.
- Tests: 16 passed, 0 failed, 0 skipped (15 DC tests + 1 smoke), 5/5 mutations caught; targets MEASURED: all hand cases within 1e-9, worst 1.33e-16 (divider).
- FAILs: none. UNKNOWNs: none.
- Files changed: `24-electronics/helqix_circuit/{errors,units,netlist,mna}.py`, `tests/test_dc.py`, `mutation_checks.py`, this status.
- Needed: nothing; continued to E2.

<!-- PHASE-REPORTS -->

## Phase evidence

### E1

Measured: `helqix_circuit` suite 0.99 s, 16 tests. Divider relative error
1.33e-16 against 10*2000/3000. Refusals verified: floating node names the node;
voltage-source loop names V1 and V2; inconsistent ideal constraints raise a
singular-circuit solver error. Mutations: 5 registered, 5 caught. One first
draft mutation (VCCS ground-row sign) was a no-op because ground rows are never
assembled; it was replaced with a live output-row sign mutation, not deleted.

### E2

Measured: `helqix_circuit` suite 3.33 s, 33 tests; full verify 20.29 s. Worst
target errors: RC trapezoidal 1.61e-6 relative (0.5% limit), backward-Euler
0.192%, step halving changed the RC result by well under 1%, RLC f0 and Q
1.70e-8 (1%), RC -3 dB 1.43e-16 (1%), finite-gain inverting op-amp exactly 0.0
vs -Rf/Rin/(1+(1+Rf/Rin)/Aol) (0.1%). Diode-with-series-R and n/p MOSFET tests
compare against formulas written in the test. Adaptive transient ends exactly
on the requested stop time and reports accepted/rejected steps and Newton
iterations. Mutations: 11 registered (5 E1 + 6 E2), 11 caught. ngspice and
PySpice are absent from this laptop and no user-level install was possible, so
the ngspice cross-check is UNKNOWN and left out.
