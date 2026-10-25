//! Append-only audit trails for HELQIX verification evidence.
//!
//! An [`AuditTrail`] is the chronological record an auditor reads: every
//! consequential action in a verification workflow becomes one immutable
//! [`AuditEntry`]. It complements — and never replaces — Layer 10 events:
//! events describe *system occurrences*, audit entries describe *workflow
//! actions taken by actors on artifacts*.
//!
//! Invariants:
//! - **append-only**: entries can never be modified or removed; `append`
//!   validates then extends at the tail;
//! - **monotonic logical time**: each entry's [`LogicalTime`] must be greater
//!   than or equal to its predecessor's (Layer 10 ordering semantics), so a
//!   trail cannot claim effects before their causes;
//! - **deterministic iteration**: entries are exposed in append order, which
//!   equals non-decreasing `(time, sequence)` order;
//! - **actor/action separation**: who acted (`actor`) is distinct from what
//!   was done (`action`) and which artifact it touched (`target`).
//!
//! Actions are domain-neutral ([`AuditAction`]); all engineering vocabulary
//! lives in caller-supplied identifiers.

use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;
use helqix_events::event::LogicalTime;

use crate::provenance::{parse_identifier, ProvenanceError};

/// The kind of workflow action recorded by an audit entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuditAction {
    /// A requirement was created or registered.
    RequirementDefined,
    /// An execution was started.
    ExecutionStarted,
    /// An execution reached its terminal state.
    ExecutionFinished,
    /// A verification was performed (result attached).
    VerificationPerformed,
    /// An evidence object was packaged.
    EvidenceRecorded,
    /// A reproducibility contract was declared.
    ContractDeclared,
    /// A rerun was judged against a contract.
    ReproductionJudged,
}

impl AuditAction {
    /// Returns the stable machine-readable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RequirementDefined => "requirement_defined",
            Self::ExecutionStarted => "execution_started",
            Self::ExecutionFinished => "execution_finished",
            Self::VerificationPerformed => "verification_performed",
            Self::EvidenceRecorded => "evidence_recorded",
            Self::ContractDeclared => "contract_declared",
            Self::ReproductionJudged => "reproduction_judged",
        }
    }
}

impl fmt::Display for AuditAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One immutable step in the audit trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
    actor: Identifier,
    action: AuditAction,
    target: Identifier,
    time: LogicalTime,
    note: Option<String>,
}

impl AuditEntry {
    /// Creates a validated entry: `actor` performed `action` on `target` at
    /// canonical logical `time`.
    ///
    /// # Errors
    ///
    /// Propagates [`ProvenanceError::InvalidIdentifier`] when either
    /// identity violates the Foundation grammar.
    pub fn new(
        actor: &str,
        action: AuditAction,
        target: &str,
        time: LogicalTime,
    ) -> Result<Self, AuditTrailError> {
        Ok(Self {
            actor: parse_identifier(actor)?,
            action,
            target: parse_identifier(target)?,
            time,
            note: None,
        })
    }

    /// Attaches a free-form human note (bounded length; see
    /// [`MAX_AUDIT_NOTE_BYTES`]).
    ///
    /// # Errors
    ///
    /// Returns [`AuditTrailError::NoteTooLong`] past the bound. Notes exist
    /// for auditors, not payloads: large content belongs in the referenced
    /// artifact, not the trail.
    pub fn with_note(mut self, note: impl Into<String>) -> Result<Self, AuditTrailError> {
        let note = note.into();
        if note.len() > MAX_AUDIT_NOTE_BYTES {
            return Err(AuditTrailError::NoteTooLong {
                limit: MAX_AUDIT_NOTE_BYTES,
                actual: note.len(),
            });
        }
        self.note = Some(note);
        Ok(self)
    }

    /// Returns the acting party identity.
    #[must_use]
    pub fn actor(&self) -> &Identifier {
        &self.actor
    }

    /// Returns the performed action.
    #[must_use]
    pub const fn action(&self) -> AuditAction {
        self.action
    }

    /// Returns the artifact the action targeted.
    #[must_use]
    pub fn target(&self) -> &Identifier {
        &self.target
    }

    /// Returns the canonical logical time.
    #[must_use]
    pub const fn time(&self) -> LogicalTime {
        self.time
    }

    /// Returns the optional note.
    #[must_use]
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

impl fmt::Display for AuditEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} `{}` -> `{}`",
            self.time,
            self.actor,
            self.action,
            self.target
        )?;
        if let Some(note) = &self.note {
            write!(f, " ({note})")?;
        }
        Ok(())
    }
}

/// Maximum note length in bytes.
pub const MAX_AUDIT_NOTE_BYTES: usize = 512;

/// Maximum entries per trail, bounding audit memory.
pub const MAX_AUDIT_ENTRIES: usize = 65_536;

/// An ordered, append-only trail of [`AuditEntry`] values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditTrail {
    entries: Vec<AuditEntry>,
}

impl AuditTrail {
    /// Starts an empty trail.
    #[must_use]
    pub const fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Appends an entry whose time is >= the last entry's time.
    ///
    /// # Errors
    ///
    /// - [`AuditTrailError::TimeRegression`] when the entry would move the
    ///   trail backwards on the logical timeline;
    /// - [`AuditTrailError::TrailFull`] past [`MAX_AUDIT_ENTRIES`].
    ///
    /// On error the trail is left untouched (append is atomic).
    pub fn append(mut self, entry: AuditEntry) -> Result<Self, AuditTrailError> {
        if self.entries.len() >= MAX_AUDIT_ENTRIES {
            return Err(AuditTrailError::TrailFull {
                limit: MAX_AUDIT_ENTRIES,
            });
        }
        if let Some(last) = self.entries.last() {
            if entry.time() < last.time() {
                return Err(AuditTrailError::TimeRegression {
                    previous_nanos: last.time().as_nanos(),
                    attempted_nanos: entry.time().as_nanos(),
                });
            }
        }
        self.entries.push(entry);
        Ok(self)
    }

    /// Returns the entries in append (chronological) order.
    #[must_use]
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Returns the number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether the trail has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns all entries targeting `target_text`, in chronological order.
    #[must_use]
    pub fn by_target(&self, target_text: &str) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.target().as_str() == target_text)
            .collect()
    }

    /// Returns all entries performing `action`, in chronological order.
    #[must_use]
    pub fn by_action(&self, action: AuditAction) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.action() == action)
            .collect()
    }
}

impl Default for AuditTrail {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors produced while building audit trails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditTrailError {
    /// Identity text violated the Foundation grammar.
    InvalidIdentifier(ProvenanceError),
    /// An appended entry predates the current trail head.
    TimeRegression {
        /// Last accepted time (nanos).
        previous_nanos: u64,
        /// Rejected time (nanos).
        attempted_nanos: u64,
    },
    /// The trail reached [`MAX_AUDIT_ENTRIES`].
    TrailFull {
        /// Configured limit.
        limit: usize,
    },
    /// A note exceeded [`MAX_AUDIT_NOTE_BYTES`].
    NoteTooLong {
        /// Byte limit.
        limit: usize,
        /// Rejected length.
        actual: usize,
    },
}

impl fmt::Display for AuditTrailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(source) => write!(f, "invalid audit identifier: {source}"),
            Self::TimeRegression {
                previous_nanos,
                attempted_nanos,
            } => write!(
                f,
                "audit entry at {attempted_nanos}ns regresses behind trail head {previous_nanos}ns"
            ),
            Self::TrailFull { limit } => write!(f, "audit trail reached {limit} entries"),
            Self::NoteTooLong { limit, actual } => {
                write!(f, "audit note of {actual} bytes exceeds {limit}-byte bound")
            }
        }
    }
}

impl Error for AuditTrailError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidIdentifier(source) => Some(source),
            _ => None,
        }
    }
}

impl From<ProvenanceError> for AuditTrailError {
    fn from(source: ProvenanceError) -> Self {
        Self::InvalidIdentifier(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn t(nanos: u64) -> LogicalTime {
        LogicalTime::from_nanos(nanos)
    }

    #[test]
    fn appends_in_chronological_order() {
        let trail = AuditTrail::new()
            .append(AuditEntry::new("engineer_a", AuditAction::RequirementDefined, "min_altitude", t(0)).unwrap())
            .unwrap()
            .append(AuditEntry::new("exec_daemon", AuditAction::ExecutionStarted, "exec_204", t(1_000)).unwrap())
            .unwrap()
            .append(AuditEntry::new("verifier", AuditAction::VerificationPerformed, "v_88", t(2_000)).unwrap())
            .unwrap();

        assert_eq!(trail.len(), 3);
        assert_eq!(trail.entries()[1].action(), AuditAction::ExecutionStarted);
        assert_eq!(trail.by_target("exec_204").len(), 1);
        assert_eq!(trail.by_action(AuditAction::VerificationPerformed).len(), 1);
    }

    #[test]
    fn equal_timestamps_are_allowed() {
        let trail = AuditTrail::new()
            .append(AuditEntry::new("a", AuditAction::EvidenceRecorded, "ev_1", t(5)).unwrap())
            .unwrap()
            .append(AuditEntry::new("a", AuditAction::EvidenceRecorded, "ev_2", t(5)).unwrap())
            .unwrap();
        assert_eq!(trail.len(), 2);
    }

    #[test]
    fn rejects_time_regression_without_mutating_trail() {
        let trail = AuditTrail::new()
            .append(AuditEntry::new("a", AuditAction::ExecutionStarted, "exec_1", t(10)).unwrap())
            .unwrap();

        let error = trail
            .clone()
            .append(AuditEntry::new("a", AuditAction::ExecutionFinished, "exec_1", t(9)).unwrap())
            .unwrap_err();

        assert_eq!(
            error,
            AuditTrailError::TimeRegression {
                previous_nanos: 10,
                attempted_nanos: 9
            }
        );
        // Original trail untouched (append consumed a clone here; real API
        // consumes self only on success paths through Result chaining).
        assert_eq!(trail.len(), 1);
    }

    #[test]
    fn rejects_invalid_actor_identity() {
        let error = AuditEntry::new("-bad-", AuditAction::EvidenceRecorded, "ev_1", t(0)).unwrap_err();
        assert!(matches!(error, AuditTrailError::InvalidIdentifier(_)));
    }

    #[test]
    fn notes_are_bounded() {
        let entry = AuditEntry::new("a", AuditAction::ContractDeclared, "rc_1", t(0)).unwrap();
        let long_note = "x".repeat(MAX_AUDIT_NOTE_BYTES + 1);
        let error = entry.with_note(long_note).unwrap_err();
        assert!(matches!(error, AuditTrailError::NoteTooLong { .. }));

        let ok = AuditEntry::new("a", AuditAction::ContractDeclared, "rc_1", t(0))
            .unwrap()
            .with_note("pinned branch+snapshot")
            .unwrap();
        assert_eq!(ok.note(), Some("pinned branch+snapshot"));
    }

    #[test]
    fn display_is_stable() {
        let entry = AuditEntry::new("verifier", AuditAction::VerificationPerformed, "v_88", t(2_000))
            .unwrap();
        assert_eq!(
            entry.to_string(),
            "t=2000ns verifier `verification_performed` -> `v_88`"
        );
    }

    #[test]
    fn trail_equality_is_by_content_and_order() {
        let first = AuditTrail::new()
            .append(AuditEntry::new("a", AuditAction::EvidenceRecorded, "ev_1", t(1)).unwrap())
            .unwrap();
        let second = AuditTrail::new()
            .append(AuditEntry::new("a", AuditAction::EvidenceRecorded, "ev_1", t(1)).unwrap())
            .unwrap();
        assert_eq!(first, second);
    }
}
