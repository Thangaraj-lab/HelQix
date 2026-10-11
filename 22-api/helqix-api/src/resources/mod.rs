use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState, store::{self, ResourceKind, ResourceRecord}};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateResource {
    pub payload: Value,
    #[serde(default)]
    pub id: Option<Uuid>,
    #[serde(default)]
    pub project_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplaceResource {
    pub payload: Value,
    #[serde(default)]
    pub project_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListParameters {
    #[serde(default)]
    pub limit: Option<u16>,
    #[serde(default)]
    pub after: Option<Uuid>,
    #[serde(default)]
    pub project_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchRequest {
    pub payload_contains: Value,
    #[serde(default)]
    pub limit: Option<u16>,
    #[serde(default)]
    pub after: Option<Uuid>,
    #[serde(default)]
    pub project_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct ResourceResponse {
    pub data: ResourceRecord,
}

#[derive(Debug, Serialize)]
pub struct ResourceList {
    pub data: Vec<ResourceRecord>,
    pub next_cursor: Option<Uuid>,
}

pub async fn list(
    State(state): State<AppState>,
    Path(resource): Path<String>,
    Query(parameters): Query<ListParameters>,
) -> Result<impl IntoResponse, ApiError> {
    let kind = resource.parse::<ResourceKind>()?;
    let limit = parameters.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return Err(ApiError::BadRequest("limit must be between 1 and 100".into()));
    }
    let page = store::list(&state.database, kind, parameters.project_id, parameters.after, limit).await?;
    Ok(Json(ResourceList { data: page.items, next_cursor: page.next_cursor }))
}

pub async fn search(
    State(state): State<AppState>,
    Path(resource): Path<String>,
    Json(body): Json<SearchRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let kind = resource.parse::<ResourceKind>()?;
    let limit = body.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return Err(ApiError::BadRequest("limit must be between 1 and 100".into()));
    }
    let page = store::search(
        &state.database,
        kind,
        body.project_id,
        body.after,
        limit,
        body.payload_contains,
    ).await?;
    Ok(Json(ResourceList { data: page.items, next_cursor: page.next_cursor }))
}

pub async fn create(
    State(state): State<AppState>,
    Path(resource): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CreateResource>,
) -> Result<impl IntoResponse, ApiError> {
    let kind = resource.parse::<ResourceKind>()?;
    validate_payload(&body.payload)?;
    let record = if let Some(key) = headers.get("idempotency-key") {
        let key = key.to_str().map_err(|_| ApiError::BadRequest("Idempotency-Key must be valid ASCII".into()))?;
        store::create_idempotent(&state.database, kind, body.id, body.project_id, body.payload, key).await?
    } else {
        store::create(&state.database, kind, body.id.unwrap_or_else(Uuid::new_v4), body.project_id, body.payload).await?
    };
    let etag = etag(record.version)?;
    Ok((StatusCode::CREATED, [(header::ETAG, etag)], Json(ResourceResponse { data: record })))
}

pub async fn get(
    State(state): State<AppState>,
    Path((resource, id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let kind = resource.parse::<ResourceKind>()?;
    let id = parse_id(&id)?;
    let record = store::get(&state.database, kind, id).await?;
    let etag = etag(record.version)?;
    Ok(([(header::ETAG, etag)], Json(ResourceResponse { data: record })))
}

pub async fn replace(
    State(state): State<AppState>,
    Path((resource, id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<ReplaceResource>,
) -> Result<impl IntoResponse, ApiError> {
    let kind = resource.parse::<ResourceKind>()?;
    let id = parse_id(&id)?;
    let expected_version = parse_if_match(&headers)?;
    validate_payload(&body.payload)?;
    let record = store::replace(
        &state.database,
        kind,
        id,
        expected_version,
        body.project_id,
        body.payload,
    ).await?;
    let etag = etag(record.version)?;
    Ok(([(header::ETAG, etag)], Json(ResourceResponse { data: record })))
}

pub async fn delete(
    State(state): State<AppState>,
    Path((resource, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let kind = resource.parse::<ResourceKind>()?;
    let id = parse_id(&id)?;
    let expected_version = parse_if_match(&headers)?;
    store::delete(&state.database, kind, id, expected_version).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn parse_id(value: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value).map_err(|_| ApiError::BadRequest("resource id must be a UUID".into()))
}

fn parse_if_match(headers: &HeaderMap) -> Result<i64, ApiError> {
    let value = headers.get(header::IF_MATCH)
        .and_then(|header| header.to_str().ok())
        .ok_or(ApiError::PreconditionRequired)?;
    let version = value.strip_prefix('"').and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| ApiError::BadRequest("If-Match must be a strong quoted version ETag".into()))?
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("If-Match must contain a positive version".into()))?;
    if version < 1 { return Err(ApiError::BadRequest("If-Match must contain a positive version".into())); }
    Ok(version)
}

fn validate_payload(payload: &Value) -> Result<(), ApiError> {
    if !payload.is_object() {
        return Err(ApiError::BadRequest("payload must be a JSON object".into()));
    }
    Ok(())
}

fn etag(version: i64) -> Result<axum::http::HeaderValue, ApiError> {
    format!("\"{version}\"").parse().map_err(|_| ApiError::Internal)
}
