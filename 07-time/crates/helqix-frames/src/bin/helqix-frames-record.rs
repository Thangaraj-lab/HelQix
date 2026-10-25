//! Records the point-mass reference run (Phase 2: 0.5 kg body dropped from 20 m) into a frame store.
//! Usage: `helqix-frames-record <store-dir> [run-id]` (default run id `reference-drop`). Prints the index entry.
use std::process::ExitCode;

use helqix_frames::{point_mass_variables, FrameStore};
use helqix_freeze::{chain_genesis, run, PointMassSpec, RunSpec};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(dir) = args.next() else { eprintln!("usage: helqix-frames-record <store-dir> [run-id]"); return ExitCode::from(1) };
    let id = args.next().unwrap_or_else(|| "reference-drop".into());
    let spec = RunSpec {
        model: PointMassSpec { mass_kg: 0.5, gravity_m_s2: [0.0, -9.80665, 0.0], air_density_kg_m3: 1.225, wind_m_s: [0.0; 3], applied_force_n: [0.0; 3], drag_coefficient: 0.47, reference_area_m2: 0.01 },
        start_seconds: 0.0, end_seconds: 2.0, step_seconds: 0.001, initial_state: vec![0.0, 20.0, 0.0, 3.0, 0.0, 0.0], seeds: vec![],
    };
    let result = run(&spec).map_err(|e| e.to_string()).and_then(|r| FrameStore::open(&dir).and_then(|s| s.put(&id, &r, &point_mass_variables(), &chain_genesis(&spec))).map_err(|e| e.to_string()));
    match result {
        Ok(entry) => { println!("{}", serde_json::to_string_pretty(&entry).expect("json")); ExitCode::SUCCESS }
        Err(e) => { eprintln!("{e}"); ExitCode::from(1) }
    }
}
