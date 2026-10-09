//! # helqix-evidence — Layer 13: Evidence, Provenance & Audit
//!
//! The evidence half of HELQIX's verification/evidence layer. Verification
//! (`helqix-verify`) decides *whether requirements hold*; this crate makes
//! those decisions **auditable**: what was executed, where each result came
//! from, and whether it can be reproduced.
//!
//! ```text
//! Execution E-204 ──▶ Snapshot S-812 ──▶ Verification V-88 ──▶ Evidence EV-91
//!        (identity)        (identity)      (from helqix-verify)     + provenance chain
//! ```
//!
//! ## Modules
//!
//! - [`execution_record`] — validated identity of an engineering execution
//!   (Layer 05 context: branch, logical time window, terminal outcome);
//! - [`provenance`] — ordered chains of typed artifact references answering
//!   "where did this result come from?";
//! - [`evidence`] — durable evidence objects packaging a verification claim
//!   with its execution, snapshot and provenance context;
//! - [`reproducibility`] — contracts declaring what must match for a rerun to
//!   count as reproduction, judged by canonical identity comparison;
//! - [`audit_trail`] — append-only, monotonically-timed workflow records.
//!
//! ## What this crate does NOT do
//!
//! - **Execute** anything — Layer 05 owns execution engines; we only capture
//!   execution *identity* after the fact.
//! - **Own state** — Layer 04 is the single source of truth; snapshots are
//!   referenced by identity, never duplicated.
//! - **Verify** — verdict logic lives in `helqix-verify`; evidence packages
//!   results, it cannot mint or alter them.
//! - **Version projects** — no Git-like model here; provenance traces
//!   artifacts within executions, per the architecture docs.
//! - **Hash content** — the workspace has no hashing primitive, so
//!   reproducibility compares producer-supplied canonical identities instead
//!   of pretending to verify digests.
//!
//! Time values use the canonical Layer 10 abstraction
//! ([`helqix_events::event::LogicalTime`], integer nanoseconds), exactly as
//! Layers 09–12 do; Layer 07 remains the authoritative clock owner.
//!
//! ## Determinism guarantees
//!
//! No wall-clock reads, no randomness, no I/O, no implicit id generation.
//! Every constructor validates in fixed field order; ordering across objects
//! uses explicit sort keys (`(time, id)`), and equality is structural.

pub mod audit_trail;
pub mod evidence;
pub mod execution_record;
pub mod provenance;
pub mod reproducibility;

pub use audit_trail::{
    AuditAction, AuditEntry, AuditTrail, AuditTrailError, MAX_AUDIT_ENTRIES,
    MAX_AUDIT_NOTE_BYTES,
};
pub use evidence::{standard_chain, Evidence, EvidenceClaim, EvidenceError, EvidenceSpec};
pub use execution_record::{
    ExecutionId, ExecutionOutcome, ExecutionRecord, ExecutionRecordError, ExecutionRecordSpec,
};
pub use provenance::{
    Provenance, ProvenanceError, ProvenanceKind, ProvenanceNode, MAX_PROVENANCE_NODES,
};
pub use reproducibility::{
    ExecutionMode, MismatchField, MismatchKind, PinSet, ReproducibilityContract,
    ReproducibilityError, ReproductionVerdict, RerunContext,
};
