//! E4.3 acceptance through the real router and PostgreSQL: a committed model whose part has an FMU behaviour runs through
//! FMPy, its frames land in the frame store, and the run is tied to the project and revision.
//!
//! Needs HELQIX_TEST_DATABASE_URL, HELQIX_PYTHON (fmpy 0.3.32, pint 0.26.2) and the Reference FMUs from
//! `tools/fixtures/fetch_reference_fmus.py` (Example data, BSD-2-Clause); without them each test prints SKIPPED and passes.
use std::{path::{Path, PathBuf}, time::Duration};

use axum::{Router, body::Body, http::{Request, StatusCode}};
use helqix_adapters::{fmi::FmiAdapter, solver::{ANY_UNIT, IoField, SolverAdapter}};
use helqix_api::{fmu_run::FmuConfig, routes, state::AppState};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

const KEY: &str = "test-api-key-not-a-secret";

fn root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..") }
fn ball_fmu() -> PathBuf { root().join(".cache/reference-fmus/v0.0.42/fmus/BouncingBall.fmu") }
fn sha(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }

struct Env { dir: PathBuf, fmus: PathBuf, python: PathBuf, sha: String }

fn env(name: &str) -> Option<Env> {
    let python: PathBuf = std::env::var("HELQIX_PYTHON").ok()?.into();
    std::env::var("HELQIX_TEST_DATABASE_URL").ok()?;
    let bytes = std::fs::read(ball_fmu()).ok()?;
    let dir = std::env::temp_dir().join(format!("helqix-fmu-run-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let fmus = dir.join("fmus");
    std::fs::create_dir_all(&fmus).unwrap();
    let sha = sha(&bytes);
    std::fs::write(fmus.join(format!("{sha}.fmu")), &bytes).unwrap();
    Some(Env { dir, fmus, python, sha })
}

async fn app(e: &Env, with_fmu: bool) -> Router {
    let url = std::env::var("HELQIX_TEST_DATABASE_URL").unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new().max_connections(4).connect(&url).await.expect("connect to test database");
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations");
    let cfg = FmuConfig { store: e.fmus.clone(), python: e.python.clone(), package_dir: root().join("18-libraries/python"), timeout: Duration::from_secs(120) };
    let state = AppState::new(pool, KEY).with_frame_store(Some(e.dir.join("frames"))).with_run_metadata_in_postgres(true).with_fmu(with_fmu.then_some(cfg));
    routes::router(state, &["http://localhost:5173".to_owned()], Duration::from_secs(120)).expect("router")
}

async fn call(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri).header("authorization", format!("Bearer {KEY}"));
    if body.is_some() { req = req.header("content-type", "application/json"); }
    let res = app.clone().oneshot(req.body(body.map_or_else(Body::empty, |b| Body::from(b.to_string()))).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 8 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn v(x: f64, unit: &str) -> Value { json!({"v": x, "unit": unit, "origin": "user_stated", "status": "unverified"}) }
fn ball_part() -> Value { json!({"op": "part.add", "id": "ball", "name": "Ball", "geometry": {"kind": "primitive", "shape": "sphere", "dims": [v(200.0, "mm")]}, "maturity": "user_defined", "prov": {"kind": "user_stated"}}) }
fn behaviour(sha: &str, params: Value, outputs: Value) -> Value { json!({"op": "part.setBehavior", "id": "ball", "behavior": {"kind": "fmu", "fmuSha256": sha, "parameters": params, "outputs": outputs}}) }

/// Creates a project, builds a model with the given ops, commits it, and returns (project id, revision number).
async fn committed(app: &Router, ops: Vec<Value>) -> (String, u64) {
    let (_, project) = call(app, "POST", "/v1/projects", Some(json!({"payload": {"name": "fmu"}}))).await;
    let pid = project["data"]["id"].as_str().unwrap().to_owned();
    let (_, build) = call(app, "POST", &format!("/v1/projects/{pid}/builds"), None).await;
    let bid = build["buildId"].as_str().unwrap().to_owned();
    let (s, r) = call(app, "POST", &format!("/v1/builds/{bid}/ops"), Some(json!({"ops": ops}))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let (s, rev) = call(app, "POST", &format!("/v1/builds/{bid}/commit"), None).await;
    assert_eq!(s, StatusCode::CREATED, "{rev}");
    (pid, rev["number"].as_u64().unwrap())
}

async fn all_frames(app: &Router, run_id: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut from = 0u64;
    loop {
        let (s, r) = call(app, "GET", &format!("/v1/runs/{run_id}/frames?from={from}&limit=1000"), None).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        out.extend(r["frames"].as_array().unwrap().iter().cloned());
        match r["next"].as_u64() { Some(n) => from = n, None => break }
    }
    out
}

#[tokio::test]
async fn a_model_with_an_fmu_behaviour_runs_through_fmpy_and_is_tied_to_its_revision() {
    let Some(e) = env("run") else { eprintln!("SKIPPED: needs HELQIX_TEST_DATABASE_URL, HELQIX_PYTHON and the Reference FMUs"); return };
    let app = app(&e, true).await;
    let (pid, n) = committed(&app, vec![ball_part(), behaviour(&e.sha, json!({"e": v(0.5, "one")}), json!(["h", "v"]))]).await;

    let req = json!({"endSeconds": 3.0, "stepSeconds": 0.01});
    let (s, r) = call(&app, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(req.clone())).await;
    assert_eq!(s, StatusCode::CREATED, "{r}");
    let run_id = r["runId"].as_str().unwrap().to_owned();
    assert_eq!(r["rows"], 301);
    assert!(r["determinism"].as_str().unwrap().to_lowercase().contains("tolerance"), "an FMU run is Tolerance, never Bitwise: {r}");
    assert_eq!(r["fmuSha256"], e.sha.as_str());

    // The stored frames are the FMU's own numbers: the same run made directly through the adapter gives the same values.
    let direct = FmiAdapter::new(e.python.clone(), &root().join("18-libraries/python"), std::fs::canonicalize(&e.fmus.join(format!("{}.fmu", e.sha))).unwrap(),
        vec![IoField { name: "h".into(), unit: ANY_UNIT.into() }, IoField { name: "v".into(), unit: ANY_UNIT.into() }], Duration::from_secs(120))
        .run(&json!({"parameters": {"e": 0.5}, "start_time": 0.0, "stop_time": 3.0, "step_size": 0.01})).unwrap();
    let frames = all_frames(&app, &run_id).await;
    assert_eq!(frames.len(), 301);
    for (i, f) in frames.iter().enumerate() {
        assert!((f["t"]["v"].as_f64().unwrap() - direct.outputs[0].values[i]).abs() < 1e-12);
        assert!((f["state"]["h"]["v"].as_f64().unwrap() - direct.outputs[1].values[i]).abs() < 1e-12, "h at frame {i}");
        assert!((f["state"]["v"]["v"].as_f64().unwrap() - direct.outputs[2].values[i]).abs() < 1e-12, "v at frame {i}");
    }
    assert_eq!((frames[0]["state"]["h"]["unit"].as_str(), frames[0]["state"]["v"]["unit"].as_str()), (Some("m"), Some("m/s")), "units are the FMU's, in SI");
    assert_eq!(frames[0]["state"]["h"]["origin"], "simulated");
    assert!((frames[0]["state"]["h"]["v"].as_f64().unwrap() - 1.0).abs() < 1e-12, "the ball starts at 1 m");

    // Tied to the revision, listed in the project's runs, and the same request gives the same run.
    let (_, list) = call(&app, "GET", &format!("/v1/projects/{pid}/runs"), None).await;
    assert_eq!((list["runs"].as_array().unwrap().len(), list["runs"][0]["revision"].as_u64(), list["runs"][0]["runId"].as_str()), (1, Some(n), Some(run_id.as_str())));
    let (s, again) = call(&app, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(req)).await;
    assert_eq!((s, again["runId"].as_str()), (StatusCode::CREATED, Some(run_id.as_str())));

    // An FMU cannot restore a state: freezing or branching its run is refused with the reason, not a wrong answer.
    let (s, r) = call(&app, "POST", &format!("/v1/runs/{run_id}/freeze"), Some(json!({"t": 1.0}))).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{r}");
    assert!(r["error"]["message"].as_str().unwrap_or("").contains("FMU"), "{r}");
    let _ = std::fs::remove_dir_all(&e.dir);
}

#[tokio::test]
async fn the_fmu_run_is_refused_for_the_right_reason() {
    let Some(e) = env("refuse") else { eprintln!("SKIPPED: needs HELQIX_TEST_DATABASE_URL, HELQIX_PYTHON and the Reference FMUs"); return };
    let app = app(&e, true).await;
    let req = json!({"endSeconds": 1.0, "stepSeconds": 0.1});
    let msg = |r: &Value| r["error"]["message"].as_str().unwrap_or("").to_owned();

    // The FMU is not in the library.
    let other = "b4".repeat(32);
    let (pid, n) = committed(&app, vec![ball_part(), behaviour(&other, json!({}), json!(["h"]))]).await;
    let (s, r) = call(&app, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(req.clone())).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{r}");
    assert!(msg(&r).contains(&other) && msg(&r).contains("library"), "{r}");

    // A parameter that measures another dimension than the FMU's own unit (e is dimensionless; a length is not).
    let (pid, n) = committed(&app, vec![ball_part(), behaviour(&e.sha, json!({"e": v(0.5, "m")}), json!(["h"]))]).await;
    let (s, r) = call(&app, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(req.clone())).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{r}");
    assert!(msg(&r).contains("dimension"), "{r}");

    // An output the FMU does not have.
    let (pid, n) = committed(&app, vec![ball_part(), behaviour(&e.sha, json!({}), json!(["no_such_output"]))]).await;
    let (s, r) = call(&app, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(req.clone())).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{r}");
    assert!(msg(&r).contains("no_such_output"), "{r}");

    // A bad time span.
    let (pid, n) = committed(&app, vec![ball_part(), behaviour(&e.sha, json!({}), json!(["h"]))]).await;
    for bad in [json!({"endSeconds": 0.0, "stepSeconds": 0.1}), json!({"endSeconds": 1.0, "stepSeconds": 0.0}), json!({"endSeconds": 1.0e9, "stepSeconds": 1.0e-6})] {
        let (s, r) = call(&app, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(bad.clone())).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "{bad} {r}");
    }

    // The file in the library is not the file its name promises: it is never run.
    std::fs::write(e.fmus.join(format!("{}.fmu", e.sha)), b"not the FMU any more").unwrap();
    let (s, r) = call(&app, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(req)).await;
    assert_ne!(s, StatusCode::CREATED, "{r}");
    assert!(msg(&r).contains("does not match"), "{r}");
    let _ = std::fs::remove_dir_all(&e.dir);
}

#[tokio::test]
async fn without_an_fmu_library_the_run_is_unavailable_not_a_wrong_point_mass() {
    let Some(e) = env("nofmu") else { eprintln!("SKIPPED: needs HELQIX_TEST_DATABASE_URL, HELQIX_PYTHON and the Reference FMUs"); return };
    let app = app(&e, false).await;
    let (pid, n) = committed(&app, vec![ball_part(), behaviour(&e.sha, json!({}), json!(["h"]))]).await;
    let (s, r) = call(&app, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(json!({"endSeconds": 1.0, "stepSeconds": 0.1}))).await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE, "{r}");
    let _ = std::fs::remove_dir_all(&e.dir);
}

#[tokio::test]
async fn the_fmu_library_lists_what_is_installed() {
    let Some(e) = env("list") else { eprintln!("SKIPPED: needs HELQIX_TEST_DATABASE_URL, HELQIX_PYTHON and the Reference FMUs"); return };
    let app = app(&e, true).await;
    let (s, r) = call(&app, "GET", "/v1/fmus", None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let fmus = r["fmus"].as_array().unwrap();
    assert_eq!(fmus.len(), 1);
    assert_eq!(fmus[0]["sha256"], e.sha.as_str());
    assert_eq!(fmus[0]["bytes"].as_u64(), Some(std::fs::metadata(ball_fmu()).unwrap().len()));
    // A file whose name is not its hash is not listed.
    std::fs::write(Path::new(&e.fmus).join("stray.fmu"), b"x").unwrap();
    // Nor is a file named like a hash whose content has another hash.
    std::fs::write(Path::new(&e.fmus).join(format!("{}.fmu", "c5".repeat(32))), b"x").unwrap();
    let (_, r) = call(&app, "GET", "/v1/fmus", None).await;
    assert_eq!(r["fmus"].as_array().unwrap().len(), 1, "{r}");
    let _ = std::fs::remove_dir_all(&e.dir);
}

#[tokio::test]
async fn when_every_fmu_run_slot_is_taken_the_answer_is_busy_not_a_queue() {
    let Some(e) = env("busy") else { eprintln!("SKIPPED: needs HELQIX_TEST_DATABASE_URL, HELQIX_PYTHON and the Reference FMUs"); return };
    let url = std::env::var("HELQIX_TEST_DATABASE_URL").unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new().max_connections(4).connect(&url).await.unwrap();
    let cfg = FmuConfig { store: e.fmus.clone(), python: e.python.clone(), package_dir: root().join("18-libraries/python"), timeout: Duration::from_secs(120) };
    let state = AppState::new(pool, KEY).with_frame_store(Some(e.dir.join("frames"))).with_run_metadata_in_postgres(true).with_fmu(Some(cfg)).with_fmu_slots(0);
    let app = routes::router(state, &["http://localhost:5173".to_owned()], Duration::from_secs(120)).unwrap();
    let (pid, n) = committed(&app, vec![ball_part(), behaviour(&e.sha, json!({}), json!(["h"]))]).await;
    let (s, r) = call(&app, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(json!({"endSeconds": 1.0, "stepSeconds": 0.1}))).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS, "{r}");
    let _ = std::fs::remove_dir_all(&e.dir);
}

#[tokio::test]
async fn an_installed_fmu_describes_itself_and_a_bad_request_is_refused() {
    let Some(e) = env("describe") else { eprintln!("SKIPPED: needs HELQIX_TEST_DATABASE_URL, HELQIX_PYTHON and the Reference FMUs"); return };
    let app = app(&e, true).await;
    let (s, d) = call(&app, "GET", &format!("/v1/fmus/{}", e.sha), None).await;
    assert_eq!(s, StatusCode::OK, "{d}");
    assert_eq!((d["sha256"].as_str(), d["modelName"].as_str(), d["fmiVersion"].as_str()), (Some(e.sha.as_str()), Some("BouncingBall"), Some("2.0")));
    let var = |n: &str| d["variables"].as_array().unwrap().iter().find(|v| v["name"] == n).unwrap().clone();
    let g = var("g");
    assert_eq!((g["causality"].as_str(), g["unit"].as_str(), g["start"].as_f64(), g["siFactor"].as_f64()), (Some("parameter"), Some("m/s2"), Some(-9.81), Some(1.0)));
    assert_eq!(g["dimension"], json!([1, 0, -2, 0, 0, 0, 0]));
    assert_eq!(var("h")["causality"], "output");
    assert!(d["defaultExperiment"]["stopTime"].is_number(), "{d}");

    // Not a hash, not installed, or changed since it was installed: refused, never run.
    for bad in ["../etc/passwd".to_owned(), "b4".repeat(32), "XYZ".to_owned()] {
        let (s, r) = call(&app, "GET", &format!("/v1/fmus/{bad}"), None).await;
        assert_ne!(s, StatusCode::OK, "{bad} {r}");
    }
    std::fs::write(e.fmus.join(format!("{}.fmu", e.sha)), b"not the FMU any more").unwrap();
    let (s, r) = call(&app, "GET", &format!("/v1/fmus/{}", e.sha), None).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{r}");
    assert!(r["error"]["message"].as_str().unwrap_or("").contains("does not match"), "{r}");
    let _ = std::fs::remove_dir_all(&e.dir);
}
