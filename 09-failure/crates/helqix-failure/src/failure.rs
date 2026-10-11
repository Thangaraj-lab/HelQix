//! Revisioned lifecycle for detected failures and their observations.
use std::collections::BTreeMap;
use helqix_core::{id::Identifier, quantity::Quantity};
use helqix_time::SimTime;
use crate::fault::Fault;

/// Lifecycle of an observed fault; mitigated records are immutable terminal facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureStatus { Active, Acknowledged, Mitigated }

/// A measured signal associated with a failure at logical simulation time.
#[derive(Debug, Clone, PartialEq)]
pub struct FailureObservation { pub metric: Identifier, pub value: Quantity, pub at: SimTime }

/// A lifecycle event retained for auditability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusTransition { pub from: Option<FailureStatus>, pub to: FailureStatus, pub at: SimTime, pub revision: u64 }

/// A fault instance with monotonic observations and an append-only status history.
#[derive(Debug, Clone, PartialEq)]
pub struct FailureRecord {
    fault: Fault,
    status: FailureStatus,
    revision: u64,
    observations: BTreeMap<Identifier, Vec<FailureObservation>>,
    history: Vec<StatusTransition>,
    last_time: SimTime,
}

impl FailureRecord {
    /// Begins tracking a newly detected fault.
    pub fn new(fault: Fault) -> Self {
        let last_time = fault.first_observed_at();
        Self { fault, status: FailureStatus::Active, revision: 1, observations: BTreeMap::new(), history: vec![StatusTransition { from: None, to: FailureStatus::Active, at: last_time, revision: 1 }], last_time }
    }
    /// Returns the originating fault definition.
    pub fn fault(&self) -> &Fault { &self.fault }
    /// Current lifecycle status.
    pub const fn status(&self) -> FailureStatus { self.status }
    /// Monotonic record revision.
    pub const fn revision(&self) -> u64 { self.revision }
    /// Ordered status history.
    pub fn history(&self) -> &[StatusTransition] { &self.history }
    /// All measurements in deterministic metric order.
    pub fn observations(&self) -> impl Iterator<Item = &FailureObservation> { self.observations.values().flat_map(|items| items.iter()) }

    /// Appends a finite, unit-bearing observation; rejects terminal records and time reversal.
    pub fn observe(&mut self, metric: Identifier, value: Quantity, at: SimTime) -> Result<(), FailureError> {
        if self.status == FailureStatus::Mitigated { return Err(FailureError::AlreadyMitigated); }
        if at < self.last_time { return Err(FailureError::TimeReversed { previous: self.last_time, next: at }); }
        self.revision = self.revision.checked_add(1).ok_or(FailureError::RevisionOverflow)?;
        self.observations.entry(metric.clone()).or_default().push(FailureObservation { metric, value, at });
        self.last_time = at;
        Ok(())
    }

    /// Acknowledges active work once; repeat acknowledgement is idempotent.
    pub fn acknowledge(&mut self, at: SimTime) -> Result<(), FailureError> {
        if self.status == FailureStatus::Acknowledged { return Ok(()); }
        self.transition(FailureStatus::Acknowledged, at)
    }
    /// Marks the fault mitigated. Mitigation is terminal and does not assert that the system is safe.
    pub fn mitigate(&mut self, at: SimTime) -> Result<(), FailureError> { self.transition(FailureStatus::Mitigated, at) }

    fn transition(&mut self, target: FailureStatus, at: SimTime) -> Result<(), FailureError> {
        if self.status == FailureStatus::Mitigated { return Err(FailureError::AlreadyMitigated); }
        if at < self.last_time { return Err(FailureError::TimeReversed { previous: self.last_time, next: at }); }
        if self.status == FailureStatus::Acknowledged && target == FailureStatus::Active { return Err(FailureError::InvalidTransition { from: self.status, to: target }); }
        if target == self.status { return Ok(()); }
        let revision = self.revision.checked_add(1).ok_or(FailureError::RevisionOverflow)?;
        self.history.push(StatusTransition { from: Some(self.status), to: target, at, revision });
        self.status = target; self.revision = revision; self.last_time = at;
        Ok(())
    }
}

/// Lifecycle invariant failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureError { AlreadyMitigated, TimeReversed { previous: SimTime, next: SimTime }, RevisionOverflow, InvalidTransition { from: FailureStatus, to: FailureStatus } }
impl core::fmt::Display for FailureError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self {
        Self::AlreadyMitigated => f.write_str("mitigated failure record is terminal"),
        Self::TimeReversed { previous, next } => write!(f, "failure time moved backwards from {previous:?} to {next:?}"),
        Self::RevisionOverflow => f.write_str("failure record revision overflow"),
        Self::InvalidTransition { from, to } => write!(f, "invalid failure transition from {from:?} to {to:?}"),
    } }
}
impl std::error::Error for FailureError {}
