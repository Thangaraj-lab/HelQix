//! Observed vs Simulated. Simulation is linearly interpolated at each observation's device time.
//! Saturated and spike-flagged samples, and samples outside the simulated time span, are excluded
//! and counted. The output is a metric and a list of anomaly events; nothing is written anywhere.
use serde::Serialize;

use crate::ingest::Observed;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Anomaly { pub device_time_s: f64, pub observed: f64, pub simulated: f64, pub residual: f64 }

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Reconciliation {
    pub sensor_id: String,
    pub unit: String,
    pub used: usize,
    pub excluded_flagged: usize,
    pub excluded_out_of_span: usize,
    /// Mean of (observed - simulated).
    pub bias: f64,
    pub rmse: f64,
    pub max_abs: f64,
    pub anomaly_threshold: f64,
    pub anomalies: Vec<Anomaly>,
}

fn interpolate(sim: &[(f64, f64)], t: f64) -> Option<f64> {
    let i = sim.partition_point(|(x, _)| *x < t);
    if i < sim.len() && sim[i].0 == t { return Some(sim[i].1); }
    if i == 0 || i == sim.len() { return None; }
    let ((t0, v0), (t1, v1)) = (sim[i - 1], sim[i]);
    Some(v0 + (v1 - v0) * (t - t0) / (t1 - t0))
}

/// `simulated` must be sorted by time with strictly increasing times. `anomaly_threshold` is the
/// residual magnitude that raises an anomaly event (a decision for the rig owner, in the sensor's unit).
pub fn reconcile(sensor_id: &str, observed: &[Observed], simulated: &[(f64, f64)], anomaly_threshold: f64) -> Result<Reconciliation, String> {
    if simulated.windows(2).any(|w| w[1].0 <= w[0].0) { return Err("simulated times must strictly increase".into()); }
    if simulated.iter().any(|(t, v)| !t.is_finite() || !v.is_finite()) { return Err("simulated series must be finite".into()); }
    let mine: Vec<&Observed> = observed.iter().filter(|o| o.measurement.sensor_id == sensor_id).collect();
    let unit = mine.first().map(|o| o.measurement.unit.clone()).ok_or_else(|| format!("no observations for {sensor_id}"))?;
    let (mut used, mut flagged, mut out) = (0usize, 0usize, 0usize);
    let (mut sum, mut sum_sq, mut max_abs) = (0.0, 0.0, 0.0_f64);
    let mut anomalies = Vec::new();
    for o in mine {
        if !o.is_clean() { flagged += 1; continue; }
        let Some(s) = interpolate(simulated, o.measurement.device_time_s) else { out += 1; continue };
        let r = o.measurement.value - s;
        used += 1;
        sum += r;
        sum_sq += r * r;
        max_abs = max_abs.max(r.abs());
        if r.abs() > anomaly_threshold { anomalies.push(Anomaly { device_time_s: o.measurement.device_time_s, observed: o.measurement.value, simulated: s, residual: r }); }
    }
    if used == 0 { return Err("no clean observation inside the simulated time span".into()); }
    Ok(Reconciliation { sensor_id: sensor_id.into(), unit, used, excluded_flagged: flagged, excluded_out_of_span: out, bias: sum / used as f64, rmse: (sum_sq / used as f64).sqrt(), max_abs, anomaly_threshold, anomalies })
}
