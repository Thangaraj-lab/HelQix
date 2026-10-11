//! End-to-end orchestration of deterministic sampling, evaluation, and ranking.
use std::{error::Error, fmt};
use crate::{execution_scheduler::{CandidateEvaluator, EvaluationFailurePolicy, ExecutionReport, ExecutionScheduler, SchedulerError}, objective::{Objective, ObjectiveError}, ranking::{rank_candidates, RankedCandidate, RankingError}, sampler::{Sampler, SamplerError}, search_space::{SearchSpace, SearchSpaceError}};

/// Complete output of one bounded search run.
#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationResult { execution: ExecutionReport, ranked: Vec<RankedCandidate> }
impl OptimizationResult {
    /// Scheduler output, including infeasible, failed, and still-pending candidates.
    pub fn execution(&self) -> &ExecutionReport { &self.execution }
    /// Ranked completed candidates, best Pareto fronts first.
    pub fn ranked(&self) -> &[RankedCandidate] { &self.ranked }
    /// Best-ranked candidate, if any candidate completed processing.
    pub fn best(&self) -> Option<&RankedCandidate> { self.ranked.first() }
}

/// Search configuration that bounds the work performed by one optimizer call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Optimizer { max_candidates: usize, failure_policy: EvaluationFailurePolicy }
impl Optimizer {
    /// Creates an optimizer with a hard candidate limit and failure policy.
    pub fn new(max_candidates: usize, failure_policy: EvaluationFailurePolicy) -> Result<Self, OptimizerError> {
        if max_candidates == 0 { return Err(OptimizerError::ZeroCandidateLimit); }
        Ok(Self { max_candidates, failure_policy })
    }
    /// Configured upper bound on generated candidates.
    pub fn max_candidates(&self) -> usize { self.max_candidates }

    /// Runs a deterministic bounded search over `space`.
    pub fn run(&self, space: &SearchSpace, objectives: &[Objective], evaluator: &dyn CandidateEvaluator) -> Result<OptimizationResult, OptimizerError> {
        validate_objectives(objectives).map_err(OptimizerError::Objective)?;
        let candidates = Sampler::new(self.max_candidates).map_err(OptimizerError::Sampler)?.sample(space).map_err(OptimizerError::Sampler)?;
        let scheduler = ExecutionScheduler::new(self.max_candidates, self.failure_policy).map_err(OptimizerError::Scheduler)?;
        let execution = scheduler.execute(space, candidates, evaluator).map_err(OptimizerError::Scheduler)?;
        let completed = execution.candidates().iter().filter(|candidate| candidate.status() != crate::CandidateStatus::Pending).cloned().collect::<Vec<_>>();
        let ranked = rank_candidates(&completed, objectives).map_err(OptimizerError::Ranking)?;
        Ok(OptimizationResult { execution, ranked })
    }
}

fn validate_objectives(objectives: &[Objective]) -> Result<(), ObjectiveError> {
    if objectives.is_empty() { return Err(ObjectiveError::NoObjectives); }
    let mut seen = std::collections::BTreeSet::new();
    for objective in objectives {
        if !seen.insert(objective.metric().clone()) { return Err(ObjectiveError::DuplicateObjective(objective.metric().clone())); }
    }
    Ok(())
}

/// Configuration, execution, or ranking error from an optimizer run.
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizerError {
    /// Candidate limit must be positive.
    ZeroCandidateLimit,
    /// Search-space setup or sampling failed.
    Sampler(SamplerError),
    /// Search-space structure is invalid.
    SearchSpace(SearchSpaceError),
    /// Objective configuration is invalid.
    Objective(ObjectiveError),
    /// Candidate scheduling failed.
    Scheduler(SchedulerError),
    /// Candidate ranking failed.
    Ranking(RankingError),
}
impl fmt::Display for OptimizerError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self {
    Self::ZeroCandidateLimit => f.write_str("optimizer candidate limit must be greater than zero"), Self::Sampler(error) => write!(f, "search sampling failed: {error}"),
    Self::SearchSpace(error) => write!(f, "search-space validation failed: {error}"), Self::Objective(error) => write!(f, "objective configuration failed: {error}"),
    Self::Scheduler(error) => write!(f, "candidate execution failed: {error}"), Self::Ranking(error) => write!(f, "candidate ranking failed: {error}"),
} } }
impl Error for OptimizerError { fn source(&self) -> Option<&(dyn Error + 'static)> { match self {
    Self::Sampler(e) => Some(e), Self::SearchSpace(e) => Some(e), Self::Objective(e) => Some(e), Self::Scheduler(e) => Some(e), Self::Ranking(e) => Some(e), Self::ZeroCandidateLimit => None,
} } }

#[cfg(test)] mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use helqix_core::Identifier;
    use crate::{Evaluation, Variable};
    fn id(s: &str) -> Identifier { Identifier::parse(s).unwrap() }
    #[test] fn orchestrates_a_bounded_search_and_returns_best_candidate() {
        let mut space = SearchSpace::new(); space.add_variable(Variable::discrete(id("x"), [3.0, 1.0, 2.0]).unwrap()).unwrap();
        let objectives = [Objective::new(id("cost"), crate::ObjectiveDirection::Minimize)];
        let result = Optimizer::new(2, EvaluationFailurePolicy::Continue).unwrap().run(&space, &objectives, &|candidate: &crate::Candidate| {
            let mut metrics = BTreeMap::new(); metrics.insert(id("cost"), candidate.value(&id("x")).unwrap()); Evaluation::new(metrics).map_err(|error| crate::EvaluationError::new(error.to_string()))
        }).unwrap();
        assert_eq!(result.execution().completed_count(), 2);
        assert_eq!(result.ranked().len(), 2);
        assert_eq!(result.best().unwrap().candidate().value(&id("x")), Some(1.0));
    }
    #[test] fn rejects_missing_objectives_before_evaluation() { let mut space = SearchSpace::new(); space.add_variable(Variable::discrete(id("x"), [1.0]).unwrap()).unwrap(); let result = Optimizer::new(1, EvaluationFailurePolicy::Continue).unwrap().run(&space, &[], &|_: &crate::Candidate| { panic!("evaluator must not run") }); assert!(matches!(result, Err(OptimizerError::Objective(ObjectiveError::NoObjectives)))); }
}
