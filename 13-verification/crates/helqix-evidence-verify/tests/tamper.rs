//! Phase 5 tamper tests. Packages are written by hand here, independently of any builder.
use std::{fs, path::{Path, PathBuf}};

use ed25519_dalek::{Signer, SigningKey};
use helqix_evidence_verify::{merkle_root, sha256_hex, signed_message, verify_package, MANIFEST, SIGNATURE};
use serde_json::{json, Value};

fn key(seed: u8) -> SigningKey { SigningKey::from_bytes(&[seed; 32]) }
fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }
fn public(k: &SigningKey) -> String { hex(k.verifying_key().as_bytes()) }

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("helqix-ev-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

/// Writes a complete, consistent, signed package. `final_hash` drives the state chain.
fn write_package(dir: &Path, k: &SigningKey, final_hash: &str, statement_final: &str) {
    let files: Vec<(&str, &str, Value)> = vec![
        ("freeze.json", "freeze", json!({"index": 1793, "sha256": "f".repeat(64)})),
        ("requirement_outcome.json", "requirement_outcome", json!({"requirement": "stay_above_ground", "failure": {"index": 1793, "time_seconds": 1.793}})),
        ("state_chain.json", "state_chain", json!({"links": ["a".repeat(64), final_hash], "final": final_hash})),
        ("verdict.json", "verdict", json!({"label": "NotValidated", "grade": "model-causal"})),
    ];
    let mut entries = Vec::new();
    for (path, role, body) in &files {
        let bytes = serde_json::to_vec_pretty(body).unwrap();
        fs::write(dir.join(path), &bytes).unwrap();
        entries.push(json!({"path": path, "role": role, "sha256": sha256_hex(&bytes)}));
    }
    let pairs: Vec<(String, String)> = entries.iter().map(|e| (e["path"].as_str().unwrap().to_owned(), e["sha256"].as_str().unwrap().to_owned())).collect();
    let manifest = json!({
        "schema": "helqix.evidence-manifest.v1", "entries": entries, "merkle_root": merkle_root(&pairs),
        "statement": {"subject": "test", "determinism": "Bitwise", "context_of_use": "test only", "verdict": "test", "label": "NotValidated", "final_state_hash": statement_final, "failure_index": 1793},
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    fs::write(dir.join(MANIFEST), &manifest_bytes).unwrap();
    let manifest_sha = sha256_hex(&manifest_bytes);
    let sig = k.sign(&signed_message(&manifest_sha, "dev", "2026-10-25T00:00:00Z"));
    let signature = json!({"schema": "helqix.evidence-signature.v1", "algorithm": "Ed25519", "public_key": public(k), "key_id": &sha256_hex(k.verifying_key().as_bytes())[..16],
        "signer": "dev", "signed_at": "2026-10-25T00:00:00Z", "manifest_sha256": manifest_sha, "signature": hex(&sig.to_bytes())});
    fs::write(dir.join(SIGNATURE), serde_json::to_vec_pretty(&signature).unwrap()).unwrap();
}

fn good(name: &str) -> (PathBuf, SigningKey) {
    let d = tmp(name);
    let k = key(7);
    write_package(&d, &k, &"b".repeat(64), &"b".repeat(64));
    (d, k)
}

fn fails_with(dir: &Path, k: &SigningKey, needle: &str) {
    let r = verify_package(dir, &public(k));
    assert!(!r.pass, "expected FAIL containing {needle:?}");
    assert!(r.reasons.iter().any(|x| x.contains(needle)), "reasons {:?} do not mention {needle:?}", r.reasons);
}

#[test]
fn a_correct_package_passes() {
    let (d, k) = good("ok");
    let r = verify_package(&d, &public(&k));
    assert!(r.pass, "{:?}", r.reasons);
}

#[test]
fn one_changed_byte_fails() {
    let (d, k) = good("byte");
    let p = d.join("verdict.json");
    let mut b = fs::read(&p).unwrap();
    let i = b.iter().position(|c| *c == b'N').unwrap();
    b[i] = b'M';
    fs::write(&p, b).unwrap();
    fails_with(&d, &k, "hash mismatch");
}

#[test]
fn a_removed_file_fails() {
    let (d, k) = good("removed");
    fs::remove_file(d.join("freeze.json")).unwrap();
    fails_with(&d, &k, "missing file");
}

#[test]
fn an_unlisted_extra_file_fails() {
    let (d, k) = good("extra");
    fs::write(d.join("notes.txt"), b"added later").unwrap();
    fails_with(&d, &k, "unlisted file");
}

#[test]
fn a_reordered_manifest_fails() {
    let (d, k) = good("reorder");
    let mut m: Value = serde_json::from_slice(&fs::read(d.join(MANIFEST)).unwrap()).unwrap();
    m["entries"].as_array_mut().unwrap().reverse();
    fs::write(d.join(MANIFEST), serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    fails_with(&d, &k, "canonical order");
}

#[test]
fn a_replaced_signature_fails() {
    let (d, k) = good("sig");
    let mut s: Value = serde_json::from_slice(&fs::read(d.join(SIGNATURE)).unwrap()).unwrap();
    s["signature"] = json!(hex(&key(7).sign(b"something else").to_bytes()));
    fs::write(d.join(SIGNATURE), serde_json::to_vec_pretty(&s).unwrap()).unwrap();
    fails_with(&d, &k, "signature");
}

#[test]
fn a_package_signed_by_another_key_fails() {
    let (d, _) = good("wrongkey");
    fails_with(&d, &key(9), "not the trusted key");
}

#[test]
fn mixing_parts_of_two_valid_packages_fails() {
    let (a, k) = good("mix_a");
    let b = tmp("mix_b");
    write_package(&b, &k, &"c".repeat(64), &"c".repeat(64));
    assert!(verify_package(&b, &public(&k)).pass);
    fs::copy(b.join(MANIFEST), a.join(MANIFEST)).unwrap();
    fs::copy(b.join(SIGNATURE), a.join(SIGNATURE)).unwrap();
    fails_with(&a, &k, "hash mismatch");
}

#[test]
fn a_signed_but_internally_inconsistent_package_fails() {
    let d = tmp("inconsistent");
    let k = key(7);
    write_package(&d, &k, &"b".repeat(64), &"e".repeat(64));
    fails_with(&d, &k, "final state hash");
}

#[test]
fn merkle_root_depends_on_every_entry() {
    let e = vec![("a".to_owned(), "1".repeat(64)), ("b".to_owned(), "2".repeat(64)), ("c".to_owned(), "3".repeat(64))];
    let mut changed = e.clone();
    changed[2].1 = "4".repeat(64);
    assert_ne!(merkle_root(&e), merkle_root(&changed));
    assert_eq!(merkle_root(&e).len(), 64);
}

/// Tolerance-level package from an external tool: invocation record + raw tool output + verdict.
fn write_tool_package(dir: &Path, k: &SigningKey, recorded_output_hash: Option<&str>, drop_record: bool) {
    let raw = br#"{"tool_version": "26.10.1.1", "outputs": []}"#.to_vec();
    let record = json!({"tool": "pybamm", "tool_version": "26.10.1.1", "output_sha256": recorded_output_hash.map(str::to_owned).unwrap_or_else(|| sha256_hex(&raw)), "exit_status": 0});
    let mut files: Vec<(&str, &str, Vec<u8>)> = vec![
        ("tool_outputs.json", "tool_outputs", raw),
        ("verdict.json", "verdict", serde_json::to_vec_pretty(&json!({"label": "NotValidated"})).unwrap()),
    ];
    if !drop_record { files.insert(0, ("invocation_record.json", "invocation_record", serde_json::to_vec_pretty(&record).unwrap())); }
    let mut entries = Vec::new();
    for (path, role, bytes) in &files {
        fs::write(dir.join(path), bytes).unwrap();
        entries.push(json!({"path": path, "role": role, "sha256": sha256_hex(bytes)}));
    }
    let pairs: Vec<(String, String)> = entries.iter().map(|e| (e["path"].as_str().unwrap().to_owned(), e["sha256"].as_str().unwrap().to_owned())).collect();
    let manifest = json!({"schema": "helqix.evidence-manifest.v1", "entries": entries, "merkle_root": merkle_root(&pairs),
        "statement": {"subject": "tool run", "determinism": "Tolerance", "context_of_use": "test", "verdict": "test", "label": "NotValidated"}});
    let bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    fs::write(dir.join(MANIFEST), &bytes).unwrap();
    let sha = sha256_hex(&bytes);
    let sig = k.sign(&signed_message(&sha, "dev", "t"));
    fs::write(dir.join(SIGNATURE), serde_json::to_vec_pretty(&json!({"schema": "helqix.evidence-signature.v1", "algorithm": "Ed25519", "public_key": public(k),
        "key_id": &sha256_hex(k.verifying_key().as_bytes())[..16], "signer": "dev", "signed_at": "t", "manifest_sha256": sha, "signature": hex(&sig.to_bytes())})).unwrap()).unwrap();
}

#[test]
fn a_tolerance_tool_package_passes_with_its_own_required_roles() {
    let d = tmp("tool_ok");
    write_tool_package(&d, &key(7), None, false);
    let r = verify_package(&d, &public(&key(7)));
    assert!(r.pass, "{:?}", r.reasons);
}

#[test]
fn a_tolerance_package_without_its_invocation_record_fails() {
    let d = tmp("tool_norecord");
    write_tool_package(&d, &key(7), None, true);
    fails_with(&d, &key(7), "missing required role invocation_record");
}

#[test]
fn a_tool_output_that_differs_from_its_recorded_hash_fails() {
    let d = tmp("tool_badhash");
    write_tool_package(&d, &key(7), Some(&"0".repeat(64)), false);
    fails_with(&d, &key(7), "tool output");
}
