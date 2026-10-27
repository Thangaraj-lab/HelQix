//! K2 acceptance through the real router and PostgreSQL. Needs HELQIX_TEST_DATABASE_URL (CI
//! provides a Postgres service); without it the test prints SKIPPED and passes trivially, so the
//! CI job asserts the variable is set.
use axum::{Router, body::Body, http::{Request, StatusCode}};
use helqix_api::{routes, state::AppState};
use serde_json::{Value, json};
use tower::ServiceExt;

const KEY: &str = "test-api-key-not-a-secret";

async fn app() -> Option<Router> {
    let url = std::env::var("HELQIX_TEST_DATABASE_URL").ok()?;
    let pool = sqlx::postgres::PgPoolOptions::new().max_connections(4).connect(&url).await.expect("connect to test database");
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations");
    Some(routes::router(AppState::new(pool, KEY), &["http://localhost:5173".to_owned()], std::time::Duration::from_secs(10)).expect("router"))
}

async fn call(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri).header("authorization", format!("Bearer {KEY}"));
    if body.is_some() { req = req.header("content-type", "application/json"); }
    let req = req.body(body.map_or_else(Body::empty, |b| Body::from(b.to_string()))).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 4 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn v(x: f64, unit: &str) -> Value { json!({"v": x, "unit": unit, "origin": "user_stated", "status": "unverified"}) }
fn part(id: &str) -> Value { json!({"op": "part.add", "id": id, "name": id, "geometry": {"kind": "primitive", "shape": "box", "dims": [v(0.1, "m")]}, "maturity": "user_defined", "prov": {"kind": "user_stated"}}) }
fn port(part: &str, id: &str, dir: &str, unit: &str) -> Value { json!({"op": "port.add", "part": part, "id": id, "kind": "electrical", "direction": dir, "unit": unit}) }

#[tokio::test]
async fn staged_build_round_trip_through_the_api() {
    let Some(app) = app().await else { eprintln!("SKIPPED: HELQIX_TEST_DATABASE_URL not set"); return };

    let (s, project) = call(&app, "POST", "/v1/projects", Some(json!({"payload": {"name": "Untitled project"}}))).await;
    assert_eq!(s, StatusCode::CREATED, "{project}");
    let pid = project["data"]["id"].as_str().unwrap().to_owned();

    let (s, build) = call(&app, "POST", &format!("/v1/projects/{pid}/builds"), None).await;
    assert_eq!(s, StatusCode::CREATED, "{build}");
    let bid = build["buildId"].as_str().unwrap().to_owned();

    // Ops with a unit mismatch (V to A): applied, but a blocking issue is raised.
    let ops = vec![part("base"), part("body"), port("base", "out", "out", "V"), port("body", "in", "in", "A"),
                   json!({"op": "connection.add", "id": "c1", "from": {"part": "base", "port": "out"}, "to": {"part": "body", "port": "in"}, "kind": "electrical"})];
    let (s, r) = call(&app, "POST", &format!("/v1/builds/{bid}/ops"), Some(json!({"ops": ops}))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let applied: Vec<Value> = r["events"].as_array().unwrap().iter().filter(|e| e["kind"] == "op.applied").map(|e| e["op"].clone()).collect();
    assert_eq!(applied, ops, "event order equals op order");
    assert!(r["events"].as_array().unwrap().iter().any(|e| e["kind"] == "issue.raised" && e["issue"]["code"] == "unit.mismatch"));

    // Commit refused while the blocking issue exists.
    let (s, r) = call(&app, "POST", &format!("/v1/builds/{bid}/commit"), None).await;
    assert_eq!(s, StatusCode::CONFLICT, "{r}");

    // Undo removes exactly one op (the connection), which clears the issue.
    let (s, r) = call(&app, "POST", &format!("/v1/builds/{bid}/undo"), None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["cursor"], 4);
    assert_eq!(r["events"][0]["kind"], "op.undone");
    assert!(r["events"].as_array().unwrap().iter().any(|e| e["kind"] == "issue.cleared"));

    // A rejected op (unknown part) changes nothing.
    let (s, _) = call(&app, "POST", &format!("/v1/builds/{bid}/ops"), Some(json!({"ops": [json!({"op": "part.remove", "id": "ghost"})]}))).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);

    // Refresh: the stored build restores the same state.
    let (s, stored) = call(&app, "GET", &format!("/v1/builds/{bid}"), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!((stored["status"].as_str(), stored["cursor"].as_u64(), stored["ops"].as_array().unwrap().len()), (Some("staged"), Some(4), 5));
    assert_eq!(stored["issues"], json!([]));

    // Commit returns r1.
    let (s, rev) = call(&app, "POST", &format!("/v1/builds/{bid}/commit"), None).await;
    assert_eq!(s, StatusCode::CREATED, "{rev}");
    assert_eq!((rev["number"].as_u64(), rev["label"].as_str(), rev["opCount"].as_u64()), (Some(1), Some("r1"), Some(4)));
    assert_eq!(rev["modelSha256"].as_str().unwrap().len(), 64);
    let (s, _) = call(&app, "POST", &format!("/v1/builds/{bid}/ops"), Some(json!({"ops": [part("late")]}))).await;
    assert_eq!(s, StatusCode::CONFLICT, "a committed build is immutable");

    // A second build of the same project commits r2.
    let (_, b2) = call(&app, "POST", &format!("/v1/projects/{pid}/builds"), None).await;
    let b2 = b2["buildId"].as_str().unwrap().to_owned();
    call(&app, "POST", &format!("/v1/builds/{b2}/ops"), Some(json!({"ops": [part("x")]}))).await;
    let (_, rev2) = call(&app, "POST", &format!("/v1/builds/{b2}/commit"), None).await;
    assert_eq!(rev2["label"], "r2");

    // SSE replays every stored event in seq order and ends after `committed`.
    let req = Request::builder().uri(format!("/v1/builds/{bid}/events")).header("authorization", format!("Bearer {KEY}")).body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.headers()["content-type"], "text/event-stream");
    let text = String::from_utf8(axum::body::to_bytes(res.into_body(), 4 << 20).await.unwrap().to_vec()).unwrap();
    let ids: Vec<u64> = text.lines().filter_map(|l| l.strip_prefix("id: ")).map(|x| x.parse().unwrap()).collect();
    assert!(!ids.is_empty() && ids.windows(2).all(|w| w[1] == w[0] + 1) && ids[0] == 1, "ids {ids:?}");
    assert!(text.contains("event: committed"));

    // Resume after an id.
    let req = Request::builder().uri(format!("/v1/builds/{bid}/events")).header("authorization", format!("Bearer {KEY}")).header("last-event-id", ids[ids.len() - 2].to_string()).body(Body::empty()).unwrap();
    let text = String::from_utf8(axum::body::to_bytes(app.clone().oneshot(req).await.unwrap().into_body(), 1 << 20).await.unwrap().to_vec()).unwrap();
    assert_eq!(text.lines().filter(|l| l.starts_with("id: ")).count(), 1);

    // Sources: none connected, said honestly.
    let (s, src) = call(&app, "GET", "/v1/sources", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(src["sources"], json!([]));
    assert!(src["note"].as_str().unwrap().starts_with("No sources connected"));

    // Unknown project and no credentials.
    let (s, _) = call(&app, "POST", &format!("/v1/projects/{}/builds", uuid::Uuid::new_v4()), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let res = app.clone().oneshot(Request::builder().uri("/v1/sources").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn committed_revisions_are_readable_with_their_ops() {
    let Some(app) = app().await else { eprintln!("SKIPPED: HELQIX_TEST_DATABASE_URL not set"); return };
    let (_, project) = call(&app, "POST", "/v1/projects", Some(json!({"payload": {"name": "Untitled project"}}))).await;
    let pid = project["data"]["id"].as_str().unwrap().to_owned();

    // Nothing committed yet, and an unknown revision is a 404.
    let (s, r) = call(&app, "GET", &format!("/v1/projects/{pid}/revisions"), None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["revisions"].as_array().unwrap().len(), 0);
    let (s, _) = call(&app, "GET", &format!("/v1/projects/{pid}/revisions/1"), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    let (_, build) = call(&app, "POST", &format!("/v1/projects/{pid}/builds"), None).await;
    let bid = build["buildId"].as_str().unwrap().to_owned();
    let ops = vec![part("base"), part("body")];
    let (s, r) = call(&app, "POST", &format!("/v1/builds/{bid}/ops"), Some(json!({"ops": ops}))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let (s, committed) = call(&app, "POST", &format!("/v1/builds/{bid}/commit"), None).await;
    assert_eq!(s, StatusCode::CREATED, "{committed}");

    let (s, list) = call(&app, "GET", &format!("/v1/projects/{pid}/revisions"), None).await;
    assert_eq!(s, StatusCode::OK, "{list}");
    assert_eq!(list["revisions"][0]["label"], "r1");
    assert_eq!(list["revisions"][0]["modelSha256"], committed["modelSha256"]);

    let (s, one) = call(&app, "GET", &format!("/v1/projects/{pid}/revisions/1"), None).await;
    assert_eq!(s, StatusCode::OK, "{one}");
    assert_eq!(one["ops"], json!(ops), "the revision's ops are exactly what was committed");
    assert_eq!(one["revision"]["number"], 1);

    // Another project's revisions are not visible through this project's id.
    let (_, other) = call(&app, "POST", "/v1/projects", Some(json!({"payload": {"name": "Other"}}))).await;
    let oid = other["data"]["id"].as_str().unwrap();
    let (s, _) = call(&app, "GET", &format!("/v1/projects/{oid}/revisions/1"), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_committed_revision_excludes_undone_ops() {
    let Some(app) = app().await else { eprintln!("SKIPPED: HELQIX_TEST_DATABASE_URL not set"); return };
    let (_, project) = call(&app, "POST", "/v1/projects", Some(json!({"payload": {"name": "Untitled project"}}))).await;
    let pid = project["data"]["id"].as_str().unwrap().to_owned();
    let (_, build) = call(&app, "POST", &format!("/v1/projects/{pid}/builds"), None).await;
    let bid = build["buildId"].as_str().unwrap().to_owned();
    let (s, _) = call(&app, "POST", &format!("/v1/builds/{bid}/ops"), Some(json!({"ops": [part("base"), part("body")]}))).await;
    assert_eq!(s, StatusCode::OK);
    // Undo the last part: it stays in the stored log past the cursor but is not part of the model.
    let (s, _) = call(&app, "POST", &format!("/v1/builds/{bid}/undo"), None).await;
    assert_eq!(s, StatusCode::OK);
    let (s, committed) = call(&app, "POST", &format!("/v1/builds/{bid}/commit"), None).await;
    assert_eq!(s, StatusCode::CREATED, "{committed}");
    let (_, one) = call(&app, "GET", &format!("/v1/projects/{pid}/revisions/1"), None).await;
    assert_eq!(one["ops"], json!([part("base")]), "the undone part must not appear in the revision");
    assert_eq!(one["revision"]["opCount"], 1);
}
