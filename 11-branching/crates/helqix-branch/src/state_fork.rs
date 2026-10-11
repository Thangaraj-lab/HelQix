//! Isolated state cloning at an immutable branch point.
use std::{error::Error, fmt};
use helqix_state::{State, StateError};
use helqix_time::SimTime;
use crate::{BranchId, BranchPoint};

/// A validated, independent working copy derived from a [`BranchPoint`].
#[derive(Debug, Clone, PartialEq)]
pub struct StateFork {
    source_branch: BranchId,
    forked_at: SimTime,
    state: State,
}

impl StateFork {
    /// Creates an isolated state copy from a validated fork point.
    pub fn from_point(point: &BranchPoint) -> Result<Self, StateForkError> {
        point.validate().map_err(StateForkError::InvalidPoint)?;
        Ok(Self { source_branch: point.branch_id().clone(), forked_at: point.at(), state: point.fork_state() })
    }
    /// Branch that supplied the captured state.
    pub fn source_branch(&self) -> &BranchId { &self.source_branch }
    /// Logical time from which this state was forked.
    pub fn forked_at(&self) -> SimTime { self.forked_at }
    /// Immutable view of the forked state.
    pub fn state(&self) -> &State { &self.state }
    /// Consumes the fork and returns its owned state for independent execution.
    pub fn into_state(self) -> State { self.state }
    /// Returns a further independent clone of this fork's state.
    pub fn duplicate(&self) -> Result<Self, StateForkError> {
        self.state.validate().map_err(StateForkError::InvalidState)?;
        Ok(Self { source_branch: self.source_branch.clone(), forked_at: self.forked_at, state: self.state.clone() })
    }
}

/// Error while validating or producing an isolated state fork.
#[derive(Debug, Clone, PartialEq)]
pub enum StateForkError {
    /// The source branch point failed validation.
    InvalidPoint(crate::BranchPointError),
    /// The state failed validation while duplicating the fork.
    InvalidState(StateError),
}
impl fmt::Display for StateForkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self {
        Self::InvalidPoint(error) => write!(f, "cannot fork invalid branch point: {error}"),
        Self::InvalidState(error) => write!(f, "cannot duplicate invalid fork state: {error}"),
    } }
}
impl Error for StateForkError {
    fn source(&self) -> Option<&(dyn Error + 'static)> { match self {
        Self::InvalidPoint(error) => Some(error), Self::InvalidState(error) => Some(error),
    } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helqix_core::Identifier;

    #[test]
    fn sibling_forks_own_equal_but_independent_values() {
        let state = State::new();
        let point = BranchPoint::capture(Identifier::parse("main").unwrap(), SimTime::ZERO, &state).unwrap();
        let first = StateFork::from_point(&point).unwrap();
        let second = first.duplicate().unwrap();
        assert_eq!(first.state(), second.state());
        assert_ne!(first.state() as *const State, second.state() as *const State);
    }
}
