# HELQIX electronics conventions

Scope: everything under `24-electronics/` plus `docs/priya/` and
`.github/workflows/priya-*.yml`. Nothing else in the repository is edited by
this work.

## Language, runtime and dependencies

- Target: CPython 3.10+ on native Windows (tested on 3.13). No WSL.
- Allowed runtime dependencies: `numpy`, `scipy`, `pytest`; `matplotlib` only
  for optional plots. No GPU, no network at run time.
- Create the venv with `python -m venv 24-electronics/.venv`. It is ignored
  by the repository `.gitignore` and must never be committed.
- A package must run from a command line with no service, database, broker or
  privileged install.

## Validation labels

Every reported number carries exactly one label:

| Label | Meaning |
|---|---|
| `MEASURED` | read from a real instrument or wall clock during this session |
| `CALCULATED` | derived by a formula in this repository from sourced inputs |
| `ASSUMPTION` | a modelling choice, not a measurement or a derivation |
| `HYPOTHESIS` | a proposed explanation not yet tested |
| `UNKNOWN` | no sourced value exists, so no number is reported |

A value that cannot be sourced is `UNKNOWN` and is omitted, never guessed.
Every physical constant, material property and table row names its source in
`data/*.json` (`source` field) and in the doc that uses it.

## Units and naming

- Netlist and CLI values are SI after unit conversion: ohm, volt, ampere,
  farad, henry, second, hertz, siemens, metre, kelvin.
- Accepted unit spellings are listed in `helqix_circuit/units.py`. A value
  without a unit is rejected; no implicit defaults.
- Node `"0"` is ground and is never a matrix unknown.
- Component ids are user strings, unique within a case.

## Sources of truth for data

- Material and ampacity data live in `24-electronics/data/*.json`. Each entry
  has `source` (standard, edition, table/row). No row is invented.
- If a wire size has no row in the cited ampacity table, the ampacity verdict
  is `UNKNOWN`; the wire still gets resistance, drop and power results.

## Tests

1. Write the prediction in `predictions.txt` before the code.
2. Write the test first; run it and show it red.
3. Hand-calculated expected values are written in the test as the formula
   with the arithmetic shown. Never take an expected value from the solver.
4. Run mutation checks: break a formula on purpose; the test must fail.
5. Never weaken, skip or delete a test to get green.

Slow tests (100000-sample statistics, 10000-message round trips) are marked
`@pytest.mark.nightly` and run by `verify.py --nightly` and the nightly CI job.

## Interface stability

`python -m helqix_circuit run CASE --out DIR` and
`python -m helqix_sensors run CASE --out DIR` are the platform contract.
Field meanings are frozen in `docs/priya/INTERFACE_ELECTRONICS.md`. Changes
are additive.

## Determinism

- Every stochastic model takes an integer seed; the same seed produces
  byte-identical output.
- No wall-clock time, hash ordering or global RNG is used in numeric results.
  Wall-clock appears only in the `seconds` field and in MEASURED timings.

## Naming

Examples, tests, docs and comments use generic circuits and generic parts.
No fixed end-product name appears anywhere in this folder.

## Branches and commits

- Work happens only on `priya/e1-circuit`, `priya/e2-nonlinear`,
  `priya/e3-harness`, `priya/e4-sensors`, `priya/e5-interface`,
  `priya/e6-automation`, `priya/e7-io`.
- No commit to `main`, `ui-studio-v2` or a branch created by someone else.
- Before every commit: `git grep -n "nva"pi-` must be empty and
  `python 24-electronics/verify.py` must be green.
