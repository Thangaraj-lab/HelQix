//! Structural validation for Layer 04 runtime state values.
//!
//! Validation here checks only state-layer invariants: canonical identifiers,
//! component membership, property storage, physical quantities, and spatial
//! values. Requirements declared by Reality or Engineering models need those
//! definitions as inputs and are intentionally handled by their validation
//! boundary.

use std::{error::Error, fmt};

use helqix_core::Identifier;

use crate::{
    component_state::{ComponentState, ComponentStateError},
    state::State,
    state_registry::StateRegistry,
    state_snapshot::StateSnapshot,
};

/// A structural state validation failure.
#[derive(Debug, Clone, PartialEq)]
pub enum StateValidationError {
    /// The map key does not match the component's own canonical identity.
    ComponentNameMismatch { key: Identifier, component: Identifier },
    /// A component's contained values failed their own validation.
    InvalidComponent { name: Identifier, source: ComponentStateError },
}

impl fmt::Display for StateValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ComponentNameMismatch { key, component } => write!(
                f,
                "component state key `{key}` does not match `{component}`"
            ),
            Self::InvalidComponent { name, source } => {
                write!(f, "invalid component state `{name}`: {source}")
            }
        }
    }
}

impl Error for StateValidationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidComponent { source, .. } => Some(source),
            Self::ComponentNameMismatch { .. } => None,
        }
    }
}

/// Entry points for validating each complete state container.
pub struct StateValidator;

impl StateValidator {
    /// Validates all components in a revisioned [`State`].
    pub fn validate(state: &State) -> Result<(), StateValidationError> {
        for (key, component) in state.named_components() {
            Self::validate_named_component(key, component)?;
        }
        Ok(())
    }

    /// Validates every component in a mutable registry without changing it.
    pub fn validate_registry(
        registry: &StateRegistry,
    ) -> Result<(), StateValidationError> {
        for (key, component) in registry.iter() {
            Self::validate_named_component(key, component)?;
        }
        Ok(())
    }

    /// Validates all components in an immutable registry snapshot.
    pub fn validate_snapshot(
        snapshot: &StateSnapshot,
    ) -> Result<(), StateValidationError> {
        for (key, component) in snapshot.iter() {
            Self::validate_named_component(key, component)?;
        }
        Ok(())
    }

    /// Validates the internal values of one component state.
    pub fn validate_component(
        component: &ComponentState,
    ) -> Result<(), StateValidationError> {
        component
            .validate()
            .map_err(|source| StateValidationError::InvalidComponent {
                name: component.name().clone(),
                source,
            })
    }

    fn validate_named_component(
        key: &Identifier,
        component: &ComponentState,
    ) -> Result<(), StateValidationError> {
        if key != component.name() {
            return Err(StateValidationError::ComponentNameMismatch {
                key: key.clone(),
                component: component.name().clone(),
            });
        }
        Self::validate_component(component)
    }
}
