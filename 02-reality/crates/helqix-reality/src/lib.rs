//! HELQIX Reality — structural engineering reality.
//!
//! This crate defines the structural representation of an engineering
//! system before runtime execution.
//!
//! Layer 02 represents:
//!
//! - system identity;
//! - components;
//! - the component registry;
//! - the structural graph;
//! - topology;
//! - environment definitions;
//! - engineering model definitions;
//! - deterministic structural validation.
//!
//! Layer 02 does **not** perform runtime simulation, physics execution,
//! time evolution, optimization, or AI inference. Those responsibilities
//! belong to later HELQIX layers.
//!
//! Foundation types such as `Identifier`, `Quantity`, `Unit`, and
//! `Component` are defined by `helqix_core` and are intentionally not
//! re-exported here. Downstream crates should depend on `helqix_core`
//! directly when they need them.

pub mod reality;
pub mod reality_graph;
pub mod component_registry;
pub mod topology;
pub mod environment;
pub mod model;
pub mod validation;

// Reality
pub use crate::reality::{Reality, RealityError};

// Reality graph
pub use crate::reality_graph::{RealityEdge, RealityGraph, RealityGraphError};

// Component registry
pub use crate::component_registry::{ComponentRegistry, ComponentRegistryError};

// Topology
pub use crate::topology::{
    ConnectionKind, Topology, TopologyConnection, TopologyConnectionKey, TopologyError,
};

// Environment
pub use crate::environment::{Environment, EnvironmentError, EnvironmentParameter};

// Engineering model
pub use crate::model::{EngineeringModel, ModelDefinition, ModelError, ModelReference};

// Validation
pub use crate::validation::{RealityValidator, ValidationError, ValidationResult};