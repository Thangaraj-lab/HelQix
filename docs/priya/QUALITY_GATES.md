# Electronics quality gates

One command runs every gate: `python 24-electronics/verify.py` (add
`--nightly` to include the slow seeded suites). It exits 0 only when all
suites pass, every registered mutation is caught, the key scan is clean and
the CI config check passes. This file lists what each gate protects and what a
reviewer should ask for in a pull request.

## 1. Suites

| suite | what it pins |
|-------|--------------|
| `helqix_circuit` | DC/transient/AC solvers against hand formulas, unit scales, refusal messages, CLI contract, routing, harness document, link-harness |
| `helqix_sensors` | quantizer grid, noise/filter/latency model, metrics and labels |
| `helqix_io` | virtual file/probe I/O, no network |

Nightly-marked tests (large seeded sweeps) are excluded from the default run
and included with `--nightly`; CI runs both (the nightly workflow is
scheduled).

## 2. Predictions before runs

`24-electronics/predictions.txt` holds every hand calculation or expected
behaviour written **before** the code ran, followed by an outcome line when a
prediction was wrong. A change that adds a formula must add a prediction block
citing the constants used. Corrections are appended, never edited away, so the
file shows what was believed and what the run taught.

## 3. Mutation checks

`24-electronics/mutation_checks.py` applies one source replacement at a time
to a throwaway copy of `24-electronics/` and requires a **named test** to fail
for each. A mutation that survives is reported with its phase and test. Every
new rule (a verdict threshold, a refusal, a unit conversion, a tie-break)
needs at least one registered mutation; the rule is not considered hardened
until it is caught.

Current phases: `E1`–`E5` (solvers, harness, CLI), `R2-4a` (routing),
`R2-4b` (harness document), `R2-4c` (link-harness).

## 4. Key scan

`24-electronics/scripts/key_scan.py` scans the tree for API-key and token
shapes and fails on a hit. Run it before every commit. A hit is a STOP: do not
commit, report it, remove the secret from the working tree and from history
before pushing.

## 5. CI config check

`24-electronics/scripts/check_ci.py` verifies the workflow files stay in the
shape the gates need (Python matrix, offline runs, nightly dispatch). It runs
inside `verify.py` and in CI.

## 6. Offline and determinism rules

- No network access at test time or run time; examples and tests use no clock
  other than the `seconds` timing field.
- The same input must produce byte-identical JSON; seeded draws use
  `random.Random(seed)` or `numpy.random.default_rng(seed)`.
- Route and link outputs are computed from the input only; no stored polyline
  or hard-coded answer may decide a result.

## 7. Refusal coverage

Every accepted field is paired with tests that feed a misspelled or invalid
variant and require a plain-word `InputError` (exit 2). New fields need a
refusal test, and message assertions use a stable substring rather than the
whole sentence.
