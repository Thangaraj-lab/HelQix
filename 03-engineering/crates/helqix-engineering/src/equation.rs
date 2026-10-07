//! Deterministic collection of engineering equation definitions for
//! HELQIX Layer 03 — Engineering.
//!
//! An [`EquationSet`] owns a set of [`Equation`] definitions. It provides
//! duplicate-safe insertion, deterministic iteration, and structural
//! validation of the contained definitions.
//!
//! The set is a **definition-only** structure. It never evaluates
//! expressions, never parses them, never solves them, and never performs
//! physics or simulation. Cross-reference and runtime semantics belong
//! to higher HELQIX layers.
//!
//! Ordering is deterministic via [`BTreeMap`]; the map key is always the
//! equation's own [`Identifier`].
//!
//! Collection-level failures are represented by [`EquationSetError`].
//! Equation-level failures are represented by [`EquationError`] and are
//! wrapped, never duplicated.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

use crate::equation::{Equation, EquationError};

/// Errors produced by [`EquationSet`] operations.
///
/// `EquationSetError` represents **collection-level** failures. Structural
/// failures of an individual [`Equation`] are represented by
/// [`EquationError`] and are wrapped in
/// [`EquationSetError::InvalidEquation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EquationSetError {
    /// An equation with the same identifier is already present in the set.
    DuplicateEquation {
        /// Name of the duplicated equation.
        name: Identifier,
    },
    /// A stored equation failed structural validation.
    InvalidEquation {
        /// Name of the invalid equation.
        name: Identifier,
        /// The underlying equation-level failure.
        source: EquationError,
    },
}

impl fmt::Display for EquationSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EquationSetError::DuplicateEquation { name } => {
                write!(f, "duplicate equation: {}", name.as_str())
            }
            EquationSetError::InvalidEquation { name, source } => {
                write!(f, "invalid equation {}: {}", name.as_str(), source)
            }
        }
    }
}

impl Error for EquationSetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            EquationSetError::InvalidEquation { source, .. } => Some(source),
            EquationSetError::DuplicateEquation { .. } => None,
        }
    }
}

/// Deterministic collection of engineering [`Equation`] definitions.
///
/// Backed by a [`BTreeMap`] keyed on [`Identifier`], giving deterministic
/// iteration order. Duplicate equation names are rejected; existing
/// entries are never overwritten. Invalid equations are rejected at
/// insertion time, so the set never contains an invalid equation through
/// its public API.
///
/// `EquationSet` holds definitions only. It performs no evaluation,
/// parsing, or solving.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquationSet {
    equations: BTreeMap<Identifier, Equation>,
}

impl EquationSet {
    /// Creates an empty equation set.
    pub fn new() -> Self {
        Self {
            equations: BTreeMap::new(),
        }
    }

    /// Returns the number of equations in the set.
    pub fn len(&self) -> usize {
        self.equations.len()
    }

    /// Returns `true` if the set contains no equations.
    pub fn is_empty(&self) -> bool {
        self.equations.is_empty()
    }

    /// Adds an equation to the set.
    ///
    /// The equation is structurally validated first, then duplicate names
    /// are rejected. On success, the equation's own identifier is used
    /// as the canonical key. The set is never mutated on failure.
    pub fn add(&mut self, equation: Equation) -> Result<(), EquationSetError> {
        equation
            .validate()
            .map_err(|err| EquationSetError::InvalidEquation {
                name: equation.name().clone(),
                source: err,
            })?;

        if self.equations.contains_key(equation.name()) {
            return Err(EquationSetError::DuplicateEquation {
                name: equation.name().clone(),
            });
        }

        let key = equation.name().clone();
        self.equations.insert(key, equation);
        Ok(())
    }

    /// Returns an immutable reference to the equation with the given name.
    pub fn get(&self, name: &Identifier) -> Option<&Equation> {
        self.equations.get(name)
    }

    /// Returns a mutable reference to the equation with the given name.
    ///
    /// The internal map is never exposed. Callers may mutate the stored
    /// [`Equation`] through this reference; the equation itself remains
    /// responsible for preserving its own invariants.
    pub fn get_mut(&mut self, name: &Identifier) -> Option<&mut Equation> {
        self.equations.get_mut(name)
    }

    /// Returns `true` if an equation with the given name is present.
    pub fn contains(&self, name: &Identifier) -> bool {
        self.equations.contains_key(name)
    }

    /// Removes and returns the equation with the given name, if present.
    pub fn remove(&mut self, name: &Identifier) -> Option<Equation> {
        self.equations.remove(name)
    }

    /// Iterates over `(name, equation)` pairs in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &Equation)> {
        self.equations.iter()
    }

    /// Iterates over equations in deterministic order.
    pub fn equations(&self) -> impl Iterator<Item = &Equation> {
        self.equations.values()
    }

    /// Performs structural validation of every stored equation.
    ///
    /// Equations are validated in the deterministic [`BTreeMap`] order.
    /// The first invalid equation aborts validation with
    /// [`EquationSetError::InvalidEquation`]. No automatic repair or
    /// removal is performed.
    ///
    /// No cross-equation dependency analysis is performed here.
    pub fn validate(&self) -> Result<(), EquationSetError> {
        for (name, equation) in &self.equations {
            equation
                .validate()
                .map_err(|err| EquationSetError::InvalidEquation {
                    name: name.clone(),
                    source: err,
                })?;
        }
        Ok(())
    }
}

impl Default for EquationSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ident(s: &str) -> Identifier {
        Identifier::parse(s).expect("known-valid identifier literal")
    }

    /// Builds a valid equation with one input, differing from its output.
    fn valid_equation(name: &str) -> Equation {
        let mut equation = Equation::new(ident(name), ident("out"), "1")
            .expect("valid equation header");
        equation
            .add_input(ident("input"))
            .expect("valid input added");
        equation
    }

    /// Builds an equation that fails structural validation (no inputs).
    fn invalid_equation(name: &str) -> Equation {
        Equation::new(ident(name), ident("out"), "1")
            .expect("valid equation header")
    }

    #[test]
    fn new_set_is_empty() {
        let set = EquationSet::new();
        assert_eq!(set.len(), 0);
        assert!(set.is_empty());
    }

    #[test]
    fn default_set_is_empty() {
        let set = EquationSet::default();
        assert_eq!(set.len(), 0);
        assert!(set.is_empty());
        assert!(set.validate().is_ok());
    }

    #[test]
    fn add_equation() {
        let mut set = EquationSet::new();
        set.add(valid_equation("force_equation"))
            .expect("add succeeds");

        assert_eq!(set.len(), 1);
        assert!(!set.is_empty());
    }

    #[test]
    fn duplicate_equation_is_rejected() {
        let mut set = EquationSet::new();
        set.add(valid_equation("force_equation"))
            .expect("first add succeeds");

        let err = set
            .add(valid_equation("force_equation"))
            .expect_err("duplicate rejected");

        assert_eq!(
            err,
            EquationSetError::DuplicateEquation {
                name: ident("force_equation"),
            }
        );
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn duplicate_does_not_replace_original() {
        let mut set = EquationSet::new();
        set.add(valid_equation("force_equation"))
            .expect("first add succeeds");

        let _ = set.add(valid_equation("force_equation"));

        let stored = set.get(&ident("force_equation")).expect("present");
        assert_eq!(stored.name().as_str(), "force_equation");
        assert_eq!(stored.input_count(), 1);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn failed_add_does_not_mutate_set() {
        let mut set = EquationSet::new();
        set.add(valid_equation("force_equation"))
            .expect("first add succeeds");

        let _ = set.add(valid_equation("force_equation"));
        assert_eq!(set.len(), 1);

        let stored = set.get(&ident("force_equation")).expect("present");
        assert_eq!(stored.input_count(), 1);
    }

    #[test]
    fn get_equation() {
        let mut set = EquationSet::new();
        set.add(valid_equation("force_equation"))
            .expect("add succeeds");

        let found = set.get(&ident("force_equation")).expect("present");
        assert_eq!(found.name().as_str(), "force_equation");

        assert!(set.get(&ident("missing_equation")).is_none());
    }

    #[test]
    fn get_mut_allows_controlled_mutation() {
        let mut set = EquationSet::new();
        set.add(valid_equation("force_equation"))
            .expect("add succeeds");

        let equation = set
            .get_mut(&ident("force_equation"))
            .expect("mutable present");
        equation
            .add_input(ident("extra_input"))
            .expect("add extra input");

        assert!(set
            .get(&ident("force_equation"))
            .expect("present")
            .has_input(&ident("extra_input")));
    }

    #[test]
    fn contains_equation() {
        let mut set = EquationSet::new();
        set.add(valid_equation("force_equation"))
            .expect("add succeeds");

        assert!(set.contains(&ident("force_equation")));
        assert!(!set.contains(&ident("power_equation")));
    }

    #[test]
    fn remove_equation() {
        let mut set = EquationSet::new();
        set.add(valid_equation("force_equation"))
            .expect("add succeeds");

        let removed = set
            .remove(&ident("force_equation"))
            .expect("removed");
        assert_eq!(removed.name().as_str(), "force_equation");
        assert!(set.is_empty());

        assert!(set.remove(&ident("force_equation")).is_none());
    }

    #[test]
    fn missing_equation_returns_none() {
        let mut set = EquationSet::new();
        assert!(set.remove(&ident("force_equation")).is_none());
    }

    #[test]
    fn deterministic_iteration_order() {
        let mut set = EquationSet::new();
        set.add(valid_equation("zeta")).expect("add zeta");
        set.add(valid_equation("alpha")).expect("add alpha");
        set.add(valid_equation("motor")).expect("add motor");
        set.add(valid_equation("battery")).expect("add battery");

        let order: Vec<&str> = set.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(order, vec!["alpha", "battery", "motor", "zeta"]);

        let equations_order: Vec<&str> =
            set.equations().map(|e| e.name().as_str()).collect();
        assert_eq!(
            equations_order,
            vec!["alpha", "battery", "motor", "zeta"]
        );
    }

    #[test]
    fn validate_empty_set() {
        let set = EquationSet::new();
        assert!(set.validate().is_ok());
    }

    #[test]
    fn validate_all_equations() {
        let mut set = EquationSet::new();
        set.add(valid_equation("force_equation"))
            .expect("add force");
        set.add(valid_equation("power_equation"))
            .expect("add power");
        set.add(valid_equation("energy_equation"))
            .expect("add energy");

        assert!(set.validate().is_ok());
    }

    #[test]
    fn invalid_equation_is_reported() {
        let mut set = EquationSet::new();
        let invalid = invalid_equation("broken_equation");

        let err = set
            .add(invalid)
            .expect_err("invalid equation rejected");

        match err {
            EquationSetError::InvalidEquation { name, source } => {
                assert_eq!(name.as_str(), "broken_equation");
                // Structural failure: no inputs declared.
                matches!(source, EquationError::MissingInput { .. });
            }
            EquationSetError::DuplicateEquation { .. } => {
                panic!("expected InvalidEquation, got DuplicateEquation");
            }
        }

        assert!(set.is_empty());
    }

    #[test]
    fn equality_behaviour() {
        let mut a = EquationSet::new();
        a.add(valid_equation("alpha")).expect("add alpha");
        a.add(valid_equation("beta")).expect("add beta");

        let mut b = EquationSet::new();
        // Insert in opposite order; BTreeMap guarantees deterministic
        // ordering regardless of insertion sequence.
        b.add(valid_equation("beta")).expect("add beta");
        b.add(valid_equation("alpha")).expect("add alpha");

        assert_eq!(a, b);

        let mut c = EquationSet::new();
        c.add(valid_equation("alpha")).expect("add alpha");
        assert_ne!(a, c);
    }
}