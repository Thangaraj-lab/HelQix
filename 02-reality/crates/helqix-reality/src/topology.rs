//! Structural topology of an engineering system.
//!
//! Topology is the engineering-semantic connection registry of the Reality
//! layer. It records which component connects to which component and what
//! kind of structural connection exists between them. It is purely
//! structural: it performs no physics, simulation, execution, or graph
//! traversal, and it never owns `Component` values. Component ownership
//! remains with `ComponentRegistry`.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;

use crate::component_registry::ComponentRegistry;

/// Structural classification of a topology connection.
///
/// This is a classification only. It carries no physical behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConnectionKind {
    /// Generic structural attachment.
    Structural,
    /// Power distribution relationship.
    Power,
    /// Signal relationship.
    Signal,
    /// Data relationship.
    Data,
    /// Mechanical relationship.
    Mechanical,
    /// Thermal relationship.
    Thermal,
    /// Fluid relationship.
    Fluid,
    /// Electrical relationship.
    Electrical,
}

impl fmt::Display for ConnectionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            ConnectionKind::Structural => "Structural",
            ConnectionKind::Power => "Power",
            ConnectionKind::Signal => "Signal",
            ConnectionKind::Data => "Data",
            ConnectionKind::Mechanical => "Mechanical",
            ConnectionKind::Thermal => "Thermal",
            ConnectionKind::Fluid => "Fluid",
            ConnectionKind::Electrical => "Electrical",
        };
        f.write_str(text)
    }
}

/// Deterministic structural identity of a topology connection.
///
/// A connection is uniquely identified by `source + target + kind`.
/// Ordering is lexicographic in that field order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TopologyConnectionKey {
    source: Identifier,
    target: Identifier,
    kind: ConnectionKind,
}

impl TopologyConnectionKey {
    /// Creates a key from source, target, and kind.
    pub fn new(source: Identifier, target: Identifier, kind: ConnectionKind) -> Self {
        Self {
            source,
            target,
            kind,
        }
    }

    /// Returns the source component identity.
    pub fn source(&self) -> &Identifier {
        &self.source
    }

    /// Returns the target component identity.
    pub fn target(&self) -> &Identifier {
        &self.target
    }

    /// Returns the connection kind.
    pub fn kind(&self) -> ConnectionKind {
        self.kind
    }
}

/// A directed, typed structural connection between two components.
///
/// Stores component identities only; it never owns components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyConnection {
    source: Identifier,
    target: Identifier,
    kind: ConnectionKind,
}

impl TopologyConnection {
    /// Creates a connection from source, target, and kind.
    pub fn new(source: Identifier, target: Identifier, kind: ConnectionKind) -> Self {
        Self {
            source,
            target,
            kind,
        }
    }

    /// Returns the source component identity.
    pub fn source(&self) -> &Identifier {
        &self.source
    }

    /// Returns the target component identity.
    pub fn target(&self) -> &Identifier {
        &self.target
    }

    /// Returns the connection kind.
    pub fn kind(&self) -> ConnectionKind {
        self.kind
    }

    /// Returns the exact structural key of this connection.
    pub fn key(&self) -> TopologyConnectionKey {
        TopologyConnectionKey::new(self.source.clone(), self.target.clone(), self.kind)
    }
}

/// Errors produced by topology registration and validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyError {
    /// An identical `source + target + kind` connection already exists.
    DuplicateConnection {
        /// Source component name.
        source: String,
        /// Target component name.
        target: String,
        /// Connection kind.
        kind: ConnectionKind,
    },
    /// The source component does not exist in the component registry.
    MissingSourceComponent {
        /// Missing component name.
        name: String,
    },
    /// The target component does not exist in the component registry.
    MissingTargetComponent {
        /// Missing component name.
        name: String,
    },
    /// A component attempted to connect to itself.
    SelfConnection {
        /// Component name.
        name: String,
    },
}

impl fmt::Display for TopologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TopologyError::DuplicateConnection {
                source,
                target,
                kind,
            } => write!(
                f,
                "duplicate topology connection: {source} -> {target} ({kind})"
            ),
            TopologyError::MissingSourceComponent { name } => {
                write!(f, "missing source component: {name}")
            }
            TopologyError::MissingTargetComponent { name } => {
                write!(f, "missing target component: {name}")
            }
            TopologyError::SelfConnection { name } => {
                write!(f, "component cannot connect to itself: {name}")
            }
        }
    }
}

impl Error for TopologyError {}

/// Engineering-semantic connection registry.
///
/// Holds typed connections between component identities. It is separate from
/// the generic `RealityGraph` and is not synchronized with it automatically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topology {
    connections: BTreeMap<TopologyConnectionKey, TopologyConnection>,
}

impl Topology {
    /// Creates an empty topology.
    pub fn new() -> Self {
        Self {
            connections: BTreeMap::new(),
        }
    }

    /// Returns the number of connections.
    pub fn len(&self) -> usize {
        self.connections.len()
    }

    /// Returns `true` if the topology has no connections.
    pub fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }

    /// Registers a connection using topology identities only.
    ///
    /// Rejects self-connections and duplicate structural keys. Component
    /// existence is not checked here; see [`Topology::connect_with_registry`].
    pub fn connect(
        &mut self,
        source: Identifier,
        target: Identifier,
        kind: ConnectionKind,
    ) -> Result<(), TopologyError> {
        if source == target {
            return Err(TopologyError::SelfConnection {
                name: source.as_str().to_owned(),
            });
        }

        let connection = TopologyConnection::new(source, target, kind);
        let key = connection.key();

        if self.connections.contains_key(&key) {
            return Err(TopologyError::DuplicateConnection {
                source: connection.source().as_str().to_owned(),
                target: connection.target().as_str().to_owned(),
                kind,
            });
        }

        self.connections.insert(key, connection);
        Ok(())
    }

    /// Registers a connection after verifying both endpoints exist in the
    /// component registry. The registry is never modified.
    pub fn connect_with_registry(
        &mut self,
        source: Identifier,
        target: Identifier,
        kind: ConnectionKind,
        registry: &ComponentRegistry,
    ) -> Result<(), TopologyError> {
        if source == target {
            return Err(TopologyError::SelfConnection {
                name: source.as_str().to_owned(),
            });
        }

        if !registry.contains(&source) {
            return Err(TopologyError::MissingSourceComponent {
                name: source.as_str().to_owned(),
            });
        }

        if !registry.contains(&target) {
            return Err(TopologyError::MissingTargetComponent {
                name: target.as_str().to_owned(),
            });
        }

        self.connect(source, target, kind)
    }

    /// Returns `true` if the exact connection exists.
    pub fn contains(
        &self,
        source: &Identifier,
        target: &Identifier,
        kind: ConnectionKind,
    ) -> bool {
        self.get(source, target, kind).is_some()
    }

    /// Returns the exact connection if it exists.
    pub fn get(
        &self,
        source: &Identifier,
        target: &Identifier,
        kind: ConnectionKind,
    ) -> Option<&TopologyConnection> {
        let key = TopologyConnectionKey::new(source.clone(), target.clone(), kind);
        self.connections.get(&key)
    }

    /// Removes exactly one connection. Returns `true` if it existed.
    ///
    /// The reverse direction and other kinds are untouched.
    pub fn disconnect(
        &mut self,
        source: &Identifier,
        target: &Identifier,
        kind: ConnectionKind,
    ) -> bool {
        let key = TopologyConnectionKey::new(source.clone(), target.clone(), kind);
        self.connections.remove(&key).is_some()
    }

    /// Iterates keys and connections in deterministic key order.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&TopologyConnectionKey, &TopologyConnection)> {
        self.connections.iter()
    }

    /// Iterates connections in deterministic key order.
    pub fn connections(&self) -> impl Iterator<Item = &TopologyConnection> {
        self.connections.values()
    }

    /// Iterates connections whose source is `source`, in deterministic order.
    pub fn outgoing<'a>(
        &'a self,
        source: &'a Identifier,
    ) -> impl Iterator<Item = &'a TopologyConnection> + 'a {
        self.connections
            .values()
            .filter(move |connection| connection.source() == source)
    }

    /// Iterates connections whose target is `target`, in deterministic order.
    pub fn incoming<'a>(
        &'a self,
        target: &'a Identifier,
    ) -> impl Iterator<Item = &'a TopologyConnection> + 'a {
        self.connections
            .values()
            .filter(move |connection| connection.target() == target)
    }

    /// Removes every connection incident to `component`.
    ///
    /// Only topology relationships are removed; the component registry is
    /// not affected.
    pub fn remove_component(&mut self, component: &Identifier) {
        self.connections.retain(|_, connection| {
            connection.source() != component && connection.target() != component
        });
    }

    /// Validates structural consistency against the component registry.
    ///
    /// Checks for self-connections and missing endpoints. Performs no
    /// physical or behavioral validation.
    pub fn validate(&self, registry: &ComponentRegistry) -> Result<(), TopologyError> {
        for connection in self.connections.values() {
            if connection.source() == connection.target() {
                return Err(TopologyError::SelfConnection {
                    name: connection.source().as_str().to_owned(),
                });
            }

            if !registry.contains(connection.source()) {
                return Err(TopologyError::MissingSourceComponent {
                    name: connection.source().as_str().to_owned(),
                });
            }

            if !registry.contains(connection.target()) {
                return Err(TopologyError::MissingTargetComponent {
                    name: connection.target().as_str().to_owned(),
                });
            }
        }

        Ok(())
    }
}

impl Default for Topology {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helqix_core::component::Component;

    fn id(value: &str) -> Identifier {
        Identifier::parse(value).expect("valid identifier")
    }

    // Single construction point for test components. Adjust here if the
    // actual `Component::new` signature in helqix_core differs.
    fn component(name: &str) -> Component {
        Component::new(id(name))
    }

    fn registry_with(names: &[&str]) -> ComponentRegistry {
        let mut registry = ComponentRegistry::new();
        for name in names {
            let _ = registry.register(component(name));
        }
        registry
    }

    #[test]
    fn new_topology_is_empty() {
        let topology = Topology::new();
        assert_eq!(topology.len(), 0);
        assert!(topology.is_empty());
    }

    #[test]
    fn connection_key_is_deterministic() {
        let a = TopologyConnectionKey::new(id("A"), id("B"), ConnectionKind::Power);
        let b = TopologyConnectionKey::new(id("A"), id("B"), ConnectionKind::Power);
        assert_eq!(a, b);
    }

    #[test]
    fn connection_exposes_source_target_kind() {
        let connection = TopologyConnection::new(id("A"), id("B"), ConnectionKind::Data);
        assert_eq!(connection.source(), &id("A"));
        assert_eq!(connection.target(), &id("B"));
        assert_eq!(connection.kind(), ConnectionKind::Data);
        assert_eq!(
            connection.key(),
            TopologyConnectionKey::new(id("A"), id("B"), ConnectionKind::Data)
        );
    }

    #[test]
    fn connect_adds_connection() {
        let mut topology = Topology::new();
        topology
            .connect(id("Motor"), id("Controller"), ConnectionKind::Power)
            .unwrap();

        assert_eq!(topology.len(), 1);
        assert!(topology.contains(&id("Motor"), &id("Controller"), ConnectionKind::Power));

        let connection = topology
            .get(&id("Motor"), &id("Controller"), ConnectionKind::Power)
            .unwrap();
        assert_eq!(connection.source(), &id("Motor"));
        assert_eq!(connection.target(), &id("Controller"));
        assert_eq!(connection.kind(), ConnectionKind::Power);
    }

    #[test]
    fn duplicate_connection_is_rejected() {
        let mut topology = Topology::new();
        topology
            .connect(id("A"), id("B"), ConnectionKind::Power)
            .unwrap();

        let result = topology.connect(id("A"), id("B"), ConnectionKind::Power);
        assert_eq!(
            result,
            Err(TopologyError::DuplicateConnection {
                source: "A".to_owned(),
                target: "B".to_owned(),
                kind: ConnectionKind::Power,
            })
        );
        assert_eq!(topology.len(), 1);
    }

    #[test]
    fn self_connection_is_rejected() {
        let mut topology = Topology::new();
        let result = topology.connect(id("Motor"), id("Motor"), ConnectionKind::Power);
        assert_eq!(
            result,
            Err(TopologyError::SelfConnection {
                name: "Motor".to_owned(),
            })
        );
        assert!(topology.is_empty());
    }

    #[test]
    fn reverse_connections_are_distinct() {
        let mut topology = Topology::new();
        topology
            .connect(id("A"), id("B"), ConnectionKind::Power)
            .unwrap();
        topology
            .connect(id("B"), id("A"), ConnectionKind::Power)
            .unwrap();

        assert_eq!(topology.len(), 2);
        assert!(topology.contains(&id("A"), &id("B"), ConnectionKind::Power));
        assert!(topology.contains(&id("B"), &id("A"), ConnectionKind::Power));
    }

    #[test]
    fn different_kinds_are_distinct() {
        let mut topology = Topology::new();
        topology
            .connect(id("A"), id("B"), ConnectionKind::Power)
            .unwrap();
        topology
            .connect(id("A"), id("B"), ConnectionKind::Signal)
            .unwrap();

        assert_eq!(topology.len(), 2);
        assert!(topology.contains(&id("A"), &id("B"), ConnectionKind::Power));
        assert!(topology.contains(&id("A"), &id("B"), ConnectionKind::Signal));
    }

    #[test]
    fn disconnect_removes_exact_connection() {
        let mut topology = Topology::new();
        topology
            .connect(id("A"), id("B"), ConnectionKind::Power)
            .unwrap();
        topology
            .connect(id("A"), id("B"), ConnectionKind::Signal)
            .unwrap();
        topology
            .connect(id("B"), id("C"), ConnectionKind::Data)
            .unwrap();

        assert!(topology.disconnect(&id("A"), &id("B"), ConnectionKind::Power));
        assert!(!topology.disconnect(&id("A"), &id("B"), ConnectionKind::Power));

        assert_eq!(topology.len(), 2);
        assert!(!topology.contains(&id("A"), &id("B"), ConnectionKind::Power));
        assert!(topology.contains(&id("A"), &id("B"), ConnectionKind::Signal));
        assert!(topology.contains(&id("B"), &id("C"), ConnectionKind::Data));
    }

    #[test]
    fn outgoing_returns_correct_connections() {
        let mut topology = Topology::new();
        topology
            .connect(id("A"), id("B"), ConnectionKind::Power)
            .unwrap();
        topology
            .connect(id("A"), id("C"), ConnectionKind::Signal)
            .unwrap();
        topology
            .connect(id("B"), id("D"), ConnectionKind::Data)
            .unwrap();

        let source = id("A");
        let result: Vec<_> = topology.outgoing(&source).cloned().collect();
        assert_eq!(
            result,
            vec![
                TopologyConnection::new(id("A"), id("B"), ConnectionKind::Power),
                TopologyConnection::new(id("A"), id("C"), ConnectionKind::Signal),
            ]
        );
    }

    #[test]
    fn incoming_returns_correct_connections() {
        let mut topology = Topology::new();
        topology
            .connect(id("A"), id("C"), ConnectionKind::Power)
            .unwrap();
        topology
            .connect(id("B"), id("C"), ConnectionKind::Signal)
            .unwrap();
        topology
            .connect(id("C"), id("D"), ConnectionKind::Data)
            .unwrap();

        let target = id("C");
        let result: Vec<_> = topology.incoming(&target).cloned().collect();
        assert_eq!(
            result,
            vec![
                TopologyConnection::new(id("A"), id("C"), ConnectionKind::Power),
                TopologyConnection::new(id("B"), id("C"), ConnectionKind::Signal),
            ]
        );
    }

    #[test]
    fn remove_component_removes_incident_connections() {
        let mut topology = Topology::new();
        topology
            .connect(id("A"), id("B"), ConnectionKind::Power)
            .unwrap();
        topology
            .connect(id("B"), id("C"), ConnectionKind::Power)
            .unwrap();
        topology
            .connect(id("C"), id("D"), ConnectionKind::Power)
            .unwrap();

        topology.remove_component(&id("B"));

        assert!(!topology.contains(&id("A"), &id("B"), ConnectionKind::Power));
        assert!(!topology.contains(&id("B"), &id("C"), ConnectionKind::Power));
        assert!(topology.contains(&id("C"), &id("D"), ConnectionKind::Power));
        assert_eq!(topology.len(), 1);
    }

    #[test]
    fn connect_with_registry_requires_source() {
        let registry = registry_with(&["Target"]);
        let mut topology = Topology::new();

        let result = topology.connect_with_registry(
            id("Source"),
            id("Target"),
            ConnectionKind::Power,
            &registry,
        );
        assert_eq!(
            result,
            Err(TopologyError::MissingSourceComponent {
                name: "Source".to_owned(),
            })
        );
        assert!(topology.is_empty());
    }

    #[test]
    fn connect_with_registry_requires_target() {
        let registry = registry_with(&["Source"]);
        let mut topology = Topology::new();

        let result = topology.connect_with_registry(
            id("Source"),
            id("Target"),
            ConnectionKind::Power,
            &registry,
        );
        assert_eq!(
            result,
            Err(TopologyError::MissingTargetComponent {
                name: "Target".to_owned(),
            })
        );
        assert!(topology.is_empty());
    }

    #[test]
    fn connect_with_registry_accepts_existing_components() {
        let registry = registry_with(&["Motor", "Controller"]);
        let mut topology = Topology::new();

        let result = topology.connect_with_registry(
            id("Motor"),
            id("Controller"),
            ConnectionKind::Power,
            &registry,
        );
        assert_eq!(result, Ok(()));
        assert!(topology.contains(&id("Motor"), &id("Controller"), ConnectionKind::Power));
    }

    #[test]
    fn validate_accepts_consistent_topology() {
        let registry = registry_with(&["Motor", "Controller", "Battery"]);
        let mut topology = Topology::new();
        topology
            .connect_with_registry(
                id("Battery"),
                id("Controller"),
                ConnectionKind::Power,
                &registry,
            )
            .unwrap();
        topology
            .connect_with_registry(
                id("Controller"),
                id("Motor"),
                ConnectionKind::Signal,
                &registry,
            )
            .unwrap();

        assert_eq!(topology.validate(&registry), Ok(()));
    }

    #[test]
    fn validate_detects_missing_component() {
        let mut topology = Topology::new();
        topology
            .connect(id("Motor"), id("Controller"), ConnectionKind::Power)
            .unwrap();

        let missing_source = registry_with(&["Controller"]);
        assert_eq!(
            topology.validate(&missing_source),
            Err(TopologyError::MissingSourceComponent {
                name: "Motor".to_owned(),
            })
        );

        let missing_target = registry_with(&["Motor"]);
        assert_eq!(
            topology.validate(&missing_target),
            Err(TopologyError::MissingTargetComponent {
                name: "Controller".to_owned(),
            })
        );
    }

    #[test]
    fn deterministic_connection_iteration() {
        let mut topology = Topology::new();
        topology
            .connect(id("Zeta"), id("Motor"), ConnectionKind::Power)
            .unwrap();
        topology
            .connect(id("Alpha"), id("Battery"), ConnectionKind::Power)
            .unwrap();
        topology
            .connect(id("Motor"), id("Controller"), ConnectionKind::Power)
            .unwrap();

        let keys: Vec<TopologyConnectionKey> =
            topology.iter().map(|(key, _)| key.clone()).collect();
        assert_eq!(
            keys,
            vec![
                TopologyConnectionKey::new(id("Alpha"), id("Battery"), ConnectionKind::Power),
                TopologyConnectionKey::new(id("Motor"), id("Controller"), ConnectionKind::Power),
                TopologyConnectionKey::new(id("Zeta"), id("Motor"), ConnectionKind::Power),
            ]
        );

        let sources: Vec<&str> = topology
            .connections()
            .map(|connection| connection.source().as_str())
            .collect();
        assert_eq!(sources, vec!["Alpha", "Motor", "Zeta"]);
    }

    #[test]
    fn registry_is_not_mutated_by_connect() {
        let registry = registry_with(&["Motor", "Controller"]);
        let before = registry.len();

        let mut topology = Topology::new();
        topology
            .connect_with_registry(
                id("Motor"),
                id("Controller"),
                ConnectionKind::Power,
                &registry,
            )
            .unwrap();

        assert_eq!(registry.len(), before);
    }

    #[test]
    fn disconnect_does_not_remove_reverse_connection() {
        let mut topology = Topology::new();
        topology
            .connect(id("A"), id("B"), ConnectionKind::Power)
            .unwrap();
        topology
            .connect(id("B"), id("A"), ConnectionKind::Power)
            .unwrap();

        assert!(topology.disconnect(&id("A"), &id("B"), ConnectionKind::Power));
        assert!(!topology.contains(&id("A"), &id("B"), ConnectionKind::Power));
        assert!(topology.contains(&id("B"), &id("A"), ConnectionKind::Power));
    }
}