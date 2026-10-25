//! Layer 7 failure investigation: from a requirement violation to candidate causes.
//!
//! [`investigate`] takes sampled node signals, the model's declared structural relations (each
//! with an equation or relation id and its parameters) and one requirement. It
//!
//! 1. finds the **failure event** as the first sample violating the requirement (failures come
//!    from requirements, not from code),
//! 2. records onset and failure events in an append-only, time-ordered [`EventLog`],
//! 3. ranks **candidate** causal chains with [`helqix_causal::candidate_chains`], and
//! 4. serves a click-through [`NodeDetail`] for every node on a chain.
//!
//! Every chain is labelled "candidate (structural + temporal)". Nothing here proves a cause:
//! upgrading a candidate needs an intervention test (Phase 4).
#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use helqix_causal::{candidate_chains, CandidateChain, CandidateMethod, CausalGraph, CausalNode, CausalNodeKind, Cause, Effect, Signal, CANDIDATE_GRADE};
use helqix_core::id::Identifier;
use helqix_events::{Event, EventLog};
use helqix_time::SimTime;

/// Direction of a requirement on one node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparator {
    /// The node must stay at or above the threshold.
    AtLeast,
    /// The node must stay at or below the threshold.
    AtMost,
}

/// Sampled values of one node at the shared times.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeSeries { pub id: Identifier, pub values: Vec<f64> }

/// A declared structural relation (from the model, not inferred).
#[derive(Debug, Clone, PartialEq)]
pub struct Relation { pub cause: Identifier, pub effect: Identifier, pub relation: Identifier, pub parameters: Vec<(String, f64)> }

/// The requirement whose violation is the failure.
#[derive(Debug, Clone, PartialEq)]
pub struct Requirement { pub id: Identifier, pub node: Identifier, pub comparator: Comparator, pub threshold: f64 }

/// Link to the exact frozen state (Phase 2 freeze) at the failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateReference { pub freeze_sha256: String, pub index: usize }

/// Everything an investigation needs.
#[derive(Debug, Clone, PartialEq)]
pub struct InvestigationInput {
    pub times_seconds: Vec<f64>,
    pub nodes: Vec<NodeSeries>,
    pub relations: Vec<Relation>,
    pub requirement: Requirement,
    pub state_reference: Option<StateReference>,
    pub method: CandidateMethod,
}

/// The failure event.
#[derive(Debug, Clone, PartialEq)]
pub struct FailureRecord { pub requirement: Identifier, pub node: Identifier, pub threshold: f64, pub observed_value: f64, pub index: usize, pub time_seconds: f64 }

/// Click-through detail for one node on a candidate chain.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeDetail {
    pub node: Identifier,
    /// Equation or relation linking the previous chain node to this one (`None` for the root).
    pub relation: Option<Identifier>,
    /// Parameters of that relation with their values.
    pub parameters: Vec<(String, f64)>,
    pub baseline_value: f64,
    /// Value one sample before the failure.
    pub previous_value: f64,
    pub value_at_failure: f64,
    /// Signal (observation) this node's values come from.
    pub signal: Identifier,
    pub state_reference: Option<StateReference>,
    pub related_events: Vec<Identifier>,
    /// Branch experiments testing this node (filled by Phase 4 interventions).
    pub branch_experiments: Vec<Identifier>,
    pub grade: &'static str,
}

/// Result of an investigation.
#[derive(Debug, Clone, PartialEq)]
pub struct Investigation { failure: Option<FailureRecord>, chains: Vec<CandidateChain>, events: EventLog, details: Vec<NodeDetail> }

/// Rejected input.
#[derive(Debug, Clone, PartialEq)]
pub enum InvestigationError { Invalid(String) }

impl std::fmt::Display for InvestigationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self { Self::Invalid(m) => write!(f, "invalid investigation input: {m}") } }
}
impl std::error::Error for InvestigationError {}

fn invalid(m: impl Into<String>) -> InvestigationError { InvestigationError::Invalid(m.into()) }
fn state_node(id: &Identifier) -> CausalNode { CausalNode::new(CausalNodeKind::State, id.clone()) }
fn sim_time(seconds: f64) -> SimTime { SimTime::from_nanos((seconds * 1e9).round() as i128) }
fn event_id(text: &str) -> Result<Identifier, InvestigationError> { Identifier::parse(&text[..text.len().min(64)]).map_err(|e| invalid(e.to_string())) }

/// Runs the investigation. Fails on malformed input rather than guessing.
pub fn investigate(input: &InvestigationInput) -> Result<Investigation, InvestigationError> {
    let n = input.times_seconds.len();
    if n == 0 || input.times_seconds.iter().any(|t| !t.is_finite()) || input.times_seconds.windows(2).any(|w| w[1] <= w[0]) {
        return Err(invalid("times must be finite and strictly increasing"));
    }
    let mut ids = BTreeSet::new();
    for s in &input.nodes {
        if s.values.len() != n { return Err(invalid(format!("node {} has {} samples, expected {n}", s.id.as_str(), s.values.len()))); }
        if !ids.insert(s.id.clone()) { return Err(invalid(format!("duplicate node {}", s.id.as_str()))); }
    }
    let mut graph = CausalGraph::bounded_default();
    for r in &input.relations {
        for end in [&r.cause, &r.effect] { if !ids.contains(end) { return Err(invalid(format!("relation {} names unknown node {}", r.relation.as_str(), end.as_str()))); } }
        graph.add_relation(Cause::new(state_node(&r.cause), None), Effect::new(state_node(&r.effect), None), r.relation.clone()).map_err(|e| invalid(e.to_string()))?;
    }
    let req = &input.requirement;
    let target = input.nodes.iter().find(|s| s.id == req.node).ok_or_else(|| invalid(format!("requirement names unknown node {}", req.node.as_str())))?;

    let violated = |v: f64| !v.is_finite() || match req.comparator { Comparator::AtLeast => v < req.threshold, Comparator::AtMost => v > req.threshold };
    let Some(index) = target.values.iter().position(|v| violated(*v)) else {
        return Ok(Investigation { failure: None, chains: Vec::new(), events: EventLog::new(), details: Vec::new() });
    };
    let failure = FailureRecord { requirement: req.id.clone(), node: req.node.clone(), threshold: req.threshold, observed_value: target.values[index], index, time_seconds: input.times_seconds[index] };

    let signals: Vec<Signal> = input.nodes.iter().map(|s| Signal { node: state_node(&s.id), values: s.values.clone() }).collect();
    let chains = candidate_chains(&graph, &signals, &state_node(&req.node), index, input.method);

    // Events: one onset per node on any chain, then the failure; appended in time order.
    let mut pending: Vec<(i128, u8, Event)> = Vec::new();
    let mut seen = BTreeSet::new();
    for change in chains.iter().flat_map(|c| &c.changes) {
        if let (true, Some(onset)) = (seen.insert(change.node.id.clone()), change.onset_index) {
            let at = sim_time(input.times_seconds[onset]);
            pending.push((at.as_nanos(), 0, Event::new(event_id(&format!("onset_{}", change.node.id.as_str()))?, event_id("onset")?, change.node.id.clone(), at)));
        }
    }
    let at = sim_time(failure.time_seconds);
    pending.push((at.as_nanos(), 1, Event::new(event_id(&format!("failure_{}", req.id.as_str()))?, event_id("failure")?, req.node.clone(), at)));
    pending.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)).then_with(|| a.2.id.cmp(&b.2.id)));
    let mut events = EventLog::new();
    for (_, _, e) in pending { events.append(e).map_err(|e| invalid(e.to_string()))?; }

    let mut details = Vec::new();
    for node in chains.iter().flat_map(|c| &c.nodes) {
        if details.iter().any(|d: &NodeDetail| d.node == node.id) { continue; }
        let chain = chains.iter().find(|c| c.nodes.contains(node)).expect("node comes from a chain");
        let pos = chain.nodes.iter().position(|x| x == node).expect("node is on its chain");
        let incoming = (pos > 0).then(|| input.relations.iter().find(|r| r.cause == chain.nodes[pos - 1].id && r.effect == node.id)).flatten();
        let values = &input.nodes.iter().find(|s| s.id == node.id).expect("validated node").values;
        details.push(NodeDetail {
            node: node.id.clone(),
            relation: incoming.map(|r| r.relation.clone()),
            parameters: incoming.map(|r| r.parameters.clone()).unwrap_or_default(),
            baseline_value: values[0],
            previous_value: values[index.saturating_sub(1)],
            value_at_failure: values[index],
            signal: node.id.clone(),
            state_reference: input.state_reference.clone(),
            related_events: events.from_source(&node.id).map(|e| e.id.clone()).collect(),
            branch_experiments: Vec::new(),
            grade: CANDIDATE_GRADE,
        });
    }
    Ok(Investigation { failure: Some(failure), chains, events, details })
}

impl Investigation {
    /// The failure event, if the requirement was violated.
    pub fn failure(&self) -> Option<&FailureRecord> { self.failure.as_ref() }
    /// Candidate chains, best first.
    pub fn chains(&self) -> &[CandidateChain] { &self.chains }
    /// Ordered onset and failure events.
    pub fn events(&self) -> &EventLog { &self.events }
    /// Click-through detail for a node on a candidate chain.
    pub fn node_detail(&self, node: &Identifier) -> Option<&NodeDetail> { self.details.iter().find(|d| &d.node == node) }
    /// All node details.
    pub fn details(&self) -> &[NodeDetail] { &self.details }
}
