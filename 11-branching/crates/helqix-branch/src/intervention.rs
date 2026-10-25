//! Counterfactual branches from a freeze: one declared intervention, run on the deterministic
//! core runtime, compared with the parent, and used to test candidate causes.
//!
//! * A branch forks from a [`Freeze`] with exactly one structured [`Intervention`] (a parameter
//!   or a state value). It records its lineage (parent freeze hash, fork index and time).
//! * Branches never touch the parent: the parent [`RecordedRun`] is only read.
//! * [`intervention_test`] verdicts: `InterventionSupported` when the failure is removed or
//!   delayed by more than the stated tolerance; `NotSupported` when the failure time is unchanged
//!   within the tolerance; `Inconclusive` when there is no parent failure or the failure moved
//!   earlier. A supported result is graded **model-causal**: it is shown inside the model only.
//!   Validated-causal needs Phase 1 validation plus real data and is never granted here.
//! * [`compare_runs`] aligns samples only at identical times (bit-equal); nothing is
//!   interpolated, unmatched samples and missing variables are counted, not hidden.
//! * [`Budget`] caps branch count and total integration steps per request.
//! * There is no merge: adopting a change is a human-approved new model revision (design only).
use std::collections::BTreeMap;

use helqix_core::id::Identifier;
use helqix_freeze::{continue_run, detect_failure, Comparator, Freeze, RecordedRun, RunSpec, StateLimit};

/// Model parameter an intervention may change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parameter { MassKg, DragCoefficient, ReferenceAreaM2, AirDensityKgM3 }

/// One structured change (never free text).
#[derive(Debug, Clone, PartialEq)]
pub enum Intervention {
    /// Set a model parameter for the rest of the run.
    Parameter { parameter: Parameter, value: f64 },
    /// Set one state variable at the fork point.
    State { variable: usize, value: f64 },
}

/// Per-request limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget { pub max_branches: usize, pub max_total_steps: usize }

/// A run branched from a freeze.
#[derive(Debug, Clone, PartialEq)]
pub struct BranchRun {
    pub id: Identifier,
    pub parent_freeze_sha256: String,
    pub fork_index: usize,
    pub fork_time_seconds: f64,
    pub intervention: Intervention,
    pub spec: RunSpec,
    pub run: RecordedRun,
}

/// A set of branches sharing one budget.
#[derive(Debug, Clone, PartialEq)]
pub struct Experiments { budget: Budget, used_steps: usize, branches: Vec<BranchRun> }

/// Rejected or failed branch.
#[derive(Debug, Clone, PartialEq)]
pub enum ExperimentError { BudgetExceeded { reason: String }, InvalidIntervention(String), DuplicateBranch(Identifier), Run(String) }

impl std::fmt::Display for ExperimentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BudgetExceeded { reason } => write!(f, "branch budget exceeded: {reason}"),
            Self::InvalidIntervention(m) => write!(f, "invalid intervention: {m}"),
            Self::DuplicateBranch(id) => write!(f, "branch {} already exists", id.as_str()),
            Self::Run(m) => write!(f, "branch run failed: {m}"),
        }
    }
}
impl std::error::Error for ExperimentError {}

/// Outcome of an intervention test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict { InterventionSupported, NotSupported, Inconclusive }

/// Intervention test result with its grade.
#[derive(Debug, Clone, PartialEq)]
pub struct InterventionResult { pub verdict: Verdict, pub grade: &'static str, pub parent_failure_seconds: Option<f64>, pub branch_failure_seconds: Option<f64>, pub branch: Identifier }

/// Requirement outcome of one run.
#[derive(Debug, Clone, PartialEq)]
pub enum RequirementStatus {
    Violated { first_seconds: f64 },
    /// Smallest distance to the threshold over the run (positive = inside the limit).
    Satisfied { margin: f64 },
}

/// Parent-versus-branch comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct CompareReport {
    pub alignment_rule: &'static str,
    pub aligned_samples: usize,
    pub unmatched_parent: usize,
    pub unmatched_branch: usize,
    pub missing_variables: Vec<String>,
    pub max_abs_delta: Vec<(String, f64)>,
    pub first_divergence_seconds: Option<f64>,
    pub parent_status: RequirementStatus,
    pub branch_status: RequirementStatus,
}

/// Failure times across intervention levels, in the given order.
#[derive(Debug, Clone, PartialEq)]
pub struct DoseResponse { pub failure_times_seconds: Vec<Option<f64>>, pub monotonic: bool }

fn invalid(m: impl Into<String>) -> ExperimentError { ExperimentError::InvalidIntervention(m.into()) }

impl Experiments {
    /// Empty experiment set with a budget.
    pub fn new(budget: Budget) -> Self { Self { budget, used_steps: 0, branches: Vec::new() } }

    /// Branches recorded so far.
    pub fn branches(&self) -> &[BranchRun] { &self.branches }

    /// Forks a branch from `from` with one intervention and runs it to the end of the grid.
    pub fn fork(&mut self, id: Identifier, from: &Freeze, intervention: Intervention) -> Result<&BranchRun, ExperimentError> {
        if self.branches.iter().any(|b| b.id == id) { return Err(ExperimentError::DuplicateBranch(id)); }
        if self.branches.len() >= self.budget.max_branches {
            return Err(ExperimentError::BudgetExceeded { reason: format!("at most {} branches per request", self.budget.max_branches) });
        }
        let mut spec = from.spec().clone();
        let mut state = from.state().to_vec();
        match intervention {
            Intervention::Parameter { parameter, value } => {
                if !value.is_finite() { return Err(invalid("parameter value must be finite")); }
                let slot = match parameter {
                    Parameter::MassKg => &mut spec.model.mass_kg,
                    Parameter::DragCoefficient => &mut spec.model.drag_coefficient,
                    Parameter::ReferenceAreaM2 => &mut spec.model.reference_area_m2,
                    Parameter::AirDensityKgM3 => &mut spec.model.air_density_kg_m3,
                };
                let must_be_positive = matches!(parameter, Parameter::MassKg);
                if value < 0.0 || (must_be_positive && value == 0.0) { return Err(invalid(format!("{parameter:?} cannot be {value}"))); }
                *slot = value;
            }
            Intervention::State { variable, value } => {
                if !value.is_finite() { return Err(invalid("state value must be finite")); }
                let slot = state.get_mut(variable).ok_or_else(|| invalid(format!("state variable {variable} does not exist")))?;
                *slot = value;
            }
        }
        let (index, time) = from.logical_time();
        let steps = ((spec.end_seconds - time) / spec.step_seconds).ceil().max(0.0) as usize;
        if self.used_steps.saturating_add(steps) > self.budget.max_total_steps {
            return Err(ExperimentError::BudgetExceeded { reason: format!("{steps} more steps would exceed {} total steps per request", self.budget.max_total_steps) });
        }
        let run = continue_run(&spec, index, &state, from.prior_link()).map_err(|e| ExperimentError::Run(e.to_string()))?;
        self.used_steps += steps;
        self.branches.push(BranchRun { id, parent_freeze_sha256: from.sha256(), fork_index: index, fork_time_seconds: time, intervention, spec, run });
        Ok(self.branches.last().expect("just pushed"))
    }
}

/// Tests whether intervening as stated removes or delays the parent's failure.
pub fn intervention_test(ex: &mut Experiments, parent: &RecordedRun, from: &Freeze, limit: &StateLimit, id: Identifier, intervention: Intervention, tolerance_seconds: f64) -> Result<InterventionResult, ExperimentError> {
    if !(tolerance_seconds.is_finite() && tolerance_seconds >= 0.0) { return Err(invalid("tolerance must be finite and non-negative")); }
    let parent_failure = detect_failure(parent, limit).map(|f| f.time_seconds);
    let branch = ex.fork(id.clone(), from, intervention)?;
    let branch_failure = detect_failure(&branch.run, limit).map(|f| f.time_seconds);
    let verdict = match (parent_failure, branch_failure) {
        (None, _) => Verdict::Inconclusive,
        (Some(_), None) => Verdict::InterventionSupported,
        (Some(p), Some(b)) if (b - p).abs() <= tolerance_seconds => Verdict::NotSupported,
        (Some(p), Some(b)) if b > p => Verdict::InterventionSupported,
        _ => Verdict::Inconclusive,
    };
    let grade = match verdict { Verdict::InterventionSupported => "model-causal", Verdict::NotSupported => "not supported by intervention", Verdict::Inconclusive => "inconclusive" };
    Ok(InterventionResult { verdict, grade, parent_failure_seconds: parent_failure, branch_failure_seconds: branch_failure, branch: id })
}

fn status(run: &RecordedRun, limit: &StateLimit) -> RequirementStatus {
    if let Some(f) = detect_failure(run, limit) { return RequirementStatus::Violated { first_seconds: f.time_seconds }; }
    let margin = run.samples.iter().filter_map(|s| s.state.get(limit.variable))
        .map(|v| match limit.comparator { Comparator::AtLeast => v - limit.threshold, Comparator::AtMost => limit.threshold - v })
        .fold(f64::INFINITY, f64::min);
    RequirementStatus::Satisfied { margin }
}

/// Compares two runs at identical times only.
pub fn compare_runs(parent: &RecordedRun, branch: &RecordedRun, variables: &[(usize, String)], limit: &StateLimit) -> CompareReport {
    let width = |r: &RecordedRun| r.samples.first().map_or(0, |s| s.state.len());
    let width = width(parent).min(width(branch));
    let (present, missing): (Vec<_>, Vec<_>) = variables.iter().partition(|(i, _)| *i < width);
    let by_time: BTreeMap<u64, &helqix_freeze::Sample> = branch.samples.iter().map(|s| (s.time_seconds.to_bits(), s)).collect();
    let mut aligned = 0;
    let mut max_delta: Vec<(String, f64)> = present.iter().map(|(_, n)| (n.clone(), 0.0)).collect();
    let mut first_divergence = None;
    for p in &parent.samples {
        let Some(b) = by_time.get(&p.time_seconds.to_bits()) else { continue };
        aligned += 1;
        for (k, (i, _)) in present.iter().enumerate() {
            let d = (p.state[*i] - b.state[*i]).abs();
            if d > max_delta[k].1 { max_delta[k].1 = d; }
            if p.state[*i].to_bits() != b.state[*i].to_bits() && first_divergence.is_none() { first_divergence = Some(p.time_seconds); }
        }
    }
    CompareReport {
        alignment_rule: "compare only samples at identical (bit-equal) times; no interpolation; unmatched samples are counted",
        aligned_samples: aligned,
        unmatched_parent: parent.samples.len() - aligned,
        unmatched_branch: branch.samples.len() - aligned,
        missing_variables: missing.into_iter().map(|(_, n)| n.clone()).collect(),
        max_abs_delta: max_delta,
        first_divergence_seconds: first_divergence,
        parent_status: status(parent, limit),
        branch_status: status(branch, limit),
    }
}

/// Runs one branch per level and checks that the failure time moves monotonically with the level order.
/// A removed failure counts as later than any failure.
pub fn dose_response(ex: &mut Experiments, from: &Freeze, limit: &StateLimit, prefix: &str, levels: &[Intervention]) -> Result<DoseResponse, ExperimentError> {
    if levels.len() < 3 { return Err(invalid("dose-response needs at least 3 levels")); }
    let mut times = Vec::new();
    for (i, level) in levels.iter().enumerate() {
        let id = Identifier::parse(&format!("{prefix}_{i}")).map_err(|e| invalid(e.to_string()))?;
        let branch = ex.fork(id, from, level.clone())?;
        times.push(detect_failure(&branch.run, limit).map(|f| f.time_seconds));
    }
    let key: Vec<f64> = times.iter().map(|t| t.unwrap_or(f64::INFINITY)).collect();
    let rising = key.windows(2).all(|w| w[0] <= w[1]);
    let falling = key.windows(2).all(|w| w[0] >= w[1]);
    let constant = key.windows(2).all(|w| w[0] == w[1]);
    Ok(DoseResponse { failure_times_seconds: times, monotonic: (rising || falling) && !constant })
}
