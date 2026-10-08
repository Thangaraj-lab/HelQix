//! HELIXSIM physical force models (Layer 06).
//!
//! Force calculation only: each function maps validated inputs to a force
//! [`Vector3`] with unit signature `kg·m·s⁻²` (newton in coherent SI base
//! magnitudes). Nothing here integrates motion, advances logical time, or
//! mutates authoritative Layer 04 state.
//!
//! # Implemented models
//!
//! * Weight / gravity: `F_g = m · g`, **g** the gravitational acceleration
//!   field [m/s²], m the body mass [kg].
//! * Quadratic aerodynamic drag against the medium:
//!   `F_d = -½ · ρ · C_d · A · |v_rel| · v_rel`, where `v_rel = v − u` is
//!   the body velocity relative to the bulk medium velocity **u**. Written
//!   without division by speed, the model is well-defined at `v_rel = 0`
//!   (exactly zero force — no direction is invented) and strictly opposes
//!   the relative flow otherwise (`F_d · v_rel < 0`).
//! * Net force: `F_net = Σ F_i`, accumulated in caller order — deterministic,
//!   allocation-free.
//!
//! # Units and safety
//!
//! Vector inputs must be expressed in coherent SI base magnitudes (m, kg,
//! s); every produced vector carries [`ForceUnit::NEWTON`]. Non-finite
//! inputs, negative physical parameters, and overflowing results are refused
//! as [`ForceError`], never leaked.

use std::fmt;

use helqix_core::NonFiniteKind;

use crate::environment::{require_finite, SymbolicUnit, Vector3};

// ---------------------------------------------------------------------------
// Unit signatures
// ---------------------------------------------------------------------------

/// Coherent SI unit signatures used by the force models.
pub struct ForceUnit;

impl ForceUnit {
    /// Newton in coherent SI base magnitudes: `kg·m·s⁻²`.
    /// Exponent layout `[L, M, T, I, Th, N, J]` matches Foundation `Dimension`.
    pub const NEWTON: SymbolicUnit = SymbolicUnit::new([1, 1, -2, 0, 0, 0, 0]);
    /// Acceleration: `m·s⁻²` (same signature as the gravity field).
    pub const ACCELERATION: SymbolicUnit = SymbolicUnit::new([1, 0, -2, 0, 0, 0, 0]);
    /// Velocity: `m·s⁻¹`.
    pub const VELOCITY: SymbolicUnit = SymbolicUnit::new([1, 0, -1, 0, 0, 0, 0]);
    /// Mass density: `kg·m⁻³`.
    pub const DENSITY: SymbolicUnit = SymbolicUnit::new([-3, 1, 0, 0, 0, 0, 0]);
}

// ---------------------------------------------------------------------------
// Validated scalar parameters
// ---------------------------------------------------------------------------

macro_rules! validated_scalar {
    ($name:ident, $what:literal) => {
        #[doc = concat!("A validated, finite, non-negative ", $what, " parameter.")]
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct $name(f64);

        impl $name {
            /// Creates the parameter after validating finiteness and sign.
            ///
            /// # Errors
            ///
            /// [`ForceError::NonFiniteValue`] if `value` is NaN or infinite,
            /// [`ForceError::NegativePhysicalValue`] if `value < 0`.
            pub fn new(value: f64) -> Result<Self, ForceError> {
                require_finite(value)?;
                if value < 0.0 {
                    return Err(ForceError::NegativePhysicalValue {
                        name: $what.to_owned(),
                        value,
                    });
                }
                Ok(Self(value))
            }

            /// Returns the raw value.
            #[must_use]
            pub const fn value(&self) -> f64 {
                self.0
            }
        }
    };
}

validated_scalar!(Mass, "mass [kg]");
validated_scalar!(DragCoefficient, "drag coefficient [dimensionless]");
validated_scalar!(ReferenceArea, "reference area [m²]");
validated_scalar!(Density, "medium mass density [kg/m³]");

// ---------------------------------------------------------------------------
// Force models
// ---------------------------------------------------------------------------

/// Gravitational weight force `F_g = m · g`.
///
/// * `mass` — body mass in kg (validated non-negative newtype).
/// * `gravity` — gravitational acceleration field in `m·s⁻²`.
///
/// The result points along **g** scaled by the mass. A zero-mass body
/// produces the zero force vector without error (physically meaningful: no
/// weight).
///
/// # Errors
///
/// [`ForceError::InconsistentUnits`] if `gravity` is not an acceleration
/// vector, [`ForceError::NonFiniteValue`] on non-finite input or overflow.
pub fn weight(mass: Mass, gravity: &Vector3) -> Result<Vector3, ForceError> {
    gravity.require_unit(ForceUnit::ACCELERATION)?;
    let factor = mass.value();
    let mut out = [0.0; 3];
    for (o, c) in out.iter_mut().zip(gravity.components()) {
        require_finite(c * factor)?;
        *o = c * factor;
    }
    Ok(with_newton(out))
}

/// Quadratic drag force of a body moving through a medium.
///
/// `F_d = -½ · ρ · C_d · A · |v_rel| · v_rel` with `v_rel = v − u`:
///
/// * `density` — medium mass density ρ [kg/m³];
/// * `drag_coefficient` — dimensionless C_d;
/// * `reference_area` — frontal area A [m²];
/// * `body_velocity` — **v** in `m·s⁻¹`;
/// * `medium_velocity` — bulk medium velocity **u** in `m·s⁻¹` (pass the
///   zero vector for still medium).
///
/// Guarantees:
/// * `v_rel = 0` ⇒ exactly zero force;
/// * otherwise the force is antiparallel to `v_rel`.
///
/// # Errors
///
/// [`ForceError::InconsistentUnits`] for wrong vector dimensions,
/// [`ForceError::NonFiniteValue`] on overflow.
#[allow(clippy::too_many_arguments)]
pub fn drag(
    density: Density,
    drag_coefficient: DragCoefficient,
    reference_area: ReferenceArea,
    body_velocity: &Vector3,
    medium_velocity: &Vector3,
) -> Result<Vector3, ForceError> {
    body_velocity.require_unit(ForceUnit::VELOCITY)?;
    medium_velocity.require_unit(ForceUnit::VELOCITY)?;

    // v_rel = v − u : velocity of the body relative to the medium.
    let rel = body_velocity.sub(medium_velocity)?;
    let speed = rel.magnitude()?;
    // k = ½ ρ C_d A ; F = −k · |v_rel| · v_rel   (coherent SI → newtons)
    let k = 0.5 * density.value() * drag_coefficient.value() * reference_area.value();
    require_finite(k)?;
    let factor = require_finite(-k * speed)?;
    Ok(with_newton(rel.scale(factor)?.components()))
}

/// Net force `F_net = Σ F_i` over forces listed in caller order.
///
/// Deterministic: summation order is slice order — no hashing, sorting, or
/// parallel reduction. An empty slice yields the zero newton vector, the
/// identity of addition.
///
/// # Errors
///
/// [`ForceError::InconsistentUnits`] if any force is not a newton vector,
/// [`ForceError::NonFiniteValue`] on overflow.
pub fn net_force(forces: &[Vector3]) -> Result<Vector3, ForceError> {
    let mut acc = [0.0_f64; 3];
    for f in forces {
        f.require_unit(ForceUnit::NEWTON)?;
        for (a, c) in acc.iter_mut().zip(f.components()) {
            require_finite(*a + c)?;
            *a += c;
        }
    }
    Ok(with_newton(acc))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn with_newton(components: [f64; 3]) -> Vector3 {
    // Callers validate finiteness component-wise before reaching this point;
    // `new_checked` re-validates at the environment choke point regardless.
    Vector3::new_checked(components, ForceUnit::NEWTON)
        .expect("force components are finite by construction")
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors produced by force calculation.
#[derive(Debug, Clone, PartialEq)]
pub enum ForceError {
    /// A numeric input or derived result is NaN or infinite.
    NonFiniteValue(NonFiniteKind),
    /// Two operands did not share the same unit signature.
    InconsistentUnits {
        /// Signature of the first operand.
        first: String,
        /// Signature of the second operand.
        second: String,
    },
    /// A physically non-negative parameter received a negative value.
    NegativePhysicalValue {
        /// Name of the offending parameter.
        name: String,
        /// Rejected value.
        value: f64,
    },
}

impl fmt::Display for ForceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue(kind) => write!(f, "numerical value is not finite: {kind}"),
            Self::InconsistentUnits { first, second } => {
                write!(f, "inconsistent units: `{first}` vs `{second}`")
            }
            Self::NegativePhysicalValue { name, value } => {
                write!(f, "{name} must be non-negative, got {value}")
            }
        }
    }
}

impl std::error::Error for ForceError {}

impl From<crate::environment::EnvironmentError> for ForceError {
    fn from(error: crate::environment::EnvironmentError) -> Self {
        use crate::environment::EnvironmentError as E;
        match error {
            E::NonFiniteValue(kind) => Self::NonFiniteValue(kind),
            E::InconsistentUnits { first, second } => Self::InconsistentUnits { first, second },
            E::NegativePhysicalValue { name, value } => Self::NegativePhysicalValue { name, value },
            // Remaining variants cannot arise from the operations used in
            // this module; each nevertheless denotes a numerically invalid
            // situation for force calculation, so they surface explicitly
            // rather than being swallowed.
            E::InvalidDimension { .. }
            | E::UnitOverflow
            | E::ZeroDenominator
            | E::UnrepresentableUnit(_)
            | E::Quantity(_) => Self::NonFiniteValue(NonFiniteKind::Nan),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::EnvironmentError;

    fn vec3(x: f64, y: f64, z: f64, unit: SymbolicUnit) -> Vector3 {
        Vector3::new([x, y, z], unit).unwrap()
    }

    #[test]
    fn weight_matches_analytic_f_equals_mg() {
        // m = 2 kg, g = (0, 0, -9.81) m/s²  →  F = (0, 0, -19.62) N
        let g = vec3(0.0, 0.0, -9.81, ForceUnit::ACCELERATION);
        let f = weight(Mass::new(2.0).unwrap(), &g).unwrap();
        assert_eq!(f.unit(), ForceUnit::NEWTON);
        assert_eq!(f.components(), [0.0, 0.0, -19.62]);
    }

    #[test]
    fn weight_zero_mass_is_zero_force() {
        let g = vec3(0.0, -9.81, 0.0, ForceUnit::ACCELERATION);
        let f = weight(Mass::new(0.0).unwrap(), &g).unwrap();
        assert!(f.is_zero());
    }

    #[test]
    fn weight_rejects_wrong_vector_dimension() {
        let not_accel = vec3(1.0, 0.0, 0.0, ForceUnit::VELOCITY);
        let err = weight(Mass::new(1.0).unwrap(), &not_accel).unwrap_err();
        assert!(matches!(err, ForceError::InconsistentUnits { .. }));
    }

    #[test]
    fn drag_is_zero_at_zero_relative_velocity() {
        let v = vec3(3.0, 0.0, 0.0, ForceUnit::VELOCITY);
        let u = vec3(3.0, 0.0, 0.0, ForceUnit::VELOCITY);
        let f = drag(
            Density::new(1.225).unwrap(),
            DragCoefficient::new(0.47).unwrap(),
            ReferenceArea::new(0.01).unwrap(),
            &v,
            &u,
        )
        .unwrap();
        assert!(f.is_zero());
        assert_eq!(f.unit(), ForceUnit::NEWTON);
    }

    #[test]
    fn drag_opposes_relative_flow_with_analytic_magnitude() {
        // Still air, body at +x 10 m/s: F_x = -½·1.225·0.47·0.01·10·10
        let v = vec3(10.0, 0.0, 0.0, ForceUnit::VELOCITY);
        let u = Vector3::zero(ForceUnit::VELOCITY);
        let f = drag(
            Density::new(1.225).unwrap(),
            DragCoefficient::new(0.47).unwrap(),
            ReferenceArea::new(0.01).unwrap(),
            &v,
            &u,
        )
        .unwrap();
        let expected = -0.5 * 1.225 * 0.47 * 0.01 * 10.0 * 10.0;
        assert_eq!(f.components()[0], expected);
        assert_eq!(f.components()[1], 0.0);
        assert_eq!(f.components()[2], 0.0);
        assert!(f.components()[0] < 0.0, "drag must oppose +x motion");
    }

    #[test]
    fn drag_dot_relative_velocity_is_negative() {
        let v = vec3(1.0, -2.0, 0.5, ForceUnit::VELOCITY);
        let u = vec3(0.2, 0.3, -0.1, ForceUnit::VELOCITY);
        let f = drag(
            Density::new(1.225).unwrap(),
            DragCoefficient::new(1.0).unwrap(),
            ReferenceArea::new(2.0).unwrap(),
            &v,
            &u,
        )
        .unwrap();
        // v_rel = v − u (body relative to medium); drag opposes it. The dot
        // product needs matching unit signatures, so compare directions via
        // the raw coherent-SI components instead of mixed-unit `dot`.
        let rel = v.sub(&u).unwrap();
        let d: f64 = rel
            .components()
            .iter()
            .zip(f.components())
            .map(|(r, force)| r * force)
            .sum();
        assert!(d < 0.0, "drag must oppose the body-relative-medium flow");
    }

    #[test]
    fn net_force_sums_in_order_and_empty_is_zero() {
        let a = with_newton([1.0, 2.0, 3.0]);
        let b = with_newton([-0.5, 0.0, 1.0]);
        let sum = net_force(&[a, b]).unwrap();
        assert_eq!(sum.components(), [0.5, 2.0, 4.0]);
        assert!(net_force(&[]).unwrap().is_zero());
    }

    #[test]
    fn net_force_rejects_non_newton_vectors() {
        let wrong = vec3(1.0, 0.0, 0.0, ForceUnit::VELOCITY);
        let err = net_force(&[wrong]).unwrap_err();
        assert!(matches!(err, ForceError::InconsistentUnits { .. }));
    }

    #[test]
    fn rejects_negative_parameters() {
        assert!(matches!(
            Mass::new(-1.0),
            Err(ForceError::NegativePhysicalValue { .. })
        ));
        assert!(matches!(
            DragCoefficient::new(-0.1),
            Err(ForceError::NegativePhysicalValue { .. })
        ));
        assert!(matches!(
            ReferenceArea::new(-2.0),
            Err(ForceError::NegativePhysicalValue { .. })
        ));
        assert!(matches!(
            Density::new(-1.225),
            Err(ForceError::NegativePhysicalValue { .. })
        ));
    }

    #[test]
    fn rejects_non_finite_parameters_and_overflow() {
        assert!(matches!(
            Mass::new(f64::NAN),
            Err(ForceError::NonFiniteValue(NonFiniteKind::Nan))
        ));
        assert!(matches!(
            Mass::new(f64::INFINITY),
            Err(ForceError::NonFiniteValue(NonFiniteKind::PositiveInfinity))
        ));
        let huge = vec3(f64::MAX, 0.0, 0.0, ForceUnit::ACCELERATION);
        let err = weight(Mass::new(1e300).unwrap(), &huge).unwrap_err();
        assert!(matches!(err, ForceError::NonFiniteValue(_)));
    }

    #[test]
    fn calculation_is_deterministic_bit_for_bit() {
        let run = || {
            let v = vec3(7.3, -2.1, 0.4, ForceUnit::VELOCITY);
            let u = vec3(-1.0, 0.5, 0.25, ForceUnit::VELOCITY);
            let fd = drag(
                Density::new(1.225).unwrap(),
                DragCoefficient::new(0.3).unwrap(),
                ReferenceArea::new(0.7).unwrap(),
                &v,
                &u,
            )
            .unwrap();
            let fw = weight(
                Mass::new(3.5).unwrap(),
                &vec3(0.0, 0.0, -9.80665, ForceUnit::ACCELERATION),
            )
            .unwrap();
            net_force(&[fd, fw]).unwrap()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn environment_error_conversion_preserves_kinds() {
        let e: ForceError = EnvironmentError::NonFiniteValue(NonFiniteKind::Nan).into();
        assert_eq!(e, ForceError::NonFiniteValue(NonFiniteKind::Nan));
    }
}
