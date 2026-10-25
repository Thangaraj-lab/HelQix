//! Measurement ingest. One JSON object per line:
//!
//! ```json
//! {"schema": "helqix.measurement.v1", "device_id": "rig-1", "sensor_id": "ina219-bus", "quantity": "voltage",
//!  "unit": "V", "device_time_s": 12.5, "seq": 125, "calibration_ref": "cal-2026-10-01", "value": 3.91}
//! ```
//!
//! Device time and receive time are both kept; neither is rewritten. A sensor must be registered
//! with its exact unit symbol, so `v`, `volt` or `mV` for a `V` sensor are rejected, not converted.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SCHEMA: &str = "helqix.measurement.v1";

/// What a sensor is and the limits of the actual part (from its datasheet; the rig owner supplies these).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorSpec {
    pub sensor_id: String,
    pub device_id: String,
    pub quantity: String,
    pub unit: String,
    /// Values at or beyond these are flagged Saturated (the sensor cannot report more).
    pub saturation_low: f64,
    pub saturation_high: f64,
    /// A value further than this from the median of the previous five is flagged Spike.
    pub spike_threshold: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IngestConfig {
    /// A sample may arrive up to this many seconds (device time) behind the newest one and still be accepted (flagged Late).
    pub reorder_window_s: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    pub device_id: String,
    pub sensor_id: String,
    pub quantity: String,
    pub unit: String,
    pub device_time_s: f64,
    pub seq: u64,
    pub calibration_ref: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QualityFlag { Saturated, Spike, Late, Dropout { missing: u64 } }

/// An accepted measurement. Origin is always Observed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observed { pub measurement: Measurement, pub receive_time_s: f64, pub flags: Vec<QualityFlag> }

impl Observed {
    pub const ORIGIN: &'static str = "Observed";
    /// Usable for comparison with simulation: not saturated and not a spike.
    pub fn is_clean(&self) -> bool { !self.flags.iter().any(|f| matches!(f, QualityFlag::Saturated | QualityFlag::Spike)) }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IngestError {
    Malformed(String),
    UnknownSensor(String),
    WrongDevice { sensor_id: String, expected: String, got: String },
    UnitMismatch { sensor_id: String, expected: String, got: String },
    QuantityMismatch { sensor_id: String, expected: String, got: String },
    NonFinite(String),
    Duplicate { sensor_id: String, seq: u64 },
    TooLate { sensor_id: String, device_time_s: f64, newest_s: f64 },
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
}

#[derive(Debug, Default, Clone)]
struct SensorState { seen: std::collections::BTreeSet<u64>, max_seq: Option<u64>, newest_s: Option<f64>, recent: Vec<f64> }

#[derive(Debug, Clone)]
pub struct Ingestor { specs: BTreeMap<String, SensorSpec>, config: IngestConfig, state: BTreeMap<String, SensorState>, accepted: Vec<Observed>, rejected: Vec<IngestError> }

impl Ingestor {
    pub fn new(specs: Vec<SensorSpec>, config: IngestConfig) -> Self {
        Self { specs: specs.into_iter().map(|s| (s.sensor_id.clone(), s)).collect(), config, state: BTreeMap::new(), accepted: Vec::new(), rejected: Vec::new() }
    }
    pub fn specs(&self) -> Vec<SensorSpec> { self.specs.values().cloned().collect() }
    pub fn config(&self) -> IngestConfig { self.config }
    /// Accepted values, ordered by sensor then device time.
    pub fn accepted(&self) -> Vec<Observed> {
        let mut v = self.accepted.clone();
        v.sort_by(|a, b| a.measurement.sensor_id.cmp(&b.measurement.sensor_id).then(a.measurement.device_time_s.total_cmp(&b.measurement.device_time_s)));
        v
    }
    pub fn rejected(&self) -> &[IngestError] { &self.rejected }

    /// Median of (receive time - device time) for one device: the clock offset estimate.
    pub fn clock_offset_s(&self, device_id: &str) -> Option<f64> {
        let mut d: Vec<f64> = self.accepted.iter().filter(|o| o.measurement.device_id == device_id).map(|o| o.receive_time_s - o.measurement.device_time_s).collect();
        if d.is_empty() { return None; }
        d.sort_by(f64::total_cmp);
        Some(d[d.len() / 2])
    }

    pub fn ingest_line(&mut self, line: &str, receive_time_s: f64) -> Result<Observed, IngestError> {
        let r = self.check(line, receive_time_s);
        if let Err(e) = &r { self.rejected.push(e.clone()); }
        r
    }

    fn check(&mut self, line: &str, receive_time_s: f64) -> Result<Observed, IngestError> {
        let v: Value = serde_json::from_str(line).map_err(|e| IngestError::Malformed(format!("not JSON: {e}")))?;
        if v["schema"] != SCHEMA { return Err(IngestError::Malformed(format!("schema must be {SCHEMA}"))); }
        let text = |k: &str| v[k].as_str().filter(|s| !s.is_empty()).map(str::to_owned).ok_or_else(|| IngestError::Malformed(format!("{k} must be non-empty text")));
        let number = |k: &str| v[k].as_f64().ok_or_else(|| IngestError::Malformed(format!("{k} must be a number")));
        let m = Measurement {
            device_id: text("device_id")?, sensor_id: text("sensor_id")?, quantity: text("quantity")?, unit: text("unit")?,
            device_time_s: number("device_time_s")?, seq: v["seq"].as_u64().ok_or_else(|| IngestError::Malformed("seq must be a non-negative integer".into()))?,
            calibration_ref: text("calibration_ref")?, value: number("value")?,
        };
        if !m.device_time_s.is_finite() || !m.value.is_finite() || !receive_time_s.is_finite() { return Err(IngestError::NonFinite(m.sensor_id)); }
        let spec = self.specs.get(&m.sensor_id).ok_or_else(|| IngestError::UnknownSensor(m.sensor_id.clone()))?;
        if spec.device_id != m.device_id { return Err(IngestError::WrongDevice { sensor_id: m.sensor_id.clone(), expected: spec.device_id.clone(), got: m.device_id.clone() }); }
        if spec.unit != m.unit { return Err(IngestError::UnitMismatch { sensor_id: m.sensor_id.clone(), expected: spec.unit.clone(), got: m.unit.clone() }); }
        if spec.quantity != m.quantity { return Err(IngestError::QuantityMismatch { sensor_id: m.sensor_id.clone(), expected: spec.quantity.clone(), got: m.quantity.clone() }); }
        let st = self.state.entry(m.sensor_id.clone()).or_default();
        if st.seen.contains(&m.seq) { return Err(IngestError::Duplicate { sensor_id: m.sensor_id.clone(), seq: m.seq }); }
        let mut flags = Vec::new();
        if let Some(newest) = st.newest_s {
            if m.device_time_s < newest - self.config.reorder_window_s { return Err(IngestError::TooLate { sensor_id: m.sensor_id.clone(), device_time_s: m.device_time_s, newest_s: newest }); }
            if m.device_time_s < newest { flags.push(QualityFlag::Late); }
        }
        if let Some(max) = st.max_seq { if m.seq > max + 1 { flags.push(QualityFlag::Dropout { missing: m.seq - max - 1 }); } }
        if m.value <= spec.saturation_low || m.value >= spec.saturation_high { flags.push(QualityFlag::Saturated); }
        if st.recent.len() >= 3 {
            let mut r = st.recent.clone();
            r.sort_by(f64::total_cmp);
            if (m.value - r[r.len() / 2]).abs() > spec.spike_threshold { flags.push(QualityFlag::Spike); }
        }
        st.seen.insert(m.seq);
        st.max_seq = Some(st.max_seq.map_or(m.seq, |x| x.max(m.seq)));
        st.newest_s = Some(st.newest_s.map_or(m.device_time_s, |x| x.max(m.device_time_s)));
        if !flags.contains(&QualityFlag::Spike) && !flags.contains(&QualityFlag::Saturated) {
            st.recent.push(m.value);
            if st.recent.len() > 5 { st.recent.remove(0); }
        }
        let o = Observed { measurement: m, receive_time_s, flags };
        self.accepted.push(o.clone());
        Ok(o)
    }
}

/// Example spec for an INA219 bus-voltage channel. **ASSUMPTION**: about 26 V bus maximum and a
/// 12-bit ADC per the common datasheet; verify against the datasheet of the actual part chosen.
pub fn ina219_bus_voltage_example(device_id: &str, sensor_id: &str, spike_threshold_v: f64) -> SensorSpec {
    SensorSpec { sensor_id: sensor_id.into(), device_id: device_id.into(), quantity: "voltage".into(), unit: "V".into(), saturation_low: 0.0, saturation_high: 26.0, spike_threshold: spike_threshold_v }
}
