//! Typed effect node in an asserted causal relationship.
use crate::cause::CausalNode;
/// A described outcome and its evidence reference, when available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect { pub node: CausalNode, pub evidence: Option<helqix_core::id::Identifier> }
impl Effect { /// Creates an effect assertion.
    pub fn new(node: CausalNode, evidence: Option<helqix_core::id::Identifier>) -> Self { Self { node, evidence } }
}
