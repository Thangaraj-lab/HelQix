//! Public Rust SDK types for defining and exchanging HELQIX realities.
//!
//! This crate provides transport-safe models and validation. It does not
//! execute actions or behaviours; runtime work belongs to the platform API.

pub mod action;
pub mod behaviour;
pub mod component;
pub mod environment;
pub mod reality;

pub use action::{ActionFailure, ActionInvocation, ActionOutcome, ActionRef};
pub use behaviour::{BehaviourDefinition, BehaviourRef};
pub use component::{Component, ComponentKey, ComponentRef};
pub use environment::{Environment, EnvironmentError};
pub use reality::{
    RealityCodecError, RealityDocument, RealityId, RealityMetadata, RealityValidationError,
    RealityValidationIssue, REALITY_SCHEMA_VERSION,
};
