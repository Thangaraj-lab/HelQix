//! Evidence export (E8.2): `POST /v1/runs/{id}/evidence` builds a signed evidence package for a stored run of the built-in engine and returns it
//! as a ZIP; `GET /v1/evidence/key` says which key signs.
//!
//! The server builds and signs (the browser never holds a key). The package is made by `helqix-evidence-pack` and checked by the independent
//! offline verifier (`helqix-evidence-verify`): a person unzips it and runs the verifier with the public key, no network and no HELQIX service.
//! A run is re-executed from its stored parameters and must end on the stored final state hash; a run that a re-run does not reproduce gets no evidence.
//!
//! **Key:** one Ed25519 key read from a file (`HELQIX_EVIDENCE_KEY_FILE`, 64 hex characters, the seed). Until the founder decides key custody (F4) it
//! is a test key, and the answer says so (`HELQIX_EVIDENCE_KEY_KIND`, default `test`). The private key never appears in a response or a log.
//! `signed_at` is the server's clock at export: a claim by the signer, not a trusted time stamp.
//!
//! **Who:** the operator's key may export any run. A user may export only a run they own (one they started, or one made from their project);
//! another user's run answers 404, so its existence is not revealed.
use std::path::PathBuf;

use axum::{Json, extract::{Extension, Path, State}, http::{HeaderValue, header}, response::{IntoResponse, Response}};
use ed25519_dalek::SigningKey;
use helqix_evidence_pack::{Bound, Declared, PackError, attest, public_key_hex, run_files, write_package};
use helqix_frames::FrameStore;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{auth::Identity, error::ApiError, frames::{map, to_spec}, specs::StoredParams, state::AppState, zipstore::zip_stored};

/// The signing key and how it is described to people who receive a package.
#[derive(Clone)]
pub struct EvidenceConfig { key: SigningKey, signer: String, kind: String }

impl std::fmt::Debug for EvidenceConfig {
    // Never print the key.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("EvidenceConfig").field("signer", &self.signer).field("kind", &self.kind).finish_non_exhaustive() }
}

fn unhex32(s: &str) -> Option<[u8; 32]> {
    let s = s.trim();
    if s.len() != 64 || !s.is_ascii() { return None; }
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() { *slot = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()?; }
    Some(out)
}

impl EvidenceConfig {
    pub fn new(seed: [u8; 32], signer: &str, kind: &str) -> Self { Self { key: SigningKey::from_bytes(&seed), signer: signer.to_owned(), kind: kind.to_owned() } }

    /// `HELQIX_EVIDENCE_KEY_FILE` turns export on (`HELQIX_EVIDENCE_SIGNER`, default `helqix-evidence-service`; `HELQIX_EVIDENCE_KEY_KIND`, default `test`);
    /// without it export answers 503. A file that is not 64 hex characters is an error, so a typo cannot silently leave export off.
    pub fn from_vars(get: impl Fn(&str) -> Option<String>) -> Result<Option<Self>, String> {
        let Some(path) = get("HELQIX_EVIDENCE_KEY_FILE") else { return Ok(None) };
        let text = std::fs::read_to_string(&path).map_err(|e| format!("could not read the evidence key file: {e}"))?;
        let seed = unhex32(&text).ok_or("the evidence key file must hold exactly 64 hex characters (a 32-byte Ed25519 seed)")?;
        let signer = get("HELQIX_EVIDENCE_SIGNER").unwrap_or_else(|| "helqix-evidence-service".into());
        let kind = get("HELQIX_EVIDENCE_KEY_KIND").unwrap_or_else(|| "test".into());
        // The name travels in a response header, so it is plain printable ASCII.
        if signer.is_empty() || signer.len() > 120 || !signer.chars().all(|c| c.is_ascii_graphic() || c == ' ') { return Err("HELQIX_EVIDENCE_SIGNER must be 1 to 120 printable ASCII characters on one line".into()); }
        if !matches!(kind.as_str(), "test" | "production") { return Err("HELQIX_EVIDENCE_KEY_KIND must be test or production".into()); }
        Ok(Some(Self::new(seed, &signer, &kind)))
    }

    fn key_id(&self) -> String {
        let digest = Sha256::digest(self.key.verifying_key().as_bytes());
        digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
    }
}

/// `GET /v1/evidence/key`: the public key a verifier must be told to trust, and what kind of key it is.
pub async fn key(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let cfg = state.evidence.as_ref().ok_or(ApiError::Unavailable)?;
    let note = if cfg.kind == "test" { "A test key held by the operator. A package signed with it shows origin and integrity for testing; it is not a production signature." } else { "The operator's production signing key." };
    Ok(Json(json!({"publicKey": public_key_hex(&cfg.key), "keyId": cfg.key_id(), "signer": cfg.signer, "keyKind": cfg.kind, "algorithm": "Ed25519", "note": note})))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Cmp { AtLeast, AtMost, Above, Below }

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RequirementBody { name: String, variable: String, comparator: Cmp, threshold: f64 }

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExportBody { requirement: RequirementBody, #[serde(default)] subject: Option<String> }

fn one_line(s: &str, max: usize) -> bool { !s.trim().is_empty() && s.chars().count() <= max && !s.contains(['\n', '\r', '\0']) }

fn pack_error(e: PackError) -> ApiError {
    match e {
        PackError::Build(m) => ApiError::StateConflict(m),
        other => { tracing::error!(error = %other, "evidence package could not be written"); ApiError::Internal }
    }
}

const README: &str = "HELQIX evidence package\n\n\
This archive holds a signed evidence package for one recorded run, its in-toto attestation and the public key it was signed with.\n\n\
To check it, with no network and no HELQIX service (the verifier is `helqix-evidence-verify`):\n\n\
  helqix-evidence-verify <package folder> --trusted-key-file trusted-public-key.txt --attestation <package folder>.intoto.dsse.json\n\n\
PASS means the files are exactly the ones the holder of that key signed and the package is consistent with itself. It does not mean the physics is right:\n\
the verdict inside says what was and was not validated. The signing time is a claim by the signer, not a trusted time stamp.\n";

/// `POST /v1/runs/{id}/evidence`: the signed package for a stored run of the built-in engine, with the requirement the caller declares.
pub async fn export(State(state): State<AppState>, Extension(who): Extension<Identity>, Path(id): Path<String>, Json(body): Json<ExportBody>) -> Result<Response, ApiError> {
    let cfg = state.evidence.clone().ok_or(ApiError::Unavailable)?;
    let dir = state.frame_store.clone().ok_or(ApiError::Unavailable)?;
    let specs = state.spec_store().ok_or(ApiError::Unavailable)?;
    if !who.is_service() && !specs.visible_to(who.owner()).await?.contains(&id) { return Err(ApiError::NotFound); }

    // One export at a time per slot: each re-runs the simulation, reads the frames and signs.
    let _slot = state.evidence_gate.clone().try_acquire_owned().map_err(|_| ApiError::Busy)?;
    let r = &body.requirement;
    if !one_line(&r.name, 120) || !one_line(&r.variable, 80) { return Err(ApiError::BadRequest("the requirement needs a name (at most 120 characters) and a variable, each on one line".into())); }
    if !r.threshold.is_finite() { return Err(ApiError::BadRequest("the threshold must be a finite number".into())); }
    let subject = match &body.subject { Some(s) if one_line(s, 200) => s.trim().to_owned(), Some(_) => return Err(ApiError::BadRequest("the subject must be one line of at most 200 characters".into())), None => format!("recorded run {id}") };

    let stored = match specs.load(&id).await {
        Ok(s) => s,
        // No stored parameters: either there is no such run (404) or it was stored without them, so a re-run cannot be checked against it.
        Err(ApiError::Rejected(_)) => {
            let (d, i) = (dir.clone(), id.clone());
            tokio::task::spawn_blocking(move || FrameStore::open(d).and_then(|s| s.load(&i)).map(|_| ())).await.map_err(|_| ApiError::Internal)?.map_err(map)?;
            return Err(ApiError::Rejected("This run has no stored parameters, so it cannot be re-run to check it, and no evidence is issued for it.".into()));
        }
        Err(e) => return Err(e),
    };
    let StoredParams::PointMass(params) = stored.params else {
        return Err(ApiError::Rejected("Evidence export covers runs of the built-in engine. Runs made with an FMU or the battery pack are not covered yet.".into()));
    };
    if let Some(m) = &state.mirror { m.ensure(&dir, &id).await?; }

    let (run_id, bound, req) = (id.clone(), match r.comparator { Cmp::AtLeast => Bound::AtLeast, Cmp::AtMost => Bound::AtMost, Cmp::Above => Bound::Above, Cmp::Below => Bound::Below }, (r.name.clone(), r.variable.clone(), r.threshold));
    let signed_by = SignedBy { key_id: cfg.key_id(), public_key: public_key_hex(&cfg.key), kind: cfg.kind.clone(), signer: cfg.signer.clone() };
    let zip = tokio::task::spawn_blocking(move || build(&cfg, &dir, &run_id, &to_spec(&params), bound, req, &subject)).await.map_err(|_| ApiError::Internal)??;

    let disposition = HeaderValue::from_str(&format!("attachment; filename=\"evidence-{id}.zip\"")).map_err(|_| ApiError::Internal)?;
    // The key that signed this very file travels with it, so a reader never has to ask the service afterwards (the service's key could have changed).
    let h = |s: &str| HeaderValue::from_str(s).map_err(|_| ApiError::Internal);
    let headers = [
        (header::CONTENT_TYPE, HeaderValue::from_static("application/zip")),
        (header::CONTENT_DISPOSITION, disposition),
        (axum::http::HeaderName::from_static("x-helqix-key-id"), h(&signed_by.key_id)?),
        (axum::http::HeaderName::from_static("x-helqix-public-key"), h(&signed_by.public_key)?),
        (axum::http::HeaderName::from_static("x-helqix-key-kind"), h(&signed_by.kind)?),
        (axum::http::HeaderName::from_static("x-helqix-signer"), h(&signed_by.signer)?),
    ];
    Ok((headers, zip).into_response())
}

/// Who signed, as it is sent beside the file.
struct SignedBy { key_id: String, public_key: String, kind: String, signer: String }

struct TempDir(PathBuf);
impl Drop for TempDir { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

fn build(cfg: &EvidenceConfig, frames_dir: &std::path::Path, run_id: &str, spec: &helqix_freeze::RunSpec, bound: Bound, req: (String, String, f64), subject: &str) -> Result<Vec<u8>, ApiError> {
    let store = FrameStore::open(frames_dir).map_err(map)?;
    let run = store.load(run_id).map_err(map)?;
    let (name, variable, threshold) = req;
    let Some(index) = run.meta.variables.iter().position(|v| v.name == variable) else {
        let known: Vec<&str> = run.meta.variables.iter().map(|v| v.name.as_str()).collect();
        return Err(ApiError::Rejected(format!("this run has no variable {variable:?}; its variables are {}", known.join(", "))));
    };
    let frames = std::fs::read(store.frames_path(&run.frames_blake3)).map_err(|e| { tracing::error!(error = %e, "stored frames could not be read"); ApiError::Internal })?;
    let declared = Declared { name, variable: index, variable_name: variable, unit: run.meta.variables[index].unit.clone(), bound, threshold };
    let (files, statement) = run_files(spec, &declared, &run.meta.final_link, frames, subject).map_err(pack_error)?;

    let temp = TempDir(std::env::temp_dir().join(format!("helqix-evidence-{}", uuid::Uuid::new_v4())));
    let signed_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    write_package(&temp.0, files, &statement, &cfg.signer, &signed_at, &cfg.key).map_err(pack_error)?;
    let attestation = attest(&temp.0, &cfg.key, "helqix-api evidence export").map_err(pack_error)?;

    let folder = format!("evidence-{run_id}");
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut names: Vec<String> = std::fs::read_dir(&temp.0).map_err(|_| ApiError::Internal)?.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    for n in names { entries.push((format!("{folder}/{n}"), std::fs::read(temp.0.join(&n)).map_err(|_| ApiError::Internal)?)); }
    entries.push((format!("{folder}.intoto.dsse.json"), attestation));
    entries.push(("trusted-public-key.txt".into(), format!("{}\n", public_key_hex(&cfg.key)).into_bytes()));
    entries.push(("README.txt".into(), README.as_bytes().to_vec()));
    zip_stored(&entries).map_err(|e| { tracing::error!(error = %e, "evidence archive could not be written"); ApiError::Internal })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_file_must_be_64_hex_characters_and_the_key_is_never_printed() {
        let seed = "0a".repeat(32);
        let dir = std::env::temp_dir().join(format!("helqix-evkey-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let good = dir.join("good.txt");
        std::fs::write(&good, format!("{seed}\n")).unwrap();
        let get = |p: String| move |k: &str| (k == "HELQIX_EVIDENCE_KEY_FILE").then(|| p.clone());
        let cfg = EvidenceConfig::from_vars(get(good.display().to_string())).unwrap().unwrap();
        assert_eq!(cfg.kind, "test");
        assert!(!format!("{cfg:?}").contains(&seed), "the seed must not appear in Debug output");
        for bad in ["", "zz", &"0a".repeat(31), &"0a".repeat(33), &"zz".repeat(32)] {
            let f = dir.join("bad.txt");
            std::fs::write(&f, bad).unwrap();
            assert!(EvidenceConfig::from_vars(get(f.display().to_string())).is_err(), "{bad:?}");
        }
        assert!(EvidenceConfig::from_vars(|_| None).unwrap().is_none(), "no variable: export is off");
        let get_signer = |signer: &'static str, p: String| move |k: &str| match k { "HELQIX_EVIDENCE_KEY_FILE" => Some(p.clone()), "HELQIX_EVIDENCE_SIGNER" => Some(signer.to_owned()), _ => None };
        assert!(EvidenceConfig::from_vars(get_signer("Acme Signing 1", good.display().to_string())).is_ok());
        for bad in ["naïve", "two\nlines", "", "tab\there"] { assert!(EvidenceConfig::from_vars(get_signer(bad, good.display().to_string())).is_err(), "{bad:?}"); }
        assert!(EvidenceConfig::from_vars(get(dir.join("missing.txt").display().to_string())).is_err());
    }
}
