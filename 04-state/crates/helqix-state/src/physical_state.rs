//! Current physical quantities of a HELQIX component or system.
//!
//! `PhysicalState` stores runtime physical values only.
//!
//! Responsibilities:
//! - deterministic storage of named physical quantities;
//! - duplicate-safe insertion;
//! - lookup and mutation;
//! - structural validation.
//!
//! It does NOT:
//! - calculate physics;
//! - advance time;
//! - integrate motion;
//! - evaluate equations;
//! - detect failures;
//! - perform execution;
//! - perform causality analysis;
//! - define units.
//!
//! Physical values use `helqix_core::quantity::Quantity` so Layer 04
//! does not introduce a second quantity or unit system.

use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;
use helqix_core::quantity::Quantity;

/// Current physical state represented as named quantities.
///
/// The `BTreeMap` guarantees deterministic ordering, which is required
/// throughout HELQIX state representation.
#[derive(Debug, Clone, PartialEq)]
pub struct PhysicalState {
    quantities: BTreeMap<Identifier, Quantity>,
}

impl Default for PhysicalState {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicalState {
    /// Creates an empty physical state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            quantities: BTreeMap::new(),
        }
    }

    /// Returns the number of physical quantities stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.quantities.len()
    }

    /// Returns `true` when no physical quantities are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.quantities.is_empty()
    }

    /// Returns `true` when a quantity with `name` exists.
    #[must_use]
    pub fn contains(&self, name: &Identifier) -> bool {
        self.quantities.contains_key(name)
    }

    /// Returns a physical quantity by name.
    #[must_use]
    pub fn get(&self, name: &Identifier) -> Option<&Quantity> {
        self.quantities.get(name)
    }

    /// Returns a mutable physical quantity by name.
    pub fn get_mut(&mut self, name: &Identifier) -> Option<&mut Quantity> {
        self.quantities.get_mut(name)
    }

    /// Inserts a physical quantity.
    ///
    /// Existing quantities are never silently overwritten.
    pub fn insert(
        &mut self,
        name: Identifier,
        value: Quantity,
    ) -> Result<(), PhysicalStateError> {
        match self.quantities.entry(name) {
            Entry::Occupied(entry) => Err(PhysicalStateError::DuplicateQuantity {
                name: entry.key().as_str().to_owned(),
            }),
            Entry::Vacant(entry) => {
                entry.insert(value);
                Ok(())
            }
        }
    }

    /// Inserts or replaces a physical quantity.
    ///
    /// This operation is intentionally explicit because runtime state
    /// changes are expected to be visible to higher execution layers.
    pub fn set(&mut self, name: Identifier, value: Quantity) {
        let _ = self.replace(name, value);
    }

    /// Inserts or replaces a quantity and returns the previous value.
    pub fn replace(&mut self, name: Identifier, value: Quantity) -> Option<Quantity> {
        self.quantities.insert(name, value)
    }

    /// Removes a physical quantity.
    pub fn remove(&mut self, name: &Identifier) -> Option<Quantity> {
        self.quantities.remove(name)
    }

    /// Iterates over physical quantities in deterministic identifier order.
    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &Quantity)> {
        self.quantities.iter()
    }

    /// Returns all physical quantities as deterministic key/value pairs.
    pub fn named_quantities(&self) -> impl Iterator<Item = (&Identifier, &Quantity)> {
        self.quantities.iter()
    }

    /// Validates the structural invariants of the physical state.
    ///
    /// `Quantity` is responsible for the validity of its numerical value
    /// and unit. This layer therefore validates only state structure.
    pub fn validate(&self) -> Result<(), PhysicalStateError> {
        for (name, _) in &self.quantities {
            if name.as_str().is_empty() {
                return Err(PhysicalStateError::InvalidQuantityName);
            }
        }

        Ok(())
    }
}

/// Errors produced by [`PhysicalState`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicalStateError {
    /// A quantity with the same identifier already exists.
    DuplicateQuantity {
        name: String,
    },

    /// A physical quantity was found under an invalid empty identifier.
    ///
    /// Normally unreachable because `Identifier` itself guarantees
    /// identifier validity. It is retained as a defensive invariant.
    InvalidQuantityName,
}

impl fmt::Display for PhysicalStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateQuantity { name } => {
                write!(f, "duplicate physical quantity: {name}")
            }
            Self::InvalidQuantityName => {
                write!(f, "physical quantity has an invalid empty name")
            }
        }
    }
}

impl Error for PhysicalStateError {}

#[cfg(test)]
mod tests {
    use super::*;
    use helqix_core::quantity::Quantity;
    use helqix_core::units::Unit;

    fn identifier(value: &str) -> Identifier {
        value.parse().expect("valid identifier")
    }

    fn quantity(value: f64, unit: &str) -> Quantity {
        Quantity::new(value, Unit::parse(unit).expect("valid unit"))
            .expect("valid quantity")
    }

    #[test]
    fn new_state_is_empty() {
        let state = PhysicalState::new();

        assert_eq!(state.len(), 0);
        assert!(state.is_empty());
    }

    #[test]
    fn insert_stores_quantity() {
        let mut state = PhysicalState::new();
        let name = identifier("mass");
        let value = quantity(10.0, "kg");

        state
            .insert(name.clone(), value.clone())
            .expect("first insertion succeeds");

        assert_eq!(state.len(), 1);
        assert!(state.contains(&name));
        assert_eq!(state.get(&name), Some(&value));
    }

    #[test]
    fn duplicate_insert_is_rejected() {
        let mut state = PhysicalState::new();
        let name = identifier("mass");

        state
            .insert(name.clone(), quantity(10.0, "kg"))
            .expect("first insertion succeeds");

        let result = state.insert(name, quantity(20.0, "kg"));

        assert_eq!(
            result,
            Err(PhysicalStateError::DuplicateQuantity {
                name: "mass".to_owned(),
            })
        );
    }

    #[test]
    fn set_replaces_existing_quantity() {
        let mut state = PhysicalState::new();
        let name = identifier("temperature");

        state.set(name.clone(), quantity(300.0, "K"));
        state.set(name.clone(), quantity(350.0, "K"));

        assert_eq!(state.len(), 1);

        let stored = state.get(&name).expect("quantity exists");
        assert_eq!(stored.value(), 350.0);
    }

    #[test]
    fn remove_returns_existing_quantity() {
        let mut state = PhysicalState::new();
        let name = identifier("mass");
        let value = quantity(10.0, "kg");

        state
            .insert(name.clone(), value.clone())
            .expect("insertion succeeds");

        assert_eq!(state.remove(&name), Some(value));
        assert!(state.is_empty());
    }

    #[test]
    fn missing_quantity_returns_none() {
        let state = PhysicalState::new();
        let name = identifier("mass");

        assert_eq!(state.get(&name), None);
    }

    #[test]
    fn iteration_is_deterministic() {
        let mut state = PhysicalState::new();

        state
            .insert(identifier("temperature"), quantity(300.0, "K"))
            .expect("insertion succeeds");

        state
            .insert(identifier("mass"), quantity(10.0, "kg"))
            .expect("insertion succeeds");

        let names: Vec<&str> = state
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();

        assert_eq!(names, vec!["mass", "temperature"]);
    }

    #[test]
    fn empty_state_validates() {
        let state = PhysicalState::new();

        assert_eq!(state.validate(), Ok(()));
    }

    #[test]
    fn populated_state_validates() {
        let mut state = PhysicalState::new();

        state
            .insert(identifier("mass"), quantity(10.0, "kg"))
            .expect("insertion succeeds");

        state
            .insert(identifier("temperature"), quantity(300.0, "K"))
            .expect("insertion succeeds");

        assert_eq!(state.validate(), Ok(()));
    }
}
