//! Read-only frame routes (K4) over a content-addressed frame store (`helqix-frames`). Every read
//! re-verifies the file's BLAKE3 address and the state chain; nothing here writes.
//! - `GET /v1/runs/{id}/frames?from=&limit=`: stored frames in order (`runFrames`), default limit 1000, max 10000.
//! - `GET /v1/runs/{id}/frame?t=`: the frame at time t (`getFrame`): exact on the grid (Origin simulated),
//!   linear between frames (Origin derived), 422 outside the run.
use axum::{Json, extract::{Path, Query, State}};
use helqix_frames::{FrameError, FrameStore, StoredRun};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{error::ApiError, state::AppState};

#[derive(Debug, Deserialize)]
pub struct FramesQuery { #[serde(default)] pub from: Option<u64>, #[serde(default)] pub limit: Option<usize> }

#[derive(Debug, Deserialize)]
pub struct FrameQuery { pub t: f64 }

fn map(e: FrameError) -> ApiError {
    match e {
        FrameError::NotFound(_) => ApiError::NotFound,
        FrameError::OutOfRange(m) => ApiError::Rejected(m),
        FrameError::Format(m) if m.starts_with("run id") => ApiError::BadRequest(m),
        FrameError::Integrity(m) => { tracing::error!(%m, "stored frames failed verification"); ApiError::StateConflict(format!("stored frames failed verification: {m}")) }
        other => { tracing::error!(error = %other, "frame store error"); ApiError::Internal }
    }
}

async fn load(state: &AppState, id: String) -> Result<StoredRun, ApiError> {
    let dir = state.frame_store.clone().ok_or(ApiError::Unavailable)?;
    tokio::task::spawn_blocking(move || FrameStore::open(dir).and_then(|s| s.load(&id))).await.map_err(|_| ApiError::Internal)?.map_err(map)
}

/// `GET /v1/runs`: the stored runs (id, rows, determinism, variables with units). The frames are
/// read with `/v1/runs/{id}/frames`; nothing here computes a run.
pub async fn list_runs(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let dir = state.frame_store.clone().ok_or(ApiError::Unavailable)?;
    let runs = tokio::task::spawn_blocking(move || FrameStore::open(dir).and_then(|s| s.list())).await.map_err(|_| ApiError::Internal)?.map_err(map)?;
    let runs: Vec<Value> = runs.into_iter().map(|r| json!({"runId": r.meta.run_id, "rows": r.meta.rows, "determinism": r.meta.determinism, "variables": r.meta.variables, "framesBlake3": r.frames_blake3})).collect();
    Ok(Json(json!({"runs": runs})))
}

pub async fn run_frames(State(state): State<AppState>, Path(id): Path<String>, Query(q): Query<FramesQuery>) -> Result<Json<Value>, ApiError> {
    let limit = q.limit.unwrap_or(1000);
    if limit == 0 || limit > 10_000 { return Err(ApiError::BadRequest("limit must be 1..10000".into())); }
    let run = load(&state, id).await?;
    let from = q.from.unwrap_or_else(|| run.frames().first().map_or(0, |f| f.index));
    let frames: Vec<_> = run.run_frames().filter(|f| f.index >= from).take(limit).collect();
    let next = frames.last().map(|f| f.index + 1).filter(|n| run.frames().last().is_some_and(|l| *n <= l.index));
    Ok(Json(json!({"runId": run.meta.run_id, "framesBlake3": run.frames_blake3, "rows": run.meta.rows, "determinism": run.meta.determinism, "frames": frames, "next": next})))
}

pub async fn get_frame(State(state): State<AppState>, Path(id): Path<String>, Query(q): Query<FrameQuery>) -> Result<Json<Value>, ApiError> {
    let run = load(&state, id).await?;
    let frame = run.get_frame(q.t).map_err(map)?;
    Ok(Json(json!({"framesBlake3": run.frames_blake3, "frame": frame})))
}

/// Body of `POST /v1/runs`: the point-mass model the engine can run today. Unknown fields are refused.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StartRun {
    pub mass_kg: f64,
    #[serde(default = "default_gravity")] pub gravity_m_s2: [f64; 3],
    #[serde(default = "default_density")] pub air_density_kg_m3: f64,
    #[serde(default)] pub wind_m_s: [f64; 3],
    #[serde(default)] pub applied_force_n: [f64; 3],
    pub drag_coefficient: f64,
    pub reference_area_m2: f64,
    pub end_seconds: f64,
    #[serde(default = "default_step")] pub step_seconds: f64,
    /// x, y, z, vx, vy, vz
    pub initial_state: [f64; 6],
}
fn default_gravity() -> [f64; 3] { [0.0, -9.80665, 0.0] }
fn default_density() -> f64 { 1.225 }
fn default_step() -> f64 { 0.001 }

const MAX_STEPS: f64 = 200_000.0;

/// `POST /v1/runs`: runs the engine and stores the frames. The run id is derived from the request, so the
/// same request gives the same id and the same frames (the engine is deterministic); repeating it is harmless.
pub async fn start_run(State(state): State<AppState>, Json(b): Json<StartRun>) -> Result<(axum::http::StatusCode, Json<Value>), ApiError> {
    let dir = state.frame_store.clone().ok_or(ApiError::Unavailable)?;
    let all = [b.mass_kg, b.air_density_kg_m3, b.drag_coefficient, b.reference_area_m2, b.end_seconds, b.step_seconds];
    if !all.iter().chain(&b.gravity_m_s2).chain(&b.wind_m_s).chain(&b.applied_force_n).chain(&b.initial_state).all(|v| v.is_finite()) { return Err(ApiError::BadRequest("every number must be finite".into())); }
    if b.mass_kg <= 0.0 || b.reference_area_m2 < 0.0 || b.drag_coefficient < 0.0 || b.air_density_kg_m3 < 0.0 { return Err(ApiError::BadRequest("mass must be positive; drag coefficient, area and density must not be negative".into())); }
    if b.step_seconds <= 0.0 || b.end_seconds <= 0.0 || b.end_seconds / b.step_seconds > MAX_STEPS { return Err(ApiError::BadRequest(format!("end/step must be positive and at most {MAX_STEPS} steps"))); }
    let id = format!("run-{}", &helqix_frames::blake3_hex(format!("{:?}", (&b.mass_kg, &b.gravity_m_s2, &b.air_density_kg_m3, &b.wind_m_s, &b.applied_force_n, &b.drag_coefficient, &b.reference_area_m2, &b.end_seconds, &b.step_seconds, &b.initial_state)).as_bytes())[..16]);
    let spec = helqix_freeze::RunSpec {
        model: helqix_freeze::PointMassSpec { mass_kg: b.mass_kg, gravity_m_s2: b.gravity_m_s2, air_density_kg_m3: b.air_density_kg_m3, wind_m_s: b.wind_m_s, applied_force_n: b.applied_force_n, drag_coefficient: b.drag_coefficient, reference_area_m2: b.reference_area_m2 },
        start_seconds: 0.0, end_seconds: b.end_seconds, step_seconds: b.step_seconds, initial_state: b.initial_state.to_vec(), seeds: vec![],
    };
    let rid = id.clone();
    let entry = tokio::task::spawn_blocking(move || -> Result<_, String> {
        let r = helqix_freeze::run(&spec).map_err(|e| e.to_string())?;
        FrameStore::open(dir).and_then(|s| s.put(&rid, &r, &helqix_frames::point_mass_variables(), &helqix_freeze::chain_genesis(&spec))).map_err(|e| e.to_string())
    }).await.map_err(|_| ApiError::Internal)?.map_err(ApiError::Rejected)?;
    Ok((axum::http::StatusCode::CREATED, Json(json!({"runId": id, "rows": entry.meta.rows, "framesBlake3": entry.frames_blake3, "determinism": entry.meta.determinism}))))
}
