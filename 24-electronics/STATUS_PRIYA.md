# Priya — HELQIX electronics status

Five lines per phase. Full evidence and round-by-round measurements are kept
in the phase sections below the five-line reports.

## E5 — Interface contract

- Done: `python -m helqix_circuit run CASE.json --out DIR` and the sensor equivalent; exit 0 writes results.json plus values.csv/series.csv, exit 2 is a one-line `error:` for a bad case with nothing written, exit 3 writes a solver_error result; `case/0.1`; limits `timeout_s`, `max_nodes`, `max_steps` enforced by cooperative checks inside the solver loops; a fresh out folder is required; probes are capped at 2000 points; a reference gives VALIDATED/UNVALIDATED, no reference gives CHECKED; JSON schemas, ten example cases per package and docs/priya/INTERFACE_ELECTRONICS.md (including the "What the Studio can show" section) let another engineer wire it without reading the code.
- Tests: 52 new passed (28 circuit CLI + 24 sensor CLI), 0 failed, 0 skipped; suites 97 circuit + 59 sensors + 1 io; 46/46 mutations caught; full verify 120.37 s MEASURED; targets MEASURED: a 10001-sample transient is written as 2000 points with first/last kept and x strictly increasing, a timeout case exits 3 with code `timeout`, a foreign out folder exits 2 and is left untouched, unknown case/field names exit 2, all 20 examples run, circuit results.json feeds the sensor CLI as a probe document.
- FAILs: none. UNKNOWNs: none.
- Files changed: new `helqix_circuit/{cli,__main__}.py`, `helqix_sensors/{cli,__main__}.py`, both `tests/test_cli_*.py`, `examples/circuit/*`, `examples/sensors/*`, `docs/priya/INTERFACE_ELECTRONICS.md`, `docs/priya/schemas/*`; edited `transient.py`/`ac.py` (deadline), `netlist.py`/`sensor.py` (unknown fields), `predictions.txt`, `mutation_checks.py`, this status.
- Needed: nothing; continued to E6.

### E5

- Independent black-box review: a separate process that read only the interface document and the schemas drove both commands 93 times (bad JSON, NaN/Infinity, negative and zero values, huge durations, singular and floating circuits, wrong units, probe and reference misuse, out folder as a file or as the case folder, the circuit-to-sensor pipeline). Result: 93 probes, 0 BAD. Four early flags were mistakes in the probe, not the product: float-looking node names are legal names, a foreign out folder is correctly left untouched, the sensor truth final value equals the circuit V(2) final value exactly, and a missing `analysis` is the documented DC default.
- What it found and I fixed: unknown fields were silently ignored. `"analysys"` next to a valid circuit ran DC, and a misspelled `"noise_densiy"` meant zero noise. Both packages now refuse unknown fields with exit 2 at every documented level (netlist root, component, value, initial, parameters, waveform, analysis; sensor model, range, resolution, saturation, synthesize spec), with regression tests and two new mutations (46/46 caught) and documented defaults (analysis -> dc, optional model fields -> 0, source -> generic).
- Timeout design: a watchdog thread was tried first and crashed the interpreter at shutdown (exit 120, SystemError from numpy inside a daemon thread), so timeout is cooperative: the transient and AC loops check a deadline between steps and report exit 3 code `timeout`; a single long step can overshoot slightly, which the document states.
- Downsampling deviation kept in predictions.txt: the implementation takes 2000 evenly spaced rounded indices (endpoints pinned) rather than the predicted min/max envelope; spikes between picks can be missed and the envelope remains the better long-term option.
- Everything offline: no network, no clock-dependent numbers except the `seconds` timing field.

## E4 — Sensor reality models

- Done: new `helqix_sensors` package; a model names kind (9 named kinds), range, bits or step, noise density, bias, drift/hour, first-order bandwidth, latency, sample rate, clip saturation and source; input is a true-signal time series, a {time_s, value} series, or one probe from a results.json ("probes" list only, unit must match); output resamples, adds bias/drift and white noise, filters with an exact -3 dB first-order stage, delays, quantizes, clips with per-sample flags, and reports a reality gap.
- Tests: 35 passed, 0 failed, 0 skipped (34 sensor tests in 1.88 s + 1 smoke); `helqix_circuit` 69 passed in 4.14 s; 32/32 mutations caught; full verify 66.71 s; targets MEASURED: noise sigma 0.0040325 vs 0.0039620 predicted (1.78% vs 3% target), quantization step exact (1e-12), -3 dB worst 4.2e-12 vs 2%, saturation exact at 1.0 V with 200 flags, same seed identical bytes, reality gap 6.344e-4 RMS noise-dominant.
- FAILs: none. UNKNOWNs: none.
- Files changed: new `helqix_sensors/{errors,sensor}.py`, `tests/test_sensors.py`; edited `__init__.py`, `pyproject.toml`, `mutation_checks.py`, this status.
- Needed: nothing; continued to E5.

## E3 — Harness wire checks

- Done: wires list added to a DC copy of the circuit as series resistors; R = rho*L/A with copper/aluminium resistivity and temperature coefficient, voltage drop and % of supply, I^2 R, steady temperature rise with h = 10 W/(m^2 K) labelled ASSUMPTION, ampacity against IEC 60364-5-52 Table B.52.2 rows, PASS/WARN/FAIL with words; untabulated sizes and aluminium get UNKNOWN and no verdict.
- Tests: 24 new passed, 0 failed, 0 skipped (E3 file 0.89 s); `helqix_circuit` 69 passed in 3.50 s; 23/23 mutations caught; full verify 40.45 s; targets MEASURED: R relative error 0.0, drop relative error 8.27e-15 (target 1e-6 both).
- FAILs: none. UNKNOWNs: aluminium ampacity rows (not transcribed, so UNKNOWN with no verdict) and every size outside the cited rows (UNKNOWN by design, not invented).
- Files changed: new `helqix_circuit/harness.py`, `helqix_circuit/ampacity_table.py`, `helqix_circuit/tests/test_harness.py`; edited `helqix_circuit/__init__.py`, `mutation_checks.py`, this status.
- Needed: nothing; continued to E4.

## E0 — Setup

- Done: inventory, device profile, branch `priya/e1-circuit`, venv, skeleton, conventions, unified verifier.
- Tests: 3 smoke suites, 3 passed, 0 failed, 0 skipped (`verify.py`, 2.12 s total).
- FAILs: none. UNKNOWNs: none.
- Files changed: new `24-electronics/` files only.
- Needed: nothing; continued to E1.

## E2 — Transient, AC and nonlinear

- Done: capacitor, inductor, Shockley diode with series R, level-1 n/p MOSFET, ideal and finite-gain op-amp, BE + trapezoidal transient with adaptive step and damped Newton, AC sweep and frequency metrics.
- Tests: 45 passed, 0 failed, 0 skipped in `helqix_circuit` (17 new E2 tests + 12 adversarial-review regressions), 15/15 mutations caught, full verify 29.85 s; targets MEASURED: RC trap 1.61e-6, BE 0.192% (target 0.5%), RLC f0/Q 1.70e-8 (1%), cutoff 1.43e-16 (1%), finite op-amp 0.0 (0.1%).
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

Measured: `helqix_circuit` suite 3.63 s, 45 tests; full verify 29.85 s. Worst
target errors: RC trapezoidal 1.61e-6 relative (0.5% limit), backward-Euler
0.192%, step halving changed the RC result by well under 1%, RLC f0 and Q
1.70e-8 (1%), RC -3 dB 1.43e-16 (1%), finite-gain inverting op-amp exactly 0.0
vs -Rf/Rin/(1+(1+Rf/Rin)/Aol) (0.1%). Diode-with-series-R and n/p MOSFET tests
compare against formulas written in the test. Adaptive transient ends exactly
on the requested stop time and reports accepted/rejected steps and Newton
iterations. Mutations: 15 registered (5 E1 + 6 E2 + 4 review regressions), 15
caught. ngspice and PySpice are absent from this laptop and no user-level
install was possible, so the ngspice cross-check is UNKNOWN and left out.

Adversarial review (the Agent tool is disabled in this environment, so the
review ran as an independent black-box probe process against the public API
only; probes are in the session temp folder, not the repo). Found and fixed:
(1) adaptive transient crawled to max_steps for any step/pulse source because
the t=0 operating point used the DC source value and breakpoints were ignored;
fixed by evaluating waveforms at t=0 and by cutting steps at waveform
breakpoints with a state restart that holds capacitor voltage and inductor
current. (2) stop_s/step_s/min/max_step_s accepted NaN or infinity, and
max_steps=0 was a solver error; now plain InputError. (3) a diode accepted
t_deg_c below absolute zero; now refused. (4) NaN or infinite AC frequencies
reached the solver; now refused. Zero/negative R, C, L, NaN values, floating
nodes, voltage loops, duplicate ids, wrong units and unknown kinds already
refused with named causes; determinism and non-mutation held.

### E4

Measured: 34 sensor tests in 1.88 s (35 with the package smoke test). Noise:
100000 samples at 1000 Hz, 10 Hz bandwidth, density 1e-3, seed 7; predicted
sigma = density*sqrt(ENBW), ENBW = (fs/2)(1-a)/(1+a); measured 0.0040325
against 0.0039620 predicted, 1.78% (3% limit). Quantization step is exactly
span/2^bits in every literal case (1e-12) and the report agrees; step 0.05 gave
reported values of 0.30 exactly. Saturation is exact at +/-1.0 V with 200
flagged samples. The -3 dB point, found by bisection on |H|, is within 4.2e-12
of the request at 10, 100, 1000 and 4000 Hz (2% limit). Seed 42 twice gives
byte-identical to_dict output; seed 43 differs. Reality gap: RMS 6.344e-4 with
noise dominant; quantization RMS 1.4105e-4 against step^2/12 = 1.4095e-4;
delay term 0.0 at zero latency. Tests hold their own formulas, including the
first-order group delay (a/(1-a))/fs and the honest one-sample-scale lag of a
drifting signal through the first-order stage. Mutations: 9 E4 entries (noise
sigma fs/2 -> fs, exact matched-Z pole replaced by exp(-2*pi*fc/fs),
quantization divisor 2^bits-1, clip scaled by 0.999, delay dropped, drift sign,
ENBW inverted, seed pinned to 1, "probes" key renamed), all caught; 32/32
overall.

Adversarial review (the Agent tool is disabled in this environment, so the
review ran as an independent black-box probe process against the public API
only; the probe script is in the session temp folder, not the repo; 57 probes).
Found and fixed: (1) a probe series ending at 1e7 s caused a 745 GiB allocation
attempt, which surfaced as a numpy MemoryError instead of a plain refusal; any
requested clock above 10,000,000 samples is now an InputError naming the count
and the limit. (2) a probe carrying a different unit from the model ("degC" into
a volt model) was silently mixed; it is now refused with both units named.
(3) negative time_s was accepted; now refused. The same probe process confirmed
refusals for every unknown kind, inverted range, bad bits/step (0, 33, 3.5,
step > span, bits+step), negative/NaN noise, latency, bandwidth, saturation
mode "fold", seed of the wrong type, NaN/inf series values, missing probe
fields, unknown probe id/index, bool probe, probe list of the wrong type,
one-sample series, missing model file (a plain InputError), non-finite signal
specs and spec duration <= 0. It also confirmed: a 1e9 s latency is accepted
and yields a constant output, identical seeds give identical bytes, and neither
the model dict nor the probe document is mutated. On the circuit side it
confirmed harness refusals (length 0/NaN, unknown material, node and method,
gauge range, ambient below absolute zero, NaN supply, inverted drop limits, no
source) and that untabulated wire sizes get UNKNOWN, never an invented verdict.

### E3

Measured: 24 E3 tests in 0.89 s; `helqix_circuit` suite 3.50 s, 69 tests;
full verify 40.45 s. Hand formulas written in the tests: d_mm =
0.127*92^((36-n)/39); R = rho*(1+alpha*(T-20))*L/A with copper 1.724e-8
ohm m / 0.00393 per K and aluminium 2.826e-8 ohm m / 0.00403 per K; 12 V
through 10 ohm and 2 m of 0.5 mm^2 copper gave R relative error 0.0 and drop
relative error 8.27e-15 against the hand formula (target 1e-6 both).
Prediction checked: AWG conversion was named as the main error source, and
the AWG formula is exact by construction in float64; the real risk was the
verdict boundary, so every limit is probed just below, exactly at and just
above (drop 3 % and 5 %, ampacity 80 % and 100 % of the row).

Ampacity rows: IEC 60364-5-52 Annex B, Table B.52.2, PVC insulation, two
loaded conductors, copper, conductor 70 degC, ambient 30 degC, methods A1-F.
The numbers were transcribed from the published ECalPro copy (accessed
2026-11-14) and spot-checked against a second copy (2.5 mm^2 method C = 27 A).
Only the copper rows were transcribed, so an aluminium wire or a size outside
1.5-240 mm^2 gets ampacity UNKNOWN with no verdict and the reason says why.
The steady temperature rise uses h = 10 W/(m^2 K) and is labelled
ASSUMPTION; it is informational and is never used to claim safety.
