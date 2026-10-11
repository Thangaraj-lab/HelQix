//! Runtime state of one engineering property.
//!
//! `PropertyState` stores the current value of a property during execution.
//!
//! Responsibilities:
//! - identify the property
//! - hold its current [`Quantity`]
//! - allow the value to be assigned or cleared
//! - report whether a value currently exists
//!
//! This type deliberately does not own:
//! - property definitions
//! - engineering parameters
//! - constraints
//! - equations
//! - physics
//! - execution
//! - time
//! - failure logic
//! - branching
//! - optimization
//!
//! Units and dimensional correctness are delegated to HELQIX Foundation's
//! `Quantity` and `Unit` system.

use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;
use helqix_core::quantity::Quantity;
use helqix_core::QuantityError;

/// Current runtime state of one engineering property.
///
/// A property may legitimately have no current value. This is important
/// because state construction and state execution are separate concerns.
/// Validation of whether a value is required for a particular engineering
/// definition belongs to the Layer 04 validation boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyState {
    name: Identifier,
    value: Option<Quantity>,
}

impl PropertyState {
    /// Creates a property state without an assigned runtime value.
    ///
    /// The supplied identifier has already been validated by Foundation.
    #[must_use]
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            value: None,
        }
    }

    /// Creates a property state with an initial runtime value.
    #[must_use]
    pub fn with_value(name: Identifier, value: Quantity) -> Self {
        Self {
            name,
            value: Some(value),
        }
    }

    /// Returns the canonical property identifier.
    #[must_use]
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the current runtime value, if one exists.
    #[must_use]
    pub fn value(&self) -> Option<&Quantity> {
        self.value.as_ref()
    }

    /// Returns mutable access to the current runtime value, if one exists.
    ///
    /// Mutation of the quantity itself remains governed by the Foundation
    /// quantity API.
    #[must_use]
    pub fn value_mut(&mut self) -> Option<&mut Quantity> {
        self.value.as_mut()
    }

    /// Returns `true` when a runtime value is currently assigned.
    #[must_use]
    pub fn has_value(&self) -> bool {
        self.value.is_some()
    }

    /// Assigns a new runtime value.
    ///
    /// The previous value, if any, is replaced atomically.
    pub fn set_value(&mut self, value: Quantity) {
        self.value = Some(value);
    }

    /// Replaces the current value and returns the previous one, if present.
    pub fn replace_value(&mut self, value: Quantity) -> Option<Quantity> {
        self.value.replace(value)
    }

    /// Removes the current runtime value.
    ///
    /// Returns the previous value when one existed.
    pub fn clear_value(&mut self) -> Option<Quantity> {
        self.value.take()
    }

    /// Returns the current value or a deterministic error when no value exists.
    ///
    /// This method is intentionally explicit rather than silently inventing a
    /// default value. A missing runtime value is meaningful state.
    pub fn require_value(&self) -> Result<&Quantity, PropertyStateError> {
        self.value
            .as_ref()
            .ok_or_else(|| PropertyStateError::MissingValue {
                name: self.name.clone(),
            })
    }

    /// Validates the internal state representation.
    ///
    /// `Quantity` is responsible for numeric validity, so there is no second
    /// numeric validation system here.
    pub fn validate(&self) -> Result<(), PropertyStateError> {
        if let Some(value) = self.value {
            Quantity::new(value.value(), *value.unit())
                .map_err(PropertyStateError::InvalidValue)?;
        }
        Ok(())
    }
}

/// Errors produced while accessing [`PropertyState`].
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyStateError {
    /// The property has no current runtime value.
    MissingValue {
        name: Identifier,
    },

    /// The stored quantity violated Foundation's finite-value invariant.
    InvalidValue(QuantityError),
}

impl fmt::Display for PropertyStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingValue { name } => {
                write!(f, "property `{name}` has no current runtime value")
            }
            Self::InvalidValue(source) => write!(f, "property contains an invalid quantity: {source}"),
        }
    }
}

impl Error for PropertyStateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidValue(source) => Some(source),
            Self::MissingValue { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helqix_core::units::Unit;

    fn identifier(value: &str) -> Identifier {
        Identifier::parse(value).expect("test identifier must be valid")
    }

    fn quantity(value: f64, unit: &str) -> Quantity {
        Quantity::new(
            value,
            Unit::parse(unit).expect("test unit must be valid"),
        )
        .expect("test quantity must be valid")
    }

    #[test]
    fn new_property_state_has_no_value() {
        let state = PropertyState::new(identifier("mass"));

        assert_eq!(state.name().as_str(), "mass");
        assert!(!state.has_value());
        assert!(state.value().is_none());
    }

    #[test]
    fn with_value_creates_initialized_state() {
        let state = PropertyState::with_value(
            identifier("mass"),
            quantity(10.0, "kg"),
        );

        assert!(state.has_value());

        let value = state.value().expect("value must exist");

        assert_eq!(value.value(), 10.0);
        assert_eq!(value.unit().symbol(), "kg");
    }

    #[test]
    fn set_value_replaces_existing_value() {
        let mut state = PropertyState::with_value(
            identifier("mass"),
            quantity(10.0, "kg"),
        );

        state.set_value(quantity(20.0, "kg"));

        let value = state.require_value().expect("value must exist");

        assert_eq!(value.value(), 20.0);
        assert_eq!(value.unit().symbol(), "kg");
    }

    #[test]
    fn clear_value_removes_current_value() {
        let mut state = PropertyState::with_value(
            identifier("mass"),
            quantity(10.0, "kg"),
        );

        let previous = state.clear_value().expect("previous value must exist");

        assert_eq!(previous.value(), 10.0);
        assert!(!state.has_value());
    }

    #[test]
    fn require_value_rejects_missing_state() {
        let state = PropertyState::new(identifier("mass"));

        let error = state
            .require_value()
            .expect_err("missing value must be rejected");

        assert_eq!(
            error,
            PropertyStateError::MissingValue {
                name: identifier("mass"),
            }
        );
    }

    #[test]
    fn validation_accepts_uninitialized_state() {
        let state = PropertyState::new(identifier("mass"));

        assert!(state.validate().is_ok());
    }

    #[test]
    fn validation_accepts_initialized_state() {
        let state = PropertyState::with_value(
            identifier("mass"),
            quantity(10.0, "kg"),
        );

        assert!(state.validate().is_ok());
    }

    #[test]
    fn clear_value_is_idempotent() {
        let mut state = PropertyState::new(identifier("mass"));

        assert!(state.clear_value().is_none());
        assert!(state.clear_value().is_none());
    }
}
