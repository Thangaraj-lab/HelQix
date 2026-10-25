use axum::{Json, http::{HeaderValue, StatusCode, header}, response::{IntoResponse, Response}};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("invalid request: {0}")]
    BadRequest(String),
    #[error("resource not found")]
    NotFound,
    #[error("resource version conflict")]
    Conflict,
    #[error("resource version precondition failed")]
    PreconditionFailed,
    #[error("If-Match precondition is required")]
    PreconditionRequired,
    #[error("authentication required")]
    Unauthorized,
    #[error("service unavailable")]
    Unavailable,
    #[error("rejected: {0}")]
    Rejected(String),
    #[error("state conflict: {0}")]
    StateConflict(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("internal server error")]
    Internal,
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorDetails,
}

#[derive(Serialize)]
struct ErrorDetails {
    code: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "invalid_request", message.clone()),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", "The requested resource was not found.".into()),
            Self::Conflict => (StatusCode::CONFLICT, "conflict", "The request conflicts with existing resource state.".into()),
            Self::PreconditionFailed => (StatusCode::PRECONDITION_FAILED, "version_conflict", "The resource changed; fetch it and retry.".into()),
            Self::PreconditionRequired => (StatusCode::PRECONDITION_REQUIRED, "precondition_required", "If-Match with the current resource version is required.".into()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized", "Valid API credentials are required.".into()),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable", "The service is temporarily unavailable.".into()),
            Self::Rejected(message) => (StatusCode::UNPROCESSABLE_ENTITY, "rejected", message.clone()),
            Self::StateConflict(message) => (StatusCode::CONFLICT, "state_conflict", message.clone()),
            Self::Database(error) => {
                if matches!(error, sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed) {
                    (StatusCode::SERVICE_UNAVAILABLE, "database_unavailable", "The service is temporarily unavailable.".into())
                } else if matches!(error, sqlx::Error::RowNotFound) {
                    (StatusCode::NOT_FOUND, "not_found", "The requested resource was not found.".into())
                } else if error.as_database_error().and_then(|db| db.code()).as_deref() == Some("23505") {
                    (StatusCode::CONFLICT, "already_exists", "A resource with this identifier already exists.".into())
                } else {
                    tracing::error!(error = %error, "database request failed");
                    (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "An unexpected server error occurred.".into())
                }
            }
            Self::Internal => {
                tracing::error!("unexpected internal API error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "An unexpected server error occurred.".into())
            }
        };
        let mut response = (status, Json(ErrorEnvelope { error: ErrorDetails { code, message } })).into_response();
        if status == StatusCode::UNAUTHORIZED {
            response.headers_mut().insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        response
    }
}
