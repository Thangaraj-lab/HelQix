//! `/metrics` through the real router. Health and metrics routes do not touch PostgreSQL, so the pool is lazy.
use axum::{Router, body::Body, http::{Request, StatusCode}};
use helqix_api::{routes, state::AppState};
use tower::ServiceExt;

const KEY: &str = "test-api-key-not-a-secret-0123456789";

fn app() -> Router {
    let pool = sqlx::postgres::PgPoolOptions::new().connect_lazy("postgres://unused:unused@127.0.0.1:1/unused").unwrap();
    routes::router(AppState::new(pool, KEY), &["http://localhost:5173".to_owned()], std::time::Duration::from_secs(10)).unwrap()
}

async fn get(app: &Router, uri: &str, key: Option<&str>) -> (StatusCode, String) {
    let mut req = Request::builder().uri(uri);
    if let Some(k) = key { req = req.header("authorization", format!("Bearer {k}")); }
    let res = app.clone().oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
    let status = res.status();
    (status, String::from_utf8(axum::body::to_bytes(res.into_body(), 1 << 20).await.unwrap().to_vec()).unwrap())
}

#[tokio::test]
async fn metrics_count_requests_by_route_pattern_and_need_the_key() {
    let app = app();
    assert_eq!(get(&app, "/health/live", None).await.0, StatusCode::OK);
    assert_eq!(get(&app, "/health/live", None).await.0, StatusCode::OK);
    assert_eq!(get(&app, "/metrics", None).await.0, StatusCode::UNAUTHORIZED, "metrics are not public");
    let (st, text) = get(&app, "/metrics", Some(KEY)).await;
    assert_eq!(st, StatusCode::OK);
    assert!(text.contains("helqix_http_requests_total{method=\"GET\",route=\"/health/live\",status=\"200\"} 2"), "{text}");
    assert!(text.contains("route=\"/metrics\",status=\"401\"} 1"), "{text}");
    assert!(text.contains("helqix_http_request_duration_seconds_count{method=\"GET\",route=\"/health/live\",status=\"200\"} 2"));
}
