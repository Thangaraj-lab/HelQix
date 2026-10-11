//! Validated design variables and hard candidate constraints.
use std::{collections::BTreeMap, error::Error, fmt};
use helqix_core::Identifier;
use crate::{candidate::Candidate, constraints::{ConstraintTarget, ConstraintViolation, SearchConstraint, SearchConstraintError}, variable::{Variable, VariableError}};

/// Finite and explicitly constrained design space.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchSpace { variables: BTreeMap<Identifier, Variable>, constraints: BTreeMap<Identifier, SearchConstraint> }
impl SearchSpace {
    /// Creates an empty search space.
    pub fn new() -> Self { Self::default() }
    /// Adds one unique variable.
    pub fn add_variable(&mut self, variable: Variable) -> Result<(), SearchSpaceError> {
        let id = variable.id().clone();
        if self.variables.contains_key(&id) { return Err(SearchSpaceError::DuplicateVariable(id)); }
        self.variables.insert(id, variable); Ok(())
    }
    /// Adds one unique hard constraint referencing a known variable.
    pub fn add_constraint(&mut self, constraint: SearchConstraint) -> Result<(), SearchSpaceError> {
        if let ConstraintTarget::Parameter(variable) = constraint.target() {
            if !self.variables.contains_key(variable) { return Err(SearchSpaceError::UnknownVariable(variable.clone())); }
        }
        let id = constraint.id().clone();
        if self.constraints.contains_key(&id) { return Err(SearchSpaceError::DuplicateConstraint(id)); }
        self.constraints.insert(id, constraint); Ok(())
    }
    /// Number of dimensions.
    pub fn variable_count(&self) -> usize { self.variables.len() }
    /// Whether no design variables are configured.
    pub fn is_empty(&self) -> bool { self.variables.is_empty() }
    /// Variables in canonical identifier order.
    pub fn variables(&self) -> impl Iterator<Item = &Variable> { self.variables.values() }
    /// Constraints in canonical identifier order.
    pub fn constraints(&self) -> impl Iterator<Item = &SearchConstraint> { self.constraints.values() }
    /// Variable by identity.
    pub fn variable(&self, id: &Identifier) -> Option<&Variable> { self.variables.get(id) }
    /// Product of all domain sizes, or an overflow error.
    pub fn candidate_count(&self) -> Result<usize, SearchSpaceError> {
        if self.variables.is_empty() { return Err(SearchSpaceError::NoVariables); }
        self.variables.values().try_fold(1usize, |count, variable| count.checked_mul(variable.value_count()).ok_or(SearchSpaceError::CardinalityOverflow))
    }
    /// Validates assignment completeness, domain membership, and hard constraints.
    /// Constraint failures are returned as a list; structural assignment problems are errors.
    pub fn validate_assignment(&self, assignments: &BTreeMap<Identifier, f64>) -> Result<Vec<ConstraintViolation>, SearchSpaceError> {
        if self.variables.is_empty() { return Err(SearchSpaceError::NoVariables); }
        for variable in self.variables.values() {
            let value = assignments.get(variable.id()).ok_or_else(|| SearchSpaceError::MissingAssignment(variable.id().clone()))?;
            if !variable.contains_value(*value) { return Err(SearchSpaceError::ValueOutsideDomain { variable: variable.id().clone(), value: *value }); }
        }
        for id in assignments.keys() { if !self.variables.contains_key(id) { return Err(SearchSpaceError::UnknownAssignment(id.clone())); } }
        let candidate = Candidate::new(0, assignments.clone());
        Ok(self.constraints.values().filter(|constraint| matches!(constraint.target(), ConstraintTarget::Parameter(_))).filter_map(|constraint| constraint.check(&candidate)).collect())
    }
}

/// Search-space definition or assignment failure.
#[derive(Debug, Clone, PartialEq)]
pub enum SearchSpaceError {
    /// A variable identity is already defined.
    DuplicateVariable(Identifier),
    /// A constraint identity is already defined.
    DuplicateConstraint(Identifier),
    /// A constraint references an undeclared variable.
    UnknownVariable(Identifier),
    /// An assignment omits a declared variable.
    MissingAssignment(Identifier),
    /// An assignment contains an undeclared variable.
    UnknownAssignment(Identifier),
    /// An assigned value is non-finite or outside its domain.
    ValueOutsideDomain { /// Variable being assigned.
        variable: Identifier, /// Invalid value.
        value: f64 },
    /// Search requires at least one variable.
    NoVariables,
    /// Product of domain sizes does not fit `usize`.
    CardinalityOverflow,
    /// Variable definition is invalid.
    Variable(VariableError),
    /// Constraint definition is invalid.
    Constraint(SearchConstraintError),
}
impl fmt::Display for SearchSpaceError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self {
    Self::DuplicateVariable(id) => write!(f, "search variable `{id}` is already defined"), Self::DuplicateConstraint(id) => write!(f, "search constraint `{id}` is already defined"),
    Self::UnknownVariable(id) => write!(f, "constraint references undeclared variable `{id}`"), Self::MissingAssignment(id) => write!(f, "candidate is missing variable `{id}`"),
    Self::UnknownAssignment(id) => write!(f, "candidate assigns undeclared variable `{id}`"), Self::ValueOutsideDomain { variable, value } => write!(f, "value {value} is outside the domain of `{variable}`"),
    Self::NoVariables => f.write_str("search space must contain at least one variable"), Self::CardinalityOverflow => f.write_str("search-space cardinality overflowed usize"),
    Self::Variable(error) => write!(f, "invalid search variable: {error}"), Self::Constraint(error) => write!(f, "invalid search constraint: {error}"),
} } }
impl Error for SearchSpaceError { fn source(&self) -> Option<&(dyn Error + 'static)> { match self { Self::Variable(e) => Some(e), Self::Constraint(e) => Some(e), _ => None } } }

#[cfg(test)] mod tests {
    use super::*;
    use crate::{ConstraintOperator, Variable};
    fn id(s: &str) -> Identifier { Identifier::parse(s).unwrap() }
    #[test] fn cardinality_multiplies_domain_sizes() { let mut s = SearchSpace::new(); s.add_variable(Variable::discrete(id("a"), [1.0, 2.0]).unwrap()).unwrap(); s.add_variable(Variable::discrete(id("b"), [3.0, 4.0, 5.0]).unwrap()).unwrap(); assert_eq!(s.candidate_count(), Ok(6)); }
    #[test] fn constraint_rejects_candidate_before_evaluation() { let mut s = SearchSpace::new(); s.add_variable(Variable::continuous(id("mass"), 1.0, 20.0, 3).unwrap()).unwrap(); s.add_constraint(SearchConstraint::new(id("limit"), id("mass"), ConstraintOperator::LessOrEqual, 10.0).unwrap()).unwrap(); let mut a = BTreeMap::new(); a.insert(id("mass"), 20.0); assert_eq!(s.validate_assignment(&a).unwrap().len(), 1); }
    #[test] fn rejects_missing_and_unknown_assignments() { let mut s = SearchSpace::new(); s.add_variable(Variable::discrete(id("a"), [1.0]).unwrap()).unwrap(); assert!(matches!(s.validate_assignment(&BTreeMap::new()), Err(SearchSpaceError::MissingAssignment(_)))); }
}
