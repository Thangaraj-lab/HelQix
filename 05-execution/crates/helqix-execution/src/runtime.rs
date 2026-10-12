//! Deterministic Layer 05 execution runtime.

use std::collections::BTreeSet;

use crate::action::{Action, ActionId};
use crate::execution_graph::{ExecutionGraph, ExecutionGraphError};
use crate::executor::{self, ExecutionBackend, ExecutionError};
use crate::scheduler::{ScheduleError, Scheduler};
use crate::transition::Transition;
use helqix_time::{Clock, SimTime};

/// Errors raised when an action cannot be submitted.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RuntimeSubmitError {
    /// An action with the same identity is already pending.
    DuplicateId { id: ActionId },
}

impl core::fmt::Display for RuntimeSubmitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DuplicateId { id } => write!(f, "action {:?} is already submitted", id),
        }
    }
}

impl std::error::Error for RuntimeSubmitError {}

/// Result of one deterministic runtime execution pass.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RuntimeOutcome<C, E> {
    /// All currently eligible actions completed.
    Completed { transitions: Vec<Transition<C>> },
    /// The scheduler head is waiting on an unresolved dependency.
    Blocked {
        transitions: Vec<Transition<C>>,
        blocked_on: ActionId,
    },
    /// Backend execution failed.
    Failed {
        transitions: Vec<Transition<C>>,
        error: ExecutionError<E>,
    },
}

impl<C, E> RuntimeOutcome<C, E> {
    /// Borrow transitions produced during this pass.
    #[must_use]
    pub fn transitions(&self) -> &[Transition<C>] {
        match self {
            Self::Completed { transitions }
            | Self::Blocked { transitions, .. }
            | Self::Failed { transitions, .. } => transitions,
        }
    }

    /// Whether execution completed without a block or failure.
    #[must_use]
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed { .. })
    }

    /// Whether execution stopped on a dependency block.
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked { .. })
    }

    /// Whether execution stopped because the backend failed.
    #[must_use]
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Consume the outcome and return its transitions.
    #[must_use]
    pub fn into_transitions(self) -> Vec<Transition<C>> {
        match self {
            Self::Completed { transitions }
            | Self::Blocked { transitions, .. }
            | Self::Failed { transitions, .. } => transitions,
        }
    }
}

/// Top-level deterministic execution coordinator for Layer 05.
pub struct Runtime<'a, B: ExecutionBackend> {
    scheduler: Scheduler<B::Payload>,
    graph: ExecutionGraph,
    clock: Clock,
    backend: &'a mut B,
    completed: BTreeSet<ActionId>,
}

impl<'a, B: ExecutionBackend> Runtime<'a, B> {
    /// Create a runtime starting at logical time zero.
    #[must_use]
    pub fn new(backend: &'a mut B) -> Self {
        Self {
            scheduler: Scheduler::new(),
            graph: ExecutionGraph::new(),
            clock: Clock::new(),
            backend,
            completed: BTreeSet::new(),
        }
    }

    /// Create a runtime using an existing logical clock.
    #[must_use]
    pub fn with_clock(clock: Clock, backend: &'a mut B) -> Self {
        Self {
            scheduler: Scheduler::new(),
            graph: ExecutionGraph::new(),
            clock,
            backend,
            completed: BTreeSet::new(),
        }
    }

    /// Submit an action at a logical time.
    pub fn submit(
        &mut self,
        action: Action<B::Payload>,
        at: SimTime,
    ) -> Result<(), RuntimeSubmitError> {
        let id = action.id();
        if self.scheduler.contains(id) {
            return Err(RuntimeSubmitError::DuplicateId { id });
        }
        if self.graph.add_node(id).is_err() {
            return Err(RuntimeSubmitError::DuplicateId { id });
        }
        match self.scheduler.schedule(action, at) {
            Ok(()) => Ok(()),
            Err(ScheduleError::DuplicateId { .. }) => {
                let _ = self.graph.remove_node(id);
                Err(RuntimeSubmitError::DuplicateId { id })
            }
        }
    }

    /// Add a dependency where `from` must complete before `to`.
    pub fn add_dependency(
        &mut self,
        from: ActionId,
        to: ActionId,
    ) -> Result<(), ExecutionGraphError> {
        self.graph.add_dependency(from, to)
    }

    /// Execute all currently eligible actions in deterministic scheduler order.
    pub fn run(&mut self) -> RuntimeOutcome<B::Change, B::Error> {
        let mut transitions = Vec::new();
        loop {
            let now = self.clock.now();
            let head_id = match self.scheduler.next_eligible(now) {
                Some(head) => head.action().id(),
                None => return RuntimeOutcome::Completed { transitions },
            };

            let ready = match self
                .graph
                .is_ready(head_id, |id| self.completed.contains(&id))
            {
                Ok(value) => value,
                Err(ExecutionGraphError::UnknownNode { .. }) => true,
                Err(_) => false,
            };
            if !ready {
                return RuntimeOutcome::Blocked {
                    transitions,
                    blocked_on: head_id,
                };
            }

            let scheduled = match self.scheduler.take_next_eligible(now) {
                Some(value) => value,
                None => return RuntimeOutcome::Completed { transitions },
            };
            let action = scheduled.into_action();
            let id = action.id();

            match executor::execute(&action, &self.clock, self.backend) {
                Ok(transition) => {
                    self.completed.insert(id);
                    transitions.push(transition);
                }
                Err(error) => return RuntimeOutcome::Failed { transitions, error },
            }
        }
    }

    /// Number of actions still pending.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.scheduler.len()
    }

    /// IDs of successfully executed actions.
    pub fn completed_ids(&self) -> impl Iterator<Item = ActionId> + '_ {
        self.completed.iter().copied()
    }

    /// Whether no action remains pending.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.scheduler.is_empty()
    }

    /// Read-only dependency graph.
    #[must_use]
    pub fn graph(&self) -> &ExecutionGraph {
        &self.graph
    }

    /// Read-only scheduler.
    #[must_use]
    pub fn scheduler(&self) -> &Scheduler<B::Payload> {
        &self.scheduler
    }

    /// Read-only logical clock.
    #[must_use]
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Mutable logical clock.
    pub fn clock_mut(&mut self) -> &mut Clock {
        &mut self.clock
    }

    /// Read-only backend access. This is useful for inspecting backend state
    /// while keeping the runtime's mutable borrow scoped to the runtime.
    #[must_use]
    pub fn backend(&self) -> &B {
        self.backend
    }

    /// Mutable backend access.
    pub fn backend_mut(&mut self) -> &mut B {
        self.backend
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionError, ActionPayload};
    use helqix_time::SimDelta;

    #[derive(Debug, Clone, Eq, PartialEq)]
    struct Payload;
    impl ActionPayload for Payload {
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

    #[derive(Default)]
    struct Backend {
        executed: Vec<ActionId>,
        fail: Option<ActionId>,
    }
    impl ExecutionBackend for Backend {
        type Payload = Payload;
        type Change = ActionId;
        type Error = TestError;
        fn validate(&self, _: &Action<Payload>) -> Result<(), TestError> { Ok(()) }
        fn execute(&mut self, action: &Action<Payload>) -> Result<ActionId, TestError> {
            if self.fail == Some(action.id()) { Err(TestError("execute failed")) } else { Ok(action.id()) }
        }
        fn apply(&mut self, change: &ActionId) -> Result<(), TestError> {
            self.executed.push(*change);
            Ok(())
        }
    }

    fn action(raw: u64) -> Action<Payload> {
        Action::try_new(ActionId::from_raw(raw), Payload).unwrap()
    }

    #[test]
    fn empty_runtime_completes() {
        let mut backend = Backend::default();
        let mut runtime = Runtime::new(&mut backend);
        let result = runtime.run();
        assert!(result.is_completed());
        assert!(result.transitions().is_empty());
        assert!(runtime.is_idle());
    }

    #[test]
    fn actions_execute_in_deterministic_order() {
        let executed = {
            let mut backend = Backend::default();
            let mut runtime = Runtime::new(&mut backend);
            runtime.submit(action(3), SimTime::ZERO).unwrap();
            runtime.submit(action(1), SimTime::ZERO).unwrap();
            runtime.submit(action(2), SimTime::ZERO).unwrap();
            assert!(runtime.run().is_completed());
            runtime.backend().executed.clone()
        };
        assert_eq!(executed, vec![ActionId::from_raw(1), ActionId::from_raw(2), ActionId::from_raw(3)]);
    }

    #[test]
    fn future_action_waits_for_logical_time() {
        let (executed_before, executed_after) = {
            let mut backend = Backend::default();
            let mut runtime = Runtime::new(&mut backend);
            runtime.submit(action(1), SimTime::from_nanos(100)).unwrap();
            let before = runtime.run().transitions().len();
            runtime.clock_mut().advance_by(SimDelta::from_nanos(100)).unwrap();
            let after = runtime.run().transitions().len();
            (before, after)
        };
        assert_eq!(executed_before, 0);
        assert_eq!(executed_after, 1);
    }

    #[test]
    fn dependency_block_is_reported_without_side_effects() {
        let (result, executed) = {
            let mut backend = Backend::default();
            let mut runtime = Runtime::new(&mut backend);
            runtime.submit(action(1), SimTime::ZERO).unwrap();
            runtime.submit(action(2), SimTime::ZERO).unwrap();
            runtime.add_dependency(ActionId::from_raw(2), ActionId::from_raw(1)).unwrap();
            let result = runtime.run();
            (result.is_blocked(), runtime.backend().executed.clone())
        };
        assert!(result);
        assert!(executed.is_empty());
    }

    #[test]
    fn backend_failure_preserves_completed_prefix() {
        let (failed, executed, completed) = {
            let mut backend = Backend { fail: Some(ActionId::from_raw(2)), ..Default::default() };
            let mut runtime = Runtime::new(&mut backend);
            runtime.submit(action(1), SimTime::ZERO).unwrap();
            runtime.submit(action(2), SimTime::ZERO).unwrap();
            let result = runtime.run();
            let executed = runtime.backend().executed.clone();
            let completed = runtime.completed_ids().collect::<Vec<_>>();
            (result.is_failed(), executed, completed)
        };
        assert!(failed);
        assert_eq!(executed, vec![ActionId::from_raw(1)]);
        assert_eq!(completed, vec![ActionId::from_raw(1)]);
    }
}
