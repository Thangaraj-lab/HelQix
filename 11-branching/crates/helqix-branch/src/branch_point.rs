//! Exact immutable state and time from which a child branch can be created.
use std::{error::Error, fmt};
use helqix_core::Identifier;
use helqix_state::{State, StateError};
use helqix_time::SimTime;

use crate::BranchId;

/// A frozen fork point belonging to one source branch.
///
/// The owned state makes the point durable and prevents later edits to the
/// source branch from changing the state used by any child branch.
#[derive(Debug, Clone, PartialEq)]
pub struct BranchPoint {
    branch_id: BranchId,
    at: SimTime,
    state: State,
}

impl BranchPoint {
    /// Captures a branch's current state at `at`.
    pub(crate) fn capture(branch_id: BranchId, at: SimTime, state: &State) -> Result<Self, BranchPointError> {
        state.validate().map_err(BranchPointError::InvalidState)?;
        Ok(Self { branch_id, at, state: state.clone() })
    }

    /// Source branch identity.
    pub fn branch_id(&self) -> &BranchId { &self.branch_id }
    /// Exact logical time represented by this point.
    pub fn at(&self) -> SimTime { self.at }
    /// State revision represented by this point.
    pub fn state_revision(&self) -> u64 { self.state.revision() }
    /// Immutable captured state.
    pub fn state(&self) -> &State { &self.state }
    /// Number of components captured in this point.
    pub fn component_count(&self) -> usize { self.state.component_count() }

    /// Validates the owned state again at an explicit trust boundary.
    pub fn validate(&self) -> Result<(), BranchPointError> {
        self.state.validate().map_err(BranchPointError::InvalidState)
    }

    pub(crate) fn fork_state(&self) -> State { self.state.clone() }
}

/// Error while capturing or checking a branch point.
#[derive(Debug, Clone, PartialEq)]
pub enum BranchPointError {
    /// The captured runtime state failed Layer 04 validation.
    InvalidState(StateError),
}
impl fmt::Display for BranchPointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self { Self::InvalidState(error) => write!(f, "cannot capture invalid branch state: {error}") }
    }
}
impl Error for BranchPointError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self { Self::InvalidState(error) => Some(error) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(value: &str) -> Identifier { Identifier::parse(value).unwrap() }

    #[test]
    fn capture_owns_immutable_state_and_exact_time() {
        let state = State::new();
        let time = SimTime::from_nanos(42);
        let point = BranchPoint::capture(id("main"), time, &state).unwrap();
        assert_eq!(point.branch_id().as_str(), "main");
        assert_eq!(point.at(), time);
        assert_eq!(point.state_revision(), state.revision());
        assert_eq!(point.state(), &state);
    }
}
