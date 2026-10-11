//! Deterministic, bounded-at-delivery event queue.
use std::collections::{BTreeMap, BTreeSet};
use helqix_core::id::Identifier;
use helqix_time::SimTime;
use crate::event::Event;

/// Maximum number of pending events accepted to bound memory growth.
pub const DEFAULT_MAX_PENDING_EVENTS: usize = 100_000;
/// Stable logical-time event bus. It is single-writer by design; concurrency is external.
#[derive(Debug, Clone)]
pub struct EventBus { pending: BTreeMap<(SimTime, u64), Event>, seen: BTreeSet<Identifier>, watermark: SimTime, next_sequence: u64, maximum_pending: usize }
impl Default for EventBus { fn default() -> Self { Self::new(DEFAULT_MAX_PENDING_EVENTS) } }
impl EventBus {
    /// Creates a bus with an explicit pending-event capacity.
    pub fn new(maximum_pending: usize) -> Self { Self { pending: BTreeMap::new(), seen: BTreeSet::new(), watermark: SimTime::ZERO, next_sequence: 0, maximum_pending } }
    /// Publishes an event if its ID is unique and it is not earlier than delivered time.
    pub fn publish(&mut self, event: Event) -> Result<u64, EventBusError> {
        if event.at < self.watermark { return Err(EventBusError::LateEvent { event_at: event.at, watermark: self.watermark }); }
        if self.pending.len() >= self.maximum_pending { return Err(EventBusError::CapacityExceeded { maximum: self.maximum_pending }); }
        if self.seen.contains(&event.id) { return Err(EventBusError::DuplicateEvent(event.id)); }
        let sequence = self.next_sequence.checked_add(1).ok_or(EventBusError::SequenceOverflow)?;
        let key = (event.at, sequence); self.seen.insert(event.id.clone()); self.pending.insert(key, event); self.next_sequence = sequence;
        Ok(sequence)
    }
    /// Removes and returns all events at or before `until`, in `(time, publish sequence)` order.
    pub fn drain_until(&mut self, until: SimTime) -> Result<Vec<Event>, EventBusError> {
        if until < self.watermark { return Err(EventBusError::WatermarkReversed { current: self.watermark, requested: until }); }
        let keys: Vec<_> = self.pending.range(..=(until, u64::MAX)).map(|(key, _)| *key).collect();
        let events = keys.into_iter().filter_map(|key| self.pending.remove(&key)).collect(); self.watermark = until; Ok(events)
    }
    /// Current delivery watermark.
    pub const fn watermark(&self) -> SimTime { self.watermark }
    /// Pending event count.
    pub fn pending_count(&self) -> usize { self.pending.len() }
    /// Total unique events accepted since bus creation.
    pub fn accepted_count(&self) -> usize { self.seen.len() }
    /// Iterates pending events in delivery order.
    pub fn pending(&self) -> impl Iterator<Item = &Event> { self.pending.values() }
}
/// Queue, ordering, and capacity errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventBusError { LateEvent { event_at: SimTime, watermark: SimTime }, WatermarkReversed { current: SimTime, requested: SimTime }, CapacityExceeded { maximum: usize }, DuplicateEvent(Identifier), SequenceOverflow }
impl core::fmt::Display for EventBusError { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self {
    Self::LateEvent { event_at, watermark } => write!(f, "event at {event_at:?} precedes delivery watermark {watermark:?}"),
    Self::WatermarkReversed { current, requested } => write!(f, "event bus watermark cannot move from {current:?} back to {requested:?}"),
    Self::CapacityExceeded { maximum } => write!(f, "event bus pending capacity {maximum} reached"),
    Self::DuplicateEvent(id) => write!(f, "event identifier already accepted: {id}"), Self::SequenceOverflow => f.write_str("event sequence overflow"),
} } }
impl std::error::Error for EventBusError {}
