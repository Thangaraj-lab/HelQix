//! Dimensions and units for HELQIX Core.
//!
//! This module answers two questions and nothing else: *what physical
//! dimension does a unit measure*, and *how does it scale relative to the
//! coherent SI unit of that dimension*. It carries no values (that is the job
//! of the quantity module), no physics, and no domain knowledge.
//!
//! # Dimensions
//!
//! A [`Dimension`] is a vector of seven `i8` exponents over the SI base
//! dimensions: length (L), mass (M), time (T), electric current (I),
//! temperature (Th), amount of substance (N) and luminous intensity (J).
//! Arithmetic on dimensions is checked; overflow yields `None` rather than
//! wrapping.
//!
//! # Units
//!
//! A [`Unit`] is looked up by symbol in a fixed, compiled-in registry. There is
//! no prefix parser and no expression parser: every accepted symbol is listed
//! explicitly, so the accepted vocabulary is auditable and deterministic.
//! Symbols are case-sensitive (`Pa` is a unit, `pa` is not).
//!
//! The registry contains only linear units (`value_in_coherent = value * scale`).
//! Affine units such as degrees Celsius need an offset and are intentionally
//! absent. Angles are dimensionless, so `rad`, `deg` and `one` are mutually
//! convertible.
//!
//! # Symbol validation order
//!
//! Failures are reported in a fixed order: empty, then the first non-ASCII-
//! alphanumeric character (zero-based `char` position), then unknown symbol.

use std::fmt;
use std::str::FromStr;

use crate::error::UnitError;

const BASE_DIMENSION_COUNT: usize = 7;

/// Display symbols for the base dimensions, in storage order.
const BASE_DIMENSION_SYMBOLS: [&str; BASE_DIMENSION_COUNT] = ["L", "M", "T", "I", "Th", "N", "J"];

// ---------------------------------------------------------------------------
// Dimension
// ---------------------------------------------------------------------------

/// A physical dimension expressed as exponents of the seven SI base dimensions.
///
/// Storage order is length, mass, time, current, temperature, amount,
/// luminous intensity. Two dimensions are equal exactly when all exponents are
/// equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Dimension {
    exponents: [i8; BASE_DIMENSION_COUNT],
}

impl Dimension {
    /// The dimension of a pure number.
    pub const DIMENSIONLESS: Self = Self::new(0, 0, 0, 0, 0, 0, 0);
    pub const LENGTH: Self = Self::new(1, 0, 0, 0, 0, 0, 0);
    pub const MASS: Self = Self::new(0, 1, 0, 0, 0, 0, 0);
    pub const TIME: Self = Self::new(0, 0, 1, 0, 0, 0, 0);
    pub const CURRENT: Self = Self::new(0, 0, 0, 1, 0, 0, 0);
    pub const TEMPERATURE: Self = Self::new(0, 0, 0, 0, 1, 0, 0);
    pub const AMOUNT: Self = Self::new(0, 0, 0, 0, 0, 1, 0);
    pub const LUMINOUS_INTENSITY: Self = Self::new(0, 0, 0, 0, 0, 0, 1);

    /// Builds a dimension from explicit exponents.
    #[must_use]
    pub const fn new(
        length: i8,
        mass: i8,
        time: i8,
        current: i8,
        temperature: i8,
        amount: i8,
        luminous_intensity: i8,
    ) -> Self {
        Self {
            exponents: [
                length,
                mass,
                time,
                current,
                temperature,
                amount,
                luminous_intensity,
            ],
        }
    }

    /// Returns the exponents in storage order (L, M, T, I, Th, N, J).
    #[must_use]
    pub const fn exponents(&self) -> [i8; BASE_DIMENSION_COUNT] {
        self.exponents
    }

    /// Returns `true` when every exponent is zero.
    #[must_use]
    pub fn is_dimensionless(&self) -> bool {
        self.exponents.iter().all(|&exponent| exponent == 0)
    }

    /// Dimension of a product: exponents are added. `None` on `i8` overflow.
    #[must_use]
    pub fn checked_mul(self, other: Self) -> Option<Self> {
        self.combine(other, i8::checked_add)
    }

    /// Dimension of a quotient: exponents are subtracted. `None` on `i8` overflow.
    #[must_use]
    pub fn checked_div(self, other: Self) -> Option<Self> {
        self.combine(other, i8::checked_sub)
    }

    /// Dimension of an integer power: exponents are multiplied.
    /// `None` on `i8` overflow.
    #[must_use]
    pub fn checked_pow(self, power: i8) -> Option<Self> {
        let mut exponents = [0_i8; BASE_DIMENSION_COUNT];
        for (out, base) in exponents.iter_mut().zip(self.exponents) {
            *out = base.checked_mul(power)?;
        }
        Some(Self { exponents })
    }

    fn combine(self, other: Self, op: fn(i8, i8) -> Option<i8>) -> Option<Self> {
        let mut exponents = [0_i8; BASE_DIMENSION_COUNT];
        for ((out, left), right) in exponents
            .iter_mut()
            .zip(self.exponents)
            .zip(other.exponents)
        {
            *out = op(left, right)?;
        }
        Some(Self { exponents })
    }
}

/// Renders non-zero exponents in storage order, separated by spaces, with
/// exponent `1` omitted: `L M T^-2`. A dimensionless value renders as `1`.
///
/// This rendering is what appears in [`UnitError::DimensionMismatch`].
impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut wrote_any = false;
        for (symbol, exponent) in BASE_DIMENSION_SYMBOLS.iter().zip(self.exponents) {
            if exponent == 0 {
                continue;
            }
            if wrote_any {
                f.write_str(" ")?;
            }
            wrote_any = true;
            if exponent == 1 {
                f.write_str(symbol)?;
            } else {
                write!(f, "{symbol}^{exponent}")?;
            }
        }
        if !wrote_any {
            f.write_str("1")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

struct Definition {
    symbol: &'static str,
    dimension: Dimension,
    /// Value of one unit expressed in the coherent SI unit of its dimension.
    scale: f64,
}

const fn define(symbol: &'static str, dimension: Dimension, scale: f64) -> Definition {
    Definition {
        symbol,
        dimension,
        scale,
    }
}

const FORCE: Dimension = Dimension::new(1, 1, -2, 0, 0, 0, 0);
const PRESSURE: Dimension = Dimension::new(-1, 1, -2, 0, 0, 0, 0);
const ENERGY: Dimension = Dimension::new(2, 1, -2, 0, 0, 0, 0);
const POWER: Dimension = Dimension::new(2, 1, -3, 0, 0, 0, 0);
const FREQUENCY: Dimension = Dimension::new(0, 0, -1, 0, 0, 0, 0);
const CHARGE: Dimension = Dimension::new(0, 0, 1, 1, 0, 0, 0);
const VOLTAGE: Dimension = Dimension::new(2, 1, -3, -1, 0, 0, 0);
const RESISTANCE: Dimension = Dimension::new(2, 1, -3, -2, 0, 0, 0);

/// Every unit HELQIX Core recognises. Order is irrelevant to behaviour; symbols
/// are unique (enforced by a test).
static REGISTRY: &[Definition] = &[
    // Dimensionless
    define("one", Dimension::DIMENSIONLESS, 1.0),
    define("rad", Dimension::DIMENSIONLESS, 1.0),
    define("deg", Dimension::DIMENSIONLESS, std::f64::consts::PI / 180.0),
    // Length (coherent: m)
    define("m", Dimension::LENGTH, 1.0),
    define("mm", Dimension::LENGTH, 1e-3),
    define("cm", Dimension::LENGTH, 1e-2),
    define("km", Dimension::LENGTH, 1e3),
    // Mass (coherent: kg)
    define("kg", Dimension::MASS, 1.0),
    define("g", Dimension::MASS, 1e-3),
    define("mg", Dimension::MASS, 1e-6),
    // Time (coherent: s)
    define("s", Dimension::TIME, 1.0),
    define("ms", Dimension::TIME, 1e-3),
    define("us", Dimension::TIME, 1e-6),
    define("min", Dimension::TIME, 60.0),
    define("h", Dimension::TIME, 3600.0),
    // Electric current (coherent: A)
    define("A", Dimension::CURRENT, 1.0),
    define("mA", Dimension::CURRENT, 1e-3),
    // Temperature (coherent: K; linear units only)
    define("K", Dimension::TEMPERATURE, 1.0),
    // Amount of substance (coherent: mol)
    define("mol", Dimension::AMOUNT, 1.0),
    // Luminous intensity (coherent: cd)
    define("cd", Dimension::LUMINOUS_INTENSITY, 1.0),
    // Frequency
    define("Hz", FREQUENCY, 1.0),
    define("kHz", FREQUENCY, 1e3),
    define("MHz", FREQUENCY, 1e6),
    // Force
    define("N", FORCE, 1.0),
    // Pressure
    define("Pa", PRESSURE, 1.0),
    define("kPa", PRESSURE, 1e3),
    // Energy
    define("J", ENERGY, 1.0),
    define("kJ", ENERGY, 1e3),
    // Power
    define("W", POWER, 1.0),
    define("kW", POWER, 1e3),
    // Electric charge
    define("C", CHARGE, 1.0),
    // Voltage
    define("V", VOLTAGE, 1.0),
    define("mV", VOLTAGE, 1e-3),
    // Resistance
    define("ohm", RESISTANCE, 1.0),
];

// ---------------------------------------------------------------------------
// Unit
// ---------------------------------------------------------------------------

/// A registered unit: a symbol, the dimension it measures, and its scale
/// relative to the coherent SI unit of that dimension.
///
/// The only way to obtain a `Unit` is [`Unit::parse`] (or `FromStr` /
/// `TryFrom<&str>`), so a `Unit` always refers to a registry entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unit {
    symbol: &'static str,
    dimension: Dimension,
    scale: f64,
}

impl Unit {
    /// Looks up a unit by symbol.
    ///
    /// # Errors
    ///
    /// - [`UnitError::EmptySymbol`] if `symbol` is empty.
    /// - [`UnitError::MalformedSymbol`] at the first character that is not an
    ///   ASCII letter or digit.
    /// - [`UnitError::UnknownSymbol`] if the symbol is well-formed but not in
    ///   the registry.
    pub fn parse(symbol: &str) -> Result<Self, UnitError> {
        validate_symbol(symbol)?;
        REGISTRY
            .iter()
            .find(|definition| definition.symbol == symbol)
            .map(|definition| Self {
                symbol: definition.symbol,
                dimension: definition.dimension,
                scale: definition.scale,
            })
            .ok_or_else(|| UnitError::UnknownSymbol {
                symbol: symbol.to_owned(),
            })
    }

    /// Returns the unit symbol.
    #[must_use]
    pub const fn symbol(&self) -> &'static str {
        self.symbol
    }

    /// Returns the dimension this unit measures.
    #[must_use]
    pub const fn dimension(&self) -> Dimension {
        self.dimension
    }

    /// Returns the value of one of this unit expressed in the coherent SI unit
    /// of its dimension (for example `1.0e3` for `km`).
    #[must_use]
    pub const fn scale_to_coherent(&self) -> f64 {
        self.scale
    }

    /// Returns the factor `f` such that `value_in_target = value_in_self * f`.
    ///
    /// # Errors
    ///
    /// [`UnitError::DimensionMismatch`] if the dimensions differ. `expected` is
    /// the target's dimension and `found` is this unit's dimension, each
    /// rendered with [`Dimension`]'s `Display`.
    pub fn conversion_factor(&self, target: &Self) -> Result<f64, UnitError> {
        if self.dimension != target.dimension {
            return Err(UnitError::DimensionMismatch {
                expected: target.dimension.to_string(),
                found: self.dimension.to_string(),
            });
        }
        Ok(self.scale / target.scale)
    }
}

/// Checks the symbol grammar: non-empty, ASCII letters and digits only.
fn validate_symbol(symbol: &str) -> Result<(), UnitError> {
    if symbol.is_empty() {
        return Err(UnitError::EmptySymbol);
    }
    match symbol
        .chars()
        .position(|character| !character.is_ascii_alphanumeric())
    {
        Some(position) => Err(UnitError::MalformedSymbol {
            symbol: symbol.to_owned(),
            position,
        }),
        None => Ok(()),
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.symbol)
    }
}

impl FromStr for Unit {
    type Err = UnitError;

    fn from_str(symbol: &str) -> Result<Self, Self::Err> {
        Self::parse(symbol)
    }
}

impl TryFrom<&str> for Unit {
    type Error = UnitError;

    fn try_from(symbol: &str) -> Result<Self, Self::Error> {
        Self::parse(symbol)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::error::{CoreError, CoreResult};

    fn unit(symbol: &str) -> Unit {
        Unit::parse(symbol).unwrap_or_else(|error| panic!("`{symbol}` should parse: {error}"))
    }

    fn assert_close(actual: f64, expected: f64) {
        let tolerance = expected.abs() * 1e-12;
        assert!(
            (actual - expected).abs() <= tolerance,
            "{actual} is not close to {expected}"
        );
    }

    // ----- Dimension -------------------------------------------------------

    #[test]
    fn dimensionless_is_all_zero() {
        assert!(Dimension::DIMENSIONLESS.is_dimensionless());
        assert_eq!(Dimension::DIMENSIONLESS.exponents(), [0; 7]);
        assert!(!Dimension::LENGTH.is_dimensionless());
    }

    #[test]
    fn base_dimensions_occupy_distinct_slots() {
        assert_eq!(Dimension::LENGTH.exponents(), [1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(Dimension::MASS.exponents(), [0, 1, 0, 0, 0, 0, 0]);
        assert_eq!(Dimension::TIME.exponents(), [0, 0, 1, 0, 0, 0, 0]);
        assert_eq!(Dimension::CURRENT.exponents(), [0, 0, 0, 1, 0, 0, 0]);
        assert_eq!(Dimension::TEMPERATURE.exponents(), [0, 0, 0, 0, 1, 0, 0]);
        assert_eq!(Dimension::AMOUNT.exponents(), [0, 0, 0, 0, 0, 1, 0]);
        assert_eq!(
            Dimension::LUMINOUS_INTENSITY.exponents(),
            [0, 0, 0, 0, 0, 0, 1]
        );
    }

    #[test]
    fn arithmetic_adds_subtracts_and_scales_exponents() {
        let area = Dimension::LENGTH.checked_mul(Dimension::LENGTH).unwrap();
        assert_eq!(area, Dimension::LENGTH.checked_pow(2).unwrap());
        assert_eq!(
            area.checked_div(Dimension::LENGTH).unwrap(),
            Dimension::LENGTH
        );
        assert_eq!(
            Dimension::LENGTH.checked_pow(0).unwrap(),
            Dimension::DIMENSIONLESS
        );
        assert_eq!(
            Dimension::TIME.checked_pow(-2).unwrap().exponents(),
            [0, 0, -2, 0, 0, 0, 0]
        );
    }

    #[test]
    fn arithmetic_reports_overflow_instead_of_wrapping() {
        let extreme = Dimension::new(127, 0, 0, 0, 0, 0, 0);
        assert_eq!(extreme.checked_mul(Dimension::LENGTH), None);
        assert_eq!(Dimension::new(-128, 0, 0, 0, 0, 0, 0).checked_div(Dimension::LENGTH), None);
        assert_eq!(Dimension::new(2, 0, 0, 0, 0, 0, 0).checked_pow(64), None);
        assert_eq!(
            Dimension::LENGTH.checked_pow(127),
            Some(Dimension::new(127, 0, 0, 0, 0, 0, 0))
        );
    }

    #[test]
    fn display_renders_non_zero_exponents_in_storage_order() {
        assert_eq!(Dimension::DIMENSIONLESS.to_string(), "1");
        assert_eq!(Dimension::LENGTH.to_string(), "L");
        assert_eq!(Dimension::TEMPERATURE.to_string(), "Th");
        assert_eq!(Dimension::AMOUNT.to_string(), "N");
        assert_eq!(Dimension::LUMINOUS_INTENSITY.to_string(), "J");
        assert_eq!(FORCE.to_string(), "L M T^-2");
        assert_eq!(VOLTAGE.to_string(), "L^2 M T^-3 I^-1");
        assert_eq!(FREQUENCY.to_string(), "T^-1");
    }

    // ----- Symbol validation ----------------------------------------------

    #[test]
    fn rejects_empty_symbol() {
        assert_eq!(Unit::parse("").unwrap_err(), UnitError::EmptySymbol);
    }

    #[test]
    fn malformed_symbols_report_char_position_of_first_bad_character() {
        assert_eq!(
            Unit::parse("m/s").unwrap_err(),
            UnitError::MalformedSymbol {
                symbol: "m/s".to_owned(),
                position: 1,
            }
        );
        assert_eq!(
            Unit::parse(" m").unwrap_err(),
            UnitError::MalformedSymbol {
                symbol: " m".to_owned(),
                position: 0,
            }
        );
        assert_eq!(
            Unit::parse("µs").unwrap_err(),
            UnitError::MalformedSymbol {
                symbol: "µs".to_owned(),
                position: 0,
            }
        );
        // Position counts chars, not bytes: 'é' is two bytes, one char.
        assert_eq!(
            Unit::parse("aé").unwrap_err(),
            UnitError::MalformedSymbol {
                symbol: "aé".to_owned(),
                position: 1,
            }
        );
    }

    #[test]
    fn malformed_is_reported_before_unknown() {
        // "furlong-" is also unknown, but the bad character wins.
        assert_eq!(
            Unit::parse("furlong-").unwrap_err(),
            UnitError::MalformedSymbol {
                symbol: "furlong-".to_owned(),
                position: 7,
            }
        );
    }

    #[test]
    fn well_formed_but_unregistered_symbols_are_unknown() {
        assert_eq!(
            Unit::parse("furlong").unwrap_err(),
            UnitError::UnknownSymbol {
                symbol: "furlong".to_owned(),
            }
        );
        assert_eq!(
            Unit::parse("123").unwrap_err(),
            UnitError::UnknownSymbol {
                symbol: "123".to_owned(),
            }
        );
    }

    #[test]
    fn symbols_are_case_sensitive() {
        assert!(Unit::parse("Pa").is_ok());
        assert!(matches!(
            Unit::parse("pa").unwrap_err(),
            UnitError::UnknownSymbol { .. }
        ));
        assert!(matches!(
            Unit::parse("MM").unwrap_err(),
            UnitError::UnknownSymbol { .. }
        ));
        assert!(Unit::parse("MHz").is_ok());
        assert!(matches!(
            Unit::parse("mHz").unwrap_err(),
            UnitError::UnknownSymbol { .. }
        ));
    }

    // ----- Registry integrity ---------------------------------------------

    #[test]
    fn registry_symbols_are_unique_and_valid() {
        let mut seen = HashSet::new();
        for definition in REGISTRY {
            assert!(
                validate_symbol(definition.symbol).is_ok(),
                "`{}` violates the symbol grammar",
                definition.symbol
            );
            assert!(
                seen.insert(definition.symbol),
                "duplicate symbol `{}`",
                definition.symbol
            );
        }
    }

    #[test]
    fn registry_scales_are_finite_and_positive() {
        for definition in REGISTRY {
            assert!(
                definition.scale.is_finite() && definition.scale > 0.0,
                "`{}` has invalid scale {}",
                definition.symbol,
                definition.scale
            );
        }
    }

    #[test]
    fn every_registry_entry_round_trips_through_parse() {
        for definition in REGISTRY {
            let parsed = unit(definition.symbol);
            assert_eq!(parsed.symbol(), definition.symbol);
            assert_eq!(parsed.dimension(), definition.dimension);
            assert_eq!(parsed.scale_to_coherent(), definition.scale);
            assert_eq!(parsed.to_string(), definition.symbol);
        }
    }

    #[test]
    fn derived_units_are_dimensionally_coherent() {
        let metre = unit("m").dimension();
        let kilogram = unit("kg").dimension();
        let second = unit("s").dimension();
        let ampere = unit("A").dimension();

        // N = kg m / s^2
        let newton = kilogram
            .checked_mul(metre)
            .and_then(|d| d.checked_div(second.checked_pow(2)?))
            .unwrap();
        assert_eq!(unit("N").dimension(), newton);

        // Pa = N / m^2
        let pascal = newton.checked_div(metre.checked_pow(2).unwrap()).unwrap();
        assert_eq!(unit("Pa").dimension(), pascal);

        // J = N m
        let joule = newton.checked_mul(metre).unwrap();
        assert_eq!(unit("J").dimension(), joule);

        // W = J / s
        let watt = joule.checked_div(second).unwrap();
        assert_eq!(unit("W").dimension(), watt);

        // Hz = 1 / s
        let hertz = Dimension::DIMENSIONLESS.checked_div(second).unwrap();
        assert_eq!(unit("Hz").dimension(), hertz);

        // C = A s
        let coulomb = ampere.checked_mul(second).unwrap();
        assert_eq!(unit("C").dimension(), coulomb);

        // V = W / A
        let volt = watt.checked_div(ampere).unwrap();
        assert_eq!(unit("V").dimension(), volt);

        // ohm = V / A
        let ohm = volt.checked_div(ampere).unwrap();
        assert_eq!(unit("ohm").dimension(), ohm);
    }

    // ----- Conversion ------------------------------------------------------

    #[test]
    fn conversion_factor_between_compatible_units() {
        assert_close(unit("km").conversion_factor(&unit("m")).unwrap(), 1e3);
        assert_close(unit("m").conversion_factor(&unit("mm")).unwrap(), 1e3);
        assert_close(unit("h").conversion_factor(&unit("min")).unwrap(), 60.0);
        assert_close(unit("min").conversion_factor(&unit("s")).unwrap(), 60.0);
        assert_close(unit("kJ").conversion_factor(&unit("J")).unwrap(), 1e3);
        assert_close(unit("g").conversion_factor(&unit("kg")).unwrap(), 1e-3);
        assert_close(
            unit("deg").conversion_factor(&unit("rad")).unwrap(),
            std::f64::consts::PI / 180.0,
        );
    }

    #[test]
    fn conversion_to_self_is_identity() {
        for definition in REGISTRY {
            let u = unit(definition.symbol);
            assert_eq!(u.conversion_factor(&u).unwrap(), 1.0);
        }
    }

    #[test]
    fn conversion_factors_are_mutually_inverse_within_a_dimension() {
        for left in REGISTRY {
            for right in REGISTRY {
                if left.dimension != right.dimension {
                    continue;
                }
                let a = unit(left.symbol);
                let b = unit(right.symbol);
                let forward = a.conversion_factor(&b).unwrap();
                let backward = b.conversion_factor(&a).unwrap();
                assert_close(forward * backward, 1.0);
            }
        }
    }

    #[test]
    fn conversion_across_dimensions_reports_both_dimensions() {
        let error = unit("m").conversion_factor(&unit("s")).unwrap_err();
        assert_eq!(
            error,
            UnitError::DimensionMismatch {
                expected: "T".to_owned(),
                found: "L".to_owned(),
            }
        );

        let error = unit("N").conversion_factor(&unit("Pa")).unwrap_err();
        assert_eq!(
            error,
            UnitError::DimensionMismatch {
                expected: "L^-1 M T^-2".to_owned(),
                found: "L M T^-2".to_owned(),
            }
        );
    }

    #[test]
    fn dimensionless_units_interconvert_but_not_with_dimensional_units() {
        assert!(unit("rad").conversion_factor(&unit("one")).is_ok());
        assert!(unit("deg").conversion_factor(&unit("one")).is_ok());
        assert!(unit("rad").conversion_factor(&unit("m")).is_err());
    }

    // ----- Construction paths and propagation ------------------------------

    #[test]
    fn all_construction_paths_agree() {
        let parsed = Unit::parse("kPa").unwrap();
        let from_str: Unit = "kPa".parse().unwrap();
        let from_ref = Unit::try_from("kPa").unwrap();
        assert_eq!(parsed, from_str);
        assert_eq!(parsed, from_ref);
        assert_eq!(
            "nope".parse::<Unit>().unwrap_err(),
            UnitError::UnknownSymbol {
                symbol: "nope".to_owned(),
            }
        );
    }

    #[test]
    fn parsing_is_deterministic() {
        assert_eq!(Unit::parse("W"), Unit::parse("W"));
        assert_eq!(Unit::parse("W/"), Unit::parse("W/"));
    }

    #[test]
    fn unit_error_propagates_into_core_error_with_question_mark() {
        fn factor(from: &str, to: &str) -> CoreResult<f64> {
            Ok(Unit::parse(from)?.conversion_factor(&Unit::parse(to)?)?)
        }

        assert!(matches!(
            factor("m", "furlong").unwrap_err(),
            CoreError::Unit(UnitError::UnknownSymbol { .. })
        ));
        assert!(matches!(
            factor("m", "s").unwrap_err(),
            CoreError::Unit(UnitError::DimensionMismatch { .. })
        ));
        assert_close(factor("km", "m").unwrap(), 1e3);
    }
}