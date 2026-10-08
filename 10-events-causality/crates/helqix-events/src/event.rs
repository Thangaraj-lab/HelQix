//! Canonical event representation for HELQIX Layer 10.
//!
//! An [`Event`] is an immutable record that *something meaningful happened*
//! during engineering execution: an action was accepted, a state changed,
//! a threshold crossed, a constraint was violated, a failure was detected.
//!
//! # Ownership boundaries
//!
//! - Layer 04 owns authoritative state; an event only ever *references* a
//!   state identity, it never embeds or mutates it.
//! - Layer 07 owns logical time; this crate consumes a caller-supplied
//!   [`LogicalTime`] value. It never reads a wall clock — no `SystemTime`,
//!   `Instant`, randomness or global registries appear in this module.
//! - Layer 09 owns failure detection; a failure is represented here as an
//!   ordinary event with a well-known kind, not as a special type.
//!
//! # Determinism
//!
//! Event identity ([`EventId`]) is constructed explicitly by the producer and
//! validated against the Foundation [`Identifier`] grammar — it is never
//! generated randomly here. Payload maps are `BTreeMap`-backed, so iteration
//! order is lexicographic and reproducible.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use helqix_core::{Identifier, IdentifierError};

/// Maximum number of entries allowed in one event payload map.
///
/// Events are lightweight occurrence records, not data warehouses: bulk
/// observation data belongs to Layer 08 telemetry, which an event may
/// reference through its identifiers. The bound keeps dispatch and replay
/// costs predictable.
pub const MAX_PAYLOAD_FIELDS: usize = 64;

/// Stable identity of an event.
///
/// Wraps a validated Foundation [`Identifier`]. The producer assigns the id;
/// this crate never generates one implicitly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(Identifier);

impl EventId {
    /// Validates `text` and builds an event id.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::InvalidId`] when `text` violates the
    /// Foundation identifier grammar.
    pub fn new(text: &str) -> Result<Self, EventError> {
        Identifier::parse(text)
            .map(Self)
            .map_err(|source| EventError::InvalidId { source })
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

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Monotonic logical time, expressed as an integer count of nanoseconds on
/// the simulation timeline.
///
/// Layer 07 (`helqix-time`) owns the authoritative clock; its sources are
/// currently unimplemented placeholders, so Layer 10 accepts time as an
/// explicit, validated input value rather than inventing a competing clock.
/// Integer nanoseconds keep ordering exact (no float rounding) and unit-safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogicalTime(u64);

impl LogicalTime {
    /// Builds a logical time from a nanosecond count.
    #[must_use]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    /// Returns the raw nanosecond count.
    #[must_use]
    pub const fn as_nanos(self) -> u64 {
        self.0
    }

    /// Returns the whole-second component.
    #[must_use]
    pub const fn seconds(self) -> u64 {
        self.0 / 1_000_000_000
    }

    /// Returns the sub-second nanosecond remainder.
    #[must_use]
    pub const fn subsec_nanos(self) -> u32 {
        (self.0 % 1_000_000_000) as u32
    }
}

impl fmt::Display for LogicalTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t={}ns", self.0)
    }
}

/// A typed, structured, immutable payload field.
///
/// Numeric fields cannot represent NaN or infinity: constructors reject
/// non-finite values instead of silently storing them.
#[derive(Debug, Clone, PartialEq)]
pub enum PayloadValue {
    /// Textual datum (names, messages, serialized references).
    Text(Arc<str>),
    /// Signed integer datum (counts, revisions, codes).
    Int(i64),
    /// Finite floating-point datum (measurements, thresholds).
    Real(f64),
    /// Boolean flag.
    Bool(bool),
    /// Reference to a canonical identifier owned elsewhere (component,
    /// action, state, failure, metric ...). Stored after validation.
    Reference(Identifier),
    /// Explicit absence of a value (distinct from "field missing").
    Null,
}

impl PayloadValue {
    /// Builds a finite real-valued datum.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::NonFinitePayload`] when `value` is NaN or
    /// infinite.
    pub fn real(value: f64) -> Result<Self, EventError> {
        if !value.is_finite() {
            return Err(EventError::NonFinitePayload { value });
        }
        Ok(Self::Real(value))
    }

    /// Builds a textual datum.
    #[must_use]
    pub fn text(value: impl AsRef<str>) -> Self {
        Self::Text(Arc::from(value.as_ref()))
    }

    /// Builds a validated identifier reference datum.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::InvalidReference`] when `text` violates the
    /// Foundation identifier grammar.
    pub fn reference(text: &str) -> Result<Self, EventError> {
        Identifier::parse(text)
            .map(Self::Reference)
            .map_err(|source| EventError::InvalidReference { source })
    }

    /// Returns the contained float, if any.
    #[must_use]
    pub fn as_real(&self) -> Option<f64> {
        match self {
            Self::Real(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the contained integer, if any.
    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the contained text, if any.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(v) => Some(v),
            Self::Reference(v) => Some(v.as_str()),
            _ => None,
        }
    }
}

/// Structured event data.
///
/// Backed by a `BTreeMap`, therefore iteration and equality are independent
/// of insertion order and fully deterministic.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventPayload {
    fields: BTreeMap<Identifier, PayloadValue>,
}

impl EventPayload {
    /// Returns an empty payload.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts (or replaces) a field.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::TooManyPayloadFields`] when the payload already
    /// holds [`MAX_PAYLOAD_FIELDS`] distinct fields and `name` is new.
    pub fn insert(
        &mut self,
        name: Identifier,
        value: PayloadValue,
    ) -> Result<(), EventError> {
        if !self.fields.contains_key(&name) && self.fields.len() >= MAX_PAYLOAD_FIELDS {
            return Err(EventError::TooManyPayloadFields {
                limit: MAX_PAYLOAD_FIELDS,
            });
        }
        self.fields.insert(name, value);
        Ok(())
    }

    /// Returns a field value.
    #[must_use]
    pub fn get(&self, name: &Identifier) -> Option<&PayloadValue> {
        self.fields.get(name)
    }

    /// Returns `true` when no fields are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Returns the number of fields.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Iterates fields in deterministic (lexicographic) key order.
    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &PayloadValue)> {
        self.fields.iter()
    }
}

/// Well-known event kinds used across HELQIX layers.
///
/// These names form the stable vocabulary shared with Layers 04/05/08/09.
/// Producers may also use [`EventKind::Custom`] for domain-neutral events
/// whose semantics live in the engineering model, not in this crate.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventKind {
    /// An action was accepted by execution (Layer 05).
    ActionAccepted,
    /// An action began executing (Layer 05).
    ActionStarted,
    /// An action completed successfully (Layer 05).
    ActionCompleted,
    /// An action failed to execute (Layer 05).
    ActionFailed,
    /// Authoritative state changed (Layer 04 transition).
    StateChanged,
    /// A component's state entry changed (Layer 04).
    ComponentChanged,
    /// A simulation step produced a candidate state (Layer 06).
    SimulationStepCompleted,
    /// A metric crossed a configured threshold (Layer 08 observation).
    ThresholdCrossed,
    /// An engineering constraint was violated (Layers 03/06 evaluation).
    ConstraintViolated,
    /// An observation/telemetry sample was produced (Layer 08).
    ObservationGenerated,
    /// A failure was detected (Layer 09 — detection authority stays there).
    FailureDetected,
    /// A causal relationship was recorded into the causal graph (Layer 10).
    CausalRelationRecorded,
    /// A branch was created (Layer 11).
    BranchCreated,
    /// Domain/model-specific kind carrying a validated identifier name.
    Custom(Identifier),
}

impl EventKind {
    /// Builds a custom kind from identifier text.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::InvalidKind`] when `text` violates the
    /// Foundation identifier grammar.
    pub fn custom(text: &str) -> Result<Self, EventError> {
        Identifier::parse(text)
            .map(Self::Custom)
            .map_err(|source| EventError::InvalidKind { source })
    }

    /// Returns a stable machine-readable name for this kind.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::ActionAccepted => "action.accepted",
            Self::ActionStarted => "action.started",
            Self::ActionCompleted => "action.completed",
            Self::ActionFailed => "action.failed",
            Self::StateChanged => "state.changed",
            Self::ComponentChanged => "component.changed",
            Self::SimulationStepCompleted => "simulation.step.completed",
            Self::ThresholdCrossed => "threshold.crossed",
            Self::ConstraintViolated => "constraint.violated",
            Self::ObservationGenerated => "observation.generated",
            Self::FailureDetected => "failure.detected",
            Self::CausalRelationRecorded => "causal.relation.recorded",
            Self::BranchCreated => "branch.created",
            Self::Custom(id) => id.as_str(),
        }
    }
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EventKind {
    type Err = EventError;

    /// Parses a kind from its stable name (inverse of [`EventKind::as_str`]).
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let known = [
            ("action.accepted", Self::ActionAccepted),
            ("action.started", Self::ActionStarted),
            ("action.completed", Self::ActionCompleted),
            ("action.failed", Self::ActionFailed),
            ("state.changed", Self::StateChanged),
            ("component.changed", Self::ComponentChanged),
            ("simulation.step.completed", Self::SimulationStepCompleted),
            ("threshold.crossed", Self::ThresholdCrossed),
            ("constraint.violated", Self::ConstraintViolated),
            ("observation.generated", Self::ObservationGenerated),
            ("failure.detected", Self::FailureDetected),
            ("causal.relation.recorded", Self::CausalRelationRecorded),
            ("branch.created", Self::BranchCreated),
        ];
        if let Some((_, kind)) = known.iter().find(|(n, _)| *n == text) {
            return Ok(kind.clone());
        }
        Self::custom(text)
    }
}

/// Severity classification of an event.
///
/// Ordinal: `Debug < Info < Warning < Critical`. Severity describes how
/// important an occurrence is; it does **not** imply causality or failure
/// (failure detection remains Layer 09 authority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Fine-grained operational detail.
    Debug,
    /// Normal lifecycle occurrence.
    Info,
    /// Degraded condition worth attention.
    Warning,
    /// Condition threatening correct operation.
    Critical,
}

impl Severity {
    /// Returns the stable machine-readable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A reference to an entity involved in an event, resolved through the
/// canonical [`Identifier`] namespace.
///
/// Producers map their layer-specific ids (`ActionId`, `StateId`,
/// `ComponentId`, `FailureId`, ...) onto these tagged references so Layer 10
/// stays domain-independent and never duplicates other layers' id types.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntityRef {
    /// A component (Layer 01 definition / Layer 04 state).
    Component(Identifier),
    /// An action (Layer 05).
    Action(Identifier),
    /// A state instance (Layer 04).
    State(Identifier),
    /// A failure (Layer 09).
    Failure(Identifier),
    /// An observation/metric (Layer 08).
    Observation(Identifier),
    /// A simulation interaction (Layer 06).
    Interaction(Identifier),
    /// A branch (Layer 11).
    Branch(Identifier),
    /// Any other validated identifier.
    Other(Identifier),
}

impl EntityRef {
    /// Returns the referenced identifier.
    #[must_use]
    pub fn id(&self) -> &Identifier {
        match self {
            Self::Component(id)
            | Self::Action(id)
            | Self::State(id)
            | Self::Failure(id)
            | Self::Observation(id)
            | Self::Interaction(id)
            | Self::Branch(id)
            | Self::Other(id) => id,
        }
    }

    /// Builds a component reference from validated text.
    ///
    /// # Errors
    ///
    /// Rejects invalid identifier grammar.
    pub fn component(text: &str) -> Result<Self, EventError> {
        parse_id(text).map(Self::Component)
    }

    /// Builds an action reference from validated text.
    ///
    /// # Errors
    ///
    /// Rejects invalid identifier grammar.
    pub fn action(text: &str) -> Result<Self, EventError> {
        parse_id(text).map(Self::Action)
    }

    /// Builds a state reference from validated text.
    ///
    /// # Errors
    ///
    /// Rejects invalid identifier grammar.
    pub fn state(text: &str) -> Result<Self, EventError> {
        parse_id(text).map(Self::State)
    }

    /// Builds a failure reference from validated text.
    ///
    /// # Errors
    ///
    /// Rejects invalid identifier grammar.
    pub fn failure(text: &str) -> Result<Self, EventError> {
        parse_id(text).map(Self::Failure)
    }

    /// Builds an observation reference from validated text.
    ///
    /// # Errors
    ///
    /// Rejects invalid identifier grammar.
    pub fn observation(text: &str) -> Result<Self, EventError> {
        parse_id(text).map(Self::Observation)
    }
}

fn parse_id(text: &str) -> Result<Identifier, EventError> {
    Identifier::parse(text).map_err(|source| EventError::InvalidReference { source })
}

/// Builder returned by [`Event::builder`].
#[derive(Debug)]
pub struct EventBuilder {
    id: Option<EventId>,
    kind: EventKind,
    time: LogicalTime,
    severity: Severity,
    source: Option<EntityRef>,
    target: Option<EntityRef>,
    payload: EventPayload,
}

impl EventBuilder {
    /// Sets the event identity (required).
    #[must_use]
    pub fn id(mut self, id: EventId) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the event kind.
    #[must_use]
    pub fn kind(mut self, kind: EventKind) -> Self {
        self.kind = kind;
        self
    }

    /// Sets the logical timestamp (supplied by the producer / Layer 07).
    #[must_use]
    pub fn time(mut self, time: LogicalTime) -> Self {
        self.time = time;
        self
    }

    /// Sets the severity.
    #[must_use]
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Sets the originating entity (required).
    #[must_use]
    pub fn source(mut self, source: EntityRef) -> Self {
        self.source = Some(source);
        self
    }

    /// Sets the affected entity, when applicable.
    #[must_use]
    pub fn target(mut self, target: EntityRef) -> Self {
        self.target = Some(target);
        self
    }

    /// Inserts a payload field.
    ///
    /// # Panics
    ///
    /// Panics when `name` is not a valid identifier or the payload-size
    /// contract is violated. Handle errors explicitly with
    /// [`EventPayload::insert`] when inputs are not developer-controlled.
    #[must_use]
    pub fn with_field(self, name: &str, value: PayloadValue) -> Self {
        match Identifier::parse(name) {
            Ok(id) => self.with_field_id(id, value),
            Err(source) => panic!("payload field name `{name}` invalid: {source}"),
        }
    }

    /// Inserts a payload field keyed by a pre-validated identifier.
    ///
    /// # Panics
    ///
    /// Panics when the payload-size contract is violated.
    #[must_use]
    pub fn with_field_id(mut self, name: Identifier, value: PayloadValue) -> Self {
        self.payload.insert(name, value).expect("payload field limit");
        self
    }

    /// Finalizes and validates the event.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::MissingId`] or [`EventError::MissingSource`]
    /// when required fields are absent. Invalid numeric payloads were
    /// already rejected at construction.
    pub fn build(self) -> Result<Event, EventError> {
        let id = self.id.ok_or(EventError::MissingId)?;
        let source = self.source.ok_or(EventError::MissingSource)?;
        Ok(Event {
            id,
            kind: self.kind,
            time: self.time,
            severity: self.severity,
            source,
            target: self.target,
            payload: self.payload,
        })
    }
}

/// An immutable record of something that happened during engineering
/// execution.
///
/// Events are historical facts: once built they cannot change — the struct
/// exposes no setters, only borrowed/`Copy` accessors. Recorded delivery adds
/// bus metadata *around* the event ([`crate::event_bus::RecordedEvent`]); it
/// never rewrites the event itself.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    id: EventId,
    kind: EventKind,
    time: LogicalTime,
    severity: Severity,
    source: EntityRef,
    target: Option<EntityRef>,
    payload: EventPayload,
}

impl Event {
    /// Starts a builder with the given kind and logical timestamp.
    #[must_use]
    pub fn builder(kind: EventKind, time: LogicalTime) -> EventBuilder {
        EventBuilder {
            id: None,
            kind,
            time,
            severity: Severity::Info,
            source: None,
            target: None,
            payload: EventPayload::new(),
        }
    }

    /// Returns the event identity.
    #[must_use]
    pub fn id(&self) -> &EventId {
        &self.id
    }

    /// Returns the event kind.
    #[must_use]
    pub fn kind(&self) -> &EventKind {
        &self.kind
    }

    /// Returns the logical timestamp.
    #[must_use]
    pub fn time(&self) -> LogicalTime {
        self.time
    }

    /// Returns the severity.
    #[must_use]
    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// Returns the originating entity.
    #[must_use]
    pub fn source(&self) -> &EntityRef {
        &self.source
    }

    /// Returns the affected entity, if any.
    #[must_use]
    pub fn target(&self) -> Option<&EntityRef> {
        self.target.as_ref()
    }

    /// Returns the structured payload.
    #[must_use]
    pub fn payload(&self) -> &EventPayload {
        &self.payload
    }

    /// Returns a copy of `base` with its payload replaced by `payload`.
    ///
    /// This is *not* a mutation primitive: historical events stay immutable.
    /// It exists so producers (e.g. triggers) can assemble rich payloads
    /// through fallible [`EventPayload`] operations before finalizing a new
    /// event value.
    #[must_use]
    pub fn with_payload(base: Event, payload: EventPayload) -> Event {
        Event {
            id: base.id,
            kind: base.kind,
            time: base.time,
            severity: base.severity,
            source: base.source,
            target: base.target,
            payload,
        }
    }
}

/// Errors produced while constructing or validating events.
#[derive(Debug, Clone, PartialEq)]
pub enum EventError {
    /// Required event id was not supplied.
    MissingId,
    /// Required source entity was not supplied.
    MissingSource,
    /// The event id text violated the identifier grammar.
    InvalidId {
        /// Underlying foundation error.
        source: IdentifierError,
    },
    /// The custom kind text violated the identifier grammar.
    InvalidKind {
        /// Underlying foundation error.
        source: IdentifierError,
    },
    /// An entity/payload reference violated the identifier grammar.
    InvalidReference {
        /// Underlying foundation error.
        source: IdentifierError,
    },
    /// A numeric payload field was NaN or infinite.
    NonFinitePayload {
        /// The rejected value.
        value: f64,
    },
    /// The payload exceeded [`MAX_PAYLOAD_FIELDS`] distinct fields.
    TooManyPayloadFields {
        /// Configured limit.
        limit: usize,
    },
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingId => f.write_str("event requires an explicit id"),
            Self::MissingSource => f.write_str("event requires a source entity"),
            Self::InvalidId { source } => write!(f, "invalid event id: {source}"),
            Self::InvalidKind { source } => write!(f, "invalid event kind: {source}"),
            Self::InvalidReference { source } => write!(f, "invalid entity reference: {source}"),
            Self::NonFinitePayload { value } => {
                write!(f, "numeric payload values must be finite, got {value}")
            }
            Self::TooManyPayloadFields { limit } => {
                write!(f, "payload exceeds the maximum of {limit} fields")
            }
        }
    }
}

impl std::error::Error for EventError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> EventId {
        EventId::new(text).expect("test id")
    }

    fn time(ns: u64) -> LogicalTime {
        LogicalTime::from_nanos(ns)
    }

    fn sample() -> Event {
        Event::builder(EventKind::ThresholdCrossed, time(1_500))
            .id(id("evt_thermal_1"))
            .source(EntityRef::component("MotorA").unwrap())
            .target(EntityRef::failure("ThermalLimit").unwrap())
            .severity(Severity::Warning)
            .with_field("metric", PayloadValue::text("temperature"))
            .with_field("value", PayloadValue::real(110.0).unwrap())
            .build()
            .expect("valid sample event")
    }

    #[test]
    fn valid_event_preserves_identity_and_timestamp() {
        let e = sample();
        assert_eq!(e.id().as_str(), "evt_thermal_1");
        assert_eq!(e.time(), time(1_500));
        assert_eq!(e.kind(), &EventKind::ThresholdCrossed);
        assert_eq!(e.severity(), Severity::Warning);
        assert_eq!(e.source().id().as_str(), "MotorA");
        assert_eq!(
            e.target().map(EntityRef::id).map(Identifier::as_str),
            Some("ThermalLimit")
        );
    }

    #[test]
    fn missing_required_fields_are_rejected() {
        let no_id = Event::builder(EventKind::StateChanged, time(0))
            .source(EntityRef::state("S1").unwrap())
            .build();
        assert_eq!(no_id.unwrap_err(), EventError::MissingId);

        let no_src = Event::builder(EventKind::StateChanged, time(0))
            .id(id("x"))
            .build();
        assert_eq!(no_src.unwrap_err(), EventError::MissingSource);
    }

    #[test]
    fn invalid_identifier_grammar_is_rejected() {
        assert!(matches!(
            EventId::new("2bad-start"),
            Err(EventError::InvalidId { .. })
        ));
        assert!(matches!(
            EntityRef::component(""),
            Err(EventError::InvalidReference { .. })
        ));
        assert!(matches!(
            EventKind::custom("has space"),
            Err(EventError::InvalidKind { .. })
        ));
    }

    #[test]
    fn non_finite_payload_values_are_rejected() {
        assert!(matches!(
            PayloadValue::real(f64::NAN),
            Err(EventError::NonFinitePayload { .. })
        ));
        assert!(matches!(
            PayloadValue::real(f64::INFINITY),
            Err(EventError::NonFinitePayload { .. })
        ));
        assert!(PayloadValue::real(-1.0e-9).is_ok());
    }

    #[test]
    fn payload_iteration_is_deterministic_regardless_of_insertion_order() {
        let mut a = EventPayload::new();
        let mut b = EventPayload::new();
        a.insert(Identifier::parse("alpha").unwrap(), PayloadValue::Bool(true))
            .unwrap();
        a.insert(Identifier::parse("beta").unwrap(), PayloadValue::Int(2))
            .unwrap();
        b.insert(Identifier::parse("beta").unwrap(), PayloadValue::Int(2))
            .unwrap();
        b.insert(Identifier::parse("alpha").unwrap(), PayloadValue::Bool(true))
            .unwrap();

        let keys_a: Vec<_> = a.iter().map(|(k, _)| k.as_str()).collect();
        let keys_b: Vec<_> = b.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys_a, vec!["alpha", "beta"]);
        assert_eq!(keys_a, keys_b);
        assert_eq!(a, b); // equality ignores insertion order
    }

    #[test]
    fn payload_field_limit_is_enforced() {
        let mut p = EventPayload::new();
        for i in 0..MAX_PAYLOAD_FIELDS {
            p.insert(
                Identifier::parse(&format!("f{i:03}")).unwrap(),
                PayloadValue::Null,
            )
            .unwrap();
        }
        let err = p
            .insert(Identifier::parse("overflow").unwrap(), PayloadValue::Null)
            .unwrap_err();
        assert_eq!(
            err,
            EventError::TooManyPayloadFields {
                limit: MAX_PAYLOAD_FIELDS
            }
        );
    }

    #[test]
    fn event_equality_is_structural_and_deterministic() {
        assert_eq!(sample(), sample());
    }

    #[test]
    fn clone_is_content_identical_history_semantics() {
        let original = sample();
        let copy = original.clone();
        assert_eq!(original, copy);
        // Event exposes no setters at all: recorded history cannot mutate.
    }

    #[test]
    fn repeated_construction_is_bitwise_identical() {
        for _ in 0..10 {
            assert_eq!(sample(), sample());
        }
    }

    #[test]
    fn kind_round_trips_through_stable_names() {
        for kind in [
            EventKind::ActionAccepted,
            EventKind::StateChanged,
            EventKind::FailureDetected,
            EventKind::custom("model_specific").unwrap(),
        ] {
            let parsed: EventKind = kind.as_str().parse().unwrap();
            assert_eq!(parsed, kind);
        }
    }

    #[test]
    fn severity_is_ordinal() {
        assert!(Severity::Debug < Severity::Info);
        assert!(Severity::Warning < Severity::Critical);
    }

    #[test]
    fn logical_time_decomposes_exactly() {
        let t = time(2_000_000_500);
        assert_eq!(t.seconds(), 2);
        assert_eq!(t.subsec_nanos(), 500);
        assert_eq!(t.as_nanos(), 2_000_000_500);
    }
}
