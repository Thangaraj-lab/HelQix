//! Deterministic storage and lineage management for branches.
use std::{collections::BTreeMap, error::Error, fmt};
use helqix_core::Identifier;
use helqix_state::State;
use helqix_time::SimTime;
use crate::{Branch, BranchError, BranchId, BranchPoint};

/// A branch forest with exactly one immutable root identity.
///
/// Branches are indexed by canonical IDs. Iteration order is lexicographic by
/// ID, while each branch's ordinal records deterministic creation order.
#[derive(Debug, Clone, PartialEq)]
pub struct BranchTree { root: BranchId, branches: BTreeMap<BranchId, Branch>, next_ordinal: u64 }

impl BranchTree {
    /// Creates a tree with one root branch and a validated initial state.
    pub fn new(root_id: BranchId, at: SimTime, initial_state: State) -> Result<Self, BranchTreeError> {
        let root = Branch::root(root_id.clone(), at, initial_state).map_err(BranchTreeError::Branch)?;
        let mut branches = BTreeMap::new();
        branches.insert(root_id.clone(), root);
        Ok(Self { root: root_id, branches, next_ordinal: 1 })
    }

    /// Root branch identity.
    pub fn root_id(&self) -> &BranchId { &self.root }
    /// Number of branches retained by the tree.
    pub fn len(&self) -> usize { self.branches.len() }
    /// Whether the tree contains only no branches (normally false after construction).
    pub fn is_empty(&self) -> bool { self.branches.is_empty() }
    /// Branch by identity.
    pub fn get(&self, id: &BranchId) -> Option<&Branch> { self.branches.get(id) }
    /// Branches in deterministic identifier order.
    pub fn branches(&self) -> impl Iterator<Item = &Branch> { self.branches.values() }
    /// Direct children of `parent`, ordered by creation ordinal.
    pub fn children(&self, parent: &BranchId) -> Result<Vec<&Branch>, BranchTreeError> {
        if !self.branches.contains_key(parent) { return Err(BranchTreeError::MissingBranch(parent.clone())); }
        let mut children = self.branches.values().filter(|branch| branch.parent() == Some(parent)).collect::<Vec<_>>();
        children.sort_by_key(|branch| branch.ordinal());
        Ok(children)
    }

    /// Creates a child fork from the parent's current committed state.
    pub fn fork(&mut self, parent: &BranchId, child_id: BranchId) -> Result<&Branch, BranchTreeError> {
        let point = self.branches.get(parent).ok_or_else(|| BranchTreeError::MissingBranch(parent.clone()))?
            .capture_point().map_err(BranchTreeError::BranchPoint)?;
        self.fork_at(parent, child_id, &point)
    }

    /// Creates a child from an earlier captured point on the requested parent.
    /// The point must belong to that parent and cannot be later than its current
    /// branch time. Historical point state remains frozen even if the parent
    /// has advanced since capture.
    pub fn fork_at(&mut self, parent: &BranchId, child_id: BranchId, point: &BranchPoint) -> Result<&Branch, BranchTreeError> {
        if self.branches.contains_key(&child_id) { return Err(BranchTreeError::DuplicateBranch(child_id)); }
        let parent_branch = self.branches.get(parent).ok_or_else(|| BranchTreeError::MissingBranch(parent.clone()))?;
        if point.branch_id() != parent { return Err(BranchTreeError::PointFromDifferentBranch { expected: parent.clone(), actual: point.branch_id().clone() }); }
        if point.at() > parent_branch.current_time() {
            return Err(BranchTreeError::PointInFuture { branch_time: parent_branch.current_time(), point_time: point.at() });
        }
        point.validate().map_err(BranchTreeError::BranchPoint)?;
        let generation = parent_branch.generation().checked_add(1).ok_or(BranchTreeError::GenerationOverflow)?;
        let ordinal = self.next_ordinal;
        let next_ordinal = ordinal.checked_add(1).ok_or(BranchTreeError::OrdinalOverflow)?;
        let branch = Branch::child(child_id.clone(), point.clone(), generation, ordinal).map_err(BranchTreeError::Branch)?;
        self.branches.insert(child_id.clone(), branch);
        self.next_ordinal = next_ordinal;
        self.branches.get(&child_id).ok_or_else(|| BranchTreeError::BrokenLineage(child_id))
    }

    /// Returns the root-to-branch lineage, including both endpoints.
    pub fn lineage(&self, id: &BranchId) -> Result<Vec<&Branch>, BranchTreeError> {
        let mut current = self.branches.get(id).ok_or_else(|| BranchTreeError::MissingBranch(id.clone()))?;
        let mut result = vec![current];
        while let Some(parent_id) = current.parent() {
            current = self.branches.get(parent_id).ok_or_else(|| BranchTreeError::BrokenLineage(parent_id.clone()))?;
            result.push(current);
        }
        result.reverse();
        Ok(result)
    }

    /// Commits a state and logical time to a branch without affecting siblings.
    pub fn commit(&mut self, id: &BranchId, at: SimTime, state: State) -> Result<(), BranchTreeError> {
        let branch = self.branches.get_mut(id).ok_or_else(|| BranchTreeError::MissingBranch(id.clone()))?;
        branch.commit(at, state).map_err(BranchTreeError::Branch)
    }
}

/// Tree construction, lookup, fork, or lineage error.
#[derive(Debug, Clone, PartialEq)]
pub enum BranchTreeError {
    /// Requested branch does not exist.
    MissingBranch(BranchId),
    /// Branch ID is already present.
    DuplicateBranch(BranchId),
    /// Fork point was captured from a different branch.
    PointFromDifferentBranch { /// Requested parent.
        expected: BranchId, /// Point's actual source.
        actual: BranchId },
    /// Fork point is later than the current branch time.
    PointInFuture { /// Parent's current time.
        branch_time: SimTime, /// Requested point time.
        point_time: SimTime },
    /// A retained parent reference is missing.
    BrokenLineage(BranchId),
    /// Branch creation depth exceeded its integer representation.
    GenerationOverflow,
    /// Branch creation ordinal exceeded its integer representation.
    OrdinalOverflow,
    /// Branch creation failed validation.
    Branch(BranchError),
    /// Fork point failed validation.
    BranchPoint(crate::BranchPointError),
}
impl fmt::Display for BranchTreeError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self {
    Self::MissingBranch(id) => write!(f, "branch `{id}` does not exist"), Self::DuplicateBranch(id) => write!(f, "branch `{id}` already exists"),
    Self::PointFromDifferentBranch { expected, actual } => write!(f, "fork point belongs to `{actual}`, not requested parent `{expected}`"),
    Self::PointInFuture { branch_time, point_time } => write!(f, "fork point at {point_time} is later than parent time {branch_time}"),
    Self::BrokenLineage(id) => write!(f, "branch lineage references missing parent `{id}`"),
    Self::GenerationOverflow => f.write_str("branch generation overflow"), Self::OrdinalOverflow => f.write_str("branch ordinal overflow"),
    Self::Branch(error) => write!(f, "branch operation failed: {error}"), Self::BranchPoint(error) => write!(f, "branch point is invalid: {error}"),
} } }
impl Error for BranchTreeError { fn source(&self) -> Option<&(dyn Error + 'static)> { match self {
    Self::Branch(error) => Some(error), Self::BranchPoint(error) => Some(error), _ => None,
} } }

#[cfg(test)]
mod tests {
    use super::*;
    fn id(value: &str) -> Identifier { Identifier::parse(value).unwrap() }

    #[test]
    fn forks_are_isolated_and_retain_shared_origin() {
        let mut tree = BranchTree::new(id("main"), SimTime::ZERO, State::new()).unwrap();
        tree.fork(&id("main"), id("left")).unwrap();
        tree.fork(&id("main"), id("right")).unwrap();
        assert_eq!(tree.get(&id("left")).unwrap().state(), tree.get(&id("right")).unwrap().state());
        assert_ne!(tree.get(&id("left")).unwrap().state() as *const State, tree.get(&id("right")).unwrap().state() as *const State);
        assert_eq!(tree.children(&id("main")).unwrap().iter().map(|b| b.id().as_str()).collect::<Vec<_>>(), vec!["left", "right"]);
    }

    #[test]
    fn historical_point_remains_frozen_after_parent_advances() {
        let mut tree = BranchTree::new(id("main"), SimTime::ZERO, State::new()).unwrap();
        let point = tree.get(&id("main")).unwrap().capture_point().unwrap();
        tree.commit(&id("main"), SimTime::from_nanos(10), State::new()).unwrap();
        let child = tree.fork_at(&id("main"), id("past"), &point).unwrap();
        assert_eq!(child.current_time(), SimTime::ZERO);
    }

    #[test]
    fn rejects_fork_point_owned_by_another_branch() {
        let mut tree = BranchTree::new(id("main"), SimTime::ZERO, State::new()).unwrap();
        let point = Branch::root(id("other"), SimTime::ZERO, State::new()).unwrap().capture_point().unwrap();
        assert!(matches!(tree.fork_at(&id("main"), id("child"), &point), Err(BranchTreeError::PointFromDifferentBranch { .. })));
    }

    #[test]
    fn lineage_is_root_first() {
        let mut tree = BranchTree::new(id("main"), SimTime::ZERO, State::new()).unwrap();
        tree.fork(&id("main"), id("child")).unwrap();
        tree.fork(&id("child"), id("grandchild")).unwrap();
        assert_eq!(tree.lineage(&id("grandchild")).unwrap().iter().map(|b| b.id().as_str()).collect::<Vec<_>>(), vec!["main", "child", "grandchild"]);
    }
}
