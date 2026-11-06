//! E10.3: two real users of a real Keycloak (the realm in deploy/keycloak/helqix-realm.json) through the real router and PostgreSQL. Alice's projects are
//! not Bob's, the keys come from Keycloak's own JWKS URL, and tokens that are not meant for this API are refused.
//!
//! Needs HELQIX_TEST_DATABASE_URL and HELQIX_TEST_KEYCLOAK_URL (for example http://127.0.0.1:8081); without them each test prints SKIPPED and passes.
//! The users' passwords are test values from the realm file (never used anywhere else).
use std::time::Duration;

use axum::{Router, body::Body, http::{Request, StatusCode}};
use helqix_api::{auth::Oidc, routes, state::AppState};
use serde_json::{Value, json};
use tower::ServiceExt;

const SERVICE_KEY: &str = "test-api-key-not-a-secret";
const SKIP: &str = "SKIPPED: needs HELQIX_TEST_DATABASE_URL and HELQIX_TEST_KEYCLOAK_URL";

fn keycloak() -> Option<String> { std::env::var("HELQIX_TEST_KEYCLOAK_URL").ok().map(|u| u.trim_end_matches('/').to_owned()) }

/// A real access token from Keycloak's password grant (the test clients only; the Studio client does not allow it).
async fn token(kc: &str, client: &str, user: &str) -> String {
    let res = reqwest::Client::new().post(format!("{kc}/realms/helqix/protocol/openid-connect/token"))
        .header("content-type", "application/x-www-form-urlencoded").body(format!("grant_type=password&client_id={client}&username={user}&password={user}-ci-only-password")).send().await.expect("Keycloak answers");
    let status = res.status();
    let body: Value = serde_json::from_str(&res.text().await.expect("an answer")).expect("a JSON answer");
    assert!(status.is_success(), "Keycloak refused {user} on {client}: {body}");
    body["access_token"].as_str().expect("an access token").to_owned()
}

async fn app(kc: &str) -> Router {
    let url = std::env::var("HELQIX_TEST_DATABASE_URL").expect("HELQIX_TEST_DATABASE_URL");
    let pool = sqlx::postgres::PgPoolOptions::new().max_connections(4).connect(&url).await.expect("connect to test database");
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations");
    let oidc = Oidc::from_url(&format!("{kc}/realms/helqix"), "helqix-api", &format!("{kc}/realms/helqix/protocol/openid-connect/certs"), Duration::from_secs(30)).await.expect("the keys come from Keycloak's JWKS URL");
    routes::router(AppState::new(pool, SERVICE_KEY).with_oidc(Some(oidc)), &["http://localhost:5173".to_owned()], Duration::from_secs(20)).expect("router")
}

async fn call(app: &Router, bearer: Option<&str>, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(b) = bearer { req = req.header("authorization", format!("Bearer {b}")); }
    if body.is_some() { req = req.header("content-type", "application/json"); }
    let res = app.clone().oneshot(req.body(body.map_or_else(Body::empty, |b| Body::from(b.to_string()))).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 4 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn two_real_users_cannot_see_each_others_projects_builds_or_revisions() {
    let Some(kc) = keycloak() else { eprintln!("{SKIP}"); return };
    if std::env::var("HELQIX_TEST_DATABASE_URL").is_err() { eprintln!("{SKIP}"); return }
    let app = app(&kc).await;
    let (alice, bob) = (token(&kc, "helqix-test", "alice").await, token(&kc, "helqix-test", "bob").await);
    assert_ne!(alice, bob);

    let (s, p) = call(&app, Some(&alice), "POST", "/v1/projects", Some(json!({"payload": {"name": "Alice's cell"}}))).await;
    assert_eq!(s, StatusCode::CREATED, "{p}");
    let pid = p["data"]["id"].as_str().unwrap().to_owned();

    // Alice sees her project and can build in it; Bob sees none of it, and the answer does not reveal it exists.
    let (_, list) = call(&app, Some(&alice), "GET", "/v1/projects", None).await;
    assert!(list["data"].as_array().unwrap().iter().any(|r| r["id"] == pid.as_str()), "{list}");
    let (_, list) = call(&app, Some(&bob), "GET", "/v1/projects", None).await;
    assert!(list["data"].as_array().unwrap().iter().all(|r| r["id"] != pid.as_str()), "Bob's list has Alice's project: {list}");
    let (s, _) = call(&app, Some(&bob), "GET", &format!("/v1/projects/{pid}"), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "Bob reading Alice's project");
    let (s, _) = call(&app, Some(&bob), "POST", &format!("/v1/projects/{pid}/builds"), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "Bob starting a build in Alice's project");
    let (s, build) = call(&app, Some(&alice), "POST", &format!("/v1/projects/{pid}/builds"), None).await;
    assert_eq!(s, StatusCode::CREATED, "{build}");
    let bid = build["buildId"].as_str().unwrap().to_owned();
    let (s, _) = call(&app, Some(&bob), "GET", &format!("/v1/builds/{bid}"), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "Bob reading Alice's build");
    let (s, _) = call(&app, Some(&bob), "POST", &format!("/v1/builds/{bid}/ops"), Some(json!({"ops": []}))).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "Bob changing Alice's build");
    let (s, _) = call(&app, Some(&bob), "GET", &format!("/v1/projects/{pid}/revisions"), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "Bob listing Alice's revisions");

    // Bob has his own space; the operator's service key sees both.
    let (s, mine) = call(&app, Some(&bob), "POST", "/v1/projects", Some(json!({"payload": {"name": "Bob's pump"}}))).await;
    assert_eq!(s, StatusCode::CREATED, "{mine}");
    let (_, all) = call(&app, Some(SERVICE_KEY), "GET", "/v1/projects?limit=100", None).await;
    let ids: Vec<&str> = all["data"].as_array().unwrap().iter().filter_map(|r| r["id"].as_str()).collect();
    assert!(ids.contains(&pid.as_str()) && ids.contains(&mine["data"]["id"].as_str().unwrap()), "{all}");
}

#[tokio::test]
async fn tokens_that_are_not_meant_for_this_api_or_are_damaged_are_refused() {
    let Some(kc) = keycloak() else { eprintln!("{SKIP}"); return };
    if std::env::var("HELQIX_TEST_DATABASE_URL").is_err() { eprintln!("{SKIP}"); return }
    let app = app(&kc).await;
    // A genuine Keycloak token whose audience is another API (other-api), not helqix-api.
    let other = token(&kc, "helqix-other", "alice").await;
    assert_eq!(call(&app, Some(&other), "GET", "/v1/projects", None).await.0, StatusCode::UNAUTHORIZED, "a token for another audience");
    // A genuine token with one character of its signature changed.
    let good = token(&kc, "helqix-test", "alice").await;
    let (head, sig) = good.rsplit_once('.').unwrap();
    let flipped = format!("{head}.{}{}", if sig.starts_with('A') { 'B' } else { 'A' }, &sig[1..]);
    assert_eq!(call(&app, Some(&flipped), "GET", "/v1/projects", None).await.0, StatusCode::UNAUTHORIZED, "a damaged signature");
    // No credentials, and a made-up token.
    assert_eq!(call(&app, None, "GET", "/v1/projects", None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, Some("not.a.token"), "GET", "/v1/projects", None).await.0, StatusCode::UNAUTHORIZED);
    // The good one still works.
    assert_eq!(call(&app, Some(&good), "GET", "/v1/projects", None).await.0, StatusCode::OK);
}
