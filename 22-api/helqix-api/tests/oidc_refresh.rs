//! E10.1: keys refreshed from the provider's JWKS URL, against a small local server whose keys the test rotates (nothing secret is committed).
use std::{sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}}, time::Duration};

use aws_lc_rs::{encoding::AsDer, rsa::{KeyPair, KeySize}, signature::KeyPair as _};
use axum::{Router, extract::State, http::StatusCode, routing::get};
use helqix_api::auth::Oidc;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::{Value, json};

const ISS: &str = "http://127.0.0.1/realms/helqix";
const AUD: &str = "helqix-api";

fn now() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() }

fn der(b: &[u8]) -> (u8, &[u8], &[u8]) {
    let (tag, first) = (b[0], b[1]);
    let (len, off) = if first < 0x80 { (first as usize, 2) } else { let n = (first & 0x7f) as usize; (b[2..2 + n].iter().fold(0usize, |a, x| (a << 8) | *x as usize), 2 + n) };
    (tag, &b[off..off + len], &b[off + len..])
}

fn b64url(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..=c.len() { out.push(A[((n >> (18 - 6 * i)) & 63) as usize] as char); }
    }
    out
}

struct Key { kid: String, sign: EncodingKey, jwk: Value }

fn key(kid: &str) -> Key {
    let pair = KeyPair::generate(KeySize::Rsa2048).unwrap();
    let pkcs8 = AsDer::<aws_lc_rs::encoding::Pkcs8V1Der>::as_der(&pair).unwrap();
    let (_, seq, _) = der(pkcs8.as_ref());
    let (_, _v, rest) = der(seq);
    let (_, _a, rest) = der(rest);
    let (_, pkcs1, _) = der(rest);
    let (_, seq, _) = der(pair.public_key().as_ref());
    let (_, n, rest) = der(seq);
    let (_, e, _) = der(rest);
    let n = if n[0] == 0 { &n[1..] } else { n };
    Key { kid: kid.to_owned(), sign: EncodingKey::from_rsa_der(pkcs1), jwk: json!({"kty": "RSA", "kid": kid, "use": "sig", "alg": "RS256", "n": b64url(n), "e": b64url(e)}) }
}

fn token(k: &Key, sub: &str) -> String {
    let mut h = Header::new(Algorithm::RS256);
    h.kid = Some(k.kid.clone());
    encode(&h, &json!({"sub": sub, "iss": ISS, "aud": AUD, "exp": now() + 600}), &k.sign).unwrap()
}

/// A JWKS endpoint whose document and status the test controls, counting the requests it gets.
struct Provider { url: String, doc: Arc<Mutex<(StatusCode, Value)>>, hits: Arc<AtomicUsize> }

async fn provider(keys: &[&Key]) -> Provider {
    #[derive(Clone)]
    struct S { doc: Arc<Mutex<(StatusCode, Value)>>, hits: Arc<AtomicUsize> }
    let doc = Arc::new(Mutex::new((StatusCode::OK, json!({"keys": keys.iter().map(|k| k.jwk.clone()).collect::<Vec<_>>()}))));
    let hits = Arc::new(AtomicUsize::new(0));
    let app = Router::new().route("/certs", get(|State(s): State<S>| async move { s.hits.fetch_add(1, Ordering::SeqCst); let d = s.doc.lock().unwrap().clone(); (d.0, axum::Json(d.1)) })).with_state(S { doc: doc.clone(), hits: hits.clone() });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/certs", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    Provider { url, doc, hits }
}

fn publish(p: &Provider, keys: &[&Key]) { *p.doc.lock().unwrap() = (StatusCode::OK, json!({"keys": keys.iter().map(|k| k.jwk.clone()).collect::<Vec<_>>()})); }

#[tokio::test]
async fn keys_are_fetched_at_start_and_a_rotation_is_followed_without_a_restart() {
    let (k1, k2) = (key("k1"), key("k2"));
    let p = provider(&[&k1]).await;
    let oidc = Oidc::from_url(ISS, AUD, &p.url, Duration::from_secs(0)).await.unwrap();
    assert_eq!(oidc.verify(&token(&k1, "alice")).as_deref(), Some("alice"));
    assert!(oidc.verify(&token(&k2, "bob")).is_none(), "k2 is not published yet");

    publish(&p, &[&k2]);
    let t2 = token(&k2, "bob");
    assert!(oidc.needs_refresh_for(&t2), "an unknown key id asks for a refresh");
    assert!(oidc.refresh().await);
    assert_eq!(oidc.verify(&t2).as_deref(), Some("bob"));
    assert!(oidc.verify(&token(&k1, "alice")).is_none(), "a key the provider stopped publishing stops working");
    assert!(!oidc.needs_refresh_for(&t2), "a known key id needs no refresh");
}

#[tokio::test]
async fn a_flood_of_unknown_key_ids_costs_the_provider_one_request_per_interval() {
    let k1 = key("k1");
    let p = provider(&[&k1]).await;
    let oidc = Oidc::from_url(ISS, AUD, &p.url, Duration::from_secs(60)).await.unwrap();
    let before = p.hits.load(Ordering::SeqCst);
    let stranger = key("random-kid");
    for _ in 0..20 { if oidc.needs_refresh_for(&token(&stranger, "x")) { oidc.refresh().await; } }
    assert!(p.hits.load(Ordering::SeqCst) - before <= 1, "hits: {}", p.hits.load(Ordering::SeqCst) - before);
    assert!(oidc.verify(&token(&stranger, "x")).is_none());
}

#[tokio::test]
async fn a_failing_provider_leaves_the_old_keys_in_place() {
    let k1 = key("k1");
    let p = provider(&[&k1]).await;
    let oidc = Oidc::from_url(ISS, AUD, &p.url, Duration::from_secs(0)).await.unwrap();
    *p.doc.lock().unwrap() = (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "down"}));
    assert!(!oidc.refresh().await, "a failed refresh reports failure");
    *p.doc.lock().unwrap() = (StatusCode::OK, json!({"keys": []}));
    assert!(!oidc.refresh().await, "a document with no usable key is a failure too");
    assert_eq!(oidc.verify(&token(&k1, "alice")).as_deref(), Some("alice"), "the keys that worked are still used");
}

#[tokio::test]
async fn start_up_fails_when_the_provider_cannot_be_reached_or_has_no_keys() {
    assert!(Oidc::from_url(ISS, AUD, "http://127.0.0.1:1/certs", Duration::from_secs(0)).await.is_err());
    let p = provider(&[]).await;
    assert!(Oidc::from_url(ISS, AUD, &p.url, Duration::from_secs(0)).await.is_err());
}

#[tokio::test]
async fn only_https_or_a_loopback_address_is_accepted_as_the_key_url() {
    for bad in ["http://sso.example.com/certs", "ftp://127.0.0.1/certs", "not a url", "file:///etc/passwd", "http://127.0.0.1.evil.example/certs"] {
        let e = Oidc::from_url(ISS, AUD, bad, Duration::from_secs(0)).await.err().unwrap_or_default();
        assert!(e.contains("https") || e.contains("URL"), "{bad}: {e:?}");
    }
}

// Through the router: a token signed with a key the API has never seen is refused, the provider publishes it, and the next request is accepted
// without a restart. Needs HELQIX_TEST_DATABASE_URL (prints SKIPPED otherwise).
#[tokio::test]
async fn the_router_follows_a_rotation_on_the_first_request_that_needs_it() {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let Ok(url) = std::env::var("HELQIX_TEST_DATABASE_URL") else { eprintln!("SKIPPED: HELQIX_TEST_DATABASE_URL not set"); return };
    let pool = sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let (k1, k2) = (key("k1"), key("k2"));
    let p = provider(&[&k1]).await;
    let oidc = Oidc::from_url(ISS, AUD, &p.url, Duration::from_secs(0)).await.unwrap();
    let app = helqix_api::routes::router(helqix_api::state::AppState::new(pool, "test-api-key-not-a-secret").with_oidc(Some(oidc)), &["http://localhost:5173".to_owned()], Duration::from_secs(10)).unwrap();
    let get = |t: String| { let app = app.clone(); async move { app.oneshot(Request::builder().uri("/v1/projects").header("authorization", format!("Bearer {t}")).body(Body::empty()).unwrap()).await.unwrap().status() } };
    assert_eq!(get(token(&k1, "alice")).await, StatusCode::OK);
    assert_eq!(get(token(&k2, "bob")).await, StatusCode::UNAUTHORIZED, "k2 is not published yet");
    publish(&p, &[&k1, &k2]);
    assert_eq!(get(token(&k2, "bob")).await, StatusCode::OK, "the provider rotated; the API followed on this request");
    assert_eq!(get(token(&k1, "alice")).await, StatusCode::OK);
}
