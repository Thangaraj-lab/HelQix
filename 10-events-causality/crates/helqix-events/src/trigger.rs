//! Deterministic condition → event triggers for HELQIX Layer 10.
//!
//! A trigger answers *"when should an event be produced?"*:
//!
//! ```text
//! Condition (evaluated against a caller-supplied value)
//!     ↓  rising edge / crossing
//! Trigger
//!     ↓
//! Event (instantiated from the trigger template + logical time)
//! ```
//!
//! # Semantics
//!
//! - [`ThresholdTrigger`] detects a *rising-edge* crossing of a finite
//!   threshold by a finite `f64` input: it fires only on the evaluation in
//!   which the predicate transitions false → true. While the condition stays
//!   true, later evaluations do not re-fire — repeated telemetry polling thus
//!   produces one event per excursion, not one per sample. When the condition
//!   falls back below the threshold the trigger re-arms automatically
//!   (unless configured one-shot).
//! - Direction is explicit ([`Comparison::GreaterOrEqual`] /
//!   [`Comparison::LessOrEqual`]) so "crossing" is unambiguous; equality at
//!   the threshold counts as satisfied (inclusive boundary).
//! - [`PredicateTrigger`] generalizes this to any boolean condition supplied
//!   by the caller (e.g. "constraint violated"), keeping domain logic out of
//!   this crate.
//! - Firing produces a fully validated [`Event`] whose id is derived
//!   deterministically from `(trigger name, logical time)`; replaying the
//!   same inputs yields identical events.
//!
//! Triggers never mutate engineering state and never read a clock: callers
//! supply both the observed value and the [`LogicalTime`] (Layer 07 owns the
//! clock, Layer 04 owns state).
//!
//! # Edge-detection state
//!
//! Rising-edge detection is inherently stateful (`last_result`). That state
//! lives inside the trigger instance owned by the caller — no global
//! registry, no interior mutability — so two pipelines (or two branches)
//! simply own two independent triggers.

use std::fmt;

use helqix_core::{Identifier, MAX_IDENTIFIER_LENGTH};

use crate::event::{
    EntityRef, Event, EventError, EventId, EventKind, EventPayload, LogicalTime, PayloadValue,
    Severity,
};

/// Comparison direction used by threshold semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Comparison {
    /// Condition holds when `value >= limit`.
    GreaterOrEqual,
    /// Condition holds when `value <= limit`.
    LessOrEqual,
}

impl Comparison {
    /// Evaluates this comparison for `value` against `limit`.
    #[must_use]
    pub fn holds(self, value: f64, limit: f64) -> bool {
        match self {
            Self::GreaterOrEqual => value >= limit,
            Self::LessOrEqual => value <= limit,
        }
    }

    /// Returns the stable machine-readable symbol.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GreaterOrEqual => "greater_or_equal",
            Self::LessOrEqual => "less_or_equal",
        }
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors produced by trigger configuration or evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum TriggerError {
    /// The threshold limit was NaN or infinite.
    NonFiniteLimit {
        /// The rejected value.
        value: f64,
    },
    /// The evaluated input was NaN or infinite. Trigger state is left
    /// unchanged: a non-finite reading is refused, never treated as a
    /// crossing.
    NonFiniteInput {
        /// The rejected value.
        value: f64,
    },
    /// Required trigger configuration (source entity or metric name) is
    /// missing. Carries the trigger name for diagnostics.
    InvalidConfiguration {
        /// Name of the mis-configured trigger.
        trigger: String,
        /// What was missing/invalid.
        reason: &'static str,
    },
    /// The instantiated event failed validation.
    EventValidation {
        /// Underlying event error.
        source: EventError,
    },
}

impl fmt::Display for TriggerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteLimit { value } => {
                write!(f, "trigger threshold limit must be finite, got {value}")
            }
            Self::NonFiniteInput { value } => {
                write!(f, "trigger input must be finite, got {value}")
            }
            Self::InvalidConfiguration { trigger, reason } => {
                write!(f, "trigger `{trigger}` is invalid: {reason}")
            }
            Self::EventValidation { source } => {
                write!(f, "trigger could not build event: {source}")
            }
        }
    }
}

impl std::error::Error for TriggerError {}

/// Description of the event a trigger emits when it fires.
#[derive(Debug, Clone, PartialEq)]
struct EventTemplate {
    kind: EventKind,
    severity: Severity,
    source: EntityRef,
    target: Option<EntityRef>,
    metric: Identifier,
}

/// Shared identifier field names used in trigger-produced payloads.
const FIELD_TRIGGER: &str = "trigger";
const FIELD_LIMIT: &str = "limit";
const FIELD_COMPARISON: &str = "comparison";
const FIELD_CONDITION: &str = "condition";

fn field(name: &str) -> Identifier {
    Identifier::parse(name).unwrap_or_else(|_| unreachable!("`{name}` is a literal valid identifier"))
}

/// Normalizes a derived event-id string into the strict Foundation
/// identifier grammar (letters/digits/underscore, starting with a letter).
fn normalize_event_id(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if !out.starts_with(|c: char| c.is_ascii_alphabetic()) {
        out.insert(0, 't');
    }
    out.truncate(MAX_IDENTIFIER_LENGTH);
    out
}

/// Builds the deterministic event id for one firing of `name` at `time`.
fn fired_event_id(name: &str, time: LogicalTime) -> Result<EventId, TriggerError> {
    let text = normalize_event_id(&format!("{name}_{}", time.as_nanos()));
    EventId::new(&text).map_err(|source| TriggerError::EventValidation { source })
}

/// Rising-edge threshold trigger over a finite `f64` input.
///
/// Constructed via [`Trigger::threshold`].
#[derive(Debug, Clone, PartialEq)]
pub struct ThresholdTrigger {
    name: Identifier,
    limit: f64,
    comparison: Comparison,
    template: EventTemplate,
    one_shot: bool,
    last_result: bool,
    fired_once: bool,
}

impl ThresholdTrigger {
    /// Evaluates a numeric observation at `time`.
    ///
    /// Returns `Some(event)` exactly on a rising edge (false → true),
    /// `None` otherwise. Non-finite input is refused with an error and does
    /// not change trigger state.
    ///
    /// # Errors
    ///
    /// See [`TriggerError`].
    pub fn evaluate(
        &mut self,
        value: f64,
        time: LogicalTime,
    ) -> Result<Option<Event>, TriggerError> {
        if !value.is_finite() {
            return Err(TriggerError::NonFiniteInput { value });
        }
        let holds = self.comparison.holds(value, self.limit);
        let rose = holds && !self.last_result;
        self.last_result = holds;
        if !rose || (self.one_shot && self.fired_once) {
            return Ok(None);
        }
        self.fired_once = true;
        Ok(Some(self.make_event(value, time)?))
    }

    /// Returns the trigger name.
    #[must_use]
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the configured threshold limit.
    #[must_use]
    pub const fn limit(&self) -> f64 {
        self.limit
    }

    /// Returns the comparison direction.
    #[must_use]
    pub const fn comparison(&self) -> Comparison {
        self.comparison
    }

    /// Resets edge-detection state (used when isolating branch replays).
    pub fn reset(&mut self) {
        self.last_result = false;
        self.fired_once = false;
    }

    fn make_event(&self, value: f64, time: LogicalTime) -> Result<Event, TriggerError> {
        let mut payload = EventPayload::new();
        let fail = |e| TriggerError::EventValidation { source: e };
        payload
            .insert(self.template.metric.clone(), PayloadValue::real(value).map_err(fail)?)
            .map_err(fail)?;
        payload
            .insert(
                field(FIELD_LIMIT),
                PayloadValue::real(self.limit).map_err(fail)?,
            )
            .map_err(fail)?;
        payload
            .insert(
                field(FIELD_COMPARISON),
                PayloadValue::text(self.comparison.as_str()),
            )
            .map_err(fail)?;
        payload
            .insert(
                field(FIELD_TRIGGER),
                PayloadValue::Text(self.name.as_str().into()),
            )
            .map_err(fail)?;

        let mut builder = Event::builder(self.template.kind.clone(), time)
            .id(fired_event_id(self.name.as_str(), time)?)
            .severity(self.template.severity)
            .source(self.template.source.clone());
        if let Some(target) = &self.template.target {
            builder = builder.target(target.clone());
        }
        // Attach the payload through the error-free path: rebuild manually.
        let base = builder.build().map_err(fail)?;
        Ok(Event::with_payload(base, payload))
    }
}

/// Rising-edge trigger over a caller-supplied boolean condition.
///
/// The caller decides how the condition is computed (constraint violation,
/// presence of an observation, ...); this crate only tracks the edge and
/// instantiates the event, so triggers remain `Clone + PartialEq`
/// (snapshot/branch friendly) without closures.
#[derive(Debug, Clone, PartialEq)]
pub struct PredicateTrigger {
    name: Identifier,
    template: EventTemplate,
    one_shot: bool,
    last_result: bool,
    fired_once: bool,
}

impl PredicateTrigger {
    /// Creates a predicate trigger named `name`, emitting `kind` events
    /// attributed to `source`, tagged with metric `metric`.
    ///
    /// # Errors
    ///
    /// Returns [`TriggerError::InvalidConfiguration`] when `metric` equals
    /// the reserved key `condition`.
    pub fn new(
        name: Identifier,
        metric: Identifier,
        kind: EventKind,
        severity: Severity,
        source: EntityRef,
    ) -> Result<Self, TriggerError> {
        if metric.as_str() == FIELD_CONDITION {
            return Err(TriggerError::InvalidConfiguration {
                trigger: name.as_str().to_owned(),
                reason: "metric name `condition` is reserved",
            });
        }
        Ok(Self {
            name,
            template: EventTemplate {
                kind,
                severity,
                source,
                target: None,
                metric,
            },
            one_shot: false,
            last_result: false,
            fired_once: false,
        })
    }

    /// Enables one-shot mode: the trigger fires at most once ever.
    #[must_use]
    pub const fn one_shot(mut self, yes: bool) -> Self {
        self.one_shot = yes;
        self
    }

    /// Sets an optional target entity for emitted events.
    #[must_use]
    pub fn target(mut self, target: EntityRef) -> Self {
        self.template.target = Some(target);
        self
    }

    /// Returns the trigger name.
    #[must_use]
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Evaluates `condition` at `time`; returns an event exactly on a
    /// false → true rising edge.
    ///
    /// # Errors
    ///
    /// See [`TriggerError`].
    pub fn evaluate(
        &mut self,
        condition: bool,
        time: LogicalTime,
    ) -> Result<Option<Event>, TriggerError> {
        let rose = condition && !self.last_result;
        self.last_result = condition;
        if !rose || (self.one_shot && self.fired_once) {
            return Ok(None);
        }
        self.fired_once = true;
        let fail = |e| TriggerError::EventValidation { source: e };
        let mut payload = EventPayload::new();
        payload
            .insert(field(FIELD_CONDITION), PayloadValue::Bool(true))
            .map_err(fail)?;
        payload
            .insert(
                field(FIELD_TRIGGER),
                PayloadValue::Text(self.name.as_str().into()),
            )
            .map_err(fail)?;
        let mut builder = Event::builder(self.template.kind.clone(), time)
            .id(fired_event_id(self.name.as_str(), time)?)
            .severity(self.template.severity)
            .source(self.template.source.clone());
        if let Some(target) = &self.template.target {
            builder = builder.target(target.clone());
        }
        let base = builder.build().map_err(fail)?;
        Ok(Some(Event::with_payload(base, payload)))
    }

    /// Resets edge-detection state.
    pub fn reset(&mut self) {
        self.last_result = false;
        self.fired_once = false;
    }
}

impl Trigger {
    /// Creates a boolean predicate edge trigger.
    ///
    /// # Errors
    ///
    /// Configuration errors as in [`PredicateTrigger::new`].
    pub fn predicate(
        name: Identifier,
        metric: Identifier,
        kind: EventKind,
        severity: Severity,
        source: EntityRef,
    ) -> Result<Self, TriggerError> {
        Ok(Self::Predicate(PredicateTrigger::new(
            name, metric, kind, severity, source,
        )?))
    }
}

/// Builder returned by [`Trigger::threshold`].
#[derive(Debug, Clone)]
pub struct ThresholdTriggerBuilder {
    name: Identifier,
    limit: f64,
    comparison: Comparison,
    kind: EventKind,
    severity: Severity,
    source: Option<EntityRef>,
    target: Option<EntityRef>,
    metric: Option<Identifier>,
    one_shot: bool,
}

impl ThresholdTriggerBuilder {
    /// Overrides the emitted event kind (default [`EventKind::ThresholdCrossed`]).
    #[must_use]
    pub fn event_kind(mut self, kind: EventKind) -> Self {
        self.kind = kind;
        self
    }

    /// Overrides the emitted event severity (default [`Severity::Warning`]).
    #[must_use]
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Sets the source entity attached to emitted events (required).
    #[must_use]
    pub fn source(mut self, source: EntityRef) -> Self {
        self.source = Some(source);
        self
    }

    /// Sets an optional target entity attached to emitted events.
    #[must_use]
    pub fn target(mut self, target: EntityRef) -> Self {
        self.target = Some(target);
        self
    }

    /// Names the monitored quantity for payload metadata (required).
    #[must_use]
    pub fn metric(mut self, metric: Identifier) -> Self {
        self.metric = Some(metric);
        self
    }

    /// Restricts the trigger to a single firing for its lifetime
    /// (default: re-armable rising-edge behavior).
    #[must_use]
    pub const fn one_shot(mut self) -> Self {
        self.one_shot = true;
        self
    }

    /// Validates and constructs the threshold trigger.
    ///
    /// # Errors
    ///
    /// Returns [`TriggerError::InvalidConfiguration`] when the source entity
    /// or metric name is missing.
    pub fn build(self) -> Result<Trigger, TriggerError> {
        let missing = |reason| TriggerError::InvalidConfiguration {
            trigger: self.name.as_str().to_owned(),
            reason,
        };
        let source = self.source.ok_or_else(|| missing("source entity is required"))?;
        let metric = self
            .metric
            .ok_or_else(|| missing("metric name is required"))?;
        Ok(Trigger::Threshold(ThresholdTrigger {
            name: self.name,
            limit: self.limit,
            comparison: self.comparison,
            template: EventTemplate {
                kind: self.kind,
                severity: self.severity,
                source,
                target: self.target,
                metric,
            },
            one_shot: self.one_shot,
            last_result: false,
            fired_once: false,
        }))
    }
}

/// A deterministic condition evaluator that requests an event on a rising
/// edge.
#[derive(Debug, Clone, PartialEq)]
pub enum Trigger {
    /// Threshold-crossing trigger over finite `f64` observations.
    Threshold(ThresholdTrigger),
    /// Boolean-predicate edge trigger.
    Predicate(PredicateTrigger),
}

impl Trigger {
    /// Starts building a threshold trigger named `name` comparing observed
    /// values against the finite `limit`.
    ///
    /// # Errors
    ///
    /// Returns [`TriggerError::NonFiniteLimit`] when `limit` is NaN/infinite.
    pub fn threshold(
        name: Identifier,
        limit: f64,
        comparison: Comparison,
    ) -> Result<ThresholdTriggerBuilder, TriggerError> {
        if !limit.is_finite() {
            return Err(TriggerError::NonFiniteLimit { value: limit });
        }
        Ok(ThresholdTriggerBuilder {
            name,
            limit,
            comparison,
            kind: EventKind::ThresholdCrossed,
            severity: Severity::Warning,
            source: None,
            target: None,
            metric: None,
            one_shot: false,
        })
    }

    /// Evaluates a numeric observation (threshold triggers) — predicate
    /// triggers reject numeric input because their condition is boolean.
    ///
    /// # Errors
    ///
    /// See [`TriggerError`].
    pub fn evaluate(
        &mut self,
        value: f64,
        time: LogicalTime,
    ) -> Result<Option<Event>, TriggerError> {
        match self {
            Self::Threshold(t) => t.evaluate(value, time),
            Self::Predicate(p) => Err(TriggerError::InvalidConfiguration {
                trigger: p.name().as_str().to_owned(),
                reason: "predicate triggers require evaluate_condition()",
            }),
        }
    }

    /// Evaluates a boolean condition (predicate triggers); threshold
    /// triggers reject boolean input because their condition is numeric.
    ///
    /// # Errors
    ///
    /// See [`TriggerError`].
    pub fn evaluate_condition(
        &mut self,
        condition: bool,
        time: LogicalTime,
    ) -> Result<Option<Event>, TriggerError> {
        match self {
            Self::Threshold(t) => Err(TriggerError::InvalidConfiguration {
                trigger: t.name().as_str().to_owned(),
                reason: "threshold triggers require evaluate()",
            }),
            Self::Predicate(p) => p.evaluate(condition, time),
        }
    }

    /// Returns the trigger name.
    #[must_use]
    pub fn name(&self) -> &Identifier {
        match self {
            Self::Threshold(t) => t.name(),
            Self::Predicate(p) => p.name(),
        }
    }

    /// Resets edge-detection state.
    pub fn reset(&mut self) {
        match self {
            Self::Threshold(t) => t.reset(),
            Self::Predicate(p) => p.reset(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ident(text: &str) -> Identifier {
        Identifier::parse(text).expect("test identifier")
    }

    fn thermal_trigger(one_shot: bool) -> Trigger {
        let b = Trigger::threshold(ident("thermal_limit"), 100.0, Comparison::GreaterOrEqual)
            .unwrap()
            .source(EntityRef::component("MotorA").unwrap())
            .metric(ident("temperature"));
        if one_shot {
            b.one_shot().build().unwrap()
        } else {
            b.build().unwrap()
        }
    }

    fn time(ns: u64) -> LogicalTime {
        LogicalTime::from_nanos(ns)
    }

    #[test]
    fn condition_false_produces_no_event() {
        let mut t = thermal_trigger(false);
        assert_eq!(t.evaluate(80.0, time(1)).unwrap(), None);
        assert_eq!(t.evaluate(99.999, time(2)).unwrap(), None);
    }

    #[test]
    fn condition_becoming_true_fires_exactly_one_event() {
        let mut t = thermal_trigger(false);
        let first = t.evaluate(105.0, time(10)).unwrap();
        assert!(first.is_some(), "rising edge must fire");
        // Still true afterwards: no repeat firing while condition holds.
        assert_eq!(t.evaluate(110.0, time(20)).unwrap(), None);
        assert_eq!(t.evaluate(120.0, time(30)).unwrap(), None);
    }

    #[test]
    fn threshold_boundary_is_inclusive() {
        let mut t = thermal_trigger(false);
        assert!(t.evaluate(100.0, time(5)).unwrap().is_some(), "value == limit fires");
    }

    #[test]
    fn falling_edge_rearms_the_trigger() {
        let mut t = thermal_trigger(false);
        assert!(t.evaluate(105.0, time(1)).unwrap().is_some());
        assert_eq!(t.evaluate(90.0, time(2)).unwrap(), None); // falls back
        assert!(t.evaluate(101.0, time(3)).unwrap().is_some()); // re-crosses
    }

    #[test]
    fn one_shot_never_refires_even_after_rearming() {
        let mut t = thermal_trigger(true);
        assert!(t.evaluate(105.0, time(1)).unwrap().is_some());
        assert_eq!(t.evaluate(50.0, time(2)).unwrap(), None);
        assert_eq!(t.evaluate(150.0, time(3)).unwrap(), None, "one-shot stays fired");
    }

    #[test]
    fn fired_event_carries_expected_semantics() {
        let mut t = thermal_trigger(false);
        let e = t.evaluate(110.0, time(42)).unwrap().expect("event");
        assert_eq!(e.kind(), &EventKind::ThresholdCrossed);
        assert_eq!(e.time(), time(42));
        assert_eq!(e.id().as_str(), "thermal_limit_42");
        assert_eq!(e.source().id().as_str(), "MotorA");
        let value = e.payload().get(&ident("temperature")).and_then(PayloadValue::as_real);
        assert_eq!(value, Some(110.0));
        assert_eq!(
            e.payload().get(&ident("limit")).and_then(PayloadValue::as_real),
            Some(100.0)
        );
        assert_eq!(
            e.payload().get(&ident("comparison")).and_then(PayloadValue::as_text),
            Some("greater_or_equal")
        );
    }

    #[test]
    fn evaluation_is_deterministic_across_identical_runs() {
        let run = || {
            let mut t = thermal_trigger(false);
            let _ = t.evaluate(90.0, time(1));
            t.evaluate(105.0, time(2))
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn non_finite_inputs_are_refused_without_state_change() {
        let mut t = thermal_trigger(false);
        assert!(matches!(
            t.evaluate(f64::NAN, time(1)),
            Err(TriggerError::NonFiniteInput { .. })
        ));
        assert!(matches!(
            t.evaluate(f64::INFINITY, time(1)),
            Err(TriggerError::NonFiniteInput { .. })
        ));
        // State untouched: the next valid crossing still fires.
        assert!(t.evaluate(105.0, time(2)).unwrap().is_some());
    }

    #[test]
    fn invalid_trigger_configuration_is_rejected() {
        // Missing metric/source.
        let err = Trigger::threshold(ident("x"), 1.0, Comparison::LessOrEqual)
            .unwrap()
            .build()
            .unwrap_err();
        assert!(matches!(err, TriggerError::InvalidConfiguration { .. }));
        // Non-finite limit.
        assert!(matches!(
            Trigger::threshold(ident("x"), f64::NAN, Comparison::LessOrEqual),
            Err(TriggerError::NonFiniteLimit { .. })
        ));
        // Reserved metric name on a predicate trigger.
        let err = Trigger::predicate(
            ident("p"),
            ident("condition"),
            EventKind::ConstraintViolated,
            Severity::Warning,
            EntityRef::component("C").unwrap(),
        )
        .unwrap_err();
        assert!(matches!(err, TriggerError::InvalidConfiguration { .. }));
    }

    #[test]
    fn predicate_trigger_fires_on_rising_edge_only() {
        let mut t = Trigger::predicate(
            ident("constraint_violation"),
            ident("violated"),
            EventKind::ConstraintViolated,
            Severity::Critical,
            EntityRef::action("DeployGear").unwrap(),
        )
        .unwrap();
        assert_eq!(
            t.evaluate_condition(false, time(1)).unwrap(),
            None,
            "false condition -> no event"
        );
        let e = t.evaluate_condition(true, time(7)).unwrap().expect("fires");
        assert_eq!(e.kind(), &EventKind::ConstraintViolated);
        assert_eq!(e.id().as_str(), "constraint_violation_7");
        assert_eq!(
            t.evaluate_condition(true, time(8)).unwrap(),
            None,
            "no repeat while held"
        );
        assert!(t.evaluate_condition(false, time(9)).unwrap().is_none());
        assert!(t.evaluate_condition(true, time(10)).unwrap().is_some(), "re-armed");
    }

    #[test]
    fn wrong_input_channel_is_a_clear_configuration_error() {
        let mut t = thermal_trigger(false);
        assert!(matches!(
            t.evaluate_condition(true, time(1)),
            Err(TriggerError::InvalidConfiguration { .. })
        ));
        let mut p = Trigger::predicate(
            ident("p"),
            ident("flag"),
            EventKind::StateChanged,
            Severity::Info,
            EntityRef::state("S").unwrap(),
        )
        .unwrap();
        assert!(matches!(
            p.evaluate(1.0, time(1)),
            Err(TriggerError::InvalidConfiguration { .. })
        ));
    }

    #[test]
    fn less_or_equal_direction_works_symmetrically() {
        let mut t = Trigger::threshold(ident("min_altitude"), 50.0, Comparison::LessOrEqual)
            .unwrap()
            .source(EntityRef::component("Vehicle").unwrap())
            .metric(ident("altitude"))
            .build()
            .unwrap();
        assert!(t.evaluate(60.0, time(1)).unwrap().is_none());
        let e = t.evaluate(49.0, time(2)).unwrap().expect("low crossing fires");
        assert_eq!(
            e.payload().get(&ident("comparison")).and_then(PayloadValue::as_text),
            Some("less_or_equal")
        );
    }
}
