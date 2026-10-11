//! Bounded deterministic propagation through explicitly declared failure links.
use std::collections::{BTreeSet, VecDeque};
use helqix_core::id::Identifier;
use crate::fault::{Fault, FaultSeverity};

/// Upper bound prevents malformed domain graphs from monopolizing a runtime.
pub const MAX_PROPAGATION_HOPS: usize = 1_024;
/// Explicitly declared consequence of a component failure mode.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PropagationRule { pub source_component: Identifier, pub source_mode: Identifier, pub target_component: Identifier, pub target_mode: Identifier, pub minimum_severity: FaultSeverity }
/// One traversed edge; it is a modeled relationship, not proof of physical causation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropagationStep { pub parent_step: Option<usize>, pub component: Identifier, pub mode: Identifier, pub severity: FaultSeverity, pub via: PropagationRule }
/// Complete deterministic propagation result for an initiating fault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailurePropagation { pub root_fault: Identifier, pub steps: Vec<PropagationStep>, pub truncated: bool }
/// Explicit fault propagation graph. Rules have no implicit physics or probability.
#[derive(Debug, Clone, Default)]
pub struct FailurePropagator { rules: BTreeSet<PropagationRule> }
impl FailurePropagator {
    /// Adds an explicit failure relationship. Duplicate declarations are rejected.
    pub fn add_rule(&mut self, rule: PropagationRule) -> Result<(), PropagationError> {
        if rule.source_component == rule.target_component && rule.source_mode == rule.target_mode { return Err(PropagationError::SelfPropagation); }
        if !self.rules.insert(rule) { return Err(PropagationError::DuplicateRule); }
        Ok(())
    }
    /// Propagates by breadth-first traversal in stable rule order with a hard hop cap.
    pub fn propagate(&self, root: &Fault) -> FailurePropagation {
        let mut result = FailurePropagation { root_fault: root.id().clone(), steps: Vec::new(), truncated: false };
        let mut queue = VecDeque::from([(None, root.component_id().clone(), root.mode().clone(), root.severity())]);
        let mut visited = BTreeSet::from([(root.component_id().clone(), root.mode().clone())]);
        while let Some((parent_step, component, mode, severity)) = queue.pop_front() {
            for rule in self.rules.iter().filter(|r| r.source_component == component && r.source_mode == mode) {
                if result.steps.len() >= MAX_PROPAGATION_HOPS { result.truncated = true; return result; }
                let child_severity = severity.max(rule.minimum_severity);
                let target = (rule.target_component.clone(), rule.target_mode.clone());
                if visited.insert(target.clone()) {
                    let index = result.steps.len();
                    result.steps.push(PropagationStep { parent_step, component: target.0.clone(), mode: target.1.clone(), severity: child_severity, via: rule.clone() });
                    queue.push_back((Some(index), target.0, target.1, child_severity));
                }
            }
        }
        result
    }
    /// Number of declared propagation edges.
    pub fn rule_count(&self) -> usize { self.rules.len() }
}
/// Invalid propagation graph changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropagationError { SelfPropagation, DuplicateRule }
impl core::fmt::Display for PropagationError { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self { Self::SelfPropagation => f.write_str("a failure mode cannot propagate to itself"), Self::DuplicateRule => f.write_str("propagation rule already exists") } } }
impl std::error::Error for PropagationError {}
