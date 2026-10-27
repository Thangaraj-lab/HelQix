use std::time::Duration;

use axum::{Json, Router, extract::State, http::{HeaderName, HeaderValue, Method, header}, middleware, routing::get};
use serde::Serialize;
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, timeout::TimeoutLayer, trace::TraceLayer};

use crate::{auth, builds, error::ApiError, frames, request_id, resources, sources, state::AppState};

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub fn router(state: AppState, origins: &[String], timeout: Duration) -> Result<Router, ApiError> {
    let allowed_origins = origins.iter()
        .map(|origin| origin.parse::<HeaderValue>().map_err(|_| ApiError::BadRequest("invalid configured CORS origin".into())))
        .collect::<Result<Vec<_>, _>>()?;
    if allowed_origins.is_empty() {
        return Err(ApiError::BadRequest("at least one CORS origin must be configured".into()));
    }

    let protected_api = Router::<AppState>::new()
        .route("/v1/projects/{project_id}/builds", axum::routing::post(builds::start))
        .route("/v1/projects/{project_id}/revisions", get(builds::revisions))
        .route("/v1/projects/{project_id}/revisions/{number}", get(builds::revision))
        .route("/v1/builds/{id}", get(builds::get))
        .route("/v1/builds/{id}/events", get(builds::events))
        .route("/v1/builds/{id}/ops", axum::routing::post(builds::apply_ops))
        .route("/v1/builds/{id}/undo", axum::routing::post(builds::undo))
        .route("/v1/builds/{id}/redo", axum::routing::post(builds::redo))
        .route("/v1/builds/{id}/commit", axum::routing::post(builds::commit))
        .route("/v1/sources", get(sources::list))
        .route("/v1/runs", get(frames::list_runs))
        .route("/v1/runs/{id}/frames", get(frames::run_frames))
        .route("/v1/runs/{id}/frame", get(frames::get_frame))
        .route("/v1/{resource}", get(resources::list).post(resources::create))
        .route("/v1/{resource}/query", axum::routing::post(resources::search))
        .route("/v1/{resource}/{id}", get(resources::get).put(resources::replace).delete(resources::delete))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::require_api_key));

    Ok(Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .merge(protected_api)
        .layer(CorsLayer::new()
            .allow_origin(allowed_origins)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::IF_MATCH, HeaderName::from_static("idempotency-key"), HeaderName::from_static("last-event-id")])
            .expose_headers([header::ETAG]))
        .layer(TimeoutLayer::new(timeout))
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(request_id::attach))
        .with_state(state))
}

async fn live() -> Json<HealthResponse> {
    Json(HealthResponse { status: "live" })
}

async fn ready(State(state): State<AppState>) -> Result<Json<HealthResponse>, ApiError> {
    sqlx::query("SELECT 1").execute(&state.database).await.map_err(|error| {
        tracing::error!(error = %error, "readiness database check failed");
        ApiError::Unavailable
    })?;
    Ok(Json(HealthResponse { status: "ready" }))
}
