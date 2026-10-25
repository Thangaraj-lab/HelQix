//! Known-answer tests for the encodings the attestation uses (from the DSSE spec and RFC 4648).
use helqix_evidence_verify::attestation::{b64_decode, b64_encode, pae};

#[test]
fn dsse_pre_authentication_encoding_matches_the_spec_example() {
    assert_eq!(pae("http://example.com/HelloWorld", b"hello world"), b"DSSEv1 29 http://example.com/HelloWorld 11 hello world".to_vec());
}

#[test]
fn base64_matches_rfc_4648_vectors_and_rejects_bad_input() {
    for (plain, enc) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("fooba", "Zm9vYmE="), ("foobar", "Zm9vYmFy")] {
        assert_eq!(b64_encode(plain.as_bytes()), enc);
        assert_eq!(b64_decode(enc).unwrap(), plain.as_bytes());
    }
    let all: Vec<u8> = (0..=255).collect();
    assert_eq!(b64_decode(&b64_encode(&all)).unwrap(), all);
    for bad in ["Zg=", "Z===", "Zg==Zg==", "Zm9v!", "Zm 9v"] { assert!(b64_decode(bad).is_none(), "{bad}"); }
}
