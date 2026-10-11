//! Admission budgets are checked before the platform accepts expensive work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionLimits { pub max_queued: usize, pub max_running: usize, pub max_payload_bytes: usize }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AdmissionUsage { pub queued: usize, pub running: usize }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitError { QueueFull { maximum: usize }, ConcurrencyFull { maximum: usize }, PayloadTooLarge { bytes: usize, maximum: usize }, ZeroLimit }
impl AdmissionLimits {
    pub fn validate(self) -> Result<Self, LimitError> {
        if self.max_queued == 0 || self.max_running == 0 || self.max_payload_bytes == 0 { return Err(LimitError::ZeroLimit); }
        Ok(self)
    }
    pub fn admit(self, usage: AdmissionUsage, payload_bytes: usize) -> Result<(), LimitError> {
        self.validate()?;
        if payload_bytes > self.max_payload_bytes { return Err(LimitError::PayloadTooLarge { bytes: payload_bytes, maximum: self.max_payload_bytes }); }
        if usage.running < self.max_running { return Ok(()); }
        if usage.queued >= self.max_queued { return Err(LimitError::QueueFull { maximum: self.max_queued }); }
        Ok(())
    }
}
impl std::fmt::Display for LimitError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self { Self::QueueFull{maximum} => write!(f,"execution queue is full (maximum {maximum})"), Self::ConcurrencyFull{maximum} => write!(f,"execution concurrency limit {maximum} reached"), Self::PayloadTooLarge{bytes,maximum} => write!(f,"payload size {bytes} exceeds limit {maximum}"), Self::ZeroLimit => f.write_str("admission limits must be positive") } } }
impl std::error::Error for LimitError {}

#[cfg(test)]
mod tests { use super::*; #[test] fn oversized_payload_is_rejected_before_queueing() { let limits=AdmissionLimits{max_queued:4,max_running:2,max_payload_bytes:10}; assert_eq!(limits.admit(AdmissionUsage::default(),11),Err(LimitError::PayloadTooLarge{bytes:11,maximum:10})); } }
