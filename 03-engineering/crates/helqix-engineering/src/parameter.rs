//! Engineering parameter definitions for HELQIX Layer 03.
//!
//! This module defines the semantic contract of engineering parameters.
//!
//! A parameter definition describes:
//!
//! - its engineering identity;
//! - its physical dimension;
//! - whether the parameter is required.
//!
//! Runtime parameter values do not belong here.
//! Runtime values belong to the later HELQIX state layer.
//!
//! # Responsibility
//!
//! ```text
//! ParameterDefinition
//!     ├── Identifier
//!     ├── Dimension
//!     └── required / optional
//!
//! ParameterRegistry
//!     └── deterministic collection of parameter definitions
//! ```
//!
//! This module intentionally does not implement:
//!
//! - runtime values;
//! - equations;
//! - physics;
//! - unit conversion;
//! - simulation;
//! - numerical solving;
//! - optimization;
//! - AI;
//! - serialization;
//! - concurrency.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;
use helqix_core::units::Dimension;

/// Defines the semantic type of an engineering parameter.
///
/// `ParameterDefinition` describes what a parameter means, not its
/// runtime value.
///
/// For example:
///
/// ```text
/// mass       → mass dimension
/// velocity   → velocity dimension
/// temperature → temperature dimension
/// ```
///
/// The parameter name provides engineering identity while the dimension
/// provides physical dimensional identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterDefinition {
    name: Identifier,
    dimension: Dimension,
    required: bool,
}

impl ParameterDefinition {
    /// Creates a new parameter definition.
    pub fn new(
        name: Identifier,
        dimension: Dimension,
        required: bool,
    ) -> Self {
        Self {
            name,
            dimension,
            required,
        }
    }

    /// Returns the parameter identifier.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the parameter's physical dimension.
    pub fn dimension(&self) -> &Dimension {
        &self.dimension
    }

    /// Returns `true` when the parameter is required.
    pub fn is_required(&self) -> bool {
        self.required
    }

    /// Returns `true` when the parameter is optional.
    pub fn is_optional(&self) -> bool {
        !self.required
    }
}

/// Errors produced by the engineering parameter registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterError {
    /// A parameter with the same identifier is already registered.
    DuplicateParameter {
        name: String,
    },

    /// A requested parameter does not exist.
    MissingParameter {
        name: String,
    },

    /// The registry key and the parameter's internal identifier differ.
    ///
    /// This protects the fundamental registry invariant:
    ///
    /// ```text
    /// key == parameter.name()
    /// ```
    NameMismatch {
        key: String,
        parameter: String,
    },
}

impl fmt::Display for ParameterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateParameter { name } => {
                write!(f, "duplicate parameter: {name}")
            }

            Self::MissingParameter { name } => {
                write!(f, "missing parameter: {name}")
            }

            Self::NameMismatch { key, parameter } => {
                write!(
                    f,
                    "parameter name mismatch: registry key '{key}' \
                     does not match parameter name '{parameter}'"
                )
            }
        }
    }
}

impl Error for ParameterError {}

/// Deterministic registry of engineering parameter definitions.
///
/// Parameters are keyed by [`Identifier`] and stored in a [`BTreeMap`].
/// Therefore iteration order is deterministic.
///
/// The registry never silently overwrites an existing definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterRegistry {
    parameters: BTreeMap<Identifier, ParameterDefinition>,
}

impl ParameterRegistry {
    /// Creates an empty parameter registry.
    pub fn new() -> Self {
        Self {
            parameters: BTreeMap::new(),
        }
    }

    /// Returns the number of registered parameters.
    pub fn len(&self) -> usize {
        self.parameters.len()
    }

    /// Returns `true` when no parameters are registered.
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty()
    }

    /// Returns `true` when a parameter with the specified identifier exists.
    pub fn contains(&self, name: &Identifier) -> bool {
        self.parameters.contains_key(name)
    }

    /// Registers a parameter definition.
    ///
    /// Registration fails when the same identifier already exists.
    pub fn register(
        &mut self,
        parameter: ParameterDefinition,
    ) -> Result<(), ParameterError> {
        let name = parameter.name().clone();

        if self.parameters.contains_key(&name) {
            return Err(ParameterError::DuplicateParameter {
                name: parameter.name().as_str().to_owned(),
            });
        }

        self.parameters.insert(name, parameter);

        Ok(())
    }

    /// Returns a parameter definition by identifier.
    pub fn get(
        &self,
        name: &Identifier,
    ) -> Option<&ParameterDefinition> {
        self.parameters.get(name)
    }

    /// Returns mutable access to a parameter definition.
    pub fn get_mut(
        &mut self,
        name: &Identifier,
    ) -> Option<&mut ParameterDefinition> {
        self.parameters.get_mut(name)
    }

    /// Returns a parameter definition or an explicit missing-parameter error.
    pub fn require(
        &self,
        name: &Identifier,
    ) -> Result<&ParameterDefinition, ParameterError> {
        self.parameters
            .get(name)
            .ok_or_else(|| ParameterError::MissingParameter {
                name: name.as_str().to_owned(),
            })
    }

    /// Removes and returns a parameter definition.
    pub fn remove(
        &mut self,
        name: &Identifier,
    ) -> Option<ParameterDefinition> {
        self.parameters.remove(name)
    }

    /// Iterates over `(identifier, definition)` pairs in deterministic order.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&Identifier, &ParameterDefinition)> {
        self.parameters.iter()
    }

    /// Iterates over parameter definitions in deterministic order.
    pub fn values(&self) -> impl Iterator<Item = &ParameterDefinition> {
        self.parameters.values()
    }

    /// Iterates over parameter identifiers in deterministic order.
    pub fn keys(&self) -> impl Iterator<Item = &Identifier> {
        self.parameters.keys()
    }

    /// Validates the registry's structural invariants.
    ///
    /// The registry must always satisfy:
    ///
    /// ```text
    /// registry key == parameter.name()
    /// ```
    pub fn validate(&self) -> Result<(), ParameterError> {
        for (key, parameter) in &self.parameters {
            if key != parameter.name() {
                return Err(ParameterError::NameMismatch {
                    key: key.as_str().to_owned(),
                    parameter: parameter.name().as_str().to_owned(),
                });
            }
        }

        Ok(())
    }
}

impl Default for ParameterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helqix_core::units::Unit;

    fn identifier(value: &str) -> Identifier {
        Identifier::parse(value)
            .expect("test identifier must be valid")
    }

    /// Obtains a real Foundation dimension through the verified Unit API.
    ///
    /// This intentionally avoids inventing a `Dimension` constructor.
    fn mass_dimension() -> Dimension {
        Unit::parse("kg")
            .expect("kg must exist in the Foundation unit registry")
            .dimension()
            .clone()
    }

    #[test]
    fn parameter_definition_preserves_identity() {
        let name = identifier("mass");

        let parameter =
            ParameterDefinition::new(
                name.clone(),
                mass_dimension(),
                true,
            );

        assert_eq!(parameter.name(), &name);
    }

    #[test]
    fn parameter_definition_preserves_dimension() {
        let dimension = mass_dimension();

        let parameter =
            ParameterDefinition::new(
                identifier("mass"),
                dimension.clone(),
                true,
            );

        assert_eq!(parameter.dimension(), &dimension);
    }

    #[test]
    fn required_parameter_is_required() {
        let parameter =
            ParameterDefinition::new(
                identifier("mass"),
                mass_dimension(),
                true,
            );

        assert!(parameter.is_required());
        assert!(!parameter.is_optional());
    }

    #[test]
    fn optional_parameter_is_optional() {
        let parameter =
            ParameterDefinition::new(
                identifier("mass"),
                mass_dimension(),
                false,
            );

        assert!(!parameter.is_required());
        assert!(parameter.is_optional());
    }

    #[test]
    fn empty_registry_is_valid() {
        let registry = ParameterRegistry::new();

        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
        assert!(registry.validate().is_ok());
    }

    #[test]
    fn register_parameter() {
        let mut registry = ParameterRegistry::new();

        registry
            .register(ParameterDefinition::new(
                identifier("mass"),
                mass_dimension(),
                true,
            ))
            .expect("registration must succeed");

        assert_eq!(registry.len(), 1);
        assert!(registry.contains(&identifier("mass")));
        assert!(registry.get(&identifier("mass")).is_some());
    }

    #[test]
    fn duplicate_parameter_is_rejected() {
        let mut registry = ParameterRegistry::new();

        registry
            .register(ParameterDefinition::new(
                identifier("mass"),
                mass_dimension(),
                true,
            ))
            .expect("first registration must succeed");

        let error = registry
            .register(ParameterDefinition::new(
                identifier("mass"),
                mass_dimension(),
                false,
            ))
            .expect_err("duplicate registration must fail");

        assert_eq!(
            error,
            ParameterError::DuplicateParameter {
                name: "mass".to_owned(),
            }
        );

        let original = registry
            .require(&identifier("mass"))
            .expect("original parameter must remain");

        assert!(original.is_required());
    }

    #[test]
    fn require_existing_parameter() {
        let mut registry = ParameterRegistry::new();

        registry
            .register(ParameterDefinition::new(
                identifier("mass"),
                mass_dimension(),
                true,
            ))
            .expect("registration must succeed");

        let parameter = registry
            .require(&identifier("mass"))
            .expect("parameter must exist");

        assert_eq!(parameter.name().as_str(), "mass");
    }

    #[test]
    fn require_missing_parameter_returns_error() {
        let registry = ParameterRegistry::new();

        let error = registry
            .require(&identifier("mass"))
            .expect_err("missing parameter must fail");

        assert_eq!(
            error,
            ParameterError::MissingParameter {
                name: "mass".to_owned(),
            }
        );
    }

    #[test]
    fn remove_parameter() {
        let mut registry = ParameterRegistry::new();

        registry
            .register(ParameterDefinition::new(
                identifier("mass"),
                mass_dimension(),
                true,
            ))
            .expect("registration must succeed");

        let removed = registry
            .remove(&identifier("mass"))
            .expect("parameter must exist");

        assert_eq!(removed.name().as_str(), "mass");
        assert!(registry.is_empty());
    }

    #[test]
    fn remove_missing_parameter_returns_none() {
        let mut registry = ParameterRegistry::new();

        assert!(
            registry
                .remove(&identifier("mass"))
                .is_none()
        );
    }

    #[test]
    fn mutable_lookup_returns_parameter() {
        let mut registry = ParameterRegistry::new();

        registry
            .register(ParameterDefinition::new(
                identifier("mass"),
                mass_dimension(),
                true,
            ))
            .expect("registration must succeed");

        let parameter = registry
            .get_mut(&identifier("mass"))
            .expect("parameter must exist");

        assert_eq!(parameter.name().as_str(), "mass");
        assert!(parameter.is_required());
    }

    #[test]
    fn deterministic_iteration_order() {
        let mut registry = ParameterRegistry::new();

        for name in [
            "zeta",
            "alpha",
            "motor_mass",
            "battery_mass",
        ] {
            registry
                .register(ParameterDefinition::new(
                    identifier(name),
                    mass_dimension(),
                    true,
                ))
                .expect("registration must succeed");
        }

        let order: Vec<&str> =
            registry.keys().map(Identifier::as_str).collect();

        assert_eq!(
            order,
            vec![
                "alpha",
                "battery_mass",
                "motor_mass",
                "zeta"
            ]
        );
    }

    #[test]
    fn values_iteration_matches_key_order() {
        let mut registry = ParameterRegistry::new();

        for name in ["zeta", "alpha", "mass"] {
            registry
                .register(ParameterDefinition::new(
                    identifier(name),
                    mass_dimension(),
                    true,
                ))
                .expect("registration must succeed");
        }

        let names: Vec<&str> =
            registry.values().map(|p| p.name().as_str()).collect();

        assert_eq!(
            names,
            vec!["alpha", "mass", "zeta"]
        );
    }

    #[test]
    fn validation_accepts_valid_registry() {
        let mut registry = ParameterRegistry::new();

        registry
            .register(ParameterDefinition::new(
                identifier("mass"),
                mass_dimension(),
                true,
            ))
            .expect("registration must succeed");

        registry
            .register(ParameterDefinition::new(
                identifier("payload_mass"),
                mass_dimension(),
                false,
            ))
            .expect("registration must succeed");

        assert!(registry.validate().is_ok());
    }

    #[test]
    fn different_parameters_can_share_dimension() {
        let dimension = mass_dimension();

        let mut registry = ParameterRegistry::new();

        registry
            .register(ParameterDefinition::new(
                identifier("mass"),
                dimension.clone(),
                true,
            ))
            .expect("registration must succeed");

        registry
            .register(ParameterDefinition::new(
                identifier("payload_mass"),
                dimension,
                true,
            ))
            .expect("registration must succeed");

        assert_eq!(registry.len(), 2);

        let mass = registry
            .require(&identifier("mass"))
            .expect("mass must exist");

        let payload_mass = registry
            .require(&identifier("payload_mass"))
            .expect("payload_mass must exist");

        assert_eq!(
            mass.dimension(),
            payload_mass.dimension()
        );

        assert_ne!(mass.name(), payload_mass.name());
    }

    #[test]
    fn required_and_optional_parameters_can_coexist() {
        let mut registry = ParameterRegistry::new();

        registry
            .register(ParameterDefinition::new(
                identifier("mass"),
                mass_dimension(),
                true,
            ))
            .expect("registration must succeed");

        registry
            .register(ParameterDefinition::new(
                identifier("payload_mass"),
                mass_dimension(),
                false,
            ))
            .expect("registration must succeed");

        assert!(
            registry
                .require(&identifier("mass"))
                .expect("mass must exist")
                .is_required()
        );

        assert!(
            registry
                .require(&identifier("payload_mass"))
                .expect("payload_mass must exist")
                .is_optional()
        );
    }

    #[test]
    fn parameter_definition_contains_no_runtime_value() {
        let parameter =
            ParameterDefinition::new(
                identifier("mass"),
                mass_dimension(),
                true,
            );

        let _: &Identifier = parameter.name();
        let _: &Dimension = parameter.dimension();
        let _: bool = parameter.is_required();
    }

    #[test]
    fn default_registry_is_empty() {
        let registry = ParameterRegistry::default();

        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }
}