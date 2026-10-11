//! Versionable, unit-aware engineering requirements.
use helqix_core::{id::Identifier, quantity::Quantity};
/// Stated importance; priority is not a pass/fail condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequirementPriority { Could, Should, Must }
/// Supported scalar comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison { GreaterThan, GreaterOrEqual, Equal, LessOrEqual, LessThan }
/// Measurable requirement whose acceptance is an explicit human review decision.
#[derive(Debug, Clone, PartialEq)]
pub struct Requirement { pub id: Identifier, pub statement: String, pub priority: RequirementPriority, pub metric: Identifier, pub comparison: Comparison, pub threshold: Quantity, pub accepted: bool, pub revision: u64 }
impl Requirement {
    /// Creates a draft requirement; no authored statement is treated as accepted implicitly.
    pub fn draft(id: Identifier, statement: impl Into<String>, priority: RequirementPriority, metric: Identifier, comparison: Comparison, threshold: Quantity) -> Result<Self, RequirementError> {
        let statement = statement.into(); if statement.trim().is_empty() { return Err(RequirementError::EmptyStatement); } if statement.len() > 4_000 { return Err(RequirementError::StatementTooLong); }
        Ok(Self { id, statement, priority, metric, comparison, threshold, accepted: false, revision: 1 })
    }
    /// Records explicit review; revision advances only when the state actually changes.
    pub fn set_accepted(&mut self, accepted: bool) -> Result<(), RequirementError> { if self.accepted != accepted { self.revision = self.revision.checked_add(1).ok_or(RequirementError::RevisionOverflow)?; self.accepted = accepted; } Ok(()) }
}
/// Requirement definition error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementError { EmptyStatement, StatementTooLong, RevisionOverflow }
impl core::fmt::Display for RequirementError { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self { Self::EmptyStatement => f.write_str("requirement statement cannot be empty"), Self::StatementTooLong => f.write_str("requirement statement exceeds 4000 bytes"), Self::RevisionOverflow => f.write_str("requirement revision overflow") } } }
impl std::error::Error for RequirementError {}
