//! Independent state and lineage metadata for one executable branch.
use std::{error::Error, fmt};
use helqix_core::Identifier;
use helqix_state::{State, StateError};
use helqix_time::SimTime;
use crate::{BranchPoint, BranchPointError, StateFork, StateForkError};

/// Stable identity of a branch.
pub type BranchId = Identifier;

/// One branch's current independently mutable continuation.
#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    id: BranchId,
    parent: Option<BranchId>,
    forked_from: Option<BranchPoint>,
    current_time: SimTime,
    state: State,
    generation: u32,
    ordinal: u64,
}

impl Branch {
    /// Creates a root branch from a valid initial state.
    pub fn root(id: BranchId, at: SimTime, state: State) -> Result<Self, BranchError> {
        state.validate().map_err(BranchError::InvalidState)?;
        Ok(Self { id, parent: None, forked_from: None, current_time: at, state, generation: 0, ordinal: 0 })
    }

    pub(crate) fn child(id: BranchId, point: BranchPoint, generation: u32, ordinal: u64) -> Result<Self, BranchError> {
        let fork = StateFork::from_point(&point).map_err(BranchError::Fork)?;
        Ok(Self { id, parent: Some(point.branch_id().clone()), current_time: fork.forked_at(), state: fork.into_state(), forked_from: Some(point), generation, ordinal })
    }

    /// Branch identifier.
    pub fn id(&self) -> &BranchId { &self.id }
    /// Parent branch, or `None` for the root.
    pub fn parent(&self) -> Option<&BranchId> { self.parent.as_ref() }
    /// Exact point from which this branch was created.
    pub fn forked_from(&self) -> Option<&BranchPoint> { self.forked_from.as_ref() }
    /// Current logical time of this branch.
    pub fn current_time(&self) -> SimTime { self.current_time }
    /// Current runtime state of this branch.
    pub fn state(&self) -> &State { &self.state }
    /// Branch depth from the root (root depth is zero).
    pub fn generation(&self) -> u32 { self.generation }
    /// Stable insertion ordinal within its tree.
    pub fn ordinal(&self) -> u64 { self.ordinal }
    /// Captures this branch's current state and time for future forks.
    pub fn capture_point(&self) -> Result<BranchPoint, BranchPointError> {
        BranchPoint::capture(self.id.clone(), self.current_time, &self.state)
    }

    /// Commits a new state at a non-decreasing logical time.
    ///
    /// Validation is performed before mutation. On any error, both the branch
    /// state and its time remain unchanged.
    pub fn commit(&mut self, at: SimTime, state: State) -> Result<(), BranchError> {
        if at < self.current_time { return Err(BranchError::BackwardInTime { current: self.current_time, attempted: at }); }
        state.validate().map_err(BranchError::InvalidState)?;
        self.current_time = at;
        self.state = state;
        Ok(())
    }
}

/// Failure while creating or advancing a branch.
#[derive(Debug, Clone, PartialEq)]
pub enum BranchError {
    /// Runtime state failed Layer 04 validation.
    InvalidState(StateError),
    /// The branch point could not be cloned into an independent state.
    Fork(StateForkError),
    /// A commit attempted to move branch time backwards.
    BackwardInTime { /// Branch's current logical time.
        current: SimTime, /// Attempted logical time.
        attempted: SimTime },
}
impl fmt::Display for BranchError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self {
    Self::InvalidState(error) => write!(f, "branch state is invalid: {error}"),
    Self::Fork(error) => write!(f, "branch fork failed: {error}"),
    Self::BackwardInTime { current, attempted } => write!(f, "branch commit at {attempted} precedes current time {current}"),
} } }
impl Error for BranchError { fn source(&self) -> Option<&(dyn Error + 'static)> { match self {
    Self::InvalidState(error) => Some(error), Self::Fork(error) => Some(error), Self::BackwardInTime { .. } => None,
} } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn root_has_no_parent_and_zero_generation() {
        let branch = Branch::root(Identifier::parse("main").unwrap(), SimTime::ZERO, State::new()).unwrap();
        assert!(branch.parent().is_none());
        assert_eq!(branch.generation(), 0);
    }
    #[test]
    fn backward_commit_is_rejected_without_mutation() {
        let mut branch = Branch::root(Identifier::parse("main").unwrap(), SimTime::from_nanos(10), State::new()).unwrap();
        let original = branch.clone();
        assert!(matches!(branch.commit(SimTime::from_nanos(9), State::new()), Err(BranchError::BackwardInTime { .. })));
        assert_eq!(branch, original);
    }
}
