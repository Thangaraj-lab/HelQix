//! Execution-side transition descriptor for HELQIX Layer 05 — Execution.
//!
//! A [`Transition<C>`] records that **a specific action** produced
//! **a specific state change** at **a specific logical time**. It is
//! the execution-side envelope around a canonical Layer 04 change
//! descriptor `C`.
//!
//! # Responsibility boundary
//!
//! `transition.rs` owns **only** the execution-side context of a state
//! change:
//!
//! * which action produced it ([`ActionId`]),
//! * at what logical time it was applied (`helqix_time::SimTime`).
//!
//! It does **not** own the state change itself. That is `C`, supplied
//! by the caller. `C` is the canonical Layer 04 representation — a
//! state diff, a mutation, a target-snapshot reference, or whatever
//! `04-state/src/transition.rs` defines. Layer 05 does not inspect,
//! interpret, or duplicate it.
//!
//! # Why not duplicate Layer 04's transition
//!
//! The HELQIX architecture has exactly one authoritative state
//! transition model, and it lives in Layer 04. Layer 05 must not create
//! a second one.
//!
//! But Layer 04 has no visibility into two facts Layer 05 knows and
//! Layer 04 does not:
//!
//! 1. **Which action** was executed (Layer 04 has no concept of an
//!    `ActionId`).
//! 2. **When in logical time** it was executed (Layer 04 has no
//!    dependency on `helqix-time`).
//!
//! `Transition<C>` exists precisely to bind those two execution-side
//! facts to the canonical Layer 04 change. It is a *descriptor*, not a
//! state model.
//!
//! # Simulation, physics, time
//!
//! This file contains none of them. Physical consequences are computed
//! by Layer 06 before the transition descriptor is built. Logical time
//! is supplied by Layer 07 (via [`helqix_time::SimTime`]). Layer 05's
//! transition is a record of what happened, not a calculation of what
//! would happen.
//!
//! # Immutability
//!
//! [`Transition<C>`] has **no `&mut self` methods**. Once constructed,
//! its action, time, and change cannot be altered by any safe
//! operation. This is required for replay, branching, causal analysis,
//! and verification: the same action at the same logical time must
//! yield the same transition descriptor.
//!
//! # Determinism
//!
//! No randomness, no wall clock, no global state, no interior
//! mutability, no `unsafe`, no I/O. Two `Transition<C>` values with
//! equal `action`, `applied_at`, and `change` are equal.
//!
//! # Composition with the executor
//!
//! The executor (`executor.rs`, future) is expected to:
//!
//! 1. take a selected action from the scheduler,
//! 2. hand the action's effect to Layer 06 for physical consequences
//!    (if any),
//! 3. ask Layer 04 to compute the canonical change descriptor `C`,
//! 4. construct `Transition::new(action.id(), clock.now(), change)`,
//! 5. return it to the runtime.
//!
//! Steps 2 and 3 are **outside** this file. `transition.rs` performs
//! none of them.

use core::fmt;

use crate::action::ActionId;
use helqix_time::SimTime;

// ============================================================================
// Transition
// ============================================================================

/// An execution-side descriptor binding an action, a logical time, and
/// a canonical Layer 04 state change.
///
/// See the module documentation for the full semantic model.
///
/// # Type parameter
///
/// `C` is the canonical Layer 04 change descriptor. Layer 05 imposes
/// **no bound** on `C`. It is treated as opaque.
///
/// # Invariants
///
/// 1. Once constructed, `action`, `applied_at`, and `change` cannot be
///    mutated by any safe operation — there are no `&mut self`
///    methods.
/// 2. Two `Transition<C>` values are equal iff their `action`,
///    `applied_at`, and `change` are equal (derived `PartialEq` /
///    `Eq`).
/// 3. There is no interior mutability, no wall-clock read, no
///    randomness, no global state, and no `unsafe` in this type or its
///    methods.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Transition<C> {
    action: ActionId,
    applied_at: SimTime,
    change: C,
}

impl<C> Transition<C> {
    /// Construct a transition from its three components.
    ///
    /// This cannot fail: `ActionId` is already validated (it was
    /// produced by [`crate::action::Action::try_new`]), `SimTime` is
    /// always a valid logical time, and `C` is opaque to Layer 05.
    #[inline]
    #[must_use]
    pub const fn new(action: ActionId, applied_at: SimTime, change: C) -> Self {
        Self {
            action,
            applied_at,
            change,
        }
    }

    /// The action that produced this transition.
    #[inline]
    #[must_use]
    pub const fn action(&self) -> ActionId {
        self.action
    }

    /// The exact logical time at which the action was applied.
    #[inline]
    #[must_use]
    pub const fn applied_at(&self) -> SimTime {
        self.applied_at
    }

    /// Borrow the canonical Layer 04 change descriptor.
    #[inline]
    #[must_use]
    pub const fn change(&self) -> &C {
        &self.change
    }

    /// Consume the transition and return the change descriptor.
    ///
    /// Useful when the caller — typically Layer 04's apply method —
    /// needs to take ownership of the change without cloning it.
    #[inline]
    #[must_use]
    pub fn into_change(self) -> C {
        self.change
    }

    /// Consume the transition and return all three parts.
    #[inline]
    #[must_use]
    pub fn into_parts(self) -> (ActionId, SimTime, C) {
        (self.action, self.applied_at, self.change)
    }

    /// Does this transition belong to `action`?
    #[inline]
    #[must_use]
    pub fn from_action(&self, action: ActionId) -> bool {
        self.action == action
    }

    /// Was this transition applied at `time`?
    #[inline]
    #[must_use]
    pub fn at_time(&self, time: SimTime) -> bool {
        self.applied_at == time
    }

    /// Was this transition applied strictly before `time`?
    ///
    /// The natural query for "did this transition occur before the
    /// failure at `time`?".
    #[inline]
    #[must_use]
    pub fn before(&self, time: SimTime) -> bool {
        self.applied_at < time
    }

    /// Was this transition applied strictly after `time`?
    #[inline]
    #[must_use]
    pub fn after(&self, time: SimTime) -> bool {
        self.applied_at > time
    }
}

impl<C: fmt::Display> fmt::Display for Transition<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Transition(action={:?}, applied_at={}, change={})",
            self.action, self.applied_at, self.change
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Action, ActionError, ActionId, ActionPayload};

    /// A stand-in for whatever Layer 04's canonical change descriptor
    /// actually is. Layer 05 does not care — the tests below exercise
    /// the envelope semantics, not the change.
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    struct FakeChange {
        component: u32,
        before: i64,
        after: i64,
    }

    /// A representative action payload, so we can construct real
    /// `ActionId`s via the canonical `Action::try_new` path.
    #[derive(Debug, Clone, Eq, PartialEq)]
    struct P;

    impl ActionPayload for P {
        fn validate(&self) -> Result<(), ActionError> {
            Ok(())
        }
    }

    fn action(id: u64) -> Action<P> {
        Action::try_new(ActionId::from_raw(id), P).unwrap()
    }

    fn ns(n: i128) -> SimTime {
        SimTime::from_nanos(n)
    }

    fn change(component: u32, before: i64, after: i64) -> FakeChange {
        FakeChange {
            component,
            before,
            after,
        }
    }

    // ------------------------------------------------------------------
    // 1. Construction
    // ------------------------------------------------------------------

    #[test]
    fn new_preserves_action_time_and_change() {
        let a = action(7);
        let t = Transition::new(a.id(), ns(12_480_000_000), change(3, 10, 15));

        assert_eq!(t.action(), a.id());
        assert_eq!(t.applied_at(), ns(12_480_000_000));
        assert_eq!(t.change(), &change(3, 10, 15));
    }

    #[test]
    fn construction_is_const_and_total() {
        // `Transition::new` is `const` and cannot fail. This test
        // proves the constructor type: no `Result`, no panic path.
        let t: Transition<FakeChange> = Transition::new(
            ActionId::from_raw(1),
            SimTime::ZERO,
            change(0, 0, 0),
        );
        assert_eq!(t.action().raw(), 1);
        assert_eq!(t.applied_at(), SimTime::ZERO);
    }

    // ------------------------------------------------------------------
    // 2. Extraction
    // ------------------------------------------------------------------

    #[test]
    fn into_change_yields_the_change_descriptor() {
        let t = Transition::new(
            ActionId::from_raw(1),
            ns(100),
            change(3, 10, 15),
        );
        assert_eq!(t.into_change(), change(3, 10, 15));
    }

    #[test]
    fn into_parts_yields_all_three_components() {
        let t = Transition::new(
            ActionId::from_raw(42),
            ns(12_345),
            change(9, 1, 2),
        );
        let (a, time, c) = t.into_parts();
        assert_eq!(a.raw(), 42);
        assert_eq!(time, ns(12_345));
        assert_eq!(c, change(9, 1, 2));
    }

    // ------------------------------------------------------------------
    // 3. Temporal queries
    // ------------------------------------------------------------------

    #[test]
    fn temporal_predicates_are_exact() {
        let t = Transition::new(
            ActionId::from_raw(1),
            ns(12_480_000_000),
            change(1, 0, 1),
        );

        assert!(t.at_time(ns(12_480_000_000)));
        assert!(!t.at_time(ns(12_480_000_001)));
        assert!(!t.at_time(ns(12_479_999_999)));

        assert!(t.before(ns(12_480_000_001)));
        assert!(!t.before(ns(12_480_000_000)));
        assert!(!t.before(ns(12_479_999_999)));

        assert!(t.after(ns(12_479_999_999)));
        assert!(!t.after(ns(12_480_000_000)));
        assert!(!t.after(ns(12_480_000_001)));
    }

    #[test]
    fn action_predicate_is_exact() {
        let a1 = action(1);
        let a2 = action(2);
        let t = Transition::new(a1.id(), ns(0), change(0, 0, 0));

        assert!(t.from_action(a1.id()));
        assert!(!t.from_action(a2.id()));
    }

    // ------------------------------------------------------------------
    // 4. Equality and hashing
    // ------------------------------------------------------------------

    #[test]
    fn equal_components_yield_equal_transitions() {
        let a = action(5);
        let t1 = Transition::new(a.id(), ns(100), change(1, 0, 1));
        let t2 = Transition::new(a.id(), ns(100), change(1, 0, 1));
        assert_eq!(t1, t2);
    }

    #[test]
    fn differing_action_yields_distinct_transitions() {
        let t1 = Transition::new(ActionId::from_raw(1), ns(100), change(1, 0, 1));
        let t2 = Transition::new(ActionId::from_raw(2), ns(100), change(1, 0, 1));
        assert_ne!(t1, t2);
    }

    #[test]
    fn differing_time_yields_distinct_transitions() {
        let t1 = Transition::new(ActionId::from_raw(1), ns(100), change(1, 0, 1));
        let t2 = Transition::new(ActionId::from_raw(1), ns(101), change(1, 0, 1));
        assert_ne!(t1, t2);
    }

    #[test]
    fn differing_change_yields_distinct_transitions() {
        let t1 = Transition::new(ActionId::from_raw(1), ns(100), change(1, 0, 1));
        let t2 = Transition::new(ActionId::from_raw(1), ns(100), change(1, 0, 2));
        assert_ne!(t1, t2);
    }

    #[test]
    fn equal_transitions_hash_equal() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        fn h<C: Hash>(t: &Transition<C>) -> u64 {
            let mut hasher = DefaultHasher::new();
            t.hash(&mut hasher);
            hasher.finish()
        }

        let t1 = Transition::new(ActionId::from_raw(1), ns(100), change(1, 0, 1));
        let t2 = Transition::new(ActionId::from_raw(1), ns(100), change(1, 0, 1));
        assert_eq!(t1, t2);
        assert_eq!(h(&t1), h(&t2));
    }

    // ------------------------------------------------------------------
    // 5. Determinism
    // ------------------------------------------------------------------

    #[test]
    fn identical_construction_is_deterministic() {
        let build = || {
            let a = action(7);
            Transition::new(a.id(), ns(12_480_000_000), change(3, 10, 15))
        };

        let t1 = build();
        let t2 = build();
        let t3 = build();

        assert_eq!(t1, t2);
        assert_eq!(t2, t3);
    }

    // ------------------------------------------------------------------
    // 6. Domain independence — two change types coexist
    // ------------------------------------------------------------------

    #[test]
    fn transitions_of_different_change_types_coexist() {
        #[derive(Debug, Clone, Eq, PartialEq, Hash)]
        struct OtherChange(u32);

        let _a: Transition<FakeChange> =
            Transition::new(ActionId::from_raw(1), ns(1), change(0, 0, 0));
        let _b: Transition<OtherChange> =
            Transition::new(ActionId::from_raw(1), ns(1), OtherChange(7));
        // Compilation is the test: two different Layer 04 change
        // descriptor types produce two distinct `Transition<C>` types
        // without Layer 05 knowing anything about either.
    }

    // ------------------------------------------------------------------
    // 7. Non-Clone change descriptors are supported
    // ------------------------------------------------------------------

    #[test]
    fn transition_can_hold_non_clone_change() {
        #[derive(Debug, Eq, PartialEq)]
        struct NonClone(String);

        let t = Transition::new(
            ActionId::from_raw(1),
            ns(0),
            NonClone("payload".into()),
        );
        assert_eq!(t.change(), &NonClone("payload".into()));
        // And it can be extracted by value.
        let extracted = t.into_change();
        assert_eq!(extracted, NonClone("payload".into()));
    }

    // ------------------------------------------------------------------
    // 8. Boundary: SimTime extremes
    // ------------------------------------------------------------------

    #[test]
    fn extreme_sim_time_boundaries_are_supported() {
        let t_min = Transition::new(ActionId::from_raw(1), SimTime::MIN, change(0, 0, 0));
        let t_max = Transition::new(ActionId::from_raw(1), SimTime::MAX, change(0, 0, 0));

        assert_eq!(t_min.applied_at(), SimTime::MIN);
        assert_eq!(t_max.applied_at(), SimTime::MAX);
        assert!(t_min.before(SimTime::MAX));
        assert!(t_max.after(SimTime::MIN));
    }

    // ------------------------------------------------------------------
    // 9. Display
    // ------------------------------------------------------------------

    #[test]
    fn display_includes_action_time_and_change() {
        // FakeChange implements Debug but not Display — so we use a
        // change type that does implement Display.
        #[derive(Debug)]
        struct DisplayChange(i32);
        impl fmt::Display for DisplayChange {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "Δ{}", self.0)
            }
        }

        let t = Transition::new(
            ActionId::from_raw(7),
            ns(12_480_000_000),
            DisplayChange(-5),
        );
        let s = t.to_string();
        assert!(s.contains("12.480000000s"), "got: {}", s);
        assert!(s.contains("Δ-5"), "got: {}", s);
    }

    // ------------------------------------------------------------------
    // 10. Failure workflow composition
    // ------------------------------------------------------------------

    #[test]
    fn worked_failure_workflow_binds_action_and_time_to_change() {
        // Mirrors the Layer 05 / Layer 07 worked example: an action
        // executed at a specific logical time produces a specific
        // canonical Layer 04 change descriptor.
        let action_id = ActionId::from_raw(101);
        let applied_at = ns(12_480_000_000);
        let change = change(3, 10, 15); // component 3: current 10A → 15A

        let t = Transition::new(action_id, applied_at, change);

        // The envelope carries the execution-side context.
        assert_eq!(t.action(), action_id);
        assert_eq!(t.applied_at(), applied_at);
        // The change is the canonical Layer 04 descriptor, untouched.
        assert_eq!(t.change(), &change);
        // The envelope answers the questions a causal analyser needs.
        assert!(t.from_action(action_id));
        assert!(t.at_time(ns(12_480_000_000)));
    }

    // ------------------------------------------------------------------
    // 11. Cloning preserves all three components
    // ------------------------------------------------------------------

    #[test]
    fn cloning_preserves_everything() {
        let t = Transition::new(
            ActionId::from_raw(1),
            ns(50),
            change(2, 3, 4),
        );
        let cloned = t.clone();
        assert_eq!(t, cloned);
        assert_eq!(t.action(), cloned.action());
        assert_eq!(t.applied_at(), cloned.applied_at());
        assert_eq!(t.change(), cloned.change());
    }

    // ------------------------------------------------------------------
    // 12. Extreme ActionId values
    // ------------------------------------------------------------------

    #[test]
    fn extreme_action_ids_are_supported() {
        let t_min = Transition::new(
            ActionId::from_raw(0),
            ns(0),
            change(0, 0, 0),
        );
        let t_max = Transition::new(
            ActionId::from_raw(u64::MAX),
            ns(0),
            change(0, 0, 0),
        );
        assert_eq!(t_min.action().raw(), 0);
        assert_eq!(t_max.action().raw(), u64::MAX);
    }
}
