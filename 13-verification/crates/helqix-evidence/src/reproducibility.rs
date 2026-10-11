//! Evidence comparison for repeat executions of the same declared inputs.
use crate::execution_record::{ExecutionRecord, ExecutionStatus};
/// Result of comparing source and replay provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReproducibilityVerdict { Reproduced, Diverged, NotComparable }
/// Which reproducibility condition prevented or qualified a comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReproducibilityReason { ExactInputsAndOutputs, OutputDigestChanged, SourceNotCompleted, DifferentToolOrModel, DifferentInputs, DifferentProjectRevision }
/// Reproducibility assessment; equality of outputs is bitwise digest equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReproducibilityAssessment { pub verdict: ReproducibilityVerdict, pub reason: ReproducibilityReason }
/// Compares two saved executions without asserting physical model validity.
pub fn assess_reproducibility(source: &ExecutionRecord, replay: &ExecutionRecord) -> ReproducibilityAssessment {
    let p = &source.provenance; let q = &replay.provenance;
    let not_comparable = |reason| ReproducibilityAssessment { verdict: ReproducibilityVerdict::NotComparable, reason };
    if source.status != ExecutionStatus::Completed || replay.status != ExecutionStatus::Completed { return not_comparable(ReproducibilityReason::SourceNotCompleted); }
    if p.project_revision != q.project_revision { return not_comparable(ReproducibilityReason::DifferentProjectRevision); }
    if p.tool_id != q.tool_id || p.tool_version != q.tool_version || p.model_id != q.model_id || p.model_version != q.model_version || p.model_sha256 != q.model_sha256 { return not_comparable(ReproducibilityReason::DifferentToolOrModel); }
    if p.inputs_sha256 != q.inputs_sha256 { return not_comparable(ReproducibilityReason::DifferentInputs); }
    if p.outputs_sha256 == q.outputs_sha256 { ReproducibilityAssessment { verdict: ReproducibilityVerdict::Reproduced, reason: ReproducibilityReason::ExactInputsAndOutputs } }
    else { ReproducibilityAssessment { verdict: ReproducibilityVerdict::Diverged, reason: ReproducibilityReason::OutputDigestChanged } }
}
