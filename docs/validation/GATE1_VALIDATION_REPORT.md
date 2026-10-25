# Gate 1 Validation Report: LG M50, PyBaMM SPM + Chen2020

Date: 2026-10-25. Branch `phase-1-trust-core`. Inputs fixed beforehand in [PHASE1_INPUTS.md](../PHASE1_INPUTS.md), [SPLIT_RECORD.md](SPLIT_RECORD.md) and [CALIBRATION_METHOD.md](CALIBRATION_METHOD.md).

## Verdict

**NotVerified** for the stated Context of Use, decided by `helqix-validate` (Rust trust layer), exit status 2.

| Held-out case (C/10 + 2 h rest) | RMSE | Threshold | Result |
|---|---|---|---|
| cell02_c10 | 73.3 mV | < 46 mV | fail |
| cell03_c10 | 71.1 mV | < 46 mV | fail |
| cell04_c10 | 77.3 mV | < 46 mV | fail |

Kill / relabel trigger applied ([plan/06](../plan/06_BUILD_ROADMAP_AND_GATES.md)): "Model error above threshold X: no Verified label." The model was **not** tuned further against the held-out data.

**Published model error bound (CALCULATED):** on held-out constant-current C/10 discharges at 25 °C of three LG M50 cells, this model's terminal-voltage RMSE is **71.1 to 77.3 mV** (mean 73.9 mV), the 95th-percentile absolute error is **134 to 148 mV**, and the peak error is **331 to 355 mV**, at the end of discharge.

## Provenance

| Item | Value |
|---|---|
| Model | PyBaMM 26.10.1.1 `lithium_ion.SPM` (contact resistance on), `Chen2020`, isothermal, IDAKLU rtol 1e-6 atol 1e-8, mesh 20 points per domain |
| Model identity SHA-256 | `024d12543a3c3e7b1a347c361a9fba83236e2ed8f9f9b9e259ae2a25758cf08d` |
| Fitted parameter | contact resistance 0.015805 Ohm (11 evaluations, 1 infeasible trial, not at a bound) |
| Dataset | Zenodo 10.5281/zenodo.4032561, CC BY 4.0, 3 files MD5-verified (see PHASE1_INPUTS.md) |
| Split SHA-256 | `9858fefda7fc8f779601bad7f25b0567ca4d0e676d8a69e8ac617fb9f28031db` (equals the value committed before fitting) |
| Fitted on | 9 calibration cases only (C/2, 1C, 1.5C); verified by the Rust layer |
| report.json SHA-256 | `a9da23c22b69ab84a28e9aea05dcde38bf4fe8a3335c5d5e08ba6ea6f3743fb9` (not committed: it contains measured data; regenerate with the commands below) |
| Library entry | [18-libraries/battery/lgm50_spm_chen2020.library-entry.json](../../18-libraries/battery/lgm50_spm_chen2020.library-entry.json), written by helqix-validate |

## Calibration (calibration cases only)

| Case | RMSE fitted | RMSE unfitted (R = 0) |
|---|---|---|
| cell02 C/2, 1C, 1.5C | 85.7, 40.6, 49.0 mV | 103.3, 75.0, 73.4 mV |
| cell03 C/2, 1C, 1.5C | 82.9, 33.2, 54.6 mV | 100.4, 67.5, 67.5 mV |
| cell04 C/2, 1C, 1.5C | 85.2, 44.2, 54.3 mV | 102.1, 73.6, 69.6 mV |

Mean calibration RMSE after fitting: 58.8 mV. For reference, held-out C/10 with R = 0 (reported, not used): 78.3, 76.2, 82.2 mV. The fitted resistance improves C/10 by about 5 mV only, as expected: at 0.5 A a 16 mOhm resistance shifts voltage by 8 mV.

## Numerical integrity (passed)

Rule: the verdict must not change under 100x tighter solver tolerances or a 2x finer mesh, and every run must complete.

| Held-out case | RMSE baseline / tight / refined | max dV tight / refined |
|---|---|---|
| cell02_c10 | 73.268 / 73.268 / 73.289 mV | 0.009 / 0.98 mV |
| cell03_c10 | 71.093 / 71.128 / 71.114 mV | 0.16 / 0.99 mV |
| cell04_c10 | 77.290 / 77.330 / 77.311 mV | 0.15 / 0.94 mV |

The miss is not a numerical artefact: solution error is below 1 mV against a 25 to 31 mV gap to the threshold. Benchmarks (23-validation/benchmarks/battery) show second-order spatial convergence (observed order 2.10, 2.34) and bitwise-identical repeat runs.

## Uncertainty

Method: per case RMSE, nearest-rank P95 and max |error|; spread across 3 cells. No distribution assumed; n = 3 cells describes observed spread, not a confidence interval. The cycler's voltage accuracy is not stated in the data source and is not included.

Cell-to-cell spread of held-out RMSE: 6.2 mV (71.1 to 77.3 mV).

## Sensitivity (calibration cases, one at a time, +/-10%)

Ranked by largest voltage change: positive-electrode maximum concentration (a 10% change in either direction stops the solver on some or all calibration runs, i.e. the model leaves its valid range), negative-electrode maximum concentration (up to 144 mV), negative particle radius (134 mV), negative particle diffusivity (65 mV), positive particle radius (31 mV), positive particle diffusivity (16 mV), contact resistance (12 mV). Local, one-at-a-time; no interactions. Why +10% positive-electrode maximum concentration stops every calibration run is UNKNOWN (not traced).

## Where the error is (diagnosis only; nothing was changed because of it)

| Held-out case | RMSE first 90% of discharge | last 10% of discharge | following rest | mean signed error in discharge |
|---|---|---|---|---|
| cell02_c10 | 48.6 mV | 160.2 mV | 92.8 mV | +54.1 mV |
| cell03_c10 | 48.4 mV | 156.2 mV | 86.7 mV | +53.6 mV |
| cell04_c10 | 49.5 mV | 167.2 mV | 103.3 mV | +55.3 mV |

The simulation sits about 54 mV above the measurement on average and the error concentrates at the end of discharge and in the relaxation. HYPOTHESIS: a mismatch of the electrode stoichiometry window or capacity between the Chen2020 parameters and these cells (the paper itself tuned maximum concentrations and the 100% SOC stoichiometry on the 1C curve), not a missing series resistance. Not tested.

## What this does and does not say

- It says: this exact model revision does not meet the 46 mV bar on held-out C/10 data, with a quantified error bound.
- It does not say the Chen2020 parameters are wrong in general, nor anything about other C-rates (those are calibration data, not validation evidence).
- **The C/10 data is now spent as held-out evidence.** Any new model or calibration method chosen after seeing these results would be biased if validated on the same C/10 curves. A future "Verified" claim needs new held-out data (another public dataset or the Phase 7A rig) or a method fixed now and judged on data not yet seen.

## Disclosures

- An API probe exposed one unfitted held-out RMSE (cell02 C/10, 78.3 mV) before calibration code existed; nothing was changed because of it ([CALIBRATION_METHOD.md](CALIBRATION_METHOD.md)).
- Two pipeline crashes were fixed during the run (calibration trial and sensitivity perturbation below the solver's 2.0 V limit), each reproduced by a failing synthetic test first; both crashes happened before any held-out result was printed or written.

## Reproduce

From the repository root (Windows: PowerShell; downloads 1.8 MB from Zenodo):

```
cd 18-libraries/python
python -m helqix_battery.validate --data ../../.data/lgm50 --out ../../.data/gate1 --date 2026-10-25
cd ../..
cargo run -p helqix-verify --bin helqix-validate -- .data/gate1/report.json --library-entry .data/gate1/library_entry.json
```

Expected: split hash as above and `LABEL: NotVerified` (exit status 2).

Reproducibility (VERIFIED, same machine and build): two complete pipeline runs on 2026-10-25 produced byte-identical `report.json` files (SHA-256 `a9da23c2...3fb9` both times). Not tested across platforms or PyBaMM versions.
