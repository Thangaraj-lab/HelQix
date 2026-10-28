use axum::{Json, extract::{Request, State}, http::{StatusCode, header}, middleware::Next, response::{IntoResponse, Response}};
use serde::Serialize;
use subtle::ConstantTimeEq;

use std::collections::HashMap;

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
/// expiry. Keys come from a JWKS document read at startup (key rotation therefore needs a restart or reload).
pub struct Oidc { issuer: String, audience: String, keys: HashMap<String, DecodingKey> }

impl Oidc {
    pub fn from_jwks_json(issuer: &str, audience: &str, jwks: &str) -> Result<Self, String> {
        let set: JwkSet = serde_json::from_str(jwks).map_err(|e| format!("the JWKS is not valid: {e}"))?;
        let mut keys = HashMap::new();
        for jwk in &set.keys {
            let Some(kid) = jwk.common.key_id.clone() else { continue };
            if let Ok(key) = DecodingKey::from_jwk(jwk) { keys.insert(kid, key); }
        }
        if keys.is_empty() { return Err("the JWKS has no usable key with a kid".into()); }
        Ok(Self { issuer: issuer.to_owned(), audience: audience.to_owned(), keys })
    }

    pub fn from_keys(issuer: &str, audience: &str, keys: HashMap<String, DecodingKey>) -> Self { Self { issuer: issuer.to_owned(), audience: audience.to_owned(), keys } }

    /// The token's subject when it is valid.
    pub fn verify(&self, token: &str) -> Option<String> {
        let header = decode_header(token).ok()?;
        if header.alg != Algorithm::RS256 { return None; }
        let key = self.keys.get(header.kid.as_deref()?)?;
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
