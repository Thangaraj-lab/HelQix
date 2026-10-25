# Candidate datasets for a fair re-test (after Gate 1)

Researched 2026-10-25 from **metadata only**. No voltage data was downloaded or compared with any model, so each candidate is still unseen and can serve as held-out evidence.

Why a new dataset: the Gate 1 held-out data (Chen2020 C/10 curves) is spent. Any model or method chosen after seeing the Gate 1 result must be judged on data not yet seen ([GATE1_VALIDATION_REPORT.md](GATE1_VALIDATION_REPORT.md)).

## Requirements for a fair re-test

1. Same cell family as the Chen2020 parameter set (LG M50 / M50T 21700, NMC811 / graphite-SiOx).
2. Matches the Context of Use: constant-current C/10 discharge at 25 °C from a known full-charge state ([PHASE1_INPUTS.md](../PHASE1_INPUTS.md)).
3. Independent of the Chen2020 tuning: different cells, ideally a different lab.
4. Measured, raw time series (time, current, voltage) and a reuse licence.

## Shortlist

| # | Dataset | Fit to requirements | Licence | Size | Open questions |
|---|---|---|---|---|---|
| 1 (recommended) | Kirkaldy et al. 2022, "measuring rapid loss of active silicon", Imperial College. Zenodo [10.5281/zenodo.7235858](https://doi.org/10.5281/zenodo.7235858); paper doi:10.1021/acsaem.2c02047 | LG M50T (GBM50T2170). Every cell gets a beginning-of-life Reference Performance Test **at 25 °C** whose first step is a **C/10 discharge** to 2.5 V after CC-CV charge to 4.2 V (C/100 cut-off). 17 cells (ageing groups at 10, 25, 40 °C; the RPT is always at 25 °C). Different lab and cells from Chen2020. Biologic BCS-815 cycler | CC BY 4.0 | 850 MB (one zip) | Rest between charge and C/10 discharge not stated in the record (our method sets the initial state from the rest voltage); sampling rate; file layout; cells are base-cooled in a bespoke rig, not in a thermal chamber |
| 1b | Kirkaldy et al. 2024, "comprehensive cycle ageing data", Zenodo [10.5281/zenodo.10637534](https://doi.org/10.5281/zenodo.10637534); paper doi:10.1016/j.jpowsour.2024.234185 | Same group and cell, 40 cells; appears to include the 2022 experiments (Expt 1 and 5), so **not independent of #1**; use one or the other | CC BY 4.0 | 49 GB | Same as #1 |
| 2 (secondary) | Pozzato et al. 2022, Stanford Energy Control Lab, OSF [osf.io/qsabn](https://osf.io/qsabn/); article doi:10.1016/j.dib.2022.107995 | LG INR21700-M50T, 10 cells; fresh-cell diagnostic included; capacity test is **C/20 at 23 °C** from full charge, so it is outside the current Context of Use (C/10, 25 °C) | Article CC BY 4.0; dataset licence **not stated** | not checked | Licence; whether C/20 at 23 °C would need a revised Context of Use |

Not suitable: Chalmers "Synthetic dataset of LG M50 batteries" (simulated, not measured); Mondragon "LG M50 cell aging dynamic profile tests" (dynamic profiles, outside the Context of Use; not examined further).

## Caveat that applies to all candidates (UNKNOWN)

Chen et al. 2020 describe their cell as "LG M50"; these datasets use "LG M50T" (GBM50T2170). Later literature treats them as the same cell family, but whether Chen's teardown cell is the identical M50T revision is **not verified**. A re-test should state this as an assumption.

## How to use #1 without spoiling it (proposal, not done)

1. **Before downloading**, write and commit a re-test protocol: which model revision is the candidate, which cells and which test (beginning-of-life RPT, first C/10 discharge only), how the initial state is set if there is no rest before discharge, the error window (discharge plus any following rest), and the unchanged threshold (RMSE < 46 mV per cell, all cells must pass).
2. Download, verify the Zenodo checksum, extract only the BoL C/10 steps, record the split hash.
3. Run once; `helqix-validate` decides the label; report it whatever it is.

Any change to the Gate 1 model (for example re-fitting the stoichiometry window, see the Gate 1 diagnosis) must be fixed in step 1, using only Chen2020 calibration data, never this dataset.

Sources: Zenodo records 7235858, 10637534 and 4032561 (metadata via the Zenodo API); PMC article PMC8892076 for the Stanford dataset.
