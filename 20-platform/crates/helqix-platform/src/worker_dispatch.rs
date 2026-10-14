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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::ResourceScope;
    use crate::execution::{ExecutionLease, ExecutionRequest};
    use crate::isolation::IsolationPolicy;
    use crate::workers::{WorkerCapabilities, WorkerRegistration};
    use std::collections::BTreeSet;
    struct Fake;
    impl SandboxedExecutor for Fake { type Output=String; type Error=String; fn execute(&self,_:&WorkerExecutionSpec,_:&AtomicBool)->Result<String,String>{Ok("ok".into())} }
    fn worker()->WorkerRegistration{WorkerRegistration::register("worker-1",WorkerCapabilities{operations:["simulation".into()].into_iter().collect(),cpu_cores:2,memory_bytes:1024,gpu_count:0},1).unwrap()}
    fn spec(token:u64)->WorkerExecutionSpec{WorkerExecutionSpec{request:ExecutionRequest{id:"job-1".into(),tenant_id:"tenant-1".into(),operation:"simulation".into(),required_capabilities:["simulation".into()].into_iter().collect(),timeout_ms:10_000,max_retries:1},lease:ExecutionLease{request_id:"job-1".into(),worker_id:"worker-1".into(),lease_id:"lease-1".into(),expires_at_ms:u64::MAX},queue_token:token,operation:"simulation".into(),isolation:IsolationPolicy{network_enabled:false,allowed_hosts:BTreeSet::new(),read_only_root:true,writable_paths:["/workspace".into()].into_iter().collect(),max_processes:2,max_memory_bytes:1024,max_cpu_millis:10_000,secret_ids:BTreeSet::new()},limits:crate::worker_runtime::RuntimeLimits{timeout_ms:10_000,max_retries:1}}}
    #[test] fn completion_is_fenced_by_queue_token(){let mut q=JobQueue::new(2).unwrap();q.submit(crate::jobs::Job::new("job-1",ResourceScope::project("w","p"),"simulation",1,1,2).unwrap()).unwrap();let lease=q.lease_next("worker-1",10,100).unwrap().unwrap();let s=spec(lease.token);let out=execute_leased_job(&mut q,&worker(),&s,20,21,&Fake,&AtomicBool::new(false)).unwrap();assert_eq!(out,"ok");assert_eq!(q.get("job-1").unwrap().state,crate::jobs::JobState::Succeeded);}
    #[test] fn forged_token_is_rejected(){let mut q=JobQueue::new(2).unwrap();q.submit(crate::jobs::Job::new("job-1",ResourceScope::project("w","p"),"simulation",1,1,2).unwrap()).unwrap();q.lease_next("worker-1",10,100).unwrap().unwrap();let s=spec(999);assert!(matches!(execute_leased_job(&mut q,&worker(),&s,20,21,&Fake,&AtomicBool::new(false)),Err(WorkerDispatchError::Queue(QueueError::StaleLease))));}
}
