//! PyBaMM adapter tests (feature `pybamm-tests`; need a Python with PyBaMM, set HELQIX_PYTHON).
use std::{future::Future, path::PathBuf, pin::pin, sync::Arc, task::{Context, Poll, Wake, Waker}, time::Duration};

use helqix_adapters::{
    adapter::{Adapter, AdapterContext, AdapterRequest},
    pybamm::{measure_variation, PybammAdapter},
    solver::{conformance, SolverAdapter},
};
use serde_json::json;

struct Noop;
impl Wake for Noop { fn wake(self: Arc<Self>) {} }
fn block_on<F: Future>(f: F) -> F::Output {
    let waker = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&waker);
    let mut f = pin!(f);
    loop { if let Poll::Ready(v) = f.as_mut().poll(&mut cx) { return v; } }
}

fn adapter() -> PybammAdapter {
    let python = std::env::var("HELQIX_PYTHON").map(PathBuf::from).unwrap_or_else(|_| if cfg!(windows) { "python".into() } else { "python3".into() });
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../18-libraries/python");
    PybammAdapter::new(python, package, Duration::from_secs(300))
}

fn input(current: f64, seconds: u32) -> serde_json::Value {
    let time: Vec<f64> = (0..=seconds).step_by(30).map(f64::from).collect();
    json!({"schema": "helqix.pybamm-input.v1", "time_s": time, "current_a": vec![current; time.len()], "initial_ocv_v": 4.18, "contact_resistance_ohm": 0.0})
}

#[test]
fn passes_the_conformance_kit() {
    let a = adapter();
    let report = conformance(&a, &input(5.0, 600), &[json!({"schema": "helqix.pybamm-input.v1", "time_s": [0, 1], "current_a": [1], "initial_ocv_v": 4.18}), json!({"schema": "wrong"})]);
    assert!(report.pass, "{:?}", report.failures);
}

#[test]
fn declares_tolerance_and_no_state_restore() {
    let c = adapter().contract().clone();
    assert_eq!(c.determinism.label(), "Tolerance");
    assert!(!c.supports_state_restore);
}

#[test]
fn run_to_run_variation_is_measured_not_assumed() {
    let v = measure_variation(&adapter(), &input(5.0, 600), 5).unwrap();
    assert_eq!(v.runs, 5);
    assert!(v.max_abs_deviation_v.is_finite());
    assert!(v.declared_tolerance_v > 0.0 && v.declared_tolerance_v >= v.max_abs_deviation_v);
}

#[test]
fn cross_check_against_charge_conservation() {
    // Closed form: discharged charge = I * t / 3600 (A.h). Error is recorded, not assumed zero.
    let run = adapter().run(&input(5.0, 1200)).unwrap();
    let capacity = run.outputs.iter().find(|o| o.name == "discharge_capacity").unwrap();
    let worst = capacity.values.iter().enumerate().map(|(i, q)| (q - 5.0 * (i as f64 * 30.0) / 3600.0).abs()).fold(0.0_f64, f64::max);
    assert!(worst < 1e-6, "charge conservation error {worst} A.h");
}

#[test]
fn simulation_failure_is_a_typed_error() {
    // 60 A for 2 h drives the model below its limit: the tool exits 3 and the adapter reports a crash.
    assert!(adapter().run(&input(60.0, 7200)).is_err());
}

#[test]
fn the_async_adapter_contract_delegates_to_the_solver() {
    let a = adapter();
    let ctx = AdapterContext::new("test-1", Duration::from_secs(300)).unwrap();
    let response = block_on(a.execute(&ctx, AdapterRequest { operation: "simulate".into(), payload: input(5.0, 300), idempotency_key: None })).unwrap();
    assert_eq!(response.payload["outputs"][0]["origin"], "Simulated");
    assert_eq!(response.metadata["determinism"], "Tolerance");
}
