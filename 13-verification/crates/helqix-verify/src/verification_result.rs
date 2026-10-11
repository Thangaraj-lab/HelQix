//! Immutable, provenance-linked verification outcome.
use helqix_core::{id::Identifier, quantity::Quantity};
use crate::requirement::Requirement;
/// Outcome of checking an accepted requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict { Pass, Fail, Inconclusive }
/// Result of evaluating one requirement against one measured output.
#[derive(Debug, Clone, PartialEq)]
pub struct VerificationResult {
    pub requirement_id: Identifier, pub verdict: Verdict, pub expected: Quantity, pub observed: Option<Quantity>,
    pub source_run_id: Option<Identifier>, pub source_model_sha256: Option<String>, pub message: String,
}
impl VerificationResult {
    /// Creates a fail-closed result for a missing, unaccepted, incompatible, or stale measurement.
    pub fn inconclusive(requirement: &Requirement, observed: Option<Quantity>, run_id: Option<Identifier>, model_digest: Option<String>, message: impl Into<String>) -> Self {
        Self { requirement_id: requirement.id.clone(), verdict: Verdict::Inconclusive, expected: requirement.threshold, observed, source_run_id: run_id, source_model_sha256: model_digest, message: message.into() }
    }
}
