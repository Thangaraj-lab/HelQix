//! Ordered temporal history of a HELQIX execution.
//!
//! A [`Timeline`] records *when* a sequence of execution-relevant
//! moments occurred and *what* each moment referred to — never the
//! engineering state itself, never the physics, never the actions.
//! Those are owned by Layers 04, 05, and 06 respectively.
//!
//! # Responsibility
//!
//! `timeline.rs` is the temporal backbone of Layer 07. It answers:
//!
//! * What happened first?
//! * What happened at time `t`?
//! * What happened immediately before / after time `t`?
//! * What is the most recent event?
//! * What is the deterministic iteration order of this execution?
//!
//! # What it is not
//!
//! * Not a state store — engineering truth lives in Layer 04.
//! * Not an executor — actions are run by Layer 05.
//! * Not a physics solver — timesteps are computed by Layer 06.
//! * Not a clock — the authoritative time source is [`crate::clock::Clock`].
//! * Not a snapshot store — snapshot semantics are owned by `snapshot.rs`.
//! * Not a rewind / replay engine — those own their own navigation.
//!
//! # Storage
//!
//! Entries are stored in a single `Vec<TimelineEntry<T>>` in strict
//! non-decreasing `SimTime` order. Equal timestamps are permitted and
//! disambiguated by insertion order — this is important because real
//! engineering sequences routinely produce several consequences at the
//! same logical instant (constraint violated + failure detected +
//! state frozen at `t = 0.40s`).
//!
//! # Determinism
//!
//! The same sequence of `(SimTime, T)` insertions produces the same
//! timeline, byte-for-byte, on every platform and in every process.
//! No wall clock, no randomness, no thread timing, no global state,
//! no interior mutability, no `unsafe`.
//!
//! # Payload genericity
//!
//! [`Timeline`] is generic over the payload `T`. This file deliberately
//! commits to **no** cross-layer type. Downstream layers parameterize
//! the timeline with their own reference type — a state revision ID,
//! a snapshot ID, an event ID, an execution step ID, or any combination
//! wrapped in their own newtype. Layer 07 never invents such a type.

use core::fmt;

use crate::clock::{Clock, SimTime};

// ============================================================================
// TimelineEntry
// ============================================================================

/// A single ordered point in a HELQIX timeline.
///
/// Carries the exact logical time and a caller-supplied subject. The
/// subject is a *reference* to whatever the caller considers the
/// meaningful content of that moment — typically an ID or a handle.
/// The timeline never stores engineering state itself.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct TimelineEntry<T> {
    at: SimTime,
    subject: T,
}

impl<T> TimelineEntry<T> {
    /// The exact logical time of this entry.
    #[inline]
    #[must_use]
    pub const fn at(&self) -> SimTime {
        self.at
    }

    /// The caller-supplied subject of this entry.
    #[inline]
    #[must_use]
    pub const fn subject(&self) -> &T {
        &self.subject
    }

    /// Consume the entry and return the subject.
    #[inline]
    #[must_use]
    pub fn into_subject(self) -> T {
        self.subject
    }
}

// ============================================================================
// TimelineError
// ============================================================================

/// Errors raised by timeline insertion.
///
/// The timeline itself performs no I/O and no allocation beyond the
/// backing `Vec`; the only invariant that can be violated by a caller
/// is temporal ordering.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TimelineError {
    /// A caller attempted to insert an entry whose temporal position
    /// precedes the last entry already present.
    ///
    /// This is rejected rather than silently sorted because silently
    /// reordering would destroy the execution semantics of a replay:
    /// `recorded order == execution order` must be an invariant.
    BackwardInTime {
        /// The temporal position of the most recent entry.
        previous: SimTime,
        /// The temporal position the caller attempted to insert.
        attempted: SimTime,
    },
}

impl fmt::Display for TimelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackwardInTime { previous, attempted } => write!(
                f,
                "timeline insertion at {} precedes previous entry at {}",
                attempted, previous
            ),
        }
    }
}

impl std::error::Error for TimelineError {}

// ============================================================================
// Timeline
// ============================================================================

/// An ordered temporal history of a HELQIX execution.
///
/// See the module documentation for the full semantic model. This type
/// owns only the temporal ordering; it never owns engineering state.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Timeline<T> {
    entries: Vec<TimelineEntry<T>>,
}

impl<T> Timeline<T> {
    /// Construct an empty timeline.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Number of entries.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is the timeline empty?
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The temporal position of the most recent entry, if any.
    #[inline]
    #[must_use]
    pub fn last_time(&self) -> Option<SimTime> {
        self.entries.last().map(|e| e.at)
    }

    /// The earliest entry, if any.
    #[inline]
    #[must_use]
    pub fn first(&self) -> Option<&TimelineEntry<T>> {
        self.entries.first()
    }

    /// The most recent entry, if any.
    #[inline]
    #[must_use]
    pub fn latest(&self) -> Option<&TimelineEntry<T>> {
        self.entries.last()
    }

    /// Record a moment at an explicit [`SimTime`].
    ///
    /// The entry is appended only if `at` is greater than or equal to
    /// the temporal position of the last entry. Equal timestamps are
    /// permitted; insertion order disambiguates them. Backward
    /// insertion is rejected with [`TimelineError::BackwardInTime`]
    /// and leaves the timeline unchanged.
    ///
    /// On success, returns a reference to the freshly inserted entry.
    pub fn record_at(
        &mut self,
        at: SimTime,
        subject: T,
    ) -> Result<&TimelineEntry<T>, TimelineError> {
        if let Some(last) = self.entries.last() {
            if at < last.at {
                return Err(TimelineError::BackwardInTime {
                    previous: last.at,
                    attempted: at,
                });
            }
        }
        let idx = self.entries.len();
        self.entries.push(TimelineEntry { at, subject });
        Ok(&self.entries[idx])
    }

    /// Record a moment using the current logical time of `clock`.
    ///
    /// This method **reads** the clock; it never advances it. Temporal
    /// progression remains the sole responsibility of [`Clock`].
    ///
    /// Returns [`TimelineError::BackwardInTime`] if the clock is now
    /// behind the last recorded position — which can only happen if a
    /// clock was reset and reused to drive the same timeline, and is
    /// therefore a genuine contract violation.
    pub fn record_with(
        &mut self,
        clock: &Clock,
        subject: T,
    ) -> Result<&TimelineEntry<T>, TimelineError> {
        self.record_at(clock.now(), subject)
    }

    // ------------------------------------------------------------------
    // Temporal queries
    // ------------------------------------------------------------------

    /// The first entry whose temporal position is exactly `at`.
    ///
    /// If several entries share `at`, the earliest inserted one is
    /// returned (deterministic by construction). To retrieve *all*
    /// entries at `at`, use `range(at, at)`.
    #[inline]
    #[must_use]
    pub fn at_exact(&self, at: SimTime) -> Option<&TimelineEntry<T>> {
        let idx = self.lower_bound(at);
        self.entries.get(idx).filter(|e| e.at == at)
    }

    /// The latest entry whose temporal position is `<= at`.
    #[inline]
    #[must_use]
    pub fn at_or_before(&self, at: SimTime) -> Option<&TimelineEntry<T>> {
        let idx = self.lower_bound(at);
        if idx < self.entries.len() && self.entries[idx].at == at {
            return self.entries.get(idx);
        }
        if idx == 0 {
            None
        } else {
            self.entries.get(idx - 1)
        }
    }

    /// The earliest entry whose temporal position is `>= at`.
    #[inline]
    #[must_use]
    pub fn at_or_after(&self, at: SimTime) -> Option<&TimelineEntry<T>> {
        self.entries.get(self.lower_bound(at))
    }

    /// The latest entry whose temporal position is strictly `< at`.
    #[inline]
    #[must_use]
    pub fn before(&self, at: SimTime) -> Option<&TimelineEntry<T>> {
        let idx = self.lower_bound(at);
        if idx == 0 {
            None
        } else {
            self.entries.get(idx - 1)
        }
    }

    /// The earliest entry whose temporal position is strictly `> at`.
    #[inline]
    #[must_use]
    pub fn after(&self, at: SimTime) -> Option<&TimelineEntry<T>> {
        let idx = self.upper_bound(at);
        self.entries.get(idx)
    }

    /// All entries whose temporal position lies in the closed interval
    /// `[from, to]`, in insertion order.
    ///
    /// Returns an empty iterator if `to < from`.
    #[inline]
    pub fn range(
        &self,
        from: SimTime,
        to: SimTime,
    ) -> impl Iterator<Item = &TimelineEntry<T>> + '_ {
        let (start, end) = if to < from {
            (0, 0)
        } else {
            (self.lower_bound(from), self.upper_bound(to))
        };
        self.entries[start..end].iter()
    }

    /// Deterministic iteration in insertion (i.e. temporal) order.
    #[inline]
    pub fn iter(&self) -> core::slice::Iter<'_, TimelineEntry<T>> {
        self.entries.iter()
    }

    // ------------------------------------------------------------------
    // Binary-search helpers
    // ------------------------------------------------------------------

    /// First index with `entries[i].at >= at`.
    #[inline]
    fn lower_bound(&self, at: SimTime) -> usize {
        self.entries.partition_point(|e| e.at < at)
    }

    /// First index with `entries[i].at > at`.
    #[inline]
    fn upper_bound(&self, at: SimTime) -> usize {
        self.entries.partition_point(|e| e.at <= at)
    }
}

impl<T> Default for Timeline<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> IntoIterator for &'a Timeline<T> {
    type Item = &'a TimelineEntry<T>;
    type IntoIter = core::slice::Iter<'a, TimelineEntry<T>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl<T> IntoIterator for Timeline<T> {
    type Item = TimelineEntry<T>;
    type IntoIter = std::vec::IntoIter<TimelineEntry<T>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{SimDelta, SimTime};

    fn ns(n: i128) -> SimTime {
        SimTime::from_nanos(n)
    }

    fn delta(n: i128) -> SimDelta {
        SimDelta::from_nanos(n)
    }

    // ------------------------------------------------------------------
    // 1. Empty timeline
    // ------------------------------------------------------------------

    #[test]
    fn empty_timeline_returns_none_everywhere() {
        let tl: Timeline<u32> = Timeline::new();
        assert!(tl.is_empty());
        assert_eq!(tl.len(), 0);
        assert!(tl.first().is_none());
        assert!(tl.latest().is_none());
        assert!(tl.last_time().is_none());
        assert!(tl.at_exact(ns(0)).is_none());
        assert!(tl.at_or_before(ns(0)).is_none());
        assert!(tl.at_or_after(ns(0)).is_none());
        assert!(tl.before(ns(0)).is_none());
        assert!(tl.after(ns(0)).is_none());
        assert_eq!(tl.iter().count(), 0);
        assert_eq!(tl.range(ns(0), ns(100)).count(), 0);
    }

    // ------------------------------------------------------------------
    // 2. Chronological insertion
    // ------------------------------------------------------------------

    #[test]
    fn chronological_insertion_preserves_order() {
        let mut tl = Timeline::new();
        tl.record_at(ns(0), "boot").unwrap();
        tl.record_at(ns(10), "motor_on").unwrap();
        tl.record_at(ns(20), "velocity_changed").unwrap();
        tl.record_at(ns(30), "temperature_rose").unwrap();

        let times: Vec<i128> = tl.iter().map(|e| e.at().as_nanos()).collect();
        assert_eq!(times, vec![0, 10, 20, 30]);

        let subs: Vec<&str> = tl.iter().map(|e| *e.subject()).collect();
        assert_eq!(subs, vec!["boot", "motor_on", "velocity_changed", "temperature_rose"]);
    }

    // ------------------------------------------------------------------
    // 3. Backward insertion is rejected and does not mutate
    // ------------------------------------------------------------------

    #[test]
    fn backward_insertion_is_rejected_and_leaves_state_unchanged() {
        let mut tl = Timeline::new();
        tl.record_at(ns(10), "a").unwrap();
        tl.record_at(ns(20), "b").unwrap();

        let before_len = tl.len();
        let err = tl.record_at(ns(15), "c").unwrap_err();
        assert_eq!(
            err,
            TimelineError::BackwardInTime {
                previous: ns(20),
                attempted: ns(15),
            }
        );
        assert_eq!(tl.len(), before_len);
        assert_eq!(*tl.latest().unwrap().subject(), "b");
    }

    // ------------------------------------------------------------------
    // 4. Equal timestamps preserved with insertion-order semantics
    // ------------------------------------------------------------------

    #[test]
    fn equal_timestamps_preserve_insertion_order() {
        // Mirrors the HELQIX worked example: constraint violation,
        // failure detection, and state freeze all at t = 0.40s.
        let t = ns(400_000_000);
        let mut tl = Timeline::new();
        tl.record_at(t, "constraint_violated").unwrap();
        tl.record_at(t, "failure_detected").unwrap();
        tl.record_at(t, "state_frozen").unwrap();

        let subs: Vec<&str> = tl.iter().map(|e| *e.subject()).collect();
        assert_eq!(
            subs,
            vec!["constraint_violated", "failure_detected", "state_frozen"]
        );

        // at_exact returns the FIRST entry at that instant.
        assert_eq!(*tl.at_exact(t).unwrap().subject(), "constraint_violated");

        // range(t, t) returns all entries at that instant.
        let at_t: Vec<&str> = tl.range(t, t).map(|e| *e.subject()).collect();
        assert_eq!(
            at_t,
            vec!["constraint_violated", "failure_detected", "state_frozen"]
        );
    }

    // ------------------------------------------------------------------
    // 5. First / latest
    // ------------------------------------------------------------------

    #[test]
    fn first_and_latest_are_stable() {
        let mut tl = Timeline::new();
        tl.record_at(ns(0), "first").unwrap();
        tl.record_at(ns(50), "middle").unwrap();
        tl.record_at(ns(100), "last").unwrap();

        assert_eq!(*tl.first().unwrap().subject(), "first");
        assert_eq!(tl.first().unwrap().at(), ns(0));
        assert_eq!(*tl.latest().unwrap().subject(), "last");
        assert_eq!(tl.latest().unwrap().at(), ns(100));
        assert_eq!(tl.last_time(), Some(ns(100)));
    }

    // ------------------------------------------------------------------
    // 6. at_exact
    // ------------------------------------------------------------------

    #[test]
    fn at_exact_is_exact() {
        let mut tl = Timeline::new();
        tl.record_at(ns(0), "a").unwrap();
        tl.record_at(ns(10), "b").unwrap();
        tl.record_at(ns(20), "c").unwrap();

        assert_eq!(*tl.at_exact(ns(10)).unwrap().subject(), "b");
        assert!(tl.at_exact(ns(5)).is_none());
        assert!(tl.at_exact(ns(15)).is_none());
        assert!(tl.at_exact(ns(25)).is_none());
    }

    // ------------------------------------------------------------------
    // 7. at_or_before / at_or_after (inclusive bounds)
    // ------------------------------------------------------------------

    #[test]
    fn at_or_before_and_at_or_after_include_boundaries() {
        let mut tl = Timeline::new();
        tl.record_at(ns(10), "a").unwrap();
        tl.record_at(ns(20), "b").unwrap();
        tl.record_at(ns(30), "c").unwrap();

        assert!(tl.at_or_before(ns(5)).is_none());
        assert_eq!(*tl.at_or_before(ns(10)).unwrap().subject(), "a");
        assert_eq!(*tl.at_or_before(ns(25)).unwrap().subject(), "b");
        assert_eq!(*tl.at_or_before(ns(30)).unwrap().subject(), "c");
        assert_eq!(*tl.at_or_before(ns(1000)).unwrap().subject(), "c");

        assert!(tl.at_or_after(ns(1000)).is_none());
        assert_eq!(*tl.at_or_after(ns(30)).unwrap().subject(), "c");
        assert_eq!(*tl.at_or_after(ns(25)).unwrap().subject(), "c");
        assert_eq!(*tl.at_or_after(ns(10)).unwrap().subject(), "a");
        assert_eq!(*tl.at_or_after(ns(0)).unwrap().subject(), "a");
    }

    // ------------------------------------------------------------------
    // 8. before / after (strict bounds)
    // ------------------------------------------------------------------

    #[test]
    fn before_and_after_are_strict() {
        let mut tl = Timeline::new();
        tl.record_at(ns(10), "a").unwrap();
        tl.record_at(ns(20), "b").unwrap();
        tl.record_at(ns(30), "c").unwrap();

        assert!(tl.before(ns(10)).is_none());
        assert_eq!(*tl.before(ns(20)).unwrap().subject(), "a");
        assert_eq!(*tl.before(ns(100)).unwrap().subject(), "c");

        assert!(tl.after(ns(30)).is_none());
        assert_eq!(*tl.after(ns(20)).unwrap().subject(), "c");
        assert_eq!(*tl.after(ns(0)).unwrap().subject(), "a");
    }

    #[test]
    fn before_and_after_skip_ties() {
        let t = ns(100);
        let mut tl = Timeline::new();
        tl.record_at(t, "x1").unwrap();
        tl.record_at(t, "x2").unwrap();
        tl.record_at(t, "x3").unwrap();
        tl.record_at(ns(200), "y").unwrap();

        // Strictly before t: nothing, since the earliest entry is at t.
        assert!(tl.before(t).is_none());
        // Strictly after t: y.
        assert_eq!(*tl.after(t).unwrap().subject(), "y");
    }

    // ------------------------------------------------------------------
    // 9. range query
    // ------------------------------------------------------------------

    #[test]
    fn range_query_is_inclusive_on_both_ends() {
        let mut tl = Timeline::new();
        for i in [10i128, 20, 30, 40, 50] {
            tl.record_at(ns(i), i).unwrap();
        }

        let got: Vec<i128> = tl.range(ns(20), ns(40)).map(|e| *e.subject()).collect();
        assert_eq!(got, vec![20, 30, 40]);

        let none: Vec<i128> = tl.range(ns(41), ns(49)).map(|e| *e.subject()).collect();
        assert!(none.is_empty());

        // Reversed range → empty (no panic).
        let rev: Vec<i128> = tl.range(ns(40), ns(20)).map(|e| *e.subject()).collect();
        assert!(rev.is_empty());
    }

    // ------------------------------------------------------------------
    // 10. Independent instances
    // ------------------------------------------------------------------

    #[test]
    fn timelines_are_independent() {
        let mut a = Timeline::new();
        let mut b = Timeline::new();
        a.record_at(ns(0), "a0").unwrap();
        a.record_at(ns(1), "a1").unwrap();
        b.record_at(ns(0), "b0").unwrap();

        assert_eq!(a.len(), 2);
        assert_eq!(b.len(), 1);
        assert_eq!(*b.latest().unwrap().subject(), "b0");
    }

    // ------------------------------------------------------------------
    // 11. Determinism: identical insertion sequence → identical timeline
    // ------------------------------------------------------------------

    #[test]
    fn identical_insertion_sequence_produces_identical_timeline() {
        let build = || {
            let mut tl = Timeline::new();
            tl.record_at(ns(0), "boot").unwrap();
            tl.record_at(ns(10), "motor_on").unwrap();
            tl.record_at(ns(20), "velocity_changed").unwrap();
            tl.record_at(ns(30), "temperature_rose").unwrap();
            tl.record_at(ns(40), "constraint_violated").unwrap();
            tl.record_at(ns(40), "failure_detected").unwrap();
            tl
        };

        let t1 = build();
        let t2 = build();
        let t3 = build();

        // Structural equality.
        assert_eq!(t1, t2);
        assert_eq!(t2, t3);

        // Observable iteration equality.
        let sig = |t: &Timeline<&'static str>| -> Vec<(i128, &'static str)> {
            t.iter().map(|e| (e.at().as_nanos(), *e.subject())).collect()
        };
        assert_eq!(sig(&t1), sig(&t2));
        assert_eq!(sig(&t2), sig(&t3));
    }

    // ------------------------------------------------------------------
    // 12. Clock integration (record_with)
    // ------------------------------------------------------------------

    #[test]
    fn record_with_reads_clock_without_advancing_it() {
        let mut clock = Clock::new();
        let mut tl = Timeline::new();

        tl.record_with(&clock, "at_zero").unwrap();
        assert_eq!(clock.now(), ns(0));

        clock.advance_by(delta(10)).unwrap();
        tl.record_with(&clock, "at_ten").unwrap();
        assert_eq!(clock.now(), ns(10)); // unchanged by recording

        clock.advance_by(delta(10)).unwrap();
        tl.record_with(&clock, "at_twenty").unwrap();

        let times: Vec<i128> = tl.iter().map(|e| e.at().as_nanos()).collect();
        assert_eq!(times, vec![0, 10, 20]);
    }

    #[test]
    fn record_with_rejects_backward_clock_use() {
        // Simulates the misuse case: a clock is reset and reused to
        // drive a timeline that has already recorded forward.
        let mut clock = Clock::new();
        let mut tl = Timeline::new();

        clock.advance_by(delta(100)).unwrap();
        tl.record_with(&clock, "forward").unwrap();

        clock.reset(); // clock back to origin
        let err = tl.record_with(&clock, "should_fail").unwrap_err();
        match err {
            TimelineError::BackwardInTime { previous, attempted } => {
                assert_eq!(previous, ns(100));
                assert_eq!(attempted, ns(0));
            }
        }
        assert_eq!(tl.len(), 1);
    }

    // ------------------------------------------------------------------
    // 13. Boundary conditions
    // ------------------------------------------------------------------

    #[test]
    fn boundary_min_and_max_times_are_handled() {
        let mut tl = Timeline::new();
        tl.record_at(SimTime::MIN, "min").unwrap();
        tl.record_at(SimTime::MAX, "max").unwrap();

        assert_eq!(*tl.first().unwrap().subject(), "min");
        assert_eq!(*tl.latest().unwrap().subject(), "max");
        assert!(tl.before(SimTime::MIN).is_none());
        assert!(tl.after(SimTime::MAX).is_none());
    }

    // ------------------------------------------------------------------
    // 14. Historical integrity
    // ------------------------------------------------------------------

    #[test]
    fn historical_entries_cannot_be_silently_reordered() {
        let mut tl = Timeline::new();
        tl.record_at(ns(0), "a").unwrap();
        tl.record_at(ns(10), "b").unwrap();
        tl.record_at(ns(20), "c").unwrap();
        tl.record_at(ns(20), "c2").unwrap();

        // Attempt an out-of-order insert that would, if silently
        // sorted, land between a and b.
        assert!(tl.record_at(ns(5), "intruder").is_err());

        // The original sequence is untouched.
        let sig: Vec<(i128, &str)> =
            tl.iter().map(|e| (e.at().as_nanos(), *e.subject())).collect();
        assert_eq!(
            sig,
            vec![
                (0, "a"),
                (10, "b"),
                (20, "c"),
                (20, "c2"),
            ]
        );
    }

    // ------------------------------------------------------------------
    // 15. Failure-timestamp scenario (worked example from the spec)
    // ------------------------------------------------------------------

    #[test]
    fn worked_failure_scenario_orders_correctly() {
        // 0.00 s  initial
        // 0.10 s  motor action
        // 0.20 s  velocity changes
        // 0.30 s  temperature increases
        // 0.40 s  thermal constraint violated
        // 0.40 s  failure detected
        // 0.40 s  exact state frozen
        let mut tl = Timeline::new();
        let secs = |s: i128| SimTime::from_secs(s).unwrap();
        let ms = |m: i128| SimTime::from_millis(m).unwrap();

        tl.record_at(secs(0), "initial").unwrap();
        tl.record_at(ms(100), "motor_action").unwrap();
        tl.record_at(ms(200), "velocity_changed").unwrap();
        tl.record_at(ms(300), "temperature_rose").unwrap();
        tl.record_at(ms(400), "constraint_violated").unwrap();
        tl.record_at(ms(400), "failure_detected").unwrap();
        tl.record_at(ms(400), "state_frozen").unwrap();

        // Rewind target: what happened immediately before failure?
        let before = tl.before(ms(400)).unwrap();
        assert_eq!(*before.subject(), "temperature_rose");
        assert_eq!(before.at(), ms(300));

        // Everything at the failure instant.
        let at_failure: Vec<&str> =
            tl.range(ms(400), ms(400)).map(|e| *e.subject()).collect();
        assert_eq!(
            at_failure,
            vec!["constraint_violated", "failure_detected", "state_frozen"]
        );
    }

    // ------------------------------------------------------------------
    // 16. Iteration determinism
    // ------------------------------------------------------------------

    #[test]
    fn iteration_order_is_deterministic() {
        let mut tl = Timeline::new();
        for i in 0..100i128 {
            tl.record_at(ns(i), i).unwrap();
        }
        let first_pass: Vec<i128> = tl.iter().map(|e| *e.subject()).collect();
        let second_pass: Vec<i128> = tl.iter().map(|e| *e.subject()).collect();
        assert_eq!(first_pass, second_pass);
        assert_eq!(first_pass, (0..100i128).collect::<Vec<_>>());

        // IntoIterator for &Timeline agrees.
        let third_pass: Vec<i128> = (&tl).into_iter().map(|e| *e.subject()).collect();
        assert_eq!(first_pass, third_pass);
    }

    // ------------------------------------------------------------------
    // 17. Owned iteration
    // ------------------------------------------------------------------

    #[test]
    fn owned_iteration_yields_subjects() {
        let mut tl = Timeline::new();
        tl.record_at(ns(0), String::from("a")).unwrap();
        tl.record_at(ns(10), String::from("b")).unwrap();

        let owned: Vec<String> = tl.into_iter().map(|e| e.into_subject()).collect();
        assert_eq!(owned, vec!["a".to_string(), "b".to_string()]);
    }

    // ------------------------------------------------------------------
    // 18. Record returns the entry just inserted
    // ------------------------------------------------------------------

    #[test]
    fn record_at_returns_the_inserted_entry() {
        let mut tl = Timeline::new();
        let entry = tl.record_at(ns(42), "hello").unwrap();
        assert_eq!(entry.at(), ns(42));
        assert_eq!(*entry.subject(), "hello");
    }
}