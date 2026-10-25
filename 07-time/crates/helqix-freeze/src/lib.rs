//! Layer 6: freeze and bit-exact replay of core-runtime runs.
//!
//! A [`Freeze`] captures everything needed to continue a run exactly: the model
//! parameters, scenario time grid, solver identity and step configuration, seeds,
//! logical time (grid index and time), the state at that index, the hash-chain link
//! before it, and what triggered the freeze. It is immutable (private fields) and
//! content-addressed: [`Freeze::sha256`] covers every field, and loading a file
//! whose content no longer matches its recorded hash fails.
//!
//! Every recorded run carries a hash chain: link *i* hashes link *i-1* with the
//! sample's grid index, time bits and state bits. Two runs reproduce bit-exactly
//! exactly when their chains match; [`compare`] reports the first divergence.
//!
//! Scope of the bitwise claim (see docs/DETERMINISM.md): same build, same platform,
//! same inputs. Floats are stored and hashed as their IEEE-754 bit patterns.
#![forbid(unsafe_code)]

use std::path::Path;

use helixsim::{
    environment::{Environment, GravityField, MediumProperties, Scalar, Vector3},
    kernel::resume, newton_unit, DragConfiguration, IntegrationMethod, PointMassModel, SimulationOptions,
};
use helqix_core::determinism::DeterminismLevel;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const SCHEMA: &str = "helqix.freeze.v1";
const MAX_STEPS: u64 = 250_000;

/// Parameters of helixsim's point-mass model (generic rigid-body translation).
#[derive(Debug, Clone, PartialEq)]
pub struct PointMassSpec {
    pub mass_kg: f64,
    pub gravity_m_s2: [f64; 3],
    pub air_density_kg_m3: f64,
    pub wind_m_s: [f64; 3],
    pub applied_force_n: [f64; 3],
    pub drag_coefficient: f64,
    pub reference_area_m2: f64,
}

/// A complete, reproducible run definition: model, scenario grid, initial state, seeds.
#[derive(Debug, Clone, PartialEq)]
pub struct RunSpec {
    pub model: PointMassSpec,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub step_seconds: f64,
    /// Position (m) then velocity (m/s), three components each.
    pub initial_state: Vec<f64>,
    /// Random seeds used by the run; empty for the deterministic core runtime.
    pub seeds: Vec<u64>,
}

/// One recorded state at an absolute grid index.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub index: usize,
    pub time_seconds: f64,
    pub state: Vec<f64>,
}

/// A run's samples, their hash chain (one link per sample) and declared determinism.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordedRun {
    pub genesis: String,
    pub samples: Vec<Sample>,
    pub chain: Vec<String>,
    pub determinism: DeterminismLevel,
}

/// Direction of a state limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparator {
    /// The variable must stay at or above the threshold.
    AtLeast,
    /// The variable must stay at or below the threshold.
    AtMost,
}

/// A requirement on one state variable; crossing it is a failure event.
#[derive(Debug, Clone, PartialEq)]
pub struct StateLimit {
    pub name: String,
    pub variable: usize,
    pub comparator: Comparator,
    pub threshold: f64,
}

/// The first sample at which a limit is violated.
#[derive(Debug, Clone, PartialEq)]
pub struct FailureEvent {
    pub limit: StateLimit,
    pub index: usize,
    pub time_seconds: f64,
    pub value: f64,
}

/// Why a freeze was taken.
#[derive(Debug, Clone, PartialEq)]
pub enum FreezeTrigger {
    Failure(FailureEvent),
    RequirementViolation(String),
    Manual,
    TimeMarker(f64),
}

/// Immutable, content-addressed capture of a run at one grid index.
#[derive(Debug, Clone, PartialEq)]
pub struct Freeze {
    spec: RunSpec,
    index: usize,
    time_seconds: f64,
    state: Vec<f64>,
    prior_link: String,
    trigger: FreezeTrigger,
}

/// Where two runs first differ.
#[derive(Debug, Clone, PartialEq)]
pub struct Divergence {
    pub index: usize,
    pub time_seconds: f64,
    /// State variable that differs, or `None` if the time or sample count differs.
    pub variable: Option<usize>,
}

/// Result of comparing two runs sample by sample from the later run's first index.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub identical: bool,
    pub compared: usize,
    pub first_divergence: Option<Divergence>,
}

/// Freeze, replay or file error.
#[derive(Debug, Clone, PartialEq)]
pub enum FreezeError {
    Simulation(String),
    IndexOutOfRange { index: usize, last: usize },
    WrongRun,
    HashMismatch { recorded: String, computed: String },
    Malformed(String),
    Io(String),
}

impl std::fmt::Display for FreezeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Simulation(m) => write!(f, "simulation failed: {m}"),
            Self::IndexOutOfRange { index, last } => write!(f, "freeze index {index} is beyond the last sample {last}"),
            Self::WrongRun => f.write_str("the recorded run was not produced from this run specification"),
            Self::HashMismatch { recorded, computed } => write!(f, "freeze content hash {computed} does not match recorded {recorded}"),
            Self::Malformed(m) => write!(f, "malformed freeze file: {m}"),
            Self::Io(m) => write!(f, "freeze file I/O failed: {m}"),
        }
    }
}

impl std::error::Error for FreezeError {}

fn hex(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() }
fn bits(v: f64) -> String { format!("{:016x}", v.to_bits()) }
fn sha(data: &[u8]) -> String { hex(&Sha256::digest(data)) }

fn options(spec: &RunSpec) -> SimulationOptions {
    SimulationOptions { step_seconds: spec.step_seconds, max_steps: MAX_STEPS, method: IntegrationMethod::RungeKutta4 }
}

fn spec_json(spec: &RunSpec) -> Value {
    let m = &spec.model;
    let v3 = |v: &[f64; 3]| Value::from(v.iter().map(|x| bits(*x)).collect::<Vec<_>>());
    json!({
        "model": {
            "kind": "helixsim.point_mass", "mass_kg": bits(m.mass_kg), "gravity_m_s2": v3(&m.gravity_m_s2),
            "air_density_kg_m3": bits(m.air_density_kg_m3), "wind_m_s": v3(&m.wind_m_s), "applied_force_n": v3(&m.applied_force_n),
            "drag_coefficient": bits(m.drag_coefficient), "reference_area_m2": bits(m.reference_area_m2),
        },
        "solver": {"name": "helixsim", "version": helixsim::VERSION, "method": "RungeKutta4", "max_steps": MAX_STEPS},
        "grid": {"start_seconds": bits(spec.start_seconds), "end_seconds": bits(spec.end_seconds), "step_seconds": bits(spec.step_seconds)},
        "initial_state": spec.initial_state.iter().map(|x| bits(*x)).collect::<Vec<_>>(),
        "seeds": spec.seeds,
    })
}

fn genesis(spec: &RunSpec) -> String {
    sha(format!("helqix.chain.v1\n{}", sha(spec_json(spec).to_string().as_bytes())).as_bytes())
}

fn link(prev: &str, index: usize, time: f64, state: &[f64]) -> String {
    let mut h = Sha256::new();
    h.update(prev.as_bytes());
    h.update((index as u64).to_le_bytes());
    h.update(time.to_bits().to_le_bytes());
    for v in state { h.update(v.to_bits().to_le_bytes()); }
    hex(&h.finalize())
}

fn build_model(m: &PointMassSpec) -> Result<PointMassModel, FreezeError> {
    let err = |e: &dyn std::fmt::Display| FreezeError::Simulation(e.to_string());
    let gravity = GravityField::new(Vector3::new(m.gravity_m_s2, GravityField::unit()).map_err(|e| err(&e))?).map_err(|e| err(&e))?;
    let medium = MediumProperties::new(Scalar::new(m.air_density_kg_m3, MediumProperties::density_unit()).map_err(|e| err(&e))?, None, None).map_err(|e| err(&e))?;
    let environment = Environment::new(gravity, medium, Vector3::new(m.wind_m_s, Environment::velocity_unit()).map_err(|e| err(&e))?).map_err(|e| err(&e))?;
    let force = Vector3::new(m.applied_force_n, newton_unit()).map_err(|e| err(&e))?;
    let drag = DragConfiguration::new(m.drag_coefficient, m.reference_area_m2).map_err(|e| err(&e))?;
    PointMassModel::new("helixsim.point_mass", "freeze-v1", m.mass_kg, environment, force, Some(drag)).map_err(|e| err(&e))
}

fn record(spec: &RunSpec, start_index: usize, state: &[f64], prior: &str) -> Result<RecordedRun, FreezeError> {
    let model = build_model(&spec.model)?;
    let result = resume(&model, spec.start_seconds, spec.end_seconds, start_index, state, options(spec))
        .map_err(|e| FreezeError::Simulation(e.to_string()))?;
    let mut chain = Vec::with_capacity(result.samples.len());
    let mut prev = prior.to_owned();
    let samples: Vec<Sample> = result.samples.into_iter().enumerate().map(|(offset, s)| {
        let index = start_index + offset;
        prev = link(&prev, index, s.time_seconds, &s.state);
        chain.push(prev.clone());
        Sample { index, time_seconds: s.time_seconds, state: s.state }
    }).collect();
    Ok(RecordedRun { genesis: genesis(spec), samples, chain, determinism: result.determinism })
}

/// Runs a specification from its start on the deterministic core runtime.
pub fn run(spec: &RunSpec) -> Result<RecordedRun, FreezeError> {
    record(spec, 0, &spec.initial_state, &genesis(spec))
}

/// Captures `run` at grid position `index` (a position in `run.samples`).
pub fn freeze(spec: &RunSpec, run: &RecordedRun, index: usize, trigger: FreezeTrigger) -> Result<Freeze, FreezeError> {
    if run.genesis != genesis(spec) { return Err(FreezeError::WrongRun); }
    let sample = run.samples.get(index).ok_or(FreezeError::IndexOutOfRange { index, last: run.samples.len().saturating_sub(1) })?;
    let prior_link = if index == 0 { run.genesis.clone() } else { run.chain[index - 1].clone() };
    Ok(Freeze { spec: spec.clone(), index: sample.index, time_seconds: sample.time_seconds, state: sample.state.clone(), prior_link, trigger })
}

/// Runs `spec` forward from grid `index` with `state`, on the same grid as a fresh run of `spec`.
/// Used by branches: a changed specification or state continues from a frozen point.
pub fn continue_run(spec: &RunSpec, index: usize, state: &[f64], prior_link: &str) -> Result<RecordedRun, FreezeError> {
    record(spec, index, state, prior_link)
}

/// Re-executes from a freeze using only its content; the chain continues the original's.
pub fn replay(freeze: &Freeze) -> Result<RecordedRun, FreezeError> {
    record(&freeze.spec, freeze.index, &freeze.state, &freeze.prior_link)
}

/// Compares `repeat` with the same grid indices of `original`; reports the first divergence.
pub fn compare(original: &RecordedRun, repeat: &RecordedRun) -> Comparison {
    let offset = repeat.samples.first().map_or(0, |s| s.index);
    let base = original.samples.iter().position(|s| s.index == offset);
    let Some(base) = base else {
        return Comparison { identical: false, compared: 0, first_divergence: Some(Divergence { index: offset, time_seconds: f64::NAN, variable: None }) };
    };
    let tail = &original.samples[base..];
    let mut compared = 0;
    // Locate the first divergence by value (time and state bits), not by chain link:
    // a different specification changes every link even where the numbers still agree.
    for (a, b) in tail.iter().zip(&repeat.samples) {
        compared += 1;
        let same_time = a.time_seconds.to_bits() == b.time_seconds.to_bits() && a.state.len() == b.state.len();
        let variable = if same_time { a.state.iter().zip(&b.state).position(|(x, y)| x.to_bits() != y.to_bits()) } else { None };
        if !same_time || variable.is_some() {
            return Comparison { identical: false, compared, first_divergence: Some(Divergence { index: a.index, time_seconds: a.time_seconds, variable }) };
        }
    }
    if tail.len() != repeat.samples.len() {
        let index = offset + tail.len().min(repeat.samples.len());
        return Comparison { identical: false, compared, first_divergence: Some(Divergence { index, time_seconds: f64::NAN, variable: None }) };
    }
    // Every number matches; the chains also match unless the runs came from different specifications.
    if original.chain[base..] != repeat.chain[..] {
        return Comparison { identical: false, compared, first_divergence: Some(Divergence { index: offset, time_seconds: repeat.samples.first().map_or(f64::NAN, |s| s.time_seconds), variable: None }) };
    }
    Comparison { identical: true, compared, first_divergence: None }
}

/// First sample (in recorded order) that violates the limit; a non-finite value counts as a violation.
pub fn detect_failure(run: &RecordedRun, limit: &StateLimit) -> Option<FailureEvent> {
    run.samples.iter().find_map(|s| {
        let value = *s.state.get(limit.variable)?;
        let violated = !value.is_finite() || match limit.comparator { Comparator::AtLeast => value < limit.threshold, Comparator::AtMost => value > limit.threshold };
        violated.then(|| FailureEvent { limit: limit.clone(), index: s.index, time_seconds: s.time_seconds, value })
    })
}

fn trigger_json(trigger: &FreezeTrigger) -> Value {
    match trigger {
        FreezeTrigger::Manual => json!({"kind": "manual"}),
        FreezeTrigger::TimeMarker(t) => json!({"kind": "time_marker", "time_seconds": bits(*t)}),
        FreezeTrigger::RequirementViolation(r) => json!({"kind": "requirement_violation", "requirement": r}),
        FreezeTrigger::Failure(f) => json!({"kind": "failure", "limit": f.limit.name, "variable": f.limit.variable,
            "comparator": match f.limit.comparator { Comparator::AtLeast => "at_least", Comparator::AtMost => "at_most" },
            "threshold": bits(f.limit.threshold), "index": f.index, "time_seconds": bits(f.time_seconds), "value": bits(f.value)}),
    }
}

impl Freeze {
    fn content(&self) -> Value {
        json!({
            "schema": SCHEMA, "spec": spec_json(&self.spec), "model_sha256": sha(spec_json(&self.spec)["model"].to_string().as_bytes()),
            "index": self.index, "time_seconds": bits(self.time_seconds),
            "state": self.state.iter().map(|x| bits(*x)).collect::<Vec<_>>(),
            "prior_link": self.prior_link, "trigger": trigger_json(&self.trigger),
            "determinism": DeterminismLevel::Bitwise.label(),
        })
    }

    /// Content address: SHA-256 of the canonical encoding of every field.
    pub fn sha256(&self) -> String { sha(self.content().to_string().as_bytes()) }

    /// Grid index and logical time of the frozen state.
    pub fn logical_time(&self) -> (usize, f64) { (self.index, self.time_seconds) }

    /// What triggered the freeze.
    pub fn trigger(&self) -> &FreezeTrigger { &self.trigger }

    /// The run specification captured by the freeze.
    pub fn spec(&self) -> &RunSpec { &self.spec }

    /// The frozen state at the freeze index.
    pub fn state(&self) -> &[f64] { &self.state }

    /// Hash-chain link of the sample before the freeze index.
    pub fn prior_link(&self) -> &str { &self.prior_link }

    #[doc(hidden)]
    pub fn tamper_state_for_test(&mut self, variable: usize, f: impl Fn(f64) -> f64) { self.state[variable] = f(self.state[variable]); }
}

/// Writes the freeze with its content hash.
pub fn save_freeze(freeze: &Freeze, path: &Path) -> Result<(), FreezeError> {
    let mut doc = freeze.content();
    doc["sha256"] = Value::from(freeze.sha256());
    std::fs::write(path, serde_json::to_string_pretty(&doc).map_err(|e| FreezeError::Io(e.to_string()))? + "\n").map_err(|e| FreezeError::Io(e.to_string()))
}

fn f(v: &Value) -> Result<f64, FreezeError> {
    let s = v.as_str().ok_or_else(|| FreezeError::Malformed("expected a hex float".into()))?;
    u64::from_str_radix(s, 16).map(f64::from_bits).map_err(|_| FreezeError::Malformed(format!("bad hex float {s:?}")))
}
fn f3(v: &Value) -> Result<[f64; 3], FreezeError> {
    let a = v.as_array().filter(|a| a.len() == 3).ok_or_else(|| FreezeError::Malformed("expected 3 values".into()))?;
    Ok([f(&a[0])?, f(&a[1])?, f(&a[2])?])
}
fn fv(v: &Value) -> Result<Vec<f64>, FreezeError> {
    v.as_array().ok_or_else(|| FreezeError::Malformed("expected an array".into()))?.iter().map(f).collect()
}

/// Reads a freeze and refuses it unless its content matches its recorded hash.
pub fn load_freeze(path: &Path) -> Result<Freeze, FreezeError> {
    let text = std::fs::read_to_string(path).map_err(|e| FreezeError::Io(e.to_string()))?;
    let mut doc: Value = serde_json::from_str(&text).map_err(|e| FreezeError::Malformed(e.to_string()))?;
    let obj: &mut Map<String, Value> = doc.as_object_mut().ok_or_else(|| FreezeError::Malformed("not an object".into()))?;
    let recorded = obj.remove("sha256").and_then(|v| v.as_str().map(str::to_owned)).ok_or_else(|| FreezeError::Malformed("missing sha256".into()))?;
    if doc["schema"] != SCHEMA { return Err(FreezeError::Malformed("unsupported schema".into())); }
    let s = &doc["spec"];
    let m = &s["model"];
    let spec = RunSpec {
        model: PointMassSpec {
            mass_kg: f(&m["mass_kg"])?, gravity_m_s2: f3(&m["gravity_m_s2"])?, air_density_kg_m3: f(&m["air_density_kg_m3"])?,
            wind_m_s: f3(&m["wind_m_s"])?, applied_force_n: f3(&m["applied_force_n"])?, drag_coefficient: f(&m["drag_coefficient"])?,
            reference_area_m2: f(&m["reference_area_m2"])?,
        },
        start_seconds: f(&s["grid"]["start_seconds"])?, end_seconds: f(&s["grid"]["end_seconds"])?, step_seconds: f(&s["grid"]["step_seconds"])?,
        initial_state: fv(&s["initial_state"])?,
        seeds: s["seeds"].as_array().ok_or_else(|| FreezeError::Malformed("seeds".into()))?.iter().map(|v| v.as_u64().ok_or_else(|| FreezeError::Malformed("seed".into()))).collect::<Result<_, _>>()?,
    };
    let t = &doc["trigger"];
    let trigger = match t["kind"].as_str() {
        Some("manual") => FreezeTrigger::Manual,
        Some("time_marker") => FreezeTrigger::TimeMarker(f(&t["time_seconds"])?),
        Some("requirement_violation") => FreezeTrigger::RequirementViolation(t["requirement"].as_str().unwrap_or_default().to_owned()),
        Some("failure") => FreezeTrigger::Failure(FailureEvent {
            limit: StateLimit { name: t["limit"].as_str().unwrap_or_default().to_owned(), variable: t["variable"].as_u64().unwrap_or(u64::MAX) as usize,
                comparator: if t["comparator"] == "at_most" { Comparator::AtMost } else { Comparator::AtLeast }, threshold: f(&t["threshold"])? },
            index: t["index"].as_u64().unwrap_or(u64::MAX) as usize, time_seconds: f(&t["time_seconds"])?, value: f(&t["value"])?,
        }),
        _ => return Err(FreezeError::Malformed("unknown trigger".into())),
    };
    let freeze = Freeze {
        spec, index: doc["index"].as_u64().ok_or_else(|| FreezeError::Malformed("index".into()))? as usize,
        time_seconds: f(&doc["time_seconds"])?, state: fv(&doc["state"])?,
        prior_link: doc["prior_link"].as_str().ok_or_else(|| FreezeError::Malformed("prior_link".into()))?.to_owned(), trigger,
    };
    let computed = freeze.sha256();
    if computed != recorded || freeze.content() != doc { return Err(FreezeError::HashMismatch { recorded, computed }); }
    Ok(freeze)
}
