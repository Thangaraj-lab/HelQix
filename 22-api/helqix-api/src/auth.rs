use axum::{Json, extract::{Request, State}, http::{StatusCode, header}, middleware::Next, response::{IntoResponse, Response}};
use serde::Serialize;
use subtle::ConstantTimeEq;

use crate::state::AppState;

#[derive(Serialize)]
struct AuthError {
    error: AuthErrorDetails,
}

#[derive(Serialize)]
struct AuthErrorDetails {
    code: &'static str,
    message: &'static str,
}

pub async fn require_api_key(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let supplied = request.headers().get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .and_then(|(scheme, token)| scheme.eq_ignore_ascii_case("Bearer").then_some(token));
    let authorized = supplied.is_some_and(|token| {
        bool::from(token.as_bytes().ct_eq(state.api_key.as_bytes()))
    });
    if !authorized {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer")],
            Json(AuthError { error: AuthErrorDetails {
                code: "unauthorized",
                message: "Valid API credentials are required.",
            } }),
        ).into_response();
    }
    next.run(request).await
}
