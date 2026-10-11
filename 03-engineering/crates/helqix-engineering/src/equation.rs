use std::{collections::BTreeSet, error::Error, fmt};
use helqix_core::id::Identifier;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EquationError {
    EmptyExpression,
    MissingInput { equation: Identifier },
    OutputIsInput { name: Identifier },
    DuplicateInput { name: Identifier },
}
impl fmt::Display for EquationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyExpression => write!(f, "equation expression is empty"),
            Self::MissingInput { equation } => write!(f, "equation {} has no inputs", equation.as_str()),
            Self::OutputIsInput { name } => write!(f, "output {} cannot also be an input", name.as_str()),
            Self::DuplicateInput { name } => write!(f, "duplicate equation input {}", name.as_str()),
        }
    }
}
impl Error for EquationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Equation {
    name: Identifier,
    output: Identifier,
    expression: String,
    inputs: BTreeSet<Identifier>,
}
impl Equation {
    pub fn new(name: Identifier, output: Identifier, expression: impl Into<String>) -> Result<Self, EquationError> {
        let expression = expression.into();
        if expression.trim().is_empty() { return Err(EquationError::EmptyExpression); }
        Ok(Self { name, output, expression, inputs: BTreeSet::new() })
    }
    pub fn name(&self) -> &Identifier { &self.name }
    pub fn output(&self) -> &Identifier { &self.output }
    pub fn expression(&self) -> &str { &self.expression }
    pub fn input_count(&self) -> usize { self.inputs.len() }
    pub fn has_input(&self, input: &Identifier) -> bool { self.inputs.contains(input) }
    pub fn inputs(&self) -> impl Iterator<Item = &Identifier> { self.inputs.iter() }
    pub fn add_input(&mut self, input: Identifier) -> Result<(), EquationError> {
        if input == self.output { return Err(EquationError::OutputIsInput { name: input }); }
        if !self.inputs.insert(input.clone()) { return Err(EquationError::DuplicateInput { name: input }); }
        Ok(())
    }
    pub fn validate(&self) -> Result<(), EquationError> {
        if self.expression.trim().is_empty() { return Err(EquationError::EmptyExpression); }
        if self.inputs.is_empty() { return Err(EquationError::MissingInput { equation: self.name.clone() }); }
        if self.inputs.contains(&self.output) { return Err(EquationError::OutputIsInput { name: self.output.clone() }); }
        Ok(())
    }
}
