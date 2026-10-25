use std::{fs, path::PathBuf};

use ed25519_dalek::SigningKey;
use helqix_evidence_pack::{public_key_hex, reexecute, reference_files, write_package, PackError};
use helqix_evidence_verify::verify_package;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("helqix-pack-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    d
}
fn key() -> SigningKey { SigningKey::from_bytes(&[3; 32]) }

#[test]
fn a_built_reference_package_verifies_independently() {
    let dir = tmp("roundtrip");
    let (files, statement) = reference_files().unwrap();
    write_package(&dir, files, &statement, "dev-test", "2026-10-25T00:00:00Z", &key()).unwrap();
    let report = verify_package(&dir, &public_key_hex(&key()));
    assert!(report.pass, "{:?}", report.reasons);
}

#[test]
fn the_same_inputs_build_the_same_manifest_and_root() {
    let (a, b) = (tmp("det_a"), tmp("det_b"));
    let (fa, sa) = reference_files().unwrap();
    let (fb, sb) = reference_files().unwrap();
    let ra = write_package(&a, fa, &sa, "dev-test", "2026-10-25T00:00:00Z", &key()).unwrap();
    let rb = write_package(&b, fb, &sb, "dev-test", "2026-10-25T00:00:00Z", &key()).unwrap();
    assert_eq!(ra.merkle_root, rb.merkle_root);
    assert_eq!(fs::read(a.join("manifest.json")).unwrap(), fs::read(b.join("manifest.json")).unwrap());
}

#[test]
fn the_package_re_executes_bit_for_bit_and_a_changed_freeze_does_not() {
    let dir = tmp("reexec");
    let (files, statement) = reference_files().unwrap();
    write_package(&dir, files, &statement, "dev-test", "2026-10-25T00:00:00Z", &key()).unwrap();
    assert!(reexecute(&dir).unwrap());
    let path = dir.join("freeze.json");
    let original = fs::read_to_string(&path).unwrap();
    let mut doc: serde_json::Value = serde_json::from_str(&original).unwrap();
    doc["index"] = serde_json::json!(doc["index"].as_u64().unwrap() - 1);
    let text = serde_json::to_string_pretty(&doc).unwrap();
    assert_ne!(text, original, "the tamper must change the file");
    fs::write(&path, text).unwrap();
    assert!(!reexecute(&dir).unwrap_or(false));
}

#[test]
fn the_verdict_is_honest_about_validation() {
    let (_, statement) = reference_files().unwrap();
    assert_eq!(statement.label, "NotValidated");
    assert!(statement.context_of_use.contains("not a validated"));
    assert_eq!(statement.determinism, "Bitwise");
}

#[test]
fn refuses_to_write_into_a_non_empty_directory() {
    let dir = tmp("nonempty");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("old.txt"), b"x").unwrap();
    let (files, statement) = reference_files().unwrap();
    assert!(matches!(write_package(&dir, files, &statement, "dev", "t", &key()), Err(PackError::NotEmpty(_))));
}
