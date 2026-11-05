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

// E4.3a: parameters stated in SI with their dimension are checked against the unit the FMU declares, and converted to it.
const ACCEL: [i32; 7] = [1, 0, -2, 0, 0, 0, 0];
const NONE: [i32; 7] = [0; 7];
fn ball() -> FmiAdapter { adapter("BouncingBall", &[("h", "m")], Duration::from_secs(120)) }
fn exit2(r: &Result<helqix_adapters::solver::ToolRun, SolverError>) -> bool { matches!(r, Err(SolverError::Crash { exit_status: Some(2), .. })) }

#[test]
fn si_parameters_are_accepted_when_the_dimension_matches_the_fmu_unit() {
    let base = ball().run(&json!({})).unwrap();
    let same = ball().run(&json!({"parameters_si": {"g": {"value_si": -9.81, "dimension": ACCEL}, "e": {"value_si": 0.7, "dimension": NONE}}})).unwrap();
    assert_eq!(base.outputs[1].values, same.outputs[1].values, "stating the default gravity and restitution changes nothing");
    let moon = ball().run(&json!({"parameters_si": {"g": {"value_si": -1.62, "dimension": ACCEL}}})).unwrap();
    assert_ne!(base.outputs[1].values, moon.outputs[1].values, "a different gravity gives a different fall");
}

#[test]
fn si_parameters_with_the_wrong_dimension_are_refused_before_the_run() {
    let mass = [0, 1, 0, 0, 0, 0, 0];
    let r = ball().run(&json!({"parameters_si": {"g": {"value_si": -9.81, "dimension": mass}}}));
    assert!(exit2(&r), "acceleration given as a mass: {r:?}");
    let r = ball().run(&json!({"parameters_si": {"e": {"value_si": 0.7, "dimension": ACCEL}}}));
    assert!(exit2(&r), "a dimensionless coefficient given as an acceleration: {r:?}");
}

#[test]
fn malformed_or_ambiguous_si_parameters_are_refused() {
    for bad in [
        json!({"parameters_si": {"g": {"value_si": -9.81}}}),
        json!({"parameters_si": {"g": {"value_si": -9.81, "dimension": [1, 0, -2]}}}),
        json!({"parameters_si": {"g": {"value_si": -9.81, "dimension": [1.5, 0, -2, 0, 0, 0, 0]}}}),
        json!({"parameters_si": {"g": {"value_si": "fast", "dimension": ACCEL}}}),
        json!({"parameters_si": {"g": {"value_si": -9.81, "dimension": ACCEL, "extra": 1}}}),
        json!({"parameters_si": {"no_such_variable": {"value_si": 1.0, "dimension": NONE}}}),
        json!({"parameters_si": [1, 2]}),
        json!({"parameters": {"g": -9.81}, "parameters_si": {"g": {"value_si": -9.81, "dimension": ACCEL}}}),
    ] {
        let r = ball().run(&bad);
        assert!(exit2(&r), "{bad} gave {r:?}");
    }
}

#[test]
fn an_output_declared_with_the_wildcard_unit_reports_the_unit_the_fmu_declares() {
    let run = adapter("BouncingBall", &[("h", helqix_adapters::solver::ANY_UNIT), ("v", helqix_adapters::solver::ANY_UNIT)], Duration::from_secs(120)).run(&json!({})).unwrap();
    assert_eq!(run.outputs.iter().map(|o| o.unit.as_str()).collect::<Vec<_>>(), ["s", "m", "m/s"], "the unit is the FMU's own, normalized to SI, never the wildcard");
    // an explicit unit is still checked
    let wrong = adapter("BouncingBall", &[("h", "mm")], Duration::from_secs(120)).run(&json!({}));
    assert!(matches!(wrong, Err(SolverError::UnitMismatch { .. })), "{wrong:?}");
}

// E4.4a: the FMU says what it contains, so a Studio can offer its parameters and outputs instead of asking for typed names.
fn var<'a>(d: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    d["variables"].as_array().unwrap().iter().find(|v| v["name"] == name).unwrap_or_else(|| panic!("no variable {name} in {d}"))
}

#[test]
fn describe_lists_variables_with_causality_unit_dimension_and_start_value() {
    let d = ball().describe().unwrap();
    assert_eq!(d["model_name"], "BouncingBall");
    assert_eq!(d["fmi_version"], "2.0");
    let (g, e, h) = (var(&d, "g"), var(&d, "e"), var(&d, "h"));
    assert_eq!((g["causality"].as_str(), g["unit"].as_str(), g["start"].as_f64()), (Some("parameter"), Some("m/s2"), Some(-9.81)));
    assert_eq!(g["dimension"], json!(ACCEL));
    assert_eq!(g["si_factor"].as_f64(), Some(1.0));
    assert_eq!((e["causality"].as_str(), e["start"].as_f64()), (Some("parameter"), Some(0.7)));
    assert_eq!(e["dimension"], json!(NONE), "no declared unit is dimensionless");
    assert_eq!((h["causality"].as_str(), h["unit"].as_str(), h["dimension"].clone()), (Some("output"), Some("m"), json!([1, 0, 0, 0, 0, 0, 0])));
}

#[test]
fn describe_never_runs_the_model_and_refuses_what_is_not_an_fmu() {
    let t = std::time::Instant::now();
    ball().describe().unwrap();
    assert!(t.elapsed() < Duration::from_secs(20), "describe reads the model description only");
    let missing = FmiAdapter::new(python(), &root().join("18-libraries/python"), root().join("no-such.fmu"), vec![], Duration::from_secs(30));
    assert!(missing.describe().is_err());
    let not_fmu = std::env::temp_dir().join(format!("helqix-not-an-fmu-{}.fmu", std::process::id()));
    std::fs::write(&not_fmu, b"this is not a zip").unwrap();
    let r = FmiAdapter::new(python(), &root().join("18-libraries/python"), not_fmu.clone(), vec![], Duration::from_secs(30)).describe();
    let _ = std::fs::remove_file(&not_fmu);
    assert!(matches!(r, Err(SolverError::Crash { exit_status: Some(2), .. })), "{r:?}");
}
