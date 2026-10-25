//! Phase 2 required tests (docs/plan/14): determinism, freeze, replay, divergence, tamper.
use helqix_freeze::{
    compare, detect_failure, freeze, load_freeze, replay, run, save_freeze, Comparator, FreezeError,
    FreezeTrigger, PointMassSpec, RunSpec, StateLimit,
};
use helqix_core::determinism::DeterminismLevel;

/// Deterministic pseudo-random inputs (no RNG crate, fixed seed).
fn lcg(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

fn spec(seed: u64) -> RunSpec {
    let mut s = seed;
    RunSpec {
        model: PointMassSpec {
            mass_kg: 0.2 + lcg(&mut s), gravity_m_s2: [0.0, -9.80665, 0.0], air_density_kg_m3: 1.225,
            wind_m_s: [lcg(&mut s) - 0.5, 0.0, 0.0], applied_force_n: [0.0, 0.0, 0.0],
            drag_coefficient: 0.47, reference_area_m2: 0.01,
        },
        start_seconds: 0.0, end_seconds: 4.0, step_seconds: 0.01,
        initial_state: vec![0.0, 0.0, 0.0, 5.0 + 10.0 * lcg(&mut s), 5.0 + 10.0 * lcg(&mut s), 0.0],
        seeds: vec![],
    }
}

fn ground() -> StateLimit { StateLimit { name: "height".into(), variable: 1, comparator: Comparator::AtLeast, threshold: 0.0 } }

#[test]
fn same_inputs_give_identical_hash_chains() {
    for seed in [1, 7, 42, 1234, 99_999] {
        let a = run(&spec(seed)).unwrap();
        let b = run(&spec(seed)).unwrap();
        assert_eq!(a.chain, b.chain, "seed {seed}");
        assert_eq!(a.determinism, DeterminismLevel::Bitwise);
    }
}

#[test]
fn replay_from_a_freeze_equals_the_original_continuation() {
    let original = run(&spec(42)).unwrap();
    for index in [0, 1, 150, original.samples.len() - 1] {
        let frozen = freeze(&spec(42), &original, index, FreezeTrigger::Manual).unwrap();
        let replayed = replay(&frozen).unwrap();
        let verdict = compare(&original, &replayed);
        assert!(verdict.identical, "index {index}: {:?}", verdict.first_divergence);
        assert_eq!(replayed.samples.len(), original.samples.len() - index);
    }
}

#[test]
fn one_flipped_bit_in_the_frozen_state_is_detected_at_the_freeze_point() {
    let original = run(&spec(42)).unwrap();
    let mut frozen = freeze(&spec(42), &original, 100, FreezeTrigger::Manual).unwrap();
    frozen.tamper_state_for_test(4, |v| f64::from_bits(v.to_bits() ^ 1));
    let verdict = compare(&original, &replay(&frozen).unwrap());
    assert!(!verdict.identical);
    let first = verdict.first_divergence.unwrap();
    assert_eq!(first.index, 100);
    assert_eq!(first.variable, Some(4));
}

#[test]
fn a_changed_parameter_diverges_after_the_freeze_point() {
    let original = run(&spec(42)).unwrap();
    let mut changed = spec(42);
    changed.model.mass_kg = f64::from_bits(changed.model.mass_kg.to_bits() + 1);
    let other = run(&changed).unwrap();
    let verdict = compare(&original, &other);
    assert!(!verdict.identical);
    assert!(verdict.first_divergence.unwrap().index >= 1);
}

#[test]
fn a_tampered_freeze_file_fails_to_load() {
    let original = run(&spec(7)).unwrap();
    let frozen = freeze(&spec(7), &original, 50, FreezeTrigger::Manual).unwrap();
    let dir = std::env::temp_dir().join(format!("helqix-freeze-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("f.json");
    save_freeze(&frozen, &path).unwrap();
    assert_eq!(load_freeze(&path).unwrap(), frozen);
    let text = std::fs::read_to_string(&path).unwrap();
    let pos = text.find("\"mass_kg\"").unwrap() + 15;
    let mut bytes = text.into_bytes();
    bytes[pos] = if bytes[pos] == b'1' { b'2' } else { b'1' };
    std::fs::write(&path, bytes).unwrap();
    assert!(matches!(load_freeze(&path), Err(FreezeError::HashMismatch { .. }) | Err(FreezeError::Malformed(_))));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn freeze_is_content_addressed() {
    let original = run(&spec(7)).unwrap();
    let a = freeze(&spec(7), &original, 50, FreezeTrigger::Manual).unwrap();
    let b = freeze(&spec(7), &original, 50, FreezeTrigger::Manual).unwrap();
    let c = freeze(&spec(7), &original, 51, FreezeTrigger::Manual).unwrap();
    assert_eq!(a.sha256(), b.sha256());
    assert_ne!(a.sha256(), c.sha256());
    assert_eq!(a.sha256().len(), 64);
}

#[test]
fn results_do_not_depend_on_threads() {
    let expected = run(&spec(1234)).unwrap().chain;
    let handles: Vec<_> = (0..4).map(|_| std::thread::spawn(|| run(&spec(1234)).unwrap().chain)).collect();
    for h in handles { assert_eq!(h.join().unwrap(), expected); }
}

#[test]
fn freeze_at_failure_replays_the_failure_at_the_same_logical_time() {
    let original = run(&spec(42)).unwrap();
    let failure = detect_failure(&original, &ground()).expect("the reference projectile must hit the ground");
    let frozen = freeze(&spec(42), &original, failure.index, FreezeTrigger::Failure(failure.clone())).unwrap();
    let replayed = replay(&frozen).unwrap();
    let again = detect_failure(&replayed, &ground()).unwrap();
    assert_eq!(again.index, failure.index);
    assert_eq!(again.time_seconds.to_bits(), failure.time_seconds.to_bits());
    assert_eq!(again.value.to_bits(), failure.value.to_bits());
}

#[test]
fn no_failure_when_the_limit_holds() {
    let original = run(&spec(42)).unwrap();
    let loose = StateLimit { name: "height".into(), variable: 1, comparator: Comparator::AtLeast, threshold: -1e9 };
    assert!(detect_failure(&original, &loose).is_none());
}
