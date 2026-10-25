//! Run-twice check (docs/plan/14 task 7): runs the reference scenario twice, freezes it
//! at its failure event, saves and reloads the freeze, replays it, and prints PASS or FAIL.
//!
//! Usage: `helqix-replay-check [freeze-output-dir]` (default: the system temp directory).
//! Exit status 0 = PASS, 1 = FAIL.
use helqix_freeze::{compare, detect_failure, freeze, load_freeze, replay, run, save_freeze, Comparator, FreezeTrigger, PointMassSpec, RunSpec, StateLimit};
use std::process::ExitCode;

/// Reference scenario: a 0.5 kg body launched at (12, 9) m/s with quadratic drag in still air,
/// required to stay at or above height 0. It reaches the ground, which is the failure event.
fn reference() -> (RunSpec, StateLimit) {
    let spec = RunSpec {
        model: PointMassSpec { mass_kg: 0.5, gravity_m_s2: [0.0, -9.80665, 0.0], air_density_kg_m3: 1.225, wind_m_s: [0.0; 3],
            applied_force_n: [0.0; 3], drag_coefficient: 0.47, reference_area_m2: 0.01 },
        start_seconds: 0.0, end_seconds: 3.0, step_seconds: 0.001,
        initial_state: vec![0.0, 0.0, 0.0, 12.0, 9.0, 0.0], seeds: vec![],
    };
    (spec, StateLimit { name: "height at or above ground".into(), variable: 1, comparator: Comparator::AtLeast, threshold: 0.0 })
}

fn main() -> ExitCode {
    let dir = std::env::args().nth(1).map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let (spec, limit) = reference();
    let result = (|| -> Result<bool, Box<dyn std::error::Error>> {
        let first = run(&spec)?;
        let second = run(&spec)?;
        let twice = compare(&first, &second);
        println!("run 1 final hash: {}", first.chain.last().unwrap());
        println!("run 2 final hash: {}", second.chain.last().unwrap());
        println!("run twice: {} ({} samples, determinism {})", if twice.identical { "identical" } else { "DIFFERENT" }, twice.compared, first.determinism.label());

        let failure = detect_failure(&first, &limit).ok_or("reference scenario produced no failure event")?;
        println!("failure: '{}' violated at index {} t = {} s (value {:.6})", limit.name, failure.index, failure.time_seconds, failure.value);
        let frozen = freeze(&spec, &first, failure.index, FreezeTrigger::Failure(failure.clone()))?;
        let path = dir.join(format!("helqix-freeze-{}.json", &frozen.sha256()[..16]));
        save_freeze(&frozen, &path)?;
        let loaded = load_freeze(&path)?;
        println!("freeze: {} (sha256 {})", path.display(), loaded.sha256());

        let replayed = replay(&loaded)?;
        let continuation = compare(&first, &replayed);
        let refailure = detect_failure(&replayed, &limit);
        let same_failure = refailure.as_ref().is_some_and(|r| r.index == failure.index && r.time_seconds.to_bits() == failure.time_seconds.to_bits());
        println!("replay final hash: {}", replayed.chain.last().unwrap());
        println!("replay vs original continuation: {} ({} samples)", if continuation.identical { "identical" } else { "DIFFERENT" }, continuation.compared);
        if let Some(d) = &continuation.first_divergence { println!("first divergence: index {} t = {} variable {:?}", d.index, d.time_seconds, d.variable); }
        println!("failure reproduced at the same logical time: {same_failure}");
        Ok(twice.identical && continuation.identical && same_failure)
    })();
    match result {
        Ok(true) => { println!("PASS (Bitwise: same build, platform and inputs; see docs/DETERMINISM.md)"); ExitCode::SUCCESS }
        Ok(false) => { println!("FAIL"); ExitCode::from(1) }
        Err(e) => { println!("FAIL: {e}"); ExitCode::from(1) }
    }
}
