//! Deterministic event publication and dispatch for HELQIX Layer 10.
//!
//! [`EventBus`] connects event producers (execution, state transitions,
//! triggers, failure detection, ...) with consumers (observation recording,
//! causal analysis, verification, UI projections):
//!
//! ```text
//! Producer ──publish──▶ Event Bus ──▶ Subscriber #1 (causality)
//!                                 └──▶ Subscriber #2 (verification) ...
//! ```
//!
//! # Semantics (normative)
//!
//! - **Synchronous & in-line.** `publish` delivers to every matching
//!   subscriber before returning. No threads, no async runtime, no queues —
//!   delivery order equals registration order (subscribers get monotonically
//!   increasing ids), which is the deterministic tie-break the platform
//!   requires. Never any hash-map iteration order.
//! - **Recorded history.** Every accepted event is appended to an in-order
//!   journal ([`RecordedEvent`]) carrying a bus-local sequence number. The
//!   journal *is* the replayable execution history: identical publish
//!   sequences produce identical journals.
//! - **Isolated subscriber failure.** A failing subscriber does not stop
//!   later subscribers and does not roll back the journal; its error is
//!   captured per-subscriber and returned inside [`DispatchReport`] so it is
//!   never silently swallowed. Callers decide how to react.
//! - **Duplicate policy.** Re-publishing an already recorded [`EventId`] is
//!   rejected by default ([`DuplicatePolicy::Reject`]) because historical
//!   events are immutable facts; the lenient policies are explicit opt-in.
//!   Duplicate publications are *never* dispatched twice under any policy.
//! - **No global state.** The bus is an ordinary value owned by its caller;
//!   there is no singleton, registry or thread-local anywhere.
//!
//! This module performs no causal reasoning (that is `helqix-causal`) and
//! owns no engineering state (Layer 04).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::event::{Event, EventId, EventKind, LogicalTime, Severity};

/// Errors produced by bus operations.
#[derive(Debug, Clone, PartialEq)]
pub enum BusError {
    /// An event with the same id was already recorded and the duplicate
    /// policy rejects it.
    DuplicateEvent {
        /// The offending id.
        id: EventId,
    },
}

impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateEvent { id } => {
                write!(f, "event `{id}` is already recorded; ids are unique facts")
            }
        }
    }
}

impl std::error::Error for BusError {}

/// Behaviour when publishing an event whose id was already recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DuplicatePolicy {
    /// Reject the publication (default: history is immutable).
    Reject,
    /// Silently ignore the publication (still never dispatched again).
    Ignore,
    /// Accept only if the new event is structurally identical to the
    /// recorded one (idempotent re-publication); otherwise reject.
    AllowIdentical,
}

/// Stable subscriber handle issued by [`EventBus::subscribe`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriberId(u64);

impl SubscriberId {
    /// Returns the numeric handle.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for SubscriberId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "subscriber#{}", self.0)
    }
}

/// Declarative subscription filter.
///
/// Kept as data (not closures) so subscriptions are comparable, printable
/// and reproducible across replays.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EventFilter {
    /// Receive every published event.
    #[default]
    All,
    /// Receive events of exactly these kinds (deterministic set semantics).
    Kinds(BTreeSet<EventKind>),
    /// Receive events at or above this severity.
    MinSeverity(Severity),
    /// Receive events matching both constraints.
    KindAndSeverity {
        /// Accepted kinds.
        kinds: BTreeSet<EventKind>,
        /// Minimum severity.
        min_severity: Severity,
    },
}

impl EventFilter {
    /// Returns `true` when `event` passes this filter.
    #[must_use]
    pub fn matches(&self, event: &Event) -> bool {
        match self {
            Self::All => true,
            Self::Kinds(kinds) => kinds.contains(event.kind()),
            Self::MinSeverity(min) => event.severity() >= *min,
            Self::KindAndSeverity {
                kinds,
                min_severity,
            } => kinds.contains(event.kind()) && event.severity() >= *min_severity,
        }
    }

    /// Convenience constructor for a fixed kind set.
    #[must_use]
    pub fn kinds(iter: impl IntoIterator<Item = EventKind>) -> Self {
        Self::Kinds(iter.into_iter().collect())
    }
}

/// An event together with its immutable bus-delivery metadata.
///
/// The wrapped [`Event`] itself is never modified; the sequence number is
/// assigned once, at recording time.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordedEvent {
    event: Event,
    sequence: u64,
}

impl RecordedEvent {
    /// Returns the underlying event.
    #[must_use]
    pub const fn event(&self) -> &Event {
        &self.event
    }

    /// Returns the bus-local delivery sequence (dense, starting at 0).
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
}

/// Per-subscriber outcome of one dispatch round.
#[derive(Debug, Clone, PartialEq)]
pub struct SubscriberOutcome {
    /// Subscriber that was invoked.
    pub subscriber: SubscriberId,
    /// Human-readable subscriber label.
    pub name: String,
    /// Number of events delivered to it during this publish call (0 or 1).
    pub delivered: usize,
    /// Captured errors, in invocation order (at most one per publish).
    pub errors: Vec<String>,
}

/// Summary returned by [`EventBus::publish`].
#[derive(Debug, Clone, PartialEq)]
pub struct DispatchReport {
    /// Whether the event was accepted into the journal.
    pub accepted: bool,
    /// Journal sequence assigned to the event (`None` when rejected).
    pub sequence: Option<u64>,
    /// Outcomes of all matching subscribers, in registration order.
    pub outcomes: Vec<SubscriberOutcome>,
}

impl DispatchReport {
    /// Total number of deliveries performed.
    #[must_use]
    pub fn delivered_total(&self) -> usize {
        self.outcomes.iter().map(|o| o.delivered).sum()
    }

    /// Total number of captured subscriber errors.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.outcomes.iter().map(|o| o.errors.len()).sum()
    }
}

struct Subscription {
    id: SubscriberId,
    name: String,
    filter: EventFilter,
    handler: Box<dyn FnMut(&RecordedEvent) -> Result<(), String>>,
}

/// Deterministic, synchronous, owner-managed event bus.
///
/// See the [module docs](self) for the normative semantics. Not `Sync`:
/// sharing a bus across threads would reintroduce scheduling-dependent
/// ordering, which the determinism contract forbids.
pub struct EventBus {
    next_subscriber: u64,
    subscribers: Vec<Subscription>,
    by_id: BTreeMap<u64, usize>, // subscriber id -> index in `subscribers`
    journal: Vec<RecordedEvent>,
    index: BTreeMap<EventId, u64>, // event id -> sequence
    duplicates: DuplicatePolicy,
    clock: Option<LogicalTime>, // last observed logical time (monotonic guard)
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Creates an empty bus with the default [`DuplicatePolicy::Reject`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_subscriber: 0,
            subscribers: Vec::new(),
            by_id: BTreeMap::new(),
            journal: Vec::new(),
            index: BTreeMap::new(),
            duplicates: DuplicatePolicy::Reject,
            clock: None,
        }
    }

    /// Selects the duplicate-id policy.
    pub fn set_duplicate_policy(&mut self, policy: DuplicatePolicy) {
        self.duplicates = policy;
    }

    /// Registers a subscriber; returns its deterministic handle.
    ///
    /// Handlers receive `&RecordedEvent` and report failures through
    /// `Err(String)`; failures are isolated per subscriber (see module docs).
    pub fn subscribe<F>(
        &mut self,
        name: impl Into<String>,
        filter: EventFilter,
        mut handler: F,
    ) -> SubscriberId
    where
        F: FnMut(&RecordedEvent) -> Result<(), String> + 'static,
    {
        let id = SubscriberId(self.next_subscriber);
        self.next_subscriber += 1;
        let idx = self.subscribers.len();
        self.subscribers.push(Subscription {
            id,
            name: name.into(),
            filter,
            handler: Box::new(move |rec| handler(rec)),
        });
        self.by_id.insert(id.as_u64(), idx);
        id
    }

    /// Removes a subscriber; returns `false` when the handle is unknown.
    ///
    /// Existing journal entries are untouched (history is immutable).
    /// Remaining subscribers keep their original relative order.
    pub fn unsubscribe(&mut self, id: SubscriberId) -> bool {
        let Some(pos) = self.by_id.remove(&id.as_u64()) else {
            return false;
        };
        self.subscribers.remove(pos);
        // Shift indices after removal (Vec::remove preserves order).
        for (_, slot) in self.by_id.iter_mut() {
            if *slot > pos {
                *slot -= 1;
            }
        }
        true
    }

    /// Publishes an event: records it in the journal, then dispatches it
    /// synchronously to every matching subscriber in registration order.
    ///
    /// # Monotonic-time guard
    ///
    /// Publishing an event whose [`LogicalTime`] precedes the last recorded
    /// event violates the bus's ordering contract and would corrupt replay
    /// determinism; such publications are rejected like duplicates. Late
    /// arrivals belong on a merge/replay API owned by higher layers, not a
    /// silently reordered journal.
    #[must_use]
    pub fn publish(&mut self, event: Event) -> DispatchReport {
        // 1. Duplicate / ordering validation *before* any side effect.
        if let Some(seq) = self.index.get(event.id()).copied() {
            let identical = matches!(self.duplicates, DuplicatePolicy::AllowIdentical)
                && self.journal[seq as usize].event() == &event;
            // Ignore and idempotent AllowIdentical report the existing journal
            // sequence; Reject (and conflicting AllowIdentical) report none.
            let reported_sequence = match self.duplicates {
                DuplicatePolicy::Ignore => Some(seq),
                DuplicatePolicy::AllowIdentical if identical => Some(seq),
                _ => None,
            };
            return DispatchReport {
                accepted: false,
                sequence: reported_sequence,
                outcomes: Vec::new(),
            };
        }
        if let Some(last) = self.clock {
            if event.time() < last {
                return DispatchReport {
                    accepted: false,
                    sequence: None,
                    outcomes: Vec::new(),
                };
            }
        }

        // 2. Record.
        let sequence = self.journal.len() as u64;
        let recorded = RecordedEvent {
            event: event.clone(),
            sequence,
        };
        self.index.insert(event.id().clone(), sequence);
        self.clock = Some(event.time());
        self.journal.push(recorded);

        // 3. Dispatch in registration order (Vec iteration, never a map).
        // The canonical record lives in the journal; dispatch borrows it there.
        let recorded = &self.journal[sequence as usize];
        let mut outcomes = Vec::new();
        for sub in self.subscribers.iter_mut() {
            if !sub.filter.matches(&recorded.event) {
                continue;
            }
            let result = (sub.handler)(recorded);
            let errors = match result {
                Ok(()) => Vec::new(),
                Err(msg) => vec![msg],
            };
            outcomes.push(SubscriberOutcome {
                subscriber: sub.id,
                name: sub.name.clone(),
                delivered: 1,
                errors,
            });
        }

        DispatchReport {
            accepted: true,
            sequence: Some(sequence),
            outcomes,
        }
    }

    /// Returns the recorded journal in deterministic sequence order.
    #[must_use]
    pub fn journal(&self) -> &[RecordedEvent] {
        &self.journal
    }

    /// Returns a recorded event by id, if present.
    #[must_use]
    pub fn find(&self, id: &EventId) -> Option<&RecordedEvent> {
        self.index.get(id).map(|seq| &self.journal[*seq as usize])
    }

    /// Returns the number of registered subscribers.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    /// Returns the names of registered subscribers in registration order.
    #[must_use]
    pub fn subscriber_names(&self) -> Vec<&str> {
        self.subscribers.iter().map(|s| s.name.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EntityRef, PayloadValue};

    fn evt(id: &str, kind: EventKind, sev: Severity, t: u64) -> Event {
        Event::builder(kind, LogicalTime::from_nanos(t))
            .id(EventId::new(id).unwrap())
            .source(EntityRef::component("MotorA").unwrap())
            .severity(sev)
            .with_field("v", PayloadValue::real(1.0).unwrap())
            .build()
            .unwrap()
    }

    #[test]
    fn publish_records_and_delivers_to_subscriber() {
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let s = std::rc::Rc::clone(&seen);
        let mut bus = EventBus::new();
        bus.subscribe("sink", EventFilter::All, move |rec| {
            s.borrow_mut().push(format!("sink:{}", rec.event().id()));
            Ok(())
        });
        let report = bus.publish(evt("e1", EventKind::StateChanged, Severity::Info, 1));
        assert!(report.accepted);
        assert_eq!(report.sequence, Some(0));
        assert_eq!(*seen.borrow(), vec!["sink:e1".to_string()]);
        assert_eq!(bus.journal().len(), 1);
    }

    #[test]
    fn multiple_subscribers_receive_in_registration_order() {
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut bus = EventBus::new();
        for name in ["first", "second", "third"] {
            let s = std::rc::Rc::clone(&seen);
            let label = name.to_owned();
            bus.subscribe(
                label.clone(),
                EventFilter::All,
                move |rec| {
                    s.borrow_mut().push(format!("{label}:{}", rec.event().id()));
                    Ok(())
                },
            );
        }
        bus.publish(evt("e1", EventKind::StateChanged, Severity::Info, 1));
        assert_eq!(
            *seen.borrow(),
            vec!["first:e1".to_string(), "second:e1".to_string(), "third:e1".to_string()]
        );
        // Report outcomes also follow registration order deterministically.
        let r = bus.publish(evt("e2", EventKind::StateChanged, Severity::Info, 2));
        let ids: Vec<_> = r.outcomes.iter().map(|o| o.subscriber.as_u64()).collect();
        assert_eq!(ids, vec![0, 1, 2]);
    }

    #[test]
    fn filters_select_events_deterministically() {
        let mut bus = EventBus::new();
        bus.subscribe(
            "failures-only",
            EventFilter::kinds([EventKind::FailureDetected]),
            |r| {
                assert_eq!(r.event().kind(), &EventKind::FailureDetected);
                Ok(())
            },
        );
        bus.subscribe(
            "criticals",
            EventFilter::MinSeverity(Severity::Critical),
            |r| {
                assert!(r.event().severity() >= Severity::Critical);
                Ok(())
            },
        );
        let r1 = bus.publish(evt("warn", EventKind::ThresholdCrossed, Severity::Warning, 1));
        assert_eq!(r1.outcomes.len(), 0, "no subscriber matches");
        let r2 = bus.publish(evt("fail", EventKind::FailureDetected, Severity::Critical, 2));
        assert_eq!(r2.outcomes.len(), 2, "both filters match a critical failure");
    }

    #[test]
    fn subscriber_failure_is_isolated_and_reported() {
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let s = std::rc::Rc::clone(&seen);
        let mut bus = EventBus::new();
        bus.subscribe("boom", EventFilter::All, |_| Err("subscriber exploded".into()));
        bus.subscribe("after", EventFilter::All, move |rec| {
            s.borrow_mut().push(format!("after:{}", rec.event().id()));
            Ok(())
        });
        let report = bus.publish(evt("e1", EventKind::StateChanged, Severity::Info, 1));
        assert!(report.accepted, "journal still records");
        assert_eq!(report.error_count(), 1, "error surfaced, not swallowed");
        assert_eq!(
            report.outcomes[0].errors,
            vec!["subscriber exploded".to_string()]
        );
        assert_eq!(
            *seen.borrow(),
            vec!["after:e1".to_string()],
            "later subscribers still run"
        );
    }

    #[test]
    fn no_subscribers_still_records_history() {
        let mut bus = EventBus::new();
        let report = bus.publish(evt("e1", EventKind::StateChanged, Severity::Info, 1));
        assert!(report.accepted);
        assert!(report.outcomes.is_empty());
        assert_eq!(bus.journal().len(), 1);
    }

    #[test]
    fn duplicate_publication_policies() {
        let e = evt("dup", EventKind::StateChanged, Severity::Info, 1);

        // Reject (default).
        let mut bus = EventBus::new();
        let first = bus.publish(e.clone());
        assert!(first.accepted);
        let second = bus.publish(e.clone());
        assert!(!second.accepted);
        assert_eq!(second.sequence, None);
        assert_eq!(bus.journal().len(), 1);

        // Ignore.
        let mut bus2 = EventBus::new();
        bus2.set_duplicate_policy(DuplicatePolicy::Ignore);
        bus2.publish(e.clone());
        let r = bus2.publish(e.clone());
        assert!(!r.accepted);
        assert_eq!(r.sequence, Some(0), "reports existing sequence");

        // AllowIdentical: identical content is idempotent, conflicting is rejected.
        let mut bus3 = EventBus::new();
        bus3.set_duplicate_policy(DuplicatePolicy::AllowIdentical);
        assert!(bus3.publish(e.clone()).accepted);
        let idem = bus3.publish(e.clone());
        assert!(
            !idem.accepted && idem.sequence == Some(0),
            "identical re-publish is a no-op"
        );
        let conflict = evt("dup", EventKind::StateChanged, Severity::Info, 5);
        let bad = bus3.publish(conflict);
        assert!(
            !bad.accepted && bad.sequence.is_none(),
            "same id different content rejected"
        );
        assert_eq!(bus3.journal().len(), 1);
    }

    #[test]
    fn out_of_order_time_is_rejected() {
        let mut bus = EventBus::new();
        assert!(bus
            .publish(evt("ok", EventKind::StateChanged, Severity::Info, 10))
            .accepted);
        let late = bus.publish(evt("late_no", EventKind::StateChanged, Severity::Info, 9));
        assert!(!late.accepted, "logical time must be monotonic on the bus");
        assert_eq!(bus.journal().len(), 1);
    }

    #[test]
    fn repeated_identical_publish_sequence_yields_identical_journal() {
        fn build() -> (Vec<(u64, Event)>, Vec<String>) {
            let mut bus = EventBus::new();
            let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            let s = std::rc::Rc::clone(&seen);
            bus.subscribe("log", EventFilter::All, move |r| {
                s.borrow_mut().push(r.event().id().as_str().to_owned());
                Ok(())
            });
            for i in 0..50u64 {
                bus.publish(evt(
                    &format!("evt_{i:03}"),
                    EventKind::SimulationStepCompleted,
                    Severity::Debug,
                    i,
                ));
            }
            let journal = bus
                .journal()
                .iter()
                .map(|r| (r.sequence(), r.event().clone()))
                .collect::<Vec<_>>();
            let delivered = seen.borrow().clone();
            (journal, delivered)
        }
        let (j1, d1) = build();
        let (j2, d2) = build();
        assert_eq!(j1, j2, "journal reproducible");
        assert_eq!(d1, d2, "delivery order reproducible");
    }

    #[test]
    fn unsubscribe_removes_only_that_subscriber_preserving_order() {
        let mut bus = EventBus::new();
        bus.subscribe("a", EventFilter::All, |_| Ok(()));
        let b = bus.subscribe("b", EventFilter::All, |_| Ok(()));
        bus.subscribe("c", EventFilter::All, |_| Ok(()));
        assert!(bus.unsubscribe(b));
        assert!(!bus.unsubscribe(b), "double-unsubscribe reports unknown handle");
        let r = bus.publish(evt("x", EventKind::StateChanged, Severity::Info, 1));
        let names: Vec<_> = r.outcomes.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, vec!["a", "c"]);
        assert_eq!(bus.subscriber_count(), 2);
    }

    #[test]
    fn find_returns_recorded_history_by_id() {
        let mut bus = EventBus::new();
        bus.publish(evt(
            "need",
            EventKind::ObservationGenerated,
            Severity::Info,
            3,
        ));
        let rec = bus.find(&EventId::new("need").unwrap()).expect("present");
        assert_eq!(rec.event().time(), LogicalTime::from_nanos(3));
        assert_eq!(rec.sequence(), 0);
        assert!(bus.find(&EventId::new("absent").unwrap()).is_none());
    }

    #[test]
    fn filter_kind_set_uses_canonical_kind_names() {
        let kind: EventKind = "failure.detected".parse().unwrap();
        let mut bus = EventBus::new();
        bus.subscribe("f", EventFilter::kinds([kind]), |_| Ok(()));
        let r = bus.publish(evt("y", EventKind::FailureDetected, Severity::Critical, 1));
        assert_eq!(r.outcomes.len(), 1);
    }
}
