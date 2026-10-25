# Edge observation (Phase 7A)

Real measurements enter HELQIX as **Observed** values. They are compared with simulation and they feed validation metrics and anomaly events. **They are never written into Authoritative state automatically.**

Crate: `08-observation/crates/helqix-edge`.

## Safety first (hard rules from the plan)

Any rig with lithium cells must have a fuse or current limiting, a temperature cutoff, a non-flammable enclosure or tray, and a written test procedure. HELQIX does not design cell abuse tests. The founder chooses the rig. The software flags saturation and out-of-range data, but it is **not** a safety system and must not be relied on as one.

## Measurement protocol `helqix.measurement.v1`

One JSON object per line:

```json
{"schema": "helqix.measurement.v1", "device_id": "rig-1", "sensor_id": "bus-v", "quantity": "voltage",
 "unit": "V", "device_time_s": 12.5, "seq": 125, "calibration_ref": "cal-2026-10-01", "value": 3.91}
```

Every sensor is registered first with its device, quantity, exact unit symbol, saturation limits and spike threshold (`SensorSpec`). These numbers come from the actual part's datasheet and are the rig owner's decisions. `ina219_bus_voltage_example` (0 to 26 V) is labelled **ASSUMPTION**: check it against the datasheet of the part you buy.

| Input | Result |
|---|---|
| Not JSON, wrong schema, missing or empty field, negative seq | rejected `Malformed` |
| Unknown sensor, wrong device, wrong quantity | rejected (typed) |
| Unit not exactly the registered symbol (`mV`, `v`, `volt` for `V`) | rejected `UnitMismatch`, never converted |
| NaN or infinity | rejected `NonFinite` |
| Same seq twice | rejected `Duplicate` |
| Older than the newest sample by more than the reorder window | rejected `TooLate` |
| Older than the newest sample, but inside the window | accepted, flagged `Late` |
| Gap in seq | accepted, flagged `Dropout { missing }` |
| At or beyond a saturation limit | accepted, flagged `Saturated` |
| Further than the spike threshold from the median of the last five clean values | accepted, flagged `Spike` |

**Clocks:** device time and receive time are both stored and neither is changed. `clock_offset_s(device)` reports the median of (receive − device) for one device. Comparisons with simulation use device time.

## Recordings

A `helqix.edge-recording.v1` recording keeps the raw lines exactly as received, with their receive times, plus the sensor specs and settings. Replaying it gives the same accepted values and the same rejections (tested). A file can be pinned by its SHA-256; one changed byte is refused.

## Reconciliation

`reconcile(sensor, observed, simulated, anomaly_threshold)` interpolates the simulation at each clean observation's device time. It reports bias, RMSE and max |residual|, counts the samples it left out (flagged, or outside the simulated time span), and raises an anomaly event when |residual| is above the threshold. The threshold is a decision for the rig owner. Nothing gets written into stored values.

`ValueStore::set_authoritative` is the only way to write an Authoritative value, and it refuses anything whose origin is Observed or Simulated (negative test; mutation-checked).

```bash
cargo run -p helqix-edge --bin helqix-edge-check -- <recording.json> <simulated.csv> <sensor_id> <anomaly_threshold> [--expect-sha256 <hex>]
```

## Tests

8 tests cover malformed data, unit mismatch, out-of-order handling, quality flags, separate clocks, recording round trip and replay, the Observed-cannot-overwrite rule, and reconciliation. 5 mutation checks were all caught. All test data is synthetic.

## Gate 7A: NOT PASSED

The gate needs **real rig data that you supply**: ingested, compared with simulation, a validation metric produced and put in an evidence package. Synthetic data cannot pass it. Still to do once real data exists: a recording role in the evidence package, and adding the reconciliation result to the Phase 1 validation report.

The phone tier is out of scope until the rig tier passes.
