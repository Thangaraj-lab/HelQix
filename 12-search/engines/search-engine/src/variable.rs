//! Typed deterministic search variables and finite domains.
use std::fmt;
use helqix_core::Identifier;

/// Domain used to produce values for one search variable.
#[derive(Debug, Clone, PartialEq)]
pub enum VariableDomain {
    /// `samples` evenly spaced values over the inclusive interval.
    Continuous { /// Inclusive lower bound.
        min: f64, /// Inclusive upper bound.
        max: f64, /// Number of generated values, including both endpoints when greater than one.
        samples: usize },
    /// Explicit finite choices, retained in caller-provided order.
    Discrete(Vec<f64>),
}

/// One named parameter that the search engine may vary.
#[derive(Debug, Clone, PartialEq)]
pub struct Variable { id: Identifier, domain: VariableDomain }
impl Variable {
    /// Creates a continuous variable sampled on a closed interval.
    pub fn continuous(id: Identifier, min: f64, max: f64, samples: usize) -> Result<Self, VariableError> {
        if !min.is_finite() || !max.is_finite() { return Err(VariableError::NonFiniteBound); }
        if min > max { return Err(VariableError::InvertedBounds { min, max }); }
        if samples == 0 || (samples == 1 && min != max) || (samples > 1 && min == max) { return Err(VariableError::InvalidSampleCount { samples }); }
        Ok(Self { id, domain: VariableDomain::Continuous { min, max, samples } })
    }
    /// Creates a finite discrete variable. Values must be finite and unique.
    pub fn discrete(id: Identifier, values: impl IntoIterator<Item = f64>) -> Result<Self, VariableError> {
        let values = values.into_iter().collect::<Vec<_>>();
        if values.is_empty() { return Err(VariableError::EmptyDomain); }
        if values.iter().any(|value| !value.is_finite()) { return Err(VariableError::NonFiniteChoice); }
        for (index, value) in values.iter().enumerate() {
            if values[..index].contains(value) { return Err(VariableError::DuplicateChoice(*value)); }
        }
        Ok(Self { id, domain: VariableDomain::Discrete(values) })
    }
    /// Canonical variable identifier.
    pub fn id(&self) -> &Identifier { &self.id }
    /// Configured finite domain.
    pub fn domain(&self) -> &VariableDomain { &self.domain }
    /// Number of candidate values in this domain.
    pub fn value_count(&self) -> usize { match &self.domain {
        VariableDomain::Continuous { samples, .. } => *samples,
        VariableDomain::Discrete(values) => values.len(),
    } }
    /// Returns a domain value by zero-based sample index.
    pub fn value_at(&self, index: usize) -> Option<f64> { match &self.domain {
        VariableDomain::Continuous { min, max, samples } => {
            if index >= *samples { return None; }
            if *samples == 1 { return Some(*min); }
            if index == 0 { return Some(*min); }
            if index + 1 == *samples { return Some(*max); }
            let fraction = index as f64 / (*samples - 1) as f64;
            Some(min * (1.0 - fraction) + max * fraction)
        }
        VariableDomain::Discrete(values) => values.get(index).copied(),
    } }
    /// Returns whether `value` is valid for this declared domain.
    pub fn contains_value(&self, value: f64) -> bool {
        if !value.is_finite() { return false; }
        match &self.domain {
            VariableDomain::Continuous { min, max, .. } => value >= *min && value <= *max,
            VariableDomain::Discrete(values) => values.contains(&value),
        }
    }
}

/// Variable domain validation error.
#[derive(Debug, Clone, PartialEq)]
pub enum VariableError {
    /// A continuous interval contains a non-finite bound.
    NonFiniteBound,
    /// The minimum is greater than the maximum.
    InvertedBounds { /// Supplied minimum.
        min: f64, /// Supplied maximum.
        max: f64 },
    /// Sample count is zero, or one for a non-constant interval.
    InvalidSampleCount { /// Supplied count.
        samples: usize },
    /// A discrete domain has no choices.
    EmptyDomain,
    /// A discrete choice is NaN or infinite.
    NonFiniteChoice,
    /// A discrete value appears more than once.
    DuplicateChoice(f64),
}
impl fmt::Display for VariableError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self {
    Self::NonFiniteBound => f.write_str("continuous variable bounds must be finite"),
    Self::InvertedBounds { min, max } => write!(f, "variable minimum {min} exceeds maximum {max}"),
    Self::InvalidSampleCount { samples } => write!(f, "invalid sample count {samples} for continuous domain"),
    Self::EmptyDomain => f.write_str("discrete variable domain cannot be empty"),
    Self::NonFiniteChoice => f.write_str("discrete values must be finite"),
    Self::DuplicateChoice(value) => write!(f, "discrete value {value} is repeated"),
} } }
impl std::error::Error for VariableError {}

#[cfg(test)] mod tests {
    use super::*;
    fn id() -> Identifier { Identifier::parse("gain").unwrap() }
    #[test] fn continuous_domain_includes_exact_endpoints() { let v = Variable::continuous(id(), -2.0, 2.0, 3).unwrap(); assert_eq!(v.value_at(0), Some(-2.0)); assert_eq!(v.value_at(1), Some(0.0)); assert_eq!(v.value_at(2), Some(2.0)); }
    #[test] fn rejects_non_finite_and_inverted_bounds() { assert_eq!(Variable::continuous(id(), f64::NAN, 1.0, 2).unwrap_err(), VariableError::NonFiniteBound); assert!(matches!(Variable::continuous(id(), 2.0, 1.0, 2), Err(VariableError::InvertedBounds { .. }))); }
    #[test] fn discrete_values_reject_duplicates() { assert_eq!(Variable::discrete(id(), [1.0, 1.0]).unwrap_err(), VariableError::DuplicateChoice(1.0)); }
}
