//! CalculiX adapter (K3): a cantilever benchmark solved by `ccx` (GPL-2.0) through
//! `18-libraries/python/helqix_calculix`. GPL boundary: ccx is a separate program started by the
//! tool as a subprocess; HELQIX neither links to nor ships it. Outputs are Simulated and Unverified.
//!
//! The ccx program path is fixed when the adapter is built and passed as a launcher argument;
//! request input cannot choose which program runs.
//!
//! Determinism: **Tolerance**, floor `TOLERANCE_FLOOR_M` (ASSUMPTION; single-threaded direct solve,
//! but different ccx builds and platforms may round differently). Exact state restore: no.
use std::{path::{Path, PathBuf}, time::Duration};

use helqix_core::determinism::DeterminismLevel;
use serde_json::Value;

use crate::solver::{run_tool, IoField, ProcessTool, SolverAdapter, SolverContract, SolverError, ToolRun};

/// Declared absolute tolerance on deflections, in metres (ASSUMPTION; see module docs).
pub const TOLERANCE_FLOOR_M: f64 = 1e-12;
const LAUNCHER: &str = "import sys; sys.path.insert(0, sys.argv[1]); from helqix_calculix.cli import main; main()";
const FIELDS: [&str; 7] = ["length_m", "width_m", "height_m", "youngs_modulus_pa", "poisson_ratio", "tip_load_n", "elements"];

pub struct CalculixAdapter { contract: SolverContract, tool: ProcessTool }

impl CalculixAdapter {
    /// `ccx`: the CalculiX program to run (for example `/usr/bin/ccx` from Ubuntu's `calculix-ccx`).
    pub fn new(python: PathBuf, package_dir: &Path, ccx: &str, timeout: Duration) -> Self {
        let m = |name: &str| IoField { name: name.into(), unit: "m".into() };
        let contract = SolverContract {
            tool: "calculix-ccx".into(),
            fragment_types: vec!["structural.linear_static.cantilever".into()],
            inputs: vec![
                IoField { name: "length_m".into(), unit: "m".into() }, IoField { name: "width_m".into(), unit: "m".into() }, IoField { name: "height_m".into(), unit: "m".into() },
                IoField { name: "youngs_modulus_pa".into(), unit: "Pa".into() }, IoField { name: "poisson_ratio".into(), unit: "1".into() },
                IoField { name: "tip_load_n".into(), unit: "N".into() }, IoField { name: "elements".into(), unit: "count".into() },
            ],
            outputs: vec![m("tip_deflection"), m("euler_bernoulli_deflection"), m("timoshenko_deflection")],
            determinism: DeterminismLevel::tolerance(TOLERANCE_FLOOR_M).expect("positive floor"),
            supports_state_restore: false,
            required_environment: vec!["CalculiX ccx (GPL-2.0, separate program; Ubuntu package calculix-ccx)".into(), "Python 3 with 18-libraries/python (helqix_calculix)".into()],
            timeout,
            max_output_bytes: 1 << 20,
            failure_modes: vec!["tool missing".into(), "timeout".into(), "invalid input (exit 2)".into(), "ccx failed or no result (exit 3)".into(), "malformed output".into(), "unit mismatch".into(), "non-finite output".into()],
        };
        let tool = ProcessTool { program: python, args: vec!["-I".into(), "-c".into(), LAUNCHER.into(), package_dir.display().to_string(), ccx.into()], timeout, max_output_bytes: 1 << 20, env_allowlist: Vec::new() };
        Self { contract, tool }
    }
}

impl SolverAdapter for CalculixAdapter {
    fn contract(&self) -> &SolverContract { &self.contract }

    fn run(&self, input: &Value) -> Result<ToolRun, SolverError> { run_tool(&self.tool, &self.contract, &self.tool_input(input)?) }

    /// Accepts only the beam fields and adds the schema tag.
    fn tool_input(&self, input: &Value) -> Result<Value, SolverError> {
        let Some(obj) = input.as_object() else { return Err(SolverError::InvalidInput("input must be a JSON object".into())) };
        if let Some(k) = obj.keys().find(|k| !FIELDS.contains(&k.as_str())) { return Err(SolverError::InvalidInput(format!("unexpected field {k}"))); }
        let mut doc = obj.clone();
        doc.insert("schema".into(), "helqix.calculix-input.v1".into());
        Ok(Value::Object(doc))
    }
}
