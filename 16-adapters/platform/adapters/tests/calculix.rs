//! K3 CalculiX adapter benchmark and failure paths. Needs HELQIX_PYTHON and a ccx program
//! (HELQIX_CCX, default `ccx`; CI installs Ubuntu's calculix-ccx). Example case: a steel-like
//! cantilever 1 m x 50 mm x 50 mm with a 1 kN tip load; numbers are a benchmark, not a design.
use std::{path::PathBuf, time::Duration};

use helqix_adapters::{calculix::CalculixAdapter, solver::{conformance, SolverAdapter, SolverError}};
use serde_json::{json, Value};

/// Benchmark acceptance: FE tip deflection within this fraction of the Timoshenko closed form
/// (ASSUMPTION for a 20 x 2 x 2 C3D8I mesh of a slender beam, L/h = 20).
const BENCHMARK_REL_TOL: f64 = 0.02;

fn root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..") }
fn python() -> PathBuf { std::env::var("HELQIX_PYTHON").expect("HELQIX_PYTHON must be set").into() }
fn ccx() -> String { std::env::var("HELQIX_CCX").unwrap_or_else(|_| "ccx".into()) }
fn adapter_with(ccx: &str, timeout: Duration) -> CalculixAdapter { CalculixAdapter::new(python(), &root().join("18-libraries/python"), ccx, timeout) }
fn adapter() -> CalculixAdapter { adapter_with(&ccx(), Duration::from_secs(300)) }
fn beam() -> Value {
    json!({"length_m": 1.0, "width_m": 0.05, "height_m": 0.05, "youngs_modulus_pa": 2.1e11, "poisson_ratio": 0.3, "tip_load_n": 1000.0, "elements": [20, 2, 2]})
}
fn out(run: &helqix_adapters::solver::ToolRun, name: &str) -> f64 { run.outputs.iter().find(|o| o.name == name).unwrap().values[0] }

#[test]
fn cantilever_matches_beam_theory_within_the_benchmark_tolerance() {
    let run = adapter().run(&beam()).unwrap();
    let (fe, eb, timo) = (out(&run, "tip_deflection"), out(&run, "euler_bernoulli_deflection"), out(&run, "timoshenko_deflection"));
    let i = 0.05_f64 * 0.05_f64.powi(3) / 12.0;
    assert!((eb - 1000.0 / (3.0 * 2.1e11 * i)).abs() < 1e-15, "closed form computed independently");
    let rel = (fe - timo).abs() / timo;
    println!("BENCHMARK ccx tip deflection {fe:.9e} m; Timoshenko {timo:.9e} m; Euler-Bernoulli {eb:.9e} m; relative error {rel:.3e}; {}", run.record.tool_version);
    assert!(fe > 0.0 && rel <= BENCHMARK_REL_TOL, "tip deflection {fe:e} m vs Timoshenko {timo:e} m: relative error {rel:.3e} > {BENCHMARK_REL_TOL}");
    assert_eq!((run.outputs[0].origin, run.outputs[0].status), ("Simulated", "Unverified"));
    assert!(run.record.tool_version.starts_with("ccx"));
}

#[test]
fn the_solve_is_linear_in_load_and_inverse_in_stiffness() {
    let base = out(&adapter().run(&beam()).unwrap(), "tip_deflection");
    let mut doubled = beam();
    doubled["tip_load_n"] = json!(2000.0);
    let d = out(&adapter().run(&doubled).unwrap(), "tip_deflection");
    assert!((d / base - 2.0).abs() < 1e-6, "2x load gives {:.9}x deflection", d / base);
    let mut stiff = beam();
    stiff["youngs_modulus_pa"] = json!(4.2e11);
    let s = out(&adapter().run(&stiff).unwrap(), "tip_deflection");
    assert!((base / s - 2.0).abs() < 1e-6, "2x modulus gives 1/{:.9} deflection", base / s);
}

#[test]
fn repeated_runs_are_identical_on_this_machine() {
    let a = adapter().run(&beam()).unwrap();
    let b = adapter().run(&beam()).unwrap();
    assert_eq!(a.record.output_sha256, b.record.output_sha256);
}

#[test]
fn invalid_input_missing_program_and_timeout_are_typed_errors() {
    assert!(matches!(adapter().run(&json!({"program": "/bin/sh"})), Err(SolverError::InvalidInput(_))), "requests cannot choose the program");
    let mut bad = beam();
    bad["poisson_ratio"] = json!(0.7);
    assert!(matches!(adapter().run(&bad), Err(SolverError::Crash { exit_status: Some(2), .. })));
    let missing = adapter_with("/nonexistent/ccx-not-installed", Duration::from_secs(60)).run(&beam());
    assert!(matches!(missing, Err(SolverError::Crash { exit_status: Some(3), .. })), "{missing:?}");
    let mut big = beam();
    big["elements"] = json!([200, 10, 10]);
    let slow = adapter_with(&ccx(), Duration::from_millis(800)).run(&big);
    assert!(matches!(slow, Err(SolverError::Timeout { .. })), "{slow:?}");
}

#[test]
fn the_conformance_kit_passes() {
    let report = conformance(&adapter(), &beam(), &[json!({"length_m": -1}), json!({"surprise": 1})]);
    assert!(report.pass, "{:?}", report.failures);
}
