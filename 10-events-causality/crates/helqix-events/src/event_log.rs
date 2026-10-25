//! Append-only, time-ordered record of what happened in a run.
use std::collections::BTreeSet;

use helqix_core::id::Identifier;
use helqix_time::SimTime;

use crate::event::Event;

/// Events in the order they happened. Nothing can be removed, reordered or replaced.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventLog { events: Vec<Event>, ids: BTreeSet<Identifier> }

/// Rejected append.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventLogError {
    /// The event is earlier than the last recorded event.
    OutOfOrder { last: SimTime, event_at: SimTime },
    /// An event with this id is already recorded.
    DuplicateId(Identifier),
}

impl core::fmt::Display for EventLogError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutOfOrder { last, event_at } => write!(f, "event at {event_at:?} is earlier than the last recorded event at {last:?}"),
            Self::DuplicateId(id) => write!(f, "event {} is already recorded", id.as_str()),
        }
    }
}

impl std::error::Error for EventLogError {}

impl EventLog {
    /// Empty log.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Appends an event that is not earlier than the last one; returns its sequence number.
    pub fn append(&mut self, event: Event) -> Result<usize, EventLogError> {
        if let Some(last) = self.events.last() {
            if event.at < last.at { return Err(EventLogError::OutOfOrder { last: last.at, event_at: event.at }); }
        }
        if !self.ids.insert(event.id.clone()) { return Err(EventLogError::DuplicateId(event.id)); }
        self.events.push(event);
        Ok(self.events.len() - 1)
    }

    /// All events, in order.
    #[must_use]
    pub fn events(&self) -> &[Event] { &self.events }

    /// Events whose source is `source`, in order.
    pub fn from_source<'a>(&'a self, source: &'a Identifier) -> impl Iterator<Item = &'a Event> + 'a {
        self.events.iter().filter(move |e| &e.source == source)
    }

    /// Number of events.
    #[must_use]
    pub fn len(&self) -> usize { self.events.len() }

    /// Whether no events are recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.events.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(v: &str) -> Identifier { Identifier::parse(v).unwrap() }
    fn ev(name: &str, source: &str, secs: i128) -> Event { Event::new(id(name), id("onset"), id(source), SimTime::from_secs(secs).unwrap()) }

    #[test]
    fn appends_in_time_order_and_rejects_going_back() {
        let mut log = EventLog::new();
        assert_eq!(log.append(ev("e1", "a", 1)).unwrap(), 0);
        assert_eq!(log.append(ev("e2", "b", 1)).unwrap(), 1);
        assert!(matches!(log.append(ev("e3", "a", 0)), Err(EventLogError::OutOfOrder { .. })));
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn rejects_duplicate_ids_and_filters_by_source() {
        let mut log = EventLog::new();
        log.append(ev("e1", "a", 1)).unwrap();
        assert_eq!(log.append(ev("e1", "b", 2)), Err(EventLogError::DuplicateId(id("e1"))));
        log.append(ev("e2", "b", 2)).unwrap();
        assert_eq!(log.from_source(&id("b")).count(), 1);
    }
}
