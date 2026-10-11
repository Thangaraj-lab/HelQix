//! Engineering constraint definitions for HELQIX Layer 03 — Engineering.
//!
//! A [`Constraint`] describes an inclusive engineering boundary that must
//! hold for a system definition to be considered valid, e.g.:
//!
//! ```text
//! motor_temperature <= 120 K
//! battery_voltage   >= 42 V
//! rotor_speed       within [1000, 8000] rpm
//! ```
//!
//! A constraint is an **engineering definition**. It does not monitor live
//! values, does not execute, does not trigger failure handling, and does
//! not simulate anything. Evaluating a constraint against runtime state
//! belongs to a later HELQIX layer.
//!
//! Structural validation ([`Constraint::validate`]) and cross-reference
//! validation ([`Constraint::validate_with`]) are deliberately separated.
//! Cross-reference validation is driven by a caller-supplied
//! [`ConstraintReferenceResolver`], so this module never depends on a
//! concrete `ParameterSet`, interface collection, behaviour collection,
//! `Reality`, or any runtime state.

use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;
use helqix_core::NonFiniteKind;
use helqix_core::ReferenceKind;

/// Classifies a non-finite `f64` into a [`NonFiniteKind`], or returns
/// `None` when the value is finite.
fn classify_non_finite(value: f64) -> Option<NonFiniteKind> {
    if value.is_nan() {
        Some(NonFiniteKind::Nan)
    } else if value.is_infinite() {
        if value.is_sign_positive() {
            Some(NonFiniteKind::PositiveInfinity)
        } else {
            Some(NonFiniteKind::NegativeInfinity)
        }
    } else {
        None
    }
}

/// A structural reference from a [`Constraint`] to an engineering concept.
///
/// A reference binds a [`ReferenceKind`] (property, interface, behaviour)
/// to an [`Identifier`] naming the referenced definition. It carries no
/// runtime value and performs no resolution.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConstraintReference {
    kind: ReferenceKind,
    name: Identifier,
}

impl ConstraintReference {
    /// Creates a new constraint reference.
    pub fn new(kind: ReferenceKind, name: Identifier) -> Self {
        Self { kind, name }
    }

    /// Creates a reference to an engineering property.
    pub fn property(name: Identifier) -> Self {
        Self::new(ReferenceKind::Property, name)
    }

    /// Creates a reference to an engineering interface.
    pub fn interface(name: Identifier) -> Self {
        Self::new(ReferenceKind::Interface, name)
    }

    /// Creates a reference to another engineering behaviour.
    pub fn behaviour(name: Identifier) -> Self {
        Self::new(ReferenceKind::Behaviour, name)
    }

    /// Returns the reference kind.
    pub fn kind(&self) -> ReferenceKind {
        self.kind
    }

    /// Returns the referenced concept's identifier.
    pub fn name(&self) -> &Identifier {
        &self.name
    }
}

impl fmt::Display for ConstraintReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}:{}", self.kind, self.name.as_str())
    }
}

/// Errors produced while constructing or validating a [`Constraint`].
///
/// `PartialEq` is derived; `Eq` is implemented manually because the
/// `InvertedBounds` variant carries `f64` bounds, which do not themselves
/// implement `Eq`. All bound values stored in this error are guaranteed
/// non-NaN by the constructor, so the equivalence relation holds.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintError {
    /// `min > max`.
    InvertedBounds {
        /// Name of the constraint.
        constraint: String,
        /// The offending minimum bound.
        min: f64,
        /// The offending maximum bound.
        max: f64,
    },
    /// A bound was NaN, +∞, or −∞.
    NonFiniteBound {
        /// Name of the constraint.
        constraint: String,
        /// Classification of the non-finite value.
        bound: NonFiniteKind,
    },
    /// A referenced concept could not be resolved by the supplied resolver.
    UnresolvedReference {
        /// Name of the constraint.
        constraint: String,
        /// Reference kind of the unresolved reference.
        kind: String,
        /// Identifier of the unresolved reference.
        name: String,
    },
    /// A structural invariant of the constraint was violated.
    InvalidConstraint {
        /// Name of the constraint.
        constraint: String,
        /// Human-readable description of the violated invariant.
        detail: String,
    },
}

impl Eq for ConstraintError {}

impl fmt::Display for ConstraintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstraintError::InvertedBounds {
                constraint,
                min,
                max,
            } => write!(
                f,
                "constraint {constraint} has inverted bounds: min {min} > max {max}"
            ),
            ConstraintError::NonFiniteBound { constraint, bound } => write!(
                f,
                "constraint {constraint} has a non-finite bound: {bound:?}"
            ),
            ConstraintError::UnresolvedReference {
                constraint,
                kind,
                name,
            } => write!(
                f,
                "constraint {constraint} references unresolved {kind}:{name}"
            ),
            ConstraintError::InvalidConstraint { constraint, detail } => {
                write!(f, "invalid constraint {constraint}: {detail}")
            }
        }
    }
}

impl Error for ConstraintError {}

/// Resolver abstraction used to validate the cross-reference held by a
/// [`Constraint`] against externally owned engineering definitions.
///
/// Higher layers implement this trait over `ParameterSet`, interface
/// collections, behaviour collections, or any composition of them. This
/// keeps `Constraint` decoupled from concrete registries.
pub trait ConstraintReferenceResolver {
    /// Returns `true` if the referenced engineering concept is known.
    fn contains(&self, kind: ReferenceKind, name: &Identifier) -> bool;
}

/// An inclusive engineering boundary on a referenced concept.
///
/// A `Constraint` binds a name, a [`ConstraintReference`], and an
/// inclusive `[min, max]` interval. Both bounds are `f64` values that are
/// guaranteed finite by construction. The constraint holds no runtime
/// state, performs no monitoring, and executes nothing.
///
/// `PartialEq` is derived; `Eq` is implemented manually because the
/// bounds are `f64` values that are guaranteed non-NaN by construction,
/// so equality is a valid equivalence relation.
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    name: Identifier,
    reference: ConstraintReference,
    min: f64,
    max: f64,
}

impl Eq for Constraint {}

impl Constraint {
    /// Creates a new constraint with inclusive `[min, max]` bounds.
    ///
    /// Rejects:
    ///
    /// - `NaN` for either bound;
    /// - `+∞` or `−∞` for either bound;
    /// - `min > max`.
    ///
    /// Bounds are never clamped or reordered.
    pub fn new(
        name: Identifier,
        reference: ConstraintReference,
        min: f64,
        max: f64,
    ) -> Result<Self, ConstraintError> {
        if let Some(kind) = classify_non_finite(min) {
            return Err(ConstraintError::NonFiniteBound {
                constraint: name.as_str().to_owned(),
                bound: kind,
            });
        }

        if let Some(kind) = classify_non_finite(max) {
            return Err(ConstraintError::NonFiniteBound {
                constraint: name.as_str().to_owned(),
                bound: kind,
            });
        }

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

    /// Returns the constraint's name.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the constraint's reference.
    pub fn reference(&self) -> &ConstraintReference {
        &self.reference
    }

    /// Returns the reference kind of the constraint.
    pub fn reference_kind(&self) -> ReferenceKind {
        self.reference.kind()
    }

    /// Returns the identifier of the referenced engineering concept.
    pub fn reference_name(&self) -> &Identifier {
        self.reference.name()
    }

    /// Returns the inclusive minimum bound.
    pub fn min(&self) -> f64 {
        self.min
    }

    /// Returns the inclusive maximum bound.
    pub fn max(&self) -> f64 {
        self.max
    }

    /// Returns the width of the interval (`max - min`).
    pub fn width(&self) -> f64 {
        self.max - self.min
    }

    /// Returns the midpoint of the interval.
    ///
    /// Computed as `min + (max - min) / 2` to avoid overflowing on the
    /// intermediate sum for large engineering magnitudes.
    pub fn midpoint(&self) -> f64 {
        self.min + (self.max - self.min) / 2.0
    }

    /// Returns `true` if the given value lies within the inclusive
    /// interval `[min, max]`.
    ///
    /// `NaN` inputs always return `false`. Infinities follow standard
    /// comparison semantics; construction of the constraint itself has
    /// already rejected non-finite bounds.
    pub fn contains(&self, value: f64) -> bool {
        if value.is_nan() {
            return false;
        }
        self.min <= value && value <= self.max
    }

    /// Returns `true` if the given value lies outside the inclusive
    /// interval.
    ///
    /// This is the logical complement of [`Constraint::contains`]. A
    /// `NaN` input is excluded (i.e. `excludes` returns `true`).
    pub fn excludes(&self, value: f64) -> bool {
        !self.contains(value)
    }

    /// Performs structural validation of the constraint.
    ///
    /// Enforced invariants:
    ///
    /// 1. Both bounds are finite.
    /// 2. `min <= max`.
    ///
    /// These invariants are already guaranteed by [`Constraint::new`];
    /// re-checking them here keeps validation robust against future
    /// construction paths without weakening the constructor.
    pub fn validate(&self) -> Result<(), ConstraintError> {
        if let Some(kind) = classify_non_finite(self.min) {
            return Err(ConstraintError::NonFiniteBound {
                constraint: self.name.as_str().to_owned(),
                bound: kind,
            });
        }

        if let Some(kind) = classify_non_finite(self.max) {
            return Err(ConstraintError::NonFiniteBound {
                constraint: self.name.as_str().to_owned(),
                bound: kind,
            });
        }

        if self.min > self.max {
            return Err(ConstraintError::InvertedBounds {
                constraint: self.name.as_str().to_owned(),
                min: self.min,
                max: self.max,
            });
        }

        Ok(())
    }

    /// Performs cross-reference validation against a caller-supplied
    /// [`ConstraintReferenceResolver`].
    ///
    /// Structural validation runs first, followed by reference resolution.
    /// The first unresolved reference produces
    /// [`ConstraintError::UnresolvedReference`].
    pub fn validate_with<R: ConstraintReferenceResolver>(
        &self,
        resolver: &R,
    ) -> Result<(), ConstraintError> {
        self.validate()?;

        if !resolver.contains(self.reference.kind(), self.reference.name()) {
            return Err(ConstraintError::UnresolvedReference {
                constraint: self.name.as_str().to_owned(),
                kind: format!("{:?}", self.reference.kind()),
                name: self.reference.name().as_str().to_owned(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeSet;

    fn ident(s: &str) -> Identifier {
        Identifier::parse(s).expect("known-valid identifier literal")
    }

    fn temp_ref() -> ConstraintReference {
        ConstraintReference::property(ident("temperature"))
    }

    /// Test resolver backed by a deterministic set of `(kind, name)` entries.
    struct TestResolver {
        known: BTreeSet<(ReferenceKind, Identifier)>,
    }

    impl TestResolver {
        fn new() -> Self {
            Self {
                known: BTreeSet::new(),
            }
        }

        fn insert(&mut self, kind: ReferenceKind, name: &str) {
            self.known.insert((kind, ident(name)));
        }
    }

    impl ConstraintReferenceResolver for TestResolver {
        fn contains(&self, kind: ReferenceKind, name: &Identifier) -> bool {
            self.known.contains(&(kind, name.clone()))
        }
    }

    #[test]
    fn valid_constraint_creation() {
        let c = Constraint::new(ident("motor_temp_limit"), temp_ref(), 0.0, 120.0)
            .expect("valid constraint");
        assert_eq!(c.name().as_str(), "motor_temp_limit");
        assert_eq!(c.min(), 0.0);
        assert_eq!(c.max(), 120.0);
    }

    #[test]
    fn minimum_boundary_is_inclusive() {
        let c = Constraint::new(ident("c"), temp_ref(), 0.0, 120.0).expect("valid");
        assert!(c.contains(0.0));
    }

    #[test]
    fn maximum_boundary_is_inclusive() {
        let c = Constraint::new(ident("c"), temp_ref(), 0.0, 120.0).expect("valid");
        assert!(c.contains(120.0));
    }

    #[test]
    fn value_inside_range() {
        let c = Constraint::new(ident("c"), temp_ref(), 0.0, 120.0).expect("valid");
        assert!(c.contains(60.0));
        assert!(c.contains(0.001));
        assert!(c.contains(119.999));
    }

    #[test]
    fn value_outside_range() {
        let c = Constraint::new(ident("c"), temp_ref(), 0.0, 120.0).expect("valid");
        assert!(!c.contains(-1.0));
        assert!(!c.contains(121.0));
        assert!(!c.contains(1e30));
        assert!(!c.contains(-1e30));
    }

    #[test]
    fn nan_value_rejected_by_contains() {
        let c = Constraint::new(ident("c"), temp_ref(), 0.0, 120.0).expect("valid");
        assert!(!c.contains(f64::NAN));
        assert!(c.excludes(f64::NAN));
    }

    #[test]
    fn inverted_bounds_rejected() {
        let err = Constraint::new(ident("bad"), temp_ref(), 120.0, 0.0)
            .expect_err("inverted bounds rejected");

        assert_eq!(
            err,
            ConstraintError::InvertedBounds {
                constraint: "bad".to_owned(),
                min: 120.0,
                max: 0.0,
            }
        );
    }

    #[test]
    fn nan_minimum_rejected() {
        let err = Constraint::new(ident("bad"), temp_ref(), f64::NAN, 100.0)
            .expect_err("NaN min rejected");

        assert_eq!(
            err,
            ConstraintError::NonFiniteBound {
                constraint: "bad".to_owned(),
                bound: NonFiniteKind::Nan,
            }
        );
    }

    #[test]
    fn nan_maximum_rejected() {
        let err = Constraint::new(ident("bad"), temp_ref(), 0.0, f64::NAN)
            .expect_err("NaN max rejected");

        assert_eq!(
            err,
            ConstraintError::NonFiniteBound {
                constraint: "bad".to_owned(),
                bound: NonFiniteKind::Nan,
            }
        );
    }

    #[test]
    fn positive_infinity_rejected() {
        let err = Constraint::new(ident("bad"), temp_ref(), 0.0, f64::INFINITY)
            .expect_err("+inf rejected");

        assert_eq!(
            err,
            ConstraintError::NonFiniteBound {
                constraint: "bad".to_owned(),
                bound: NonFiniteKind::PositiveInfinity,
            }
        );
    }

    #[test]
    fn negative_infinity_rejected() {
        let err = Constraint::new(ident("bad"), temp_ref(), f64::NEG_INFINITY, 100.0)
            .expect_err("-inf rejected");

        assert_eq!(
            err,
            ConstraintError::NonFiniteBound {
                constraint: "bad".to_owned(),
                bound: NonFiniteKind::NegativeInfinity,
            }
        );
    }

    #[test]
    fn width() {
        let c = Constraint::new(ident("c"), temp_ref(), 10.0, 30.0).expect("valid");
        assert_eq!(c.width(), 20.0);

        let zero = Constraint::new(ident("z"), temp_ref(), 5.0, 5.0).expect("valid");
        assert_eq!(zero.width(), 0.0);
    }

    #[test]
    fn midpoint() {
        let c = Constraint::new(ident("c"), temp_ref(), 10.0, 30.0).expect("valid");
        assert_eq!(c.midpoint(), 20.0);

        let symmetric = Constraint::new(ident("s"), temp_ref(), -10.0, 10.0).expect("valid");
        assert_eq!(symmetric.midpoint(), 0.0);

        let zero = Constraint::new(ident("z"), temp_ref(), 5.0, 5.0).expect("valid");
        assert_eq!(zero.midpoint(), 5.0);
    }

    #[test]
    fn reference_access() {
        let c = Constraint::new(ident("c"), temp_ref(), 0.0, 100.0).expect("valid");
        assert_eq!(c.reference().kind(), ReferenceKind::Property);
        assert_eq!(c.reference_name().as_str(), "temperature");
        assert_eq!(c.reference_kind(), ReferenceKind::Property);
    }

    #[test]
    fn property_reference() {
        let r = ConstraintReference::property(ident("temperature"));
        assert_eq!(r.kind(), ReferenceKind::Property);
        assert_eq!(r.name().as_str(), "temperature");
    }

    #[test]
    fn interface_reference() {
        let r = ConstraintReference::interface(ident("power_input"));
        assert_eq!(r.kind(), ReferenceKind::Interface);
        assert_eq!(r.name().as_str(), "power_input");
    }

    #[test]
    fn behaviour_reference() {
        let r = ConstraintReference::behaviour(ident("thermal_protection"));
        assert_eq!(r.kind(), ReferenceKind::Behaviour);
        assert_eq!(r.name().as_str(), "thermal_protection");
    }

    #[test]
    fn structural_validation() {
        let c = Constraint::new(ident("c"), temp_ref(), 0.0, 100.0).expect("valid");
        assert!(c.validate().is_ok());
    }

    #[test]
    fn resolver_accepts_existing_reference() {
        let mut resolver = TestResolver::new();
        resolver.insert(ReferenceKind::Property, "temperature");

        let c = Constraint::new(ident("c"), temp_ref(), 0.0, 100.0).expect("valid");
        assert!(c.validate_with(&resolver).is_ok());
    }

    #[test]
    fn resolver_rejects_missing_reference() {
        let resolver = TestResolver::new();

        let c = Constraint::new(ident("c"), temp_ref(), 0.0, 100.0).expect("valid");
        let err = c
            .validate_with(&resolver)
            .expect_err("missing reference rejected");

        assert_eq!(
            err,
            ConstraintError::UnresolvedReference {
                constraint: "c".to_owned(),
                kind: "Property".to_owned(),
                name: "temperature".to_owned(),
            }
        );
    }

    #[test]
    fn deterministic_equality() {
        let a = Constraint::new(ident("c"), temp_ref(), 0.0, 100.0).expect("valid");
        let b = Constraint::new(ident("c"), temp_ref(), 0.0, 100.0).expect("valid");
        assert_eq!(a, b);

        let different_bounds =
            Constraint::new(ident("c"), temp_ref(), 0.0, 101.0).expect("valid");
        assert_ne!(a, different_bounds);

        let different_name =
            Constraint::new(ident("d"), temp_ref(), 0.0, 100.0).expect("valid");
        assert_ne!(a, different_name);
    }

    #[test]
    fn excludes_semantics() {
        let c = Constraint::new(ident("c"), temp_ref(), 0.0, 100.0).expect("valid");

        assert!(c.excludes(-1.0));
        assert!(c.excludes(101.0));
        assert!(c.excludes(f64::NAN));

        assert!(!c.excludes(0.0));
        assert!(!c.excludes(50.0));
        assert!(!c.excludes(100.0));
    }

    #[test]
    fn zero_width_constraint_is_valid() {
        let c = Constraint::new(ident("exact"), temp_ref(), 42.0, 42.0).expect("valid");
        assert!(c.validate().is_ok());
        assert!(c.contains(42.0));
        assert!(!c.contains(42.0001));
        assert_eq!(c.width(), 0.0);
    }
}
