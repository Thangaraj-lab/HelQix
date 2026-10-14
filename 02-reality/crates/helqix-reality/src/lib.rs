//! HELQIX Reality — structural engineering reality.
//!
//! This crate defines the structural representation of an engineering
//! system before runtime execution.

pub mod reality;
pub mod reality_graph;
pub mod component_registry;
pub mod topology;
pub mod environment;
pub mod model;
#[path = "validation_fixed.rs"]
pub mod validation;

pub use crate::reality::{Reality, RealityError};
pub use crate::reality_graph::{RealityEdge, RealityGraph, RealityGraphError};
pub use crate::component_registry::{ComponentRegistry, ComponentRegistryError};
pub use crate::topology::{ConnectionKind, Topology, TopologyConnection, TopologyConnectionKey, TopologyError};
pub use crate::environment::{Environment, EnvironmentError, EnvironmentParameter};
pub use crate::model::{EngineeringModel, ModelDefinition, ModelError, ModelReference};
pub use crate::validation::{RealityValidator, ValidationError, ValidationResult};
