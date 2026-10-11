//! Pure comparator for finite, dimensionally compatible quantities.
use helqix_core::quantity::Quantity;
use crate::{requirement::{Comparison, Requirement}, validator::{Measurement, Validator}, verification_result::VerificationResult};
/// Evaluates a measurement against a threshold after unit conversion.
pub fn compare(actual: Quantity, comparison: Comparison, expected: Quantity) -> Result<bool, helqix_core::UnitError> {
    let value = actual.value_in(expected.unit())?; let limit = expected.value();
    Ok(match comparison { Comparison::GreaterThan => value > limit, Comparison::GreaterOrEqual => value >= limit, Comparison::Equal => value == limit, Comparison::LessOrEqual => value <= limit, Comparison::LessThan => value < limit })
}
/// Auditable requirement-to-measurement check facade.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintCheck { pub result: VerificationResult }
impl ConstraintCheck {
    /// Evaluates one requirement through the fail-closed verifier.
    pub fn evaluate(requirement: &Requirement, measurement: Option<&Measurement>, expected_model_sha256: &str) -> Self { Self { result: Validator.evaluate(requirement, measurement, expected_model_sha256) } }
}
