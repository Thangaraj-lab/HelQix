//! Structural directed graph of an engineering Reality.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealityGraphError {
    DuplicateNode { node: String },
    MissingNode { node: String },
    DuplicateEdge { source: String, target: String },
}

impl fmt::Display for RealityGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateNode { node } => write!(f, "graph node '{node}' already exists"),
            Self::MissingNode { node } => write!(f, "graph node '{node}' does not exist"),
            Self::DuplicateEdge { source, target } => write!(f, "graph edge '{source}' -> '{target}' already exists"),
        }
    }
}
impl Error for RealityGraphError {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphEdge { source: Identifier, target: Identifier }
impl GraphEdge {
    pub fn source(&self) -> &Identifier { &self.source }
    pub fn target(&self) -> &Identifier { &self.target }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RealityGraph {
    nodes: BTreeSet<Identifier>,
    edges: BTreeMap<(Identifier, Identifier), GraphEdge>,
}

impl RealityGraph {
    pub fn new() -> Self { Self::default() }
    pub fn add_node(&mut self, node: Identifier) -> Result<(), RealityGraphError> {
        if !self.nodes.insert(node.clone()) { return Err(RealityGraphError::DuplicateNode { node: node.as_str().to_owned() }); }
        Ok(())
    }
    pub fn add_edge(&mut self, source: &Identifier, target: &Identifier) -> Result<(), RealityGraphError> {
        if !self.nodes.contains(source) { return Err(RealityGraphError::MissingNode { node: source.as_str().to_owned() }); }
        if !self.nodes.contains(target) { return Err(RealityGraphError::MissingNode { node: target.as_str().to_owned() }); }
        let key = (source.clone(), target.clone());
        if self.edges.contains_key(&key) { return Err(RealityGraphError::DuplicateEdge { source: source.as_str().to_owned(), target: target.as_str().to_owned() }); }
        self.edges.insert(key, GraphEdge { source: source.clone(), target: target.clone() });
        Ok(())
    }
    pub fn remove_edge(&mut self, source: &Identifier, target: &Identifier) -> bool { self.edges.remove(&(source.clone(), target.clone())).is_some() }
    pub fn contains_edge(&self, source: &Identifier, target: &Identifier) -> bool { self.edges.contains_key(&(source.clone(), target.clone())) }
    pub fn nodes(&self) -> impl Iterator<Item = &Identifier> { self.nodes.iter() }
    pub fn incoming(&self, target: &Identifier) -> impl Iterator<Item = &GraphEdge> { self.edges.values().filter(move |edge| edge.target == *target) }
    pub fn outgoing(&self, source: &Identifier) -> impl Iterator<Item = &GraphEdge> { self.edges.values().filter(move |edge| edge.source == *source) }
    pub fn validate(&self) -> Result<(), RealityGraphError> {
        for edge in self.edges.values() {
            if !self.nodes.contains(edge.source()) { return Err(RealityGraphError::MissingNode { node: edge.source().as_str().to_owned() }); }
            if !self.nodes.contains(edge.target()) { return Err(RealityGraphError::MissingNode { node: edge.target().as_str().to_owned() }); }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identifier(value: &str) -> Identifier { Identifier::parse(value).unwrap() }

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
        let a = identifier("A"); let b = identifier("B"); let c = identifier("C");
        for node in [&a, &b, &c] { graph.add_node((*node).clone()).unwrap(); }
        graph.add_edge(&a, &b).unwrap(); graph.add_edge(&a, &c).unwrap(); graph.add_edge(&b, &c).unwrap();
        let incoming: Vec<(&str, &str)> = graph.incoming(&c).map(|edge| (edge.source().as_str(), edge.target().as_str())).collect();
        let outgoing: Vec<(&str, &str)> = graph.outgoing(&a).map(|edge| (edge.source().as_str(), edge.target().as_str())).collect();
        assert_eq!(incoming, vec![("A", "C"), ("B", "C")]);
        assert_eq!(outgoing, vec![("A", "B"), ("A", "C")]);
    }
}