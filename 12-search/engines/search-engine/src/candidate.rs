//! Candidate assignments and their evaluation lifecycle.
use std::collections::BTreeMap;
use helqix_core::Identifier;
use crate::{constraints::ConstraintViolation, objective::Evaluation};

/// Stable ordinal assigned to a generated candidate.
pub type CandidateId = u64;

/// Lifecycle state of a candidate in one search run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateStatus {
    /// Generated but not yet submitted to an evaluator.
    Pending,
    /// Evaluated successfully and satisfied all search constraints.
    Feasible,
    /// Evaluated successfully but violates one or more constraints.
    Infeasible,
    /// Evaluation failed; details are retained on the candidate.
    Failed,
}

/// One deterministic assignment and its evaluation outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    id: CandidateId,
    assignments: BTreeMap<Identifier, f64>,
    status: CandidateStatus,
    evaluation: Option<Evaluation>,
    violations: Vec<ConstraintViolation>,
    failure: Option<String>,
}
impl Candidate {
    /// Creates a pending candidate. Search-space validation is performed by the sampler/scheduler.
    pub fn new(id: CandidateId, assignments: BTreeMap<Identifier, f64>) -> Self {
        Self { id, assignments, status: CandidateStatus::Pending, evaluation: None, violations: Vec::new(), failure: None }
    }
    /// Candidate ordinal.
    pub fn id(&self) -> CandidateId { self.id }
    /// Named parameter assignments in identifier order.
    pub fn assignments(&self) -> &BTreeMap<Identifier, f64> { &self.assignments }
    /// One assigned value.
    pub fn value(&self, variable: &Identifier) -> Option<f64> { self.assignments.get(variable).copied() }
    /// Current lifecycle status.
    pub fn status(&self) -> CandidateStatus { self.status }
    /// Evaluation result when evaluation completed successfully.
    pub fn evaluation(&self) -> Option<&Evaluation> { self.evaluation.as_ref() }
    /// Constraint violations discovered before or after evaluation.
    pub fn violations(&self) -> &[ConstraintViolation] { &self.violations }
    /// Failure message when status is [`CandidateStatus::Failed`].
    pub fn failure(&self) -> Option<&str> { self.failure.as_deref() }

    pub(crate) fn set_infeasible(&mut self, violations: Vec<ConstraintViolation>) {
        self.status = CandidateStatus::Infeasible;
        self.violations = violations;
    }
    pub(crate) fn set_evaluated(&mut self, evaluation: Evaluation, violations: Vec<ConstraintViolation>) {
        self.evaluation = Some(evaluation);
        self.violations = violations;
        self.status = if self.violations.is_empty() { CandidateStatus::Feasible } else { CandidateStatus::Infeasible };
    }
    pub(crate) fn set_failed(&mut self, message: String) {
        self.status = CandidateStatus::Failed;
        self.failure = Some(message);
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn new_candidate_starts_pending_and_keeps_assignments() { let mut values = BTreeMap::new(); values.insert(Identifier::parse("mass").unwrap(), 4.0); let candidate = Candidate::new(7, values); assert_eq!(candidate.status(), CandidateStatus::Pending); assert_eq!(candidate.value(&Identifier::parse("mass").unwrap()), Some(4.0)); }
}
