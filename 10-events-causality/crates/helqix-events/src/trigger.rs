//! Explicit event-to-action triggers; actions remain proposals for the execution layer.
use std::collections::BTreeMap;
use helqix_core::id::Identifier;
use crate::event::{Event, EventValue};

/// Optional exact-match payload guard.
#[derive(Debug, Clone, PartialEq)]
pub struct PayloadCondition { pub field: Identifier, pub equals: EventValue }
/// Rule mapping one event type and optional value predicate to a typed action request.
#[derive(Debug, Clone, PartialEq)]
pub struct Trigger { pub id: Identifier, pub event_type: Identifier, pub action_type: Identifier, pub target: Identifier, pub condition: Option<PayloadCondition>, pub enabled: bool }
/// Result suitable for submission to Layer 05; not an executed action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggeredAction { pub trigger_id: Identifier, pub source_event_id: Identifier, pub action_type: Identifier, pub target: Identifier, pub at: helqix_time::SimTime }
/// Deterministic trigger registry and evaluator.
#[derive(Debug, Clone, Default)]
pub struct TriggerEngine { triggers: BTreeMap<Identifier, Trigger> }
impl TriggerEngine {
    /// Registers a trigger; duplicate IDs are rejected.
    pub fn add(&mut self, trigger: Trigger) -> Result<(), TriggerError> { if self.triggers.contains_key(&trigger.id) { return Err(TriggerError::DuplicateId(trigger.id)); } self.triggers.insert(trigger.id.clone(), trigger); Ok(()) }
    /// Enables or disables an existing trigger without removing audit identity.
    pub fn set_enabled(&mut self, id: &Identifier, enabled: bool) -> Result<(), TriggerError> { let trigger = self.triggers.get_mut(id).ok_or_else(|| TriggerError::UnknownId(id.clone()))?; trigger.enabled = enabled; Ok(()) }
    /// Emits matching action requests in stable trigger-ID order.
    pub fn evaluate(&self, event: &Event) -> Vec<TriggeredAction> { self.triggers.values().filter(|t| t.enabled && t.event_type == event.event_type && t.condition.as_ref().is_none_or(|c| event.field(&c.field) == Some(&c.equals))).map(|t| TriggeredAction { trigger_id: t.id.clone(), source_event_id: event.id.clone(), action_type: t.action_type.clone(), target: t.target.clone(), at: event.at }).collect() }
    /// Number of configured triggers.
    pub fn len(&self) -> usize { self.triggers.len() }
    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool { self.triggers.is_empty() }
}
/// Trigger registry error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerError { DuplicateId(Identifier), UnknownId(Identifier) }
impl core::fmt::Display for TriggerError { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self { Self::DuplicateId(id) => write!(f, "trigger already exists: {id}"), Self::UnknownId(id) => write!(f, "trigger does not exist: {id}") } } }
impl std::error::Error for TriggerError {}
