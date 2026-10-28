//! The frame mirror against a real S3-compatible server (SeaweedFS in CI). Runs only when HELQIX_S3_ENDPOINT and the
//! other HELQIX_S3_* variables are set (CI sets them); otherwise it prints SKIPPED.
use axum::{Router, body::Body, http::{Request, StatusCode}};
use helqix_api::{mirror::ObjectMirror, routes, state::AppState};
use serde_json::{Value, json};
use tower::ServiceExt;

const KEY: &str = "test-api-key-not-a-secret-0123456789";

fn instance(tag: &str) -> Router {
    let dir = std::env::temp_dir().join(format!("helqix-s3-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let pool = sqlx::postgres::PgPoolOptions::new().connect_lazy("postgres://unused:unused@127.0.0.1:1/unused").unwrap();
    let state = AppState::new(pool, KEY).with_frame_store(Some(dir)).with_mirror(ObjectMirror::from_env().unwrap());
    routes::router(state, &["http://localhost:5173".to_owned()], std::time::Duration::from_secs(20)).unwrap()
}

async fn call(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri).header("authorization", format!("Bearer {KEY}"));
    if body.is_some() { req = req.header("content-type", "application/json"); }
    let res = app.clone().oneshot(req.body(body.map_or_else(Body::empty, |b| Body::from(b.to_string()))).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 64 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn runs_survive_a_restart_through_a_real_object_store() {
    if std::env::var("HELQIX_S3_ENDPOINT").is_err() { eprintln!("SKIPPED: HELQIX_S3_ENDPOINT not set"); return; }
    let a = instance("a");
    let req = json!({"massKg": 0.5, "dragCoefficient": 0.47, "referenceAreaM2": 0.01, "endSeconds": 0.3, "initialState": [0.0, 20.0, 0.0, 3.0, 0.0, 0.0]});
    let (s, run) = call(&a, "POST", "/v1/runs", Some(req)).await;
    assert_eq!(s, StatusCode::CREATED, "{run}");
    let id = run["runId"].as_str().unwrap().to_owned();
    let (_, original) = call(&a, "GET", &format!("/v1/runs/{id}/frames?limit=1000"), None).await;

    // A new instance with an empty directory (a restart on another machine) serves the same frames.
    let b = instance("b");
    let (s, again) = call(&b, "GET", &format!("/v1/runs/{id}/frames?limit=1000"), None).await;
    assert_eq!(s, StatusCode::OK, "{again}");
    assert_eq!(again["framesBlake3"], original["framesBlake3"]);
    assert_eq!(again["frames"], original["frames"], "identical frames after the round trip through the object store");
    let (_, list) = call(&instance("c"), "GET", "/v1/runs", None).await;
    assert!(list["runs"].as_array().unwrap().iter().any(|r| r["runId"] == id.as_str()));
}
