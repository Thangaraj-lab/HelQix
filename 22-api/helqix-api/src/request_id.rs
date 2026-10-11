use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use tracing::Instrument;
use uuid::Uuid;

/// Accept only UUID request identifiers, so values are bounded and safe to log.
/// Invalid or absent identifiers are replaced with a server-generated UUID.
pub async fn attach(mut request: Request, next: Next) -> Response {
    let id = request.headers().get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .unwrap_or_else(Uuid::new_v4);
    let span = tracing::info_span!("api_request", request_id = %id);
    let mut response = next.run(request).instrument(span).await;
    if let Ok(value) = HeaderValue::from_str(&id.to_string()) {
        response.headers_mut().insert("x-request-id", value);
    }
    response
}
