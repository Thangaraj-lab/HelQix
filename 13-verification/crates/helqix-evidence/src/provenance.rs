//! Provenance chains for HELQIX verification evidence.
//!
//! Provenance answers one question: *"where did this result come from?"*
//!
//! ```text
//! Requirement R-17
//!     ↓
//! Execution E-204
//!     ↓
//! State Snapshot S-812
//!     ↓
//! Observation O-421
//!     ↓
//! Verification V-88
//!     ↓
//! Evidence EV-91
//! ```
//!
//! A [`Provenance`] is an ordered chain of typed [`ProvenanceNode`]s, each
//! wrapping a validated canonical identifier ([`Identifier`]) plus the
//! canonical Layer 10 logical time at which that step occurred (when known).
//!
//! This module deliberately does NOT:
//! - implement Git-like versioning or a project/version system;
//! - compute content hashes (the workspace has no hashing primitive; see
//!   `reproducibility.rs` for the contract-based alternative);
//! - generate ids implicitly — producers assign every identity.
//!
//! Determinism: nodes are appended explicitly and compared by value; chain
//! order is part of the data, never derived from maps or clocks.

use std::error::Error;
use std::fmt;

use helqix_core::error::IdentifierError;
use helqix_core::id::Identifier;
use helqix_events::event::LogicalTime;

/// The kind of engineering artifact a provenance node refers to.
///
/// Each variant names the owning HELQIX layer so an auditor can resolve the
/// reference against the authoritative source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProvenanceKind {
    /// An engineering requirement (`helqix-verify`, Layer 13).
    Requirement,
    /// A constraint definition (Layer 01/03).
    Constraint,
    /// An execution (Layer 05), referenced by identity.
    Execution,
    /// A state snapshot (Layer 04 `StateSnapshot`), referenced by identity.
    StateSnapshot,
    /// An observation/metric sample (Layer 08).
    Observation,
    /// An event (Layer 10).
    Event,
    /// A branch (Layer 11).
    Branch,
    /// A search candidate (Layer 12).
    Candidate,
    /// A verification verdict (`helqix-verify`, Layer 13).
    Verification,
    /// An evidence object (this crate, Layer 13).
    Evidence,
}

impl ProvenanceKind {
    /// Returns the stable machine-readable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Requirement => "requirement",
            Self::Constraint => "constraint",
            Self::Execution => "execution",
            Self::StateSnapshot => "state_snapshot",
            Self::Observation => "observation",
            Self::Event => "event",
            Self::Branch => "branch",
            Self::Candidate => "candidate",
            Self::Verification => "verification",
            Self::Evidence => "evidence",
        }
    }
}

impl fmt::Display for ProvenanceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One traced step in a provenance chain.
///
/// Identity fields use the canonical Foundation [`Identifier`]; the optional
/// [`LogicalTime`] records where the step sits on the execution timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceNode {
    kind: ProvenanceKind,
    id: Identifier,
    time: Option<LogicalTime>,
}

impl ProvenanceNode {
    /// Builds a node referencing `id_text` of `kind`.
    ///
    /// # Errors
    ///
    /// Returns [`ProvenanceError::InvalidIdentifier`] when `id_text` violates
    /// the Foundation grammar.
    pub fn new(kind: ProvenanceKind, id_text: &str) -> Result<Self, ProvenanceError> {
        Ok(Self {
            kind,
            id: parse_identifier(id_text)?,
            time: None,
        })
    }

    /// Builds a node with an explicit canonical logical time.
    ///
    /// # Errors
    ///
    /// As [`ProvenanceNode::new`].
    pub fn with_time(
        kind: ProvenanceKind,
        id_text: &str,
        time: LogicalTime,
    ) -> Result<Self, ProvenanceError> {
        Ok(Self {
            kind,
            id: parse_identifier(id_text)?,
            time: Some(time),
        })
    }

    /// Wraps an already-validated identifier.
    #[must_use]
    pub const fn from_identifier(kind: ProvenanceKind, id: Identifier) -> Self {
        Self {
            kind,
            id,
            time: None,
        }
    }

    /// Returns the referenced artifact kind.
    #[must_use]
    pub const fn kind(&self) -> ProvenanceKind {
        self.kind
    }

    /// Returns the canonical identifier of the referenced artifact.
    #[must_use]
    pub fn id(&self) -> &Identifier {
        &self.id
    }

    /// Returns the recorded logical time, when the producer supplied one.
    #[must_use]
    pub const fn time(&self) -> Option<LogicalTime> {
        self.time
    }
}

impl fmt::Display for ProvenanceNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} `{}`", self.kind, self.id)?;
        if let Some(time) = self.time {
            write!(f, " @ {}", time)?;
        }
        Ok(())
    }
}

/// An ordered, immutable-by-construction provenance chain.
///
/// The chain records *what produced what*. It references artifacts by
/// canonical identity only; it never embeds their contents, so provenance
/// stays small and cannot drift from the authoritative sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    nodes: Vec<ProvenanceNode>,
}

/// Maximum chain length, keeping audit traversal cost bounded.
pub const MAX_PROVENANCE_NODES: usize = 256;

impl Provenance {
    /// Starts a chain from its first node.
    #[must_use]
    pub fn start(first: ProvenanceNode) -> Self {
        Self {
            nodes: vec![first],
        }
    }

    /// Returns the number of nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns `true` when the chain is empty (only possible via
    /// [`Provenance::empty`] for serialization round-trips).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Appends a downstream node to the chain.
    ///
    /// # Errors
    ///
    /// Returns [`ProvenanceError::ChainTooLong`] past
    /// [`MAX_PROVENANCE_NODES`].
    pub fn pushed(mut self, node: ProvenanceNode) -> Result<Self, ProvenanceError> {
        if self.nodes.len() >= MAX_PROVENANCE_NODES {
            return Err(ProvenanceError::ChainTooLong {
                limit: MAX_PROVENANCE_NODES,
            });
        }
        self.nodes.push(node);
        Ok(self)
    }

    /// Returns the nodes in recorded order.
    pub fn nodes(&self) -> &[ProvenanceNode] {
        &self.nodes
    }

    /// Returns the first node, if any.
    #[must_use]
    pub fn first(&self) -> Option<&ProvenanceNode> {
        self.nodes.first()
    }

    /// Returns the last (most downstream) node, if any.
    #[must_use]
    pub fn last(&self) -> Option<&ProvenanceNode> {
        self.nodes.last()
    }

    /// Returns whether the chain references `kind` with identifier `id_text`.
    #[must_use]
    pub fn contains(&self, kind: ProvenanceKind, id_text: &str) -> bool {
        self.nodes
            .iter()
            .any(|node| node.kind() == kind && node.id().as_str() == id_text)
    }

    /// Rebuilds a chain from stored nodes without re-validating identities
    /// (identities were validated when the nodes were first created).
    #[must_use]
    pub(crate) const fn from_nodes(nodes: Vec<ProvenanceNode>) -> Self {
        Self { nodes }
    }
}

/// Errors produced while building provenance chains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceError {
    /// Identifier text violated the Foundation grammar.
    InvalidIdentifier {
        /// The rejected text.
        text: String,
        /// Underlying Foundation error.
        source: IdentifierError,
    },
    /// The chain exceeded [`MAX_PROVENANCE_NODES`].
    ChainTooLong {
        /// Configured limit.
        limit: usize,
    },
}

impl fmt::Display for ProvenanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { text, source } => {
                write!(f, "invalid provenance identifier `{text}`: {source}")
            }
            Self::ChainTooLong { limit } => {
                write!(f, "provenance chain exceeds {limit} nodes")
            }
        }
    }
}

impl Error for ProvenanceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidIdentifier { source, .. } => Some(source),
            Self::ChainTooLong { .. } => None,
        }
    }
}

/// Validates identifier text against the Foundation grammar.
pub(crate) fn parse_identifier(text: &str) -> Result<Identifier, ProvenanceError> {
    Identifier::parse(text).map_err(|source| ProvenanceError::InvalidIdentifier {
        text: text.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_requirement_to_evidence_chain() {
        let chain = Provenance::start(
            ProvenanceNode::new(ProvenanceKind::Requirement, "r_17").unwrap(),
        )
        .pushed(ProvenanceNode::with_time(
            ProvenanceKind::Execution,
            "exec_204",
            LogicalTime::from_nanos(12_480_000_000),
        )
        .unwrap())
        .unwrap()
        .pushed(ProvenanceNode::new(ProvenanceKind::Evidence, "ev_91").unwrap())
        .unwrap();

        assert_eq!(chain.len(), 3);
        assert!(chain.contains(ProvenanceKind::Requirement, "r_17"));
        assert!(!chain.contains(ProvenanceKind::Requirement, "r_18"));
        assert_eq!(chain.last().unwrap().kind(), ProvenanceKind::Evidence);
    }

    #[test]
    fn rejects_invalid_identifiers() {
        let error = ProvenanceNode::new(ProvenanceKind::Execution, "204-exec").unwrap_err();
        assert!(matches!(
            error,
            ProvenanceError::InvalidIdentifier { .. }
        ));
    }

    #[test]
    fn chain_order_is_part_of_value() {
        let a = Provenance::start(ProvenanceNode::new(ProvenanceKind::Requirement, "r").unwrap())
            .pushed(ProvenanceNode::new(ProvenanceKind::Execution, "e").unwrap())
            .unwrap();
        let b = Provenance::start(ProvenanceNode::new(ProvenanceKind::Execution, "e").unwrap())
            .pushed(ProvenanceNode::new(ProvenanceKind::Requirement, "r").unwrap())
            .unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn display_is_stable_and_readable() {
        let node = ProvenanceNode::with_time(
            ProvenanceKind::StateSnapshot,
            "s_812",
            LogicalTime::from_nanos(5_000_000_000),
        )
        .unwrap();
        assert_eq!(node.to_string(), "state_snapshot `s_812` @ t=5000000000ns");
    }
}
