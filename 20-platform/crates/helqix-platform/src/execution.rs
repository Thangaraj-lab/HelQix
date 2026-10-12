//! Production-facing execution admission and lease contracts.
//!
//! This layer intentionally contains no process spawning. It defines the
//! deterministic control-plane boundary used by API schedulers and workers.

use std::collections::BTreeSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::workers::{WorkerCapabilities, WorkerError, WorkerRegistration, WorkerState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRequest {
    pub id: String,
    pub tenant_id: String,
    pub operation: String,
    pub required_capabilities: BTreeSet<String>,
    pub timeout_ms: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionState {
    Queued,
    Leased,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionLease {
    pub request_id: String,
    pub worker_id: String,
    pub lease_id: String,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    InvalidRequest(&'static str),
    WorkerUnavailable,
    CapabilityMismatch,
    InvalidState,
    LeaseExpired,
}

pub fn validate_request(request: &ExecutionRequest) -> Result<(), ExecutionError> {
    if request.id.trim().is_empty() || request.tenant_id.trim().is_empty() || request.operation.trim().is_empty() {
        return Err(ExecutionError::InvalidRequest("id, tenant_id and operation are required"));
    }
    if request.timeout_ms == 0 {
        return Err(ExecutionError::InvalidRequest("timeout_ms must be positive"));
    }
    Ok(())
}

pub fn select_worker<'a>(request: &ExecutionRequest, workers: &'a [WorkerRegistration]) -> Result<&'a WorkerRegistration, ExecutionError> {
    validate_request(request)?;
    workers.iter().find(|worker| {
        worker.state == WorkerState::Ready
            && capabilities_match(&worker.capabilities, &request.required_capabilities)
    }).ok_or_else(|| {
        if workers.iter().any(|worker| worker.state == WorkerState::Ready) {
            ExecutionError::CapabilityMismatch
        } else {
            ExecutionError::WorkerUnavailable
        }
    })
}

fn capabilities_match(capabilities: &WorkerCapabilities, required: &BTreeSet<String>) -> bool {
    required.iter().all(|item| capabilities.supported_operations.contains(item))
}

pub fn create_lease(request: &ExecutionRequest, worker: &WorkerRegistration, lease_id: String) -> Result<ExecutionLease, ExecutionError> {
    validate_request(request)?;
    if worker.state != WorkerState::Ready {
        return Err(ExecutionError::WorkerUnavailable);
    }
    if !capabilities_match(&worker.capabilities, &request.required_capabilities) {
        return Err(ExecutionError::CapabilityMismatch);
    }
    let now = now_ms();
    Ok(ExecutionLease { request_id: request.id.clone(), worker_id: worker.id.clone(), lease_id, expires_at_ms: now.saturating_add(request.timeout_ms) })
}

pub fn lease_is_valid(lease: &ExecutionLease) -> bool { now_ms() < lease.expires_at_ms }

pub fn validate_lease(lease: &ExecutionLease, request_id: &str, worker_id: &str) -> Result<(), ExecutionError> {
    if lease.request_id != request_id || lease.worker_id != worker_id { return Err(ExecutionError::LeaseExpired); }
    if !lease_is_valid(lease) { return Err(ExecutionError::LeaseExpired); }
    Ok(())
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_millis() as u64
}

impl From<WorkerError> for ExecutionError {
    fn from(_: WorkerError) -> Self { ExecutionError::WorkerUnavailable }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workers::{WorkerCapabilities, WorkerRegistration, WorkerState};

    fn worker() -> WorkerRegistration {
        WorkerRegistration {
            id: "worker-1".into(),
            state: WorkerState::Ready,
            capabilities: WorkerCapabilities { supported_operations: ["simulation".into(), "openfoam".into()].into_iter().collect(), cpu_cores: 8, memory_mb: 16_384, gpu_count: 0 },
            heartbeat_at_ms: 1,
        }
    }

    #[test]
    fn selects_ready_worker_with_required_capability() {
        let request = ExecutionRequest { id: "job-1".into(), tenant_id: "tenant-1".into(), operation: "simulation".into(), required_capabilities: ["simulation".into()].into_iter().collect(), timeout_ms: 1000, max_retries: 2 };
        assert_eq!(select_worker(&request, &[worker()]).unwrap().id, "worker-1");
    }

    #[test]
    fn rejects_missing_capability() {
        let request = ExecutionRequest { id: "job-1".into(), tenant_id: "tenant-1".into(), operation: "quantum".into(), required_capabilities: ["quantum".into()].into_iter().collect(), timeout_ms: 1000, max_retries: 0 };
        assert_eq!(select_worker(&request, &[worker()]), Err(ExecutionError::CapabilityMismatch));
    }
}
