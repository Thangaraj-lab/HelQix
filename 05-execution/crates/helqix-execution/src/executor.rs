//! Executor for HELQIX Layer 05 — Execution.
//!
//! An executor coordinates the execution of one eligible [`Action`]'s
//! through HELQIX's existing contracts and produces the resulting
//! [`Transition<C>`].
//!
//! # What the executor does
//!
//! Given an action, a [`Clock`], and a caller-supplied
//! [`ExecutionBackend`], the executor runs a fixed three-stage
//! pipeline:
//!
//! 1. **Validate** — ask the backend whether the action can execute
//!    against current engineering reality. (`ExecutionBackend::validate`)
//! 2. **Execute** — ask the backend to compute the physical /
//!    engineering consequences and return the canonical Layer 04
//!    change descriptor. Must not mutate authoritative state.
//!    (`ExecutionBackend::execute`)
//! 3. **Apply** — ask the backend to apply the change atomically.
//!    (`ExecutionBackend::apply`)
//!
//! On success, the executor returns
//! [`Transition::new(action.id(), clock.now(), change)`]. The logical
//! time is captured **once**, before stage 1.
//!
//! On failure, no transition is produced, the authoritative state is
//! (as required by the backend contract) unchanged, and the error
//! carries an [`ExecutionStage`] marker identifying where the pipeline
//! stopped.
//!
//! # What the executor does NOT do
//!
//! The executor never:
//!
//! * mutates engineering state directly (Layer 04 does, inside `apply`),
//! * performs physics or simulation (Layer 06 does, inside `execute`),
//! * advances or reads logical time from anywhere except the injected
//!   [`Clock`] (Layer 07 owns time),
//! * schedules (Layer 05's [`crate::scheduler`] does),
//! * consults the execution graph (Layer 05's
//!   [`crate::execution_graph`] does; the caller decides when to use it),
//! * owns action payloads (Layer 05's [`crate::action`] does),
//! * owns transitions (Layer 05's [`crate::transition`] does; Layer 04
//!   defines the change descriptor `C`).
//!
//! # Atomicity
//!
//! The executor guarantees that **either** the pipeline fully completes
//! (a [`Transition`] is produced and the backend reports the change
//! applied), **or** no transition is produced. It cannot, by itself,
//! undo a partial mutation performed inside `apply` — that is the
//! backend's responsibility, and it is stated as an explicit
//! requirement on [`ExecutionBackend::apply`].
//!
//! # Determinism
//!
//! Same `(Action, Clock::now(), backend-state)` ⇒ same outcome. No
//! wall clock, no randomness, no global state, no interior mutability,
//! no `unsafe`, no I/O, no threads.
//!
//! # Why the backend is a trait, not closures
//!
//! The three stages coordinate on shared underlying state (typically
//! Layer 04's state store and Layer 06's simulation engine). Grouping
//! them under one `&mut self` avoids forcing the caller to introduce
//! interior mutability or fragment ownership across three closures.

use core::fmt;

use crate::action::{Action, ActionPayload};
use crate::transition::Transition;
use helqix_time::Clock;

// ============================================================================
// ExecutionStage
// ============================================================================

/// The stage of the execution pipeline at which a failure occurred.
///
/// Distinguishes the three structural points at which
/// [`execute`] can abort:
///
/// * [`ExecutionStage::Validate`] — the backend rejected the action
///   because current reality does not permit it. No computation was
///   attempted, no state was touched.
/// * [`ExecutionStage::Execute`] — the backend failed while computing
///   the physical / engineering consequences. No state was touched.
/// * [`ExecutionStage::Apply`] — the backend failed while applying the
///   change. Per the [`ExecutionBackend::apply`] contract, the state
///   must be unchanged.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ExecutionStage {
    /// The validate stage.
    Validate,
    /// The execute stage.
    Execute,
    /// The apply stage.
    Apply,
}

impl fmt::Display for ExecutionStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Validate => "validate",
            Self::Execute => "execute",
            Self::Apply => "apply",
        })
    }
}

// ============================================================================
// ExecutionError
// ============================================================================

/// Error produced by [`execute`].
///
/// Carries the pipeline [`ExecutionStage`] at which the backend
/// reported failure, plus the backend's own error value `E`. Layer 05
/// adds the *stage*; the backend owns the *kind*.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExecutionError<E> {
    stage: ExecutionStage,
    source: E,
}

impl<E> ExecutionError<E> {
    /// Construct an error at a specific stage.
    #[inline]
    #[must_use]
    pub const fn new(stage: ExecutionStage, source: E) -> Self {
        Self { stage, source }
    }

    /// The stage at which the failure was reported.
    #[inline]
    #[must_use]
    pub const fn stage(&self) -> ExecutionStage {
        self.stage
    }

    /// Borrow the backend's underlying error.
    #[inline]
    #[must_use]
    pub const fn source_ref(&self) -> &E {
        &self.source
    }

    /// Consume the error and return the backend's underlying error.
    #[inline]
    #[must_use]
    pub fn into_source(self) -> E {
        self.source
    }
}

impl<E: fmt::Display> fmt::Display for ExecutionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "execution failed at stage `{}`: {}", self.stage, self.source)
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ExecutionError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

// ============================================================================
// ExecutionBackend
// ============================================================================

/// The caller-supplied bridge from Layer 05's execution pipeline to
/// HELQIX's existing engineering / simulation / state contracts.
///
/// A HELQIX runtime implements this trait **once**, delegating each
/// method to the appropriate layer:
///
/// * `validate` — typically delegates to Layer 04's state validation
///   ("is the target component in a state that permits this action?").
/// * `execute`  — typically delegates to Layer 06's simulation kernel
///   ("given this action and the current state, what physical change
///   results?") and asks Layer 04 to *describe* the change without
///   applying it.
/// * `apply`    — delegates to Layer 04's state store ("apply this
///   change atomically").
///
/// The executor never assumes these delegations; it only requires the
/// three methods.
///
/// # Contract for `apply`
///
/// **`apply` must be atomic.** If it returns `Err`, the authoritative
/// state must be unchanged from the caller's perspective. The executor
/// relies on this to guarantee that a failed execution produces no
/// transition and no partial state mutation. A backend that cannot
/// satisfy this must perform its own transactional / snapshot-based
/// rollback before returning.
///
/// # Contract for `execute`
///
/// **`execute` must not mutate authoritative state.** It may use
/// backend-internal scratch space freely. The returned `Self::Change`
/// is the canonical Layer 04 descriptor that `apply` will then apply.
pub trait ExecutionBackend {
    /// The action payload type this backend executes.
    type Payload: ActionPayload;

    /// The canonical Layer 04 change descriptor produced by `execute`
    /// and consumed by `apply`.
    type Change;

    /// The backend's own error type. Layer 05 does not constrain its
    /// shape — callers are free to use their canonical Layer 04 / 06
    /// error types.
    type Error;

    /// Structurally validate that `action` can execute against the
    /// backend's current state.
    ///
    /// Must not mutate any state. Returning `Err` means the action is
    /// rejected before any computation is attempted.
    fn validate(&self, action: &Action<Self::Payload>) -> Result<(), Self::Error>;

    /// Compute the canonical Layer 04 change descriptor produced by
    /// executing `action`.
    ///
    /// Must **not** mutate authoritative state. May use backend-
    /// internal scratch space freely.
    fn execute(
        &mut self,
        action: &Action<Self::Payload>,
    ) -> Result<Self::Change, Self::Error>;

    /// Atomically apply `change` to authoritative state.
    ///
    /// Must **either** fully succeed **or** leave state unchanged.
    /// The executor relies on this for its atomicity guarantee.
    fn apply(&mut self, change: &Self::Change) -> Result<(), Self::Error>;
}

// ============================================================================
// execute
// ============================================================================

/// Execute one eligible action through the three-stage pipeline.
///
/// See the module documentation for the full semantic model.
///
/// # Pipeline
///
/// 1. Read `applied_at = clock.now()` — once, before any stage.
/// 2. `backend.validate(action)`.
/// 3. `backend.execute(action)` — compute the change (no state mutation).
/// 4. `backend.apply(&change)` — atomically apply the change.
/// 5. Return `Transition::new(action.id(), applied_at, change)`.
///
/// At any failing stage, the pipeline aborts and returns
/// `ExecutionError { stage, source }`. No transition is produced, and
/// the state is guaranteed unchanged by the backend contract.
///
/// # Determinism
///
/// Same `(action, clock.now(), backend-state)` and same backend
/// behaviour ⇒ same outcome. No wall clock, no randomness.
pub fn execute<B>(
    action: &Action<B::Payload>,
    clock: &Clock,
    backend: &mut B,
) -> Result<Transition<B::Change>, ExecutionError<B::Error>>
where
    B: ExecutionBackend,
{
    let applied_at = clock.now();

    backend
        .validate(action)
        .map_err(|source| ExecutionError::new(ExecutionStage::Validate, source))?;

    let change = backend
        .execute(action)
        .map_err(|source| ExecutionError::new(ExecutionStage::Execute, source))?;

    backend
        .apply(&change)
        .map_err(|source| ExecutionError::new(ExecutionStage::Apply, source))?;

    Ok(Transition::new(action.id(), applied_at, change))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionError, ActionId};
    use core::cell::RefCell;

    // ------------------------------------------------------------------
    // Test fixtures
    // ------------------------------------------------------------------

    /// The action payload used in tests. Layer 05 knows nothing about
    /// its meaning.
    #[derive(Debug, Clone, Eq, PartialEq)]
    struct TestPayload(&'static str);

    impl ActionPayload for TestPayload {
        fn validate(&self) -> Result<(), ActionError> {
            if self.0.is_empty() {
                return Err(ActionError::new("empty payload"));
            }
            Ok(())
        }
    }

    /// The change descriptor — a stand-in for whatever Layer 04
    /// produces. Layer 05 treats it opaquely.
    #[derive(Debug, Clone, Eq, PartialEq)]
    struct TestChange {
        from: i64,
        to: i64,
    }

    /// A test-only backend error.
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    struct TestError(&'static str);

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    impl std::error::Error for TestError {}

    /// A test backend that records the sequence of stages executed,
    /// exposes per-stage injectable errors, and mutates internal state
    /// only inside `apply`.
    struct TestBackend {
        calls: RefCell<Vec<&'static str>>,
        state: i64,
        validate_error: Option<TestError>,
        execute_error: Option<TestError>,
        apply_error: Option<TestError>,
        /// The delta applied to `state` when `apply` succeeds.
        apply_delta: i64,
    }

    impl TestBackend {
        fn new(state: i64, apply_delta: i64) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                state,
                validate_error: None,
                execute_error: None,
                apply_error: None,
                apply_delta,
            }
        }

        fn with_validate_error(mut self, err: &'static str) -> Self {
            self.validate_error = Some(TestError(err));
            self
        }

        fn with_execute_error(mut self, err: &'static str) -> Self {
            self.execute_error = Some(TestError(err));
            self
        }

        fn with_apply_error(mut self, err: &'static str) -> Self {
            self.apply_error = Some(TestError(err));
            self
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls.borrow().clone()
        }
    }

    impl ExecutionBackend for TestBackend {
        type Payload = TestPayload;
        type Change = TestChange;
        type Error = TestError;

        fn validate(&self, _action: &Action<TestPayload>) -> Result<(), TestError> {
            // Note: `validate` takes `&self`, so we cannot record into
            // `calls` here. We rely instead on whether validate's own
            // error fires (and whether downstream stages execute).
            match self.validate_error {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }

        fn execute(
            &mut self,
            _action: &Action<TestPayload>,
        ) -> Result<TestChange, TestError> {
            self.calls.borrow_mut().push("execute");
            if let Some(e) = self.execute_error {
                return Err(e);
            }
            Ok(TestChange {
                from: self.state,
                to: self.state + self.apply_delta,
            })
        }

        fn apply(&mut self, change: &TestChange) -> Result<(), TestError> {
            self.calls.borrow_mut().push("apply");
            if let Some(e) = self.apply_error {
                return Err(e);
            }
            // Atomic by construction in the test: the only mutation
            // happens here, on the success path.
            self.state = change.to;
            Ok(())
        }
    }

    fn action(id: u64, payload: &'static str) -> Action<TestPayload> {
        Action::try_new(ActionId::from_raw(id), TestPayload(payload)).unwrap()
    }

    fn clock_at(nanos: i128) -> Clock {
        let mut c = Clock::new();
        if nanos != 0 {
            c.advance_by(helqix_time::SimDelta::from_nanos(nanos)).unwrap();
        }
        c
    }

    fn ns(n: i128) -> helqix_time::SimTime {
        helqix_time::SimTime::from_nanos(n)
    }

    // ------------------------------------------------------------------
    // 1. Successful execution
    // ------------------------------------------------------------------

    #[test]
    fn successful_execution_produces_correct_transition() {
        let a = action(7, "apply");
        let clock = clock_at(12_480_000_000);
        let mut backend = TestBackend::new(10, 5);

        let t = execute(&a, &clock, &mut backend).unwrap();

        assert_eq!(t.action(), a.id());
        assert_eq!(t.applied_at(), ns(12_480_000_000));
        assert_eq!(t.change(), &TestChange { from: 10, to: 15 });
    }

    #[test]
    fn successful_execution_runs_stages_in_order() {
        let a = action(1, "x");
        let clock = clock_at(0);
        let mut backend = TestBackend::new(0, 1);

        execute(&a, &clock, &mut backend).unwrap();

        // `validate` cannot record into `calls` (it takes `&self`), but
        // its successful completion is proven by execute+apply running.
        assert_eq!(backend.calls(), vec!["execute", "apply"]);
    }

    #[test]
    fn successful_execution_updates_backend_state() {
        let a = action(1, "x");
        let clock = clock_at(0);
        let mut backend = TestBackend::new(100, 7);

        execute(&a, &clock, &mut backend).unwrap();

        assert_eq!(backend.state, 107);
    }

    // ------------------------------------------------------------------
    // 2. Failure at each stage
    // ------------------------------------------------------------------

    #[test]
    fn validate_failure_aborts_before_execute_and_leaves_state_unchanged() {
        let a = action(1, "x");
        let clock = clock_at(0);
        let mut backend = TestBackend::new(50, 10).with_validate_error("nope");

        let err = execute(&a, &clock, &mut backend).unwrap_err();

        assert_eq!(err.stage(), ExecutionStage::Validate);
        assert_eq!(err.source_ref(), &TestError("nope"));
        // Execute and apply were never called.
        assert_eq!(backend.calls(), Vec::<&str>::new());
        // State untouched.
        assert_eq!(backend.state, 50);
    }

    #[test]
    fn execute_failure_aborts_before_apply_and_leaves_state_unchanged() {
        let a = action(1, "x");
        let clock = clock_at(0);
        let mut backend = TestBackend::new(50, 10).with_execute_error("boom");

        let err = execute(&a, &clock, &mut backend).unwrap_err();

        assert_eq!(err.stage(), ExecutionStage::Execute);
        assert_eq!(err.source_ref(), &TestError("boom"));
        // Execute ran (and recorded), apply did not.
        assert_eq!(backend.calls(), vec!["execute"]);
        // State untouched.
        assert_eq!(backend.state, 50);
    }

    #[test]
    fn apply_failure_aborts_before_transition_construction() {
        let a = action(1, "x");
        let clock = clock_at(0);
        let mut backend = TestBackend::new(50, 10).with_apply_error("rejected");

        let err = execute(&a, &clock, &mut backend).unwrap_err();

        assert_eq!(err.stage(), ExecutionStage::Apply);
        assert_eq!(err.source_ref(), &TestError("rejected"));
        // Both stages ran; the pipeline aborted after apply.
        assert_eq!(backend.calls(), vec!["execute", "apply"]);
        // State untouched by the failing apply (per the trait contract).
        assert_eq!(backend.state, 50);
    }

    // ------------------------------------------------------------------
    // 3. Error semantics
    // ------------------------------------------------------------------

    #[test]
    fn error_stage_markers_discriminate_all_three_stages() {
        let a = action(1, "x");
        let clock = clock_at(0);

        let v = execute(
            &a,
            &clock,
            &mut TestBackend::new(0, 1).with_validate_error("v"),
        )
        .unwrap_err();
        let e = execute(
            &a,
            &clock,
            &mut TestBackend::new(0, 1).with_execute_error("e"),
        )
        .unwrap_err();
        let p = execute(
            &a,
            &clock,
            &mut TestBackend::new(0, 1).with_apply_error("p"),
        )
        .unwrap_err();

        assert_eq!(v.stage(), ExecutionStage::Validate);
        assert_eq!(e.stage(), ExecutionStage::Execute);
        assert_eq!(p.stage(), ExecutionStage::Apply);
    }

    #[test]
    fn error_into_source_unwraps_backend_error() {
        let a = action(1, "x");
        let clock = clock_at(0);
        let err = execute(
            &a,
            &clock,
            &mut TestBackend::new(0, 1).with_execute_error("specific"),
        )
        .unwrap_err();
        assert_eq!(err.into_source(), TestError("specific"));
    }

    #[test]
    fn error_display_includes_stage_and_source() {
        let err: ExecutionError<TestError> =
            ExecutionError::new(ExecutionStage::Execute, TestError("explosion"));
        let s = err.to_string();
        assert!(s.contains("execute"), "got: {}", s);
        assert!(s.contains("explosion"), "got: {}", s);
    }

    #[test]
    fn error_implements_std_error_with_source() {
        let err: ExecutionError<TestError> =
            ExecutionError::new(ExecutionStage::Apply, TestError("x"));
        let dyn_err: &dyn std::error::Error = &err;
        assert!(dyn_err.source().is_some());
    }

    // ------------------------------------------------------------------
    // 4. Time semantics
    // ------------------------------------------------------------------

    #[test]
    fn time_is_captured_at_execution_start() {
        let a = action(1, "x");
        let clock = clock_at(1_000);
        let mut backend = TestBackend::new(0, 1);

        let t = execute(&a, &clock, &mut backend).unwrap();
        assert_eq!(t.applied_at(), ns(1_000));
    }

    #[test]
    fn executor_does_not_advance_the_clock() {
        let a = action(1, "x");
        let clock = clock_at(500);
        let before = clock.now();
        let mut backend = TestBackend::new(0, 1);

        execute(&a, &clock, &mut backend).unwrap();

        assert_eq!(clock.now(), before);
    }

    #[test]
    fn zero_time_is_supported() {
        let a = action(1, "x");
        let clock = clock_at(0);
        let mut backend = TestBackend::new(0, 1);
        let t = execute(&a, &clock, &mut backend).unwrap();
        assert_eq!(t.applied_at(), ns(0));
    }

    // ------------------------------------------------------------------
    // 5. Determinism
    // ------------------------------------------------------------------

    #[test]
    fn deterministic_repeated_execution_yields_identical_transition() {
        let a = action(1, "x");
        let clock = clock_at(100);
        let mut b1 = TestBackend::new(0, 5);
        let mut b2 = TestBackend::new(0, 5);
        let mut b3 = TestBackend::new(0, 5);

        let t1 = execute(&a, &clock, &mut b1).unwrap();
        let t2 = execute(&a, &clock, &mut b2).unwrap();
        let t3 = execute(&a, &clock, &mut b3).unwrap();

        assert_eq!(t1, t2);
        assert_eq!(t2, t3);
    }

    #[test]
    fn different_actions_produce_different_transitions() {
        let clock = clock_at(0);
        let mut b1 = TestBackend::new(0, 1);
        let mut b2 = TestBackend::new(0, 1);
        let t1 = execute(&action(1, "x"), &clock, &mut b1).unwrap();
        let t2 = execute(&action(2, "x"), &clock, &mut b2).unwrap();
        assert_ne!(t1, t2);
    }

    #[test]
    fn different_times_produce_different_transitions() {
        let a = action(1, "x");
        let mut b1 = TestBackend::new(0, 1);
        let mut b2 = TestBackend::new(0, 1);
        let t1 = execute(&a, &clock_at(100), &mut b1).unwrap();
        let t2 = execute(&a, &clock_at(200), &mut b2).unwrap();
        assert_ne!(t1, t2);
    }

    #[test]
    fn different_changes_produce_different_transitions() {
        let a = action(1, "x");
        let clock = clock_at(0);
        let mut b1 = TestBackend::new(0, 1);
        let mut b2 = TestBackend::new(0, 2);
        let t1 = execute(&a, &clock, &mut b1).unwrap();
        let t2 = execute(&a, &clock, &mut b2).unwrap();
        assert_ne!(t1, t2);
    }

    // ------------------------------------------------------------------
    // 6. Domain independence
    // ------------------------------------------------------------------

    #[test]
    fn different_payload_and_change_types_coexist() {
        #[derive(Debug, Eq, PartialEq)]
        struct OtherPayload;
        impl ActionPayload for OtherPayload {
            fn validate(&self) -> Result<(), ActionError> {
                Ok(())
            }
        }

        #[derive(Debug, Eq, PartialEq)]
        struct OtherChange(u32);

        struct OtherBackend;
        impl ExecutionBackend for OtherBackend {
            type Payload = OtherPayload;
            type Change = OtherChange;
            type Error = TestError;

            fn validate(&self, _: &Action<OtherPayload>) -> Result<(), TestError> {
                Ok(())
            }
            fn execute(
                &mut self,
                _: &Action<OtherPayload>,
            ) -> Result<OtherChange, TestError> {
                Ok(OtherChange(99))
            }
            fn apply(&mut self, _: &OtherChange) -> Result<(), TestError> {
                Ok(())
            }
        }

        let clock = clock_at(0);
        let mut b = OtherBackend;
        let a = Action::try_new(ActionId::from_raw(1), OtherPayload).unwrap();
        let t: Transition<OtherChange> = execute(&a, &clock, &mut b).unwrap();
        assert_eq!(t.change(), &OtherChange(99));

        // And the original TestBackend still works with its own types.
        let mut tb = TestBackend::new(0, 1);
        let ta = action(1, "x");
        let _t2 = execute(&ta, &clock, &mut tb).unwrap();
    }

    // ------------------------------------------------------------------
    // 7. Ownership / borrowing
    // ------------------------------------------------------------------

    #[test]
    fn action_is_borrowed_not_consumed() {
        let a = action(1, "x");
        let clock = clock_at(0);
        let mut backend = TestBackend::new(0, 1);

        execute(&a, &clock, &mut backend).unwrap();
        // `a` is still usable after execution.
        assert_eq!(a.id().raw(), 1);
        assert_eq!(a.payload().0, "x");
    }

    #[test]
    fn into_change_extracts_backend_produced_change() {
        let a = action(1, "x");
        let clock = clock_at(0);
        let mut backend = TestBackend::new(3, 4);
        let t = execute(&a, &clock, &mut backend).unwrap();
        assert_eq!(t.into_change(), TestChange { from: 3, to: 7 });
    }

    // ------------------------------------------------------------------
    // 8. Boundary
    // ------------------------------------------------------------------

    #[test]
    fn extreme_sim_time_boundaries_are_supported() {
        let a = action(1, "x");
        let mut backend = TestBackend::new(0, 1);

        // Build a clock at SimTime::MIN without advancing via the
        // public delta API (which forbids going below MIN anyway).
        let mut clock_min = Clock::new();
        clock_min
            .advance_by(helqix_time::SimDelta::from_nanos(i128::MIN))
            .unwrap();
        assert_eq!(clock_min.now(), helqix_time::SimTime::MIN);

        let t = execute(&a, &clock_min, &mut backend).unwrap();
        assert_eq!(t.applied_at(), helqix_time::SimTime::MIN);
    }

    #[test]
    fn extreme_action_id_boundaries_are_supported() {
        let clock = clock_at(0);
        let mut b1 = TestBackend::new(0, 1);
        let mut b2 = TestBackend::new(0, 1);
        let t_min = execute(
            &Action::try_new(ActionId::from_raw(0), TestPayload("x")).unwrap(),
            &clock,
            &mut b1,
        )
        .unwrap();
        let t_max = execute(
            &Action::try_new(ActionId::from_raw(u64::MAX), TestPayload("x")).unwrap(),
            &clock,
            &mut b2,
        )
        .unwrap();
        assert_eq!(t_min.action().raw(), 0);
        assert_eq!(t_max.action().raw(), u64::MAX);
    }

    // ------------------------------------------------------------------
    // 9. Worked failure workflow
    // ------------------------------------------------------------------

    #[test]
    fn worked_failure_workflow_produces_attributable_transition() {
        // Action at t = 12.48 s produces a change on component 3
        // (current 10 → 15). Downstream layers (09-failure, 10-events-
        // causality) get the exact action id, exact time, and exact
        // change descriptor.
        let a = action(101, "increase_current");
        let clock = clock_at(12_480_000_000);
        let mut backend = TestBackend::new(10, 5);

        let t = execute(&a, &clock, &mut backend).unwrap();

        assert_eq!(t.action().raw(), 101);
        assert_eq!(t.applied_at(), ns(12_480_000_000));
        assert_eq!(t.change(), &TestChange { from: 10, to: 15 });
    }
}