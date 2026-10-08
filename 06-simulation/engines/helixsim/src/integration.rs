//! HELIXSIM deterministic numerical integration (Layer 06).
//!
//! Pure stepping functions that advance a [`TranslationalState`] by one step
//! using classical Runge–Kutta schemes. Integration here is *mechanics*:
//! given a slope function `f(state) -> StateDerivative` and a step size h
//! [s], produce the next state. Nothing else happens:
//!
//! * no wall-clock reading, no randomness, no global mutable state —
//!   identical inputs always yield bit-identical outputs;
//! * no timestep policy or logical-time advancement (that belongs to the
//!   kernel / Layer 07); the caller supplies `h`;
//! * no mutation of authoritative Layer 04 state — the input state is
//!   consumed by value and a new state is returned.
//!
//! # Schemes
//!
//! * [`euler_step`] — explicit Euler, O(h²) local error. Provided because
//!   it is the reference scheme every higher-order method is compared
//!   against; production kernels should prefer RK4.
//! * [`rk4_step`] — classic fixed-step RK4 (k1..k4 with weights 1/6, 1/3,
//!   1/3, 1/6), O(h⁵) local error. The weight sequence and evaluation order
//!   are exactly the textbook scheme: k1 = f(s); k2 = f(s + h/2·k1);
//!   k3 = f(s + h/2·k2); k4 = f(s + h·k3); s' = s + h/6·(k1 + 2k2 + 2k3 + k4).
//!
//! # Error handling
//!
//! Every intermediate component is finiteness-checked; NaN/infinity from a
//! diverging step or a failing slope function is refused as
//! [`IntegrationError`] rather than propagated into future steps.

use std::fmt;

use helqix_core::NonFiniteKind;

use crate::dynamics::{StateDerivative, TranslationalState};
use crate::environment::{require_finite, Vector3};

/// A slope function evaluated at a state.
///
/// Implemented by anything that can turn a [`TranslationalState`] into its
/// time derivative — e.g. a closure capturing mass and environment that
/// calls [`crate::forces`] + [`crate::dynamics::state_to_derivative`].
pub trait SlopeFunction {
    /// Evaluates `f(state)`.
    ///
    /// # Errors
    ///
    /// Propagated to the integrator and surfaced as
    /// [`IntegrationError::Slope`].
    fn evaluate(&self, state: &TranslationalState) -> Result<StateDerivative, crate::dynamics::DynamicsError>;
}

impl<F> SlopeFunction for F
where
    F: for<'a> Fn(&'a TranslationalState) -> Result<StateDerivative, crate::dynamics::DynamicsError>,
{
    fn evaluate(&self, state: &TranslationalState) -> Result<StateDerivative, crate::dynamics::DynamicsError> {
        self(state)
    }
}

/// Explicit Euler step: `s' = s + h · f(s)`.
///
/// * `h` — step size in seconds; must be finite and strictly positive.
///
/// # Errors
///
/// [`IntegrationError::InvalidStepSize`] for `h <= 0`, NaN, or infinity;
/// [`IntegrationError::Slope`] if the slope function fails;
/// [`IntegrationError::NonFiniteValue`] if any advanced component overflows.
pub fn euler_step<S: SlopeFunction + ?Sized>(
    state: &TranslationalState,
    slope: &S,
    h: f64,
) -> Result<TranslationalState, IntegrationError> {
    require_positive_step(h)?;
    let d = slope.evaluate(state)?;
    advance(state, &weighted_delta(&[(1.0, &d)], h))
}

/// Classic fixed-step RK4: `s' = s + h/6 · (k1 + 2·k2 + 2·k3 + k4)`.
///
/// * `h` — step size in seconds; must be finite and strictly positive.
///
/// Evaluation order is fixed (k1→k4) so results are reproducible across
/// runs and platforms that follow IEEE-754 basic operations.
///
/// # Errors
///
/// As [`euler_step`].
pub fn rk4_step<S: SlopeFunction + ?Sized>(
    state: &TranslationalState,
    slope: &S,
    h: f64,
) -> Result<TranslationalState, IntegrationError> {
    require_positive_step(h)?;
    let half = require_finite(h / 2.0).map_err(|e| IntegrationError::from_env(e))?;

    let k1 = slope.evaluate(state)?;
    let s2 = advance(state, &scaled(&k1, half))?;
    let k2 = slope.evaluate(&s2)?;
    let s3 = advance(state, &scaled(&k2, half))?;
    let k3 = slope.evaluate(&s3)?;
    let s4 = advance(state, &scaled(&k3, h))?;
    let k4 = slope.evaluate(&s4)?;

    // delta = h/6 · (k1 + 2k2 + 2k3 + k4)
    let sixth = require_finite(h / 6.0).map_err(|e| IntegrationError::from_env(e))?;
    let delta = scaled(&k1, sixth)
        .add(&scaled(&k2, 2.0 * sixth))
        .and_then(|a| a.add(&scaled(&k3, 2.0 * sixth)))
        .and_then(|a| a.add(&scaled(&k4, sixth)))
        .map_err(|e| IntegrationError::from_env(e))?;
    advance(state, &delta)
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Raw coherent-SI delta vector (three components per derivative pair).
#[derive(Debug, Clone, Copy)]
struct Delta {
    dp: [f64; 3],
    dv: [f64; 3],
}

fn scaled(d: &StateDerivative, factor: f64) -> Delta {
    let mut dp = [0.0; 3];
    let mut dv = [0.0; 3];
    for i in 0..3 {
        dp[i] = d.d_position().components()[i] * factor;
        dv[i] = d.d_velocity().components()[i] * factor;
    }
    Delta { dp, dv }
}

fn weighted_delta(items: &[(f64, &StateDerivative)], h: f64) -> Delta {
    let mut dp = [0.0; 3];
    let mut dv = [0.0; 3];
    for (w, d) in items {
        for i in 0..3 {
            dp[i] += w * d.d_position().components()[i] * h;
            dv[i] += w * d.d_velocity().components()[i] * h;
        }
    }
    Delta { dp, dv }
}

impl Delta {
    fn add(&self, other: &Delta) -> Result<Delta, crate::environment::EnvironmentError> {
        let mut dp = [0.0; 3];
        let mut dv = [0.0; 3];
        for i in 0..3 {
            dp[i] = require_finite(self.dp[i] + other.dp[i])?;
            dv[i] = require_finite(self.dv[i] + other.dv[i])?;
        }
        Ok(Delta { dp, dv })
    }
}

fn advance(state: &TranslationalState, delta: &Delta) -> Result<TranslationalState, IntegrationError> {
    let mut p = [0.0; 3];
    let mut v = [0.0; 3];
    for i in 0..3 {
        p[i] = require_finite(state.position().components()[i] + delta.dp[i])
            .map_err(|e| IntegrationError::from_env(e))?;
        v[i] = require_finite(state.velocity().components()[i] + delta.dv[i])
            .map_err(|e| IntegrationError::from_env(e))?;
    }
    let position = Vector3::new_checked(p, crate::dynamics::POSITION_UNIT)
        .map_err(|e| IntegrationError::from_env(e))?;
    let velocity = Vector3::new_checked(v, crate::forces::ForceUnit::VELOCITY)
        .map_err(|e| IntegrationError::from_env(e))?;
    TranslationalState::new(position, velocity).map_err(IntegrationError::Dynamics)
}

fn require_positive_step(h: f64) -> Result<(), IntegrationError> {
    match require_finite(h) {
        Ok(_) => {}
        Err(e) => return Err(IntegrationError::from_env(e)),
    }
    if h <= 0.0 {
        return Err(IntegrationError::InvalidStepSize(h));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors produced while integrating one step.
#[derive(Debug, Clone, PartialEq)]
pub enum IntegrationError {
    /// A numeric intermediate is NaN or infinite (diverging step).
    NonFiniteValue(NonFiniteKind),
    /// Step size was not finite and strictly positive.
    InvalidStepSize(f64),
    /// The slope function failed.
    Slope(crate::dynamics::DynamicsError),
    /// State reconstruction failed (unit signature violation).
    Dynamics(crate::dynamics::DynamicsError),
}

impl IntegrationError {
    fn from_env(error: crate::environment::EnvironmentError) -> Self {
        use crate::environment::EnvironmentError as E;
        match error {
            E::NonFiniteValue(kind) => Self::NonFiniteValue(kind),
            other => Self::Dynamics(crate::dynamics::DynamicsError::from(other)),
        }
    }
}

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue(kind) => write!(f, "numerical value is not finite: {kind}"),
            Self::InvalidStepSize(h) => {
                write!(f, "step size must be finite and strictly positive, got {h}")
            }
            Self::Slope(error) | Self::Dynamics(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for IntegrationError {}

impl From<crate::dynamics::DynamicsError> for IntegrationError {
    fn from(error: crate::dynamics::DynamicsError) -> Self {
        Self::Slope(error)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamics::{state_to_derivative, POSITION_UNIT};
    use crate::forces::ForceUnit;

    fn state(px: f64, vx: f64) -> TranslationalState {
        TranslationalState::new(
            Vector3::new([px, 0.0, 0.0], POSITION_UNIT).unwrap(),
            Vector3::new([vx, 0.0, 0.0], ForceUnit::VELOCITY).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn euler_is_exact_for_constant_velocity_zero_acceleration() {
        // f(s) = (v, 0): pure translation; Euler has no truncation error here.
        let slope = |s: &TranslationalState| -> Result<StateDerivative, crate::dynamics::DynamicsError> {
            Ok(StateDerivative::new(s.velocity(), Vector3::new([0.0; 3], ForceUnit::ACCELERATION).unwrap()).unwrap())
        };
        let s0 = state(1.0, 2.0);
        let s1 = euler_step(&s0, &slope, 0.5).unwrap();
        assert_eq!(s1.position().components()[0], 2.0); // 1 + 2*0.5
        assert_eq!(s1.velocity().components()[0], 2.0);
    }

    #[test]
    fn rk4_is_exact_for_linear_in_time_free_flight() {
        // Constant acceleration: x(t)=x0+v0 t+½ a t². RK4 integrates
        // quadratics exactly (its local error term involves the third
        // derivative, which vanishes).
        let mass = 2.0;
        let force = Vector3::new([4.0, 0.0, 0.0], ForceUnit::NEWTON).unwrap();
        let slope = |s: &TranslationalState| -> Result<StateDerivative, crate::dynamics::DynamicsError> {
            state_to_derivative(s, &force, mass)
        };
        let s0 = state(0.0, 1.0); // x0=0, v0=1 ; a = 2
        let h = 0.25;
        let s1 = rk4_step(&s0, &slope, h).unwrap();
        // exact: x = 1*0.25 + ½*2*0.0625 = 0.3125 ; v = 1 + 2*0.25 = 1.5
        assert_eq!(s1.position().components()[0], 0.3125);
        assert_eq!(s1.velocity().components()[0], 1.5);
    }

    #[test]
    fn euler_and_rk4_agree_on_small_steps_within_expected_order() {
        // Exponential-free quadratic system: Euler error O(h²), RK4 ~exact.
        let mass = 1.0;
        let force = Vector3::new([1.0, 0.0, 0.0], ForceUnit::NEWTON).unwrap();
        let slope = |s: &TranslationalState| -> Result<StateDerivative, crate::dynamics::DynamicsError> {
            state_to_derivative(s, &force, mass)
        };
        let s0 = state(0.0, 0.0);
        let he = 1e-3;
        let euler = euler_step(&s0, &slope, he).unwrap();
        let rk4 = rk4_step(&s0, &slope, he).unwrap();
        // exact x = 5e-7; Euler gives x = 0 (v0 = 0) → error = h²/2 ✓
        assert_eq!(euler.position().components()[0], 0.0);
        let expected = 0.5 * he * he;
        let got = rk4.position().components()[0];
        assert!(
            (got - expected).abs() <= f64::EPSILON * expected.abs(),
            "RK4 should match the analytic value up to rounding: {got} vs {expected}"
        );
    }

    #[test]
    fn repeated_rk4_runs_are_bit_identical() {
        let mass = 3.0;
        let force = Vector3::new([0.123456789, -2.5, 9.80665], ForceUnit::NEWTON).unwrap();
        let slope = |s: &TranslationalState| -> Result<StateDerivative, crate::dynamics::DynamicsError> {
            state_to_derivative(s, &force, mass)
        };
        let run = || {
            let mut s = state(10.0, -4.25);
            for _ in 0..100 {
                s = rk4_step(&s, &slope, 1.0 / 60.0).unwrap();
            }
            s
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn rejects_invalid_step_sizes() {
        let slope = |s: &TranslationalState| -> Result<StateDerivative, crate::dynamics::DynamicsError> {
            Ok(StateDerivative::new(s.velocity(), Vector3::new([0.0; 3], ForceUnit::ACCELERATION).unwrap()).unwrap())
        };
        let s0 = state(0.0, 1.0);
        for h in [0.0, -0.5, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                euler_step(&s0, &slope, h),
                Err(IntegrationError::InvalidStepSize(_))
                    | Err(IntegrationError::NonFiniteValue(_))
            ));
            assert!(matches!(
                rk4_step(&s0, &slope, h),
                Err(IntegrationError::InvalidStepSize(_))
                    | Err(IntegrationError::NonFiniteValue(_))
            ));
        }
    }

    #[test]
    fn diverging_step_is_refused_not_leaked() {
        // Huge slope × huge step overflows → must error, never produce inf.
        let mass = 1e-300;
        let force = Vector3::new([f64::MAX, 0.0, 0.0], ForceUnit::NEWTON).unwrap();
        let slope = |s: &TranslationalState| -> Result<StateDerivative, crate::dynamics::DynamicsError> {
            state_to_derivative(s, &force, mass)
        };
        let s0 = state(0.0, 0.0);
        assert!(rk4_step(&s0, &slope, 1.0).is_err());
    }

    #[test]
    fn slope_function_errors_propagate() {
        let slope = |_s: &TranslationalState| -> Result<StateDerivative, crate::dynamics::DynamicsError> {
            Err(crate::dynamics::DynamicsError::InvalidMass(0.0))
        };
        let err = euler_step(&state(0.0, 0.0), &slope, 0.1).unwrap_err();
        assert!(matches!(err, IntegrationError::Slope(_)));
    }
}
