//! HELQIX Layer 10 causal assertions and bounded path analysis.
//!
//! Graph links are user- or model-declared hypotheses with traceable references.
//! A graph traversal cannot establish physical root cause by itself.
#![forbid(unsafe_code)]
/// Cause nodes and evidence references.
pub mod cause;
/// Candidate causal chains (structural + temporal + magnitude).
pub mod candidate;
/// Directed graph and cycle validation.
pub mod causal_graph;
/// Effect assertions.
pub mod effect;
/// Path and upstream queries.
pub mod propagation;
/// Root-cause candidate ranking.
pub mod root_cause;
pub use candidate::{candidate_chains, CandidateChain, CandidateMethod, NodeChange, Signal, CANDIDATE_GRADE};
pub use cause::{Cause, CausalNode, CausalNodeKind};
pub use causal_graph::{CausalEdge, CausalGraph, CausalGraphError};
pub use effect::Effect;
pub use propagation::{paths_between, upstream_nodes, MAX_CAUSAL_PATH_LENGTH};
pub use root_cause::{RootCauseAnalyzer, RootCauseCandidate, RootCauseError};

#[cfg(test)]
mod tests {
    use super::*; use helqix_core::id::Identifier;
    fn node(kind: CausalNodeKind, name: &str) -> CausalNode { CausalNode::new(kind, Identifier::parse(name).unwrap()) }
    fn edge(graph: &mut CausalGraph, from: CausalNode, to: CausalNode, score: f64) { graph.add_edge(Cause::new(from, None), Effect::new(to, None), score).unwrap(); }
    #[test]
    fn graph_rejects_cycles_and_ranks_declared_roots() {
        let a = node(CausalNodeKind::Event, "overcurrent"); let b = node(CausalNodeKind::Failure, "cell_fault"); let c = node(CausalNodeKind::State, "voltage_drop");
        let mut graph = CausalGraph::new(20).unwrap(); edge(&mut graph, a.clone(), b.clone(), 0.8); edge(&mut graph, b.clone(), c.clone(), 0.9);
        assert_eq!(RootCauseAnalyzer.rank(&graph, &c).unwrap()[0].node, a);
        assert_eq!(paths_between(&graph, &node(CausalNodeKind::Event, "overcurrent"), &c, 4).len(), 1);
        assert_eq!(graph.add_edge(Cause::new(c, None), Effect::new(b, None), 0.5), Err(CausalGraphError::CycleDetected));
    }
}
