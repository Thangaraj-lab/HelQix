//! K4 frame routes through the real router. Frame routes do not touch PostgreSQL, so the pool is
//! lazy and never connects; this test needs no database.
use axum::{Router, body::Body, http::{Request, StatusCode}};
use helqix_api::{routes, state::AppState};
use helqix_frames::{point_mass_variables, FrameStore};
use helqix_freeze::{chain_genesis, run, PointMassSpec, RunSpec};
use serde_json::Value;
use tower::ServiceExt;

const KEY: &str = "test-api-key-not-a-secret";

fn spec() -> RunSpec {
    RunSpec {
        model: PointMassSpec { mass_kg: 0.5, gravity_m_s2: [0.0, -9.80665, 0.0], air_density_kg_m3: 1.225, wind_m_s: [0.0; 3], applied_force_n: [0.0; 3], drag_coefficient: 0.47, reference_area_m2: 0.01 },
        start_seconds: 0.0, end_seconds: 1.0, step_seconds: 0.001, initial_state: vec![0.0, 20.0, 0.0, 3.0, 0.0, 0.0], seeds: vec![],
    }
}

fn app(store: Option<std::path::PathBuf>) -> Router {
    let pool = sqlx::postgres::PgPoolOptions::new().connect_lazy("postgres://unused:unused@127.0.0.1:1/unused").unwrap();
    routes::router(AppState::new(pool, KEY).with_frame_store(store), &["http://localhost:5173".to_owned()], std::time::Duration::from_secs(10)).unwrap()
}

async fn get(app: &Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder().uri(uri).header("authorization", format!("Bearer {KEY}")).body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 64 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn frame_routes_serve_verified_stored_frames() {
    let dir = std::env::temp_dir().join(format!("helqix-api-frames-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let s = spec();
    let r = run(&s).unwrap();
    let entry = FrameStore::open(&dir).unwrap().put("ref", &r, &point_mass_variables(), &chain_genesis(&s)).unwrap();
    let app = app(Some(dir.clone()));

    let (st, body) = get(&app, "/v1/runs/ref/frames?limit=3").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert_eq!(body["framesBlake3"], entry.frames_blake3.as_str());
    assert_eq!(body["frames"].as_array().unwrap().len(), 3);
    assert_eq!(body["next"], 3);
    let (_, page2) = get(&app, "/v1/runs/ref/frames?from=999&limit=10").await;
    assert_eq!(page2["frames"].as_array().unwrap().len(), 2);
    assert_eq!(page2["next"], Value::Null);

    let t = r.samples[400].time_seconds;
    let (st, f) = get(&app, &format!("/v1/runs/ref/frame?t={t}")).await;
    assert_eq!(st, StatusCode::OK, "{f}");
    assert_eq!(f["frame"]["index"], 400);
    assert_eq!(f["frame"]["state"]["y"]["origin"], "simulated");
    assert_eq!(f["frame"]["stateSha256"], r.chain[400].as_str());
    let (_, mid) = get(&app, &format!("/v1/runs/ref/frame?t={}", t + 0.0005)).await;
    assert_eq!(mid["frame"]["state"]["y"]["origin"], "derived");

    assert_eq!(get(&app, "/v1/runs/ref/frame?t=5").await.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(get(&app, "/v1/runs/nope/frames").await.0, StatusCode::NOT_FOUND);
    assert_eq!(get(&app, "/v1/runs/..%2Fescape/frames").await.0, StatusCode::BAD_REQUEST);
    assert_eq!(get(&app, "/v1/runs/ref/frames?limit=0").await.0, StatusCode::BAD_REQUEST);

    // A tampered frame file is refused, not served.
    let path = dir.join("frames").join(format!("{}.parquet", entry.frames_blake3));
    let mut bytes = std::fs::read(&path).unwrap();
    let mid_byte = bytes.len() / 2;
    bytes[mid_byte] ^= 1;
    std::fs::write(&path, bytes).unwrap();
    assert_eq!(get(&app, "/v1/runs/ref/frames").await.0, StatusCode::CONFLICT);

    assert_eq!(get(&self::app(None), "/v1/runs/ref/frames").await.0, StatusCode::SERVICE_UNAVAILABLE, "no store configured");
}

#[tokio::test]
async fn stored_runs_are_listed_with_their_variables() {
    let dir = std::env::temp_dir().join(format!("helqix-api-runs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let s = spec();
    let r = run(&s).unwrap();
    FrameStore::open(&dir).unwrap().put("ref", &r, &point_mass_variables(), &chain_genesis(&s)).unwrap();
    let app = app(Some(dir.clone()));

    let (status, body) = get(&app, "/v1/runs").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let runs = body["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["runId"], "ref");
    assert_eq!(runs[0]["variables"][1], serde_json::json!({"name": "y", "unit": "m"}));
    assert!(runs[0]["rows"].as_u64().unwrap() > 1);

    // Without a frame store there is nothing to list, and the API says so.
    let (status, _) = get(&app_none(), "/v1/runs").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let _ = std::fs::remove_dir_all(&dir);
}

fn app_none() -> Router { app(None) }

async fn post(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder().method("POST").uri(uri).header("authorization", format!("Bearer {KEY}")).header("content-type", "application/json").body(Body::from(body.to_string())).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn a_run_started_through_the_api_is_stored_and_repeatable() {
    let dir = std::env::temp_dir().join(format!("helqix-api-start-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let app = app(Some(dir.clone()));
    let req = serde_json::json!({"massKg": 0.5, "dragCoefficient": 0.47, "referenceAreaM2": 0.01, "endSeconds": 0.5, "initialState": [0.0, 20.0, 0.0, 3.0, 0.0, 0.0]});
    let (st, a) = post(&app, "/v1/runs", req.clone()).await;
    assert_eq!(st, StatusCode::CREATED, "{a}");
    let (_, b) = post(&app, "/v1/runs", req).await;
    assert_eq!(a["runId"], b["runId"]);
    assert_eq!(a["framesBlake3"], b["framesBlake3"], "same request, same frames");
    let id = a["runId"].as_str().unwrap();
    let (st, frames) = get(&app, &format!("/v1/runs/{id}/frames?limit=2")).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(frames["frames"].as_array().unwrap().len(), 2);
    let (_, list) = get(&app, "/v1/runs").await;
    assert_eq!(list["runs"].as_array().unwrap().len(), 1);
    // Refused: unknown field, zero mass, too many steps.
    assert_eq!(post(&app, "/v1/runs", serde_json::json!({"massKg": 1.0, "oops": 1})).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let bad = |m: f64, end: f64| serde_json::json!({"massKg": m, "dragCoefficient": 0.1, "referenceAreaM2": 0.01, "endSeconds": end, "initialState": [0.0, 0.0, 0.0, 0.0, 0.0, 0.0]});
    assert_eq!(post(&app, "/v1/runs", bad(0.0, 1.0)).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(post(&app, "/v1/runs", bad(1.0, 1.0e6)).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(post(&app, "/v1/runs", bad(1.0, 0.1)).await.0, StatusCode::CREATED);
}

#[tokio::test]
async fn a_branch_keeps_the_past_and_changes_only_the_future() {
    let dir = std::env::temp_dir().join(format!("helqix-api-branch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let app = app(Some(dir.clone()));
    let (_, run) = post(&app, "/v1/runs", serde_json::json!({"massKg": 0.5, "dragCoefficient": 0.47, "referenceAreaM2": 0.01, "endSeconds": 1.0, "initialState": [0.0, 20.0, 0.0, 3.0, 0.0, 0.0]})).await;
    let id = run["runId"].as_str().unwrap().to_owned();

    let (st, cp) = post(&app, &format!("/v1/runs/{id}/freeze"), serde_json::json!({"t": 0.5})).await;
    assert_eq!(st, StatusCode::OK, "{cp}");
    assert_eq!(cp["index"], 500);
    let cp_id = cp["checkpointId"].as_str().unwrap().to_owned();
    let (_, again) = post(&app, &format!("/v1/runs/{id}/freeze"), serde_json::json!({"t": 0.5})).await;
    assert_eq!(cp["hash"], again["hash"], "the same point freezes to the same hash");

    let body = serde_json::json!({"checkpointId": cp_id, "overrides": {"referenceAreaM2": 0.3}});
    let (st, br) = post(&app, "/v1/branches", body.clone()).await;
    assert_eq!(st, StatusCode::CREATED, "{br}");
    assert_eq!(br["parentRunId"], id.as_str());
    let (_, br2) = post(&app, "/v1/branches", body).await;
    assert_eq!(br["framesBlake3"], br2["framesBlake3"], "a branch is repeatable");

    let bid = br["runId"].as_str().unwrap();
    let (_, parent) = get(&app, &format!("/v1/runs/{id}/frames?limit=1001")).await;
    let (_, child) = get(&app, &format!("/v1/runs/{bid}/frames?limit=1001")).await;
    let (p, c) = (parent["frames"].as_array().unwrap(), child["frames"].as_array().unwrap());
    assert_eq!(p.len(), c.len());
    assert_eq!(p[..500], c[..500], "everything before the checkpoint is the parent's");
    assert_ne!(p[1000]["state"]["vy"], c[1000]["state"]["vy"], "the changed area changes the fall afterwards");

    // Refused: no change, branch of a branch, unknown run, a run without saved parameters.
    assert_eq!(post(&app, "/v1/branches", serde_json::json!({"checkpointId": format!("{id}@10"), "overrides": {}})).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(post(&app, "/v1/branches", serde_json::json!({"checkpointId": format!("{bid}@10"), "overrides": {"massKg": 1.0}})).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(post(&app, "/v1/runs/nope/freeze", serde_json::json!({"t": 0.1})).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(post(&app, "/v1/branches", serde_json::json!({"checkpointId": "nope", "overrides": {"massKg": 1.0}})).await.0, StatusCode::BAD_REQUEST);
}
