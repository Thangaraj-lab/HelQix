//! Directed acyclic graph of explicitly asserted relationships.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use crate::{cause::{Cause, CausalNode}, effect::Effect};

/// One assertion from an upstream cause to a downstream effect.
#[derive(Debug, Clone, PartialEq)]
pub struct CausalEdge { pub cause: Cause, pub effect: Effect, pub confidence: f64 }
/// Bounded deterministic causal graph. Confidence is metadata, not a probability unless calibrated externally.
#[derive(Debug, Clone)]
pub struct CausalGraph { edges: BTreeMap<(CausalNode, CausalNode), CausalEdge>, nodes: BTreeSet<CausalNode>, maximum_nodes: usize }
impl CausalGraph {
    /// Creates a graph limited to `maximum_nodes`; a zero limit is rejected.
    pub fn new(maximum_nodes: usize) -> Result<Self, CausalGraphError> { if maximum_nodes == 0 { return Err(CausalGraphError::InvalidCapacity); } Ok(Self { edges: BTreeMap::new(), nodes: BTreeSet::new(), maximum_nodes }) }
    /// Creates a graph with a conservative default limit.
    pub fn bounded_default() -> Self { Self::new(10_000).expect("positive built-in limit") }
    /// Adds a new edge if it is finite, uniquely identified, and does not create a directed cycle.
    pub fn add_edge(&mut self, cause: Cause, effect: Effect, confidence: f64) -> Result<(), CausalGraphError> {
        if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) { return Err(CausalGraphError::ConfidenceOutOfRange); }
        if cause.node == effect.node { return Err(CausalGraphError::SelfEdge); }
        let key = (cause.node.clone(), effect.node.clone()); if self.edges.contains_key(&key) { return Err(CausalGraphError::DuplicateEdge); }
        if self.reachable(&effect.node, &cause.node) { return Err(CausalGraphError::CycleDetected); }
        let additional = usize::from(!self.nodes.contains(&cause.node)) + usize::from(!self.nodes.contains(&effect.node));
        if self.nodes.len().saturating_add(additional) > self.maximum_nodes { return Err(CausalGraphError::CapacityExceeded { maximum: self.maximum_nodes }); }
        self.nodes.insert(cause.node.clone()); self.nodes.insert(effect.node.clone());
        self.edges.insert(key, CausalEdge { cause, effect, confidence }); Ok(())
    }
    /// Checks graph reachability using a bounded iterative walk.
    pub fn reachable(&self, from: &CausalNode, to: &CausalNode) -> bool { let mut queue = VecDeque::from([from.clone()]); let mut visited = BTreeSet::new(); while let Some(node) = queue.pop_front() { if &node == to { return true; } if !visited.insert(node.clone()) { continue; } for edge in self.edges.values().filter(|edge| edge.cause.node == node) { queue.push_back(edge.effect.node.clone()); } } false }
    /// Returns all direct upstream neighbors.
    pub fn causes_of(&self, node: &CausalNode) -> Vec<&CausalEdge> { self.edges.values().filter(|edge| &edge.effect.node == node).collect() }
    /// Returns all direct downstream neighbors.
    pub fn effects_of(&self, node: &CausalNode) -> Vec<&CausalEdge> { self.edges.values().filter(|edge| &edge.cause.node == node).collect() }
    /// Deterministically walks all upstream nodes reachable from a target.
    pub fn ancestors(&self, target: &CausalNode) -> BTreeSet<CausalNode> { let mut found = BTreeSet::new(); let mut queue = VecDeque::from([target.clone()]); while let Some(node) = queue.pop_front() { for edge in self.causes_of(&node) { if found.insert(edge.cause.node.clone()) { queue.push_back(edge.cause.node.clone()); } } } found }
    /// Number of nodes.
    pub fn node_count(&self) -> usize { self.nodes.len() }
    /// Number of asserted relationships.
    pub fn edge_count(&self) -> usize { self.edges.len() }
    /// Whether the graph contains a node.
    pub fn contains(&self, node: &CausalNode) -> bool { self.nodes.contains(node) }
    pub(crate) fn edges(&self) -> impl Iterator<Item = &CausalEdge> { self.edges.values() }
}
impl Default for CausalGraph { fn default() -> Self { Self::bounded_default() } }
/// Causal graph invariant error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalGraphError { InvalidCapacity, ConfidenceOutOfRange, SelfEdge, DuplicateEdge, CycleDetected, CapacityExceeded { maximum: usize } }
impl core::fmt::Display for CausalGraphError { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self { Self::InvalidCapacity => f.write_str("causal graph capacity must be positive"), Self::ConfidenceOutOfRange => f.write_str("assertion confidence must be finite and between zero and one"), Self::SelfEdge => f.write_str("causal self-relationships are not allowed"), Self::DuplicateEdge => f.write_str("causal relationship already exists"), Self::CycleDetected => f.write_str("causal relationship would create a directed cycle"), Self::CapacityExceeded { maximum } => write!(f, "causal graph node limit {maximum} reached") } } }
impl std::error::Error for CausalGraphError {}
