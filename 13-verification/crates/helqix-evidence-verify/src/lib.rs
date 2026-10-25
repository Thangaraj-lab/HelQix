//! Independent, offline verifier for HELQIX evidence packages.
//!
//! This crate deliberately imports no HELQIX runtime crate: it re-checks a package from its
//! files alone. A package is a directory with:
//!
//! * `manifest.json` (`helqix.evidence-manifest.v1`): `entries` (path, role, SHA-256), sorted by
//!   path; `merkle_root` over the entries; `statement` (subject, context of use, determinism,
//!   verdict, label, final state hash, failure index).
//! * `signature.json` (`helqix.evidence-signature.v1`): Ed25519 signature over
//!   [`signed_message`] = `helqix.evidence.v1\n<manifest sha256>\n<signer>\n<signed_at>`.
//! * every listed file, and nothing else.
//!
//! Merkle tree: leaf = SHA-256(`helqix.leaf\0` + path + `\0` + file hash); parent =
//! SHA-256(`helqix.node\0` + left hex + right hex); an odd last node is promoted unchanged.
//!
//! **What a PASS means:** the files are exactly those signed by the holder of the trusted key, and
//! the package is internally consistent. **It does not mean the physics is correct.** Correctness
//! claims come from validation evidence inside the package (or its absence). `signed_at` is a
//! claim by the signer, not a proven time.
#![forbid(unsafe_code)]

pub mod attestation;

use std::{collections::BTreeSet, fs, path::Path};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Manifest file name.
pub const MANIFEST: &str = "manifest.json";
/// Signature file name.
pub const SIGNATURE: &str = "signature.json";
/// Roles a Bitwise (core-runtime) package must contain.
pub const REQUIRED_ROLES: [&str; 4] = ["freeze", "state_chain", "requirement_outcome", "verdict"];
/// Roles a Tolerance or Statistical (external-tool) package must contain.
pub const TOOL_REQUIRED_ROLES: [&str; 3] = ["invocation_record", "tool_outputs", "verdict"];
/// Statement printed with every result.
pub const SCOPE_NOTE: &str = "Signing proves origin and integrity of the package, not correctness of the physics.";

/// Verification outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyReport { pub pass: bool, pub checks: Vec<(String, bool)>, pub reasons: Vec<String> }

fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }

/// Lower-case hex SHA-256.
pub fn sha256_hex(data: &[u8]) -> String { hex(&Sha256::digest(data)) }

/// Merkle root over (path, file hash) entries in the given order.
pub fn merkle_root(entries: &[(String, String)]) -> String {
    let mut level: Vec<String> = entries.iter().map(|(path, h)| sha256_hex(format!("helqix.leaf\0{path}\0{h}").as_bytes())).collect();
    if level.is_empty() { return sha256_hex(b"helqix.empty"); }
    while level.len() > 1 {
        level = level.chunks(2).map(|pair| match pair { [l, r] => sha256_hex(format!("helqix.node\0{l}{r}").as_bytes()), [one] => one.clone(), _ => unreachable!() }).collect();
    }
    level.remove(0)
}

/// Exact bytes covered by the signature.
pub fn signed_message(manifest_sha256: &str, signer: &str, signed_at: &str) -> Vec<u8> {
    format!("helqix.evidence.v1\n{manifest_sha256}\n{signer}\n{signed_at}").into_bytes()
}

/// A trusted Ed25519 public key from hex (used by the attestation check).
pub(crate) fn unhex_pub(s: &str) -> Option<[u8; 32]> { unhex(s.trim())?.try_into().ok() }

fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 { return None; }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

fn read_json(path: &Path) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let value = serde_json::from_slice(&bytes).map_err(|e| format!("{} is not valid JSON: {e}", path.display()))?;
    Ok((bytes, value))
}

/// Verifies a package directory against a trusted Ed25519 public key (hex).
pub fn verify_package(dir: &Path, trusted_public_key_hex: &str) -> VerifyReport {
    let mut checks: Vec<(String, bool)> = Vec::new();
    let mut reasons: Vec<String> = Vec::new();
    let mut check = |name: &str, ok: bool, reason: String, reasons: &mut Vec<String>| { checks.push((name.to_owned(), ok)); if !ok { reasons.push(reason); } };

    let (manifest_bytes, manifest) = match read_json(&dir.join(MANIFEST)) { Ok(x) => x, Err(e) => return VerifyReport { pass: false, checks: vec![("manifest".into(), false)], reasons: vec![e] } };
    let (_, signature) = match read_json(&dir.join(SIGNATURE)) { Ok(x) => x, Err(e) => return VerifyReport { pass: false, checks: vec![("signature".into(), false)], reasons: vec![e] } };
    check("manifest schema", manifest["schema"] == "helqix.evidence-manifest.v1", "manifest schema is not helqix.evidence-manifest.v1".into(), &mut reasons);

    let entries: Vec<(String, String, String)> = manifest["entries"].as_array().map(|a| a.iter().map(|e| (
        e["path"].as_str().unwrap_or_default().to_owned(), e["role"].as_str().unwrap_or_default().to_owned(), e["sha256"].as_str().unwrap_or_default().to_owned())).collect()).unwrap_or_default();
    let paths: Vec<&str> = entries.iter().map(|e| e.0.as_str()).collect();
    check("entries in canonical order", paths.windows(2).all(|w| w[0] < w[1]), "manifest entries are not in canonical order (sorted by path, unique)".into(), &mut reasons);
    let safe = paths.iter().all(|p| !p.is_empty() && !p.contains(['/', '\\']) && *p != ".." && *p != "." && *p != MANIFEST && *p != SIGNATURE);
    check("entry paths are plain file names", safe, "an entry path is not a plain file name inside the package".into(), &mut reasons);

    let roles: BTreeSet<&str> = entries.iter().map(|e| e.1.as_str()).collect();
    let determinism = manifest["statement"]["determinism"].as_str().unwrap_or_default().to_owned();
    let required: &[&str] = match determinism.as_str() {
        "Bitwise" => &REQUIRED_ROLES,
        "Tolerance" | "Statistical" => &TOOL_REQUIRED_ROLES,
        _ => { check("determinism declared", false, "statement must declare determinism Bitwise, Tolerance or Statistical".into(), &mut reasons); &[] }
    };
    for role in required { check(&format!("role {role} present"), roles.contains(role), format!("missing required role {role}"), &mut reasons); }

    for (path, _, expected) in &entries {
        match fs::read(dir.join(path)) {
            Ok(bytes) => { let ok = &sha256_hex(&bytes) == expected; check(&format!("hash {path}"), ok, format!("hash mismatch for {path}"), &mut reasons); }
            Err(_) => check(&format!("file {path}"), false, format!("missing file {path}"), &mut reasons),
        }
    }
    let listed: BTreeSet<&str> = paths.iter().copied().chain([MANIFEST, SIGNATURE]).collect();
    if let Ok(read) = fs::read_dir(dir) {
        for item in read.flatten() {
            let name = item.file_name().to_string_lossy().into_owned();
            if !listed.contains(name.as_str()) { check(&format!("unlisted {name}"), false, format!("unlisted file {name}"), &mut reasons); }
        }
    }
    let pairs: Vec<(String, String)> = entries.iter().map(|e| (e.0.clone(), e.2.clone())).collect();
    check("Merkle root", manifest["merkle_root"].as_str() == Some(merkle_root(&pairs).as_str()), "Merkle root mismatch".into(), &mut reasons);

    // Signature.
    let manifest_sha = sha256_hex(&manifest_bytes);
    let public = signature["public_key"].as_str().unwrap_or_default().to_ascii_lowercase();
    check("trusted key", public == trusted_public_key_hex.to_ascii_lowercase(), "signing key is not the trusted key".into(), &mut reasons);
    check("signature covers this manifest", signature["manifest_sha256"].as_str() == Some(manifest_sha.as_str()), "signature covers a different manifest".into(), &mut reasons);
    let verified = (|| -> Option<bool> {
        if signature["algorithm"] != "Ed25519" { return Some(false); }
        let key: [u8; 32] = unhex(trusted_public_key_hex)?.try_into().ok()?;
        let sig: [u8; 64] = unhex(signature["signature"].as_str()?)?.try_into().ok()?;
        let key = VerifyingKey::from_bytes(&key).ok()?;
        let message = signed_message(&manifest_sha, signature["signer"].as_str()?, signature["signed_at"].as_str()?);
        Some(key.verify(&message, &Signature::from_bytes(&sig)).is_ok())
    })().unwrap_or(false);
    check("Ed25519 signature", verified, "signature does not verify with the trusted key".into(), &mut reasons);

    // Internal consistency.
    let load = |role: &str| entries.iter().find(|e| e.1 == role).and_then(|e| read_json(&dir.join(&e.0)).ok()).map(|x| x.1);
    let statement = &manifest["statement"];
    if determinism != "Bitwise" {
        let record = load("invocation_record").unwrap_or(Value::Null);
        let raw = entries.iter().find(|e| e.1 == "tool_outputs").and_then(|e| fs::read(dir.join(&e.0)).ok());
        let ok = raw.is_some_and(|b| record["output_sha256"].as_str() == Some(sha256_hex(&b).as_str()));
        check("tool output matches its invocation record", ok, "tool output does not match the hash in its invocation record".into(), &mut reasons);
        return VerifyReport { pass: reasons.is_empty(), checks, reasons };
    }
    let chain = load("state_chain").unwrap_or(Value::Null);
    let last_link = chain["links"].as_array().and_then(|l| l.last()).and_then(Value::as_str);
    let final_ok = chain["final"].as_str().is_some() && chain["final"].as_str() == last_link && chain["final"] == statement["final_state_hash"];
    check("final state hash consistent", final_ok, "final state hash in the state chain does not match the chain end or the statement".into(), &mut reasons);
    let outcome = load("requirement_outcome").unwrap_or(Value::Null);
    let freeze = load("freeze").unwrap_or(Value::Null);
    let index_ok = outcome["failure"]["index"].is_u64() && outcome["failure"]["index"] == statement["failure_index"] && freeze["index"] == statement["failure_index"];
    check("failure index consistent", index_ok, "failure index differs between requirement outcome, freeze and statement".into(), &mut reasons);

    VerifyReport { pass: reasons.is_empty(), checks, reasons }
}
