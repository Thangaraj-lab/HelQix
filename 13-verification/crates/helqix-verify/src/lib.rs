//! # helqix-verify — Layer 13: Verification Requirements & Constraint Checking
//!
//! The verification half of HELQIX's verification/evidence layer. It answers
//! one question and nothing else: **do observed/executed results satisfy the
//! stated requirements?**
//!
//! ```text
//! Requirement ──▶ constraint_check::check() ──▶ VerificationResult ──▶ VerificationReport
//! (what must hold)       (pure comparison)      (PASS/FAIL/INCONCLUSIVE)   (aggregation)
//! ```
//!
//! ## Modules
//!
//! - [`requirement`] — explicit engineering conditions (`altitude >= 100 m`)
//!   with unit-aware bounds, operators, tolerances and severities;
//! - [`constraint_check`] — the only place comparisons are performed, with a
//!   single canonical unit conversion per check;
//! - [`verification_result`] — three-way verdicts with structured
//!   explanations and deterministic aggregation into reports.
//!
//! ## What this crate does NOT do
//!
//! - **Simulation** — Layer 06 (`helqix-simulation`) owns models and physics.
//! - **Validation** — Layer 23 owns model-vs-reality correctness claims.
//! - **State ownership** — Layer 04 (`helqix-state`) is the single source of
//!   truth; verification consumes observations supplied by callers, it never
//!   stores or mutates state.
//! - **Execution** — Layer 05 owns running systems.
//! - **Evidence storage** — the sibling `helqix-evidence` crate owns
//!   execution records, provenance and reproducibility contracts; results
//!   here are pure values that *become* evidence when packaged there.
//!
//! ## Verdict discipline
//!
//! PASS means "verified against supplied data". FAIL means "violated".
//! INCONCLUSIVE means "not verified" (missing subject, missing property,
//! non-finite observation) and is never aggregated into PASS — false
//! assurance is treated as a design defect, not a rounding detail.
//!
//! ## Determinism guarantees
//!
//! No clocks, no randomness, no I/O, no global registries. Batch evaluation
//! sorts by validated requirement identity before comparing, so identical
//! inputs produce byte-identical reports in any environment. Domain
//! independence is absolute: every domain concept arrives through caller-
//! supplied identifiers and Foundation units.

pub mod constraint_check;
pub mod requirement;
pub mod verification_result;

pub use constraint_check::{
    check, check_against_state, check_all, check_missing_subject, summarize,
};
pub use requirement::{Bound, Operator, Requirement, RequirementError, Severity};
pub use verification_result::{
    InconclusiveReason, ReportStatus, Verdict, VerdictExplanation, VerdictViolation,
    VerificationError, VerificationReport, VerificationResult,
};
