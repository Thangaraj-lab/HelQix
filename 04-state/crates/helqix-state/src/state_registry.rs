//! Deterministic registry of runtime component states for HELQIX Layer 04.
//!
//! `StateRegistry` maps each Reality component identifier to its current
//! `ComponentState`.
//!
//! Responsibilities:
//! - deterministic component-state storage;
//! - duplicate-safe registration;
//! - lookup and mutable lookup;
//! - explicit removal;
//! - structural validation.
//!
//! This module does NOT:
//! - execute anything;
//! - advance time;
//! - perform physics;
//! - calculate state changes;
//! - create snapshots;
//! - perform branching;
//! - perform failure analysis;
//! - perform optimization;
//! - perform rendering;
//! - perform AI operations.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

use crate::component_state::{ComponentState, ComponentStateError};

/// Deterministic registry of runtime states belonging to Reality components.
///
/// The registry key is always the component state's canonical identifier
/// returned by [`ComponentState::name`].
///
/// `BTreeMap` is used deliberately so iteration order is deterministic.
#[derive(Debug, Clone, PartialEq)]
pub struct StateRegistry {
    components: BTreeMap<Identifier, ComponentState>,
}

impl Default for StateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl StateRegistry {
    /// Creates an empty state registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            components: BTreeMap::new(),
        }
    }

    /// Returns the number of registered component states.
    #[must_use]
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Returns `true` when the registry contains no component states.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Returns `true` when a component state with the given identifier exists.
    #[must_use]
    pub fn contains(&self, name: &Identifier) -> bool {
        self.components.contains_key(name)
    }

    /// Registers a component state.
    ///
    /// Registration is duplicate-safe: an existing component state is never
    /// silently replaced.
    ///
    /// The component state is validated before it becomes part of the registry.
    pub fn register(
        &mut self,
        state: ComponentState,
    ) -> Result<(), StateRegistryError> {
        let name = state.name().clone();

        if self.components.contains_key(&name) {
            return Err(StateRegistryError::DuplicateComponent {
                name: name.as_str().to_owned(),
            });
        }

        state
            .validate()
            .map_err(|source| StateRegistryError::InvalidComponentState {
                name: name.as_str().to_owned(),
                source,
            })?;

        self.components.insert(name, state);

        Ok(())
    }

    /// Returns an immutable reference to a component state.
    #[must_use]
    pub fn get(&self, name: &Identifier) -> Option<&ComponentState> {
        self.components.get(name)
    }

    /// Returns a mutable reference to a component state.
    ///
    /// The component state's own identifier cannot be changed through
    /// `ComponentState`, so the registry key remains stable.
    pub fn get_mut(&mut self, name: &Identifier) -> Option<&mut ComponentState> {
        self.components.get_mut(name)
    }

    /// Returns a component state or a typed error when it does not exist.
    pub fn require(
        &self,
        name: &Identifier,
    ) -> Result<&ComponentState, StateRegistryError> {
        self.components
            .get(name)
            .ok_or_else(|| StateRegistryError::MissingComponent {
                name: name.as_str().to_owned(),
            })
    }

    /// Removes and returns a component state when present.
    pub fn remove(&mut self, name: &Identifier) -> Option<ComponentState> {
        self.components.remove(name)
    }

    /// Iterates over `(identifier, component_state)` pairs in deterministic
    /// identifier order.
    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &ComponentState)> {
        self.components.iter()
    }

    /// Iterates over component states in deterministic identifier order.
    pub fn states(&self) -> impl Iterator<Item = &ComponentState> {
        self.components.values()
    }

    /// Validates the complete registry.
    ///
    /// Validation order is deterministic:
    ///
    /// 1. registry key matches component-state identity;
    /// 2. component state itself is valid.
    pub fn validate(&self) -> Result<(), StateRegistryError> {
        for (key, state) in &self.components {
            if key != state.name() {
                return Err(StateRegistryError::NameMismatch {
                    key: key.as_str().to_owned(),
                    component: state.name().as_str().to_owned(),
                });
            }

            state
                .validate()
                .map_err(|source| StateRegistryError::InvalidComponentState {
                    name: state.name().as_str().to_owned(),
                    source,
                })?;
        }

        Ok(())
    }
}

/// Errors produced by [`StateRegistry`].
#[derive(Debug, Clone, PartialEq)]
pub enum StateRegistryError {
    /// A component state with the same identifier already exists.
    DuplicateComponent {
        name: String,
    },

    /// A requested component state does not exist.
    MissingComponent {
        name: String,
    },

    /// The registry key does not match the component state's canonical name.
    NameMismatch {
        key: String,
        component: String,
    },

    /// A registered component state failed its own validation.
    InvalidComponentState {
        name: String,
        source: ComponentStateError,
    },
}

impl fmt::Display for StateRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateComponent { name } => {
                write!(f, "duplicate component state: {name}")
            }

            Self::MissingComponent { name } => {
                write!(f, "missing component state: {name}")
            }

            Self::NameMismatch { key, component } => {
                write!(
                    f,
                    "state registry key '{key}' does not match component state '{component}'"
                )
            }

            Self::InvalidComponentState { name, source } => {
                write!(
                    f,
                    "invalid component state '{name}': {source}"
                )
            }
        }
    }
}

impl Error for StateRegistryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidComponentState { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identifier(value: &str) -> Identifier {
        value.parse().expect("test identifier must be valid")
    }

    fn component(name: &str) -> ComponentState {
        ComponentState::new(identifier(name))
    }

    #[test]
    fn new_registry_is_empty() {
        let registry = StateRegistry::new();

        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn default_registry_is_empty() {
        let registry = StateRegistry::default();

        assert!(registry.is_empty());
    }

    #[test]
    fn register_adds_component_state() {
        let mut registry = StateRegistry::new();

        registry
            .register(component("motor"))
            .expect("registration should succeed");

        let name = identifier("motor");

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert!(registry.contains(&name));
    }

    #[test]
    fn register_preserves_component_identity() {
        let mut registry = StateRegistry::new();

        registry
            .register(component("motor"))
            .expect("registration should succeed");

        let name = identifier("motor");

        let state = registry
            .get(&name)
            .expect("registered component state should exist");

        assert_eq!(state.name(), &name);
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let mut registry = StateRegistry::new();

        registry
            .register(component("motor"))
            .expect("first registration should succeed");

        let result = registry.register(component("motor"));

        assert_eq!(
            result,
            Err(StateRegistryError::DuplicateComponent {
                name: "motor".to_owned(),
            })
        );

        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn duplicate_registration_does_not_replace_existing_state() {
        let mut registry = StateRegistry::new();

        registry
            .register(component("motor"))
            .expect("first registration should succeed");

        let result = registry.register(component("motor"));

        assert!(result.is_err());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn get_returns_registered_state() {
        let mut registry = StateRegistry::new();

        registry
            .register(component("battery"))
            .expect("registration should succeed");

        let name = identifier("battery");

        let state = registry
            .get(&name)
            .expect("component state should exist");

        assert_eq!(state.name(), &name);
    }

    #[test]
    fn get_returns_none_for_missing_state() {
        let registry = StateRegistry::new();

        let name = identifier("battery");

        assert!(registry.get(&name).is_none());
    }

    #[test]
    fn require_returns_registered_state() {
        let mut registry = StateRegistry::new();

        registry
            .register(component("battery"))
            .expect("registration should succeed");

        let name = identifier("battery");

        let state = registry
            .require(&name)
            .expect("component state should exist");

        assert_eq!(state.name(), &name);
    }

    #[test]
    fn require_returns_missing_component_error() {
        let registry = StateRegistry::new();

        let name = identifier("battery");

        let result = registry.require(&name);

        assert_eq!(
            result,
            Err(StateRegistryError::MissingComponent {
                name: "battery".to_owned(),
            })
        );
    }

    #[test]
    fn get_mut_returns_registered_state() {
        let mut registry = StateRegistry::new();

        registry
            .register(component("motor"))
            .expect("registration should succeed");

        let name = identifier("motor");

        let state = registry
            .get_mut(&name)
            .expect("component state should exist");

        assert_eq!(state.name(), &name);
    }

    #[test]
    fn remove_returns_registered_state() {
        let mut registry = StateRegistry::new();

        registry
            .register(component("motor"))
            .expect("registration should succeed");

        let name = identifier("motor");

        let removed = registry.remove(&name);

        assert!(removed.is_some());
        assert!(!registry.contains(&name));
        assert!(registry.is_empty());
    }

    #[test]
    fn remove_missing_state_returns_none() {
        let mut registry = StateRegistry::new();

        let name = identifier("motor");

        assert!(registry.remove(&name).is_none());
    }

    #[test]
    fn iteration_is_deterministic() {
        let mut registry = StateRegistry::new();

        registry
            .register(component("wing"))
            .expect("registration should succeed");

        registry
            .register(component("motor"))
            .expect("registration should succeed");

        registry
            .register(component("battery"))
            .expect("registration should succeed");

        let names: Vec<&str> = registry
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();

        assert_eq!(
            names,
            vec!["battery", "motor", "wing"]
        );
    }

    #[test]
    fn states_iteration_is_deterministic() {
        let mut registry = StateRegistry::new();

        registry
            .register(component("wing"))
            .expect("registration should succeed");

        registry
            .register(component("motor"))
            .expect("registration should succeed");

        let names: Vec<&str> = registry
            .states()
            .map(|state| state.name().as_str())
            .collect();

        assert_eq!(
            names,
            vec!["motor", "wing"]
        );
    }

    #[test]
    fn empty_registry_validates() {
        let registry = StateRegistry::new();

        assert_eq!(registry.validate(), Ok(()));
    }

    #[test]
    fn populated_registry_validates() {
        let mut registry = StateRegistry::new();

        registry
            .register(component("motor"))
            .expect("registration should succeed");

        registry
            .register(component("battery"))
            .expect("registration should succeed");

        assert_eq!(registry.validate(), Ok(()));
    }

    #[test]
    fn registry_clone_is_independent() {
        let mut registry = StateRegistry::new();

        registry
            .register(component("motor"))
            .expect("registration should succeed");

        let mut cloned = registry.clone();

        cloned
            .register(component("battery"))
            .expect("registration should succeed");

        assert_eq!(registry.len(), 1);
        assert_eq!(cloned.len(), 2);
    }

    #[test]
    fn registry_equality_is_deterministic() {
        let mut first = StateRegistry::new();
        let mut second = StateRegistry::new();

        first
            .register(component("motor"))
            .expect("registration should succeed");

        first
            .register(component("battery"))
            .expect("registration should succeed");

        second
            .register(component("battery"))
            .expect("registration should succeed");

        second
            .register(component("motor"))
            .expect("registration should succeed");

        assert_eq!(first, second);
    }
}s