//! Layer 10 / helqix-events — deterministic event *delivery* propagation.
//!
//! This module answers one question only:
//!
//! > "How should an event be forwarded through the event system?"
//!
//! It is **not** engineering causality (that lives in `helqix-causal`), and it
//! is **not** failure propagation (Layer 09). The distinction matters:
//!
//! * A forwarding rule here says *"when Event A is recorded, also record
//!   Event B"*. Whether A actually *caused* B is a separate, explicitly
//!   evidenced relationship owned by the causal crate.
//! * Propagation rules are therefore a *distribution mechanism*: they let
//!   one canonical event give rise to derived events (e.g. fan-out
//!   notification or mirroring events) without every producer knowing about
//!   every consumer.
//!
//! ## Determinism guarantees
//!
//! * Rules are evaluated in **registration order** (a `Vec`, never a map).
//! * Derived ids are checked against the journal; a collision is an error,
//!   never a silent overwrite or skip.
//! * No wall-clock reads, no randomness, no global state. Identical inputs
//!   yield identical outputs (tested).
//!
//! ## Termination guarantees
//!
//! Unbounded forwarding chains are impossible by construction:
//!
//! 1. **Cycle detection.** The rule graph (`rule i -> rule j` when rule `i`
//!    emits kind `k` and rule `j` matches `k`) is analysed at registration
//!    time with deterministic DFS. Any cycle — including self-loops — is
//!    rejected up front with [`PropagationError::RuleCycle`]. A rule set that
//!    passes validation can therefore never loop.
//! 2. **Depth bound.** Even acyclic rule sets can form long chains. Callers
//!    enforce an explicit maximum derivation depth ([`DEFAULT_MAX_DEPTH`]) so
//!    pathological chains fail loudly instead of recursing forever.
//! 3. **Single-level engine.** [`PropagationEngine::derive`] performs exactly
//!    one derivation level; the cascade driver owns breadth-first processing
//!    and applies the depth bound.

use crate::event::{Event, EventKind};
use std::fmt;

/// Default bound on derivation depth (levels below the original publish).
///
/// Chosen deliberately small: HELQIX forwarding is meant for shallow fan-out
/// (notification, mirroring, escalation), not for multi-hop transformation
/// pipelines. Deep chains are almost always mis-specified causality dressed
/// up as routing, and they belong in `helqix-causal` relationships instead.
pub const DEFAULT_MAX_DEPTH: usize = 4;

/// Hard ceiling on registered rules per bus (configuration sanity bound).
pub const MAX_RULES: usize = 1024;

/// Errors produced while configuring or running event propagation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropagationError {
    /// Too many propagation rules registered.
    TooManyRules {
        /// Current rule count before the rejected insertion.
        current: usize,
        /// Configured limit.
        limit: usize,
    },
    /// Registering this rule would create a forwarding cycle
    /// (including a self-loop: the rule emits a kind it also matches).
    RuleCycle {
        /// Description of the offending path, e.g. `"rule#0 -> rule#1 -> rule#0"`.
        path: String,
    },
    /// A propagation rule produced an event whose id already exists in the
    /// journal. Forwarding refuses to silently drop or overwrite it.
    DerivedIdCollision {
        /// Display form of the colliding event id.
        id: String,
    },
    /// Derivation exceeded the configured maximum depth.
    MaxDepthExceeded {
        /// The configured bound.
        max_depth: usize,
        /// Display form of the event at which the bound was hit.
        at_event: String,
    },
    /// A matched rule's emitter returned `None`.
    RuleProducedNothing {
        /// Index of the rule in registration order.
        rule_index: usize,
    },
}

impl fmt::Display for PropagationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyRules { current, limit } => {
                write!(f, "propagation: {current} rules registered, limit {limit}")
            }
            Self::RuleCycle { path } => {
                write!(f, "propagation: forwarding cycle detected ({path})")
            }
            Self::DerivedIdCollision { id } => {
                write!(f, "propagation: derived event id '{id}' already recorded")
            }
            Self::MaxDepthExceeded { max_depth, at_event } => {
                write!(
                    f,
                    "propagation: derivation exceeded max depth {max_depth} at event '{at_event}'"
                )
            }
            Self::RuleProducedNothing { rule_index } => {
                write!(f, "propagation: rule #{rule_index} matched but emitted no event")
            }
        }
    }
}

impl std::error::Error for PropagationError {}

/// Tunables for the propagation engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropagationConfig {
    /// Maximum number of derivation levels below an external publish.
    pub max_depth: usize,
}

impl Default for PropagationConfig {
    fn default() -> Self {
        Self {
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

/// A single forwarding rule: matched kinds -> derived event.
///
/// The emitter receives a reference to the source event and returns the
/// derived event. Rules declared to emit a kind they also match are rejected
/// at registration (self-loop), so an emitter must never feed its own input
/// kind back into the network.
pub struct PropagationRule<F> {
    name: String,
    matches: Vec<EventKind>,
    emit: F,
}

impl<F> PropagationRule<F> {
    /// Builds a named rule from a set of matched kinds and an emitter.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        matches: impl IntoIterator<Item = EventKind>,
        emit: F,
    ) -> Self {
        Self {
            name: name.into(),
            matches: matches.into_iter().collect(),
            emit,
        }
    }
}

struct Rule {
    name: String,
    matches: Vec<EventKind>,
    emits: Vec<EventKind>,
    emit: Box<dyn Fn(&Event) -> Option<Event>>,
}

/// Registration-time + run-time propagation machinery, owned by the bus.
pub struct PropagationEngine {
    rules: Vec<Rule>,
    config: PropagationConfig,
}

impl Default for PropagationEngine {
    fn default() -> Self {
        Self::new(PropagationConfig::default())
    }
}

impl PropagationEngine {
    /// Creates an empty engine with the given configuration.
    #[must_use]
    pub fn new(config: PropagationConfig) -> Self {
        Self {
            rules: Vec::new(),
            config,
        }
    }

    /// Configuration in force.
    #[must_use]
    pub fn config(&self) -> PropagationConfig {
        self.config
    }

    /// Number of registered rules.
    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Registered rule names in registration order.
    #[must_use]
    pub fn rule_names(&self) -> Vec<&str> {
        self.rules.iter().map(|r| r.name.as_str()).collect()
    }

    /// Registers a rule, rejecting structural violations.
    ///
    /// The emitter's *output kinds* are supplied explicitly (`emits`) because
    /// they cannot generally be inferred from a closure. A rule whose declared
    /// emissions close a cycle in the rule graph (including a self-loop) is
    /// rejected with [`PropagationError::RuleCycle`]. Undeclared emission of a
    /// matched kind is caught at run time by the depth bound, never allowed to
    /// loop silently.
    pub fn register<F>(
        &mut self,
        mut rule: PropagationRule<F>,
        emits: impl IntoIterator<Item = EventKind>,
    ) -> Result<usize, PropagationError>
    where
        F: Fn(&Event) -> Option<Event> + 'static,
    {
        if self.rules.len() >= MAX_RULES {
            return Err(PropagationError::TooManyRules {
                current: self.rules.len(),
                limit: MAX_RULES,
            });
        }
        let mut candidate_emits: Vec<EventKind> = emits.into_iter().collect();
        candidate_emits.sort_by_key(|k: &EventKind| -> String { k.to_string() });
        let mut candidate_matches = rule.matches.clone();
        candidate_matches.sort_by_key(|k: &EventKind| -> String { k.to_string() });
        // Deterministic cycle check over the rule graph after appending.
        if let Some(path) = find_cycle_path(&self.rules, &candidate_matches, &candidate_emits) {
            return Err(PropagationError::RuleCycle { path });
        }
        let index = self.rules.len();
        self.rules.push(Rule {
            name: std::mem::take(&mut rule.name),
            matches: rule.matches,
            emits: candidate_emits,
            emit: Box::new(rule.emit),
        });
        Ok(index)
    }

    /// Given one freshly recorded source event, returns the derived events
    /// produced by matching rules **in registration order**.
    ///
    /// Performs exactly one derivation level; the caller drives the
    /// breadth-first cascade and enforces the depth bound. A matched rule
    /// whose emitter returns `None` is reported as a structural error rather
    /// than silently skipped.
    pub fn derive(&self, source: &Event) -> Result<Vec<Event>, PropagationError> {
        let mut out = Vec::new();
        for (i, rule) in self.rules.iter().enumerate() {
            if !rule.matches.iter().any(|k| k == source.kind()) {
                continue;
            }
            match (rule.emit)(source) {
                Some(ev) => out.push(ev),
                None => return Err(PropagationError::RuleProducedNothing { rule_index: i }),
            }
        }
        Ok(out)
    }

    /// Drives a full breadth-first cascade from one externally published
    /// event, enforcing the depth bound. Returns all derived events in
    /// deterministic order (depth-major, then parent order, then rule
    /// registration order).
    ///
    /// `record_and_check` is invoked for each derived event *before* its own
    /// children are derived; the bus uses it to append to the journal and to
    /// detect id collisions.
    pub fn cascade(
        &self,
        root: &Event,
        mut record_and_check: impl FnMut(&Event) -> Result<(), PropagationError>,
    ) -> Result<Vec<Event>, PropagationError> {
        let mut all = Vec::new();
        let mut frontier = vec![root.clone()];
        // Derivation levels performed so far. The externally published root
        // is level 0; each non-empty expansion of the frontier adds one.
        let mut depth = 0usize;
        while !frontier.is_empty() {
            let mut next = Vec::new();
            for src in &frontier {
                for derived in self.derive(src)? {
                    record_and_check(&derived)?;
                    all.push(derived.clone());
                    next.push(derived);
                }
            }
            if !next.is_empty() {
                depth += 1;
                if depth > self.config.max_depth {
                    let at = next[0].id().to_string();
                    return Err(PropagationError::MaxDepthExceeded {
                        max_depth: self.config.max_depth,
                        at_event: at,
                    });
                }
            }
            frontier = next;
        }
        Ok(all)
    }
}

impl fmt::Debug for PropagationEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropagationEngine")
            .field("rules", &self.rule_names())
            .field("config", &self.config)
            .finish()
    }
}

/// Deterministic DFS over the rule graph after hypothetically appending the
/// candidate rule. Returns the first cycle path found, if any.
///
/// Node space: indices `0..=n` where `n` is the candidate. Edge `i -> j`
/// exists iff some kind emitted by `i` is matched by `j`. Edges are explored
/// in ascending index order, so results never depend on hash ordering.
fn find_cycle_path(
    existing: &[Rule],
    candidate_matches: &[EventKind],
    candidate_emits: &[EventKind],
) -> Option<String> {
    let n = existing.len(); // candidate index == n
    let matches_of = |i: usize| -> Vec<&str> {
        if i == n {
            candidate_matches.iter().map(EventKind::as_str).collect()
        } else {
            existing[i].matches.iter().map(EventKind::as_str).collect()
        }
    };
    let emits_of = |i: usize| -> Vec<&str> {
        if i == n {
            candidate_emits.iter().map(EventKind::as_str).collect()
        } else {
            existing[i].emits.iter().map(EventKind::as_str).collect()
        }
    };
    let successors = |i: usize| -> Vec<usize> {
        let e = emits_of(i);
        let mut succ = Vec::new();
        for j in 0..=n {
            let m = matches_of(j);
            if e.iter().any(|k| m.contains(k)) {
                succ.push(j);
            }
        }
        succ
    };

    const WHITE: u8 = 0;
    const GREY: u8 = 1;
    const BLACK: u8 = 2;
    let mut color = vec![WHITE; n + 1];
    let mut stack: Vec<usize> = Vec::new();

    fn dfs(
        u: usize,
        color: &mut [u8],
        stack: &mut Vec<usize>,
        successors: &dyn Fn(usize) -> Vec<usize>,
    ) -> Option<String> {
        color[u] = GREY;
        stack.push(u);
        for v in successors(u) {
            if color[v] == GREY {
                let start = stack.iter().position(|&x| x == v).unwrap_or(0);
                let mut names: Vec<String> =
                    stack[start..].iter().map(|&i| format!("rule#{i}")).collect();
                names.push(format!("rule#{v}"));
                return Some(names.join(" -> "));
            }
            if color[v] == WHITE {
                if let Some(path) = dfs(v, color, stack, successors) {
                    return Some(path);
                }
            }
        }
        stack.pop();
        color[u] = BLACK;
        None
    }

    for i in 0..=n {
        if color[i] == WHITE {
            if let Some(p) = dfs(i, &mut color, &mut stack, &successors) {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EntityRef, EventId, LogicalTime, Severity};

    fn evt(id: &str, kind: EventKind, t: u64) -> Event {
        Event::builder(kind, LogicalTime::from_nanos(t))
            .id(EventId::new(id).unwrap())
            .severity(Severity::Info)
            .source(EntityRef::component("comp_source").unwrap())
            .build()
            .unwrap()
    }

    // Emitters are plain `fn` items parameterised by (prefix, kind) via a
    // small enum-dispatch helper, so they coerce to `fn(&Event) -> Option<Event>`
    // and stay comparable/simple in tests.
    #[derive(Clone, Copy)]
    enum EmitterSpec {
        L1,
        L2,
        L3,
        D1,
        D2,
        Derived,
        Same,
    }

    impl EmitterSpec {
        fn prefix(self) -> &'static str {
            match self {
                Self::L1 => "l1",
                Self::L2 => "l2",
                Self::L3 => "l3",
                Self::D1 => "d1",
                Self::D2 => "d2",
                Self::Derived => "derived",
                Self::Same => "same",
            }
        }
        fn kind(self) -> EventKind {
            match self {
                Self::L1 | Self::D1 | Self::Same => EventKind::ActionCompleted,
                Self::L2 | Self::D2 | Self::Derived => EventKind::ObservationGenerated,
                Self::L3 => EventKind::ThresholdCrossed,
            }
        }
        fn emit(self, src: &Event) -> Option<Event> {
            // Canonical Foundation identifier grammar is [A-Za-z][A-Za-z0-9_]*;
            // sanitize non-conforming characters instead of failing silently.
            let mut derived_id = String::from(self.prefix());
            for c in src.id().as_str().chars() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    derived_id.push(c);
                } else {
                    derived_id.push('_');
                }
            }
            Some(
                Event::builder(self.kind(), src.time())
                    .id(EventId::new(&derived_id).ok()?)
                    .severity(Severity::Info)
                    .source(src.source().clone())
                    .build()
                    .ok()?,
            )
        }
        fn f(self) -> fn(&Event) -> Option<Event> {
            // Cannot capture `self` into a fn pointer; use a match instead.
            match self {
                Self::L1 => emit_l1,
                Self::L2 => emit_l2,
                Self::L3 => emit_l3,
                Self::D1 => emit_d1,
                Self::D2 => emit_d2,
                Self::Derived => emit_derived,
                Self::Same => emit_same,
            }
        }
    }

    fn emit_l1(src: &Event) -> Option<Event> { EmitterSpec::L1.emit(src) }
    fn emit_l2(src: &Event) -> Option<Event> { EmitterSpec::L2.emit(src) }
    fn emit_l3(src: &Event) -> Option<Event> { EmitterSpec::L3.emit(src) }
    fn emit_d1(src: &Event) -> Option<Event> { EmitterSpec::D1.emit(src) }
    fn emit_d2(src: &Event) -> Option<Event> { EmitterSpec::D2.emit(src) }
    fn emit_derived(src: &Event) -> Option<Event> { EmitterSpec::Derived.emit(src) }
    fn emit_same(src: &Event) -> Option<Event> { EmitterSpec::Same.emit(src) }

    #[test]
    fn registration_returns_sequential_indices() {
        let mut eng = PropagationEngine::default();
        for i in 0..8 {
            let idx = eng
                .register(
                    PropagationRule::new(
                        format!("r{i}"),
                        [EventKind::StateChanged],
                        |_e: &Event| None,
                    ),
                    [EventKind::ActionCompleted],
                )
                .expect("register");
            assert_eq!(idx, i);
        }
        assert_eq!(eng.rule_count(), 8);
    }

    #[test]
    fn self_loop_rule_is_rejected_at_registration() {
        let mut eng = PropagationEngine::default();
        let res = eng.register(
            PropagationRule::new("loop", [EventKind::StateChanged], |_e: &Event| None),
            [EventKind::StateChanged], // emits what it matches
        );
        assert!(
            matches!(res, Err(PropagationError::RuleCycle { .. })),
            "self-forwarding must be rejected, got {res:?}"
        );
    }

    #[test]
    fn two_rule_cycle_is_rejected() {
        let mut eng = PropagationEngine::default();
        eng.register(
            PropagationRule::new("r0", [EventKind::StateChanged], |_e: &Event| None),
            [EventKind::ActionCompleted],
        )
        .unwrap();
        let res = eng.register(
            PropagationRule::new("r1", [EventKind::ActionCompleted], |_e: &Event| None),
            [EventKind::StateChanged], // closes r0 -> r1 -> r0
        );
        assert!(
            matches!(res, Err(PropagationError::RuleCycle { .. })),
            "got {res:?}"
        );
    }

    #[test]
    fn acyclic_chain_registers_and_derives_in_registration_order() {
        let mut eng = PropagationEngine::default();
        eng.register(
            PropagationRule::new(
                "first",
                [EventKind::StateChanged],
                emit_d1,
            ),
            [EventKind::ActionCompleted],
        )
        .unwrap();
        eng.register(
            PropagationRule::new(
                "second",
                [EventKind::StateChanged],
                emit_d2,
            ),
            [EventKind::ObservationGenerated],
        )
        .unwrap();
        let derived = eng.derive(&evt("root", EventKind::StateChanged, 5)).unwrap();
        assert_eq!(derived.len(), 2);
        assert_eq!(derived[0].id().as_str(), "d1root");
        assert_eq!(derived[1].id().as_str(), "d2root");
    }

    #[test]
    fn non_matching_kind_produces_nothing() {
        let mut eng = PropagationEngine::default();
        eng.register(
            PropagationRule::new(
                "r",
                [EventKind::StateChanged],
                |_e: &Event| panic!("must not be called"),
            ),
            [EventKind::ActionCompleted],
        )
        .unwrap();
        let derived = eng
            .derive(&evt("x", EventKind::FailureDetected, 1))
            .unwrap();
        assert!(derived.is_empty());
    }

    #[test]
    fn matched_rule_returning_none_is_a_structural_error() {
        let mut eng = PropagationEngine::default();
        eng.register(
            PropagationRule::new("bad", [EventKind::StateChanged], |_e: &Event| None),
            [EventKind::ActionCompleted],
        )
        .unwrap();
        let res = eng.derive(&evt("y", EventKind::StateChanged, 1));
        assert!(matches!(
            res,
            Err(PropagationError::RuleProducedNothing { rule_index: 0 })
        ));
    }

    #[test]
    fn cascade_is_breadth_first_and_depth_bounded() {
        // Chain: StateChanged -> ActionExecuted -> ObservationGenerated -> ...
        let mut eng = PropagationEngine::new(PropagationConfig { max_depth: 2 });
        eng.register(
            PropagationRule::new(
                "L1",
                [EventKind::StateChanged],
                emit_l1,
            ),
            [EventKind::ActionCompleted],
        )
        .unwrap();
        eng.register(
            PropagationRule::new(
                "L2",
                [EventKind::ActionCompleted],
                emit_l2,
            ),
            [EventKind::ObservationGenerated],
        )
        .unwrap();
        // Depth 0 -> l1.* ; depth 1 -> l2.l1.* — fits within max_depth 2.
        let derived = eng
            .cascade(&evt("root", EventKind::StateChanged, 1), |_| Ok(()))
            .expect("two levels fit within max_depth 2");
        assert_eq!(derived.len(), 2);
        assert_eq!(derived[0].id().as_str(), "l1root");
        assert_eq!(derived[1].id().as_str(), "l2l1root");

        // Add a third link; now the chain exceeds the bound and fails loudly.
        let mut eng2 = PropagationEngine::new(PropagationConfig { max_depth: 2 });
        eng2.register(
            PropagationRule::new(
                "L1",
                [EventKind::StateChanged],
                emit_l1,
            ),
            [EventKind::ActionCompleted],
        )
        .unwrap();
        eng2.register(
            PropagationRule::new(
                "L2",
                [EventKind::ActionCompleted],
                emit_l2,
            ),
            [EventKind::ObservationGenerated],
        )
        .unwrap();
        eng2.register(
            PropagationRule::new(
                "L3",
                [EventKind::ObservationGenerated],
                emit_l3,
            ),
            [EventKind::ThresholdCrossed],
        )
        .unwrap();
        let err = eng2
            .cascade(&evt("root", EventKind::StateChanged, 1), |_| Ok(()))
            .expect_err("depth bound must fire");
        assert!(
            matches!(err, PropagationError::MaxDepthExceeded { max_depth: 2, .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn cascade_reports_id_collision_immediately() {
        let mut eng = PropagationEngine::default();
        eng.register(
            PropagationRule::new(
                "dup",
                [EventKind::StateChanged],
                emit_same,
            ),
            [EventKind::ActionCompleted],
        )
        .unwrap();
        let res = eng.cascade(&evt("root", EventKind::StateChanged, 1), |e| {
            Err(PropagationError::DerivedIdCollision {
                id: e.id().to_string(),
            })
        });
        assert!(matches!(
            res,
            Err(PropagationError::DerivedIdCollision { ref id }) if id == "sameroot"
        ));
    }

    #[test]
    fn derivation_is_deterministic_across_runs() {
        fn build() -> Vec<String> {
            let mut eng = PropagationEngine::default();
            eng.register(
                PropagationRule::new(
                    "a-to-b",
                    [EventKind::StateChanged],
                    emit_derived,
                ),
                [EventKind::ObservationGenerated],
            )
            .unwrap();
            eng.derive(&evt("state_change_7", EventKind::StateChanged, 7))
                .unwrap()
                .iter()
                .map(|e| e.id().as_str().to_owned())
                .collect()
        }
        assert_eq!(build(), build());
    }
}
