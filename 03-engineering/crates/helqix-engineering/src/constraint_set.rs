//! Deterministic collection of engineering constraint definitions for
//! HELQIX Layer 03 — Engineering.
//!
//! A [`ConstraintSet`] owns a set of [`Constraint`] definitions. It
//! provides duplicate-safe insertion, deterministic iteration, and
//! structural validation of the contained definitions.
//!
//! The set is a **definition-only** structure. It never evaluates
//! constraints against runtime state, never monitors execution, never
//! triggers failures, and never performs physics, simulation, or
//! optimization. Cross-reference and evaluation semantics belong to
//! higher HELQIX layers.
//!
//! Ordering is deterministic via [`BTreeMap`]; the map key is always the
//! constraint's own [`Identifier`].

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

use crate::constraint::Constraint;

/// Errors produced by [`ConstraintSet`] operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintSetError {
    /// A constraint with the same identifier is already present.
    DuplicateConstraint {
        /// Name of the duplicated constraint.
        name: String,
    },
    /// A constraint with the requested identifier is not present.
    MissingConstraint {
        /// Name of the missing constraint.
        name: String,
    },
    /// A stored constraint failed structural validation.
    InvalidConstraint {
        /// Name of the invalid constraint.
        name: String,
        /// Human-readable description of the failure.
        detail: String,
    },
}

impl fmt::Display for ConstraintSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstraintSetError::DuplicateConstraint { name } => {
                write!(f, "duplicate constraint: {name}")
            }
            ConstraintSetError::MissingConstraint { name } => {
                write!(f, "missing constraint: {name}")
            }
            ConstraintSetError::InvalidConstraint { name, detail } => {
                write!(f, "invalid constraint {name}: {detail}")
            }
        }
    }
}

impl Error for ConstraintSetError {}

/// Deterministic collection of engineering [`Constraint`] definitions.
///
/// Backed by a [`BTreeMap`] keyed on [`Identifier`], giving deterministic
/// iteration order. Duplicate constraint names are rejected; existing
/// entries are never overwritten. Invalid constraints are rejected at
/// insertion time, so the set never contains an invalid constraint
/// through its public API.
///
/// `ConstraintSet` holds definitions only. It performs no evaluation,
/// monitoring, simulation, or optimization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintSet {
    constraints: BTreeMap<Identifier, Constraint>,
}

impl ConstraintSet {
    /// Creates an empty constraint set.
    pub fn new() -> Self {
        Self {
            constraints: BTreeMap::new(),
        }
    }

    /// Returns the number of constraints in the set.
    pub fn len(&self) -> usize {
        self.constraints.len()
    }

    /// Returns `true` if the set contains no constraints.
    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }

    /// Returns `true` if a constraint with the given name is present.
    pub fn contains(&self, name: &Identifier) -> bool {
        self.constraints.contains_key(name)
    }

    /// Adds a constraint to the set.
    ///
    /// The constraint is structurally validated first, then duplicate
    /// names are rejected. On success, the constraint's own identifier
    /// is used as the canonical key. The set is never mutated on failure.
    pub fn add(&mut self, constraint: Constraint) -> Result<(), ConstraintSetError> {
        constraint
            .validate()
            .map_err(|err| ConstraintSetError::InvalidConstraint {
                name: constraint.name().as_str().to_owned(),
                detail: err.to_string(),
            })?;

        if self.constraints.contains_key(constraint.name()) {
            return Err(ConstraintSetError::DuplicateConstraint {
                name: constraint.name().as_str().to_owned(),
            });
        }

        let key = constraint.name().clone();
        self.constraints.insert(key, constraint);
        Ok(())
    }

    /// Returns an immutable reference to the constraint with the given name.
    pub fn get(&self, name: &Identifier) -> Option<&Constraint> {
        self.constraints.get(name)
    }

    /// Returns a mutable reference to the constraint with the given name.
    ///
    /// The internal map is never exposed. Callers may mutate the stored
    /// [`Constraint`] through this reference; the constraint itself
    /// remains responsible for preserving its own invariants.
    pub fn get_mut(&mut self, name: &Identifier) -> Option<&mut Constraint> {
        self.constraints.get_mut(name)
    }

    /// Returns the constraint with the given name.
    ///
    /// Returns [`ConstraintSetError::MissingConstraint`] if the
    /// constraint is not present.
    pub fn require(&self, name: &Identifier) -> Result<&Constraint, ConstraintSetError> {
        self.constraints
            .get(name)
            .ok_or_else(|| ConstraintSetError::MissingConstraint {
                name: name.as_str().to_owned(),
            })
    }

    /// Removes and returns the constraint with the given name, if present.
    pub fn remove(&mut self, name: &Identifier) -> Option<Constraint> {
        self.constraints.remove(name)
    }

    /// Iterates over `(name, constraint)` pairs in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &Constraint)> {
        self.constraints.iter()
    }

    /// Iterates over constraints in deterministic order.
    pub fn values(&self) -> impl Iterator<Item = &Constraint> {
        self.constraints.values()
    }

    /// Iterates over constraint names in deterministic order.
    pub fn keys(&self) -> impl Iterator<Item = &Identifier> {
        self.constraints.keys()
    }

    /// Performs structural validation of every stored constraint.
    ///
    /// Constraints are validated in the deterministic [`BTreeMap`] order.
    /// The first invalid constraint aborts validation with
    /// [`ConstraintSetError::InvalidConstraint`]. No automatic repair or
    /// removal is performed.
    pub fn validate(&self) -> Result<(), ConstraintSetError> {
        for (name, constraint) in &self.constraints {
            constraint
                .validate()
                .map_err(|err| ConstraintSetError::InvalidConstraint {
                    name: name.as_str().to_owned(),
                    detail: err.to_string(),
                })?;
        }
        Ok(())
    }
}

impl Default for ConstraintSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::constraint::ConstraintReference;

    fn ident(s: &str) -> Identifier {
        Identifier::parse(s).expect("known-valid identifier literal")
    }

    fn temp_ref() -> ConstraintReference {
        ConstraintReference::property(ident("temperature"))
    }

    fn constraint(name: &str, min: f64, max: f64) -> Constraint {
        Constraint::new(ident(name), temp_ref(), min, max).expect("valid constraint")
    }

    #[test]
    fn new_set_is_empty() {
        let set = ConstraintSet::new();
        assert_eq!(set.len(), 0);
        assert!(set.is_empty());
    }

    #[test]
    fn default_creates_empty_set() {
        let set = ConstraintSet::default();
        assert_eq!(set.len(), 0);
        assert!(set.is_empty());
        assert!(set.validate().is_ok());
    }

    #[test]
    fn add_valid_constraint() {
        let mut set = ConstraintSet::new();
        set.add(constraint("motor_temp_limit", 0.0, 120.0))
            .expect("add succeeds");

        assert_eq!(set.len(), 1);
        assert!(!set.is_empty());
    }

    #[test]
    fn len() {
        let mut set = ConstraintSet::new();
        assert_eq!(set.len(), 0);

        set.add(constraint("a", 0.0, 1.0)).expect("add a");
        assert_eq!(set.len(), 1);

        set.add(constraint("b", 0.0, 1.0)).expect("add b");
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn is_empty() {
        let mut set = ConstraintSet::new();
        assert!(set.is_empty());

        set.add(constraint("a", 0.0, 1.0)).expect("add a");
        assert!(!set.is_empty());
    }

    #[test]
    fn contains() {
        let mut set = ConstraintSet::new();
        set.add(constraint("a", 0.0, 1.0)).expect("add a");

        assert!(set.contains(&ident("a")));
        assert!(!set.contains(&ident("b")));
    }

    #[test]
    fn get() {
        let mut set = ConstraintSet::new();
        set.add(constraint("a", 0.0, 1.0)).expect("add a");

        let found = set.get(&ident("a")).expect("present");
        assert_eq!(found.name().as_str(), "a");

        assert!(set.get(&ident("b")).is_none());
    }

    #[test]
    fn get_mut() {
        let mut set = ConstraintSet::new();
        set.add(constraint("a", 0.0, 1.0)).expect("add a");

        let stored = set.get_mut(&ident("a")).expect("mutable present");
        assert_eq!(stored.name().as_str(), "a");

        assert!(set.get_mut(&ident("missing")).is_none());
    }

    #[test]
    fn require_existing_constraint() {
        let mut set = ConstraintSet::new();
        set.add(constraint("a", 0.0, 1.0)).expect("add a");

        let found = set.require(&ident("a")).expect("present");
        assert_eq!(found.name().as_str(), "a");
    }

    #[test]
    fn require_missing_constraint() {
        let set = ConstraintSet::new();
        let err = set
            .require(&ident("a"))
            .expect_err("missing rejected");

        assert_eq!(
            err,
            ConstraintSetError::MissingConstraint {
                name: "a".to_owned(),
            }
        );
    }

    #[test]
    fn remove() {
        let mut set = ConstraintSet::new();
        set.add(constraint("a", 0.0, 1.0)).expect("add a");

        let removed = set.remove(&ident("a")).expect("removed");
        assert_eq!(removed.name().as_str(), "a");
        assert!(set.is_empty());

        assert!(set.remove(&ident("a")).is_none());
    }

    #[test]
    fn duplicate_constraint_rejected() {
        let mut set = ConstraintSet::new();
        set.add(constraint("a", 0.0, 1.0)).expect("first add");

        let err = set
            .add(constraint("a", 5.0, 10.0))
            .expect_err("duplicate rejected");

        assert_eq!(
            err,
            ConstraintSetError::DuplicateConstraint {
                name: "a".to_owned(),
            }
        );
    }

    #[test]
    fn duplicate_does_not_replace_original() {
        let mut set = ConstraintSet::new();
        set.add(constraint("a", 0.0, 1.0)).expect("first add");

        let _ = set.add(constraint("a", 5.0, 10.0));

        let stored = set.require(&ident("a")).expect("present");
        assert_eq!(stored.min(), 0.0);
        assert_eq!(stored.max(), 1.0);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn failed_add_does_not_mutate_set() {
        let mut set = ConstraintSet::new();
        set.add(constraint("a", 0.0, 1.0)).expect("first add");

        // Duplicate attempt fails; the set remains unchanged.
        let _ = set.add(constraint("a", 5.0, 10.0));
        assert_eq!(set.len(), 1);

        // Verify the stored definition is untouched.
        let stored = set.require(&ident("a")).expect("present");
        assert_eq!(stored.min(), 0.0);
        assert_eq!(stored.max(), 1.0);
    }

    #[test]
    fn deterministic_iteration_order() {
        let mut set = ConstraintSet::new();
        set.add(constraint("zeta", 0.0, 1.0)).expect("add zeta");
        set.add(constraint("alpha", 0.0, 1.0)).expect("add alpha");
        set.add(constraint("motor", 0.0, 1.0)).expect("add motor");
        set.add(constraint("battery", 0.0, 1.0)).expect("add battery");

        let order: Vec<&str> = set.keys().map(|k| k.as_str()).collect();
        assert_eq!(order, vec!["alpha", "battery", "motor", "zeta"]);
    }

    #[test]
    fn values() {
        let mut set = ConstraintSet::new();
        set.add(constraint("a", 0.0, 1.0)).expect("add a");
        set.add(constraint("b", 2.0, 3.0)).expect("add b");

        let names: Vec<&str> = set.values().map(|c| c.name().as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);
    }

    #[test]
    fn keys() {
        let mut set = ConstraintSet::new();
        set.add(constraint("b", 0.0, 1.0)).expect("add b");
        set.add(constraint("a", 0.0, 1.0)).expect("add a");

        let names: Vec<&str> = set.keys().map(|k| k.as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);
    }

    #[test]
    fn validate_empty_set() {
        let set = ConstraintSet::new();
        assert!(set.validate().is_ok());
    }

    #[test]
    fn validate_valid_set() {
        let mut set = ConstraintSet::new();
        set.add(constraint("a", 0.0, 1.0)).expect("add a");
        set.add(constraint("b", 10.0, 20.0)).expect("add b");

        assert!(set.validate().is_ok());
    }

    #[test]
    fn validate_reports_invalid_stored_constraint_if_reachable() {
        // Through the public API, invalid constraints cannot enter the
        // set. This test confirms that a well-formed set always passes
        // validation, i.e. `validate` is consistent with `add`.
        let mut set = ConstraintSet::new();
        set.add(constraint("a", -10.0, 10.0)).expect("add a");
        set.add(constraint("b", 42.0, 42.0)).expect("add b");

        assert!(set.validate().is_ok());
    }

    #[test]
    fn equality_behaviour() {
        let mut a = ConstraintSet::new();
        a.add(constraint("x", 0.0, 1.0)).expect("add x");
        a.add(constraint("y", 2.0, 3.0)).expect("add y");

        let mut b = ConstraintSet::new();
        // Insert in opposite order; BTreeMap guarantees deterministic
        // ordering regardless of insertion sequence.
        b.add(constraint("y", 2.0, 3.0)).expect("add y");
        b.add(constraint("x", 0.0, 1.0)).expect("add x");

        assert_eq!(a, b);

        let mut c = ConstraintSet::new();
        c.add(constraint("x", 0.0, 1.0)).expect("add x");
        assert_ne!(a, c);
    }
}