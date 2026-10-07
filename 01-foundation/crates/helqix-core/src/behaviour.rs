//! Behaviour contracts for HELQIX Core.
//!
//! A behaviour contract defines the structural dependencies required by an
//! engineering behaviour.
//!
//! This module intentionally does NOT execute behaviour.
//!
//! The responsibility boundary is:
//!
//! ```text
//! Foundation
//!     |
//!     +-- BehaviourContract
//!     |       |
//!     |       +-- named references
//!     |       +-- structural validation
//!     |
//!     v
//! Engineering
//!     |
//!     +-- behaviour semantics
//!     |
//!     v
//! HELIXSIM
//!     |
//!     +-- executable behaviour
//! ```
//!
//! A behaviour may reference:
//!
//! - properties;
//! - interfaces;
//! - other behaviours.
//!
//! References are represented by [`Identifier`] values and resolved later
//! against the appropriate engineering/reality registry.
//!
//! This separation is deliberate. Foundation must describe the engineering
//! contract without depending on runtime state, simulation engines, physics,
//! AI, or external registries.

use std::fmt;

use crate::error::{BehaviourError, ReferenceKind};
use crate::id::Identifier;

/// A single dependency clause inside a behaviour contract.
///
/// A clause states that the behaviour depends on a named member of a
/// particular Foundation category.
///
/// The reference is intentionally unresolved at construction time. This allows
/// behaviour definitions to be assembled before the complete engineering
/// model exists.
///
/// # Example
///
/// A behaviour might reference:
///
/// ```text
/// Property   → speed
/// Property   → temperature
/// Interface  → motor_input
/// Behaviour  → thermal_limit
/// ```
///
/// Resolution against actual declarations is performed separately.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BehaviourClause {
    kind: ReferenceKind,
    name: Identifier,
}

impl BehaviourClause {
    /// Creates a reference clause.
    #[must_use]
    pub const fn new(kind: ReferenceKind, name: Identifier) -> Self {
        Self { kind, name }
    }

    /// Creates a property reference clause.
    #[must_use]
    pub const fn property(name: Identifier) -> Self {
        Self::new(ReferenceKind::Property, name)
    }

    /// Creates an interface reference clause.
    #[must_use]
    pub const fn interface(name: Identifier) -> Self {
        Self::new(ReferenceKind::Interface, name)
    }

    /// Creates a behaviour-to-behaviour reference clause.
    #[must_use]
    pub const fn behaviour(name: Identifier) -> Self {
        Self::new(ReferenceKind::Behaviour, name)
    }

    /// Returns the referenced member kind.
    #[must_use]
    pub const fn kind(&self) -> ReferenceKind {
        self.kind
    }

    /// Returns the referenced member name.
    #[must_use]
    pub const fn name(&self) -> &Identifier {
        &self.name
    }
}

/// A structural contract describing an engineering behaviour.
///
/// [`BehaviourContract`] is immutable in meaning but incrementally constructible.
/// Clauses can be added while the engineering model is being assembled.
///
/// The contract does not contain:
///
/// - executable code;
/// - equations;
/// - runtime state;
/// - time;
/// - forces;
/// - telemetry;
/// - simulation results.
///
/// Those belong to later HELQIX layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviourContract {
    name: Identifier,
    clauses: Vec<BehaviourClause>,
}

impl BehaviourContract {
    /// Creates an empty behaviour contract.
    ///
    /// The contract is structurally incomplete until at least one clause is
    /// added. This is represented by [`BehaviourError::EmptyContract`] during
    /// validation rather than by making construction itself fallible.
    #[must_use]
    pub const fn new(name: Identifier) -> Self {
        Self {
            name,
            clauses: Vec::new(),
        }
    }

    /// Creates a behaviour contract containing the supplied clauses.
    ///
    /// Duplicate clauses are removed while preserving the first occurrence.
    #[must_use]
    pub fn from_clauses(
        name: Identifier,
        clauses: impl IntoIterator<Item = BehaviourClause>,
    ) -> Self {
        let mut contract = Self::new(name);

        for clause in clauses {
            contract.add_clause(clause);
        }

        contract
    }

    /// Returns the behaviour identifier.
    #[must_use]
    pub const fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the number of clauses.
    #[must_use]
    pub fn clause_count(&self) -> usize {
        self.clauses.len()
    }

    /// Returns whether the contract contains no clauses.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.clauses.is_empty()
    }

    /// Returns all clauses in declaration order.
    pub fn clauses(&self) -> impl Iterator<Item = &BehaviourClause> {
        self.clauses.iter()
    }

    /// Adds a clause to the contract.
    ///
    /// Returns `true` when the clause was newly inserted.
    ///
    /// Returns `false` when the exact same `(kind, name)` reference already
    /// exists.
    ///
    /// Duplicate references are therefore prevented without introducing a new
    /// error category that does not exist in the Foundation error vocabulary.
    pub fn add_clause(&mut self, clause: BehaviourClause) -> bool {
        if self
            .clauses
            .iter()
            .any(|existing| existing == &clause)
        {
            return false;
        }

        self.clauses.push(clause);
        true
    }

    /// Removes an exact clause.
    ///
    /// Returns `true` when a clause was removed.
    pub fn remove_clause(&mut self, clause: &BehaviourClause) -> bool {
        let Some(index) = self.clauses.iter().position(|candidate| candidate == clause)
        else {
            return false;
        };

        self.clauses.remove(index);
        true
    }

    /// Returns whether the contract references a particular member.
    #[must_use]
    pub fn references(
        &self,
        kind: ReferenceKind,
        name: &Identifier,
    ) -> bool {
        self.clauses
            .iter()
            .any(|clause| clause.kind() == kind && clause.name() == name)
    }

    /// Returns all clauses belonging to one reference kind.
    pub fn clauses_of_kind(
        &self,
        kind: ReferenceKind,
    ) -> impl Iterator<Item = &BehaviourClause> {
        self.clauses
            .iter()
            .filter(move |clause| clause.kind() == kind)
    }

    /// Performs Foundation-level validation.
    ///
    /// At this level the only universally knowable structural requirement is
    /// that a behaviour contract must contain at least one clause.
    ///
    /// Reference existence cannot be checked here because the referenced
    /// engineering model/registry belongs to later layers.
    pub fn validate(&self) -> Result<(), BehaviourError> {
        if self.is_empty() {
            return Err(BehaviourError::EmptyContract {
                behaviour: self.name.as_str().to_owned(),
            });
        }

        Ok(())
    }

    /// Validates this contract against a caller-provided reference resolver.
    ///
    /// This is the bridge between Foundation semantics and later engineering
    /// registries without coupling this crate to those registries.
    ///
    /// The resolver receives the reference kind and identifier for every
    /// clause. The first unresolved reference is reported deterministically
    /// according to clause declaration order.
    pub fn validate_with<R>(
        &self,
        resolver: &R,
    ) -> Result<(), BehaviourError>
    where
        R: BehaviourReferenceResolver + ?Sized,
    {
        self.validate()?;

        for clause in &self.clauses {
            if !resolver.contains(clause.kind(), clause.name()) {
                return Err(BehaviourError::UnresolvedReference {
                    behaviour: self.name.as_str().to_owned(),
                    kind: clause.kind(),
                    name: clause.name().as_str().to_owned(),
                });
            }
        }

        Ok(())
    }

    /// Returns the first clause of the requested kind.
    #[must_use]
    pub fn first_clause(
        &self,
        kind: ReferenceKind,
    ) -> Option<&BehaviourClause> {
        self.clauses
            .iter()
            .find(|clause| clause.kind() == kind)
    }
}

/// Resolves names referenced by a [`BehaviourContract`].
///
/// The trait intentionally contains only one operation.
///
/// Later HELQIX layers can implement this trait for:
///
/// - component definitions;
/// - engineering models;
/// - reality graphs;
/// - component registries;
/// - project-level model registries.
///
/// This prevents `helqix-core` from depending on those higher layers.
pub trait BehaviourReferenceResolver {
    /// Returns whether the referenced member exists.
    fn contains(&self, kind: ReferenceKind, name: &Identifier) -> bool;
}

impl fmt::Display for BehaviourClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} `{}`", self.kind, self.name)
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
            self.properties.push(
                Identifier::parse(name).expect("test identifier must be valid"),
            );
            self
        }

        fn interface(mut self, name: &str) -> Self {
            self.interfaces.push(
                Identifier::parse(name).expect("test identifier must be valid"),
            );
            self
        }

        fn behaviour(mut self, name: &str) -> Self {
            self.behaviours.push(
                Identifier::parse(name).expect("test identifier must be valid"),
            );
            self
        }
    }

    impl BehaviourReferenceResolver for TestResolver {
        fn contains(
            &self,
            kind: ReferenceKind,
            name: &Identifier,
        ) -> bool {
            match kind {
                ReferenceKind::Property => self.properties.iter().any(|id| id == name),
                ReferenceKind::Interface => self.interfaces.iter().any(|id| id == name),
                ReferenceKind::Behaviour => self.behaviours.iter().any(|id| id == name),
            }
        }
    }

    fn id(value: &str) -> Identifier {
        Identifier::parse(value).expect("test identifier must be valid")
    }

    #[test]
    fn empty_contract_is_structurally_invalid() {
        let contract = BehaviourContract::new(id("start"));

        assert_eq!(
            contract.validate(),
            Err(BehaviourError::EmptyContract {
                behaviour: "start".to_owned(),
            })
        );
    }

    #[test]
    fn contract_with_one_clause_is_valid() {
        let mut contract = BehaviourContract::new(id("start"));

        assert!(contract.add_clause(BehaviourClause::property(id("speed"))));
        assert!(contract.validate().is_ok());
    }

    #[test]
    fn property_clause_has_correct_kind_and_name() {
        let clause = BehaviourClause::property(id("speed"));

        assert_eq!(clause.kind(), ReferenceKind::Property);
        assert_eq!(clause.name().as_str(), "speed");
    }

    #[test]
    fn interface_clause_has_correct_kind_and_name() {
        let clause = BehaviourClause::interface(id("motor_input"));

        assert_eq!(clause.kind(), ReferenceKind::Interface);
        assert_eq!(clause.name().as_str(), "motor_input");
    }

    #[test]
    fn behaviour_clause_has_correct_kind_and_name() {
        let clause = BehaviourClause::behaviour(id("thermal_limit"));

        assert_eq!(clause.kind(), ReferenceKind::Behaviour);
        assert_eq!(clause.name().as_str(), "thermal_limit");
    }

    #[test]
    fn duplicate_clause_is_not_inserted() {
        let mut contract = BehaviourContract::new(id("start"));

        let clause = BehaviourClause::property(id("speed"));

        assert!(contract.add_clause(clause.clone()));
        assert!(!contract.add_clause(clause));

        assert_eq!(contract.clause_count(), 1);
    }

    #[test]
    fn same_name_with_different_reference_kind_is_allowed() {
        let mut contract = BehaviourContract::new(id("start"));

        assert!(contract.add_clause(BehaviourClause::property(id("input"))));
        assert!(contract.add_clause(BehaviourClause::interface(id("input"))));
        assert!(contract.add_clause(BehaviourClause::behaviour(id("input"))));

        assert_eq!(contract.clause_count(), 3);
    }

    #[test]
    fn references_checks_kind_and_name_together() {
        let mut contract = BehaviourContract::new(id("start"));

        contract.add_clause(BehaviourClause::property(id("speed")));

        assert!(contract.references(
            ReferenceKind::Property,
            &id("speed")
        ));

        assert!(!contract.references(
            ReferenceKind::Interface,
            &id("speed")
        ));

        assert!(!contract.references(
            ReferenceKind::Property,
            &id("temperature")
        ));
    }

    #[test]
    fn clauses_of_kind_filters_without_mutating_contract() {
        let mut contract = BehaviourContract::new(id("start"));

        contract.add_clause(BehaviourClause::property(id("speed")));
        contract.add_clause(BehaviourClause::interface(id("motor_input")));
        contract.add_clause(BehaviourClause::property(id("temperature")));

        let properties = contract
            .clauses_of_kind(ReferenceKind::Property)
            .map(|clause| clause.name().as_str())
            .collect::<Vec<_>>();

        assert_eq!(properties, vec!["speed", "temperature"]);
        assert_eq!(contract.clause_count(), 3);
    }

    #[test]
    fn first_clause_returns_first_matching_reference() {
        let mut contract = BehaviourContract::new(id("start"));

        contract.add_clause(BehaviourClause::property(id("speed")));
        contract.add_clause(BehaviourClause::interface(id("motor_input")));
        contract.add_clause(BehaviourClause::property(id("temperature")));

        let first_property = contract
            .first_clause(ReferenceKind::Property)
            .expect("property clause must exist");

        assert_eq!(first_property.name().as_str(), "speed");
    }

    #[test]
    fn remove_clause_removes_only_requested_clause() {
        let mut contract = BehaviourContract::new(id("start"));

        let speed = BehaviourClause::property(id("speed"));
        let temperature = BehaviourClause::property(id("temperature"));

        contract.add_clause(speed.clone());
        contract.add_clause(temperature.clone());

        assert!(contract.remove_clause(&speed));
        assert!(!contract.references(
            ReferenceKind::Property,
            &id("speed")
        ));

        assert!(contract.references(
            ReferenceKind::Property,
            &id("temperature")
        ));

        assert!(!contract.remove_clause(&speed));
    }

    #[test]
    fn resolver_accepts_all_declared_references() {
        let mut contract = BehaviourContract::new(id("start"));

        contract.add_clause(BehaviourClause::property(id("speed")));
        contract.add_clause(BehaviourClause::property(id("temperature")));
        contract.add_clause(BehaviourClause::interface(id("motor_input")));
        contract.add_clause(BehaviourClause::behaviour(id("thermal_limit")));

        let resolver = TestResolver::new()
            .property("speed")
            .property("temperature")
            .interface("motor_input")
            .behaviour("thermal_limit");

        assert!(contract.validate_with(&resolver).is_ok());
    }

    #[test]
    fn resolver_reports_unresolved_property() {
        let mut contract = BehaviourContract::new(id("start"));

        contract.add_clause(BehaviourClause::property(id("speed")));

        let resolver = TestResolver::new();

        assert_eq!(
            contract.validate_with(&resolver),
            Err(BehaviourError::UnresolvedReference {
                behaviour: "start".to_owned(),
                kind: ReferenceKind::Property,
                name: "speed".to_owned(),
            })
        );
    }

    #[test]
    fn resolver_reports_unresolved_interface() {
        let mut contract = BehaviourContract::new(id("start"));

        contract.add_clause(BehaviourClause::interface(id("motor_input")));

        let resolver = TestResolver::new();

        assert_eq!(
            contract.validate_with(&resolver),
            Err(BehaviourError::UnresolvedReference {
                behaviour: "start".to_owned(),
                kind: ReferenceKind::Interface,
                name: "motor_input".to_owned(),
            })
        );
    }

    #[test]
    fn resolver_reports_unresolved_behaviour() {
        let mut contract = BehaviourContract::new(id("start"));

        contract.add_clause(BehaviourClause::behaviour(id("thermal_limit")));

        let resolver = TestResolver::new();

        assert_eq!(
            contract.validate_with(&resolver),
            Err(BehaviourError::UnresolvedReference {
                behaviour: "start".to_owned(),
                kind: ReferenceKind::Behaviour,
                name: "thermal_limit".to_owned(),
            })
        );
    }

    #[test]
    fn resolver_reports_first_unresolved_clause_deterministically() {
        let mut contract = BehaviourContract::new(id("start"));

        contract.add_clause(BehaviourClause::property(id("speed")));
        contract.add_clause(BehaviourClause::interface(id("motor_input")));
        contract.add_clause(BehaviourClause::property(id("temperature")));

        let resolver = TestResolver::new();

        let error = contract
            .validate_with(&resolver)
            .expect_err("references must not resolve");

        assert_eq!(
            error,
            BehaviourError::UnresolvedReference {
                behaviour: "start".to_owned(),
                kind: ReferenceKind::Property,
                name: "speed".to_owned(),
            }
        );
    }

    #[test]
    fn from_clauses_removes_exact_duplicates() {
        let contract = BehaviourContract::from_clauses(
            id("start"),
            [
                BehaviourClause::property(id("speed")),
                BehaviourClause::property(id("speed")),
                BehaviourClause::interface(id("motor_input")),
            ],
        );

        assert_eq!(contract.clause_count(), 2);
    }

    #[test]
    fn clause_display_is_structured() {
        let property = BehaviourClause::property(id("speed"));

        assert_eq!(property.to_string(), "property `speed`");

        let interface = BehaviourClause::interface(id("motor_input"));

        assert_eq!(interface.to_string(), "interface `motor_input`");
    }

    #[test]
    fn contract_preserves_declaration_order() {
        let contract = BehaviourContract::from_clauses(
            id("start"),
            [
                BehaviourClause::property(id("temperature")),
                BehaviourClause::property(id("speed")),
                BehaviourClause::interface(id("motor_input")),
            ],
        );

        let clauses = contract
            .clauses()
            .map(|clause| clause.name().as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            clauses,
            vec!["temperature", "speed", "motor_input"]
        );
    }

    #[test]
    fn validation_does_not_require_a_resolver_for_structural_validation() {
        let mut contract = BehaviourContract::new(id("start"));

        contract.add_clause(BehaviourClause::property(id("speed")));

        assert!(contract.validate().is_ok());
    }

    #[test]
    fn empty_contract_fails_before_reference_resolution() {
        let contract = BehaviourContract::new(id("start"));
        let resolver = TestResolver::new();

        assert_eq!(
            contract.validate_with(&resolver),
            Err(BehaviourError::EmptyContract {
                behaviour: "start".to_owned(),
            })
        );
    }
}