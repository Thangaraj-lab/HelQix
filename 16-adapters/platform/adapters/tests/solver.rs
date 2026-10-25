//! Solver adapter contract tests with fake tools (plain Python scripts; no PyBaMM needed).
use std::{path::PathBuf, time::Duration};

use helqix_adapters::solver::{conformance, run_tool, IoField, ProcessTool, SolverAdapter, SolverContract, SolverError, ToolRun};
use helqix_core::determinism::DeterminismLevel;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn python() -> PathBuf {
    if let Ok(p) = std::env::var("HELQIX_PYTHON") { return p.into(); }
    if cfg!(windows) { "python".into() } else { "python3".into() }
}

fn contract() -> SolverContract {
    SolverContract {
        tool: "fake".into(), fragment_types: vec!["test".into()],
        inputs: vec![IoField { name: "x".into(), unit: "1".into() }],
        outputs: vec![IoField { name: "y".into(), unit: "V".into() }],
        determinism: DeterminismLevel::tolerance(1e-9).unwrap(), supports_state_restore: false,
        required_environment: vec!["python".into()], timeout: Duration::from_secs(20), max_output_bytes: 1 << 20,
        failure_modes: vec!["tool missing".into(), "timeout".into(), "crash".into(), "malformed output".into(), "unit mismatch".into()],
    }
}

/// A fake tool whose behaviour is a short Python program reading the JSON input on stdin.
fn fake(body: &str) -> ProcessTool {
    let script = format!("import sys, json, os\ndoc = json.loads(sys.stdin.read())\n{body}\n");
    ProcessTool { program: python(), args: vec!["-I".into(), "-c".into(), script], timeout: Duration::from_secs(20), max_output_bytes: 1 << 20, env_allowlist: Vec::new() }
}

const GOOD: &str = "print(json.dumps({'tool_version': '9.9.9', 'outputs': [{'name': 'y', 'unit': 'V', 'values': [doc['x'], doc['x'] * 2]}]}))";

struct Fake { tool: ProcessTool, contract: SolverContract }
impl SolverAdapter for Fake {
    fn contract(&self) -> &SolverContract { &self.contract }
    fn run(&self, input: &Value) -> Result<ToolRun, SolverError> {
        if !input["x"].is_number() { return Err(SolverError::InvalidInput("x must be a number".into())); }
        run_tool(&self.tool, &self.contract, input)
    }
}

#[test]
fn a_good_run_is_validated_labelled_and_fully_recorded() {
    let input = json!({"x": 1.5});
    let run = run_tool(&fake(GOOD), &contract(), &input).unwrap();
    assert_eq!(run.outputs[0].values, vec![1.5, 3.0]);
    assert_eq!((run.outputs[0].origin, run.outputs[0].status), ("Simulated", "Unverified"));
    let r = &run.record;
    assert_eq!(r.tool_version, "9.9.9");
    assert_eq!(r.exit_status, Some(0));
    assert_eq!(r.input_sha256, format!("{:x}", Sha256::digest(serde_json::to_vec(&input).unwrap())));
    assert_eq!(r.output_sha256.len(), 64);
    assert_eq!(r.output_sha256, format!("{:x}", Sha256::digest(&run.raw_output)));
}

#[test]
fn a_missing_tool_is_a_typed_error() {
    let mut t = fake(GOOD);
    t.program = "helqix-definitely-not-installed-tool".into();
    assert!(matches!(run_tool(&t, &contract(), &json!({"x": 1})), Err(SolverError::ToolMissing(_))));
}

#[test]
fn a_hanging_tool_times_out() {
    let mut t = fake("import time\ntime.sleep(30)");
    t.timeout = Duration::from_millis(1500);
    assert!(matches!(run_tool(&t, &contract(), &json!({"x": 1})), Err(SolverError::Timeout { .. })));
}

#[test]
fn a_crash_keeps_exit_status_and_stderr() {
    let r = run_tool(&fake("sys.stderr.write('boom happened')\nsys.exit(5)"), &contract(), &json!({"x": 1}));
    match r { Err(SolverError::Crash { exit_status, stderr_summary }) => { assert_eq!(exit_status, Some(5)); assert!(stderr_summary.contains("boom")); } other => panic!("{other:?}") }
}

#[test]
fn malformed_and_non_finite_output_is_rejected() {
    for body in ["print('not json')", "print(json.dumps({'tool_version': '1', 'outputs': []}))", "print('{\"tool_version\": \"1\", \"outputs\": [{\"name\": \"y\", \"unit\": \"V\", \"values\": [NaN]}]}')"] {
        let r = run_tool(&fake(body), &contract(), &json!({"x": 1}));
        assert!(matches!(r, Err(SolverError::MalformedOutput(_)) | Err(SolverError::NonFinite { .. })), "{body}: {r:?}");
    }
}

#[test]
fn a_unit_mismatch_is_rejected() {
    let r = run_tool(&fake("print(json.dumps({'tool_version': '1', 'outputs': [{'name': 'y', 'unit': 'mV', 'values': [1.0]}]}))"), &contract(), &json!({"x": 1}));
    assert!(matches!(r, Err(SolverError::UnitMismatch { .. })), "{r:?}");
}

#[test]
fn oversized_output_is_rejected() {
    let mut t = fake("print('x' * 5000)");
    t.max_output_bytes = 1000;
    assert!(matches!(run_tool(&t, &contract(), &json!({"x": 1})), Err(SolverError::OutputTooLarge { .. })));
}

#[test]
fn the_tool_runs_in_a_fresh_directory_without_inherited_secrets() {
    std::env::set_var("HELQIX_TEST_SECRET", "must-not-leak");
    let body = "print(json.dumps({'tool_version': '1', 'outputs': [{'name': 'y', 'unit': 'V', 'values': [float(len(os.listdir('.'))), float('HELQIX_TEST_SECRET' in os.environ)]}]}))";
    let run = run_tool(&fake(body), &contract(), &json!({"x": 1})).unwrap();
    assert_eq!(run.outputs[0].values, vec![0.0, 0.0]);
}

#[test]
fn shell_characters_in_the_input_are_data_not_commands() {
    let marker = "$(echo hacked); rm -rf / && `id`";
    let body = "print(json.dumps({'tool_version': '1', 'outputs': [{'name': 'y', 'unit': 'V', 'values': [float(len(doc['note']))]}]}))";
    let run = run_tool(&fake(body), &contract(), &json!({"x": 1, "note": marker})).unwrap();
    assert_eq!(run.outputs[0].values, vec![marker.len() as f64]);
}

#[test]
fn the_conformance_kit_passes_a_good_adapter_and_fails_a_bad_one() {
    let good = Fake { tool: fake(GOOD), contract: contract() };
    let report = conformance(&good, &json!({"x": 2.0}), &[json!({"x": "two"})]);
    assert!(report.pass, "{:?}", report.failures);
    let wrong_unit = Fake { tool: fake("print(json.dumps({'tool_version': '1', 'outputs': [{'name': 'y', 'unit': 'A', 'values': [1.0]}]}))"), contract: contract() };
    assert!(!conformance(&wrong_unit, &json!({"x": 2.0}), &[json!({"x": "two"})]).pass);
    let mut bitwise = contract();
    bitwise.determinism = DeterminismLevel::Bitwise;
    let overclaims = Fake { tool: fake(GOOD), contract: bitwise };
    let r = conformance(&overclaims, &json!({"x": 2.0}), &[json!({"x": "two"})]);
    assert!(r.failures.iter().any(|f| f.contains("Bitwise")), "{:?}", r.failures);
}
