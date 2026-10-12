//! Structural validation boundary for Layer 02 — Reality.

use std::error::Error;
use std::fmt;
use crate::component_registry::ComponentRegistry;
use crate::environment::Environment;
use crate::model::EngineeringModel;
use crate::reality::Reality;
use crate::reality_graph::RealityGraph;
use crate::topology::Topology;

pub type ValidationResult<T> = Result<T, ValidationError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    Reality { detail: String }, RealityGraph { detail: String }, ComponentRegistry { detail: String },
    Topology { detail: String }, Environment { detail: String }, EngineeringModel { detail: String }, CrossReference { detail: String },
}
impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self {
        Self::Reality { detail } => write!(f, "reality validation failed: {detail}"),
        Self::RealityGraph { detail } => write!(f, "reality graph validation failed: {detail}"),
        Self::ComponentRegistry { detail } => write!(f, "component registry validation failed: {detail}"),
        Self::Topology { detail } => write!(f, "topology validation failed: {detail}"),
        Self::Environment { detail } => write!(f, "environment validation failed: {detail}"),
        Self::EngineeringModel { detail } => write!(f, "engineering model validation failed: {detail}"),
        Self::CrossReference { detail } => write!(f, "cross-reference validation failed: {detail}"),
    }} }
impl Error for ValidationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RealityValidator;
impl RealityValidator {
    pub fn new() -> Self { Self }
    pub fn validate_reality(&self, reality: &Reality) -> ValidationResult<()> { reality.validate().map_err(|e| ValidationError::Reality { detail: e.to_string() }) }
    pub fn validate_graph(&self, graph: &RealityGraph) -> ValidationResult<()> { graph.validate().map_err(|e| ValidationError::RealityGraph { detail: e.to_string() }) }
    pub fn validate_components(&self, registry: &ComponentRegistry) -> ValidationResult<()> { registry.validate().map_err(|e| ValidationError::ComponentRegistry { detail: e.to_string() }) }
    pub fn validate_topology(&self, topology: &Topology, registry: &ComponentRegistry) -> ValidationResult<()> { topology.validate(registry).map_err(|e| ValidationError::Topology { detail: e.to_string() }) }
    pub fn validate_environment(&self, environment: &Environment) -> ValidationResult<()> { environment.validate().map_err(|e| ValidationError::Environment { detail: e.to_string() }) }
    pub fn validate_model(&self, model: &EngineeringModel) -> ValidationResult<()> { model.validate().map_err(|e| ValidationError::EngineeringModel { detail: e.to_string() }) }
    pub fn validate(&self, reality: &Reality, graph: &RealityGraph, registry: &ComponentRegistry, topology: &Topology, environment: &Environment, model: &EngineeringModel) -> ValidationResult<()> {
        self.validate_reality(reality)?; self.validate_components(registry)?; self.validate_graph(graph)?; self.validate_topology(topology, registry)?; self.validate_environment(environment)?; self.validate_model(model)?; self.validate_cross_references(reality, graph, registry)
    }
    fn validate_cross_references(&self, reality: &Reality, graph: &RealityGraph, registry: &ComponentRegistry) -> ValidationResult<()> {
        for component in reality.components() { let name = component.name(); if !registry.contains(name) { return Err(ValidationError::CrossReference { detail: format!("component '{}' exists in Reality but is missing from ComponentRegistry", name.as_str()) }); } }
        for name in registry.keys() { if !reality.contains_component(name) { return Err(ValidationError::CrossReference { detail: format!("component '{}' exists in ComponentRegistry but is missing from Reality", name.as_str()) }); } }
        for node in graph.nodes() { if !registry.contains(node) { return Err(ValidationError::CrossReference { detail: format!("graph node '{}' is missing from ComponentRegistry", node.as_str()) }); } if !reality.contains_component(node) { return Err(ValidationError::CrossReference { detail: format!("graph node '{}' is missing from Reality", node.as_str()) }); } }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helqix_core::component::Component;
    use helqix_core::id::Identifier;
    fn id(name: &str) -> Identifier { Identifier::parse(name).expect("valid identifier") }
    fn component(name: &str) -> Component { Component::new(id(name)).expect("valid component") }
    #[test] fn empty_system_validates() { let r=Reality::new(id("system")); let g=RealityGraph::new(); let c=ComponentRegistry::new(); let t=Topology::new(); assert!(RealityValidator::new().validate(&r,&g,&c,&t,&Environment::new(),&EngineeringModel::new()).is_ok()); }
    #[test] fn public_component_import_is_valid() { let mut r=Reality::new(id("system")); let mut c=ComponentRegistry::new(); r.add_component(component("battery")).expect("add"); c.register(component("battery")).expect("register"); assert!(RealityValidator::new().validate_cross_references(&r,&RealityGraph::new(),&c).is_ok()); }
    #[test] fn mismatched_component_sets_are_rejected() { let mut r=Reality::new(id("system")); r.add_component(component("battery")).expect("add"); assert!(matches!(RealityValidator::new().validate_cross_references(&r,&RealityGraph::new(),&ComponentRegistry::new()),Err(ValidationError::CrossReference{..}))); }
}
