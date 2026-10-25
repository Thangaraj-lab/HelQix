//! PyBaMM solver adapter: runs the battery domain pack's tool entry point
//! (`18-libraries/python/helqix_battery/adapter_cli.py`) as a sandboxed subprocess.
//!
//! Determinism: **Tolerance**. The declared tolerance follows a stated rule (see
//! [`measure_variation`]): at least ten times the run-to-run variation measured here, and never
//! below the solver's requested relative tolerance (1e-6) times the largest cell voltage (4.2 V),
//! because zero variation on one machine does not prove zero variation elsewhere.
//! Exact state restore is not supported.
use std::{path::PathBuf, time::Duration};

use async_trait::async_trait;
use helqix_core::determinism::DeterminismLevel;
use serde_json::{json, Map, Value};

use crate::{
    adapter::{Adapter, AdapterContext, AdapterError, AdapterRequest, AdapterResponse, HealthReport, HealthStatus},
    capability::{AdapterCapability, AdapterDescriptor},
    lifecycle::AdapterState,
    solver::{run_tool, IoField, ProcessTool, SolverAdapter, SolverContract, SolverError, ToolRun},
};

/// Solver relative tolerance requested by the domain pack (`helqix_battery.model.ModelConfig.rtol`).
pub const SOLVER_RTOL: f64 = 1e-6;
/// Largest terminal voltage in the cell's operating window (V).
pub const MAX_VOLTAGE_V: f64 = 4.2;
/// Fixed launcher: the package directory is passed as an argument, never interpolated into code.
const LAUNCHER: &str = "import sys; sys.path.insert(0, sys.argv[1]); from helqix_battery.adapter_cli import main; main()";

/// PyBaMM SPM / Chen2020 battery-cell adapter.
pub struct PybammAdapter { contract: SolverContract, tool: ProcessTool, descriptor: AdapterDescriptor }

/// Measured run-to-run variation and the tolerance declared from it.
#[derive(Debug, Clone, PartialEq)]
pub struct Variation { pub runs: usize, pub max_abs_deviation_v: f64, pub declared_tolerance_v: f64, pub margin_rule: &'static str }

impl PybammAdapter {
    /// `python` must have PyBaMM installed; `package_dir` is `18-libraries/python`.
    pub fn new(python: PathBuf, package_dir: PathBuf, timeout: Duration) -> Self {
        let floor = SOLVER_RTOL * MAX_VOLTAGE_V;
        let contract = SolverContract {
            tool: "pybamm".into(),
            fragment_types: vec!["battery.cell.spm_chen2020".into()],
            inputs: vec![IoField { name: "time_s".into(), unit: "s".into() }, IoField { name: "current_a".into(), unit: "A".into() },
                         IoField { name: "initial_ocv_v".into(), unit: "V".into() }, IoField { name: "contact_resistance_ohm".into(), unit: "Ohm".into() }],
            outputs: vec![IoField { name: "terminal_voltage".into(), unit: "V".into() }, IoField { name: "discharge_capacity".into(), unit: "A.h".into() }],
            determinism: DeterminismLevel::tolerance(floor).expect("positive floor"),
            supports_state_restore: false,
            required_environment: vec!["Python with pybamm >=26.6,<27".into(), "18-libraries/python (helqix_battery)".into()],
            timeout, max_output_bytes: 64 << 20,
            failure_modes: vec!["tool missing".into(), "timeout".into(), "invalid input (exit 2)".into(), "simulation incomplete (exit 3)".into(), "malformed output".into(), "unit mismatch".into(), "non-finite output".into()],
        };
        let tool = ProcessTool { program: python, args: vec!["-I".into(), "-c".into(), LAUNCHER.into(), package_dir.display().to_string()], timeout, max_output_bytes: 64 << 20, env_allowlist: Vec::new() };
        let descriptor = AdapterDescriptor {
            id: "pybamm".into(), name: "PyBaMM SPM battery cell".into(), version: "0.1.0".into(),
            capabilities: vec![AdapterCapability { name: "battery.cell.simulate".into(), operations: vec!["simulate".into()], supports_idempotency: true, supports_cancellation: false, max_concurrency: Some(2) }],
            provider: Some("pybamm (BSD-3-Clause)".into()), description: Some("Tolerance-level external solver; outputs are Simulated and Unverified".into()),
        };
        Self { contract, tool, descriptor }
    }
}

impl SolverAdapter for PybammAdapter {
    fn contract(&self) -> &SolverContract { &self.contract }
    fn run(&self, input: &Value) -> Result<ToolRun, SolverError> {
        if input["schema"] != "helqix.pybamm-input.v1" { return Err(SolverError::InvalidInput("schema must be helqix.pybamm-input.v1".into())); }
        for key in ["time_s", "current_a"] {
            if !input[key].as_array().is_some_and(|a| !a.is_empty() && a.iter().all(Value::is_number)) { return Err(SolverError::InvalidInput(format!("{key} must be a non-empty number array"))); }
        }
        if input["time_s"].as_array().map(Vec::len) != input["current_a"].as_array().map(Vec::len) { return Err(SolverError::InvalidInput("time_s and current_a must have the same length".into())); }
        run_tool(&self.tool, &self.contract, input)
    }
}

/// Runs the same input `runs` times and declares a tolerance from the measured spread.
pub fn measure_variation(adapter: &PybammAdapter, input: &Value, runs: usize) -> Result<Variation, SolverError> {
    if runs < 2 { return Err(SolverError::InvalidInput("variation needs at least 2 runs".into())); }
    let first = adapter.run(input)?;
    let reference = &first.outputs[0].values;
    let mut worst = 0.0_f64;
    for _ in 1..runs {
        let again = adapter.run(input)?;
        if again.outputs[0].values.len() != reference.len() { return Err(SolverError::MalformedOutput("sample count changed between runs".into())); }
        worst = again.outputs[0].values.iter().zip(reference).map(|(a, b)| (a - b).abs()).fold(worst, f64::max);
    }
    Ok(Variation { runs, max_abs_deviation_v: worst, declared_tolerance_v: (10.0 * worst).max(SOLVER_RTOL * MAX_VOLTAGE_V),
                   margin_rule: "declared = max(10 x measured max deviation, solver rtol 1e-6 x 4.2 V)" })
}

#[async_trait]
impl Adapter for PybammAdapter {
    fn descriptor(&self) -> &AdapterDescriptor { &self.descriptor }
    fn state(&self) -> AdapterState { AdapterState::Ready }
    async fn initialize(&self, context: &AdapterContext) -> Result<(), AdapterError> { context.check_active() }
    async fn execute(&self, context: &AdapterContext, request: AdapterRequest) -> Result<AdapterResponse, AdapterError> {
        context.check_active()?;
        request.validate()?;
        if request.operation != "simulate" { return Err(AdapterError::InvalidRequest(format!("unsupported operation {}", request.operation))); }
        let run = self.run(&request.payload).map_err(|e| AdapterError::Provider { code: "pybamm".into(), message: e.to_string(), retryable: matches!(e, SolverError::Timeout { .. }) })?;
        let mut metadata = Map::new();
        metadata.insert("determinism".into(), json!(self.contract.determinism.label()));
        metadata.insert("tool_version".into(), json!(run.record.tool_version));
        metadata.insert("input_sha256".into(), json!(run.record.input_sha256));
        metadata.insert("output_sha256".into(), json!(run.record.output_sha256));
        metadata.insert("wall_time_ms".into(), json!(run.record.wall_time_ms as u64));
        Ok(AdapterResponse {
            payload: json!({"outputs": run.outputs.iter().map(|o| json!({"name": o.name, "unit": o.unit, "values": o.values, "origin": o.origin, "status": o.status})).collect::<Vec<_>>()}),
            metadata,
        })
    }
    async fn health_check(&self, _context: &AdapterContext) -> Result<HealthReport, AdapterError> {
        Ok(HealthReport { status: HealthStatus::Healthy, checked_at: "unknown (no wall clock in the adapter)".into(), message: Some("health is proven only by a run".into()) })
    }
    async fn shutdown(&self, _context: &AdapterContext) -> Result<(), AdapterError> { Ok(()) }
}
