//! Running a committed model's battery cell through the PyBaMM adapter (E5.2).
//!
//! The cell's pack is the battery domain pack (`18-libraries/python/helqix_battery`: PyBaMM SPM with the Chen2020 parameters). The run is the plan
//! from `helqix_modelbuild::battery::battery_plan` handed to `PybammAdapter` (a sandboxed Python subprocess with a cleared environment, a time limit and typed
//! errors). Terminal voltage and discharge capacity become frames (determinism Tolerance), the run's description is saved with the project and
//! revision, and the same request gives the same run id.
//!
//! **A run never travels without its pack's label.** The label (`NotVerified`), the published held-out error and the limit it failed come from the pack's
//! library entry (`<library>/<pack>.library-entry.json`, written by `helqix-validate`). If that entry is missing or unreadable nothing is run: an unlabelled
//! run of an unverified model would be a claim the evidence does not support.
use std::{path::PathBuf, time::Duration};

use axum::{Json, extract::State};
use helqix_adapters::{pybamm::PybammAdapter, solver::SolverAdapter};
use helqix_frames::blake3_hex;
use helqix_modelbuild::battery::{BatteryPlan, BATTERY_PACKS};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{error::ApiError, fmu_run::map_solver, recorded::{Series, store}, specs::ModelLink, state::AppState};

/// Where the battery pack's Python and library are.
#[derive(Debug, Clone)]
pub struct BatteryConfig {
    /// A Python with pybamm (`HELQIX_BATTERY_PYTHON`).
    pub python: PathBuf,
    /// The directory that holds the `helqix_battery` package (`18-libraries/python`).
    pub package_dir: PathBuf,
    /// The directory of `<pack>.library-entry.json` files (`18-libraries/battery`).
    pub library_dir: PathBuf,
    pub timeout: Duration,
}

impl BatteryConfig {
    /// `HELQIX_BATTERY_PYTHON`, `HELQIX_BATTERY_PACKAGE` and `HELQIX_BATTERY_LIBRARY` together turn battery runs on (`HELQIX_BATTERY_TIMEOUT_SECONDS`, default 300);
    /// none turns them off; a partial setting is an error so a typo cannot silently leave them off.
    pub fn from_vars(get: impl Fn(&str) -> Option<String>) -> Result<Option<Self>, String> {
        const NAMES: [&str; 3] = ["HELQIX_BATTERY_PYTHON", "HELQIX_BATTERY_PACKAGE", "HELQIX_BATTERY_LIBRARY"];
        let values: Vec<Option<String>> = NAMES.iter().map(|n| get(n).filter(|v| !v.is_empty())).collect();
        if values.iter().all(Option::is_none) { return Ok(None); }
        let missing: Vec<&str> = NAMES.iter().zip(&values).filter(|(_, v)| v.is_none()).map(|(n, _)| *n).collect();
        if !missing.is_empty() { return Err(format!("battery runs need all of {}; missing {}", NAMES.join(", "), missing.join(", "))); }
        let timeout = match get("HELQIX_BATTERY_TIMEOUT_SECONDS") {
            None => Duration::from_secs(300),
            Some(t) => { let secs: f64 = t.parse().map_err(|_| "HELQIX_BATTERY_TIMEOUT_SECONDS must be a number of seconds".to_owned())?; Duration::try_from_secs_f64(secs).ok().filter(|d| !d.is_zero()).ok_or_else(|| "HELQIX_BATTERY_TIMEOUT_SECONDS must be positive".to_owned())? }
        };
        let [python, package, library] = [0, 1, 2].map(|i| values[i].clone().unwrap_or_default());
        Ok(Some(Self { python: python.into(), package_dir: package.into(), library_dir: library.into(), timeout }))
    }
}

/// What the pack's library entry says about the pack: its label and how far to trust it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackInfo {
    pub id: String,
    /// `NotVerified` or `Verified`, as the trust layer decided.
    pub label: String,
    pub context_of_use: String,
    /// Held-out RMSE of the terminal voltage, in millivolts, one per held-out case.
    pub held_out_rmse_mv: Vec<f64>,
    /// The limit the requirement set, in millivolts.
    pub threshold_mv: f64,
    /// The contact resistance the pack was calibrated to. Offered to the user, never applied for them.
    pub calibrated_contact_resistance_ohm: Option<f64>,
    /// The identity hash of the model the library entry was written for (it includes the solver version, so a run on another version differs).
    pub validated_model_identity_sha256: String,
    pub report_date: String,
}

/// A C99 hexadecimal float such as `0x1.02f0fed59ae0bp-6` (how the model identity writes its numbers, so they round-trip exactly).
pub(crate) fn parse_hex_float(s: &str) -> Option<f64> {
    let (neg, rest) = match s.strip_prefix('-') { Some(r) => (true, r), None => (false, s) };
    let rest = rest.strip_prefix("0x")?;
    let (mant, exp) = rest.split_once('p')?;
    let exp: i32 = exp.parse().ok()?;
    let (int, frac) = mant.split_once('.').unwrap_or((mant, ""));
    let mut value = 0.0_f64;
    for c in int.chars() { value = value * 16.0 + f64::from(c.to_digit(16)?); }
    let mut scale = 1.0 / 16.0;
    for c in frac.chars() { value += f64::from(c.to_digit(16)?) * scale; scale /= 16.0; }
    let out = value * 2.0_f64.powi(exp);
    out.is_finite().then_some(if neg { -out } else { out })
}

fn pack_info(cfg: &BatteryConfig, id: &str) -> Result<PackInfo, ApiError> {
    if !BATTERY_PACKS.contains(&id) { return Err(ApiError::Rejected(format!("the battery pack '{id}' is not installed"))); }
    let path = cfg.library_dir.join(format!("{id}.library-entry.json"));
    let bytes = std::fs::read(&path).map_err(|e| { tracing::error!(error = %e, pack = id, "the pack's library entry cannot be read"); ApiError::Unavailable })?;
    let d: Value = serde_json::from_slice(&bytes).map_err(|e| { tracing::error!(error = %e, pack = id, "the pack's library entry is not JSON"); ApiError::Unavailable })?;
    let unusable = || { tracing::error!(pack = id, "the pack's library entry lacks its label or its published error"); ApiError::Unavailable };
    let label = d["label"].as_str().ok_or_else(unusable)?.to_owned();
    let held_out: Vec<f64> = d["held_out_errors"].as_array().ok_or_else(unusable)?.iter().map(|e| e["rmse"].as_f64().map(|v| v * 1000.0)).collect::<Option<_>>().ok_or_else(unusable)?;
    let threshold = &d["requirement"]["threshold"];
    let threshold_mv = match (threshold["value"].as_f64(), threshold["unit"].as_str()) { (Some(v), Some("mV")) => v, _ => return Err(unusable()) };
    Ok(PackInfo {
        id: id.to_owned(), label,
        context_of_use: d["context_of_use"]["statement"].as_str().ok_or_else(unusable)?.to_owned(),
        held_out_rmse_mv: held_out, threshold_mv,
        calibrated_contact_resistance_ohm: d["model"]["contact_resistance_ohm"].as_str().and_then(parse_hex_float),
        validated_model_identity_sha256: d["model"]["sha256"].as_str().ok_or_else(unusable)?.to_owned(),
        report_date: d["report_date"].as_str().unwrap_or_default().to_owned(),
    })
}

/// `GET /v1/packs`: the installed packs with their label and published error.
pub async fn packs(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let cfg = state.battery.clone().ok_or(ApiError::Unavailable)?;
    let packs = tokio::task::spawn_blocking(move || BATTERY_PACKS.iter().map(|id| pack_info(&cfg, id)).collect::<Result<Vec<_>, _>>()).await.map_err(|_| ApiError::Internal)??;
    Ok(Json(json!({"packs": packs})))
}

/// What a battery run was made from, saved beside the run.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BatteryRunSpec {
    pub pack: String,
    pub initial_ocv_v: f64,
    pub contact_resistance_ohm: f64,
    pub current_a: f64,
    pub end_seconds: f64,
    pub step_seconds: f64,
    pub tool_version: String,
    /// The identity of the model the tool ran (it includes the solver version and the contact resistance).
    pub model_identity_sha256: String,
    pub input_sha256: String,
    pub output_sha256: String,
}

/// Runs `plan` for `end_s` seconds on a grid of `step_s`, stores the frames and the run's description. The caller checked `end_s` and `step_s` are positive.
pub(crate) async fn run_plan(state: &AppState, cfg: BatteryConfig, plan: BatteryPlan, end_s: f64, step_s: f64, link: ModelLink) -> Result<(String, helqix_frames::RunIndex, BatteryRunSpec, PackInfo), ApiError> {
    let dir = state.frame_store.clone().ok_or(ApiError::Unavailable)?;
    let _slot = state.fmu_gate.clone().try_acquire_owned().map_err(|_| ApiError::Busy)?;
    let input = plan.adapter_input(end_s, step_s).map_err(ApiError::BadRequest)?;
    let description = format!("{:?}", (&link, &plan.pack, input.to_string()));
    let id = format!("run-{}", &blake3_hex(description.as_bytes())[..16]);
    let genesis = blake3_hex(format!("helqix.battery-run.v1|{description}").as_bytes());
    let rid = id.clone();
    let (entry, spec, info) = tokio::task::spawn_blocking(move || -> Result<_, ApiError> {
        let info = pack_info(&cfg, &plan.pack)?;
        let adapter = PybammAdapter::new(cfg.python.clone(), cfg.package_dir.clone(), cfg.timeout);
        let determinism = adapter.contract().determinism.clone();
        let run = adapter.run(&input).map_err(map_solver)?;
        let time: Vec<f64> = input["time_s"].as_array().map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
        let series: Vec<Series> = run.outputs.iter().map(|o| Series { name: o.name.clone(), unit: o.unit.clone(), values: o.values.clone() }).collect();
        let entry = store(&dir, &rid, &genesis, &time, &series, determinism)?;
        let identity = serde_json::from_slice::<Value>(&run.raw_output).ok().and_then(|d| d["model_identity_sha256"].as_str().map(str::to_owned)).ok_or_else(|| ApiError::Rejected("the pack did not report the identity of the model it ran".into()))?;
        let spec = BatteryRunSpec {
            pack: plan.pack.clone(), initial_ocv_v: plan.initial_ocv_v, contact_resistance_ohm: plan.contact_resistance_ohm, current_a: plan.current_a, end_seconds: end_s, step_seconds: step_s,
            tool_version: run.record.tool_version.clone(), model_identity_sha256: identity, input_sha256: run.record.input_sha256.clone(), output_sha256: run.record.output_sha256.clone(),
        };
        Ok((entry, spec, info))
    }).await.map_err(|_| ApiError::Internal)??;
    Ok((id, entry, spec, info))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(vars: &[(&str, &str)]) -> Result<Option<BatteryConfig>, String> {
        let vars: std::collections::HashMap<String, String> = vars.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        BatteryConfig::from_vars(|k| vars.get(k).cloned())
    }

    #[test]
    fn hex_floats_read_exactly() {
        assert_eq!(parse_hex_float("0x1.0000000000000p+1"), Some(2.0));
        assert_eq!(parse_hex_float("0x1.8p+0"), Some(1.5));
        assert_eq!(parse_hex_float("-0x1p-2"), Some(-0.25));
        assert!((parse_hex_float("0x1.02f0fed59ae0bp-6").unwrap() - 0.015805).abs() < 1e-6);
        for bad in ["", "1.5", "0x", "0x1.zp+0", "0x1.0", "0x1.0p+x", "0x1p+5000"] { assert_eq!(parse_hex_float(bad), None, "{bad:?}"); }
    }

    #[test]
    fn none_of_the_three_settings_turns_battery_runs_off_and_a_partial_one_is_an_error() {
        assert!(cfg(&[]).unwrap().is_none());
        let c = cfg(&[("HELQIX_BATTERY_PYTHON", "/py"), ("HELQIX_BATTERY_PACKAGE", "/pkg"), ("HELQIX_BATTERY_LIBRARY", "/lib")]).unwrap().unwrap();
        assert_eq!((c.python, c.package_dir, c.library_dir, c.timeout), ("/py".into(), "/pkg".into(), "/lib".into(), Duration::from_secs(300)));
        let e = cfg(&[("HELQIX_BATTERY_PYTHON", "/py")]).unwrap_err();
        assert!(e.contains("HELQIX_BATTERY_PACKAGE") && e.contains("HELQIX_BATTERY_LIBRARY"), "{e}");
    }

    #[test]
    fn the_time_limit_must_be_a_positive_number_of_seconds() {
        let base = [("HELQIX_BATTERY_PYTHON", "/p"), ("HELQIX_BATTERY_PACKAGE", "/k"), ("HELQIX_BATTERY_LIBRARY", "/l")];
        let with = |t: &str| { let mut v = base.to_vec(); v.push(("HELQIX_BATTERY_TIMEOUT_SECONDS", t)); cfg(&v) };
        assert_eq!(with("45").unwrap().unwrap().timeout, Duration::from_secs(45));
        for bad in ["0", "-1", "soon", ""] { assert!(with(bad).is_err(), "{bad:?}"); }
    }
}
