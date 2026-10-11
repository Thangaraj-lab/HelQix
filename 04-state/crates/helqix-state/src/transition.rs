//! Explicit, revision-checked, atomic changes to a complete state.
//!
//! A transition is a patch against exactly one expected revision. Its changes
//! are validated on a private clone and committed as one new revision. The
//! transition API therefore provides optimistic concurrency control without
//! allowing partially applied state.

use std::{
    collections::BTreeSet,
    error::Error,
    fmt,
};

use helqix_core::Identifier;

use crate::{component_state::ComponentState, state::{State, StateError}};

/// One complete component-level operation in a transition.
#[derive(Debug, Clone, PartialEq)]
pub enum StateChange {
    /// Inserts a component that is not present in the target state.
    Add(ComponentState),
    /// Removes a component that is present in the target state.
    Remove(Identifier),
    /// Replaces all runtime values for a component that already exists.
    Replace(ComponentState),
}

/// A set of component changes based on an exact state revision.
#[derive(Debug, Clone, PartialEq)]
pub struct StateTransition {
    expected_revision: u64,
    changes: Vec<StateChange>,
}

impl StateTransition {
    /// Creates a transition. Structural checks are performed by [`Self::apply`]
    /// so callers may build transitions incrementally.
    #[must_use]
    pub fn new(expected_revision: u64, changes: Vec<StateChange>) -> Self {
        Self { expected_revision, changes }
    }

    /// Creates a transition that adds one component.
    #[must_use]
    pub fn add(expected_revision: u64, component: ComponentState) -> Self {
        Self::new(expected_revision, vec![StateChange::Add(component)])
    }

    /// Creates a transition that removes one component.
    #[must_use]
    pub fn remove(expected_revision: u64, name: Identifier) -> Self {
        Self::new(expected_revision, vec![StateChange::Remove(name)])
    }

    /// Creates a transition that replaces one component's complete state.
    #[must_use]
    pub fn replace(expected_revision: u64, component: ComponentState) -> Self {
        Self::new(expected_revision, vec![StateChange::Replace(component)])
    }

    /// Adds an operation to a transition builder.
    #[must_use]
    pub fn with_change(mut self, change: StateChange) -> Self {
        self.changes.push(change);
        self
    }

    /// Returns the revision against which this patch was authored.
    #[must_use]
    pub const fn expected_revision(&self) -> u64 { self.expected_revision }

    /// Returns the operations in their authored order.
    #[must_use]
    pub fn changes(&self) -> &[StateChange] { &self.changes }

    /// Applies the patch to a clone of `current` and returns the next revision.
    ///
    /// Each component may appear at most once. Add/remove/replace failures,
    /// invalid component contents, revision conflicts, and counter overflow
    /// all reject the entire transition.
    pub fn apply(&self, current: &State) -> Result<State, TransitionError> {
        if current.revision() != self.expected_revision {
            return Err(TransitionError::RevisionConflict {
                expected: self.expected_revision,
                actual: current.revision(),
            });
        }
        if self.changes.is_empty() {
            return Err(TransitionError::EmptyTransition);
        }

        let mut targets = BTreeSet::new();
        for change in &self.changes {
            let name = match change {
                StateChange::Add(component) | StateChange::Replace(component) => component.name(),
                StateChange::Remove(name) => name,
            };
            if !targets.insert((*name).clone()) {
                return Err(TransitionError::DuplicateTarget { name: name.clone() });
            }
        }

        let mut next = current.clone();
        for change in &self.changes {
            match change {
                StateChange::Add(component) => {
                    let name = component.name().clone();
                    if next.contains_component(&name) {
                        return Err(TransitionError::ComponentExists { name });
                    }
                    next.components_mut_for_transition().insert(name, component.clone());
                }
                StateChange::Remove(name) => {
                    if next.components_mut_for_transition().remove(name).is_none() {
                        return Err(TransitionError::ComponentMissing { name: name.clone() });
                    }
                }
                StateChange::Replace(component) => {
                    let name = component.name().clone();
                    if !next.contains_component(&name) {
                        return Err(TransitionError::ComponentMissing { name });
                    }
                    next.components_mut_for_transition().insert(name, component.clone());
                }
            }
        }

        let next_revision = self.expected_revision
            .checked_add(1)
            .ok_or(TransitionError::RevisionOverflow)?;
        next.set_revision(next_revision);
        next.validate().map_err(TransitionError::InvalidState)?;
        Ok(next)
    }
}

/// Reasons a [`StateTransition`] could not be applied.
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionError {
    /// The current state has moved since this patch was authored.
    RevisionConflict { expected: u64, actual: u64 },
    /// A transaction must contain at least one operation.
    EmptyTransition,
    /// More than one operation targets the same component.
    DuplicateTarget { name: Identifier },
    /// An add operation targeted an existing component.
    ComponentExists { name: Identifier },
    /// A remove or replace operation targeted an absent component.
    ComponentMissing { name: Identifier },
    /// The revision counter cannot represent the next state.
    RevisionOverflow,
    /// The candidate state failed structural validation.
    InvalidState(StateError),
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RevisionConflict { expected, actual } => {
                write!(f, "state revision conflict: expected {expected}, got {actual}")
            }
            Self::EmptyTransition => write!(f, "state transition has no changes"),
            Self::DuplicateTarget { name } => {
                write!(f, "component appears more than once in transition: {name}")
            }
            Self::ComponentExists { name } => write!(f, "component already exists: {name}"),
            Self::ComponentMissing { name } => write!(f, "component does not exist: {name}"),
            Self::RevisionOverflow => write!(f, "state revision overflow"),
            Self::InvalidState(error) => write!(f, "transition produced invalid state: {error}"),
        }
    }
}

impl Error for TransitionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidState(error) => Some(error),
            _ => None,
        }
    }
}

impl From<StateError> for TransitionError {
    fn from(error: StateError) -> Self { Self::InvalidState(error) }
}
