//! Deterministic structural comparison between two branch states.
use std::collections::BTreeSet;
use helqix_core::Identifier;
use helqix_state::{ComponentState, State};
use helqix_time::SimTime;
use crate::{Branch, BranchId};

/// Structural difference for one component identity.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentChange {
    component: Identifier,
    source_present: bool,
    candidate_present: bool,
    source_state: Option<ComponentState>,
    candidate_state: Option<ComponentState>,
}
impl ComponentChange {
    /// Component identity.
    pub fn component(&self) -> &Identifier { &self.component }
    /// Whether the component exists in the source branch.
    pub fn source_present(&self) -> bool { self.source_present }
    /// Whether the component exists in the candidate branch.
    pub fn candidate_present(&self) -> bool { self.candidate_present }
    /// Full component value in the source branch, if present.
    pub fn source_state(&self) -> Option<&ComponentState> { self.source_state.as_ref() }
    /// Full component value in the candidate branch, if present.
    pub fn candidate_state(&self) -> Option<&ComponentState> { self.candidate_state.as_ref() }
}

/// Immutable result comparing the current states of two branches.
#[derive(Debug, Clone, PartialEq)]
pub struct BranchComparison {
    source_branch: BranchId,
    candidate_branch: BranchId,
    source_time: SimTime,
    candidate_time: SimTime,
    source_revision: u64,
    candidate_revision: u64,
    changes: Vec<ComponentChange>,
}
impl BranchComparison {
    /// Source branch identity.
    pub fn source_branch(&self) -> &BranchId { &self.source_branch }
    /// Candidate branch identity.
    pub fn candidate_branch(&self) -> &BranchId { &self.candidate_branch }
    /// Source logical time.
    pub fn source_time(&self) -> SimTime { self.source_time }
    /// Candidate logical time.
    pub fn candidate_time(&self) -> SimTime { self.candidate_time }
    /// Source state revision.
    pub fn source_revision(&self) -> u64 { self.source_revision }
    /// Candidate state revision.
    pub fn candidate_revision(&self) -> u64 { self.candidate_revision }
    /// Changed components, ordered by canonical identifier.
    pub fn changes(&self) -> &[ComponentChange] { &self.changes }
    /// Number of components that differ or exist on only one side.
    pub fn changed_component_count(&self) -> usize { self.changes.len() }
    /// Whether the component values are identical (time and revision may differ).
    pub fn states_equal(&self) -> bool { self.changes.is_empty() }
}

/// Compares the current component state of `source` and `candidate`.
///
/// The result includes full before/after values for every changed component,
/// and is deterministic regardless of insertion order because state and
/// identifiers are traversed in canonical order. It does not judge outcomes.
pub fn compare_branches(source: &Branch, candidate: &Branch) -> BranchComparison {
    let source_state = source.state();
    let candidate_state = candidate.state();
    let ids = source_state.named_components().map(|(id, _)| id.clone())
        .chain(candidate_state.named_components().map(|(id, _)| id.clone()))
        .collect::<BTreeSet<_>>();
    let changes = ids.into_iter().filter_map(|component| {
        let before = source_state.component(&component);
        let after = candidate_state.component(&component);
        if before == after { return None; }
        Some(ComponentChange {
            component,
            source_present: before.is_some(),
            candidate_present: after.is_some(),
            source_state: before.cloned(),
            candidate_state: after.cloned(),
        })
    }).collect();
    BranchComparison {
        source_branch: source.id().clone(),
        candidate_branch: candidate.id().clone(),
        source_time: source.current_time(),
        candidate_time: candidate.current_time(),
        source_revision: source_state.revision(),
        candidate_revision: candidate_state.revision(),
        changes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(value: &str) -> Identifier { Identifier::parse(value).unwrap() }
    fn state_with(name: &str) -> State { let mut state = State::new(); state.add_component(ComponentState::new(id(name))).unwrap(); state }

    #[test]
    fn identical_states_have_no_component_differences() {
        let a = Branch::root(id("a"), SimTime::ZERO, State::new()).unwrap();
        let b = Branch::root(id("b"), SimTime::from_nanos(1), State::new()).unwrap();
        let comparison = compare_branches(&a, &b);
        assert!(comparison.states_equal());
        assert_eq!(comparison.source_time(), SimTime::ZERO);
        assert_eq!(comparison.candidate_time(), SimTime::from_nanos(1));
    }

    #[test]
    fn reports_added_component_and_full_candidate_value() {
        let a = Branch::root(id("a"), SimTime::ZERO, State::new()).unwrap();
        let b = Branch::root(id("b"), SimTime::ZERO, state_with("motor")).unwrap();
        let comparison = compare_branches(&a, &b);
        assert_eq!(comparison.changed_component_count(), 1);
        let change = &comparison.changes()[0];
        assert_eq!(change.component().as_str(), "motor");
        assert!(!change.source_present());
        assert!(change.candidate_present());
        assert!(change.candidate_state().is_some());
    }

    #[test]
    fn changed_component_order_is_deterministic() {
        let a = Branch::root(id("a"), SimTime::ZERO, State::new()).unwrap();
        let mut state = State::new();
        state.add_component(ComponentState::new(id("zeta"))).unwrap();
        state.add_component(ComponentState::new(id("alpha"))).unwrap();
        let b = Branch::root(id("b"), SimTime::ZERO, state).unwrap();
        assert_eq!(compare_branches(&a, &b).changes().iter().map(|c| c.component().as_str()).collect::<Vec<_>>(), vec!["alpha", "zeta"]);
    }
}
