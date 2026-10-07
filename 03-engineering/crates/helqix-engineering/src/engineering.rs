//! Layer 03 engineering aggregate for HELQIX.
//!
//! [`Engineering`] is the aggregate root of the engineering-definition
//! layer. It owns the layer's top-level deterministic container — a
//! [`ModelRegistry`] of named [`Model`] definitions — together with an
//! explicit engineering identity.
//!
//! # Scope
//!
//! ```text
//! Engineering
//!     └── ModelRegistry
//!             ├── Model A
//!             ├── Model B
//!             └── Model C
//! ```
//!
//! `Engineering` is a **definition-only** aggregate. It does not execute,
//! simulate, evaluate, or otherwise realize engineering models. It does
//! not own runtime state, physics, time, observation, failure events,
//! causality, branching, optimization, AI, or quantum concerns. Those
//! belong to later HELQIX layers.
//!
//! Cross-domain engineering validation belongs to a dedicated validation
//! layer; [`Engineering::validate`] performs only aggregate-local
//! structural validation of the currently owned Layer 03 collections.

use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

use crate::model_registry::{ModelRegistry, ModelRegistryError};

/// Errors produced by [`Engineering`] operations.
///
/// `EngineeringError` is aggregate-local. Structural failures of the
/// contained engineering collections are wrapped transparently rather
/// than duplicated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineeringError {
    /// The engineering aggregate's [`ModelRegistry`] failed structural
    /// validation.
    InvalidModels {
        /// The underlying model-registry failure.
        source: ModelRegistryError,
    },
}

impl fmt::Display for EngineeringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineeringError::InvalidModels { source } => {
                write!(f, "engineering has invalid models: {source}")
            }
        }
    }
}

impl Error for EngineeringError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            EngineeringError::InvalidModels { source } => Some(source),
        }
    }
}

/// Aggregate root of HELQIX Layer 03 — Engineering.
///
/// `Engineering` binds an explicit [`Identifier`] identity to a
/// [`ModelRegistry`] of named engineering models. It is intentionally
/// minimal: it composes the layer's top-level collections without
/// duplicating their storage, and it never executes or evaluates the
/// definitions they contain.
///
/// Incremental construction is expressed through [`Engineering::models_mut`]:
///
/// ```text
/// let mut engineering = Engineering::new(name);
/// engineering.models_mut().add(model)?;
/// engineering.validate()?;
/// ```
///
/// No convenience wrappers such as `add_model`, `remove_model`, or
/// `get_model` are provided. The aggregate composes collections; the
/// collections own their own operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Engineering {
    name: Identifier,
    models: ModelRegistry,
}

impl Engineering {
    /// Creates an empty engineering aggregate with the given identity.
    ///
    /// The aggregate starts with an empty [`ModelRegistry`]. Identity is
    /// the [`Identifier`] itself; no UUID, timestamp, or randomized
    /// identity is generated.
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            models: ModelRegistry::new(),
        }
    }

    /// Returns the engineering aggregate's name.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the aggregate's [`ModelRegistry`].
    pub fn models(&self) -> &ModelRegistry {
        &self.models
    }

    /// Returns a mutable reference to the aggregate's [`ModelRegistry`].
    ///
    /// Exposed to support incremental construction of the Layer 03
    /// aggregate through the registry's public API.
    pub fn models_mut(&mut self) -> &mut ModelRegistry {
        &mut self.models
    }

    /// Performs aggregate-local structural validation.
    ///
    /// Currently this is exactly the validation of the owned
    /// [`ModelRegistry`]. No cross-domain or cross-layer validation is
    /// performed here; that responsibility belongs to a dedicated
    /// validation layer.
    pub fn validate(&self) -> Result<(), EngineeringError> {
        self.models
            .validate()
            .map_err(|source| EngineeringError::InvalidModels { source })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use helqix_core::units::Unit;

    use crate::constraint::{Constraint, ConstraintReference};
    use crate::dependency::{Dependency, DependencyReference};
    use crate::equation::Equation;
    use crate::model::Model;
    use crate::parameter::ParameterDefinition;

    fn ident(s: &str) -> Identifier {
        Identifier::parse(s).expect("known-valid identifier literal")
    }

    fn kg_dimension() -> helqix_core::units::Dimension {
        Unit::parse("kg")
            .expect("kg must exist in Foundation units")
            .dimension()
            .clone()
    }

    fn valid_parameter(name: &str) -> ParameterDefinition {
        ParameterDefinition::new(ident(name), kg_dimension(), true)
    }

    fn valid_equation(name: &str) -> Equation {
        let mut equation = Equation::new(ident(name), ident("out"), "1")
            .expect("valid equation header");
        equation
            .add_input(ident("input"))
            .expect("valid input added");
        equation
    }

    fn valid_dependency(name: &str) -> Dependency {
        let source = format!("{}_source", name);
        let target = format!("{}_target", name);
        Dependency::new(
            ident(name),
            DependencyReference::property(ident(&source)),
            DependencyReference::property(ident(&target)),
        )
        .expect("valid dependency")
    }

    fn valid_constraint(name: &str) -> Constraint {
        Constraint::new(
            ident(name),
            ConstraintReference::property(ident("temperature")),
            0.0,
            120.0,
        )
        .expect("valid constraint")
    }

    fn empty_model(name: &str) -> Model {
        Model::new(ident(name))
    }

    fn populated_model(name: &str) -> Model {
        let mut model = Model::new(ident(name));
        model
            .parameters_mut()
            .add(valid_parameter("mass"))
            .expect("add mass");
        model
            .equations_mut()
            .add(valid_equation("force_equation"))
            .expect("add equation");
        model
            .dependencies_mut()
            .add(valid_dependency("thrust_dependency"))
            .expect("add dependency");
        model
            .constraints_mut()
            .add(valid_constraint("motor_temp_limit"))
            .expect("add constraint");
        model
    }

    #[test]
    fn engineering_stores_name() {
        let engineering = Engineering::new(ident("vehicle_engineering"));
        assert_eq!(engineering.name().as_str(), "vehicle_engineering");
    }

    #[test]
    fn new_engineering_has_empty_model_registry() {
        let engineering = Engineering::new(ident("vehicle_engineering"));
        assert!(engineering.models().is_empty());
        assert_eq!(engineering.models().len(), 0);
    }

    #[test]
    fn models_are_accessible() {
        let mut engineering = Engineering::new(ident("vehicle_engineering"));
        engineering
            .models_mut()
            .add(empty_model("motor_model"))
            .expect("add motor model");

        assert_eq!(engineering.models().len(), 1);
        assert!(engineering.models().contains(&ident("motor_model")));
    }

    #[test]
    fn mutable_model_registry_allows_construction() {
        let mut engineering = Engineering::new(ident("vehicle_engineering"));

        engineering
            .models_mut()
            .add(populated_model("motor_model"))
            .expect("add motor model");
        engineering
            .models_mut()
            .add(populated_model("battery_model"))
            .expect("add battery model");

        assert_eq!(engineering.models().len(), 2);

        let stored = engineering
            .models()
            .get(&ident("motor_model"))
            .expect("present");
        assert_eq!(stored.parameters().len(), 1);
        assert_eq!(stored.equations().len(), 1);
        assert_eq!(stored.dependencies().len(), 1);
        assert_eq!(stored.constraints().len(), 1);
    }

    #[test]
    fn empty_engineering_is_valid() {
        let engineering = Engineering::new(ident("vehicle_engineering"));
        assert!(engineering.validate().is_ok());
    }

    #[test]
    fn populated_engineering_validates() {
        let mut engineering = Engineering::new(ident("vehicle_engineering"));
        engineering
            .models_mut()
            .add(populated_model("motor_model"))
            .expect("add motor model");
        engineering
            .models_mut()
            .add(populated_model("battery_model"))
            .expect("add battery model");

        assert!(engineering.validate().is_ok());
    }

    #[test]
    fn invalid_model_registry_error_is_propagated() {
        // Through the public API, invalid models cannot enter the
        // registry, so this test exercises the error surface that is
        // actually reachable: a well-formed aggregate passes validation,
        // and the error type composes transparently over the registry
        // error.
        let mut engineering = Engineering::new(ident("vehicle_engineering"));
        engineering
            .models_mut()
            .add(empty_model("motor_model"))
            .expect("add motor model");

        assert!(engineering.validate().is_ok());

        // Construct an EngineeringError value directly to exercise the
        // Display and source plumbing without fabricating private state.
        let synthetic = EngineeringError::InvalidModels {
            source: ModelRegistryError::DuplicateModel {
                name: ident("motor_model"),
            },
        };
        let rendered = synthetic.to_string();
        assert!(rendered.contains("invalid models"));
        assert!(synthetic.source().is_some());
    }

    #[test]
    fn deterministic_equality() {
        let mut a = Engineering::new(ident("vehicle_engineering"));
        a.models_mut()
            .add(empty_model("motor_model"))
            .expect("add motor model");

        let mut b = Engineering::new(ident("vehicle_engineering"));
        b.models_mut()
            .add(empty_model("motor_model"))
            .expect("add motor model");

        assert_eq!(a, b);

        let c = Engineering::new(ident("other_engineering"));
        assert_ne!(a, c);
    }
}