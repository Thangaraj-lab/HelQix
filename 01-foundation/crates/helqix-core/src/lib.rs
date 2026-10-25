//! HELQIX Core Foundation.
//!
//! `helqix-core` is the deterministic engineering-definition layer of HELQIX.
//!
//! It provides the primitive contracts required by higher HELQIX layers:
//!
//! ```text
//! Identifier
//!     ↓
//! Units / Dimensions / Quantities
//!     ↓
//! Properties
//!     ↓
//! Components
//!     ↓
//! Behaviours
//!     ↓
//! Interfaces
//!     ↓
//! Constraints
//! ```
//!
//! The Core crate deliberately does not contain:
//!
//! - runtime state;
//! - simulation;
//! - physics integration;
//! - execution scheduling;
//! - visualization;
//! - AI execution;
//! - optimization engines;
//! - network services;
//! - persistence backends.
//!
//! Those concerns belong to higher HELQIX layers.
//!
//! ## Determinism
//!
//! Foundation values are designed to be deterministic:
//!
//! - identifiers contain no generated randomness;
//! - units come from a fixed vocabulary;
//! - maps use deterministic ordering;
//! - definitions contain no wall-clock state;
//! - validation does not depend on external I/O.
//!
//! This allows the same engineering definition to become the stable source
//! from which runtime state, execution, branching and verification are later
//! derived.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// -----------------------------------------------------------------------------
// Foundation modules
// -----------------------------------------------------------------------------

/// Declared reproducibility level of a run.
pub mod determinism;

/// Structured errors produced by HELQIX Foundation.
pub mod error;

/// Validated deterministic identifiers.
pub mod id;

/// Physical dimensions and units.
pub mod units;

/// Finite numeric engineering quantities.
pub mod quantity;

/// Engineering property declarations and values.
pub mod property;

/// Engineering component definitions.
pub mod component;

/// Structural behaviour contracts.
pub mod behaviour;

/// Typed engineering interfaces.
pub mod interface;

/// Engineering constraint definitions.
pub mod constraint;

// -----------------------------------------------------------------------------
// Error API
// -----------------------------------------------------------------------------

pub use error::{
    BehaviourError,
    ComponentError,
    ConstraintError,
    CoreError,
    CoreResult,
    IdentifierError,
    IncompatibilityReason,
    InterfaceError,
    InvariantViolation,
    NonFiniteKind,
    PropertyError,
    QuantityError,
    ReferenceKind,
    SerializationDirection,
    SerializationError,
    SourceLocation,
    UnitError,
};

// -----------------------------------------------------------------------------
// Identifier API
// -----------------------------------------------------------------------------

pub use id::{
    Identifier,
    MAX_IDENTIFIER_LENGTH,
};

// -----------------------------------------------------------------------------
// Units / quantities API
// -----------------------------------------------------------------------------

pub use quantity::{
    Quantity,
    QuantityConversionError,
};

pub use units::{
    Dimension,
    Unit,
};

// -----------------------------------------------------------------------------
// Property API
// -----------------------------------------------------------------------------

pub use property::{
    Property,
    PropertyAssignmentError,
    PropertyDefinition,
    PropertySet,
};

// -----------------------------------------------------------------------------
// Component API
// -----------------------------------------------------------------------------

pub use component::{
    Component,
    ComponentPropertyError,
    ComponentValidationError,
};

// -----------------------------------------------------------------------------
// Behaviour API
// -----------------------------------------------------------------------------

pub use behaviour::{
    BehaviourClause,
    BehaviourContract,
    BehaviourReferenceResolver,
};

// -----------------------------------------------------------------------------
// Interface API
// -----------------------------------------------------------------------------

pub use interface::{
    Interface,
    InterfaceDirection,
    InterfaceMember,
};

// -----------------------------------------------------------------------------
// Constraint API
// -----------------------------------------------------------------------------

pub use constraint::{
    Constraint,
    ConstraintReference,
    ConstraintReferenceResolver,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> Identifier {
        Identifier::parse(value)
            .expect("test identifier must be valid")
    }

    #[test]
    fn foundation_public_api_exposes_identifier() {
        let identifier = Identifier::parse("motor")
            .expect("identifier should be valid");

        assert_eq!(identifier.as_str(), "motor");
    }

    #[test]
    fn foundation_public_api_exposes_units_and_quantities() {
        let metres = Unit::parse("m")
            .expect("metre unit should exist");

        let quantity = Quantity::new(2.5, metres)
            .expect("finite quantity should be valid");

        assert_eq!(quantity.value(), 2.5);
        assert_eq!(quantity.unit().symbol(), "m");
    }

    #[test]
    fn foundation_public_api_exposes_properties() {
        let definition =
            PropertyDefinition::new(id("mass"), true);

        let property = Property::new(definition);

        assert_eq!(property.name().as_str(), "mass");
        assert!(property.is_required());
        assert!(!property.has_value());
    }

    #[test]
    fn foundation_public_api_exposes_component() {
        let component = Component::new(id("motor"));

        assert_eq!(component.name().as_str(), "motor");
        assert_eq!(component.property_count(), 0);
    }

    #[test]
    fn foundation_public_api_exposes_behaviour_contract() {
        let mut behaviour =
            BehaviourContract::new(id("start"));

        behaviour.add_clause(
            BehaviourClause::property(id("speed")),
        );

        assert!(behaviour.validate().is_ok());
    }

    #[test]
    fn foundation_public_api_exposes_interface() {
        let mut interface =
            Interface::new(id("motor_output"));

        interface
            .add_member(InterfaceMember::new(
                id("speed"),
                InterfaceDirection::Output,
                Dimension::TIME,
            ))
            .expect("member should be accepted");

        assert_eq!(interface.member_count(), 1);
    }

    #[test]
    fn foundation_public_api_exposes_constraint() {
        let constraint = Constraint::new(
            id("speed_limit"),
            ConstraintReference::property(id("speed")),
            0.0,
            100.0,
        )
        .expect("constraint should be valid");

        assert!(constraint.contains(50.0));
        assert!(!constraint.contains(101.0));
    }

    #[test]
    fn core_error_converts_from_foundation_errors() {
        let error = CoreError::from(
            ConstraintError::InvertedBounds {
                constraint: "speed_limit".to_owned(),
                min: 100.0,
                max: 10.0,
            },
        );

        assert!(matches!(
            error,
            CoreError::Constraint(
                ConstraintError::InvertedBounds { .. }
            )
        ));
    }

    #[test]
    fn reference_kind_is_shared_across_foundation() {
        assert_eq!(
            ReferenceKind::Property.to_string(),
            "property"
        );

        assert_eq!(
            ReferenceKind::Interface.to_string(),
            "interface"
        );

        assert_eq!(
            ReferenceKind::Behaviour.to_string(),
            "behaviour"
        );
    }

    #[test]
    fn foundation_types_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<Identifier>();
        assert_send_sync::<Dimension>();
        assert_send_sync::<Unit>();
        assert_send_sync::<Quantity>();
        assert_send_sync::<Property>();
        assert_send_sync::<PropertySet>();
        assert_send_sync::<Component>();
        assert_send_sync::<BehaviourContract>();
        assert_send_sync::<Interface>();
        assert_send_sync::<Constraint>();
    }

    #[test]
    fn foundation_definitions_are_cloneable() {
        let component =
            Component::new(id("motor"));

        let behaviour =
            BehaviourContract::new(id("start"));

        let interface =
            Interface::new(id("motor_input"));

        let constraint = Constraint::new(
            id("speed_limit"),
            ConstraintReference::property(id("speed")),
            0.0,
            100.0,
        )
        .expect("constraint should be valid");

        assert_eq!(component.clone(), component);
        assert_eq!(behaviour.clone(), behaviour);
        assert_eq!(interface.clone(), interface);
        assert_eq!(constraint.clone(), constraint);
    }
}