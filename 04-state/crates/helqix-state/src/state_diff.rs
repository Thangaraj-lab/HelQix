//! Deterministic difference between two HELQIX state snapshots.
//!
//! `StateDiff` answers one question:
//!
//!     "What changed between these two exact states?"
//!
//! It does not explain why the change happened.
//! It does not classify failures.
//! It does not create branches.
//! It does not execute physics.
//!
//! Those responsibilities belong to later HELQIX layers.
//!
//! Determinism is guaranteed by ordering component identifiers through
//! `BTreeSet`.

use std::collections::BTreeSet;
use std::{error::Error, fmt};

use helqix_core::id::Identifier;

use crate::component_state::ComponentState;
use crate::state_snapshot::StateSnapshot;
use crate::snapshot::Snapshot;

/// A change to one component between two state snapshots.
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentStateChange {
    /// The component did not exist in the earlier state and exists in the
    /// later state.
    Added {
        state: ComponentState,
    },

    /// The component existed in the earlier state but no longer exists in
    /// the later state.
    Removed {
        state: ComponentState,
    },

    /// The component existed in both states but its state changed.
    Modified {
        before: ComponentState,
        after: ComponentState,
    },
}

impl ComponentStateChange {
    /// Returns the canonical component name associated with this change.
    #[must_use]
    pub fn name(&self) -> &Identifier {
        match self {
            Self::Added { state } => state.name(),
            Self::Removed { state } => state.name(),
            Self::Modified { after, .. } => after.name(),
        }
    }

    /// Returns `true` when this change represents an addition.
    #[must_use]
    pub fn is_added(&self) -> bool {
        matches!(self, Self::Added { .. })
    }

    /// Returns `true` when this change represents a removal.
    #[must_use]
    pub fn is_removed(&self) -> bool {
        matches!(self, Self::Removed { .. })
    }

    /// Returns `true` when this change represents a modification.
    #[must_use]
    pub fn is_modified(&self) -> bool {
        matches!(self, Self::Modified { .. })
    }

    /// Returns the state before the change, when one exists.
    ///
    /// Added components have no previous state.
    #[must_use]
    pub fn before(&self) -> Option<&ComponentState> {
        match self {
            Self::Added { .. } => None,
            Self::Removed { state } => Some(state),
            Self::Modified { before, .. } => Some(before),
        }
    }

    /// Returns the state after the change, when one exists.
    ///
    /// Removed components have no resulting state.
    #[must_use]
    pub fn after(&self) -> Option<&ComponentState> {
        match self {
            Self::Added { state } => Some(state),
            Self::Removed { .. } => None,
            Self::Modified { after, .. } => Some(after),
        }
    }
}

/// Deterministic difference between two immutable [`StateSnapshot`] values.
///
/// Changes are ordered by canonical component identifier.
///
/// A `StateDiff` owns its changed component states. Therefore, it remains
/// valid independently of the snapshots from which it was produced.
#[derive(Debug, Clone, PartialEq)]
pub struct StateDiff {
    changes: Vec<ComponentStateChange>,
}

impl StateDiff {
    /// Computes a component diff while retaining both source revisions.
    pub fn between_revisioned(
        before: &Snapshot,
        after: &Snapshot,
    ) -> Result<RevisionedStateDiff, RevisionedStateDiffError> {
        let before_components = before.component_snapshot()
            .map_err(RevisionedStateDiffError::InvalidSnapshot)?;
        let after_components = after.component_snapshot()
            .map_err(RevisionedStateDiffError::InvalidSnapshot)?;
        Ok(RevisionedStateDiff {
            before_revision: before.revision(),
            after_revision: after.revision(),
            diff: Self::between(&before_components, &after_components),
        })
    }

    /// Computes the deterministic difference from `before` to `after`.
    ///
    /// The direction matters:
    ///
    /// `StateDiff::between(a, b)` describes how state changes from `a` to `b`.
    #[must_use]
    pub fn between(before: &StateSnapshot, after: &StateSnapshot) -> Self {
        let mut names = BTreeSet::new();

        for (name, _) in before.iter() {
            names.insert(name.clone());
        }

        for (name, _) in after.iter() {
            names.insert(name.clone());
        }

        let mut changes = Vec::new();

        for name in names {
            match (before.get(&name), after.get(&name)) {
                (None, Some(state)) => {
                    changes.push(ComponentStateChange::Added {
                        state: state.clone(),
                    });
                }

                (Some(state), None) => {
                    changes.push(ComponentStateChange::Removed {
                        state: state.clone(),
                    });
                }

                (Some(before_state), Some(after_state))
                    if before_state != after_state =>
                {
                    changes.push(ComponentStateChange::Modified {
                        before: before_state.clone(),
                        after: after_state.clone(),
                    });
                }

                (Some(_), Some(_)) => {}

                (None, None) => {
                    unreachable!("component name came from at least one snapshot");
                }
            }
        }

        Self { changes }
    }

    /// Returns the number of changed components.
    #[must_use]
    pub fn len(&self) -> usize {
        self.changes.len()
    }

    /// Returns `true` when no component changed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Returns the number of added components.
    #[must_use]
    pub fn added_count(&self) -> usize {
        self.changes
            .iter()
            .filter(|change| change.is_added())
            .count()
    }

    /// Returns the number of removed components.
    #[must_use]
    pub fn removed_count(&self) -> usize {
        self.changes
            .iter()
            .filter(|change| change.is_removed())
            .count()
    }

    /// Returns the number of modified components.
    #[must_use]
    pub fn modified_count(&self) -> usize {
        self.changes
            .iter()
            .filter(|change| change.is_modified())
            .count()
    }

    /// Iterates over changes in deterministic component-name order.
    #[must_use]
    pub fn iter(&self) -> impl Iterator<Item = &ComponentStateChange> {
        self.changes.iter()
    }

    /// Returns all changes in deterministic order.
    #[must_use]
    pub fn changes(&self) -> &[ComponentStateChange] {
        &self.changes
    }

    /// Returns the change for a component, if that component changed.
    #[must_use]
    pub fn get(&self, name: &Identifier) -> Option<&ComponentStateChange> {
        self.changes
            .binary_search_by(|change| change.name().cmp(name))
            .ok()
            .map(|index| &self.changes[index])
    }

    /// Returns `true` when the given component changed.
    #[must_use]
    pub fn contains(&self, name: &Identifier) -> bool {
        self.get(name).is_some()
    }
}

/// A deterministic diff that records the revisions it compares.
#[derive(Debug, Clone, PartialEq)]
pub struct RevisionedStateDiff {
    before_revision: u64,
    after_revision: u64,
    diff: StateDiff,
}

impl RevisionedStateDiff {
    #[must_use]
    pub const fn before_revision(&self) -> u64 { self.before_revision }
    #[must_use]
    pub const fn after_revision(&self) -> u64 { self.after_revision }
    #[must_use]
    pub const fn diff(&self) -> &StateDiff { &self.diff }
    #[must_use]
    pub fn into_diff(self) -> StateDiff { self.diff }
}

/// Failure while producing a diff from revisioned snapshots.
#[derive(Debug, Clone, PartialEq)]
pub enum RevisionedStateDiffError {
    InvalidSnapshot(crate::state_snapshot::StateSnapshotError),
}

impl fmt::Display for RevisionedStateDiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSnapshot(error) => write!(f, "cannot diff invalid snapshot: {error}"),
        }
    }
}

impl Error for RevisionedStateDiffError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self { Self::InvalidSnapshot(error) => Some(error) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component_state::ComponentState;
    use crate::property_state::PropertyState;
    use crate::state_registry::StateRegistry;
    use crate::state_snapshot::StateSnapshot;

    use helqix_core::id::Identifier;
    use helqix_core::quantity::Quantity;
    use helqix_core::units::Unit;

    fn identifier(value: &str) -> Identifier {
        value.parse().expect("test identifier must be valid")
    }

    fn snapshot_with_component(
        component_name: &str,
        property_name: &str,
        property_value: f64,
    ) -> StateSnapshot {
        let component_id = identifier(component_name);
        let property_id = identifier(property_name);
        let unit = Unit::parse("kg").expect("kg must be registered");
        let quantity =
            Quantity::new(property_value, unit).expect("test quantity must be valid");

        let mut component = ComponentState::new(component_id);
        component
            .add_property(PropertyState::with_value(property_id, quantity))
            .expect("test property must be accepted");

        let mut registry = StateRegistry::new();
        registry
            .register(component)
            .expect("test component must be accepted");

        StateSnapshot::new(registry).expect("test snapshot must be valid")
    }

    fn snapshot_with_components(names: &[&str]) -> StateSnapshot {
        let mut registry = StateRegistry::new();

        for name in names {
            registry
                .register(ComponentState::new(identifier(name)))
                .expect("test component must be accepted");
        }

        StateSnapshot::new(registry).expect("test snapshot must be valid")
    }

    #[test]
    fn identical_snapshots_produce_empty_diff() {
        let before = snapshot_with_components(&["engine"]);
        let after = snapshot_with_components(&["engine"]);

        let diff = StateDiff::between(&before, &after);

        assert!(diff.is_empty());
        assert_eq!(diff.len(), 0);
        assert_eq!(diff.added_count(), 0);
        assert_eq!(diff.removed_count(), 0);
        assert_eq!(diff.modified_count(), 0);
    }

    #[test]
    fn added_component_is_detected() {
        let before = snapshot_with_components(&["engine"]);
        let after = snapshot_with_components(&["engine", "battery"]);

        let diff = StateDiff::between(&before, &after);

        assert_eq!(diff.len(), 1);
        assert_eq!(diff.added_count(), 1);
        assert_eq!(diff.removed_count(), 0);
        assert_eq!(diff.modified_count(), 0);

        let battery = identifier("battery");
        let change = diff.get(&battery).expect("battery must be changed");

        assert!(change.is_added());
        assert!(change.before().is_none());
        assert_eq!(
            change.after().expect("added component must have after state").name(),
            &battery
        );
    }

    #[test]
    fn removed_component_is_detected() {
        let before = snapshot_with_components(&["engine", "battery"]);
        let after = snapshot_with_components(&["engine"]);

        let diff = StateDiff::between(&before, &after);

        assert_eq!(diff.len(), 1);
        assert_eq!(diff.added_count(), 0);
        assert_eq!(diff.removed_count(), 1);
        assert_eq!(diff.modified_count(), 0);

        let battery = identifier("battery");
        let change = diff.get(&battery).expect("battery must be changed");

        assert!(change.is_removed());
        assert!(change.before().is_some());
        assert!(change.after().is_none());
    }

    #[test]
    fn modified_component_is_detected() {
        let before = snapshot_with_component("engine", "mass", 10.0);
        let after = snapshot_with_component("engine", "mass", 15.0);

        let diff = StateDiff::between(&before, &after);

        assert_eq!(diff.len(), 1);
        assert_eq!(diff.modified_count(), 1);

        let engine = identifier("engine");
        let change = diff.get(&engine).expect("engine must be changed");

        assert!(change.is_modified());

        let before_state = change.before().expect("modified state needs before");
        let after_state = change.after().expect("modified state needs after");

        assert_eq!(before_state.name(), &engine);
        assert_eq!(after_state.name(), &engine);
        assert_ne!(before_state, after_state);
    }

    #[test]
    fn unchanged_component_is_not_reported() {
        let before = snapshot_with_components(&["battery", "engine"]);
        let after = snapshot_with_components(&["battery", "engine"]);

        let diff = StateDiff::between(&before, &after);

        assert!(diff.is_empty());
    }

    #[test]
    fn changes_are_deterministically_ordered() {
        let before = snapshot_with_components(&[]);
        let after = snapshot_with_components(&[
            "motor",
            "battery",
            "controller",
            "actuator",
        ]);

        let diff = StateDiff::between(&before, &after);

        let names: Vec<&str> = diff
            .iter()
            .map(|change| change.name().as_str())
            .collect();

        assert_eq!(
            names,
            vec!["actuator", "battery", "controller", "motor"]
        );
    }

    #[test]
    fn get_uses_component_identity() {
        let before = snapshot_with_components(&["engine"]);
        let after = snapshot_with_components(&["engine", "battery"]);

        let diff = StateDiff::between(&before, &after);

        assert!(diff.contains(&identifier("battery")));
        assert!(!diff.contains(&identifier("engine")));
    }

    #[test]
    fn diff_direction_is_meaningful() {
        let before = snapshot_with_component("engine", "mass", 10.0);
        let after = snapshot_with_component("engine", "mass", 20.0);

        let forward = StateDiff::between(&before, &after);
        let reverse = StateDiff::between(&after, &before);

        let engine = identifier("engine");

        let forward_change = forward.get(&engine).expect("forward change");
        let reverse_change = reverse.get(&engine).expect("reverse change");

        let forward_before = forward_change
            .before()
            .expect("forward before state");

        let reverse_before = reverse_change
            .before()
            .expect("reverse before state");

        assert_ne!(forward_before, reverse_before);
    }

    #[test]
    fn diff_owns_changed_states() {
        let before = snapshot_with_component("engine", "mass", 10.0);
        let after = snapshot_with_component("engine", "mass", 20.0);

        let diff = StateDiff::between(&before, &after);

        let engine = identifier("engine");
        let change = diff.get(&engine).expect("engine must be changed");

        assert!(change.before().is_some());
        assert!(change.after().is_some());
    }
}
