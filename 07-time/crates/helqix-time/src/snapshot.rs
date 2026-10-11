//! Temporal snapshots for HELQIX.
//!
//! A [`TemporalSnapshot<S>`] binds an *exact logical time* to an
//! *opaque state payload*. It is the Layer 07 temporal coordinate of a
//! snapshot; the state payload `S` is supplied by the caller and is
//! typically a Layer 04 state identity, state handle, or state snapshot
//! reference.
//!
//! # Responsibility boundary
//!
//! This file deliberately owns **only**:
//!
//! * the exact [`SimTime`] at which the snapshot was taken, and
//! * a borrow-safe, immutable wrapper around a caller-supplied `S`.
//!
//! It deliberately does **not** own:
//!
//! * engineering state (Layer 04),
//! * state snapshot storage (Layer 04),
//! * physical consequences (Layer 06),
//! * action execution (Layer 05),
//! * timeline ordering (see [`crate::timeline`]),
//! * rewind navigation (see [`crate::rewind`]),
//! * replay reconstruction (`replay.rs`),
//! * branching (Layer 11).
//!
//! # Why generic over `S`
//!
//! The prompt for this file explicitly forbids creating a second state
//! authority or a competing snapshot representation. Layer 04 already
//! defines what a state snapshot is. Layer 07 has no business
//! duplicating it. The generic parameter `S` is the architectural seam:
//! it lets Layer 04's actual snapshot type — whether a value, an ID, a
//! handle, an `Arc`, or a store position — be slotted in without
//! Layer 07 ever learning what it is.
//!
//! # Why no `SnapshotError`
//!
//! Constructing a `TemporalSnapshot<S>` cannot fail: every `SimTime`
//! is valid, every `S` is valid, and no cross-field invariant can be
//! violated. Introducing an error type would be an invented API with no
//! downstream consumer. Callers who need validation (e.g. "does this
//! state payload still exist in the store?") do so at Layer 04's
//! boundary — not here.
//!
//! # Immutability
//!
//! `TemporalSnapshot<S>` has **no `&mut self` methods**. Once created,
//! a snapshot's temporal position and payload cannot be changed by any
//! safe operation. This satisfies the invariant that a committed
//! historical point must not silently mutate — see Section 7 of the
//! Layer 07 snapshot contract.
//!
//! # Determinism
//!
//! Time is obtained either from an explicit `SimTime` or from
//! `clock.now()`. Wall clock, `SystemTime`, `Instant`, and sleeping are
//! never consulted. Same `(SimTime, S)` construction ⇒ same
//! `TemporalSnapshot<S>`, byte-for-byte, in every run, process, and
//! platform.
//!
//! # Intended composition
//!
//! The natural way to build a HELQIX temporal history is:
//!
//! ```text
//! Timeline<TemporalSnapshot<StateHandle>>
//! ```
//!
//! i.e. the timeline's generic payload `T` is a `TemporalSnapshot<S>`.
//! This file does not mandate that composition — the payload is
//! whatever the caller chooses — but the tests exercise it end to end.
//!
//! # Worked failure workflow
//!
//! ```text
//! t = 12.40  motor_temperature_rising
//! t = 12.48  thermal_constraint_violated
//! t = 12.48  failure_detected
//! t = 12.48  snapshot taken  ──► TemporalSnapshot { at: 12.48, state: H }
//! ```
//!
//! From that single snapshot, Layer 11 can fork Branch A and Branch B
//! against the *same* recoverable point, Layer 07's `rewind.rs` can
//! navigate back to it, and `replay.rs` can reconstruct from it. None
//! of those responsibilities live in this file.

use core::fmt;

use crate::clock::{Clock, SimTime};

// ============================================================================
// TemporalSnapshot
// ============================================================================

/// An exact recoverable temporal point in HELQIX executable reality.
///
/// Binds a [`SimTime`] (from [`crate::clock`]) to an opaque state
/// payload `S` supplied by the caller. The payload is *not* interpreted
/// by this module; it is whatever downstream layers chose to record as
/// the recoverable identity of the state at that instant — typically a
/// Layer 04 state handle or snapshot reference.
///
/// # Invariants
///
/// 1. Once constructed, `at` and `state` cannot be mutated by any safe
///    operation. (`TemporalSnapshot` has no `&mut self` methods.)
/// 2. `at` is exactly the logical time the snapshot represents. No
///    rounding, no adjustment, no wall-clock smoothing.
/// 3. Two `TemporalSnapshot<S>` values are structurally equal iff their
///    `at` and `state` are equal (derived `PartialEq` when `S: PartialEq`).
/// 4. Ordering is lexicographic: primarily by `at`, secondarily by
///    `state` (derived `Ord` when `S: Ord`).
///
/// # Ownership
///
/// `TemporalSnapshot<S>` owns its payload `S`. If a caller wants to
/// share state across snapshots, they may set `S` to their own handle
/// type (e.g. an `Arc<...>` or a `Copy` identifier). This module makes
/// no assumption about that choice.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TemporalSnapshot<S> {
    at: SimTime,
    state: S,
}

impl<S> TemporalSnapshot<S> {
    /// Construct a snapshot at an explicit logical time.
    ///
    /// This is the primary constructor. It cannot fail: every
    /// [`SimTime`] is a valid temporal position and every `S` is a
    /// valid payload.
    #[inline]
    #[must_use]
    pub const fn new(at: SimTime, state: S) -> Self {
        Self { at, state }
    }

    /// Construct a snapshot using the current logical time of `clock`.
    ///
    /// Reads `clock.now()` and never advances the clock. This is the
    /// canonical way for Layer 05 execution to freeze the state
    /// produced by a transition at the moment the transition completed.
    #[inline]
    #[must_use]
    pub fn capture(clock: &Clock, state: S) -> Self {
        Self::new(clock.now(), state)
    }

    /// The exact logical time this snapshot represents.
    #[inline]
    #[must_use]
    pub const fn at(&self) -> SimTime {
        self.at
    }

    /// Borrow the state payload.
    #[inline]
    #[must_use]
    pub const fn state(&self) -> &S {
        &self.state
    }

    /// Consume the snapshot and return the state payload.
    ///
    /// Useful when a downstream layer (branching, replay) needs to
    /// take ownership of the payload without cloning.
    #[inline]
    #[must_use]
    pub fn into_state(self) -> S {
        self.state
    }

    /// Consume the snapshot and return both parts.
    #[inline]
    #[must_use]
    pub fn into_parts(self) -> (SimTime, S) {
        (self.at, self.state)
    }

    /// Is this snapshot exactly at `time`?
    #[inline]
    #[must_use]
    pub fn is_at(&self, time: SimTime) -> bool {
        self.at == time
    }

    /// Does this snapshot occur strictly before `time`?
    ///
    /// This is the natural query for "was this snapshot taken before
    /// the failure at `time`?".
    #[inline]
    #[must_use]
    pub fn is_before(&self, time: SimTime) -> bool {
        self.at < time
    }

    /// Does this snapshot occur strictly after `time`?
    #[inline]
    #[must_use]
    pub fn is_after(&self, time: SimTime) -> bool {
        self.at > time
    }

    /// Does this snapshot occur at or before `time`?
    #[inline]
    #[must_use]
    pub fn is_at_or_before(&self, time: SimTime) -> bool {
        self.at <= time
    }

    /// Does this snapshot occur at or after `time`?
    #[inline]
    #[must_use]
    pub fn is_at_or_after(&self, time: SimTime) -> bool {
        self.at >= time
    }
}

impl<S> TemporalSnapshot<S> {
    /// Does this snapshot precede `other` in logical time?
    ///
    /// Only compares `at`. Equal times yield `false` — use
    /// [`same_time_as`](Self::same_time_as) for equality. Deliberately
    /// defined for **any** `S` and **any** `T`, because temporal
    /// ordering is independent of the state payload type.
    #[inline]
    #[must_use]
    pub fn precedes<T>(&self, other: &TemporalSnapshot<T>) -> bool {
        self.at < other.at
    }

    /// Does this snapshot follow `other` in logical time?
    #[inline]
    #[must_use]
    pub fn follows<T>(&self, other: &TemporalSnapshot<T>) -> bool {
        self.at > other.at
    }

    /// Does this snapshot occur at the same logical time as `other`?
    ///
    /// Same time does **not** imply same state — the payloads may
    /// differ, and that is exactly the situation two branches forked
    /// from the same instant exhibit.
    #[inline]
    #[must_use]
    pub fn same_time_as<T>(&self, other: &TemporalSnapshot<T>) -> bool {
        self.at == other.at
    }
}

impl<S: fmt::Debug> fmt::Display for TemporalSnapshot<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TemporalSnapshot @ {} :: {:?}", self.at, self.state)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::SimDelta;
    use crate::timeline::Timeline;

    fn ns(n: i128) -> SimTime {
        SimTime::from_nanos(n)
    }

    fn delta(n: i128) -> SimDelta {
        SimDelta::from_nanos(n)
    }

    /// A stand-in for whatever Layer 04 actually hands us — a state
    /// handle, snapshot ID, or store position. `Copy` so we can build
    /// and compare snapshots cheaply in tests without guessing
    /// Layer 04's ownership model.
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
    struct StateHandle(u64);

    // ------------------------------------------------------------------
    // 1. Construction
    // ------------------------------------------------------------------

    #[test]
    fn new_binds_time_and_state_exactly() {
        let s = TemporalSnapshot::new(ns(12_480_000_000), StateHandle(7));
        assert_eq!(s.at(), ns(12_480_000_000));
        assert_eq!(*s.state(), StateHandle(7));
    }

    #[test]
    fn capture_reads_clock_without_advancing_it() {
        let mut clock = Clock::new();
        clock.advance_by(delta(12_480_000_000)).unwrap();

        let before = clock.now();
        let s = TemporalSnapshot::capture(&clock, StateHandle(1));
        let after = clock.now();

        assert_eq!(before, after, "capture must not advance the clock");
        assert_eq!(s.at(), before);
        assert_eq!(*s.state(), StateHandle(1));
    }

    #[test]
    fn capture_at_origin_yields_zero_time() {
        let clock = Clock::new();
        let s = TemporalSnapshot::capture(&clock, StateHandle(0));
        assert_eq!(s.at(), SimTime::ZERO);
    }

    // ------------------------------------------------------------------
    // 2. Immutability — no &mut self methods exist
    // ------------------------------------------------------------------

    #[test]
    fn snapshot_payload_is_extractable_by_value() {
        let s = TemporalSnapshot::new(ns(100), StateHandle(9));
        let (t, h) = s.into_parts();
        assert_eq!(t, ns(100));
        assert_eq!(h, StateHandle(9));
    }

    #[test]
    fn into_state_yields_payload() {
        let s = TemporalSnapshot::new(ns(100), StateHandle(9));
        assert_eq!(s.into_state(), StateHandle(9));
    }

    // ------------------------------------------------------------------
    // 3. Temporal queries
    // ------------------------------------------------------------------

    #[test]
    fn temporal_predicates_are_exact() {
        let s = TemporalSnapshot::new(ns(100), StateHandle(1));

        assert!(s.is_at(ns(100)));
        assert!(!s.is_at(ns(99)));
        assert!(!s.is_at(ns(101)));

        assert!(s.is_before(ns(101)));
        assert!(!s.is_before(ns(100)));
        assert!(!s.is_before(ns(99)));

        assert!(s.is_after(ns(99)));
        assert!(!s.is_after(ns(100)));
        assert!(!s.is_after(ns(101)));

        assert!(s.is_at_or_before(ns(100)));
        assert!(s.is_at_or_before(ns(101)));
        assert!(!s.is_at_or_before(ns(99)));

        assert!(s.is_at_or_after(ns(100)));
        assert!(s.is_at_or_after(ns(99)));
        assert!(!s.is_at_or_after(ns(101)));
    }

    // ------------------------------------------------------------------
    // 4. Ordering — primary by time, secondary by payload
    // ------------------------------------------------------------------

    #[test]
    fn ordering_is_lexicographic_time_first() {
        let a = TemporalSnapshot::new(ns(0), StateHandle(9));
        let b = TemporalSnapshot::new(ns(100), StateHandle(1));
        let c = TemporalSnapshot::new(ns(100), StateHandle(2));

        assert!(a < b);
        assert!(b < c);
        assert!(a < c);

        // Sort preserves the deterministic order.
        let mut v = vec![c, a, b];
        v.sort();
        assert_eq!(v, vec![a, b, c]);
    }

    #[test]
    fn equal_time_and_payload_are_equal() {
        let a = TemporalSnapshot::new(ns(50), StateHandle(3));
        let b = TemporalSnapshot::new(ns(50), StateHandle(3));
        assert_eq!(a, b);
    }

    #[test]
    fn equal_time_different_payload_are_distinct() {
        // Two branches forked from the same instant: same time, distinct
        // state handles. They must NOT be equal.
        let a = TemporalSnapshot::new(ns(50), StateHandle(1));
        let b = TemporalSnapshot::new(ns(50), StateHandle(2));
        assert_ne!(a, b);
        assert!(a.same_time_as(&b));
    }

    // ------------------------------------------------------------------
    // 5. Cross-payload temporal comparison
    // ------------------------------------------------------------------

    #[test]
    fn precedes_and_follows_ignore_payload_type() {
        // Same temporal ordering must be observable across different
        // state payload types — temporal ordering does not depend on
        // what the state is.
        let a: TemporalSnapshot<StateHandle> = TemporalSnapshot::new(ns(10), StateHandle(1));
        let b: TemporalSnapshot<&str> = TemporalSnapshot::new(ns(20), "some-state");

        assert!(a.precedes(&b));
        assert!(b.follows(&a));
        assert!(!a.same_time_as(&b));
    }

    #[test]
    fn same_time_across_payload_types() {
        let a: TemporalSnapshot<u32> = TemporalSnapshot::new(ns(100), 1);
        let b: TemporalSnapshot<String> = TemporalSnapshot::new(ns(100), "x".into());
        assert!(a.same_time_as(&b));
        assert!(!a.precedes(&b));
        assert!(!a.follows(&b));
    }

    // ------------------------------------------------------------------
    // 6. Independence between snapshots
    // ------------------------------------------------------------------

    #[test]
    fn snapshots_are_independent() {
        let a = TemporalSnapshot::new(ns(10), StateHandle(1));
        let b = TemporalSnapshot::new(ns(20), StateHandle(1));
        let c = TemporalSnapshot::new(ns(10), StateHandle(1));

        // Copy semantics: mutating one does not affect another. (There
        // are no mutators, but `Copy` + independent storage guarantees
        // this at the type level.)
        let d = a;
        assert_eq!(a, d);
        assert_ne!(a, b);
        assert_eq!(a, c);
    }

    // ------------------------------------------------------------------
    // 7. Determinism: identical inputs → identical snapshots
    // ------------------------------------------------------------------

    #[test]
    fn identical_construction_is_deterministic() {
        let build = || {
            let mut clock = Clock::new();
            clock.advance_by(delta(12_480_000_000)).unwrap();
            TemporalSnapshot::capture(&clock, StateHandle(7))
        };

        let s1 = build();
        let s2 = build();
        let s3 = build();

        assert_eq!(s1, s2);
        assert_eq!(s2, s3);
        assert_eq!(s1.at(), ns(12_480_000_000));
    }

    // ------------------------------------------------------------------
    // 8. Integration with timeline.rs
    // ------------------------------------------------------------------

    #[test]
    fn composes_with_timeline_as_payload() {
        // The intended composition is Timeline<TemporalSnapshot<S>>.
        let mut clock = Clock::new();
        let mut tl: Timeline<TemporalSnapshot<StateHandle>> = Timeline::new();

        // t=0 initial state
        tl.record_at(clock.now(), TemporalSnapshot::capture(&clock, StateHandle(0)))
            .unwrap();

        // t=1 motor action applied → new state
        clock.advance_by(delta(1_000_000_000)).unwrap();
        tl.record_at(clock.now(), TemporalSnapshot::capture(&clock, StateHandle(1)))
            .unwrap();

        // t=2 velocity change
        clock.advance_by(delta(1_000_000_000)).unwrap();
        tl.record_at(clock.now(), TemporalSnapshot::capture(&clock, StateHandle(2)))
            .unwrap();

        assert_eq!(tl.len(), 3);

        // Exact-time lookup for replay.
        let s_at_1s = tl.at_exact(ns(1_000_000_000)).unwrap().subject();
        assert_eq!(s_at_1s.at(), ns(1_000_000_000));
        assert_eq!(*s_at_1s.state(), StateHandle(1));

        // The snapshot payload preserves the temporal coordinate.
        assert!(s_at_1s.is_before(ns(2_000_000_000)));
        assert!(s_at_1s.is_after(ns(0)));
    }

    // ------------------------------------------------------------------
    // 9. Worked failure / branch scenario
    // ------------------------------------------------------------------

    #[test]
    fn worked_failure_and_branch_scenario() {
        // Mirrors the prompt's failure workflow:
        //   t = 12.40  temperature rising
        //   t = 12.48  constraint violated + failure detected
        //   t = 12.48  snapshot taken
        // Both branches must originate from the SAME snapshot.
        let mut clock = Clock::new();
        clock.advance_by(delta(12_400_000_000)).unwrap(); // t = 12.40
        let pre_failure = TemporalSnapshot::capture(&clock, StateHandle(100));

        clock.advance_by(delta(80_000_000)).unwrap(); // t = 12.48
        let failure_point = TemporalSnapshot::capture(&clock, StateHandle(101));

        assert_eq!(failure_point.at(), ns(12_480_000_000));
        assert!(pre_failure.is_before(failure_point.at()));
        assert!(failure_point.is_at(ns(12_480_000_000)));

        // Branch A and Branch B both reference the same snapshot
        // (here: a Copy-able handle). Layer 11 would fork from this
        // point; this file merely guarantees the temporal coordinate
        // and the payload are identical for both branches.
        let branch_a_origin = failure_point;
        let branch_b_origin = failure_point;
        assert_eq!(branch_a_origin, branch_b_origin);
        assert_eq!(branch_a_origin.at(), branch_b_origin.at());
        assert_eq!(
            branch_a_origin.state(),
            branch_b_origin.state()
        );
    }

    // ------------------------------------------------------------------
    // 10. Boundary conditions
    // ------------------------------------------------------------------

    #[test]
    fn extreme_sim_time_boundaries_are_supported() {
        let s_min = TemporalSnapshot::new(SimTime::MIN, StateHandle(0));
        let s_max = TemporalSnapshot::new(SimTime::MAX, StateHandle(0));

        assert!(s_min.precedes(&s_max));
        assert!(s_max.follows(&s_min));
        assert!(s_min.is_at(SimTime::MIN));
        assert!(s_max.is_at(SimTime::MAX));
        assert!(s_min.is_at_or_before(SimTime::MAX));
        assert!(s_max.is_at_or_after(SimTime::MIN));
    }

    // ------------------------------------------------------------------
    // 11. Display
    // ------------------------------------------------------------------

    #[test]
    fn display_includes_time_and_state() {
        let s = TemporalSnapshot::new(ns(12_480_000_000), StateHandle(7));
        let text = s.to_string();
        assert!(text.contains("12.480000000s"), "got: {}", text);
        assert!(text.contains("StateHandle(7)"), "got: {}", text);
    }

    // ------------------------------------------------------------------
    // 12. Copy / Clone
    // ------------------------------------------------------------------

    #[test]
    fn snapshot_is_copy_when_payload_is_copy() {
        let a = TemporalSnapshot::new(ns(1), StateHandle(1));
        let b = a; // Copy
        let c = a.clone(); // Clone
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    // ------------------------------------------------------------------
    // 13. Hash consistency with Eq
    // ------------------------------------------------------------------

    #[test]
    fn equal_snapshots_hash_equal() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        fn h<S: Hash>(s: &TemporalSnapshot<S>) -> u64 {
            let mut hasher = DefaultHasher::new();
            s.hash(&mut hasher);
            hasher.finish()
        }

        let a = TemporalSnapshot::new(ns(100), StateHandle(5));
        let b = TemporalSnapshot::new(ns(100), StateHandle(5));
        assert_eq!(a, b);
        assert_eq!(h(&a), h(&b));
    }
}
