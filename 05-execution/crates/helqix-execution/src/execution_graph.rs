//! Execution dependency graph for HELQIX Layer 05 — Execution.
//!
//! An [`ExecutionGraph`] models **execution ordering constraints**
//! between actions: "action `A` must complete before action `B` can be
//! considered eligible to run."
//!
//! The graph is a **pure structure**. It does not execute, does not
//! own state, does not own time, does not simulate, does not schedule.
//! It answers one question, deterministically:
//!
//! > "Given which actions have already completed, which actions have
//! >  had all their execution dependencies satisfied?"
//!
//! # Responsibility boundary
//!
//! `execution_graph.rs` owns:
//!
//! * the set of node identities ([`ActionId`]s),
//! * the set of directed edges (predecessor → successor),
//! * the invariant that the graph is **acyclic**,
//! * deterministic structural queries: predecessors, successors, roots,
//!   leaves, readiness, topological order.
//!
//! It explicitly does **not** own:
//!
//! * action payloads ([`crate::action`]),
//! * scheduling decisions ([`crate::scheduler`]),
//! * execution ([`crate::executor`] / [`crate::runtime`]),
//! * state transitions ([`crate::transition`] and Layer 04),
//! * engineering dependencies (Layer 03),
//! * physical simulation (Layer 06),
//! * logical time (Layer 07).
//!
//! # Engineering vs. execution dependency
//!
//! `03-engineering/dependency_graph.rs` (in the HELQIX repository)
//! describes **engineering** relationships between components and
//! models: "component `X` depends on model `Y`."
//!
//! An `ExecutionGraph` describes **execution** relationships between
//! actions: "action `A` must occur before action `B`."
//!
//! These are distinct. The graph does not merge them and does not
//! translate between them. Any translation, if the repository later
//! requires it, belongs in an adapter outside this file.
//!
//! # Determinism
//!
//! All graph storage uses `BTreeSet` / `BTreeMap`. Iteration is always
//! ascending by [`ActionId`]. Same node set + same edge set ⇒ same
//! `ExecutionGraph`, byte-for-byte, in every run, process, and
//! platform. No wall clock, no randomness, no thread scheduling, no
//! hash iteration, no global state, no interior mutability, no
//! `unsafe`.
//!
//! # Acyclicity
//!
//! Execution dependencies must form a DAG. Adding an edge that would
//! create a cycle is rejected with
//! [`ExecutionGraphError::DependencyCycle`]. The pending edge is not
//! inserted; the graph is unchanged.
//!
//! # Node identity
//!
//! Nodes are [`ActionId`] values, not full actions. The scheduler owns
//! the actions; the graph only reasons about their identities. This
//! keeps the graph monomorphic (no generic `P` parameter) and
//! independent of any payload type.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use crate::action::ActionId;

// ============================================================================
// ExecutionGraphError
// ============================================================================

/// Errors raised by [`ExecutionGraph`] mutation operations.
///
/// Every variant is a normal, expected condition the caller must
/// handle explicitly — never a panic. A failed operation leaves the
/// graph unchanged.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ExecutionGraphError {
    /// [`ExecutionGraph::add_node`] was called with an `ActionId` that
    /// is already present in the graph.
    DuplicateNode { id: ActionId },

    /// A query or mutation referenced an `ActionId` that is not a node
    /// of the graph.
    UnknownNode { id: ActionId },

    /// [`ExecutionGraph::add_dependency`] was called with `from == to`.
    SelfDependency { id: ActionId },

    /// [`ExecutionGraph::add_dependency`] was called for an edge that
    /// is already present.
    DuplicateEdge { from: ActionId, to: ActionId },

    /// [`ExecutionGraph::remove_dependency`] was called for an edge
    /// that is not present.
    EdgeNotFound { from: ActionId, to: ActionId },

    /// [`ExecutionGraph::add_dependency`] was called with an edge that
    /// would introduce a cycle (i.e. the successor already reaches the
    /// predecessor via existing edges).
    DependencyCycle { from: ActionId, to: ActionId },
}

impl fmt::Display for ExecutionGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateNode { id } => {
                write!(f, "node {:?} is already present", id)
            }
            Self::UnknownNode { id } => {
                write!(f, "node {:?} is not present", id)
            }
            Self::SelfDependency { id } => {
                write!(f, "node {:?} cannot depend on itself", id)
            }
            Self::DuplicateEdge { from, to } => write!(
                f,
                "edge {:?} -> {:?} is already present",
                from, to
            ),
            Self::EdgeNotFound { from, to } => write!(
                f,
                "edge {:?} -> {:?} is not present",
                from, to
            ),
            Self::DependencyCycle { from, to } => write!(
                f,
                "edge {:?} -> {:?} would create a dependency cycle",
                from, to
            ),
        }
    }
}

impl std::error::Error for ExecutionGraphError {}

// ============================================================================
// ExecutionGraph
// ============================================================================

/// A deterministic DAG of execution ordering constraints between
/// [`ActionId`]s.
///
/// See the module documentation for the full semantic model.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct ExecutionGraph {
    /// Every node in the graph.
    nodes: BTreeSet<ActionId>,
    /// Forward adjacency: `successors[a]` = set of `b` such that
    /// `a → b` is an edge. Absent keys mean "no successors".
    successors: BTreeMap<ActionId, BTreeSet<ActionId>>,
    /// Backward adjacency: `predecessors[b]` = set of `a` such that
    /// `a → b` is an edge. Absent keys mean "no predecessors".
    predecessors: BTreeMap<ActionId, BTreeSet<ActionId>>,
}

impl ExecutionGraph {
    /// Create an empty graph.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            nodes: BTreeSet::new(),
            successors: BTreeMap::new(),
            predecessors: BTreeMap::new(),
        }
    }

    /// Is the graph empty (no nodes)?
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Number of nodes.
    #[inline]
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges.
    #[inline]
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.successors.values().map(BTreeSet::len).sum()
    }

    /// Is `id` a node of the graph?
    #[inline]
    #[must_use]
    pub fn contains_node(&self, id: ActionId) -> bool {
        self.nodes.contains(&id)
    }

    /// Is there an edge `from → to`?
    ///
    /// Returns [`ExecutionGraphError::UnknownNode`] if `from` is not a
    /// node. (If `from` exists but `to` does not, the answer is simply
    /// `false` — a nonexistent node cannot be the head of an edge.)
    pub fn has_edge(
        &self,
        from: ActionId,
        to: ActionId,
    ) -> Result<bool, ExecutionGraphError> {
        if !self.nodes.contains(&from) {
            return Err(ExecutionGraphError::UnknownNode { id: from });
        }
        Ok(self
            .successors
            .get(&from)
            .map_or(false, |s| s.contains(&to)))
    }

    /// Iterate over all nodes in ascending [`ActionId`] order.
    #[inline]
    pub fn nodes(&self) -> impl Iterator<Item = ActionId> + '_ {
        self.nodes.iter().copied()
    }

    /// Iterate over all edges as `(from, to)` pairs.
    ///
    /// Order: primary ascending `from`, secondary ascending `to`.
    /// Deterministic by construction.
    #[inline]
    pub fn edges(&self) -> impl Iterator<Item = (ActionId, ActionId)> + '_ {
        self.successors
            .iter()
            .flat_map(|(&from, tos)| tos.iter().map(move |&to| (from, to)))
    }

    // ------------------------------------------------------------------
    // Mutation
    // ------------------------------------------------------------------

    /// Add a node.
    ///
    /// Fails with [`ExecutionGraphError::DuplicateNode`] if `id` is
    /// already present. The graph is unchanged on failure.
    pub fn add_node(&mut self, id: ActionId) -> Result<(), ExecutionGraphError> {
        if !self.nodes.insert(id) {
            return Err(ExecutionGraphError::DuplicateNode { id });
        }
        Ok(())
    }

    /// Remove a node and all edges incident to it (both incoming and
    /// outgoing).
    ///
    /// Fails with [`ExecutionGraphError::UnknownNode`] if `id` is not
    /// present. The graph is unchanged on failure.
    pub fn remove_node(&mut self, id: ActionId) -> Result<(), ExecutionGraphError> {
        if !self.nodes.remove(&id) {
            return Err(ExecutionGraphError::UnknownNode { id });
        }
        // Outgoing edges: id → s for each s in successors[id].
        if let Some(succs) = self.successors.remove(&id) {
            for s in succs {
                if let Some(pre) = self.predecessors.get_mut(&s) {
                    pre.remove(&id);
                    if pre.is_empty() {
                        self.predecessors.remove(&s);
                    }
                }
            }
        }
        // Incoming edges: p → id for each p in predecessors[id].
        if let Some(preds) = self.predecessors.remove(&id) {
            for p in preds {
                if let Some(succ) = self.successors.get_mut(&p) {
                    succ.remove(&id);
                    if succ.is_empty() {
                        self.successors.remove(&p);
                    }
                }
            }
        }
        Ok(())
    }

    /// Add an execution dependency edge `from → to`.
    ///
    /// Semantics: `from` must complete before `to` becomes eligible.
    ///
    /// Fails with:
    ///
    /// * [`ExecutionGraphError::UnknownNode`] if `from` or `to` is not
    ///   a node,
    /// * [`ExecutionGraphError::SelfDependency`] if `from == to`,
    /// * [`ExecutionGraphError::DuplicateEdge`] if the edge already
    ///   exists,
    /// * [`ExecutionGraphError::DependencyCycle`] if adding the edge
    ///   would make `from` reachable from `to` (i.e. create a cycle).
    ///
    /// The graph is unchanged on failure.
    pub fn add_dependency(
        &mut self,
        from: ActionId,
        to: ActionId,
    ) -> Result<(), ExecutionGraphError> {
        if from == to {
            return Err(ExecutionGraphError::SelfDependency { id: from });
        }
        if !self.nodes.contains(&from) {
            return Err(ExecutionGraphError::UnknownNode { id: from });
        }
        if !self.nodes.contains(&to) {
            return Err(ExecutionGraphError::UnknownNode { id: to });
        }
        if self
            .successors
            .get(&from)
            .map_or(false, |s| s.contains(&to))
        {
            return Err(ExecutionGraphError::DuplicateEdge { from, to });
        }
        if self.reaches(to, from) {
            return Err(ExecutionGraphError::DependencyCycle { from, to });
        }
        self.successors.entry(from).or_default().insert(to);
        self.predecessors.entry(to).or_default().insert(from);
        Ok(())
    }

    /// Remove the execution dependency edge `from → to`.
    ///
    /// Fails with [`ExecutionGraphError::UnknownNode`] if `from` is not
    /// a node, or [`ExecutionGraphError::EdgeNotFound`] if the edge is
    /// not present. The graph is unchanged on failure.
    pub fn remove_dependency(
        &mut self,
        from: ActionId,
        to: ActionId,
    ) -> Result<(), ExecutionGraphError> {
        if !self.nodes.contains(&from) {
            return Err(ExecutionGraphError::UnknownNode { id: from });
        }
        let present = self
            .successors
            .get(&from)
            .map_or(false, |s| s.contains(&to));
        if !present {
            return Err(ExecutionGraphError::EdgeNotFound { from, to });
        }
        if let Some(succ) = self.successors.get_mut(&from) {
            succ.remove(&to);
            if succ.is_empty() {
                self.successors.remove(&from);
            }
        }
        if let Some(pre) = self.predecessors.get_mut(&to) {
            pre.remove(&from);
            if pre.is_empty() {
                self.predecessors.remove(&to);
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Structural queries
    // ------------------------------------------------------------------

    /// Predecessors of `id` (nodes that must complete before `id`),
    /// in ascending [`ActionId`] order.
    ///
    /// Fails with [`ExecutionGraphError::UnknownNode`] if `id` is not a
    /// node.
    pub fn predecessors(
        &self,
        id: ActionId,
    ) -> Result<impl Iterator<Item = ActionId> + '_, ExecutionGraphError> {
        if !self.nodes.contains(&id) {
            return Err(ExecutionGraphError::UnknownNode { id });
        }
        Ok(self
            .predecessors
            .get(&id)
            .into_iter()
            .flatten()
            .copied())
    }

    /// Successors of `id` (nodes that wait for `id`), in ascending
    /// [`ActionId`] order.
    ///
    /// Fails with [`ExecutionGraphError::UnknownNode`] if `id` is not a
    /// node.
    pub fn successors(
        &self,
        id: ActionId,
    ) -> Result<impl Iterator<Item = ActionId> + '_, ExecutionGraphError> {
        if !self.nodes.contains(&id) {
            return Err(ExecutionGraphError::UnknownNode { id });
        }
        Ok(self.successors.get(&id).into_iter().flatten().copied())
    }

    /// Root nodes — nodes with no predecessors.
    ///
    /// Order: ascending [`ActionId`]. Deterministic by construction.
    #[inline]
    pub fn roots(&self) -> impl Iterator<Item = ActionId> + '_ {
        let preds = &self.predecessors;
        self.nodes
            .iter()
            .copied()
            .filter(move |id| preds.get(id).map_or(true, BTreeSet::is_empty))
    }

    /// Leaf nodes — nodes with no successors.
    ///
    /// Order: ascending [`ActionId`]. Deterministic by construction.
    #[inline]
    pub fn leaves(&self) -> impl Iterator<Item = ActionId> + '_ {
        let succs = &self.successors;
        self.nodes
            .iter()
            .copied()
            .filter(move |id| succs.get(id).map_or(true, BTreeSet::is_empty))
    }

    // ------------------------------------------------------------------
    // Readiness
    // ------------------------------------------------------------------

    /// Is `id` ready given a completion predicate `is_completed`?
    ///
    /// A node is ready iff **every** predecessor satisfies
    /// `is_completed`. A node with no predecessors is trivially ready.
    ///
    /// The predicate is supplied by the caller — the graph does not own
    /// execution state. A caller typically closes over a set of
    /// completed [`ActionId`]s maintained by the runtime.
    ///
    /// Fails with [`ExecutionGraphError::UnknownNode`] if `id` is not a
    /// node.
    pub fn is_ready<F>(&self, id: ActionId, is_completed: F) -> Result<bool, ExecutionGraphError>
    where
        F: Fn(ActionId) -> bool,
    {
        if !self.nodes.contains(&id) {
            return Err(ExecutionGraphError::UnknownNode { id });
        }
        Ok(self
            .predecessors
            .get(&id)
            .map_or(true, |pre| pre.iter().all(|p| is_completed(*p))))
    }

    /// All nodes that are ready given a completion predicate, in
    /// ascending [`ActionId`] order.
    ///
    /// Deterministic by construction — the underlying `nodes` set is
    /// a `BTreeSet`.
    #[inline]
    pub fn ready_nodes<'a, F>(&'a self, is_completed: F) -> impl Iterator<Item = ActionId> + 'a
    where
        F: Fn(ActionId) -> bool + 'a,
    {
        let preds = &self.predecessors;
        self.nodes.iter().copied().filter(move |id| {
            preds
                .get(id)
                .map_or(true, |pre| pre.iter().all(|p| is_completed(*p)))
        })
    }

    // ------------------------------------------------------------------
    // Topological order
    // ------------------------------------------------------------------

    /// A deterministic topological ordering of all nodes.
    ///
    /// Every edge `from → to` satisfies
    /// `position(from) < position(to)`. When multiple nodes are
    /// simultaneously eligible, the one with the lower [`ActionId`] is
    /// emitted first. The result is therefore stable across runs,
    /// processes, and platforms.
    ///
    /// The graph is acyclic by construction, so this always returns
    /// exactly `node_count()` entries.
    pub fn topological_order(&self) -> Vec<ActionId> {
        // Kahn's algorithm with a BTreeSet frontier → deterministic.
        let mut in_degree: BTreeMap<ActionId, usize> = self
            .nodes
            .iter()
            .map(|&id| {
                let d = self.predecessors.get(&id).map_or(0, BTreeSet::len);
                (id, d)
            })
            .collect();

        let mut frontier: BTreeSet<ActionId> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut out = Vec::with_capacity(self.nodes.len());
        while let Some(id) = frontier.pop_first() {
            out.push(id);
            if let Some(succs) = self.successors.get(&id) {
                for &s in succs {
                    if let Some(d) = in_degree.get_mut(&s) {
                        *d -= 1;
                        if *d == 0 {
                            frontier.insert(s);
                        }
                    }
                }
            }
        }
        out
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Does `start` reach `target` via zero or more successor edges?
    ///
    /// Used to detect cycle-creating edge insertions. O(V + E).
    fn reaches(&self, start: ActionId, target: ActionId) -> bool {
        if start == target {
            return true;
        }
        let mut stack: Vec<ActionId> = vec![start];
        let mut visited: BTreeSet<ActionId> = BTreeSet::new();
        visited.insert(start);
        while let Some(current) = stack.pop() {
            if let Some(succs) = self.successors.get(&current) {
                for &s in succs {
                    if s == target {
                        return true;
                    }
                    if visited.insert(s) {
                        stack.push(s);
                    }
                }
            }
        }
        false
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn id(n: u64) -> ActionId {
        ActionId::from_raw(n)
    }

    /// The worked example from the Layer 05 spec:
    ///
    /// ```text
    /// A ──┐
    ///     ├──> C ───> D
    /// B ──┘
    /// ```
    fn worked_graph() -> ExecutionGraph {
        let mut g = ExecutionGraph::new();
        g.add_node(id(1)).unwrap(); // A
        g.add_node(id(2)).unwrap(); // B
        g.add_node(id(3)).unwrap(); // C
        g.add_node(id(4)).unwrap(); // D
        g.add_dependency(id(1), id(3)).unwrap(); // A -> C
        g.add_dependency(id(2), id(3)).unwrap(); // B -> C
        g.add_dependency(id(3), id(4)).unwrap(); // C -> D
        g
    }

    /// Assert that `topological_order()` is a valid topological order
    /// for `g`: every node appears exactly once, and every edge goes
    /// forward.
    fn assert_valid_topological_order(g: &ExecutionGraph) {
        let order = g.topological_order();
        assert_eq!(order.len(), g.node_count(), "topo order must cover every node");
        let positions: BTreeMap<ActionId, usize> = order
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, i))
            .collect();
        assert_eq!(
            positions.len(),
            order.len(),
            "topo order must not contain duplicates"
        );
        for (from, to) in g.edges() {
            let pf = positions[&from];
            let pt = positions[&to];
            assert!(
                pf < pt,
                "edge {:?} -> {:?} went backward in topo order",
                from,
                to
            );
        }
    }

    // ------------------------------------------------------------------
    // 1. Empty graph
    // ------------------------------------------------------------------

    #[test]
    fn empty_graph_is_empty() {
        let g = ExecutionGraph::new();
        assert!(g.is_empty());
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
        assert!(g.nodes().next().is_none());
        assert!(g.edges().next().is_none());
        assert!(g.roots().next().is_none());
        assert!(g.leaves().next().is_none());
        assert_eq!(g.topological_order(), Vec::<ActionId>::new());
    }

    #[test]
    fn default_is_empty() {
        let g: ExecutionGraph = ExecutionGraph::default();
        assert!(g.is_empty());
    }

    // ------------------------------------------------------------------
    // 2. Nodes
    // ------------------------------------------------------------------

    #[test]
    fn add_and_contains_node() {
        let mut g = ExecutionGraph::new();
        g.add_node(id(1)).unwrap();
        assert!(g.contains_node(id(1)));
        assert!(!g.contains_node(id(2)));
        assert_eq!(g.node_count(), 1);
    }

    #[test]
    fn duplicate_node_is_rejected_and_graph_unchanged() {
        let mut g = ExecutionGraph::new();
        g.add_node(id(1)).unwrap();
        let err = g.add_node(id(1)).unwrap_err();
        assert_eq!(err, ExecutionGraphError::DuplicateNode { id: id(1) });
        assert_eq!(g.node_count(), 1);
    }

    #[test]
    fn nodes_iterate_in_ascending_id_order() {
        let mut g = ExecutionGraph::new();
        g.add_node(id(5)).unwrap();
        g.add_node(id(1)).unwrap();
        g.add_node(id(3)).unwrap();
        g.add_node(id(2)).unwrap();
        g.add_node(id(4)).unwrap();
        let order: Vec<u64> = g.nodes().map(|id| id.raw()).collect();
        assert_eq!(order, vec![1, 2, 3, 4, 5]);
    }

    // ------------------------------------------------------------------
    // 3. Edges
    // ------------------------------------------------------------------

    #[test]
    fn add_dependency_creates_edge() {
        let mut g = ExecutionGraph::new();
        g.add_node(id(1)).unwrap();
        g.add_node(id(2)).unwrap();
        g.add_dependency(id(1), id(2)).unwrap();
        assert!(g.has_edge(id(1), id(2)).unwrap());
        assert!(!g.has_edge(id(2), id(1)).unwrap());
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn unknown_dependency_from_is_rejected() {
        let mut g = ExecutionGraph::new();
        g.add_node(id(2)).unwrap();
        let err = g.add_dependency(id(1), id(2)).unwrap_err();
        assert_eq!(err, ExecutionGraphError::UnknownNode { id: id(1) });
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn unknown_dependency_to_is_rejected() {
        let mut g = ExecutionGraph::new();
        g.add_node(id(1)).unwrap();
        let err = g.add_dependency(id(1), id(2)).unwrap_err();
        assert_eq!(err, ExecutionGraphError::UnknownNode { id: id(2) });
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn self_dependency_is_rejected() {
        let mut g = ExecutionGraph::new();
        g.add_node(id(1)).unwrap();
        let err = g.add_dependency(id(1), id(1)).unwrap_err();
        assert_eq!(err, ExecutionGraphError::SelfDependency { id: id(1) });
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn duplicate_edge_is_rejected() {
        let mut g = ExecutionGraph::new();
        g.add_node(id(1)).unwrap();
        g.add_node(id(2)).unwrap();
        g.add_dependency(id(1), id(2)).unwrap();
        let err = g.add_dependency(id(1), id(2)).unwrap_err();
        assert_eq!(
            err,
            ExecutionGraphError::DuplicateEdge {
                from: id(1),
                to: id(2),
            }
        );
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn edges_iterate_in_from_then_to_order() {
        let mut g = ExecutionGraph::new();
        for n in 1..=3u64 {
            g.add_node(id(n)).unwrap();
        }
        // Insert in reverse-of-iteration order.
        g.add_dependency(id(3), id(2)).unwrap();
        g.add_dependency(id(1), id(3)).unwrap();
        g.add_dependency(id(1), id(2)).unwrap();
        let edges: Vec<(u64, u64)> = g.edges().map(|(a, b)| (a.raw(), b.raw())).collect();
        assert_eq!(edges, vec![(1, 2), (1, 3), (3, 2)]);
    }

    // ------------------------------------------------------------------
    // 4. Predecessors / successors
    // ------------------------------------------------------------------

    #[test]
    fn predecessors_and_successors_of_worked_graph() {
        let g = worked_graph();

        let preds_of_c: Vec<u64> = g
            .predecessors(id(3))
            .unwrap()
            .map(|id| id.raw())
            .collect();
        assert_eq!(preds_of_c, vec![1, 2]);

        let succ_of_a: Vec<u64> = g.successors(id(1)).unwrap().map(|id| id.raw()).collect();
        assert_eq!(succ_of_a, vec![3]);

        let succ_of_c: Vec<u64> = g.successors(id(3)).unwrap().map(|id| id.raw()).collect();
        assert_eq!(succ_of_c, vec![4]);

        // A has no predecessors, D has no successors.
        assert!(g.predecessors(id(1)).unwrap().next().is_none());
        assert!(g.successors(id(4)).unwrap().next().is_none());
    }

    #[test]
    fn unknown_node_predecessor_query_errors() {
        let g = ExecutionGraph::new();
        assert_eq!(
            g.predecessors(id(99)).err(),
            Some(ExecutionGraphError::UnknownNode { id: id(99) })
        );
    }

    #[test]
    fn unknown_node_successor_query_errors() {
        let g = ExecutionGraph::new();
        assert_eq!(
            g.successors(id(99)).err(),
            Some(ExecutionGraphError::UnknownNode { id: id(99) })
        );
    }

    #[test]
    fn has_edge_on_unknown_from_errors() {
        let g = ExecutionGraph::new();
        assert_eq!(
            g.has_edge(id(1), id(2)).err(),
            Some(ExecutionGraphError::UnknownNode { id: id(1) })
        );
    }

    // ------------------------------------------------------------------
    // 5. Roots and leaves
    // ------------------------------------------------------------------

    #[test]
    fn roots_and_leaves_of_worked_graph() {
        let g = worked_graph();
        let roots: Vec<u64> = g.roots().map(|id| id.raw()).collect();
        let leaves: Vec<u64> = g.leaves().map(|id| id.raw()).collect();
        assert_eq!(roots, vec![1, 2]); // A, B
        assert_eq!(leaves, vec![4]); // D
    }

    #[test]
    fn independent_nodes_are_both_roots_and_leaves() {
        let mut g = ExecutionGraph::new();
        g.add_node(id(1)).unwrap();
        g.add_node(id(2)).unwrap();
        g.add_node(id(3)).unwrap();
        let roots: Vec<u64> = g.roots().map(|id| id.raw()).collect();
        let leaves: Vec<u64> = g.leaves().map(|id| id.raw()).collect();
        assert_eq!(roots, vec![1, 2, 3]);
        assert_eq!(leaves, vec![1, 2, 3]);
    }

    // ------------------------------------------------------------------
    // 6. Readiness
    // ------------------------------------------------------------------

    #[test]
    fn readiness_depends_on_all_predecessors() {
        let g = worked_graph();

        // Nothing completed.
        assert!(g.is_ready(id(1), |_| false).unwrap()); // A is a root.
        assert!(g.is_ready(id(2), |_| false).unwrap()); // B is a root.
        assert!(!g.is_ready(id(3), |_| false).unwrap()); // C blocked by A, B.
        assert!(!g.is_ready(id(4), |_| false).unwrap()); // D blocked by C.

        // Only A completed.
        assert!(!g.is_ready(id(3), |id| id == crate::action::ActionId::from_raw(1)).unwrap());

        // A and B completed.
        let ab = |id: crate::action::ActionId| id.raw() == 1 || id.raw() == 2;
        assert!(g.is_ready(id(3), ab).unwrap());
        assert!(!g.is_ready(id(4), ab).unwrap());

        // A, B, C completed.
        let abc = |id: crate::action::ActionId| (1..=3).contains(&id.raw());
        assert!(g.is_ready(id(4), abc).unwrap());
    }

    #[test]
    fn is_ready_unknown_node_errors() {
        let g = ExecutionGraph::new();
        assert_eq!(
            g.is_ready(id(99), |_| false).err(),
            Some(ExecutionGraphError::UnknownNode { id: id(99) })
        );
    }

    #[test]
    fn ready_nodes_reflects_completion_set() {
        let g = worked_graph();

        // Nothing done: only the roots are ready.
        let ready: Vec<u64> = g.ready_nodes(|_| false).map(|id| id.raw()).collect();
        assert_eq!(ready, vec![1, 2]);

        // A done: still only roots (C still blocked by B). Completed roots
        // stay ready, as the final assertion below documents.
        let ready: Vec<u64> = g
            .ready_nodes(|id| id.raw() == 1)
            .map(|id| id.raw())
            .collect();
        assert_eq!(ready, vec![1, 2]);

        // A and B done: C becomes ready.
        let ready: Vec<u64> = g
            .ready_nodes(|id| id.raw() == 1 || id.raw() == 2)
            .map(|id| id.raw())
            .collect();
        assert_eq!(ready, vec![1, 2, 3]);

        // Everything done: nothing is "ready" (the graph is
        // statically defined; readiness only depends on predecessors,
        // not on "already completed self").
        let ready: Vec<u64> = g.ready_nodes(|_| true).map(|id| id.raw()).collect();
        assert_eq!(ready, vec![1, 2, 3, 4]);
    }

    // ------------------------------------------------------------------
    // 7. Cycle rejection
    // ------------------------------------------------------------------

    #[test]
    fn direct_back_edge_is_rejected_as_cycle() {
        let mut g = ExecutionGraph::new();
        g.add_node(id(1)).unwrap();
        g.add_node(id(2)).unwrap();
        g.add_dependency(id(1), id(2)).unwrap();
        let err = g.add_dependency(id(2), id(1)).unwrap_err();
        assert_eq!(
            err,
            ExecutionGraphError::DependencyCycle {
                from: id(2),
                to: id(1),
            }
        );
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn longer_cycle_is_detected() {
        // A → B → C → A is a cycle; the last edge must be rejected.
        let mut g = ExecutionGraph::new();
        for n in 1..=3u64 {
            g.add_node(id(n)).unwrap();
        }
        g.add_dependency(id(1), id(2)).unwrap();
        g.add_dependency(id(2), id(3)).unwrap();
        let err = g.add_dependency(id(3), id(1)).unwrap_err();
        assert_eq!(
            err,
            ExecutionGraphError::DependencyCycle {
                from: id(3),
                to: id(1),
            }
        );
        assert_eq!(g.edge_count(), 2);
    }

    #[test]
    fn transitive_shortcut_is_not_a_cycle() {
        // A → B → C, then add A → C. This is a *shortcut*, not a cycle.
        let mut g = ExecutionGraph::new();
        for n in 1..=3u64 {
            g.add_node(id(n)).unwrap();
        }
        g.add_dependency(id(1), id(2)).unwrap();
        g.add_dependency(id(2), id(3)).unwrap();
        g.add_dependency(id(1), id(3)).unwrap();
        assert_eq!(g.edge_count(), 3);
    }

    // ------------------------------------------------------------------
    // 8. Removal
    // ------------------------------------------------------------------

    #[test]
    fn remove_node_drops_incident_edges() {
        let mut g = worked_graph(); // A→C, B→C, C→D
        g.remove_node(id(3)).unwrap(); // remove C

        assert!(!g.contains_node(id(3)));
        assert_eq!(g.node_count(), 3);
        // Every edge incident to C is gone.
        assert_eq!(g.edge_count(), 0);
        // Adjacency maps are internally consistent.
        assert!(g.successors(id(1)).unwrap().next().is_none());
        assert!(g.predecessors(id(4)).unwrap().next().is_none());
    }

    #[test]
    fn remove_unknown_node_errors() {
        let mut g = ExecutionGraph::new();
        assert_eq!(
            g.remove_node(id(1)).err(),
            Some(ExecutionGraphError::UnknownNode { id: id(1) })
        );
    }

    #[test]
    fn remove_dependency_updates_both_adjacencies() {
        let mut g = worked_graph();
        g.remove_dependency(id(1), id(3)).unwrap();

        assert!(!g.has_edge(id(1), id(3)).unwrap());
        assert_eq!(g.edge_count(), 2);
        // B → C still present; C → D still present.
        assert!(g.has_edge(id(2), id(3)).unwrap());
        assert!(g.has_edge(id(3), id(4)).unwrap());
        // Adjacency symmetry.
        assert!(!g.predecessors(id(3)).unwrap().any(|x| x == id(1)));
        assert!(!g.successors(id(1)).unwrap().any(|x| x == id(3)));
    }

    #[test]
    fn remove_missing_dependency_errors() {
        let mut g = worked_graph();
        assert_eq!(
            g.remove_dependency(id(4), id(1)).err(),
            Some(ExecutionGraphError::EdgeNotFound {
                from: id(4),
                to: id(1),
            })
        );
    }

    // ------------------------------------------------------------------
    // 9. Topological order
    // ------------------------------------------------------------------

    #[test]
    fn topological_order_of_worked_graph_is_deterministic() {
        let g = worked_graph();
        let order: Vec<u64> = g.topological_order().iter().map(|id| id.raw()).collect();
        // A (1) and B (2) both have in-degree 0; A < B so A first.
        // Then C (3), then D (4).
        assert_eq!(order, vec![1, 2, 3, 4]);
        assert_valid_topological_order(&g);
    }

    #[test]
    fn topological_order_handles_branches_and_convergence() {
        //   A ──┐
        //       ├──> C
        //   B ──┘
        //   D ────> E
        let mut g = ExecutionGraph::new();
        for n in 1..=5u64 {
            g.add_node(id(n)).unwrap();
        }
        g.add_dependency(id(1), id(3)).unwrap();
        g.add_dependency(id(2), id(3)).unwrap();
        g.add_dependency(id(4), id(5)).unwrap();

        assert_valid_topological_order(&g);
        let order: Vec<u64> = g.topological_order().iter().map(|id| id.raw()).collect();
        // After A and B, both C (3) and D (4) are eligible; lower id first.
        assert_eq!(order, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn topological_order_of_chain() {
        let mut g = ExecutionGraph::new();
        for n in 1..=5u64 {
            g.add_node(id(n)).unwrap();
            if n > 1 {
                g.add_dependency(id(n - 1), id(n)).unwrap();
            }
        }
        let order: Vec<u64> = g.topological_order().iter().map(|id| id.raw()).collect();
        assert_eq!(order, vec![1, 2, 3, 4, 5]);
        assert_valid_topological_order(&g);
    }

    // ------------------------------------------------------------------
    // 10. Determinism
    // ------------------------------------------------------------------

    #[test]
    fn deterministic_construction_yields_equal_graphs() {
        let build = || {
            let mut g = ExecutionGraph::new();
            for n in [4u64, 1, 3, 2] {
                g.add_node(id(n)).unwrap();
            }
            g.add_dependency(id(1), id(3)).unwrap();
            g.add_dependency(id(2), id(3)).unwrap();
            g.add_dependency(id(3), id(4)).unwrap();
            g
        };

        let a = build();
        let b = build();
        let c = build();

        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(a.topological_order(), b.topological_order());
        assert_eq!(b.topological_order(), c.topological_order());
    }

    #[test]
    fn equal_graphs_have_equal_edge_sets_in_same_order() {
        let g1 = worked_graph();
        let g2 = worked_graph();
        let e1: Vec<(u64, u64)> = g1.edges().map(|(a, b)| (a.raw(), b.raw())).collect();
        let e2: Vec<(u64, u64)> = g2.edges().map(|(a, b)| (a.raw(), b.raw())).collect();
        assert_eq!(e1, e2);
    }

    // ------------------------------------------------------------------
    // 11. Error Display
    // ------------------------------------------------------------------

    #[test]
    fn error_display_contains_ids() {
        let err = ExecutionGraphError::DuplicateEdge {
            from: id(1),
            to: id(2),
        };
        let s = err.to_string();
        assert!(s.contains("1"), "got: {}", s);
        assert!(s.contains("2"), "got: {}", s);
    }

    // ------------------------------------------------------------------
    // 12. Boundary: extreme ActionId values
    // ------------------------------------------------------------------

    #[test]
    fn extreme_action_ids_are_supported() {
        let mut g = ExecutionGraph::new();
        g.add_node(id(0)).unwrap();
        g.add_node(id(u64::MAX)).unwrap();
        g.add_dependency(id(0), id(u64::MAX)).unwrap();
        let order: Vec<u64> = g.topological_order().iter().map(|id| id.raw()).collect();
        assert_eq!(order, vec![0, u64::MAX]);
    }
}