//! Evidence-linked ranking of declared source nodes in a causal graph.
use std::collections::{BTreeMap, VecDeque};
use crate::{causal_graph::{CausalGraph, CausalGraphError}, cause::CausalNode};

/// Root-cause candidate and strongest declared confidence path to the selected effect.
#[derive(Debug, Clone, PartialEq)]
pub struct RootCauseCandidate { pub node: CausalNode, pub path_confidence: f64 }
/// Deterministic graph analyzer. Ranking reflects declared edge metadata only.
#[derive(Debug, Clone, Copy, Default)]
pub struct RootCauseAnalyzer;
impl RootCauseAnalyzer {
    /// Finds source ancestors and ranks each by its strongest multiplicative path score.
    pub fn rank(&self, graph: &CausalGraph, target: &CausalNode) -> Result<Vec<RootCauseCandidate>, RootCauseError> {
        if !graph.contains(target) { return Err(RootCauseError::UnknownTarget); }
        let ancestors = graph.ancestors(target);
        let roots: Vec<_> = ancestors.iter().filter(|node| graph.causes_of(node).iter().all(|edge| !ancestors.contains(&edge.cause.node))).cloned().collect();
        let mut ranked = roots.into_iter().map(|root| RootCauseCandidate { path_confidence: strongest_path(graph, &root, target), node: root }).collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.path_confidence.total_cmp(&a.path_confidence).then_with(|| a.node.cmp(&b.node)));
        Ok(ranked)
    }
}
fn strongest_path(graph: &CausalGraph, source: &CausalNode, target: &CausalNode) -> f64 {
    let mut best = BTreeMap::from([(source.clone(), 1.0)]); let mut queue = VecDeque::from([source.clone()]);
    while let Some(node) = queue.pop_front() { let score = *best.get(&node).unwrap_or(&0.0);
        for edge in graph.effects_of(&node) { let candidate = score * edge.confidence; if candidate > *best.get(&edge.effect.node).unwrap_or(&-1.0) { best.insert(edge.effect.node.clone(), candidate); queue.push_back(edge.effect.node.clone()); } }
    }
    *best.get(target).unwrap_or(&0.0)
}
/// Root-cause query failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootCauseError { UnknownTarget, Graph(CausalGraphError) }
impl core::fmt::Display for RootCauseError { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self { Self::UnknownTarget => f.write_str("causal target is not in the graph"), Self::Graph(error) => error.fmt(f) } } }
impl std::error::Error for RootCauseError {}
