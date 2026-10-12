//! Worker control-plane contracts. Concrete process/container orchestration is
//! deliberately outside this crate; these types define the safe boundary used
//! by API and worker adapters.
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerState { Starting, Ready, Draining, Offline }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerCapabilities {
    pub operations: BTreeSet<String>,
    pub cpu_cores: u32,
    pub memory_bytes: u64,
    pub gpu_count: u32,
}

impl WorkerCapabilities {
    pub fn supports(&self, operation: &str) -> bool { self.operations.contains(operation) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerRegistration {
    pub worker_id: String,
    pub state: WorkerState,
    pub capabilities: WorkerCapabilities,
    pub registered_at_ms: u64,
    pub last_heartbeat_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerError {
    EmptyWorkerId,
    EmptyOperation,
    InvalidCapacity,
    ClockReversed,
    UnsupportedOperation,
    StaleHeartbeat,
}

impl WorkerRegistration {
    pub fn register(worker_id: impl Into<String>, capabilities: WorkerCapabilities, now_ms: u64) -> Result<Self, WorkerError> {
        let worker_id = worker_id.into();
        if worker_id.trim().is_empty() { return Err(WorkerError::EmptyWorkerId); }
        if capabilities.cpu_cores == 0 || capabilities.memory_bytes == 0 { return Err(WorkerError::InvalidCapacity); }
        Ok(Self { worker_id, state: WorkerState::Ready, capabilities, registered_at_ms: now_ms, last_heartbeat_ms: now_ms })
    }

    pub fn heartbeat(&mut self, now_ms: u64) -> Result<(), WorkerError> {
        if now_ms < self.last_heartbeat_ms { return Err(WorkerError::ClockReversed); }
        if self.state == WorkerState::Offline { return Err(WorkerError::StaleHeartbeat); }
        self.last_heartbeat_ms = now_ms;
        if self.state == WorkerState::Starting { self.state = WorkerState::Ready; }
        Ok(())
    }

    pub fn can_accept(&self, operation: &str) -> Result<(), WorkerError> {
        if operation.trim().is_empty() { return Err(WorkerError::EmptyOperation); }
        if self.state != WorkerState::Ready { return Err(WorkerError::UnsupportedOperation); }
        if !self.capabilities.supports(operation) { return Err(WorkerError::UnsupportedOperation); }
        Ok(())
    }

    pub fn drain(&mut self) { self.state = WorkerState::Draining; }
    pub fn offline(&mut self) { self.state = WorkerState::Offline; }
}

impl std::fmt::Display for WorkerError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") } }
impl std::error::Error for WorkerError {}

#[cfg(test)]
mod tests {
    use super::*;
    fn capabilities() -> WorkerCapabilities {
        WorkerCapabilities { operations: ["simulation".into(), "quantum".into()].into_iter().collect(), cpu_cores: 4, memory_bytes: 1024, gpu_count: 0 }
    }
    #[test] fn worker_rejects_unsupported_operations() {
        let worker = WorkerRegistration::register("worker-1", capabilities(), 10).unwrap();
        assert_eq!(worker.can_accept("openfoam"), Err(WorkerError::UnsupportedOperation));
    }
    #[test] fn heartbeat_never_moves_backwards() {
        let mut worker = WorkerRegistration::register("worker-1", capabilities(), 10).unwrap();
        assert_eq!(worker.heartbeat(9), Err(WorkerError::ClockReversed));
    }
}
