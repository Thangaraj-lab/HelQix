//! Dimension-aware force construction for the Layer 06 point-mass models.

use core::fmt;

use crate::environment::{Environment, EnvironmentError, SymbolicUnit, Vector3};

/// Unit signature of force in coherent SI: `kg·m·s⁻²` (newtons).
#[must_use]
pub fn newton_unit() -> SymbolicUnit {
    SymbolicUnit::from_mass(1)
        .checked_mul(SymbolicUnit::from_length(1))
        .and_then(|unit| unit.checked_mul(SymbolicUnit::from_time(-2)))
        .expect("internal: newton unit is representable")
}

/// Validated reference area and drag coefficient for a quadratic drag model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragConfiguration {
    /// Dimensionless drag coefficient, non-negative.
    pub coefficient: f64,
    /// Reference area in square metres, non-negative.
    pub reference_area_m2: f64,
}

impl DragConfiguration {
    /// Creates a validated drag configuration.
    pub fn new(coefficient: f64, reference_area_m2: f64) -> Result<Self, ForceError> {
        if !coefficient.is_finite() || !reference_area_m2.is_finite() {
            return Err(ForceError::NonFiniteInput);
        }
        if coefficient < 0.0 || reference_area_m2 < 0.0 {
            return Err(ForceError::NegativeParameter);
        }
        Ok(Self { coefficient, reference_area_m2 })
    }
}

/// Physically separated force terms used by [`crate::dynamics::PointMassModel`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForceBreakdown {
    /// Gravitational force in newtons.
    pub gravity: Vector3,
    /// Externally applied force in newtons.
    pub applied: Vector3,
    /// Aerodynamic drag in newtons.
    pub drag: Vector3,
    /// Vector sum of all force terms in newtons.
    pub net: Vector3,
}

/// Failure to construct or combine a force.
#[derive(Debug, Clone, PartialEq)]
pub enum ForceError {
    /// A required numeric value was NaN or infinite.
    NonFiniteInput,
    /// Mass, area, or drag coefficient was negative.
    NegativeParameter,
    /// A mass used to calculate gravitational force was zero.
    NonPositiveMass,
    /// An applied force or velocity carried an unexpected unit.
    InvalidUnit,
    /// The environment or a checked vector operation returned an error.
    Environment(EnvironmentError),
}

impl fmt::Display for ForceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "force inputs must be finite"),
            Self::NegativeParameter => write!(f, "mass, drag coefficient, and reference area cannot be negative"),
            Self::NonPositiveMass => write!(f, "mass must be greater than zero"),
            Self::InvalidUnit => write!(f, "force or velocity uses an incompatible SI unit signature"),
            Self::Environment(error) => write!(f, "environment force calculation failed: {error}"),
        }
    }
}

impl std::error::Error for ForceError {}

impl From<EnvironmentError> for ForceError {
    fn from(error: EnvironmentError) -> Self { Self::Environment(error) }
}

/// Calculates uniform gravitational force `F = m·g` in newtons.
pub fn gravity_force(environment: &Environment, mass_kg: f64) -> Result<Vector3, ForceError> {
    if !mass_kg.is_finite() { return Err(ForceError::NonFiniteInput); }
    if mass_kg <= 0.0 { return Err(ForceError::NonPositiveMass); }
    let gravity = environment.gravity().acceleration().components();
    Vector3::new(gravity.map(|component| component * mass_kg), newton_unit()).map_err(Into::into)
}

/// Calculates quadratic drag `F = ½ρ C_d A |u−v| (u−v)` in newtons.
///
/// Drag points along the local medium velocity relative to the body, which
/// opposes motion through still air and follows a moving wind. Zero density,
/// coefficient, reference area, or relative speed produces a zero force.
pub fn aerodynamic_drag_force(
    environment: &Environment,
    body_velocity: &Vector3,
    configuration: DragConfiguration,
) -> Result<Vector3, ForceError> {
    body_velocity.require_unit(Environment::velocity_unit()).map_err(|_| ForceError::InvalidUnit)?;
    if !configuration.coefficient.is_finite() || !configuration.reference_area_m2.is_finite() {
        return Err(ForceError::NonFiniteInput);
    }
    if configuration.coefficient < 0.0 || configuration.reference_area_m2 < 0.0 {
        return Err(ForceError::NegativeParameter);
    }
    let relative = environment.relative_velocity(body_velocity)?;
    let speed = relative.magnitude()?;
    let magnitude = environment.dynamic_pressure(body_velocity)?.value()
        * configuration.coefficient * configuration.reference_area_m2;
    if speed == 0.0 || magnitude == 0.0 {
        return Ok(Vector3::zero(newton_unit()));
    }
    let scale = magnitude / speed;
    Vector3::new(relative.components().map(|component| component * scale), newton_unit()).map_err(Into::into)
}

/// Adds force vectors after requiring each vector to use newtons.
pub fn sum_forces(forces: &[Vector3]) -> Result<Vector3, ForceError> {
    let mut net = Vector3::zero(newton_unit());
    for force in forces {
        if force.unit() != newton_unit() { return Err(ForceError::InvalidUnit); }
        net = net.add(force)?;
    }
    Ok(net)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::{GravityField, MediumProperties, Scalar};

    fn environment(density: f64, wind: Vector3) -> Environment {
        let gravity = GravityField::new(Vector3::new([0.0, -9.80665, 0.0], GravityField::unit()).unwrap()).unwrap();
        let medium = MediumProperties::new(Scalar::new(density, MediumProperties::density_unit()).unwrap(), None, None).unwrap();
        Environment::new(gravity, medium, wind).unwrap()
    }

    #[test]
    fn gravity_scales_linearly_with_mass_and_uses_newtons() {
        let env = environment(0.0, Vector3::zero(Environment::velocity_unit()));
        let weight = gravity_force(&env, 2.0).unwrap();
        assert_eq!(weight.components(), [0.0, -19.6133, 0.0]);
        assert_eq!(weight.unit(), newton_unit());
    }

    #[test]
    fn aerodynamic_drag_opposes_body_motion_in_still_air() {
        let env = environment(1.225, Vector3::zero(Environment::velocity_unit()));
        let velocity = Vector3::new([10.0, 0.0, 0.0], Environment::velocity_unit()).unwrap();
        let drag = aerodynamic_drag_force(&env, &velocity, DragConfiguration::new(1.0, 2.0).unwrap()).unwrap();
        assert!((drag.components()[0] + 122.5).abs() < 1e-12);
        assert_eq!(drag.components()[1], 0.0);
        assert_eq!(drag.components()[2], 0.0);
    }

    #[test]
    fn moving_with_wind_has_no_drag_and_wrong_units_are_rejected() {
        let wind = Vector3::new([5.0, 0.0, 0.0], Environment::velocity_unit()).unwrap();
        let env = environment(1.225, wind);
        assert!(aerodynamic_drag_force(&env, &wind, DragConfiguration::new(1.0, 1.0).unwrap()).unwrap().is_zero());
        let wrong = Vector3::new([1.0, 0.0, 0.0], GravityField::unit()).unwrap();
        assert_eq!(aerodynamic_drag_force(&env, &wrong, DragConfiguration::new(1.0, 1.0).unwrap()).unwrap_err(), ForceError::InvalidUnit);
    }
}
