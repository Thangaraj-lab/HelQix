//! E8.2: a stored run becomes a signed evidence package (a ZIP) that the independent offline verifier passes; users export only runs they own.
//! Needs HELQIX_TEST_DATABASE_URL (prints SKIPPED otherwise). The key is a fixed test seed, never a real key.
use std::collections::HashMap;

use aws_lc_rs::{encoding::AsDer, rsa::{KeyPair, KeySize}, signature::KeyPair as _};
use axum::{Router, body::Body, http::{Request, StatusCode}};
use helqix_api::{auth::Oidc, evidence_export::EvidenceConfig, routes, state::AppState};
use helqix_evidence_verify::verify_package;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, encode};
use serde_json::{Value, json};
use tower::ServiceExt;

const KEY: &str = "test-api-key-not-a-secret-0123456789";
const ISS: &str = "https://sso.example.test/realms/helqix";
const AUD: &str = "helqix-api";
const SEED: [u8; 32] = [9; 32];

fn der(b: &[u8]) -> (u8, &[u8], &[u8]) {
    let (tag, first) = (b[0], b[1]);
    let (len, off) = if first < 0x80 { (first as usize, 2) } else { let n = (first & 0x7f) as usize; (b[2..2 + n].iter().fold(0usize, |a, x| (a << 8) | *x as usize), 2 + n) };
    (tag, &b[off..off + len], &b[off + len..])
}

fn pkcs1_from_pkcs8(pkcs8: &[u8]) -> Vec<u8> {
    let (_, seq, _) = der(pkcs8);
    let (_, _version, rest) = der(seq);
    let (_, _algorithm, rest) = der(rest);
    der(rest).1.to_vec()
}

struct Env { app: Router, sign: EncodingKey, dir: std::path::PathBuf }

async fn env(name: &str, with_key: bool) -> Option<Env> {
    let url = std::env::var("HELQIX_TEST_DATABASE_URL").ok()?;
    let pool = sqlx::postgres::PgPoolOptions::new().max_connections(4).connect(&url).await.expect("connect to test database");
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations");
    let pair = KeyPair::generate(KeySize::Rsa2048).unwrap();
    let sign = EncodingKey::from_rsa_der(&pkcs1_from_pkcs8(AsDer::<aws_lc_rs::encoding::Pkcs8V1Der>::as_der(&pair).unwrap().as_ref()));
    let oidc = Oidc::from_keys(ISS, AUD, HashMap::from([("k1".to_owned(), DecodingKey::from_rsa_der(pair.public_key().as_ref()))]));
    let dir = std::env::temp_dir().join(format!("helqix-evidence-api-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let state = AppState::new(pool, KEY).with_oidc(Some(oidc)).with_frame_store(Some(dir.clone())).with_run_metadata_in_postgres(true)
        .with_evidence(with_key.then(|| EvidenceConfig::new(SEED, "helqix-test-signer", "test")));
    Some(Env { app: routes::router(state, &["http://localhost:5173".to_owned()], std::time::Duration::from_secs(60)).unwrap(), sign, dir })
}

fn token(e: &Env, sub: &str) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let mut h = Header::new(Algorithm::RS256);
    h.kid = Some("k1".into());
    encode(&h, &json!({"sub": sub, "iss": ISS, "aud": AUD, "exp": now + 600}), &e.sign).unwrap()
}

async fn raw(app: &Router, bearer: &str, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let mut req = Request::builder().method(method).uri(uri).header("authorization", format!("Bearer {bearer}"));
    if body.is_some() { req = req.header("content-type", "application/json"); }
    let res = app.clone().oneshot(req.body(body.map_or_else(Body::empty, |b| Body::from(b.to_string()))).unwrap()).await.unwrap();
    let (status, headers) = (res.status(), res.headers().clone());
    (status, headers, axum::body::to_bytes(res.into_body(), 64 << 20).await.unwrap().to_vec())
}

async fn call(app: &Router, bearer: &str, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let (s, _, b) = raw(app, bearer, method, uri, body).await;
    (s, serde_json::from_slice(&b).unwrap_or(Value::Null))
}

/// The body of the reference scenario: 0.5 kg dropped from 20 m, which crosses the ground inside 4 s.
fn falling() -> Value { json!({"massKg": 0.5, "dragCoefficient": 0.47, "referenceAreaM2": 0.01, "endSeconds": 4.0, "stepSeconds": 0.01, "initialState": [0.0, 20.0, 0.0, 3.0, 0.0, 0.0]}) }
fn above_ground() -> Value { json!({"requirement": {"name": "stay above ground", "variable": "y", "comparator": "atLeast", "threshold": 0.0}}) }

/// Reads a stored-method ZIP through its central directory (what an extractor does).
fn unzip(zip: &[u8], into: &std::path::Path) -> Vec<String> {
    let le16 = |at: usize| u16::from_le_bytes([zip[at], zip[at + 1]]) as usize;
    let le32 = |at: usize| u32::from_le_bytes([zip[at], zip[at + 1], zip[at + 2], zip[at + 3]]) as usize;
    let end = zip.len() - 22;
    assert_eq!(&zip[end..end + 4], b"PK\x05\x06");
    let (count, mut at) = (le16(end + 10), le32(end + 16));
    let mut names = Vec::new();
    for _ in 0..count {
        let (size, nlen, local) = (le32(at + 24), le16(at + 28), le32(at + 42));
        let name = String::from_utf8(zip[at + 46..at + 46 + nlen].to_vec()).unwrap();
        let start = local + 30 + le16(local + 26) + le16(local + 28);
        let path = into.join(&name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, &zip[start..start + size]).unwrap();
        names.push(name);
        at += 46 + nlen;
    }
    names
}

fn extract(e: &Env, zip: &[u8], tag: &str) -> (std::path::PathBuf, Vec<String>) {
    let out = e.dir.join(format!("unzipped-{tag}"));
    let _ = std::fs::remove_dir_all(&out);
    (out.clone(), unzip(zip, &out))
}

async fn public_key(e: &Env) -> String {
    let (s, k) = call(&e.app, KEY, "GET", "/v1/evidence/key", None).await;
    assert_eq!(s, StatusCode::OK, "{k}");
    k["publicKey"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn an_exported_package_passes_the_offline_verifier_for_a_failing_and_a_holding_requirement() {
    let Some(e) = env("pass", true).await else { eprintln!("SKIPPED: HELQIX_TEST_DATABASE_URL not set"); return };
    let (s, run) = call(&e.app, KEY, "POST", "/v1/runs", Some(falling())).await;
    assert_eq!(s, StatusCode::CREATED, "{run}");
    let id = run["runId"].as_str().unwrap().to_owned();
    let key = public_key(&e).await;
    assert!(key.len() == 64 && key.chars().all(|c| c.is_ascii_hexdigit()));

    let (s, headers, zip) = raw(&e.app, KEY, "POST", &format!("/v1/runs/{id}/evidence"), Some(above_ground())).await;
    assert_eq!(s, StatusCode::OK, "{}", String::from_utf8_lossy(&zip));
    assert_eq!(headers["content-type"], "application/zip");
    assert!(headers["content-disposition"].to_str().unwrap().contains(&format!("evidence-{id}.zip")));
    // CI hands this file to unzip, Python's zipfile and the real verifier binary: checks that do not share this test's reader.
    if let Ok(path) = std::env::var("HELQIX_TEST_ZIP_OUT") { std::fs::write(path, &zip).unwrap(); }
    let (out, names) = extract(&e, &zip, "fail");
    for n in ["trusted-public-key.txt", "README.txt"] { assert!(names.contains(&n.to_owned()), "{names:?}"); }
    assert!(names.contains(&format!("evidence-{id}.intoto.dsse.json")) && names.contains(&format!("evidence-{id}/signature.json")), "{names:?}");
    assert_eq!(std::fs::read_to_string(out.join("trusted-public-key.txt")).unwrap().trim(), key, "the key in the archive is the key the service names");
    let report = verify_package(&out.join(format!("evidence-{id}")), &key);
    assert!(report.pass, "{:?}", report.reasons);
    let outcome: Value = serde_json::from_slice(&std::fs::read(out.join(format!("evidence-{id}/requirement_outcome.json"))).unwrap()).unwrap();
    assert_eq!(outcome["satisfied"], false);

    // The same run against a requirement that holds.
    let holds = json!({"requirement": {"name": "stay below the ceiling", "variable": "y", "comparator": "atMost", "threshold": 1000.0}, "subject": "a body that never reaches 1 km"});
    let (s, _, zip) = raw(&e.app, KEY, "POST", &format!("/v1/runs/{id}/evidence"), Some(holds)).await;
    assert_eq!(s, StatusCode::OK);
    let (out, _) = extract(&e, &zip, "holds");
    let report = verify_package(&out.join(format!("evidence-{id}")), &key);
    assert!(report.pass, "{:?}", report.reasons);
    let manifest: Value = serde_json::from_slice(&std::fs::read(out.join(format!("evidence-{id}/manifest.json"))).unwrap()).unwrap();
    assert_eq!(manifest["statement"]["subject"], "a body that never reaches 1 km");
    assert!(manifest["statement"]["failure_index"].is_null());

    // A changed byte in the downloaded package fails the verifier.
    let frames = out.join(format!("evidence-{id}/frames.parquet"));
    let mut bytes = std::fs::read(&frames).unwrap();
    bytes[10] ^= 1;
    std::fs::write(&frames, bytes).unwrap();
    assert!(!verify_package(&out.join(format!("evidence-{id}")), &key).pass);
    let _ = std::fs::remove_dir_all(&e.dir);
}

#[tokio::test]
async fn a_request_that_cannot_be_answered_honestly_is_refused() {
    let Some(e) = env("refuse", true).await else { eprintln!("SKIPPED: HELQIX_TEST_DATABASE_URL not set"); return };
    let (_, run) = call(&e.app, KEY, "POST", "/v1/runs", Some(falling())).await;
    let id = run["runId"].as_str().unwrap().to_owned();
    let uri = format!("/v1/runs/{id}/evidence");
    let mut bad = above_ground();
    bad["requirement"]["variable"] = json!("temperature");
    let (s, b) = call(&e.app, KEY, "POST", &uri, Some(bad)).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "an unknown variable names the known ones: {b}");
    assert!(b["error"]["message"].as_str().unwrap().contains("vx"));
    let mut extra = above_ground();
    extra["requirement"]["note"] = json!("not a field");
    assert!(call(&e.app, KEY, "POST", &uri, Some(extra)).await.0.is_client_error());
    let mut multi = above_ground();
    multi["subject"] = json!("two\nlines");
    assert_eq!(call(&e.app, KEY, "POST", &uri, Some(multi)).await.0, StatusCode::BAD_REQUEST);
    let (s, b) = call(&e.app, KEY, "POST", "/v1/runs/run-doesnotexist00/evidence", Some(above_ground())).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "{b}");
    let _ = std::fs::remove_dir_all(&e.dir);
}

#[tokio::test]
async fn without_a_signing_key_export_is_unavailable() {
    let Some(e) = env("nokey", false).await else { eprintln!("SKIPPED: HELQIX_TEST_DATABASE_URL not set"); return };
    let (_, run) = call(&e.app, KEY, "POST", "/v1/runs", Some(falling())).await;
    let id = run["runId"].as_str().unwrap().to_owned();
    assert_eq!(call(&e.app, KEY, "POST", &format!("/v1/runs/{id}/evidence"), Some(above_ground())).await.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(call(&e.app, KEY, "GET", "/v1/evidence/key", None).await.0, StatusCode::SERVICE_UNAVAILABLE);
    let _ = std::fs::remove_dir_all(&e.dir);
}

#[tokio::test]
async fn a_user_exports_only_a_run_they_own_and_the_private_key_is_never_served() {
    let Some(e) = env("owner", true).await else { eprintln!("SKIPPED: HELQIX_TEST_DATABASE_URL not set"); return };
    let pid = std::process::id();
    let (alice, bob) = (token(&e, &format!("alice-ev-{pid}")), token(&e, &format!("bob-ev-{pid}")));
    // Each user's run differs (the run id is a hash of the request), so ownership is not shared by accident.
    let mut mine = falling();
    mine["massKg"] = json!(0.61 + (pid % 1000) as f64 / 10_000.0);
    let (s, run) = call(&e.app, &alice, "POST", "/v1/runs", Some(mine)).await;
    assert_eq!(s, StatusCode::CREATED, "{run}");
    let id = run["runId"].as_str().unwrap().to_owned();
    let uri = format!("/v1/runs/{id}/evidence");

    let (s, _, zip) = raw(&e.app, &alice, "POST", &uri, Some(above_ground())).await;
    assert_eq!(s, StatusCode::OK, "the owner exports");
    let key = public_key(&e).await;
    let (out, _) = extract(&e, &zip, "alice");
    assert!(verify_package(&out.join(format!("evidence-{id}")), &key).pass);

    let (s, b) = call(&e.app, &bob, "POST", &uri, Some(above_ground())).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "another user's run is not revealed: {b}");
    assert_eq!(call(&e.app, KEY, "POST", &uri, Some(above_ground())).await.0, StatusCode::OK, "the operator's key may export any run");

    // Nothing the service says carries the private seed.
    let seed_hex: String = SEED.iter().map(|b| format!("{b:02x}")).collect();
    let (_, headers, body) = raw(&e.app, &alice, "GET", "/v1/evidence/key", None).await;
    assert!(!String::from_utf8_lossy(&body).contains(&seed_hex) && !format!("{headers:?}").contains(&seed_hex));
    assert!(!zip.windows(seed_hex.len()).any(|w| w == seed_hex.as_bytes()), "the archive must not contain the seed");
    let _ = std::fs::remove_dir_all(&e.dir);
}
