//! Explicit event routing subscriptions.
use std::collections::BTreeSet;
use helqix_core::id::Identifier;
use crate::event::Event;

/// Declared subscription from one event type to one receiving component.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Subscription { pub event_type: Identifier, pub receiver: Identifier }
/// Delivery reference that preserves the immutable source event identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDelivery { pub event_id: Identifier, pub event_type: Identifier, pub receiver: Identifier, pub at: helqix_time::SimTime }
/// Stable event subscription table.
#[derive(Debug, Clone, Default)]
pub struct EventRouter { subscriptions: BTreeSet<Subscription> }
impl EventRouter {
    /// Subscribes a receiver; duplicate subscription is an error.
    pub fn subscribe(&mut self, event_type: Identifier, receiver: Identifier) -> Result<(), RouterError> { if !self.subscriptions.insert(Subscription { event_type, receiver }) { return Err(RouterError::AlreadySubscribed); } Ok(()) }
    /// Removes a subscription if it exists.
    pub fn unsubscribe(&mut self, event_type: &Identifier, receiver: &Identifier) -> bool { self.subscriptions.remove(&Subscription { event_type: event_type.clone(), receiver: receiver.clone() }) }
    /// Produces deterministic delivery references; the router does not mutate or execute receivers.
    pub fn route(&self, event: &Event) -> Vec<EventDelivery> { self.subscriptions.iter().filter(|s| s.event_type == event.event_type).map(|s| EventDelivery { event_id: event.id.clone(), event_type: event.event_type.clone(), receiver: s.receiver.clone(), at: event.at }).collect() }
    /// Number of subscriptions.
    pub fn len(&self) -> usize { self.subscriptions.len() }
}
/// Subscription-table error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterError { AlreadySubscribed }
impl core::fmt::Display for RouterError { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { f.write_str("event receiver is already subscribed") } }
impl std::error::Error for RouterError {}
