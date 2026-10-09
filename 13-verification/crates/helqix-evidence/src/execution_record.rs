//! Authoritative execution context supporting verification evidence.
//!
//! An [`ExecutionRecord`] is *not* an execution engine and *not* a second
//! source of truth. Layer 05 owns execution, Layer 04 owns state, Layer 07
//! owns logical time, and Layer 10 owns the event vocabulary. This module
//! only captures the identity information that later verification and audit
//! need in order to answer: "which execution produced this result?"
//!
//! The record stores validated canonical identifiers ([`Identifier`]) and the
//! canonical Layer 10 temporal abstraction ([`LogicalTime`], integer
//! nanoseconds — Layer 07 remains the authoritative clock owner; its crate is
//! currently an unimplemented placeholder, so HELQIX consumes the shared
//! `helqix-events::event::LogicalTime` value type exactly as Layers 09–12 do).
//!
//! Determinism contract:
//! - no wall-clock reads, no randomness, no registries;
//! - all fields are caller-supplied and validated at construction;
//! - ordering between records is derived from `(start_time, id)`, never from
//!   insertion or hashing.

use std::error::Error;
use std::fmt;

use helqix_core::id::Identifier;
use helqix_events::event::{EntityRef, LogicalTime};

/// Validated identity of one engineering execution.
///
/// Wraps a canonical Foundation [`Identifier`]. Identities are always
/// assigned explicitly by the producer (Layer 05 when it materializes);
/// this crate never generates ids implicitly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExecutionId(Identifier);

impl ExecutionId {
    /// Validates `text` against the Foundation identifier grammar.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionRecordError::InvalidIdentifier`] when `text` is
    /// rejected by [`Identifier::parse`].
    pub fn new(text: &str) -> Result<Self, ExecutionRecordError> {
        Identifier::parse(text)
            .map(Self)
            .map_err(|source| ExecutionRecordError::InvalidIdentifier {
                field: "execution_id",
                source,
            })
    }

    /// Wraps an already-validated canonical identifier.
    #[must_use]
    pub const fn from_identifier(identifier: Identifier) -> Self {
        Self(identifier)
    }

    /// Returns the id text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns the underlying canonical identifier.
    #[must_use]
    pub fn identifier(&self) -> &Identifier {
        &self.0
    }
}

impl fmt::Display for ExecutionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Terminal outcome of a recorded execution.
///
/// This describes how the execution itself ended. It is deliberately not a
/// verification verdict: whether requirements were satisfied is decided by
/// `helqix-verify`, never by the record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExecutionOutcome {
    /// The execution reached its planned end without error.
    Completed,
    /// The execution was stopped before completion by an explicit request.
    Aborted,
    /// The execution ended because a failure was detected (Layer 09).
    Failed,
    /// The execution's terminal condition is not established by upstream
    /// layers yet. Verification must treat this as missing context, never
    /// as success.
    Unknown,
}

impl ExecutionOutcome {
    /// Returns the stable machine-readable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Aborted => "aborted",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for ExecutionOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Builder input for [`ExecutionRecord::create`].
///
/// Every field is explicit: constructing a record forces the caller to state
/// which execution, branch, start/end time and terminal outcome it refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRecordSpec {
    id: String,
    branch: Option<String>,
    start_time: LogicalTime,
    end_time: LogicalTime,
    outcome: ExecutionOutcome,
}

impl ExecutionRecordSpec {
    /// Starts a spec for the execution identified by `id` between the given
    /// canonical logical times with the given terminal outcome.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        start_time: LogicalTime,
        end_time: LogicalTime,
        outcome: ExecutionOutcome,
    ) -> Self {
        Self {
            id: id.into(),
            branch: None,
            start_time,
            end_time,
            outcome,
        }
    }

    /// Attaches the Layer 11 branch identity this execution belongs to.
    #[must_use]
    pub fn on_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    /// Creates a validated [`ExecutionRecord`] from this spec.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionRecordError`] when any identifier text violates the
    /// Foundation grammar or the temporal bounds are inverted.
    pub fn build(self) -> Result<ExecutionRecord, ExecutionRecordError> {
        ExecutionRecord::new(
            self.id.as_str(),
            self.branch.as_deref(),
            self.start_time,
            self.end_time,
            self.outcome,
        )
    }
}

/// Immutable capture of the authoritative execution context behind evidence.
///
/// See the module documentation for ownership boundaries. The record holds
/// references-by-identity only: it never embeds component states, telemetry
/// or events, so it stays cheap to clone into many evidence objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRecord {
    id: ExecutionId,
    branch: Option<EntityRef>,
    start_time: LogicalTime,
    end_time: LogicalTime,
    outcome: ExecutionOutcome,
}

impl ExecutionRecord {
    /// Creates a validated execution record.
    ///
    /// Validation order is fixed (id, branch, then temporal bounds) so the
    /// same invalid input always yields the same error.
    ///
    /// # Errors
    ///
    /// - [`ExecutionRecordError::InvalidIdentifier`] for bad id/branch text;
    /// - [`ExecutionRecordError::InvertedTimeBounds`] when
    ///   `end_time < start_time`.
    pub fn new(
        id: &str,
        branch: Option<&str>,
        start_time: LogicalTime,
        end_time: LogicalTime,
        outcome: ExecutionOutcome,
    ) -> Result<Self, ExecutionRecordError> {
        let id = ExecutionId::new(id)?;

        let branch = match branch {
            Some(text) => {
                let identifier = Identifier::parse(text).map_err(|source| {
                    ExecutionRecordError::InvalidIdentifier {
                        field: "branch",
                        source,
                    }
                })?;
                Some(EntityRef::Branch(identifier))
            }
            None => None,
        };

        if end_time < start_time {
            return Err(ExecutionRecordError::InvertedTimeBounds {
                execution: id.as_str().to_owned(),
                start_nanos: start_time.as_nanos(),
                end_nanos: end_time.as_nanos(),
            });
        }

        Ok(Self {
            id,
            branch,
            start_time,
            end_time,
            outcome,
        })
    }

    /// Convenience constructor through the explicit [`ExecutionRecordSpec`].
    pub fn create(spec: ExecutionRecordSpec) -> Result<Self, ExecutionRecordError> {
        spec.build()
    }

    /// Returns the execution identity.
    #[must_use]
    pub const fn id(&self) -> &ExecutionId {
        &self.id
    }

    /// Returns the Layer 11 branch reference, when the execution was
    /// attributed to a branch.
    #[must_use]
    pub const fn branch(&self) -> Option<&EntityRef> {
        self.branch.as_ref()
    }

    /// Returns the canonical logical start time.
    #[must_use]
    pub const fn start_time(&self) -> LogicalTime {
        self.start_time
    }

    /// Returns the canonical logical end time.
    #[must_use]
    pub const fn end_time(&self) -> LogicalTime {
        self.end_time
    }

    /// Returns the terminal execution outcome.
    #[must_use]
    pub const fn outcome(&self) -> ExecutionOutcome {
        self.outcome
    }

    /// Returns whether `time` lies inside the inclusive execution window.
    #[must_use]
    pub fn contains_time(&self, time: LogicalTime) -> bool {
        self.start_time <= time && time <= self.end_time
    }
}

/// Errors produced while constructing an [`ExecutionRecord`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionRecordError {
    /// Identifier text violated the Foundation grammar.
    InvalidIdentifier {
        /// Which field failed ("execution_id" or "branch").
        field: &'static str,
        /// Underlying Foundation error.
        source: helqix_core::error::IdentifierError,
    },
    /// The execution window ends before it starts.
    InvertedTimeBounds {
        /// Offending execution id.
        execution: String,
        /// Start bound in nanoseconds.
        start_nanos: u64,
        /// End bound in nanoseconds.
        end_nanos: u64,
    },
}

impl fmt::Display for ExecutionRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { field, source } => {
                write!(f, "invalid {field} identifier: {source}")
            }
            Self::InvertedTimeBounds {
                execution,
                start_nanos,
                end_nanos,
            } => write!(
                f,
                "execution `{execution}` ends at {end_nanos}ns before starting at {start_nanos}ns"
            ),
        }
    }
}

impl Error for ExecutionRecordError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidIdentifier { source, .. } => Some(source),
            Self::InvertedTimeBounds { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn t(nanos: u64) -> LogicalTime {
        LogicalTime::from_nanos(nanos)
    }

    #[test]
    fn creates_valid_record() {
        let record = ExecutionRecord::new(
            "exec_204",
            Some("branch_a"),
            t(0),
            t(12_480_000_000),
            ExecutionOutcome::Completed,
        )
        .expect("valid record");

        assert_eq!(record.id().as_str(), "exec_204");
        assert_eq!(record.branch().map(EntityRef::id).map(Identifier::as_str), Some("branch_a"));
        assert_eq!(record.end_time(), t(12_480_000_000));
        assert_eq!(record.outcome(), ExecutionOutcome::Completed);
    }

    #[test]
    fn rejects_invalid_execution_id() {
        let error = ExecutionRecord::new("1bad", None, t(0), t(1), ExecutionOutcome::Completed)
            .unwrap_err();
        assert!(matches!(
            error,
            ExecutionRecordError::InvalidIdentifier { field: "execution_id", .. }
        ));
    }

    #[test]
    fn rejects_inverted_time_bounds() {
        let error = ExecutionRecord::new(
            "exec_1",
            None,
            t(10),
            t(5),
            ExecutionOutcome::Completed,
        )
        .unwrap_err();
        assert_eq!(
            error,
            ExecutionRecordError::InvertedTimeBounds {
                execution: "exec_1".to_owned(),
                start_nanos: 10,
                end_nanos: 5,
            }
        );
    }

    #[test]
    fn zero_length_window_is_allowed() {
        let record = ExecutionRecord::new("exec_0", None, t(7), t(7), ExecutionOutcome::Aborted)
            .expect("instantaneous execution window is valid");
        assert!(record.contains_time(t(7)));
        assert!(!record.contains_time(t(8)));
    }

    #[test]
    fn spec_builder_matches_direct_constructor() {
        let via_spec = ExecutionRecord::create(
            ExecutionRecordSpec::new("exec_9", t(0), t(100), ExecutionOutcome::Failed)
                .on_branch("branch_b"),
        )
        .unwrap();
        let direct = ExecutionRecord::new(
            "exec_9",
            Some("branch_b"),
            t(0),
            t(100),
            ExecutionOutcome::Failed,
        )
        .unwrap();
        assert_eq!(via_spec, direct);
    }

    #[test]
    fn construction_is_deterministic() {
        let first = ExecutionRecord::new("e", None, t(0), t(1), ExecutionOutcome::Completed).unwrap();
        let second = ExecutionRecord::new("e", None, t(0), t(1), ExecutionOutcome::Completed).unwrap();
        assert_eq!(first, second);
    }
}
