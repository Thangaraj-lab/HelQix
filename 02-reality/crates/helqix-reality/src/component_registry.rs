//! Deterministic component registry for a HELQIX engineering reality.
//!
//! The [`ComponentRegistry`] is the structural lookup and index layer for
//! [`Component`] values. It registers components under their canonical
//! identity (`Component::name()`), rejects duplicates, refuses components that
//! fail their own structural validation, and iterates in a deterministic order.
//!
//! The registry deliberately knows nothing about relationships (that is
//! `RealityGraph`), topology, environment, engineering models, runtime state,
//! physics or rendering.
//!
//! # Core invariant
//!
//! Every entry satisfies `key == component.name()`. The registry never
//! generates, alters or renames component identity, and the normal
//! registration path can never produce a mismatching entry.
//!
//! # Encapsulation
//!
//! The underlying `BTreeMap` is never exposed. Callers get immutable lookup,
//! scoped mutable access to a single component, controlled removal and
//! deterministic iteration.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::component::Component;
use helqix_core::id::Identifier;

/// Errors produced by [`ComponentRegistry`] operations.
///
/// These are Layer-02 domain errors, separate from the Foundation error model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentRegistryError {
    /// A component with the same canonical identity is already registered.
    /// The existing component was left untouched.
    DuplicateComponent {
        /// Canonical identity of the rejected component.
        name: String,
    },
    /// A registry key disagrees with the identity of the component stored
    /// under it.
    NameMismatch {
        /// The key under which the component is stored.
        key: String,
        /// The canonical name reported by the stored component.
        component: String,
    },
    /// A component failed its own structural validation.
    ValidationFailure {
        /// Canonical identity of the invalid component.
        name: String,
        /// Stable textual description of the Foundation validation failure.
        detail: String,
    },
    /// A required component is not registered.
    MissingComponent {
        /// Identity of the component that was not found.
        name: String,
    },
}

impl fmt::Display for ComponentRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComponentRegistryError::DuplicateComponent { name } => {
                write!(f, "component '{name}' is already registered")
            }
            ComponentRegistryError::NameMismatch { key, component } => {
                write!(
                    f,
                    "registry key '{key}' does not match component name '{component}'"
                )
            }
            ComponentRegistryError::ValidationFailure { name, detail } => {
                write!(f, "component '{name}' failed structural validation: {detail}")
            }
            ComponentRegistryError::MissingComponent { name } => {
                write!(f, "component '{name}' is not registered")
            }
        }
    }
}

impl Error for ComponentRegistryError {}

/// A deterministic registry of components indexed by canonical identity.
///
/// The registry owns the registered [`Component`] values. Keys are always the
/// exact `Identifier` returned by `Component::name()`.
#[derive(Debug, Default)]
pub struct ComponentRegistry {
    components: BTreeMap<Identifier, Component>,
}

impl ComponentRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            components: BTreeMap::new(),
        }
    }

    /// Returns the number of registered components.
    #[must_use]
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Returns `true` if no components are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Returns `true` if a component with the given identity is registered.
    #[must_use]
    pub fn contains(&self, name: &Identifier) -> bool {
        self.components.contains_key(name)
    }

    /// Returns the registered component with the given identity, if any.
    #[must_use]
    pub fn get(&self, name: &Identifier) -> Option<&Component> {
        self.components.get(name)
    }

    /// Registers a component under its canonical name.
    ///
    /// The registry takes ownership of the component. It never renames the
    /// component and never overwrites an existing entry.
    ///
    /// # Errors
    ///
    /// * [`ComponentRegistryError::DuplicateComponent`] if the name is already
    ///   registered.
    /// * [`ComponentRegistryError::ValidationFailure`] if the component fails
    ///   `Component::validate()`.
    ///
    /// On error the registry is unchanged.
    pub fn register(&mut self, component: Component) -> Result<(), ComponentRegistryError> {
        let name = component.name().clone();

        if self.components.contains_key(&name) {
            return Err(ComponentRegistryError::DuplicateComponent {
                name: name.as_str().to_owned(),
            });
        }

        if let Err(err) = component.validate() {
            return Err(ComponentRegistryError::ValidationFailure {
                name: name.as_str().to_owned(),
                detail: format!("{err:?}"),
            });
        }

        self.components.insert(name, component);
        Ok(())
    }

    /// Returns mutable access to a single registered component.
    ///
    /// Access is scoped to the one component. The registry's collection stays
    /// private. Component identity must not change through this access; use
    /// [`validate`](Self::validate) after structural edits where required.
    #[must_use]
    pub fn get_mut(&mut self, name: &Identifier) -> Option<&mut Component> {
        self.components.get_mut(name)
    }

    /// Removes and returns the component with the given identity.
    ///
    /// Returns `None` if it is not registered. No other entry is affected.
    pub fn remove(&mut self, name: &Identifier) -> Option<Component> {
        self.components.remove(name)
    }

    /// Iterates over `(identity, component)` pairs in deterministic identity
    /// order.
    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &Component)> {
        self.components.iter()
    }

    /// Iterates over registered components in deterministic identity order.
    pub fn values(&self) -> impl Iterator<Item = &Component> {
        self.components.values()
    }

    /// Iterates over registered identities in deterministic order.
    pub fn keys(&self) -> impl Iterator<Item = &Identifier> {
        self.components.keys()
    }

    /// Performs structural validation of the registry.
    ///
    /// For every entry, in deterministic identity order, checks that the key
    /// equals `component.name()` and that the component passes its own
    /// `Component::validate()`. No graph, topology, environment, model, physics
    /// or runtime validation is performed here.
    ///
    /// # Errors
    ///
    /// Returns [`ComponentRegistryError::NameMismatch`] or
    /// [`ComponentRegistryError::ValidationFailure`] for the first violation.
    pub fn validate(&self) -> Result<(), ComponentRegistryError> {
        for (key, component) in &self.components {
            if key != component.name() {
                return Err(ComponentRegistryError::NameMismatch {
                    key: key.as_str().to_owned(),
                    component: component.name().as_str().to_owned(),
                });
            }
            if let Err(err) = component.validate() {
                return Err(ComponentRegistryError::ValidationFailure {
                    name: component.name().as_str().to_owned(),
                    detail: format!("{err:?}"),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> Identifier {
        Identifier::parse(value).expect("test identifier must be valid")
    }

    /// Builds a component through the real Foundation constructor.
    fn component(value: &str) -> Component {
        Component::new(id(value))
    }

    fn registry_with(names: &[&str]) -> ComponentRegistry {
        let mut registry = ComponentRegistry::new();
        for name in names {
            registry
                .register(component(name))
                .expect("test component must register");
        }
        registry
    }

    #[test]
    fn new_registry_is_empty() {
        let registry = ComponentRegistry::new();

        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
        assert_eq!(registry.validate(), Ok(()));
        assert_eq!(ComponentRegistry::default().len(), 0);
    }

    #[test]
    fn register_component() {
        let mut registry = ComponentRegistry::new();

        registry.register(component("Motor")).unwrap();

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert!(registry.contains(&id("Motor")));
        assert_eq!(registry.get(&id("Motor")).unwrap().name(), &id("Motor"));
        assert!(registry.get(&id("Unknown")).is_none());
    }

    #[test]
    fn duplicate_component_is_rejected() {
        let mut registry = ComponentRegistry::new();
        registry.register(component("Motor")).unwrap();

        let result = registry.register(component("Motor"));

        assert_eq!(
            result,
            Err(ComponentRegistryError::DuplicateComponent {
                name: "Motor".to_owned(),
            })
        );
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get(&id("Motor")).unwrap().name(), &id("Motor"));
    }

    #[test]
    fn register_applies_component_validation() {
        // The Foundation contract provided here offers no verified way to build
        // a component that fails `validate()`, so this test pins the strongest
        // supported property: registration succeeds exactly when the component
        // passes its own validation, and a failure maps to `ValidationFailure`
        // while leaving the registry unchanged.
        let candidate = component("Probe");
        let passes = candidate.validate().is_ok();

        let mut registry = ComponentRegistry::new();
        let result = registry.register(candidate);

        assert_eq!(result.is_ok(), passes);
        match result {
            Ok(()) => assert_eq!(registry.len(), 1),
            Err(err) => {
                assert!(matches!(
                    err,
                    ComponentRegistryError::ValidationFailure { ref name, .. } if name == "Probe"
                ));
                assert!(registry.is_empty());
            }
        }
    }

    #[test]
    fn get_mut_allows_controlled_component_mutation() {
        let mut registry = registry_with(&["Motor", "Battery"]);

        let before = registry.get(&id("Motor")).unwrap().property_count();
        {
            let motor = registry
                .get_mut(&id("Motor"))
                .expect("Motor must be registered");
            // Scoped access through the existing Component API; identity is
            // never touched.
            let _ = motor.properties_mut();
        }

        let motor = registry.get(&id("Motor")).unwrap();
        assert_eq!(motor.name(), &id("Motor"));
        assert_eq!(motor.property_count(), before);
        assert_eq!(registry.len(), 2);
        assert_eq!(registry.validate(), Ok(()));
        assert!(registry.get_mut(&id("Unknown")).is_none());
    }

    #[test]
    fn remove_component() {
        let mut registry = registry_with(&["Motor", "Battery"]);

        let removed = registry
            .remove(&id("Motor"))
            .expect("Motor must be removed");

        assert_eq!(removed.name(), &id("Motor"));
        assert_eq!(registry.len(), 1);
        assert!(!registry.contains(&id("Motor")));
        assert!(registry.get(&id("Motor")).is_none());
        assert!(registry.contains(&id("Battery")));
    }

    #[test]
    fn remove_last_component_leaves_registry_empty() {
        let mut registry = registry_with(&["Motor"]);

        assert!(registry.remove(&id("Motor")).is_some());

        assert!(registry.is_empty());
        assert!(!registry.contains(&id("Motor")));
        assert!(registry.register(component("Motor")).is_ok());
    }

    #[test]
    fn remove_missing_component_returns_none() {
        let mut registry = registry_with(&["Motor"]);

        assert!(registry.remove(&id("Unknown")).is_none());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn deterministic_iteration() {
        let registry = registry_with(&["Zeta", "Alpha", "Motor", "Battery"]);

        let from_iter: Vec<&str> = registry.iter().map(|(k, _)| k.as_str()).collect();

        assert_eq!(from_iter, vec!["Alpha", "Battery", "Motor", "Zeta"]);
    }

    #[test]
    fn iteration_order_is_independent_of_insertion_order() {
        let forward = registry_with(&["Alpha", "Battery", "Motor", "Zeta"]);
        let reverse = registry_with(&["Zeta", "Motor", "Battery", "Alpha"]);

        let a: Vec<&str> = forward.keys().map(Identifier::as_str).collect();
        let b: Vec<&str> = reverse.keys().map(Identifier::as_str).collect();

        assert_eq!(a, b);
    }

    #[test]
    fn validation_accepts_valid_registry() {
        let registry = registry_with(&["Motor", "Battery", "Payload"]);

        assert_eq!(registry.validate(), Ok(()));
    }

    #[test]
    fn registry_key_matches_component_name() {
        let registry = registry_with(&["Zeta", "Alpha", "Motor"]);

        for (key, component) in registry.iter() {
            assert_eq!(key, component.name());
        }
        assert_eq!(registry.iter().count(), 3);
    }

    #[test]
    fn values_iteration_is_deterministic() {
        let registry = registry_with(&["Zeta", "Alpha", "Motor", "Battery"]);

        let names: Vec<&str> = registry.values().map(|c| c.name().as_str()).collect();

        assert_eq!(names, vec!["Alpha", "Battery", "Motor", "Zeta"]);
    }

    #[test]
    fn keys_iteration_is_deterministic() {
        let registry = registry_with(&["Zeta", "Alpha", "Motor", "Battery"]);

        let keys: Vec<&str> = registry.keys().map(Identifier::as_str).collect();

        assert_eq!(keys, vec!["Alpha", "Battery", "Motor", "Zeta"]);
    }

    #[test]
    fn registry_error_messages_are_stable() {
        let duplicate = ComponentRegistryError::DuplicateComponent {
            name: "Motor".to_owned(),
        };
        assert_eq!(duplicate.to_string(), "component 'Motor' is already registered");

        let mismatch = ComponentRegistryError::NameMismatch {
            key: "A".to_owned(),
            component: "B".to_owned(),
        };
        assert_eq!(
            mismatch.to_string(),
            "registry key 'A' does not match component name 'B'"
        );

        let missing = ComponentRegistryError::MissingComponent {
            name: "Motor".to_owned(),
        };
        assert_eq!(missing.to_string(), "component 'Motor' is not registered");

        let _: &dyn Error = &duplicate;
    }
}