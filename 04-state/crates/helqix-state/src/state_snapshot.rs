//! Immutable runtime-state snapshot for HELQIX Layer 04.
//!
//! A `StateSnapshot` represents an exact captured state of the engineering
//! reality at a particular point in the state model.
//!
//! Responsibilities:
//! - capture a complete `StateRegistry`;
//! - preserve the captured state immutably;
//! - provide deterministic read access;
//! - support exact equality and cloning.
//!
//! This module does NOT:
//! - execute state transitions;
//! - advance time;
//! - perform physics;
//! - calculate causality;
//! - perform failure analysis;
//! - create branches;
//! - compare branches;
//! - optimize;
//! - render;
//! - perform AI operations.
//!
//! Branching belongs to later layers. This type only provides the immutable
//! frozen state required for branching.

use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

use crate::component_state::ComponentState;
use crate::state_registry::{StateRegistry, StateRegistryError};

/// An immutable captured runtime state.
///
/// The snapshot owns its registry so later mutation of the original
/// `StateRegistry` cannot modify the captured state.
///
/// This is the foundational "freeze exact state" primitive used later by
/// HELQIX branching and verification.
#[derive(Debug, Clone, PartialEq)]
pub struct StateSnapshot {
    registry: StateRegistry,
}

impl StateSnapshot {
    /// Creates a snapshot from a state registry.
    ///
    /// The registry is validated before the snapshot is created.
    ///
    /// The registry is consumed, which guarantees that the snapshot owns
    /// its captured state rather than borrowing mutable runtime state.
    pub fn new(registry: StateRegistry) -> Result<Self, StateSnapshotError> {
        registry
            .validate()
            .map_err(StateSnapshotError::InvalidRegistry)?;

        Ok(Self { registry })
    }

    /// Returns the number of component states captured by this snapshot.
    #[must_use]
    pub fn len(&self) -> usize {
        self.registry.len()
    }

    /// Returns `true` when the snapshot contains no component states.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.registry.is_empty()
    }

    /// Returns `true` when the snapshot contains the specified component.
    #[must_use]
    pub fn contains(&self, name: &Identifier) -> bool {
        self.registry.contains(name)
    }

    /// Returns an immutable reference to a captured component state.
    #[must_use]
    pub fn get(&self, name: &Identifier) -> Option<&ComponentState> {
        self.registry.get(name)
    }

    /// Returns a captured component state or an error when it is absent.
    pub fn require(
        &self,
        name: &Identifier,
    ) -> Result<&ComponentState, StateSnapshotError> {
        self.registry
            .require(name)
            .map_err(StateSnapshotError::MissingComponent)
    }

    /// Iterates over captured component states in deterministic identifier
    /// order.
    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &ComponentState)> {
        self.registry.iter()
    }

    /// Iterates over captured component states in deterministic identifier
    /// order.
    pub fn states(&self) -> impl Iterator<Item = &ComponentState> {
        self.registry.states()
    }

    /// Returns an immutable reference to the complete captured registry.
    ///
    /// No mutable access is exposed intentionally: a snapshot is frozen.
    #[must_use]
    pub fn registry(&self) -> &StateRegistry {
        &self.registry
    }

    /// Validates the captured snapshot.
    ///
    /// A successfully constructed snapshot is already validated, but this
    /// method provides an explicit integrity check for callers.
    pub fn validate(&self) -> Result<(), StateSnapshotError> {
        self.registry
            .validate()
            .map_err(StateSnapshotError::InvalidRegistry)
    }
}

/// Errors produced while creating or accessing a [`StateSnapshot`].
#[derive(Debug, Clone, PartialEq)]
pub enum StateSnapshotError {
    /// The source registry was structurally invalid.
    InvalidRegistry(StateRegistryError),

    /// A requested component does not exist in the snapshot.
    MissingComponent(StateRegistryError),
}

impl fmt::Display for StateSnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRegistry(source) => {
                write!(f, "cannot create valid state snapshot: {source}")
            }

            Self::MissingComponent(source) => {
                write!(f, "component state is not present in snapshot: {source}")
            }
        }
    }
}

impl Error for StateSnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidRegistry(source) | Self::MissingComponent(source) => {
                Some(source)
            }
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

    fn registry_with_components(names: &[&str]) -> StateRegistry {
        let mut registry = StateRegistry::new();

        for name in names {
            registry
                .register(component(name))
                .expect("test component registration should succeed");
        }

        registry
    }

    #[test]
    fn snapshot_can_capture_empty_registry() {
        let registry = StateRegistry::new();

        let snapshot =
            StateSnapshot::new(registry).expect("empty registry should be valid");

        assert!(snapshot.is_empty());
        assert_eq!(snapshot.len(), 0);
    }

    #[test]
    fn snapshot_captures_component_states() {
        let registry = registry_with_components(&["motor", "battery"]);

        let snapshot =
            StateSnapshot::new(registry).expect("registry should be valid");

        assert_eq!(snapshot.len(), 2);

        assert!(snapshot.contains(&identifier("motor")));
        assert!(snapshot.contains(&identifier("battery")));
    }

    #[test]
    fn snapshot_returns_captured_component_state() {
        let registry = registry_with_components(&["motor"]);

        let snapshot =
            StateSnapshot::new(registry).expect("registry should be valid");

        let name = identifier("motor");

        let state = snapshot
            .get(&name)
            .expect("motor state should exist");

        assert_eq!(state.name(), &name);
    }

    #[test]
    fn snapshot_returns_none_for_missing_component() {
        let registry = registry_with_components(&["motor"]);

        let snapshot =
            StateSnapshot::new(registry).expect("registry should be valid");

        assert!(
            snapshot
                .get(&identifier("battery"))
                .is_none()
        );
    }

    #[test]
    fn require_returns_existing_component() {
        let registry = registry_with_components(&["motor"]);

        let snapshot =
            StateSnapshot::new(registry).expect("registry should be valid");

        let name = identifier("motor");

        let state = snapshot
            .require(&name)
            .expect("motor state should exist");

        assert_eq!(state.name(), &name);
    }

    #[test]
    fn require_returns_error_for_missing_component() {
        let registry = registry_with_components(&["motor"]);

        let snapshot =
            StateSnapshot::new(registry).expect("registry should be valid");

        let result = snapshot.require(&identifier("battery"));

        assert!(matches!(
            result,
            Err(StateSnapshotError::MissingComponent(
                StateRegistryError::MissingComponent { name }
            )) if name == "battery"
        ));
    }

    #[test]
    fn snapshot_iteration_is_deterministic() {
        let registry =
            registry_with_components(&["wing", "motor", "battery"]);

        let snapshot =
            StateSnapshot::new(registry).expect("registry should be valid");

        let names: Vec<&str> = snapshot
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();

        assert_eq!(
            names,
            vec!["battery", "motor", "wing"]
        );
    }

    #[test]
    fn snapshot_states_iteration_is_deterministic() {
        let registry =
            registry_with_components(&["wing", "motor", "battery"]);

        let snapshot =
            StateSnapshot::new(registry).expect("registry should be valid");

        let names: Vec<&str> = snapshot
            .states()
            .map(|state| state.name().as_str())
            .collect();

        assert_eq!(
            names,
            vec!["battery", "motor", "wing"]
        );
    }

    #[test]
    fn snapshot_exposes_only_immutable_registry_access() {
        let registry = registry_with_components(&["motor"]);

        let snapshot =
            StateSnapshot::new(registry).expect("registry should be valid");

        let registry_ref = snapshot.registry();

        assert_eq!(registry_ref.len(), 1);
        assert!(registry_ref.contains(&identifier("motor")));
    }

    #[test]
    fn snapshot_validates_after_creation() {
        let registry = registry_with_components(&["motor"]);

        let snapshot =
            StateSnapshot::new(registry).expect("registry should be valid");

        assert_eq!(snapshot.validate(), Ok(()));
    }

    #[test]
    fn snapshot_is_independent_of_original_registry() {
        let mut registry = registry_with_components(&["motor"]);

        let snapshot =
            StateSnapshot::new(registry.clone())
                .expect("registry should be valid");

        registry
            .register(component("battery"))
            .expect("registration should succeed");

        assert_eq!(registry.len(), 2);
        assert_eq!(snapshot.len(), 1);

        assert!(snapshot.contains(&identifier("motor")));
        assert!(!snapshot.contains(&identifier("battery")));
    }

    #[test]
    fn identical_captured_states_are_equal() {
        let first_registry =
            registry_with_components(&["battery", "motor"]);

        let second_registry =
            registry_with_components(&["motor", "battery"]);

        let first =
            StateSnapshot::new(first_registry)
                .expect("first registry should be valid");

        let second =
            StateSnapshot::new(second_registry)
                .expect("second registry should be valid");

        assert_eq!(first, second);
    }

    #[test]
    fn cloned_snapshot_is_equal_to_original() {
        let registry = registry_with_components(&["motor"]);

        let snapshot =
            StateSnapshot::new(registry).expect("registry should be valid");

        let cloned = snapshot.clone();

        assert_eq!(snapshot, cloned);
    }
}