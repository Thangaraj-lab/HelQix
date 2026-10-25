//! Replays a recording and reconciles one sensor against a simulated series.
//!
//! Usage: `helqix-edge-check <recording.json> <simulated.csv> <sensor_id> <anomaly_threshold> [--expect-sha256 <hex>]`
//! `simulated.csv` has a header line and rows `time_s,value` in the sensor's unit.
//! Prints the recording hash, ingest counts, clock offset and the reconciliation as JSON.
use std::process::ExitCode;

use helqix_edge::{reconcile::reconcile, recording::Recording};
use serde_json::json;

fn run(a: &[String]) -> Result<(), String> {
    let [rec_path, sim_path, sensor, threshold, rest @ ..] = a else { return Err("usage: helqix-edge-check <recording.json> <simulated.csv> <sensor_id> <anomaly_threshold> [--expect-sha256 <hex>]".into()) };
    let expect = match rest { [f, h] if f == "--expect-sha256" => Some(h.as_str()), [] => None, _ => return Err("unexpected arguments".into()) };
    let threshold: f64 = threshold.parse().map_err(|_| "anomaly_threshold must be a number")?;
    let bytes = std::fs::read(rec_path).map_err(|e| format!("{rec_path}: {e}"))?;
    let rec = Recording::from_bytes(&bytes, expect)?;
    let ing = rec.replay();
    let sim = std::fs::read_to_string(sim_path).map_err(|e| format!("{sim_path}: {e}"))?.lines().skip(1).filter(|l| !l.trim().is_empty()).enumerate()
        .map(|(i, l)| {
            let mut p = l.split(',').map(|x| x.trim().parse::<f64>());
            match (p.next(), p.next(), p.next()) { (Some(Ok(t)), Some(Ok(v)), None) => Ok((t, v)), _ => Err(format!("{sim_path} row {}: expected time_s,value", i + 2)) }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let accepted = ing.accepted();
    let r = reconcile(sensor, &accepted, &sim, threshold)?;
    let devices: std::collections::BTreeSet<_> = accepted.iter().map(|o| o.measurement.device_id.clone()).collect();
    let out = json!({
        "recording_file_sha256": format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(&bytes)), "origin": "Observed", "lines": rec.entries.len(), "accepted": accepted.len(), "rejected": ing.rejected().len(),
        "clock_offset_s": devices.iter().map(|d| (d.clone(), ing.clock_offset_s(d))).collect::<std::collections::BTreeMap<_, _>>(),
        "reconciliation": r,
        "note": "Observed vs Simulated metric. Not written into authoritative state. Counts toward validation only with real, held-out rig data.",
    });
    println!("{}", serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match run(&a) { Ok(()) => ExitCode::SUCCESS, Err(e) => { eprintln!("{e}"); ExitCode::from(1) } }
}
