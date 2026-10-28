use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::{PgPool, types::Json};
use uuid::Uuid;

use crate::error::ApiError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceKind {
    Reality,
    Project,
    Branch,
    Execution,
    Evidence,
    Verification,
    Search,
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reality => "realities",
            Self::Project => "projects",
            Self::Branch => "branches",
            Self::Execution => "executions",
            Self::Evidence => "evidence",
            Self::Verification => "verification",
            Self::Search => "searches",
        }
    }
}

impl std::str::FromStr for ResourceKind {
    type Err = ApiError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "realities" => Ok(Self::Reality),
            "projects" => Ok(Self::Project),
            "branches" => Ok(Self::Branch),
            "executions" => Ok(Self::Execution),
            "evidence" => Ok(Self::Evidence),
            "verification" => Ok(Self::Verification),
            "searches" => Ok(Self::Search),
            _ => Err(ApiError::NotFound),
        }
    }
}

#[derive(Clone, Debug, sqlx::FromRow)]
struct DbResource {
    id: Uuid,
    project_id: Option<Uuid>,
    payload: Json<Value>,
    version: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResourceRecord {
    pub id: Uuid,
    pub kind: String,
    pub project_id: Option<Uuid>,
    pub payload: Value,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<(ResourceKind, DbResource)> for ResourceRecord {
    fn from((kind, resource): (ResourceKind, DbResource)) -> Self {
        Self {
            id: resource.id,
            kind: kind.as_str().to_owned(),
            project_id: resource.project_id,
            payload: resource.payload.0,
            version: resource.version,
            created_at: resource.created_at,
            updated_at: resource.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ResourcePage {
    pub items: Vec<ResourceRecord>,
    pub next_cursor: Option<Uuid>,
}

pub async fn create(
    pool: &PgPool,
    kind: ResourceKind,
    id: Uuid,
    project_id: Option<Uuid>,
    payload: Value,
) -> Result<ResourceRecord, ApiError> {
    let row = sqlx::query_as::<_, DbResource>(
        "INSERT INTO helqix_resources (resource_type, id, project_id, payload) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, project_id, payload, version, created_at, updated_at",
    )
    .bind(kind.as_str())
    .bind(id)
    .bind(project_id)
    .bind(Json(payload))
    .fetch_one(pool)
    .await?;
    Ok((kind, row).into())
}

/// Create once for an idempotency key and replay the original resource on a
/// matching retry. Reusing the key for a different request returns conflict.
pub async fn create_idempotent(
    pool: &PgPool,
    kind: ResourceKind,
    requested_id: Option<Uuid>,
    project_id: Option<Uuid>,
    payload: Value,
    idempotency_key: &str,
) -> Result<ResourceRecord, ApiError> {
    if idempotency_key.trim().is_empty() || idempotency_key.trim() != idempotency_key || idempotency_key.len() > 200 {
        return Err(ApiError::BadRequest("Idempotency-Key must be 1-200 trimmed bytes".into()));
    }
    let id = requested_id.unwrap_or_else(Uuid::new_v4);
    let fingerprint = json!({ "requestedId": requested_id, "projectId": project_id, "payload": payload });
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "DELETE FROM helqix_idempotency_keys WHERE resource_type = $1 AND idempotency_key = $2 AND expires_at <= now()",
    )
    .bind(kind.as_str())
    .bind(idempotency_key)
    .execute(&mut *transaction)
    .await?;

    let claim = sqlx::query(
        "INSERT INTO helqix_idempotency_keys (resource_type, idempotency_key, fingerprint, resource_id) \
         VALUES ($1, $2, $3, $4) ON CONFLICT (resource_type, idempotency_key) DO NOTHING",
    )
    .bind(kind.as_str())
    .bind(idempotency_key)
    .bind(Json(fingerprint.clone()))
    .bind(id)
    .execute(&mut *transaction)
    .await?;

    if claim.rows_affected() == 0 {
        let existing: Option<(Json<Value>, Uuid)> = sqlx::query_as(
            "SELECT fingerprint, resource_id FROM helqix_idempotency_keys \
             WHERE resource_type = $1 AND idempotency_key = $2",
        )
        .bind(kind.as_str())
        .bind(idempotency_key)
        .fetch_optional(&mut *transaction)
        .await?;
        let (existing_fingerprint, resource_id) = existing.ok_or(ApiError::Internal)?;
        if existing_fingerprint.0 != fingerprint {
            transaction.rollback().await?;
            return Err(ApiError::Conflict);
        }
        transaction.commit().await?;
        return get(pool, kind, resource_id).await;
    }

    let row = sqlx::query_as::<_, DbResource>(
        "INSERT INTO helqix_resources (resource_type, id, project_id, payload) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, project_id, payload, version, created_at, updated_at",
    )
    .bind(kind.as_str())
    .bind(id)
    .bind(project_id)
    .bind(Json(payload))
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok((kind, row).into())
}

pub async fn get(pool: &PgPool, kind: ResourceKind, id: Uuid) -> Result<ResourceRecord, ApiError> {
    let row = sqlx::query_as::<_, DbResource>(
        "SELECT id, project_id, payload, version, created_at, updated_at \
         FROM helqix_resources WHERE resource_type = $1 AND id = $2",
    )
    .bind(kind.as_str())
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    Ok((kind, row).into())
}

pub async fn list(
    pool: &PgPool,
    kind: ResourceKind,
    project_id: Option<Uuid>,
    after: Option<Uuid>,
    limit: u16,
    owner: Option<&str>,
) -> Result<ResourcePage, ApiError> {
    list_filtered(pool, kind, project_id, after, limit, None, owner).await
}

pub async fn search(
    pool: &PgPool,
    kind: ResourceKind,
    project_id: Option<Uuid>,
    after: Option<Uuid>,
    limit: u16,
    payload_contains: Value,
    owner: Option<&str>,
) -> Result<ResourcePage, ApiError> {
    if !payload_contains.is_object() {
        return Err(ApiError::BadRequest("payloadContains must be a JSON object".into()));
    }
    list_filtered(pool, kind, project_id, after, limit, Some(payload_contains), owner).await
}

async fn list_filtered(
    pool: &PgPool,
    kind: ResourceKind,
    project_id: Option<Uuid>,
    after: Option<Uuid>,
    limit: u16,
    payload_contains: Option<Value>,
    owner: Option<&str>,
) -> Result<ResourcePage, ApiError> {
    if limit == 0 { return Err(ApiError::BadRequest("limit must be greater than zero".into())); }
    let cursor_time: Option<DateTime<Utc>> = if let Some(cursor) = after {
        Some(sqlx::query_scalar(
            "SELECT created_at FROM helqix_resources WHERE resource_type = $1 AND id = $2",
        )
        .bind(kind.as_str())
        .bind(cursor)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::BadRequest("after cursor does not identify a resource of this type".into()))?)
    } else {
        None
    };

    let rows = sqlx::query_as::<_, DbResource>(
        "SELECT id, project_id, payload, version, created_at, updated_at \
         FROM helqix_resources \
         WHERE resource_type = $1 AND ($2::uuid IS NULL OR project_id = $2) \
           AND ($3::jsonb IS NULL OR payload @> $3) \
           AND ($4::timestamptz IS NULL OR (created_at, id) < ($4, $5::uuid)) \
           AND ($7::text IS NULL OR id IN (SELECT resource_id FROM helqix_owners WHERE owner = $7)) \
         ORDER BY created_at DESC, id DESC LIMIT $6",
    )
    .bind(kind.as_str())
    .bind(project_id)
    .bind(payload_contains.map(Json))
    .bind(cursor_time)
    .bind(after)
    .bind(i64::from(limit) + 1)
    .bind(owner)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > usize::from(limit);
    let next_cursor = if has_more { rows.get(usize::from(limit) - 1).map(|row| row.id) } else { None };
    let mut rows = rows;
    rows.truncate(usize::from(limit));
    Ok(ResourcePage { items: rows.into_iter().map(|row| (kind, row).into()).collect(), next_cursor })
}

pub async fn replace(
    pool: &PgPool,
    kind: ResourceKind,
    id: Uuid,
    expected_version: i64,
    project_id: Option<Uuid>,
    payload: Value,
) -> Result<ResourceRecord, ApiError> {
    let updated = sqlx::query_as::<_, DbResource>(
        "UPDATE helqix_resources SET project_id = $4, payload = $5, version = version + 1, updated_at = now() \
         WHERE resource_type = $1 AND id = $2 AND version = $3 \
         RETURNING id, project_id, payload, version, created_at, updated_at",
    )
    .bind(kind.as_str())
    .bind(id)
    .bind(expected_version)
    .bind(project_id)
    .bind(Json(payload))
    .fetch_optional(pool)
    .await?;
    if let Some(row) = updated { return Ok((kind, row).into()); }
    if sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM helqix_resources WHERE resource_type = $1 AND id = $2)")
        .bind(kind.as_str()).bind(id).fetch_one(pool).await? {
        Err(ApiError::PreconditionFailed)
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn delete(pool: &PgPool, kind: ResourceKind, id: Uuid, expected_version: i64) -> Result<(), ApiError> {
    let mut transaction = pool.begin().await?;
    let result = sqlx::query("DELETE FROM helqix_resources WHERE resource_type = $1 AND id = $2 AND version = $3")
        .bind(kind.as_str()).bind(id).bind(expected_version).execute(&mut *transaction).await?;
    if result.rows_affected() == 1 {
        sqlx::query("DELETE FROM helqix_idempotency_keys WHERE resource_type = $1 AND resource_id = $2")
            .bind(kind.as_str()).bind(id).execute(&mut *transaction).await?;
        transaction.commit().await?;
        return Ok(());
    }
    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM helqix_resources WHERE resource_type = $1 AND id = $2)")
        .bind(kind.as_str()).bind(id).fetch_one(&mut *transaction).await?;
    transaction.rollback().await?;
    if exists {
        Err(ApiError::PreconditionFailed)
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn cleanup_expired_idempotency_keys(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "WITH expired AS ( \
           SELECT resource_type, idempotency_key FROM helqix_idempotency_keys \
           WHERE expires_at <= now() ORDER BY expires_at LIMIT 1000 FOR UPDATE SKIP LOCKED \
         ) \
         DELETE FROM helqix_idempotency_keys AS keys \
         USING expired \
         WHERE keys.resource_type = expired.resource_type AND keys.idempotency_key = expired.idempotency_key",
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
