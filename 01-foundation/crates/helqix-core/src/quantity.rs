//! Engineering quantities for HELQIX Core.
//!
//! A quantity combines a finite numeric value with a validated [`Unit`].
//!
//! This module is intentionally limited to:
//! - storing a numeric value;
//! - associating that value with a unit;
//! - validating numeric finiteness;
//! - converting between compatible units.
//!
//! Domain physics, equations, runtime state, and simulation do not belong here.

use crate::error::{NonFiniteKind, QuantityError};
use crate::units::Unit;

/// A finite numeric value associated with a unit.
///
/// `Quantity` is the value layer above [`Unit`]. `Unit` defines the physical
/// meaning and scaling; `Quantity` supplies the actual numeric value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantity {
    value: f64,
    unit: Unit,
}

impl Quantity {
    /// Creates a new quantity.
    ///
    /// # Errors
    ///
    /// Returns [`QuantityError::NonFinite`] when `value` is NaN or infinite.
    pub fn new(value: f64, unit: Unit) -> Result<Self, QuantityError> {
        validate_finite(value)?;

        Ok(Self { value, unit })
    }

    /// Returns the numeric value.
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.value
    }

    /// Returns the unit.
    #[must_use]
    pub const fn unit(&self) -> &Unit {
        &self.unit
    }

    /// Returns the value expressed in `target`.
    ///
    /// The target unit must have the same dimension as the current unit.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::UnitError`] when the units are incompatible.
    pub fn value_in(
        &self,
        target: &Unit,
    ) -> Result<f64, crate::error::UnitError> {
        let factor = self.unit.conversion_factor(target)?;
        Ok(self.value * factor)
    }

    /// Returns this quantity converted to `target`.
    ///
    /// # Errors
    ///
    /// Returns a unit error when the units have incompatible dimensions.
    /// Returns a quantity error if conversion produces a non-finite value.
    pub fn convert_to(
        &self,
        target: Unit,
    ) -> Result<Self, QuantityConversionError> {
        let value = self
            .value_in(&target)
            .map_err(QuantityConversionError::Unit)?;

        validate_finite(value)
            .map_err(QuantityConversionError::Quantity)?;

        Ok(Self {
            value,
            unit: target,
        })
    }

    /// Validates the quantity against an inclusive numeric range.
    ///
    /// The range is expressed in the quantity's current unit.
    ///
    /// # Errors
    ///
    /// Returns [`QuantityError::OutOfRange`] when the value is outside
    /// `[min, max]`.
    pub fn validate_range(
        &self,
        min: f64,
        max: f64,
    ) -> Result<(), QuantityError> {
        validate_finite(min)?;
        validate_finite(max)?;

        if min > max || self.value < min || self.value > max {
            return Err(QuantityError::OutOfRange {
                value: self.value,
                min,
                max,
            });
        }

        Ok(())
    }
}

/// Failure while converting a quantity.
///
/// Conversion can fail either because the units are incompatible or because
/// the resulting numeric value is not finite.
#[derive(Debug, Clone, PartialEq)]
pub enum QuantityConversionError {
    /// The source and target units are incompatible.
    Unit(crate::error::UnitError),

    /// The converted numeric value is invalid.
    Quantity(QuantityError),
}

impl std::fmt::Display for QuantityConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit(error) => error.fmt(f),
            Self::Quantity(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for QuantityConversionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Unit(error) => Some(error),
            Self::Quantity(error) => Some(error),
        }
    }
}

impl From<crate::error::UnitError> for QuantityConversionError {
    fn from(error: crate::error::UnitError) -> Self {
        Self::Unit(error)
    }
}

impl From<QuantityError> for QuantityConversionError {
    fn from(error: QuantityError) -> Self {
        Self::Quantity(error)
    }
}

/// Validates that a floating-point value is finite.
fn validate_finite(value: f64) -> Result<(), QuantityError> {
    match NonFiniteKind::classify(value) {
        Some(kind) => Err(QuantityError::NonFinite(kind)),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::UnitError;

    fn metre() -> Unit {
        Unit::parse("m").expect("metre must be a registered unit")
    }

    #[test]
    fn creates_finite_quantity() {
        let quantity = Quantity::new(12.5, metre()).unwrap();

        assert_eq!(quantity.value(), 12.5);
        assert_eq!(quantity.unit().symbol(), "m");
    }

    #[test]
    fn rejects_nan() {
        let error = Quantity::new(f64::NAN, metre()).unwrap_err();

        assert_eq!(
            error,
            QuantityError::NonFinite(NonFiniteKind::Nan)
        );
    }

    #[test]
    fn rejects_positive_infinity() {
        let error = Quantity::new(f64::INFINITY, metre()).unwrap_err();

        assert_eq!(
            error,
            QuantityError::NonFinite(NonFiniteKind::PositiveInfinity)
        );
    }

    #[test]
    fn rejects_negative_infinity() {
        let error = Quantity::new(f64::NEG_INFINITY, metre()).unwrap_err();

        assert_eq!(
            error,
            QuantityError::NonFinite(NonFiniteKind::NegativeInfinity)
        );
    }

    #[test]
    fn preserves_value_and_unit() {
        let quantity = Quantity::new(42.0, metre()).unwrap();

        assert_eq!(quantity.value(), 42.0);
        assert_eq!(quantity.unit().symbol(), "m");
    }

    #[test]
    fn validates_inclusive_range() {
        let quantity = Quantity::new(10.0, metre()).unwrap();

        assert!(quantity.validate_range(0.0, 10.0).is_ok());
        assert!(quantity.validate_range(10.0, 20.0).is_ok());
    }

    #[test]
    fn rejects_value_below_range() {
        let quantity = Quantity::new(2.0, metre()).unwrap();

        assert_eq!(
            quantity.validate_range(3.0, 10.0).unwrap_err(),
            QuantityError::OutOfRange {
                value: 2.0,
                min: 3.0,
                max: 10.0,
            }
        );
    }

    #[test]
    fn rejects_value_above_range() {
        let quantity = Quantity::new(12.0, metre()).unwrap();

        assert_eq!(
            quantity.validate_range(0.0, 10.0).unwrap_err(),
            QuantityError::OutOfRange {
                value: 12.0,
                min: 0.0,
                max: 10.0,
            }
        );
    }

    #[test]
    fn rejects_inverted_range() {
        let quantity = Quantity::new(5.0, metre()).unwrap();

        assert_eq!(
            quantity.validate_range(10.0, 0.0).unwrap_err(),
            QuantityError::OutOfRange {
                value: 5.0,
                min: 10.0,
                max: 0.0,
            }
        );
    }

    #[test]
    fn conversion_uses_unit_conversion_factor() {
        let source = Quantity::new(1.0, Unit::parse("km").unwrap()).unwrap();
        let target = Unit::parse("m").unwrap();

        let converted = source.convert_to(target).unwrap();

        assert_eq!(converted.unit().symbol(), "m");
        assert_eq!(converted.value(), 1000.0);
    }

    #[test]
    fn incompatible_conversion_is_rejected() {
        let source = Quantity::new(1.0, Unit::parse("m").unwrap()).unwrap();
        let target = Unit::parse("s").unwrap();

        let error = source.convert_to(target).unwrap_err();

        assert!(matches!(
            error,
            QuantityConversionError::Unit(
                UnitError::DimensionMismatch { .. }
            )
        ));
    }

    #[test]
    fn value_in_does_not_modify_quantity() {
        let source = Quantity::new(2.0, Unit::parse("km").unwrap()).unwrap();
        let target = Unit::parse("m").unwrap();

        assert_eq!(source.value_in(&target).unwrap(), 2000.0);
        assert_eq!(source.value(), 2.0);
        assert_eq!(source.unit().symbol(), "km");
    }

    #[test]
    fn quantity_is_copy() {
        let original = Quantity::new(5.0, metre()).unwrap();
        let copied = original;

        assert_eq!(original, copied);
    }
}