# HELQIX electronics interface

This document is the contract between the electronics packages and the platform.
Another engineer should be able to wire both commands into the gateway without
reading the source code.

Everything is offline: no network calls, no clock-dependent numbers. The only
wall-clock value in an output is `seconds`, which measures how long the run took.

## Commands

```text
python -m helqix_circuit run CASE.json --out DIR
python -m helqix_circuit link-harness CASE.json HARNESS.json --out DIR [--fault TEXT ...] [--seed N]
python -m helqix_sensors run CASE.json --out DIR
```

Both packages must be importable from the working directory (or installed into
the environment). `CASE.json` is read but never modified. `DIR` is created if it
does not exist.

Exit codes, the out-folder rule and the case version are the same for all three
commands. `link-harness` writes only `results.json`.

## Exit codes

| code | meaning | what is written |
|------|---------|-----------------|
| 0 | a result was produced | `results.json`, `values.csv`, and `series.csv` when there are series |
| 2 | the case is wrong (bad JSON, wrong version, bad field, limit exceeded by the input, folder not fresh) | nothing; a one-line `error: ...` on stderr |
| 3 | the case was understood but no result was produced (singular circuit, max_steps reached, timeout) | `results.json` with `"status": "solver_error"`, `message`, `code`, `seconds` |

Unexpected exceptions (a bug, not a bad case) are not converted; the process
fails with a traceback. That distinction is deliberate: the gateway should
treat 2 and 3 as case problems it can report to a user.

## Out folder

The folder must be empty, or contain only the three files this tool writes
(`results.json`, `series.csv`, `values.csv`). If it contains anything else the
command exits 2 and writes nothing. On a rerun the three known files are
replaced. Nothing outside `DIR` is written or deleted.

## Case version

Every case starts with `"version": "case/0.1"`. A different version is exit 2.

## Circuit case

```json
{
  "version": "case/0.1",
  "circuit": { "version": "circuit/0.1", "components": [...], "analysis": {...} },
  "wires": [...],
  "limits": {"timeout_s": 30, "max_nodes": 10000, "max_steps": 200000},
  "reference": {"quantity": "node_voltage", "name": "V(2)", "value": 6.666666666666667, "relative_tolerance": 1e-9}
}
```

### `circuit`

The netlist object from the circuit package: `version` must be `"circuit/0.1"`,
`components` is a non-empty list (resistor, vsource, isource, capacitor,
inductor, diode, mosfet, opamp, vcvs, vccs, switch) — an empty list is exit 2
with `circuit has no components` — and `analysis.type` selects the run:

- `"dc"` — DC operating point.
- `"tran"` (or `"transient"`) — time stepping. The analysis object also carries
  `stop_s`, `step_s`, `method` (`"trapezoidal"` or `"backward_euler"`),
  `adaptive`, `rtol`, `max_step_s`, `min_step_s`.
- `"ac"` — small-signal sweep. Carries `start_hz`, `stop_hz` and either
  `points` or `points_per_decade`.

`analysis` may be omitted; it then defaults to `{"type": "dc"}`. A misspelled
key would otherwise look like an omission, so unknown fields are refused with
exit 2 everywhere inside the circuit object: the netlist root (`version`,
`title`, `ground`, `components`, `analysis`), each component (`id`, `kind`,
`nodes`, `value`, `state`, `parameters`, `initial`, `waveform`), `value` and
`initial` (`value`, `unit`), `parameters` and `waveform` (the fields of that
kind and waveform type), and `analysis` (the names listed above plus `type`).

Two optional AC extras produce numbers in `values`:

- `"cutoff": {"source": "V1", "node": "2"}` — adds `cutoff_hz`, found by
  bisection on the transfer magnitude (unit `Hz`).
- `"resonance": {"source": "V1", "node": "1", "reference_node": "2",
  "f_min_hz": 100, "f_max_hz": 100000, "points_per_decade": 200}` — adds
  `f_peak_hz` and `q` from the peak search and the half-power points.

### `wires`

Optional list of wire checks, only with a `"dc"` analysis (anything else is
exit 2). Each wire:

| field | meaning |
|-------|---------|
| `id` | unique non-empty string |
| `from`, `to` | node names that already exist in the circuit |
| `length_mm` | one-way length in millimetres, > 0 |
| `gauge_awg` | AWG size, between -20 and 60 (give this or `diameter_mm`, not both) |
| `diameter_mm` | conductor diameter in millimetres, > 0 |
| `material` | `"copper"` or `"aluminium"` |
| `insulation_rating_c` | required rating in degC, > 0; echoed in the per-wire report (a missing or non-positive value is exit 2 naming the field) |
| `installation_method` | `A1 A2 B1 B2 C E F`; default `C` |

The insulation rating does not change today's ampacity or temperature verdict;
it is required so every harness check records the rating it was judged against,
and it is reported back as `GIVEN` in the wire report.

The wire is inserted as a series resistor in a copy of the circuit and the
current through it is measured. `wires` in the result is the full per-wire
report (resistance, drop in V and %, power, temperature rise with its
ASSUMPTION label, ampacity and its source, verdicts and a worded reason).
Ampacity rows come from IEC 60364-5-52 Table B.52.2 (PVC, two loaded
conductors, copper); aluminium and untabulated sizes are `UNKNOWN` and get no
ampacity verdict. `wire_verdict` is the worst per-wire verdict
(`FAIL` > `WARN` > `UNKNOWN` > `PASS`).

### `limits`

| field | default | effect |
|-------|---------|--------|
| `timeout_s` | 30 | checked between solver steps; exceeding it is exit 3 with code `timeout` |
| `max_nodes` | 10000 | non-ground circuit nodes; more is exit 2 |
| `max_steps` | 200000 | passed to the transient solver; hitting it is exit 3 |

The timeout is cooperative: a single very long solver step can overshoot it
slightly. It never changes the numbers when the case finishes in time.

Separately from `limits`, a transient is refused **before any solver work**
when `stop_s / step_s` exceeds **2,000,000 requested steps** (exit 2, and the
message states the computed count and the cap). When `step_s` is omitted the
requested count is `stop_s / (stop_s / 100) = 100`, so only an explicit
`step_s` can trip it. The cap exists because an absurd grid (for example
`stop_s: 10, step_s: 1e-7`, 100 million rows) would otherwise be built
step-by-step inside the timeout thread, burning memory and CPU until either
`max_steps` or `timeout_s` stops it, while the case itself proves the request
impossible. Two million rows is far beyond any example in `examples/circuit`
and still cheap to march; `max_steps` remains the run-time guard on steps
actually taken (exit 3), and the cap only rejects the request up front.

### `reference`

Optional. Compares one produced number against a value the case author trusts:

| field | meaning |
|-------|---------|
| `quantity` | `node_voltage`, `branch_current`, `probe_final` (last sample of a probe) or `metric` (a `values` entry) |
| `name` | `V(2)`, `I(V1)`, the probe id, or the metric name (`cutoff_hz`, `f_peak_hz`, ...) |
| `value` | the expected number |
| `relative_tolerance` | default 1e-6; must be > 0 |

The check is `abs(actual - value) / max(abs(value), 1e-300) <= relative_tolerance`.
A matching reference gives `validation_label` `VALIDATED`; a mismatch gives
`UNVALIDATED` (exit code is still 0). No reference gives `CHECKED`.

## Harness document (`harness/0.1`)

A separate JSON file that describes the physical harness: connectors, wires,
bundles and obstacles. It carries no route polyline — routes are **computed**
by `helqix_circuit.routing` from the two pin positions, the wire bend radius
and the obstacle boxes, and a wire that supplies a `polyline` (or any other
unknown wire key) is refused with exit 2. Colours are transcribed from IEC
60757 / IEC 60445 and labelled `REPORTED`; they are hints, not a safety claim.

```json
{
  "version": "harness/0.1",
  "connectors": [{"id": "J1", "at_mm": [0, 0], "pins": {"1": [0, 0], "2": [0, 30]}}],
  "wires": [{
    "id": "W1", "from": "n1", "to": "n2", "from_pin": "J1.1", "to_pin": "J2.1",
    "gauge_awg": 20, "material": "copper",
    "insulation": {"colour": "red", "rating_c": 105},
    "length_mm": 760, "slack_mm": 20, "bend_radius_mm": 6, "bundle": "B1"
  }],
  "bundles": [{"id": "B1", "diameter_mm": 9}],
  "obstacles": [{"id": "fan", "box_mm": [250, -60, 350, 90]}]
}
```

| field | meaning |
|-------|---------|
| `version` | must be `"harness/0.1"` |
| `connectors` | list of `{id, at_mm, pins}`; `pins` maps a pin id to `[x, y]` in mm; ids must not contain a dot because pin references are `"connector.pin"` |
| `wires` | non-empty list, see below |
| `bundles` | optional list of `{id, diameter_mm > 0}`; a wire may name one |
| `obstacles` | optional list of `{id, box_mm: [x0, y0, x1, y1]}` with `x0 < x1` and `y0 < y1` |

Each wire requires `id`, `from`, `to` (circuit node names), `from_pin`,
`to_pin`, exactly one of `gauge_awg` (between -20 and 60 exclusive) or
`diameter_mm` (> 0), `material` (`copper` or `aluminium`), `insulation`
(`{colour, rating_c}` with `rating_c > 0`), `length_mm` (> 0) and
`bend_radius_mm` (> 0); `slack_mm` (>= 0, default 0) and `bundle` are
optional. An unknown colour name or code is refused listing the known ones.
The loader refuses unknown top-level and wire keys; the schema also closes the
connector, bundle and obstacle objects, and a document should keep to the
fields above.

Loading a document produces, per wire: the given pins with their `at_mm`
points, `diameter_mm` and `area_mm2` derived from `gauge_awg` when that was
given, the insulation echoed as `GIVEN`, the `route` object, and
`length_margin_mm` = `length_mm - route.cut_length_mm` with `route_fits` =
whether the declared length plus slack covers the cut length. A blocked route
is reported with `blocked: true`, null lengths labelled `UNKNOWN` and a
plain-word reason; the document still loads.

### Routes

`helqix_circuit.routing.route(start, end, boxes, bend_radius_mm=..., clearance_mm=...)`
returns axis-aligned polylines with quarter-circle corners. It tries a clear
direct segment first (diagonal is allowed when nothing is in the way), then the
two L corners, then a shortest path on a visibility grid built from the
endpoints and the box edges. Boxes are inflated by `bend_radius_mm +
clearance_mm`; each straight section must be at least the radius at an end and
`2 * radius` between two bends. `cut_length_mm = polyline - 2*R*bends +
bends*pi*R/2` is the length to cut, shorter than the sharp polyline. Equal-cost
routes resolve toward lower y then lower x, so the same input always gives the
same polyline. Anything that cannot be routed within the radius is returned as
blocked with a plain-word reason, never drawn sharp.

## link-harness

```text
python -m helqix_circuit link-harness CASE.json HARNESS.json --out DIR \
  [--fault open:W2] [--fault joint:W1:0.5] [--fault short:W1,W2] \
  [--fault intermittent:W1:0.15] [--seed 11]
```

The case must be a circuit case with a `"dc"` analysis; a case that carries
its own `wires` list is exit 2 (the harness document supplies the wires).
Each harness wire becomes a series resistor between its two circuit nodes,
and the DC operating point gives every wire its current, drop, loss and
verdict. Routes are taken from the harness document model; the wire length in
the resistance comes from `length_mm`, not from the route.

`limits` applies `max_nodes` only; the report is one DC solve per scenario.

### Wire labels and verdicts

| number | label | source |
|--------|-------|--------|
| `supply` | `GIVEN` | the largest non-zero voltage source, or the `argument` overload |
| `resistance_ohm`, `current_a`, `drop_v`, `drop_percent`, `power_w` | `CALCULATED` | resistance `rho20 * L / A` at 20 degC plus any injected joint, then Ohm's law on the solved voltages |
| `temperature_rise_c` | `UNKNOWN` | no cited thermal method; the value is `null` with a reason, never an assumed number |
| `ampacity_a` | `TABLE` / `UNKNOWN` | the IEC 60364-5-52 Table B.52.2 row for copper method C; aluminium or a size without a row is `UNKNOWN` |
| intermittent maxima/minima | `MEASURED` | across the 20 seeded checks |

The drop verdict is PASS to 3 % of the supply, WARN to 5 %, FAIL above. When
the ampacity is known it is judged too (PASS to 80 % of the row, WARN up to
the row, FAIL above) and the worse verdict wins; an UNKNOWN ampacity does not
block a verdict, the drop decides. `colour_hint` maps PASS→`OK`, WARN→`WARN`,
FAIL→`FAIL`, UNKNOWN→`UNKNOWN`.

### Faults

Faults are one scenario each and fully deterministic; they are reported, not
raised as case errors. The scenario list always starts with `as_built`.

| fault | model |
|-------|-------|
| `open:W` | the wire is removed; its current is 0 A and its verdict 'FAIL'. If the remaining circuit still has a DC solution the scenario reports those numbers; if a node loses its only conductive path the scenario reports `status: "no_solution"` with the solver message and verdict `FAIL` |
| `joint:W:OHM` | `OHM` (> 0) is added in series inside that wire, in the stamp and in the drop |
| `short:W1,W2` | a `0.01` ohm branch joins the two wires' `to` nodes; the scenario adds `fault_current_a` and states the model so it is not mistaken for a mid-run short |
| `intermittent:W:P` | 20 checks, one `random.Random(seed)` draw each; the wire is open when the draw is below `P` (0 < P < 1). The scenario reports `checks` `{trials, seed, opened, solved, no_solution}`, the max current over solved checks with label `MEASURED`, the min current, and the wire verdict `FAIL` with "opens in N of 20 seeded checks" |

The same seed and document give the same JSON twice. An unknown fault kind,
an unknown wire name, an unknown field or a bad value is exit 2.

### link results.json

| field | meaning |
|-------|---------|
| `status` | `"ok"`, or `"solver_error"` when the as-built solve itself has no solution (exit 3) |
| `command` | `"link-harness"` |
| `analysis` | `"dc"` |
| `supply` | `{value, unit: "V", label: "GIVEN", source}` |
| `labels` | the label used for each family of numbers |
| `notes` | plain-word temperature, ampacity, short-model and verdict notes |
| `wires` | the as-built per-wire table (fields in the next table) |
| `verdict` | worst as-built wire verdict; exit code stays 0 for a FAIL verdict |
| `fault_verdict` | worst scenario verdict, or `null` when no fault was given |
| `scenarios` | `as_built` then one entry per fault, each with `id`, `fault`, `description`, and the same `wires`/`verdict` fields (a `no_solution` scenario has `message` and an empty `wires` list) |

Each wire entry has `id`, `from`, `to`, `resistance_ohm`, `joint_ohm`
(`null` unless injected), `current_a`, `drop_v`, `drop_percent`, `power_w`,
`temperature_rise_c`, `ampacity_a`, `drop_verdict`, `verdict`, `colour_hint`,
`reason`, `insulation` and the harness `route`.

## Sensor case

```json
{
  "version": "case/0.1",
  "model": { ... },
  "signal": {"synthesize": {"type": "dc", "value": 21.5, "duration_s": 60}},
  "probe": null,
  "seed": 3,
  "limits": {"timeout_s": 30},
  "reference": {"quantity": "reported_mean", "value": 21.5, "relative_tolerance": 1e-3}
}
```

### `model`

Either an object or a path to a JSON file holding that object.

| field | meaning |
|-------|---------|
| `id` | non-empty string |
| `kind` | `temperature pressure acceleration angular_rate current voltage strain distance light`; any other kind is data for the platform, not code |
| `unit` | the physical unit of every value in this case (e.g. `V`, `degC`, `kPa`, `g`, `mm`) |
| `range` | `{"min": ..., "max": ...}` with min < max; readings are clipped to it |
| `resolution` | exactly one of `{"bits": n}` (1..32, step = span / 2^n) or `{"step": s}` (s > 0, at most the span) |
| `noise_density` | white noise density in unit per sqrt(Hz), >= 0; default 0 |
| `bias` | constant offset in unit; default 0 |
| `drift_per_hour` | linear drift in unit per hour of sensor time; default 0 |
| `bandwidth_hz` | first-order -3 dB bandwidth, > 0 and below the Nyquist limit |
| `latency_s` | pure delay in seconds, >= 0 (fractional samples allowed); default 0 |
| `sample_rate_hz` | sensor clock in samples per second, > 0 |
| `saturation` | `{"mode": "clip"}`; default `clip` |
| `source` | where the numbers came from: a datasheet name or `"generic"`; default `"generic"` |

Only the fields in this table are accepted; an unknown field inside the model,
`range`, `resolution` or `saturation` is exit 2, because a misspelled
`noise_densiy` would otherwise silently mean "no noise".

**Quantizer grid convention: zero-anchored.** With `{"bits": n}` the step is
`(max - min) / 2^n` and every reading is rounded to the nearest level
`k * step` for integer `k`, then clipped to `[min, max]`. The grid is anchored
at 0 V, not at `min`. Worked example, range `min = -1`, `max = 4`, 2 bits:
step `= 5 / 4 = 1.25`, so levels include 0, ±1.25, ±2.5, … A reading of
`-0.6` is nearest to level `0` (result 0.0); `0.7` → `1.25`; `2.9` →
`2.5`. Ties are resolved by round-half-to-even (NumPy `rint`), so `0.625`
(0.5 steps) → `0.0` and `-0.625` → `-0.0`, while `1.875` (1.5 steps) →
`2.5`. A range-minimum-anchored grid would place levels at `-1 + k*1.25`
and report `-0.6` as `-0.25`; this package does not do that, and
`test_quantizer_grid_is_zero_anchored` pins the behaviour. `{"step": s}`
uses the same zero-anchored rounding with the given step.

### `signal`

One of three forms:

1. **Synthesized truth** — `{"synthesize": {"type": "dc" | "sine" | "step",
   "duration_s": ..., "value": ..., "amplitude": ..., "frequency_hz": ...,
   "offset": ..., "phase_deg": ..., "delay_s": ...}}`. Must be the only key.
   Each type accepts only its own fields (dc: `value`; sine: `amplitude`,
   `frequency_hz`, `offset`, `phase_deg`; step: `amplitude`, `delay_s`,
   `offset`; all: `duration_s`); anything else is exit 2.
2. **A truth series** — `{"time_s": [...], "value": [...]}` with at least two
   finite samples, strictly increasing time starting at zero or later.
3. **A probe document** — any object with a non-empty `"probes"` list, which is
   exactly the `probes` array of a circuit `results.json`. Each entry needs
   `id`, `time_s`, `value`, and optionally `unit`. If `unit` is present it must
   equal the model unit, otherwise exit 2. With more than one probe, choose one
   with `probe` (an id string or a zero-based index).

An array of numbers is also accepted as a signal; sample interval is 1 /
`sample_rate_hz` and time starts at zero.

### `probe`, `seed`, `limits`, `reference`

- `probe` — id or index selecting one entry of a probe document; ignored for
  other signal forms (an error if the signal is not a probe document).
- `seed` — integer, default 0. The same case file always produces
  byte-identical results; a different seed changes only the noise draw.
- `limits` — sensor cases honor `timeout_s` only (default 30). Any other limit
  key is exit 2.
- `reference` — `quantity` is `reported_mean` (mean of the reported series),
  `reality_gap_rms`, `clipped_samples`, or `metric` (any number in the report,
  named by `name`). Same matching rule and labels as the circuit case.

## results.json

Every field below is present on success unless marked optional.

| field | meaning |
|-------|---------|
| `status` | `"ok"` or `"solver_error"` |
| `analysis` | `dc`, `transient`, `ac` or `sensor` |
| `validation_label` | `VALIDATED` (a reference matched within its tolerance), `UNVALIDATED` (a reference was given and missed), `CHECKED` (ran, no reference) |
| `solver` | `{"name": ..., "version": ...}` of the package that produced the numbers |
| `seconds` | wall-clock seconds for validation-free work; a timing, not a physical quantity |
| `limits` | the effective limits after defaults |
| `values` | list of scalar results, each `{name, quantity, value, unit, label}` |
| `probes` | list of series, see below; may be empty |
| `series_csv` | `"series.csv"` when `probes` is non-empty, else `null` |
| `wires` | optional; per-wire report objects (circuit DC cases with `wires`) |
| `wire_verdict` | optional; worst wire verdict |
| `report` | optional solver report: transient step stats or AC sweep size |
| `reference` | optional; `{quantity, name, expected, actual, relative_error, relative_tolerance, matched}` |
| `model` | sensor cases only: `{id, kind, unit, source}` |
| `seed` | sensor cases only: the seed used |
| `probe` | sensor cases only: the selected probe id/index, or null |

`solver_error` documents contain only `status`, `message`, `code`, `analysis`,
`solver` and `seconds`.

### `values`

- DC: `V(node)` with quantity `node_voltage`, and `I(component)` for every
  branch with quantity `branch_current`.
- Transient: `probe_final` per probe (the last sample).
- AC: `probe_final` per node plus `cutoff_hz` / `f_peak_hz` / `q` with quantity
  `metric`.
- Sensor: `sensor_metric` entries for `sample_count`, `reality_gap_rms`,
  `quantization_step`, `quantization_rms_theory`, `quantization_error_rms`,
  `noise_sigma_predicted`, `noise_sigma_measured`, `noise_error_rms`,
  `delay_error_rms`, `cutoff_requested_hz`, `cutoff_measured_hz`, `enbw_hz`,
  `group_delay_s`, `clipped_samples`. Physical metrics carry the model unit;
  `cutoff_*` and `enbw_hz` are `Hz`, `group_delay_s` is `s`, counts are `1`.
  `label` is `CALCULATED`, `MEASURED` or `ASSUMPTION` depending on how the
  number was obtained.

### `probes`

Each probe is a series ready to draw:

| field | meaning |
|-------|---------|
| `id` | `V(node)`, `I(component)`, `truth`, `analog`, `reported` or `clipped` |
| `unit` | physical unit (`V`, `A`, model unit, or `1` for clipped flags) |
| `x_name` | name of the x axis key: `time_s` for transient and sensor, `frequency_hz` for AC |
| `time_s` or `frequency_hz` | the x values, at most 2000 points |
| `value` | y values, same length as the x array |
| `samples_total` | how many points the solver produced |
| `samples_written` | how many are in this JSON (always <= 2000) |
| `label` | how the numbers are labelled |

Downsampling keeps the first and last point and rounds the remaining picks
uniformly; x stays strictly increasing. The CSV keeps every series at the same
downsampled grid.

Sensor `truth` is the truth resampled to the sensor clock, `analog` is after
noise, filtering and latency (before quantization/clipping), `reported` is what
a real sensor would report, and `clipped` is 1 where the analog value was
outside the range.

### Sensor report

`report` repeats the sensor metrics with their labels: quantization step and
theory RMS, measured and predicted noise sigma with their relative error, ENBW,
measured -3 dB point and its relative error, group delay, clipped sample count,
reality gap RMS and the error decomposition (`noise_error_rms`,
`quantization_error_rms`, `delay_error_rms`, `dominant_error`). `reality_gap_rms`
is the RMS difference between the truth and the reported series: a larger
number means the sensor lies more.

## CSVs

`values.csv` — one row per `values` entry:
`name,quantity,value,unit,label`. Numbers are written with `repr` so a reader
gets the same float64 value back.

`series.csv` — one column per probe, first column named `time_s` or
`frequency_hz`:
`time_s,truth,analog,reported,clipped`. Only written when `probes` is
non-empty.

## Result bundle (`electronics-result/0.1`)

`python 24-electronics/scripts/result_bundle.py RUN_DIR BUNDLE_DIR` copies one
finished result folder into the hand-off bundle: `results.json` and the CSVs
byte for byte, `manifest.json` (version, source package and version, status,
analysis, validation label, the labels and units the payload used, and a
size plus SHA-256 per payload file) and a plain-word `README.md` with the
label legend. The builder refuses a missing or malformed `results.json`, an
unknown `status`, a missing `solver` identity, a non-finite number, a missing
payload file and a non-fresh output folder; the same source gives
byte-identical bundles. A link-harness result has no scalar table, so its
bundle is `results.json`, `manifest.json` and `README.md`.
`docs/priya/INTERFACE_GATEWAY.md` is the consumer contract; no gateway code is
in this repository.

## Pipelines

A transient circuit result feeds a sensor case directly: take the whole
`results.json` of the circuit run and put it under `signal` (or just copy its
`probes` array into a document), set `probe` to a `V(...)` id, and make the
model unit match the probe unit (`V`). AC probes are frequency-domain and are
not valid sensor input.

## What the Studio can show

- **Waveform screen** — plot `probes[]`: x from `time_s` / `frequency_hz`,
  y from `value`, label from `unit` and the probe `id`. AC uses
  `frequency_hz` (log axis is natural). Show `samples_written` vs
  `samples_total` if the user asks whether the curve is complete.
- **Wire table with verdict colors** — one row per `wires[]`: `id`, `from`,
  `to`, `drop_percent`, `current_a`, `ampacity_a`, `temperature_rise_c`,
  `verdict`, `reason`. Color `PASS` green, `WARN` amber, `FAIL` red, `UNKNOWN`
  grey. Show `ampacity_a.source` and the `temperature_note`; do not present the
  temperature rise as a safety claim.
- **Harness layout and link table** — draw the harness `route.points_mm`
  polyline per wire from the link report (or the harness document model) and
  the `obstacles` boxes behind it; a blocked route has no cut length, so show
  its `reason` instead of a line. For link runs, show `colour_hint` next to
  each row and the `scenarios[]` list: `as_built` first, then one row per
  fault with its `fault` text, `description` and verdict. A `no_solution`
  scenario has no wire rows; show its `message`, not an empty table.
- **Sensor reality gap** — big number `reality_gap_rms` in the model unit,
  the decomposition (`noise_error_rms`, `quantization_error_rms`,
  `delay_error_rms`) and `dominant_error`. Plot `truth` against `reported` from
  the probes on one chart; mark `clipped` samples in a different color.
- **Validation badge** — `validation_label` next to every result:
  `VALIDATED` (reference matched), `CHECKED` (ran, no reference),
  `UNVALIDATED` (reference missed — show `reference.relative_error` against
  `relative_tolerance`). Never show `UNVALIDATED` as a pass.
- **Solver line** — `solver.name`, `solver.version`, `seconds`, and the
  effective `limits`, so a repeated run can be recognised.

## Examples

`24-electronics/examples/circuit/` and `24-electronics/examples/sensors/` each
hold ten complete cases. `24-electronics/examples/harness/divider_harness.json`
is the harness document, paired for a link run with
`24-electronics/examples/harness/cases/feed_case.json` (V1 12 V from `p` to
`0`, load R1 10 ohm from `a` to `b`, wires W1 `p`-`a` and W2 `b`-`0`). Run any
of them with:

```text
python -m helqix_circuit run 24-electronics/examples/circuit/dc_divider.json --out out/divider
python -m helqix_circuit link-harness 24-electronics/examples/harness/cases/feed_case.json 24-electronics/examples/harness/divider_harness.json --out out/link --fault joint:W1:1.0
python -m helqix_sensors run 24-electronics/examples/sensors/voltage_reference.json --out out/voltage
```

## JSON schemas

- `docs/priya/schemas/case_circuit.schema.json`
- `docs/priya/schemas/case_sensor.schema.json`
- `docs/priya/schemas/harness.schema.json`
- `docs/priya/schemas/bundle_manifest.schema.json`
- `docs/priya/schemas/results.schema.json`

They follow JSON Schema draft 2020-12 and are stricter than the loader about
field names, but the loader remains the authority on values.
