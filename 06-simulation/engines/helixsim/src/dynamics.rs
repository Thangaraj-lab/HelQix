//! SI translational dynamics for a point mass in an explicit environment.
//!
//! This is a reusable first physical model, not a rigid-body, vehicle, pack,
//! structural, or flight solver. Domain models must supply the correct forces,
//! assumptions, and evidence before their outputs can support engineering claims.

use core::fmt;

use crate::{
    environment::{Environment, EnvironmentError, SymbolicUnit, Vector3},
    forces::{aerodynamic_drag_force, gravity_force, newton_unit, sum_forces, DragConfiguration, ForceBreakdown, ForceError},
    kernel::{simulate, ContinuousModel, SimulationError, SimulationOptions},
    solver::DerivativeModel,
};

/// Position in metres and linear velocity in metres per second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointMassState {
    /// Position in coherent SI metres.
    pub position_m: Vector3,
    /// Linear velocity in coherent SI metres per second.
    pub velocity_m_s: Vector3,
}

impl PointMassState {
    /// Creates a state after checking both vector dimensions.
    pub fn new(position_m: Vector3, velocity_m_s: Vector3) -> Result<Self, DynamicsError> {
        if position_m.unit() != SymbolicUnit::METRE || velocity_m_s.unit() != Environment::velocity_unit() {
            return Err(DynamicsError::InvalidStateUnit);
        }
        Ok(Self { position_m, velocity_m_s })
    }

    /// Converts the state to the six-value solver layout `[x,y,z,vx,vy,vz]`.
    #[must_use]
    pub fn to_solver_values(self) -> [f64; 6] {
        let position = self.position_m.components();
        let velocity = self.velocity_m_s.components();
        [position[0], position[1], position[2], velocity[0], velocity[1], velocity[2]]
    }

    /// Builds a typed state from the solver layout `[x,y,z,vx,vy,vz]`.
    pub fn from_solver_values(values: &[f64]) -> Result<Self, DynamicsError> {
        if values.len() != 6 { return Err(DynamicsError::InvalidStateDimension); }
        Self::new(
            Vector3::new([values[0], values[1], values[2]], SymbolicUnit::METRE)?,
            Vector3::new([values[3], values[4], values[5]], Environment::velocity_unit())?,
        )
    }
}

/// A constant-force, point-mass model coupled to gravity and optional quadratic drag.
#[derive(Debug, Clone, PartialEq)]
pub struct PointMassModel {
    id: String,
    version: String,
    mass_kg: f64,
    environment: Environment,
    applied_force_n: Vector3,
    drag: Option<DragConfiguration>,
}

impl PointMassModel {
    /// Constructs a point-mass model with explicit SI mass, environment, force, and drag.
    pub fn new(
        id: impl Into<String>,
        version: impl Into<String>,
        mass_kg: f64,
        environment: Environment,
        applied_force_n: Vector3,
        drag: Option<DragConfiguration>,
    ) -> Result<Self, DynamicsError> {
        let id = id.into().trim().to_owned();
        let version = version.into().trim().to_owned();
        if id.trim().is_empty() || version.trim().is_empty() || id.len() > 200 || version.len() > 200 {
            return Err(DynamicsError::InvalidModelIdentity);
        }
        if !mass_kg.is_finite() { return Err(DynamicsError::NonFiniteMass); }
        if mass_kg <= 0.0 { return Err(DynamicsError::NonPositiveMass); }
        if applied_force_n.unit() != newton_unit() { return Err(DynamicsError::InvalidForceUnit); }
        let drag = drag.map(|value| DragConfiguration::new(value.coefficient, value.reference_area_m2)).transpose()?;
        Ok(Self { id, version, mass_kg, environment, applied_force_n, drag })
    }

    /// Model mass in kilograms.
    #[must_use]
    pub const fn mass_kg(&self) -> f64 { self.mass_kg }

    /// Returns force components evaluated at a body's linear velocity.
    pub fn forces_at(&self, velocity_m_s: &Vector3) -> Result<ForceBreakdown, DynamicsError> {
        let gravity = gravity_force(&self.environment, self.mass_kg)?;
        let drag = match self.drag {
            Some(parameters) => aerodynamic_drag_force(&self.environment, velocity_m_s, parameters)?,
            None => Vector3::zero(newton_unit()),
        };
        let net = sum_forces(&[gravity, self.applied_force_n, drag])?;
        Ok(ForceBreakdown { gravity, applied: self.applied_force_n, drag, net })
    }
}

impl ContinuousModel for PointMassModel {
    fn model_id(&self) -> &str { &self.id }
    fn model_version(&self) -> &str { &self.version }
}

impl DerivativeModel for PointMassModel {
    fn derivative(&self, _time_seconds: f64, state: &[f64], derivative: &mut [f64]) -> Result<(), String> {
        if state.len() != 6 || derivative.len() != 6 {
            return Err("point-mass model requires [x, y, z, vx, vy, vz]".to_owned());
        }
        let velocity = Vector3::new([state[3], state[4], state[5]], Environment::velocity_unit())
            .map_err(|error| error.to_string())?;
        let forces = self.forces_at(&velocity).map_err(|error| error.to_string())?;
        let force = forces.net.components();
        derivative[..3].copy_from_slice(&state[3..6]);
        for axis in 0..3 { derivative[axis + 3] = force[axis] / self.mass_kg; }
        Ok(())
    }
}

/// One typed translational state at a recorded model time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointMassSample {
    /// Simulation time in seconds.
    pub time_seconds: f64,
    /// Position and velocity at that time.
    pub state: PointMassState,
}

/// A successful, provenance-bearing point-mass trajectory.
#[derive(Debug, Clone, PartialEq)]
pub struct PointMassRun {
    /// Model identifier recorded by the solver.
    pub model_id: String,
    /// Immutable model version or parameter-set identifier.
    pub model_version: String,
    /// Explicit numerical method used for integration.
    pub method: crate::solver::IntegrationMethod,
    /// Configured maximum integration interval in seconds.
    pub configured_step_seconds: f64,
    /// Typed SI position/velocity samples.
    pub trajectory: Vec<PointMassSample>,
}

/// Runs the point-mass model and returns both solver provenance and typed motion.
pub fn simulate_point_mass(
    model: &PointMassModel,
    start_seconds: f64,
    end_seconds: f64,
    initial_state: PointMassState,
    options: SimulationOptions,
) -> Result<PointMassRun, SimulationError> {
    let integration = simulate(model, start_seconds, end_seconds, &initial_state.to_solver_values(), options)?;
    let model_id = integration.model_id;
    let model_version = integration.model_version;
    let method = integration.method;
    let configured_step_seconds = integration.configured_step_seconds;
    let trajectory = integration.samples.into_iter().map(|sample| {
        PointMassState::from_solver_values(&sample.state)
            .map(|state| PointMassSample { time_seconds: sample.time_seconds, state })
            .map_err(|error| SimulationError::Model(error.to_string()))
    }).collect::<Result<Vec<_>, _>>()?;
    Ok(PointMassRun { model_id, model_version, method, configured_step_seconds, trajectory })
}

/// Errors raised while constructing or decoding point-mass motion.
#[derive(Debug, Clone, PartialEq)]
pub enum DynamicsError {
    /// The state vectors do not carry metre and metre-per-second signatures.
    InvalidStateUnit,
    /// A solver state did not contain six values.
    InvalidStateDimension,
    /// The model identifier or version is empty or too long.
    InvalidModelIdentity,
    /// Mass is NaN or infinite.
    NonFiniteMass,
    /// Mass is zero or negative.
    NonPositiveMass,
    /// An applied force does not carry the newton signature.
    InvalidForceUnit,
    /// A force calculation or vector construction failed.
    Force(ForceError),
    /// A state vector contained NaN or infinity.
    Environment(EnvironmentError),
}

impl fmt::Display for DynamicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStateUnit => write!(f, "point-mass state requires metres and metres per second"),
            Self::InvalidStateDimension => write!(f, "point-mass state requires six values [x, y, z, vx, vy, vz]"),
            Self::InvalidModelIdentity => write!(f, "model identifier and version must be non-empty and at most 200 characters"),
            Self::NonFiniteMass => write!(f, "mass must be finite"),
            Self::NonPositiveMass => write!(f, "mass must be greater than zero"),
            Self::InvalidForceUnit => write!(f, "applied force must use newtons"),
            Self::Force(error) => write!(f, "point-mass force calculation failed: {error}"),
            Self::Environment(error) => write!(f, "point-mass state construction failed: {error}"),
        }
    }
}

impl std::error::Error for DynamicsError {}

impl From<ForceError> for DynamicsError {
    fn from(error: ForceError) -> Self { Self::Force(error) }
}

impl From<EnvironmentError> for DynamicsError {
    fn from(error: EnvironmentError) -> Self { Self::Environment(error) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{environment::{GravityField, MediumProperties, Scalar}, solver::IntegrationMethod};

    fn environment(gravity_y: f64, density: f64) -> Environment {
        let gravity = GravityField::new(Vector3::new([0.0, gravity_y, 0.0], GravityField::unit()).unwrap()).unwrap();
        let medium = MediumProperties::new(Scalar::new(density, MediumProperties::density_unit()).unwrap(), None, None).unwrap();
        Environment::new(gravity, medium, Vector3::zero(Environment::velocity_unit())).unwrap()
    }

    fn state() -> PointMassState {
        PointMassState::new(Vector3::zero(SymbolicUnit::METRE), Vector3::zero(Environment::velocity_unit())).unwrap()
    }

    #[test]
    fn point_mass_integrates_real_gravitational_motion_and_records_provenance() {
        let model = PointMassModel::new("point-mass", "gravity-v1", 2.0, environment(-9.80665, 0.0), Vector3::zero(newton_unit()), None).unwrap();
        let run = simulate_point_mass(&model, 0.0, 1.0, state(), SimulationOptions { step_seconds: 0.01, max_steps: 100, method: IntegrationMethod::RungeKutta4 }).unwrap();
        let final_state = run.trajectory.last().unwrap().state;
        assert!((final_state.position_m.y() + 4.903325).abs() < 1e-9);
        assert!((final_state.velocity_m_s.y() + 9.80665).abs() < 1e-9);
        assert_eq!(run.model_version, "gravity-v1");
    }

    #[test]
    fn point_mass_applied_force_changes_motion_and_drag_opposes_motion() {
        let force = Vector3::new([10.0, 0.0, 0.0], newton_unit()).unwrap();
        let model = PointMassModel::new("body", "1", 2.0, environment(0.0, 1.225), force, Some(DragConfiguration::new(1.0, 1.0).unwrap())).unwrap();
        let initial = PointMassState::new(Vector3::zero(SymbolicUnit::METRE), Vector3::new([10.0, 0.0, 0.0], Environment::velocity_unit()).unwrap()).unwrap();
        let forces = model.forces_at(&initial.velocity_m_s).unwrap();
        assert!(forces.drag.x() < 0.0);
        assert!(forces.net.x() < 10.0);
        let run = simulate_point_mass(&model, 0.0, 0.5, initial, SimulationOptions { step_seconds: 0.01, max_steps: 50, method: IntegrationMethod::RungeKutta4 }).unwrap();
        assert!(run.trajectory.last().unwrap().state.position_m.x() > initial.position_m.x());
    }

    #[test]
    fn rejects_wrong_state_shape_and_non_positive_mass() {
        assert_eq!(PointMassState::from_solver_values(&[1.0]).unwrap_err(), DynamicsError::InvalidStateDimension);
        assert_eq!(PointMassModel::new("body", "1", 0.0, environment(0.0, 0.0), Vector3::zero(newton_unit()), None).unwrap_err(), DynamicsError::NonPositiveMass);
    }
}
