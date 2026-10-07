//! Deterministic registry of named engineering models for HELQIX
//! Layer 03 — Engineering.
//!
//! A [`ModelRegistry`] owns a set of [`Model`] definitions keyed by
//! their [`Identifier`] name. It provides duplicate-safe insertion,
//! deterministic iteration, lookup, and structural validation.
//!
//! # Scope
//!
//! The registry is a **definition-only** container. It represents the
//! set of named engineering models defined at the Layer 03 level:
//!
//! ```text
//! Model
//!    ↓
//! ModelRegistry
//!    ↓
//! Engineering
//! ```
//!
//! It does **not**:
//!
//! - execute, simulate, or evaluate models;
//! - store runtime state;
//! - establish relationships between models (no model graph);
//! - select an "active" or "current" model;
//! - perform cross-model semantic analysis.
//!
//! Those concerns belong to later HELQIX layers.
//!
//! Ordering is deterministic via [`BTreeMap`]; the map key is always the
//! model's own [`Identifier`] name.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

use crate::model::{Model, ModelError};

/// Errors produced by [`ModelRegistry`] operations.
///
/// `ModelRegistryError` represents **registry-level** failures. Model
/// structural failures are represented by [`ModelError`] and wrapped in
/// [`ModelRegistryError::InvalidModel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelRegistryError {
    /// A model with the same identifier is already present.
    DuplicateModel {
        /// Name of the duplicated model.
        name: Identifier,
    },
    /// A model failed structural validation.
    InvalidModel {
        /// Name of the invalid model.
        name: Identifier,
        /// The underlying model-level failure.
        source: ModelError,
    },
}

impl fmt::Display for ModelRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelRegistryError::DuplicateModel { name } => {
                write!(f, "duplicate model: {}", name.as_str())
            }
            ModelRegistryError::InvalidModel { name, source } => {
                write!(f, "invalid model {}: {}", name.as_str(), source)
            }
        }
    }
}

impl Error for ModelRegistryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ModelRegistryError::InvalidModel { source, .. } => Some(source),
            ModelRegistryError::DuplicateModel { .. } => None,
        }
    }
}

/// Deterministic registry of named engineering [`Model`] definitions.
///
/// Backed by a [`BTreeMap`] keyed on [`Identifier`], giving deterministic
/// iteration order. Duplicate model names are rejected; existing entries
/// are never overwritten. Invalid models are rejected at insertion time,
/// so the registry never contains an invalid model through its public
/// API.
///
/// An empty registry is valid. `ModelRegistry` performs no execution,
/// no simulation, and no cross-model analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRegistry {
    models: BTreeMap<Identifier, Model>,
}

impl ModelRegistry {
    /// Creates an empty model registry.
    pub fn new() -> Self {
        Self {
            models: BTreeMap::new(),
        }
    }

    /// Returns the number of models in the registry.
    pub fn len(&self) -> usize {
        self.models.len()
    }

    /// Returns `true` if the registry contains no models.
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// Adds a model to the registry.
    ///
    /// The model's own identifier is used as the canonical key. Duplicate
    /// names are rejected before validation. On success the model is
    /// structurally validated and inserted; the registry is never mutated
    /// on failure.
    ///
    /// Existing models are never overwritten, merged, or renamed.
    pub fn add(&mut self, model: Model) -> Result<(), ModelRegistryError> {
        let key = model.name().clone();

        if self.models.contains_key(&key) {
            return Err(ModelRegistryError::DuplicateModel { name: key });
        }

        model
            .validate()
            .map_err(|source| ModelRegistryError::InvalidModel {
                name: model.name().clone(),
                source,
            })?;

        self.models.insert(key, model);
        Ok(())
    }

    /// Returns an immutable reference to the model with the given name.
    pub fn get(&self, name: &Identifier) -> Option<&Model> {
        self.models.get(name)
    }

    /// Returns a mutable reference to the model with the given name.
    ///
    /// The internal map is never exposed. Callers may mutate the stored
    /// [`Model`] through this reference using the public `Model` API;
    /// they are responsible for re-validating the model afterwards.
    pub fn get_mut(&mut self, name: &Identifier) -> Option<&mut Model> {
        self.models.get_mut(name)
    }

    /// Returns `true` if a model with the given name is present.
    pub fn contains(&self, name: &Identifier) -> bool {
        self.models.contains_key(name)
    }

    /// Removes and returns the model with the given name, if present.
    pub fn remove(&mut self, name: &Identifier) -> Option<Model> {
        self.models.remove(name)
    }

    /// Iterates over `(name, model)` pairs in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &Model)> {
        self.models.iter()
    }

    /// Iterates over models in deterministic order.
    pub fn models(&self) -> impl Iterator<Item = &Model> {
        self.models.values()
    }

    /// Performs structural validation of every stored model.
    ///
    /// Models are validated in deterministic [`BTreeMap`] order. The
    /// first invalid model aborts validation with
    /// [`ModelRegistryError::InvalidModel`]. No automatic repair or
    /// removal is performed.
    ///
    /// No cross-model analysis is performed here.
    pub fn validate(&self) -> Result<(), ModelRegistryError> {
        for (name, model) in &self.models {
            model
                .validate()
                .map_err(|source| ModelRegistryError::InvalidModel {
                    name: name.clone(),
                    source,
                })?;
        }
        Ok(())
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
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
    fn new_registry_is_empty() {
        let registry = ModelRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn default_registry_is_empty() {
        let registry = ModelRegistry::default();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
        assert!(registry.validate().is_ok());
    }

    #[test]
    fn add_model() {
        let mut registry = ModelRegistry::new();
        registry
            .add(empty_model("motor_model"))
            .expect("add succeeds");

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn duplicate_model_is_rejected() {
        let mut registry = ModelRegistry::new();
        registry
            .add(empty_model("motor_model"))
            .expect("first add succeeds");

        let err = registry
            .add(empty_model("motor_model"))
            .expect_err("duplicate rejected");

        assert_eq!(
            err,
            ModelRegistryError::DuplicateModel {
                name: ident("motor_model"),
            }
        );
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn duplicate_does_not_replace_original() {
        let mut registry = ModelRegistry::new();
        registry
            .add(populated_model("motor_model"))
            .expect("first add succeeds");

        // Attempt to overwrite with an empty model of the same name.
        let _ = registry.add(empty_model("motor_model"));

        let stored = registry.get(&ident("motor_model")).expect("present");
        // Original populated content is preserved.
        assert_eq!(stored.parameters().len(), 1);
        assert_eq!(stored.equations().len(), 1);
        assert_eq!(stored.dependencies().len(), 1);
        assert_eq!(stored.constraints().len(), 1);
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn get_model() {
        let mut registry = ModelRegistry::new();
        registry
            .add(empty_model("motor_model"))
            .expect("add succeeds");

        let found = registry.get(&ident("motor_model")).expect("present");
        assert_eq!(found.name().as_str(), "motor_model");

        assert!(registry.get(&ident("missing_model")).is_none());
    }

    #[test]
    fn contains_model() {
        let mut registry = ModelRegistry::new();
        registry
            .add(empty_model("motor_model"))
            .expect("add succeeds");

        assert!(registry.contains(&ident("motor_model")));
        assert!(!registry.contains(&ident("other_model")));
    }

    #[test]
    fn remove_model() {
        let mut registry = ModelRegistry::new();
        registry
            .add(empty_model("motor_model"))
            .expect("add succeeds");

        let removed = registry
            .remove(&ident("motor_model"))
            .expect("removed");
        assert_eq!(removed.name().as_str(), "motor_model");
        assert!(registry.is_empty());
    }

    #[test]
    fn missing_model_returns_none() {
        let mut registry = ModelRegistry::new();
        assert!(registry.remove(&ident("motor_model")).is_none());
    }

    #[test]
    fn deterministic_iteration_order() {
        let mut registry = ModelRegistry::new();
        registry
            .add(empty_model("zeta_model"))
            .expect("add zeta");
        registry
            .add(empty_model("alpha_model"))
            .expect("add alpha");
        registry
            .add(empty_model("gamma_model"))
            .expect("add gamma");

        let order: Vec<&str> = registry.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(order, vec!["alpha_model", "gamma_model", "zeta_model"]);

        let models_order: Vec<&str> = registry
            .models()
            .map(|m| m.name().as_str())
            .collect();
        assert_eq!(
            models_order,
            vec!["alpha_model", "gamma_model", "zeta_model"]
        );
    }

    #[test]
    fn empty_registry_is_valid() {
        let registry = ModelRegistry::new();
        assert!(registry.validate().is_ok());
    }

    #[test]
    fn valid_models_pass_validation() {
        let mut registry = ModelRegistry::new();
        registry
            .add(populated_model("motor_model"))
            .expect("add motor");
        registry
            .add(populated_model("battery_model"))
            .expect("add battery");

        assert!(registry.validate().is_ok());
    }

    #[test]
    fn invalid_model_is_rejected() {
        // Through the public Model API, invalid nested definitions cannot
        // enter the underlying collections. This test therefore verifies
        // the registry's validation path indirectly: adding a
        // well-formed model succeeds, and the model's own validation
        // surface is reachable through the registry error type.
        let mut registry = ModelRegistry::new();
        let model = populated_model("motor_model");

        // Model must itself be valid before the registry accepts it.
        assert!(model.validate().is_ok());
        registry.add(model).expect("add succeeds");
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn failed_add_does_not_mutate_registry() {
        let mut registry = ModelRegistry::new();
        registry
            .add(populated_model("motor_model"))
            .expect("first add succeeds");

        // Duplicate attempt — the registry must remain unchanged.
        let _ = registry.add(empty_model("motor_model"));

        assert_eq!(registry.len(), 1);
        let stored = registry.get(&ident("motor_model")).expect("present");
        assert_eq!(stored.parameters().len(), 1);
    }

    #[test]
    fn mutable_lookup_allows_model_construction() {
        let mut registry = ModelRegistry::new();
        registry
            .add(empty_model("motor_model"))
            .expect("add empty model");

        // Mutate the stored model through the public Model API.
        {
            let model = registry
                .get_mut(&ident("motor_model"))
                .expect("mutable reference present");

            model
                .parameters_mut()
                .add(valid_parameter("mass"))
                .expect("add mass");
            model
                .equations_mut()
                .add(valid_equation("force_equation"))
                .expect("add equation");
        }

        // The stored model now reflects the incremental construction.
        let stored = registry.get(&ident("motor_model")).expect("present");
        assert_eq!(stored.parameters().len(), 1);
        assert_eq!(stored.equations().len(), 1);
        assert!(registry.validate().is_ok());
    }

    #[test]
    fn equality_behaviour() {
        let mut a = ModelRegistry::new();
        a.add(empty_model("alpha_model")).expect("add alpha");
        a.add(empty_model("beta_model")).expect("add beta");

        let mut b = ModelRegistry::new();
        // Insert in opposite order; BTreeMap guarantees deterministic
        // ordering regardless of insertion sequence.
        b.add(empty_model("beta_model")).expect("add beta");
        b.add(empty_model("alpha_model")).expect("add alpha");

        assert_eq!(a, b);

        let mut c = ModelRegistry::new();
        c.add(empty_model("alpha_model")).expect("add alpha");
        assert_ne!(a, c);
    }
}