//! Deterministic rewind navigation for HELQIX.
//!
//! This module answers a single question, precisely:
//!
//! > "Given a [`Timeline`], which existing historical point should the
//! > active execution context move to?"
//!
//! # What rewind is
//!
//! Rewind is **non-destructive navigation** over an existing execution
//! timeline. It does not:
//!
//! * advance or move the clock (that is [`crate::clock`]'s job);
//! * mutate or truncate the timeline (history must remain available for
//!   replay, comparison, causal analysis, branching, verification, and
//!   evidence — see INVARIANT 7 of the Layer 07 contract);
//! * restore engineering state (that belongs to Layer 04, driven by the
//!   consumer of a [`RewindTarget`]);
//! * execute actions (Layer 05);
//! * re-run history (that is `replay.rs`).
//!
//! # What rewind is not
//!
//! Rewind is **not** replay. Rewind *identifies* a point already present
//! in the timeline. Replay *reconstructs* the sequence from that point.
//! They are deliberately separate modules with separate responsibilities.
//!
//! Rewind is **not** a snapshot store. Snapshots are owned elsewhere in
//! Layer 07 (`snapshot.rs`) and Layer 04. `rewind.rs` treats the
//! timeline payload `T` opaquely — it is whatever downstream layers
//! chose to record as the recoverable identity of a moment (a snapshot
//! reference, a state revision ID, a branch-point handle, etc.).
//!
//! # Non-destructive guarantee
//!
//! Every operation in this module takes `&Timeline<T>` — never
//! `&mut Timeline<T>`. The borrow checker therefore makes it
//! **impossible** for a rewind resolution to mutate, reorder, or
//! truncate the historical record.
//!
//! # Example workflow (prose, not doctest)
//!
//! Original execution recorded in a timeline `tl`:
//!
//! ```text
//! t0 ─ t1 ─ t2 ─ t3 ─ t4(failure)
//! ```
//!
//! To branch from the point immediately before the failure:
//!
//! ```text
//! let target = RewindRequest::StrictlyBefore(t4).resolve(&tl)?;
//! assert_eq!(target.direction_from(t4), RewindDirection::Backward);
//! // Hand `target.subject()` (the snapshot/state identity at t3) to the
//! // branching engine. The timeline `tl` is untouched.
//! ```

use core::fmt;

use crate::clock::SimTime;
use crate::timeline::{Timeline, TimelineEntry};

// ============================================================================
// RewindDirection
// ============================================================================

/// The direction of a resolved rewind, relative to a reference time.
///
/// Used by verification and safety checks to confirm that a rewind
/// actually moved the active execution context backward, rather than
/// accidentally performing a no-op or a forward jump.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum RewindDirection {
    /// Target time < reference time: a genuine rewind.
    Backward,
    /// Target time == reference time: a degenerate no-op.
    Same,
    /// Target time > reference time: a forward move (not a rewind).
    Forward,
}

// ============================================================================
// RewindTarget
// ============================================================================

/// A resolved historical point that the active execution context should
/// move to.
///
/// Holds a borrowed reference into the timeline — no cloning, no
/// allocation, no duplication of the payload. Because it borrows the
/// timeline immutably, the borrow checker guarantees the target cannot
/// outlive a mutation of the timeline, which is exactly the correct
/// temporal safety property: *resolve, act, then continue*.
///
/// The generic `T` is whatever the caller stored as the recoverable
/// identity of a moment. `rewind.rs` never interprets it.
#[derive(Debug)]
pub struct RewindTarget<'a, T> {
    entry: &'a TimelineEntry<T>,
}

impl<'a, T> RewindTarget<'a, T> {
    /// The exact logical time of the target point.
    #[inline]
    #[must_use]
    pub fn at(&self) -> SimTime {
        self.entry.at()
    }

    /// The caller-supplied identity of the target point (typically a
    /// snapshot reference or state revision handle).
    #[inline]
    #[must_use]
    pub fn subject(&self) -> &T {
        self.entry.subject()
    }

    /// The underlying timeline entry. Provided so that downstream
    /// layers (branching, verification) can inspect or forward it
    /// without this module re-exposing every entry accessor.
    #[inline]
    #[must_use]
    pub fn entry(&self) -> &'a TimelineEntry<T> {
        self.entry
    }

    /// Classify this target relative to a reference time.
    ///
    /// Callers typically pass the current active execution time:
    /// a [`RewindDirection::Backward`] result is a genuine rewind, while
    /// [`RewindDirection::Same`] or [`RewindDirection::Forward`] signal
    /// that no rewind occurred (a no-op request or a caller bug).
    #[inline]
    #[must_use]
    pub fn direction_from(&self, from: SimTime) -> RewindDirection {
        use core::cmp::Ordering;
        match self.entry.at().cmp(&from) {
            Ordering::Less => RewindDirection::Backward,
            Ordering::Equal => RewindDirection::Same,
            Ordering::Greater => RewindDirection::Forward,
        }
    }
}

// RewindTarget is a thin borrow wrapper; it is Copy/Clone regardless of
// whether `T` is, because the only field is a shared reference.
impl<'a, T> Clone for RewindTarget<'a, T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T> Copy for RewindTarget<'a, T> {}

// ============================================================================
// RewindError
// ============================================================================

/// Reasons a rewind request could not be resolved against a timeline.
///
/// Every variant is a *normal, expected* condition a caller may need to
/// handle — not a panic. The timeline is never modified by a failed
/// resolve.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RewindError {
    /// The timeline contains no entries. No rewind target exists.
    EmptyTimeline,

    /// `RewindRequest::Exact(at)` was issued but no entry occurs exactly
    /// at `at`.
    NoEntryAt { at: SimTime },

    /// `RewindRequest::AtOrBefore(at)` was issued but no entry occurs at
    /// or before `at`.
    NoEntryAtOrBefore { at: SimTime },

    /// `RewindRequest::StrictlyBefore(at)` was issued but no entry occurs
    /// strictly before `at`.
    NoEntryStrictlyBefore { at: SimTime },
}

impl fmt::Display for RewindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTimeline => {
                f.write_str("cannot resolve rewind target: timeline is empty")
            }
            Self::NoEntryAt { at } => {
                write!(f, "cannot resolve rewind target: no entry exists at {}", at)
            }
            Self::NoEntryAtOrBefore { at } => write!(
                f,
                "cannot resolve rewind target: no entry exists at or before {}",
                at
            ),
            Self::NoEntryStrictlyBefore { at } => write!(
                f,
                "cannot resolve rewind target: no entry exists strictly before {}",
                at
            ),
        }
    }
}

impl std::error::Error for RewindError {}

// ============================================================================
// RewindRequest
// ============================================================================

/// An intent to move the active execution context to a historical point.
///
/// This is the semantic API of `rewind.rs`. Each variant captures a
/// distinct HELQIX workflow:
///
/// * [`RewindRequest::Exact`] — replay from a recorded start;
/// * [`RewindRequest::AtOrBefore`] — rewind to the last known-good state
///   at or before a threshold;
/// * [`RewindRequest::StrictlyBefore`] — the canonical "rewind to just
///   before the failure" operation;
/// * [`RewindRequest::Earliest`] — restart from the timeline origin;
/// * [`RewindRequest::Latest`] — a degenerate no-op, useful for
///   verification ("is this timeline still at the tip?").
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum RewindRequest {
    /// Resolve to the first entry whose time is exactly `at`.
    Exact(SimTime),
    /// Resolve to the latest entry whose time is `<= at`.
    AtOrBefore(SimTime),
    /// Resolve to the latest entry whose time is strictly `< at`.
    StrictlyBefore(SimTime),
    /// Resolve to the first entry of the timeline.
    Earliest,
    /// Resolve to the last entry of the timeline.
    Latest,
}

impl RewindRequest {
    /// Resolve this request against a timeline, returning a borrowed
    /// [`RewindTarget`] on success.
    ///
    /// The timeline is never modified. On failure the timeline is
    /// unchanged and the caller receives a precise [`RewindError`].
    ///
    /// # Determinism
    ///
    /// Resolution uses only the timeline's own ordering semantics.
    /// Given the same timeline and the same request, the same target is
    /// returned — across runs, processes, and platforms.
    pub fn resolve<'a, T>(
        self,
        timeline: &'a Timeline<T>,
    ) -> Result<RewindTarget<'a, T>, RewindError> {
        if timeline.is_empty() {
            return Err(RewindError::EmptyTimeline);
        }
        match self {
            Self::Exact(at) => timeline
                .at_exact(at)
                .map(|entry| RewindTarget { entry })
                .ok_or(RewindError::NoEntryAt { at }),

            Self::AtOrBefore(at) => timeline
                .at_or_before(at)
                .map(|entry| RewindTarget { entry })
                .ok_or(RewindError::NoEntryAtOrBefore { at }),

            Self::StrictlyBefore(at) => timeline
                .before(at)
                .map(|entry| RewindTarget { entry })
                .ok_or(RewindError::NoEntryStrictlyBefore { at }),

            Self::Earliest => timeline
                .first()
                .map(|entry| RewindTarget { entry })
                .ok_or(RewindError::EmptyTimeline),

            Self::Latest => timeline
                .latest()
                .map(|entry| RewindTarget { entry })
                .ok_or(RewindError::EmptyTimeline),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::SimTime;

    fn ns(n: i128) -> SimTime {
        SimTime::from_nanos(n)
    }

    /// Build the standard worked-example timeline used by the Layer 07
    /// contract:
    ///
    /// ```text
    /// 0.00 s  initial
    /// 0.10 s  motor_action
    /// 0.20 s  velocity_changed
    /// 0.30 s  temperature_rose
    /// 0.40 s  constraint_violated
    /// 0.40 s  failure_detected
    /// 0.40 s  state_frozen
    /// ```
    fn worked_timeline() -> Timeline<&'static str> {
        let mut tl = Timeline::new();
        tl.record_at(ns(0), "initial").unwrap();
        tl.record_at(ns(100_000_000), "motor_action").unwrap();
        tl.record_at(ns(200_000_000), "velocity_changed").unwrap();
        tl.record_at(ns(300_000_000), "temperature_rose").unwrap();
        tl.record_at(ns(400_000_000), "constraint_violated").unwrap();
        tl.record_at(ns(400_000_000), "failure_detected").unwrap();
        tl.record_at(ns(400_000_000), "state_frozen").unwrap();
        tl
    }

    // ------------------------------------------------------------------
    // 1. Empty timeline is uniformly rejected
    // ------------------------------------------------------------------

    #[test]
    fn all_requests_against_empty_timeline_return_empty_error() {
        let tl: Timeline<u32> = Timeline::new();

        for req in [
            RewindRequest::Exact(ns(0)),
            RewindRequest::AtOrBefore(ns(0)),
            RewindRequest::StrictlyBefore(ns(0)),
            RewindRequest::Earliest,
            RewindRequest::Latest,
        ] {
            assert_eq!(
                req.resolve(&tl).unwrap_err(),
                RewindError::EmptyTimeline,
                "request {:?} against empty timeline",
                req
            );
        }
    }

    // ------------------------------------------------------------------
    // 2. Exact request
    // ------------------------------------------------------------------

    #[test]
    fn exact_request_resolves_to_matching_entry() {
        let tl = worked_timeline();
        let target = RewindRequest::Exact(ns(300_000_000)).resolve(&tl).unwrap();
        assert_eq!(target.at(), ns(300_000_000));
        assert_eq!(*target.subject(), "temperature_rose");
    }

    #[test]
    fn exact_request_on_missing_time_fails_with_no_entry_at() {
        let tl = worked_timeline();
        let err = RewindRequest::Exact(ns(123)).resolve(&tl).unwrap_err();
        assert_eq!(err, RewindError::NoEntryAt { at: ns(123) });
    }

    // ------------------------------------------------------------------
    // 3. AtOrBefore request
    // ------------------------------------------------------------------

    #[test]
    fn at_or_before_finds_latest_prior_entry() {
        let tl = worked_timeline();
        // Any time between temperature_rose (300ms) and constraint_violated
        // (400ms) should resolve to temperature_rose.
        let target = RewindRequest::AtOrBefore(ns(350_000_000))
            .resolve(&tl)
            .unwrap();
        assert_eq!(target.at(), ns(300_000_000));
        assert_eq!(*target.subject(), "temperature_rose");
    }

    #[test]
    fn at_or_before_is_inclusive_at_the_boundary() {
        let tl = worked_timeline();
        let target = RewindRequest::AtOrBefore(ns(300_000_000))
            .resolve(&tl)
            .unwrap();
        assert_eq!(target.at(), ns(300_000_000));
        assert_eq!(*target.subject(), "temperature_rose");
    }

    #[test]
    fn at_or_before_before_first_entry_fails() {
        let tl = worked_timeline();
        let err = RewindRequest::AtOrBefore(ns(-1)).resolve(&tl).unwrap_err();
        assert_eq!(err, RewindError::NoEntryAtOrBefore { at: ns(-1) });
    }

    // ------------------------------------------------------------------
    // 4. StrictlyBefore request — the canonical "before failure" case
    // ------------------------------------------------------------------

    #[test]
    fn strictly_before_failure_resolves_to_preceding_moment() {
        let tl = worked_timeline();
        let failure_t = ns(400_000_000);
        let target = RewindRequest::StrictlyBefore(failure_t)
            .resolve(&tl)
            .unwrap();
        assert_eq!(target.at(), ns(300_000_000));
        assert_eq!(*target.subject(), "temperature_rose");
        assert_eq!(target.direction_from(failure_t), RewindDirection::Backward);
    }

    #[test]
    fn strictly_before_skips_all_equal_timestamp_ties() {
        // All three entries at 400ms must be skipped — the request is
        // *strictly* before, and 400ms is not strictly before 400ms.
        let tl = worked_timeline();
        let target = RewindRequest::StrictlyBefore(ns(400_000_000))
            .resolve(&tl)
            .unwrap();
        assert_ne!(target.at(), ns(400_000_000));
        assert_eq!(target.at(), ns(300_000_000));
    }

    #[test]
    fn strictly_before_first_entry_fails() {
        let tl = worked_timeline();
        let err = RewindRequest::StrictlyBefore(ns(0))
            .resolve(&tl)
            .unwrap_err();
        assert_eq!(err, RewindError::NoEntryStrictlyBefore { at: ns(0) });
    }

    // ------------------------------------------------------------------
    // 5. Earliest / Latest
    // ------------------------------------------------------------------

    #[test]
    fn earliest_resolves_to_first_entry() {
        let tl = worked_timeline();
        let target = RewindRequest::Earliest.resolve(&tl).unwrap();
        assert_eq!(target.at(), ns(0));
        assert_eq!(*target.subject(), "initial");
    }

    #[test]
    fn latest_resolves_to_last_entry() {
        let tl = worked_timeline();
        let target = RewindRequest::Latest.resolve(&tl).unwrap();
        assert_eq!(target.at(), ns(400_000_000));
        // Among the three entries at 400ms, Latest is the last inserted.
        assert_eq!(*target.subject(), "state_frozen");
    }

    // ------------------------------------------------------------------
    // 6. Direction classification
    // ------------------------------------------------------------------

    #[test]
    fn direction_classification_is_correct() {
        let tl = worked_timeline();
        let target = RewindRequest::Exact(ns(200_000_000)).resolve(&tl).unwrap();

        assert_eq!(
            target.direction_from(ns(400_000_000)),
            RewindDirection::Backward
        );
        assert_eq!(
            target.direction_from(ns(200_000_000)),
            RewindDirection::Same
        );
        assert_eq!(
            target.direction_from(ns(0)),
            RewindDirection::Forward
        );
    }

    // ------------------------------------------------------------------
    // 7. Non-destructive: resolving a rewind does not mutate the timeline
    // ------------------------------------------------------------------

    #[test]
    fn resolve_does_not_mutate_the_timeline() {
        let mut tl = worked_timeline();
        let len_before = tl.len();
        let first_before = *tl.first().unwrap().subject();
        let last_before = *tl.latest().unwrap().subject();

        // Exercise every request type against the same timeline.
        let _ = RewindRequest::Exact(ns(200_000_000)).resolve(&tl).unwrap();
        let _ = RewindRequest::AtOrBefore(ns(250_000_000)).resolve(&tl).unwrap();
        let _ = RewindRequest::StrictlyBefore(ns(400_000_000)).resolve(&tl).unwrap();
        let _ = RewindRequest::Earliest.resolve(&tl).unwrap();
        let _ = RewindRequest::Latest.resolve(&tl).unwrap();

        assert_eq!(tl.len(), len_before);
        assert_eq!(*tl.first().unwrap().subject(), first_before);
        assert_eq!(*tl.latest().unwrap().subject(), last_before);

        // And the timeline still accepts a forward append — history is
        // intact and extensible.
        tl.record_at(ns(500_000_000), "post_rewind").unwrap();
        assert_eq!(tl.len(), len_before + 1);
    }

    // ------------------------------------------------------------------
    // 8. Determinism
    // ------------------------------------------------------------------

    #[test]
    fn identical_requests_produce_identical_targets() {
        let tl = worked_timeline();

        let a = RewindRequest::StrictlyBefore(ns(400_000_000))
            .resolve(&tl)
            .unwrap();
        let b = RewindRequest::StrictlyBefore(ns(400_000_000))
            .resolve(&tl)
            .unwrap();
        let c = RewindRequest::StrictlyBefore(ns(400_000_000))
            .resolve(&tl)
            .unwrap();

        assert_eq!(a.at(), b.at());
        assert_eq!(b.at(), c.at());
        assert_eq!(a.subject(), b.subject());
        assert_eq!(b.subject(), c.subject());

        // Also verify that the entries are identical pointers — resolution
        // is purely deterministic and produces the same borrow.
        assert!(core::ptr::eq(a.entry(), b.entry()));
        assert!(core::ptr::eq(b.entry(), c.entry()));
    }

    // ------------------------------------------------------------------
    // 9. Target is Copy / Clone
    // ------------------------------------------------------------------

    #[test]
    fn rewind_target_is_copy() {
        let tl = worked_timeline();
        let target = RewindRequest::Latest.resolve(&tl).unwrap();
        let copied = target; // Copy
        let cloned = target.clone(); // Clone
        assert_eq!(copied.at(), cloned.at());
        assert_eq!(copied.subject(), cloned.subject());
    }

    // ------------------------------------------------------------------
    // 10. Independent timelines produce independent targets
    // ------------------------------------------------------------------

    #[test]
    fn targets_from_independent_timelines_are_independent() {
        let tl_a = worked_timeline();
        let tl_b = worked_timeline();

        let ta = RewindRequest::Exact(ns(100_000_000)).resolve(&tl_a).unwrap();
        let tb = RewindRequest::Exact(ns(100_000_000)).resolve(&tl_b).unwrap();

        // Same observable content.
        assert_eq!(ta.at(), tb.at());
        assert_eq!(ta.subject(), tb.subject());
        // But different timelines, therefore different borrows.
        assert!(!core::ptr::eq(ta.entry(), tb.entry()));
    }

    // ------------------------------------------------------------------
    // 11. Boundary conditions
    // ------------------------------------------------------------------

    #[test]
    fn extreme_sim_time_boundaries_are_handled() {
        let mut tl = Timeline::new();
        tl.record_at(SimTime::MIN, "min").unwrap();
        tl.record_at(SimTime::MAX, "max").unwrap();

        assert_eq!(
            *RewindRequest::Earliest.resolve(&tl).unwrap().subject(),
            "min"
        );
        assert_eq!(
            *RewindRequest::Latest.resolve(&tl).unwrap().subject(),
            "max"
        );
        assert_eq!(
            RewindRequest::StrictlyBefore(SimTime::MIN)
                .resolve(&tl)
                .unwrap_err(),
            RewindError::NoEntryStrictlyBefore { at: SimTime::MIN }
        );
    }

    // ------------------------------------------------------------------
    // 12. Failure-workflow composition
    // ------------------------------------------------------------------

    #[test]
    fn composed_rewind_and_branch_workflow() {
        // Mirrors the Layer 07 branching workflow:
        //
        //   Original execution
        //          ↓
        //   Failure at t = 0.40 s
        //          ↓
        //   Rewind to just before the failure
        //          ↓
        //   Hand the target's subject (a state identity) to branching
        //
        let tl = worked_timeline();
        let failure_t = ns(400_000_000);

        // 1. Rewind resolution.
        let target = RewindRequest::StrictlyBefore(failure_t)
            .resolve(&tl)
            .unwrap();
        assert_eq!(target.direction_from(failure_t), RewindDirection::Backward);
        assert_eq!(target.at(), ns(300_000_000));

        // 2. Branching would receive the subject as the fork point.
        //    (Branching is not implemented here — this test only proves
        //    the target carries the correct identity.)
        let fork_subject = target.subject();
        assert_eq!(*fork_subject, "temperature_rose");

        // 3. Historical truth is intact — the failure is still recorded.
        assert_eq!(
            *tl.at_exact(failure_t).unwrap().subject(),
            "constraint_violated"
        );
        assert_eq!(tl.len(), 7);
    }

    // ------------------------------------------------------------------
    // 13. Replay-start composition
    // ------------------------------------------------------------------

    #[test]
    fn composed_replay_start_selection() {
        // Replay needs a deterministic start point. Exact(t) is the
        // natural request; this test verifies that it lands on the
        // first entry at the requested instant.
        let tl = worked_timeline();
        let target = RewindRequest::Exact(ns(400_000_000)).resolve(&tl).unwrap();
        assert_eq!(*target.subject(), "constraint_violated");
        // Followed by the tie-broken entries via range — but that is the
        // replay module's job, not rewind.rs's.
    }

    // ------------------------------------------------------------------
    // 14. Rewind is not replay
    // ------------------------------------------------------------------

    #[test]
    fn rewind_returns_a_point_not_a_sequence() {
        // Type-level assertion: RewindTarget carries *one* borrowed
        // entry, not a collection. This is deliberately verified so
        // that future refactors cannot accidentally turn rewind into
        // replay.
        let tl = worked_timeline();
        let target = RewindRequest::Earliest.resolve(&tl).unwrap();
        let _entry: &TimelineEntry<&'static str> = target.entry();
        // If a refactor changed RewindTarget to hold multiple entries,
        // this test would no longer compile.
    }

    // ------------------------------------------------------------------
    // 15. Error Display coverage
    // ------------------------------------------------------------------

    #[test]
    fn errors_display_distinct_messages() {
        let msgs = [
            RewindError::EmptyTimeline.to_string(),
            RewindError::NoEntryAt { at: ns(1) }.to_string(),
            RewindError::NoEntryAtOrBefore { at: ns(1) }.to_string(),
            RewindError::NoEntryStrictlyBefore { at: ns(1) }.to_string(),
        ];
        // Every variant produces a non-empty message.
        for m in &msgs {
            assert!(!m.is_empty());
        }
        // And no two variants collide.
        for i in 0..msgs.len() {
            for j in 0..msgs.len() {
                if i != j {
                    assert_ne!(msgs[i], msgs[j]);
                }
            }
        }
    }
}