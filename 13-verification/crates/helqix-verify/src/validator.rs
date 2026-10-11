//! Evidence-aware requirement validation with exact model-revision matching.
use helqix_core::{id::Identifier, quantity::Quantity};
use crate::{constraint_check::compare, requirement::Requirement, verification_result::{Verdict, VerificationResult}};

/// One measured model output, linked to an immutable run and model digest.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement { pub metric: Identifier, pub value: Quantity, pub source_run_id: Identifier, pub model_sha256: String }
impl Measurement {
    /// Validates measurement provenance fields and a hexadecimal SHA-256 model digest.
    pub fn new(metric: Identifier, value: Quantity, source_run_id: Identifier, model_sha256: impl Into<String>) -> Result<Self, ValidationError> {
        let model_sha256 = model_sha256.into(); if !is_sha256(&model_sha256) { return Err(ValidationError::InvalidModelDigest); }
        Ok(Self { metric, value, source_run_id, model_sha256 })
    }
}
/// Evaluates requirements against the recorded model result; a mismatch never passes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Validator;
impl Validator {
    /// Checks an accepted requirement against an exact model revision. Missing/mismatched inputs are inconclusive.
    pub fn evaluate(&self, requirement: &Requirement, measurement: Option<&Measurement>, expected_model_sha256: &str) -> VerificationResult {
        let Some(measurement) = measurement else { return VerificationResult::inconclusive(requirement, None, None, None, "No measured evidence was supplied"); };
        let observed = Some(measurement.value);
        let metadata = (Some(measurement.source_run_id.clone()), Some(measurement.model_sha256.clone()));
        if !requirement.accepted { return VerificationResult::inconclusive(requirement, observed, metadata.0, metadata.1, "Requirement has not been explicitly accepted"); }
        if measurement.metric != requirement.metric { return VerificationResult::inconclusive(requirement, observed, metadata.0, metadata.1, "Evidence metric does not match the requirement"); }
        if !is_sha256(expected_model_sha256) || measurement.model_sha256 != expected_model_sha256 { return VerificationResult::inconclusive(requirement, observed, metadata.0, metadata.1, "Evidence is not from the exact required model revision"); }
        match compare(measurement.value, requirement.comparison, requirement.threshold) {
            Ok(true) => VerificationResult { requirement_id: requirement.id.clone(), verdict: Verdict::Pass, expected: requirement.threshold, observed, source_run_id: metadata.0, source_model_sha256: metadata.1, message: "Recorded value satisfies the accepted numeric threshold".into() },
            Ok(false) => VerificationResult { requirement_id: requirement.id.clone(), verdict: Verdict::Fail, expected: requirement.threshold, observed, source_run_id: metadata.0, source_model_sha256: metadata.1, message: "Recorded value does not satisfy the accepted numeric threshold".into() },
            Err(_) => VerificationResult::inconclusive(requirement, observed, metadata.0, metadata.1, "Evidence units are dimensionally incompatible with the requirement"),
        }
    }
}
/// Requirement or measurement validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError { InvalidModelDigest }
impl core::fmt::Display for ValidationError { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self { Self::InvalidModelDigest => f.write_str("model digest must be 64 lowercase hexadecimal characters") } } }
impl std::error::Error for ValidationError {}
fn is_sha256(value: &str) -> bool { value.len() == 64 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) }

#[cfg(test)]
mod tests {
    use super::*; use crate::requirement::{Comparison, RequirementPriority}; use helqix_core::{quantity::Quantity, units::Unit};
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    #[test]
    fn pass_requires_acceptance_matching_units_and_exact_revision() {
        let volts = Unit::parse("V").unwrap(); let amps = Unit::parse("A").unwrap();
        let mut requirement = Requirement::draft(id("min_voltage"), "Cell voltage stays above cutoff", RequirementPriority::Must, id("voltage"), Comparison::GreaterOrEqual, Quantity::new(3.0, volts).unwrap()).unwrap();
        let run = Measurement::new(id("voltage"), Quantity::new(3.2, volts).unwrap(), id("run_1"), "a".repeat(64)).unwrap();
        assert_eq!(Validator.evaluate(&requirement, Some(&run), &"a".repeat(64)).verdict, Verdict::Inconclusive);
        requirement.set_accepted(true).unwrap(); assert_eq!(Validator.evaluate(&requirement, Some(&run), &"b".repeat(64)).verdict, Verdict::Inconclusive);
        assert_eq!(Validator.evaluate(&requirement, Some(&run), &"a".repeat(64)).verdict, Verdict::Pass);
        let wrong = Measurement::new(id("voltage"), Quantity::new(3.2, amps).unwrap(), id("run_2"), "a".repeat(64)).unwrap();
        assert_eq!(Validator.evaluate(&requirement, Some(&wrong), &"a".repeat(64)).verdict, Verdict::Inconclusive);
    }
}
