//! Running a committed model's FMU through FMPy (E4.3).
//!
//! The FMU files live in an **operator-installed library**: a directory (`HELQIX_FMU_STORE`) of files named `<sha256>.fmu`.
//! There is deliberately no upload route. An FMU contains native code, so running one runs that code on the server: only
//! files an operator put there are ever run, and a file is re-hashed on every use, so a swapped or edited file is refused.
//! A model names its FMU by hash (`part.setBehavior`), never by path.
//!
//! The run is the plan from `helqix_modelbuild::fmu::fmu_plan` handed to the FMI adapter (a sandboxed Python subprocess with a
//! cleared environment, a time limit and typed errors). The FMI tool checks each parameter's dimension against the unit the FMU
//! declares. The output series become frames in the frame store (determinism level Tolerance: native code can round differently on
//! another platform), the run's description is saved with the project and revision, and the same request gives the same run id.
use std::{path::PathBuf, time::Duration};

use axum::{Json, extract::State};
use helqix_adapters::{fmi::{FmiAdapter, TOLERANCE_FLOOR}, solver::{ANY_UNIT, IoField, SolverAdapter, SolverError}};
use helqix_core::determinism::DeterminismLevel;
use helqix_frames::{FrameStore, Variable, blake3_hex};
use helqix_freeze::{RecordedRun, Sample, chain_link};
use helqix_modelbuild::fmu::FmuPlan;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{error::ApiError, specs::ModelLink, state::AppState};

/// Where the FMU library is and how to run the FMI tool.
#[derive(Debug, Clone)]
pub struct FmuConfig {
    /// Directory of `<sha256>.fmu` files, installed by the operator.
    pub store: PathBuf,
    /// A Python with fmpy and pint (`HELQIX_FMI_PYTHON`).
    pub python: PathBuf,
    /// The directory that holds the `helqix_fmi` package (`18-libraries/python`).
    pub package_dir: PathBuf,
    pub timeout: Duration,
}

impl FmuConfig {
    /// `HELQIX_FMU_STORE`, `HELQIX_FMI_PYTHON` and `HELQIX_FMI_PACKAGE` together turn FMU runs on (`HELQIX_FMU_TIMEOUT_SECONDS`, default 120);
    /// none turns them off; a partial setting is an error so a typo cannot silently leave them off.
    pub fn from_vars(get: impl Fn(&str) -> Option<String>) -> Result<Option<Self>, String> {
        const NAMES: [&str; 3] = ["HELQIX_FMU_STORE", "HELQIX_FMI_PYTHON", "HELQIX_FMI_PACKAGE"];
        let values: Vec<Option<String>> = NAMES.iter().map(|n| get(n).filter(|v| !v.is_empty())).collect();
        if values.iter().all(Option::is_none) { return Ok(None); }
        let missing: Vec<&str> = NAMES.iter().zip(&values).filter(|(_, v)| v.is_none()).map(|(n, _)| *n).collect();
        if !missing.is_empty() { return Err(format!("FMU runs need all of {}; missing {}", NAMES.join(", "), missing.join(", "))); }
        let timeout = match get("HELQIX_FMU_TIMEOUT_SECONDS") {
            None => Duration::from_secs(120),
            Some(t) => { let secs: f64 = t.parse().map_err(|_| "HELQIX_FMU_TIMEOUT_SECONDS must be a number of seconds".to_owned())?; Duration::try_from_secs_f64(secs).ok().filter(|d| !d.is_zero()).ok_or_else(|| "HELQIX_FMU_TIMEOUT_SECONDS must be positive".to_owned())? }
        };
        let [store, python, package] = [0, 1, 2].map(|i| values[i].clone().unwrap_or_default());
        Ok(Some(Self { store: store.into(), python: python.into(), package_dir: package.into(), timeout }))
    }
}

/// What an FMU run was made from, saved beside the run so it can be shown and audited.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FmuRunSpec {
    pub fmu_sha256: String,
    pub part_id: String,
    /// The parameters as sent to the tool: SI value and dimension exponents.
    pub parameters_si: Value,
    pub outputs: Vec<String>,
    pub end_seconds: f64,
    pub step_seconds: f64,
    pub tool_version: String,
    /// SHA-256 of the exact input the tool received and of the exact output it returned.
    pub input_sha256: String,
    pub output_sha256: String,
}

fn is_sha256(s: &str) -> bool { s.len() == 64 && s.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) }

fn sha256_hex(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }

/// The path of a library file after checking that its content still has the hash in its name.
fn locate(cfg: &FmuConfig, sha: &str) -> Result<PathBuf, ApiError> {
    if !is_sha256(sha) { return Err(ApiError::Rejected(format!("'{sha}' is not an FMU hash (64 lowercase hex characters)"))); }
    let path = cfg.store.join(format!("{sha}.fmu"));
    let bytes = std::fs::read(&path).map_err(|_| ApiError::Rejected(format!("the FMU {sha} is not in the FMU library; ask the operator to install it")))?;
    if sha256_hex(&bytes) != sha {
        tracing::error!(%sha, "an FMU library file does not match its name");
        return Err(ApiError::Rejected(format!("the file in the FMU library named {sha} does not match its name (it was changed or replaced), so it is not run")));
    }
    let absolute = std::fs::canonicalize(&path).map_err(|_| ApiError::Internal)?;
    Ok(PathBuf::from(absolute.display().to_string().trim_start_matches(r"\\?\")))
}

/// `GET /v1/fmus`: the FMUs installed in the library (hash and size). A file whose content does not match its name is not listed.
pub async fn list(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let cfg = state.fmu.clone().ok_or(ApiError::Unavailable)?;
    let fmus = tokio::task::spawn_blocking(move || -> Vec<Value> {
        let Ok(rd) = std::fs::read_dir(&cfg.store) else { return Vec::new() };
        let mut out: Vec<(String, u64)> = Vec::new();
        for e in rd.flatten() {
            let path = e.path();
            let (Some(stem), Some("fmu")) = (path.file_stem().and_then(|s| s.to_str()), path.extension().and_then(|s| s.to_str())) else { continue };
            if !is_sha256(stem) { continue; }
            let Ok(bytes) = std::fs::read(&path) else { continue };
            if sha256_hex(&bytes) == stem { out.push((stem.to_owned(), bytes.len() as u64)); }
        }
        out.sort();
        out.into_iter().map(|(sha256, bytes)| json!({"sha256": sha256, "bytes": bytes})).collect()
    }).await.map_err(|_| ApiError::Internal)?;
    Ok(Json(json!({"fmus": fmus})))
}

/// The FMI tool hashes the file it is about to load and records that hash. If it is not the hash the plan named, the file in the library
/// changed between our check and the run, and the run is refused.
fn check_provenance(planned: &str, record: &helqix_adapters::solver::InvocationRecord) -> Result<(), ApiError> {
    match record.configuration["fmu_sha256"].as_str() {
        Some(h) if h == planned => Ok(()),
        _ => Err(ApiError::Rejected(format!("the FMU {planned} changed while it was being run, so the result is not used"))),
    }
}

/// A tool failure as the API's answer. The tool's own words are kept for a refusal the user can act on (bad input); everything else is
/// logged and answered without detail.
fn map_solver(e: SolverError) -> ApiError {
    match e {
        SolverError::Crash { exit_status: Some(2), stderr_summary } => ApiError::Rejected(stderr_summary),
        SolverError::Crash { exit_status: Some(3), stderr_summary } => ApiError::Rejected(format!("the FMU run failed: {stderr_summary}")),
        SolverError::Timeout { seconds } => ApiError::Rejected(format!("the FMU run took longer than its {seconds} s limit")),
        SolverError::InvalidInput(m) => ApiError::Rejected(m),
        SolverError::ToolMissing(p) => { tracing::error!(program = %p, "the FMI tool's Python is missing"); ApiError::Unavailable }
        other => { tracing::error!(error = %other, "the FMI tool returned something unusable"); ApiError::Rejected(format!("the FMU run returned unusable output: {other}")) }
    }
}

/// Runs `plan` for `end_s` seconds on a grid of `step_s`, stores the frames and the run's description, and returns the run id with
/// the stored index and the spec. The caller checked `end_s` and `step_s`.
pub(crate) async fn run_plan(state: &AppState, cfg: FmuConfig, plan: FmuPlan, end_s: f64, step_s: f64, link: ModelLink) -> Result<(String, helqix_frames::RunIndex, FmuRunSpec), ApiError> {
    let dir = state.frame_store.clone().ok_or(ApiError::Unavailable)?;
    let _slot = state.fmu_gate.clone().try_acquire_owned().map_err(|_| ApiError::Busy)?;
    let input = plan.adapter_input(0.0, end_s, step_s).map_err(ApiError::BadRequest)?;
    let description = format!("{:?}", (&link, &plan.fmu_sha256, input.to_string(), &plan.outputs));
    let id = format!("run-{}", &blake3_hex(description.as_bytes())[..16]);
    let genesis = blake3_hex(format!("helqix.fmu-run.v1|{description}").as_bytes());
    let rid = id.clone();
    let (entry, spec) = tokio::task::spawn_blocking(move || -> Result<_, ApiError> {
        let path = locate(&cfg, &plan.fmu_sha256)?;
        let fields: Vec<IoField> = plan.outputs.iter().map(|name| IoField { name: name.clone(), unit: ANY_UNIT.into() }).collect();
        let run = FmiAdapter::new(cfg.python.clone(), &cfg.package_dir, path, fields, cfg.timeout).run(&input).map_err(map_solver)?;
        check_provenance(&plan.fmu_sha256, &run.record)?;
        let (time, series) = run.outputs.split_first().ok_or(ApiError::Internal)?;
        let rows = time.values.len();
        if rows == 0 { return Err(ApiError::Rejected("the FMU run returned no samples".into())); }
        let mut samples = Vec::with_capacity(rows);
        let mut chain: Vec<String> = Vec::with_capacity(rows);
        for i in 0..rows {
            let state: Vec<f64> = series.iter().map(|s| s.values[i]).collect();
            let prev = chain.last().map_or(genesis.as_str(), String::as_str);
            let link = chain_link(prev, i, time.values[i], &state);
            samples.push(Sample { index: i, time_seconds: time.values[i], state });
            chain.push(link);
        }
        let determinism = DeterminismLevel::tolerance(TOLERANCE_FLOOR).map_err(|_| ApiError::Internal)?;
        let recorded = RecordedRun { genesis: genesis.clone(), samples, chain, determinism };
        let variables: Vec<Variable> = series.iter().map(|s| Variable { name: s.name.clone(), unit: s.unit.clone() }).collect();
        let entry = FrameStore::open(&dir).and_then(|s| s.put(&rid, &recorded, &variables, &genesis)).map_err(crate::frames::map)?;
        let spec = FmuRunSpec {
            fmu_sha256: plan.fmu_sha256.clone(), part_id: plan.part_id.clone(), parameters_si: input["parameters_si"].clone(), outputs: plan.outputs.clone(),
            end_seconds: end_s, step_seconds: step_s, tool_version: run.record.tool_version.clone(), input_sha256: run.record.input_sha256.clone(), output_sha256: run.record.output_sha256.clone(),
        };
        Ok((entry, spec))
    }).await.map_err(|_| ApiError::Internal)??;
    Ok((id, entry, spec))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(vars: &[(&str, &str)]) -> Result<Option<FmuConfig>, String> {
        let vars: std::collections::HashMap<String, String> = vars.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        FmuConfig::from_vars(|k| vars.get(k).cloned())
    }

    #[test]
    fn the_file_the_tool_hashed_must_be_the_file_the_plan_named() {
        let sha = "a3".repeat(32);
        let rec = |h: Value| helqix_adapters::solver::InvocationRecord { tool: "fmpy".into(), tool_version: "fmpy 0".into(), input_sha256: "i".into(), configuration: json!({"fmu_sha256": h}), wall_time_ms: 1, output_sha256: "o".into(), exit_status: Some(0), stderr_summary: String::new() };
        assert!(check_provenance(&sha, &rec(json!(sha.clone()))).is_ok());
        let swapped = check_provenance(&sha, &rec(json!("b4".repeat(32)))).unwrap_err();
        assert!(matches!(swapped, ApiError::Rejected(m) if m.contains("changed while")));
        assert!(check_provenance(&sha, &rec(Value::Null)).is_err(), "a record without the FMU hash proves nothing");
    }

    #[test]
    fn none_of_the_three_settings_turns_fmu_runs_off() {
        assert_eq!(cfg(&[]).unwrap().map(|c| c.store), None);
    }

    #[test]
    fn all_three_turn_them_on_with_a_default_time_limit() {
        let c = cfg(&[("HELQIX_FMU_STORE", "/fmus"), ("HELQIX_FMI_PYTHON", "/py"), ("HELQIX_FMI_PACKAGE", "/lib")]).unwrap().unwrap();
        assert_eq!((c.store, c.python, c.package_dir, c.timeout), ("/fmus".into(), "/py".into(), "/lib".into(), Duration::from_secs(120)));
    }

    #[test]
    fn a_partial_setting_is_an_error_naming_what_is_missing_so_a_typo_cannot_turn_it_off() {
        let e = cfg(&[("HELQIX_FMU_STORE", "/fmus"), ("HELQIX_FMI_PYTHON", "/py")]).unwrap_err();
        assert!(e.contains("HELQIX_FMI_PACKAGE"), "{e}");
        let e = cfg(&[("HELQIX_FMI_PACKAGE", "/lib")]).unwrap_err();
        assert!(e.contains("HELQIX_FMU_STORE") && e.contains("HELQIX_FMI_PYTHON"), "{e}");
    }

    #[test]
    fn the_time_limit_must_be_a_positive_number_of_seconds() {
        let base = [("HELQIX_FMU_STORE", "/f"), ("HELQIX_FMI_PYTHON", "/p"), ("HELQIX_FMI_PACKAGE", "/l")];
        let with = |t: &str| { let mut v = base.to_vec(); v.push(("HELQIX_FMU_TIMEOUT_SECONDS", t)); cfg(&v) };
        assert_eq!(with("30").unwrap().unwrap().timeout, Duration::from_secs(30));
        for bad in ["0", "-1", "soon", "1e400", ""] { assert!(with(bad).is_err(), "{bad:?}"); }
    }
}
