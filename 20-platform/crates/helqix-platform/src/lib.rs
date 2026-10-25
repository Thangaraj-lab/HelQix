//! HELQIX Layer 20 — platform control plane and worker execution contracts.
//!
//! Durable storage, distributed coordination, authentication and container
//! runtime installation remain production adapters; this crate exposes the
//! fail-closed contracts and Linux Podman execution boundary.

#![forbid(unsafe_code)]

pub mod access;
pub mod audit;
pub mod execution;
pub mod isolation;
pub mod jobs;
pub mod limits;
pub mod podman_executor;
pub mod process_executor;
pub mod worker_dispatch;
pub mod worker_runtime;
pub mod workers;

pub use access::{AccessDecision, AccessPolicy, Action, Principal, ResourceScope};
pub use audit::{AuditEvent, AuditLog, AuditOutcome, AuditSubject};
pub use execution::{create_lease, lease_is_valid, select_worker, validate_lease, validate_request, ExecutionError, ExecutionLease, ExecutionRequest, ExecutionState};
pub use isolation::{IsolationError, IsolationPolicy};
pub use jobs::{Job, JobId, JobQueue, JobState, Lease, QueueError};
pub use limits::{AdmissionLimits, AdmissionUsage, LimitError};
pub use podman_executor::{PodmanCommand, PodmanError, PodmanSandboxExecutor, SandboxOutput};
pub use process_executor::{LocalProcessExecutor, ProcessExecutionError, ProcessOutput, ProcessSpec};
pub use worker_dispatch::{execute_leased_job, WorkerDispatchError};
pub use worker_runtime::{validate_worker_execution, RuntimeLimits, RuntimeValidationError, SandboxedExecutor, WorkerExecutionSpec};
pub use workers::{WorkerCapabilities, WorkerError, WorkerRegistration, WorkerState};
