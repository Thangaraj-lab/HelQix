//! A signed-in user lists the runs that are theirs: the ones they started, the ones made from their projects (even by the operator's key) and the
//! branches of those. Another user's runs are not in the list. Found by the browser walkthrough with real Keycloak: the Studio's run list was a 403 for users.
//! Needs HELQIX_TEST_DATABASE_URL (prints SKIPPED otherwise).
use std::collections::HashMap;

use aws_lc_rs::{encoding::AsDer, rsa::{KeyPair, KeySize}, signature::KeyPair as _};
use axum::{Router, body::Body, http::{Request, StatusCode}};
use helqix_api::{auth::Oidc, routes, state::AppState};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, encode};
use serde_json::{Value, json};
use tower::ServiceExt;

const KEY: &str = "test-api-key-not-a-secret-0123456789";
const ISS: &str = "https://sso.example.test/realms/helqix";
const AUD: &str = "helqix-api";

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

async fn env(name: &str) -> Option<Env> {
    let url = std::env::var("HELQIX_TEST_DATABASE_URL").ok()?;
    let pool = sqlx::postgres::PgPoolOptions::new().max_connections(4).connect(&url).await.expect("connect to test database");
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations");
    let pair = KeyPair::generate(KeySize::Rsa2048).unwrap();
    let sign = EncodingKey::from_rsa_der(&pkcs1_from_pkcs8(AsDer::<aws_lc_rs::encoding::Pkcs8V1Der>::as_der(&pair).unwrap().as_ref()));
    let oidc = Oidc::from_keys(ISS, AUD, HashMap::from([("k1".to_owned(), DecodingKey::from_rsa_der(pair.public_key().as_ref()))]));
    let dir = std::env::temp_dir().join(format!("helqix-runs-owner-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let state = AppState::new(pool, KEY).with_oidc(Some(oidc)).with_frame_store(Some(dir.clone())).with_run_metadata_in_postgres(true);
    Some(Env { app: routes::router(state, &["http://localhost:5173".to_owned()], std::time::Duration::from_secs(20)).unwrap(), sign, dir })
}

fn token(e: &Env, sub: &str) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let mut h = Header::new(Algorithm::RS256);
    h.kid = Some("k1".into());
    encode(&h, &json!({"sub": sub, "iss": ISS, "aud": AUD, "exp": now + 600}), &e.sign).unwrap()
}

async fn call(app: &Router, bearer: &str, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri).header("authorization", format!("Bearer {bearer}"));
    if body.is_some() { req = req.header("content-type", "application/json"); }
    let res = app.clone().oneshot(req.body(body.map_or_else(Body::empty, |b| Body::from(b.to_string()))).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 4 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn run_body(mass: f64) -> Value { json!({"massKg": mass, "dragCoefficient": 0.5, "referenceAreaM2": 0.01, "endSeconds": 0.1, "stepSeconds": 0.01, "initialState": [0.0, 10.0, 0.0, 0.0, 0.0, 0.0]}) }

async fn ids(app: &Router, who: &str) -> Vec<String> {
    let (s, r) = call(app, who, "GET", "/v1/runs", None).await;
    assert_eq!(s, StatusCode::OK, "GET /v1/runs for a user must work: {r}");
    r["runs"].as_array().unwrap().iter().map(|x| x["runId"].as_str().unwrap().to_owned()).collect()
}

fn v(x: f64, unit: &str) -> Value { json!({"v": x, "unit": unit, "origin": "user_stated", "status": "unverified"}) }

#[tokio::test]
async fn a_user_lists_their_own_runs_and_not_anyone_elses() {
    let Some(e) = env("list").await else { eprintln!("SKIPPED: HELQIX_TEST_DATABASE_URL not set"); return };
    let pid = std::process::id();
    let (alice, bob) = (token(&e, &format!("alice-{pid}")), token(&e, &format!("bob-{pid}")));

    // Runs started directly.
    let (s, a) = call(&e.app, &alice, "POST", "/v1/runs", Some(run_body(1.0))).await;
    assert_eq!(s, StatusCode::CREATED, "{a}");
    let (s, b) = call(&e.app, &bob, "POST", "/v1/runs", Some(run_body(2.0))).await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    let (ra, rb) = (a["runId"].as_str().unwrap().to_owned(), b["runId"].as_str().unwrap().to_owned());
    let (alices, bobs) = (ids(&e.app, &alice).await, ids(&e.app, &bob).await);
    assert!(alices.contains(&ra) && !alices.contains(&rb), "{alices:?}");
    assert!(bobs.contains(&rb) && !bobs.contains(&ra), "{bobs:?}");

    // A branch of Alice's run is Alice's.
    let (s, cp) = call(&e.app, &alice, "POST", &format!("/v1/runs/{ra}/freeze"), Some(json!({"t": 0.05}))).await;
    assert_eq!(s, StatusCode::OK, "{cp}");
    let (s, br) = call(&e.app, &alice, "POST", "/v1/branches", Some(json!({"checkpointId": cp["checkpointId"], "overrides": {"massKg": 3.0}}))).await;
    assert_eq!(s, StatusCode::CREATED, "{br}");
    let rbr = br["runId"].as_str().unwrap().to_owned();
    assert!(ids(&e.app, &alice).await.contains(&rbr));
    assert!(!ids(&e.app, &bob).await.contains(&rbr));

    // A run the operator's key made from Alice's project belongs to Alice through her project, not to Bob.
    let (_, project) = call(&e.app, &alice, "POST", "/v1/projects", Some(json!({"payload": {"name": "hers"}}))).await;
    let proj = project["data"]["id"].as_str().unwrap().to_owned();
    let (_, build) = call(&e.app, &alice, "POST", &format!("/v1/projects/{proj}/builds"), None).await;
    let bid = build["buildId"].as_str().unwrap().to_owned();
    let ops = json!([
        {"op": "part.add", "id": "box", "name": "box", "geometry": {"kind": "primitive", "shape": "box", "dims": [v(100.0, "mm"), v(100.0, "mm"), v(100.0, "mm")]}, "maturity": "user_defined", "prov": {"kind": "user_stated"}},
        {"op": "part.place", "id": "box", "transform": {"pos": [v(0.0, "m"), v(0.0, "m"), v(0.0, "m")], "rot": [v(0.0, "deg"), v(0.0, "deg"), v(0.0, "deg")], "scale": [1.0, 1.0, 1.0], "frame": "world"}},
        {"op": "part.setParam", "id": "box", "key": "density_kg_m3", "value": v(1000.0, "kg/m3")}
    ]);
    let (s, r) = call(&e.app, &alice, "POST", &format!("/v1/builds/{bid}/ops"), Some(json!({"ops": ops}))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let (s, rev) = call(&e.app, &alice, "POST", &format!("/v1/builds/{bid}/commit"), None).await;
    assert_eq!(s, StatusCode::CREATED, "{rev}");
    let n = rev["number"].as_u64().unwrap();
    let (s, run) = call(&e.app, KEY, "POST", &format!("/v1/projects/{proj}/revisions/{n}/run"), Some(json!({"dragCoefficient": 0.5, "endSeconds": 0.1, "stepSeconds": 0.01, "initialState": [0.0, 10.0, 0.0, 0.0, 0.0, 0.0]}))).await;
    assert_eq!(s, StatusCode::CREATED, "{run}");
    let rop = run["runId"].as_str().unwrap().to_owned();
    assert!(ids(&e.app, &alice).await.contains(&rop), "the operator's run of Alice's project is Alice's");
    assert!(!ids(&e.app, &bob).await.contains(&rop));

    // The operator's key still sees everything.
    let all = ids(&e.app, KEY).await;
    for r in [&ra, &rb, &rbr, &rop] { assert!(all.contains(r), "{r} missing from the operator's list"); }
    let _ = std::fs::remove_dir_all(&e.dir);
}
