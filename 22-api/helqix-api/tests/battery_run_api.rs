//! E5.2 acceptance through the real router and PostgreSQL: a committed model whose part is a battery cell runs through the PyBaMM adapter, its frames
//! land in the frame store, the run is tied to the project and revision, and it carries the pack's label (NotVerified) with the published error.
//!
//! Needs HELQIX_TEST_DATABASE_URL and HELQIX_BATTERY_PYTHON (a Python with pybamm >=26.6,<27); without them each test prints SKIPPED and passes.
use std::{path::PathBuf, time::Duration};

use axum::{Router, body::Body, http::{Request, StatusCode}};
use helqix_adapters::{pybamm::PybammAdapter, solver::SolverAdapter};
use helqix_api::{battery_run::BatteryConfig, routes, state::AppState};
use serde_json::{Value, json};
use tower::ServiceExt;

const KEY: &str = "test-api-key-not-a-secret";
const PACK: &str = "lgm50_spm_chen2020";

fn root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..") }

struct Env { dir: PathBuf, python: PathBuf }

fn env(name: &str) -> Option<Env> {
    let python: PathBuf = std::env::var("HELQIX_BATTERY_PYTHON").ok()?.into();
    std::env::var("HELQIX_TEST_DATABASE_URL").ok()?;
    let dir = std::env::temp_dir().join(format!("helqix-battery-run-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Some(Env { dir, python })
}

fn cfg(e: &Env, library: PathBuf) -> BatteryConfig {
    BatteryConfig { python: e.python.clone(), package_dir: root().join("18-libraries/python"), library_dir: library, timeout: Duration::from_secs(300) }
}

async fn app_with(e: &Env, battery: Option<BatteryConfig>) -> Router {
    let url = std::env::var("HELQIX_TEST_DATABASE_URL").unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new().max_connections(4).connect(&url).await.expect("connect to test database");
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations");
    let state = AppState::new(pool, KEY).with_frame_store(Some(e.dir.join("frames"))).with_run_metadata_in_postgres(true).with_battery(battery);
    routes::router(state, &["http://localhost:5173".to_owned()], Duration::from_secs(300)).expect("router")
}

async fn app(e: &Env) -> Router { app_with(e, Some(cfg(e, root().join("18-libraries/battery")))).await }

async fn call(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri).header("authorization", format!("Bearer {KEY}"));
    if body.is_some() { req = req.header("content-type", "application/json"); }
    let res = app.clone().oneshot(req.body(body.map_or_else(Body::empty, |b| Body::from(b.to_string()))).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 8 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn v(x: f64, unit: &str) -> Value { json!({"v": x, "unit": unit, "origin": "user_stated", "status": "unverified"}) }
fn cell_part(id: &str) -> Value { json!({"op": "part.add", "id": id, "name": id, "geometry": {"kind": "primitive", "shape": "cylinder", "dims": [v(21.0, "mm"), v(70.0, "mm")]}, "maturity": "user_defined", "prov": {"kind": "user_stated"}}) }
fn cell_behaviour(id: &str, current_a: f64) -> Value {
    json!({"op": "part.setBehavior", "id": id, "behavior": {"kind": "battery_cell", "pack": PACK, "initialOcv": v(4.18, "V"), "contactResistance": v(0.0158, "ohm"), "load": {"kind": "constant_current", "current": v(current_a, "A")}}})
}

async fn committed(app: &Router, ops: Vec<Value>) -> (String, u64) {
    let (_, project) = call(app, "POST", "/v1/projects", Some(json!({"payload": {"name": "cell"}}))).await;
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
    let (s, r) = call(app, "GET", &format!("/v1/runs/{run_id}/frames?limit=1000"), None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    r["frames"].as_array().unwrap().clone()
}

const SKIP: &str = "SKIPPED: needs HELQIX_TEST_DATABASE_URL and HELQIX_BATTERY_PYTHON (a Python with pybamm)";

#[tokio::test]
async fn a_battery_cell_model_runs_through_pybamm_and_equals_a_direct_run_of_the_pack() {
    let Some(e) = env("run") else { eprintln!("{SKIP}"); return };
    let app = app(&e).await;
    let (pid, n) = committed(&app, vec![cell_part("cell"), cell_behaviour("cell", 5.0)]).await;

    let req = json!({"endSeconds": 600.0, "stepSeconds": 30.0});
    let (s, r) = call(&app, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(req.clone())).await;
    assert_eq!(s, StatusCode::CREATED, "{r}");
    let run_id = r["runId"].as_str().unwrap().to_owned();
    assert_eq!(r["rows"], 21);
    assert!(r["determinism"].as_str().unwrap().to_lowercase().contains("tolerance"), "{r}");
    // The pack's label travels with the run: NotVerified, with the published held-out error and the limit it failed.
    assert_eq!(r["pack"]["id"], PACK);
    assert_eq!(r["pack"]["label"], "NotVerified", "{r}");
    assert_eq!(r["pack"]["thresholdMv"].as_f64(), Some(46.0));
    let errs: Vec<f64> = r["pack"]["heldOutRmseMv"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
    assert_eq!(errs.len(), 3);
    assert!(errs.iter().all(|x| *x > 46.0 && *x < 80.0), "{errs:?}");
    assert_eq!(r["modelIdentitySha256"].as_str().unwrap().len(), 64);

    // The reference: the pack run directly with the same numbers. The Studio-built model reproduces it.
    let time: Vec<f64> = (0..=20).map(|i| f64::from(i) * 30.0).collect();
    let direct = PybammAdapter::new(e.python.clone(), root().join("18-libraries/python"), Duration::from_secs(300))
        .run(&json!({"schema": "helqix.pybamm-input.v1", "time_s": time, "current_a": vec![5.0; 21], "initial_ocv_v": 4.18, "contact_resistance_ohm": 0.0158})).unwrap();
    let frames = all_frames(&app, &run_id).await;
    assert_eq!(frames.len(), 21);
    let volts = &direct.outputs.iter().find(|o| o.name == "terminal_voltage").unwrap().values;
    let cap = &direct.outputs.iter().find(|o| o.name == "discharge_capacity").unwrap().values;
    for (i, f) in frames.iter().enumerate() {
        assert!((f["t"]["v"].as_f64().unwrap() - time[i]).abs() < 1e-12);
        assert!((f["state"]["terminal_voltage"]["v"].as_f64().unwrap() - volts[i]).abs() < 1e-9, "voltage at frame {i}");
        assert!((f["state"]["discharge_capacity"]["v"].as_f64().unwrap() - cap[i]).abs() < 1e-9, "capacity at frame {i}");
    }
    // Physics that does not depend on the model: charge conservation, a falling voltage inside the cell's window.
    for (i, f) in frames.iter().enumerate() {
        let q = f["state"]["discharge_capacity"]["v"].as_f64().unwrap();
        assert!((q - 5.0 * time[i] / 3600.0).abs() < 1e-6, "charge conservation at frame {i}: {q}");
    }
    let vs: Vec<f64> = frames.iter().map(|f| f["state"]["terminal_voltage"]["v"].as_f64().unwrap()).collect();
    assert!(vs.windows(2).all(|w| w[1] <= w[0] + 1e-9) && vs.iter().all(|x| (2.5..=4.25).contains(x)), "{vs:?}");
    assert_eq!((frames[0]["state"]["terminal_voltage"]["unit"].as_str(), frames[0]["state"]["discharge_capacity"]["unit"].as_str()), (Some("V"), Some("A.h")));
    assert_eq!(frames[0]["state"]["terminal_voltage"]["status"], "unverified");

    // Tied to the revision, the same request gives the same run, and a pack run cannot be frozen.
    let (_, list) = call(&app, "GET", &format!("/v1/projects/{pid}/runs"), None).await;
    assert_eq!((list["runs"].as_array().unwrap().len(), list["runs"][0]["revision"].as_u64(), list["runs"][0]["runId"].as_str()), (1, Some(n), Some(run_id.as_str())));
    let (s, again) = call(&app, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(req)).await;
    assert_eq!((s, again["runId"].as_str()), (StatusCode::CREATED, Some(run_id.as_str())));
    let (s, r) = call(&app, "POST", &format!("/v1/runs/{run_id}/freeze"), Some(json!({"t": 300.0}))).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{r}");
    let _ = std::fs::remove_dir_all(&e.dir);
}

#[tokio::test]
async fn a_different_current_is_a_different_run_and_a_deeper_discharge() {
    let Some(e) = env("current") else { eprintln!("{SKIP}"); return };
    let app = app(&e).await;
    let last_voltage = |frames: &[Value]| frames.last().unwrap()["state"]["terminal_voltage"]["v"].as_f64().unwrap();
    let mut seen = Vec::new();
    for current in [1.0, 5.0] {
        let (pid, n) = committed(&app, vec![cell_part("cell"), cell_behaviour("cell", current)]).await;
        let (s, r) = call(&app, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(json!({"endSeconds": 600.0, "stepSeconds": 60.0}))).await;
        assert_eq!(s, StatusCode::CREATED, "{r}");
        let id = r["runId"].as_str().unwrap().to_owned();
        seen.push((id.clone(), last_voltage(&all_frames(&app, &id).await)));
    }
    assert_ne!(seen[0].0, seen[1].0);
    assert!(seen[1].1 < seen[0].1, "5 A sags further than 1 A in the same time: {seen:?}");
    let _ = std::fs::remove_dir_all(&e.dir);
}

#[tokio::test]
async fn the_battery_run_is_refused_for_the_right_reason() {
    let Some(e) = env("refuse") else { eprintln!("{SKIP}"); return };
    let app = app(&e).await;
    let (pid, n) = committed(&app, vec![cell_part("cell"), cell_behaviour("cell", 2.0)]).await;
    let msg = |r: &Value| r["error"]["message"].as_str().unwrap_or("").to_owned();
    for bad in [json!({"endSeconds": 0.0, "stepSeconds": 30.0}), json!({"endSeconds": 600.0, "stepSeconds": 0.0}), json!({"endSeconds": 1.0e9, "stepSeconds": 1.0})] {
        let (s, r) = call(&app, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(bad.clone())).await;
        assert!(s.is_client_error(), "{bad} {s} {r}");
    }
    // A duration that is not a whole number of steps still has a sample count the pack can take; fewer than two samples is refused.
    let (s, r) = call(&app, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(json!({"endSeconds": 10.0, "stepSeconds": 30.0}))).await;
    assert!(s.is_client_error(), "{s} {r}");
    // Two behaviours in one model: refused, both parts named.
    let two = vec![cell_part("a"), cell_part("b"), cell_behaviour("a", 1.0), cell_behaviour("b", 1.0)];
    let (pid2, n2) = committed(&app, two).await;
    let (s, r) = call(&app, "POST", &format!("/v1/projects/{pid2}/revisions/{n2}/run"), Some(json!({"endSeconds": 60.0, "stepSeconds": 30.0}))).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{r}");
    assert!(msg(&r).contains("a (a)") && msg(&r).contains("b (b)") && msg(&r).contains("only one"), "{r}");
    let _ = std::fs::remove_dir_all(&e.dir);
}

#[tokio::test]
async fn without_the_pack_or_its_label_the_run_is_unavailable_never_served_unlabelled() {
    let Some(e) = env("nolabel") else { eprintln!("{SKIP}"); return };
    let req = json!({"endSeconds": 60.0, "stepSeconds": 30.0});
    // No battery configuration at all.
    let app_none = app_with(&e, None).await;
    let (pid, n) = committed(&app_none, vec![cell_part("cell"), cell_behaviour("cell", 2.0)]).await;
    let (s, r) = call(&app_none, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(req.clone())).await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE, "{r}");
    let (s, _) = call(&app_none, "GET", "/v1/packs", None).await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
    // A library directory without the pack's entry: the label is missing, so nothing is run.
    let empty = e.dir.join("empty-library");
    std::fs::create_dir_all(&empty).unwrap();
    let app_empty = app_with(&e, Some(cfg(&e, empty))).await;
    let (s, r) = call(&app_empty, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(req.clone())).await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE, "{r}");
    // An entry that exists but has lost its label or its published error is no better: nothing is run.
    for strip in ["label", "held_out_errors", "requirement"] {
        let mut entry: Value = serde_json::from_slice(&std::fs::read(root().join(format!("18-libraries/battery/{PACK}.library-entry.json"))).unwrap()).unwrap();
        entry.as_object_mut().unwrap().remove(strip);
        let lib = e.dir.join(format!("library-without-{strip}"));
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::write(lib.join(format!("{PACK}.library-entry.json")), entry.to_string()).unwrap();
        let app_cut = app_with(&e, Some(cfg(&e, lib))).await;
        let (s, r) = call(&app_cut, "POST", &format!("/v1/projects/{pid}/revisions/{n}/run"), Some(req.clone())).await;
        assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE, "without {strip}: {r}");
    }
    let _ = std::fs::remove_dir_all(&e.dir);
}

#[tokio::test]
async fn the_pack_list_says_what_is_installed_and_how_far_to_trust_it() {
    let Some(e) = env("packs") else { eprintln!("{SKIP}"); return };
    let app = app(&e).await;
    let (s, r) = call(&app, "GET", "/v1/packs", None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let packs = r["packs"].as_array().unwrap();
    assert_eq!(packs.len(), 1);
    assert_eq!((packs[0]["id"].as_str(), packs[0]["label"].as_str()), (Some(PACK), Some("NotVerified")));
    assert_eq!(packs[0]["thresholdMv"].as_f64(), Some(46.0));
    assert!(packs[0]["contextOfUse"].as_str().unwrap().contains("LG M50"), "{r}");
    assert!(packs[0]["calibratedContactResistanceOhm"].as_f64().is_some_and(|x| (x - 0.015805).abs() < 1e-5), "the calibrated resistance is offered, not applied: {r}");
    let _ = std::fs::remove_dir_all(&e.dir);
}
