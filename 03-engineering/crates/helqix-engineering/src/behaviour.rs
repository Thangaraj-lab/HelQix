//! Engineering behaviour definitions for HELQIX Layer 03 — Engineering.
//!
//! A [`Behaviour`] describes an engineering contract: a named set of
//! [`BehaviourClause`]s, each of which structurally references another
//! engineering concept (a property, an interface, or another behaviour).
//!
//! A behaviour is a *definition*. It is not runtime execution, not a
//! simulation, and not state. It contains no numerical values, no
//! timestamps, no timesteps, no physics calculations, no AI decisions,
//! and no quantum algorithms.
//!
//! Structural validation ([`Behaviour::validate`]) and cross-reference
//! validation ([`Behaviour::validate_with`]) are deliberately separated:
//!
//! - Structural validation ensures the behaviour itself is well-formed
//!   (for example: it declares at least one clause).
//! - Cross-reference validation, driven by a caller-supplied
//!   [`BehaviourReferenceResolver`], verifies that the referenced
//!   concepts actually exist. This keeps `Behaviour` decoupled from any
//!   concrete registry (`ParameterSet`, interface collections, other
//!   `Behaviour` collections, Reality, and so on).

use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

/// Kind of engineering concept referenced by a [`BehaviourClause`].
///
/// This is the Layer 03 reference taxonomy for behaviour contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReferenceKind {
    /// A reference to an engineering property (parameter).
    Property,
    /// A reference to an engineering interface.
    Interface,
    /// A reference to another engineering behaviour.
    Behaviour,
}

impl fmt::Display for ReferenceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReferenceKind::Property => f.write_str("property"),
            ReferenceKind::Interface => f.write_str("interface"),
            ReferenceKind::Behaviour => f.write_str("behaviour"),
        }
    }
}

/// A single structural reference inside a [`Behaviour`].
///
/// A clause binds a [`ReferenceKind`] to an [`Identifier`] naming the
/// referenced engineering concept. It carries no value and performs no
/// resolution by itself.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BehaviourClause {
    kind: ReferenceKind,
    name: Identifier,
}

impl BehaviourClause {
    /// Creates a new clause referencing the given engineering concept.
    pub fn new(kind: ReferenceKind, name: Identifier) -> Self {
        Self { kind, name }
    }

    /// Creates a clause referencing an engineering property.
    pub fn property(name: Identifier) -> Self {
        Self::new(ReferenceKind::Property, name)
    }

    /// Creates a clause referencing an engineering interface.
    pub fn interface(name: Identifier) -> Self {
        Self::new(ReferenceKind::Interface, name)
    }

    /// Creates a clause referencing another engineering behaviour.
    pub fn behaviour(name: Identifier) -> Self {
        Self::new(ReferenceKind::Behaviour, name)
    }

    /// Returns the clause's reference kind.
    pub fn kind(&self) -> ReferenceKind {
        self.kind
    }

    /// Returns the referenced concept's identifier.
    pub fn name(&self) -> &Identifier {
        &self.name
    }
}

impl fmt::Display for BehaviourClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.name.as_str())
    }
}

/// Errors produced while constructing or validating a [`Behaviour`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BehaviourError {
    /// The behaviour declares no clauses and is structurally invalid.
    EmptyContract {
        /// Name of the behaviour.
        behaviour: String,
    },
    /// The same clause (kind + identifier) was added more than once.
    DuplicateClause {
        /// Name of the behaviour.
        behaviour: String,
        /// Reference kind of the duplicated clause.
        kind: String,
        /// Identifier of the duplicated clause.
        name: String,
    },
    /// A clause could not be resolved by the supplied resolver.
    UnresolvedReference {
        /// Name of the behaviour.
        behaviour: String,
        /// Reference kind of the unresolved clause.
        kind: String,
        /// Identifier of the unresolved clause.
        name: String,
    },
    /// A structural invariant of the behaviour was violated.
    InvalidBehaviour {
        /// Name of the behaviour.
        behaviour: String,
        /// Human-readable description of the violated invariant.
        detail: String,
    },
}

impl fmt::Display for BehaviourError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BehaviourError::EmptyContract { behaviour } => {
                write!(f, "behaviour {behaviour} declares no clauses")
            }
            BehaviourError::DuplicateClause {
                behaviour,
                kind,
                name,
            } => write!(
                f,
                "behaviour {behaviour} declares clause {kind}:{name} more than once"
            ),
            BehaviourError::UnresolvedReference {
                behaviour,
                kind,
                name,
            } => write!(
                f,
                "behaviour {behaviour} references unresolved {kind}:{name}"
            ),
            BehaviourError::InvalidBehaviour { behaviour, detail } => {
                write!(f, "invalid behaviour {behaviour}: {detail}")
            }
        }
    }
}

impl Error for BehaviourError {}

/// Resolver abstraction used to validate cross-references of a
/// [`Behaviour`] against externally held engineering definitions.
///
/// Implementors answer a single structural question: does a definition
/// of the given [`ReferenceKind`] exist under the given [`Identifier`]?
///
/// This keeps `Behaviour` independent from any concrete registry. Higher
/// layers can implement this trait over `ParameterSet`, interface
/// collections, other `Behaviour` collections, or any composition of
/// them.
pub trait BehaviourReferenceResolver {
    /// Returns `true` if the given engineering concept is known to exist.
    fn contains(&self, kind: ReferenceKind, name: &Identifier) -> bool;
}

/// An engineering behaviour contract.
///
/// A behaviour is a named, ordered list of [`BehaviourClause`]s. Clause
/// order is the insertion order; determinism comes from the fact that
/// clauses are appended to a [`Vec`] and never reordered.
///
/// A behaviour is structurally valid only when it declares at least one
/// clause. Reference resolution is a separate concern, exposed through
/// [`Behaviour::validate_with`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Behaviour {
    name: Identifier,
    clauses: Vec<BehaviourClause>,
}

impl Behaviour {
    /// Creates an empty behaviour with the given name.
    ///
    /// The behaviour is not yet structurally valid; callers must add at
    /// least one clause and then call [`Behaviour::validate`].
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            clauses: Vec::new(),
        }
    }

    /// Creates a behaviour from a list of clauses.
    ///
    /// Duplicate clauses are rejected; each clause is added through
    /// [`Behaviour::add_clause`]. Structural validity (non-empty clause
    /// set) is *not* enforced here — call [`Behaviour::validate`].
    pub fn from_clauses(
        name: Identifier,
        clauses: Vec<BehaviourClause>,
    ) -> Result<Self, BehaviourError> {
        let mut behaviour = Self::new(name);
        for clause in clauses {
            behaviour.add_clause(clause)?;
        }
        Ok(behaviour)
    }

    /// Returns the behaviour's name.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the number of clauses declared by the behaviour.
    pub fn clause_count(&self) -> usize {
        self.clauses.len()
    }

    /// Returns `true` if the behaviour declares no clauses.
    pub fn is_empty(&self) -> bool {
        self.clauses.is_empty()
    }

    /// Iterates over the behaviour's clauses in insertion order.
    pub fn clauses(&self) -> impl Iterator<Item = &BehaviourClause> {
        self.clauses.iter()
    }

    /// Adds a clause to the behaviour.
    ///
    /// Duplicate clauses — those matching on both [`ReferenceKind`] and
    /// [`Identifier`] — are rejected with
    /// [`BehaviourError::DuplicateClause`]. Existing clauses are never
    /// overwritten.
    pub fn add_clause(&mut self, clause: BehaviourClause) -> Result<(), BehaviourError> {
        let duplicate = self
            .clauses
            .iter()
            .any(|existing| existing.kind() == clause.kind() && existing.name() == clause.name());

        if duplicate {
            return Err(BehaviourError::DuplicateClause {
                behaviour: self.name.as_str().to_owned(),
                kind: clause.kind().to_string(),
                name: clause.name().as_str().to_owned(),
            });
        }

        self.clauses.push(clause);
        Ok(())
    }

    /// Removes and returns the clause matching the given kind and name,
    /// if present.
    pub fn remove_clause(
        &mut self,
        kind: ReferenceKind,
        name: &Identifier,
    ) -> Option<BehaviourClause> {
        let position = self
            .clauses
            .iter()
            .position(|clause| clause.kind() == kind && clause.name() == name)?;
        Some(self.clauses.remove(position))
    }

    /// Iterates over all clauses as engineering references, in insertion
    /// order.
    ///
    /// This is semantically equivalent to [`Behaviour::clauses`] and is
    /// provided to make the "referenced concepts" intent explicit at
    /// call sites.
    pub fn references(&self) -> impl Iterator<Item = &BehaviourClause> {
        self.clauses.iter()
    }

    /// Iterates over only the clauses of the given [`ReferenceKind`],
    /// preserving insertion order.
    pub fn clauses_of_kind(
        &self,
        kind: ReferenceKind,
    ) -> impl Iterator<Item = &BehaviourClause> {
        self.clauses.iter().filter(move |clause| clause.kind() == kind)
    }

    /// Returns the first clause, if any.
    pub fn first_clause(&self) -> Option<&BehaviourClause> {
        self.clauses.first()
    }

    /// Performs structural validation of the behaviour.
    ///
    /// Enforced invariants:
    ///
    /// 1. The behaviour declares at least one clause.
    ///
    /// No cross-reference resolution is performed. Reference existence
    /// is checked by [`Behaviour::validate_with`].
    pub fn validate(&self) -> Result<(), BehaviourError> {
        if self.clauses.is_empty() {
            return Err(BehaviourError::EmptyContract {
                behaviour: self.name.as_str().to_owned(),
            });
        }
        Ok(())
    }

    /// Performs cross-reference validation of the behaviour against a
    /// caller-supplied [`BehaviourReferenceResolver`].
    ///
    /// Structural validation is performed first. Then each clause is
    /// checked against the resolver; the first unresolved clause
    /// produces [`BehaviourError::UnresolvedReference`].
    ///
    /// No concrete registry is assumed. Higher layers are responsible
    /// for supplying a resolver over their own collections.
    pub fn validate_with<R: BehaviourReferenceResolver>(
        &self,
        resolver: &R,
    ) -> Result<(), BehaviourError> {
        self.validate()?;

        for clause in &self.clauses {
            if !resolver.contains(clause.kind(), clause.name()) {
                return Err(BehaviourError::UnresolvedReference {
                    behaviour: self.name.as_str().to_owned(),
                    kind: clause.kind().to_string(),
                    name: clause.name().as_str().to_owned(),
                });
            }
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

    /// Test resolver backed by a deterministic set of `(kind, name)`
    /// entries.
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

    impl BehaviourReferenceResolver for TestResolver {
        fn contains(&self, kind: ReferenceKind, name: &Identifier) -> bool {
            self.known.contains(&(kind, name.clone()))
        }
    }

    #[test]
    fn create_behaviour() {
        let behaviour = Behaviour::new(ident("thermal_protection"));
        assert_eq!(behaviour.name().as_str(), "thermal_protection");
        assert_eq!(behaviour.clause_count(), 0);
        assert!(behaviour.is_empty());
    }

    #[test]
    fn create_property_clause() {
        let clause = BehaviourClause::property(ident("temperature"));
        assert_eq!(clause.kind(), ReferenceKind::Property);
        assert_eq!(clause.name().as_str(), "temperature");
    }

    #[test]
    fn create_interface_clause() {
        let clause = BehaviourClause::interface(ident("power_input"));
        assert_eq!(clause.kind(), ReferenceKind::Interface);
        assert_eq!(clause.name().as_str(), "power_input");
    }

    #[test]
    fn create_behaviour_clause() {
        let clause = BehaviourClause::behaviour(ident("thermal_protection"));
        assert_eq!(clause.kind(), ReferenceKind::Behaviour);
        assert_eq!(clause.name().as_str(), "thermal_protection");
    }

    #[test]
    fn add_clause() {
        let mut behaviour = Behaviour::new(ident("thermal_protection"));
        behaviour
            .add_clause(BehaviourClause::property(ident("temperature")))
            .expect("add property");

        assert_eq!(behaviour.clause_count(), 1);
        assert!(!behaviour.is_empty());
    }

    #[test]
    fn duplicate_clause_rejected() {
        let mut behaviour = Behaviour::new(ident("thermal_protection"));
        behaviour
            .add_clause(BehaviourClause::property(ident("temperature")))
            .expect("first add succeeds");

        let err = behaviour
            .add_clause(BehaviourClause::property(ident("temperature")))
            .expect_err("duplicate rejected");

        assert_eq!(
            err,
            BehaviourError::DuplicateClause {
                behaviour: "thermal_protection".to_owned(),
                kind: "property".to_owned(),
                name: "temperature".to_owned(),
            }
        );
        assert_eq!(behaviour.clause_count(), 1);
    }

    #[test]
    fn clause_count() {
        let mut behaviour = Behaviour::new(ident("motor_behaviour"));
        assert_eq!(behaviour.clause_count(), 0);

        behaviour
            .add_clause(BehaviourClause::property(ident("voltage")))
            .expect("add voltage");
        assert_eq!(behaviour.clause_count(), 1);

        behaviour
            .add_clause(BehaviourClause::property(ident("current")))
            .expect("add current");
        assert_eq!(behaviour.clause_count(), 2);

        behaviour
            .add_clause(BehaviourClause::interface(ident("power_input")))
            .expect("add power_input");
        assert_eq!(behaviour.clause_count(), 3);
    }

    #[test]
    fn empty_behaviour_detected() {
        let behaviour = Behaviour::new(ident("thermal_protection"));
        let err = behaviour.validate().expect_err("empty rejected");

        assert_eq!(
            err,
            BehaviourError::EmptyContract {
                behaviour: "thermal_protection".to_owned(),
            }
        );
    }

    #[test]
    fn valid_behaviour_passes_validation() {
        let mut behaviour = Behaviour::new(ident("thermal_protection"));
        behaviour
            .add_clause(BehaviourClause::property(ident("temperature")))
            .expect("add temperature");
        behaviour
            .add_clause(BehaviourClause::interface(ident("thermal_sensor")))
            .expect("add thermal_sensor");

        assert!(behaviour.validate().is_ok());
    }

    #[test]
    fn remove_clause() {
        let mut behaviour = Behaviour::new(ident("thermal_protection"));
        behaviour
            .add_clause(BehaviourClause::property(ident("temperature")))
            .expect("add temperature");
        behaviour
            .add_clause(BehaviourClause::interface(ident("thermal_sensor")))
            .expect("add thermal_sensor");

        let removed = behaviour
            .remove_clause(ReferenceKind::Property, &ident("temperature"))
            .expect("removed");
        assert_eq!(removed.name().as_str(), "temperature");
        assert_eq!(behaviour.clause_count(), 1);

        assert!(behaviour
            .remove_clause(ReferenceKind::Property, &ident("temperature"))
            .is_none());
    }

    #[test]
    fn first_clause() {
        let mut behaviour = Behaviour::new(ident("thermal_protection"));
        assert!(behaviour.first_clause().is_none());

        behaviour
            .add_clause(BehaviourClause::property(ident("temperature")))
            .expect("add temperature");
        behaviour
            .add_clause(BehaviourClause::interface(ident("thermal_sensor")))
            .expect("add thermal_sensor");

        let first = behaviour.first_clause().expect("first present");
        assert_eq!(first.kind(), ReferenceKind::Property);
        assert_eq!(first.name().as_str(), "temperature");
    }

    #[test]
    fn deterministic_clause_ordering() {
        let behaviour = Behaviour::from_clauses(
            ident("motor_behaviour"),
            vec![
                BehaviourClause::property(ident("zeta")),
                BehaviourClause::property(ident("alpha")),
                BehaviourClause::property(ident("motor")),
            ],
        )
        .expect("valid behaviour");

        let order: Vec<&str> = behaviour
            .clauses()
            .map(|clause| clause.name().as_str())
            .collect();
        assert_eq!(order, vec!["zeta", "alpha", "motor"]);
    }

    #[test]
    fn clauses_of_kind() {
        let behaviour = Behaviour::from_clauses(
            ident("motor_behaviour"),
            vec![
                BehaviourClause::property(ident("voltage")),
                BehaviourClause::interface(ident("power_input")),
                BehaviourClause::property(ident("current")),
                BehaviourClause::behaviour(ident("thermal_protection")),
            ],
        )
        .expect("valid behaviour");

        let properties: Vec<&str> = behaviour
            .clauses_of_kind(ReferenceKind::Property)
            .map(|clause| clause.name().as_str())
            .collect();
        assert_eq!(properties, vec!["voltage", "current"]);

        let interfaces: Vec<&str> = behaviour
            .clauses_of_kind(ReferenceKind::Interface)
            .map(|clause| clause.name().as_str())
            .collect();
        assert_eq!(interfaces, vec!["power_input"]);

        let behaviours: Vec<&str> = behaviour
            .clauses_of_kind(ReferenceKind::Behaviour)
            .map(|clause| clause.name().as_str())
            .collect();
        assert_eq!(behaviours, vec!["thermal_protection"]);
    }

    #[test]
    fn references() {
        let behaviour = Behaviour::from_clauses(
            ident("motor_behaviour"),
            vec![
                BehaviourClause::property(ident("voltage")),
                BehaviourClause::interface(ident("power_input")),
            ],
        )
        .expect("valid behaviour");

        let refs: Vec<(ReferenceKind, &str)> = behaviour
            .references()
            .map(|clause| (clause.kind(), clause.name().as_str()))
            .collect();

        assert_eq!(
            refs,
            vec![
                (ReferenceKind::Property, "voltage"),
                (ReferenceKind::Interface, "power_input"),
            ]
        );
    }

    #[test]
    fn resolver_accepts_valid_references() {
        let mut resolver = TestResolver::new();
        resolver.insert(ReferenceKind::Property, "voltage");
        resolver.insert(ReferenceKind::Interface, "power_input");
        resolver.insert(ReferenceKind::Property, "current");

        let behaviour = Behaviour::from_clauses(
            ident("motor_behaviour"),
            vec![
                BehaviourClause::property(ident("voltage")),
                BehaviourClause::interface(ident("power_input")),
                BehaviourClause::property(ident("current")),
            ],
        )
        .expect("valid behaviour");

        assert!(behaviour.validate_with(&resolver).is_ok());
    }

    #[test]
    fn resolver_rejects_unresolved_references() {
        let mut resolver = TestResolver::new();
        resolver.insert(ReferenceKind::Property, "voltage");

        let behaviour = Behaviour::from_clauses(
            ident("motor_behaviour"),
            vec![
                BehaviourClause::property(ident("voltage")),
                BehaviourClause::interface(ident("power_input")),
            ],
        )
        .expect("valid behaviour");

        let err = behaviour
            .validate_with(&resolver)
            .expect_err("unresolved reference rejected");

        assert_eq!(
            err,
            BehaviourError::UnresolvedReference {
                behaviour: "motor_behaviour".to_owned(),
                kind: "interface".to_owned(),
                name: "power_input".to_owned(),
            }
        );
    }

    #[test]
    fn invalid_behaviour_is_rejected() {
        // An empty behaviour must fail structural validation.
        let behaviour = Behaviour::new(ident("empty_behaviour"));
        assert!(behaviour.validate().is_err());

        // And validate_with must fail on the same structural grounds
        // before consulting the resolver.
        let resolver = TestResolver::new();
        let err = behaviour
            .validate_with(&resolver)
            .expect_err("empty behaviour rejected");
        assert_eq!(
            err,
            BehaviourError::EmptyContract {
                behaviour: "empty_behaviour".to_owned(),
            }
        );
    }

    #[test]
    fn equality_behaves_deterministically() {
        let a = Behaviour::from_clauses(
            ident("motor_behaviour"),
            vec![
                BehaviourClause::property(ident("voltage")),
                BehaviourClause::interface(ident("power_input")),
            ],
        )
        .expect("valid behaviour");

        let b = Behaviour::from_clauses(
            ident("motor_behaviour"),
            vec![
                BehaviourClause::property(ident("voltage")),
                BehaviourClause::interface(ident("power_input")),
            ],
        )
        .expect("valid behaviour");

        assert_eq!(a, b);
    }

    #[test]
    fn display_output_is_deterministic() {
        let property = BehaviourClause::property(ident("temperature"));
        assert_eq!(property.to_string(), "property:temperature");

        let interface = BehaviourClause::interface(ident("power_input"));
        assert_eq!(interface.to_string(), "interface:power_input");

        let nested = BehaviourClause::behaviour(ident("thermal_protection"));
        assert_eq!(nested.to_string(), "behaviour:thermal_protection");
    }

    #[test]
    fn from_clauses_rejects_duplicates() {
        let err = Behaviour::from_clauses(
            ident("motor_behaviour"),
            vec![
                BehaviourClause::property(ident("voltage")),
                BehaviourClause::property(ident("voltage")),
            ],
        )
        .expect_err("duplicate rejected");

        assert_eq!(
            err,
            BehaviourError::DuplicateClause {
                behaviour: "motor_behaviour".to_owned(),
                kind: "property".to_owned(),
                name: "voltage".to_owned(),
            }
        );
    }

    #[test]
    fn same_name_different_kind_are_distinct_clauses() {
        let behaviour = Behaviour::from_clauses(
            ident("motor_behaviour"),
            vec![
                BehaviourClause::property(ident("power")),
                BehaviourClause::interface(ident("power")),
                BehaviourClause::behaviour(ident("power")),
            ],
        )
        .expect("valid behaviour");

        assert_eq!(behaviour.clause_count(), 3);
        assert_eq!(
            behaviour.clauses_of_kind(ReferenceKind::Property).count(),
            1
        );
        assert_eq!(
            behaviour.clauses_of_kind(ReferenceKind::Interface).count(),
            1
        );
        assert_eq!(
            behaviour.clauses_of_kind(ReferenceKind::Behaviour).count(),
            1
        );
    }
}