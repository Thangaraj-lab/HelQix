use std::{collections::VecDeque, num::NonZeroUsize, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};

use serde::{Deserialize, Serialize};

use crate::adapter::AdapterError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterState {
    Registered,
    Initializing,
    Ready,
    Degraded,
    Stopping,
    Stopped,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleEvent {
    pub sequence: u64,
    pub from: AdapterState,
    pub to: AdapterState,
    pub changed_at_unix_ms: u128,
    pub reason: Option<String>,
}

#[derive(Debug)]
struct LifecycleInner {
    state: AdapterState,
    sequence: u64,
    history: VecDeque<LifecycleEvent>,
}

/// Thread-safe state machine that rejects invalid adapter lifecycle changes.
#[derive(Debug)]
pub struct AdapterLifecycle {
    inner: Mutex<LifecycleInner>,
    history_limit: usize,
}

const DEFAULT_HISTORY_LIMIT: usize = 256;

impl Default for AdapterLifecycle {
    fn default() -> Self { Self::new() }
}

impl AdapterLifecycle {
    pub fn new() -> Self {
        Self::with_history_limit(NonZeroUsize::new(DEFAULT_HISTORY_LIMIT).expect("non-zero constant"))
    }

    pub fn with_history_limit(limit: NonZeroUsize) -> Self {
        Self {
            inner: Mutex::new(LifecycleInner {
                state: AdapterState::Registered,
                sequence: 0,
                history: VecDeque::with_capacity(limit.get()),
            }),
            history_limit: limit.get(),
        }
    }

    pub fn state(&self) -> Result<AdapterState, AdapterError> {
        self.inner.lock().map(|inner| inner.state).map_err(|_| AdapterError::RegistryPoisoned)
    }

    pub fn history(&self) -> Result<Vec<LifecycleEvent>, AdapterError> {
        self.inner.lock().map(|inner| inner.history.iter().cloned().collect()).map_err(|_| AdapterError::RegistryPoisoned)
    }

    pub fn transition(&self, target: AdapterState, reason: Option<String>) -> Result<LifecycleEvent, AdapterError> {
        if reason.as_ref().is_some_and(|value| value.trim().is_empty() || value.trim() != value) {
            return Err(AdapterError::InvalidRequest("lifecycle reason must be non-empty and trimmed when provided".into()));
        }
        let mut inner = self.inner.lock().map_err(|_| AdapterError::RegistryPoisoned)?;
        let current = inner.state;
        if !is_valid_transition(current, target) {
            return Err(AdapterError::InvalidState { expected: allowed_targets(current), actual: current });
        }
        let changed_at_unix_ms = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_millis();
        inner.sequence = inner.sequence.saturating_add(1);
        let event = LifecycleEvent { sequence: inner.sequence, from: current, to: target, changed_at_unix_ms, reason };
        inner.state = target;
        inner.history.push_back(event.clone());
        if inner.history.len() > self.history_limit {
            inner.history.pop_front();
        }
        Ok(event)
    }
}

fn is_valid_transition(from: AdapterState, to: AdapterState) -> bool {
    use AdapterState::*;
    matches!((from, to),
        (Registered, Initializing | Failed) |
        (Initializing, Ready | Degraded | Failed | Stopping) |
        (Ready, Degraded | Stopping | Failed) |
        (Degraded, Ready | Stopping | Failed) |
        (Stopping, Stopped | Failed) |
        (Stopped, Initializing) |
        (Failed, Initializing | Stopping | Stopped))
}

fn allowed_targets(state: AdapterState) -> &'static str {
    match state {
        AdapterState::Registered => "initializing or failed",
        AdapterState::Initializing => "ready, degraded, stopping, or failed",
        AdapterState::Ready => "degraded, stopping, or failed",
        AdapterState::Degraded => "ready, stopping, or failed",
        AdapterState::Stopping => "stopped or failed",
        AdapterState::Stopped => "initializing",
        AdapterState::Failed => "initializing, stopping, or stopped",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::adapter::{AdapterContext, CancellationToken};

    #[test]
    fn lifecycle_rejects_illegal_transitions_and_keeps_bounded_history() {
        let lifecycle = AdapterLifecycle::with_history_limit(NonZeroUsize::new(2).unwrap());
        assert!(lifecycle.transition(AdapterState::Ready, None).is_err());
        lifecycle.transition(AdapterState::Initializing, None).unwrap();
        lifecycle.transition(AdapterState::Ready, None).unwrap();
        lifecycle.transition(AdapterState::Degraded, Some("upstream latency".into())).unwrap();
        let history = lifecycle.history().unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].sequence, 2);
        assert_eq!(history[1].to, AdapterState::Degraded);
    }

    #[test]
    fn cancelled_context_fails_before_dispatch() {
        let cancellation = CancellationToken::default();
        let mut context = AdapterContext::new("request-1", Duration::from_secs(5)).unwrap();
        context.cancellation = cancellation.clone();
        cancellation.cancel();
        assert!(matches!(context.check_active(), Err(AdapterError::Cancelled)));
    }
}
