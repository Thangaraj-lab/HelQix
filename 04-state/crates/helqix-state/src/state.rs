use std::collections::BTreeMap;

use helqix_core::Identifier;

use crate::component_state::ComponentState;
use crate::validation::{StateValidationError, StateValidator};

/// The current deterministic state of a HELQIX engineering reality.
///
/// `State` represents what is true at a particular point in the system.
/// It does not execute actions, advance time, perform physics calculations,
/// or determine how the state changes.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    components: BTreeMap<Identifier, ComponentState>,
    revision: u64,
}

impl State {
    /// Creates an empty state with revision `0`.
    pub fn new() -> Self {
        Self {
            components: BTreeMap::new(),
            revision: 0,
        }
    }

    /// Returns the current state revision.
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns the number of component states.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Returns `true` when no component states are present.
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Returns whether a component state with the given identifier exists.
    pub fn contains_component(&self, name: &Identifier) -> bool {
        self.components.contains_key(name)
    }

    /// Returns a component state by identifier.
    pub fn component(&self, name: &Identifier) -> Option<&ComponentState> {
        self.components.get(name)
    }

    /// Returns all component states in deterministic identifier order.
    pub fn components(&self) -> impl Iterator<Item = &ComponentState> {
        self.components.values()
    }

    /// Returns all component states with their identifiers in deterministic order.
    pub fn named_components(&self) -> impl Iterator<Item = (&Identifier, &ComponentState)> {
        self.components.iter()
    }

    /// Inserts a component state.
    ///
    /// Existing component state with the same identifier is rejected rather
    /// than silently replaced.
    pub fn add_component(
        &mut self,
        component: ComponentState,
    ) -> Result<(), StateError> {
        let name = component.name().clone();

        if self.components.contains_key(&name) {
            return Err(StateError::DuplicateComponent { name });
        }
        component.validate().map_err(|source| {
            StateError::Validation(StateValidationError::InvalidComponent {
                name: name.clone(),
                source,
            })
        })?;

        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(StateError::RevisionOverflow)?;

        self.components.insert(name, component);
        self.revision = next_revision;

        Ok(())
    }

    /// Replaces an existing component state as one revisioned operation.
    /// The replacement must retain the component's canonical identity.
    pub fn replace_component(
        &mut self,
        component: ComponentState,
    ) -> Result<Option<ComponentState>, StateError> {
        let name = component.name().clone();
        let Some(_) = self.components.get(&name) else {
            return Ok(None);
        };
        component.validate().map_err(|source| {
            StateError::Validation(StateValidationError::InvalidComponent { name: name.clone(), source })
        })?;
        let next_revision = self.revision.checked_add(1).ok_or(StateError::RevisionOverflow)?;
        let previous = self.components.insert(name, component);
        self.revision = next_revision;
        Ok(previous)
    }

    /// Applies an edit to a cloned component and commits it only after
    /// validation succeeds. Failed edits leave the state and revision intact.
    pub fn update_component<F>(
        &mut self,
        name: &Identifier,
        update: F,
    ) -> Result<(), StateError>
    where
        F: FnOnce(&mut ComponentState),
    {
        let mut component = self.components.get(name).cloned().ok_or_else(|| {
            StateError::MissingComponent { name: name.clone() }
        })?;
        update(&mut component);
        component.validate().map_err(|source| {
            StateError::Validation(StateValidationError::InvalidComponent { name: name.clone(), source })
        })?;
        let next_revision = self.revision.checked_add(1).ok_or(StateError::RevisionOverflow)?;
        self.components.insert(name.clone(), component);
        self.revision = next_revision;
        Ok(())
    }

    /// Removes a component state by identifier.
    pub fn remove_component(
        &mut self,
        name: &Identifier,
    ) -> Result<Option<ComponentState>, StateError> {
        if !self.components.contains_key(name) {
            return Ok(None);
        }

        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(StateError::RevisionOverflow)?;
        let removed = self.components.remove(name);
        self.revision = next_revision;
        Ok(removed)
    }

    /// Validates the current state using the Layer 04 semantic validator.
    pub fn validate(&self) -> Result<(), StateError> {
        StateValidator::validate(self).map_err(StateError::Validation)
    }

    /// Returns the number of state revisions represented by this object.
    ///
    /// This is a state revision counter only. It is not a simulation clock
    /// and does not represent physical time.
    pub(crate) fn set_revision(&mut self, revision: u64) {
        self.revision = revision;
    }

    pub(crate) fn components_mut_for_transition(
        &mut self,
    ) -> &mut BTreeMap<Identifier, ComponentState> {
        &mut self.components
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors produced while modifying or validating [`State`].
#[derive(Debug, Clone, PartialEq)]
pub enum StateError {
    DuplicateComponent {
        name: Identifier,
    },
    MissingComponent { name: Identifier },
    RevisionOverflow,
    Validation(StateValidationError),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateComponent { name } => {
                write!(formatter, "component state already exists: {name}")
            }
            Self::MissingComponent { name } => write!(formatter, "component state does not exist: {name}"),
            Self::RevisionOverflow => {
                write!(formatter, "state revision overflow")
            }
            Self::Validation(error) => {
                write!(formatter, "state validation failed: {error}")
            }
        }
    }
}

impl std::error::Error for StateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Validation(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_empty() {
        let state = State::new();

        assert_eq!(state.component_count(), 0);
        assert!(state.is_empty());
        assert_eq!(state.revision(), 0);
    }

    #[test]
    fn default_state_is_empty() {
        let state = State::default();

        assert!(state.is_empty());
        assert_eq!(state.revision(), 0);
    }

    #[test]
    fn component_count_matches_contents() {
        let state = State::new();

        assert_eq!(state.component_count(), state.components().count());
    }

    #[test]
    fn revision_is_not_physical_time() {
        let state = State::new();

        assert_eq!(state.revision(), 0);
    }
}
