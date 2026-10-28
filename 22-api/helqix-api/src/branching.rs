//! Freeze and branch over stored runs (the point-mass engine).
//! - `POST /v1/runs/{id}/freeze {t}`: the checkpoint at the grid point at or before `t`.
//! - `POST /v1/branches {checkpointId, overrides}`: the run continued from that checkpoint with changed parameters.
//!
//! A run started through `POST /v1/runs` has its parameters saved (see `specs.rs`), so the
//! engine can reproduce any point of it exactly. A checkpoint is therefore just `run@index` plus the state hash;
//! nothing else is stored. Runs recorded some other way have no saved parameters and are refused.
use axum::{Json, extract::{Path, State}, http::StatusCode};
use helqix_frames::{FrameStore, point_mass_variables};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::{error::ApiError, frames::{map, to_spec}, specs::{Parent, StoredSpec}, state::AppState};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreezeBody { pub t: f64 }

pub async fn freeze_run(State(state): State<AppState>, Path(id): Path<String>, Json(b): Json<FreezeBody>) -> Result<Json<Value>, ApiError> {
    let dir = state.frame_store.clone().ok_or(ApiError::Unavailable)?;
    if !b.t.is_finite() || b.t < 0.0 { return Err(ApiError::BadRequest("t must be a finite time of at least 0".into())); }
    let stored = state.spec_store().ok_or(ApiError::Unavailable)?.load(&id).await?;
    let out = tokio::task::spawn_blocking(move || -> Result<Value, ApiError> {
        let run = FrameStore::open(&dir).and_then(|s| s.load(&id)).map_err(map)?;
        let t = b.t.min(stored.params.end_seconds);
        let index = ((t / stored.params.step_seconds) + 1e-9).floor() as usize;
        let frame = run.frames().get(index).ok_or_else(|| ApiError::Rejected("no such grid point".into()))?;
        let state: Map<String, Value> = ["x", "y", "z", "vx", "vy", "vz"].iter().zip(&frame.state).map(|(n, v)| ((*n).to_owned(), json!(v))).collect();
        Ok(json!({"checkpointId": format!("{id}@{index}"), "runId": id, "index": index, "t": frame.t, "hash": frame.link, "state": state}))
    }).await.map_err(|_| ApiError::Internal)??;
    Ok(Json(out))
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Overrides {
    pub mass_kg: Option<f64>,
    pub drag_coefficient: Option<f64>,
    pub reference_area_m2: Option<f64>,
    pub wind_m_s: Option<[f64; 3]>,
    pub applied_force_n: Option<[f64; 3]>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BranchBody { pub checkpoint_id: String, pub overrides: Overrides }

pub async fn branch_run(State(state): State<AppState>, Json(b): Json<BranchBody>) -> Result<(StatusCode, Json<Value>), ApiError> {
    let dir = state.frame_store.clone().ok_or(ApiError::Unavailable)?;
    let (parent_id, index) = b.checkpoint_id.rsplit_once('@').and_then(|(r, i)| i.parse::<usize>().ok().map(|i| (r.to_owned(), i)))
        .ok_or_else(|| ApiError::BadRequest("checkpointId must look like run@index".into()))?;
    let o = b.overrides;
    let scalars: Vec<f64> = [o.mass_kg, o.drag_coefficient, o.reference_area_m2].into_iter().flatten().collect();
    let vectors: Vec<f64> = o.wind_m_s.into_iter().chain(o.applied_force_n).flatten().collect();
    if scalars.is_empty() && vectors.is_empty() { return Err(ApiError::BadRequest("change at least one parameter".into())); }
    if !scalars.iter().chain(&vectors).all(|v| v.is_finite()) { return Err(ApiError::BadRequest("every number must be finite".into())); }
    if o.mass_kg.is_some_and(|m| m <= 0.0) || o.drag_coefficient.is_some_and(|c| c < 0.0) || o.reference_area_m2.is_some_and(|a| a < 0.0) {
        return Err(ApiError::BadRequest("mass must be positive; drag coefficient and area must not be negative".into()));
    }
    let id = format!("br-{}", &helqix_frames::blake3_hex(format!("{}|{:?}", b.checkpoint_id, (o.mass_kg, o.drag_coefficient, o.reference_area_m2, o.wind_m_s, o.applied_force_n)).as_bytes())[..16]);
    let rid = id.clone();
    let specs = state.spec_store().ok_or(ApiError::Unavailable)?;
    let parent = specs.load(&parent_id).await?;
    if parent.parent.is_some() { return Err(ApiError::Rejected("branch from an original run, not from another branch".into())); }
    let parent_model = parent.model.clone();
    let parent_run = parent_id.clone();
    let (entry, changed) = tokio::task::spawn_blocking(move || -> Result<(helqix_frames::RunIndex, crate::frames::StartRun), ApiError> {
        let original = helqix_freeze::run(&to_spec(&parent.params)).map_err(|e| ApiError::Rejected(e.to_string()))?;
        if index >= original.samples.len() { return Err(ApiError::Rejected("checkpoint is outside the run".into())); }
        let mut changed = parent.params.clone();
        if let Some(v) = o.mass_kg { changed.mass_kg = v; }
        if let Some(v) = o.drag_coefficient { changed.drag_coefficient = v; }
        if let Some(v) = o.reference_area_m2 { changed.reference_area_m2 = v; }
        if let Some(v) = o.wind_m_s { changed.wind_m_s = v; }
        if let Some(v) = o.applied_force_n { changed.applied_force_n = v; }
        let prior = if index == 0 { original.genesis.clone() } else { original.chain[index - 1].clone() };
        let tail = helqix_freeze::continue_run(&to_spec(&changed), index, &original.samples[index].state, &prior).map_err(|e| ApiError::Rejected(e.to_string()))?;
        let mut samples = original.samples[..index].to_vec();
        let mut chain = original.chain[..index].to_vec();
        samples.extend(tail.samples);
        chain.extend(tail.chain);
        let whole = helqix_freeze::RecordedRun { genesis: original.genesis.clone(), samples, chain, determinism: tail.determinism };
        let entry = FrameStore::open(&dir).and_then(|s| s.put(&rid, &whole, &point_mass_variables(), &original.genesis)).map_err(map)?;
        Ok((entry, changed))
    }).await.map_err(|_| ApiError::Internal)??;
    specs.save(&id, &StoredSpec { params: changed, parent: Some(Parent { run_id: parent_run.clone(), index }), model: parent_model }).await?;
    Ok((StatusCode::CREATED, Json(json!({"runId": id, "parentRunId": parent_run, "index": index, "rows": entry.meta.rows, "framesBlake3": entry.frames_blake3}))))
}
