//! Solver adapter contract: external tools as capabilities, never as authorities.
//!
//! * A [`SolverContract`] declares the tool, the model fragments it accepts, input and output
//!   fields with units, the determinism level it can honestly offer (external tools: Tolerance or
//!   Statistical, never Bitwise), whether it supports exact state restore, required environment,
//!   resource limits and failure modes.
//! * [`run_tool`] runs a tool as a subprocess in a fresh working directory, with a cleared
//!   environment (allow-list only), an argument list (never a shell string), a timeout and an
//!   output cap. Every invocation is recorded: input hash, configuration, tool version, wall time,
//!   output hash, exit status, stderr summary.
//! * Tool output is untrusted: it is parsed and checked against the contract (names, units,
//!   finite numbers) before anything uses it, and enters as Origin Simulated, Status Unverified.
//! * Every failure is a typed [`SolverError`]; there are no silent partial results.
use std::{
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use helqix_core::determinism::DeterminismLevel;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

/// One named input or output with its unit.
#[derive(Debug, Clone, PartialEq)]
pub struct IoField { pub name: String, pub unit: String }

/// What an adapter promises.
#[derive(Debug, Clone, PartialEq)]
pub struct SolverContract {
    pub tool: String,
    pub fragment_types: Vec<String>,
    pub inputs: Vec<IoField>,
    pub outputs: Vec<IoField>,
    pub determinism: DeterminismLevel,
    /// Whether the tool can restore an exact saved state (FMI-style); false unless proven.
    pub supports_state_restore: bool,
    pub required_environment: Vec<String>,
    pub timeout: Duration,
    pub max_output_bytes: usize,
    pub failure_modes: Vec<String>,
}

/// Provenance of one invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct InvocationRecord {
    pub tool: String,
    pub tool_version: String,
    pub input_sha256: String,
    pub configuration: Value,
    pub wall_time_ms: u128,
    pub output_sha256: String,
    pub exit_status: Option<i32>,
    pub stderr_summary: String,
}

/// One validated output series. Always Simulated / Unverified when it leaves the adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputSeries { pub name: String, pub unit: String, pub values: Vec<f64>, pub origin: &'static str, pub status: &'static str }

/// A successful, validated run.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolRun { pub record: InvocationRecord, pub outputs: Vec<OutputSeries>, /** Exact tool stdout; `record.output_sha256` is its hash. */ pub raw_output: Vec<u8> }

/// Typed failure of a tool run.
#[derive(Debug, Clone, PartialEq)]
pub enum SolverError {
    ToolMissing(String),
    Timeout { seconds: f64 },
    Crash { exit_status: Option<i32>, stderr_summary: String },
    MalformedOutput(String),
    NonFinite { output: String },
    UnitMismatch { output: String, expected: String, found: String },
    InvalidInput(String),
    OutputTooLarge { limit: usize },
}

impl std::fmt::Display for SolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToolMissing(p) => write!(f, "tool not found: {p}"),
            Self::Timeout { seconds } => write!(f, "tool exceeded its {seconds} s time limit"),
            Self::Crash { exit_status, stderr_summary } => write!(f, "tool failed (exit {exit_status:?}): {stderr_summary}"),
            Self::MalformedOutput(m) => write!(f, "malformed tool output: {m}"),
            Self::NonFinite { output } => write!(f, "output {output} contains a non-finite value"),
            Self::UnitMismatch { output, expected, found } => write!(f, "output {output} has unit {found}, contract says {expected}"),
            Self::InvalidInput(m) => write!(f, "invalid input: {m}"),
            Self::OutputTooLarge { limit } => write!(f, "tool output exceeded {limit} bytes"),
        }
    }
}
impl std::error::Error for SolverError {}

/// How to start a tool: program and fixed arguments; the input goes to stdin as JSON.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessTool { pub program: PathBuf, pub args: Vec<String>, pub timeout: Duration, pub max_output_bytes: usize, pub env_allowlist: Vec<String> }

/// Environment variables every tool may see (process start-up needs); nothing else is passed.
pub const BASE_ENV_ALLOWLIST: [&str; 12] = ["PATH", "SYSTEMROOT", "SystemRoot", "WINDIR", "TEMP", "TMP", "TMPDIR", "HOME", "USERPROFILE", "LOCALAPPDATA", "APPDATA", "LANG"];

/// A solver exposed through the contract.
pub trait SolverAdapter {
    fn contract(&self) -> &SolverContract;
    fn run(&self, input: &Value) -> Result<ToolRun, SolverError>;
    /// The exact document the adapter sends to the tool for `input` (the invocation record hashes
    /// this). Adapters that add provenance, such as a model file hash, override it.
    fn tool_input(&self, input: &Value) -> Result<Value, SolverError> { Ok(input.clone()) }
}

fn sha256_hex(data: &[u8]) -> String { format!("{:x}", Sha256::digest(data)) }

fn summary(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();
    let start = trimmed.char_indices().rev().nth(499).map_or(0, |(i, _)| i);
    trimmed[start..].to_owned()
}

static RUN_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Runs a tool once, sandboxed, and validates its output against the contract.
pub fn run_tool(tool: &ProcessTool, contract: &SolverContract, input: &Value) -> Result<ToolRun, SolverError> {
    let input_bytes = serde_json::to_vec(input).map_err(|e| SolverError::InvalidInput(e.to_string()))?;
    let workdir = std::env::temp_dir().join(format!("helqix-tool-{}-{}", std::process::id(), RUN_COUNTER.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&workdir).map_err(|e| SolverError::Crash { exit_status: None, stderr_summary: format!("cannot create sandbox: {e}") })?;
    let result = run_in(tool, contract, input, &input_bytes, &workdir);
    let _ = std::fs::remove_dir_all(&workdir);
    result
}

fn run_in(tool: &ProcessTool, contract: &SolverContract, input: &Value, input_bytes: &[u8], workdir: &std::path::Path) -> Result<ToolRun, SolverError> {
    let mut command = Command::new(&tool.program);
    command.args(&tool.args).current_dir(workdir).env_clear().stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    for key in BASE_ENV_ALLOWLIST.iter().copied().chain(tool.env_allowlist.iter().map(String::as_str)) {
        if let Some(value) = std::env::var_os(key) { command.env(key, value); }
    }
    let started = Instant::now();
    let mut child = command.spawn().map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { SolverError::ToolMissing(tool.program.display().to_string()) } else { SolverError::Crash { exit_status: None, stderr_summary: e.to_string() } })?;

    let mut stdin = child.stdin.take().expect("piped stdin");
    let payload = input_bytes.to_vec();
    let writer = std::thread::spawn(move || { let _ = stdin.write_all(&payload); });
    let limit = tool.max_output_bytes;
    let reader = |mut pipe: Box<dyn Read + Send>| std::thread::spawn(move || { let mut buf = Vec::new(); let _ = (&mut pipe).take(limit as u64 + 1).read_to_end(&mut buf); buf });
    let out = reader(Box::new(child.stdout.take().expect("piped stdout")));
    let err = reader(Box::new(child.stderr.take().expect("piped stderr")));

    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| SolverError::Crash { exit_status: None, stderr_summary: e.to_string() })? { break status; }
        if started.elapsed() > tool.timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(SolverError::Timeout { seconds: tool.timeout.as_secs_f64() });
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let wall_time_ms = started.elapsed().as_millis();
    let _ = writer.join();
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    if stdout.len() > limit || stderr.len() > limit { return Err(SolverError::OutputTooLarge { limit }); }
    if !status.success() { return Err(SolverError::Crash { exit_status: status.code(), stderr_summary: summary(&stderr) }); }

    let doc: Value = serde_json::from_slice(&stdout).map_err(|e| SolverError::MalformedOutput(format!("not JSON: {e}")))?;
    let tool_version = doc["tool_version"].as_str().filter(|v| !v.trim().is_empty()).ok_or_else(|| SolverError::MalformedOutput("missing tool_version".into()))?.to_owned();
    let produced = doc["outputs"].as_array().ok_or_else(|| SolverError::MalformedOutput("missing outputs".into()))?;
    let mut outputs = Vec::new();
    for field in &contract.outputs {
        let item = produced.iter().find(|o| o["name"] == field.name.as_str()).ok_or_else(|| SolverError::MalformedOutput(format!("missing output {}", field.name)))?;
        let unit = item["unit"].as_str().ok_or_else(|| SolverError::MalformedOutput(format!("output {} has no unit", field.name)))?;
        if unit != field.unit { return Err(SolverError::UnitMismatch { output: field.name.clone(), expected: field.unit.clone(), found: unit.to_owned() }); }
        let raw = item["values"].as_array().filter(|v| !v.is_empty()).ok_or_else(|| SolverError::MalformedOutput(format!("output {} has no values", field.name)))?;
        let values: Vec<f64> = raw.iter().map(|v| v.as_f64().ok_or_else(|| SolverError::MalformedOutput(format!("output {} has a non-number", field.name)))).collect::<Result<_, _>>()?;
        if values.iter().any(|v| !v.is_finite()) { return Err(SolverError::NonFinite { output: field.name.clone() }); }
        outputs.push(OutputSeries { name: field.name.clone(), unit: unit.to_owned(), values, origin: "Simulated", status: "Unverified" });
    }
    let configuration = Value::Object(input.as_object().map(|m| m.iter().filter(|(_, v)| !v.is_array()).map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_else(Map::new));
    Ok(ToolRun {
        record: InvocationRecord { tool: contract.tool.clone(), tool_version, input_sha256: sha256_hex(input_bytes), configuration, wall_time_ms, output_sha256: sha256_hex(&stdout), exit_status: status.code(), stderr_summary: summary(&stderr) },
        outputs,
        raw_output: stdout,
    })
}

/// Result of the conformance kit.
#[derive(Debug, Clone, PartialEq)]
pub struct ConformanceReport { pub pass: bool, pub failures: Vec<String> }

/// Checks an adapter against the contract rules with one good input and some bad inputs.
pub fn conformance(adapter: &dyn SolverAdapter, good: &Value, bad: &[Value]) -> ConformanceReport {
    let c = adapter.contract();
    let mut failures = Vec::new();
    if c.tool.trim().is_empty() || c.fragment_types.is_empty() || c.inputs.is_empty() || c.outputs.is_empty() || c.failure_modes.is_empty() || c.required_environment.is_empty() {
        failures.push("contract is incomplete (tool, fragment types, inputs, outputs, environment and failure modes are required)".into());
    }
    if c.timeout.is_zero() || c.max_output_bytes == 0 { failures.push("contract must set a timeout and an output limit".into()); }
    if c.determinism == DeterminismLevel::Bitwise { failures.push("an external tool must not declare Bitwise determinism; use Tolerance or Statistical".into()); }
    match adapter.run(good) {
        Ok(run) => {
            for field in &c.outputs {
                if !run.outputs.iter().any(|o| o.name == field.name && o.unit == field.unit) { failures.push(format!("output {} ({}) missing from a good run", field.name, field.unit)); }
            }
            if run.outputs.iter().any(|o| o.origin != "Simulated" || o.status != "Unverified") { failures.push("outputs must enter as Simulated / Unverified".into()); }
            let r = &run.record;
            let input_hash = adapter.tool_input(good).ok().and_then(|v| serde_json::to_vec(&v).ok()).map(|b| sha256_hex(&b)).unwrap_or_default();
            if r.tool_version.is_empty() || r.input_sha256 != input_hash || r.output_sha256.len() != 64 || r.exit_status != Some(0) {
                failures.push("invocation record is incomplete or not hash-consistent".into());
            }
        }
        Err(e) => failures.push(format!("good input failed: {e}")),
    }
    for (i, input) in bad.iter().enumerate() {
        if adapter.run(input).is_ok() { failures.push(format!("bad input #{i} was accepted")); }
    }
    ConformanceReport { pass: failures.is_empty(), failures }
}
