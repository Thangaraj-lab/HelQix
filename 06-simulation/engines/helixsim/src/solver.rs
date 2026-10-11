//! Deterministic numerical integration primitives.

/// Supported fixed-step integration methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationMethod {
    /// First-order explicit Euler method. Use only where its stability and
    /// error are appropriate for the declared model.
    ExplicitEuler,
    /// Classical fourth-order Runge–Kutta method for smooth ordinary
    /// differential equations.
    RungeKutta4,
}

/// Evaluates `dy/dt` for a model state at a time in seconds.
///
/// Implementations must write exactly one derivative for each state value.
/// The solver checks dimensions and finiteness after every evaluation. It
/// cannot check that the caller's states, derivatives, or equations use
/// consistent physical units.
pub trait DerivativeModel {
    /// Computes state derivatives into `derivative`.
    fn derivative(&self, time_seconds: f64, state: &[f64], derivative: &mut [f64]) -> Result<(), String>;
}

impl<F> DerivativeModel for F
where
    F: Fn(f64, &[f64], &mut [f64]) -> Result<(), String>,
{
    fn derivative(&self, time_seconds: f64, state: &[f64], derivative: &mut [f64]) -> Result<(), String> {
        self(time_seconds, state, derivative)
    }
}

/// Integrates one step and returns the next state.
pub(crate) fn advance<M: DerivativeModel + ?Sized>(
    model: &M,
    method: IntegrationMethod,
    time: f64,
    step: f64,
    state: &[f64],
) -> Result<Vec<f64>, String> {
    match method {
        IntegrationMethod::ExplicitEuler => {
            let k1 = evaluate(model, time, state)?;
            combine(state, step, &[(&k1, 1.0)])
        }
        IntegrationMethod::RungeKutta4 => {
            let k1 = evaluate(model, time, state)?;
            let y2 = combine(state, step, &[(&k1, 0.5)])?;
            let k2 = evaluate(model, time + step * 0.5, &y2)?;
            let y3 = combine(state, step, &[(&k2, 0.5)])?;
            let k3 = evaluate(model, time + step * 0.5, &y3)?;
            let y4 = combine(state, step, &[(&k3, 1.0)])?;
            let k4 = evaluate(model, time + step, &y4)?;
            combine(state, step, &[(&k1, 1.0 / 6.0), (&k2, 1.0 / 3.0), (&k3, 1.0 / 3.0), (&k4, 1.0 / 6.0)])
        }
    }
}

fn evaluate<M: DerivativeModel + ?Sized>(model: &M, time: f64, state: &[f64]) -> Result<Vec<f64>, String> {
    // NaN sentinels make an implementation that forgets to assign a state
    // derivative fail closed rather than silently simulating zero change.
    let mut derivative = vec![f64::NAN; state.len()];
    model.derivative(time, state, &mut derivative)?;
    if derivative.iter().any(|value| !value.is_finite()) {
        return Err("model produced a non-finite derivative".to_owned());
    }
    Ok(derivative)
}

fn combine(state: &[f64], step: f64, terms: &[(&[f64], f64)]) -> Result<Vec<f64>, String> {
    let mut next = state.to_vec();
    for (derivative, coefficient) in terms {
        for (value, rate) in next.iter_mut().zip(derivative.iter()) {
            *value += step * coefficient * rate;
        }
    }
    if next.iter().any(|value| !value.is_finite()) {
        return Err("integration produced a non-finite state".to_owned());
    }
    Ok(next)
}
