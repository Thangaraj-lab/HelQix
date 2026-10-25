# Battery split record (fixed before fitting)

Recorded 2026-10-25, before any calibration or validation code existed in the repository (see git history: this file is committed before `helqix_battery/calibrate.py`).

- Protocol: `helqix.battery-split.v1` ([PHASE1_INPUTS.md](../PHASE1_INPUTS.md)): calibration rates C/2, 1C, 1.5C; held-out rate C/10.
- Data: Zenodo 10.5281/zenodo.4032561, files verified against published MD5.
- Split SHA-256: `9858fefda7fc8f779601bad7f25b0567ca4d0e676d8a69e8ac617fb9f28031db`
  (spec: `helqix.split.v1`, over case id, role and SHA-256 of each case's measured time/current/voltage as little-endian f64; identical in `helqix_battery.split` and `helqix-verify`).

| Role | Cases |
|---|---|
| calibration | cell02_c2, cell02_1c, cell02_1c5, cell03_c2, cell03_1c, cell03_1c5, cell04_c2, cell04_1c, cell04_1c5 |
| held out | cell02_c10, cell03_c10, cell04_c10 |

Excluded: step 2 of each file (a 0.5 A discharge that does not follow a charge, so its starting state is unknown). Each C/10 case drops 1 sample whose timestamp repeats the previous one (at the cycler's SBr marker).

Any later report whose split hash differs from this value is refused by `helqix-validate`.
