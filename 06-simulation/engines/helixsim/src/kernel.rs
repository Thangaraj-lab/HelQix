//! Bounded simulation orchestration over caller-supplied continuous models.

use std::fmt;

use crate::{solver::{advance, DerivativeModel, IntegrationMethod}, timestep::{TimeGrid, TimeGridError}};

/// Upper bound for a single in-memory result's recorded samples.
pub const MAX_RECORDED_SAMPLES: usize = 250_000;
/// Upper bound for state values retained by one result (at most 128 MiB of f64 data).
pub const MAX_RECORDED_STATE_VALUES: usize = 16_000_000;

/// A continuous-time model contract for HELIXSIM.
pub trait ContinuousModel: DerivativeModel {
    /// Stable model identifier recorded with the resulting run.
    fn model_id(&self) -> &str;

    /// Model version or immutable parameter-set identifier recorded with the run.
    fn model_version(&self) -> &str;
}

/// Numerical choices and resource bounds for one run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimulationOptions {
    /// Fixed maximum step in seconds; the final step may be shorter.
    pub step_seconds: f64,
    /// Maximum allowed integration interval count.
    pub max_steps: u64,
    /// Explicit numerical method. HELIXSIM never chooses one silently.
    pub method: IntegrationMethod,
}

impl Default for SimulationOptions {
    fn default() -> Self {
        Self { step_seconds: 0.01, max_steps: 100_000, method: IntegrationMethod::RungeKutta4 }
    }
}

/// One recorded state sample at a simulation time in seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct StateSample {
    /// Logical model time in seconds.
    pub time_seconds: f64,
    /// Caller-defined continuous state vector.
    pub state: Vec<f64>,
}

/// Successful simulation output, with explicit model/method provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct SimulationResult {
    /// Model identifier supplied by the model implementation.
    pub model_id: String,
    /// Model version supplied by the model implementation.
    pub model_version: String,
    /// Integration method selected by the caller.
    pub method: IntegrationMethod,
    /// Configured maximum integration interval.
    pub configured_step_seconds: f64,
    /// Initial and accepted final states, including both time endpoints.
    pub samples: Vec<StateSample>,
}

/// Rejection or model failure during a simulation.
#[derive(Debug, Clone, PartialEq)]
pub enum SimulationError {
    /// Invalid run timeline or step bound.
    TimeGrid(TimeGridError),
    /// State vector was empty, excessive, or contained NaN/infinity.
    InvalidInitialState,
    /// Model returned an empty or overly long identifier/version.
    InvalidModelIdentity,
    /// Requested output would exceed the engine's bounded in-memory result size.
    OutputLimitExceeded {
        /// Requested number of samples, including the initial state.
        requested_samples: usize,
        /// Requested number of retained scalar state values.
        requested_state_values: usize,
    },
    /// Model callback failed; its message is retained as untrusted diagnostic data.
    Model(String),
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TimeGrid(error) => write!(f, "invalid simulation time grid: {error}"),
            Self::InvalidInitialState => write!(f, "initial state must contain 1..=4096 finite values"),
            Self::InvalidModelIdentity => write!(f, "model identifier and version must be non-empty and at most 200 characters"),
            Self::OutputLimitExceeded { requested_samples, requested_state_values } => write!(f, "simulation output requests {requested_samples} samples and {requested_state_values} state values; limits are {MAX_RECORDED_SAMPLES} samples and {MAX_RECORDED_STATE_VALUES} state values"),
            Self::Model(message) => write!(f, "model evaluation failed: {message}"),
        }
    }
}

impl std::error::Error for SimulationError {}

impl From<TimeGridError> for SimulationError {
    fn from(error: TimeGridError) -> Self { Self::TimeGrid(error) }
}

/// Runs a deterministic fixed-step integration and records every accepted state.
///
/// The function includes the initial state and the exact final-time state in
/// its output. It refuses invalid model output rather than returning a partial
/// result as a successful run. Physical validity, dimensional consistency,
/// solver suitability, and fidelity are declared by higher layers and are not
/// inferred here.
pub fn simulate<M: ContinuousModel + ?Sized>(
    model: &M,
    start_seconds: f64,
    end_seconds: f64,
    initial_state: &[f64],
    options: SimulationOptions,
) -> Result<SimulationResult, SimulationError> {
    if initial_state.is_empty() || initial_state.len() > 4096 || initial_state.iter().any(|value| !value.is_finite()) {
        return Err(SimulationError::InvalidInitialState);
    }
    let model_id = model.model_id().trim();
    let model_version = model.model_version().trim();
    if model_id.is_empty() || model_version.is_empty() || model_id.len() > 200 || model_version.len() > 200 {
        return Err(SimulationError::InvalidModelIdentity);
    }
    TimeGrid::new(start_seconds, end_seconds, options.step_seconds, options.max_steps)?;
    let ratio = (end_seconds - start_seconds) / options.step_seconds;
    let nearest = ratio.round();
    let tolerance = f64::EPSILON * ratio.abs().max(1.0) * 16.0;
    let required_steps = (if (ratio - nearest).abs() <= tolerance { nearest } else { ratio.ceil() }) as usize;
    let requested_samples = required_steps.saturating_add(1);
    let requested_state_values = requested_samples.saturating_mul(initial_state.len());
    if requested_samples > MAX_RECORDED_SAMPLES || requested_state_values > MAX_RECORDED_STATE_VALUES {
        return Err(SimulationError::OutputLimitExceeded { requested_samples, requested_state_values });
    }
    let mut samples = Vec::with_capacity(requested_samples);
    let mut time = start_seconds;
    let mut state = initial_state.to_vec();
    samples.push(StateSample { time_seconds: time, state: state.clone() });

    for step_index in 1..=required_steps {
        // Compute each timestamp from the start plus its integer index rather
        // than accumulating dt. This prevents long runs from drifting past
        // their validated step count due to repeated floating-point addition.
        let next_time = if step_index == required_steps {
            end_seconds
        } else {
            (start_seconds + step_index as f64 * options.step_seconds).min(end_seconds)
        };
        let step = next_time - time;
        if !step.is_finite() || step <= 0.0 {
            return Err(SimulationError::Model("time step is below representable precision at this simulation time".to_owned()));
        }
        let next = advance(model, options.method, time, step, &state).map_err(SimulationError::Model)?;
        if !next_time.is_finite() || next_time <= time {
            return Err(SimulationError::Model("time step did not advance the simulation".to_owned()));
        }
        time = next_time;
        state = next;
        samples.push(StateSample { time_seconds: time, state: state.clone() });
    }

    Ok(SimulationResult {
        model_id: model_id.to_owned(), model_version: model_version.to_owned(),
        method: options.method, configured_step_seconds: options.step_seconds, samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ConstantAcceleration;
    impl DerivativeModel for ConstantAcceleration {
        fn derivative(&self, _time: f64, state: &[f64], derivative: &mut [f64]) -> Result<(), String> {
            if state.len() != 2 || derivative.len() != 2 { return Err("expected position and velocity".into()); }
            derivative[0] = state[1];
            derivative[1] = 2.0;
            Ok(())
        }
    }
    impl ContinuousModel for ConstantAcceleration {
        fn model_id(&self) -> &str { "test.constant-acceleration" }
        fn model_version(&self) -> &str { "1" }
    }

    #[test]
    fn rk4_records_exact_end_time_and_accurate_acceleration_state() {
        let result = simulate(&ConstantAcceleration, 0.0, 1.0, &[0.0, 0.0], SimulationOptions { step_seconds: 0.3, max_steps: 4, method: IntegrationMethod::RungeKutta4 }).unwrap();
        assert_eq!(result.samples.len(), 5);
        assert_eq!(result.samples.last().unwrap().time_seconds, 1.0);
        assert!((result.samples.last().unwrap().state[0] - 1.0).abs() < 1e-12);
        assert!((result.samples.last().unwrap().state[1] - 2.0).abs() < 1e-12);
        assert_eq!(result.model_id, "test.constant-acceleration");
    }

    #[test]
    fn rejects_bad_state_and_non_finite_model_derivative() {
        assert_eq!(simulate(&ConstantAcceleration, 0.0, 1.0, &[f64::NAN], SimulationOptions::default()).unwrap_err(), SimulationError::InvalidInitialState);
        let invalid = |_: f64, state: &[f64], derivative: &mut [f64]| -> Result<(), String> { derivative[0] = f64::NAN; assert_eq!(state.len(), 1); Ok(()) };
        struct BadModel<F>(F);
        impl<F> DerivativeModel for BadModel<F> where F: Fn(f64, &[f64], &mut [f64]) -> Result<(), String> {
            fn derivative(&self, t: f64, y: &[f64], d: &mut [f64]) -> Result<(), String> { (self.0)(t, y, d) }
        }
        impl<F> ContinuousModel for BadModel<F> where F: Fn(f64, &[f64], &mut [f64]) -> Result<(), String> {
            fn model_id(&self) -> &str { "bad" }
            fn model_version(&self) -> &str { "1" }
        }
        let result = simulate(&BadModel(invalid), 0.0, 1.0, &[1.0], SimulationOptions::default());
        assert!(matches!(result, Err(SimulationError::Model(_))));
    }

    #[test]
    fn euler_produces_declared_method_and_bounded_samples() {
        let result = simulate(&ConstantAcceleration, 0.0, 1.0, &[0.0, 0.0], SimulationOptions { step_seconds: 0.25, max_steps: 4, method: IntegrationMethod::ExplicitEuler }).unwrap();
        assert_eq!(result.method, IntegrationMethod::ExplicitEuler);
        assert_eq!(result.samples.len(), 5);
        assert_eq!(result.samples.last().unwrap().state[1], 2.0);
    }

    #[test]
    fn decimal_step_schedule_has_no_drift_extra_sample() {
        let result = simulate(
            &ConstantAcceleration,
            0.0,
            1.0,
            &[0.0, 0.0],
            SimulationOptions { step_seconds: 0.1, max_steps: 10, method: IntegrationMethod::RungeKutta4 },
        ).unwrap();
        assert_eq!(result.samples.len(), 11);
        assert_eq!(result.samples.last().unwrap().time_seconds, 1.0);
    }

    #[test]
    fn rejects_output_that_exceeds_the_memory_budget_before_integrating() {
        let state = vec![0.0; 4096];
        let error = simulate(
            &ConstantAcceleration,
            0.0,
            4.0,
            &state,
            SimulationOptions {
                step_seconds: 0.001,
                max_steps: 4000,
                method: IntegrationMethod::ExplicitEuler,
            },
        )
        .unwrap_err();
        assert!(matches!(error, SimulationError::OutputLimitExceeded { .. }));
    }
}
