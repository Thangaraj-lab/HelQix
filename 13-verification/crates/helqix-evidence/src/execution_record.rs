//! Immutable execution record that preserves timing and provenance.
use helqix_core::id::Identifier;
use helqix_time::SimTime;
use crate::provenance::{Provenance, ProvenanceError};
/// Terminal or in-progress execution state retained in evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus { Queued, Running, Completed, Failed, Cancelled }
/// Tool invocation with explicit model/tool revisions and content hashes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRecord { pub id: Identifier, pub provenance: Provenance, pub status: ExecutionStatus, pub started_at: SimTime, pub finished_at: Option<SimTime>, pub failure_reason: Option<String> }
impl ExecutionRecord {
    /// Creates a record and checks temporal and status invariants.
    pub fn new(id: Identifier, provenance: Provenance, status: ExecutionStatus, started_at: SimTime, finished_at: Option<SimTime>, failure_reason: Option<String>) -> Result<Self, ExecutionRecordError> {
        if finished_at.is_some_and(|end| end < started_at) { return Err(ExecutionRecordError::TimeReversed); }
        if matches!(status, ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Cancelled) != finished_at.is_some() { return Err(ExecutionRecordError::TerminalTimeMismatch); }
        if (status == ExecutionStatus::Failed) != failure_reason.is_some() { return Err(ExecutionRecordError::FailureReasonMismatch); }
        if failure_reason.as_ref().is_some_and(|value| value.trim().is_empty() || value.len() > 4_000) { return Err(ExecutionRecordError::InvalidFailureReason); }
        Ok(Self { id, provenance, status, started_at, finished_at, failure_reason })
    }
}
/// Execution record validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionRecordError { TimeReversed, TerminalTimeMismatch, FailureReasonMismatch, InvalidFailureReason, Provenance(ProvenanceError) }
impl core::fmt::Display for ExecutionRecordError { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self { Self::TimeReversed => f.write_str("execution finish precedes start"), Self::TerminalTimeMismatch => f.write_str("terminal execution requires finish time and nonterminal execution forbids it"), Self::FailureReasonMismatch => f.write_str("failure reason must exist only for failed executions"), Self::InvalidFailureReason => f.write_str("failure reason must contain 1–4000 bytes"), Self::Provenance(error) => error.fmt(f) } } }
impl std::error::Error for ExecutionRecordError {}
