//! Bounded, deterministic fixed-step simulation timelines.

use std::fmt;

/// Maximum number of integration intervals accepted in one run.
pub const HARD_MAX_STEPS: u64 = 10_000_000;

/// A validated, finite simulation interval and maximum integration step.
///
/// The final interval is shortened when necessary so the final sample lands
/// exactly on `end_seconds`. Time is expressed in seconds; a caller that uses
/// another time base must convert it before constructing this value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeGrid {
    start_seconds: f64,
    end_seconds: f64,
    max_step_seconds: f64,
    max_steps: u64,
}

impl TimeGrid {
    /// Creates a validated grid. `max_steps` must be between 1 and
    /// [`HARD_MAX_STEPS`].
    pub fn new(
        start_seconds: f64,
        end_seconds: f64,
        max_step_seconds: f64,
        max_steps: u64,
    ) -> Result<Self, TimeGridError> {
        if !start_seconds.is_finite() || !end_seconds.is_finite() {
            return Err(TimeGridError::NonFiniteTime);
        }
        if !max_step_seconds.is_finite() {
            return Err(TimeGridError::NonFiniteStep);
        }
        if end_seconds <= start_seconds {
            return Err(TimeGridError::NonIncreasingInterval);
        }
        if max_step_seconds <= 0.0 {
            return Err(TimeGridError::NonPositiveStep);
        }
        if max_steps == 0 || max_steps > HARD_MAX_STEPS {
            return Err(TimeGridError::InvalidStepLimit { max_steps });
        }
        let ratio = (end_seconds - start_seconds) / max_step_seconds;
        let nearest = ratio.round();
        let integral_tolerance = f64::EPSILON * ratio.abs().max(1.0) * 16.0;
        let required = if (ratio - nearest).abs() <= integral_tolerance { nearest } else { ratio.ceil() };
        if !required.is_finite() || required > max_steps as f64 {
            return Err(TimeGridError::StepLimitExceeded {
                required: if required.is_finite() { required as u64 } else { u64::MAX },
                max_steps,
            });
        }
        Ok(Self { start_seconds, end_seconds, max_step_seconds, max_steps })
    }

    /// Start time in seconds.
    #[must_use]
    pub const fn start_seconds(self) -> f64 { self.start_seconds }

    /// End time in seconds.
    #[must_use]
    pub const fn end_seconds(self) -> f64 { self.end_seconds }

    /// Maximum step size in seconds.
    #[must_use]
    pub const fn max_step_seconds(self) -> f64 { self.max_step_seconds }

    /// Maximum accepted interval count.
    #[must_use]
    pub const fn max_steps(self) -> u64 { self.max_steps }
}

/// Rejection reason for an invalid or unbounded simulation timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeGridError {
    /// Start or end time was NaN or infinite.
    NonFiniteTime,
    /// Step size was NaN or infinite.
    NonFiniteStep,
    /// End time did not follow start time.
    NonIncreasingInterval,
    /// Step size was zero or negative.
    NonPositiveStep,
    /// The caller supplied zero or an excessive maximum step count.
    InvalidStepLimit {
        /// Invalid configured step limit.
        max_steps: u64,
    },
    /// The requested interval requires more steps than the declared limit.
    StepLimitExceeded {
        /// Estimated number of required integration intervals.
        required: u64,
        /// Configured maximum interval count.
        max_steps: u64,
    },
}

impl fmt::Display for TimeGridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteTime => write!(f, "simulation start and end times must be finite"),
            Self::NonFiniteStep => write!(f, "simulation step size must be finite"),
            Self::NonIncreasingInterval => write!(f, "simulation end time must follow start time"),
            Self::NonPositiveStep => write!(f, "simulation step size must be greater than zero"),
            Self::InvalidStepLimit { max_steps } => write!(f, "step limit {max_steps} is outside 1..={HARD_MAX_STEPS}"),
            Self::StepLimitExceeded { required, max_steps } => write!(f, "simulation requires {required} steps; configured limit is {max_steps}"),
        }
    }
}

impl std::error::Error for TimeGridError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_rejects_non_finite_and_unbounded_requests() {
        assert_eq!(TimeGrid::new(f64::NAN, 1.0, 0.1, 10).unwrap_err(), TimeGridError::NonFiniteTime);
        assert_eq!(TimeGrid::new(0.0, 1.0, 0.0, 10).unwrap_err(), TimeGridError::NonPositiveStep);
        assert!(matches!(TimeGrid::new(0.0, 10.0, 0.1, 50), Err(TimeGridError::StepLimitExceeded { .. })));
    }

    #[test]
    fn grid_accepts_partial_final_interval() {
        let grid = TimeGrid::new(0.0, 1.0, 0.3, 4).unwrap();
        assert_eq!(grid.end_seconds(), 1.0);
        assert_eq!(grid.max_steps(), 4);
    }
}
