//! Staged builds over HTTP (K2). The engine logic is `helqix-modelbuild`; this module stores the
//! op log and cursor in PostgreSQL, appends BuildEvents, and streams them over SSE. Every mutation
//! runs in one transaction holding the build row lock, so concurrent requests serialize.
use std::{convert::Infallible, time::Duration};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, sse::{Event, KeepAlive, Sse}},
};
use futures_util::stream::{self, Stream};
use helqix_contracts::{BuildEvent, BuildEventKind, ModelOp, ModelRevision};
use helqix_modelbuild::{BuildError, Emitted, StagedBuild};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction, types::Json as Jsonb};
use uuid::Uuid;

use crate::{error::ApiError, state::AppState, store::{self, ResourceKind}};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpsBody {
    pub ops: Vec<ModelOp>,
}

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    #[serde(default)]
    pub after: Option<i64>,
}

fn parse_id(text: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(text).map_err(|_| ApiError::BadRequest("id must be a UUID".into()))
}

fn build_error(error: BuildError) -> ApiError {
    match error {
        BuildError::Op { .. } => ApiError::Rejected(error.to_string()),
        _ => ApiError::StateConflict(error.to_string()),
    }
}

/// `POST /v1/projects/{project_id}/builds`: starts an empty staged build for an existing project.
pub async fn start(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<impl IntoResponse, ApiError> {
    let project_id = parse_id(&project_id)?;
    store::get(&state.database, ResourceKind::Project, project_id).await?;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO helqix_builds (id, project_id, status, op_cursor) VALUES ($1, $2, 'staged', 0)")
        .bind(id).bind(project_id).execute(&state.database).await?;
    Ok((StatusCode::CREATED, Json(json!({"buildId": id, "projectId": project_id, "status": "staged"}))))
}

async fn load(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<(Uuid, StagedBuild), ApiError> {
    let (project_id, status, cursor): (Uuid, String, i32) = sqlx::query_as("SELECT project_id, status, op_cursor FROM helqix_builds WHERE id = $1 FOR UPDATE")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(ApiError::NotFound)?;
    let ops: Vec<Jsonb<ModelOp>> = sqlx::query_scalar("SELECT op FROM helqix_build_ops WHERE build_id = $1 ORDER BY position")
        .bind(id).fetch_all(&mut **tx).await?;
    let build = StagedBuild::restore(ops.into_iter().map(|j| j.0).collect(), usize::try_from(cursor).map_err(|_| ApiError::Internal)?, status == "committed")
        .map_err(|error| {
            tracing::error!(%error, build = %id, "stored build does not replay");
            ApiError::Internal
        })?;
    Ok((project_id, build))
}

async fn save(tx: &mut Transaction<'_, Postgres>, id: Uuid, build: &StagedBuild) -> Result<(), ApiError> {
    sqlx::query("DELETE FROM helqix_build_ops WHERE build_id = $1").bind(id).execute(&mut **tx).await?;
    for (position, op) in build.ops().iter().enumerate() {
        sqlx::query("INSERT INTO helqix_build_ops (build_id, position, op) VALUES ($1, $2, $3)")
            .bind(id).bind(i32::try_from(position).map_err(|_| ApiError::Internal)?).bind(Jsonb(op)).execute(&mut **tx).await?;
    }
    sqlx::query("UPDATE helqix_builds SET op_cursor = $2, status = $3, updated_at = now() WHERE id = $1")
        .bind(id).bind(i32::try_from(build.cursor()).map_err(|_| ApiError::Internal)?)
        .bind(if build.is_committed() { "committed" } else { "staged" })
        .execute(&mut **tx).await?;
    Ok(())
}

async fn append_events(tx: &mut Transaction<'_, Postgres>, id: Uuid, emitted: Vec<Emitted>) -> Result<Vec<BuildEvent>, ApiError> {
    let last: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) FROM helqix_build_events WHERE build_id = $1").bind(id).fetch_one(&mut **tx).await?;
    let ts = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mut out = Vec::with_capacity(emitted.len());
    for (offset, e) in emitted.into_iter().enumerate() {
        let event = BuildEvent { seq: (last + 1 + offset as i64) as u64, build_id: id.to_string(), ts: ts.clone(), kind: e.kind, step: None, op: e.op, issue: e.issue, question: None, note: e.note };
        sqlx::query("INSERT INTO helqix_build_events (build_id, seq, event) VALUES ($1, $2, $3)")
            .bind(id).bind(event.seq as i64).bind(Jsonb(&event)).execute(&mut **tx).await?;
        out.push(event);
    }
    Ok(out)
}

async fn mutate(state: &AppState, id: &str, change: impl FnOnce(&mut StagedBuild) -> Result<Vec<Emitted>, BuildError>) -> Result<Json<Value>, ApiError> {
    let id = parse_id(id)?;
    let mut tx = state.database.begin().await?;
    let (_, mut build) = load(&mut tx, id).await?;
    let emitted = change(&mut build).map_err(build_error)?;
    save(&mut tx, id, &build).await?;
    let events = append_events(&mut tx, id, emitted).await?;
    tx.commit().await?;
    Ok(Json(json!({"events": events, "cursor": build.cursor(), "issues": build.issues()})))
}

/// `POST /v1/builds/{id}/ops` with `{"ops": [ModelOp, ...]}`: all ops apply or none (422).
pub async fn apply_ops(State(state): State<AppState>, Path(id): Path<String>, Json(body): Json<OpsBody>) -> Result<Json<Value>, ApiError> {
    if body.ops.is_empty() { return Err(ApiError::BadRequest("ops must not be empty".into())); }
    mutate(&state, &id, |b| b.apply(&body.ops)).await
}

/// `POST /v1/builds/{id}/undo`: removes exactly one op from the active log.
pub async fn undo(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    mutate(&state, &id, StagedBuild::undo).await
}

/// `POST /v1/builds/{id}/redo`.
pub async fn redo(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    mutate(&state, &id, StagedBuild::redo).await
}

/// `GET /v1/builds/{id}`: the stored build, enough for a refreshed browser to restore it.
pub async fn get(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let id = parse_id(&id)?;
    let mut tx = state.database.begin().await?;
    let (project_id, build) = load(&mut tx, id).await?;
    let revision: Option<(i32,)> = sqlx::query_as("SELECT number FROM helqix_model_revisions WHERE build_id = $1").bind(id).fetch_optional(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({
        "buildId": id, "projectId": project_id, "status": if build.is_committed() { "committed" } else { "staged" },
        "cursor": build.cursor(), "ops": build.ops(), "issues": build.issues(),
        "revision": revision.map(|(n,)| format!("r{n}")),
    })))
}

/// `POST /v1/builds/{id}/commit`: refused (409) while blocking issues exist; otherwise creates rN.
pub async fn commit(State(state): State<AppState>, Path(id): Path<String>) -> Result<impl IntoResponse, ApiError> {
    let id = parse_id(&id)?;
    let mut tx = state.database.begin().await?;
    let (project_id, mut build) = load(&mut tx, id).await?;
    let model_sha256 = build.commit().map_err(build_error)?;
    // Serialize revision numbering per project.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))").bind(project_id.to_string()).execute(&mut *tx).await?;
    let number: i32 = sqlx::query_scalar("SELECT COALESCE(MAX(number), 0) + 1 FROM helqix_model_revisions WHERE project_id = $1").bind(project_id).fetch_one(&mut *tx).await?;
    let committed_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "INSERT INTO helqix_model_revisions (project_id, number, build_id, op_count, model_sha256) VALUES ($1, $2, $3, $4, $5) RETURNING committed_at")
        .bind(project_id).bind(number).bind(id).bind(build.cursor() as i64).bind(&model_sha256).fetch_one(&mut *tx).await?;
    save(&mut tx, id, &build).await?;
    let label = format!("r{number}");
    append_events(&mut tx, id, vec![Emitted { kind: BuildEventKind::Committed, op: None, issue: None, note: Some(label.clone()) }]).await?;
    tx.commit().await?;
    let revision = ModelRevision {
        project_id: project_id.to_string(), number: number as u32, label, build_id: id.to_string(), op_count: build.cursor() as u64, model_sha256,
        committed_at: committed_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    };
    Ok((StatusCode::CREATED, Json(revision)))
}

/// `GET /v1/builds/{id}/events`: Server-Sent Events. Resumes after `Last-Event-ID` or `?after=`;
/// replays stored events in order, then waits for new ones; ends after `committed`.
pub async fn events(State(state): State<AppState>, Path(id): Path<String>, headers: HeaderMap, Query(query): Query<EventsQuery>)
    -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let id = parse_id(&id)?;
    let exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM helqix_builds WHERE id = $1").bind(id).fetch_optional(&state.database).await?;
    exists.ok_or(ApiError::NotFound)?;
    let after = headers.get("last-event-id").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<i64>().ok()).or(query.after).unwrap_or(0);
    Ok(Sse::new(event_stream(state.database.clone(), id, after)).keep_alive(KeepAlive::default()))
}

fn event_stream(pool: PgPool, id: Uuid, after: i64) -> impl Stream<Item = Result<Event, Infallible>> {
    stream::unfold((pool, after, false), move |(pool, after, done)| async move {
        if done { return None; }
        loop {
            let next: Result<Option<(i64, Jsonb<Value>)>, sqlx::Error> =
                sqlx::query_as("SELECT seq, event FROM helqix_build_events WHERE build_id = $1 AND seq > $2 ORDER BY seq LIMIT 1")
                    .bind(id).bind(after).fetch_optional(&pool).await;
            match next {
                Ok(Some((seq, Jsonb(event)))) => {
                    let kind = event["kind"].as_str().unwrap_or("event").to_owned();
                    let sse = Event::default().id(seq.to_string()).event(kind.clone()).json_data(&event)
                        .unwrap_or_else(|_| Event::default().event("failed").data("event could not be encoded"));
                    return Some((Ok(sse), (pool, seq, kind == "committed")));
                }
                Ok(None) => tokio::time::sleep(Duration::from_millis(250)).await,
                Err(error) => {
                    tracing::warn!(%error, build = %id, "event stream stopped");
                    return Some((Ok(Event::default().event("failed").data("event store unavailable")), (pool, after, true)));
                }
            }
        }
    })
}

/// `GET /v1/projects/{project_id}/revisions`: committed revisions, oldest first (`r1`, `r2`, ...).
pub async fn revisions(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<Value>, ApiError> {
    let project_id = parse_id(&project_id)?;
    store::get(&state.database, ResourceKind::Project, project_id).await?;
    let rows: Vec<(i32, Uuid, i64, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT number, build_id, op_count, model_sha256, committed_at FROM helqix_model_revisions WHERE project_id = $1 ORDER BY number")
        .bind(project_id).fetch_all(&state.database).await?;
    let revisions: Vec<ModelRevision> = rows.into_iter().map(|(number, build_id, op_count, model_sha256, committed_at)| ModelRevision {
        project_id: project_id.to_string(), number: number as u32, label: format!("r{number}"), build_id: build_id.to_string(),
        op_count: op_count as u64, model_sha256, committed_at: committed_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }).collect();
    Ok(Json(json!({"revisions": revisions})))
}

/// `GET /v1/projects/{project_id}/revisions/{number}`: one committed revision with the op list that
/// defines it. The client folds the ops into parts, ports and connections.
pub async fn revision(State(state): State<AppState>, Path((project_id, number)): Path<(String, i32)>) -> Result<Json<Value>, ApiError> {
    let project_id = parse_id(&project_id)?;
    let row: Option<(Uuid, i64, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT build_id, op_count, model_sha256, committed_at FROM helqix_model_revisions WHERE project_id = $1 AND number = $2")
        .bind(project_id).bind(number).fetch_optional(&state.database).await?;
    let (build_id, op_count, model_sha256, committed_at) = row.ok_or(ApiError::NotFound)?;
    let mut tx = state.database.begin().await?;
    let (_, build) = load(&mut tx, build_id).await?;
    tx.commit().await?;
    let revision = ModelRevision {
        project_id: project_id.to_string(), number: number as u32, label: format!("r{number}"), build_id: build_id.to_string(),
        op_count: op_count as u64, model_sha256, committed_at: committed_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    };
    // Active ops only: undone ops past the cursor were never part of the committed model.
    Ok(Json(json!({"revision": revision, "ops": build.active_ops()})))
}
