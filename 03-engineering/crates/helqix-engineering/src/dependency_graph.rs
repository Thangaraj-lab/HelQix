//! Deterministic structural dependency graph for HELQIX Layer 03 —
//! Engineering.
//!
//! A [`DependencyGraph`] owns a set of named [`Dependency`] definitions.
//! It provides duplicate-safe insertion, deterministic iteration, and
//! structural validation of the contained dependencies.
//!
//! # Scope
//!
//! The graph is a **structural storage** container. It represents the
//! engineering dependency relationships defined at the model level:
//!
//! ```text
//! source  →  target
//! ```
//!
//! It is deliberately **not**:
//!
//! - a runtime event graph;
//! - a causal or failure graph;
//! - a physics, execution, or simulation graph;
//! - an optimization or AI graph;
//! - a temporal graph.
//!
//! Those concerns belong to later HELQIX layers.
//!
//! # What this file does not do
//!
//! - It does **not** detect cycles. A structural dependency cycle may be
//!   valid, invalid, or policy-dependent depending on the engineering
//!   model in question. That decision belongs to a higher validation
//!   layer, not to structural storage.
//! - It does **not** implement graph traversal, topological sorting,
//!   shortest paths, or transitive closure.
//! - It does **not** resolve [`DependencyReference`]s against concrete
//!   registries. Cross-domain reference resolution belongs to
//!   higher-level engineering validation.
//!
//! Ordering is deterministic via [`BTreeMap`]; the map key is always the
//! dependency's own [`Identifier`].

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

use crate::dependency::{Dependency, DependencyError};

/// Errors produced by [`DependencyGraph`] operations.
///
/// `DependencyGraphError` represents **graph-level** failures. Structural
/// failures of an individual [`Dependency`] are represented by
/// [`DependencyError`] and are wrapped in
/// [`DependencyGraphError::InvalidDependency`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyGraphError {
    /// A dependency with the same name is already present in the graph.
    DuplicateDependency {
        /// Name of the duplicated dependency.
        name: Identifier,
    },
    /// A stored dependency failed structural validation.
    InvalidDependency {
        /// Name of the invalid dependency.
        name: Identifier,
        /// The underlying dependency-level failure.
        source: DependencyError,
    },
}

impl fmt::Display for DependencyGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependencyGraphError::DuplicateDependency { name } => {
                write!(f, "duplicate dependency: {}", name.as_str())
            }
            DependencyGraphError::InvalidDependency { name, source } => {
                write!(f, "invalid dependency {}: {}", name.as_str(), source)
            }
        }
    }
}

impl Error for DependencyGraphError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DependencyGraphError::InvalidDependency { source, .. } => Some(source),
            DependencyGraphError::DuplicateDependency { .. } => None,
        }
    }
}

/// Deterministic collection of engineering [`Dependency`] definitions.
///
/// Backed by a [`BTreeMap`] keyed on [`Identifier`], giving deterministic
/// iteration order. Duplicate dependency names are rejected; existing
/// entries are never overwritten. Invalid dependencies are rejected at
/// insertion time, so the graph never contains an invalid dependency
/// through its public API.
///
/// An empty graph is structurally valid. `DependencyGraph` performs no
/// traversal, no cycle detection, and no cross-reference resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyGraph {
    dependencies: BTreeMap<Identifier, Dependency>,
}

impl DependencyGraph {
    /// Creates an empty dependency graph.
    pub fn new() -> Self {
        Self {
            dependencies: BTreeMap::new(),
        }
    }

    /// Returns the number of dependencies in the graph.
    pub fn len(&self) -> usize {
        self.dependencies.len()
    }

    /// Returns `true` if the graph contains no dependencies.
    pub fn is_empty(&self) -> bool {
        self.dependencies.is_empty()
    }

    /// Adds a dependency to the graph.
    ///
    /// The dependency's own identifier is used as the canonical key.
    /// Duplicate names are rejected before validation. On success the
    /// dependency is structurally validated and inserted; the graph is
    /// never mutated on failure.
    pub fn add(&mut self, dependency: Dependency) -> Result<(), DependencyGraphError> {
        let key = dependency.name().clone();

        if self.dependencies.contains_key(&key) {
            return Err(DependencyGraphError::DuplicateDependency { name: key });
        }

        dependency
            .validate()
            .map_err(|err| DependencyGraphError::InvalidDependency {
                name: dependency.name().clone(),
                source: err,
            })?;

        self.dependencies.insert(key, dependency);
        Ok(())
    }

    /// Returns an immutable reference to the dependency with the given name.
    pub fn get(&self, name: &Identifier) -> Option<&Dependency> {
        self.dependencies.get(name)
    }

    /// Returns a mutable reference to the dependency with the given name.
    ///
    /// The internal map is never exposed. Callers may mutate the stored
    /// [`Dependency`] through this reference; they are responsible for
    /// re-validating the graph afterwards. No hidden validation is
    /// performed here.
    pub fn get_mut(&mut self, name: &Identifier) -> Option<&mut Dependency> {
        self.dependencies.get_mut(name)
    }

    /// Returns `true` if a dependency with the given name is present.
    pub fn contains(&self, name: &Identifier) -> bool {
        self.dependencies.contains_key(name)
    }

    /// Removes and returns the dependency with the given name, if present.
    pub fn remove(&mut self, name: &Identifier) -> Option<Dependency> {
        self.dependencies.remove(name)
    }

    /// Iterates over `(name, dependency)` pairs in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &Dependency)> {
        self.dependencies.iter()
    }

    /// Iterates over dependencies in deterministic order.
    pub fn dependencies(&self) -> impl Iterator<Item = &Dependency> {
        self.dependencies.values()
    }

    /// Performs structural validation of every stored dependency.
    ///
    /// Dependencies are validated in the deterministic [`BTreeMap`] order.
    /// The first invalid dependency aborts validation with
    /// [`DependencyGraphError::InvalidDependency`]. No automatic repair or
    /// removal is performed.
    ///
    /// No cross-dependency analysis is performed here: neither cycle
    /// detection nor reference resolution.
    pub fn validate(&self) -> Result<(), DependencyGraphError> {
        for (name, dependency) in &self.dependencies {
            dependency
                .validate()
                .map_err(|err| DependencyGraphError::InvalidDependency {
                    name: name.clone(),
                    source: err,
                })?;
        }
        Ok(())
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::dependency::DependencyReference;

    fn ident(s: &str) -> Identifier {
        Identifier::parse(s).expect("known-valid identifier literal")
    }

    fn prop(name: &str) -> DependencyReference {
        DependencyReference::property(ident(name))
    }

    /// Builds a valid dependency with distinct source and target.
    fn valid_dependency(name: &str) -> Dependency {
        Dependency::new(
            ident(name),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid dependency")
    }

    /// Builds a dependency for the given name with source/target derived
    /// from the name so distinct names yield distinct endpoints.
    fn named_dependency(name: &str) -> Dependency {
        let source = format!("{}_source", name);
        let target = format!("{}_target", name);
        Dependency::new(
            ident(name),
            DependencyReference::property(ident(&source)),
            DependencyReference::property(ident(&target)),
        )
        .expect("valid dependency")
    }

    #[test]
    fn new_graph_is_empty() {
        let graph = DependencyGraph::new();
        assert_eq!(graph.len(), 0);
        assert!(graph.is_empty());
    }

    #[test]
    fn default_graph_is_empty() {
        let graph = DependencyGraph::default();
        assert_eq!(graph.len(), 0);
        assert!(graph.is_empty());
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn add_dependency() {
        let mut graph = DependencyGraph::new();
        graph
            .add(valid_dependency("thrust_dependency"))
            .expect("add succeeds");

        assert_eq!(graph.len(), 1);
        assert!(!graph.is_empty());
    }

    #[test]
    fn duplicate_dependency_is_rejected() {
        let mut graph = DependencyGraph::new();
        graph
            .add(valid_dependency("thrust_dependency"))
            .expect("first add succeeds");

        let err = graph
            .add(valid_dependency("thrust_dependency"))
            .expect_err("duplicate rejected");

        assert_eq!(
            err,
            DependencyGraphError::DuplicateDependency {
                name: ident("thrust_dependency"),
            }
        );
        assert_eq!(graph.len(), 1);
    }

    #[test]
    fn duplicate_does_not_replace_original() {
        let mut graph = DependencyGraph::new();
        let original = Dependency::new(
            ident("d"),
            prop("payload_mass"),
            prop("required_thrust"),
        )
        .expect("valid");
        graph.add(original).expect("first add succeeds");

        // Attempt to overwrite with a different dependency under the same
        // name.
        let replacement = Dependency::new(
            ident("d"),
            prop("motor_temperature"),
            prop("motor_efficiency"),
        )
        .expect("valid");
        let _ = graph.add(replacement);

        let stored = graph.get(&ident("d")).expect("present");
        assert_eq!(stored.source_name().as_str(), "payload_mass");
        assert_eq!(stored.target_name().as_str(), "required_thrust");
        assert_eq!(graph.len(), 1);
    }

    #[test]
    fn get_dependency() {
        let mut graph = DependencyGraph::new();
        graph
            .add(valid_dependency("thrust_dependency"))
            .expect("add succeeds");

        let found = graph
            .get(&ident("thrust_dependency"))
            .expect("present");
        assert_eq!(found.name().as_str(), "thrust_dependency");

        assert!(graph.get(&ident("missing_dependency")).is_none());
    }

    #[test]
    fn get_mut_allows_controlled_mutation() {
        let mut graph = DependencyGraph::new();
        graph
            .add(valid_dependency("thrust_dependency"))
            .expect("add succeeds");

        let stored = graph
            .get_mut(&ident("thrust_dependency"))
            .expect("mutable present");
        assert_eq!(stored.source_name().as_str(), "payload_mass");

        assert!(graph.get_mut(&ident("missing")).is_none());
    }

    #[test]
    fn contains_dependency() {
        let mut graph = DependencyGraph::new();
        graph
            .add(valid_dependency("thrust_dependency"))
            .expect("add succeeds");

        assert!(graph.contains(&ident("thrust_dependency")));
        assert!(!graph.contains(&ident("power_dependency")));
    }

    #[test]
    fn remove_dependency() {
        let mut graph = DependencyGraph::new();
        graph
            .add(valid_dependency("thrust_dependency"))
            .expect("add succeeds");

        let removed = graph
            .remove(&ident("thrust_dependency"))
            .expect("removed");
        assert_eq!(removed.name().as_str(), "thrust_dependency");
        assert!(graph.is_empty());
    }

    #[test]
    fn missing_dependency_returns_none() {
        let mut graph = DependencyGraph::new();
        assert!(graph.remove(&ident("thrust_dependency")).is_none());
    }

    #[test]
    fn deterministic_iteration_order() {
        let mut graph = DependencyGraph::new();
        graph
            .add(named_dependency("zeta"))
            .expect("add zeta");
        graph
            .add(named_dependency("alpha"))
            .expect("add alpha");
        graph
            .add(named_dependency("gamma"))
            .expect("add gamma");

        let order: Vec<&str> = graph.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(order, vec!["alpha", "gamma", "zeta"]);

        let deps_order: Vec<&str> =
            graph.dependencies().map(|d| d.name().as_str()).collect();
        assert_eq!(deps_order, vec!["alpha", "gamma", "zeta"]);
    }

    #[test]
    fn empty_graph_is_valid() {
        let graph = DependencyGraph::new();
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn valid_dependencies_pass_validation() {
        let mut graph = DependencyGraph::new();
        graph
            .add(named_dependency("alpha"))
            .expect("add alpha");
        graph
            .add(named_dependency("beta"))
            .expect("add beta");

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn invalid_dependency_is_reported() {
        // Construct a dependency whose source and target are identical so
        // that `Dependency::new` itself rejects it.
        let same = prop("temperature");
        let invalid_result = Dependency::new(ident("self_dep"), same.clone(), same);
        assert!(invalid_result.is_err());

        // Since `Dependency::new` already rejects self-dependencies, an
        // invalid `Dependency` cannot enter the graph through the public
        // API. Verify that the graph remains empty after such an attempt.
        let mut graph = DependencyGraph::new();
        if let Ok(invalid) = invalid_result {
            let _ = graph.add(invalid);
        }
        assert!(graph.is_empty());
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn failed_add_does_not_mutate_graph() {
        let mut graph = DependencyGraph::new();
        graph
            .add(valid_dependency("a"))
            .expect("add a");

        // A duplicate attempt (which is the reachable public failure
        // mode) must not mutate the graph.
        let _ = graph.add(valid_dependency("a"));

        assert_eq!(graph.len(), 1);
        let stored = graph.get(&ident("a")).expect("present");
        assert_eq!(stored.source_name().as_str(), "payload_mass");
        assert_eq!(stored.target_name().as_str(), "required_thrust");
    }

    #[test]
    fn equality_behaviour() {
        let mut a = DependencyGraph::new();
        a.add(named_dependency("alpha")).expect("add alpha");
        a.add(named_dependency("beta")).expect("add beta");

        let mut b = DependencyGraph::new();
        // Insert in opposite order; BTreeMap guarantees deterministic
        // ordering regardless of insertion sequence.
        b.add(named_dependency("beta")).expect("add beta");
        b.add(named_dependency("alpha")).expect("add alpha");

        assert_eq!(a, b);

        let mut c = DependencyGraph::new();
        c.add(named_dependency("alpha")).expect("add alpha");
        assert_ne!(a, c);
    }
}