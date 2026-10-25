# Calibration method (fixed before calibration code)

Recorded 2026-10-25, committed before `helqix_battery/calibrate.py` exists.

## Candidate model (declared before validation)

PyBaMM `lithium_ion.SPM` with option `contact resistance = true`, parameter set `Chen2020` (PyBaMM 26.10.1.1), isothermal, driven by the measured current profile of each case (discharge plus following rest), initial state set from the measured open-circuit voltage at the end of the rest before discharge. The voltage cut-off event is lowered to 2.0 V so the simulation follows the measured profile instead of stopping at its own cut-off. The **candidate** whose label is decided is this model with the fitted resistance below. The unfitted model (resistance 0) is reported for reference only.

## What is fitted

One parameter: `Contact resistance [Ohm]`, a lumped series resistance standing in for ohmic and electrolyte losses that the SPM does not model. Search range 0 to 0.1 Ohm. Objective: mean RMSE over the 9 calibration cases (C/2, 1C, 1.5C of cells 02, 03, 04). Method: bounded scalar minimisation (`scipy.optimize.minimize_scalar`, method `bounded`, `xatol` 1e-5 Ohm). If the optimum lies within 1e-4 Ohm of a bound, it is flagged. No other parameter is changed.

Reason for this choice: a single, physically interpretable parameter limits over-fitting; the published Chen2020 parameters already absorbed the paper's tuning on these same calibration curves.

## Disclosure

While checking the PyBaMM API (scratch probe, not committed), I ran the **unfitted** model on all four cell02 cases, which exposed one held-out result: cell02_c10 RMSE 78.3 mV with resistance 0. This was a procedural slip (the probe should have used calibration cases only). The method above was chosen before the probe and is not changed because of it; the threshold, split and candidate are unchanged. Calibration cases seen in the probe (unfitted): cell02 C/2 103.3 mV, 1C 75.0 mV, 1.5C 73.4 mV.
