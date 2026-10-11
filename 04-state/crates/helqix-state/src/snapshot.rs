//! Immutable, revision-aware copy of a complete runtime state.
//!
//! [`Snapshot`] is the authoritative freeze point for the revisioned [`State`]
//! API. It preserves the revision and complete component values while
//! exposing only immutable access. [`StateSnapshot`] remains available as a
//! registry-only snapshot for callers that do not need revision metadata.

use std::{error::Error, fmt};

use helqix_core::Identifier;

use crate::{
    component_state::ComponentState,
    state::{State, StateError},
    state_diff::{RevisionedStateDiff, RevisionedStateDiffError, StateDiff},
    state_registry::StateRegistry,
    state_snapshot::{StateSnapshot, StateSnapshotError},
};

/// An immutable captured runtime state with its source revision.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    state: State,
}

impl Snapshot {
    /// Captures a validated copy of `state`, preserving its revision.
    pub fn capture(state: &State) -> Result<Self, SnapshotError> {
        state.validate().map_err(SnapshotError::InvalidState)?;
        Ok(Self { state: state.clone() })
    }

    /// Returns the revision represented by this snapshot.
    #[must_use]
    pub const fn revision(&self) -> u64 { self.state.revision() }

    /// Returns an immutable reference to the complete captured state.
    #[must_use]
    pub const fn state(&self) -> &State { &self.state }

    /// Consumes the snapshot and returns its owned state.
    pub fn into_state(self) -> State { self.state }

    /// Returns the number of component states captured.
    #[must_use]
    pub fn component_count(&self) -> usize { self.state.component_count() }

    /// Returns whether the captured state contains no components.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.state.is_empty() }

    /// Looks up a component without exposing mutable snapshot contents.
    #[must_use]
    pub fn component(&self, name: &Identifier) -> Option<&ComponentState> {
        self.state.component(name)
    }

    /// Iterates over components in deterministic identifier order.
    pub fn components(&self) -> impl Iterator<Item = (&Identifier, &ComponentState)> {
        self.state.named_components()
    }

    /// Revalidates the captured state as an integrity check.
    pub fn validate(&self) -> Result<(), SnapshotError> {
        self.state.validate().map_err(SnapshotError::InvalidState)
    }

    /// Produces a component diff that records both snapshot revisions.
    pub fn diff_to(
        &self,
        after: &Snapshot,
    ) -> Result<RevisionedStateDiff, RevisionedStateDiffError> {
        StateDiff::between_revisioned(self, after)
    }

    /// Builds the registry-based immutable snapshot used by [`StateDiff`].
    ///
    /// This projection contains the same component values but intentionally
    /// omits revision metadata; retain this snapshot when that metadata matters.
    pub fn component_snapshot(&self) -> Result<StateSnapshot, StateSnapshotError> {
        let mut registry = StateRegistry::new();
        for component in self.state.components() {
            registry
                .register(component.clone())
                .map_err(StateSnapshotError::InvalidRegistry)?;
        }
        StateSnapshot::new(registry)
    }
}

/// Failure to capture or validate a complete state snapshot.
#[derive(Debug, Clone, PartialEq)]
pub enum SnapshotError {
    /// The source state failed structural validation.
    InvalidState(StateError),
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState(error) => write!(f, "cannot snapshot invalid state: {error}"),
        }
    }
}

impl Error for SnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidState(error) => Some(error),
        }
    }
}
