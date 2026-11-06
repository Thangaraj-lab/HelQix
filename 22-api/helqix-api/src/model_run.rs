//! Run a committed model revision, and list the runs of a project.
//!
//! The translation from model to solver lives in `helqix_modelbuild::lumped` (the engine layer); this module only
//! reads the revision's active operations, asks it for the lumped body, and stores the run linked to the project
//! and revision. A model that cannot be translated is refused with the parts and reasons named.
use axum::{Json, extract::{Extension, Path, State}, http::StatusCode};
use helqix_modelbuild::{lumped::lumped_body, plan::{RunPlan, run_plan as plan_for}};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{auth::Identity, battery_run::run_plan as run_battery, error::ApiError, fmu_run::run_plan as run_fmu, frames::{StartRun, store_run}, specs::{ModelLink, StoredParams, StoredSpec}, state::AppState};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RunRevision {
    /// Needed by the built-in point-mass body only.
    #[serde(default)] pub drag_coefficient: Option<f64>,
    pub end_seconds: f64,
    #[serde(default = "step")] pub step_seconds: f64,
    /// x, y, z (m) and vx, vy, vz (m/s) of the lumped body at t = 0. Needed by the built-in point-mass body only.
    #[serde(default)] pub initial_state: Option<[f64; 6]>,
}
fn step() -> f64 { 0.001 }

const MAX_STEPS: f64 = 200_000.0;

/// `POST /v1/projects/{project_id}/revisions/{number}/run`
///
/// A model with a part that has an FMU behaviour runs that FMU through FMPy; any other model runs as one lumped body.
pub async fn run_revision(State(state): State<AppState>, Extension(who): Extension<Identity>, Path((project_id, number)): Path<(String, i32)>, Json(b): Json<RunRevision>) -> Result<(StatusCode, Json<Value>), ApiError> {
    let dir = state.frame_store.clone().ok_or(ApiError::Unavailable)?;
    let pid = Uuid::parse_str(&project_id).map_err(|_| ApiError::BadRequest("id must be a UUID".into()))?;
    if !(b.end_seconds.is_finite() && b.step_seconds.is_finite() && b.end_seconds > 0.0 && b.step_seconds > 0.0 && b.end_seconds / b.step_seconds <= MAX_STEPS) {
        return Err(ApiError::BadRequest(format!("end and step must be positive finite numbers, at most {MAX_STEPS} steps")));
    }
    let ops = crate::builds::revision_ops(&state, pid, number).await?;
    let link = ModelLink { project_id: pid.to_string(), revision: number as u32 };
    let specs = state.spec_store().ok_or(ApiError::Unavailable)?;
    let reject = |missing: Vec<helqix_modelbuild::lumped::Missing>| {
        let list: Vec<String> = missing.iter().map(|m| if m.part.is_empty() { m.reason.clone() } else { format!("{}: {}", m.part, m.reason) }).collect();
        ApiError::Rejected(format!("this model cannot be run yet: {}", list.join("; ")))
    };

    match plan_for(&ops).map_err(reject)? {
        RunPlan::Fmu(plan) => {
            let cfg = state.fmu.clone().ok_or(ApiError::Unavailable)?;
            let sha = plan.fmu_sha256.clone();
            let (id, entry, spec) = run_fmu(&state, cfg, plan, b.end_seconds, b.step_seconds, link.clone()).await?;
            specs.save(&id, &StoredSpec { params: StoredParams::Fmu(spec), parent: None, model: Some(link), owner: Some(who.owner().to_owned()) }).await?;
            if let Some(m) = &state.mirror { m.push(&dir, &id).await?; }
            return Ok((StatusCode::CREATED, Json(json!({"runId": id, "rows": entry.meta.rows, "framesBlake3": entry.frames_blake3, "determinism": entry.meta.determinism, "fmuSha256": sha, "parts": 1}))));
        }
        RunPlan::Battery(plan) => {
            let cfg = state.battery.clone().ok_or(ApiError::Unavailable)?;
            let (id, entry, spec, pack) = run_battery(&state, cfg, plan, b.end_seconds, b.step_seconds, link.clone()).await?;
            let identity = spec.model_identity_sha256.clone();
            specs.save(&id, &StoredSpec { params: StoredParams::Battery(spec), parent: None, model: Some(link), owner: Some(who.owner().to_owned()) }).await?;
            if let Some(m) = &state.mirror { m.push(&dir, &id).await?; }
            return Ok((StatusCode::CREATED, Json(json!({"runId": id, "rows": entry.meta.rows, "framesBlake3": entry.frames_blake3, "determinism": entry.meta.determinism, "pack": pack, "modelIdentitySha256": identity, "parts": 1}))));
        }
        RunPlan::PointMass => {}
    }

    let (Some(drag_coefficient), Some(initial_state)) = (b.drag_coefficient, b.initial_state) else {
        return Err(ApiError::BadRequest("a model without a behaviour (an FMU or a battery cell) runs as a point mass: dragCoefficient and initialState are required".into()));
    };
    let body = lumped_body(&ops).map_err(reject)?;
    let run = StartRun {
        mass_kg: body.mass_kg, gravity_m_s2: [0.0, -9.80665, 0.0], air_density_kg_m3: 1.225, wind_m_s: [0.0; 3], applied_force_n: [0.0; 3],
        drag_coefficient, reference_area_m2: body.area_m2, end_seconds: b.end_seconds, step_seconds: b.step_seconds, initial_state,
    };
    let (id, entry) = store_run(dir.clone(), &specs, run, Some(link), Some(who.owner().to_owned())).await?;
    if let Some(m) = &state.mirror { m.push(&dir, &id).await?; }
    Ok((StatusCode::CREATED, Json(json!({"runId": id, "rows": entry.meta.rows, "framesBlake3": entry.frames_blake3, "massKg": body.mass_kg, "areaM2": body.area_m2, "parts": body.parts}))))
}

/// `GET /v1/projects/{project_id}/runs`: the runs made from this project's revisions, newest first.
pub async fn project_runs(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<Value>, ApiError> {
    let specs = state.spec_store().ok_or(ApiError::Unavailable)?;
    let pid = Uuid::parse_str(&project_id).map_err(|_| ApiError::BadRequest("id must be a UUID".into()))?.to_string();
    let runs = specs.project_runs(&pid).await?;
    let runs: Vec<Value> = runs.into_iter().map(|r| json!({"runId": r.run_id, "revision": r.revision, "parentRunId": r.parent_run_id, "startedAt": r.started_at})).collect();
    Ok(Json(json!({"runs": runs})))
}
