//! OIDC token checks with a key generated for the test (nothing secret is committed).
use std::collections::HashMap;

use aws_lc_rs::{encoding::AsDer, rsa::{KeyPair, KeySize}, signature::KeyPair as _};
use helqix_api::auth::Oidc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, encode};
use serde_json::{Value, json};

const ISS: &str = "https://sso.example.test/realms/helqix";
const AUD: &str = "helqix-api";

fn now() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() }

/// Reads one DER element: (tag, content, rest).
fn der(b: &[u8]) -> (u8, &[u8], &[u8]) {
    let (tag, first) = (b[0], b[1]);
    let (len, off) = if first < 0x80 { (first as usize, 2) } else { let n = (first & 0x7f) as usize; (b[2..2 + n].iter().fold(0usize, |a, x| (a << 8) | *x as usize), 2 + n) };
    (tag, &b[off..off + len], &b[off + len..])
}

/// aws-lc-rs exports PKCS#8; jsonwebtoken wants the PKCS#1 key that PKCS#8 wraps in its OCTET STRING.
fn pkcs1_from_pkcs8(pkcs8: &[u8]) -> Vec<u8> {
    let (_, seq, _) = der(pkcs8);
    let (_, _version, rest) = der(seq);
    let (_, _algorithm, rest) = der(rest);
    let (tag, key, _) = der(rest);
    assert_eq!(tag, 0x04, "the private key is an OCTET STRING");
    key.to_vec()
}

struct Keys { sign: EncodingKey, oidc: Oidc }

fn keys() -> Keys {
    let pair = KeyPair::generate(KeySize::Rsa2048).unwrap();
    let sign = EncodingKey::from_rsa_der(&pkcs1_from_pkcs8(AsDer::<aws_lc_rs::encoding::Pkcs8V1Der>::as_der(&pair).unwrap().as_ref()));
    let verify = DecodingKey::from_rsa_der(pair.public_key().as_ref());
    Keys { sign, oidc: Oidc::from_keys(ISS, AUD, HashMap::from([("k1".to_owned(), verify)])) }
}

fn token(k: &Keys, kid: &str, claims: Value) -> String {
    let mut h = Header::new(Algorithm::RS256);
    h.kid = Some(kid.to_owned());
    encode(&h, &claims, &k.sign).unwrap()
}

fn good() -> Value { json!({"sub": "user-1", "iss": ISS, "aud": AUD, "exp": now() + 600}) }

#[test]
fn a_valid_token_gives_its_subject() {
    let k = keys();
    assert_eq!(k.oidc.verify(&token(&k, "k1", good())).as_deref(), Some("user-1"));
}

#[test]
fn every_way_a_token_can_be_wrong_is_refused() {
    let k = keys();
    let with = |name: &str, v: Value| { let mut c = good(); c[name] = v; c };
    let without = |name: &str| { let mut c = good(); c.as_object_mut().unwrap().remove(name); c };
    assert!(k.oidc.verify(&token(&k, "k1", with("exp", json!(now() - 3600)))).is_none(), "expired");
    assert!(k.oidc.verify(&token(&k, "k1", with("iss", json!("https://evil.example")))).is_none(), "wrong issuer");
    assert!(k.oidc.verify(&token(&k, "k1", with("aud", json!("someone-else")))).is_none(), "wrong audience");
    assert!(k.oidc.verify(&token(&k, "k1", without("sub"))).is_none(), "no subject");
    assert!(k.oidc.verify(&token(&k, "k1", without("exp"))).is_none(), "no expiry");
    assert!(k.oidc.verify(&token(&k, "other", good())).is_none(), "unknown key id");
    let other = keys();
    assert!(k.oidc.verify(&token(&other, "k1", good())).is_none(), "signed by a different key");
    let mut t = token(&k, "k1", good());
    t.push('x');
    assert!(k.oidc.verify(&t).is_none(), "tampered signature");
    assert!(k.oidc.verify("not a token").is_none());
}

#[test]
fn a_token_using_a_symmetric_algorithm_is_refused() {
    let k = keys();
    let mut h = Header::new(Algorithm::HS256);
    h.kid = Some("k1".into());
    let t = encode(&h, &good(), &EncodingKey::from_secret(b"anything")).unwrap();
    assert!(k.oidc.verify(&t).is_none(), "alg confusion");
}

#[test]
fn a_jwks_document_is_read_and_a_bad_one_is_refused() {
    // The public example key from RFC 7517 appendix A.1 (public values only).
    let jwks = json!({"keys": [{"kty": "RSA", "kid": "k1", "alg": "RS256", "use": "sig", "e": "AQAB",
        "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw"}]});
    assert!(Oidc::from_jwks_json(ISS, AUD, &jwks.to_string()).is_ok());
    assert!(Oidc::from_jwks_json(ISS, AUD, "{\"keys\": []}").is_err());
    assert!(Oidc::from_jwks_json(ISS, AUD, "nonsense").is_err());
}
