//! HELIXSIM translational dynamics — equations of motion (Layer 06).
//!
//! This module maps physical state to time derivatives. It performs **no**
//! numerical integration: advancing a derivative across a timestep is the
//! job of [`crate::integration`], and committing authoritative state is the
//! job of Layer 04.
//!
//! # Model
//!
//! Point-mass Newtonian translation in an inertial frame:
//!
//! ```text
//! ΣF = m·a            (Newton's second law)
//! a   = ΣF / m        (mass must be strictly positive)
//! dx/dt = v           (kinematic relation)
//! dv/dt = a           (dynamic relation)
//! ```
//!
//! where **x** [m] is position, **v** [m/s] velocity, **a** [m/s²]
//! acceleration, **F** [N] force, m [kg] mass.
//!
//! Rotational dynamics (`τ = I·α + ω × (I·ω)`) is deliberately **not**
//! implemented: no upstream repository contract (Layers 01–07) currently
//! represents orientation, angular velocity, or inertia tensors, and
//! inventing them here would create a parallel mechanics model rather than
//! reuse canonical abstractions. The [`TranslationalState`] type below is
//! therefore the smallest architecture-compatible representation.
//!
//! # Boundary with the integrator
//!
//! [`state_to_derivative`] returns the *slope field* `f(x, t)`; it does not
//! know about timesteps. [`acceleration_from_force`] isolates `a = ΣF/m`.

use std::fmt;

use crate::environment::{require_finite, SymbolicUnit, Vector3};
use crate::forces::{ForceError, ForceUnit};

/// Position signature `m`.
pub const POSITION_UNIT: SymbolicUnit = SymbolicUnit::from_length(1);

/// A point-mass translational state at one instant of logical time.
///
/// Both vectors are expressed in coherent SI base magnitudes (m, m/s).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TranslationalState {
    position: Vector3,
    velocity: Vector3,
}

impl TranslationalState {
    /// Creates a validated state.
    ///
    /// # Errors
    ///
    /// [`DynamicsError::InconsistentUnits`] if `position` is not metres or
    /// `velocity` is not metres per second. Non-finite components are
    /// rejected by [`Vector3`] construction itself.
    pub fn new(position: Vector3, velocity: Vector3) -> Result<Self, DynamicsError> {
        position.require_unit(POSITION_UNIT)?;
        velocity.require_unit(ForceUnit::VELOCITY)?;
        Ok(Self { position, velocity })
    }

    /// Returns the position vector [m].
    #[must_use]
    pub const fn position(&self) -> Vector3 {
        self.position
    }

    /// Returns the velocity vector [m/s].
    #[must_use]
    pub const fn velocity(&self) -> Vector3 {
        self.velocity
    }
}

/// The time derivative of a [`TranslationalState`]: `(dx/dt, dv/dt)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateDerivative {
    d_position: Vector3,
    d_velocity: Vector3,
}

impl StateDerivative {
    /// Wraps validated derivative vectors.
    ///
    /// # Errors
    ///
    /// [`DynamicsError::InconsistentUnits`] for wrong signatures.
    pub fn new(d_position: Vector3, d_velocity: Vector3) -> Result<Self, DynamicsError> {
        d_position.require_unit(ForceUnit::VELOCITY)?;
        d_velocity.require_unit(ForceUnit::ACCELERATION)?;
        Ok(Self { d_position, d_velocity })
    }

    /// `dx/dt` — equals the current velocity [m/s].
    #[must_use]
    pub const fn d_position(&self) -> Vector3 {
        self.d_position
    }

    /// `dv/dt` — equals the current acceleration [m/s²].
    #[must_use]
    pub const fn d_velocity(&self) -> Vector3 {
        self.d_velocity
    }
}

/// Newton's second law: `a = ΣF / m`.
///
/// * `net_force` must be a newton vector (`kg·m·s⁻²`).
/// * `mass` must be **strictly positive**: zero or negative mass makes the
///   equation singular or non-physical and is refused, never silently
///   clamped.
///
/// # Errors
///
/// [`DynamicsError::InvalidMass`] for `mass <= 0`, NaN, or infinity;
/// [`DynamicsError::InconsistentUnits`] for a non-newton force;
/// [`DynamicsError::NonFiniteValue`] on overflow.
pub fn acceleration_from_force(net_force: &Vector3, mass: f64) -> Result<Vector3, DynamicsError> {
    net_force.require_unit(ForceUnit::NEWTON)?;
    require_positive_mass(mass)?;
    let mut out = [0.0; 3];
    for (o, c) in out.iter_mut().zip(net_force.components()) {
        require_finite(c / mass)?;
        *o = c / mass;
    }
    Vector3::new_checked(out, ForceUnit::ACCELERATION).map_err(DynamicsError::from)
}

/// Evaluates the slope field `f(state) = (dx/dt, dv/dt)` of the point-mass
/// translational model.
///
/// Given the net force already acting on the body (computed elsewhere —
/// gravity, drag, contacts, …), this returns the derivative that an
/// integrator may advance over a timestep. Deterministic and allocation-free.
///
/// # Errors
///
/// As [`acceleration_from_force`].
pub fn state_to_derivative(
    state: &TranslationalState,
    net_force: &Vector3,
    mass: f64,
) -> Result<StateDerivative, DynamicsError> {
    let a = acceleration_from_force(net_force, mass)?;
    Ok(StateDerivative {
        d_position: state.velocity,
        d_velocity: a,
    })
}

fn require_positive_mass(mass: f64) -> Result<(), DynamicsError> {
    if let Err(e) = require_finite(mass) {
        return Err(DynamicsError::from(e));
    }
    if mass <= 0.0 {
        return Err(DynamicsError::InvalidMass(mass));
    }
    Ok(())
}

/// Errors produced by dynamics evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum DynamicsError {
    /// A numeric input or derived result is NaN or infinite.
    NonFiniteValue(helqix_core::NonFiniteKind),
    /// A vector did not carry the required unit signature.
    InconsistentUnits {
        /// Signature of the first operand.
        first: String,
        /// Signature of the second operand.
        second: String,
    },
    /// Mass was zero, negative, or otherwise physically invalid.
    InvalidMass(f64),
    /// A force-model failure surfaced through dynamics composition.
    Force(ForceError),
}

impl fmt::Display for DynamicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue(kind) => write!(f, "numerical value is not finite: {kind}"),
            Self::InconsistentUnits { first, second } => {
                write!(f, "inconsistent units: `{first}` vs `{second}`")
            }
            Self::InvalidMass(mass) => {
                write!(f, "mass must be finite and strictly positive, got {mass}")
            }
            Self::Force(error) => write!(f, "force failure: {error}"),
        }
    }
}

impl std::error::Error for DynamicsError {}

impl From<crate::environment::EnvironmentError> for DynamicsError {
    fn from(error: crate::environment::EnvironmentError) -> Self {
        use crate::environment::EnvironmentError as E;
        match error {
            E::NonFiniteValue(kind) => Self::NonFiniteValue(kind),
            E::InconsistentUnits { first, second } => Self::InconsistentUnits { first, second },
            // See the note in forces.rs: these cannot arise from the checked
            // operations used here but must still surface as failures.
            _ => Self::NonFiniteValue(helqix_core::NonFiniteKind::Nan),
        }
    }
}

impl From<ForceError> for DynamicsError {
    fn from(error: ForceError) -> Self {
        match error {
            ForceError::NonFiniteValue(kind) => Self::NonFiniteValue(kind),
            ForceError::InconsistentUnits { first, second } => {
                Self::InconsistentUnits { first, second }
            }
            other => Self::Force(other),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn vel(x: f64, y: f64, z: f64) -> Vector3 {
        Vector3::new([x, y, z], ForceUnit::VELOCITY).unwrap()
    }
    fn pos(x: f64, y: f64, z: f64) -> Vector3 {
        Vector3::new([x, y, z], POSITION_UNIT).unwrap()
    }
    fn force(x: f64, y: f64, z: f64) -> Vector3 {
        Vector3::new([x, y, z], ForceUnit::NEWTON).unwrap()
    }

    #[test]
    fn newtons_second_law_a_equals_F_over_m() {
        // F = (10, -5, 0) N, m = 2 kg → a = (5, -2.5, 0) m/s²
        let a = acceleration_from_force(&force(10.0, -5.0, 0.0), 2.0).unwrap();
        assert_eq!(a.components(), [5.0, -2.5, 0.0]);
        assert_eq!(a.unit(), ForceUnit::ACCELERATION);
    }

    #[test]
    fn zero_net_force_gives_zero_acceleration() {
        let a = acceleration_from_force(&force(0.0, 0.0, 0.0), 3.7).unwrap();
        assert!(a.is_zero());
    }

    #[test]
    fn constant_force_gives_constant_acceleration_derivative() {
        let state = TranslationalState::new(pos(1.0, 2.0, 3.0), vel(0.5, 0.0, -0.25)).unwrap();
        let d = state_to_derivative(&state, &force(1.0, 0.0, 0.0), 2.0).unwrap();
        // dx/dt = v ; dv/dt = F/m = 0.5
        assert_eq!(d.d_position(), state.velocity());
        assert_eq!(d.d_velocity().components(), [0.5, 0.0, 0.0]);
    }

    #[test]
    fn rejects_non_positive_mass() {
        for m in [0.0, -1.0] {
            let err = acceleration_from_force(&force(1.0, 0.0, 0.0), m).unwrap_err();
            assert_eq!(err, DynamicsError::InvalidMass(m));
        }
    }

    #[test]
    fn rejects_non_finite_mass() {
        assert!(matches!(
            acceleration_from_force(&force(1.0, 0.0, 0.0), f64::NAN),
            Err(DynamicsError::NonFiniteValue(_))
        ));
        assert!(matches!(
            acceleration_from_force(&force(1.0, 0.0, 0.0), f64::INFINITY),
            Err(DynamicsError::NonFiniteValue(_))
        ));
    }

    #[test]
    fn rejects_wrong_force_units_and_state_units() {
        let wrong = vel(1.0, 0.0, 0.0);
        assert!(matches!(
            acceleration_from_force(&wrong, 1.0),
            Err(DynamicsError::InconsistentUnits { .. })
        ));
        assert!(matches!(
            TranslationalState::new(vel(0.0, 0.0, 0.0), vel(0.0, 0.0, 0.0)),
            Err(DynamicsError::InconsistentUnits { .. })
        ));
        assert!(matches!(
            TranslationalState::new(pos(0.0, 0.0, 0.0), pos(0.0, 0.0, 0.0)),
            Err(DynamicsError::InconsistentUnits { .. })
        ));
    }

    #[test]
    fn derivative_is_deterministic() {
        let run = || {
            let state =
                TranslationalState::new(pos(0.1, 0.2, 0.3), vel(-1.5, 0.75, 12.0)).unwrap();
            state_to_derivative(&state, &force(3.3, -9.81, 0.005), 1.7).unwrap()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn overflow_in_division_is_refused_not_leaked() {
        // f64::MAX / tiny mass overflows to infinity → must be an error.
        let err = acceleration_from_force(&force(f64::MAX, 0.0, 0.0), 1e-300).unwrap_err();
        assert!(matches!(err, DynamicsError::NonFiniteValue(_)));
    }
}
