//! Who may see what. The service key sees everything; a signed-in user sees only the resources they created and
//! whatever hangs off their projects (builds, revisions, runs of their models). A resource that is not theirs
//! answers 404, so its existence is not revealed. Routes a user may not use at all answer 403, and a route this
//! module does not know is refused for users by default.
use axum::{extract::{Request, State}, middleware::Next, response::Response};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{auth::Identity, error::ApiError, state::AppState};

/// Records the caller as the owner of a new resource. The first claim wins.
pub async fn claim(pool: &PgPool, id: Uuid, who: &Identity) -> Result<(), ApiError> {
    sqlx::query("INSERT INTO helqix_owners (resource_id, owner) VALUES ($1, $2) ON CONFLICT (resource_id) DO NOTHING")
        .bind(id).bind(who.owner()).execute(pool).await?;
    Ok(())
}

/// The owner to filter lists by: none for the service key.
pub fn list_filter(who: &Identity) -> Option<String> { (!who.is_service()).then(|| who.owner().to_owned()) }

pub async fn ensure_owner(pool: &PgPool, who: &Identity, id: Uuid) -> Result<(), ApiError> {
    if who.is_service() { return Ok(()); }
    let owned: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM helqix_owners WHERE resource_id = $1 AND owner = $2").bind(id).bind(who.owner()).fetch_optional(pool).await?;
    owned.map(|_| ()).ok_or(ApiError::NotFound)
}

fn uuid(s: &str) -> Result<Uuid, ApiError> { Uuid::parse_str(s).map_err(|_| ApiError::NotFound) }

/// What a path needs before the handler runs.
#[derive(Debug, PartialEq, Eq)]
pub enum Need { Nothing, Owner(String), BuildOwner(String), ServiceOnly, Unknown }

const KINDS: [&str; 7] = ["realities", "projects", "branches", "executions", "evidence", "verification", "searches"];

pub fn need(method: &str, path: &str) -> Need {
    let seg: Vec<&str> = path.trim_matches('/').split('/').collect();
    match seg.as_slice() {
        ["v1", "projects", pid, "builds" | "revisions" | "runs", ..] => Need::Owner((*pid).to_owned()),
        ["v1", "builds", bid, ..] => Need::BuildOwner((*bid).to_owned()),
        ["v1", "runs"] if method == "GET" => Need::ServiceOnly,
        ["v1", "runs"] | ["v1", "branches"] | ["v1", "interpret"] | ["v1", "sources"] => Need::Nothing,
        ["v1", "runs", _, "freeze" | "frames" | "frame"] => Need::Nothing,
        ["v1", kind] if KINDS.contains(kind) => Need::Nothing,
        ["v1", kind, "query"] if KINDS.contains(kind) => Need::Nothing,
        ["v1", kind, id] if KINDS.contains(kind) => Need::Owner((*id).to_owned()),
        _ => Need::Unknown,
    }
}

/// Runs after authentication. For users it checks ownership of the project, build or resource the path names.
pub async fn guard(State(state): State<AppState>, request: Request, next: Next) -> Result<Response, ApiError> {
    let who = request.extensions().get::<Identity>().cloned().ok_or(ApiError::Unauthorized)?;
    if !who.is_service() {
        match need(request.method().as_str(), request.uri().path()) {
            Need::Nothing => {}
            Need::Owner(id) => ensure_owner(&state.database, &who, uuid(&id)?).await?,
            Need::BuildOwner(id) => {
                let project: Option<(Uuid,)> = sqlx::query_as("SELECT project_id FROM helqix_builds WHERE id = $1").bind(uuid(&id)?).fetch_optional(&state.database).await?;
                ensure_owner(&state.database, &who, project.ok_or(ApiError::NotFound)?.0).await?;
            }
            Need::ServiceOnly | Need::Unknown => return Err(ApiError::Forbidden),
        }
    }
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_map_to_what_they_need() {
        let o = |s: &str| Need::Owner(s.to_owned());
        assert_eq!(need("POST", "/v1/projects/P/builds"), o("P"));
        assert_eq!(need("GET", "/v1/projects/P/revisions/3"), o("P"));
        assert_eq!(need("POST", "/v1/projects/P/revisions/3/run"), o("P"));
        assert_eq!(need("GET", "/v1/projects/P/runs"), o("P"));
        assert_eq!(need("GET", "/v1/projects/P"), o("P"));
        assert_eq!(need("PUT", "/v1/branches/B"), o("B"));
        assert_eq!(need("POST", "/v1/builds/X/ops"), Need::BuildOwner("X".into()));
        assert_eq!(need("GET", "/v1/projects"), Need::Nothing);
        assert_eq!(need("POST", "/v1/projects/query"), Need::Nothing);
        assert_eq!(need("GET", "/v1/runs"), Need::ServiceOnly);
        assert_eq!(need("POST", "/v1/runs"), Need::Nothing);
        assert_eq!(need("POST", "/v1/runs/R/freeze"), Need::Nothing);
        assert_eq!(need("POST", "/v1/interpret"), Need::Nothing);
    }

    #[test]
    fn an_unknown_path_is_refused_for_users_by_default() {
        assert_eq!(need("GET", "/v1/admin/everything"), Need::Unknown);
        assert_eq!(need("GET", "/v1/projects/P/secrets"), Need::Unknown);
        assert_eq!(need("DELETE", "/v1/"), Need::Unknown);
    }
}
