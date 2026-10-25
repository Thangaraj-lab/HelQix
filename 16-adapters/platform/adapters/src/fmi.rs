//! FMI 2.0 co-simulation adapter (K3): runs an FMU through FMPy (`18-libraries/python/helqix_fmi`)
//! in the sandboxed subprocess runner. Outputs are Simulated and Unverified.
//!
//! Provenance: the FMU file's SHA-256 is added to the tool input as `fmu_sha256`, so the invocation
//! record's input hash identifies the exact model binary that ran.
//!
//! Determinism: **Tolerance**. A fixed-step FMU is deterministic on one machine, but compiled
//! native code may round differently on another platform, so the declared floor is
//! `TOLERANCE_FLOOR` (ASSUMPTION, absolute, in each output's SI unit). Exact state restore is not
//! supported. An FMU contains native code; only run FMUs from trusted sources.
//!
//! Lifecycle mapping (file 39 K3): Capabilities = [`SolverContract`]; Validate = input checks here
//! and in the tool (exit 2); Prepare = fresh sandbox directory; Execute = `run_tool`; Status and
//! Cancel = synchronous run with a hard timeout (no asynchronous status yet); Collect and Normalize =
//! output parsing with Pint units checked against the contract; Provenance = `InvocationRecord`.
use std::{path::{Path, PathBuf}, time::Duration};

use helqix_core::determinism::DeterminismLevel;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::solver::{run_tool, IoField, ProcessTool, SolverAdapter, SolverContract, SolverError, ToolRun};

/// Declared absolute tolerance floor across platforms (ASSUMPTION; see module docs).
pub const TOLERANCE_FLOOR: f64 = 1e-12;
const LAUNCHER: &str = "import sys; sys.path.insert(0, sys.argv[1]); from helqix_fmi.cli import main; main()";

pub struct FmiAdapter { contract: SolverContract, tool: ProcessTool, fmu: PathBuf }

impl FmiAdapter {
    /// `fmu` must be an absolute path; `outputs` are the variables to return with their SI unit
    /// (`"1"` for dimensionless). `time` in seconds is always returned first.
    pub fn new(python: PathBuf, package_dir: &Path, fmu: PathBuf, outputs: Vec<IoField>, timeout: Duration) -> Self {
        let mut all = vec![IoField { name: "time".into(), unit: "s".into() }];
        all.extend(outputs);
        let contract = SolverContract {
            tool: "fmpy".into(),
            fragment_types: vec!["fmi2.cosimulation".into()],
            inputs: vec![IoField { name: "parameters".into(), unit: "per FMU model description".into() }],
            outputs: all,
            determinism: DeterminismLevel::tolerance(TOLERANCE_FLOOR).expect("positive floor"),
            supports_state_restore: false,
            required_environment: vec!["Python with fmpy ==0.3.32 and pint ==0.26.2".into(), "18-libraries/python (helqix_fmi)".into()],
            timeout,
            max_output_bytes: 64 << 20,
            failure_modes: vec!["tool missing".into(), "timeout".into(), "invalid input (exit 2)".into(), "simulation failed (exit 3)".into(), "malformed output".into(), "unit mismatch".into(), "non-finite output".into()],
        };
        let tool = ProcessTool { program: python, args: vec!["-I".into(), "-c".into(), LAUNCHER.into(), package_dir.display().to_string()], timeout, max_output_bytes: 64 << 20, env_allowlist: Vec::new() };
        Self { contract, tool, fmu }
    }

    pub fn fmu_sha256(&self) -> Result<String, SolverError> {
        let bytes = std::fs::read(&self.fmu).map_err(|e| SolverError::InvalidInput(format!("cannot read FMU {}: {e}", self.fmu.display())))?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

impl SolverAdapter for FmiAdapter {
    fn contract(&self) -> &SolverContract { &self.contract }

    /// `input`: `{"parameters": {...}, "start_time"?, "stop_time"?, "step_size"?}`.
    fn run(&self, input: &Value) -> Result<ToolRun, SolverError> {
        run_tool(&self.tool, &self.contract, &self.tool_input(input)?)
    }

    /// Adds the FMU path, the output list and the FMU file's SHA-256 to the caller's input.
    fn tool_input(&self, input: &Value) -> Result<Value, SolverError> {
        let Some(obj) = input.as_object() else { return Err(SolverError::InvalidInput("input must be a JSON object".into())) };
        for key in obj.keys() {
            if !matches!(key.as_str(), "parameters" | "start_time" | "stop_time" | "step_size") { return Err(SolverError::InvalidInput(format!("unexpected field {key}"))); }
        }
        let mut doc = obj.clone();
        doc.insert("schema".into(), "helqix.fmi-input.v1".into());
        doc.insert("fmu_path".into(), self.fmu.display().to_string().into());
        doc.insert("fmu_sha256".into(), self.fmu_sha256()?.into());
        doc.insert("outputs".into(), self.contract.outputs.iter().skip(1).map(|f| Value::from(f.name.clone())).collect());
        Ok(Value::Object(doc))
    }
}
