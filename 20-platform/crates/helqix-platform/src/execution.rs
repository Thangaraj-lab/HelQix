//! Deterministic execution admission and lease contracts.
use std::collections::BTreeSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::workers::{WorkerRegistration, WorkerState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRequest { pub id: String, pub tenant_id: String, pub operation: String, pub required_capabilities: BTreeSet<String>, pub timeout_ms: u64, pub max_retries: u32 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionState { Queued, Leased, Running, Succeeded, Failed, Cancelled, TimedOut }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionLease { pub request_id: String, pub worker_id: String, pub lease_id: String, pub expires_at_ms: u64 }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError { InvalidRequest(&'static str), WorkerUnavailable, CapabilityMismatch, LeaseExpired }

pub fn validate_request(request: &ExecutionRequest) -> Result<(), ExecutionError> {
    if request.id.trim().is_empty() || request.tenant_id.trim().is_empty() || request.operation.trim().is_empty() { return Err(ExecutionError::InvalidRequest("id, tenant_id and operation are required")); }
    if request.timeout_ms == 0 { return Err(ExecutionError::InvalidRequest("timeout_ms must be positive")); }
    Ok(())
}
pub fn select_worker<'a>(request: &ExecutionRequest, workers: &'a [WorkerRegistration]) -> Result<&'a WorkerRegistration, ExecutionError> {
    validate_request(request)?; let mut ready=false;
    for worker in workers { if worker.state != WorkerState::Ready { continue; } ready=true; if request.required_capabilities.iter().all(|cap| worker.capabilities.supports(cap)) { return Ok(worker); } }
    Err(if ready { ExecutionError::CapabilityMismatch } else { ExecutionError::WorkerUnavailable })
}
pub fn create_lease(request: &ExecutionRequest, worker: &WorkerRegistration, lease_id: String) -> Result<ExecutionLease, ExecutionError> {
    validate_request(request)?; if worker.state != WorkerState::Ready { return Err(ExecutionError::WorkerUnavailable); }
    if !request.required_capabilities.iter().all(|cap| worker.capabilities.supports(cap)) { return Err(ExecutionError::CapabilityMismatch); }
    Ok(ExecutionLease { request_id: request.id.clone(), worker_id: worker.worker_id.clone(), lease_id, expires_at_ms: now_ms().saturating_add(request.timeout_ms) })
}
pub fn lease_is_valid(lease: &ExecutionLease) -> bool { now_ms() < lease.expires_at_ms }
pub fn validate_lease(lease: &ExecutionLease, request_id: &str, worker_id: &str) -> Result<(), ExecutionError> { if lease.request_id != request_id || lease.worker_id != worker_id || !lease_is_valid(lease) { return Err(ExecutionError::LeaseExpired); } Ok(()) }
fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_millis() as u64 }

#[cfg(test)]
mod tests {
 use super::*; use crate::workers::{WorkerCapabilities, WorkerRegistration};
 fn worker()->WorkerRegistration { WorkerRegistration::register("worker-1", WorkerCapabilities { operations:["simulation".into(),"openfoam".into()].into_iter().collect(), cpu_cores:8, memory_bytes:16_384, gpu_count:0 }, 1).unwrap() }
 #[test] fn selects_matching_worker(){ let r=ExecutionRequest{id:"job-1".into(),tenant_id:"t-1".into(),operation:"simulation".into(),required_capabilities:["simulation".into()].into_iter().collect(),timeout_ms:1000,max_retries:2}; assert_eq!(select_worker(&r,&[worker()]).unwrap().worker_id,"worker-1"); }
 #[test] fn rejects_missing_capability(){ let r=ExecutionRequest{id:"job-1".into(),tenant_id:"t-1".into(),operation:"quantum".into(),required_capabilities:["quantum".into()].into_iter().collect(),timeout_ms:1000,max_retries:0}; assert_eq!(select_worker(&r,&[worker()]),Err(ExecutionError::CapabilityMismatch)); }
}
