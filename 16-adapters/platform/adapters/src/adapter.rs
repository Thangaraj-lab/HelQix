use std::{fmt, time::Duration};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{capability::AdapterDescriptor, lifecycle::AdapterState};

/// Per-operation execution metadata supplied by the host runtime.
#[derive(Clone, Debug)]
pub struct AdapterContext {
    pub request_id: String,
    pub deadline: std::time::Instant,
    pub timeout: Duration,
    pub cancellation: CancellationToken,
}

impl AdapterContext {
    pub fn new(request_id: impl Into<String>, timeout: Duration) -> Result<Self, AdapterError> {
        let request_id = request_id.into();
        if request_id.trim().is_empty() || request_id.trim() != request_id {
            return Err(AdapterError::InvalidRequest("request_id must be non-empty and trimmed".into()));
        }
        if timeout.is_zero() {
            return Err(AdapterError::InvalidRequest("timeout must be greater than zero".into()));
        }
        Ok(Self {
            request_id,
            deadline: std::time::Instant::now().checked_add(timeout)
                .ok_or_else(|| AdapterError::InvalidRequest("timeout exceeds the supported deadline range".into()))?,
            timeout,
            cancellation: CancellationToken::default(),
        })
    }

    pub fn is_cancelled(&self) -> bool { self.cancellation.is_cancelled() }

    pub fn check_active(&self) -> Result<(), AdapterError> {
        if self.is_cancelled() { return Err(AdapterError::Cancelled); }
        if self.remaining().is_zero() { return Err(AdapterError::DeadlineExceeded); }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), AdapterError> {
        if self.request_id.trim().is_empty() || self.request_id.trim() != self.request_id {
            return Err(AdapterError::InvalidRequest("request_id must be non-empty and trimmed".into()));
        }
        if self.timeout.is_zero() {
            return Err(AdapterError::InvalidRequest("timeout must be greater than zero".into()));
        }
        Ok(())
    }

    pub fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(std::time::Instant::now())
    }
}

/// Small cooperative cancellation primitive that can be cloned across tasks.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, std::sync::atomic::Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::Acquire)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterRequest {
    pub operation: String,
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

impl AdapterRequest {
    pub fn validate(&self) -> Result<(), AdapterError> {
        if self.operation.trim().is_empty() || self.operation.trim() != self.operation {
            return Err(AdapterError::InvalidRequest("operation must be non-empty and trimmed".into()));
        }
        if self.operation.len() > 128 {
            return Err(AdapterError::InvalidRequest("operation must be at most 128 bytes".into()));
        }
        if self.idempotency_key.as_ref().is_some_and(|key| key.trim().is_empty() || key.trim() != key) {
            return Err(AdapterError::InvalidRequest("idempotency_key must be non-empty and trimmed".into()));
        }
        if self.idempotency_key.as_ref().is_some_and(|key| key.len() > 256) {
            return Err(AdapterError::InvalidRequest("idempotency_key must be at most 256 bytes".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterResponse {
    pub payload: serde_json::Value,
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthReport {
    pub status: HealthStatus,
    pub checked_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Host-facing adapter contract. Implementations must honor deadlines and
/// cancellation where their underlying provider supports it.
#[async_trait]
pub trait Adapter: Send + Sync {
    fn descriptor(&self) -> &AdapterDescriptor;
    fn state(&self) -> AdapterState;

    async fn initialize(&self, context: &AdapterContext) -> Result<(), AdapterError>;
    async fn execute(&self, context: &AdapterContext, request: AdapterRequest) -> Result<AdapterResponse, AdapterError>;
    async fn health_check(&self, context: &AdapterContext) -> Result<HealthReport, AdapterError>;
    async fn shutdown(&self, context: &AdapterContext) -> Result<(), AdapterError>;
}

#[derive(Debug)]
pub enum AdapterError {
    InvalidRequest(String),
    InvalidDescriptor(String),
    InvalidState { expected: &'static str, actual: AdapterState },
    Cancelled,
    DeadlineExceeded,
    ConcurrencyLimitExceeded { capability: String, limit: usize },
    Provider { code: String, message: String, retryable: bool },
    RegistryPoisoned,
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(formatter, "invalid adapter request: {message}"),
            Self::InvalidDescriptor(message) => write!(formatter, "invalid adapter descriptor: {message}"),
            Self::InvalidState { expected, actual } => write!(formatter, "adapter state is {actual:?}; expected {expected}"),
            Self::Cancelled => write!(formatter, "adapter operation was cancelled"),
            Self::DeadlineExceeded => write!(formatter, "adapter operation exceeded its deadline"),
            Self::ConcurrencyLimitExceeded { capability, limit } => write!(formatter, "adapter capability {capability:?} is at its concurrency limit ({limit})"),
            Self::Provider { code, message, retryable } => write!(formatter, "provider error {code} (retryable={retryable}): {message}"),
            Self::RegistryPoisoned => write!(formatter, "adapter registry lock is poisoned"),
        }
    }
}

impl std::error::Error for AdapterError {}

impl AdapterError {
    /// Whether retrying the same request may succeed without changing input.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::DeadlineExceeded | Self::ConcurrencyLimitExceeded { .. } | Self::Provider { retryable: true, .. })
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest(_) => "invalid_request",
            Self::InvalidDescriptor(_) => "invalid_descriptor",
            Self::InvalidState { .. } => "invalid_state",
            Self::Cancelled => "cancelled",
            Self::DeadlineExceeded => "deadline_exceeded",
            Self::ConcurrencyLimitExceeded { .. } => "concurrency_limit_exceeded",
            Self::Provider { .. } => "provider_error",
            Self::RegistryPoisoned => "registry_poisoned",
        }
    }
}
