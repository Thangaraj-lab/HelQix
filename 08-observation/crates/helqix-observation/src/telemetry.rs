//! Bounded in-memory collection of immutable observation frames.
use std::{collections::VecDeque, fmt};
use helqix_time::SimTime;
use crate::observer::ObservationFrame;

/// Behavior when a telemetry buffer reaches its configured capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryPolicy {
    /// Remove the oldest frame to retain the newest sample.
    DropOldest,
    /// Reject the incoming frame and preserve all existing data.
    RejectNewest,
}

/// Bounded, ordered telemetry history. Equal-time frames preserve insertion order.
#[derive(Debug, Clone)]
pub struct TelemetryBuffer {
    capacity: usize,
    policy: TelemetryPolicy,
    frames: VecDeque<ObservationFrame>,
    dropped_frames: u64,
}
impl TelemetryBuffer {
    /// Creates a buffer. Capacity must be greater than zero.
    pub fn new(capacity: usize, policy: TelemetryPolicy) -> Result<Self, TelemetryError> {
        if capacity == 0 { return Err(TelemetryError::ZeroCapacity); }
        Ok(Self { capacity, policy, frames: VecDeque::with_capacity(capacity), dropped_frames: 0 })
    }
    /// Configured maximum frame count.
    pub fn capacity(&self) -> usize { self.capacity }
    /// Current frame count.
    pub fn len(&self) -> usize { self.frames.len() }
    /// Whether the buffer has no frames.
    pub fn is_empty(&self) -> bool { self.frames.is_empty() }
    /// Number of frames discarded due to the configured capacity policy.
    pub fn dropped_frames(&self) -> u64 { self.dropped_frames }
    /// Frames in insertion order (and non-decreasing logical time).
    pub fn frames(&self) -> impl DoubleEndedIterator<Item = &ObservationFrame> { self.frames.iter() }
    /// Latest frame, if present.
    pub fn latest(&self) -> Option<&ObservationFrame> { self.frames.back() }
    /// Earliest retained frame, if present.
    pub fn earliest(&self) -> Option<&ObservationFrame> { self.frames.front() }

    /// Appends a frame atomically after checking temporal order and capacity policy.
    pub fn push(&mut self, frame: ObservationFrame) -> Result<(), TelemetryError> {
        if let Some(last) = self.frames.back() {
            if frame.observed_at() < last.observed_at() {
                return Err(TelemetryError::OutOfOrder { previous: last.observed_at(), attempted: frame.observed_at() });
            }
        }
        if self.frames.len() == self.capacity {
            match self.policy {
                TelemetryPolicy::RejectNewest => return Err(TelemetryError::CapacityReached { capacity: self.capacity }),
                TelemetryPolicy::DropOldest => {
                    let next_drop_count = self.dropped_frames.checked_add(1).ok_or(TelemetryError::DropCountOverflow)?;
                    self.frames.pop_front();
                    self.dropped_frames = next_drop_count;
                }
            }
        }
        self.frames.push_back(frame);
        Ok(())
    }
    /// Clears retained frames. Lifetime dropped-frame count is preserved.
    pub fn clear(&mut self) { self.frames.clear(); }
}

/// Telemetry configuration or append failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryError {
    /// Capacity cannot be zero.
    ZeroCapacity,
    /// The incoming frame is earlier than the latest retained frame.
    OutOfOrder { /// Timestamp of latest retained frame.
        previous: SimTime, /// Incoming timestamp.
        attempted: SimTime },
    /// A reject-newest buffer is already full.
    CapacityReached { /// Configured maximum frame count.
        capacity: usize },
    /// Lifetime drop counter cannot be incremented further.
    DropCountOverflow,
}
impl fmt::Display for TelemetryError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self {
    Self::ZeroCapacity => f.write_str("telemetry capacity must be greater than zero"),
    Self::OutOfOrder { previous, attempted } => write!(f, "telemetry frame at {attempted} precedes latest frame at {previous}"),
    Self::CapacityReached { capacity } => write!(f, "telemetry buffer is full (capacity {capacity})"),
    Self::DropCountOverflow => f.write_str("telemetry dropped-frame counter overflowed") } } }
impl std::error::Error for TelemetryError {}

#[cfg(test)] mod tests {
    use super::*;
    use crate::Observer;
    use helqix_state::State;
    fn frame(n: i128) -> ObservationFrame { Observer::new().observe(&State::new(), SimTime::from_nanos(n)).unwrap() }
    #[test] fn zero_capacity_is_rejected() { assert_eq!(TelemetryBuffer::new(0, TelemetryPolicy::DropOldest).unwrap_err(), TelemetryError::ZeroCapacity); }
    #[test] fn drop_oldest_keeps_newest_and_counts_drops() { let mut b = TelemetryBuffer::new(2, TelemetryPolicy::DropOldest).unwrap(); b.push(frame(1)).unwrap(); b.push(frame(2)).unwrap(); b.push(frame(3)).unwrap(); assert_eq!(b.len(), 2); assert_eq!(b.earliest().unwrap().observed_at(), SimTime::from_nanos(2)); assert_eq!(b.latest().unwrap().observed_at(), SimTime::from_nanos(3)); assert_eq!(b.dropped_frames(), 1); }
    #[test] fn reject_newest_preserves_existing_data() { let mut b = TelemetryBuffer::new(1, TelemetryPolicy::RejectNewest).unwrap(); b.push(frame(1)).unwrap(); assert_eq!(b.push(frame(2)), Err(TelemetryError::CapacityReached { capacity: 1 })); assert_eq!(b.latest().unwrap().observed_at(), SimTime::from_nanos(1)); }
    #[test] fn rejects_backwards_time_without_mutating_history() { let mut b = TelemetryBuffer::new(2, TelemetryPolicy::DropOldest).unwrap(); b.push(frame(5)).unwrap(); assert!(matches!(b.push(frame(4)), Err(TelemetryError::OutOfOrder { .. }))); assert_eq!(b.len(), 1); }
    #[test] fn equal_times_are_valid_and_stably_ordered() { let mut b = TelemetryBuffer::new(2, TelemetryPolicy::DropOldest).unwrap(); b.push(frame(5)).unwrap(); b.push(frame(5)).unwrap(); assert_eq!(b.len(), 2); }
}
