//! Deterministic execution ordering for HELQIX Layer 05 — Execution.
//!
//! A [`Scheduler`] holds a set of [`Action`]s awaiting execution and
//! answers one question, deterministically:
//!
//! > "Given the current logical time, which pending action should be
//! >  selected next?"
//!
//! It does not execute. It does not run physics. It does not mutate
//! state. It does not own time. It does not own the action — it
//! borrows it, orders it, and hands it to the executor.
//!
//! # Responsibility boundary
//!
//! `scheduler.rs` owns **only**:
//!
//! * the set of pending scheduled actions,
//! * the deterministic ordering rule `(scheduled_at, ActionId)`,
//! * selection of the earliest eligible action,
//! * duplicate-`ActionId` rejection,
//! * consumption semantics via
//!   [`Scheduler::take_next_eligible`].
//!
//! It explicitly does **not** own:
//!
//! * execution — [`crate::executor`],
//! * state transitions — [`crate::transition`] and Layer 04,
//! * physical simulation — Layer 06,
//! * logical time — Layer 07 (consumed as [`helqix_time::SimTime`]),
//! * dependency ordering — [`crate::execution_graph`] (future),
//! * the runtime loop — [`crate::runtime`].
//!
//! # Ordering
//!
//! Pending actions are held in strictly ascending
//! `(scheduled_at, ActionId)` order. When two actions share the same
//! scheduled time, the lower `ActionId` is selected first. This is the
//! minimal deterministic rule justified by the current Layer 05
//! contract:
//!
//! * `scheduled_at` — the caller-supplied logical time at which the
//!   action becomes eligible, obtained from Layer 07's
//!   [`Clock`](helqix_time::Clock).
//! * `ActionId` — the stable action identity from
//!   [`crate::action::ActionId`], which the caller is responsible for
//!   making unique within the scheduler.
//!
//! No other ordering dimension (priority, dependency depth, insertion
//! sequence) is present, because no Layer 05 contract currently
//! defines one. When `execution_graph.rs` establishes dependency
//! ordering, it will be added here as an additional dimension.
//!
//! # Determinism
//!
//! Same pending set + same `now` ⇒ same selected action, in every run,
//! every process, every platform. No `HashMap` iteration, no wall
//! clock, no thread scheduling, no randomness, no global state, no
//! interior mutability, no `unsafe`.
//!
//! # Time boundary
//!
//! The scheduler **stores** `SimTime` values but **never** creates,
//! advances, or reads them from a global source. The caller supplies
//! every time — typically `clock.now()` — both at `schedule` and at
//! `next_eligible` / `take_next_eligible`. Layer 07 remains the sole
//! time authority.
//!
//! # Lifecycle
//!
//! ```text
//!     Created        Action::try_new validates the payload
//!       ↓
//!     Scheduled      Scheduler::schedule places it in the pending set
//!       ↓
//!     Eligible       scheduled_at <= now
//!       ↓
//!     Selected       Scheduler::take_next_eligible removes it
//!       ↓
//!     Executed       ← by the Executor, not by this file
//! ```
//!
//! Selection by the scheduler is **not** execution success. That
//! distinction is preserved deliberately for failure analysis,
//! replay, and verification.

use core::fmt;

use crate::action::{Action, ActionId, ActionPayload};
use helqix_time::SimTime;

// ============================================================================
// ScheduledAction
// ============================================================================

/// An [`Action`] paired with the logical time at which it becomes
/// eligible for execution.
///
/// Produced by [`Scheduler::schedule`] and returned by
/// [`Scheduler::next_eligible`] / [`Scheduler::take_next_eligible`].
///
/// The scheduled time is a caller-supplied [`SimTime`] obtained from
/// Layer 07. It is stored by the scheduler but never created or
/// advanced by it — Layer 07 remains the sole time authority.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScheduledAction<P: ActionPayload> {
    scheduled_at: SimTime,
    action: Action<P>,
}

impl<P: ActionPayload> ScheduledAction<P> {
    /// The logical time at which this action becomes eligible.
    #[inline]
    #[must_use]
    pub fn scheduled_at(&self) -> SimTime {
        self.scheduled_at
    }

    /// The scheduled action.
    #[inline]
    #[must_use]
    pub fn action(&self) -> &Action<P> {
        &self.action
    }

    /// Consume the scheduled action and return the underlying action,
    /// discarding the scheduled time.
    #[inline]
    #[must_use]
    pub fn into_action(self) -> Action<P> {
        self.action
    }

    /// Consume the scheduled action and return both parts.
    #[inline]
    #[must_use]
    pub fn into_parts(self) -> (SimTime, Action<P>) {
        (self.scheduled_at, self.action)
    }
}

// ============================================================================
// ScheduleError
// ============================================================================

/// Errors raised by [`Scheduler::schedule`].
///
/// Deliberately minimal: the only structural failure the scheduler
/// itself can detect is duplicate identity. Action-validity failures
/// are caught earlier, by [`Action::try_new`]; execution-time
/// eligibility failures are a concern of the executor, not the
/// scheduler.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ScheduleError {
    /// An action with the same [`ActionId`] is already pending in the
    /// scheduler. The scheduler rejects the second scheduling attempt
    /// and leaves its pending set unchanged.
    DuplicateId { id: ActionId },
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId { id } => {
                write!(f, "action {:?} is already scheduled", id)
            }
        }
    }
}

impl std::error::Error for ScheduleError {}

// ============================================================================
// Scheduler
// ============================================================================

/// A deterministic set of pending [`ScheduledAction`]s.
///
/// See the module documentation for the full semantic model.
#[derive(Debug, Clone)]
pub struct Scheduler<P: ActionPayload> {
    /// Pending actions, kept in strictly ascending `(scheduled_at,
    /// ActionId)` order. The front is always the earliest-eligible
    /// action, so a single front-lookup answers the eligibility
    /// question for any `now`.
    pending: Vec<ScheduledAction<P>>,
}

impl<P: ActionPayload> Scheduler<P> {
    /// Create an empty scheduler.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    /// Number of pending actions.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Is the scheduler empty?
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Is an action with the given identity currently pending?
    #[inline]
    #[must_use]
    pub fn contains(&self, id: ActionId) -> bool {
        self.pending.iter().any(|s| s.action.id() == id)
    }

    /// Insert an action into the pending set, scheduled to become
    /// eligible at `at`.
    ///
    /// The pending set remains in ascending `(scheduled_at, ActionId)`
    /// order. Duplicate [`ActionId`]s are rejected with
    /// [`ScheduleError::DuplicateId`] and leave the scheduler
    /// unchanged.
    pub fn schedule(&mut self, action: Action<P>, at: SimTime) -> Result<(), ScheduleError> {
        let id = action.id();
        if self.pending.iter().any(|s| s.action.id() == id) {
            return Err(ScheduleError::DuplicateId { id });
        }
        let pos = self
            .pending
            .partition_point(|s| (s.scheduled_at, s.action.id()) < (at, id));
        self.pending.insert(
            pos,
            ScheduledAction {
                scheduled_at: at,
                action,
            },
        );
        Ok(())
    }

    /// The earliest pending action whose `scheduled_at` is at or
    /// before `now`, without removing it.
    ///
    /// Returns `None` if no such action exists.
    #[inline]
    #[must_use]
    pub fn next_eligible(&self, now: SimTime) -> Option<&ScheduledAction<P>> {
        self.pending.first().filter(|s| s.scheduled_at <= now)
    }

    /// Remove and return the earliest pending action whose
    /// `scheduled_at` is at or before `now`.
    ///
    /// Returns `None` and leaves the scheduler unchanged if no action
    /// is eligible.
    ///
    /// # Selection is not execution
    ///
    /// A successful return means the scheduler has handed the action
    /// to the caller. It does **not** mean the action was executed
    /// successfully — that determination belongs to the executor and
    /// to Layer 04.
    pub fn take_next_eligible(&mut self, now: SimTime) -> Option<ScheduledAction<P>> {
        if self
            .pending
            .first()
            .map_or(false, |s| s.scheduled_at <= now)
        {
            Some(self.pending.remove(0))
        } else {
            None
        }
    }

    /// Iterate over pending actions in deterministic temporal order.
    ///
    /// The yielded order is exactly `(scheduled_at, ActionId)`
    /// ascending. Two schedulers holding equal pending sets yield
    /// identical iteration sequences.
    #[inline]
    pub fn iter(&self) -> core::slice::Iter<'_, ScheduledAction<P>> {
        self.pending.iter()
    }
}

impl<P: ActionPayload> Default for Scheduler<P> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, P: ActionPayload> IntoIterator for &'a Scheduler<P> {
    type Item = &'a ScheduledAction<P>;
    type IntoIter = core::slice::Iter<'a, ScheduledAction<P>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.pending.iter()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionError, ActionId};

    // A representative test payload. Layer 05 knows nothing about it.
    #[derive(Debug, Clone, Eq, PartialEq)]
    struct TestPayload(u32);

    impl ActionPayload for TestPayload {
        fn validate(&self) -> Result<(), ActionError> {
            Ok(())
        }
    }

    fn action(id: u64, v: u32) -> Action<TestPayload> {
        Action::try_new(ActionId::from_raw(id), TestPayload(v)).unwrap()
    }

    fn ns(n: i128) -> SimTime {
        SimTime::from_nanos(n)
    }

    // ------------------------------------------------------------------
    // 1. Empty scheduler
    // ------------------------------------------------------------------

    #[test]
    fn empty_scheduler_behaves_uniformly() {
        let s: Scheduler<TestPayload> = Scheduler::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert!(s.next_eligible(ns(0)).is_none());
        assert!(s.next_eligible(SimTime::MAX).is_none());
        assert!(s.iter().next().is_none());
        assert!(!s.contains(ActionId::from_raw(1)));
    }

    #[test]
    fn default_is_empty() {
        let s: Scheduler<TestPayload> = Scheduler::default();
        assert!(s.is_empty());
    }

    // ------------------------------------------------------------------
    // 2. Scheduling a valid action
    // ------------------------------------------------------------------

    #[test]
    fn schedule_then_pending() {
        let mut s = Scheduler::new();
        s.schedule(action(1, 10), ns(100)).unwrap();
        assert!(!s.is_empty());
        assert_eq!(s.len(), 1);
        assert!(s.contains(ActionId::from_raw(1)));
        assert!(!s.contains(ActionId::from_raw(2)));
    }

    #[test]
    fn scheduled_action_preserves_id_and_time() {
        let mut s = Scheduler::new();
        s.schedule(action(42, 7), ns(12_345)).unwrap();
        let taken = s.take_next_eligible(SimTime::MAX).unwrap();
        assert_eq!(taken.action().id().raw(), 42);
        assert_eq!(taken.scheduled_at(), ns(12_345));
        assert_eq!(taken.action().payload(), &TestPayload(7));
    }

    #[test]
    fn into_parts_yields_time_and_action() {
        let mut s = Scheduler::new();
        s.schedule(action(42, 7), ns(12_345)).unwrap();
        let taken = s.take_next_eligible(SimTime::MAX).unwrap();
        let (time, act) = taken.into_parts();
        assert_eq!(time, ns(12_345));
        assert_eq!(act.id().raw(), 42);
    }

    #[test]
    fn into_action_discards_time() {
        let mut s = Scheduler::new();
        s.schedule(action(42, 7), ns(12_345)).unwrap();
        let taken = s.take_next_eligible(SimTime::MAX).unwrap();
        let act = taken.into_action();
        assert_eq!(act.id().raw(), 42);
    }

    // ------------------------------------------------------------------
    // 3. Deterministic ordering — time then id
    // ------------------------------------------------------------------

    #[test]
    fn ordering_is_by_time_then_id() {
        let mut s = Scheduler::new();
        s.schedule(action(2, 20), ns(100)).unwrap();
        s.schedule(action(1, 10), ns(100)).unwrap(); // same time, lower id
        s.schedule(action(3, 30), ns(200)).unwrap();

        let order: Vec<u64> = s.iter().map(|sa| sa.action().id().raw()).collect();
        assert_eq!(order, vec![1, 2, 3]);
    }

    #[test]
    fn ordering_is_independent_of_insertion_sequence() {
        // Insert in reverse order and confirm sorted structure.
        let mut s = Scheduler::new();
        s.schedule(action(3, 30), ns(300)).unwrap();
        s.schedule(action(1, 10), ns(100)).unwrap();
        s.schedule(action(2, 20), ns(200)).unwrap();

        let order: Vec<u64> = s.iter().map(|sa| sa.action().id().raw()).collect();
        assert_eq!(order, vec![1, 2, 3]);
    }

    // ------------------------------------------------------------------
    // 4. Eligibility
    // ------------------------------------------------------------------

    #[test]
    fn eligibility_is_by_now_time() {
        let mut s = Scheduler::new();
        s.schedule(action(1, 10), ns(100)).unwrap();
        s.schedule(action(2, 20), ns(200)).unwrap();

        // Before the first scheduled time: nothing eligible.
        assert!(s.next_eligible(ns(50)).is_none());

        // At the first scheduled time: action 1 eligible.
        assert_eq!(s.next_eligible(ns(100)).unwrap().action().id().raw(), 1);

        // Between: still action 1 (earliest).
        assert_eq!(s.next_eligible(ns(150)).unwrap().action().id().raw(), 1);

        // At the second scheduled time: still action 1 (earliest first).
        assert_eq!(s.next_eligible(ns(200)).unwrap().action().id().raw(), 1);

        // Far future: still action 1.
        assert_eq!(
            s.next_eligible(SimTime::MAX).unwrap().action().id().raw(),
            1
        );
    }

    #[test]
    fn eligibility_boundary_is_inclusive() {
        let mut s = Scheduler::new();
        s.schedule(action(1, 10), ns(100)).unwrap();
        // Exactly at scheduled_at: eligible.
        assert!(s.next_eligible(ns(100)).is_some());
        // One nanosecond before: not eligible.
        assert!(s.next_eligible(ns(99)).is_none());
    }

    // ------------------------------------------------------------------
    // 5. Consumption
    // ------------------------------------------------------------------

    #[test]
    fn take_removes_action() {
        let mut s = Scheduler::new();
        s.schedule(action(1, 10), ns(100)).unwrap();
        assert_eq!(s.len(), 1);

        let taken = s.take_next_eligible(ns(100)).unwrap();
        assert_eq!(taken.action().id().raw(), 1);
        assert_eq!(taken.scheduled_at(), ns(100));
        assert!(s.is_empty());
    }

    #[test]
    fn take_is_noop_when_not_eligible() {
        let mut s = Scheduler::new();
        s.schedule(action(1, 10), ns(100)).unwrap();

        assert!(s.take_next_eligible(ns(50)).is_none());
        assert_eq!(s.len(), 1); // unchanged
    }

    #[test]
    fn take_reveals_next_after_consumption() {
        let mut s = Scheduler::new();
        s.schedule(action(1, 10), ns(100)).unwrap();
        s.schedule(action(2, 20), ns(200)).unwrap();

        let a = s.take_next_eligible(SimTime::MAX).unwrap();
        assert_eq!(a.action().id().raw(), 1);
        let b = s.take_next_eligible(SimTime::MAX).unwrap();
        assert_eq!(b.action().id().raw(), 2);
        assert!(s.take_next_eligible(SimTime::MAX).is_none());
    }

    // ------------------------------------------------------------------
    // 6. Duplicate rejection
    // ------------------------------------------------------------------

    #[test]
    fn duplicate_id_is_rejected() {
        let mut s = Scheduler::new();
        s.schedule(action(1, 10), ns(100)).unwrap();

        let err = s.schedule(action(1, 20), ns(200)).unwrap_err();
        assert_eq!(
            err,
            ScheduleError::DuplicateId {
                id: ActionId::from_raw(1)
            }
        );
        // Original remains untouched.
        assert_eq!(s.len(), 1);
        assert_eq!(s.iter().next().unwrap().action().id().raw(), 1);
    }

    #[test]
    fn duplicate_id_is_rejected_regardless_of_time() {
        let mut s = Scheduler::new();
        s.schedule(action(1, 10), ns(100)).unwrap();
        // Same id, earlier time → still rejected.
        assert!(s.schedule(action(1, 10), ns(50)).is_err());
        // Same id, later time → still rejected.
        assert!(s.schedule(action(1, 10), ns(500)).is_err());
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn error_display_is_informative() {
        let err = ScheduleError::DuplicateId {
            id: ActionId::from_raw(7),
        };
        let s = err.to_string();
        assert!(s.contains("7"), "got: {}", s);
        // And it implements std::error::Error.
        let _boxed: Box<dyn std::error::Error> = Box::new(err);
    }

    // ------------------------------------------------------------------
    // 7. Determinism: repeated identical inputs produce identical output
    // ------------------------------------------------------------------

    #[test]
    fn deterministic_order_across_instances() {
        let build = || {
            let mut s = Scheduler::new();
            s.schedule(action(3, 30), ns(200)).unwrap();
            s.schedule(action(1, 10), ns(100)).unwrap();
            s.schedule(action(2, 20), ns(100)).unwrap();
            s.schedule(action(4, 40), ns(300)).unwrap();
            s
        };

        let a = build();
        let b = build();
        let c = build();

        let ids_a: Vec<u64> = a.iter().map(|sa| sa.action().id().raw()).collect();
        let ids_b: Vec<u64> = b.iter().map(|sa| sa.action().id().raw()).collect();
        let ids_c: Vec<u64> = c.iter().map(|sa| sa.action().id().raw()).collect();

        assert_eq!(ids_a, ids_b);
        assert_eq!(ids_b, ids_c);
        assert_eq!(ids_a, vec![1, 2, 3, 4]);
    }

    #[test]
    fn repeated_takes_yield_same_sequence() {
        let build = || {
            let mut s = Scheduler::new();
            s.schedule(action(3, 30), ns(300)).unwrap();
            s.schedule(action(1, 10), ns(100)).unwrap();
            s.schedule(action(2, 20), ns(200)).unwrap();
            s
        };

        let drain = |mut s: Scheduler<TestPayload>| -> Vec<u64> {
            let mut out = Vec::new();
            while let Some(taken) = s.take_next_eligible(SimTime::MAX) {
                out.push(taken.action().id().raw());
            }
            out
        };

        let a = drain(build());
        let b = drain(build());
        let c = drain(build());

        assert_eq!(a, vec![1, 2, 3]);
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    // ------------------------------------------------------------------
    // 8. Boundary conditions
    // ------------------------------------------------------------------

    #[test]
    fn extreme_sim_time_boundaries_are_handled() {
        let mut s = Scheduler::new();
        s.schedule(action(1, 1), SimTime::MIN).unwrap();
        s.schedule(action(2, 2), ns(0)).unwrap();
        s.schedule(action(3, 3), SimTime::MAX).unwrap();

        let order: Vec<u64> = s.iter().map(|sa| sa.action().id().raw()).collect();
        assert_eq!(order, vec![1, 2, 3]);

        // At SimTime::MIN only the first is eligible.
        assert_eq!(
            s.take_next_eligible(SimTime::MIN).unwrap().action().id().raw(),
            1
        );
        // At ns(0) the second becomes eligible.
        assert_eq!(s.take_next_eligible(ns(0)).unwrap().action().id().raw(), 2);
        // Nothing else until SimTime::MAX.
        assert!(s.take_next_eligible(ns(0)).is_none());
        assert_eq!(
            s.take_next_eligible(SimTime::MAX).unwrap().action().id().raw(),
            3
        );
    }

    #[test]
    fn maximum_and_minimum_action_ids_are_supported() {
        let mut s = Scheduler::new();
        s.schedule(action(u64::MAX, 1), ns(1)).unwrap();
        s.schedule(action(0, 2), ns(1)).unwrap();

        // Same time → lower id first.
        let order: Vec<u64> = s.iter().map(|sa| sa.action().id().raw()).collect();
        assert_eq!(order, vec![0, u64::MAX]);
    }

    // ------------------------------------------------------------------
    // 9. Domain independence — two payload types coexist
    // ------------------------------------------------------------------

    #[test]
    fn schedulers_of_different_payload_types_coexist() {
        #[derive(Debug, Clone, Eq, PartialEq)]
        struct OtherPayload;
        impl ActionPayload for OtherPayload {
            fn validate(&self) -> Result<(), ActionError> {
                Ok(())
            }
        }

        let mut a: Scheduler<TestPayload> = Scheduler::new();
        a.schedule(action(1, 1), ns(1)).unwrap();

        let mut b: Scheduler<OtherPayload> = Scheduler::new();
        b.schedule(
            Action::try_new(ActionId::from_raw(1), OtherPayload).unwrap(),
            ns(1),
        )
        .unwrap();

        // Independent instance state.
        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 1);
        assert!(a.take_next_eligible(ns(1)).is_some());
        assert_eq!(b.len(), 1);
    }

    // ------------------------------------------------------------------
    // 10. &Scheduler iterates like Timeline in Layer 07
    // ------------------------------------------------------------------

    #[test]
    fn into_iterator_for_shared_ref_agrees_with_iter() {
        let mut s = Scheduler::new();
        s.schedule(action(1, 1), ns(100)).unwrap();
        s.schedule(action(2, 2), ns(200)).unwrap();

        let from_iter: Vec<u64> = s.iter().map(|sa| sa.action().id().raw()).collect();
        let from_into_iter: Vec<u64> = (&s)
            .into_iter()
            .map(|sa| sa.action().id().raw())
            .collect();
        assert_eq!(from_iter, from_into_iter);
    }
}