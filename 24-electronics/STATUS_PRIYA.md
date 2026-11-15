# Priya — HELQIX electronics status

Five lines per phase. Full evidence and round-by-round measurements are kept
in the phase sections below the five-line reports.

## Final sweep — all seven phases

Status: **DONE**, nothing BLOCKED. Work is on branches `priya/e1-circuit` …
`priya/e7-io`, committed locally, not pushed.

| phase | status | commit(s) | tests now | main targets, measured | UNKNOWNs |
|-------|--------|-----------|-----------|------------------------|----------|
| E0 Setup | DONE | 8875dae | 3 smoke | verify.py runs every suite end to end, 2.12 s | none |
| E1 DC MNA | DONE | 2d5a48d | 16 fast | 15 hand cases within 1e-9, worst 1.33e-16; 5/5 mutations | none |
| E2 Transient/AC/nonlinear | DONE | f54e98c, 5cd642a | 45 fast | RC trap 1.61e-6, BE 0.192% (0.5%), RLC f0/Q 1.70e-8, cutoff 1.43e-16, op-amp 0.0 (0.1%) | ngspice cross-check |
| E3 Harness | DONE | dd8018f | 24 new | R error 0.0, drop 8.27e-15; IEC B.52.2 verdicts both sides of every limit | aluminium and untabulated ampacity |
| E4 Sensors | DONE | 579b4d1 | 35 fast | noise 1.78% (3%), step exact 1e-12, -3 dB 4.2e-12 (2%), clip exact, seed-identical bytes, reality gap 6.344e-4 | none |
| E5 Interface | DONE | f96afca, 8ab6af8 | 52 new CLI | 2000-point cap, exit 0/2/3, fresh folder, VALIDATED/CHECKED/UNVALIDATED, 20 examples, 93 external probes 0 BAD | none (envelope deviation kept) |
| E6 Automation | DONE | ced94ef | 46/46 mutations | fast 123.41 s, nightly 120.51 s (limit 180 s); workflow YAML parsed once with PyYAML 6.0.3 | Actions runtime, hook install |
| E7 Virtual I/O | DONE | 3b00e4d | 19 fast + 20 nightly | 10000 frames 0.049 s, 10000 messages 0.080 s, 11/11 mutations, amqtt 49/49 topic decisions | wire-level MQTT |

Verification runs after E7 (MEASURED): verify.py three times green, 127.96 s,
136.85 s and 142.48 s; nightly verify green at 132.05 s; every run reports
57/57 mutations caught, key scan clean and CONFIG clean. Current totals:
175 fast tests (97 circuit + 59 sensors + 19 io) and 235 in nightly mode
(117 + 79 + 39).

Every missed or unknown target was re-attempted in this sweep:
- ngspice: retried with network available. `pip` has no ngspice distribution;
  PySpice 1.5 installs but its `ngspice.dll` is not shipped, so loading the
  shared library fails (OSError 0x7e). Cross-check stays **UNKNOWN**.
- Workflow YAML: parsed with PyYAML 6.0.3 in a throwaway venv. Triggers
  (`push`, `pull_request`, `schedule`), the cron line, both jobs, both OS
  values and both verify commands are exactly as intended (**MEASURED**);
  running Actions itself offline remains **UNKNOWN**.
- E2 numbers, E3 verdict sides, E4 targets, E5 contract probes, E6 timing and
  E7 round trips were all re-run by the three verify passes above and held.

UNKNOWNs, in one place:
1. E2 ngspice cross-check: no user-level ngspice binary or library exists on
   this laptop.
2. E3 aluminium and untabulated wire ampacity: UNKNOWN by design, no verdict,
   because those rows are not transcribed.
3. E6 GitHub Actions runtime: the YAML is valid and structurally checked, but
   the runner cannot execute offline; first real run is on first push.
4. E6 pre-commit hook: one documented copy command, not installed
   automatically.
5. E7 wire-level MQTT interop: deliberately not implemented; only topic
   matching was compared with a real broker.
6. E5 downsampling: rounded-even-spacing indices, not a min/max envelope;
   kept as a documented limitation, not a lowered target.

## E7 — Virtual I/O

- Done: new `helqix_io` package, standard library only: `serial_pair` with length + CRC32 framing, bounded FIFO queues, read/write timeouts, write back-pressure, disconnect/reconnect, seeded drop/corrupt/delay faults and counters; in-process MQTT-style `Broker`/`Client` with `+`/`#` topics, QoS 0/1, ack plus `retry_unacked()` after a reconnect, whole-publish back-pressure refusal; documented as a reusable building block in `docs/priya/INTERFACE_IO.md`.
- Tests: 19 fast + 20 nightly seeded property cases, 0 failed, 0 skipped; `helqix_io` suite 1.28 s (fast and nightly together); 11 new mutations, 57/57 caught; targets MEASURED: 10000 serial frames round trip in 0.049 s and 10000 bus messages in 0.080 s with every payload identical and in order; drop/corrupt never deliver an altered payload; the same seed reproduces the same drops and corruptions; 20–50 ms timeouts return bounded; a QoS 1 message published while the subscriber is away is redelivered after reconnect and stops after the ack.
- FAILs: none. UNKNOWNs: wire-level MQTT compatibility (deliberately not implemented); the offline suite has no real-broker test, but a one-off probe against amqtt 0.12.2 in a throwaway venv agreed on 49 of 49 filter/topic decisions.
- Files changed: new `helqix_io/{__init__,errors,faults,serial_port,bus}.py`, `helqix_io/tests/{test_serial,test_bus,test_properties}.py`, `docs/priya/INTERFACE_IO.md`; edited `mutation_checks.py`, `scripts/check_ci.py`, `predictions.txt`, this status.
- Needed: nothing; ready for the final sweep.

### E7

- Round trips, measured on this laptop (Python 3.13.12, in-process, no threads):
  10000 serial frames written and read back in 0.049 s and 10000 broker
  messages published and received in 0.080 s, every payload identical, every
  read in send order, ids strictly increasing. Framing is a 4-byte big-endian
  length, the payload and a CRC32; empty and binary payloads are covered by
  tests.
- Faults are applied by the receiving side with a per-port `random.Random`
  seeded from the profile: `test_drop_injection_is_seeded_and_counted` runs
  1000 frames twice with seed 7 and drop 0.5 and gets identical counts, with
  received + dropped = 1000; `test_corrupt_injection_never_delivers_bad_bytes`
  flips bytes, counts them and verifies every delivered payload is unchanged;
  `test_delay_holds_the_frame_until_its_time` shows a 100 ms delay is not
  visible to a 20 ms read; the nightly property suite replays 20 random seeds
  with random drop/corrupt/delay and checks reproducibility, counters and that
  survivors are an in-order subsequence.
- Back-pressure and disconnect: the serial writer polls a 1 ms deadline and
  raises `BackpressureError` without partial delivery; the broker refuses the
  whole publish when any target queue is full; both ends of a cut serial pair
  answer `DisconnectedError` and clear queued frames; reconnect starts clean.
- QoS 1 retry: a message published while the subscriber is disconnected stays
  pending on the publisher; `retry_unacked()` resends it under a new id after
  reconnect, `ack()` records it, the next `retry_unacked()` returns 0 and a
  second receive times out. QoS 0 is never retried.
- Real-broker probe: amqtt 0.12.2 plus paho-mqtt installed in a throwaway venv
  under the user temp folder (no admin, network used only for installation);
  seven filters against seven topics gave 49 decisions, all identical to
  `topic_matches`, including the `a/#` matches `a` case. The probe is not in
  the repo or the offline suite; wire-protocol interop is UNKNOWN by design.
- Mutation harness: 11 E7 mutations registered, 11 caught (57/57 overall). The
  first "corrupted frame delivered" mutation turned out to be a no-op because
  the corrupt branch still skipped the frame; it was rewritten to deliver the
  frame and was then caught by the counter assertion. Kept in predictions.txt
  rather than deleted.

## E6 — Automation

- Done: `verify.py` runs every suite, every registered mutation, the key scan and the automation config, and exits 0 only when all four are green; `--nightly` adds 20 random-seeded property tests per package (circuit: random series ladder against the hand formula plus scaling invariance; sensors: range, quantization grid, clip flags, determinism and the reality-gap identity); `.github/workflows/priya-electronics.yml` runs the fast verify on push and pull request on windows-latest and ubuntu-latest and a nightly cron job with `--nightly` on both; `scripts/precommit.py` blocks a commit when a staged file looks like a credential and has a planted-key selftest; `README.md` gives a new person the three commands.
- Tests: fast verify 123.41 s MEASURED (limit 180 s), nightly 120.51 s MEASURED; suites 117 circuit + 79 sensors + 1 io in nightly mode (97 + 59 + 1 in fast mode); 46/46 mutations caught; key scan clean; CONFIG clean.
- FAILs: none. UNKNOWNs: the workflow file is checked as text only (no YAML parser under the allowed dependencies and no Actions runtime offline); its first real run happens on the first push. The pre-commit hook is installed by one documented copy command, not automatically.
- Files changed: new `.github/workflows/priya-electronics.yml`, `24-electronics/README.md`, `scripts/precommit.py`, `scripts/check_ci.py`, `helqix_circuit/tests/test_properties.py`, `helqix_sensors/tests/test_properties.py`; edited `verify.py`, `predictions.txt`, this status.
- Needed: nothing; continued to E7.

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
