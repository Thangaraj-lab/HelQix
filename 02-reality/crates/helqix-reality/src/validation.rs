//! Structural validation boundary of Layer 02 — Reality.
//!
//! `RealityValidator` answers one question: "Is this Reality structurally
//! valid before later execution layers consume it?"
//!
//! It is stateless, deterministic, and read-only. It delegates to the public
//! `validate` APIs of each module and then checks cross-references between
//! them. It never mutates, repairs, simulates, or evaluates anything.

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
    Reality { detail: String },
    RealityGraph { detail: String },
    ComponentRegistry { detail: String },
    Topology { detail: String },
    Environment { detail: String },
    EngineeringModel { detail: String },
    CrossReference { detail: String },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::Reality { detail } => write!(f, "reality validation failed: {detail}"),
            ValidationError::RealityGraph { detail } => write!(f, "reality graph validation failed: {detail}"),
            ValidationError::ComponentRegistry { detail } => write!(f, "component registry validation failed: {detail}"),
            ValidationError::Topology { detail } => write!(f, "topology validation failed: {detail}"),
            ValidationError::Environment { detail } => write!(f, "environment validation failed: {detail}"),
            ValidationError::EngineeringModel { detail } => write!(f, "engineering model validation failed: {detail}"),
            ValidationError::CrossReference { detail } => write!(f, "cross-reference validation failed: {detail}"),
        }
    }
}

impl Error for ValidationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealityValidator;

impl RealityValidator {
    pub fn new() -> Self { RealityValidator }

    pub fn validate_reality(&self, reality: &Reality) -> ValidationResult<()> {
        reality.validate().map_err(|error| ValidationError::Reality { detail: error.to_string() })
    }

    pub fn validate_graph(&self, graph: &RealityGraph) -> ValidationResult<()> {
        graph.validate().map_err(|error| ValidationError::RealityGraph { detail: error.to_string() })
    }

    pub fn validate_components(&self, registry: &ComponentRegistry) -> ValidationResult<()> {
        registry.validate().map_err(|error| ValidationError::ComponentRegistry { detail: error.to_string() })
    }

    pub fn validate_topology(&self, topology: &Topology, registry: &ComponentRegistry) -> ValidationResult<()> {
        topology.validate(registry).map_err(|error| ValidationError::Topology { detail: error.to_string() })
    }

    pub fn validate_environment(&self, environment: &Environment) -> ValidationResult<()> {
        environment.validate().map_err(|error| ValidationError::Environment { detail: error.to_string() })
    }

    pub fn validate_model(&self, model: &EngineeringModel) -> ValidationResult<()> {
        model.validate().map_err(|error| ValidationError::EngineeringModel { detail: error.to_string() })
    }

    pub fn validate(
        &self,
        reality: &Reality,
        graph: &RealityGraph,
        registry: &ComponentRegistry,
        topology: &Topology,
        environment: &Environment,
        model: &EngineeringModel,
    ) -> ValidationResult<()> {
        self.validate_reality(reality)?;
        self.validate_components(registry)?;
        self.validate_graph(graph)?;
        self.validate_topology(topology, registry)?;
        self.validate_environment(environment)?;
        self.validate_model(model)?;
        self.validate_cross_references(reality, graph, registry)
    }

    fn validate_cross_references(&self, reality: &Reality, graph: &RealityGraph, registry: &ComponentRegistry) -> ValidationResult<()> {
        for component in reality.components() {
            let name = component.name();
            if !registry.contains(name) {
                return Err(ValidationError::CrossReference { detail: format!("component '{}' exists in Reality but is missing from ComponentRegistry", name.as_str()) });
            }
        }
        for name in registry.keys() {
            if !reality.contains_component(name) {
                return Err(ValidationError::CrossReference { detail: format!("component '{}' exists in ComponentRegistry but is missing from Reality", name.as_str()) });
            }
        }
        for node in graph.nodes() {
            if !registry.contains(node) {
                return Err(ValidationError::CrossReference { detail: format!("graph node '{}' is missing from ComponentRegistry", node.as_str()) });
            }
        }
        for node in graph.nodes() {
            if !reality.contains_component(node) {
                return Err(ValidationError::CrossReference { detail: format!("graph node '{}' is missing from Reality", node.as_str()) });
            }
        }
        Ok(())
    }
}

impl Default for RealityValidator {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component_registry::Component;
    use crate::topology::ConnectionKind;
    use helqix_core::id::Identifier;

    fn id(name: &str) -> Identifier {
        Identifier::parse(name).expect("valid identifier")
    }

    fn component(name: &str) -> Component {
        Component::new(id(name)).expect("valid component")
    }

    fn empty_reality() -> Reality {
        Reality::new(id("system"))
    }

    fn assert_cross_reference(result: ValidationResult<()>, expected: &[&str]) {
        match result {
            Err(ValidationError::CrossReference { detail }) => {
                for fragment in expected {
                    assert!(detail.contains(fragment), "detail '{}' should contain '{}'", detail, fragment);
                }
            }
            other => panic!("expected CrossReference error, got {:?}", other),
        }
    }

    struct Parts {
        reality: Reality,
        graph: RealityGraph,
        registry: ComponentRegistry,
        topology: Topology,
        environment: Environment,
        model: EngineeringModel,
    }

    impl Parts {
        fn empty() -> Self {
            Self {
                reality: empty_reality(),
                graph: RealityGraph::new(),
                registry: ComponentRegistry::new(),
                topology: Topology::new(),
                environment: Environment::new(),
                model: EngineeringModel::new(),
            }
        }

        fn run(&self) -> ValidationResult<()> {
            RealityValidator::new().validate(&self.reality, &self.graph, &self.registry, &self.topology, &self.environment, &self.model)
        }

        fn add_registered_component(&mut self, name: &str) {
            self.reality.add_component(component(name)).expect("add component to reality");
            self.registry.register(component(name)).expect("register component");
        }
    }

    #[test]
    fn validator_can_be_created() {
        assert_eq!(RealityValidator::new(), RealityValidator::default());
    }

    #[test]
    fn empty_structures_validate_individually() {
        let validator = RealityValidator::new();
        let parts = Parts::empty();
        assert!(validator.validate_reality(&parts.reality).is_ok());
        assert!(validator.validate_graph(&parts.graph).is_ok());
        assert!(validator.validate_components(&parts.registry).is_ok());
        assert!(validator.validate_topology(&parts.topology, &parts.registry).is_ok());
        assert!(validator.validate_environment(&parts.environment).is_ok());
        assert!(validator.validate_model(&parts.model).is_ok());
    }

    #[test]
    fn empty_reality_validates() { assert!(RealityValidator::new().validate_reality(&empty_reality()).is_ok()); }
    #[test]
    fn empty_graph_validates() { assert!(RealityValidator::new().validate_graph(&RealityGraph::new()).is_ok()); }
    #[test]
    fn empty_registry_validates() { assert!(RealityValidator::new().validate_components(&ComponentRegistry::new()).is_ok()); }
    #[test]
    fn empty_topology_validates() { assert!(RealityValidator::new().validate_topology(&Topology::new(), &ComponentRegistry::new()).is_ok()); }
    #[test]
    fn empty_environment_validates() { assert!(RealityValidator::new().validate_environment(&Environment::new()).is_ok()); }
    #[test]
    fn empty_model_validates() { assert!(RealityValidator::new().validate_model(&EngineeringModel::new()).is_ok()); }
    #[test]
    fn complete_empty_system_validates() { assert!(Parts::empty().run().is_ok()); }

    #[test]
    fn reality_component_missing_from_registry_is_rejected() {
        let mut parts = Parts::empty();
        parts.reality.add_component(component("battery")).expect("add component to reality");
        assert_cross_reference(parts.run(), &["Reality", "battery"]);
    }

    #[test]
    fn registry_component_missing_from_reality_is_rejected() {
        let mut parts = Parts::empty();
        parts.registry.register(component("battery")).expect("register component");
        assert_cross_reference(parts.run(), &["ComponentRegistry", "battery"]);
    }

    #[test]
    fn graph_node_missing_from_registry_is_rejected() {
        let mut parts = Parts::empty();
        parts.graph.add_node(id("battery")).expect("add graph node");
        assert_cross_reference(parts.run(), &["graph node", "battery", "ComponentRegistry"]);
    }

    #[test]
    fn graph_node_missing_from_reality_is_rejected() {
        let mut parts = Parts::empty();
        parts.registry.register(component("battery")).expect("register component");
        parts.graph.add_node(id("battery")).expect("add graph node");
        assert_cross_reference(parts.run(), &["battery"]);
        let result = RealityValidator::new().validate_cross_references(&parts.reality, &parts.graph, &parts.registry);
        assert!(matches!(result, Err(ValidationError::CrossReference { .. })));
    }

    #[test]
    fn isolated_reality_component_does_not_require_graph_node() {
        let mut parts = Parts::empty();
        parts.add_registered_component("battery");
        assert!(parts.run().is_ok());
    }

    #[test]
    fn topology_with_registered_components_validates() {
        let validator = RealityValidator::new();
        let mut registry = ComponentRegistry::new();
        registry.register(component("battery")).expect("register battery");
        registry.register(component("controller")).expect("register controller");
        let mut topology = Topology::new();
        topology.connect(id("battery"), id("controller"), ConnectionKind::Structural).expect("connect components");
        assert!(validator.validate_topology(&topology, &registry).is_ok());
    }

    #[test]
    fn topology_missing_component_is_rejected() {
        let validator = RealityValidator::new();
        let registry = ComponentRegistry::new();
        let mut topology = Topology::new();
        topology.connect(id("battery"), id("controller"), ConnectionKind::Structural).expect("connect components");
        let result = validator.validate_topology(&topology, &registry);
        assert!(matches!(result, Err(ValidationError::Topology { .. })));
    }

    #[test]
    fn environment_can_be_empty() {
        let mut parts = Parts::empty();
        parts.add_registered_component("battery");
        assert!(parts.run().is_ok());
    }

    #[test]
    fn model_can_be_empty() {
        let mut parts = Parts::empty();
        parts.add_registered_component("battery");
        assert!(parts.run().is_ok());
    }

    #[test]
    fn deterministic_cross_reference_failure() {
        let mut parts = Parts::empty();
        parts.graph.add_node(id("zeta")).expect("add zeta");
        parts.graph.add_node(id("alpha")).expect("add alpha");
        parts.graph.add_node(id("mid")).expect("add mid");
        let first = parts.run();
        let second = parts.run();
        assert_eq!(first, second);
        assert_cross_reference(first, &["alpha", "ComponentRegistry"]);
    }

    #[test]
    fn valid_structural_system_validates() {
        let mut parts = Parts::empty();
        for name in ["battery", "motor", "controller"] {
            parts.add_registered_component(name);
            parts.graph.add_node(id(name)).expect("add graph node");
        }
        parts.topology.connect(id("battery"), id("controller"), ConnectionKind::Structural).expect("connect battery to controller");
        assert!(parts.run().is_ok());
    }
}