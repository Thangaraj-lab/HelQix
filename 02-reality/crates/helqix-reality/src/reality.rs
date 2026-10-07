//! The HELQIX `Reality` aggregate root.
//!
//! A [`Reality`] is the structural engineering truth for one engineering
//! system: *what exists*, not *what is happening*. It owns the membership
//! boundary for [`Component`]s and nothing else.
//!
//! Runtime state, physics, rendering, telemetry, execution and AI belong to
//! later HELQIX layers and must not be stored in this type.
//!
//! # Determinism
//!
//! Components are stored in a [`BTreeMap`] keyed by the component's canonical
//! [`Identifier`] returned by [`Component::name`]. Therefore component lookup,
//! insertion, removal, validation and iteration are deterministic and
//! independent of insertion order.
//!
//! # Encapsulation
//!
//! The internal component collection is never exposed directly. Membership
//! changes occur through [`Reality::add_component`] and
//! [`Reality::remove_component`]. Mutable access is limited to one individual
//! component through [`Reality::component_mut`].

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::component::Component;
use helqix_core::id::Identifier;

/// Errors produced by structural operations on a [`Reality`].
///
/// These errors belong to the Reality layer rather than the Foundation layer.
/// For example, a duplicate component is a Reality aggregate-membership
/// violation, not a duplicate member inside a Foundation [`Component`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealityError {
    /// A component with the same canonical identifier already belongs to the
    /// Reality.
    ///
    /// The existing component is never replaced.
    DuplicateComponent {
        /// Canonical identifier of the rejected component.
        name: String,
    },

    /// A member component failed its own structural validation.
    ComponentValidation {
        /// Canonical identifier of the invalid component.
        name: String,

        /// Human-readable representation of the Foundation validation error.
        detail: String,
    },

    /// The internal Reality structure is inconsistent.
    ///
    /// This is intended for structural invariants that should normally be
    /// impossible to violate through the safe public API.
    InvalidStructure {
        /// Description of the structural inconsistency.
        detail: String,
    },
}

impl fmt::Display for RealityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateComponent { name } => {
                write!(
                    f,
                    "component '{name}' already exists in this reality"
                )
            }

            Self::ComponentValidation { name, detail } => {
                write!(
                    f,
                    "component '{name}' failed structural validation: {detail}"
                )
            }

            Self::InvalidStructure { detail } => {
                write!(f, "invalid reality structure: {detail}")
            }
        }
    }
}

impl Error for RealityError {}

/// The aggregate root of one structurally defined HELQIX engineering system.
///
/// `Reality` defines which engineering components belong to the system.
///
/// It deliberately does **not** contain:
///
/// - runtime state
/// - simulation time
/// - physics state
/// - execution state
/// - telemetry
/// - rendering state
/// - AI state
/// - optimization state
///
/// Those concerns belong to later HELQIX layers.
#[derive(Debug)]
pub struct Reality {
    /// Stable identity of this Reality.
    ///
    /// `Identifier` guarantees that the identity has already passed
    /// Foundation-level identifier validation.
    name: Identifier,

    /// Component membership indexed by each component's canonical name.
    ///
    /// The key is always the value returned by `Component::name()`.
    components: BTreeMap<Identifier, Component>,
}

impl Reality {
    /// Creates an empty Reality with the supplied identity.
    ///
    /// # Examples
    ///
    /// ```
    /// # use helqix_core::id::Identifier;
    /// # use helqix_reality::reality::Reality;
    /// #
    /// let name = Identifier::parse("drone_system").unwrap();
    /// let reality = Reality::new(name);
    ///
    /// assert!(reality.is_empty());
    /// ```
    #[must_use]
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            components: BTreeMap::new(),
        }
    }

    /// Returns the stable identity of this Reality.
    #[must_use]
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the number of components belonging to this Reality.
    #[must_use]
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Returns `true` when this Reality contains no components.
    ///
    /// An empty Reality is structurally valid.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Adds a component to this Reality.
    ///
    /// The component's canonical [`Component::name`] is used as its identity
    /// and as the key in the internal [`BTreeMap`].
    ///
    /// Duplicate component identities are rejected. The existing component is
    /// never replaced.
    ///
    /// # Errors
    ///
    /// Returns [`RealityError::DuplicateComponent`] when a component with the
    /// same canonical identifier already exists.
    pub fn add_component(
        &mut self,
        component: Component,
    ) -> Result<(), RealityError> {
        let name = component.name();

        if self.components.contains_key(name) {
            return Err(RealityError::DuplicateComponent {
                name: name.as_str().to_owned(),
            });
        }

        let key = name.clone();

        self.components.insert(key, component);

        Ok(())
    }

    /// Returns an immutable reference to a component by its canonical
    /// identifier.
    ///
    /// Returns `None` when the component does not belong to this Reality.
    #[must_use]
    pub fn component(
        &self,
        name: &Identifier,
    ) -> Option<&Component> {
        self.components.get(name)
    }

    /// Returns mutable access to one component by its canonical identifier.
    ///
    /// The component collection itself remains private, so callers cannot
    /// directly insert, remove or re-key components through this method.
    ///
    /// Returns `None` when the component does not belong to this Reality.
    #[must_use]
    pub fn component_mut(
        &mut self,
        name: &Identifier,
    ) -> Option<&mut Component> {
        self.components.get_mut(name)
    }

    /// Returns `true` when a component with the supplied identifier belongs to
    /// this Reality.
    #[must_use]
    pub fn contains_component(
        &self,
        name: &Identifier,
    ) -> bool {
        self.components.contains_key(name)
    }

    /// Removes and returns a component by its canonical identifier.
    ///
    /// Returns `None` when no component with that identifier exists.
    pub fn remove_component(
        &mut self,
        name: &Identifier,
    ) -> Option<Component> {
        self.components.remove(name)
    }

    /// Iterates over all components in deterministic identifier order.
    ///
    /// The order is determined by [`Identifier`] ordering and is independent
    /// of insertion order.
    pub fn components(
        &self,
    ) -> impl Iterator<Item = &Component> {
        self.components.values()
    }

    /// Validates the structural integrity of this Reality.
    ///
    /// Validation performs only structural checks:
    ///
    /// 1. Every BTreeMap key must equal the canonical name of its component.
    /// 2. Every component must pass its own Foundation-level validation.
    ///
    /// This method does **not**:
    ///
    /// - execute behaviours
    /// - evaluate runtime state
    /// - execute physics
    /// - validate simulation results
    /// - evaluate telemetry
    /// - perform AI reasoning
    /// - optimize the engineering system
    ///
    /// Those responsibilities belong to later HELQIX layers.
    ///
    /// # Errors
    ///
    /// Returns [`RealityError::InvalidStructure`] when the internal key does
    /// not match the component identity.
    ///
    /// Returns [`RealityError::ComponentValidation`] when a member component
    /// fails Foundation structural validation.
    pub fn validate(&self) -> Result<(), RealityError> {
        for (key, component) in &self.components {
            // The public API guarantees this invariant during normal
            // construction. Keep the check here because Reality is an
            // aggregate boundary and validation must explicitly verify its
            // structural invariants.
            if key != component.name() {
                return Err(RealityError::InvalidStructure {
                    detail: format!(
                        "component stored under key '{}' reports name '{}'",
                        key.as_str(),
                        component.name().as_str()
                    ),
                });
            }

            if let Err(error) = component.validate() {
                return Err(RealityError::ComponentValidation {
                    name: key.as_str().to_owned(),
                    detail: format!("{error:?}"),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identifier(value: &str) -> Identifier {
        Identifier::parse(value)
            .expect("test identifier must be valid")
    }

    fn component(value: &str) -> Component {
        Component::new(identifier(value))
    }

    #[test]
    fn new_reality_is_empty() {
        let reality = Reality::new(identifier("drone_system"));

        assert_eq!(
            reality.name(),
            &identifier("drone_system")
        );
        assert_eq!(reality.component_count(), 0);
        assert!(reality.is_empty());
        assert_eq!(reality.components().count(), 0);
        assert!(reality.validate().is_ok());
    }

    #[test]
    fn add_component_registers_component() {
        let mut reality =
            Reality::new(identifier("drone_system"));

        reality
            .add_component(component("Motor"))
            .unwrap();

        assert_eq!(reality.component_count(), 1);
        assert!(!reality.is_empty());

        assert!(reality.contains_component(
            &identifier("Motor")
        ));

        let stored = reality
            .component(&identifier("Motor"))
            .expect("Motor must be a Reality member");

        assert_eq!(
            stored.name(),
            &identifier("Motor")
        );
    }

    #[test]
    fn duplicate_component_is_rejected() {
        let mut reality =
            Reality::new(identifier("drone_system"));

        reality
            .add_component(component("Motor"))
            .unwrap();

        let result =
            reality.add_component(component("Motor"));

        assert_eq!(
            result,
            Err(RealityError::DuplicateComponent {
                name: "Motor".to_owned(),
            })
        );

        // The failed insertion must not increase membership.
        assert_eq!(reality.component_count(), 1);

        // The original membership remains available.
        let stored = reality
            .component(&identifier("Motor"))
            .expect("original Motor must remain");

        assert_eq!(
            stored.name(),
            &identifier("Motor")
        );
    }

    #[test]
    fn duplicate_identity_is_case_sensitive() {
        let mut reality =
            Reality::new(identifier("drone_system"));

        reality
            .add_component(component("Motor"))
            .unwrap();

        reality
            .add_component(component("motor"))
            .unwrap();

        assert_eq!(reality.component_count(), 2);
        assert!(reality.contains_component(
            &identifier("Motor")
        ));
        assert!(reality.contains_component(
            &identifier("motor")
        ));
    }

    #[test]
    fn component_lookup_is_identity_based() {
        let mut reality =
            Reality::new(identifier("drone_system"));

        reality
            .add_component(component("Motor"))
            .unwrap();

        reality
            .add_component(component("Battery"))
            .unwrap();

        reality
            .add_component(component("Payload"))
            .unwrap();

        assert_eq!(
            reality
                .component(&identifier("Motor"))
                .unwrap()
                .name(),
            &identifier("Motor")
        );

        assert_eq!(
            reality
                .component(&identifier("Battery"))
                .unwrap()
                .name(),
            &identifier("Battery")
        );

        assert_eq!(
            reality
                .component(&identifier("Payload"))
                .unwrap()
                .name(),
            &identifier("Payload")
        );
    }

    #[test]
    fn missing_component_returns_none() {
        let mut reality =
            Reality::new(identifier("drone_system"));

        reality
            .add_component(component("Motor"))
            .unwrap();

        let unknown = identifier("Unknown");

        assert!(!reality.contains_component(&unknown));
        assert!(reality.component(&unknown).is_none());
        assert!(reality.component_mut(&unknown).is_none());
        assert!(reality.remove_component(&unknown).is_none());

        assert_eq!(reality.component_count(), 1);
    }

    #[test]
    fn component_removal_is_deterministic() {
        let mut reality =
            Reality::new(identifier("drone_system"));

        reality
            .add_component(component("Motor"))
            .unwrap();

        let removed = reality
            .remove_component(&identifier("Motor"))
            .expect("Motor must exist");

        assert_eq!(
            removed.name(),
            &identifier("Motor")
        );

        assert!(reality.is_empty());

        assert!(!reality.contains_component(
            &identifier("Motor")
        ));

        assert!(
            reality
                .component(&identifier("Motor"))
                .is_none()
        );

        assert!(
            reality
                .remove_component(&identifier("Motor"))
                .is_none()
        );
    }

    #[test]
    fn removed_component_can_be_added_again() {
        let mut reality =
            Reality::new(identifier("drone_system"));

        reality
            .add_component(component("Motor"))
            .unwrap();

        reality
            .remove_component(&identifier("Motor"))
            .unwrap();

        assert!(
            reality
                .add_component(component("Motor"))
                .is_ok()
        );

        assert_eq!(reality.component_count(), 1);
    }

    #[test]
    fn deterministic_component_iteration() {
        let mut reality =
            Reality::new(identifier("drone_system"));

        for name in ["Zeta", "Alpha", "Motor", "Battery"] {
            reality
                .add_component(component(name))
                .unwrap();
        }

        let order: Vec<&str> = reality
            .components()
            .map(|component| component.name().as_str())
            .collect();

        assert_eq!(
            order,
            vec![
                "Alpha",
                "Battery",
                "Motor",
                "Zeta"
            ]
        );
    }

    #[test]
    fn iteration_order_is_independent_of_insertion_order() {
        let mut first =
            Reality::new(identifier("system_a"));

        let mut second =
            Reality::new(identifier("system_b"));

        let names = [
            "Alpha",
            "Battery",
            "Motor",
            "Zeta",
        ];

        for name in names {
            first
                .add_component(component(name))
                .unwrap();
        }

        for name in names.iter().rev() {
            second
                .add_component(component(name))
                .unwrap();
        }

        let first_order: Vec<&str> = first
            .components()
            .map(|component| component.name().as_str())
            .collect();

        let second_order: Vec<&str> = second
            .components()
            .map(|component| component.name().as_str())
            .collect();

        assert_eq!(first_order, second_order);
    }

    #[test]
    fn mutable_component_access_is_scoped() {
        let mut reality =
            Reality::new(identifier("drone_system"));

        reality
            .add_component(component("Motor"))
            .unwrap();

        reality
            .add_component(component("Battery"))
            .unwrap();

        {
            let motor = reality
                .component_mut(&identifier("Motor"))
                .expect("Motor must exist");

            // Mutation is scoped to the individual Component.
            //
            // The Reality membership collection remains inaccessible.
            let _ = motor.properties_mut();
        }

        assert_eq!(reality.component_count(), 2);

        assert_eq!(
            reality
                .component(&identifier("Motor"))
                .unwrap()
                .name(),
            &identifier("Motor")
        );

        assert_eq!(
            reality
                .component(&identifier("Battery"))
                .unwrap()
                .name(),
            &identifier("Battery")
        );

        assert!(reality.validate().is_ok());
    }

    #[test]
    fn validation_accepts_valid_reality() {
        let mut reality =
            Reality::new(identifier("drone_system"));

        for name in ["Motor", "Battery", "Payload"] {
            reality
                .add_component(component(name))
                .unwrap();
        }

        assert!(reality.validate().is_ok());
    }

    #[test]
    fn validation_holds_after_membership_changes() {
        let mut reality =
            Reality::new(identifier("drone_system"));

        reality
            .add_component(component("Motor"))
            .unwrap();

        reality
            .add_component(component("Battery"))
            .unwrap();

        assert!(reality.validate().is_ok());

        // Duplicate membership must not corrupt Reality.
        assert!(
            reality
                .add_component(component("Motor"))
                .is_err()
        );

        assert!(reality.validate().is_ok());

        reality
            .remove_component(&identifier("Motor"))
            .unwrap();

        assert!(reality.validate().is_ok());
        assert_eq!(reality.component_count(), 1);
    }

    #[test]
    fn reality_error_display_is_stable() {
        let duplicate =
            RealityError::DuplicateComponent {
                name: "Motor".to_owned(),
            };

        assert_eq!(
            duplicate.to_string(),
            "component 'Motor' already exists in this reality"
        );

        let error: &dyn Error = &duplicate;

        assert_eq!(
            error.to_string(),
            "component 'Motor' already exists in this reality"
        );
    }
}