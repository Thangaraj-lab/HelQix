//! Deterministic threshold detectors over typed, unit-bearing observations.
use std::collections::{BTreeMap, BTreeSet};
use helqix_core::{id::Identifier, quantity::Quantity};
use helqix_time::SimTime;
use crate::fault::{Fault, FaultError, FaultSeverity};

/// Comparison used by a threshold rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdOperator { GreaterThan, GreaterOrEqual, LessThan, LessOrEqual }
/// Declarative metric threshold that can raise one named fault mode.
#[derive(Debug, Clone, PartialEq)]
pub struct ThresholdRule { pub id: Identifier, pub component_id: Identifier, pub metric: Identifier, pub mode: Identifier, pub operator: ThresholdOperator, pub threshold: Quantity, pub severity: FaultSeverity, pub description: String }
impl ThresholdRule {
    /// Creates a validated rule. Unit conversion is checked when an observation arrives.
    pub fn new(id: Identifier, component_id: Identifier, metric: Identifier, mode: Identifier, operator: ThresholdOperator, threshold: Quantity, severity: FaultSeverity, description: impl Into<String>) -> Result<Self, DetectorError> {
        let description = description.into();
        if description.trim().is_empty() { return Err(DetectorError::EmptyDescription); }
        if description.len() > 2_000 { return Err(DetectorError::DescriptionTooLong); }
        Ok(Self { id, component_id, metric, mode, operator, threshold, severity, description })
    }
    fn is_triggered(&self, observed: Quantity) -> Result<bool, DetectorError> {
        let value = observed.value_in(self.threshold.unit()).map_err(|_| DetectorError::IncompatibleUnits { metric: self.metric.clone() })?;
        let threshold = self.threshold.value();
        Ok(match self.operator { ThresholdOperator::GreaterThan => value > threshold, ThresholdOperator::GreaterOrEqual => value >= threshold, ThresholdOperator::LessThan => value < threshold, ThresholdOperator::LessOrEqual => value <= threshold })
    }
}

/// Stateful edge-trigger detector: one active fault per rule until the condition clears.
#[derive(Debug, Clone, Default)]
pub struct FailureDetector { id: Option<Identifier>, rules: BTreeMap<Identifier, ThresholdRule>, active: BTreeSet<Identifier>, sequence: u64 }
impl FailureDetector {
    /// Creates a detector with a stable identity used to namespace emitted fault IDs.
    pub fn new(id: Identifier) -> Self { Self { id: Some(id), ..Self::default() } }
    /// Adds a rule; duplicate rule identifiers are rejected.
    pub fn add_rule(&mut self, rule: ThresholdRule) -> Result<(), DetectorError> {
        if self.rules.contains_key(&rule.id) { return Err(DetectorError::DuplicateRule(rule.id)); }
        self.rules.insert(rule.id.clone(), rule); Ok(())
    }
    /// Evaluates all rules for one sampled metric in stable rule-ID order.
    /// Returns only rising-edge faults, preventing duplicate active faults on every sample.
    pub fn observe(&mut self, metric: &Identifier, value: Quantity, at: SimTime) -> Result<Vec<Fault>, DetectorError> {
        let matching: Vec<_> = self.rules.values().filter(|rule| &rule.metric == metric).cloned().collect();
        let mut emitted = Vec::new();
        for rule in matching {
            if rule.is_triggered(value)? {
                if self.active.insert(rule.id.clone()) {
                    self.sequence = self.sequence.checked_add(1).ok_or(DetectorError::SequenceOverflow)?;
                    let detector = self.id.as_ref().map_or("detector", Identifier::as_str);
                    let prefix: String = detector.chars().take(45).collect();
                    let id = Identifier::parse(&format!("{prefix}_{}", self.sequence)).map_err(|_| DetectorError::SequenceOverflow)?;
                    emitted.push(Fault::new(id, rule.component_id, rule.mode, rule.severity, rule.description, at, Some(value))?);
                }
            } else { self.active.remove(&rule.id); }
        }
        Ok(emitted)
    }
    /// Number of configured rules.
    pub fn rule_count(&self) -> usize { self.rules.len() }
}

/// Configuration, unit, and identifier failures from fault detection.
#[derive(Debug, Clone, PartialEq)]
pub enum DetectorError { EmptyDescription, DescriptionTooLong, DuplicateRule(Identifier), IncompatibleUnits { metric: Identifier }, SequenceOverflow, Fault(FaultError) }
impl From<FaultError> for DetectorError { fn from(error: FaultError) -> Self { Self::Fault(error) } }
impl core::fmt::Display for DetectorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self {
        Self::EmptyDescription => f.write_str("detector description cannot be empty"), Self::DescriptionTooLong => f.write_str("detector description exceeds 2000 bytes"),
        Self::DuplicateRule(id) => write!(f, "detector rule already exists: {id}"), Self::IncompatibleUnits { metric } => write!(f, "observation unit does not match the rule for {metric}"),
        Self::SequenceOverflow => f.write_str("detector sequence overflow"), Self::Fault(error) => error.fmt(f),
    } }
}
impl std::error::Error for DetectorError {}
