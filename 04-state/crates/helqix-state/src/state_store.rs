//! In-memory state owner with revisioned, bounded snapshot history.
//!
//! A store accepts only complete transitions. It constructs the resulting
//! snapshot before changing the current state, so a rejected transition or
//! failed validation leaves both current state and history unchanged. History
//! retention is explicit and bounded to prevent an unbounded memory leak.

use std::{
    collections::VecDeque,
    error::Error,
    fmt,
    num::NonZeroUsize,
};

use crate::{
    snapshot::{Snapshot, SnapshotError},
    state_diff::{RevisionedStateDiff, RevisionedStateDiffError, StateDiff},
    state::State,
    transition::{StateTransition, TransitionError},
};

/// Default number of state revisions retained by a [`StateStore`].
pub const DEFAULT_HISTORY_LIMIT: usize = 256;

/// Owns the current state and the most recent accepted revision snapshots.
#[derive(Debug, Clone, PartialEq)]
pub struct StateStore {
    current: State,
    history: VecDeque<Snapshot>,
    history_limit: NonZeroUsize,
}

impl StateStore {
    /// Creates a store retaining the latest [`DEFAULT_HISTORY_LIMIT`] states.
    pub fn new(initial: State) -> Result<Self, StateStoreError> {
        let limit = NonZeroUsize::new(DEFAULT_HISTORY_LIMIT)
            .expect("the built-in history limit is nonzero");
        Self::with_history_limit(initial, limit)
    }

    /// Creates a store with a caller-selected positive history capacity.
    pub fn with_history_limit(
        initial: State,
        history_limit: NonZeroUsize,
    ) -> Result<Self, StateStoreError> {
        let initial_snapshot = Snapshot::capture(&initial).map_err(StateStoreError::Snapshot)?;
        let mut history = VecDeque::new();
        history.push_back(initial_snapshot);
        Ok(Self { current: initial, history, history_limit })
    }

    /// Returns the current validated state.
    #[must_use]
    pub const fn current(&self) -> &State { &self.current }

    /// Returns retained snapshots from oldest to newest.
    #[must_use]
    pub fn history(&self) -> &VecDeque<Snapshot> { &self.history }

    /// Returns the configured maximum number of retained snapshots.
    #[must_use]
    pub const fn history_limit(&self) -> NonZeroUsize { self.history_limit }

    /// Returns the snapshot corresponding to the current state.
    #[must_use]
    pub fn latest_snapshot(&self) -> &Snapshot {
        self.history.back().expect("store invariant: current snapshot is retained")
    }

    /// Finds a retained snapshot by its state revision.
    #[must_use]
    pub fn snapshot_at(&self, revision: u64) -> Option<&Snapshot> {
        self.history.iter().find(|snapshot| snapshot.revision() == revision)
    }

    /// Computes the deterministic difference from a retained revision to the
    /// current state. Returns an error if retention has already evicted it.
    pub fn diff_from(&self, revision: u64) -> Result<RevisionedStateDiff, StateStoreError> {
        let before = self.snapshot_at(revision)
            .ok_or(StateStoreError::SnapshotNotRetained { revision })?;
        StateDiff::between_revisioned(before, self.latest_snapshot())
            .map_err(StateStoreError::Diff)
    }

    /// Applies a transition and commits the resulting state and snapshot.
    ///
    /// If the transition fails validation, has a stale revision, or overflows
    /// the revision counter, this store remains unchanged.
    pub fn apply(
        &mut self,
        transition: &StateTransition,
    ) -> Result<&State, StateStoreError> {
        let next = transition.apply(&self.current).map_err(StateStoreError::Transition)?;
        let next_snapshot = Snapshot::capture(&next).map_err(StateStoreError::Snapshot)?;

        self.current = next;
        self.history.push_back(next_snapshot);
        while self.history.len() > self.history_limit.get() {
            self.history.pop_front();
        }
        Ok(&self.current)
    }
}

/// Errors raised while initializing or updating a [`StateStore`].
#[derive(Debug, Clone, PartialEq)]
pub enum StateStoreError {
    /// The initial state or resulting snapshot failed validation.
    Snapshot(SnapshotError),
    /// A requested transition could not be committed.
    Transition(TransitionError),
    /// A diff requested a revision older than the retained history window.
    SnapshotNotRetained { revision: u64 },
    /// The retained snapshots could not be converted into component snapshots.
    Diff(RevisionedStateDiffError),
}

impl fmt::Display for StateStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(error) => write!(f, "state store snapshot error: {error}"),
            Self::Transition(error) => write!(f, "state store transition error: {error}"),
            Self::SnapshotNotRetained { revision } => {
                write!(f, "state revision {revision} is no longer retained")
            }
            Self::Diff(error) => write!(f, "state store diff error: {error}"),
        }
    }
}

impl Error for StateStoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            Self::Transition(error) => Some(error),
            Self::SnapshotNotRetained { .. } => None,
            Self::Diff(error) => Some(error),
        }
    }
}
