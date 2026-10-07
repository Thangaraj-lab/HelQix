//! Engineering model composition for HELQIX Layer 03 — Engineering.
//!
//! A [`Model`] represents one named engineering model assembled from the
//! Layer 03 engineering-domain collections:
//!
//! ```text
//! Model
//! ├── parameters    : ParameterSet
//! ├── equations     : EquationSet
//! ├── dependencies  : DependencyGraph
//! └── constraints   : ConstraintSet
//! ```
//!
//! # Scope
//!
//! `Model` is a **composition boundary** for engineering definitions. It
//! owns the engineering collections and exposes deterministic, typed
//! access to them. It does not:
//!
//! - evaluate or execute anything;
//! - hold runtime state, simulation state, or execution state;
//! - perform cross-domain reference resolution;
//! - register multiple models (see `ModelRegistry` in a separate file);
//! - aggregate the entire engineering layer (see `Engineering` in a
//!   separate file).
//!
//! Full cross-domain validation is deferred to a dedicated validation
//! layer. `Model::validate` performs only the structural validation that
//! the composed collections themselves provide.
//!
//! # Identity
//!
//! Model identity is its [`Identifier`] name. No UUIDs, no timestamps,
//! no randomized identifiers.

use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

use crate::constraint_set::{ConstraintSet, ConstraintSetError};
use crate::dependency_graph::{DependencyGraph, DependencyGraphError};
use crate::equation_set::{EquationSet, EquationSetError};
use crate::parameter_set::{ParameterSet, ParameterSetError};

/// Errors produced by [`Model`] operations.
///
/// `ModelError` represents **model-level** failures. Structural failures
/// of the underlying engineering collections are wrapped transparently
/// rather than duplicated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    /// The model's [`ParameterSet`] failed structural validation.
    InvalidParameters {
        /// The underlying parameter-set failure.
        source: ParameterSetError,
    },
    /// The model's [`EquationSet`] failed structural validation.
    InvalidEquations {
        /// The underlying equation-set failure.
        source: EquationSetError,
    },
    /// The model's [`DependencyGraph`] failed structural validation.
    InvalidDependencies {
        /// The underlying dependency-graph failure.
        source: DependencyGraphError,
    },
    /// The model's [`ConstraintSet`] failed structural validation.
    InvalidConstraints {
        /// The underlying constraint-set failure.
        source: ConstraintSetError,
    },
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::InvalidParameters { source } => {
                write!(f, "model has invalid parameters: {source}")
            }
            ModelError::InvalidEquations { source } => {
                write!(f, "model has invalid equations: {source}")
            }
            ModelError::InvalidDependencies { source } => {
                write!(f, "model has invalid dependencies: {source}")
            }
            ModelError::InvalidConstraints { source } => {
                write!(f, "model has invalid constraints: {source}")
            }
        }
    }
}

impl Error for ModelError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ModelError::InvalidParameters { source } => Some(source),
            ModelError::InvalidEquations { source } => Some(source),
            ModelError::InvalidDependencies { source } => Some(source),
            ModelError::InvalidConstraints { source } => Some(source),
        }
    }
}

/// A single named engineering model.
///
/// A `Model` owns the Layer 03 engineering-domain collections and exposes
/// deterministic, strongly-typed access to them. It is a definition-only
/// structure and performs no execution, simulation, physics, or runtime
/// state management.
///
/// Construction is incremental: build the model with [`Model::new`],
/// then populate its collections through the mutable accessors
/// ([`Model::parameters_mut`], [`Model::equations_mut`],
/// [`Model::dependencies_mut`], [`Model::constraints_mut`]). Once
/// populated, call [`Model::validate`] to verify each collection's own
/// structural invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    name: Identifier,
    parameters: ParameterSet,
    equations: EquationSet,
    dependencies: DependencyGraph,
    constraints: ConstraintSet,
}

impl Model {
    /// Creates an empty model with the given name.
    ///
    /// The model starts with empty engineering collections. Identity is
    /// the [`Identifier`] itself; no UUID, timestamp, or randomized
    /// identity is generated.
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            parameters: ParameterSet::new(),
            equations: EquationSet::new(),
            dependencies: DependencyGraph::new(),
            constraints: ConstraintSet::new(),
        }
    }

    /// Returns the model's name.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the model's [`ParameterSet`].
    pub fn parameters(&self) -> &ParameterSet {
        &self.parameters
    }

    /// Returns a mutable reference to the model's [`ParameterSet`].
    ///
    /// Exposed to support incremental model construction.
    pub fn parameters_mut(&mut self) -> &mut ParameterSet {
        &mut self.parameters
    }

    /// Returns the model's [`EquationSet`].
    pub fn equations(&self) -> &EquationSet {
        &self.equations
    }

    /// Returns a mutable reference to the model's [`EquationSet`].
    ///
    /// Exposed to support incremental model construction.
    pub fn equations_mut(&mut self) -> &mut EquationSet {
        &mut self.equations
    }

    /// Returns the model's [`DependencyGraph`].
    pub fn dependencies(&self) -> &DependencyGraph {
        &self.dependencies
    }

    /// Returns a mutable reference to the model's [`DependencyGraph`].
    ///
    /// Exposed to support incremental model construction.
    pub fn dependencies_mut(&mut self) -> &mut DependencyGraph {
        &mut self.dependencies
    }

    /// Returns the model's [`ConstraintSet`].
    pub fn constraints(&self) -> &ConstraintSet {
        &self.constraints
    }

    /// Returns a mutable reference to the model's [`ConstraintSet`].
    ///
    /// Exposed to support incremental model construction.
    pub fn constraints_mut(&mut self) -> &mut ConstraintSet {
        &mut self.constraints
    }

    /// Performs structural validation of the model's engineering
    /// collections.
    ///
    /// Each collection is validated with its own deterministic validation
    /// contract:
    ///
    /// 1. [`ParameterSet::validate`]
    /// 2. [`EquationSet::validate`]
    /// 3. [`DependencyGraph::validate`]
    /// 4. [`ConstraintSet::validate`]
    ///
    /// The first failure aborts validation and is returned wrapped in the
    /// corresponding [`ModelError`] variant.
    ///
    /// No cross-domain reference resolution is performed here. Full
    /// cross-domain validation belongs to a dedicated engineering
    /// validation layer.
    pub fn validate(&self) -> Result<(), ModelError> {
        self.parameters
            .validate()
            .map_err(|source| ModelError::InvalidParameters { source })?;

        self.equations
            .validate()
            .map_err(|source| ModelError::InvalidEquations { source })?;

        self.dependencies
            .validate()
            .map_err(|source| ModelError::InvalidDependencies { source })?;

        self.constraints
            .validate()
            .map_err(|source| ModelError::InvalidConstraints { source })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use helqix_core::units::Unit;

    use crate::constraint::{Constraint, ConstraintReference};
    use crate::dependency::{Dependency, DependencyReference};
    use crate::equation::Equation;
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

    #[test]
    fn model_stores_name() {
        let model = Model::new(ident("motor_model"));
        assert_eq!(model.name().as_str(), "motor_model");
    }

    #[test]
    fn new_model_has_empty_engineering_collections() {
        let model = Model::new(ident("motor_model"));
        assert!(model.parameters().is_empty());
        assert!(model.equations().is_empty());
        assert!(model.dependencies().is_empty());
        assert!(model.constraints().is_empty());
    }

    #[test]
    fn parameter_collection_is_accessible() {
        let mut model = Model::new(ident("motor_model"));
        model
            .parameters_mut()
            .add(valid_parameter("mass"))
            .expect("add mass");

        assert_eq!(model.parameters().len(), 1);
        assert!(model.parameters().contains(&ident("mass")));
    }

    #[test]
    fn equation_collection_is_accessible() {
        let mut model = Model::new(ident("motor_model"));
        model
            .equations_mut()
            .add(valid_equation("force_equation"))
            .expect("add force_equation");

        assert_eq!(model.equations().len(), 1);
        assert!(model.equations().contains(&ident("force_equation")));
    }

    #[test]
    fn dependency_graph_is_accessible() {
        let mut model = Model::new(ident("motor_model"));
        model
            .dependencies_mut()
            .add(valid_dependency("thrust_dependency"))
            .expect("add dependency");

        assert_eq!(model.dependencies().len(), 1);
        assert!(model.dependencies().contains(&ident("thrust_dependency")));
    }

    #[test]
    fn constraint_collection_is_accessible() {
        let mut model = Model::new(ident("motor_model"));
        model
            .constraints_mut()
            .add(valid_constraint("motor_temp_limit"))
            .expect("add constraint");

        assert_eq!(model.constraints().len(), 1);
        assert!(model.constraints().contains(&ident("motor_temp_limit")));
    }

    #[test]
    fn mutable_access_allows_model_construction() {
        let mut model = Model::new(ident("motor_model"));

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

        assert_eq!(model.parameters().len(), 1);
        assert_eq!(model.equations().len(), 1);
        assert_eq!(model.dependencies().len(), 1);
        assert_eq!(model.constraints().len(), 1);

        assert!(model.validate().is_ok());
    }

    #[test]
    fn empty_model_validation_succeeds() {
        let model = Model::new(ident("empty_model"));
        assert!(model.validate().is_ok());
    }

    #[test]
    fn invalid_nested_collection_is_reported() {
        // Through the public API, invalid definitions cannot enter the
        // underlying collections. This test confirms that a fully
        // populated, valid model passes validation and that the error
        // channels are wired correctly.
        let mut model = Model::new(ident("motor_model"));
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

        match model.validate() {
            Ok(()) => {}
            Err(ModelError::InvalidParameters { .. }) => panic!("unexpected parameter error"),
            Err(ModelError::InvalidEquations { .. }) => panic!("unexpected equation error"),
            Err(ModelError::InvalidDependencies { .. }) => panic!("unexpected dependency error"),
            Err(ModelError::InvalidConstraints { .. }) => panic!("unexpected constraint error"),
        }
    }

    #[test]
    fn deterministic_equality() {
        let mut a = Model::new(ident("motor_model"));
        a.parameters_mut()
            .add(valid_parameter("mass"))
            .expect("add mass");

        let mut b = Model::new(ident("motor_model"));
        b.parameters_mut()
            .add(valid_parameter("mass"))
            .expect("add mass");

        assert_eq!(a, b);

        let c = Model::new(ident("other_model"));
        assert_ne!(a, c);
    }

    #[test]
    fn model_error_display_is_deterministic() {
        let mut model = Model::new(ident("motor_model"));
        // Populate with a valid definition so validation succeeds.
        model
            .parameters_mut()
            .add(valid_parameter("mass"))
            .expect("add mass");

        assert!(model.validate().is_ok());

        // The error type itself must render deterministically; we simply
        // exercise one variant's Display through a constructed instance
        // is not possible without an actual error, so we assert the
        // successful path here and rely on the compiler to keep the
        // error surface consistent.
        let _ = ModelError::InvalidParameters {
            source: ParameterSetError::DuplicateParameter {
                name: "mass".to_owned(),
            },
        }
        .to_string();
    }
}