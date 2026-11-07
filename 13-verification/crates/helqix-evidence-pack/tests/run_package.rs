//! E8.1: a package for a stored run of the built-in engine, with a requirement that fails and one that holds.
use std::{fs, path::PathBuf};

use ed25519_dalek::SigningKey;
use helqix_evidence_pack::{public_key_hex, run_files, write_package, Declared};
use helqix_evidence_verify::verify_package;
use helqix_freeze::{chain_genesis, run, Comparator, PointMassSpec, RunSpec};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("helqix-runpack-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    d
}
fn key() -> SigningKey { SigningKey::from_bytes(&[5; 32]) }

fn spec() -> RunSpec {
    RunSpec {
        model: PointMassSpec { mass_kg: 0.5, gravity_m_s2: [0.0, -9.80665, 0.0], air_density_kg_m3: 1.225, wind_m_s: [0.0; 3], applied_force_n: [0.0; 3], drag_coefficient: 0.47, reference_area_m2: 0.01 },
        start_seconds: 0.0, end_seconds: 4.0, step_seconds: 0.001, initial_state: vec![0.0, 20.0, 0.0, 3.0, 0.0, 0.0], seeds: vec![],
    }
}
fn declared(threshold: f64) -> Declared { Declared { name: "stay above ground".into(), variable: 1, variable_name: "y".into(), unit: "m".into(), comparator: Comparator::AtLeast, threshold } }
fn final_link() -> String { let _ = chain_genesis(&spec()); run(&spec()).unwrap().chain.last().cloned().unwrap() }

fn build(name: &str, threshold: f64) -> (PathBuf, serde_json::Value) {
    let dir = tmp(name);
    let (files, statement) = run_files(&spec(), &declared(threshold), &final_link(), b"parquet bytes".to_vec(), "a 0.5 kg body dropped from 20 m").unwrap();
    write_package(&dir, files, &statement, "dev-test", "2026-11-07T00:00:00Z", &key()).unwrap();
    let outcome = serde_json::from_slice(&fs::read(dir.join("requirement_outcome.json")).unwrap()).unwrap();
    (dir, outcome)
}

#[test]
fn a_failing_requirement_gives_a_package_the_offline_verifier_passes() {
    let (dir, outcome) = build("fail", 0.0);
    assert_eq!(outcome["satisfied"], false);
    assert!(outcome["failure"]["index"].is_u64());
    let report = verify_package(&dir, &public_key_hex(&key()));
    assert!(report.pass, "{:?}", report.reasons);
    assert!(dir.join("frames.parquet").exists());
}

#[test]
fn a_requirement_that_holds_gives_a_package_the_offline_verifier_passes() {
    let (dir, outcome) = build("holds", -1000.0);
    assert_eq!(outcome["satisfied"], true);
    assert!(outcome["failure"].is_null());
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(dir.join("manifest.json")).unwrap()).unwrap();
    assert!(manifest["statement"]["failure_index"].is_null());
    let report = verify_package(&dir, &public_key_hex(&key()));
    assert!(report.pass, "{:?}", report.reasons);
}

#[test]
fn a_stored_run_that_a_re_run_does_not_reproduce_gets_no_evidence() {
    let r = run_files(&spec(), &declared(0.0), "00deadbeef", b"x".to_vec(), "s");
    assert!(r.is_err());
    assert!(r.unwrap_err().to_string().contains("no evidence"));
}

#[test]
fn a_changed_frames_file_fails_the_verifier() {
    let (dir, _) = build("tamper", 0.0);
    fs::write(dir.join("frames.parquet"), b"other bytes").unwrap();
    assert!(!verify_package(&dir, &public_key_hex(&key())).pass);
}

#[test]
fn a_signer_that_claims_the_requirement_holds_while_listing_a_failure_is_refused() {
    // Build honestly, then re-sign a package whose outcome says satisfied while it still lists a failure.
    let dir = tmp("lie");
    let (mut files, statement) = run_files(&spec(), &declared(0.0), &final_link(), b"p".to_vec(), "s").unwrap();
    let f = files.iter_mut().find(|f| f.path == "requirement_outcome.json").unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&f.bytes).unwrap();
    v["satisfied"] = serde_json::json!(true);
    f.bytes = serde_json::to_vec_pretty(&v).unwrap();
    write_package(&dir, files, &statement, "dev-test", "2026-11-07T00:00:00Z", &key()).unwrap();
    let report = verify_package(&dir, &public_key_hex(&key()));
    assert!(!report.pass);
    assert!(report.reasons.iter().any(|r| r.contains("failure index")), "{:?}", report.reasons);
}

#[test]
fn a_package_that_holds_but_froze_before_the_last_sample_is_refused() {
    let dir = tmp("early");
    let (mut files, statement) = run_files(&spec(), &declared(-1000.0), &final_link(), b"p".to_vec(), "s").unwrap();
    let f = files.iter_mut().find(|f| f.path == "state_chain.json").unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&f.bytes).unwrap();
    v["samples"] = serde_json::json!(v["samples"].as_u64().unwrap() + 5);
    f.bytes = serde_json::to_vec_pretty(&v).unwrap();
    write_package(&dir, files, &statement, "dev-test", "2026-11-07T00:00:00Z", &key()).unwrap();
    assert!(!verify_package(&dir, &public_key_hex(&key())).pass);
}

#[test]
fn an_outcome_that_lists_a_failure_while_the_statement_says_none_is_refused() {
    // The statement and the freeze say "holds"; only the outcome file carries a failure next to satisfied = true.
    let dir = tmp("outcome_lie");
    let (mut files, statement) = run_files(&spec(), &declared(-1000.0), &final_link(), b"p".to_vec(), "s").unwrap();
    let f = files.iter_mut().find(|f| f.path == "requirement_outcome.json").unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&f.bytes).unwrap();
    v["failure"] = serde_json::json!({"index": 3, "time_seconds": 0.003, "value": -1.0});
    f.bytes = serde_json::to_vec_pretty(&v).unwrap();
    write_package(&dir, files, &statement, "dev-test", "2026-11-07T00:00:00Z", &key()).unwrap();
    assert!(!verify_package(&dir, &public_key_hex(&key())).pass);
}
