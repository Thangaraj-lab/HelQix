//! `GET /v1/sources`: connected component and material sources. None are connected by default,
//! and the response says so instead of inventing any.
use axum::{Json, extract::State};
use helqix_contracts::Source;
use serde_json::{Value, json};

use crate::{error::ApiError, state::AppState};

pub async fn list(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let rows: Vec<(String, String, String, Option<String>, bool)> =
        sqlx::query_as("SELECT id, name, kind, as_of, example FROM helqix_sources ORDER BY id").fetch_all(&state.database).await?;
    let sources: Vec<Source> = rows.into_iter().map(|(id, name, kind, as_of, example)| Source { id, name, kind, as_of, example }).collect();
    let note = sources.is_empty().then_some("No sources connected. Parts will be placeholders or defined from scratch.");
    Ok(Json(json!({"sources": sources, "note": note})))
}
