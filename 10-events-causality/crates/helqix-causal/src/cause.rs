//! Typed node participating as an asserted cause.
use helqix_core::id::Identifier;
/// Kind of entity represented in an observed causal graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CausalNodeKind { Event, Failure, State, Action, Measurement }
/// Stable cross-layer node key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalNode { pub kind: CausalNodeKind, pub id: Identifier }
impl CausalNode { /// Creates a causal node key.
    pub fn new(kind: CausalNodeKind, id: Identifier) -> Self { Self { kind, id } }
}
/// An asserted cause plus optional evidence reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cause { pub node: CausalNode, pub evidence: Option<Identifier> }
impl Cause { /// Creates a cause assertion.
    pub fn new(node: CausalNode, evidence: Option<Identifier>) -> Self { Self { node, evidence } }
}
