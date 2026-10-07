//! Engineering property declarations and values for HELQIX Core.
//!
//! A property is a named engineering datum belonging to a definition.
//!
//! This module provides three levels of responsibility:
//!
//! - [`PropertyDefinition`] describes what a property is and whether a value
//!   is required.
//! - [`Property`] binds a definition to an optional [`Quantity`] value.
//! - [`PropertySet`] provides deterministic declaration, assignment and
//!   validation of multiple properties.
//!
//! The module deliberately does not contain:
//!
//! - domain-specific property names;
//! - physics equations;
//! - runtime state;
//! - simulation logic;
//! - component logic;
//! - serialization;
//! - AI logic;
//! - global registries.
//!
//! Those responsibilities belong to later HELQIX layers.
//!
//! Property names use [`Identifier`] so invalid names cannot enter a valid
//! property definition. Engineering values use [`Quantity`] so numeric
//! finiteness and unit semantics remain centralized in the quantity/unit
//! foundation modules.
//!
//! [`PropertySet`] uses [`BTreeMap`] rather than [`HashMap`] because
//! deterministic ordering is important for HELQIX engineering models,
//! reproducible serialization, hashing, comparison and evidence generation.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::error::{PropertyError, QuantityError};
use crate::id::Identifier;
use crate::quantity::Quantity;

/// A validated declaration describing one engineering property.
///
/// A definition does not contain a runtime value. It describes the semantic
/// contract for a property before a value is assigned.
///
/// # Examples
///
/// A component can declare:
///
/// - `mass` as required;
/// - `maximum_thrust` as required;
/// - `manufacturer` as optional.
///
/// The Foundation layer does not attach domain meaning to those names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PropertyDefinition {
    name: Identifier,
    required: bool,
}

impl PropertyDefinition {
    /// Creates a new property definition.
    ///
    /// The supplied [`Identifier`] has already passed identifier validation,
    /// therefore this constructor cannot create an invalid property name.
    #[must_use]
    pub const fn new(name: Identifier, required: bool) -> Self {
        Self { name, required }
    }

    /// Returns the property name.
    #[must_use]
    pub const fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns whether this property must have a value.
    #[must_use]
    pub const fn is_required(&self) -> bool {
        self.required
    }

    /// Returns whether this property may remain without a value.
    #[must_use]
    pub const fn is_optional(&self) -> bool {
        !self.required
    }
}

/// A property declaration together with its currently assigned value.
///
/// The value remains optional even for required properties. A required
/// property becomes invalid only when the containing property set is
/// validated or when a required value is explicitly requested.
///
/// This distinction is important because HELQIX must be able to construct
/// engineering definitions incrementally.
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    definition: PropertyDefinition,
    value: Option<Quantity>,
}

impl Property {
    /// Creates an unset property from a definition.
    #[must_use]
    pub const fn new(definition: PropertyDefinition) -> Self {
        Self {
            definition,
            value: None,
        }
    }

    /// Creates a property with an already validated quantity.
    #[must_use]
    pub const fn with_value(
        definition: PropertyDefinition,
        value: Quantity,
    ) -> Self {
        Self {
            definition,
            value: Some(value),
        }
    }

    /// Returns the property's definition.
    #[must_use]
    pub const fn definition(&self) -> &PropertyDefinition {
        &self.definition
    }

    /// Returns the property's name.
    #[must_use]
    pub const fn name(&self) -> &Identifier {
        self.definition.name()
    }

    /// Returns whether the property requires a value.
    #[must_use]
    pub const fn is_required(&self) -> bool {
        self.definition.is_required()
    }

    /// Returns the current value, if one has been assigned.
    #[must_use]
    pub const fn value(&self) -> Option<&Quantity> {
        self.value.as_ref()
    }

    /// Returns whether a value has been assigned.
    #[must_use]
    pub const fn has_value(&self) -> bool {
        self.value.is_some()
    }

    /// Assigns a validated quantity to the property.
    pub fn set_value(&mut self, value: Quantity) {
        self.value = Some(value);
    }

    /// Removes the current value.
    ///
    /// This is valid even for a required property. The property becomes
    /// incomplete and will subsequently fail [`PropertySet::validate`].
    pub fn clear_value(&mut self) {
        self.value = None;
    }

    /// Returns the assigned value or a structured missing-value error.
    ///
    /// This is useful at execution boundaries where a required property must
    /// be materialized before computation.
    pub fn require_value(&self) -> Result<&Quantity, PropertyError> {
        self.value.as_ref().ok_or_else(|| PropertyError::MissingRequired {
            name: self.name().as_str().to_owned(),
        })
    }

    /// Replaces the current definition while preserving the current value.
    ///
    /// The operation is only valid when the caller already owns the property.
    /// No registry-level uniqueness is handled here; [`PropertySet`] owns
    /// that responsibility.
    pub fn redefine(&mut self, definition: PropertyDefinition) {
        self.definition = definition;
    }
}

/// A deterministic collection of declared engineering properties.
///
/// [`PropertySet`] is the Foundation-level container used by later HELQIX
/// component/model layers to manage property declarations and assignments.
///
/// The collection guarantees:
///
/// - one declaration per property name;
/// - deterministic iteration order;
/// - no assignment to undeclared properties;
/// - required properties can be validated before execution;
/// - all stored values are already valid [`Quantity`] values.
///
/// `BTreeMap` is intentional. HELQIX needs reproducible engineering models,
/// deterministic diffs and stable evidence generation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PropertySet {
    properties: BTreeMap<Identifier, Property>,
}

impl PropertySet {
    /// Creates an empty property set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            properties: BTreeMap::new(),
        }
    }

    /// Returns the number of declared properties.
    #[must_use]
    pub fn len(&self) -> usize {
        self.properties.len()
    }

    /// Returns whether no properties are declared.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }

    /// Declares a new property.
    ///
    /// # Errors
    ///
    /// Returns [`PropertyError::DuplicateName`] when another property with
    /// the same identifier is already declared.
    pub fn declare(
        &mut self,
        definition: PropertyDefinition,
    ) -> Result<(), PropertyError> {
        let name = definition.name().clone();

        if self.properties.contains_key(&name) {
            return Err(PropertyError::DuplicateName {
                name: name.as_str().to_owned(),
            });
        }

        self.properties
            .insert(name, Property::new(definition));

        Ok(())
    }

    /// Declares a property and assigns its initial value atomically.
    ///
    /// If the declaration already exists, no mutation occurs.
    ///
    /// The quantity itself is already validated by [`Quantity`], so this
    /// method only performs property-level declaration semantics.
    pub fn declare_with_value(
        &mut self,
        definition: PropertyDefinition,
        value: Quantity,
    ) -> Result<(), PropertyError> {
        let name = definition.name().clone();

        if self.properties.contains_key(&name) {
            return Err(PropertyError::DuplicateName {
                name: name.as_str().to_owned(),
            });
        }

        self.properties
            .insert(name, Property::with_value(definition, value));

        Ok(())
    }

    /// Assigns a value to an already declared property.
    ///
    /// # Errors
    ///
    /// Returns [`PropertyError::Undeclared`] when the property does not exist.
    pub fn set_value(
        &mut self,
        name: &Identifier,
        value: Quantity,
    ) -> Result<(), PropertyError> {
        let property = self
            .properties
            .get_mut(name)
            .ok_or_else(|| PropertyError::Undeclared {
                name: name.as_str().to_owned(),
            })?;

        property.set_value(value);
        Ok(())
    }

    /// Assigns a value using a property name string.
    ///
    /// This method intentionally parses the identifier before accessing the
    /// collection. Invalid names therefore fail at the identifier boundary
    /// rather than being silently normalized.
    pub fn set_value_by_name(
        &mut self,
        name: &str,
        value: Quantity,
    ) -> Result<(), PropertyAssignmentError> {
        let identifier = Identifier::parse(name)
            .map_err(PropertyAssignmentError::InvalidIdentifier)?;

        self.set_value(&identifier, value)
            .map_err(PropertyAssignmentError::Property)
    }

    /// Removes the value from a declared property.
    ///
    /// The declaration itself remains intact.
    ///
    /// # Errors
    ///
    /// Returns [`PropertyError::Undeclared`] when the property does not exist.
    pub fn clear_value(
        &mut self,
        name: &Identifier,
    ) -> Result<(), PropertyError> {
        let property = self
            .properties
            .get_mut(name)
            .ok_or_else(|| PropertyError::Undeclared {
                name: name.as_str().to_owned(),
            })?;

        property.clear_value();
        Ok(())
    }

    /// Returns a property by identifier.
    #[must_use]
    pub fn get(&self, name: &Identifier) -> Option<&Property> {
        self.properties.get(name)
    }

    /// Returns a mutable property by identifier.
    pub fn get_mut(&mut self, name: &Identifier) -> Option<&mut Property> {
        self.properties.get_mut(name)
    }

    /// Returns a property by string name when the name is valid.
    ///
    /// Invalid identifier syntax simply results in `None`. This method is
    /// intended for read/query paths where malformed external input should
    /// not be converted into a mutation error.
    #[must_use]
    pub fn get_by_name(&self, name: &str) -> Option<&Property> {
        let identifier = Identifier::parse(name).ok()?;
        self.get(&identifier)
    }

    /// Returns an iterator over properties in deterministic identifier order.
    pub fn iter(&self) -> impl Iterator<Item = &Property> {
        self.properties.values()
    }

    /// Returns an iterator over `(name, property)` pairs in deterministic
    /// identifier order.
    pub fn iter_named(&self) -> impl Iterator<Item = (&Identifier, &Property)> {
        self.properties.iter()
    }

    /// Validates the complete property set.
    ///
    /// Every declared property is checked. Optional properties may remain
    /// unset; required properties may not.
    ///
    /// Validation is deterministic because [`BTreeMap`] provides a stable
    /// traversal order.
    pub fn validate(&self) -> Result<(), PropertyError> {
        for property in self.properties.values() {
            if property.is_required() && !property.has_value() {
                return Err(PropertyError::MissingRequired {
                    name: property.name().as_str().to_owned(),
                });
            }

            if let Some(value) = property.value() {
                validate_quantity(property.name(), value)?;
            }
        }

        Ok(())
    }

    /// Returns all currently missing required properties.
    ///
    /// This method is intentionally non-failing so callers such as model
    /// builders and UI validation can present the complete set of missing
    /// requirements instead of discovering them one at a time.
    #[must_use]
    pub fn missing_required(&self) -> Vec<&Identifier> {
        self.properties
            .values()
            .filter(|property| property.is_required() && !property.has_value())
            .map(Property::name)
            .collect()
    }

    /// Removes a declaration and its assigned value.
    ///
    /// # Errors
    ///
    /// Returns [`PropertyError::Undeclared`] if the property is not present.
    pub fn remove(
        &mut self,
        name: &Identifier,
    ) -> Result<Property, PropertyError> {
        self.properties
            .remove(name)
            .ok_or_else(|| PropertyError::Undeclared {
                name: name.as_str().to_owned(),
            })
    }
}

/// Errors that can occur while assigning a property using external text.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyAssignmentError {
    /// The supplied property name does not satisfy HELQIX identifier rules.
    InvalidIdentifier(crate::error::IdentifierError),

    /// The identifier was valid, but the property was not declared.
    Property(PropertyError),
}

impl fmt::Display for PropertyAssignmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(error) => error.fmt(f),
            Self::Property(error) => error.fmt(f),
        }
    }
}

impl Error for PropertyAssignmentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidIdentifier(error) => Some(error),
            Self::Property(error) => Some(error),
        }
    }
}

impl From<crate::error::IdentifierError> for PropertyAssignmentError {
    fn from(error: crate::error::IdentifierError) -> Self {
        Self::InvalidIdentifier(error)
    }
}

impl From<PropertyError> for PropertyAssignmentError {
    fn from(error: PropertyError) -> Self {
        Self::Property(error)
    }
}

/// Revalidates a quantity at the property boundary.
///
/// `Quantity` already guarantees numeric finiteness. This function keeps the
/// property boundary explicit and provides the correct `PropertyError`
/// context when future quantity validation expands.
fn validate_quantity(
    name: &Identifier,
    quantity: &Quantity,
) -> Result<(), PropertyError> {
    let value = quantity.value();

    if let Some(kind) = crate::error::NonFiniteKind::classify(value) {
        return Err(PropertyError::InvalidValue {
            name: name.as_str().to_owned(),
            source: QuantityError::NonFinite(kind),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantity::Quantity;
    use crate::units::Unit;

    fn identifier(text: &str) -> Identifier {
        Identifier::parse(text).expect("test identifier must be valid")
    }

    fn quantity(value: f64, unit: &str) -> Quantity {
        let unit = Unit::parse(unit).expect("test unit must exist");
        Quantity::new(value, unit).expect("test quantity must be finite")
    }

    #[test]
    fn property_definition_preserves_identifier_and_required_flag() {
        let definition =
            PropertyDefinition::new(identifier("mass"), true);

        assert_eq!(definition.name().as_str(), "mass");
        assert!(definition.is_required());
        assert!(!definition.is_optional());
    }

    #[test]
    fn optional_property_can_exist_without_value() {
        let property =
            Property::new(PropertyDefinition::new(identifier("mass"), false));

        assert_eq!(property.name().as_str(), "mass");
        assert!(!property.is_required());
        assert!(!property.has_value());
        assert!(property.value().is_none());
    }

    #[test]
    fn required_property_reports_missing_value() {
        let property =
            Property::new(PropertyDefinition::new(identifier("mass"), true));

        assert_eq!(
            property.require_value(),
            Err(PropertyError::MissingRequired {
                name: "mass".to_owned()
            })
        );
    }

    #[test]
    fn setting_and_clearing_value_preserves_declaration() {
        let definition =
            PropertyDefinition::new(identifier("mass"), true);

        let mut property = Property::new(definition);

        property.set_value(quantity(2.5, "kg"));

        assert!(property.has_value());
        assert_eq!(
            property.value().expect("value should exist").value(),
            2.5
        );

        property.clear_value();

        assert!(!property.has_value());
        assert_eq!(property.name().as_str(), "mass");
        assert!(property.is_required());
    }

    #[test]
    fn duplicate_property_declaration_is_rejected() {
        let mut properties = PropertySet::new();

        properties
            .declare(PropertyDefinition::new(identifier("mass"), true))
            .expect("first declaration must succeed");

        let error = properties
            .declare(PropertyDefinition::new(identifier("mass"), false))
            .expect_err("duplicate declaration must fail");

        assert_eq!(
            error,
            PropertyError::DuplicateName {
                name: "mass".to_owned()
            }
        );

        assert_eq!(properties.len(), 1);
    }

    #[test]
    fn undeclared_assignment_is_rejected() {
        let mut properties = PropertySet::new();

        let error = properties
            .set_value(&identifier("mass"), quantity(2.5, "kg"))
            .expect_err("undeclared assignment must fail");

        assert_eq!(
            error,
            PropertyError::Undeclared {
                name: "mass".to_owned()
            }
        );
    }

    #[test]
    fn declared_assignment_succeeds() {
        let mut properties = PropertySet::new();

        properties
            .declare(PropertyDefinition::new(identifier("mass"), true))
            .expect("declaration must succeed");

        properties
            .set_value(&identifier("mass"), quantity(2.5, "kg"))
            .expect("assignment must succeed");

        let property = properties
            .get(&identifier("mass"))
            .expect("property must exist");

        assert_eq!(
            property.value().expect("value must exist").value(),
            2.5
        );
    }

    #[test]
    fn string_assignment_uses_identifier_validation() {
        let mut properties = PropertySet::new();

        properties
            .declare(PropertyDefinition::new(identifier("mass"), true))
            .expect("declaration must succeed");

        properties
            .set_value_by_name("mass", quantity(3.0, "kg"))
            .expect("valid assignment must succeed");

        assert_eq!(
            properties
                .get_by_name("mass")
                .expect("property must exist")
                .value()
                .expect("value must exist")
                .value(),
            3.0
        );
    }

    #[test]
    fn invalid_string_assignment_does_not_mutate_set() {
        let mut properties = PropertySet::new();

        properties
            .declare(PropertyDefinition::new(identifier("mass"), true))
            .expect("declaration must succeed");

        let error = properties
            .set_value_by_name("9mass", quantity(3.0, "kg"))
            .expect_err("invalid identifier must fail");

        assert!(matches!(
            error,
            PropertyAssignmentError::InvalidIdentifier(_)
        ));

        assert!(!properties.get_by_name("9mass").is_some());
        assert_eq!(properties.len(), 1);
    }

    #[test]
    fn required_validation_fails_until_value_is_assigned() {
        let mut properties = PropertySet::new();

        properties
            .declare(PropertyDefinition::new(identifier("mass"), true))
            .expect("declaration must succeed");

        assert_eq!(
            properties.validate(),
            Err(PropertyError::MissingRequired {
                name: "mass".to_owned()
            })
        );

        properties
            .set_value(&identifier("mass"), quantity(4.0, "kg"))
            .expect("assignment must succeed");

        assert!(properties.validate().is_ok());
    }

    #[test]
    fn optional_unset_property_does_not_fail_validation() {
        let mut properties = PropertySet::new();

        properties
            .declare(PropertyDefinition::new(identifier("mass"), false))
            .expect("declaration must succeed");

        assert!(properties.validate().is_ok());
    }

    #[test]
    fn missing_required_returns_all_missing_properties_in_sorted_order() {
        let mut properties = PropertySet::new();

        properties
            .declare(PropertyDefinition::new(identifier("zeta"), true))
            .expect("declaration must succeed");

        properties
            .declare(PropertyDefinition::new(identifier("alpha"), true))
            .expect("declaration must succeed");

        properties
            .declare(PropertyDefinition::new(identifier("optional"), false))
            .expect("declaration must succeed");

        let missing = properties.missing_required();

        assert_eq!(
            missing
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "zeta"]
        );
    }

    #[test]
    fn property_iteration_is_deterministic() {
        let mut properties = PropertySet::new();

        properties
            .declare(PropertyDefinition::new(identifier("zeta"), false))
            .expect("declaration must succeed");

        properties
            .declare(PropertyDefinition::new(identifier("alpha"), false))
            .expect("declaration must succeed");

        properties
            .declare(PropertyDefinition::new(identifier("motor"), false))
            .expect("declaration must succeed");

        let names = properties
            .iter()
            .map(Property::name)
            .map(Identifier::as_str)
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["alpha", "motor", "zeta"]);
    }

    #[test]
    fn remove_returns_complete_property() {
        let mut properties = PropertySet::new();

        properties
            .declare_with_value(
                PropertyDefinition::new(identifier("mass"), true),
                quantity(5.0, "kg"),
            )
            .expect("declaration must succeed");

        let removed = properties
            .remove(&identifier("mass"))
            .expect("property must exist");

        assert_eq!(removed.name().as_str(), "mass");
        assert_eq!(
            removed.value().expect("value must exist").value(),
            5.0
        );
        assert!(properties.is_empty());
    }

    #[test]
    fn remove_undeclared_property_returns_structured_error() {
        let mut properties = PropertySet::new();

        let error = properties
            .remove(&identifier("mass"))
            .expect_err("missing property must fail");

        assert_eq!(
            error,
            PropertyError::Undeclared {
                name: "mass".to_owned()
            }
        );
    }

    #[test]
    fn property_assignment_error_preserves_error_chain() {
        let mut properties = PropertySet::new();

        properties
            .declare(PropertyDefinition::new(identifier("mass"), true))
            .expect("declaration must succeed");

        let error = properties
            .set_value_by_name("9mass", quantity(1.0, "kg"))
            .expect_err("invalid identifier must fail");

        assert!(error.source().is_some());
    }

    #[test]
    fn property_set_is_cloneable_and_deterministically_equal() {
        let mut properties = PropertySet::new();

        properties
            .declare_with_value(
                PropertyDefinition::new(identifier("mass"), true),
                quantity(10.0, "kg"),
            )
            .expect("declaration must succeed");

        let clone = properties.clone();

        assert_eq!(properties, clone);
    }
}