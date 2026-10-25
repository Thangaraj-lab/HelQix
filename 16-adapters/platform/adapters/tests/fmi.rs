//! K3 FMI adapter benchmark and failure paths. Needs HELQIX_PYTHON (fmpy 0.3.32, pint 0.26.2) and
//! the Reference FMUs from `tools/fixtures/fetch_reference_fmus.py` (Example data, BSD-2-Clause).
//! Benchmark: Dahlquist (dx/dt = -k x, fixed-step explicit Euler, h = 0.1) against the reference
//! output shipped inside the FMU, and against the scheme's closed form x_n = (1 - k h)^n.
use std::{path::PathBuf, time::Duration};

use helqix_adapters::{
    fmi::{FmiAdapter, TOLERANCE_FLOOR},
    solver::{conformance, IoField, SolverAdapter, SolverError},
};
use serde_json::json;

fn root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..") }
fn cache() -> PathBuf { root().join(".cache/reference-fmus/v0.0.42") }
fn python() -> PathBuf { std::env::var("HELQIX_PYTHON").expect("HELQIX_PYTHON must point at a Python with fmpy").into() }
fn adapter(model: &str, outputs: &[(&str, &str)], timeout: Duration) -> FmiAdapter {
    let fmu = std::fs::canonicalize(cache().join("fmus").join(format!("{model}.fmu"))).expect("run tools/fixtures/fetch_reference_fmus.py first");
    let fmu = PathBuf::from(fmu.display().to_string().trim_start_matches(r"\\?\"));
    FmiAdapter::new(python(), &root().join("18-libraries/python"), fmu, outputs.iter().map(|(n, u)| IoField { name: (*n).into(), unit: (*u).into() }).collect(), timeout)
}
fn reference(model: &str, column: &str) -> (Vec<f64>, Vec<f64>) {
    let text = std::fs::read_to_string(cache().join("refs").join(format!("{model}_out.csv"))).unwrap();
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    let col = header.iter().position(|h| *h == column).unwrap();
    lines.filter(|l| !l.trim().is_empty()).map(|l| { let f: Vec<f64> = l.split(',').map(|x| x.parse().unwrap()).collect(); (f[0], f[col]) }).unzip()
}
fn dahlquist() -> FmiAdapter { adapter("Dahlquist", &[("x", "1")], Duration::from_secs(120)) }

#[test]
fn dahlquist_matches_the_shipped_reference_within_the_declared_tolerance() {
    let run = dahlquist().run(&json!({})).unwrap();
    let (t_ref, x_ref) = reference("Dahlquist", "x");
    assert_eq!((run.outputs[0].values.len(), run.outputs[1].values.len()), (t_ref.len(), x_ref.len()));
    let worst = run.outputs[1].values.iter().zip(&x_ref).map(|(a, b)| (a - b).abs()).fold(0.0_f64, f64::max);
    assert!(worst <= TOLERANCE_FLOOR, "max |x - reference| = {worst:e} > {TOLERANCE_FLOOR:e}");
    for (a, b) in run.outputs[0].values.iter().zip(&t_ref) { assert!((a - b).abs() < 1e-9, "time grid differs: {a} vs {b}"); }
    assert_eq!((run.outputs[1].origin, run.outputs[1].status), ("Simulated", "Unverified"));
    assert!(run.record.tool_version.starts_with("fmpy "));
    assert!(run.record.configuration["fmu_sha256"].as_str().unwrap().len() == 64, "the FMU hash is part of the provenance");
}

#[test]
fn dahlquist_follows_the_closed_form_of_its_scheme_not_the_exact_ode() {
    // The Reference FMUs integrate with a compiled-in FIXED_SOLVER_STEP of 0.1 s (sources/config.h),
    // so the scheme check uses h = 0.1 and varies k.
    for (k, h) in [(1.0_f64, 0.1_f64), (2.0, 0.1), (0.5, 0.1)] {
        let run = dahlquist().run(&json!({"parameters": {"k": k}, "step_size": h, "stop_time": 5.0})).unwrap();
        let x = &run.outputs[1].values;
        let scheme = x.iter().enumerate().map(|(n, v)| (v - (1.0 - k * h).powi(n as i32)).abs()).fold(0.0_f64, f64::max);
        assert!(scheme <= 1e-12, "k={k} h={h}: max |x_n - (1-kh)^n| = {scheme:e}");
        // Honesty check: fixed-step Euler is visibly different from the exact solution exp(-k t).
        let exact = run.outputs[0].values.iter().zip(x).map(|(t, v)| (v - (-k * t).exp()).abs()).fold(0.0_f64, f64::max);
        assert!(exact > 1e-3, "k={k} h={h}: the exact-ODE gap should be visible, got {exact:e}");
    }
}

#[test]
fn units_come_from_the_model_description_normalized_to_si() {
    let run = adapter("BouncingBall", &[("h", "m"), ("v", "m/s")], Duration::from_secs(120)).run(&json!({})).unwrap();
    assert_eq!(run.outputs.iter().map(|o| o.unit.as_str()).collect::<Vec<_>>(), ["s", "m", "m/s"]);
    let (_, h_ref) = reference("BouncingBall", "h");
    let worst = run.outputs[1].values.iter().zip(&h_ref).map(|(a, b)| (a - b).abs()).fold(0.0_f64, f64::max);
    assert!(worst <= TOLERANCE_FLOOR, "BouncingBall h max deviation {worst:e}");
    let wrong = adapter("BouncingBall", &[("h", "mm")], Duration::from_secs(120)).run(&json!({}));
    assert!(matches!(wrong, Err(SolverError::UnitMismatch { .. })), "{wrong:?}");
}

#[test]
fn repeated_runs_are_identical_on_this_machine() {
    let a = dahlquist().run(&json!({})).unwrap();
    let b = dahlquist().run(&json!({})).unwrap();
    assert_eq!(a.raw_output, b.raw_output);
    assert_eq!(a.record.output_sha256, b.record.output_sha256);
}

#[test]
fn invalid_inputs_are_typed_errors() {
    let ad = dahlquist();
    assert!(matches!(ad.run(&json!({"fmu_path": "elsewhere.fmu"})), Err(SolverError::InvalidInput(_))), "callers cannot redirect the FMU");
    assert!(matches!(ad.run(&json!([])), Err(SolverError::InvalidInput(_))));
    let r = ad.run(&json!({"parameters": {"not_a_variable": 1.0}}));
    assert!(matches!(r, Err(SolverError::Crash { exit_status: Some(2), .. })), "{r:?}");
    let r = ad.run(&json!({"step_size": -1.0}));
    assert!(matches!(r, Err(SolverError::Crash { exit_status: Some(2), .. })), "{r:?}");
    let missing = FmiAdapter::new(python(), &root().join("18-libraries/python"), root().join("no-such.fmu"), vec![IoField { name: "x".into(), unit: "1".into() }], Duration::from_secs(30));
    assert!(matches!(missing.run(&json!({})), Err(SolverError::InvalidInput(_))));
}

#[test]
fn a_long_run_is_stopped_by_the_timeout() {
    let ad = adapter("VanDerPol", &[("x0", "1")], Duration::from_millis(1500));
    let r = ad.run(&json!({"stop_time": 1.0e5, "step_size": 1.0e-4}));
    assert!(matches!(r, Err(SolverError::Timeout { .. })), "{r:?}");
}

#[test]
fn the_conformance_kit_passes() {
    let report = conformance(&dahlquist(), &json!({}), &[json!({"parameters": {"k": "fast"}}), json!({"surprise": 1})]);
    assert!(report.pass, "{:?}", report.failures);
}
