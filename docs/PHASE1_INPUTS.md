# Phase 1 Inputs (filled placeholders for [plan/10](plan/10_CLAUDE_CODE_PHASE1_PROMPT_TEMPLATE.md))

Recorded 2026-10-25. D1, D4 and D5 were chosen by the founder. D2's quantity was chosen by the founder; the founder then delegated the threshold number, Context of Use and risk level to Claude ("the decision is yours ... safe and original, not fake data"). Each delegated value below is anchored to a cited source, not invented, and the founder can change it at any time (log the change in [plan/08](plan/08_OPEN_DECISIONS_AND_HYPOTHESES.md)).

These values are fixed **before any model fitting or validation run**. They must not be changed after seeing validation results.

## DOMAIN_PACK (D1, founder)

Battery cell: LG M50 21700 cylindrical cell (NMC811 / graphite-SiOx), modelled with PyBaMM SPM and the `Chen2020` parameter set, which was derived from this cell.

## DATASET (D4, founder chose public data first; dataset selected by Claude)

- **Source:** Chen, C.-H. et al., "Experimental data for 'Development of Experimental Techniques for Parameterization of Multi-scale Lithium-ion Battery Models'", Zenodo, DOI [10.5281/zenodo.4032561](https://doi.org/10.5281/zenodo.4032561), published 2020-05-15.
- **Licence:** CC BY 4.0 (attribution required; reuse allowed). Data files are **not** committed to git (engineering rule 7); they are downloaded and checked by MD5.
- **Files and published MD5** (verified on download 2026-10-25):
  - `LGM50_cell02.csv` `0bbf959cbfc367296ec69d7a74d2f5c1`
  - `LGM50_cell03.csv` `a09cbd8eff0af55dfe9bc8728fa89908`
  - `LGM50_cell04.csv` `f17615476283d2e15087165697ff9d7c`
- **Who and how:** University of Birmingham / Warwick; Maccor tester; chamber at 25 °C (paper, "Parameter validation testing"). Three cells (ageing history not stated in the source; treated as not deliberately aged), each: CC-CV charge at 1.5 A to 4.2 V, 2 h rest, then constant-current discharge to 2.5 V followed by 2 h rest, at 0.5 A (C/10), 2.5 A (C/2), 5 A (1C), 7.5 A (1.5C). Current, voltage, cell surface temperature and chamber temperature, about 30 s sampling.
- **Reference paper:** Chen et al., J. Electrochem. Soc. 167 080534 (2020), DOI 10.1149/1945-7111/ab9050 (paper licence CC BY-NC-ND 4.0; cited, not redistributed).

## SPLIT_PROTOCOL (D4, fixed before fitting)

The paper states that 8 parameters of the Chen2020 set were **tuned by trial and error against the C/2, 1C and 1.5C discharges** of this data (validation section and Table IX). Those rates are therefore not independent of the Chen2020 parameters.

- **Calibration set:** the C/2, 1C and 1.5C discharges and their following 2 h rests (steps 12-13, 17-18, 22-23), all three cells. Any parameter fitting in Phase 1 uses only these.
- **Held-out validation set:** the C/10 discharge and its following rest (steps 7-8, Maccor step numbering), all three cells. Not mentioned in the paper's tuning; never used for fitting in HELQIX. The code must make fitting on these steps impossible, and the split is hashed before fitting.
- **Excluded:** the short initial 0.5 A discharge from an unknown state (step 2) and all charge steps.

Known limitation (stated, not hidden): C/10 is the least demanding regime (low polarisation). A pass at C/10 says nothing about 1C or 1.5C; those stay "calibrated, not validated".

## CONTEXT_OF_USE (D2, delegated to Claude)

> The model (PyBaMM SPM, Chen2020 parameters, plus any Phase 1 recalibration) is used to predict the terminal voltage of an LG M50 21700 cell during constant-current discharge at 25 °C (cells like those in the dataset, ageing history not stated), in order to reproduce when the cell reaches its 2.5 V cutoff under a given constant load. This supports the HELQIX wedge demonstration "failure reproduced, fix proven in a branch". A "Verified" label applies only to the validated conditions: constant current C/10, 25 °C, cells like those in the dataset. Any other condition (higher C-rates, other temperatures, aged cells, dynamic loads) is outside the Context of Use and is labelled "not validated".

Reason: matches the paying moment in [plan/01](plan/01_PRODUCT_DEFINITION.md) and the only held-out data available.

## RISK_LEVEL (delegated to Claude)

**Low.** Consequence of a wrong prediction: a research or demonstration conclusion is wrong, costing time. No safety, certification or purchase decision depends on it. If a pilot customer ever uses the model for a design or safety decision, the risk must be raised and the threshold reviewed.

## ERROR_THRESHOLD_X (D2, quantity by founder, number delegated to Claude)

**Terminal-voltage RMSE below 46 mV on the held-out C/10 discharge plus its following 2 h rest, for each of the three cells separately.** All three must pass for "Verified for this Context of Use"; otherwise "Not verified".

Anchor (not invented): 46 mV is the largest RMSE the published reference model reached at any C-rate in Chen et al. 2020, Table VIII (C/2 46 mV, 1C 41 mV, 1.5C 36 mV), measured over discharge plus relaxation, as here. The rule is: HELQIX's model must be at least as accurate on data it has never seen as the published model was on the data it was tuned to. Peak error is reported but not gated (the paper's peaks were 271-302 mV, at end of discharge).

## RIG_SAFETY_CONFIRMED

Not applicable to Phase 1: no hardware, no lithium cells handled. Public measured data only. A physical rig is deferred to Phase 7A and needs the safety plan in [plan/08](plan/08_OPEN_DECISIONS_AND_HYPOTHESES.md) first.

## Runtime owner (D5, founder)

Rust core owns the authoritative state; the Python gateway calls it. Phase 1 consequence: requirement, Context of Use and validation-record types live in the Rust trust layer (13-verification); PyBaMM simulation and parameter fitting stay in a Python battery domain pack and hand results to Rust as data, never as authority.

## Claims this does NOT make

- No claim that the model is valid beyond C/10, 25 °C, and cells like those in the dataset.
- No digital-twin claim.
- No claim that the Chen2020 parameters are optimal (the paper itself says they are "not an optimal set").
