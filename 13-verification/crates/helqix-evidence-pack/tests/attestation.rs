//! K5 attestation tests: a signed in-toto statement over a real package verifies offline, and every
//! tampering is refused (changed, added or removed file; altered payload; wrong key; swapped package).
use std::{fs, path::{Path, PathBuf}};

use ed25519_dalek::{Signer, SigningKey};
use helqix_evidence_pack::{attest, public_key_hex, reference_files, write_package};
use helqix_evidence_verify::attestation::{b64_decode, b64_encode, envelope, statement, verify_attestation, PAYLOAD_TYPE};
use serde_json::Value;

fn key(seed: u8) -> SigningKey { SigningKey::from_bytes(&[seed; 32]) }

fn package(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("helqix-attest-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let (files, st) = reference_files().unwrap();
    write_package(&dir, files, &st, "test", "2026-10-26T00:00:00Z", &key(3)).unwrap();
    dir
}

fn verify(dir: &Path, env: &[u8], k: &SigningKey) -> (bool, Vec<String>) {
    let r = verify_attestation(dir, env, &public_key_hex(k));
    (r.pass, r.reasons)
}

#[test]
fn an_attestation_of_a_real_package_verifies() {
    let dir = package("ok");
    let env = attest(&dir, &key(3), "test-builder").unwrap();
    let (pass, reasons) = verify(&dir, &env, &key(3));
    assert!(pass, "{reasons:?}");
    let doc: Value = serde_json::from_slice(&env).unwrap();
    let st: Value = serde_json::from_slice(&b64_decode(doc["payload"].as_str().unwrap()).unwrap()).unwrap();
    assert_eq!(st["_type"], "https://in-toto.io/Statement/v1");
    let names: Vec<&str> = st["subject"].as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"manifest.json") && names.contains(&"signature.json") && names.contains(&"freeze.json"), "{names:?}");
    assert!(st["predicate"]["scope"].as_str().unwrap().contains("not correctness of the physics"));
}

#[test]
fn a_changed_added_or_removed_file_fails() {
    let env = attest(&package("base"), &key(3), "b").unwrap();
    let changed = package("changed");
    let env_c = attest(&changed, &key(3), "b").unwrap();
    let mut v: Value = serde_json::from_slice(&fs::read(changed.join("verdict.json")).unwrap()).unwrap();
    v["label"] = "Validated".into();
    fs::write(changed.join("verdict.json"), serde_json::to_vec_pretty(&v).unwrap()).unwrap();
    let (pass, reasons) = verify(&changed, &env_c, &key(3));
    assert!(!pass && reasons.iter().any(|r| r.contains("hash differs") && r.contains("verdict.json")), "{reasons:?}");

    let added = package("added");
    let env_a = attest(&added, &key(3), "b").unwrap();
    fs::write(added.join("notes.txt"), b"added later").unwrap();
    assert!(!verify(&added, &env_a, &key(3)).0);

    let removed = package("removed");
    let env_r = attest(&removed, &key(3), "b").unwrap();
    fs::remove_file(removed.join("freeze.json")).unwrap();
    assert!(!verify(&removed, &env_r, &key(3)).0);
    let _ = env;
}

#[test]
fn an_altered_payload_or_payload_type_breaks_the_signature() {
    let dir = package("payload");
    let env = attest(&dir, &key(3), "b").unwrap();
    let mut doc: Value = serde_json::from_slice(&env).unwrap();
    let mut st: Value = serde_json::from_slice(&b64_decode(doc["payload"].as_str().unwrap()).unwrap()).unwrap();
    st["predicate"]["builder"] = "someone else".into();
    doc["payload"] = b64_encode(&serde_json::to_vec(&st).unwrap()).into();
    let (pass, reasons) = verify(&dir, &serde_json::to_vec(&doc).unwrap(), &key(3));
    assert!(!pass && reasons.iter().any(|r| r.contains("no signature")), "{reasons:?}");

    let mut doc: Value = serde_json::from_slice(&env).unwrap();
    doc["payloadType"] = "application/json".into();
    assert!(!verify(&dir, &serde_json::to_vec(&doc).unwrap(), &key(3)).0);
}

#[test]
fn the_wrong_key_or_another_package_fails() {
    let a = package("pkg-a");
    let env = attest(&a, &key(3), "b").unwrap();
    assert!(!verify(&a, &env, &key(9)).0, "trusting a different key");
    // A correctly formed envelope signed by an untrusted key.
    let forged = envelope(&statement(&a, "b").unwrap(), "x", |m| key(9).sign(m).to_bytes().to_vec());
    assert!(!verify(&a, &serde_json::to_vec(&forged).unwrap(), &key(3)).0);
    // The attestation of package A does not cover package B (different signer, so different files).
    let b = std::env::temp_dir().join(format!("helqix-attest-{}-pkg-b", std::process::id()));
    let _ = fs::remove_dir_all(&b);
    let (files, st) = reference_files().unwrap();
    write_package(&b, files, &st, "other signer", "2026-10-26T00:00:00Z", &key(3)).unwrap();
    let (pass, reasons) = verify(&b, &env, &key(3));
    assert!(!pass && reasons.iter().any(|r| r.contains("signature.json")), "{reasons:?}");
    assert_eq!(PAYLOAD_TYPE, "application/vnd.in-toto+json");
}
