use axum::{Json, extract::{Request, State}, http::{StatusCode, header}, middleware::Next, response::{IntoResponse, Response}};
use serde::Serialize;
use subtle::ConstantTimeEq;

use std::{collections::HashMap, sync::{Mutex, RwLock}, time::{Duration, Instant}};

use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};

use crate::state::AppState;

/// Who is calling. The shared service key is the operator or a trusted backend and sees everything; a signed-in
/// user (an OIDC access token, for example from Keycloak) sees only what they own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identity { Service, User(String) }

impl Identity {
    /// The owner recorded on new records: the user's subject, or `service` for the operator key.
    pub fn owner(&self) -> &str { match self { Self::Service => "service", Self::User(sub) => sub } }
    pub fn is_service(&self) -> bool { matches!(self, Self::Service) }
}

/// Validates OIDC access tokens: RS256 only, a known `kid`, the configured issuer and audience, and a valid
/// expiry. Keys come from a JWKS document read at startup, or from the provider's JWKS URL: then they are fetched at start and
/// again when a token names a key id the API does not know (a rotation), at most once per `min_interval`, and the provider's current
/// set replaces the old one, so a key the provider stopped publishing stops working. A failed fetch keeps the keys that worked.
pub struct Oidc { issuer: String, audience: String, keys: RwLock<HashMap<String, DecodingKey>>, source: Option<Source> }

struct Source { url: String, client: reqwest::Client, min_interval: Duration, last: Mutex<Option<Instant>> }

fn keys_from_jwks(jwks: &str) -> Result<HashMap<String, DecodingKey>, String> {
    let set: JwkSet = serde_json::from_str(jwks).map_err(|e| format!("the JWKS is not valid: {e}"))?;
    let mut keys = HashMap::new();
    for jwk in &set.keys {
        let Some(kid) = jwk.common.key_id.clone() else { continue };
        if let Ok(key) = DecodingKey::from_jwk(jwk) { keys.insert(kid, key); }
    }
    if keys.is_empty() { return Err("the JWKS has no usable key with a kid".into()); }
    Ok(keys)
}

/// The key URL must be https, or http only for a loopback address (a local identity provider).
fn check_key_url(url: &str) -> Result<(), String> {
    let u = reqwest::Url::parse(url).map_err(|_| "the key URL is not a valid URL".to_owned())?;
    let host = u.host_str().unwrap_or("");
    let loopback = host == "localhost" || host.trim_matches(|c| c == '[' || c == ']').parse::<std::net::IpAddr>().is_ok_and(|a| a.is_loopback());
    match u.scheme() {
        "https" => Ok(()),
        "http" if loopback => Ok(()),
        _ => Err("the key URL must be https (http is accepted only for a loopback address)".into()),
    }
}

impl Oidc {
    pub fn from_jwks_json(issuer: &str, audience: &str, jwks: &str) -> Result<Self, String> {
        Ok(Self { issuer: issuer.to_owned(), audience: audience.to_owned(), keys: RwLock::new(keys_from_jwks(jwks)?), source: None })
    }

    pub fn from_keys(issuer: &str, audience: &str, keys: HashMap<String, DecodingKey>) -> Self { Self { issuer: issuer.to_owned(), audience: audience.to_owned(), keys: RwLock::new(keys), source: None } }

    /// Keys from the provider's JWKS URL, fetched now (an unreachable provider or an empty key set stops start-up) and refreshed on a rotation.
    pub async fn from_url(issuer: &str, audience: &str, url: &str, min_interval: Duration) -> Result<Self, String> {
        check_key_url(url)?;
        let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).redirect(reqwest::redirect::Policy::none()).build().map_err(|e| e.to_string())?;
        let keys = fetch_keys(&client, url).await?;
        Ok(Self { issuer: issuer.to_owned(), audience: audience.to_owned(), keys: RwLock::new(keys), source: Some(Source { url: url.to_owned(), client, min_interval, last: Mutex::new(Some(Instant::now())) }) })
    }

    /// True when the keys come from a URL and this token names a key id the API does not know: the provider may have rotated its keys.
    pub fn needs_refresh_for(&self, token: &str) -> bool {
        if self.source.is_none() { return false; }
        let Ok(header) = decode_header(token) else { return false };
        header.alg == Algorithm::RS256 && header.kid.as_deref().is_some_and(|kid| !self.keys.read().expect("keys lock").contains_key(kid))
    }

    /// Fetches the provider's keys again (at most once per interval, so unknown key ids cannot be used to hammer the provider). True when the
    /// key set was replaced; false when it was too soon, the fetch failed, or the answer had no usable key (the old keys stay).
    pub async fn refresh(&self) -> bool {
        let Some(src) = &self.source else { return false };
        {
            let mut last = src.last.lock().expect("refresh lock");
            if last.is_some_and(|t| t.elapsed() < src.min_interval) { return false; }
            *last = Some(Instant::now());
        }
        match fetch_keys(&src.client, &src.url).await {
            Ok(keys) => { *self.keys.write().expect("keys lock") = keys; true }
            Err(error) => { tracing::warn!(%error, "could not refresh the identity provider's keys; the old keys stay in use"); false }
        }
    }

    /// The token's subject when it is valid.
    pub fn verify(&self, token: &str) -> Option<String> {
        let header = decode_header(token).ok()?;
        if header.alg != Algorithm::RS256 { return None; }
        let keys = self.keys.read().expect("keys lock");
        let key = keys.get(header.kid.as_deref()?)?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        validation.leeway = 30;
        #[derive(serde::Deserialize)]
        struct Claims { sub: String }
        let data = decode::<Claims>(token, key, &validation).ok()?;
        (!data.claims.sub.is_empty() && data.claims.sub.len() <= 128).then_some(data.claims.sub)
    }
}

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
    let identity = supplied.and_then(|token| {
        if bool::from(token.as_bytes().ct_eq(state.api_key.as_bytes())) { return Some(Identity::Service); }
        state.oidc.as_ref().and_then(|o| o.verify(token)).map(Identity::User)
    });
    let identity = match (identity, supplied, state.oidc.as_ref()) {
        (None, Some(token), Some(o)) if o.needs_refresh_for(token) => { o.refresh().await; o.verify(token).map(Identity::User) }
        (found, _, _) => found,
    };
    let Some(identity) = identity else {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer")],
            Json(AuthError { error: AuthErrorDetails {
                code: "unauthorized",
                message: "Valid API credentials are required.",
            } }),
        ).into_response();
    };
    let mut request = request;
    request.extensions_mut().insert(identity);
    next.run(request).await
}

async fn fetch_keys(client: &reqwest::Client, url: &str) -> Result<HashMap<String, DecodingKey>, String> {
    let res = client.get(url).send().await.map_err(|e| format!("cannot reach the identity provider's keys: {e}"))?;
    if !res.status().is_success() { return Err(format!("the identity provider's key URL answered {}", res.status())); }
    let text = res.text().await.map_err(|e| format!("cannot read the identity provider's keys: {e}"))?;
    keys_from_jwks(&text)
}
