//! Semantic validation boundary for HELQIX Layer 03 — Engineering.
//!
//! This module provides deterministic, side-effect-free validation that
//! spans multiple Layer 03 engineering domains. Local structural
//! invariants remain the responsibility of the individual collections
//! ([`crate::parameter_set`], [`crate::equation_set`],
//! [`crate::dependency_graph`], [`crate::constraint_set`],
//! [`crate::model`], [`crate::model_registry`],
//! [`crate::engineering`]); this module adds the cross-domain checks
//! that only become meaningful when multiple collections are considered
//! together.
//!
//! # Scope
//!
//! - Validate the [`Engineering`] aggregate and its contained
//!   [`crate::model_registry::ModelRegistry`].
//! - Validate each contained [`Model`] structurally.
//! - Resolve cross-domain references that the current architecture
//!   actually exposes:
//!
//!   - [`ReferenceKind::Property`] endpoints on
//!     [`crate::dependency::Dependency`] values are resolved against
//!     the enclosing model's parameter set;
//!   - [`ReferenceKind::Property`] references on
//!     [`crate::constraint::Constraint`] values are resolved against
//!     the enclosing model's parameter set.
//!
//! - Leave interface and behaviour references for a higher-layer
//!   validator. The current [`Model`] does not expose interface or
//!   behaviour collections; rather than silently accepting or fabricating
//!   those registries, the validator simply does not resolve them here.
//!
//! # Non-scope
//!
//! Physics, simulation, execution, runtime state, time, failure
//! detection, causal analysis, branching, optimization, AI, and quantum
//! concerns belong to later HELQIX layers. The validator performs no
//! graph algorithms (no DFS, BFS, cycle detection, or topological sort)
//! and no cross-model analysis.
//!
//! # Determinism
//!
//! The validator iterates collections in their deterministic
//! [`BTreeMap`]/[`Vec`]-backed order and returns the first failure it
//! finds. Calling [`EngineeringValidator::validate`] twice on an
//! unchanged [`Engineering`] yields equivalent results.

use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;
use helqix_core::ReferenceKind;

use crate::engineering::{Engineering, EngineeringError};
use crate::model::{Model, ModelError};

/// Result type for the Layer 03 engineering validator.
pub type ValidationResult = Result<(), ValidationError>;

/// Errors produced by the Layer 03 engineering validator.
///
/// `ValidationError` composes transparently over the structural errors
/// already defined by the engineering collections. It adds cross-domain
/// reference failures that can only be detected once a whole model is
/// considered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// The engineering aggregate failed structural validation.
    InvalidEngineering {
        /// The underlying aggregate error.
        source: EngineeringError,
    },
    /// A model failed structural validation.
    InvalidModel {
        /// Name of the offending model.
        model: Identifier,
        /// The underlying model error.
        source: ModelError,
    },
    /// A dependency's endpoint could not be resolved against the
    /// enclosing model's parameter set.
    UnresolvedDependencyReference {
        /// Name of the model containing the dependency.
        model: Identifier,
        /// Name of the offending dependency.
        dependency: Identifier,
        /// Which endpoint failed to resolve.
        endpoint: DependencyEndpoint,
        /// The unresolvable reference's identifier.
        reference: Identifier,
    },
    /// A constraint's reference could not be resolved against the
    /// enclosing model's parameter set.
    UnresolvedConstraintReference {
        /// Name of the model containing the constraint.
        model: Identifier,
        /// Name of the offending constraint.
        constraint: Identifier,
        /// The unresolvable reference's identifier.
        reference: Identifier,
    },
}

/// Identifies which endpoint of a dependency failed to resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyEndpoint {
    /// The dependency's source endpoint (`source → target`).
    Source,
    /// The dependency's target endpoint (`source → target`).
    Target,
}

impl fmt::Display for DependencyEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependencyEndpoint::Source => f.write_str("source"),
            DependencyEndpoint::Target => f.write_str("target"),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::InvalidEngineering { source } => {
                write!(f, "invalid engineering aggregate: {source}")
            }
            ValidationError::InvalidModel { model, source } => {
                write!(f, "invalid model {}: {source}", model.as_str())
            }
            ValidationError::UnresolvedDependencyReference {
                model,
                dependency,
                endpoint,
                reference,
            } => write!(
                f,
                "model {} dependency {} has unresolved {} property reference to {}",
                model.as_str(),
                dependency.as_str(),
                endpoint,
                reference.as_str()
            ),
            ValidationError::UnresolvedConstraintReference {
                model,
                constraint,
                reference,
            } => write!(
                f,
                "model {} constraint {} has unresolved property reference to {}",
                model.as_str(),
                constraint.as_str(),
                reference.as_str()
            ),
        }
    }
}

impl Error for ValidationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ValidationError::InvalidEngineering { source } => Some(source),
            ValidationError::InvalidModel { source, .. } => Some(source),
            ValidationError::UnresolvedDependencyReference { .. } => None,
            ValidationError::UnresolvedConstraintReference { .. } => None,
        }
    }
}

/// Deterministic, side-effect-free semantic validator for the Layer 03
/// engineering aggregate.
///
/// The validator is stateless. All operations are associated functions
/// operating on explicit references.
///
/// # Validation phases
///
/// 1. Aggregate-local validation of the [`Engineering`] value.
/// 2. Per-model structural validation.
/// 3. Cross-domain reference resolution limited to what the current
///    architecture actually exposes:
///
///    - [`ReferenceKind::Property`] references on dependencies and
///      constraints are resolved against the enclosing model's
///      parameter set;
///    - other reference kinds are deferred to a higher-layer validator.
///
/// On failure, the first deterministic error is returned. No partial
/// mutation, repair, or normalization is performed.
#[derive(Debug, Clone, Copy, Default)]
pub struct EngineeringValidator;

impl EngineeringValidator {
    /// Validates the entire Layer 03 engineering aggregate.
    ///
    /// Returns [`ValidationResult::Ok`] when the aggregate, its models,
    /// and all resolvable cross-domain references are structurally
    /// consistent. Otherwise returns the first deterministic failure.
    pub fn validate(engineering: &Engineering) -> ValidationResult {
        // Phase 1 — aggregate-local structural validation.
        engineering
            .validate()
            .map_err(|source| ValidationError::InvalidEngineering { source })?;

        // Phase 2 — per-model structural validation, in deterministic
        // registry order.
        for (name, model) in engineering.models().iter() {
            Self::validate_model_structure(name, model)?;
        }

        // Phase 3 — cross-domain reference resolution, in the same
        // deterministic registry order.
        for (_, model) in engineering.models().iter() {
            Self::validate_model_references(model)?;
        }

        Ok(())
    }

    fn validate_model_structure(
        name: &Identifier,
        model: &Model,
    ) -> Result<(), ValidationError> {
        model
            .validate()
            .map_err(|source| ValidationError::InvalidModel {
                model: name.clone(),
                source,
            })
    }

    fn validate_model_references(model: &Model) -> Result<(), ValidationError> {
        let parameters = model.parameters();

        for (dependency_name, dependency) in model.dependencies().iter() {
            if dependency.source_kind() == ReferenceKind::Property
                && !parameters.contains(dependency.source_name())
            {
                return Err(ValidationError::UnresolvedDependencyReference {
                    model: model.name().clone(),
                    dependency: dependency_name.clone(),
                    endpoint: DependencyEndpoint::Source,
                    reference: dependency.source_name().clone(),
                });
            }
            if dependency.target_kind() == ReferenceKind::Property
                && !parameters.contains(dependency.target_name())
            {
                return Err(ValidationError::UnresolvedDependencyReference {
                    model: model.name().clone(),
                    dependency: dependency_name.clone(),
                    endpoint: DependencyEndpoint::Target,
                    reference: dependency.target_name().clone(),
                });
            }
        }

        for (constraint_name, constraint) in model.constraints().iter() {
            if constraint.reference_kind() == ReferenceKind::Property
                && !parameters.contains(constraint.reference_name())
            {
                return Err(ValidationError::UnresolvedConstraintReference {
                    model: model.name().clone(),
                    constraint: constraint_name.clone(),
                    reference: constraint.reference_name().clone(),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use helqix_core::units::Unit;

    use crate::constraint::{Constraint, ConstraintReference};
    use crate::dependency::{Dependency, DependencyReference};
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

    /// Builds a model containing two parameters (`payload_mass`,
    /// `required_thrust`) and a dependency whose source and target
    /// resolve against them.
    fn model_with_resolved_dependency() -> Model {
        let mut model = Model::new(ident("motor_model"));
        model
            .parameters_mut()
            .add(valid_parameter("payload_mass"))
            .expect("add payload_mass");
        model
            .parameters_mut()
            .add(valid_parameter("required_thrust"))
            .expect("add required_thrust");

        let dependency = Dependency::new(
            ident("thrust_dependency"),
            DependencyReference::property(ident("payload_mass")),
            DependencyReference::property(ident("required_thrust")),
        )
        .expect("structurally valid dependency");
        model
            .dependencies_mut()
            .add(dependency)
            .expect("add dependency");
        model
    }

    /// Builds a model containing a constraint whose property reference
    /// resolves against a declared parameter.
    fn model_with_resolved_constraint() -> Model {
        let mut model = Model::new(ident("thermal_model"));
        model
            .parameters_mut()
            .add(valid_parameter("temperature"))
            .expect("add temperature");

        let constraint = Constraint::new(
            ident("motor_temp_limit"),
            ConstraintReference::property(ident("temperature")),
            0.0,
            120.0,
        )
        .expect("structurally valid constraint");
        model
            .constraints_mut()
            .add(constraint)
            .expect("add constraint");
        model
    }

    #[test]
    fn empty_engineering_is_valid() {
        let engineering = Engineering::new(ident("empty_engineering"));
        assert!(EngineeringValidator::validate(&engineering).is_ok());
    }

    #[test]
    fn valid_engineering_is_valid() {
        let mut engineering = Engineering::new(ident("vehicle_engineering"));
        engineering
            .models_mut()
            .add(model_with_resolved_dependency())
            .expect("add dependency model");
        engineering
            .models_mut()
            .add(model_with_resolved_constraint())
            .expect("add constraint model");

        assert!(EngineeringValidator::validate(&engineering).is_ok());
    }

    #[test]
    fn valid_model_is_valid() {
        let mut engineering = Engineering::new(ident("vehicle_engineering"));
        engineering
            .models_mut()
            .add(model_with_resolved_dependency())
            .expect("add model");

        assert!(EngineeringValidator::validate(&engineering).is_ok());
    }

    #[test]
    fn invalid_model_structure_is_reported() {
        // Through the public Model API, invalid nested definitions
        // cannot enter the underlying collections, so a fully public
        // construction path cannot produce an invalid Model. We verify
        // the error surface and source propagation directly on the
        // error type instead.
        use crate::parameter_set::ParameterSetError;

        let synthetic = ValidationError::InvalidModel {
            model: ident("motor_model"),
            source: ModelError::InvalidParameters {
                source: ParameterSetError::DuplicateParameter {
                    name: "mass".to_owned(),
                },
            },
        };

        let rendered = synthetic.to_string();
        assert!(rendered.contains("invalid model"));
        assert!(rendered.contains("motor_model"));
        assert!(synthetic.source().is_some());
    }

    #[test]
    fn deterministic_validation_order() {
        // Two models, each with an unresolved dependency. Validation
        // must deterministically report the model whose identifier
        // sorts first in BTreeMap order.
        let mut engineering = Engineering::new(ident("vehicle_engineering"));

        let mut zeta = Model::new(ident("zeta_model"));
        zeta.dependencies_mut()
            .add(
                Dependency::new(
                    ident("z_dep"),
                    DependencyReference::property(ident("missing_z")),
                    DependencyReference::property(ident("missing_z2")),
                )
                .expect("structurally valid"),
            )
            .expect("add z_dep");
        engineering.models_mut().add(zeta).expect("add zeta");

        let mut alpha = Model::new(ident("alpha_model"));
        alpha
            .dependencies_mut()
            .add(
                Dependency::new(
                    ident("a_dep"),
                    DependencyReference::property(ident("missing_a")),
                    DependencyReference::property(ident("missing_a2")),
                )
                .expect("structurally valid"),
            )
            .expect("add a_dep");
        engineering.models_mut().add(alpha).expect("add alpha");

        let err = EngineeringValidator::validate(&engineering)
            .expect_err("unresolved reference reported");

        match err {
            ValidationError::UnresolvedDependencyReference {
                model,
                dependency,
                endpoint,
                reference,
            } => {
                assert_eq!(model.as_str(), "alpha_model");
                assert_eq!(dependency.as_str(), "a_dep");
                assert_eq!(endpoint, DependencyEndpoint::Source);
                assert_eq!(reference.as_str(), "missing_a");
            }
            other => panic!("expected UnresolvedDependencyReference, got {other:?}"),
        }
    }

    #[test]
    fn validation_does_not_mutate_engineering() {
        let mut engineering = Engineering::new(ident("vehicle_engineering"));
        engineering
            .models_mut()
            .add(model_with_resolved_dependency())
            .expect("add model");

        let snapshot = engineering.clone();

        assert!(EngineeringValidator::validate(&engineering).is_ok());

        // Aggregate is unchanged after validation.
        assert_eq!(engineering, snapshot);
    }

    #[test]
    fn repeated_validation_is_deterministic() {
        let mut engineering = Engineering::new(ident("vehicle_engineering"));
        engineering
            .models_mut()
            .add(model_with_resolved_dependency())
            .expect("add dependency model");
        engineering
            .models_mut()
            .add(model_with_resolved_constraint())
            .expect("add constraint model");

        let first = EngineeringValidator::validate(&engineering);
        let second = EngineeringValidator::validate(&engineering);
        let third = EngineeringValidator::validate(&engineering);

        assert!(first.is_ok());
        assert_eq!(first, second);
        assert_eq!(second, third);
    }

    #[test]
    fn valid_cross_domain_references_pass() {
        // A model whose dependencies and constraints reference declared
        // parameters must pass cross-domain validation.
        let mut engineering = Engineering::new(ident("vehicle_engineering"));
        engineering
            .models_mut()
            .add(model_with_resolved_dependency())
            .expect("add dependency model");
        engineering
            .models_mut()
            .add(model_with_resolved_constraint())
            .expect("add constraint model");

        assert!(EngineeringValidator::validate(&engineering).is_ok());
    }

    #[test]
    fn unresolved_cross_domain_reference_is_reported() {
        // A dependency references a parameter that does not exist in the
        // model. Because the dependency itself is structurally valid,
        // only cross-domain validation can detect the failure.
        let mut engineering = Engineering::new(ident("vehicle_engineering"));

        let mut model = Model::new(ident("motor_model"));
        // No parameters declared.
        model
            .dependencies_mut()
            .add(
                Dependency::new(
                    ident("thrust_dependency"),
                    DependencyReference::property(ident("payload_mass")),
                    DependencyReference::property(ident("required_thrust")),
                )
                .expect("structurally valid dependency"),
            )
            .expect("add dependency");
        engineering.models_mut().add(model).expect("add model");

        let err = EngineeringValidator::validate(&engineering)
            .expect_err("unresolved reference reported");

        match err {
            ValidationError::UnresolvedDependencyReference {
                model,
                dependency,
                endpoint,
                reference,
            } => {
                assert_eq!(model.as_str(), "motor_model");
                assert_eq!(dependency.as_str(), "thrust_dependency");
                assert_eq!(endpoint, DependencyEndpoint::Source);
                assert_eq!(reference.as_str(), "payload_mass");
            }
            other => panic!("expected UnresolvedDependencyReference, got {other:?}"),
        }
    }

    #[test]
    fn unresolved_constraint_reference_is_reported() {
        let mut engineering = Engineering::new(ident("vehicle_engineering"));

        let mut model = Model::new(ident("thermal_model"));
        // No parameters declared.
        model
            .constraints_mut()
            .add(
                Constraint::new(
                    ident("motor_temp_limit"),
                    ConstraintReference::property(ident("temperature")),
                    0.0,
                    120.0,
                )
                .expect("structurally valid constraint"),
            )
            .expect("add constraint");
        engineering.models_mut().add(model).expect("add model");

        let err = EngineeringValidator::validate(&engineering)
            .expect_err("unresolved constraint reference reported");

        match err {
            ValidationError::UnresolvedConstraintReference {
                model,
                constraint,
                reference,
            } => {
                assert_eq!(model.as_str(), "thermal_model");
                assert_eq!(constraint.as_str(), "motor_temp_limit");
                assert_eq!(reference.as_str(), "temperature");
            }
            other => panic!("expected UnresolvedConstraintReference, got {other:?}"),
        }
    }

    #[test]
    fn non_property_references_are_deferred() {
        // Interface and behaviour references cannot be resolved at the
        // Model level in the current architecture. The validator must
        // defer them rather than fabricating registries or silently
        // rejecting structurally valid dependencies.
        let mut engineering = Engineering::new(ident("vehicle_engineering"));

        let mut model = Model::new(ident("motor_model"));
        model
            .dependencies_mut()
            .add(
                Dependency::new(
                    ident("power_dependency"),
                    DependencyReference::interface(ident("power_output")),
                    DependencyReference::interface(ident("power_input")),
                )
                .expect("structurally valid dependency"),
            )
            .expect("add dependency");
        engineering.models_mut().add(model).expect("add model");

        assert!(EngineeringValidator::validate(&engineering).is_ok());
    }
}