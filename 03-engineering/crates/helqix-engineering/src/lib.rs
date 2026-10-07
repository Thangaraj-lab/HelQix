//! HELQIX Layer 03 — Engineering.
//!
//! Layer 03 defines deterministic engineering models and their
//! structural relationships. It is a **definition-only** layer: it
//! describes engineering intent, structure, and constraints without
//! executing, simulating, or otherwise realizing those definitions.
//!
//! # Scope
//!
//! Layer 03 exposes:
//!
//! - **Parameters** — named engineering quantities with declared
//!   dimensional identity (`parameter`, `parameter_set`).
//! - **Equations** — opaque structural relationships over parameters
//!   (`equation`, `equation_set`).
//! - **Interfaces** — typed structural boundaries (`interface`).
//! - **Behaviours** — engineering contracts referencing other
//!   engineering concepts (`behaviour`).
//! - **Constraints** — inclusive boundaries on referenced concepts
//!   (`constraint`, `constraint_set`).
//! - **Dependencies** — directional structural relationships between
//!   engineering definitions (`dependency`, `dependency_graph`).
//! - **Models** — composition of parameters, equations, dependencies,
//!   and constraints under a single identifier (`model`,
//!   `model_registry`).
//! - **Engineering** — the Layer 03 aggregate root owning a
//!   `ModelRegistry` (`engineering`).
//! - **Validation** — deterministic cross-domain validation
//!   (`validation`).
//!
//! # Non-scope
//!
//! Runtime state, execution, simulation, physics, time stepping,
//! observation, failure detection, causal analysis, branching,
//! optimization, AI, quantum execution, and visualization belong to
//! later HELQIX layers and are intentionally absent here.
//!
//! # Dependency direction
//!
//! ```text
//! Foundation (helqix_core)
//!     ↓
//! Reality
//!     ↓
//! Engineering  ← this crate
//!     ↓
//! later runtime layers
//! ```
//!
//! Layer 03 never depends on later layers. `lib.rs` is a pure API
//! boundary: it declares modules and re-exports the public domain types
//! and error types owned by each module.

pub mod behaviour;
pub mod constraint;
pub mod constraint_set;
pub mod dependency;
pub mod dependency_graph;
pub mod engineering;
pub mod equation;
pub mod equation_set;
pub mod interface;
pub mod model;
pub mod model_registry;
pub mod parameter;
pub mod parameter_set;
pub mod validation;

// ---------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------

pub use crate::parameter::{
    ParameterDefinition, ParameterError, ParameterRegistry,
};

pub use crate::parameter_set::{
    ParameterSet, ParameterSetError,
};

// ---------------------------------------------------------------------
// Equations
// ---------------------------------------------------------------------

pub use crate::equation::{
    Equation, EquationError,
};

pub use crate::equation_set::{
    EquationSet, EquationSetError,
};

// ---------------------------------------------------------------------
// Interfaces
// ---------------------------------------------------------------------

pub use crate::interface::{
    IncompatibilityReason, Interface, InterfaceDirection, InterfaceError,
    InterfaceMember,
};

// ---------------------------------------------------------------------
// Behaviours
// ---------------------------------------------------------------------

pub use crate::behaviour::{
    Behaviour, BehaviourClause, BehaviourError, BehaviourReferenceResolver,
    ReferenceKind,
};

// ---------------------------------------------------------------------
// Constraints
// ---------------------------------------------------------------------

pub use crate::constraint::{
    Constraint, ConstraintError, ConstraintReference,
    ConstraintReferenceResolver,
};

pub use crate::constraint_set::{
    ConstraintSet, ConstraintSetError,
};

// ---------------------------------------------------------------------
// Dependencies
// ---------------------------------------------------------------------

pub use crate::dependency::{
    Dependency, DependencyError, DependencyReference,
    DependencyReferenceResolver,
};

pub use crate::dependency_graph::{
    DependencyGraph, DependencyGraphError,
};

// ---------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------

pub use crate::model::{
    Model, ModelError,
};

pub use crate::model_registry::{
    ModelRegistry, ModelRegistryError,
};

// ---------------------------------------------------------------------
// Engineering aggregate
// ---------------------------------------------------------------------

pub use crate::engineering::{
    Engineering, EngineeringError,
};

// ---------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------

pub use crate::validation::{
    DependencyEndpoint, EngineeringValidator, ValidationError,
    ValidationResult,
};