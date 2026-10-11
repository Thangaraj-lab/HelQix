//! Bounded path and neighborhood queries over the declared causal graph.
use std::collections::{BTreeSet, VecDeque};
use crate::{causal_graph::CausalGraph, cause::CausalNode};
/// Maximum length of returned path.
pub const MAX_CAUSAL_PATH_LENGTH: usize = 256;
/// Enumerates deterministic simple paths from `source` to `target` up to `limit`.
pub fn paths_between(graph: &CausalGraph, source: &CausalNode, target: &CausalNode, limit: usize) -> Vec<Vec<CausalNode>> {
    if limit == 0 || !graph.contains(source) || !graph.contains(target) { return Vec::new(); }
    let mut result = Vec::new(); let mut queue = VecDeque::from([vec![source.clone()]]);
    while let Some(path) = queue.pop_front() { if result.len() >= limit { break; } let Some(last) = path.last() else { continue; };
        if last == target { result.push(path); continue; }
        if path.len() >= MAX_CAUSAL_PATH_LENGTH { continue; }
        for edge in graph.effects_of(last) { if !path.contains(&edge.effect.node) { let mut next = path.clone(); next.push(edge.effect.node.clone()); queue.push_back(next); } }
    }
    result
}
/// Returns all nodes causally upstream, including the target itself only when it has a cycle (impossible by contract).
pub fn upstream_nodes(graph: &CausalGraph, target: &CausalNode) -> BTreeSet<CausalNode> { graph.ancestors(target) }
