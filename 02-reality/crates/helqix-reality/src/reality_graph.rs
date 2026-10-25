//! Deterministic structural relationship graph for HELQIX Reality.
//!
//! `RealityGraph` stores relationships between engineering component
//! identities. It does not own `Component` values; component ownership remains
//! with the `Reality` aggregate.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

/// A directed structural relationship between two component identities.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RealityEdge {
    source: Identifier,
    target: Identifier,
}

impl RealityEdge {
    #[must_use]
    pub fn new(source: Identifier, target: Identifier) -> Self { Self { source, target } }
    #[must_use]
    pub fn source(&self) -> &Identifier { &self.source }
    #[must_use]
    pub fn target(&self) -> &Identifier { &self.target }
}

/// Errors produced by structural operations on [`RealityGraph`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealityGraphError {
    DuplicateNode { name: String },
    DuplicateEdge { source: String, target: String },
    MissingSourceNode { name: String },
    MissingTargetNode { name: String },
    SelfLoop { name: String },
}

impl fmt::Display for RealityGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateNode { name } => write!(f, "node '{name}' already exists in the reality graph"),
            Self::DuplicateEdge { source, target } => write!(f, "edge '{source} -> {target}' already exists in the reality graph"),
            Self::MissingSourceNode { name } => write!(f, "source node '{name}' does not exist in the reality graph"),
            Self::MissingTargetNode { name } => write!(f, "target node '{name}' does not exist in the reality graph"),
            Self::SelfLoop { name } => write!(f, "self-loop on node '{name}' is not allowed"),
        }
    }
}
impl Error for RealityGraphError {}

/// Deterministic directed graph describing structural relationships between
/// engineering component identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealityGraph {
    nodes: BTreeSet<Identifier>,
    edges: BTreeSet<RealityEdge>,
}

impl RealityGraph {
    #[must_use]
    pub fn new() -> Self { Self { nodes: BTreeSet::new(), edges: BTreeSet::new() } }
    #[must_use]
    pub fn node_count(&self) -> usize { self.nodes.len() }
    #[must_use]
    pub fn edge_count(&self) -> usize { self.edges.len() }
    #[must_use]
    pub fn is_empty(&self) -> bool { self.nodes.is_empty() }

    pub fn add_node(&mut self, name: Identifier) -> Result<(), RealityGraphError> {
        if self.nodes.contains(&name) { return Err(RealityGraphError::DuplicateNode { name: name.as_str().to_owned() }); }
        self.nodes.insert(name);
        Ok(())
    }

    #[must_use]
    pub fn contains_node(&self, name: &Identifier) -> bool { self.nodes.contains(name) }
    #[must_use]
    pub fn node(&self, name: &Identifier) -> Option<&Identifier> { self.nodes.get(name) }

    pub fn remove_node(&mut self, name: &Identifier) -> bool {
        if !self.nodes.remove(name) { return false; }
        self.edges.retain(|edge| edge.source() != name && edge.target() != name);
        true
    }

    pub fn add_edge(&mut self, source: &Identifier, target: &Identifier) -> Result<(), RealityGraphError> {
        if source == target { return Err(RealityGraphError::SelfLoop { name: source.as_str().to_owned() }); }
        if !self.nodes.contains(source) { return Err(RealityGraphError::MissingSourceNode { name: source.as_str().to_owned() }); }
        if !self.nodes.contains(target) { return Err(RealityGraphError::MissingTargetNode { name: target.as_str().to_owned() }); }
        let edge = RealityEdge::new(source.clone(), target.clone());
        if self.edges.contains(&edge) { return Err(RealityGraphError::DuplicateEdge { source: source.as_str().to_owned(), target: target.as_str().to_owned() }); }
        self.edges.insert(edge);
        Ok(())
    }

    #[must_use]
    pub fn contains_edge(&self, source: &Identifier, target: &Identifier) -> bool {
        self.edges.contains(&RealityEdge::new(source.clone(), target.clone()))
    }

    #[must_use]
    pub fn edge(&self, source: &Identifier, target: &Identifier) -> Option<&RealityEdge> {
        self.edges.get(&RealityEdge::new(source.clone(), target.clone()))
    }

    pub fn remove_edge(&mut self, source: &Identifier, target: &Identifier) -> bool {
        self.edges.remove(&RealityEdge::new(source.clone(), target.clone()))
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Identifier> { self.nodes.iter() }
    pub fn edges(&self) -> impl Iterator<Item = &RealityEdge> { self.edges.iter() }

    pub fn outgoing<'a>(&'a self, source: &'a Identifier) -> impl Iterator<Item = &'a RealityEdge> + 'a {
        self.edges.iter().filter(move |edge| edge.source() == source)
    }

    pub fn incoming<'a>(&'a self, target: &'a Identifier) -> impl Iterator<Item = &'a RealityEdge> + 'a {
        self.edges.iter().filter(move |edge| edge.target() == target)
    }

    pub fn validate(&self) -> Result<(), RealityGraphError> {
        for edge in &self.edges {
            if edge.source() == edge.target() { return Err(RealityGraphError::SelfLoop { name: edge.source().as_str().to_owned() }); }
            if !self.nodes.contains(edge.source()) { return Err(RealityGraphError::MissingSourceNode { name: edge.source().as_str().to_owned() }); }
            if !self.nodes.contains(edge.target()) { return Err(RealityGraphError::MissingTargetNode { name: edge.target().as_str().to_owned() }); }
        }
        Ok(())
    }
}

impl Default for RealityGraph { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    fn identifier(value: &str) -> Identifier { Identifier::parse(value).expect("test identifier must be valid") }

    #[test]
    fn new_graph_is_empty() {
        let graph = RealityGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.is_empty());
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn duplicate_node_is_rejected() {
        let mut graph = RealityGraph::new();
        graph.add_node(identifier("Motor")).unwrap();
        assert_eq!(graph.add_node(identifier("Motor")), Err(RealityGraphError::DuplicateNode { name: "Motor".to_owned() }));
    }

    #[test]
    fn edge_requires_existing_endpoints() {
        let mut graph = RealityGraph::new();
        graph.add_node(identifier("Motor")).unwrap();
        assert_eq!(graph.add_edge(&identifier("Unknown"), &identifier("Motor")), Err(RealityGraphError::MissingSourceNode { name: "Unknown".to_owned() }));
        assert_eq!(graph.add_edge(&identifier("Motor"), &identifier("Unknown")), Err(RealityGraphError::MissingTargetNode { name: "Unknown".to_owned() }));
    }

    #[test]
    fn self_loop_is_rejected_even_for_missing_node() {
        let mut graph = RealityGraph::new();
        assert_eq!(graph.add_edge(&identifier("Unknown"), &identifier("Unknown")), Err(RealityGraphError::SelfLoop { name: "Unknown".to_owned() }));
    }

    #[test]
    fn remove_edge_removes_exact_direction() {
        let mut graph = RealityGraph::new();
        let a = identifier("A");
        let b = identifier("B");
        graph.add_node(a.clone()).unwrap();
        graph.add_node(b.clone()).unwrap();
        graph.add_edge(&a, &b).unwrap();
        graph.add_edge(&b, &a).unwrap();
        assert!(graph.remove_edge(&a, &b));
        assert!(!graph.contains_edge(&a, &b));
        assert!(graph.contains_edge(&b, &a));
    }

    #[test]
    fn incoming_and_outgoing_views_are_deterministic() {
        let mut graph = RealityGraph::new();
        let a = identifier("A");
        let b = identifier("B");
        let c = identifier("C");
        graph.add_node(a.clone()).unwrap();
        graph.add_node(b.clone()).unwrap();
        graph.add_node(c.clone()).unwrap();
        graph.add_edge(&a, &b).unwrap();
        graph.add_edge(&a, &c).unwrap();
        graph.add_edge(&b, &c).unwrap();
        let incoming: Vec<(&str, &str)> = graph.incoming(&c).map(|edge| (edge.source().as_str(), edge.target().as_str())).collect();
        let outgoing: Vec<(&str, &str)> = graph.outgoing(&a).map(|edge| (edge.source().as_str(), edge.target().as_str())).collect();
        assert_eq!(incoming, vec![("A", "C"), ("B", "C")]);
        assert_eq!(outgoing, vec![("A", "B"), ("A", "C")]);
    }

    #[test]
    fn removing_node_removes_incident_edges() {
        let mut graph = RealityGraph::new();
        let a = identifier("A"); let b = identifier("B"); let c = identifier("C");
        graph.add_node(a.clone()).unwrap(); graph.add_node(b.clone()).unwrap(); graph.add_node(c.clone()).unwrap();
        graph.add_edge(&a, &b).unwrap(); graph.add_edge(&c, &a).unwrap();
        assert!(graph.remove_node(&a));
        assert_eq!(graph.edge_count(), 0);
        assert!(!graph.contains_node(&a));
    }
}