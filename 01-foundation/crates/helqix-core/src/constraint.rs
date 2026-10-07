//! Engineering constraint definitions for HELQIX Core.
//!
//! A [`Constraint`] describes a bounded engineering requirement over a named
//! Foundation member.
//!
//! Foundation owns:
//!
//! - constraint identity;
//! - referenced member identity;
//! - reference kind;
//! - finite lower and upper bounds;
//! - structural validation.
//!
//! Foundation does NOT evaluate constraints against runtime state.
//!
//! Runtime evaluation belongs to later HELQIX layers:
//!
//! ```text
//! Foundation
//!     |
//!     +-- Constraint definition
//!     |
//!     v
//! H-REALITY / H-STATE
//!     |
//!     +-- current engineering value
//!     |
//!     v
//! H-VERIFY
//!     |
//!     +-- constraint evaluation
//!     +-- violation evidence
//! ```
//!
//! This separation is essential for HELQIX because the same constraint
//! definition must be usable across:
//!
//! - different runtime states;
//! - execution branches;
//! - what-if simulations;
//! - verification runs;
//! - optimization searches.
//!
//! A constraint therefore describes the invariant requirement, not the result
//! of evaluating that requirement.

use std::fmt;

use crate::error::{ConstraintError, NonFiniteKind, ReferenceKind};
use crate::id::Identifier;

/// A reference to the engineering member controlled by a constraint.
///
/// The referenced member is intentionally unresolved at construction time.
/// This keeps the Foundation crate independent of component registries and
/// higher-level engineering models.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstraintReference {
    kind: ReferenceKind,
    name: Identifier,
}

impl ConstraintReference {
    /// Creates a constraint reference.
    #[must_use]
    pub const fn new(kind: ReferenceKind, name: Identifier) -> Self {
        Self { kind, name }
    }

    /// Creates a property constraint reference.
    #[must_use]
    pub const fn property(name: Identifier) -> Self {
        Self::new(ReferenceKind::Property, name)
    }

    /// Creates an interface constraint reference.
    #[must_use]
    pub const fn interface(name: Identifier) -> Self {
        Self::new(ReferenceKind::Interface, name)
    }

    /// Creates a behaviour constraint reference.
    #[must_use]
    pub const fn behaviour(name: Identifier) -> Self {
        Self::new(ReferenceKind::Behaviour, name)
    }

    /// Returns the referenced member kind.
    #[must_use]
    pub const fn kind(&self) -> ReferenceKind {
        self.kind
    }

    /// Returns the referenced member identifier.
    #[must_use]
    pub const fn name(&self) -> &Identifier {
        &self.name
    }
}

impl fmt::Display for ConstraintReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} `{}`", self.kind, self.name)
    }
}

/// A bounded engineering constraint.
///
/// Bounds are stored as raw finite `f64` values because Foundation defines
/// structural bounds here. Unit-aware engineering quantities can be associated
/// with the referenced property/interface at the higher engineering-model
/// layer.
///
/// The bounds are inclusive:
///
/// ```text
/// min <= value <= max
/// ```
///
/// Evaluation of that expression is intentionally outside this module.
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    name: Identifier,
    reference: ConstraintReference,
    min: f64,
    max: f64,
}

impl Constraint {
    /// Creates a validated constraint.
    ///
    /// Construction rejects:
    ///
    /// - NaN bounds;
    /// - positive infinity;
    /// - negative infinity;
    /// - `min > max`.
    ///
    /// The constraint identifier itself is already validated by
    /// [`Identifier`].
    pub fn new(
        name: Identifier,
        reference: ConstraintReference,
        min: f64,
        max: f64,
    ) -> Result<Self, ConstraintError> {
        validate_bound(name.as_str(), min)?;
        validate_bound(name.as_str(), max)?;

        if min > max {
            return Err(ConstraintError::InvertedBounds {
                constraint: name.as_str().to_owned(),
                min,
                max,
            });
        }

        Ok(Self {
            name,
            reference,
            min,
            max,
        })
    }

    /// Returns the constraint identifier.
    #[must_use]
    pub const fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the referenced member.
    #[must_use]
    pub const fn reference(&self) -> &ConstraintReference {
        &self.reference
    }

    /// Returns the referenced member kind.
    #[must_use]
    pub const fn reference_kind(&self) -> ReferenceKind {
        self.reference.kind()
    }

    /// Returns the referenced member identifier.
    #[must_use]
    pub const fn reference_name(&self) -> &Identifier {
        self.reference.name()
    }

    /// Returns the inclusive lower bound.
    #[must_use]
    pub const fn min(&self) -> f64 {
        self.min
    }

    /// Returns the inclusive upper bound.
    #[must_use]
    pub const fn max(&self) -> f64 {
        self.max
    }

    /// Returns the width of the permitted interval.
    ///
    /// Because construction guarantees finite ordered bounds, this operation
    /// is deterministic and cannot produce NaN or infinity through invalid
    /// bounds.
    #[must_use]
    pub fn width(&self) -> f64 {
        self.max - self.min
    }

    /// Returns the midpoint of the permitted interval.
    ///
    /// The result is finite because both bounds are finite and ordered.
    #[must_use]
    pub fn midpoint(&self) -> f64 {
        self.min + (self.max - self.min) * 0.5
    }

    /// Returns whether the supplied finite value lies inside the inclusive
    /// constraint interval.
    ///
    /// This method only performs mathematical interval checking. It does not
    /// resolve the referenced engineering member and does not inspect runtime
    /// state.
    ///
    /// # Panics
    ///
    /// This method never panics.
    #[must_use]
    pub fn contains(&self, value: f64) -> bool {
        if !value.is_finite() {
            return false;
        }

        value >= self.min && value <= self.max
    }

    /// Returns whether the supplied finite value violates this constraint.
    ///
    /// This is a pure interval operation and does not represent runtime
    /// verification. Higher layers should attach state, time, branch and
    /// evidence information when reporting an actual engineering violation.
    #[must_use]
    pub fn excludes(&self, value: f64) -> bool {
        !self.contains(value)
    }

    /// Validates the structural invariants of an already-created constraint.
    ///
    /// This is primarily useful for deserialization and future persistence
    /// layers where values may enter the system without passing through
    /// [`Constraint::new`].
    pub fn validate(&self) -> Result<(), ConstraintError> {
        validate_bound(self.name.as_str(), self.min)?;
        validate_bound(self.name.as_str(), self.max)?;

        if self.min > self.max {
            return Err(ConstraintError::InvertedBounds {
                constraint: self.name.as_str().to_owned(),
                min: self.min,
                max: self.max,
            });
        }

        Ok(())
    }
}

/// Validates a single constraint bound.
fn validate_bound(
    constraint: &str,
    value: f64,
) -> Result<(), ConstraintError> {
    let Some(kind) = NonFiniteKind::classify(value) else {
        return Ok(());
    };

    Err(ConstraintError::NonFiniteBound {
        constraint: constraint.to_owned(),
        bound: kind,
    })
}

/// Resolves a constraint reference against a caller-owned registry.
///
/// The Foundation layer intentionally does not know what concrete registry
/// owns the referenced definition.
///
/// Later layers can implement this trait for:
///
/// - component registries;
/// - engineering models;
/// - reality graphs;
/// - project model registries.
pub trait ConstraintReferenceResolver {
    /// Returns whether the referenced member exists.
    fn contains(
        &self,
        kind: ReferenceKind,
        name: &Identifier,
    ) -> bool;
}

impl Constraint {
    /// Validates the constraint and resolves its target through a caller-owned
    /// registry.
    ///
    /// Structural validation is always performed first. Therefore an invalid
    /// bound is reported before reference-resolution errors.
    pub fn validate_with<R>(
        &self,
        resolver: &R,
    ) -> Result<(), ConstraintError>
    where
        R: ConstraintReferenceResolver + ?Sized,
    {
        self.validate()?;

        if !resolver.contains(
            self.reference.kind(),
            self.reference.name(),
        ) {
            return Err(ConstraintError::UnresolvedReference {
                constraint: self.name.as_str().to_owned(),
                kind: self.reference.kind(),
                name: self.reference.name().as_str().to_owned(),
            });
        }

        Ok(())
    }
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "constraint `{}`: {} `{}` in [{}, {}]",
            self.name,
            self.reference.kind(),
            self.reference.name(),
            self.min,
            self.max
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestResolver {
        properties: Vec<Identifier>,
        interfaces: Vec<Identifier>,
        behaviours: Vec<Identifier>,
    }

    impl TestResolver {
        fn new() -> Self {
            Self {
                properties: Vec::new(),
                interfaces: Vec::new(),
                behaviours: Vec::new(),
            }
        }

        fn property(mut self, name: &str) -> Self {
            self.properties
                .push(id(name));
            self
        }

        fn interface(mut self, name: &str) -> Self {
            self.interfaces
                .push(id(name));
            self
        }

        fn behaviour(mut self, name: &str) -> Self {
            self.behaviours
                .push(id(name));
            self
        }
    }

    impl ConstraintReferenceResolver for TestResolver {
        fn contains(
            &self,
            kind: ReferenceKind,
            name: &Identifier,
        ) -> bool {
            match kind {
                ReferenceKind::Property => {
                    self.properties.iter().any(|item| item == name)
                }
                ReferenceKind::Interface => {
                    self.interfaces.iter().any(|item| item == name)
                }
                ReferenceKind::Behaviour => {
                    self.behaviours.iter().any(|item| item == name)
                }
            }
        }
    }

    fn id(value: &str) -> Identifier {
        Identifier::parse(value)
            .expect("test identifier must be valid")
    }

    #[test]
    fn valid_property_constraint_is_created() {
        let constraint = Constraint::new(
            id("maximum_speed"),
            ConstraintReference::property(id("speed")),
            0.0,
            100.0,
        )
        .expect("constraint should be valid");

        assert_eq!(constraint.name().as_str(), "maximum_speed");
        assert_eq!(
            constraint.reference_kind(),
            ReferenceKind::Property
        );
        assert_eq!(
            constraint.reference_name().as_str(),
            "speed"
        );
        assert_eq!(constraint.min(), 0.0);
        assert_eq!(constraint.max(), 100.0);
    }

    #[test]
    fn equal_bounds_create_a_single_point_constraint() {
        let constraint = Constraint::new(
            id("fixed_value"),
            ConstraintReference::property(id("temperature")),
            25.0,
            25.0,
        )
        .expect("equal bounds should be valid");

        assert_eq!(constraint.width(), 0.0);
        assert_eq!(constraint.midpoint(), 25.0);

        assert!(constraint.contains(25.0));
        assert!(!constraint.contains(24.999));
        assert!(!constraint.contains(25.001));
    }

    #[test]
    fn inverted_bounds_are_rejected() {
        let error = Constraint::new(
            id("invalid_range"),
            ConstraintReference::property(id("speed")),
            100.0,
            20.0,
        )
        .expect_err("inverted bounds must fail");

        assert_eq!(
            error,
            ConstraintError::InvertedBounds {
                constraint: "invalid_range".to_owned(),
                min: 100.0,
                max: 20.0,
            }
        );
    }

    #[test]
    fn nan_lower_bound_is_rejected() {
        let error = Constraint::new(
            id("invalid_nan"),
            ConstraintReference::property(id("speed")),
            f64::NAN,
            100.0,
        )
        .expect_err("NaN must fail");

        assert_eq!(
            error,
            ConstraintError::NonFiniteBound {
                constraint: "invalid_nan".to_owned(),
                bound: NonFiniteKind::Nan,
            }
        );
    }

    #[test]
    fn nan_upper_bound_is_rejected() {
        let error = Constraint::new(
            id("invalid_nan"),
            ConstraintReference::property(id("speed")),
            0.0,
            f64::NAN,
        )
        .expect_err("NaN must fail");

        assert_eq!(
            error,
            ConstraintError::NonFiniteBound {
                constraint: "invalid_nan".to_owned(),
                bound: NonFiniteKind::Nan,
            }
        );
    }

    #[test]
    fn positive_infinity_is_rejected() {
        let error = Constraint::new(
            id("invalid_upper"),
            ConstraintReference::property(id("speed")),
            0.0,
            f64::INFINITY,
        )
        .expect_err("infinity must fail");

        assert_eq!(
            error,
            ConstraintError::NonFiniteBound {
                constraint: "invalid_upper".to_owned(),
                bound: NonFiniteKind::PositiveInfinity,
            }
        );
    }

    #[test]
    fn negative_infinity_is_rejected() {
        let error = Constraint::new(
            id("invalid_lower"),
            ConstraintReference::property(id("speed")),
            f64::NEG_INFINITY,
            100.0,
        )
        .expect_err("negative infinity must fail");

        assert_eq!(
            error,
            ConstraintError::NonFiniteBound {
                constraint: "invalid_lower".to_owned(),
                bound: NonFiniteKind::NegativeInfinity,
            }
        );
    }

    #[test]
    fn contains_is_inclusive() {
        let constraint = Constraint::new(
            id("range"),
            ConstraintReference::property(id("speed")),
            10.0,
            20.0,
        )
        .expect("constraint should be valid");

        assert!(constraint.contains(10.0));
        assert!(constraint.contains(15.0));
        assert!(constraint.contains(20.0));

        assert!(!constraint.contains(9.999));
        assert!(!constraint.contains(20.001));
    }

    #[test]
    fn non_finite_runtime_value_is_never_inside_constraint() {
        let constraint = Constraint::new(
            id("range"),
            ConstraintReference::property(id("speed")),
            0.0,
            100.0,
        )
        .expect("constraint should be valid");

        assert!(!constraint.contains(f64::NAN));
        assert!(!constraint.contains(f64::INFINITY));
        assert!(!constraint.contains(f64::NEG_INFINITY));
    }

    #[test]
    fn excludes_is_inverse_of_contains_for_finite_values() {
        let constraint = Constraint::new(
            id("range"),
            ConstraintReference::property(id("speed")),
            10.0,
            20.0,
        )
        .expect("constraint should be valid");

        for value in [9.0, 10.0, 15.0, 20.0, 21.0] {
            assert_eq!(
                constraint.excludes(value),
                !constraint.contains(value)
            );
        }
    }

    #[test]
    fn midpoint_is_stable() {
        let constraint = Constraint::new(
            id("range"),
            ConstraintReference::property(id("speed")),
            10.0,
            30.0,
        )
        .expect("constraint should be valid");

        assert_eq!(constraint.midpoint(), 20.0);
        assert_eq!(constraint.width(), 20.0);
    }

    #[test]
    fn reference_constructor_sets_correct_kind() {
        let property =
            ConstraintReference::property(id("speed"));
        let interface =
            ConstraintReference::interface(id("motor_input"));
        let behaviour =
            ConstraintReference::behaviour(id("thermal_limit"));

        assert_eq!(property.kind(), ReferenceKind::Property);
        assert_eq!(interface.kind(), ReferenceKind::Interface);
        assert_eq!(behaviour.kind(), ReferenceKind::Behaviour);
    }

    #[test]
    fn resolver_accepts_declared_property() {
        let constraint = Constraint::new(
            id("speed_limit"),
            ConstraintReference::property(id("speed")),
            0.0,
            100.0,
        )
        .expect("constraint should be valid");

        let resolver = TestResolver::new()
            .property("speed");

        assert!(constraint.validate_with(&resolver).is_ok());
    }

    #[test]
    fn resolver_accepts_declared_interface() {
        let constraint = Constraint::new(
            id("input_limit"),
            ConstraintReference::interface(id("motor_input")),
            0.0,
            10.0,
        )
        .expect("constraint should be valid");

        let resolver = TestResolver::new()
            .interface("motor_input");

        assert!(constraint.validate_with(&resolver).is_ok());
    }

    #[test]
    fn resolver_accepts_declared_behaviour() {
        let constraint = Constraint::new(
            id("thermal_limit"),
            ConstraintReference::behaviour(id("thermal_limit")),
            0.0,
            1.0,
        )
        .expect("constraint should be valid");

        let resolver = TestResolver::new()
            .behaviour("thermal_limit");

        assert!(constraint.validate_with(&resolver).is_ok());
    }

    #[test]
    fn resolver_reports_unresolved_property() {
        let constraint = Constraint::new(
            id("speed_limit"),
            ConstraintReference::property(id("speed")),
            0.0,
            100.0,
        )
        .expect("constraint should be valid");

        let resolver = TestResolver::new();

        assert_eq!(
            constraint.validate_with(&resolver),
            Err(ConstraintError::UnresolvedReference {
                constraint: "speed_limit".to_owned(),
                kind: ReferenceKind::Property,
                name: "speed".to_owned(),
            })
        );
    }

    #[test]
    fn resolver_reports_unresolved_interface() {
        let constraint = Constraint::new(
            id("motor_limit"),
            ConstraintReference::interface(id("motor_output")),
            0.0,
            100.0,
        )
        .expect("constraint should be valid");

        let resolver = TestResolver::new();

        assert_eq!(
            constraint.validate_with(&resolver),
            Err(ConstraintError::UnresolvedReference {
                constraint: "motor_limit".to_owned(),
                kind: ReferenceKind::Interface,
                name: "motor_output".to_owned(),
            })
        );
    }

    #[test]
    fn resolver_reports_unresolved_behaviour() {
        let constraint = Constraint::new(
            id("thermal_constraint"),
            ConstraintReference::behaviour(id("thermal_limit")),
            0.0,
            1.0,
        )
        .expect("constraint should be valid");

        let resolver = TestResolver::new();

        assert_eq!(
            constraint.validate_with(&resolver),
            Err(ConstraintError::UnresolvedReference {
                constraint: "thermal_constraint".to_owned(),
                kind: ReferenceKind::Behaviour,
                name: "thermal_limit".to_owned(),
            })
        );
    }

    #[test]
    fn structural_validation_precedes_reference_resolution() {
        let constraint = Constraint {
            name: id("invalid"),
            reference: ConstraintReference::property(id("missing")),
            min: 100.0,
            max: 10.0,
        };

        let resolver = TestResolver::new();

        assert_eq!(
            constraint.validate_with(&resolver),
            Err(ConstraintError::InvertedBounds {
                constraint: "invalid".to_owned(),
                min: 100.0,
                max: 10.0,
            })
        );
    }

    #[test]
    fn validate_accepts_valid_definition() {
        let constraint = Constraint::new(
            id("speed_limit"),
            ConstraintReference::property(id("speed")),
            0.0,
            100.0,
        )
        .expect("constraint should be valid");

        assert!(constraint.validate().is_ok());
    }

    #[test]
    fn display_contains_definition_and_reference() {
        let constraint = Constraint::new(
            id("speed_limit"),
            ConstraintReference::property(id("speed")),
            0.0,
            100.0,
        )
        .expect("constraint should be valid");

        assert_eq!(
            constraint.to_string(),
            "constraint `speed_limit`: property `speed` in [0, 100]"
        );
    }

    #[test]
    fn constraint_is_deterministic_value_data() {
        let first = Constraint::new(
            id("speed_limit"),
            ConstraintReference::property(id("speed")),
            0.0,
            100.0,
        )
        .expect("constraint should be valid");

        let second = Constraint::new(
            id("speed_limit"),
            ConstraintReference::property(id("speed")),
            0.0,
            100.0,
        )
        .expect("constraint should be valid");

        assert_eq!(first, second);
    }
}