//! Engineering component definitions for HELQIX Core.
//!
//! A component is a deterministic Foundation-level definition. Runtime state,
//! simulation behaviour and visual state belong to later HELQIX layers.

use std::collections::BTreeMap;
use std::fmt;

use crate::error::{ComponentError, PropertyError, ReferenceKind};
use crate::id::Identifier;
use crate::property::{Property, PropertyDefinition, PropertySet};
use crate::quantity::Quantity;

/// A Foundation-level engineering component definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    name: Identifier,
    properties: PropertySet,
    behaviours: BTreeMap<Identifier, ()>,
    interfaces: BTreeMap<Identifier, ()>,
}

impl Component {
    /// Creates an empty component definition.
    #[must_use]
    pub const fn new(name: Identifier) -> Self {
        Self { name, properties: PropertySet::new(), behaviours: BTreeMap::new(), interfaces: BTreeMap::new() }
    }

    /// Returns the component identifier.
    #[must_use]
    pub const fn name(&self) -> &Identifier { &self.name }

    /// Returns declared properties.
    #[must_use]
    pub const fn properties(&self) -> &PropertySet { &self.properties }

    /// Returns mutable access to declared properties.
    pub const fn properties_mut(&mut self) -> &mut PropertySet { &mut self.properties }

    /// Returns the number of declared properties.
    #[must_use]
    pub fn property_count(&self) -> usize { self.properties.len() }

    /// Returns the number of behaviour references.
    #[must_use]
    pub fn behaviour_count(&self) -> usize { self.behaviours.len() }

    /// Returns the number of interface references.
    #[must_use]
    pub fn interface_count(&self) -> usize { self.interfaces.len() }

    /// Declares a property.
    pub fn declare_property(&mut self, definition: PropertyDefinition) -> Result<(), ComponentPropertyError> {
        self.properties.declare(definition).map_err(ComponentPropertyError::Property)
    }

    /// Declares a property with an initial value.
    pub fn declare_property_with_value(&mut self, definition: PropertyDefinition, value: Quantity) -> Result<(), ComponentPropertyError> {
        self.properties.declare_with_value(definition, value).map_err(ComponentPropertyError::Property)
    }

    /// Assigns a value to an already declared property.
    pub fn set_property(&mut self, name: &Identifier, value: Quantity) -> Result<(), ComponentPropertyError> {
        self.properties.set_value(name, value).map_err(ComponentPropertyError::Property)
    }

    /// Returns a property by identifier.
    #[must_use]
    pub fn property(&self, name: &Identifier) -> Option<&Property> { self.properties.get(name) }

    /// Adds a behaviour reference.
    pub fn add_behaviour(&mut self, behaviour: Identifier) -> Result<(), ComponentError> {
        insert_reference(&mut self.behaviours, &self.name, ReferenceKind::Behaviour, behaviour)
    }

    /// Adds an interface reference.
    pub fn add_interface(&mut self, interface: Identifier) -> Result<(), ComponentError> {
        insert_reference(&mut self.interfaces, &self.name, ReferenceKind::Interface, interface)
    }

    /// Removes a behaviour reference and reports whether it existed.
    pub fn remove_behaviour(&mut self, behaviour: &Identifier) -> bool { self.behaviours.remove(behaviour).is_some() }

    /// Removes an interface reference and reports whether it existed.
    pub fn remove_interface(&mut self, interface: &Identifier) -> bool { self.interfaces.remove(interface).is_some() }

    /// Returns whether the behaviour is referenced.
    #[must_use]
    pub fn has_behaviour(&self, behaviour: &Identifier) -> bool { self.behaviours.contains_key(behaviour) }

    /// Returns whether the interface is referenced.
    #[must_use]
    pub fn has_interface(&self, interface: &Identifier) -> bool { self.interfaces.contains_key(interface) }

    /// Iterates behaviour references deterministically.
    pub fn behaviours(&self) -> impl Iterator<Item = &Identifier> { self.behaviours.keys() }

    /// Iterates interface references deterministically.
    pub fn interfaces(&self) -> impl Iterator<Item = &Identifier> { self.interfaces.keys() }

    /// Validates locally enforceable component invariants.
    pub fn validate(&self) -> Result<(), ComponentValidationError> {
        self.properties.validate().map_err(ComponentValidationError::Property)
    }

    /// Returns required properties without assigned values.
    #[must_use]
    pub fn missing_required_properties(&self) -> Vec<&Identifier> { self.properties.missing_required() }

    /// Returns the total number of properties, behaviours and interfaces.
    #[must_use]
    pub fn member_count(&self) -> usize { self.property_count() + self.behaviour_count() + self.interface_count() }
}

fn insert_reference(registry: &mut BTreeMap<Identifier, ()>, component: &Identifier, kind: ReferenceKind, reference: Identifier) -> Result<(), ComponentError> {
    if registry.contains_key(&reference) {
        return Err(ComponentError::DuplicateMember { component: component.as_str().to_owned(), kind, name: reference.as_str().to_owned() });
    }
    registry.insert(reference, ());
    Ok(())
}

/// Error returned by component property operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentPropertyError { Property(PropertyError) }
impl fmt::Display for ComponentPropertyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self { Self::Property(error) => error.fmt(f) } }
}
impl std::error::Error for ComponentPropertyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { match self { Self::Property(error) => Some(error) } }
}
impl From<PropertyError> for ComponentPropertyError { fn from(error: PropertyError) -> Self { Self::Property(error) } }

/// Errors produced while validating a component.
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentValidationError { Property(PropertyError) }
impl fmt::Display for ComponentValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self { Self::Property(error) => error.fmt(f) } }
}
impl std::error::Error for ComponentValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { match self { Self::Property(error) => Some(error) } }
}
impl From<PropertyError> for ComponentValidationError { fn from(error: PropertyError) -> Self { Self::Property(error) } }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantity::Quantity;
    use crate::units::Unit;

    fn id(value: &str) -> Identifier { Identifier::parse(value).expect("test identifier must be valid") }
    fn quantity(value: f64, unit: &str) -> Quantity { Quantity::new(value, Unit::parse(unit).expect("test unit must exist")).expect("test quantity must be finite") }

    #[test]
    fn component_starts_empty() {
        let component = Component::new(id("motor"));
        assert_eq!(component.name().as_str(), "motor");
        assert_eq!(component.property_count(), 0);
        assert_eq!(component.behaviour_count(), 0);
        assert_eq!(component.interface_count(), 0);
        assert_eq!(component.member_count(), 0);
    }

    #[test]
    fn property_can_be_declared_on_component() {
        let mut component = Component::new(id("motor"));
        component.declare_property(PropertyDefinition::new(id("mass"), true)).expect("property declaration must succeed");
        let property = component.property(&id("mass")).expect("property must exist");
        assert_eq!(component.property_count(), 1);
        assert_eq!(property.name().as_str(), "mass");
        assert!(property.is_required());
        assert!(!property.has_value());
    }

    #[test]
    fn property_can_be_declared_with_initial_value() {
        let mut component = Component::new(id("motor"));
        component.declare_property_with_value(PropertyDefinition::new(id("mass"), true), quantity(80.0, "g")).expect("property declaration must succeed");
        let property = component.property(&id("mass")).expect("property must exist");
        assert_eq!(property.value().expect("value must exist").value(), 80.0);
        assert_eq!(property.value().expect("value must exist").unit().symbol(), "g");
    }

    #[test]
    fn property_assignment_requires_existing_declaration() {
        let mut component = Component::new(id("motor"));
        let error = component.set_property(&id("mass"), quantity(80.0, "g")).expect_err("undeclared property must fail");
        assert!(matches!(error, ComponentPropertyError::Property(PropertyError::Undeclared { .. })));
    }

    #[test]
    fn duplicate_property_is_rejected() {
        let mut component = Component::new(id("motor"));
        component.declare_property(PropertyDefinition::new(id("mass"), true)).expect("first declaration must succeed");
        let error = component.declare_property(PropertyDefinition::new(id("mass"), false)).expect_err("duplicate declaration must fail");
        assert!(matches!(error, ComponentPropertyError::Property(PropertyError::DuplicateName { .. })));
        assert_eq!(component.property_count(), 1);
    }

    #[test]
    fn behaviour_reference_can_be_added() {
        let mut component = Component::new(id("motor"));
        component.add_behaviour(id("rotational_dynamics")).expect("behaviour reference must succeed");
        assert_eq!(component.behaviour_count(), 1);
        assert!(component.has_behaviour(&id("rotational_dynamics")));
    }

    #[test]
    fn interface_reference_can_be_added() {
        let mut component = Component::new(id("motor"));
        component.add_interface(id("electrical_input")).expect("interface reference must succeed");
        assert_eq!(component.interface_count(), 1);
        assert!(component.has_interface(&id("electrical_input")));
    }

    #[test]
    fn duplicate_behaviour_reference_is_rejected() {
        let mut component = Component::new(id("motor"));
        component.add_behaviour(id("rotational_dynamics")).expect("first reference must succeed");
        let error = component.add_behaviour(id("rotational_dynamics")).expect_err("duplicate reference must fail");
        assert_eq!(error, ComponentError::DuplicateMember { component: "motor".to_owned(), kind: ReferenceKind::Behaviour, name: "rotational_dynamics".to_owned() });
        assert_eq!(component.behaviour_count(), 1);
    }

    #[test]
    fn duplicate_interface_reference_is_rejected() {
        let mut component = Component::new(id("motor"));
        component.add_interface(id("electrical_input")).expect("first reference must succeed");
        let error = component.add_interface(id("electrical_input")).expect_err("duplicate reference must fail");
        assert_eq!(error, ComponentError::DuplicateMember { component: "motor".to_owned(), kind: ReferenceKind::Interface, name: "electrical_input".to_owned() });
        assert_eq!(component.interface_count(), 1);
    }

    #[test]
    fn behaviour_and_interface_names_are_independent_namespaces() {
        let mut component = Component::new(id("motor"));
        component.add_behaviour(id("electrical")).expect("behaviour reference must succeed");
        component.add_interface(id("electrical")).expect("interface reference must succeed");
        assert_eq!(component.behaviour_count(), 1);
        assert_eq!(component.interface_count(), 1);
    }

    #[test]
    fn references_are_iterated_deterministically() {
        let mut component = Component::new(id("motor"));
        component.add_behaviour(id("zeta")).expect("reference must succeed");
        component.add_behaviour(id("alpha")).expect("reference must succeed");
        component.add_behaviour(id("middle")).expect("reference must succeed");
        let names = component.behaviours().map(Identifier::as_str).collect::<Vec<_>>();
        assert_eq!(names, vec!["alpha", "middle", "zeta"]);
    }

    #[test]
    fn required_property_makes_component_invalid_until_assigned() {
        let mut component = Component::new(id("motor"));
        component.declare_property(PropertyDefinition::new(id("mass"), true)).expect("property declaration must succeed");
        assert!(matches!(component.validate(), Err(ComponentValidationError::Property(PropertyError::MissingRequired { .. }))));
        component.set_property(&id("mass"), quantity(80.0, "g")).expect("property assignment must succeed");
        assert!(component.validate().is_ok());
    }

    #[test]
    fn optional_property_does_not_block_component_validation() {
        let mut component = Component::new(id("motor"));
        component.declare_property(PropertyDefinition::new(id("mass"), false)).expect("property declaration must succeed");
        assert!(component.validate().is_ok());
    }

    #[test]
    fn missing_required_properties_are_exposed() {
        let mut component = Component::new(id("motor"));
        component.declare_property(PropertyDefinition::new(id("mass"), true)).expect("property declaration must succeed");
        component.declare_property(PropertyDefinition::new(id("temperature"), true)).expect("property declaration must succeed");
        component.declare_property(PropertyDefinition::new(id("label"), false)).expect("property declaration must succeed");
        let missing = component.missing_required_properties();
        assert_eq!(missing.iter().map(|identifier| identifier.as_str()).collect::<Vec<_>>(), vec!["mass", "temperature"]);
    }

    #[test]
    fn removing_behaviour_reference_is_idempotent() {
        let mut component = Component::new(id("motor"));
        component.add_behaviour(id("rotational_dynamics")).expect("reference must succeed");
        assert!(component.remove_behaviour(&id("rotational_dynamics")));
        assert!(!component.remove_behaviour(&id("rotational_dynamics")));
        assert_eq!(component.behaviour_count(), 0);
    }

    #[test]
    fn removing_interface_reference_is_idempotent() {
        let mut component = Component::new(id("motor"));
        component.add_interface(id("electrical_input")).expect("reference must succeed");
        assert!(component.remove_interface(&id("electrical_input")));
        assert!(!component.remove_interface(&id("electrical_input")));
        assert_eq!(component.interface_count(), 0);
    }

    #[test]
    fn member_count_tracks_all_component_members() {
        let mut component = Component::new(id("motor"));
        component.declare_property(PropertyDefinition::new(id("mass"), true)).expect("property declaration must succeed");
        component.declare_property(PropertyDefinition::new(id("max_rpm"), true)).expect("property declaration must succeed");
        component.add_behaviour(id("rotational_dynamics")).expect("behaviour reference must succeed");
        component.add_interface(id("electrical_input")).expect("interface reference must succeed");
        assert_eq!(component.member_count(), 4);
    }

    #[test]
    fn component_clone_is_semantically_equal() {
        let mut component = Component::new(id("motor"));
        component.declare_property_with_value(PropertyDefinition::new(id("mass"), true), quantity(80.0, "g")).expect("property declaration must succeed");
        component.add_behaviour(id("rotational_dynamics")).expect("behaviour reference must succeed");
        component.add_interface(id("electrical_input")).expect("interface reference must succeed");
        assert_eq!(component, component.clone());
    }
}
