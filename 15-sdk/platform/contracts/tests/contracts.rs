//! Contract tests. The shared fixtures in `fixtures/contracts.json` must be accepted (valid) or
//! rejected (invalid) by serde here, and by the JSON Schemas in `tests_py/test_schemas.py`.
//! Valid fixtures must also round-trip unchanged. The committed schema files must match the code.
use helqix_contracts::{schemas, BuildEvent, ModelOp, ModelRevision, RunFrame, Value};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value as Json;

fn fixtures() -> Json {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/contracts.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn check<T: DeserializeOwned + Serialize>(name: &str) {
    let f = fixtures();
    let valid = f[name]["valid"].as_array().unwrap_or_else(|| panic!("no valid fixtures for {name}"));
    let invalid = f[name]["invalid"].as_array().unwrap_or_else(|| panic!("no invalid fixtures for {name}"));
    assert!(!valid.is_empty() && !invalid.is_empty());
    for doc in valid {
        let parsed: T = serde_json::from_value(doc.clone()).unwrap_or_else(|e| panic!("{name} rejected valid fixture {doc}: {e}"));
        assert_eq!(&serde_json::to_value(parsed).unwrap(), doc, "{name} round trip changed {doc}");
    }
    for doc in invalid {
        assert!(serde_json::from_value::<T>(doc.clone()).is_err(), "{name} accepted invalid fixture {doc}");
    }
}

#[test]
fn model_ops_cover_every_op_and_reject_bare_numbers() {
    check::<ModelOp>("ModelOp");
    let ops: Vec<String> = fixtures()["ModelOp"]["valid"].as_array().unwrap().iter().map(|o| o["op"].as_str().unwrap().to_owned()).collect();
    for op in ["part.add", "part.setParam", "part.place", "port.add", "connection.add", "material.assign", "binding.add", "requirement.add", "part.remove", "connection.remove"] {
        assert!(ops.iter().any(|o| o == op), "no fixture for {op}");
    }
}

#[test]
fn values_have_closed_origin_and_status() { check::<Value>("Value"); }

#[test]
fn build_events_have_no_timer_kinds() { check::<BuildEvent>("BuildEvent"); }

#[test]
fn revisions_and_frames() {
    check::<ModelRevision>("ModelRevision");
    check::<RunFrame>("RunFrame");
}

#[test]
fn committed_schema_files_match_the_code() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("schema");
    for (name, schema) in schemas() {
        let path = dir.join(format!("{name}.schema.json"));
        let committed = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing {}; run helqix-contracts-schema", path.display()));
        let expected = serde_json::to_string_pretty(&schema).unwrap() + "\n";
        assert_eq!(committed.replace("\r\n", "\n"), expected, "{name}.schema.json is out of date; run: cargo run -p helqix-contracts --features schema --bin helqix-contracts-schema -- 15-sdk/platform/contracts/schema");
    }
}
