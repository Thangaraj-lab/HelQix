//! Connects the bounded job queue, leased worker contract and sandbox executor.
use crate::jobs::{JobQueue, QueueError};
use crate::worker_runtime::{validate_worker_execution, RuntimeValidationError, SandboxedExecutor, WorkerExecutionSpec};
use crate::workers::WorkerRegistration;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerDispatchError<E> { Queue(QueueError), Validation(RuntimeValidationError), Executor(E) }
impl<E> From<QueueError> for WorkerDispatchError<E> { fn from(error: QueueError) -> Self { Self::Queue(error) } }

/// Execute one leased queue job and fence its completion with the exact token
/// issued by JobQueue. Clock values are injected so callers/tests control time.
pub fn execute_leased_job<E: SandboxedExecutor>(
    queue: &mut JobQueue,
    worker: &WorkerRegistration,
    spec: &WorkerExecutionSpec,
    now_ms: u64,
    finish_ms: u64,
    executor: &E,
    cancellation: &AtomicBool,
) -> Result<E::Output, WorkerDispatchError<E::Error>> {
    validate_worker_execution(worker, spec).map_err(WorkerDispatchError::Validation)?;
    let lease = queue.get(&spec.request.id).and_then(|job| match &job.state {
        crate::jobs::JobState::Leased { worker_id, token, lease_until_ms }
            if worker_id == &worker.worker_id && *token == spec.queue_token => Some((*token, *lease_until_ms)),
        _ => None,
    });
    let Some((token, lease_until_ms)) = lease else { return Err(WorkerDispatchError::Queue(QueueError::StaleLease)); };
    if now_ms >= lease_until_ms { return Err(WorkerDispatchError::Queue(QueueError::LeaseExpired)); }
    match executor.execute(spec, cancellation) {
        Ok(output) => { queue.finish(&spec.request.id, &worker.worker_id, token, finish_ms, Ok(()), false)?; Ok(output) }
        Err(error) => { queue.finish(&spec.request.id, &worker.worker_id, token, finish_ms, Err("sandbox execution failed".into()), true)?; Err(WorkerDispatchError::Executor(error)) }
    }
}
