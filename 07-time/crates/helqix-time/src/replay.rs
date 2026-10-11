//! Deterministic replay orchestration for HELQIX.
//!
//! Replay in HELQIX is **deterministic reconstruction** of an already
//! recorded execution history. It is not "play the past again" — it is
//! the temporal driver that lets Layer 13 verification demonstrate that
//! an engineering execution reproduces.
//!
//! # What this module owns
//!
//! [`replay.rs`] is deliberately narrow. It owns:
//!
//! * **Planning** — deciding which slice of a [`Timeline`] to replay
//!   ([`ReplayPlan`], [`ReplayStart`], [`ReplayEnd`]).
//! * **Window resolution** — turning a plan into a borrow-only,
//!   zero-copy [`ReplayWindow`] over the timeline.
//! * **Temporal iteration** — walking the window in deterministic
//!   insertion/temporal order.
//! * **Divergence propagation** — stopping at the exact index and
//!   `SimTime` where the caller reported divergence, and never
//!   silently swallowing it.
//! * **Outcome reporting** — a strongly-typed [`ReplayOutcome`] suitable
//!   for verification.
//!
//! # What this module does NOT own
//!
//! * **Execution** — Layer 05. The per-step semantics are supplied by
//!   the caller as a closure (see below).
//! * **Simulation** — Layer 06. Invoked inside the caller's closure.
//! * **Authoritative state** — Layer 04. Held by the caller's closure.
//! * **Timeline ordering** — [`crate::timeline`].
//! * **Snapshot semantics** — [`crate::snapshot`].
//! * **Rewind navigation** — [`crate::rewind`].
//! * **Event causality** — Layer 10.
//! * **Verification logic** — Layer 13.
//! * **Mutation of the original history** — impossible by construction:
//!   every public entry point takes `&Timeline<T>`.
//!
//! # The callback design
//!
//! [`replay`] drives a `FnMut(&TimelineEntry<T>, &S) -> Result<ReplayStep<S, D>, E>`
//! callback. Each invocation represents one replay step:
//!
//! 1. The callback receives the *recorded* entry and the *current
//!    replayed* state.
//! 2. The callback re-executes the recorded transition — via Layer 05,
//!    Layer 06, or whatever orchestration the caller has wired — and
//!    returns either [`ReplayStep::Matched`] (with the newly-reached
//!    state) or [`ReplayStep::Diverged`] (with the state actually
//!    reached and a caller-supplied divergence detail).
//! 3. Errors from the callback abort the replay without masking the
//!    original error.
//!
//! Replay never invokes the callback out of temporal order, never
//! invokes it for entries outside the plan window, and never invokes
//! it after a divergence or an error.
//!
//! # Determinism
//!
//! Same timeline + same plan + same callback behaviour ⇒ same outcome,
//! byte-for-byte, in every run, process, and platform. No wall clock,
//! no randomness, no thread scheduling, no unordered collections, no
//! global state, no interior mutability, no `unsafe`.
//!
//! # Original-history immutability
//!
//! Every public function takes `&Timeline<T>` (or a [`ReplayWindow`]
//! which borrows it). The borrow checker therefore makes it
//! **impossible** for a replay to mutate, reorder, truncate, or
//! otherwise corrupt the recorded history. The caller's re-execution
//! produces a *separate* reconstructed state; the timeline itself is
//! untouched. This satisfies Invariant 5 ("original history remains
//! unchanged") and Invariant 11 ("branch replay cannot mutate another
//! branch or original history") at the type level.
//!
//! # Failure-workflow example
//!
//! ```text
//! Recorded timeline:
//!   t=0.00  initial
//!   t=0.10  motor_on
//!   t=0.20  velocity_changed
//!   t=0.30  temperature_rose
//!   t=0.40  constraint_violated
//!   t=0.40  failure_detected
//!
//! Replay plan: From(0.30) .. Through(0.40)
//! Outcome on reproduction: Completed — 3 entries replayed, final
//!   time = 0.40s.
//! Outcome on divergence at t=0.40: Diverged { at: 0.40s,
//!   at_index: 2, detail: <caller-supplied> }.
//! ```

use core::fmt;

use crate::clock::SimTime;
use crate::timeline::{Timeline, TimelineEntry};

// ============================================================================
// ReplayStart
// ============================================================================

/// Where a replay should begin within a timeline.
///
/// The two variants cover the two canonical HELQIX replay starts:
/// "from the beginning of recorded history" and "from a specific
/// temporal point onward" (e.g. from a snapshot, a rewind target, or a
/// branch point).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ReplayStart {
    /// Begin at the earliest recorded entry.
    Earliest,
    /// Begin at the earliest entry whose time is at or after `time`.
    ///
    /// Fails with [`ReplayError::NoStart`] if no such entry exists.
    From(SimTime),
}

// ============================================================================
// ReplayEnd
// ============================================================================

/// Where a replay should end within a timeline.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ReplayEnd {
    /// End at the latest recorded entry.
    Latest,
    /// End at the latest entry whose time is at or before `time`.
    ///
    /// Fails with [`ReplayError::NoEnd`] if no such entry exists.
    Through(SimTime),
}

// ============================================================================
// ReplayPlan
// ============================================================================

/// A declarative request to replay a range of a timeline.
///
/// A plan is a *specification*, not a borrowed view. It is `Copy` and
/// can be stored, compared, and re-resolved against different timelines
/// without allocation.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ReplayPlan {
    start: ReplayStart,
    end: ReplayEnd,
}

impl ReplayPlan {
    /// Construct a plan from a start and end specification.
    #[inline]
    #[must_use]
    pub const fn new(start: ReplayStart, end: ReplayEnd) -> Self {
        Self { start, end }
    }

    /// A plan that replays the entire timeline.
    #[inline]
    #[must_use]
    pub const fn full() -> Self {
        Self {
            start: ReplayStart::Earliest,
            end: ReplayEnd::Latest,
        }
    }

    /// A plan that replays from `time` to the end of the timeline.
    ///
    /// If `time` falls between two entries, the replay begins at the
    /// earliest entry at or after `time`.
    #[inline]
    #[must_use]
    pub const fn from(time: SimTime) -> Self {
        Self {
            start: ReplayStart::From(time),
            end: ReplayEnd::Latest,
        }
    }

    /// A plan that replays from the earliest entry through `time`.
    #[inline]
    #[must_use]
    pub const fn through(time: SimTime) -> Self {
        Self {
            start: ReplayStart::Earliest,
            end: ReplayEnd::Through(time),
        }
    }

    /// A plan that replays the inclusive temporal range `[from, to]`.
    #[inline]
    #[must_use]
    pub const fn range(from: SimTime, to: SimTime) -> Self {
        Self {
            start: ReplayStart::From(from),
            end: ReplayEnd::Through(to),
        }
    }

    /// The start specification.
    #[inline]
    #[must_use]
    pub const fn start(&self) -> ReplayStart {
        self.start
    }

    /// The end specification.
    #[inline]
    #[must_use]
    pub const fn end(&self) -> ReplayEnd {
        self.end
    }

    /// Resolve this plan against a timeline, producing a borrow-only
    /// [`ReplayWindow`] over the exact slice to replay.
    ///
    /// The timeline is never modified. On failure the timeline is
    /// unchanged and the caller receives a precise [`ReplayError`].
    pub fn resolve<'a, T>(
        &self,
        timeline: &'a Timeline<T>,
    ) -> Result<ReplayWindow<'a, T>, ReplayError> {
        if timeline.is_empty() {
            return Err(ReplayError::EmptyTimeline);
        }
        let len = timeline.len();

        // Start index: first entry in [start, ∞).
        let start_idx = match self.start {
            ReplayStart::Earliest => 0,
            ReplayStart::From(time) => timeline
                .iter()
                .position(|e| e.at() >= time)
                .ok_or(ReplayError::NoStart { at: time })?,
        };

        // End index (exclusive): first entry strictly after the last
        // entry in (-∞, end].
        let end_idx = match self.end {
            ReplayEnd::Latest => len,
            ReplayEnd::Through(time) => {
                let last = timeline
                    .iter()
                    .rposition(|e| e.at() <= time)
                    .ok_or(ReplayError::NoEnd { at: time })?;
                last + 1
            }
        };

        if start_idx >= end_idx {
            return Err(ReplayError::EmptyRange {
                requested_start: self.start,
                requested_end: self.end,
            });
        }

        debug_assert!(start_idx < end_idx);
        debug_assert!(end_idx <= len);

        Ok(ReplayWindow {
            timeline,
            start: start_idx,
            end: end_idx,
        })
    }
}

// ============================================================================
// ReplayWindow
// ============================================================================

/// A resolved, borrow-only view over a contiguous range of a timeline.
///
/// Produced exclusively by [`ReplayPlan::resolve`], which guarantees
/// the window is non-empty. A `ReplayWindow` never owns entries and
/// never copies them — it is a pair of indices plus a shared reference
/// to the timeline.
///
/// Because it only holds `&Timeline<T>`, iteration cannot corrupt the
/// recorded history.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ReplayWindow<'a, T> {
    timeline: &'a Timeline<T>,
    /// Inclusive start index.
    start: usize,
    /// Exclusive end index. `start < end` is guaranteed by construction.
    end: usize,
}

impl<'a, T> ReplayWindow<'a, T> {
    /// Number of entries in this window. Always `>= 1` (guaranteed by
    /// [`ReplayPlan::resolve`]).
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Always `false` for a window produced by [`ReplayPlan::resolve`].
    /// Provided so callers can satisfy `len()`/`is_empty()` lint pairs
    /// without an `unreachable!`.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// The first entry in the window.
    #[inline]
    #[must_use]
    pub fn first(&self) -> Option<&'a TimelineEntry<T>> {
        self.timeline.iter().nth(self.start)
    }

    /// The last entry in the window.
    #[inline]
    #[must_use]
    pub fn last(&self) -> Option<&'a TimelineEntry<T>> {
        if self.end == 0 {
            None
        } else {
            self.timeline.iter().nth(self.end - 1)
        }
    }

    /// The exact logical time of the first entry, if any.
    #[inline]
    #[must_use]
    pub fn start_time(&self) -> Option<SimTime> {
        self.first().map(|e| e.at())
    }

    /// The exact logical time of the last entry, if any.
    #[inline]
    #[must_use]
    pub fn end_time(&self) -> Option<SimTime> {
        self.last().map(|e| e.at())
    }

    /// Deterministic iteration in temporal (insertion) order.
    ///
    /// Yielded entries borrow from the underlying timeline for its full
    /// lifetime, not just for the duration of the `&self` borrow.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &'a TimelineEntry<T>> + 'a {
        self.timeline
            .iter()
            .skip(self.start)
            .take(self.end - self.start)
    }
}

impl<'a, T> IntoIterator for &'a ReplayWindow<'_, T> {
    type Item = &'a TimelineEntry<T>;
    type IntoIter = core::iter::Take<core::iter::Skip<core::slice::Iter<'a, TimelineEntry<T>>>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.timeline
            .iter()
            .skip(self.start)
            .take(self.end - self.start)
    }
}

// ============================================================================
// ReplayError
// ============================================================================

/// Reasons a [`ReplayPlan`] could not be resolved against a timeline.
///
/// Every variant is a normal, expected condition the caller must
/// handle explicitly — never a panic, never a silent clamp. A failed
/// resolve leaves the timeline unchanged.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ReplayError {
    /// The timeline contains no entries at all.
    EmptyTimeline,
    /// [`ReplayStart::From`] was requested but no entry exists at or
    /// after the requested time.
    NoStart { at: SimTime },
    /// [`ReplayEnd::Through`] was requested but no entry exists at or
    /// before the requested time.
    NoEnd { at: SimTime },
    /// The requested start and end points do not overlap any entries
    /// in the timeline.
    EmptyRange {
        requested_start: ReplayStart,
        requested_end: ReplayEnd,
    },
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTimeline => {
                f.write_str("cannot plan replay: timeline is empty")
            }
            Self::NoStart { at } => write!(
                f,
                "cannot plan replay: no entry at or after {}",
                at
            ),
            Self::NoEnd { at } => write!(
                f,
                "cannot plan replay: no entry at or before {}",
                at
            ),
            Self::EmptyRange {
                requested_start,
                requested_end,
            } => write!(
                f,
                "cannot plan replay: requested window ({:?} .. {:?}) contains no entries",
                requested_start, requested_end
            ),
        }
    }
}

impl std::error::Error for ReplayError {}

// ============================================================================
// ReplayStep
// ============================================================================

/// The outcome of one replayed step, as reported by the caller's
/// re-execution callback.
///
/// `Matched` transfers ownership of the newly-reached state back to
/// the replay driver, which passes it to the next step. `Diverged`
/// does the same but signals that the reconstructed result did **not**
/// agree with the recorded entry — replay stops immediately, and the
/// divergence is surfaced in [`ReplayOutcome::status`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ReplayStep<S, D> {
    /// The step reproduced cleanly. The state is the newly-reached
    /// reconstructed state.
    Matched(S),
    /// The step produced a divergent result. The state is the state
    /// actually reached (which may or may not equal the recorded one —
    /// that is precisely the caller's finding). `detail` carries
    /// caller-supplied divergence information.
    Diverged {
        /// The reconstructed state that was actually reached.
        state: S,
        /// Caller-supplied divergence information.
        detail: D,
    },
}

// ============================================================================
// ReplayStatus
// ============================================================================

/// Overall status of a completed replay run.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ReplayStatus<D, E> {
    /// Every entry in the window was replayed and matched.
    Completed,
    /// The caller's step function reported divergence at a specific
    /// entry. Replay stopped there.
    Diverged {
        /// Exact logical time of the diverging entry.
        at: SimTime,
        /// Zero-based index within the window of the diverging entry.
        at_index: usize,
        /// Caller-supplied divergence detail.
        detail: D,
    },
    /// The caller's step function returned an error before the replay
    /// could complete.
    Aborted {
        /// Exact logical time of the entry whose step failed.
        at: SimTime,
        /// Zero-based index within the window of the failing entry.
        at_index: usize,
        /// The caller-supplied error.
        error: E,
    },
}

// ============================================================================
// ReplayOutcome
// ============================================================================

/// Full outcome of a replay run.
///
/// Suitable for comparison by Layer 13 verification: two
/// [`ReplayOutcome`]s with equal `status`, `replayed`, and `final_time`
/// represent the same reconstruction of the same execution.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReplayOutcome<S, D, E> {
    status: ReplayStatus<D, E>,
    replayed: usize,
    total: usize,
    final_state: S,
    final_time: Option<SimTime>,
}

impl<S, D, E> ReplayOutcome<S, D, E> {
    /// The terminal status of the run.
    #[inline]
    #[must_use]
    pub fn status(&self) -> &ReplayStatus<D, E> {
        &self.status
    }

    /// Number of entries that were replayed and matched.
    #[inline]
    #[must_use]
    pub fn replayed(&self) -> usize {
        self.replayed
    }

    /// Total number of entries in the window.
    #[inline]
    #[must_use]
    pub fn total(&self) -> usize {
        self.total
    }

    /// The reconstructed state at the point where replay stopped.
    #[inline]
    #[must_use]
    pub fn final_state(&self) -> &S {
        &self.final_state
    }

    /// The logical time of the last entry that was replayed (or, on
    /// divergence, the time of the diverging entry).
    ///
    /// `None` only when the window was empty, which
    /// [`ReplayPlan::resolve`] makes impossible — so in practice this
    /// is always `Some`.
    #[inline]
    #[must_use]
    pub fn final_time(&self) -> Option<SimTime> {
        self.final_time
    }

    /// Consume the outcome and take ownership of the final state.
    #[inline]
    #[must_use]
    pub fn into_final_state(self) -> S {
        self.final_state
    }

    /// Did every entry replay cleanly?
    #[inline]
    #[must_use]
    pub fn is_completed(&self) -> bool {
        matches!(self.status, ReplayStatus::Completed)
    }

    /// Did the replay diverge from the recorded history?
    #[inline]
    #[must_use]
    pub fn is_diverged(&self) -> bool {
        matches!(self.status, ReplayStatus::Diverged { .. })
    }

    /// Was the replay aborted by a caller-side error?
    #[inline]
    #[must_use]
    pub fn is_aborted(&self) -> bool {
        matches!(self.status, ReplayStatus::Aborted { .. })
    }
}

// ============================================================================
// replay
// ============================================================================

/// Drive a deterministic replay of the entries in `window`.
///
/// For each entry in `window`, in temporal (insertion) order, invokes
/// `step(entry, &current_state)`. Depending on the returned
/// [`ReplayStep`]:
///
/// * [`ReplayStep::Matched`] — the newly-reached state replaces
///   `current_state`, and iteration proceeds.
/// * [`ReplayStep::Diverged`] — replay stops immediately; the outcome
///   records the exact entry (time and index) and the caller-supplied
///   divergence detail.
///
/// If `step` returns `Err(e)`, replay stops immediately and the outcome
/// records the exact entry and the error. Replay never silently
/// swallows a divergence or an error.
///
/// # Determinism
///
/// Given the same window and a `step` function whose behaviour is a
/// pure function of its arguments, the outcome is identical across
/// runs, processes, and platforms.
///
/// # Original-history protection
///
/// Takes `&ReplayWindow<'_, T>`, which itself borrows `&Timeline<T>`.
/// The original history cannot be mutated by this function.
///
/// # Non-empty guarantee
///
/// `window` is guaranteed non-empty by [`ReplayPlan::resolve`], its
/// only constructor. In the (unreachable) empty case, the outcome is
/// `Completed` with `replayed == 0` — but that condition cannot occur
/// through the public API.
pub fn replay<T, S, D, E, F>(
    window: &ReplayWindow<'_, T>,
    initial: S,
    mut step: F,
) -> ReplayOutcome<S, D, E>
where
    F: FnMut(&TimelineEntry<T>, &S) -> Result<ReplayStep<S, D>, E>,
{
    let total = window.len();
    let mut state = initial;
    let mut replayed: usize = 0;
    let mut final_time: Option<SimTime> = None;

    for (idx, entry) in window.iter().enumerate() {
        match step(entry, &state) {
            Ok(ReplayStep::Matched(next)) => {
                state = next;
                replayed += 1;
                final_time = Some(entry.at());
            }
            Ok(ReplayStep::Diverged {
                state: next,
                detail,
            }) => {
                return ReplayOutcome {
                    status: ReplayStatus::Diverged {
                        at: entry.at(),
                        at_index: idx,
                        detail,
                    },
                    replayed,
                    total,
                    final_state: next,
                    final_time: Some(entry.at()),
                };
            }
            Err(error) => {
                return ReplayOutcome {
                    status: ReplayStatus::Aborted {
                        at: entry.at(),
                        at_index: idx,
                        error,
                    },
                    replayed,
                    total,
                    final_state: state,
                    final_time,
                };
            }
        }
    }

    ReplayOutcome {
        status: ReplayStatus::Completed,
        replayed,
        total,
        final_state: state,
        final_time,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::SimTime;
    use crate::timeline::Timeline;

    fn ns(n: i128) -> SimTime {
        SimTime::from_nanos(n)
    }

    /// The standard Layer-07 worked-example timeline.
    fn sample() -> Timeline<&'static str> {
        let mut tl = Timeline::new();
        tl.record_at(ns(0), "initial").unwrap();
        tl.record_at(ns(100), "motor_on").unwrap();
        tl.record_at(ns(200), "velocity_changed").unwrap();
        tl.record_at(ns(300), "temperature_rose").unwrap();
        tl.record_at(ns(400), "constraint_violated").unwrap();
        tl.record_at(ns(400), "failure_detected").unwrap();
        tl
    }

    // ------------------------------------------------------------------
    // 1. Plan resolution — errors
    // ------------------------------------------------------------------

    #[test]
    fn resolve_rejects_empty_timeline() {
        let tl: Timeline<u32> = Timeline::new();
        assert_eq!(
            ReplayPlan::full().resolve(&tl).unwrap_err(),
            ReplayError::EmptyTimeline
        );
    }

    #[test]
    fn resolve_from_past_end_fails() {
        let tl = sample();
        // No entry at or after SimTime::MAX.
        assert_eq!(
            ReplayPlan::from(SimTime::MAX).resolve(&tl).unwrap_err(),
            ReplayError::NoStart { at: SimTime::MAX }
        );
    }

    #[test]
    fn resolve_through_before_start_fails() {
        let tl = sample();
        // No entry at or before SimTime::MIN.
        assert_eq!(
            ReplayPlan::through(SimTime::MIN).resolve(&tl).unwrap_err(),
            ReplayError::NoEnd { at: SimTime::MIN }
        );
    }

    #[test]
    fn resolve_empty_range_fails() {
        let tl = sample();
        // From(300) resolves to index 3; Through(100) resolves to
        // index 1 (exclusive 2). No overlap.
        let err = ReplayPlan::range(ns(300), ns(100))
            .resolve(&tl)
            .unwrap_err();
        assert!(matches!(err, ReplayError::EmptyRange { .. }));
    }

    // ------------------------------------------------------------------
    // 2. Plan resolution — successful windows
    // ------------------------------------------------------------------

    #[test]
    fn resolve_full_covers_entire_timeline() {
        let tl = sample();
        let w = ReplayPlan::full().resolve(&tl).unwrap();
        assert_eq!(w.len(), 6);
        assert_eq!(w.start_time(), Some(ns(0)));
        assert_eq!(w.end_time(), Some(ns(400)));
        assert_eq!(*w.first().unwrap().subject(), "initial");
        assert_eq!(*w.last().unwrap().subject(), "failure_detected");
    }

    #[test]
    fn resolve_from_starts_at_or_after_requested_time() {
        let tl = sample();
        // 250ns falls between entries at 200 and 300 → start at 300.
        let w = ReplayPlan::from(ns(250)).resolve(&tl).unwrap();
        assert_eq!(w.start_time(), Some(ns(300)));
        assert_eq!(w.end_time(), Some(ns(400)));
        assert_eq!(w.len(), 3); // 300, 400, 400
    }

    #[test]
    fn resolve_through_ends_at_or_before_requested_time() {
        let tl = sample();
        // 350ns falls between entries at 300 and 400 → end at 300.
        let w = ReplayPlan::through(ns(350)).resolve(&tl).unwrap();
        assert_eq!(w.start_time(), Some(ns(0)));
        assert_eq!(w.end_time(), Some(ns(300)));
        assert_eq!(w.len(), 4);
    }

    #[test]
    fn resolve_range_bounds_both_ends() {
        let tl = sample();
        let w = ReplayPlan::range(ns(100), ns(300)).resolve(&tl).unwrap();
        let subjects: Vec<&str> = w.iter().map(|e| *e.subject()).collect();
        assert_eq!(
            subjects,
            vec!["motor_on", "velocity_changed", "temperature_rose"]
        );
    }

    #[test]
    fn resolve_range_spanning_exact_ties_is_inclusive() {
        // All three entries at 400ns must be included when the range
        // ends at 400.
        let mut tl = Timeline::new();
        tl.record_at(ns(0), "a").unwrap();
        tl.record_at(ns(400), "b").unwrap();
        tl.record_at(ns(400), "c").unwrap();

        let w = ReplayPlan::range(ns(0), ns(400)).resolve(&tl).unwrap();
        let subjects: Vec<&str> = w.iter().map(|e| *e.subject()).collect();
        assert_eq!(subjects, vec!["a", "b", "c"]);
    }

    #[test]
    fn resolve_range_starting_at_a_tie_begins_at_first_tie() {
        let mut tl = Timeline::new();
        tl.record_at(ns(0), "a").unwrap();
        tl.record_at(ns(400), "b").unwrap();
        tl.record_at(ns(400), "c").unwrap();

        let w = ReplayPlan::range(ns(400), ns(400)).resolve(&tl).unwrap();
        let subjects: Vec<&str> = w.iter().map(|e| *e.subject()).collect();
        assert_eq!(subjects, vec!["b", "c"]);
    }

    // ------------------------------------------------------------------
    // 3. Replay driver — success path
    // ------------------------------------------------------------------

    #[test]
    fn replay_full_completes_and_reaches_final_state() {
        let tl = sample();
        let w = ReplayPlan::full().resolve(&tl).unwrap();

        // The callback echoes each entry's subject back as the new state.
        let outcome: ReplayOutcome<&'static str, (), ()> =
            replay(&w, "start", |entry, _state| {
                Ok(ReplayStep::Matched(*entry.subject()))
            });

        assert!(outcome.is_completed());
        assert_eq!(outcome.replayed(), 6);
        assert_eq!(outcome.total(), 6);
        assert_eq!(outcome.final_time(), Some(ns(400)));
        assert_eq!(*outcome.final_state(), "failure_detected");
    }

    #[test]
    fn replay_visits_entries_in_temporal_order() {
        use std::cell::RefCell;
        let tl = sample();
        let w = ReplayPlan::full().resolve(&tl).unwrap();

        let visited: RefCell<Vec<&'static str>> = RefCell::new(Vec::new());
        let outcome: ReplayOutcome<usize, (), ()> = replay(&w, 0usize, |entry, n| {
            visited.borrow_mut().push(*entry.subject());
            Ok(ReplayStep::Matched(*n + 1))
        });

        assert!(outcome.is_completed());
        assert_eq!(
            *visited.borrow(),
            vec![
                "initial",
                "motor_on",
                "velocity_changed",
                "temperature_rose",
                "constraint_violated",
                "failure_detected",
            ]
        );
    }

    #[test]
    fn replay_window_bounds_restrict_which_entries_are_visited() {
        use std::cell::RefCell;
        let tl = sample();
        let w = ReplayPlan::range(ns(200), ns(400)).resolve(&tl).unwrap();

        let visited: RefCell<Vec<SimTime>> = RefCell::new(Vec::new());
        let outcome: ReplayOutcome<usize, (), ()> = replay(&w, 0usize, |entry, n| {
            visited.borrow_mut().push(entry.at());
            Ok(ReplayStep::Matched(*n + 1))
        });

        assert!(outcome.is_completed());
        assert_eq!(outcome.replayed(), 4);
        assert_eq!(
            *visited.borrow(),
            vec![ns(200), ns(300), ns(400), ns(400)]
        );
    }

    // ------------------------------------------------------------------
    // 4. Replay driver — divergence
    // ------------------------------------------------------------------

    #[test]
    fn divergence_stops_at_exact_point_and_is_surfaced() {
        use std::cell::RefCell;
        let tl = sample();
        let w = ReplayPlan::full().resolve(&tl).unwrap();

        let visited: RefCell<usize> = RefCell::new(0);
        let outcome: ReplayOutcome<usize, &'static str, ()> =
            replay(&w, 0usize, |entry, n| {
                *visited.borrow_mut() += 1;
                if *entry.subject() == "constraint_violated" {
                    Ok(ReplayStep::Diverged {
                        state: *n + 1,
                        detail: "failure reproduced at a different instant",
                    })
                } else {
                    Ok(ReplayStep::Matched(*n + 1))
                }
            });

        assert!(outcome.is_diverged());
        assert_eq!(outcome.replayed(), 4);
        assert_eq!(outcome.total(), 6);
        assert_eq!(outcome.final_time(), Some(ns(400)));
        match outcome.status() {
            ReplayStatus::Diverged {
                at,
                at_index,
                detail,
            } => {
                assert_eq!(*at, ns(400));
                assert_eq!(*at_index, 4);
                assert_eq!(*detail, "failure reproduced at a different instant");
            }
            other => panic!("expected Diverged, got {:?}", other),
        }
        // The callback was NOT invoked for the entry after divergence.
        assert_eq!(*visited.borrow(), 5);
    }

    // ------------------------------------------------------------------
    // 5. Replay driver — caller error
    // ------------------------------------------------------------------

    #[test]
    fn abort_propagates_caller_error_at_exact_point() {
        let tl = sample();
        let w = ReplayPlan::full().resolve(&tl).unwrap();

        let outcome: ReplayOutcome<usize, (), &'static str> =
            replay(&w, 0usize, |entry, n| {
                if *entry.subject() == "velocity_changed" {
                    Err("simulation dependency missing")
                } else {
                    Ok(ReplayStep::Matched(*n + 1))
                }
            });

        assert!(outcome.is_aborted());
        assert_eq!(outcome.replayed(), 2);
        match outcome.status() {
            ReplayStatus::Aborted {
                at,
                at_index,
                error,
            } => {
                assert_eq!(*at, ns(200));
                assert_eq!(*at_index, 2);
                assert_eq!(*error, "simulation dependency missing");
            }
            other => panic!("expected Aborted, got {:?}", other),
        }
        // Final state is the one reached before the failing step.
        assert_eq!(*outcome.final_state(), 2usize);
    }

    // ------------------------------------------------------------------
    // 6. Original-history immutability

    #[test]
    fn replay_does_not_mutate_original_timeline() {
        let tl = sample();
        let original = tl.clone();
        let window = ReplayPlan::full().resolve(&tl).unwrap();
        let outcome = replay(&window, 0usize, |_, state| {
            Ok::<_, ()>(ReplayStep::<usize, ()>::Matched(*state + 1))
        });

        assert!(outcome.is_completed());
        assert_eq!(outcome.replayed(), original.len());
        assert_eq!(tl, original);
    }
}
