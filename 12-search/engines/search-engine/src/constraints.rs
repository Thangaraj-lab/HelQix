//! Hard parameter constraints evaluated before expensive candidate execution.
use std::fmt;
use helqix_core::Identifier;
use crate::{candidate::Candidate, objective::Evaluation};

/// Input or output value tested by a hard search constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintTarget {
    /// Candidate design parameter checked before execution.
    Parameter(Identifier),
    /// Evaluator output metric checked after execution.
    Metric(Identifier),
}

/// Supported comparison for one parameter constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintOperator { /// Value is at most the bound.
    LessOrEqual, /// Value is at least the bound.
    GreaterOrEqual, /// Value is strictly less than the bound.
    LessThan, /// Value is strictly greater than the bound.
    GreaterThan, /// Value exactly equals the finite bound.
    Equal }
impl ConstraintOperator {
    /// Tests a finite value against the configured bound.
    pub fn accepts(self, value: f64, bound: f64) -> bool { match self {
        Self::LessOrEqual => value <= bound, Self::GreaterOrEqual => value >= bound,
        Self::LessThan => value < bound, Self::GreaterThan => value > bound, Self::Equal => value == bound,
    } }
}

/// A named hard constraint on one candidate parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchConstraint { id: Identifier, target: ConstraintTarget, operator: ConstraintOperator, bound: f64 }
impl SearchConstraint {
    /// Constructs a parameter constraint with a finite bound.
    pub fn new(id: Identifier, variable: Identifier, operator: ConstraintOperator, bound: f64) -> Result<Self, SearchConstraintError> {
        Self::parameter(id, variable, operator, bound)
    }
    /// Constructs a parameter constraint checked before candidate execution.
    pub fn parameter(id: Identifier, variable: Identifier, operator: ConstraintOperator, bound: f64) -> Result<Self, SearchConstraintError> {
        Self::with_target(id, ConstraintTarget::Parameter(variable), operator, bound)
    }
    /// Constructs an outcome-metric constraint checked after candidate execution.
    pub fn metric(id: Identifier, metric: Identifier, operator: ConstraintOperator, bound: f64) -> Result<Self, SearchConstraintError> {
        Self::with_target(id, ConstraintTarget::Metric(metric), operator, bound)
    }
    fn with_target(id: Identifier, target: ConstraintTarget, operator: ConstraintOperator, bound: f64) -> Result<Self, SearchConstraintError> {
        if !bound.is_finite() { return Err(SearchConstraintError::NonFiniteBound); }
        Ok(Self { id, target, operator, bound })
    }
    /// Constraint identity.
    pub fn id(&self) -> &Identifier { &self.id }
    /// Value source tested by this rule.
    pub fn target(&self) -> &ConstraintTarget { &self.target }
    /// Comparison operation.
    pub fn operator(&self) -> ConstraintOperator { self.operator }
    /// Finite comparison bound.
    pub fn bound(&self) -> f64 { self.bound }
    /// Returns a violation when the candidate fails this constraint.
    pub fn check(&self, candidate: &Candidate) -> Option<ConstraintViolation> {
        let ConstraintTarget::Parameter(variable) = &self.target else { return None; };
        let value = candidate.value(variable);
        match value {
            Some(value) if self.operator.accepts(value, self.bound) => None,
            _ => Some(ConstraintViolation { constraint: self.id.clone(), target: self.target.clone(), value, operator: self.operator, bound: self.bound }),
        }
    }
    /// Returns a violation when evaluator output fails an outcome constraint.
    pub fn check_evaluation(&self, evaluation: &Evaluation) -> Option<ConstraintViolation> {
        let ConstraintTarget::Metric(metric) = &self.target else { return None; };
        let value = evaluation.value(metric);
        match value {
            Some(value) if self.operator.accepts(value, self.bound) => None,
            _ => Some(ConstraintViolation { constraint: self.id.clone(), target: self.target.clone(), value, operator: self.operator, bound: self.bound }),
        }
    }
}

/// Detailed explanation of a rejected candidate parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintViolation { constraint: Identifier, target: ConstraintTarget, value: Option<f64>, operator: ConstraintOperator, bound: f64 }
impl ConstraintViolation {
    /// Constraint that rejected the candidate.
    pub fn constraint(&self) -> &Identifier { &self.constraint }
    /// Parameter or output metric inspected.
    pub fn target(&self) -> &ConstraintTarget { &self.target }
    /// Actual value, or `None` when the assignment omitted the variable.
    pub fn value(&self) -> Option<f64> { self.value }
    /// Required comparison.
    pub fn operator(&self) -> ConstraintOperator { self.operator }
    /// Required bound.
    pub fn bound(&self) -> f64 { self.bound }
}
impl fmt::Display for ConstraintViolation { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "constraint `{}` failed for `{:?}`: {:?} {:?} {}", self.constraint, self.target, self.value, self.operator, self.bound)
} }

/// Invalid constraint definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchConstraintError { /// Constraint bound is NaN or infinite.
    NonFiniteBound }
impl fmt::Display for SearchConstraintError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("search constraint bound must be finite") } }
impl std::error::Error for SearchConstraintError {}

#[cfg(test)] mod tests {
    use super::*;
    use std::collections::BTreeMap;
    #[test] fn accepts_inclusive_bound_and_reports_failure() { let variable = Identifier::parse("mass").unwrap(); let constraint = SearchConstraint::new(Identifier::parse("mass_limit").unwrap(), variable.clone(), ConstraintOperator::LessOrEqual, 10.0).unwrap(); let mut values = BTreeMap::new(); values.insert(variable, 11.0); let violation = constraint.check(&Candidate::new(0, values)).unwrap(); assert_eq!(violation.value(), Some(11.0)); }
    #[test] fn missing_parameter_is_a_violation() { let constraint = SearchConstraint::new(Identifier::parse("positive").unwrap(), Identifier::parse("mass").unwrap(), ConstraintOperator::GreaterThan, 0.0).unwrap(); let violation = constraint.check(&Candidate::new(0, BTreeMap::new())).unwrap(); assert_eq!(violation.value(), None); }
}
