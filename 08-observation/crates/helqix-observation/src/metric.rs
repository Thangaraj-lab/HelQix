//! Typed, validated values emitted by Layer 08 observers.
use std::fmt;
use helqix_core::Identifier;
use helqix_time::SimTime;

/// Stable identifier of an observation metric.
pub type MetricId = Identifier;

/// Meaning of a sampled metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricKind { /// A point-in-time value.
    Gauge, /// A monotonically increasing total.
    Counter, /// A sampled value that represents a rate.
    Rate, /// A boolean condition represented as 0 or 1.
    Status }

/// Unit annotation retained with a metric sample.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MetricUnit { /// Dimensionless value.
    Dimensionless, /// Named unit symbol originating from the engineering model.
    Symbol(String) }

impl MetricUnit {
    /// Creates a unit annotation from a non-empty symbol.
    pub fn symbol(value: impl Into<String>) -> Result<Self, MetricError> {
        let value = value.into();
        if value.trim().is_empty() { return Err(MetricError::EmptyUnit); }
        if value.chars().any(char::is_control) { return Err(MetricError::InvalidUnit); }
        Ok(Self::Symbol(value))
    }
}

/// Supported scalar payloads for observations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricValue { /// Finite floating-point measurement.
    Number(f64), /// Exact signed integer measurement or count.
    Integer(i128), /// Boolean status.
    Boolean(bool) }

impl MetricValue {
    /// Creates a finite numeric metric value.
    pub fn number(value: f64) -> Result<Self, MetricError> {
        if !value.is_finite() { return Err(MetricError::NonFiniteValue); }
        Ok(Self::Number(value))
    }
}

/// One immutable observation sample.
#[derive(Debug, Clone, PartialEq)]
pub struct Metric {
    id: MetricId,
    kind: MetricKind,
    value: MetricValue,
    unit: MetricUnit,
    observed_at: SimTime,
    state_revision: u64,
}

impl Metric {
    /// Constructs a validated sample.
    pub fn new(id: MetricId, kind: MetricKind, value: MetricValue, unit: MetricUnit,
        observed_at: SimTime, state_revision: u64) -> Result<Self, MetricError> {
        if let MetricValue::Number(value) = value { if !value.is_finite() { return Err(MetricError::NonFiniteValue); } }
        if kind == MetricKind::Status && !matches!(value, MetricValue::Boolean(_)) {
            return Err(MetricError::StatusMustBeBoolean);
        }
        if kind == MetricKind::Counter && matches!(value, MetricValue::Number(v) if v < 0.0)
            || kind == MetricKind::Counter && matches!(value, MetricValue::Integer(v) if v < 0) {
            return Err(MetricError::CounterMustBeNonNegative);
        }
        Ok(Self { id, kind, value, unit, observed_at, state_revision })
    }
    /// Metric identity.
    pub fn id(&self) -> &MetricId { &self.id }
    /// Metric semantic kind.
    pub fn kind(&self) -> MetricKind { self.kind }
    /// Sample payload.
    pub fn value(&self) -> MetricValue { self.value }
    /// Unit annotation.
    pub fn unit(&self) -> &MetricUnit { &self.unit }
    /// Logical sampling time.
    pub fn observed_at(&self) -> SimTime { self.observed_at }
    /// Runtime state revision sampled.
    pub fn state_revision(&self) -> u64 { self.state_revision }
}

/// Validation failures for metric values and metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetricError { /// Numeric payload is NaN or infinite.
    NonFiniteValue, /// Status metrics must use a boolean payload.
    StatusMustBeBoolean, /// Counter values cannot be negative.
    CounterMustBeNonNegative, /// Unit symbol cannot be empty.
    EmptyUnit, /// Unit contains control characters.
    InvalidUnit }
impl fmt::Display for MetricError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", match self {
    Self::NonFiniteValue => "metric value must be finite", Self::StatusMustBeBoolean => "status metrics require a boolean value",
    Self::CounterMustBeNonNegative => "counter values cannot be negative", Self::EmptyUnit => "unit symbol cannot be empty", Self::InvalidUnit => "unit symbol contains a control character" }) } }
impl std::error::Error for MetricError {}

#[cfg(test)] mod tests {
    use super::*;
    fn id(s: &str) -> Identifier { Identifier::parse(s).unwrap() }
    #[test] fn rejects_non_finite_values() { assert_eq!(MetricValue::number(f64::NAN), Err(MetricError::NonFiniteValue)); }
    #[test] fn status_requires_boolean() { assert_eq!(Metric::new(id("alive"), MetricKind::Status, MetricValue::Integer(1), MetricUnit::Dimensionless, SimTime::ZERO, 2).unwrap_err(), MetricError::StatusMustBeBoolean); }
    #[test] fn counter_rejects_negative_values() { assert_eq!(Metric::new(id("samples"), MetricKind::Counter, MetricValue::Integer(-1), MetricUnit::Dimensionless, SimTime::ZERO, 0).unwrap_err(), MetricError::CounterMustBeNonNegative); }
    #[test] fn preserves_sample_provenance() { let m = Metric::new(id("speed"), MetricKind::Gauge, MetricValue::Number(3.0), MetricUnit::symbol("m/s").unwrap(), SimTime::from_nanos(9), 7).unwrap(); assert_eq!(m.observed_at(), SimTime::from_nanos(9)); assert_eq!(m.state_revision(), 7); }
}
