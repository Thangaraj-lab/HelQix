//! HELQIX evidence contracts for reproducible model execution and verification.
#![forbid(unsafe_code)]
/// Project-scoped immutable evidence bundle.
pub mod evidence;
/// Immutable tool execution record.
pub mod execution_record;
/// Model and input provenance.
pub mod provenance;
/// Run-to-run reproducibility assessment.
pub mod reproducibility;
pub use evidence::{EvidenceBundle, EvidenceError};
pub use execution_record::{ExecutionRecord, ExecutionRecordError, ExecutionStatus};
pub use provenance::{Provenance, ProvenanceError};
pub use reproducibility::{assess_reproducibility, ReproducibilityAssessment, ReproducibilityReason, ReproducibilityVerdict};
