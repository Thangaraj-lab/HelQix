//! HELIXSIM — HELQIX deterministic physical consequence engine (Layer 06).
//!
//! HELIXSIM provides a bounded numerical integration kernel. It advances an
//! explicitly supplied continuous model and records each accepted sample.
//! It does not infer physics, select a model, or claim that an arbitrary model
//! is valid for a product. Model equations and SI/unit semantics remain the
//! responsibility of the caller and must be recorded by higher layers.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Environmental conditions influencing simulation.
pub mod environment;
/// Force composition for simple physical models.
pub mod forces;
/// Translational point-mass dynamics and trajectory output.
pub mod dynamics;
/// Fixed-step simulation orchestration and validated inputs.
pub mod kernel;
/// Numerical integration methods and derivative contracts.
pub mod solver;
/// Validated simulation time grids and resource bounds.
pub mod timestep;

pub use kernel::{ContinuousModel, SimulationOptions, SimulationResult, simulate};
pub use dynamics::{PointMassModel, PointMassRun, PointMassSample, PointMassState, simulate_point_mass};
pub use forces::{DragConfiguration, ForceBreakdown, ForceError, aerodynamic_drag_force, gravity_force, newton_unit};
pub use solver::IntegrationMethod;
pub use timestep::{TimeGrid, TimeGridError};
