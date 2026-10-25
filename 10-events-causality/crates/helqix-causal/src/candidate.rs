//! Candidate causal chains for a failure: structural dependency + temporal precedence + magnitude.
//!
//! Method (stated, deterministic):
//! 1. Structure: only paths of declared structural relations ending at the failed node count.
//! 2. Magnitude: a node is *changed* if its largest absolute change from its first sample, up to
//!    the failure sample, is at least `min_relative_change` of its own scale
//!    (`max(|first value|, largest absolute change)`).
//! 3. Onset: the first sample where a node's change reaches `onset_fraction` of that largest change.
//! 4. A chain is kept only if every node is changed and onsets never go backwards along the chain
//!    (a cause cannot start changing after its effect).
//! 5. Ranking: earlier root onset first, then fewer nodes, then node order. The score is reported
//!    as `(root onset index, chain length)`, not as a probability.
//!
//! Grade: every result is a **candidate (structural + temporal)**. Only an intervention test
//! (Phase 4) can upgrade it, and only to model-causal. A shared driver is never reported as an
//! effect causing another effect unless the declared structure contains that link.
use std::collections::BTreeMap;

use crate::{causal_graph::CausalGraph, cause::CausalNode, propagation::paths_between};

/// Grade label carried by every candidate chain.
pub const CANDIDATE_GRADE: &str = "candidate (structural + temporal)";

/// Sampled values of one node, aligned to a shared time base.
#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    /// Node the signal belongs to.
    pub node: CausalNode,
    /// Sample values; index `i` corresponds to shared time index `i`.
    pub values: Vec<f64>,
}

/// Thresholds of the method (caller-stated, recorded with every result).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandidateMethod {
    /// Minimum change relative to the node's scale for it to count as changed.
    pub min_relative_change: f64,
    /// Fraction of the largest change that marks the onset.
    pub onset_fraction: f64,
    /// Maximum number of paths examined.
    pub path_limit: usize,
}

/// How one node behaved up to the failure.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeChange {
    pub node: CausalNode,
    pub changed: bool,
    pub onset_index: Option<usize>,
    pub largest_change: f64,
}

/// One candidate chain from an upstream node to the failed node.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateChain {
    pub nodes: Vec<CausalNode>,
    pub changes: Vec<NodeChange>,
    pub root_onset_index: usize,
    pub grade: &'static str,
    pub method: CandidateMethod,
}

/// Candidate chains ending at `failed` whose failure is observed at `failure_index`.
pub fn candidate_chains(graph: &CausalGraph, signals: &[Signal], failed: &CausalNode, failure_index: usize, method: CandidateMethod) -> Vec<CandidateChain> {
    let by_node: BTreeMap<&CausalNode, &Signal> = signals.iter().map(|s| (&s.node, s)).collect();
    let change_of = |node: &CausalNode| -> NodeChange {
        let unchanged = NodeChange { node: node.clone(), changed: false, onset_index: None, largest_change: 0.0 };
        let Some(signal) = by_node.get(node) else { return unchanged };
        let window = &signal.values[..signal.values.len().min(failure_index.saturating_add(1))];
        let Some(&first) = window.first() else { return unchanged };
        if window.iter().any(|v| !v.is_finite()) { return unchanged; }
        let largest = window.iter().map(|v| (v - first).abs()).fold(0.0_f64, f64::max);
        let scale = first.abs().max(largest);
        if largest == 0.0 || largest < method.min_relative_change * scale { return NodeChange { largest_change: largest, ..unchanged }; }
        let onset = window.iter().position(|v| (v - first).abs() >= method.onset_fraction * largest);
        NodeChange { node: node.clone(), changed: true, onset_index: onset, largest_change: largest }
    };
    if !change_of(failed).changed { return Vec::new(); }
    let mut chains: Vec<CandidateChain> = graph.ancestors(failed).iter()
        .flat_map(|source| paths_between(graph, source, failed, method.path_limit))
        .filter_map(|nodes| {
            let changes: Vec<NodeChange> = nodes.iter().map(&change_of).collect();
            let onsets: Option<Vec<usize>> = changes.iter().map(|c| if c.changed { c.onset_index } else { None }).collect();
            let onsets = onsets?;
            if onsets.windows(2).any(|w| w[0] > w[1]) { return None; }
            Some(CandidateChain { root_onset_index: onsets[0], nodes, changes, grade: CANDIDATE_GRADE, method })
        })
        .collect();
    chains.sort_by(|a, b| a.root_onset_index.cmp(&b.root_onset_index).then(a.nodes.len().cmp(&b.nodes.len())).then_with(|| a.nodes.cmp(&b.nodes)));
    chains
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cause::{Cause, CausalNodeKind}, effect::Effect};
    use helqix_core::id::Identifier;

    fn n(name: &str) -> CausalNode { CausalNode::new(CausalNodeKind::State, Identifier::parse(name).unwrap()) }
    fn rel(g: &mut CausalGraph, a: &str, b: &str) { g.add_relation(Cause::new(n(a), None), Effect::new(n(b), None), Identifier::parse(&format!("eq_{a}_{b}")).unwrap()).unwrap(); }
    fn sig(name: &str, values: &[f64]) -> Signal { Signal { node: n(name), values: values.to_vec() } }
    const M: CandidateMethod = CandidateMethod { min_relative_change: 0.05, onset_fraction: 0.1, path_limit: 64 };

    /// A steps at t=2, B follows at t=4, C at t=6 (ground truth A -> B -> C).
    fn chain_signals() -> Vec<Signal> {
        vec![sig("a", &[1., 1., 2., 2., 2., 2., 2., 2.]), sig("b", &[0., 0., 0., 0., 3., 3., 3., 3.]), sig("c", &[5., 5., 5., 5., 5., 5., 9., 9.])]
    }

    #[test]
    fn recovers_a_known_chain_and_ranks_it_first() {
        let mut g = CausalGraph::new(10).unwrap();
        rel(&mut g, "a", "b"); rel(&mut g, "b", "c");
        let chains = candidate_chains(&g, &chain_signals(), &n("c"), 7, M);
        assert_eq!(chains[0].nodes, vec![n("a"), n("b"), n("c")]);
        assert_eq!(chains[0].grade, CANDIDATE_GRADE);
        assert_eq!(chains[0].root_onset_index, 2);
    }

    #[test]
    fn a_common_driver_is_not_reported_as_one_effect_causing_another() {
        let mut g = CausalGraph::new(10).unwrap();
        rel(&mut g, "a", "b"); rel(&mut g, "a", "c");
        let chains = candidate_chains(&g, &chain_signals(), &n("c"), 7, M);
        assert!(chains.iter().all(|c| !c.nodes.contains(&n("b"))), "{chains:?}");
        assert_eq!(chains[0].nodes, vec![n("a"), n("c")]);
    }

    #[test]
    fn a_declared_mediator_that_changes_after_the_effect_is_rejected() {
        let mut g = CausalGraph::new(10).unwrap();
        rel(&mut g, "a", "b"); rel(&mut g, "b", "c"); rel(&mut g, "a", "c");
        let late_b = vec![sig("a", &[1., 1., 2., 2., 2., 2., 2., 2.]), sig("b", &[0., 0., 0., 0., 0., 0., 0., 3.]), sig("c", &[5., 5., 5., 5., 5., 5., 9., 9.])];
        let chains = candidate_chains(&g, &late_b, &n("c"), 7, M);
        assert!(chains.iter().all(|c| !c.nodes.contains(&n("b"))), "{chains:?}");
    }

    #[test]
    fn an_unchanged_upstream_node_cannot_be_a_cause() {
        let mut g = CausalGraph::new(10).unwrap();
        rel(&mut g, "x", "c"); rel(&mut g, "a", "c");
        let signals = vec![sig("x", &[4.; 8]), sig("a", &[1., 1., 2., 2., 2., 2., 2., 2.]), sig("c", &[5., 5., 5., 5., 5., 5., 9., 9.])];
        let chains = candidate_chains(&g, &signals, &n("c"), 7, M);
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].nodes, vec![n("a"), n("c")]);
    }

    #[test]
    fn no_change_at_the_failed_node_means_no_chain() {
        let mut g = CausalGraph::new(10).unwrap();
        rel(&mut g, "a", "c");
        let signals = vec![sig("a", &[1., 1., 2., 2.]), sig("c", &[5., 5., 5., 5.])];
        assert!(candidate_chains(&g, &signals, &n("c"), 3, M).is_empty());
    }

    #[test]
    fn simultaneous_failures_get_separate_chains() {
        let mut g = CausalGraph::new(10).unwrap();
        rel(&mut g, "a", "c"); rel(&mut g, "p", "q");
        let signals = vec![sig("a", &[1., 2., 2., 2.]), sig("c", &[0., 0., 3., 3.]), sig("p", &[7., 7., 1., 1.]), sig("q", &[2., 2., 2., 8.])];
        let left = candidate_chains(&g, &signals, &n("c"), 3, M);
        let right = candidate_chains(&g, &signals, &n("q"), 3, M);
        assert_eq!(left[0].nodes, vec![n("a"), n("c")]);
        assert_eq!(right[0].nodes, vec![n("p"), n("q")]);
        assert!(left.iter().all(|c| !c.nodes.contains(&n("p"))));
    }

    #[test]
    fn changes_after_the_failure_sample_are_ignored() {
        let mut g = CausalGraph::new(10).unwrap();
        rel(&mut g, "a", "c");
        let signals = vec![sig("a", &[1., 1., 1., 1., 9.]), sig("c", &[0., 0., 3., 3., 3.])];
        assert!(candidate_chains(&g, &signals, &n("c"), 3, M).is_empty());
    }
}
