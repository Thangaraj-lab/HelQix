//! Connects the bounded job queue, leased worker contract and sandbox executor.
//!
//! This is the first end-to-end control-plane path: a queued job is leased,
//! validated against the selected worker, executed through a sandbox boundary,
//! and fenced completion is written back to the queue. The queue remains the
//! source of truth for lifecycle state; an executor never gets to self-assign
//! or complete a job without the lease token.

use crate::jobs::{JobQueue, QueueError};
use crate::worker_runtime::{validate_worker_execution, SandboxedExecutor, RuntimeValidationError, WorkerExecutionSpec};
use crate::workers::WorkerRegistration;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerDispatchError<E> {
    Queue(QueueError),
    Validation(RuntimeValidationError),
    Executor(E),
}

impl<E> From<QueueError> for WorkerDispatchError<E> {
    fn from(error: QueueError) -> Self { Self::Queue(error) }
}

/// Execute one already-admitted job. The caller supplies the exact queue job
/// id, worker registration, immutable execution specification and clock values.
/// This keeps wall-clock access outside the deterministic queue contract.
pub fn execute_leased_job<E: SandboxedExecutor>(
    queue: &mut JobQueue,
    worker: &WorkerRegistration,
    spec: &WorkerExecutionSpec,
    now_ms: u64,
    finish_ms: u64,
    executor: &E,
    cancellation: &AtomicBool,
) -> Result<E::Output, WorkerDispatchError<E::Error>> {
    if spec.request.id != spec.lease.request_id {
        return Err(WorkerDispatchError::Validation(RuntimeValidationError::LeaseIdentityMismatch));
    }
    validate_worker_execution(worker, spec).map_err(WorkerDispatchError::Validation)?;
    let lease = queue
        .get(&spec.request.id)
        .and_then(|job| match &job.state {
            crate::jobs::JobState::Leased { worker_id, token, lease_until_ms } if worker_id == &worker.worker_id && *token == spec.lease.lease_id.parse::<u64>().unwrap_or(0) => Some((*token, *lease_until_ms)),
            _ => None,
        });

    // Queue fencing uses a numeric token while the generic execution lease has
    // its own opaque lease id. A caller must provide the numeric queue token in
    // the lease id for this bridge; production adapters should persist this
    // mapping rather than derive it from mutable process state.
    let Some((token, lease_until_ms)) = lease else {
        return Err(WorkerDispatchError::Queue(QueueError::StaleLease));
    };
    if now_ms >= lease_until_ms {
        return Err(WorkerDispatchError::Queue(QueueError::LeaseExpired));
    }

    let output = match executor.execute(spec, cancellation) {
        Ok(output) => {
            queue.finish(&spec.request.id, &worker.worker_id, token, finish_ms, Ok(()), false)?;
            output
        }
        Err(error) => {
            let reason = "sandbox execution failed".to_owned();
            queue.finish(&spec.request.id, &worker.worker_id, token, finish_ms, Err(reason), true)?;
            return Err(WorkerDispatchError::Executor(error));
        }
    };
    Ok(output)
}
