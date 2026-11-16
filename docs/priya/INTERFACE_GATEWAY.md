# HELQIX interface gateway (contract only)

This file is the contract a gateway, Studio screen or script implements when it
consumes HELQIX electronics results. **No gateway code lives in this
repository**; the packages produce files, and the gateway reads them.

## What the gateway runs

```text
python -m helqix_circuit run CASE.json --out RUN_DIR
python -m helqix_circuit link-harness CASE.json HARNESS.json --out RUN_DIR [--fault TEXT ...] [--seed N]
python -m helqix_sensors run CASE.json --out RUN_DIR
python scripts/result_bundle.py RUN_DIR BUNDLE_DIR
```

Each command is offline, reads only the files named on its command line
(plus the circuit case for a link run), and writes only inside the given
folder. The gateway passes paths, never library calls, so the packages stay
independently testable.

| exit | meaning | gateway action |
|------|---------|----------------|
| 0 | a result was written | read `results.json`; show verdicts and the validation badge |
| 2 | the input is wrong (JSON, version, field, limit, foreign file in the out folder) | show the one-line `error:` text on stderr; do not retry unchanged |
| 3 | the input was understood but no result exists (singular circuit, floating node, limits, timeout) | read `results.json` (`status: "solver_error"`), show `message` and `code` |

A non-zero exit with a traceback means a package bug, not a bad case; the
gateway should report it as a crash and keep the inputs for a bug report.

## The bundle

`electronics-result/0.1` is the hand-off folder:

| file | meaning |
|------|---------|
| `results.json` | the package output, byte-for-byte unchanged |
| `values.csv` | the scalar table, unchanged (`name,quantity,value,unit,label`); absent in a link-harness bundle, which has no scalar table |
| `series.csv` | the series table when the result has one |
| `manifest.json` | version, source package and version, status, analysis, validation label, the labels and units actually used, and a size plus SHA-256 for every payload file |
| `README.md` | the same facts in plain words, including the label legend |

A gateway must:

1. Read `manifest.json` first and refuse a bundle whose `version` is not
   `electronics-result/0.1`.
2. Recompute each file's SHA-256 and compare it with the manifest before
   showing numbers; a mismatch means the bundle was edited and the numbers
   cannot be trusted.
3. Use `labels` and `units` from the manifest to build its legend; they are
   the complete sets found in the payload.

The manifest schema is `docs/priya/schemas/bundle_manifest.schema.json`.
`build_bundle` refuses a source folder with no `results.json`, a result whose
`status` is neither `ok` nor `solver_error`, a result with no `solver`
identity, a non-finite number, a missing payload file, a `series_csv` name
other than `series.csv`, an out folder holding foreign files, and
`OUT_DIR == RUN_DIR`. The same source always gives byte-identical bundles: no
timestamps, no absolute paths, no reformatting.

## Labels

Every number carries a label; the gateway renders the label next to the
number and never silently promotes one.

| label | meaning |
|-------|---------|
| `GIVEN` | supplied in the input and used as is |
| `CALCULATED` | computed from the input by the package |
| `MEASURED` | observed on a simulated or recorded series |
| `REPORTED` | transcribed from a published table or document, not re-derived here |
| `TABLE` | read from a cited ampacity row |
| `UNKNOWN` | not known; the value is `null` and a `reason` explains why |
| `ASSUMPTION` | depends on a stated modelling assumption |

`validation_label` on a result is separate: `VALIDATED` (a reference matched),
`UNVALIDATED` (a reference was given and missed — never show as a pass), or
`CHECKED` (ran, no reference given).

An `UNKNOWN` number is shown as "unknown" with its reason, never as zero,
never as a pass. The wire temperature rise is `UNKNOWN` by design; the
ampacity is `UNKNOWN` for aluminium and for sizes with no cited row.

## Verdicts

Verdicts order as `FAIL` > `WARN` > `UNKNOWN` > `PASS`. A link report adds
`colour_hint` (`OK`/`WARN`/`FAIL`/`UNKNOWN`) for a colour-coded wire table.
A `FAIL` verdict is still exit 0: the run succeeded, the design failed.

`scenarios[]` in a link report is ordered `as_built` first, then one entry per
fault in command-line order; `fault_verdict` is the worst scenario verdict.
A `no_solution` scenario has an empty `wires` list and a `message`; render the
message, not an empty table.

## Reproducibility

- The same case and harness files give the same results; seeded features use
  an explicit seed (`--seed` for intermittent faults, `seed` in a sensor
  case).
- `seconds` is the only wall-clock number and is not part of any verdict.
- The gateway should keep the case, harness and bundle together; the bundle
  hashes cover only the payload, so the inputs are the other half of a
  reproducible run.

## What the gateway must not do

- Do not parse or re-derive package internals (no importing solver helpers,
  no recomputing a formula from a verdict).
- Do not edit `results.json` or the CSVs inside a bundle; build a new run
  instead.
- Do not trust a bundle with a hash mismatch, a foreign file, or an unknown
  version.
- Do not treat a missing `temperature_rise_c` or `ampacity_a` value as zero.
