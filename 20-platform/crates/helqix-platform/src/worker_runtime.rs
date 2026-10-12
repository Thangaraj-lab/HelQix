//! Worker-side execution boundary.
//!
//! This module validates a leased request against a worker's registration and
//! the isolation manifest before handing control to a platform-specific
//! sandbox adapter. It deliberately does not pretend that policy validation
//! alone is an OS/container sandbox.
use crate::execution::{validate_lease, ExecutionError, ExecutionLease, ExecutionRequest};
use crate::isolation::{IsolationError, IsolationPolicy};
use crate::workers::{WorkerRegistration, WorkerState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLimits {
    pub timeout_ms: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerExecutionSpec {
    pub request: ExecutionRequest,
    pub lease: ExecutionLease,
    pub operation: String,
    pub isolation: IsolationPolicy,
    pub limits: RuntimeLimits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeValidationError {
    InvalidOperation,
    WorkerNotReady,
    OperationUnsupported,
    Lease(ExecutionError),
    Isolation(IsolationError),
    TimeoutMismatch,
    RetryBudgetMismatch,
    LeaseIdentityMismatch,
}

pub fn validate_worker_execution(
    worker: &WorkerRegistration,
    spec: &WorkerExecutionSpec,
) -> Result<(), RuntimeValidationError> {
    if spec.operation.trim().is_empty() {
        return Err(RuntimeValidationError::InvalidOperation);
    }
    if worker.state != WorkerState::Ready {
        return Err(RuntimeValidationError::WorkerNotReady);
    }
    if !worker.capabilities.supports(&spec.operation) {
        return Err(RuntimeValidationError::OperationUnsupported);
    }
    spec.isolation
        .validate()
        .map_err(RuntimeValidationError::Isolation)?;
    if spec.limits.timeout_ms == 0 || spec.request.timeout_ms != spec.limits.timeout_ms {
        return Err(RuntimeValidationError::TimeoutMismatch);
    }
    if spec.request.max_retries != spec.limits.max_retries {
        return Err(RuntimeValidationError::RetryBudgetMismatch);
    }
    if spec.lease.request_id != spec.request.id || spec.lease.worker_id != worker.worker_id {
        return Err(RuntimeValidationError::LeaseIdentityMismatch);
    }
    validate_lease(&spec.lease, &spec.request.id, &worker.worker_id)
        .map_err(RuntimeValidationError::Lease)?;
    if !spec
        .request
        .required_capabilities
        .iter()
        .all(|cap| worker.capabilities.supports(cap))
    {
        return Err(RuntimeValidationError::OperationUnsupported);
    }
    Ok(())
}

/// Interface implemented by a concrete container/OS sandbox. A production
/// implementation must enforce network/filesystem/process/resource rules, not
/// merely store this specification.
pub trait SandboxedExecutor {
    type Output;
    type Error;

    fn execute(
        &self,
        spec: &WorkerExecutionSpec,
        cancellation: &std::sync::atomic::AtomicBool,
    ) -> Result<Self::Output, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isolation::IsolationPolicy;
    use crate::workers::{WorkerCapabilities, WorkerRegistration};
    use std::collections::BTreeSet;

    fn worker() -> WorkerRegistration {
        WorkerRegistration::register(
            "worker-1",
            WorkerCapabilities {
                operations: ["simulation".to_owned()].into_iter().collect(),
                cpu_cores: 4,
                memory_bytes: 1_073_741_824,
                gpu_count: 0,
            },
            100,
        )
        .unwrap()
    }

    fn spec() -> WorkerExecutionSpec {
        let request = ExecutionRequest {
            id: "job-1".into(),
            tenant_id: "tenant-1".into(),
            operation: "simulation".into(),
            required_capabilities: ["simulation".into()].into_iter().collect(),
            timeout_ms: 5_000,
            max_retries: 1,
        };
        let lease = ExecutionLease {
            request_id: "job-1".into(),
            worker_id: "worker-1".into(),
            lease_id: "lease-1".into(),
            expires_at_ms: u64::MAX,
        };
        WorkerExecutionSpec {
            request,
            lease,
            operation: "simulation".into(),
            isolation: IsolationPolicy {
                network_enabled: false,
                allowed_hosts: BTreeSet::new(),
                read_only_root: true,
                writable_paths: ["/workspace".into()].into_iter().collect(),
                max_processes: 4,
                max_memory_bytes: 1_073_741_824,
                max_cpu_millis: 5_000,
                secret_ids: BTreeSet::new(),
            },
            limits: RuntimeLimits {
                timeout_ms: 5_000,
                max_retries: 1,
            },
        }
    }

    #[test]
    fn accepts_a_valid_worker_specification() {
        assert!(validate_worker_execution(&worker(), &spec()).is_ok());
    }

    #[test]
    fn rejects_mismatched_lease_identity() {
        let mut s = spec();
        s.lease.worker_id = "other".into();
        assert_eq!(
            validate_worker_execution(&worker(), &s),
            Err(RuntimeValidationError::LeaseIdentityMismatch)
        );
    }

    #[test]
    fn rejects_unbounded_or_invalid_isolation() {
        let mut s = spec();
        s.isolation.max_memory_bytes = 0;
        assert!(matches!(
            validate_worker_execution(&worker(), &s),
            Err(RuntimeValidationError::Isolation(_))
        ));
    }

    #[test]
    fn rejects_mismatched_runtime_timeout() {
        let mut s = spec();
        s.limits.timeout_ms = 1;
        assert_eq!(
            validate_worker_execution(&worker(), &s),
            Err(RuntimeValidationError::TimeoutMismatch)
        );
    }
}
