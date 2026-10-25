//! in-toto attestation for an evidence package (K5), verified offline.
//!
//! An attestation is an in-toto Statement v1 in a DSSE envelope (payload type
//! `application/vnd.in-toto+json`), signed with Ed25519 over the DSSE pre-authentication encoding.
//! Its subjects are every file of the package (name and SHA-256), so the attestation binds the exact
//! package. It is stored beside the package, not inside it, so the package's file list is unchanged.
//! The predicate records the package's Merkle root and the scope note. No network, no HELQIX runtime.
use std::{fs, path::Path};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde_json::Value;

use crate::{sha256_hex, VerifyReport, MANIFEST, SCOPE_NOTE};

pub const STATEMENT_TYPE: &str = "https://in-toto.io/Statement/v1";
pub const PAYLOAD_TYPE: &str = "application/vnd.in-toto+json";
pub const PREDICATE_TYPE: &str = "https://helqix.dev/attestations/evidence-package/v1";

/// DSSE pre-authentication encoding: `DSSEv1 <len(type)> <type> <len(payload)> <payload>`.
pub fn pae(payload_type: &str, payload: &[u8]) -> Vec<u8> {
    let mut out = format!("DSSEv1 {} {} {} ", payload_type.len(), payload_type, payload.len()).into_bytes();
    out.extend_from_slice(payload);
    out
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding (RFC 4648 section 4), as DSSE uses.
pub fn b64_encode(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = (u32::from(chunk[0]) << 16) | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8) | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= chunk.len() { s.push(B64[((n >> (18 - 6 * i)) & 63) as usize] as char); } else { s.push('='); }
        }
    }
    s
}

pub fn b64_decode(text: &str) -> Option<Vec<u8>> {
    let t = text.trim().as_bytes();
    if t.len() % 4 != 0 { return None; }
    let val = |c: u8| B64.iter().position(|&b| b == c).map(|p| p as u32);
    let mut out = Vec::with_capacity(t.len() / 4 * 3);
    for (ci, chunk) in t.chunks(4).enumerate() {
        let last = ci == t.len() / 4 - 1;
        let pad = chunk.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 || (pad > 0 && !last) { return None; }
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            let v = if i >= 4 - pad { 0 } else { val(c)? };
            n |= v << (18 - 6 * i);
        }
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&bytes[..3 - pad]);
    }
    Some(out)
}

/// Every file of a package as `(name, sha256)`, sorted by name.
pub fn package_subjects(dir: &Path) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_file() { return Err(format!("unexpected non-file {}", entry.path().display())); }
        let name = entry.file_name().into_string().map_err(|_| "non-UTF-8 file name".to_owned())?;
        out.push((name, sha256_hex(&fs::read(entry.path()).map_err(|e| e.to_string())?)));
    }
    out.sort();
    Ok(out)
}

/// The in-toto statement for a package (canonical JSON bytes are what gets signed).
pub fn statement(dir: &Path, builder: &str) -> Result<Value, String> {
    let manifest: Value = serde_json::from_slice(&fs::read(dir.join(MANIFEST)).map_err(|e| format!("cannot read manifest: {e}"))?).map_err(|e| e.to_string())?;
    let subjects: Vec<Value> = package_subjects(dir)?.into_iter().map(|(name, sha)| serde_json::json!({"name": name, "digest": {"sha256": sha}})).collect();
    Ok(serde_json::json!({
        "_type": STATEMENT_TYPE,
        "subject": subjects,
        "predicateType": PREDICATE_TYPE,
        "predicate": {
            "builder": builder,
            "merkleRoot": manifest["merkle_root"],
            "statement": manifest["statement"],
            "scope": SCOPE_NOTE,
        },
    }))
}

/// Builds a signed DSSE envelope from a statement, given a signing function over the PAE bytes.
pub fn envelope(statement: &Value, keyid: &str, sign: impl FnOnce(&[u8]) -> Vec<u8>) -> Value {
    let payload = serde_json::to_vec(statement).expect("statement serializes");
    let sig = sign(&pae(PAYLOAD_TYPE, &payload));
    serde_json::json!({"payloadType": PAYLOAD_TYPE, "payload": b64_encode(&payload), "signatures": [{"keyid": keyid, "sig": b64_encode(&sig)}]})
}

/// Verifies an attestation envelope against a package directory and a trusted Ed25519 key (hex).
pub fn verify_attestation(dir: &Path, envelope_bytes: &[u8], trusted_public_key_hex: &str) -> VerifyReport {
    let mut checks = Vec::new();
    let mut reasons = Vec::new();
    let mut check = |name: &str, ok: bool, why: String| { checks.push((name.to_owned(), ok)); if !ok { reasons.push(why); } };
    let env: Value = match serde_json::from_slice(envelope_bytes) { Ok(v) => v, Err(e) => return VerifyReport { pass: false, checks: vec![("attestation is JSON".into(), false)], reasons: vec![e.to_string()] } };
    check("payload type is in-toto", env["payloadType"] == PAYLOAD_TYPE, format!("payload type is {}", env["payloadType"]));
    let payload = env["payload"].as_str().and_then(b64_decode);
    check("payload is base64", payload.is_some(), "payload is not valid base64".into());
    let Some(payload) = payload else { return VerifyReport { pass: false, checks, reasons } };
    let key = crate::unhex_pub(trusted_public_key_hex).and_then(|k| VerifyingKey::from_bytes(&k).ok());
    check("trusted key parses", key.is_some(), "trusted key is not a 32-byte Ed25519 public key in hex".into());
    let message = pae(env["payloadType"].as_str().unwrap_or_default(), &payload);
    let signed = key.is_some_and(|k| env["signatures"].as_array().is_some_and(|sigs| sigs.iter().any(|s| {
        s["sig"].as_str().and_then(b64_decode).and_then(|b| <[u8; 64]>::try_from(b).ok()).is_some_and(|b| k.verify(&message, &Signature::from_bytes(&b)).is_ok())
    })));
    check("envelope signed by the trusted key", signed, "no signature in the envelope verifies with the trusted key".into());
    let st: Value = serde_json::from_slice(&payload).unwrap_or(Value::Null);
    check("statement is in-toto v1", st["_type"] == STATEMENT_TYPE, format!("statement type is {}", st["_type"]));
    check("predicate type", st["predicateType"] == PREDICATE_TYPE, format!("predicate type is {}", st["predicateType"]));
    match package_subjects(dir) {
        Ok(actual) => {
            let attested: Vec<(String, String)> = st["subject"].as_array().map(|a| a.iter().map(|s| (s["name"].as_str().unwrap_or_default().to_owned(), s["digest"]["sha256"].as_str().unwrap_or_default().to_owned())).collect()).unwrap_or_default();
            let mut sorted = attested.clone();
            sorted.sort();
            check("attested files equal the package files", sorted == actual, describe_difference(&sorted, &actual));
        }
        Err(e) => check("package readable", false, e),
    }
    let merkle = fs::read(dir.join(MANIFEST)).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).map(|m| m["merkle_root"].clone());
    check("attested Merkle root equals the manifest", merkle.as_ref() == Some(&st["predicate"]["merkleRoot"]), "attested Merkle root differs from the package manifest".into());
    check("scope note present", st["predicate"]["scope"] == SCOPE_NOTE, "the attestation does not carry the scope note".into());
    VerifyReport { pass: reasons.is_empty(), checks, reasons }
}

fn describe_difference(attested: &[(String, String)], actual: &[(String, String)]) -> String {
    let missing: Vec<&str> = actual.iter().filter(|a| !attested.iter().any(|b| b.0 == a.0)).map(|a| a.0.as_str()).collect();
    let extra: Vec<&str> = attested.iter().filter(|a| !actual.iter().any(|b| b.0 == a.0)).map(|a| a.0.as_str()).collect();
    let changed: Vec<&str> = actual.iter().filter(|a| attested.iter().any(|b| b.0 == a.0 && b.1 != a.1)).map(|a| a.0.as_str()).collect();
    format!("files not attested: {missing:?}; attested but absent: {extra:?}; hash differs: {changed:?}")
}
