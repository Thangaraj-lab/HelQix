//! Deterministic Pareto-front ranking for feasible candidates.
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use helqix_core::Identifier;
use crate::{candidate::{Candidate, CandidateId, CandidateStatus}, objective::{Objective, ObjectiveDirection}};

/// Candidate plus its stable position in the ranking result.
#[derive(Debug, Clone, PartialEq)]
pub struct RankedCandidate { candidate: Candidate, rank: usize, pareto_front: Option<usize> }
impl RankedCandidate {
    /// Candidate and its full evaluation record.
    pub fn candidate(&self) -> &Candidate { &self.candidate }
    /// Zero-based Pareto rank; equal for candidates on the same front.
    pub fn rank(&self) -> usize { self.rank }
    /// Zero-based Pareto front for feasible candidates, or `None` otherwise.
    pub fn pareto_front(&self) -> Option<usize> { self.pareto_front }
}

/// Ranks a completed candidate batch by feasibility, Pareto dominance, then stable IDs.
///
/// Feasible candidates are assigned nondominated fronts. Infeasible candidates
/// follow in ascending violation-count order, then failed candidates. Candidate
/// IDs break ties deterministically; input order does not affect the result.
pub fn rank_candidates(candidates: &[Candidate], objectives: &[Objective]) -> Result<Vec<RankedCandidate>, RankingError> {
    if objectives.is_empty() { return Err(RankingError::NoObjectives); }
    let mut objective_ids = BTreeSet::new();
    for objective in objectives {
        if !objective_ids.insert(objective.metric().clone()) { return Err(RankingError::DuplicateObjective(objective.metric().clone())); }
    }
    let mut ids = BTreeSet::new();
    for candidate in candidates {
        if !ids.insert(candidate.id()) { return Err(RankingError::DuplicateCandidate(candidate.id())); }
        if candidate.status() == CandidateStatus::Pending { return Err(RankingError::PendingCandidate(candidate.id())); }
        if candidate.status() == CandidateStatus::Feasible {
            let evaluation = candidate.evaluation().ok_or(RankingError::MissingEvaluation(candidate.id()))?;
            for objective in objectives {
                if evaluation.value(objective.metric()).is_none() { return Err(RankingError::MissingObjectiveMetric { candidate: candidate.id(), metric: objective.metric().clone() }); }
            }
        }
    }

    let mut remaining = candidates.iter().filter(|c| c.status() == CandidateStatus::Feasible).collect::<Vec<_>>();
    let mut result = Vec::with_capacity(candidates.len());
    let mut front = 0usize;
    while !remaining.is_empty() {
        let current_front = remaining.iter().copied().filter(|candidate| {
            !remaining.iter().copied().any(|other| other.id() != candidate.id() && dominates(other, candidate, objectives))
        }).collect::<Vec<_>>();
        if current_front.is_empty() { return Err(RankingError::NoProgress); }
        let current_ids = current_front.iter().map(|candidate| candidate.id()).collect::<BTreeSet<_>>();
        let mut ordered_front = current_front;
        ordered_front.sort_by_key(|candidate| candidate.id());
        for candidate in ordered_front { result.push(RankedCandidate { candidate: candidate.clone(), rank: front, pareto_front: Some(front) }); }
        remaining.retain(|candidate| !current_ids.contains(&candidate.id()));
        front += 1;
    }

    let mut infeasible = candidates.iter().filter(|c| c.status() == CandidateStatus::Infeasible).collect::<Vec<_>>();
    infeasible.sort_by_key(|candidate| (candidate.violations().len(), candidate.id()));
    for candidate in infeasible { result.push(RankedCandidate { candidate: candidate.clone(), rank: front, pareto_front: None }); front += 1; }

    let mut failed = candidates.iter().filter(|c| c.status() == CandidateStatus::Failed).collect::<Vec<_>>();
    failed.sort_by_key(|candidate| candidate.id());
    for candidate in failed { result.push(RankedCandidate { candidate: candidate.clone(), rank: front, pareto_front: None }); front += 1; }
    Ok(result)
}

fn dominates(left: &Candidate, right: &Candidate, objectives: &[Objective]) -> bool {
    let (Some(left_result), Some(right_result)) = (left.evaluation(), right.evaluation()) else { return false; };
    let mut strictly_better = false;
    for objective in objectives {
        let (Some(a), Some(b)) = (left_result.value(objective.metric()), right_result.value(objective.metric())) else { return false; };
        let (no_worse, better) = match objective.direction() {
            ObjectiveDirection::Minimize => (a <= b, a < b),
            ObjectiveDirection::Maximize => (a >= b, a > b),
        };
        if !no_worse { return false; }
        strictly_better |= better;
    }
    strictly_better
}

/// Ranking input or objective error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RankingError {
    /// At least one objective is required.
    NoObjectives,
    /// An objective metric was declared twice.
    DuplicateObjective(Identifier),
    /// Candidate ID is duplicated.
    DuplicateCandidate(CandidateId),
    /// A candidate was not processed by the scheduler.
    PendingCandidate(CandidateId),
    /// A candidate marked feasible has no evaluation payload.
    MissingEvaluation(CandidateId),
    /// A feasible candidate omitted a required objective output.
    MissingObjectiveMetric { /// Candidate ordinal.
        candidate: CandidateId, /// Required metric.
        metric: Identifier },
    /// No candidate could be selected for the next Pareto front.
    NoProgress,
}
impl fmt::Display for RankingError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self {
    Self::NoObjectives => f.write_str("ranking requires at least one objective"), Self::DuplicateObjective(id) => write!(f, "objective `{id}` is repeated"),
    Self::DuplicateCandidate(id) => write!(f, "candidate ID {id} is repeated"), Self::PendingCandidate(id) => write!(f, "candidate {id} has not been evaluated"),
    Self::MissingEvaluation(id) => write!(f, "feasible candidate {id} has no evaluation"), Self::MissingObjectiveMetric { candidate, metric } => write!(f, "candidate {candidate} omitted objective `{metric}`"),
    Self::NoProgress => f.write_str("Pareto ranking could not make progress"),
} } }
impl Error for RankingError {}

#[cfg(test)] mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use crate::{objective::Evaluation, Candidate};
    fn id(s: &str) -> Identifier { Identifier::parse(s).unwrap() }
    fn candidate(id_num: u64, a: f64, b: f64) -> Candidate { let mut values = BTreeMap::new(); values.insert(id("a"), a); values.insert(id("b"), b); let mut output = BTreeMap::new(); output.insert(id("cost"), a); output.insert(id("strength"), b); let mut candidate = Candidate::new(id_num, values); candidate.set_evaluated(Evaluation::new(output).unwrap(), Vec::new()); candidate }
    fn objectives() -> Vec<Objective> { vec![Objective::new(id("cost"), ObjectiveDirection::Minimize), Objective::new(id("strength"), ObjectiveDirection::Maximize)] }
    #[test] fn finds_pareto_front_and_is_deterministic() { let candidates = vec![candidate(3, 3.0, 3.0), candidate(2, 2.0, 2.0), candidate(1, 1.0, 1.0)]; let ranked = rank_candidates(&candidates, &objectives()).unwrap(); assert_eq!(ranked.iter().map(|r| r.candidate().id()).collect::<Vec<_>>(), vec![1, 2, 3]); assert!(ranked.iter().all(|r| r.pareto_front() == Some(0))); }
    #[test] fn single_objective_dominance_creates_ordered_fronts() { let ranked = rank_candidates(&[candidate(0, 5.0, 1.0), candidate(1, 1.0, 5.0)], &[Objective::new(id("cost"), ObjectiveDirection::Minimize)]).unwrap(); assert_eq!(ranked[0].candidate().id(), 1); assert_eq!(ranked[0].rank(), 0); assert_eq!(ranked[1].rank(), 1); }
    #[test] fn missing_metric_fails_explicitly() { let mut c = Candidate::new(0, BTreeMap::new()); c.set_evaluated(Evaluation::new(BTreeMap::new()).unwrap(), Vec::new()); assert!(matches!(rank_candidates(&[c], &objectives()), Err(RankingError::MissingObjectiveMetric { .. }))); }
}
