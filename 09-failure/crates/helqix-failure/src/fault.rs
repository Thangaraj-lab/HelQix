//! Stable fault definitions and severity classification.
use helqix_core::{id::Identifier, quantity::Quantity};
use helqix_time::SimTime;

/// Engineering consequence class, ordered from informational to hazardous.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FaultSeverity { Info, Warning, Major, Critical }

/// A named, typed failure mode attached to a component.
#[derive(Debug, Clone, PartialEq)]
pub struct Fault {
    id: Identifier,
    component_id: Identifier,
    mode: Identifier,
    severity: FaultSeverity,
    description: String,
    first_observed_at: SimTime,
    trigger_value: Option<Quantity>,
}

impl Fault {
    /// Creates a fault after validating that its human-readable description is present.
    pub fn new(id: Identifier, component_id: Identifier, mode: Identifier, severity: FaultSeverity,
        description: impl Into<String>, first_observed_at: SimTime, trigger_value: Option<Quantity>) -> Result<Self, FaultError> {
        let description = description.into();
        if description.trim().is_empty() { return Err(FaultError::EmptyDescription); }
        if description.len() > 2_000 { return Err(FaultError::DescriptionTooLong); }
        Ok(Self { id, component_id, mode, severity, description, first_observed_at, trigger_value })
    }
    /// Stable identifier for this detector event.
    pub fn id(&self) -> &Identifier { &self.id }
    /// Component affected by the fault.
    pub fn component_id(&self) -> &Identifier { &self.component_id }
    /// Fault mode identifier.
    pub fn mode(&self) -> &Identifier { &self.mode }
    /// Severity classification.
    pub const fn severity(&self) -> FaultSeverity { self.severity }
    /// User-facing description.
    pub fn description(&self) -> &str { &self.description }
    /// Logical simulation time of first detection.
    pub const fn first_observed_at(&self) -> SimTime { self.first_observed_at }
    /// Optional measured quantity that triggered the detector.
    pub const fn trigger_value(&self) -> Option<Quantity> { self.trigger_value }
}

/// Fault-definition validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultError { EmptyDescription, DescriptionTooLong }
impl core::fmt::Display for FaultError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self { Self::EmptyDescription => f.write_str("fault description cannot be empty"), Self::DescriptionTooLong => f.write_str("fault description exceeds 2000 bytes") }
    }
}
impl std::error::Error for FaultError {}
