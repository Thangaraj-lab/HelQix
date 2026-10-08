//! HELIXSIM environmental conditions (Layer 06).
//!
//! An [`Environment`] is a pure, immutable description of the physical
//! surroundings acting on a simulated system at one instant of logical time:
//!
//! * a gravitational field acceleration **g** [m/s²];
//! * medium mass density ρ [kg/m³] with its unit recorded as `SymbolicUnit`;
//! * optional medium static pressure p and absolute temperature T;
//! * a bulk medium (wind) velocity **u** [m/s].
//!
//! Scalars reuse the canonical Foundation [`Quantity`] (finite value +
//! validated [`Unit`]). Vectors are component-wise `[f64; 3]` in coherent SI
//! base units plus an explicit [`SymbolicUnit`] — the smallest
//! architecture-compatible representation, since Layer 04's spatial module
//! does not yet provide concrete types.
//!
//! # Dimensional discipline
//!
//! Every vector declares the dimension it measures; construction rejects
//! values whose declared dimension cannot be composed from the seven SI base
//! exponents (checked arithmetic — no silent wrapping). Derived quantities
//! expose their unit signature so downstream layers can verify consistency:
//!
//! ```text
//! gravity   g : L¹ T⁻²          → m·s⁻²
//! velocity  u : L¹ T⁻¹          → m·s⁻¹
//! density   ρ : M¹ L⁻³ T⁰       → kg·m⁻³
//! pressure  p : M¹ L⁻¹ T⁻²      → Pa = kg·m⁻¹·s⁻²
//! ```
//!
//! # Physical relations provided here
//!
//! Only relations that follow directly from the stored fields:
//!
//! * weight per unit mass — the gravity vector itself (`F_g = m·g` is applied
//!   by the `forces` module, never here);
//! * dynamic pressure of the medium relative to a body velocity:
//!   `q = ½ · ρ · |v_rel|²`, `v_rel = u − v`.
//!
//! # Layer boundary
//!
//! This module does NOT mutate authoritative Layer 04 state, integrate
//! motion, advance logical time (Layer 07), or execute actions (Layer 05).
//! Evaluation is deterministic: identical inputs yield bit-identical outputs;
//! no wall-clock, randomness, or global state is involved. Non-finite results
//! are refused as errors, never leaked.

use std::fmt;

use helqix_core::{NonFiniteKind, Quantity};

// ---------------------------------------------------------------------------
// Symbolic units
// ---------------------------------------------------------------------------

/// A signed product of SI base-unit powers: `kg^M · m^L · s^T · A^I · K^Th ·
/// mol^N · cd^J`.
///
/// The Foundation unit registry contains only single-symbol linear units, so
/// compound dimensions such as kg/m³ cannot be expressed as a [`Unit`].
/// `SymbolicUnit` fills exactly that gap for HELIXSIM vectors and derived
/// scalars while remaining pure data: total equality, deterministic display,
/// checked exponent algebra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SymbolicUnit {
    exponents: [i8; BASE_DIMENSION_COUNT],
}

/// Number of SI base dimensions tracked by Foundation's [`helqix_core::Dimension`].
const BASE_DIMENSION_COUNT: usize = 7;

impl SymbolicUnit {
    /// The dimensionless unit `1`.
    pub const DIMENSIONLESS: Self = Self::new([0; BASE_DIMENSION_COUNT]);
    /// Metre, `m`.
    pub const METRE: Self = Self::from_length(1);
    /// Kilogram, `kg`.
    pub const KILOGRAM: Self = Self::from_mass(1);
    /// Second, `s`.
    pub const SECOND: Self = Self::from_time(1);
    /// Kelvin, `K`.
    pub const KELVIN: Self = Self::from_temperature(1);

    /// Creates a symbolic unit from base exponents `[L, M, T, I, Th, N, J]`
    /// (same storage order as Foundation `Dimension`).
    #[must_use]
    pub const fn new(exponents: [i8; BASE_DIMENSION_COUNT]) -> Self {
        Self { exponents }
    }

    /// Builds `A^n` (electric current).
    #[must_use]
    pub const fn from_current(n: i8) -> Self {
        let mut e = [0i8; BASE_DIMENSION_COUNT];
        e[3] = n;
        Self::new(e)
    }

    /// Builds `mol^n` (amount of substance).
    #[must_use]
    pub const fn from_amount(n: i8) -> Self {
        let mut e = [0i8; BASE_DIMENSION_COUNT];
        e[5] = n;
        Self::new(e)
    }

    /// Builds `cd^n` (luminous intensity).
    #[must_use]
    pub const fn from_luminosity(n: i8) -> Self {
        let mut e = [0i8; BASE_DIMENSION_COUNT];
        e[6] = n;
        Self::new(e)
    }

    /// Builds `m^n`.
    #[must_use]
    pub const fn from_length(n: i8) -> Self {
        let mut e = [0i8; BASE_DIMENSION_COUNT];
        e[0] = n;
        Self::new(e)
    }

    /// Builds `kg^n`.
    #[must_use]
    pub const fn from_mass(n: i8) -> Self {
        let mut e = [0i8; BASE_DIMENSION_COUNT];
        e[1] = n;
        Self::new(e)
    }

    /// Builds `s^n`.
    #[must_use]
    pub const fn from_time(n: i8) -> Self {
        let mut e = [0i8; BASE_DIMENSION_COUNT];
        e[2] = n;
        Self::new(e)
    }

    /// Builds `K^n`.
    #[must_use]
    pub const fn from_temperature(n: i8) -> Self {
        let mut e = [0i8; BASE_DIMENSION_COUNT];
        e[4] = n;
        Self::new(e)
    }

    /// Returns the raw exponent vector.
    #[must_use]
    pub const fn exponents(&self) -> [i8; BASE_DIMENSION_COUNT] {
        self.exponents
    }

    /// Returns whether this unit is dimensionless.
    #[must_use]
    pub fn is_dimensionless(&self) -> bool {
        self.exponents.iter().all(|e| *e == 0)
    }

    /// Checked multiplication of unit signatures (`self · other`).
    #[must_use]
    pub fn checked_mul(self, other: Self) -> Option<Self> {
        let mut out = [0i8; BASE_DIMENSION_COUNT];
        let mut i = 0;
        while i < BASE_DIMENSION_COUNT {
            let Some(sum) = self.exponents[i].checked_add(other.exponents[i]) else {
                return None;
            };
            out[i] = sum;
            i += 1;
        }
        Some(Self::new(out))
    }

    /// Checked division of unit signatures (`self / other`).
    #[must_use]
    pub fn checked_div(self, other: Self) -> Option<Self> {
        let mut out = [0i8; BASE_DIMENSION_COUNT];
        let mut i = 0;
        while i < BASE_DIMENSION_COUNT {
            let Some(diff) = self.exponents[i].checked_sub(other.exponents[i]) else {
                return None;
            };
            out[i] = diff;
            i += 1;
        }
        Some(Self::new(out))
    }

    /// Checked exponentiation of a unit signature (`self^n`).
    #[must_use]
    pub fn checked_pow(self, power: i8) -> Option<Self> {
        let mut out = [0i8; BASE_DIMENSION_COUNT];
        let mut i = 0;
        while i < BASE_DIMENSION_COUNT {
            let Some(prod) = self.exponents[i].checked_mul(power) else {
                return None;
            };
            out[i] = prod;
            i += 1;
        }
        Some(Self::new(out))
    }

    /// Coherent symbolic unit of a Foundation `Unit`, matching its dimension
    /// when the dimension is expressible as base powers, e.g. `N → kg·m·s⁻²`.
    #[must_use]
    pub fn from_dimension(dimension: &helqix_core::Dimension) -> Self {
        Self::new(dimension.exponents())
    }
}

impl fmt::Display for SymbolicUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const NAMES: [&str; BASE_DIMENSION_COUNT] = ["m", "kg", "s", "A", "K", "mol", "cd"];
        if self.is_dimensionless() {
            return write!(f, "1");
        }
        let mut first = true;
        for (name, exponent) in NAMES.iter().zip(self.exponents) {
            if exponent == 0 {
                continue;
            }
            if !first {
                write!(f, "·")?;
            }
            if exponent == 1 {
                write!(f, "{name}")?;
            } else {
                write!(f, "{name}^{exponent}")?;
            }
            first = false;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Vector3
// ---------------------------------------------------------------------------

/// A three-dimensional vector: components in coherent SI base magnitudes,
/// plus the explicit [`SymbolicUnit`] they are expressed in.
///
/// Components are plain `f64` because the Foundation [`Quantity`] cannot
/// represent compound units; finiteness is therefore enforced at every
/// boundary (construction and every arithmetic result).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3 {
    components: [f64; 3],
    unit: SymbolicUnit,
}

impl Vector3 {
    /// Crate-internal constructor for components already verified finite by
    /// downstream HELIXSIM modules (forces, dynamics, integration). Finiteness
    /// of every element is re-checked here as the single choke point.
    ///
    /// # Errors
    ///
    /// [`EnvironmentError::NonFiniteValue`] if any component is NaN or
    /// infinite.
    pub(crate) fn new_checked(
        components: [f64; 3],
        unit: SymbolicUnit,
    ) -> Result<Self, EnvironmentError> {
        Self::new(components, unit)
    }

    /// Creates a validated vector.
    ///
    /// # Errors
    ///
    /// [`EnvironmentError::NonFiniteValue`] if any component is NaN or
    /// infinite.
    pub fn new(components: [f64; 3], unit: SymbolicUnit) -> Result<Self, EnvironmentError> {
        for c in components {
            require_finite(c)?;
        }
        Ok(Self { components, unit })
    }

    /// The zero vector in `unit`.
    #[must_use]
    pub const fn zero(unit: SymbolicUnit) -> Self {
        Self {
            components: [0.0; 3],
            unit,
        }
    }

    /// Returns the components (x, y, z) in coherent SI magnitude.
    #[must_use]
    pub const fn components(&self) -> [f64; 3] {
        self.components
    }

    /// Returns the x component.
    #[must_use]
    pub const fn x(&self) -> f64 {
        self.components[0]
    }

    /// Returns the y component.
    #[must_use]
    pub const fn y(&self) -> f64 {
        self.components[1]
    }

    /// Returns the z component.
    #[must_use]
    pub const fn z(&self) -> f64 {
        self.components[2]
    }

    /// Returns the unit signature of the components.
    #[must_use]
    pub const fn unit(&self) -> SymbolicUnit {
        self.unit
    }

    /// Returns `true` when all components are exactly zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.components.iter().all(|c| *c == 0.0)
    }

    /// Requires that this vector carries `expected` unit signature.
    ///
    /// # Errors
    ///
    /// [`EnvironmentError::InconsistentUnits`] otherwise.
    pub fn require_unit(&self, expected: SymbolicUnit) -> Result<(), EnvironmentError> {
        if self.unit != expected {
            return Err(EnvironmentError::InconsistentUnits {
                first: self.unit.to_string(),
                second: expected.to_string(),
            });
        }
        Ok(())
    }

    /// Euclidean magnitude √(x²+y²+z²) in the same unit.
    ///
    /// Uses max-scaling so vectors with components near `f64::MAX` do not
    /// overflow spuriously.
    ///
    /// # Errors
    ///
    /// [`EnvironmentError::NonFiniteValue`] if the magnitude still overflows.
    #[must_use]
    pub fn magnitude(&self) -> Result<f64, EnvironmentError> {
        let scale = self
            .components
            .iter()
            .map(|c| c.abs())
            .fold(0.0_f64, f64::max);
        if scale == 0.0 {
            return Ok(0.0);
        }
        let n0 = self.components[0] / scale;
        let n1 = self.components[1] / scale;
        let n2 = self.components[2] / scale;
        require_finite(scale * (n0 * n0 + n1 * n1 + n2 * n2).sqrt())
    }

    /// Squared magnitude |v|² with unit signature `unit²`.
    ///
    /// # Errors
    ///
    /// [`EnvironmentError::NonFiniteValue`] on overflow,
    /// [`EnvironmentError::UnitOverflow`] if the squared signature cannot be
    /// represented.
    #[must_use]
    pub fn magnitude_squared(&self) -> Result<Scalar, EnvironmentError> {
        let value = require_finite(
            self.components[0] * self.components[0]
                + self.components[1] * self.components[1]
                + self.components[2] * self.components[2],
        )?;
        let unit = self
            .unit
            .checked_pow(2)
            .ok_or(EnvironmentError::UnitOverflow)?;
        Ok(Scalar::new_unchecked(value, unit))
    }

    /// Dot product `self · other`; requires equal unit signatures.
    ///
    /// # Errors
    ///
    /// [`EnvironmentError::InconsistentUnits`] or
    /// [`EnvironmentError::NonFiniteValue`] on overflow.
    #[must_use]
    pub fn dot(&self, other: &Self) -> Result<Scalar, EnvironmentError> {
        self.require_unit(other.unit)?;
        let value = require_finite(
            self.components[0] * other.components[0]
                + self.components[1] * other.components[1]
                + self.components[2] * other.components[2],
        )?;
        let unit = self
            .unit
            .checked_mul(other.unit)
            .ok_or(EnvironmentError::UnitOverflow)?;
        Ok(Scalar::new_unchecked(value, unit))
    }

    /// Cross product `self × other` with unit signature `unit·unit`.
    ///
    /// # Errors
    ///
    /// As for [`Vector3::dot`].
    #[must_use]
    pub fn cross(&self, other: &Self) -> Result<Self, EnvironmentError> {
        self.require_unit(other.unit)?;
        let a = self.components;
        let b = other.components;
        let products = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        for p in products {
            require_finite(p)?;
        }
        let unit = self
            .unit
            .checked_mul(other.unit)
            .ok_or(EnvironmentError::UnitOverflow)?;
        Ok(Self {
            components: products,
            unit,
        })
    }

    /// Component-wise addition; requires equal unit signatures.
    ///
    /// # Errors
    ///
    /// As for [`Vector3::dot`] (sum overflow is refused).
    #[must_use]
    pub fn add(&self, other: &Self) -> Result<Self, EnvironmentError> {
        self.combine(other, |a, b| a + b)
    }

    /// Component-wise subtraction `self − other`; requires equal units.
    #[must_use]
    pub fn sub(&self, other: &Self) -> Result<Self, EnvironmentError> {
        self.combine(other, |a, b| a - b)
    }

    /// Negation `-self`.
    #[must_use]
    pub fn neg(&self) -> Self {
        Self {
            components: [
                -self.components[0],
                -self.components[1],
                -self.components[2],
            ],
            unit: self.unit,
        }
    }

    /// Scaling by a dimensionless factor.
    ///
    /// # Errors
    ///
    /// [`EnvironmentError::NonFiniteValue`] when the scaled result overflows
    /// or the factor is non-finite.
    #[must_use]
    pub fn scale(&self, factor: f64) -> Result<Self, EnvironmentError> {
        require_finite(factor)?;
        let mut out = [0.0; 3];
        for (o, c) in out.iter_mut().zip(self.components) {
            *o = require_finite(c * factor)?;
        }
        Ok(Self {
            components: out,
            unit: self.unit,
        })
    }

    fn combine(
        &self,
        other: &Self,
        op: impl Fn(f64, f64) -> f64,
    ) -> Result<Self, EnvironmentError> {
        self.require_unit(other.unit)?;
        let mut out = [0.0; 3];
        for (o, (a, b)) in out
            .iter_mut()
            .zip(self.components.iter().zip(other.components.iter()))
        {
            *o = require_finite(op(*a, *b))?;
        }
        Ok(Self {
            components: out,
            unit: self.unit,
        })
    }
}

// ---------------------------------------------------------------------------
// Scalar
// ---------------------------------------------------------------------------

/// A finite scalar carrying a [`SymbolicUnit`] signature.
///
/// Used for derived quantities (density, dynamic pressure, mass flow, …)
/// whose compound units the Foundation registry cannot name. For quantities
/// expressible with a single registered symbol (temperature in K, force in N
/// when already reduced, …) callers may instead use the canonical
/// [`Quantity`]; both views coexist without a second scalar authority.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scalar {
    value: f64,
    unit: SymbolicUnit,
}

impl Scalar {
    /// Creates a validated scalar.
    ///
    /// # Errors
    ///
    /// [`EnvironmentError::NonFiniteValue`] for NaN/infinity.
    pub fn new(value: f64, unit: SymbolicUnit) -> Result<Self, EnvironmentError> {
        Ok(Self::new_unchecked(require_finite(value)?, unit))
    }

    /// Internal constructor after the caller has verified finiteness.
    #[must_use]
    pub(crate) const fn new_unchecked(value: f64, unit: SymbolicUnit) -> Self {
        Self { value, unit }
    }

    /// Returns the numeric value.
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.value
    }

    /// Returns the unit signature.
    #[must_use]
    pub const fn unit(&self) -> SymbolicUnit {
        self.unit
    }

    /// Multiplies two scalars, composing unit signatures.
    ///
    /// # Errors
    ///
    /// Refuses non-finite products and unrepresentable unit exponents.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Result<Self, EnvironmentError> {
        Ok(Self::new_unchecked(
            require_finite(self.value * other.value)?,
            self.unit
                .checked_mul(other.unit)
                .ok_or(EnvironmentError::UnitOverflow)?,
        ))
    }

    /// Divides two scalars, composing unit signatures.
    ///
    /// # Errors
    ///
    /// [`EnvironmentError::ZeroDenominator`] for `other.value == 0`,
    /// non-finite quotients, or unrepresentable unit exponents.
    #[must_use]
    pub fn div(&self, other: &Self) -> Result<Self, EnvironmentError> {
        if other.value == 0.0 {
            return Err(EnvironmentError::ZeroDenominator);
        }
        Ok(Self::new_unchecked(
            require_finite(self.value / other.value)?,
            self.unit
                .checked_div(other.unit)
                .ok_or(EnvironmentError::UnitOverflow)?,
        ))
    }

    /// Attempts to re-express this scalar as a canonical Foundation
    /// [`Quantity`] when the unit signature matches exactly one registered
    /// coherent symbol (scale 1.0), e.g. `K` for temperature.
    ///
    /// # Errors
    ///
    /// [`EnvironmentError::UnrepresentableUnit`] when no single registry
    /// symbol carries this exact dimension at scale 1.
    #[must_use]
    pub fn to_quantity(&self) -> Result<Quantity, EnvironmentError> {
        // Probe the registry for a coherent single-symbol match.
        for symbol in [
            "m", "kg", "s", "A", "K", "mol", "cd", "Hz", "N", "Pa", "J", "W", "C", "V", "ohm",
            "one",
        ] {
            if let Ok(unit) = helqix_core::Unit::parse(symbol) {
                if SymbolicUnit::from_dimension(&unit.dimension()) == self.unit
                    && unit.scale_to_coherent() == 1.0
                {
                    return Quantity::new(self.value, unit).map_err(EnvironmentError::from);
                }
            }
        }
        Err(EnvironmentError::UnrepresentableUnit(self.unit.to_string()))
    }
}

// ---------------------------------------------------------------------------
// GravityField
// ---------------------------------------------------------------------------

/// A uniform gravitational field acceleration **g** [m·s⁻²].
///
/// Direction is explicit in the components (e.g. `(0, -9.80665, 0)` with +y
/// up). The weight force `F = m·g` is computed by the `forces` module.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GravityField {
    acceleration: Vector3,
}

impl GravityField {
    /// Unit signature of gravitational acceleration: `m·s⁻²`.
    #[must_use]
    pub fn unit() -> SymbolicUnit {
        let length = SymbolicUnit::from_length(1);
        let inv_time_sq = SymbolicUnit::from_time(-2);
        length
            .checked_mul(inv_time_sq)
            .expect("internal: m·s⁻² is representable")
    }

    /// Creates a gravity field; the vector must carry the `m·s⁻²` signature.
    ///
    /// # Errors
    ///
    /// [`EnvironmentError::InconsistentUnits`] for a wrong-signature vector.
    pub fn new(acceleration: Vector3) -> Result<Self, EnvironmentError> {
        acceleration.require_unit(Self::unit())?;
        Ok(Self { acceleration })
    }

    /// Returns the acceleration vector.
    #[must_use]
    pub const fn acceleration(&self) -> Vector3 {
        self.acceleration
    }

    /// Returns the field strength |g| [m/s²].
    ///
    /// # Errors
    ///
    /// Propagates [`Vector3::magnitude`] overflow.
    pub fn strength(&self) -> Result<f64, EnvironmentError> {
        self.acceleration.magnitude()
    }
}

// ---------------------------------------------------------------------------
// MediumProperties
// ---------------------------------------------------------------------------

/// Properties of the ambient medium (air, water, …).
///
/// Density ρ, pressure p and temperature T are independent fields. HELIXSIM
/// deliberately assumes **no equation of state**: none exists in the current
/// repository contracts, and inventing one would embed domain physics into
/// the engine core. Consistency between the fields is the model author's
/// responsibility; structural validity is the engine's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MediumProperties {
    density: Scalar,
    pressure: Option<Scalar>,
    temperature: Option<Quantity>,
}

impl MediumProperties {
    /// Unit signature of mass density: `kg·m⁻³`.
    #[must_use]
    pub fn density_unit() -> SymbolicUnit {
        SymbolicUnit::from_mass(1)
            .checked_div(SymbolicUnit::from_length(3))
            .expect("internal: kg·m⁻³ is representable")
    }

    /// Unit signature of pressure: `kg·m⁻¹·s⁻²` (= Pa).
    #[must_use]
    pub fn pressure_unit() -> SymbolicUnit {
        SymbolicUnit::from_mass(1)
            .checked_div(SymbolicUnit::from_length(1))
            .and_then(|u| u.checked_mul(SymbolicUnit::from_time(-2)))
            .expect("internal: Pa signature is representable")
    }

    /// Creates medium properties.
    ///
    /// * `density` — required, signature `kg·m⁻³`, non-negative;
    /// * `pressure` — optional, signature `kg·m⁻¹·s⁻²`, non-negative;
    /// * `temperature` — optional, canonical [`Quantity`] measuring absolute
    ///   thermodynamic temperature (Foundation registers only `K`, affine
    ///   scales intentionally absent), non-negative.
    ///
    /// # Errors
    ///
    /// Wrong signatures, negative values, or non-finite numbers.
    pub fn new(
        density: Scalar,
        pressure: Option<Scalar>,
        temperature: Option<Quantity>,
    ) -> Result<Self, EnvironmentError> {
        if density.unit() != Self::density_unit() {
            return Err(EnvironmentError::InconsistentUnits {
                first: density.unit().to_string(),
                second: Self::density_unit().to_string(),
            });
        }
        if density.value() < 0.0 {
            return Err(EnvironmentError::NegativePhysicalValue {
                name: "density".to_owned(),
                value: density.value(),
            });
        }

        if let Some(p) = pressure {
            if p.unit() != Self::pressure_unit() {
                return Err(EnvironmentError::InconsistentUnits {
                    first: p.unit().to_string(),
                    second: Self::pressure_unit().to_string(),
                });
            }
            if p.value() < 0.0 {
                return Err(EnvironmentError::NegativePhysicalValue {
                    name: "pressure".to_owned(),
                    value: p.value(),
                });
            }
        }

        if let Some(t) = temperature {
            if t.unit().dimension() != helqix_core::Dimension::TEMPERATURE {
                return Err(EnvironmentError::InvalidDimension {
                    unit: t.unit().symbol().to_owned(),
                    expected: "temperature (K)".to_owned(),
                    found: t.unit().symbol().to_owned(),
                });
            }
            if t.value() < 0.0 {
                return Err(EnvironmentError::NegativePhysicalValue {
                    name: "temperature".to_owned(),
                    value: t.value(),
                });
            }
        }

        Ok(Self {
            density,
            pressure,
            temperature,
        })
    }

    /// Zero-density vacuum with no pressure or temperature.
    #[must_use]
    pub fn vacuum() -> Self {
        Self {
            density: Scalar::new_unchecked(0.0, Self::density_unit()),
            pressure: None,
            temperature: None,
        }
    }

    /// Returns the mass density [kg/m³].
    #[must_use]
    pub const fn density(&self) -> Scalar {
        self.density
    }

    /// Returns the static pressure, if supplied.
    #[must_use]
    pub const fn pressure(&self) -> Option<Scalar> {
        self.pressure
    }

    /// Returns the absolute temperature, if supplied.
    #[must_use]
    pub const fn temperature(&self) -> Option<Quantity> {
        self.temperature
    }
}

// ---------------------------------------------------------------------------
// Environment
// ---------------------------------------------------------------------------

/// The complete environmental condition set at one instant of logical time.
///
/// Construction validates every field; afterwards evaluation is pure and
/// infallible except for declared numerical-overflow refusal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Environment {
    gravity: GravityField,
    medium: MediumProperties,
    wind: Vector3,
}

impl Environment {
    /// Unit signature of velocity: `m·s⁻¹`.
    #[must_use]
    pub fn velocity_unit() -> SymbolicUnit {
        SymbolicUnit::from_length(1)
            .checked_div(SymbolicUnit::from_time(1))
            .expect("internal: m·s⁻¹ is representable")
    }

    /// Assembles a validated environment.
    ///
    /// # Errors
    ///
    /// Rejects a wind vector whose signature is not `m·s⁻¹`. Gravity and
    /// medium were validated at their own construction.
    pub fn new(
        gravity: GravityField,
        medium: MediumProperties,
        wind: Vector3,
    ) -> Result<Self, EnvironmentError> {
        wind.require_unit(Self::velocity_unit())?;
        Ok(Self {
            gravity,
            medium,
            wind,
        })
    }

    /// Returns the gravitational field.
    #[must_use]
    pub const fn gravity(&self) -> GravityField {
        self.gravity
    }

    /// Returns the medium properties.
    #[must_use]
    pub const fn medium(&self) -> &MediumProperties {
        &self.medium
    }

    /// Returns the bulk medium velocity (wind).
    #[must_use]
    pub const fn wind(&self) -> Vector3 {
        self.wind
    }

    /// Medium velocity relative to a body velocity **v**:
    ///
    /// ```text
    /// v_rel = u_wind − v_body
    /// ```
    ///
    /// # Errors
    ///
    /// Propagates [`Vector3::sub`] unit-mismatch/refusal errors.
    pub fn relative_velocity(&self, body_velocity: &Vector3) -> Result<Vector3, EnvironmentError> {
        self.wind.sub(body_velocity)
    }

    /// Dynamic pressure of the medium seen by a body moving at
    /// `body_velocity`:
    ///
    /// ```text
    /// q = ½ · ρ · |u_wind − v_body|²      [kg·m⁻¹·s⁻² = Pa]
    /// ```
    ///
    /// # Errors
    ///
    /// Unit mismatch, or refusal of a non-finite intermediate/result.
    pub fn dynamic_pressure(&self, body_velocity: &Vector3) -> Result<Scalar, EnvironmentError> {
        let v_rel = self.relative_velocity(body_velocity)?;
        let speed_sq = v_rel.magnitude_squared()?;
        let half_rho = Scalar::new_unchecked(
            require_finite(0.5 * self.medium.density().value())?,
            MediumProperties::density_unit(),
        );
        half_rho.mul(&speed_sq)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Classifies a scalar, converting non-finite values into errors.
pub(crate) fn require_finite(value: f64) -> Result<f64, EnvironmentError> {
    match NonFiniteKind::classify(value) {
        Some(kind) => Err(EnvironmentError::NonFiniteValue(kind)),
        None => Ok(value),
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors produced while constructing or evaluating environments.
///
/// Plain data with total equality — consistent with the Foundation error
/// philosophy (no boxed payloads, no hidden context).
#[derive(Debug, Clone, PartialEq)]
pub enum EnvironmentError {
    /// A numeric input or derived result is NaN or infinite; refused, never
    /// propagated silently.
    NonFiniteValue(NonFiniteKind),
    /// Two operands did not share the same unit signature.
    InconsistentUnits { first: String, second: String },
    /// A quantity carried a unit of the wrong physical dimension.
    InvalidDimension {
        expected: String,
        found: String,
        unit: String,
    },
    /// A physically non-negative field received a negative value.
    NegativePhysicalValue { name: String, value: f64 },
    /// A compound unit signature exceeds the representable exponent range.
    UnitOverflow,
    /// Division by an exactly-zero denominator.
    ZeroDenominator,
    /// The unit signature has no single-symbol coherent representation in
    /// the Foundation registry.
    UnrepresentableUnit(String),
    /// A Foundation quantity validation failure.
    Quantity(helqix_core::QuantityError),
}

impl fmt::Display for EnvironmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue(kind) => write!(f, "numerical value is not finite: {kind}"),
            Self::InconsistentUnits { first, second } => {
                write!(f, "inconsistent units: `{first}` vs `{second}`")
            }
            Self::InvalidDimension {
                unit,
                expected,
                found,
            } => {
                write!(f, "unit `{unit}` measures {found}, expected {expected}")
            }
            Self::NegativePhysicalValue { name, value } => {
                write!(f, "{name} must be non-negative, got {value}")
            }
            Self::UnitOverflow => write!(f, "compound unit exponent overflow"),
            Self::ZeroDenominator => write!(f, "division by zero denominator"),
            Self::UnrepresentableUnit(unit) => {
                write!(
                    f,
                    "unit `{unit}` has no single-symbol coherent registry entry"
                )
            }
            Self::Quantity(error) => write!(f, "invalid quantity: {error}"),
        }
    }
}

impl std::error::Error for EnvironmentError {}

impl From<helqix_core::QuantityError> for EnvironmentError {
    fn from(error: helqix_core::QuantityError) -> Self {
        Self::Quantity(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helqix_core::Unit;

    fn ms() -> SymbolicUnit {
        Environment::velocity_unit()
    }

    fn ms2() -> SymbolicUnit {
        GravityField::unit()
    }

    #[test]
    fn vector_rejects_nan_component() {
        let error = Vector3::new([f64::NAN, 0.0, 0.0], ms()).unwrap_err();
        assert_eq!(error, EnvironmentError::NonFiniteValue(NonFiniteKind::Nan));
    }

    #[test]
    fn vector_rejects_infinite_component() {
        let error = Vector3::new([0.0, f64::INFINITY, 0.0], ms()).unwrap_err();
        assert_eq!(
            error,
            EnvironmentError::NonFiniteValue(NonFiniteKind::PositiveInfinity)
        );
    }

    #[test]
    fn gravity_requires_acceleration_signature() {
        let velocity = Vector3::new([0.0, -9.8, 0.0], ms()).unwrap();
        let error = GravityField::new(velocity).unwrap_err();
        assert!(matches!(error, EnvironmentError::InconsistentUnits { .. }));

        let accel = Vector3::new([0.0, -9.80665, 0.0], ms2()).unwrap();
        let field = GravityField::new(accel).unwrap();
        assert_eq!(field.strength().unwrap(), 9.80665);
    }

    #[test]
    fn magnitude_matches_pythagoras_exactly() {
        let v = Vector3::new([3.0, 4.0, 12.0], ms()).unwrap();
        assert_eq!(v.magnitude().unwrap(), 13.0);
    }

    #[test]
    fn magnitude_survives_huge_components_without_overflow() {
        let big = 1e308_f64;
        let v = Vector3::new([big, big, 0.0], ms()).unwrap();
        let m = v.magnitude().unwrap();
        assert!(m.is_finite());
        // sqrt(2)*1e308 within scaling tolerance
        assert!((m - big * std::f64::consts::SQRT_2).abs() <= m * 1e-12);
    }

    #[test]
    fn dot_and_cross_require_matching_units() {
        let a = Vector3::new([1.0, 2.0, 3.0], ms()).unwrap();
        let b = Vector3::new([1.0, 0.0, 0.0], ms2()).unwrap();
        assert!(matches!(
            a.dot(&b).unwrap_err(),
            EnvironmentError::InconsistentUnits { .. }
        ));
        assert!(matches!(
            a.cross(&b).unwrap_err(),
            EnvironmentError::InconsistentUnits { .. }
        ));
    }

    #[test]
    fn dot_product_value_and_unit() {
        let a = Vector3::new([1.0, 2.0, 3.0], ms()).unwrap();
        let b = Vector3::new([4.0, 5.0, 6.0], ms()).unwrap();
        let d = a.dot(&b).unwrap();
        assert_eq!(d.value(), 32.0);
        assert_eq!(d.unit().to_string(), "m^2·s^-2");
    }

    #[test]
    fn cross_product_is_anticommutative_newton_symmetry_basis() {
        let a = Vector3::new([1.0, 0.0, 0.0], ms()).unwrap();
        let b = Vector3::new([0.0, 1.0, 0.0], ms()).unwrap();
        let ab = a.cross(&b).unwrap();
        let ba = b.cross(&a).unwrap();
        assert_eq!(ab.components(), [0.0, 0.0, 1.0]);
        assert_eq!(ba.components(), [0.0, 0.0, -1.0]);
    }

    #[test]
    fn medium_rejects_wrong_density_signature() {
        let wrong = Scalar::new(1.225, SymbolicUnit::from_mass(1)).unwrap();
        let error = MediumProperties::new(wrong, None, None).unwrap_err();
        assert!(matches!(error, EnvironmentError::InconsistentUnits { .. }));
    }

    #[test]
    fn medium_rejects_negative_density() {
        let rho = Scalar::new(-1.0, MediumProperties::density_unit()).unwrap();
        let error = MediumProperties::new(rho, None, None).unwrap_err();
        assert_eq!(
            error,
            EnvironmentError::NegativePhysicalValue {
                name: "density".to_owned(),
                value: -1.0
            }
        );
    }

    #[test]
    fn medium_accepts_valid_triplet() {
        let rho = Scalar::new(1.225, MediumProperties::density_unit()).unwrap();
        let p = Scalar::new(101_325.0, MediumProperties::pressure_unit()).unwrap();
        let t = Quantity::new(288.15, Unit::parse("K").unwrap()).unwrap();
        let medium = MediumProperties::new(rho, Some(p), Some(t)).unwrap();
        assert_eq!(medium.density().value(), 1.225);
        assert_eq!(medium.pressure().unwrap().value(), 101_325.0);
        assert_eq!(medium.temperature().unwrap().value(), 288.15);
    }

    #[test]
    fn medium_rejects_negative_absolute_temperature() {
        let rho = Scalar::new(1.0, MediumProperties::density_unit()).unwrap();
        let t = Quantity::new(-1.0, Unit::parse("K").unwrap()).unwrap();
        let error = MediumProperties::new(rho, None, Some(t)).unwrap_err();
        assert!(matches!(
            error,
            EnvironmentError::NegativePhysicalValue { .. }
        ));
    }

    #[test]
    fn environment_rejects_non_velocity_wind() {
        let gravity = GravityField::new(Vector3::new([0.0, -9.80665, 0.0], ms2()).unwrap());
        let medium = MediumProperties::vacuum();
        let bad_wind = Vector3::new([1.0, 0.0, 0.0], ms2()).unwrap();
        let error = Environment::new(gravity.unwrap(), medium, bad_wind).unwrap_err();
        assert!(matches!(error, EnvironmentError::InconsistentUnits { .. }));
    }

    #[test]
    fn dynamic_pressure_matches_analytical_formula() {
        // ρ = 1.225 kg/m³, wind = 0, body v = (10, 0, 0) m/s
        // q = ½ · 1.225 · 100 = 61.25 Pa
        let gravity = GravityField::new(Vector3::new([0.0, 0.0, 0.0], ms2()).unwrap()).unwrap();
        let rho = Scalar::new(1.225, MediumProperties::density_unit()).unwrap();
        let medium = MediumProperties::new(rho, None, None).unwrap();
        let env = Environment::new(gravity, medium, Vector3::zero(ms())).unwrap();
        let body = Vector3::new([10.0, 0.0, 0.0], ms()).unwrap();
        let q = env.dynamic_pressure(&body).unwrap();
        // 61.25 is not exactly representable as f64; the computed product
        // rounds to 61.25000000000001. Tolerance: one ulp of 61.25 (~7.1e-15).
        assert!((q.value() - 61.25_f64).abs() <= f64::EPSILON * 61.25 * 2.0);
        assert_eq!(q.unit(), MediumProperties::pressure_unit());
        // And the pressure unit renders as the Pa signature.
        assert_eq!(q.unit().to_string(), "m^-1·kg·s^-2");
    }

    #[test]
    fn dynamic_pressure_is_zero_when_body_moves_with_the_wind() {
        let gravity = GravityField::new(Vector3::zero(ms2())).unwrap();
        let rho = Scalar::new(1.225, MediumProperties::density_unit()).unwrap();
        let medium = MediumProperties::new(rho, None, None).unwrap();
        let wind = Vector3::new([5.0, 0.0, 0.0], ms()).unwrap();
        let env = Environment::new(gravity, medium, wind).unwrap();
        let q = env.dynamic_pressure(&wind).unwrap();
        assert_eq!(q.value(), 0.0);
    }

    #[test]
    fn relative_velocity_is_wind_minus_body() {
        let gravity = GravityField::new(Vector3::zero(ms2())).unwrap();
        let wind = Vector3::new([5.0, 0.0, 0.0], ms()).unwrap();
        let env = Environment::new(gravity, MediumProperties::vacuum(), wind).unwrap();
        let body = Vector3::new([2.0, 0.0, 0.0], ms()).unwrap();
        let rel = env.relative_velocity(&body).unwrap();
        assert_eq!(rel.components(), [3.0, 0.0, 0.0]);
    }

    #[test]
    fn repeated_evaluation_is_bit_identical() {
        let gravity =
            GravityField::new(Vector3::new([0.1, -9.80665, 0.2], ms2()).unwrap()).unwrap();
        let rho = Scalar::new(1.225, MediumProperties::density_unit()).unwrap();
        let medium = MediumProperties::new(rho, None, None).unwrap();
        let wind = Vector3::new([3.0, -1.0, 0.5], ms()).unwrap();
        let env = Environment::new(gravity, medium, wind).unwrap();
        let body = Vector3::new([12.3, 4.5, -0.7], ms()).unwrap();

        for _ in 0..8 {
            let q1 = env.dynamic_pressure(&body).unwrap();
            let q2 = env.dynamic_pressure(&body).unwrap();
            assert_eq!(q1, q2);
        }
    }

    #[test]
    fn scalar_div_refuses_zero_denominator() {
        let a = Scalar::new(1.0, SymbolicUnit::METRE).unwrap();
        let b = Scalar::new(0.0, SymbolicUnit::SECOND).unwrap();
        assert_eq!(a.div(&b).unwrap_err(), EnvironmentError::ZeroDenominator);
    }

    #[test]
    fn scalar_to_quantity_maps_kelvin_back_to_registry() {
        let t = Scalar::new(288.15, SymbolicUnit::from_temperature(1)).unwrap();
        let q = t.to_quantity().unwrap();
        assert_eq!(q.value(), 288.15);
        assert_eq!(q.unit().symbol(), "K");
    }

    #[test]
    fn symbolic_unit_display_and_algebra() {
        let density = MediumProperties::density_unit();
        assert_eq!(density.to_string(), "m^-3·kg");
        let v2 = ms().checked_pow(2).unwrap();
        let pressure = density.checked_mul(v2).unwrap();
        assert_eq!(pressure, MediumProperties::pressure_unit());
    }
}
