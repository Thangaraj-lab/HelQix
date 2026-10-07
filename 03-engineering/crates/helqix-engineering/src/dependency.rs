//! Structural engineering dependencies for HELQIX Layer 03 — Engineering.
//!
//! A [`Dependency`] represents a directional engineering relationship
//! between two engineering definitions:
//!
//! ```text
//! source  →  target
//! ```
//!
//! Example:
//!
//! ```text
//! property:payload_mass   →   property:required_thrust
//! ```
//!
//! meaning *"required_thrust depends on payload_mass"* in an engineering
//! definitional sense.
//!
//! # Structural vs. runtime causality
//!
//! A `Dependency` is a **structural engineering relationship**. It does
//! not represent a runtime event, a state transition, an execution step,
//! a failure, an observation, a timestamp, or a physics calculation.
//!
//! Runtime causality — inferring *why* something actually happened during
//! execution — is derived in later HELQIX layers from observed state
//! transitions combined with the structural dependency relations defined
//! here. Keeping these two notions strictly separate is fundamental to
//! HELQIX.
//!
//! This file therefore contains:
//!
//! - a directional reference type ([`DependencyReference`]);
//! - a single directional dependency ([`Dependency`]);
//! - local structural validation;
//! - an optional cross-reference resolver abstraction
//!   ([`DependencyReferenceResolver`]).
//!
//! It does **not** contain graph algorithms, cycle detection, topological
//! sorting, propagation, impact analysis, or runtime causal records.
//! Those belong to separate files and later layers.

use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;
use helqix_core::ReferenceKind;

/// An endpoint of a [`Dependency`].
///
/// A reference binds a [`ReferenceKind`] (property, interface, behaviour)
/// to an [`Identifier`]. It carries no runtime value and performs no
/// resolution by itself.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DependencyReference {
    kind: ReferenceKind,
    name: Identifier,
}

impl DependencyReference {
    /// Creates a new dependency reference.
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

    /// Creates a reference to an engineering behaviour.
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

impl fmt::Display for DependencyReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}:{}", self.kind, self.name.as_str())
    }
}

/// Errors produced while constructing or validating a [`Dependency`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyError {
    /// The dependency's source and target are identical.
    SelfDependency {
        /// Name of the dependency.
        dependency: String,
        /// Rendered reference that appeared on both sides.
        reference: String,
    },
    /// A structural invariant of the dependency was violated.
    InvalidDependency {
        /// Name of the dependency.
        dependency: String,
        /// Human-readable description of the violated invariant.
        detail: String,
    },
    /// A referenced concept could not be resolved by the supplied resolver.
    UnresolvedReference {
        /// Name of the dependency.
        dependency: String,
        /// Reference kind of the unresolved endpoint.
        kind: String,
        /// Identifier of the unresolved endpoint.
        name: String,
    },
}

impl fmt::Display for DependencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependencyError::SelfDependency {
                dependency,
                reference,
            } => write!(
                f,
                "dependency {dependency} is a self-dependency on {reference}"
            ),
            DependencyError::InvalidDependency { dependency, detail } => {
                write!(f, "invalid dependency {dependency}: {detail}")
            }
            DependencyError::UnresolvedReference {
                dependency,
                kind,
                name,
            } => write!(
                f,
                "dependency {dependency} references unresolved {kind}:{name}"
            ),
        }
    }
}

impl Error for DependencyError {}

/// Resolver abstraction used to validate the cross-references held by a
/// [`Dependency`] against externally owned engineering definitions.
///
/// Higher layers implement this trait over parameter sets, interface
/// collections, behaviour collections, or any composition of them.
/// `Dependency` itself never depends on a concrete registry.
pub trait DependencyReferenceResolver {
    /// Returns `true` if the given engineering concept is known.
    fn contains(&self, kind: ReferenceKind, name: &Identifier) -> bool;
}

/// A single directional structural engineering dependency.
///
/// A `Dependency` binds a name, a `source` endpoint, and a `target`
/// endpoint. It reads as:
///
/// ```text
/// source  →  target
/// ```
///
/// The direction is explicit and preserved. A dependency never contains
/// runtime data, numerical values, timestamps, or state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    name: Identifier,
    source: DependencyReference,
    target: DependencyReference,
}

impl Dependency {
    /// Creates a new directional dependency.
    ///
    /// Rejects a dependency whose source and target are identical; such
    /// a relationship is not a meaningful engineering dependency and
    /// would corrupt downstream graph analysis. Direction is never
    /// inferred, swapped, or silently repaired.
    pub fn new(
        name: Identifier,
        source: DependencyReference,
        target: DependencyReference,
    ) -> Result<Self, DependencyError> {
        if source == target {
            return Err(DependencyError::SelfDependency {
                dependency: name.as_str().to_owned(),
                reference: source.to_string(),
            });
        }

        Ok(Self {
            name,
            source,
            target,
        })
    }

    /// Returns the dependency's name.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the source endpoint (`source → target`).
    pub fn source(&self) -> &DependencyReference {
        &self.source
    }

    /// Returns the target endpoint (`source → target`).
    pub fn target(&self) -> &DependencyReference {
        &self.target
    }

    /// Returns the source endpoint's [`ReferenceKind`].
    pub fn source_kind(&self) -> ReferenceKind {
        self.source.kind()
    }

    /// Returns the source endpoint's [`Identifier`].
    pub fn source_name(&self) -> &Identifier {
        self.source.name()
    }

    /// Returns the target endpoint's [`ReferenceKind`].
    pub fn target_kind(&self) -> ReferenceKind {
        self.target.kind()
    }

    /// Returns the target endpoint's [`Identifier`].
    pub fn target_name(&self) -> &Identifier {
        self.target.name()
    }

    /// Returns `true` if this dependency is a self-dependency.
    ///
    /// A `Dependency` constructed through [`Dependency::new`] is never a
    /// self-dependency; this accessor is provided so callers can inspect
    /// dependencies obtained from higher layers or external sources.
    pub fn is_self_dependency(&self) -> bool {
        self.source == self.target
    }

    /// Performs structural validation of the dependency.
    ///
    /// Enforced invariants:
    ///
    /// 1. The source and target are not identical.
    ///
    /// No cross-reference resolution is performed. Endpoint existence is
    /// checked by [`Dependency::validate_with`].
    pub fn validate(&self) -> Result<(), DependencyError> {
        if self.source == self.target {
            return Err(DependencyError::SelfDependency {
                dependency: self.name.as_str().to_owned(),
                reference: self.source.to_string(),
            });
        }
        Ok(())
    }

    /// Performs cross-reference validation of both endpoints against a
    /// caller-supplied [`DependencyReferenceResolver`].
    ///
    /// Structural validation is performed first, then both the source
    /// and target are checked. The first unresolved endpoint produces
    /// [`DependencyError::UnresolvedReference`]. No automatic repair is
    /// performed and the dependency is never mutated.
    pub fn validate_with<R: DependencyReferenceResolver>(
        &self,
        resolver: &R,
    ) -> Result<(), DependencyError> {
        self.validate()?;

        if !resolver.contains(self.source.kind(), self.source.name()) {
            return Err(DependencyError::UnresolvedReference {
                dependency: self.name.as_str().to_owned(),
                kind: format!("{:?}", self.source.kind()),
                name: self.source.name().as_str().to_owned(),
            });
        }

        if !resolver.contains(self.target.kind(), self.target.name()) {
            return Err(DependencyError::UnresolvedReference {
                dependency: self.name.as_str().to_owned(),
                kind: format!("{:?}", self.target.kind()),
                name: self.target.name().as_str().to_owned(),
            });
        }

        Ok(())
    }
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} -> {}",
            self.name.as_str(),
            self.source,
            self.target
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeSet;

    fn ident(s: &str) -> Identifier {
        Identifier::parse(s).expect("known-valid identifier literal")
    }

    fn prop(name: &str) -> DependencyReference {
        DependencyReference::property(ident(name))
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

    impl DependencyReferenceResolver for TestResolver {
        fn contains(&self, kind: ReferenceKind, name: &Identifier) -> bool {
            self.known.contains(&(kind, name.clone()))
        }
    }

    #[test]
    fn create_property_dependency() {
        let dep = Dependency::new(
            ident("thrust_dependency"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid dependency");

        assert_eq!(dep.name().as_str(), "thrust_dependency");
        assert_eq!(dep.source_kind(), ReferenceKind::Property);
        assert_eq!(dep.target_kind(), ReferenceKind::Property);
    }

    #[test]
    fn create_interface_dependency() {
        let dep = Dependency::new(
            ident("power_dependency"),
            DependencyReference::interface(ident("power_output")),
            DependencyReference::interface(ident("power_input")),
        )
        .expect("valid dependency");

        assert_eq!(dep.source_kind(), ReferenceKind::Interface);
        assert_eq!(dep.target_kind(), ReferenceKind::Interface);
    }

    #[test]
    fn create_behaviour_dependency() {
        let dep = Dependency::new(
            ident("protection_dependency"),
            DependencyReference::behaviour(ident("thermal_protection")),
            DependencyReference::behaviour(ident("overcurrent_protection")),
        )
        .expect("valid dependency");

        assert_eq!(dep.source_kind(), ReferenceKind::Behaviour);
        assert_eq!(dep.target_kind(), ReferenceKind::Behaviour);
    }

    #[test]
    fn source_accessor() {
        let source = prop("payload_mass");
        let dep = Dependency::new(
            ident("d"),
            source.clone(),
            prop("required_thrust"),
        )
        .expect("valid");

        assert_eq!(dep.source(), &source);
    }

    #[test]
    fn target_accessor() {
        let target = prop("required_thrust");
        let dep = Dependency::new(ident("d"), prop("payload_mass"), target.clone())
            .expect("valid");

        assert_eq!(dep.target(), &target);
    }

    #[test]
    fn source_kind_accessor() {
        let dep = Dependency::new(
            ident("d"),
            DependencyReference::interface(ident("power_output")),
            prop("available_power"),
        )
        .expect("valid");

        assert_eq!(dep.source_kind(), ReferenceKind::Interface);
    }

    #[test]
    fn target_kind_accessor() {
        let dep = Dependency::new(
            ident("d"),
            prop("available_power"),
            DependencyReference::behaviour(ident("power_protection")),
        )
        .expect("valid");

        assert_eq!(dep.target_kind(), ReferenceKind::Behaviour);
    }

    #[test]
    fn source_name_accessor() {
        let dep = Dependency::new(
            ident("d"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        assert_eq!(dep.source_name().as_str(), "payload_mass");
    }

    #[test]
    fn target_name_accessor() {
        let dep = Dependency::new(
            ident("d"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        assert_eq!(dep.target_name().as_str(), "required_thrust");
    }

    #[test]
    fn dependency_name_accessor() {
        let dep = Dependency::new(
            ident("thrust_dependency"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        assert_eq!(dep.name().as_str(), "thrust_dependency");
    }

    #[test]
    fn self_dependency_rejected() {
        let err = Dependency::new(
            ident("self_dep"),
            prop("temperature"),
            prop("temperature"),
        )
        .expect_err("self dependency rejected");

        assert_eq!(
            err,
            DependencyError::SelfDependency {
                dependency: "self_dep".to_owned(),
                reference: "Property:temperature".to_owned(),
            }
        );
    }

    #[test]
    fn self_dependency_detection() {
        // A dependency constructed through the public API is never a
        // self-dependency.
        let dep = Dependency::new(
            ident("d"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        assert!(!dep.is_self_dependency());
    }

    #[test]
    fn property_to_property_dependency() {
        let dep = Dependency::new(
            ident("d"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        assert_eq!(dep.source_kind(), ReferenceKind::Property);
        assert_eq!(dep.target_kind(), ReferenceKind::Property);
    }

    #[test]
    fn property_to_interface_dependency() {
        let dep = Dependency::new(
            ident("d"),
            prop("required_power"),
            DependencyReference::interface(ident("power_input")),
        )
        .expect("valid");

        assert_eq!(dep.source_kind(), ReferenceKind::Property);
        assert_eq!(dep.target_kind(), ReferenceKind::Interface);
    }

    #[test]
    fn interface_to_behaviour_dependency() {
        let dep = Dependency::new(
            ident("d"),
            DependencyReference::interface(ident("power_input")),
            DependencyReference::behaviour(ident("overcurrent_protection")),
        )
        .expect("valid");

        assert_eq!(dep.source_kind(), ReferenceKind::Interface);
        assert_eq!(dep.target_kind(), ReferenceKind::Behaviour);
    }

    #[test]
    fn behaviour_to_property_dependency() {
        let dep = Dependency::new(
            ident("d"),
            DependencyReference::behaviour(ident("thermal_protection")),
            prop("motor_temperature"),
        )
        .expect("valid");

        assert_eq!(dep.source_kind(), ReferenceKind::Behaviour);
        assert_eq!(dep.target_kind(), ReferenceKind::Property);
    }

    #[test]
    fn structural_validation_succeeds() {
        let dep = Dependency::new(
            ident("d"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        assert!(dep.validate().is_ok());
    }

    #[test]
    fn resolver_accepts_valid_source_and_target() {
        let mut resolver = TestResolver::new();
        resolver.insert(ReferenceKind::Property, "payload_mass");
        resolver.insert(ReferenceKind::Property, "required_thrust");

        let dep = Dependency::new(
            ident("d"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        assert!(dep.validate_with(&resolver).is_ok());
    }

    #[test]
    fn resolver_rejects_missing_source() {
        let mut resolver = TestResolver::new();
        resolver.insert(ReferenceKind::Property, "required_thrust");

        let dep = Dependency::new(
            ident("d"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        let err = dep
            .validate_with(&resolver)
            .expect_err("missing source rejected");

        assert_eq!(
            err,
            DependencyError::UnresolvedReference {
                dependency: "d".to_owned(),
                kind: "Property".to_owned(),
                name: "payload_mass".to_owned(),
            }
        );
    }

    #[test]
    fn resolver_rejects_missing_target() {
        let mut resolver = TestResolver::new();
        resolver.insert(ReferenceKind::Property, "payload_mass");

        let dep = Dependency::new(
            ident("d"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        let err = dep
            .validate_with(&resolver)
            .expect_err("missing target rejected");

        assert_eq!(
            err,
            DependencyError::UnresolvedReference {
                dependency: "d".to_owned(),
                kind: "Property".to_owned(),
                name: "required_thrust".to_owned(),
            }
        );
    }

    #[test]
    fn deterministic_equality() {
        let a = Dependency::new(
            ident("d"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        let b = Dependency::new(
            ident("d"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        assert_eq!(a, b);

        let reversed = Dependency::new(
            ident("d"),
            prop("required_thrust"),
            prop("payload_mass"),
        )
        .expect("valid");

        assert_ne!(a, reversed);
    }

    #[test]
    fn deterministic_display() {
        let dep = Dependency::new(
            ident("thrust_dependency"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        assert_eq!(
            dep.to_string(),
            "thrust_dependency: Property:payload_mass -> Property:required_thrust"
        );

        let reference = DependencyReference::interface(ident("power_input"));
        assert_eq!(reference.to_string(), "Interface:power_input");

        let behaviour_ref = DependencyReference::behaviour(ident("thermal_protection"));
        assert_eq!(behaviour_ref.to_string(), "Behaviour:thermal_protection");
    }

    #[test]
    fn source_direction_is_preserved() {
        let dep = Dependency::new(
            ident("d"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        assert_eq!(dep.source_name().as_str(), "payload_mass");
        assert_eq!(dep.target_name().as_str(), "required_thrust");
    }

    #[test]
    fn target_direction_is_preserved() {
        let dep = Dependency::new(
            ident("d"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        // The direction must not be silently reversed.
        assert_ne!(dep.source_name(), dep.target_name());
        assert_eq!(dep.source_name().as_str(), "payload_mass");
        assert_eq!(dep.target_name().as_str(), "required_thrust");
    }

    #[test]
    fn dependency_does_not_mutate_after_validation() {
        let mut resolver = TestResolver::new();
        resolver.insert(ReferenceKind::Property, "payload_mass");
        resolver.insert(ReferenceKind::Property, "required_thrust");

        let dep = Dependency::new(
            ident("d"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");

        let snapshot_source = dep.source().clone();
        let snapshot_target = dep.target().clone();

        assert!(dep.validate().is_ok());
        assert!(dep.validate_with(&resolver).is_ok());

        // No mutation occurred during validation.
        assert_eq!(dep.source(), &snapshot_source);
        assert_eq!(dep.target(), &snapshot_target);
    }
}