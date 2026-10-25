//! Deterministic, bounded candidate evaluation scheduling.
use std::{collections::BTreeSet, error::Error, fmt};
use crate::{candidate::{Candidate, CandidateId}, constraints::{ConstraintTarget, ConstraintViolation}, objective::Evaluation, search_space::{SearchSpace, SearchSpaceError}};

/// Error returned by a simulation, engineering model, or external evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationError { message: String }
impl EvaluationError {
    /// Creates an evaluation failure with a user-readable explanation.
    pub fn new(message: impl Into<String>) -> Self { Self { message: message.into() } }
    /// Failure explanation.
    pub fn message(&self) -> &str { &self.message }
}
impl fmt::Display for EvaluationError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.message) } }
impl Error for EvaluationError {}

/// Contract implemented by the product's simulation or model evaluation adapter.
pub trait CandidateEvaluator {
    /// Evaluates one valid assignment and returns named finite output metrics.
    fn evaluate(&self, candidate: &Candidate) -> Result<Evaluation, EvaluationError>;
}
impl<F> CandidateEvaluator for F where F: Fn(&Candidate) -> Result<Evaluation, EvaluationError> {
    fn evaluate(&self, candidate: &Candidate) -> Result<Evaluation, EvaluationError> { self(candidate) }
}

/// Behavior after one candidate evaluation fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationFailurePolicy {
    /// Retain the failure and continue with later candidates.
    Continue,
    /// Retain the failure and leave later candidates pending.
    Stop,
}

/// Bounded deterministic evaluation scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionScheduler { max_evaluations: usize, failure_policy: EvaluationFailurePolicy }
impl ExecutionScheduler {
    /// Creates a scheduler with an explicit evaluator-call budget.
    pub fn new(max_evaluations: usize, failure_policy: EvaluationFailurePolicy) -> Result<Self, SchedulerError> {
        if max_evaluations == 0 { return Err(SchedulerError::ZeroBudget); }
        Ok(Self { max_evaluations, failure_policy })
    }
    /// Maximum number of candidate records accepted per execution call.
    pub fn max_evaluations(&self) -> usize { self.max_evaluations }

    /// Validates the full batch, then evaluates valid candidates in input order.
    ///
    /// Hard-constraint failures are marked infeasible and never reach the
    /// evaluator. Structural errors are discovered before any evaluation begins.
    pub fn execute(&self, space: &SearchSpace, candidates: Vec<Candidate>, evaluator: &dyn CandidateEvaluator) -> Result<ExecutionReport, SchedulerError> {
        if candidates.len() > self.max_evaluations { return Err(SchedulerError::BudgetExceeded { requested: candidates.len(), budget: self.max_evaluations }); }
        let mut seen = BTreeSet::new();
        let mut prepared = Vec::with_capacity(candidates.len());
        for candidate in &candidates {
            if !seen.insert(candidate.id()) { return Err(SchedulerError::DuplicateCandidate(candidate.id())); }
            if candidate.status() != crate::CandidateStatus::Pending { return Err(SchedulerError::CandidateAlreadyProcessed(candidate.id())); }
            let violations = space.validate_assignment(candidate.assignments()).map_err(|error| SchedulerError::InvalidAssignment { candidate: candidate.id(), source: error })?;
            prepared.push(violations);
        }
        let mut attempted = 0usize;
        let mut completed = 0usize;
        let mut failed = 0usize;
        let mut rejected = 0usize;
        let mut halted = false;
        let mut output = Vec::with_capacity(candidates.len());
        let mut batch = candidates.into_iter().zip(prepared).peekable();
        while let Some((mut candidate, violations)) = batch.next() {
            if !violations.is_empty() {
                candidate.set_infeasible(violations);
                rejected += 1;
                output.push(candidate);
                continue;
            }
            attempted += 1;
            match evaluator.evaluate(&candidate) {
                Ok(evaluation) => {
                    let outcome_violations = space.constraints()
                        .filter(|constraint| matches!(constraint.target(), ConstraintTarget::Metric(_)))
                        .filter_map(|constraint| constraint.check_evaluation(&evaluation))
                        .collect::<Vec<ConstraintViolation>>();
                    candidate.set_evaluated(evaluation, outcome_violations);
                    completed += 1;
                }
                Err(error) => {
                    candidate.set_failed(error.to_string());
                    failed += 1;
                    if self.failure_policy == EvaluationFailurePolicy::Stop {
                        halted = true;
                        output.push(candidate);
                        output.extend(batch.map(|(pending, _)| pending));
                        break;
                    }
                }
            }
            output.push(candidate);
        }
        Ok(ExecutionReport { candidates: output, attempted, completed, failed, rejected, halted })
    }
}

/// Results from one scheduler pass.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionReport { candidates: Vec<Candidate>, attempted: usize, completed: usize, failed: usize, rejected: usize, halted: bool }
impl ExecutionReport {
    /// Candidate records, preserving input order; unprocessed candidates after a stop remain pending.
    pub fn candidates(&self) -> &[Candidate] { &self.candidates }
    /// Number of evaluator calls made.
    pub fn attempted_count(&self) -> usize { self.attempted }
    /// Number of successful evaluator results.
    pub fn completed_count(&self) -> usize { self.completed }
    /// Number of evaluator failures.
    pub fn failed_count(&self) -> usize { self.failed }
    /// Number of hard-constraint rejects.
    pub fn rejected_count(&self) -> usize { self.rejected }
    /// Whether stop-on-error halted the batch.
    pub fn halted(&self) -> bool { self.halted }
    /// Returns candidates not visited after an early stop. They are retained in this report as pending.
    pub fn pending_count(&self) -> usize { self.candidates.iter().filter(|candidate| candidate.status() == crate::CandidateStatus::Pending).count() }
}

/// Scheduler configuration or batch validation error.
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulerError {
    /// Evaluation budget must be positive.
    ZeroBudget,
    /// Candidate batch exceeds the configured bound.
    BudgetExceeded { /// Requested candidate count.
        requested: usize, /// Configured budget.
        budget: usize },
    /// Candidate ID appears more than once in a batch.
    DuplicateCandidate(CandidateId),
    /// Candidate has already been evaluated or rejected.
    CandidateAlreadyProcessed(CandidateId),
    /// Candidate assignments are structurally invalid.
    InvalidAssignment { /// Candidate ordinal.
        candidate: CandidateId, /// Assignment error.
        source: SearchSpaceError },
}
impl fmt::Display for SchedulerError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self {
    Self::ZeroBudget => f.write_str("scheduler budget must be greater than zero"), Self::BudgetExceeded { requested, budget } => write!(f, "batch of {requested} candidates exceeds evaluation budget {budget}"),
    Self::DuplicateCandidate(id) => write!(f, "candidate ID {id} is repeated in batch"), Self::CandidateAlreadyProcessed(id) => write!(f, "candidate {id} is not pending"),
    Self::InvalidAssignment { candidate, source } => write!(f, "candidate {candidate} has invalid assignments: {source}"),
} } }
impl Error for SchedulerError { fn source(&self) -> Option<&(dyn Error + 'static)> { match self { Self::InvalidAssignment { source, .. } => Some(source), _ => None } } }

#[cfg(test)] mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use helqix_core::Identifier;
    use crate::{Sampler, SearchSpace, Variable};
    fn space() -> SearchSpace { let mut s = SearchSpace::new(); s.add_variable(Variable::discrete(Identifier::parse("x").unwrap(), [1.0, 2.0]).unwrap()).unwrap(); s }
    fn evaluator(_: &Candidate) -> Result<Evaluation, EvaluationError> { let mut map = BTreeMap::new(); map.insert(Identifier::parse("cost").unwrap(), 3.0); Evaluation::new(map).map_err(|e| EvaluationError::new(e.to_string())) }
    #[test] fn evaluates_in_order_under_budget() { let candidates = Sampler::new(2).unwrap().sample(&space()).unwrap(); let report = ExecutionScheduler::new(2, EvaluationFailurePolicy::Continue).unwrap().execute(&space(), candidates, &evaluator).unwrap(); assert_eq!(report.completed_count(), 2); assert_eq!(report.attempted_count(), 2); }
    #[test] fn outcome_constraints_are_checked_after_evaluation() { let mut s = space(); s.add_constraint(crate::SearchConstraint::metric(Identifier::parse("cost_cap").unwrap(), Identifier::parse("cost").unwrap(), crate::ConstraintOperator::LessOrEqual, 2.0).unwrap()).unwrap(); let candidates = Sampler::new(2).unwrap().sample(&s).unwrap(); let cost_from_x = |candidate: &Candidate| { let x = candidate.value(&Identifier::parse("x").unwrap()).unwrap(); let mut map = BTreeMap::new(); map.insert(Identifier::parse("cost").unwrap(), x + 1.0); Evaluation::new(map).map_err(|e| EvaluationError::new(e.to_string())) }; let report = ExecutionScheduler::new(2, EvaluationFailurePolicy::Continue).unwrap().execute(&s, candidates, &cost_from_x).unwrap(); assert_eq!(report.candidates()[0].status(), crate::CandidateStatus::Feasible); assert_eq!(report.candidates()[1].status(), crate::CandidateStatus::Infeasible); assert!(report.candidates()[1].evaluation().is_some()); }
    #[test] fn validates_whole_batch_before_evaluation() { use std::cell::Cell; let bad = Candidate::new(0, BTreeMap::new()); let scheduler = ExecutionScheduler::new(2, EvaluationFailurePolicy::Continue).unwrap(); let called = Cell::new(false); let result = scheduler.execute(&space(), vec![bad], &|candidate: &Candidate| { called.set(true); evaluator(candidate) }); assert!(result.is_err()); assert!(!called.get()); }
    #[test] fn stop_policy_leaves_remaining_candidates_pending() { let candidates = Sampler::new(2).unwrap().sample(&space()).unwrap(); let report = ExecutionScheduler::new(2, EvaluationFailurePolicy::Stop).unwrap().execute(&space(), candidates, &|_: &Candidate| Err(EvaluationError::new("sim failed"))).unwrap(); assert!(report.halted()); assert_eq!(report.failed_count(), 1); assert_eq!(report.pending_count(), 1); }
}
