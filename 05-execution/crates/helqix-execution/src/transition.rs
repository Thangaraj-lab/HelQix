//! Execution-side transition descriptor for HELQIX Layer 05.
//!
//! A `Transition<C>` binds an executed action and logical time to the
//! canonical Layer 04 change descriptor `C`. The change remains opaque to
//! Layer 05; this type only adds execution context.

use core::fmt;

use crate::action::ActionId;
use helqix_time::SimTime;

/// Execution context wrapped around a canonical Layer 04 change.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Transition<C> {
    action: ActionId,
    applied_at: SimTime,
    change: C,
}

impl<C> Transition<C> {
    /// Construct a deterministic transition descriptor.
    #[inline]
    #[must_use]
    pub const fn new(action: ActionId, applied_at: SimTime, change: C) -> Self {
        Self { action, applied_at, change }
    }

    #[inline]
    #[must_use]
    pub const fn action(&self) -> ActionId { self.action }

    #[inline]
    #[must_use]
    pub const fn applied_at(&self) -> SimTime { self.applied_at }

    #[inline]
    #[must_use]
    pub const fn change(&self) -> &C { &self.change }

    #[inline]
    #[must_use]
    pub fn into_change(self) -> C { self.change }

    #[inline]
    #[must_use]
    pub fn into_parts(self) -> (ActionId, SimTime, C) {
        (self.action, self.applied_at, self.change)
    }

    #[inline]
    #[must_use]
    pub fn from_action(&self, action: ActionId) -> bool { self.action == action }

    #[inline]
    #[must_use]
    pub fn at_time(&self, time: SimTime) -> bool { self.applied_at == time }

    #[inline]
    #[must_use]
    pub fn before(&self, time: SimTime) -> bool { self.applied_at < time }

    #[inline]
    #[must_use]
    pub fn after(&self, time: SimTime) -> bool { self.applied_at > time }
}

impl<C: fmt::Display> fmt::Display for Transition<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Transition(action={:?}, applied_at={}, change={})", self.action, self.applied_at, self.change)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Action, ActionError, ActionPayload};

    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    struct FakeChange { component: u32, before: i64, after: i64 }

    fn change(component: u32, before: i64, after: i64) -> FakeChange {
        FakeChange { component, before, after }
    }

    #[derive(Debug, Clone, Eq, PartialEq)]
    struct Payload;

    impl ActionPayload for Payload {
        fn validate(&self) -> Result<(), ActionError> { Ok(()) }
    }

    fn action(id: u64) -> Action<Payload> {
        Action::try_new(ActionId::from_raw(id), Payload).unwrap()
    }

    fn ns(value: i128) -> SimTime { SimTime::from_nanos(value) }

    #[test]
    fn construction_preserves_all_components() {
        let a = action(7);
        let t = Transition::new(a.id(), ns(100), change(3, 10, 15));
        assert_eq!(t.action(), a.id());
        assert_eq!(t.applied_at(), ns(100));
        assert_eq!(t.change(), &change(3, 10, 15));
    }

    #[test]
    fn extraction_preserves_change() {
        let t = Transition::new(ActionId::from_raw(1), ns(100), change(3, 10, 15));
        assert_eq!(t.into_change(), change(3, 10, 15));
    }

    #[test]
    fn into_parts_returns_all_values() {
        let t = Transition::new(ActionId::from_raw(42), ns(123), change(9, 1, 2));
        let (action, time, extracted) = t.into_parts();
        assert_eq!(action.raw(), 42);
        assert_eq!(time, ns(123));
        assert_eq!(extracted, change(9, 1, 2));
    }

    #[test]
    fn temporal_and_action_predicates_are_exact() {
        let t = Transition::new(ActionId::from_raw(1), ns(100), change(0, 0, 1));
        assert!(t.at_time(ns(100)));
        assert!(!t.at_time(ns(101)));
        assert!(t.before(ns(101)));
        assert!(!t.before(ns(100)));
        assert!(t.after(ns(99)));
        assert!(!t.after(ns(100)));
        assert!(t.from_action(ActionId::from_raw(1)));
        assert!(!t.from_action(ActionId::from_raw(2)));
    }

    #[test]
    fn equality_and_hashing_are_deterministic() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let t1 = Transition::new(ActionId::from_raw(1), ns(100), change(1, 0, 1));
        let t2 = Transition::new(ActionId::from_raw(1), ns(100), change(1, 0, 1));
        assert_eq!(t1, t2);
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        t1.hash(&mut h1);
        t2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn cloned_transition_is_identical() {
        let t = Transition::new(ActionId::from_raw(1), ns(50), change(2, 3, 4));
        assert_eq!(t, t.clone());
    }

    #[test]
    fn non_clone_change_can_be_owned_and_extracted() {
        #[derive(Debug, Eq, PartialEq)]
        struct NonClone(String);
        let t = Transition::new(ActionId::from_raw(1), ns(0), NonClone("payload".into()));
        assert_eq!(t.change(), &NonClone("payload".into()));
        assert_eq!(t.into_change(), NonClone("payload".into()));
    }

    #[test]
    fn display_contains_execution_context_and_change() {
        #[derive(Debug)]
        struct DisplayChange(i32);
        impl fmt::Display for DisplayChange {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "delta={}", self.0) }
        }
        let t = Transition::new(ActionId::from_raw(7), ns(12_480_000_000), DisplayChange(-5));
        let rendered = t.to_string();
        assert!(rendered.contains("12.480000000s"));
        assert!(rendered.contains("delta=-5"));
    }

    #[test]
    fn extreme_values_are_supported() {
        let min = Transition::new(ActionId::from_raw(0), SimTime::MIN, change(0, 0, 0));
        let max = Transition::new(ActionId::from_raw(u64::MAX), SimTime::MAX, change(0, 0, 0));
        assert_eq!(min.action().raw(), 0);
        assert_eq!(max.action().raw(), u64::MAX);
        assert!(min.before(SimTime::MAX));
        assert!(max.after(SimTime::MIN));
    }

    #[test]
    fn canonical_failure_workflow_keeps_change_borrowable() {
        let action_id = ActionId::from_raw(101);
        let applied_at = ns(12_480_000_000);
        let expected_change = change(3, 10, 15);
        let t = Transition::new(action_id, applied_at, expected_change.clone());
        assert_eq!(t.action(), action_id);
        assert_eq!(t.applied_at(), applied_at);
        assert_eq!(t.change(), &expected_change);
        assert!(t.from_action(action_id));
        assert!(t.at_time(applied_at));
    }
}
