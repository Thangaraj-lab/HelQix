//! Typed, immutable events with deterministic logical-time ordering.
use std::collections::BTreeMap;
use helqix_core::{id::Identifier, quantity::Quantity};
use helqix_time::SimTime;

/// Supported event values preserve type and engineering units.
#[derive(Debug, Clone, PartialEq)]
pub enum EventValue { Bool(bool), Integer(i64), Text(String), Identifier(Identifier), Quantity(Quantity) }
/// Immutable observation or runtime fact published to the local event bus.
#[derive(Debug, Clone, PartialEq)]
pub struct Event { pub id: Identifier, pub event_type: Identifier, pub source: Identifier, pub at: SimTime, pub payload: BTreeMap<Identifier, EventValue> }
impl Event {
    /// Creates an event. Bus ordering is assigned only when it is accepted by an [`crate::EventBus`].
    pub fn new(id: Identifier, event_type: Identifier, source: Identifier, at: SimTime) -> Self { Self { id, event_type, source, at, payload: BTreeMap::new() } }
    /// Adds one typed payload field; duplicate keys are rejected.
    pub fn with_field(mut self, key: Identifier, value: EventValue) -> Result<Self, EventError> {
        if self.payload.contains_key(&key) { return Err(EventError::DuplicatePayloadField(key)); }
        self.payload.insert(key, value); Ok(self)
    }
    /// Finds a payload value by stable identifier.
    pub fn field(&self, key: &Identifier) -> Option<&EventValue> { self.payload.get(key) }
}
/// Invalid event payload operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventError { DuplicatePayloadField(Identifier) }
impl core::fmt::Display for EventError { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self { Self::DuplicatePayloadField(id) => write!(f, "event payload field already exists: {id}") } } }
impl std::error::Error for EventError {}
