//! Execution runtime for HELQIX Layer 05.
//!
//! A [`Runtime`] is the top-level coordinator that ties together the
//! four prior Layer 05 components:
//!
//! * [`Scheduler`] — pending actions ordered by `(scheduled_at, ActionId)`,
//! * [`ExecutionGraph`] — dependency constraints between actions,
//! * [`executor::execute`] — the three-stage execution pipeline,
//! * [`Transition`] — the execution-side descriptor produced by each step.
//!
//! # Responsibility boundary
//!
//! `runtime.rs` owns **only**:
//!
//! * the execution loop (one iteration per eligible action),
//! * the set of completed [`ActionId`]s (used to answer
//!   graph-readiness queries),
//! * the deterministic termination conditions
//!   ([`RuntimeOutcome::Completed`] / [`Blocked`](RuntimeOutcome::Blocked)
//!   / [`Failed`](RuntimeOutcome::Failed)).
//!
//! It explicitly does **not** own:
//!
//! * engineering state (Layer 04 — reached through the caller's
//!   [`ExecutionBackend`]),
//! * physical consequences (Layer 06 — reached through the caller's
//!   [`ExecutionBackend`]),
//! * logical time (Layer 07 — the [`Clock`] is a dependency, not a
//!   reimplementation),
//! * action ordering (delegated to [`Scheduler`]),
//! * dependency structure (delegated to [`ExecutionGraph`]),
//! * pipeline semantics (delegated to
//!   [`executor::execute`]),
//! * the transition descriptor's shape (defined by Layer 04 and
//!   carried by [`Transition<C>`]).
//!
//! # Determinism
//!
//! Same `(scheduler, graph, clock, backend-state)` ⇒ same
//! [`RuntimeOutcome`]. The runtime introduces no randomness, no wall
//! clock, no global state, no interior mutability, no threads, no
//! `unsafe`. All iteration is over `BTree*` collections or `Vec`.
//!
//! # Well-formedness
//!
//! The runtime assumes the caller submits actions in an order
//! consistent with the dependency structure — i.e. if `A` depends on
//! `B`, then `B` must be earlier than `A` in the scheduler's
//! `(scheduled_at, ActionId)` order, or `A` will be reported as
//! [`RuntimeOutcome::Blocked`] even though `B` is in the queue. This
//! is a caller-visible contract because the scheduler's deterministic
//! order is not negotiable without modifying its contract.
//!
//! # Lifecycle
//!
//! ```text
//!     Runtime::new(backend)
//!       │
//!       ▼
//!     runtime.submit(action, at)      ← one per action
//!     runtime.add_dependency(a, b)    ← one per edge
//!       │
//!       ▼
//!     runtime.run()                   ← repeatable
//!       │
//!       ▼
//!     RuntimeOutcome::{Completed | Blocked | Failed}
//! ```
//!
//! `run()` executes every currently-eligible action in scheduler
//! order, and stops when the scheduler is drained, when the head is
//! graph-blocked, or when the backend reports an execution failure.

use std::collections::BTreeSet;

use crate::action::{Action, ActionId, ActionPayload};
use crate::execution_graph::{ExecutionGraph, ExecutionGraphError};
use crate::executor::{self, ExecutionBackend, ExecutionError};
use crate::scheduler::{ScheduleError, Scheduler};
use crate::transition::Transition;
use helqix_time::{Clock, SimTime};

// ============================================================================
// RuntimeSubmitError
// ============================================================================

/// Errors raised by [`Runtime::submit`].
///
/// The only structural failure the runtime can detect on submission
/// is a duplicate identity: the same [`ActionId`] already being
/// present in the scheduler. `Scheduler` and `ExecutionGraph` both
/// reject duplicates; the runtime unifies those rejections here.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RuntimeSubmitError {
    /// An action with the same [`ActionId`] is already pending.
    DuplicateId { id: ActionId },
}

impl core::fmt::Display for RuntimeSubmitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DuplicateId { id } => {
                write!(f, "action {:?} is already submitted", id)
            }
        }
    }
}

impl std::error::Error for RuntimeSubmitError {}

// ============================================================================
// RuntimeOutcome
// ============================================================================

/// The result of one call to [`Runtime::run`].
///
/// Carries the transitions produced during that call and a
/// deterministic termination reason. `C` and `E` are the backend's
/// `Change` and `Error` types.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RuntimeOutcome<C, E> {
    /// Every currently-eligible action was executed successfully, and
    /// the scheduler is now empty for `clock.now()`.
    Completed {
        /// Transitions produced during this run, in execution order.
        transitions: Vec<Transition<C>>,
    },

    /// The head of the scheduler was graph-blocked — at least one of
    /// its predecessors has not yet been recorded as completed.
    Blocked {
        /// Transitions produced before the block, in execution order.
        transitions: Vec<Transition<C>>,
        /// The action that is waiting on dependencies.
        blocked_on: ActionId,
    },

    /// Execution failed at some stage of the pipeline.
    Failed {
        /// Transitions produced before the failure, in execution order.
        transitions: Vec<Transition<C>>,
        /// The stage-aware execution error.
        error: ExecutionError<E>,
    },
}

impl<C, E> RuntimeOutcome<C, E> {
    /// The transitions produced during this run.
    #[inline]
    #[must_use]
    pub fn transitions(&self) -> &[Transition<C>] {
        match self {
            Self::Completed { transitions }
            | Self::Blocked { transitions, .. }
            | Self::Failed { transitions, .. } => transitions,
        }
    }

    /// Did this run complete normally (scheduler drained)?
    #[inline]
    #[must_use]
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed { .. })
    }

    /// Did this run halt on an unresolved dependency?
    #[inline]
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked { .. })
    }

    /// Did this run fail during execution?
    #[inline]
    #[must_use]
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Consume the outcome and return the transitions.
    #[inline]
    #[must_use]
    pub fn into_transitions(self) -> Vec<Transition<C>> {
        match self {
            Self::Completed { transitions }
            | Self::Blocked { transitions, .. }
            | Self::Failed { transitions, .. } => transitions,
        }
    }
}

// ============================================================================
// Runtime
// ============================================================================

/// The Layer 05 execution runtime.
///
/// See the module documentation for the full responsibility model.
pub struct Runtime<'a, B: ExecutionBackend> {
    scheduler: Scheduler<B::Payload>,
    graph: ExecutionGraph,
    clock: Clock,
    backend: &'a mut B,
    completed: BTreeSet<ActionId>,
}

impl<'a, B: ExecutionBackend> Runtime<'a, B> {
    /// Create a runtime with a fresh clock at [`SimTime::ZERO`], an
    /// empty scheduler, and an empty dependency graph.
    #[inline]
    pub fn new(backend: &'a mut B) -> Self {
        Self {
            scheduler: Scheduler::new(),
            graph: ExecutionGraph::new(),
            clock: Clock::new(),
            backend,
            completed: BTreeSet::new(),
        }
    }

    /// Create a runtime starting from an explicit clock position.
    ///
    /// This is the constructor used by replay, branching, and any
    /// workflow that resumes from a non-zero logical time.
    #[inline]
    pub fn with_clock(clock: Clock, backend: &'a mut B) -> Self {
        Self {
            scheduler: Scheduler::new(),
            graph: ExecutionGraph::new(),
            clock,
            backend,
            completed: BTreeSet::new(),
        }
    }

    // ------------------------------------------------------------------
    // Setup
    // ------------------------------------------------------------------

    /// Submit an action to be executed at the logical time `at`.
    ///
    /// Adds the action to the scheduler and registers its identity as
    /// a node in the dependency graph. Returns
    /// [`RuntimeSubmitError::DuplicateId`] if an action with the same
    /// [`ActionId`] is already pending; the runtime is unchanged on
    /// failure.
    ///
    /// # Time
    ///
    /// `at` is a caller-supplied logical time from Layer 07. The
    /// runtime does not advance the clock to `at`; it only schedules
    /// the action. When `at <= clock.now()`, the action becomes
    /// eligible on the next call to [`Self::run`].
    pub fn submit(
        &mut self,
        action: Action<B::Payload>,
        at: SimTime,
    ) -> Result<(), RuntimeSubmitError> {
        let id = action.id();

        if self.scheduler.contains(id) {
            return Err(RuntimeSubmitError::DuplicateId { id });
        }

        // Add the graph node first. On success, schedule the action.
        // If scheduling unexpectedly fails, roll the graph node back
        // so no half-submitted state is visible.
        if self.graph.add_node(id).is_err() {
            return Err(RuntimeSubmitError::DuplicateId { id });
        }

        match self.scheduler.schedule(action, at) {
            Ok(()) => Ok(()),
            Err(ScheduleError::DuplicateId { .. }) => {
                // Defensive rollback: the guard above means this branch
                // is unreachable, but we keep the runtime invariant
                // — no dangling graph nodes — even under future
                // scheduler changes.
                let _ = self.graph.remove_node(id);
                Err(RuntimeSubmitError::DuplicateId { id })
            }
        }
    }

    /// Add an execution dependency: `from` must complete before `to`
    /// becomes eligible.
    ///
    /// Delegates to [`ExecutionGraph::add_dependency`]. All failure
    /// modes (unknown node, self-dependency, duplicate edge, cycle)
    /// are surfaced unchanged.
    pub fn add_dependency(
        &mut self,
        from: ActionId,
        to: ActionId,
    ) -> Result<(), ExecutionGraphError> {
        self.graph.add_dependency(from, to)
    }

    // ------------------------------------------------------------------
    // Execution
    // ------------------------------------------------------------------

    /// Execute every currently-eligible action, in scheduler order,
    /// until the scheduler drains, the head is graph-blocked, or the
    /// backend reports an execution failure.
    ///
    /// See the module documentation for the termination semantics.
    pub fn run(&mut self) -> RuntimeOutcome<B::Change, B::Error> {
        let mut transitions: Vec<Transition<B::Change>> = Vec::new();

        loop {
            let now = self.clock.now();

            // Peek at the scheduler's head without consuming it.
            let head_id = match self.scheduler.next_eligible(now) {
                Some(head) => head.action().id(),
                None => return RuntimeOutcome::Completed { transitions },
            };

            // Ask the graph whether the head is ready. Unknown-node is
            // treated as trivially ready: an action submitted through
            // `submit` always has a node, but a scheduler prepopulated
            // through other means might not.
            let ready = match self
                .graph
                .is_ready(head_id, |id| self.completed.contains(&id))
            {
                Ok(r) => r,
                Err(ExecutionGraphError::UnknownNode { .. }) => true,
                Err(_) => false,
            };

            if !ready {
                return RuntimeOutcome::Blocked {
                    transitions,
                    blocked_on: head_id,
                };
            }

            // Take the head. The guard above guarantees presence.
            let scheduled = match self.scheduler.take_next_eligible(now) {
                Some(s) => s,
                None => return RuntimeOutcome::Completed { transitions },
            };

            let scheduled_at = scheduled.scheduled_at();
            let action = scheduled.into_action();
            let id = action.id();

            match executor::execute(&action, &self.clock, self.backend) {
                Ok(transition) => {
                    self.completed.insert(id);
                    transitions.push(transition);
                }
                Err(error) => {
                    // A failed action stays pending at its original time so
                    // the caller can inspect and retry it; dropping it would
                    // lose the action, since `Failed` does not return it.
                    // It was just taken, so re-scheduling cannot collide.
                    let _ = self.scheduler.schedule(action, scheduled_at);
                    return RuntimeOutcome::Failed { transitions, error };
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Introspection
    // ------------------------------------------------------------------

    /// Number of actions still pending in the scheduler.
    #[inline]
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.scheduler.len()
    }

    /// The set of action identities that have executed successfully
    /// during this runtime's lifetime, in ascending order.
    #[inline]
    pub fn completed_ids(&self) -> impl Iterator<Item = ActionId> + '_ {
        self.completed.iter().copied()
    }

    /// Has every submitted action been executed?
    #[inline]
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.scheduler.is_empty()
    }

    /// Read-only access to the dependency graph.
    #[inline]
    #[must_use]
    pub fn graph(&self) -> &ExecutionGraph {
        &self.graph
    }

    /// Read-only access to the scheduler.
    #[inline]
    #[must_use]
    pub fn scheduler(&self) -> &Scheduler<B::Payload> {
        &self.scheduler
    }

    /// Read-only access to the logical clock.
    #[inline]
    #[must_use]
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Mutable access to the logical clock.
    ///
    /// This delegates directly to Layer 07's [`Clock`] API — it does
    /// not redefine time semantics. Callers use this to advance the
    /// clock between runs.
    #[inline]
    pub fn clock_mut(&mut self) -> &mut Clock {
        &mut self.clock
    }

    /// Read-only access to the execution backend.
    #[inline]
    #[must_use]
    pub fn backend(&self) -> &B {
        self.backend
    }

    /// Mutable access to the execution backend.
    #[inline]
    pub fn backend_mut(&mut self) -> &mut B {
        self.backend
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionError, ActionPayload};
    use helqix_time::SimDelta;

    // ------------------------------------------------------------------
    // Test fixtures
    // ------------------------------------------------------------------

    #[derive(Debug, Clone, Eq, PartialEq)]
    struct TestPayload(&'static str);

    impl ActionPayload for TestPayload {
        fn validate(&self) -> Result<(), ActionError> {
            Ok(())
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq)]
    struct TestError(&'static str);

    impl core::fmt::Display for TestError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str(self.0)
        }
    }

    impl std::error::Error for TestError {}

    /// A backend that records which actions were executed (by id) and
    /// supports per-id injection of execute/apply failures.
    struct TestBackend {
        executed: Vec<ActionId>,
        execute_fail: Option<(ActionId, &'static str)>,
        apply_fail: Option<(ActionId, &'static str)>,
    }

    impl TestBackend {
        fn new() -> Self {
            Self {
                executed: Vec::new(),
                execute_fail: None,
                apply_fail: None,
            }
        }

        fn with_execute_failure(mut self, id: ActionId, reason: &'static str) -> Self {
            self.execute_fail = Some((id, reason));
            self
        }

        fn with_apply_failure(mut self, id: ActionId, reason: &'static str) -> Self {
            self.apply_fail = Some((id, reason));
            self
        }
    }

    impl ExecutionBackend for TestBackend {
        type Payload = TestPayload;
        type Change = ActionId;
        type Error = TestError;

        fn validate(&self, _action: &Action<TestPayload>) -> Result<(), TestError> {
            Ok(())
        }

        fn execute(
            &mut self,
            action: &Action<TestPayload>,
        ) -> Result<ActionId, TestError> {
            if let Some((id, reason)) = self.execute_fail {
                if id == action.id() {
                    return Err(TestError(reason));
                }
            }
            Ok(action.id())
        }

        fn apply(&mut self, change: &ActionId) -> Result<(), TestError> {
            if let Some((id, reason)) = self.apply_fail {
                if id == *change {
                    return Err(TestError(reason));
                }
            }
            self.executed.push(*change);
            Ok(())
        }
    }

    fn id(n: u64) -> ActionId {
        ActionId::from_raw(n)
    }

    fn action(id_val: u64) -> Action<TestPayload> {
        Action::try_new(id(id_val), TestPayload("x")).unwrap()
    }

    fn ns(n: i128) -> SimTime {
        SimTime::from_nanos(n)
    }

    // ------------------------------------------------------------------
    // 1. Empty runtime
    // ------------------------------------------------------------------

    #[test]
    fn empty_runtime_runs_to_completion_immediately() {
        let mut backend = TestBackend::new();
        let mut rt = Runtime::new(&mut backend);

        assert!(rt.is_idle());
        assert_eq!(rt.pending_count(), 0);

        let outcome = rt.run();
        assert!(outcome.is_completed());
        assert!(outcome.transitions().is_empty());
        assert!(backend.executed.is_empty());
    }

    // ------------------------------------------------------------------
    // 2. Single action
    // ------------------------------------------------------------------

    #[test]
    fn one_action_executes_and_produces_transition() {
        let mut backend = TestBackend::new();
        let mut rt = Runtime::new(&mut backend);
        rt.submit(action(1), SimTime::ZERO).unwrap();

        let outcome = rt.run();

        assert!(outcome.is_completed());
        assert_eq!(outcome.transitions().len(), 1);
        let t = &outcome.transitions()[0];
        assert_eq!(t.action(), id(1));
        assert_eq!(t.applied_at(), SimTime::ZERO);
        assert_eq!(t.change(), &id(1));

        assert_eq!(rt.backend().executed, vec![id(1)]);
        assert!(rt.is_idle());
        assert_eq!(rt.completed_ids().collect::<Vec<_>>(), vec![id(1)]);
    }

    // ------------------------------------------------------------------
    // 3. Multiple actions execute in scheduler order
    // ------------------------------------------------------------------

    #[test]
    fn multiple_actions_execute_in_scheduler_order() {
        let mut backend = TestBackend::new();
        let mut rt = Runtime::new(&mut backend);

        // Same time, different ids — sorted by (time, id).
        rt.submit(action(3), ns(100)).unwrap();
        rt.submit(action(1), ns(100)).unwrap();
        rt.submit(action(2), ns(100)).unwrap();

        let outcome = rt.run();

        assert!(outcome.is_completed());
        let order: Vec<u64> = backend.executed.iter().map(|a| a.raw()).collect();
        assert_eq!(order, vec![1, 2, 3]);
    }

    #[test]
    fn action_order_reflects_scheduled_at_first() {
        let mut backend = TestBackend::new();
        let mut rt = Runtime::new(&mut backend);

        rt.submit(action(1), ns(300)).unwrap();
        rt.submit(action(2), ns(100)).unwrap();
        rt.submit(action(3), ns(200)).unwrap();

        let _ = rt.run();

        let order: Vec<u64> = backend.executed.iter().map(|a| a.raw()).collect();
        assert_eq!(order, vec![2, 3, 1]);
    }

    // ------------------------------------------------------------------
    // 4. Time gating
    // ------------------------------------------------------------------

    #[test]
    fn future_actions_are_not_eligible() {
        let mut backend = TestBackend::new();
        let mut rt = Runtime::new(&mut backend);
        rt.submit(action(1), ns(1_000_000)).unwrap();

        // Clock is at zero — the action isn't eligible.
        let outcome = rt.run();
        assert!(outcome.is_completed());
        assert!(outcome.transitions().is_empty());
        assert!(rt.backend().executed.is_empty());
        assert_eq!(rt.pending_count(), 1);
    }

    #[test]
    fn advancing_clock_makes_future_actions_eligible() {
        let mut backend = TestBackend::new();
        let mut rt = Runtime::new(&mut backend);
        rt.submit(action(1), ns(1_000_000)).unwrap();

        // Not eligible yet.
        assert!(rt.run().transitions().is_empty());

        // Advance.
        rt.clock_mut()
            .advance_by(SimDelta::from_nanos(1_000_000))
            .unwrap();

        // Now eligible.
        let outcome = rt.run();
        assert_eq!(outcome.transitions().len(), 1);
        assert_eq!(backend.executed, vec![id(1)]);
    }

    #[test]
    fn transition_carries_the_execution_time() {
        let mut backend = TestBackend::new();
        let mut rt = Runtime::with_clock(
            {
                let mut c = Clock::new();
                c.advance_by(SimDelta::from_nanos(12_480_000_000)).unwrap();
                c
            },
            &mut backend,
        );
        rt.submit(action(1), ns(12_480_000_000)).unwrap();

        let outcome = rt.run();
        assert_eq!(outcome.transitions()[0].applied_at(), ns(12_480_000_000));
    }

    // ------------------------------------------------------------------
    // 5. Dependencies
    // ------------------------------------------------------------------

    #[test]
    fn chain_executes_in_dependency_order() {
        // A(1) → B(2) → C(3). Scheduler order = id order = topological.
        let mut backend = TestBackend::new();
        let mut rt = Runtime::new(&mut backend);

        rt.submit(action(1), SimTime::ZERO).unwrap();
        rt.submit(action(2), SimTime::ZERO).unwrap();
        rt.submit(action(3), SimTime::ZERO).unwrap();
        rt.add_dependency(id(1), id(2)).unwrap();
        rt.add_dependency(id(2), id(3)).unwrap();

        let outcome = rt.run();
        assert!(outcome.is_completed());
        let order: Vec<u64> = backend.executed.iter().map(|a| a.raw()).collect();
        assert_eq!(order, vec![1, 2, 3]);
    }

    #[test]
    fn blocked_head_halts_execution() {
        // A(2) depends on B(1). Scheduler orders by id: 1 first.
        // But we make B's id larger so A is head: A(1), B(2); A depends on B.
        let mut backend = TestBackend::new();
        let mut rt = Runtime::new(&mut backend);

        rt.submit(action(1), SimTime::ZERO).unwrap(); // A
        rt.submit(action(2), SimTime::ZERO).unwrap(); // B
        rt.add_dependency(id(2), id(1)).unwrap(); // A depends on B

        let outcome = rt.run();

        assert!(outcome.is_blocked());
        match &outcome {
            RuntimeOutcome::Blocked { blocked_on, .. } => {
                assert_eq!(*blocked_on, id(1));
            }
            _ => unreachable!(),
        }
        // Nothing executed.
        assert!(rt.backend().executed.is_empty());
        // Scheduler still has both.
        assert_eq!(rt.pending_count(), 2);
    }

    #[test]
    fn diamond_dependency_resolves_in_order() {
        //   A(1) ──┐
        //         ├──> C(3)
        //   B(2) ──┘
        let mut backend = TestBackend::new();
        let mut rt = Runtime::new(&mut backend);

        rt.submit(action(1), SimTime::ZERO).unwrap();
        rt.submit(action(2), SimTime::ZERO).unwrap();
        rt.submit(action(3), SimTime::ZERO).unwrap();
        rt.add_dependency(id(1), id(3)).unwrap();
        rt.add_dependency(id(2), id(3)).unwrap();

        let outcome = rt.run();
        assert!(outcome.is_completed());
        let order: Vec<u64> = backend.executed.iter().map(|a| a.raw()).collect();
        assert_eq!(order, vec![1, 2, 3]);
    }

    // ------------------------------------------------------------------
    // 6. Failure semantics
    // ------------------------------------------------------------------

    #[test]
    fn execute_failure_halts_and_carries_stage() {
        let mut backend = TestBackend::new().with_execute_failure(id(2), "boom");
        let mut rt = Runtime::new(&mut backend);
        rt.submit(action(1), SimTime::ZERO).unwrap();
        rt.submit(action(2), SimTime::ZERO).unwrap();
        rt.submit(action(3), SimTime::ZERO).unwrap();

        let outcome = rt.run();

        assert!(outcome.is_failed());
        match outcome {
            RuntimeOutcome::Failed { transitions, error } => {
                assert_eq!(transitions.len(), 1);
                assert_eq!(transitions[0].action(), id(1));
                assert_eq!(error.stage(), crate::executor::ExecutionStage::Execute);
                assert_eq!(error.source_ref(), &TestError("boom"));
            }
            _ => unreachable!(),
        }
        // Action 1 executed; actions 2 and 3 did not.
        assert_eq!(rt.backend().executed, vec![id(1)]);
        // Scheduler still holds the unexecuted actions.
        assert_eq!(rt.pending_count(), 2);
        // Action 1 is marked completed.
        assert_eq!(rt.completed_ids().collect::<Vec<_>>(), vec![id(1)]);
    }

    #[test]
    fn apply_failure_carries_apply_stage() {
        let mut backend = TestBackend::new().with_apply_failure(id(1), "rejected");
        let mut rt = Runtime::new(&mut backend);
        rt.submit(action(1), SimTime::ZERO).unwrap();

        let outcome = rt.run();
        assert!(outcome.is_failed());
        match outcome {
            RuntimeOutcome::Failed { error, .. } => {
                assert_eq!(error.stage(), crate::executor::ExecutionStage::Apply);
                assert_eq!(error.source_ref(), &TestError("rejected"));
            }
            _ => unreachable!(),
        }
        // No side effects.
        assert!(rt.backend().executed.is_empty());
        // Not marked completed.
        assert_eq!(rt.completed_ids().count(), 0);
        // Still pending.
        assert_eq!(rt.pending_count(), 1);
    }

    #[test]
    fn failed_action_can_be_retried_after_clearing_failure() {
        let mut backend = TestBackend::new().with_execute_failure(id(1), "transient");
        let mut rt = Runtime::new(&mut backend);
        rt.submit(action(1), SimTime::ZERO).unwrap();

        // First run fails.
        assert!(rt.run().is_failed());

        // Clear the failure on the backend.
        rt.backend_mut().execute_fail = None;

        // Second run succeeds — same action, same time.
        let outcome = rt.run();
        assert!(outcome.is_completed());
        assert_eq!(outcome.transitions().len(), 1);
        assert_eq!(backend.executed, vec![id(1)]);
    }

    // ------------------------------------------------------------------
    // 7. Determinism
    // ------------------------------------------------------------------

    #[test]
    fn deterministic_execution_across_instances() {
        let build = || {
            let mut backend = TestBackend::new();
            {
                let mut rt = Runtime::new(&mut backend);
                rt.submit(action(3), ns(200)).unwrap();
                rt.submit(action(1), ns(100)).unwrap();
                rt.submit(action(2), ns(100)).unwrap();
                rt.submit(action(4), ns(300)).unwrap();
                let outcome = rt.run();
                assert!(outcome.is_completed());
            }
            backend.executed
        };

        let a = build();
        let b = build();
        let c = build();
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(
            a.iter().map(|x| x.raw()).collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
    }

    #[test]
    fn deterministic_transitions_across_instances() {
        let build = || {
            let mut backend = TestBackend::new();
            let mut rt = Runtime::with_clock(
                {
                    let mut c = Clock::new();
                    c.advance_by(SimDelta::from_nanos(500)).unwrap();
                    c
                },
                &mut backend,
            );
            rt.submit(action(1), ns(0)).unwrap();
            rt.submit(action(2), ns(0)).unwrap();
            rt.run().into_transitions()
        };

        let a = build();
        let b = build();
        assert_eq!(a, b);
    }

    // ------------------------------------------------------------------
    // 8. Submission semantics
    // ------------------------------------------------------------------

    #[test]
    fn duplicate_submission_is_rejected_and_leaves_runtime_unchanged() {
        let mut backend = TestBackend::new();
        let mut rt = Runtime::new(&mut backend);
        rt.submit(action(1), SimTime::ZERO).unwrap();

        let err = rt.submit(action(1), ns(999)).unwrap_err();
        assert_eq!(err, RuntimeSubmitError::DuplicateId { id: id(1) });

        // Runtime state unchanged: one pending, no graph dangling node.
        assert_eq!(rt.pending_count(), 1);
        assert_eq!(rt.graph().node_count(), 1);
        assert!(rt.graph().contains_node(id(1)));
    }

    #[test]
    fn add_dependency_on_unknown_node_is_rejected() {
        let mut backend = TestBackend::new();
        let mut rt = Runtime::new(&mut backend);
        rt.submit(action(1), SimTime::ZERO).unwrap();

        let err = rt.add_dependency(id(1), id(99)).unwrap_err();
        match err {
            ExecutionGraphError::UnknownNode { id: unknown } => {
                assert_eq!(unknown.raw(), 99);
            }
            _ => panic!("expected UnknownNode"),
        }
    }

    // ------------------------------------------------------------------
    // 9. Repeated runs
    // ------------------------------------------------------------------

    #[test]
    fn run_is_idempotent_when_idle() {
        let mut backend = TestBackend::new();
        let mut rt = Runtime::new(&mut backend);

        let r1 = rt.run();
        let r2 = rt.run();
        assert!(r1.is_completed());
        assert!(r2.is_completed());
        assert!(r1.transitions().is_empty());
        assert!(r2.transitions().is_empty());
    }

    #[test]
    fn run_can_be_invoked_repeatedly_as_time_advances() {
        let mut backend = TestBackend::new();
        let mut rt = Runtime::new(&mut backend);
        rt.submit(action(1), ns(100)).unwrap();
        rt.submit(action(2), ns(200)).unwrap();
        rt.submit(action(3), ns(300)).unwrap();

        // Run 1: nothing eligible yet.
        assert!(rt.run().transitions().is_empty());
        assert_eq!(rt.backend().executed.len(), 0);

        // Advance to 100.
        rt.clock_mut()
            .advance_by(SimDelta::from_nanos(100))
            .unwrap();
        let r2 = rt.run();
        assert_eq!(r2.transitions().len(), 1);
        assert_eq!(rt.backend().executed, vec![id(1)]);

        // Advance to 300.
        rt.clock_mut()
            .advance_by(SimDelta::from_nanos(200))
            .unwrap();
        let r3 = rt.run();
        assert_eq!(r3.transitions().len(), 2);
        assert_eq!(backend.executed, vec![id(1), id(2), id(3)]);
    }

    // ------------------------------------------------------------------
    // 10. Introspection
    // ------------------------------------------------------------------

    #[test]
    fn clock_is_not_advanced_by_run() {
        let mut backend = TestBackend::new();
        let mut rt = Runtime::new(&mut backend);
        rt.submit(action(1), SimTime::ZERO).unwrap();

        let before = rt.clock().now();
        let _ = rt.run();
        assert_eq!(rt.clock().now(), before);
    }

    #[test]
    fn completed_ids_reflect_execution_after_failure() {
        let mut backend = TestBackend::new().with_execute_failure(id(2), "stop");
        let mut rt = Runtime::new(&mut backend);
        rt.submit(action(1), SimTime::ZERO).unwrap();
        rt.submit(action(2), SimTime::ZERO).unwrap();

        let _ = rt.run();

        let done: Vec<u64> = rt.completed_ids().map(|a| a.raw()).collect();
        assert_eq!(done, vec![1]);
    }

    // ------------------------------------------------------------------
    // 11. Worked failure workflow
    // ------------------------------------------------------------------

    #[test]
    fn worked_failure_workflow_preserves_evidence() {
        // Sequence: A(1) executes, B(2) executes, C(3) fails.
        // Runtime must:
        //   * produce 2 transitions before the failure,
        //   * carry the ExecutionError with its stage,
        //   * leave the scheduler with only C(3) pending,
        //   * leave completed = {1, 2}.
        let mut backend = TestBackend::new().with_execute_failure(id(3), "thermal_limit");
        let mut rt = Runtime::new(&mut backend);

        rt.submit(action(1), SimTime::ZERO).unwrap();
        rt.submit(action(2), SimTime::ZERO).unwrap();
        rt.submit(action(3), SimTime::ZERO).unwrap();

        let outcome = rt.run();

        assert!(outcome.is_failed());
        let transitions = outcome.transitions();
        assert_eq!(transitions.len(), 2);
        assert_eq!(transitions[0].action(), id(1));
        assert_eq!(transitions[1].action(), id(2));

        assert_eq!(rt.pending_count(), 1);
        assert_eq!(rt.backend().executed, vec![id(1), id(2)]);
        assert_eq!(
            rt.completed_ids().map(|a| a.raw()).collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    // ------------------------------------------------------------------
    // 12. Cross-type independence
    // ------------------------------------------------------------------

    #[test]
    fn runtimes_of_different_payload_types_coexist() {
        #[derive(Debug, Clone, Eq, PartialEq)]
        struct OtherPayload;
        impl ActionPayload for OtherPayload {
            fn validate(&self) -> Result<(), ActionError> {
                Ok(())
            }
        }

        struct OtherBackend;
        impl ExecutionBackend for OtherBackend {
            type Payload = OtherPayload;
            type Change = ();
            type Error = TestError;
            fn validate(&self, _: &Action<OtherPayload>) -> Result<(), TestError> {
                Ok(())
            }
            fn execute(&mut self, _: &Action<OtherPayload>) -> Result<(), TestError> {
                Ok(())
            }
            fn apply(&mut self, _: &()) -> Result<(), TestError> {
                Ok(())
            }
        }

        let mut b1 = TestBackend::new();
        let mut b2 = OtherBackend;

        let mut rt1 = Runtime::new(&mut b1);
        let mut rt2 = Runtime::new(&mut b2);

        rt1.submit(action(1), SimTime::ZERO).unwrap();
        rt2.submit(
            Action::try_new(id(1), OtherPayload).unwrap(),
            SimTime::ZERO,
        )
        .unwrap();

        assert!(rt1.run().is_completed());
        assert!(rt2.run().is_completed());
        assert_eq!(b1.executed, vec![id(1)]);
    }
}