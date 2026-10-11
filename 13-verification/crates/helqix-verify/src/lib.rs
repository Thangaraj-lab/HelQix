//! HELQIX Layer 13 typed requirements and fail-closed verification.
//! Passing checks establish only that the recorded value met the declared condition.
#![forbid(unsafe_code)]
/// Numeric comparison helpers.
pub mod constraint_check;
/// Human-authored engineering requirements.
pub mod requirement;
/// Requirement evaluator.
pub mod validator;
/// Immutable pass/fail/inconclusive result.
pub mod verification_result;
pub use constraint_check::compare;
pub use constraint_check::ConstraintCheck;
pub use requirement::{Comparison, Requirement, RequirementError, RequirementPriority};
pub use validator::{Measurement, ValidationError, Validator};
pub use verification_result::{Verdict, VerificationResult};
