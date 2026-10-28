//! Two signed-in users and the service key through the real router and PostgreSQL. Needs HELQIX_TEST_DATABASE_URL
//! (CI provides it; without it the test prints SKIPPED).
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

struct Env { app: Router, sign: EncodingKey }

async fn env() -> Option<Env> {
    let url = std::env::var("HELQIX_TEST_DATABASE_URL").ok()?;
    let pool = sqlx::postgres::PgPoolOptions::new().max_connections(4).connect(&url).await.expect("connect to test database");
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations");
    let pair = KeyPair::generate(KeySize::Rsa2048).unwrap();
    let sign = EncodingKey::from_rsa_der(&pkcs1_from_pkcs8(AsDer::<aws_lc_rs::encoding::Pkcs8V1Der>::as_der(&pair).unwrap().as_ref()));
    let oidc = Oidc::from_keys(ISS, AUD, HashMap::from([("k1".to_owned(), DecodingKey::from_rsa_der(pair.public_key().as_ref()))]));
    let state = AppState::new(pool, KEY).with_oidc(Some(oidc));
    Some(Env { app: routes::router(state, &["http://localhost:5173".to_owned()], std::time::Duration::from_secs(10)).unwrap(), sign })
}

fn token(e: &Env, sub: &str) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let mut h = Header::new(Algorithm::RS256);
    h.kid = Some("k1".into());
    encode(&h, &json!({"sub": sub, "iss": ISS, "aud": AUD, "exp": now + 600}), &e.sign).unwrap()
}

async fn call(app: &Router, bearer: Option<&str>, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(b) = bearer { req = req.header("authorization", format!("Bearer {b}")); }
    if body.is_some() { req = req.header("content-type", "application/json"); }
    let res = app.clone().oneshot(req.body(body.map_or_else(Body::empty, |b| Body::from(b.to_string()))).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 4 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn users_see_only_their_own_projects_builds_and_runs() {
    let Some(e) = env().await else { eprintln!("SKIPPED: HELQIX_TEST_DATABASE_URL not set"); return };
    let (alice, bob) = (token(&e, &format!("alice-{}", std::process::id())), token(&e, &format!("bob-{}", std::process::id())));

    let (s, p) = call(&e.app, Some(&alice), "POST", "/v1/projects", Some(json!({"payload": {"name": "Alice's"}}))).await;
    assert_eq!(s, StatusCode::CREATED, "{p}");
    let pid = p["data"]["id"].as_str().unwrap().to_owned();

    // Bob cannot read, change, delete, build on, or list Alice's project; the answer does not reveal that it exists.
    assert_eq!(call(&e.app, Some(&bob), "GET", &format!("/v1/projects/{pid}"), None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&e.app, Some(&bob), "PUT", &format!("/v1/projects/{pid}"), Some(json!({"payload": {"name": "mine now"}}))).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&e.app, Some(&bob), "DELETE", &format!("/v1/projects/{pid}"), None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&e.app, Some(&bob), "POST", &format!("/v1/projects/{pid}/builds"), None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&e.app, Some(&bob), "GET", &format!("/v1/projects/{pid}/runs"), None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&e.app, Some(&bob), "GET", &format!("/v1/projects/{pid}/revisions"), None).await.0, StatusCode::NOT_FOUND);
    let (_, bobs) = call(&e.app, Some(&bob), "GET", "/v1/projects", None).await;
    assert_eq!(bobs["data"], json!([]), "Bob's list is empty");
    let (s, none) = call(&e.app, Some(&bob), "POST", "/v1/executions", Some(json!({"payload": {"x": 1}, "project_id": pid}))).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "cannot attach a resource to another user's project: {none}");

    // Alice can, and sees exactly her project.
    assert_eq!(call(&e.app, Some(&alice), "GET", &format!("/v1/projects/{pid}"), None).await.0, StatusCode::OK);
    let (s, b) = call(&e.app, Some(&alice), "POST", &format!("/v1/projects/{pid}/builds"), None).await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    let bid = b["buildId"].as_str().unwrap().to_owned();
    let (_, hers) = call(&e.app, Some(&alice), "GET", "/v1/projects", None).await;
    assert_eq!(hers["data"].as_array().unwrap().len(), 1);

    // The build belongs to the project's owner.
    assert_eq!(call(&e.app, Some(&bob), "GET", &format!("/v1/builds/{bid}"), None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&e.app, Some(&bob), "POST", &format!("/v1/builds/{bid}/ops"), Some(json!({"ops": []}))).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&e.app, Some(&alice), "GET", &format!("/v1/builds/{bid}"), None).await.0, StatusCode::OK);

    // Routes users may not use, and routes nobody told the guard about.
    assert_eq!(call(&e.app, Some(&bob), "GET", "/v1/runs", None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(call(&e.app, Some(&bob), "GET", "/v1/unknown-thing", None).await.0, StatusCode::FORBIDDEN);

    // The service key sees it all; no token and a bad token get nothing.
    assert_eq!(call(&e.app, Some(KEY), "GET", &format!("/v1/projects/{pid}"), None).await.0, StatusCode::OK);
    assert_eq!(call(&e.app, None, "GET", "/v1/projects", None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&e.app, Some("garbage"), "GET", "/v1/projects", None).await.0, StatusCode::UNAUTHORIZED);

    // Alice can delete her own project.
    let (_, rec) = call(&e.app, Some(&alice), "GET", &format!("/v1/projects/{pid}"), None).await;
    let version = rec["data"]["version"].as_i64().unwrap();
    let req = Request::builder().method("DELETE").uri(format!("/v1/projects/{pid}")).header("authorization", format!("Bearer {alice}")).header("if-match", format!("\"{version}\"")).body(Body::empty()).unwrap();
    assert_eq!(e.app.clone().oneshot(req).await.unwrap().status(), StatusCode::NO_CONTENT);
}
