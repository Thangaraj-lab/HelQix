//! Structural engineering model attached to a Reality.
//!
//! This module declares engineering relationships: named model definitions,
//! each with input references, output references, and an opaque expression.
//!
//! **Model definition != model execution.** Nothing here parses, evaluates,
//! solves, or simulates an expression. Expressions are stored as declarative
//! strings for later engineering layers to execute. References are identifiers
//! only; this layer owns no components, quantities, or runtime state.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

/// Errors produced by model definition and model aggregate operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    /// A model with this name is already registered.
    DuplicateModel {
        /// Model name.
        name: String,
    },
    /// The expression is empty or whitespace-only.
    EmptyExpression {
        /// Model name.
        name: String,
    },
    /// The reference is already declared as an input of the model.
    DuplicateInput {
        /// Model name.
        model: String,
        /// Reference name.
        name: String,
    },
    /// The reference is already declared as an output of the model.
    DuplicateOutput {
        /// Model name.
        model: String,
        /// Reference name.
        name: String,
    },
    /// The reference would be both an input and an output of the model.
    InputOutputConflict {
        /// Model name.
        model: String,
        /// Reference name.
        name: String,
    },
    /// The requested model is not registered.
    MissingModel {
        /// Model name.
        name: String,
    },
    /// A model failed structural validation.
    InvalidModel {
        /// Model name.
        name: String,
        /// Description of the validation failure.
        detail: String,
    },
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::DuplicateModel { name } => {
                write!(f, "duplicate model: {name}")
            }
            ModelError::EmptyExpression { name } => {
                write!(f, "model {name} has an empty expression")
            }
            ModelError::DuplicateInput { model, name } => {
                write!(f, "model {model} already declares input: {name}")
            }
            ModelError::DuplicateOutput { model, name } => {
                write!(f, "model {model} already declares output: {name}")
            }
            ModelError::InputOutputConflict { model, name } => {
                write!(
                    f,
                    "model {model} cannot use {name} as both input and output"
                )
            }
            ModelError::MissingModel { name } => {
                write!(f, "missing model: {name}")
            }
            ModelError::InvalidModel { name, detail } => {
                write!(f, "invalid model {name}: {detail}")
            }
        }
    }
}

impl Error for ModelError {}

/// Model-local reference to an engineering quantity or property.
///
/// A reference is an identity only. It is not resolved against components,
/// properties, or any registry in this layer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelReference {
    name: Identifier,
}

impl ModelReference {
    /// Creates a reference from an identifier.
    pub fn new(name: Identifier) -> Self {
        Self { name }
    }

    /// Returns the referenced name.
    pub fn name(&self) -> &Identifier {
        &self.name
    }
}

/// Declaration of one deterministic engineering relationship.
///
/// A definition holds a unique name, input and output references, and an
/// opaque expression string. It is a declaration only: the expression is
/// never parsed or evaluated here (model definition != model execution).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDefinition {
    name: Identifier,
    inputs: BTreeSet<ModelReference>,
    outputs: BTreeSet<ModelReference>,
    expression: String,
}

impl ModelDefinition {
    /// Creates a definition with no inputs or outputs.
    ///
    /// Returns [`ModelError::EmptyExpression`] if the expression is empty or
    /// whitespace-only. Any other non-empty text is accepted as opaque.
    pub fn new(name: Identifier, expression: String) -> Result<Self, ModelError> {
        if expression.trim().is_empty() {
            return Err(ModelError::EmptyExpression {
                name: name.as_str().to_owned(),
            });
        }

        Ok(Self {
            name,
            inputs: BTreeSet::new(),
            outputs: BTreeSet::new(),
            expression,
        })
    }

    /// Declares an input reference.
    ///
    /// Rejects duplicates and references already declared as outputs. The
    /// definition is unchanged on error.
    pub fn add_input(&mut self, reference: ModelReference) -> Result<(), ModelError> {
        if self.inputs.contains(&reference) {
            return Err(ModelError::DuplicateInput {
                model: self.name().as_str().to_owned(),
                name: reference.name().as_str().to_owned(),
            });
        }

        if self.outputs.contains(&reference) {
            return Err(ModelError::InputOutputConflict {
                model: self.name().as_str().to_owned(),
                name: reference.name().as_str().to_owned(),
            });
        }

        self.inputs.insert(reference);
        Ok(())
    }

    /// Declares an output reference.
    ///
    /// Rejects duplicates and references already declared as inputs. The
    /// definition is unchanged on error.
    pub fn add_output(&mut self, reference: ModelReference) -> Result<(), ModelError> {
        if self.outputs.contains(&reference) {
            return Err(ModelError::DuplicateOutput {
                model: self.name().as_str().to_owned(),
                name: reference.name().as_str().to_owned(),
            });
        }

        if self.inputs.contains(&reference) {
            return Err(ModelError::InputOutputConflict {
                model: self.name().as_str().to_owned(),
                name: reference.name().as_str().to_owned(),
            });
        }

        self.outputs.insert(reference);
        Ok(())
    }

    /// Returns the model name.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the opaque expression text. It is never evaluated.
    pub fn expression(&self) -> &str {
        &self.expression
    }

    /// Iterates input references in deterministic order.
    pub fn inputs(&self) -> impl Iterator<Item = &ModelReference> {
        self.inputs.iter()
    }

    /// Iterates output references in deterministic order.
    pub fn outputs(&self) -> impl Iterator<Item = &ModelReference> {
        self.outputs.iter()
    }

    /// Returns the number of inputs.
    pub fn input_count(&self) -> usize {
        self.inputs.len()
    }

    /// Returns the number of outputs.
    pub fn output_count(&self) -> usize {
        self.outputs.len()
    }

    /// Returns `true` if `name` is declared as an input.
    pub fn has_input(&self, name: &Identifier) -> bool {
        self.inputs.contains(&ModelReference::new(name.clone()))
    }

    /// Returns `true` if `name` is declared as an output.
    pub fn has_output(&self, name: &Identifier) -> bool {
        self.outputs.contains(&ModelReference::new(name.clone()))
    }

    /// Validates structural invariants.
    ///
    /// Checks that the expression is non-blank and that no reference is both
    /// an input and an output. Input and output uniqueness is guaranteed by
    /// the underlying sets. The expression itself is never parsed.
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.expression.trim().is_empty() {
            return Err(ModelError::EmptyExpression {
                name: self.name.as_str().to_owned(),
            });
        }

        for input in &self.inputs {
            if self.outputs.contains(input) {
                return Err(ModelError::InputOutputConflict {
                    model: self.name.as_str().to_owned(),
                    name: input.name().as_str().to_owned(),
                });
            }
        }

        Ok(())
    }
}

/// Aggregate of named model definitions.
///
/// Owns its [`ModelDefinition`] values and stores them in deterministic name
/// order. It declares relationships only and executes nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineeringModel {
    definitions: BTreeMap<Identifier, ModelDefinition>,
}

impl EngineeringModel {
    /// Creates an empty model.
    pub fn new() -> Self {
        Self {
            definitions: BTreeMap::new(),
        }
    }

    /// Returns the number of definitions.
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Returns `true` if no definitions are registered.
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Returns `true` if a definition with this name is registered.
    pub fn contains(&self, name: &Identifier) -> bool {
        self.definitions.contains_key(name)
    }

    /// Registers a definition under its own name.
    ///
    /// Rejects duplicates and structurally invalid definitions. Never
    /// overwrites and never renames.
    pub fn add(&mut self, definition: ModelDefinition) -> Result<(), ModelError> {
        let name = definition.name().clone();

        if self.definitions.contains_key(&name) {
            return Err(ModelError::DuplicateModel {
                name: name.as_str().to_owned(),
            });
        }

        if let Err(error) = definition.validate() {
            return Err(ModelError::InvalidModel {
                name: name.as_str().to_owned(),
                detail: error.to_string(),
            });
        }

        self.definitions.insert(name, definition);
        Ok(())
    }

    /// Returns the definition with this name, if registered.
    pub fn get(&self, name: &Identifier) -> Option<&ModelDefinition> {
        self.definitions.get(name)
    }

    /// Returns a mutable reference to the definition with this name, if
    /// registered. Mutation is limited to the definition's validated API.
    pub fn get_mut(&mut self, name: &Identifier) -> Option<&mut ModelDefinition> {
        self.definitions.get_mut(name)
    }

    /// Removes and returns the definition with this name, if registered.
    pub fn remove(&mut self, name: &Identifier) -> Option<ModelDefinition> {
        self.definitions.remove(name)
    }

    /// Returns the definition with this name or [`ModelError::MissingModel`].
    pub fn require(&self, name: &Identifier) -> Result<&ModelDefinition, ModelError> {
        self.definitions
            .get(name)
            .ok_or_else(|| ModelError::MissingModel {
                name: name.as_str().to_owned(),
            })
    }

    /// Iterates names and definitions in deterministic name order.
    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &ModelDefinition)> {
        self.definitions.iter()
    }

    /// Iterates definitions in deterministic name order.
    pub fn values(&self) -> impl Iterator<Item = &ModelDefinition> {
        self.definitions.values()
    }

    /// Iterates definition names in deterministic order.
    pub fn keys(&self) -> impl Iterator<Item = &Identifier> {
        self.definitions.keys()
    }

    /// Validates structural consistency of every definition.
    ///
    /// Checks that each key equals its definition name and that each
    /// definition validates. The mathematics of expressions is not examined.
    pub fn validate(&self) -> Result<(), ModelError> {
        for (key, definition) in &self.definitions {
            if key != definition.name() {
                return Err(ModelError::InvalidModel {
                    name: key.as_str().to_owned(),
                    detail: format!(
                        "registry key does not match definition name {}",
                        definition.name().as_str()
                    ),
                });
            }

            if let Err(error) = definition.validate() {
                return Err(ModelError::InvalidModel {
                    name: definition.name().as_str().to_owned(),
                    detail: error.to_string(),
                });
            }
        }

        Ok(())
    }
}

impl Default for EngineeringModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> Identifier {
        Identifier::parse(value).expect("valid identifier")
    }

    fn reference(value: &str) -> ModelReference {
        ModelReference::new(id(value))
    }

    fn definition(name: &str, expression: &str) -> ModelDefinition {
        ModelDefinition::new(id(name), expression.to_owned()).expect("valid definition")
    }

    fn newton_second_law() -> ModelDefinition {
        let mut model = definition("NewtonSecondLaw", "F = m * a");
        model.add_input(reference("mass")).unwrap();
        model.add_input(reference("acceleration")).unwrap();
        model.add_output(reference("force")).unwrap();
        model
    }

    #[test]
    fn new_model_is_empty() {
        let model = EngineeringModel::new();
        assert_eq!(model.len(), 0);
        assert!(model.is_empty());
        assert_eq!(model.validate(), Ok(()));
    }

    #[test]
    fn model_reference_exposes_name() {
        assert_eq!(reference("mass").name(), &id("mass"));
    }

    #[test]
    fn model_definition_rejects_empty_expression() {
        let result = ModelDefinition::new(id("Empty"), String::new());
        assert_eq!(
            result,
            Err(ModelError::EmptyExpression {
                name: "Empty".to_owned(),
            })
        );
    }

    #[test]
    fn model_definition_rejects_whitespace_expression() {
        let result = ModelDefinition::new(id("Blank"), "   ".to_owned());
        assert_eq!(
            result,
            Err(ModelError::EmptyExpression {
                name: "Blank".to_owned(),
            })
        );
    }

    #[test]
    fn model_definition_accepts_expression() {
        let model = definition("NewtonSecondLaw", "F = m * a");
        assert_eq!(model.name(), &id("NewtonSecondLaw"));
        assert_eq!(model.expression(), "F = m * a");
        assert_eq!(model.input_count(), 0);
        assert_eq!(model.output_count(), 0);
    }

    #[test]
    fn add_input_registers_reference() {
        let mut model = definition("M", "x = y");
        model.add_input(reference("mass")).unwrap();
        assert_eq!(model.input_count(), 1);
        assert!(model.has_input(&id("mass")));
        assert!(!model.has_output(&id("mass")));
    }

    #[test]
    fn duplicate_input_is_rejected() {
        let mut model = definition("M", "x = y");
        model.add_input(reference("mass")).unwrap();
        assert_eq!(
            model.add_input(reference("mass")),
            Err(ModelError::DuplicateInput {
                model: "M".to_owned(),
                name: "mass".to_owned(),
            })
        );
        assert_eq!(model.input_count(), 1);
    }

    #[test]
    fn add_output_registers_reference() {
        let mut model = definition("M", "x = y");
        model.add_output(reference("force")).unwrap();
        assert_eq!(model.output_count(), 1);
        assert!(model.has_output(&id("force")));
        assert!(!model.has_input(&id("force")));
    }

    #[test]
    fn duplicate_output_is_rejected() {
        let mut model = definition("M", "x = y");
        model.add_output(reference("force")).unwrap();
        assert_eq!(
            model.add_output(reference("force")),
            Err(ModelError::DuplicateOutput {
                model: "M".to_owned(),
                name: "force".to_owned(),
            })
        );
        assert_eq!(model.output_count(), 1);
    }

    #[test]
    fn input_output_conflict_is_rejected() {
        let mut model = definition("M", "x = y");
        model.add_input(reference("force")).unwrap();
        assert_eq!(
            model.add_output(reference("force")),
            Err(ModelError::InputOutputConflict {
                model: "M".to_owned(),
                name: "force".to_owned(),
            })
        );
        assert_eq!(model.output_count(), 0);
    }

    #[test]
    fn reverse_input_output_conflict_is_rejected() {
        let mut model = definition("M", "x = y");
        model.add_output(reference("force")).unwrap();
        assert_eq!(
            model.add_input(reference("force")),
            Err(ModelError::InputOutputConflict {
                model: "M".to_owned(),
                name: "force".to_owned(),
            })
        );
        assert_eq!(model.input_count(), 0);
    }

    #[test]
    fn model_definition_validation_accepts_valid_definition() {
        assert_eq!(newton_second_law().validate(), Ok(()));
    }

    #[test]
    fn add_model_registers_definition() {
        let mut model = EngineeringModel::new();
        let definition = newton_second_law();
        model.add(definition.clone()).unwrap();

        assert_eq!(model.len(), 1);
        assert!(model.contains(&id("NewtonSecondLaw")));
        assert_eq!(model.get(&id("NewtonSecondLaw")), Some(&definition));
    }

    #[test]
    fn duplicate_model_is_rejected() {
        let mut model = EngineeringModel::new();
        model.add(newton_second_law()).unwrap();
        assert_eq!(
            model.add(newton_second_law()),
            Err(ModelError::DuplicateModel {
                name: "NewtonSecondLaw".to_owned(),
            })
        );
        assert_eq!(model.len(), 1);
    }

    #[test]
    fn invalid_model_is_rejected() {
        // The constructor prevents invalid expressions.
        assert!(matches!(
            ModelDefinition::new(id("Bad"), "\t \n".to_owned()),
            Err(ModelError::EmptyExpression { .. })
        ));

        // Child modules may reach private fields; force an invalid state to
        // exercise the aggregate's validation path.
        let mut invalid = definition("Conflicted", "x = y");
        invalid.inputs.insert(reference("force"));
        invalid.outputs.insert(reference("force"));

        let mut model = EngineeringModel::new();
        let result = model.add(invalid);
        match result {
            Err(ModelError::InvalidModel { name, detail }) => {
                assert_eq!(name, "Conflicted");
                assert!(detail.contains("force"));
            }
            other => panic!("expected InvalidModel, got {other:?}"),
        }
        assert!(model.is_empty());
    }

    #[test]
    fn get_mut_allows_structural_mutation() {
        let mut model = EngineeringModel::new();
        model.add(definition("M", "x = y")).unwrap();

        let definition = model.get_mut(&id("M")).unwrap();
        definition.add_input(reference("mass")).unwrap();
        definition.add_output(reference("force")).unwrap();

        let stored = model.get(&id("M")).unwrap();
        assert!(stored.has_input(&id("mass")));
        assert!(stored.has_output(&id("force")));
        assert_eq!(model.validate(), Ok(()));
    }

    #[test]
    fn remove_model() {
        let mut model = EngineeringModel::new();
        let definition = newton_second_law();
        model.add(definition.clone()).unwrap();

        assert_eq!(model.remove(&id("NewtonSecondLaw")), Some(definition));
        assert!(model.is_empty());
        assert!(!model.contains(&id("NewtonSecondLaw")));
    }

    #[test]
    fn remove_missing_model_returns_none() {
        let mut model = EngineeringModel::new();
        assert_eq!(model.remove(&id("Missing")), None);
    }

    #[test]
    fn require_existing_model() {
        let mut model = EngineeringModel::new();
        let definition = newton_second_law();
        model.add(definition.clone()).unwrap();
        assert_eq!(model.require(&id("NewtonSecondLaw")), Ok(&definition));
    }

    #[test]
    fn require_missing_model_returns_error() {
        let model = EngineeringModel::new();
        assert_eq!(
            model.require(&id("Missing")),
            Err(ModelError::MissingModel {
                name: "Missing".to_owned(),
            })
        );
    }

    #[test]
    fn deterministic_model_iteration() {
        let mut model = EngineeringModel::new();
        for name in ["Zeta", "Alpha", "Motor", "Battery"] {
            model.add(definition(name, "x = y")).unwrap();
        }

        let expected = vec!["Alpha", "Battery", "Motor", "Zeta"];

        let keys: Vec<&str> = model.keys().map(Identifier::as_str).collect();
        assert_eq!(keys, expected);

        let names: Vec<&str> = model.values().map(|d| d.name().as_str()).collect();
        assert_eq!(names, expected);

        let pairs: Vec<&str> = model.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(pairs, expected);
    }

    #[test]
    fn deterministic_input_iteration() {
        let mut model = definition("M", "x = y");
        for name in ["Zeta", "Alpha", "Motor", "Battery"] {
            model.add_input(reference(name)).unwrap();
        }

        let names: Vec<&str> = model.inputs().map(|r| r.name().as_str()).collect();
        assert_eq!(names, vec!["Alpha", "Battery", "Motor", "Zeta"]);
    }

    #[test]
    fn deterministic_output_iteration() {
        let mut model = definition("M", "x = y");
        for name in ["Zeta", "Alpha", "Motor", "Battery"] {
            model.add_output(reference(name)).unwrap();
        }

        let names: Vec<&str> = model.outputs().map(|r| r.name().as_str()).collect();
        assert_eq!(names, vec!["Alpha", "Battery", "Motor", "Zeta"]);
    }

    #[test]
    fn model_validation_accepts_multiple_valid_models() {
        let mut model = EngineeringModel::new();
        model.add(newton_second_law()).unwrap();

        let mut power = definition("ElectricalPower", "P = V * I");
        power.add_input(reference("voltage")).unwrap();
        power.add_input(reference("current")).unwrap();
        power.add_output(reference("power")).unwrap();
        model.add(power).unwrap();

        model.add(definition("Heat", "Q = m * c * ΔT")).unwrap();

        assert_eq!(model.len(), 3);
        assert_eq!(model.validate(), Ok(()));
    }

    #[test]
    fn model_validation_detects_key_mismatch() {
        let mut model = EngineeringModel::new();
        model
            .definitions
            .insert(id("WrongKey"), definition("ActualName", "x = y"));

        match model.validate() {
            Err(ModelError::InvalidModel { name, detail }) => {
                assert_eq!(name, "WrongKey");
                assert!(detail.contains("ActualName"));
            }
            other => panic!("expected InvalidModel, got {other:?}"),
        }
    }

    #[test]
    fn model_validation_detects_invalid_definition() {
        let mut invalid = definition("Conflicted", "x = y");
        invalid.inputs.insert(reference("force"));
        invalid.outputs.insert(reference("force"));

        let mut model = EngineeringModel::new();
        model.definitions.insert(id("Conflicted"), invalid);

        assert!(matches!(
            model.validate(),
            Err(ModelError::InvalidModel { .. })
        ));
    }

    #[test]
    fn expression_is_not_evaluated() {
        let model = definition("Opaque", "THIS IS NOT A VALID PHYSICS PROGRAM");
        assert_eq!(model.expression(), "THIS IS NOT A VALID PHYSICS PROGRAM");
        assert_eq!(model.validate(), Ok(()));
    }

    #[test]
    fn model_does_not_require_inputs_or_outputs() {
        let model = definition("Bare", "x = y");
        assert_eq!(model.input_count(), 0);
        assert_eq!(model.output_count(), 0);
        assert_eq!(model.validate(), Ok(()));

        let mut aggregate = EngineeringModel::default();
        aggregate.add(model).unwrap();
        assert_eq!(aggregate.validate(), Ok(()));
    }

    #[test]
    fn error_display_includes_identities() {
        let error = ModelError::InputOutputConflict {
            model: "M".to_owned(),
            name: "force".to_owned(),
        };
        let text = error.to_string();
        assert!(text.contains("M"));
        assert!(text.contains("force"));
    }
}