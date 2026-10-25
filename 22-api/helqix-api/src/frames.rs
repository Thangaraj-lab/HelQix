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
