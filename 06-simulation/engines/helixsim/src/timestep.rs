//! Deterministic simulation timestep policy (Layer 06 / HELIXSIM).
//!
//! This module decides *how long* the next integration step is. It does NOT
//! own logical time: Layer 07 owns the authoritative clock, replay and rewind.
//! Here we only produce a validated step size `Δt = t_(n+1) − t_n` from a
//! policy plus the caller-supplied current logical time — never from a wall
//! clock, thread scheduling or hidden state.
//!
//! ## Policy model
//!
//! ```text
//! Δt_raw        = policy base step
//! Δt            = clamp(Δt_raw, [Δt_min, Δt_max]) then min with stability cap
//! t_(n+1)       = t_n + Δt          (exact f64 arithmetic, IEEE-754)
//! ```
//!
//! The stability cap implements an explicit user-declared condition of the
//! form `Δt ≤ coefficient / max_signal`. We do **not** invent CFL or
//! stability numbers for physics the repository does not model; the cap
//! exists only when the caller supplies it as part of the simulation
//! configuration.
//!
//! ## Numerical safety
//!
//! Every parameter passes finiteness validation at construction; negative or
//! zero bounds are refused rather than clamped. Time advancement uses
//! `f64::checked_add`, so overflow past `f64::MAX` surfaces as
//! [`TimestepError::TimeOverflow`] instead of silently wrapping to infinity.

use std::fmt;

use helqix_core::NonFiniteKind;

use crate::environment::require_finite;

/// A validated strictly-positive timestep value in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepDuration(f64);

impl StepDuration {
    /// Validates `seconds`: finite and strictly positive.
    ///
    /// # Errors
    ///
    /// [`TimestepError::NonFiniteValue`] for NaN/infinity,
    /// [`TimestepError::InvalidStep`] for `<= 0`.
    pub fn new(seconds: f64) -> Result<Self, TimestepError> {
        if let Err(e) = require_finite(seconds) {
            return Err(TimestepError::from_env(e));
        }
        if seconds <= 0.0 {
            return Err(TimestepError::InvalidStep(seconds));
        }
        Ok(Self(seconds))
    }

    /// Returns the raw duration in seconds.
    #[must_use]
    pub const fn as_seconds(&self) -> f64 {
        self.0
    }
}

/// An optional explicit numerical-stability restriction on `Δt`.
///
/// `cap = coefficient / max_signal`, where `max_signal` is a caller-provided
/// upper bound on the fastest quantity the chosen integrator must resolve
/// (for instance a maximum acceleration magnitude). Absence of a limit means
/// the configuration declares no stability restriction — HELIXSIM will not
/// invent one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StabilityLimit {
    coefficient: f64,
    max_signal: f64,
}

impl StabilityLimit {
    /// Creates a limit `Δt ≤ coefficient / max_signal`.
    ///
    /// # Errors
    ///
    /// Non-finite inputs, non-positive coefficient, or zero/negative signal
    /// (which would make the quotient singular) are refused.
    pub fn new(coefficient: f64, max_signal: f64) -> Result<Self, TimestepError> {
        let (c, s) = match (require_finite(coefficient), require_finite(max_signal)) {
            (Ok(c), Ok(s)) => (c, s),
            (Err(e), _) | (_, Err(e)) => return Err(TimestepError::from_env(e)),
        };
        if c <= 0.0 || s <= 0.0 {
            return Err(TimestepError::InvalidConfiguration(format!(
                "stability limit needs positive coefficient and signal, got {c}, {s}"
            )));
        }
        Ok(Self {
            coefficient: c,
            max_signal: s,
        })
    }

    /// The largest step permitted by this limit.
    #[must_use]
    pub fn max_step(&self) -> f64 {
        // Both fields are finite and positive by construction; the quotient
        // can still underflow toward 0.0 which remains a legal (tiny) bound.
        self.coefficient / self.max_signal
    }
}

/// How the next step size is derived.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StepPolicy {
    /// Always use `base` (after clamping into `[min, max]`).
    Fixed {
        /// Nominal step in seconds.
        base: StepDuration,
    },
    /// Halve `base` once per exceeded threshold decade while
    /// `signal > threshold`, floored at `min_step`.
    ///
    /// This is a pure function of `(signal, configuration)` — the same
    /// signal always yields the same step, preserving determinism and
    /// replayability. It models "slow down near fast dynamics" without
    /// reading any clock.
    SignalAdaptive {
        /// Nominal (largest) step in seconds.
        base: StepDuration,
        /// Signal value above which refinement begins.
        threshold: f64,
        /// Smallest permitted refined step.
        min_step: StepDuration,
    },
}

/// Complete timestep configuration: policy plus hard bounds plus optional
/// explicitly-declared stability limit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimestepConfig {
    policy: StepPolicy,
    dt_min: StepDuration,
    dt_max: StepDuration,
    stability: Option<StabilityLimit>,
}

impl TimestepConfig {
    /// Builds a fixed-step configuration.
    ///
    /// # Errors
    ///
    /// [`TimestepError::InvertedBounds`] if `dt_min > dt_max`; other variants
    /// propagate parameter validation failures.
    pub fn fixed(dt: f64, dt_min: f64, dt_max: f64) -> Result<Self, TimestepError> {
        Self::new(
            StepPolicy::Fixed {
                base: StepDuration::new(dt)?,
            },
            dt_min,
            dt_max,
            None,
        )
    }

    /// Builds a fully specified configuration.
    ///
    /// # Errors
    ///
    /// As [`Self::fixed`], plus invalid stability-limit parameters.
    pub fn new(
        policy: StepPolicy,
        dt_min: f64,
        dt_max: f64,
        stability: Option<StabilityLimit>,
    ) -> Result<Self, TimestepError> {
        let dt_min = StepDuration::new(dt_min)?;
        let dt_max = StepDuration::new(dt_max)?;
        if dt_min.as_seconds() > dt_max.as_seconds() {
            return Err(TimestepError::InvertedBounds {
                min: dt_min.as_seconds(),
                max: dt_max.as_seconds(),
            });
        }
        Ok(Self {
            policy,
            dt_min,
            dt_max,
            stability,
        })
    }

    /// Returns the configured policy.
    #[must_use]
    pub const fn policy(&self) -> StepPolicy {
        self.policy
    }

    /// Returns the minimum permitted step.
    #[must_use]
    pub const fn dt_min(&self) -> StepDuration {
        self.dt_min
    }

    /// Returns the maximum permitted step.
    #[must_use]
    pub const fn dt_max(&self) -> StepDuration {
        self.dt_max
    }
}

/// The outcome of one timestep decision: how big the step is and where the
/// simulation lands in logical time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimestepDecision {
    dt: StepDuration,
    current_time: f64,
    next_time: f64,
}

impl TimestepDecision {
    /// The validated step size `Δt`.
    #[must_use]
    pub const fn dt(&self) -> StepDuration {
        self.dt
    }

    /// Logical time at the start of the step (`t_n`).
    #[must_use]
    pub const fn current_time(&self) -> f64 {
        self.current_time
    }

    /// Logical time at the end of the step (`t_(n+1)`).
    #[must_use]
    pub const fn next_time(&self) -> f64 {
        self.next_time
    }
}

/// A deterministic timestep scheduler bound to one logical instant.
///
/// The scheduler carries its own `current_time` field supplied by the caller
/// (ultimately from Layer 07). Advancing produces a *new* scheduler; nothing
/// mutates globally.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimestepScheduler {
    config: TimestepConfig,
    current_time: f64,
}

impl TimestepScheduler {
    /// Creates a scheduler positioned at logical time `t_n`.
    ///
    /// # Errors
    ///
    /// Non-finite `current_time` or invalid configuration values.
    pub fn new(config: TimestepConfig, current_time: f64) -> Result<Self, TimestepError> {
        match require_finite(current_time) {
            Ok(t) => Ok(Self {
                config,
                current_time: t,
            }),
            Err(e) => Err(TimestepError::from_env(e)),
        }
    }

    /// Decides the step starting at the current logical time using the given
    /// observed signal (used only by [`StepPolicy::SignalAdaptive`]; ignored
    /// otherwise, keeping the API uniform).
    ///
    /// # Errors
    ///
    /// [`TimestepError::NonFiniteValue`] if the signal is NaN/infinite while
    /// the adaptive policy needs it; [`TimestepError::TimeOverflow`] if
    /// `t_n + Δt` exceeds finite range;
    /// [`TimestepError::InvalidConfiguration`] on contradictory settings.
    pub fn decide(&self, signal: Option<f64>) -> Result<TimestepDecision, TimestepError> {
        let raw = match self.config.policy {
            StepPolicy::Fixed { base } => base.as_seconds(),
            StepPolicy::SignalAdaptive {
                base,
                threshold,
                min_step,
            } => {
                let s = signal.ok_or_else(|| {
                    TimestepError::InvalidConfiguration(
                        "adaptive policy requires a signal value".to_owned(),
                    )
                })?;
                if let Err(e) = require_finite(s) {
                    return Err(TimestepError::from_env(e));
                }
                if s > threshold {
                    // Deterministic geometric refinement: halve once per
                    // exceeded threshold decade (signal/threshold ∈ (1,2] →
                    // one halving, (2,4] → two, …), floored at min_step. A
                    // pure function of the signal — identical signals always
                    // yield identical steps, preserving replay determinism.
                    let mut step = base.as_seconds();
                    let mut level = threshold;
                    while s > level && step > min_step.as_seconds() {
                        step *= 0.5;
                        level *= 2.0;
                    }
                    step.max(min_step.as_seconds())
                } else {
                    base.as_seconds()
                }
            }
        };

        let mut dt = raw;
        if dt < self.config.dt_min.as_seconds() {
            dt = self.config.dt_min.as_seconds();
        }
        if dt > self.config.dt_max.as_seconds() {
            dt = self.config.dt_max.as_seconds();
        }
        if let Some(limit) = self.config.stability {
            let cap = limit.max_step();
            if dt > cap {
                dt = cap;
            }
            // If the declared cap falls below dt_min the configuration is
            // contradictory; surface it instead of silently violating either.
            if dt < self.config.dt_min.as_seconds() {
                return Err(TimestepError::InvalidConfiguration(format!(
                    "stability cap {cap} conflicts with dt_min {}",
                    self.config.dt_min.as_seconds()
                )));
            }
        }

        let dt = StepDuration::new(dt)?;
        // `f64` has no checked_add (that is an integer API); the finiteness
        // check below is the equivalent guard for IEEE-754 addition: overflow
        // yields infinity, which we refuse instead of wrapping silently.
        let next_time = self.current_time + dt.as_seconds();
        if !next_time.is_finite() {
            return Err(TimestepError::TimeOverflow {
                current: self.current_time,
                dt: dt.as_seconds(),
            });
        }

        Ok(TimestepDecision {
            dt,
            current_time: self.current_time,
            next_time,
        })
    }

    /// Returns a decision plus the scheduler advanced to the end of that step.
    ///
    /// Purely functional: `advance` consumes `self` (Copy) and returns the new
    /// position, so callers keep full control over committing time — HELIXSIM
    /// never writes back into Layer 07.
    ///
    /// # Errors
    ///
    /// As [`Self::decide`].
    pub fn advance(&self, signal: Option<f64>) -> Result<(TimestepDecision, Self), TimestepError> {
        let decision = self.decide(signal)?;
        let next = Self {
            config: self.config,
            current_time: decision.next_time,
        };
        Ok((decision, next))
    }

    /// Current logical time position of the scheduler.
    #[must_use]
    pub const fn current_time(&self) -> f64 {
        self.current_time
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors produced by timestep policy evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum TimestepError {
    /// A numeric input is NaN or infinite.
    NonFiniteValue(NonFiniteKind),
    /// A step value was not strictly positive.
    InvalidStep(f64),
    /// `dt_min > dt_max` or another internally contradictory bound set.
    InvertedBounds {
        /// Configured minimum.
        min: f64,
        /// Configured maximum.
        max: f64,
    },
    /// Configuration fields contradict each other.
    InvalidConfiguration(String),
    /// `t_n + Δt` left the finite representable range.
    TimeOverflow {
        /// Start of the failed step.
        current: f64,
        /// Attempted step size.
        dt: f64,
    },
}

impl TimestepError {
    fn from_env(error: crate::environment::EnvironmentError) -> Self {
        use crate::environment::EnvironmentError as E;
        match error {
            E::NonFiniteValue(kind) => Self::NonFiniteValue(kind),
            other => Self::InvalidConfiguration(other.to_string()),
        }
    }
}

impl fmt::Display for TimestepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue(kind) => write!(f, "timestep value is not finite: {kind}"),
            Self::InvalidStep(dt) => {
                write!(f, "timestep must be strictly positive, got {dt}")
            }
            Self::InvertedBounds { min, max } => {
                write!(f, "timestep bounds inverted: min {min} > max {max}")
            }
            Self::InvalidConfiguration(detail) => {
                write!(f, "invalid timestep configuration: {detail}")
            }
            Self::TimeOverflow { current, dt } => {
                write!(f, "logical time overflow adding {dt} to {current}")
            }
        }
    }
}

impl std::error::Error for TimestepError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_policy_produces_exact_constant_steps() {
        let cfg = TimestepConfig::fixed(0.1, 1e-9, 10.0).unwrap();
        let sched = TimestepScheduler::new(cfg, 0.0).unwrap();
        let d0 = sched.decide(None).unwrap();
        assert_eq!(d0.dt().as_seconds(), 0.1);
        assert_eq!(d0.current_time(), 0.0);
        // Exact IEEE addition: 0.0 + 0.1 == 0.1 bit-for-bit.
        assert_eq!(d0.next_time(), 0.1);
    }

    #[test]
    fn deterministic_timestep_sequence_over_many_steps() {
        let cfg = TimestepConfig::fixed(0.25, 1e-9, 1.0).unwrap();
        let mut sched = TimestepScheduler::new(cfg, 0.0).unwrap();
        let mut run_a = Vec::new();
        for _ in 0..40 {
            let (d, next) = sched.advance(None).unwrap();
            run_a.push((d.dt().as_seconds(), d.next_time()));
            sched = next;
        }
        // Independent replay from scratch must reproduce every step exactly.
        let mut sched2 = TimestepScheduler::new(cfg, 0.0).unwrap();
        for expected in run_a {
            let (d, next) = sched2.advance(None).unwrap();
            assert_eq!((d.dt().as_seconds(), d.next_time()), expected);
            sched2 = next;
        }
    }

    #[test]
    fn rejects_invalid_step_values() {
        assert!(matches!(
            StepDuration::new(0.0),
            Err(TimestepError::InvalidStep(0.0))
        ));
        assert!(matches!(
            StepDuration::new(-1.0),
            Err(TimestepError::InvalidStep(-1.0))
        ));
        assert!(matches!(
            StepDuration::new(f64::NAN),
            Err(TimestepError::NonFiniteValue(NonFiniteKind::Nan))
        ));
        assert!(matches!(
            StepDuration::new(f64::INFINITY),
            Err(TimestepError::NonFiniteValue(
                NonFiniteKind::PositiveInfinity
            ))
        ));
        assert!(matches!(
            StepDuration::new(f64::NEG_INFINITY),
            Err(TimestepError::NonFiniteValue(
                NonFiniteKind::NegativeInfinity
            ))
        ));
    }

    #[test]
    fn rejects_inverted_bounds() {
        let err = TimestepConfig::fixed(0.1, 1.0, 0.5).unwrap_err();
        assert_eq!(err, TimestepError::InvertedBounds { min: 1.0, max: 0.5 });
    }

    #[test]
    fn clamps_base_into_bounds() {
        // Base above max → clamped to max.
        let cfg = TimestepConfig::fixed(5.0, 0.01, 1.0).unwrap();
        let sched = TimestepScheduler::new(cfg, 0.0).unwrap();
        assert_eq!(sched.decide(None).unwrap().dt().as_seconds(), 1.0);
        // Base below min → clamped up to min.
        let cfg = TimestepConfig::fixed(1e-12, 1e-6, 1.0).unwrap();
        let sched = TimestepScheduler::new(cfg, 0.0).unwrap();
        assert_eq!(sched.decide(None).unwrap().dt().as_seconds(), 1e-6);
    }

    #[test]
    fn boundary_values_are_accepted() {
        // Exactly at min and at max — degenerate but legal band.
        let cfg = TimestepConfig::fixed(1.0, 1.0, 1.0).unwrap();
        let sched = TimestepScheduler::new(cfg, 0.0).unwrap();
        assert_eq!(sched.decide(None).unwrap().dt().as_seconds(), 1.0);
    }

    #[test]
    fn stability_limit_caps_step() {
        let limit = StabilityLimit::new(0.5, 10.0).unwrap(); // cap = 0.05
        let cfg = TimestepConfig::new(
            StepPolicy::Fixed {
                base: StepDuration::new(0.1).unwrap(),
            },
            1e-9,
            1.0,
            Some(limit),
        )
        .unwrap();
        let sched = TimestepScheduler::new(cfg, 0.0).unwrap();
        assert_eq!(sched.decide(None).unwrap().dt().as_seconds(), 0.05);
    }

    #[test]
    fn stability_limit_conflicting_with_dt_min_is_reported() {
        let limit = StabilityLimit::new(0.5, 10.0).unwrap(); // cap = 0.05
        let cfg = TimestepConfig::new(
            StepPolicy::Fixed {
                base: StepDuration::new(0.1).unwrap(),
            },
            0.08,
            1.0,
            Some(limit),
        )
        .unwrap();
        let sched = TimestepScheduler::new(cfg, 0.0).unwrap();
        assert!(matches!(
            sched.decide(None),
            Err(TimestepError::InvalidConfiguration(_))
        ));
    }

    #[test]
    fn stability_limit_rejects_singular_input() {
        assert!(StabilityLimit::new(0.5, 0.0).is_err());
        assert!(StabilityLimit::new(-1.0, 5.0).is_err());
        assert!(matches!(
            StabilityLimit::new(f64::NAN, 5.0),
            Err(TimestepError::NonFiniteValue(NonFiniteKind::Nan))
        ));
    }

    #[test]
    fn adaptive_policy_is_deterministic_function_of_signal() {
        let cfg = TimestepConfig::new(
            StepPolicy::SignalAdaptive {
                base: StepDuration::new(0.2).unwrap(),
                threshold: 100.0,
                min_step: StepDuration::new(0.05).unwrap(),
            },
            1e-9,
            1.0,
            None,
        )
        .unwrap();
        let sched = TimestepScheduler::new(cfg, 0.0).unwrap();
        // Below threshold → base step.
        assert_eq!(sched.decide(Some(99.0)).unwrap().dt().as_seconds(), 0.2);
        // One decade over (100,200] → one halving.
        assert_eq!(sched.decide(Some(150.0)).unwrap().dt().as_seconds(), 0.1);
        // Two decades over (200,400] → two halvings.
        assert_eq!(sched.decide(Some(400.0)).unwrap().dt().as_seconds(), 0.05);
        // Extreme signal → floored at min_step, never below.
        assert_eq!(sched.decide(Some(1e300)).unwrap().dt().as_seconds(), 0.05);
        // Same signal twice → identical decision.
        assert_eq!(sched.decide(Some(150.0)), sched.decide(Some(150.0)));
        // Adaptive without a signal is a configuration misuse, not a guess.
        assert!(matches!(
            sched.decide(None),
            Err(TimestepError::InvalidConfiguration(_))
        ));
        // NaN signal is refused.
        assert!(matches!(
            sched.decide(Some(f64::NAN)),
            Err(TimestepError::NonFiniteValue(NonFiniteKind::Nan))
        ));
    }

    #[test]
    fn time_overflow_is_reported_not_wrapped() {
        // dt_max itself is near f64::MAX so t_n + Δt overflows to infinity.
        let cfg = TimestepConfig::fixed(f64::MAX, 1e-9, f64::MAX).unwrap();
        let sched = TimestepScheduler::new(cfg, f64::MAX / 2.0).unwrap();
        assert!(matches!(
            sched.decide(None),
            Err(TimestepError::TimeOverflow { .. })
        ));
    }

    #[test]
    fn scheduler_does_not_mutate_itself_on_decide() {
        let cfg = TimestepConfig::fixed(0.1, 1e-9, 1.0).unwrap();
        let sched = TimestepScheduler::new(cfg, 3.0).unwrap();
        let _ = sched.decide(None).unwrap();
        assert_eq!(sched.current_time(), 3.0);
    }

    #[test]
    fn logical_time_consistency_across_advance_chain() {
        let cfg = TimestepConfig::fixed(0.5, 1e-9, 1.0).unwrap();
        let sched = TimestepScheduler::new(cfg, 10.0).unwrap();
        let (d1, s1) = sched.advance(None).unwrap();
        let (d2, _s2) = s1.advance(None).unwrap();
        assert_eq!(d1.next_time(), d2.current_time());
        assert_eq!(d2.next_time(), 11.0);
    }

    #[test]
    fn negative_current_time_is_allowed_and_advances_correctly() {
        // Logical time origin is a convention; pre-origin times must work.
        let cfg = TimestepConfig::fixed(0.5, 1e-9, 1.0).unwrap();
        let sched = TimestepScheduler::new(cfg, -2.0).unwrap();
        let d = sched.decide(None).unwrap();
        assert_eq!(d.next_time(), -1.5);
    }
}
