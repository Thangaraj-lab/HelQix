//! Objective metrics and validated evaluation output.
use std::{collections::BTreeMap, fmt};
use helqix_core::Identifier;

/// Direction used when ranking one objective metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveDirection { /// Lower values rank ahead of higher values.
    Minimize, /// Higher values rank ahead of lower values.
    Maximize }

/// One metric that a search run seeks to minimize or maximize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Objective { metric: Identifier, direction: ObjectiveDirection }
impl Objective {
    /// Creates an objective over a named evaluator output metric.
    pub fn new(metric: Identifier, direction: ObjectiveDirection) -> Self { Self { metric, direction } }
    /// Metric read from evaluation results.
    pub fn metric(&self) -> &Identifier { &self.metric }
    /// Optimization direction.
    pub fn direction(&self) -> ObjectiveDirection { self.direction }
}

/// Finite metric outputs produced by one candidate evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation { metrics: BTreeMap<Identifier, f64> }
impl Evaluation {
    /// Creates an evaluation after validating every output is finite.
    pub fn new(metrics: BTreeMap<Identifier, f64>) -> Result<Self, ObjectiveError> {
        if let Some((metric, _)) = metrics.iter().find(|(_, value)| !value.is_finite()) {
            return Err(ObjectiveError::NonFiniteMetric(metric.clone()));
        }
        Ok(Self { metrics })
    }
    /// Metric values in canonical identifier order.
    pub fn metrics(&self) -> &BTreeMap<Identifier, f64> { &self.metrics }
    /// Returns a metric value, if emitted by the evaluator.
    pub fn value(&self, metric: &Identifier) -> Option<f64> { self.metrics.get(metric).copied() }
}

/// Invalid objective or evaluator output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectiveError {
    /// Evaluator emitted NaN or an infinite value.
    NonFiniteMetric(Identifier),
    /// Ranking requires at least one objective.
    NoObjectives,
    /// Objective list repeats a metric.
    DuplicateObjective(Identifier),
    /// Candidate evaluation omitted a required objective metric.
    MissingObjectiveMetric { /// Candidate ordinal.
        candidate: u64, /// Missing metric identity.
        metric: Identifier },
}
impl fmt::Display for ObjectiveError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self {
    Self::NonFiniteMetric(metric) => write!(f, "evaluation metric `{metric}` must be finite"), Self::NoObjectives => f.write_str("at least one objective is required"),
    Self::DuplicateObjective(metric) => write!(f, "objective metric `{metric}` is declared more than once"),
    Self::MissingObjectiveMetric { candidate, metric } => write!(f, "candidate {candidate} has no objective metric `{metric}`"),
} } }
impl std::error::Error for ObjectiveError {}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn evaluation_rejects_non_finite_output() { let mut map = BTreeMap::new(); map.insert(Identifier::parse("mass").unwrap(), f64::INFINITY); assert!(matches!(Evaluation::new(map), Err(ObjectiveError::NonFiniteMetric(_)))); }
    #[test] fn evaluation_keeps_named_metric() { let id = Identifier::parse("efficiency").unwrap(); let mut map = BTreeMap::new(); map.insert(id.clone(), 0.91); let evaluation = Evaluation::new(map).unwrap(); assert_eq!(evaluation.value(&id), Some(0.91)); }
}
