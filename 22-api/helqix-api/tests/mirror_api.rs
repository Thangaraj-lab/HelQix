//! Frame files in an object store, with an in-memory store standing in for S3 (no network, no database).
//! Three "API instances" with empty local directories share one object store.
use std::sync::Arc;

use axum::{Router, body::Body, http::{Request, StatusCode}};
use helqix_api::{mirror::ObjectMirror, routes, state::AppState};
use object_store::{ObjectStore, ObjectStoreExt, PutPayload, memory::InMemory, path::Path as ObjectPath};
use serde_json::{Value, json};
use tower::ServiceExt;

const KEY: &str = "test-api-key-not-a-secret-0123456789";

fn instance(tag: &str, store: Arc<dyn ObjectStore>) -> (Router, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("helqix-mirror-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let pool = sqlx::postgres::PgPoolOptions::new().connect_lazy("postgres://unused:unused@127.0.0.1:1/unused").unwrap();
    let state = AppState::new(pool, KEY).with_frame_store(Some(dir.clone())).with_mirror(Some(ObjectMirror::new(store)));
    (routes::router(state, &["http://localhost:5173".to_owned()], std::time::Duration::from_secs(10)).unwrap(), dir)
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
async fn a_run_stored_by_one_instance_is_served_by_another_and_corruption_is_caught() {
    let store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    let (a, dir_a) = instance("a", store.clone());
    let req = json!({"massKg": 0.5, "dragCoefficient": 0.47, "referenceAreaM2": 0.01, "endSeconds": 0.1, "initialState": [0.0, 20.0, 0.0, 3.0, 0.0, 0.0]});
    let (s, run) = call(&a, "POST", "/v1/runs", Some(req)).await;
    assert_eq!(s, StatusCode::CREATED, "{run}");
    let id = run["runId"].as_str().unwrap().to_owned();
    let hash = run["framesBlake3"].as_str().unwrap().to_owned();
    assert!(store.head(&ObjectPath::from(format!("frames/{hash}.parquet"))).await.is_ok(), "the frames were uploaded");
    assert!(store.head(&ObjectPath::from(format!("runs/{id}.json"))).await.is_ok(), "the index was uploaded");

    // A second instance with an empty directory reads it directly (download before read).
    let (b, dir_b) = instance("b", store.clone());
    assert!(!dir_b.join("runs").join(format!("{id}.json")).exists());
    let (s, frames) = call(&b, "GET", &format!("/v1/runs/{id}/frames?limit=3"), None).await;
    assert_eq!(s, StatusCode::OK, "{frames}");
    assert_eq!(frames["framesBlake3"], hash.as_str());
    assert!(dir_b.join("frames").join(format!("{hash}.parquet")).exists(), "downloaded into the local cache");

    // A fresh instance that only lists sees it too (index sync).
    let (d, _dir_d) = instance("d", store.clone());
    let (s, list) = call(&d, "GET", "/v1/runs", None).await;
    assert_eq!(s, StatusCode::OK, "{list}");
    assert!(list["runs"].as_array().unwrap().iter().any(|r| r["runId"] == id.as_str()), "{list}");

    // Someone swaps the object for other bytes: a third instance refuses to serve it.
    store.put(&ObjectPath::from(format!("frames/{hash}.parquet")), PutPayload::from(b"not the frames".to_vec())).await.unwrap();
    let (c, _dir_c) = instance("c", store.clone());
    let (s, body) = call(&c, "GET", &format!("/v1/runs/{id}/frames"), None).await;
    assert_ne!(s, StatusCode::OK, "a corrupted object must not be served: {body}");

    // A run nobody has is simply not found.
    assert_eq!(call(&b, "GET", "/v1/runs/run-doesnotexist/frames", None).await.0, StatusCode::NOT_FOUND);
    let _ = (dir_a, dir_b);
}
