//! Deterministic structural relationship graph for HELQIX Reality.
//!
//! `RealityGraph` stores relationships between engineering component
//! identities. It does not own `Component` values; component ownership remains
//! with the `Reality` aggregate.
//!
//! This module is intentionally limited to structural graph semantics.
//! Physics, simulation state, execution, telemetry, rendering, AI,
//! optimization, and causal analysis belong to later HELQIX layers.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

/// A directed structural relationship between two component identities.
///
/// An edge:
///
/// `A -> B`
///
/// is distinct from:
///
/// `B -> A`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RealityEdge {
    source: Identifier,
    target: Identifier,
}

impl RealityEdge {
    /// Creates a directed edge from `source` to `target`.
    ///
    /// This constructor only creates the edge value. Graph-level validation
    /// such as node existence and self-loop rejection is performed by
    /// [`RealityGraph::add_edge`].
    #[must_use]
    pub fn new(source: Identifier, target: Identifier) -> Self {
        Self { source, target }
    }

    /// Returns the source component identity.
    #[must_use]
    pub fn source(&self) -> &Identifier {
        &self.source
    }

    /// Returns the target component identity.
    #[must_use]
    pub fn target(&self) -> &Identifier {
        &self.target
    }
}

/// Errors produced by structural operations on [`RealityGraph`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealityGraphError {
    /// A node with the same identity already exists.
    DuplicateNode {
        /// Identity of the duplicated node.
        name: String,
    },

    /// An identical directed edge already exists.
    DuplicateEdge {
        /// Source identity of the duplicated edge.
        source: String,

        /// Target identity of the duplicated edge.
        target: String,
    },

    /// The source node does not exist.
    MissingSourceNode {
        /// Missing source identity.
        name: String,
    },

    /// The target node does not exist.
    MissingTargetNode {
        /// Missing target identity.
        name: String,
    },

    /// An edge attempts to connect a node to itself.
    SelfLoop {
        /// Identity of the self-referencing node.
        name: String,
    },
}

impl fmt::Display for RealityGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateNode { name } => {
                write!(f, "node '{name}' already exists in the reality graph")
            }

            Self::DuplicateEdge { source, target } => {
                write!(
                    f,
                    "edge '{source} -> {target}' already exists in the reality graph"
                )
            }

            Self::MissingSourceNode { name } => {
                write!(
                    f,
                    "source node '{name}' does not exist in the reality graph"
                )
            }

            Self::MissingTargetNode { name } => {
                write!(
                    f,
                    "target node '{name}' does not exist in the reality graph"
                )
            }

            Self::SelfLoop { name } => {
                write!(
                    f,
                    "self-loop on node '{name}' is not allowed"
                )
            }
        }
    }
}

impl Error for RealityGraphError {}

/// Deterministic directed graph describing structural relationships between
/// engineering component identities.
///
/// `RealityGraph` does not own [`Component`](helqix_core::component::Component)
/// values. It stores only their [`Identifier`] values.
///
/// The owning `Reality` aggregate remains the single source of truth for
/// component objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealityGraph {
    nodes: BTreeSet<Identifier>,
    edges: BTreeSet<RealityEdge>,
}

impl RealityGraph {
    /// Creates an empty Reality graph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: BTreeSet::new(),
            edges: BTreeSet::new(),
        }
    }

    /// Returns the number of nodes in the graph.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of directed edges in the graph.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns `true` when the graph contains no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Adds a component identity as a graph node.
    ///
    /// The graph stores only the identity. The actual `Component` remains
    /// owned by `Reality`.
    ///
    /// # Errors
    ///
    /// Returns [`RealityGraphError::DuplicateNode`] when the identity already
    /// exists.
    pub fn add_node(
        &mut self,
        name: Identifier,
    ) -> Result<(), RealityGraphError> {
        if self.nodes.contains(&name) {
            return Err(RealityGraphError::DuplicateNode {
                name: name.as_str().to_owned(),
            });
        }

        self.nodes.insert(name);

        Ok(())
    }

    /// Returns `true` when the graph contains the supplied node identity.
    #[must_use]
    pub fn contains_node(
        &self,
        name: &Identifier,
    ) -> bool {
        self.nodes.contains(name)
    }

    /// Returns the stored node identity.
    ///
    /// The returned reference points to the identity stored inside the graph.
    #[must_use]
    pub fn node(
        &self,
        name: &Identifier,
    ) -> Option<&Identifier> {
        self.nodes.get(name)
    }

    /// Removes a node and all edges incident to it.
    ///
    /// Both incoming and outgoing relationships are removed.
    ///
    /// Returns `true` when the node existed and was removed.
    pub fn remove_node(
        &mut self,
        name: &Identifier,
    ) -> bool {
        if !self.nodes.remove(name) {
            return false;
        }

        self.edges.retain(|edge| {
            edge.source() != name && edge.target() != name
        });

        true
    }

    /// Adds a directed structural edge.
    ///
    /// Both endpoint nodes must already exist.
    ///
    /// The graph never implicitly creates missing nodes.
    ///
    /// # Errors
    ///
    /// Returns:
    ///
    /// - [`RealityGraphError::SelfLoop`] for `A -> A`
    /// - [`RealityGraphError::MissingSourceNode`] when the source is absent
    /// - [`RealityGraphError::MissingTargetNode`] when the target is absent
    /// - [`RealityGraphError::DuplicateEdge`] when the directed edge already
    ///   exists
    pub fn add_edge(
        &mut self,
        source: &Identifier,
        target: &Identifier,
    ) -> Result<(), RealityGraphError> {
        // Self-loop validation is intentionally performed first so A -> A
        // has deterministic self-loop semantics even when A is not registered.
        if source == target {
            return Err(RealityGraphError::SelfLoop {
                name: source.as_str().to_owned(),
            });
        }

        if !self.nodes.contains(source) {
            return Err(RealityGraphError::MissingSourceNode {
                name: source.as_str().to_owned(),
            });
        }

        if !self.nodes.contains(target) {
            return Err(RealityGraphError::MissingTargetNode {
                name: target.as_str().to_owned(),
            });
        }

        let edge = RealityEdge::new(
            source.clone(),
            target.clone(),
        );

        if self.edges.contains(&edge) {
            return Err(RealityGraphError::DuplicateEdge {
                source: source.as_str().to_owned(),
                target: target.as_str().to_owned(),
            });
        }

        self.edges.insert(edge);

        Ok(())
    }

    /// Returns `true` when the exact directed edge exists.
    #[must_use]
    pub fn contains_edge(
        &self,
        source: &Identifier,
        target: &Identifier,
    ) -> bool {
        let edge = RealityEdge::new(
            source.clone(),
            target.clone(),
        );

        self.edges.contains(&edge)
    }

    /// Returns the exact directed edge when it exists.
    #[must_use]
    pub fn edge(
        &self,
        source: &Identifier,
        target: &Identifier,
    ) -> Option<&RealityEdge> {
        let edge = RealityEdge::new(
            source.clone(),
            target.clone(),
        );

        self.edges.get(&edge)
    }

    /// Removes the exact directed edge.
    ///
    /// `A -> B` does not remove `B -> A`.
    ///
    /// Returns `true` when the edge existed.
    pub fn remove_edge(
        &mut self,
        source: &Identifier,
        target: &Identifier,
    ) -> bool {
        let edge = RealityEdge::new(
            source.clone(),
            target.clone(),
        );

        self.edges.remove(&edge)
    }

    /// Iterates over all nodes in deterministic identifier order.
    pub fn nodes(
        &self,
    ) -> impl Iterator<Item = &Identifier> {
        self.nodes.iter()
    }

    /// Iterates over all edges in deterministic edge order.
    pub fn edges(
        &self,
    ) -> impl Iterator<Item = &RealityEdge> {
        self.edges.iter()
    }

    /// Iterates over all outgoing edges from `source`.
    ///
    /// The returned edges are deterministic because the underlying collection
    /// is a [`BTreeSet`].
    pub fn outgoing<'a>(
        &'a self,
        source: &'a Identifier,
    ) -> impl Iterator<Item = &'a RealityEdge> + 'a {
        self.edges
            .iter()
            .filter(move |edge| edge.source() == source)
    }

    /// Iterates over all incoming edges to `target`.
    ///
    /// The returned edges are deterministic because the underlying collection
    /// is a [`BTreeSet`].
    pub fn incoming<'a>(
        &'a self,
        target: &'a Identifier,
    ) -> impl Iterator<Item = &'a RealityEdge> + 'a {
        self.edges
            .iter()
            .filter(move |edge| edge.target() == target)
    }

    /// Validates the structural integrity of the graph.
    ///
    /// The validation checks that:
    ///
    /// 1. every edge source exists as a node;
    /// 2. every edge target exists as a node;
    /// 3. no self-loop exists;
    /// 4. the graph's ordered sets contain no duplicate values.
    ///
    /// The last condition is structurally guaranteed by `BTreeSet`, but the
    /// graph remains explicitly defined around these invariants.
    ///
    /// This method performs no physics, simulation, topology, or engineering
    /// equation validation.
    pub fn validate(&self) -> Result<(), RealityGraphError> {
        for edge in &self.edges {
            if edge.source() == edge.target() {
                return Err(RealityGraphError::SelfLoop {
                    name: edge.source().as_str().to_owned(),
                });
            }

            if !self.nodes.contains(edge.source()) {
                return Err(RealityGraphError::MissingSourceNode {
                    name: edge.source().as_str().to_owned(),
                });
            }

            if !self.nodes.contains(edge.target()) {
                return Err(RealityGraphError::MissingTargetNode {
                    name: edge.target().as_str().to_owned(),
                });
            }
        }

        Ok(())
    }
}

impl Default for RealityGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identifier(value: &str) -> Identifier {
        Identifier::parse(value)
            .expect("test identifier must be valid")
    }

    #[test]
    fn new_graph_is_empty() {
        let graph = RealityGraph::new();

        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.is_empty());
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn add_node_registers_identity() {
        let mut graph = RealityGraph::new();

        graph
            .add_node(identifier("Motor"))
            .unwrap();

        assert_eq!(graph.node_count(), 1);
        assert!(!graph.is_empty());

        assert!(
            graph.contains_node(&identifier("Motor"))
        );

        assert_eq!(
            graph.node(&identifier("Motor")),
            Some(&identifier("Motor"))
        );
    }

    #[test]
    fn duplicate_node_is_rejected() {
        let mut graph = RealityGraph::new();

        graph
            .add_node(identifier("Motor"))
            .unwrap();

        let result =
            graph.add_node(identifier("Motor"));

        assert_eq!(
            result,
            Err(RealityGraphError::DuplicateNode {
                name: "Motor".to_owned(),
            })
        );

        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn add_edge_requires_existing_source() {
        let mut graph = RealityGraph::new();

        graph
            .add_node(identifier("Motor"))
            .unwrap();

        let result = graph.add_edge(
            &identifier("Unknown"),
            &identifier("Motor"),
        );

        assert_eq!(
            result,
            Err(RealityGraphError::MissingSourceNode {
                name: "Unknown".to_owned(),
            })
        );

        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn add_edge_requires_existing_target() {
        let mut graph = RealityGraph::new();

        graph
            .add_node(identifier("Motor"))
            .unwrap();

        let result = graph.add_edge(
            &identifier("Motor"),
            &identifier("Unknown"),
        );

        assert_eq!(
            result,
            Err(RealityGraphError::MissingTargetNode {
                name: "Unknown".to_owned(),
            })
        );

        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn self_loop_is_rejected() {
        let mut graph = RealityGraph::new();

        graph
            .add_node(identifier("Motor"))
            .unwrap();

        let result = graph.add_edge(
            &identifier("Motor"),
            &identifier("Motor"),
        );

        assert_eq!(
            result,
            Err(RealityGraphError::SelfLoop {
                name: "Motor".to_owned(),
            })
        );

        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn self_loop_is_rejected_even_when_node_is_missing() {
        let mut graph = RealityGraph::new();

        let result = graph.add_edge(
            &identifier("Unknown"),
            &identifier("Unknown"),
        );

        assert_eq!(
            result,
            Err(RealityGraphError::SelfLoop {
                name: "Unknown".to_owned(),
            })
        );
    }

    #[test]
    fn directed_edge_is_registered() {
        let mut graph = RealityGraph::new();

        graph
            .add_node(identifier("Controller"))
            .unwrap();

        graph
            .add_node(identifier("Motor"))
            .unwrap();

        graph
            .add_edge(
                &identifier("Controller"),
                &identifier("Motor"),
            )
            .unwrap();

        assert_eq!(graph.edge_count(), 1);

        assert!(
            graph.contains_edge(
                &identifier("Controller"),
                &identifier("Motor"),
            )
        );

        assert!(
            !graph.contains_edge(
                &identifier("Motor"),
                &identifier("Controller"),
            )
        );

        let edge = graph
            .edge(
                &identifier("Controller"),
                &identifier("Motor"),
            )
            .expect("edge must exist");

        assert_eq!(
            edge.source(),
            &identifier("Controller")
        );

        assert_eq!(
            edge.target(),
            &identifier("Motor")
        );
    }

    #[test]
    fn duplicate_edge_is_rejected() {
        let mut graph = RealityGraph::new();

        graph
            .add_node(identifier("A"))
            .unwrap();

        graph
            .add_node(identifier("B"))
            .unwrap();

        graph
            .add_edge(
                &identifier("A"),
                &identifier("B"),
            )
            .unwrap();

        let result = graph.add_edge(
            &identifier("A"),
            &identifier("B"),
        );

        assert_eq!(
            result,
            Err(RealityGraphError::DuplicateEdge {
                source: "A".to_owned(),
                target: "B".to_owned(),
            })
        );

        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn reverse_direction_is_distinct() {
        let mut graph = RealityGraph::new();

        graph
            .add_node(identifier("A"))
            .unwrap();

        graph
            .add_node(identifier("B"))
            .unwrap();

        graph
            .add_edge(
                &identifier("A"),
                &identifier("B"),
            )
            .unwrap();

        graph
            .add_edge(
                &identifier("B"),
                &identifier("A"),
            )
            .unwrap();

        assert_eq!(graph.edge_count(), 2);

        assert!(
            graph.contains_edge(
                &identifier("A"),
                &identifier("B"),
            )
        );

        assert!(
            graph.contains_edge(
                &identifier("B"),
                &identifier("A"),
            )
        );
    }

    #[test]
    fn remove_edge_removes_exact_direction() {
        let mut graph = RealityGraph::new();

        graph
            .add_node(identifier("A"))
            .unwrap();

        graph
            .add_node(identifier("B"))
            .unwrap();

        graph
            .add_edge(
                &identifier("A"),
                &identifier("B"),
            )
            .unwrap();

        graph
            .add_edge(
                &identifier("B"),
                &identifier("A"),
            )
            .unwrap();

        assert!(
            graph.remove_edge(
                &identifier("A"),
                &identifier("B"),
            )
        );

        assert!(
            !graph.contains_edge(
                &identifier("A"),
                &identifier("B"),
            )
        );

        assert!(
            graph.contains_edge(
                &identifier("B"),
                &identifier("A"),
            )
        );

        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn remove_missing_edge_returns_false() {
        let mut graph = RealityGraph::new();

        graph
            .add_node(identifier("A"))
            .unwrap();

        graph
            .add_node(identifier("B"))
            .unwrap();

        assert!(
            !graph.remove_edge(
                &identifier("A"),
                &identifier("B"),
            )
        );

        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn remove_node_removes_incident_edges() {
        let mut graph = RealityGraph::new();

        for name in ["A", "B", "C"] {
            graph
                .add_node(identifier(name))
                .unwrap();
        }

        graph
            .add_edge(
                &identifier("A"),
                &identifier("B"),
            )
            .unwrap();

        graph
            .add_edge(
                &identifier("B"),
                &identifier("C"),
            )
            .unwrap();

        graph
            .add_edge(
                &identifier("C"),
                &identifier("B"),
            )
            .unwrap();

        assert!(graph.remove_node(&identifier("B")));

        assert!(
            !graph.contains_node(&identifier("B"))
        );

        assert!(
            !graph.contains_edge(
                &identifier("A"),
                &identifier("B"),
            )
        );

        assert!(
            !graph.contains_edge(
                &identifier("B"),
                &identifier("C"),
            )
        );

        assert!(
            !graph.contains_edge(
                &identifier("C"),
                &identifier("B"),
            )
        );

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn remove_missing_node_returns_false() {
        let mut graph = RealityGraph::new();

        graph
            .add_node(identifier("A"))
            .unwrap();

        assert!(
            !graph.remove_node(&identifier("Unknown"))
        );

        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn deterministic_node_iteration() {
        let mut graph = RealityGraph::new();

        for name in ["Zeta", "Alpha", "Motor", "Battery"] {
            graph
                .add_node(identifier(name))
                .unwrap();
        }

        let order: Vec<&str> = graph
            .nodes()
            .map(Identifier::as_str)
            .collect();

        assert_eq!(
            order,
            vec![
                "Alpha",
                "Battery",
                "Motor",
                "Zeta"
            ]
        );
    }

    #[test]
    fn deterministic_edge_iteration() {
        let mut graph = RealityGraph::new();

        for name in ["A", "B", "C", "D"] {
            graph
                .add_node(identifier(name))
                .unwrap();
        }

        graph
            .add_edge(
                &identifier("C"),
                &identifier("D"),
            )
            .unwrap();

        graph
            .add_edge(
                &identifier("A"),
                &identifier("B"),
            )
            .unwrap();

        graph
            .add_edge(
                &identifier("B"),
                &identifier("C"),
            )
            .unwrap();

        graph
            .add_edge(
                &identifier("A"),
                &identifier("C"),
            )
            .unwrap();

        let order: Vec<(&str, &str)> = graph
            .edges()
            .map(|edge| {
                (
                    edge.source().as_str(),
                    edge.target().as_str(),
                )
            })
            .collect();

        assert_eq!(
            order,
            vec![
                ("A", "B"),
                ("A", "C"),
                ("B", "C"),
                ("C", "D"),
            ]
        );
    }

    #[test]
    fn insertion_order_does_not_change_node_iteration() {
        let mut first = RealityGraph::new();
        let mut second = RealityGraph::new();

        for name in ["Alpha", "Battery", "Motor", "Zeta"] {
            first
                .add_node(identifier(name))
                .unwrap();
        }

        for name in ["Zeta", "Motor", "Battery", "Alpha"] {
            second
                .add_node(identifier(name))
                .unwrap();
        }

        let first_order: Vec<&str> = first
            .nodes()
            .map(Identifier::as_str)
            .collect();

        let second_order: Vec<&str> = second
            .nodes()
            .map(Identifier::as_str)
            .collect();

        assert_eq!(first_order, second_order);
    }

    #[test]
    fn incoming_edges_are_correct() {
        let mut graph = RealityGraph::new();

        for name in ["A", "B", "C", "D"] {
            graph
                .add_node(identifier(name))
                .unwrap();
        }

        graph
            .add_edge(
                &identifier("A"),
                &identifier("C"),
            )
            .unwrap();

        graph
            .add_edge(
                &identifier("B"),
                &identifier("C"),
            )
            .unwrap();

        graph
            .add_edge(
                &identifier("C"),
                &identifier("D"),
            )
            .unwrap();

        let node_c = identifier("C");
        let incoming: Vec<(&str, &str)> = graph
            .incoming(&node_c)
            .map(|edge| {
                (
                    edge.source().as_str(),
                    edge.target().as_str(),
                )
            })
            .collect();

        assert_eq!(
            incoming,
            vec![
                ("A", "C"),
                ("B", "C"),
            ]
        );
    }

    #[test]
    fn outgoing_edges_are_correct() {
        let mut graph = RealityGraph::new();

        for name in ["A", "B", "C", "D"] {
            graph
                .add_node(identifier(name))
                .unwrap();
        }

        graph
            .add_edge(
                &identifier("A"),
                &identifier("B"),
            )
            .unwrap();

        graph
            .add_edge(
                &identifier("A"),
                &identifier("C"),
            )
            .unwrap();

        graph
            .add_edge(
                &identifier("B"),
                &identifier("D"),
            )
            .unwrap();

        let node_a = identifier("A");
        let outgoing: Vec<(&str, &str)> = graph
            .outgoing(&node_a)
            .map(|edge| {
                (
                    edge.source().as_str(),
                    edge.target().as_str(),
                )
            })
            .collect();

        assert_eq!(
            outgoing,
            vec![
                ("A", "B"),
                ("A", "C"),
            ]
        );
    }

    #[test]
    fn incoming_and_outgoing_for_missing_node_are_empty() {
        let mut graph = RealityGraph::new();

        graph
            .add_node(identifier("A"))
            .unwrap();

        assert_eq!(
            graph
                .incoming(&identifier("Unknown"))
                .count(),
            0
        );

        assert_eq!(
            graph
                .outgoing(&identifier("Unknown"))
                .count(),
            0
        );
    }

    #[test]
    fn graph_validation_accepts_valid_graph() {
        let mut graph = RealityGraph::new();

        for name in ["Battery", "Controller", "Motor"] {
            graph
                .add_node(identifier(name))
                .unwrap();
        }

        graph
            .add_edge(
                &identifier("Battery"),
                &identifier("Controller"),
            )
            .unwrap();

        graph
            .add_edge(
                &identifier("Controller"),
                &identifier("Motor"),
            )
            .unwrap();

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn removed_node_can_be_readded() {
        let mut graph = RealityGraph::new();

        graph
            .add_node(identifier("Motor"))
            .unwrap();

        assert!(
            graph.remove_node(&identifier("Motor"))
        );

        assert!(
            graph
                .add_node(identifier("Motor"))
                .is_ok()
        );

        assert_eq!(graph.node_count(), 1);
        assert!(
            graph.contains_node(&identifier("Motor"))
        );
    }

    #[test]
    fn removed_edge_can_be_readded() {
        let mut graph = RealityGraph::new();

        graph
            .add_node(identifier("A"))
            .unwrap();

        graph
            .add_node(identifier("B"))
            .unwrap();

        graph
            .add_edge(
                &identifier("A"),
                &identifier("B"),
            )
            .unwrap();

        assert!(
            graph.remove_edge(
                &identifier("A"),
                &identifier("B"),
            )
        );

        assert!(
            graph
                .add_edge(
                    &identifier("A"),
                    &identifier("B"),
                )
                .is_ok()
        );

        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn graph_remains_valid_after_membership_changes() {
        let mut graph = RealityGraph::new();

        for name in ["A", "B", "C"] {
            graph
                .add_node(identifier(name))
                .unwrap();
        }

        graph
            .add_edge(
                &identifier("A"),
                &identifier("B"),
            )
            .unwrap();

        graph
            .add_edge(
                &identifier("B"),
                &identifier("C"),
            )
            .unwrap();

        assert!(graph.validate().is_ok());

        graph
            .remove_edge(
                &identifier("A"),
                &identifier("B"),
            );

        assert!(graph.validate().is_ok());

        graph.remove_node(&identifier("B"));

        assert!(graph.validate().is_ok());
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 0);
    }
}