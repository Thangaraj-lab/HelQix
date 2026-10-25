//! Phase 7A required tests. All data here is synthetic: it exercises the code and cannot pass Gate 7A.
use helqix_edge::{
    ingest::{ina219_bus_voltage_example, IngestConfig, IngestError, Ingestor, QualityFlag},
    reconcile::reconcile,
    recording::Recording,
    store::{Origin, StoreError, ValueStore},
};
use serde_json::json;

fn spec() -> Vec<helqix_edge::ingest::SensorSpec> { vec![ina219_bus_voltage_example("rig-1", "bus-v", 0.2)] }
fn config() -> IngestConfig { IngestConfig { reorder_window_s: 1.0 } }
fn line(seq: u64, t: f64, v: f64) -> String {
    json!({"schema": "helqix.measurement.v1", "device_id": "rig-1", "sensor_id": "bus-v", "quantity": "voltage", "unit": "V",
           "device_time_s": t, "seq": seq, "calibration_ref": "cal-synthetic", "value": v}).to_string()
}
fn with(field: &str, value: serde_json::Value) -> String {
    let mut v: serde_json::Value = serde_json::from_str(&line(0, 0.0, 4.0)).unwrap();
    v[field] = value;
    v.to_string()
}

#[test]
fn malformed_data_is_rejected_with_a_typed_error() {
    let mut ing = Ingestor::new(spec(), config());
    for bad in ["not json".to_owned(), with("schema", json!("v0")), with("value", json!("4.0")), with("seq", json!(-1)), with("calibration_ref", json!("")),
                with("sensor_id", json!("unknown")), with("device_id", json!("rig-2")), with("quantity", json!("current"))] {
        assert!(ing.ingest_line(&bad, 0.0).is_err(), "{bad}");
    }
    assert!(ing.accepted().is_empty());
    assert_eq!(ing.rejected().len(), 8);
    assert!(matches!(ing.ingest_line(&with("sensor_id", json!("unknown")), 0.0), Err(IngestError::UnknownSensor(_))));
}

#[test]
fn unit_mismatch_and_ambiguous_units_are_rejected_not_converted() {
    let mut ing = Ingestor::new(spec(), config());
    for unit in ["mV", "v", "volt", "V ", ""] {
        let r = ing.ingest_line(&with("unit", json!(unit)), 0.0);
        assert!(matches!(r, Err(IngestError::UnitMismatch { .. }) | Err(IngestError::Malformed(_))), "{unit:?}: {r:?}");
    }
    assert!(ing.accepted().is_empty());
}

#[test]
fn out_of_order_within_the_window_is_flagged_and_beyond_it_rejected() {
    let mut ing = Ingestor::new(spec(), config());
    ing.ingest_line(&line(0, 0.0, 4.0), 10.0).unwrap();
    ing.ingest_line(&line(2, 2.0, 4.0), 12.0).unwrap();
    let late = ing.ingest_line(&line(1, 1.5, 4.0), 12.1).unwrap();
    assert!(late.flags.contains(&QualityFlag::Late));
    assert!(matches!(ing.ingest_line(&line(3, 0.5, 4.0), 12.2), Err(IngestError::TooLate { .. })));
    assert!(matches!(ing.ingest_line(&line(2, 2.0, 4.0), 12.3), Err(IngestError::Duplicate { .. })));
    let times: Vec<f64> = ing.accepted().iter().map(|o| o.measurement.device_time_s).collect();
    assert_eq!(times, vec![0.0, 1.5, 2.0], "accepted values are ordered by device time");
}

#[test]
fn saturation_dropout_and_spikes_are_flagged() {
    let mut ing = Ingestor::new(spec(), config());
    for (i, v) in [4.0, 4.01, 3.99, 4.0].iter().enumerate() { assert!(ing.ingest_line(&line(i as u64, i as f64, *v), 0.0).unwrap().flags.is_empty()); }
    assert!(ing.ingest_line(&line(4, 4.0, 5.0), 0.0).unwrap().flags.contains(&QualityFlag::Spike));
    assert!(ing.ingest_line(&line(5, 5.0, 26.0), 0.0).unwrap().flags.contains(&QualityFlag::Saturated));
    assert!(ing.ingest_line(&line(9, 6.0, 4.0), 0.0).unwrap().flags.contains(&QualityFlag::Dropout { missing: 3 }));
}

#[test]
fn device_and_receive_times_are_kept_separately() {
    let mut ing = Ingestor::new(spec(), config());
    for i in 0..5 { ing.ingest_line(&line(i, i as f64, 4.0), 100.0 + i as f64 + if i == 2 { 0.4 } else { 0.0 }).unwrap(); }
    let o = &ing.accepted()[2];
    assert_eq!((o.measurement.device_time_s, o.receive_time_s), (2.0, 102.4));
    assert_eq!(ing.clock_offset_s("rig-1"), Some(100.0));
}

#[test]
fn a_recording_round_trips_and_replays_to_the_same_result() {
    let mut rec = Recording::new(spec(), config());
    let mut live = Ingestor::new(spec(), config());
    let lines = [line(0, 0.0, 4.0), "garbage".to_owned(), line(1, 1.0, 4.02), line(3, 3.0, 26.0), line(2, 2.5, 4.01)];
    for (i, l) in lines.iter().enumerate() { rec.push(i as f64, l); let _ = live.ingest_line(l, i as f64); }
    let bytes = rec.to_bytes();
    let loaded = Recording::from_bytes(&bytes, Some(&rec.sha256())).unwrap();
    assert_eq!(loaded, rec);
    let replayed = loaded.replay();
    assert_eq!(replayed.accepted(), live.accepted());
    assert_eq!(replayed.rejected(), live.rejected());
    let mut tampered = bytes.clone();
    let i = tampered.iter().position(|b| *b == b'4').unwrap();
    tampered[i] = b'5';
    assert!(Recording::from_bytes(&tampered, Some(&rec.sha256())).is_err());
}

#[test]
fn an_observed_value_cannot_overwrite_authoritative_state() {
    let mut store = ValueStore::default();
    store.set_authoritative("cell.initial_ocv_v", 4.18, Origin::Authoritative).unwrap();
    let mut ing = Ingestor::new(spec(), config());
    let o = ing.ingest_line(&line(0, 0.0, 3.70), 0.0).unwrap();
    store.record_observed([o.clone()]);
    assert_eq!(store.set_authoritative("cell.initial_ocv_v", o.measurement.value, Origin::Observed), Err(StoreError::ObservedCannotOverwriteAuthoritative { key: "cell.initial_ocv_v".into() }));
    assert!(store.set_authoritative("cell.initial_ocv_v", 3.9, Origin::Simulated).is_err());
    assert_eq!(store.authoritative("cell.initial_ocv_v"), Some(4.18));
    assert_eq!(store.observed().len(), 1);
}

#[test]
fn reconciliation_measures_residuals_and_raises_anomalies() {
    // Spike threshold 0.5 V so the 0.35 V drop at 3 s is kept and reaches reconciliation as an anomaly.
    let mut ing = Ingestor::new(vec![ina219_bus_voltage_example("rig-1", "bus-v", 0.5)], config());
    let observed = [(0.0, 4.10), (1.0, 4.05), (2.0, 4.00), (3.0, 3.70), (4.0, 26.0), (9.0, 3.9)];
    for (i, (t, v)) in observed.iter().enumerate() { ing.ingest_line(&line(i as u64, *t, *v), 0.0).unwrap(); }
    let sim = [(0.0, 4.1), (2.0, 4.0), (4.0, 3.9)];
    let r = reconcile("bus-v", &ing.accepted(), &sim, 0.1).unwrap();
    assert_eq!((r.used, r.excluded_flagged, r.excluded_out_of_span), (4, 1, 1));
    assert!((r.max_abs - 0.25).abs() < 1e-12);
    assert!((r.rmse - (0.25_f64 * 0.25 / 4.0).sqrt()).abs() < 1e-12);
    assert_eq!(r.anomalies.len(), 1);
    assert_eq!(r.anomalies[0].device_time_s, 3.0);
    assert!(reconcile("bus-v", &ing.accepted(), &[(1.0, 1.0), (0.5, 1.0)], 0.1).is_err());
}
