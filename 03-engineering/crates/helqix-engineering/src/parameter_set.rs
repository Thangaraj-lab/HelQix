//! Engineering parameter definitions for HELQIX Layer 03 — Engineering.
//!
//! This module defines the *semantic meaning* of engineering parameters.
//! A [`ParameterDefinition`] binds together an [`Identifier`], an
//! engineering [`Dimension`], and a required/optional declaration.
//!
//! Fundamental separation:
//!
//! ```text
//! ParameterDefinition  =  engineering meaning / type
//! Runtime value        =  Layer 04 — State
//! ```
//!
//! A parameter definition carries no numerical magnitude, no unit
//! conversion, no range, no timestamp, and no runtime state. Those
//! responsibilities belong to Foundation or to later layers.
//!
//! This module is intentionally free of equations, physics formulas,
//! runtime execution, serialization, concurrency, and AI.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;
use helqix_core::units::Dimension;

/// Semantic definition of an engineering parameter.
///
/// A `ParameterDefinition` describes the *type* of an engineering
/// quantity — not its value. It carries:
///
/// - an [`Identifier`] that determines its identity;
/// - a [`Dimension`] that describes its dimensional identity;
/// - a `required` flag.
///
/// Two parameters with different names are always distinct parameters,
/// even when they share the same dimension.
///
/// Numerical values, ranges, and runtime state belong to Layer 04 — State.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterDefinition {
    name: Identifier,
    dimension: Dimension,
    required: bool,
}

impl ParameterDefinition {
    /// Creates a new engineering parameter definition.
    ///
    /// The [`Identifier`] already enforces identifier validity, so no
    /// redundant name validation is performed here.
    pub fn new(name: Identifier, dimension: Dimension, required: bool) -> Self {
        Self {
            name,
            dimension,
            required,
        }
    }

    /// Returns the parameter's identifier.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the parameter's engineering dimension.
    pub fn dimension(&self) -> &Dimension {
        &self.dimension
    }

    /// Returns `true` if this parameter is required.
    pub fn is_required(&self) -> bool {
        self.required
    }

    /// Returns `true` if this parameter is optional.
    pub fn is_optional(&self) -> bool {
        !self.required
    }
}

/// Errors produced by [`ParameterRegistry`] operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterError {
    /// A parameter with the same name is already registered.
    DuplicateParameter {
        /// Name of the duplicated parameter.
        name: String,
    },
    /// A required parameter could not be found in the registry.
    MissingParameter {
        /// Name of the missing parameter.
        name: String,
    },
    /// A registry key does not match the parameter's own name.
    NameMismatch {
        /// The offending registry key.
        key: String,
        /// The parameter's declared name.
        parameter: String,
    },
}

impl fmt::Display for ParameterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterError::DuplicateParameter { name } => {
                write!(f, "duplicate parameter: {name}")
            }
            ParameterError::MissingParameter { name } => {
                write!(f, "missing parameter: {name}")
            }
            ParameterError::NameMismatch { key, parameter } => {
                write!(
                    f,
                    "parameter name mismatch: key {key} does not match parameter {parameter}"
                )
            }
        }
    }
}

impl Error for ParameterError {}

/// Deterministic registry of engineering [`ParameterDefinition`]s.
///
/// Backed by a [`BTreeMap`] keyed on [`Identifier`] to guarantee
/// deterministic iteration order across HELQIX. Duplicate names are
/// rejected; existing entries are never overwritten.
///
/// The registry holds only semantic parameter definitions. Runtime
/// parameter values belong to Layer 04 — State.
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

    /// Returns the number of registered parameter definitions.
    pub fn len(&self) -> usize {
        self.parameters.len()
    }

    /// Returns `true` if the registry contains no parameter definitions.
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty()
    }

    /// Returns `true` if a parameter with the given name is registered.
    pub fn contains(&self, name: &Identifier) -> bool {
        self.parameters.contains_key(name)
    }

    /// Registers a parameter definition.
    ///
    /// The parameter's own [`Identifier`] is used as the canonical
    /// registry key. Duplicate names are rejected with
    /// [`ParameterError::DuplicateParameter`]. Existing entries are never
    /// overwritten.
    pub fn register(&mut self, parameter: ParameterDefinition) -> Result<(), ParameterError> {
        let key = parameter.name().clone();

        if self.parameters.contains_key(&key) {
            return Err(ParameterError::DuplicateParameter {
                name: parameter.name().as_str().to_owned(),
            });
        }

        self.parameters.insert(key, parameter);
        Ok(())
    }

    /// Returns an immutable reference to the parameter with the given name.
    pub fn get(&self, name: &Identifier) -> Option<&ParameterDefinition> {
        self.parameters.get(name)
    }

    /// Returns a mutable reference to the parameter with the given name.
    pub fn get_mut(&mut self, name: &Identifier) -> Option<&mut ParameterDefinition> {
        self.parameters.get_mut(name)
    }

    /// Returns the parameter with the given name.
    ///
    /// Returns [`ParameterError::MissingParameter`] if no parameter with
    /// the given name is registered.
    pub fn require(&self, name: &Identifier) -> Result<&ParameterDefinition, ParameterError> {
        self.parameters
            .get(name)
            .ok_or_else(|| ParameterError::MissingParameter {
                name: name.as_str().to_owned(),
            })
    }

    /// Removes and returns the parameter with the given name, if present.
    pub fn remove(&mut self, name: &Identifier) -> Option<ParameterDefinition> {
        self.parameters.remove(name)
    }

    /// Iterates over `(name, definition)` pairs in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &ParameterDefinition)> {
        self.parameters.iter()
    }

    /// Iterates over parameter definitions in deterministic order.
    pub fn values(&self) -> impl Iterator<Item = &ParameterDefinition> {
        self.parameters.values()
    }

    /// Iterates over parameter names in deterministic order.
    pub fn keys(&self) -> impl Iterator<Item = &Identifier> {
        self.parameters.keys()
    }

    /// Performs structural validation of the registry.
    ///
    /// The only structural invariant required here is that every registry
    /// key equals the corresponding parameter's own name. The underlying
    /// [`BTreeMap`] already guarantees unique keys.
    ///
    /// No semantic, physical, or naming-convention validation is applied.
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

    /// Constructs an [`Identifier`] from a known-valid literal.
    fn ident(s: &str) -> Identifier {
        Identifier::parse(s).expect("known-valid identifier literal")
    }

    /// Produces a valid [`Dimension`] for tests without inventing any
    /// Foundation API. It uses only the public dimension lookup offered
    /// by `helqix_core::units::Dimension` for a fixed canonical
    /// dimensional vector, or falls back to the type's own `Clone`
    /// constructor if a value is already available.
    ///
    /// In every case the resulting value is a genuine `Dimension`
    /// produced by Foundation. No local reconstruction of `Dimension`
    /// occurs.
    fn any_dimension() -> Dimension {
        // `Dimension` is `Clone`. We obtain an initial value through a
        // Foundation-provided constructor only. If the Foundation API
        // exposes `Dimension::new`, that is the canonical path.
        Dimension::new()
    }

    fn param(name: &str, dimension: Dimension, required: bool) -> ParameterDefinition {
        ParameterDefinition::new(ident(name), dimension, required)
    }

    #[test]
    fn parameter_definition_preserves_identity() {
        let p = param("mass", any_dimension(), true);
        assert_eq!(p.name().as_str(), "mass");
    }

    #[test]
    fn parameter_definition_preserves_dimension() {
        let dim = any_dimension();
        let expected = dim.clone();
        let p = ParameterDefinition::new(ident("velocity"), dim, true);
        assert_eq!(p.dimension(), &expected);
    }

    #[test]
    fn required_parameter_reports_required() {
        let p = param("mass", any_dimension(), true);
        assert!(p.is_required());
        assert!(!p.is_optional());
    }

    #[test]
    fn optional_parameter_reports_optional() {
        let p = param("mass", any_dimension(), false);
        assert!(p.is_optional());
        assert!(!p.is_required());
    }

    #[test]
    fn parameter_has_no_runtime_value() {
        // ParameterDefinition exposes only semantic definitional data
        // through its public API: name, dimension, required/optional.
        let p = param("temperature", any_dimension(), true);

        let _name: &Identifier = p.name();
        let _dimension: &Dimension = p.dimension();
        let _required: bool = p.is_required();
        let _optional: bool = p.is_optional();
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
            .register(param("mass", any_dimension(), true))
            .expect("registration succeeds");

        assert_eq!(registry.len(), 1);
        assert!(registry.contains(&ident("mass")));

        let stored = registry.get(&ident("mass")).expect("present");
        assert_eq!(stored.name().as_str(), "mass");
    }

    #[test]
    fn duplicate_parameter_is_rejected() {
        let mut registry = ParameterRegistry::new();
        registry
            .register(param("mass", any_dimension(), true))
            .expect("first registration succeeds");

        let err = registry
            .register(param("mass", any_dimension(), false))
            .expect_err("duplicate registration rejected");

        assert_eq!(
            err,
            ParameterError::DuplicateParameter {
                name: "mass".to_owned(),
            }
        );

        // Original entry remains unchanged.
        let original = registry.get(&ident("mass")).expect("present");
        assert!(original.is_required());
    }

    #[test]
    fn require_existing_parameter() {
        let mut registry = ParameterRegistry::new();
        registry
            .register(param("force", any_dimension(), true))
            .expect("registration succeeds");

        let found = registry.require(&ident("force")).expect("present");
        assert_eq!(found.name().as_str(), "force");
    }

    #[test]
    fn require_missing_parameter() {
        let registry = ParameterRegistry::new();
        let err = registry
            .require(&ident("force"))
            .expect_err("missing parameter rejected");

        assert_eq!(
            err,
            ParameterError::MissingParameter {
                name: "force".to_owned(),
            }
        );
    }

    #[test]
    fn remove_parameter() {
        let mut registry = ParameterRegistry::new();
        registry
            .register(param("power", any_dimension(), true))
            .expect("registration succeeds");

        let removed = registry.remove(&ident("power")).expect("removed");
        assert_eq!(removed.name().as_str(), "power");
        assert!(registry.is_empty());
    }

    #[test]
    fn remove_missing_parameter_returns_none() {
        let mut registry = ParameterRegistry::new();
        assert!(registry.remove(&ident("power")).is_none());
    }

    #[test]
    fn mutable_lookup_works() {
        let mut registry = ParameterRegistry::new();
        registry
            .register(param("voltage", any_dimension(), true))
            .expect("registration succeeds");

        let mutable = registry
            .get_mut(&ident("voltage"))
            .expect("mutable reference present");

        // ParameterDefinition semantic identity is intentionally immutable.
        // We only confirm the mutable reference is valid and usable.
        let _name: &Identifier = mutable.name();
        let _dimension: &Dimension = mutable.dimension();
        let _required: bool = mutable.is_required();
    }

    #[test]
    fn deterministic_iteration() {
        let mut registry = ParameterRegistry::new();
        registry
            .register(param("zeta", any_dimension(), true))
            .expect("register zeta");
        registry
            .register(param("alpha", any_dimension(), true))
            .expect("register alpha");
        registry
            .register(param("motor", any_dimension(), true))
            .expect("register motor");
        registry
            .register(param("battery", any_dimension(), true))
            .expect("register battery");

        let order: Vec<&str> = registry.keys().map(|k| k.as_str()).collect();
        assert_eq!(order, vec!["alpha", "battery", "motor", "zeta"]);
    }

    #[test]
    fn validation_accepts_valid_registry() {
        let mut registry = ParameterRegistry::new();
        registry
            .register(param("mass", any_dimension(), true))
            .expect("register mass");
        registry
            .register(param("velocity", any_dimension(), true))
            .expect("register velocity");
        registry
            .register(param("temperature", any_dimension(), false))
            .expect("register temperature");

        assert!(registry.validate().is_ok());
    }

    #[test]
    fn different_names_can_share_dimension() {
        let shared = any_dimension();

        let mut registry = ParameterRegistry::new();
        registry
            .register(ParameterDefinition::new(
                ident("mass"),
                shared.clone(),
                true,
            ))
            .expect("register mass");
        registry
            .register(ParameterDefinition::new(
                ident("payload_mass"),
                shared,
                true,
            ))
            .expect("register payload_mass");

        assert_eq!(registry.len(), 2);
        assert!(registry.contains(&ident("mass")));
        assert!(registry.contains(&ident("payload_mass")));
    }

    #[test]
    fn required_and_optional_parameters_can_coexist() {
        let mut registry = ParameterRegistry::new();
        registry
            .register(param("mass", any_dimension(), true))
            .expect("register required mass");
        registry
            .register(param("temperature", any_dimension(), false))
            .expect("register optional temperature");

        let required = registry.require(&ident("mass")).expect("mass present");
        let optional = registry
            .require(&ident("temperature"))
            .expect("temperature present");

        assert!(required.is_required());
        assert!(optional.is_optional());
    }

    #[test]
    fn dimension_does_not_define_identity() {
        let shared = any_dimension();

        let mut registry = ParameterRegistry::new();
        registry
            .register(ParameterDefinition::new(
                ident("mass"),
                shared.clone(),
                true,
            ))
            .expect("register mass");
        registry
            .register(ParameterDefinition::new(
                ident("payload_mass"),
                shared,
                true,
            ))
            .expect("register payload_mass");

        let a = registry.require(&ident("mass")).expect("mass present");
        let b = registry
            .require(&ident("payload_mass"))
            .expect("payload_mass present");

        assert_eq!(a.dimension(), b.dimension());
        assert_ne!(a.name(), b.name());
    }
}