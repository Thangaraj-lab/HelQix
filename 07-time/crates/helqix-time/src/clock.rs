//! Deterministic logical clock for HELQIX.
//!
//! This module is the temporal foundation of the HELQIX Engineering
//! Executable-Reality Platform. It defines:
//!
//! * [`SimTime`]  — an exact point in HELQIX logical time.
//! * [`SimDelta`] — a signed temporal delta between two points.
//! * [`Clock`]    — a deterministic forward-only clock that owns the
//!   current temporal position of a single execution timeline.
//! * [`ClockError`] — the error surface for clock operations.
//!
//! # Responsibility
//!
//! `clock.rs` is deliberately narrow. It does **not** own engineering
//! state, execute actions, run simulation, observe failures, or manage
//! snapshots / branches. Those responsibilities belong to Layers 04–11.
//! The clock exists solely to answer:
//!
//! > "What is the current logical time of this execution, and how did
//! > it get there?"
//!
//! # Determinism
//!
//! Same initial clock state + same sequence of valid advancement
//! operations ⇒ same resulting clock state, on every platform, in
//! every process, in every thread. No wall clock is consulted, no
//! floating point is used, no global state is mutated, no locking or
//! interior mutability is required.
//!
//! # Numeric representation
//!
//! [`SimTime`] and [`SimDelta`] are signed integer nanosecond counts
//! stored in `i128`. This yields:
//!
//! * exact, order-preserving comparison;
//! * exact, associative integer addition;
//! * a representable range of roughly ±5.4 × 10¹² years;
//! * a resolution of one nanosecond;
//! * trivial, portable serialization as an `i128`.
//!
//! Floating point is never used for temporal identity or arithmetic.
//! Conversions to `f64` are deliberately **not** provided by this
//! module; display formatting uses exact integer arithmetic.
//!
//! # Boundaries
//!
//! * Wall-clock time and real-time scheduling belong to an
//!   integration/platform boundary, not here.
//! * Physics timesteps belong to Layer 06 (simulation).
//! * Rewind, replay, and branch timelines belong to
//!   `07-time::rewind`, `07-time::replay`, `07-time::timeline`, and
//!   `11-branching`. They construct [`Clock`] values via
//!   [`Clock::starting_at`] when moving the logical position.
//! * Pause / resume is intentionally **not** part of the clock
//!   contract. HELQIX does not model time as "flowing"; it is
//!   advanced explicitly by the execution layer.

use core::fmt;

// ============================================================================
// SimTime
// ============================================================================

/// An exact point in HELQIX logical time.
///
/// Internally a signed nanosecond count in `i128`.
///
/// * Range: approximately ±5.4 × 10¹² years.
/// * Resolution: 1 ns.
/// * Arithmetic: exact integer.
///
/// Signed because:
///
/// * timestamps of historical events (rewind / branch points) may lie
///   before a given timeline's origin;
/// * deltas between timestamps are naturally signed.
///
/// The [`Clock`] itself only moves forward from its origin — a
/// negative `SimTime` does not imply a clock can move backwards.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SimTime(i128);

impl SimTime {
    /// The temporal origin of a new timeline.
    pub const ZERO: Self = Self(0);

    /// Smallest representable `SimTime`.
    pub const MIN: Self = Self(i128::MIN);

    /// Largest representable `SimTime`.
    pub const MAX: Self = Self(i128::MAX);

    /// Nanoseconds in one microsecond.
    pub const NANOS_PER_MICRO: i128 = 1_000;

    /// Nanoseconds in one millisecond.
    pub const NANOS_PER_MILLI: i128 = 1_000_000;

    /// Nanoseconds in one second.
    pub const NANOS_PER_SEC: i128 = 1_000_000_000;

    /// Construct a `SimTime` from a raw nanosecond count.
    #[inline]
    #[must_use]
    pub const fn from_nanos(nanos: i128) -> Self {
        Self(nanos)
    }

    /// Construct from microseconds. Returns `None` on overflow.
    #[inline]
    #[must_use]
    pub const fn from_micros(micros: i128) -> Option<Self> {
        match micros.checked_mul(Self::NANOS_PER_MICRO) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Construct from milliseconds. Returns `None` on overflow.
    #[inline]
    #[must_use]
    pub const fn from_millis(millis: i128) -> Option<Self> {
        match millis.checked_mul(Self::NANOS_PER_MILLI) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Construct from seconds. Returns `None` on overflow.
    #[inline]
    #[must_use]
    pub const fn from_secs(secs: i128) -> Option<Self> {
        match secs.checked_mul(Self::NANOS_PER_SEC) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Construct from `secs` seconds plus `nanos` nanoseconds.
    ///
    /// The interpretation is purely mathematical:
    /// `result = secs * 1e9 + nanos` nanoseconds. In particular
    /// `from_secs_nanos(-2, 500_000_000)` yields `-1.5` seconds.
    ///
    /// Returns `None` if `nanos >= 1_000_000_000` or the arithmetic
    /// overflows.
    #[inline]
    #[must_use]
    pub const fn from_secs_nanos(secs: i128, nanos: u32) -> Option<Self> {
        if nanos >= Self::NANOS_PER_SEC as u32 {
            return None;
        }
        match secs.checked_mul(Self::NANOS_PER_SEC) {
            Some(base) => match base.checked_add(nanos as i128) {
                Some(n) => Some(Self(n)),
                None => None,
            },
            None => None,
        }
    }

    /// Raw nanosecond count.
    #[inline]
    #[must_use]
    pub const fn as_nanos(self) -> i128 {
        self.0
    }

    /// Is this exactly the temporal origin?
    #[inline]
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Add a delta, returning `None` on overflow.
    #[inline]
    #[must_use]
    pub const fn checked_add(self, delta: SimDelta) -> Option<Self> {
        match self.0.checked_add(delta.0) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Subtract a delta, returning `None` on overflow.
    #[inline]
    #[must_use]
    pub const fn checked_sub(self, delta: SimDelta) -> Option<Self> {
        match self.0.checked_sub(delta.0) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Compute `self - earlier` as a [`SimDelta`], or `None` on overflow.
    #[inline]
    #[must_use]
    pub const fn checked_duration_since(self, earlier: Self) -> Option<SimDelta> {
        match self.0.checked_sub(earlier.0) {
            Some(n) => Some(SimDelta(n)),
            None => None,
        }
    }
}

impl fmt::Display for SimTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.0;
        let sign = if n < 0 { "-" } else { "" };
        let abs = n.unsigned_abs();
        let secs = abs / (Self::NANOS_PER_SEC as u128);
        let nanos = abs % (Self::NANOS_PER_SEC as u128);
        write!(f, "{}{}.{:09}s", sign, secs, nanos)
    }
}

impl fmt::Debug for SimTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SimTime({})", self)
    }
}

impl Default for SimTime {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

// ============================================================================
// SimDelta
// ============================================================================

/// A signed temporal delta between two [`SimTime`] values.
///
/// Stored internally as a signed nanosecond count in `i128`. Sharing
/// the numeric representation of [`SimTime`] ensures that
/// `SimTime + SimDelta` and `SimTime − SimTime` are exact.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SimDelta(i128);

impl SimDelta {
    /// The zero-length delta.
    pub const ZERO: Self = Self(0);

    /// Smallest representable `SimDelta`.
    pub const MIN: Self = Self(i128::MIN);

    /// Largest representable `SimDelta`.
    pub const MAX: Self = Self(i128::MAX);

    /// Construct from a raw nanosecond count.
    #[inline]
    #[must_use]
    pub const fn from_nanos(nanos: i128) -> Self {
        Self(nanos)
    }

    /// Construct from microseconds. Returns `None` on overflow.
    #[inline]
    #[must_use]
    pub const fn from_micros(micros: i128) -> Option<Self> {
        match micros.checked_mul(SimTime::NANOS_PER_MICRO) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Construct from milliseconds. Returns `None` on overflow.
    #[inline]
    #[must_use]
    pub const fn from_millis(millis: i128) -> Option<Self> {
        match millis.checked_mul(SimTime::NANOS_PER_MILLI) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Construct from seconds. Returns `None` on overflow.
    #[inline]
    #[must_use]
    pub const fn from_secs(secs: i128) -> Option<Self> {
        match secs.checked_mul(SimTime::NANOS_PER_SEC) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Construct from `secs` seconds plus `nanos` nanoseconds.
    ///
    /// Same mathematical interpretation as
    /// [`SimTime::from_secs_nanos`].
    #[inline]
    #[must_use]
    pub const fn from_secs_nanos(secs: i128, nanos: u32) -> Option<Self> {
        if nanos >= SimTime::NANOS_PER_SEC as u32 {
            return None;
        }
        match secs.checked_mul(SimTime::NANOS_PER_SEC) {
            Some(base) => match base.checked_add(nanos as i128) {
                Some(n) => Some(Self(n)),
                None => None,
            },
            None => None,
        }
    }

    /// Raw nanosecond count.
    #[inline]
    #[must_use]
    pub const fn as_nanos(self) -> i128 {
        self.0
    }

    #[inline]
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    #[inline]
    #[must_use]
    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }

    #[inline]
    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }
}

impl fmt::Display for SimDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.0;
        let sign = if n < 0 { "-" } else { "" };
        let abs = n.unsigned_abs();
        let secs = abs / (SimTime::NANOS_PER_SEC as u128);
        let nanos = abs % (SimTime::NANOS_PER_SEC as u128);
        write!(f, "{}{}.{:09}s", sign, secs, nanos)
    }
}

impl fmt::Debug for SimDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SimDelta({})", self)
    }
}

impl Default for SimDelta {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

// ============================================================================
// ClockError
// ============================================================================

/// Errors produced by [`Clock`] operations.
///
/// Deliberately small. Each variant represents an invariant violation
/// the caller is expected to handle explicitly, not a panic.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ClockError {
    /// The clock was asked to advance by a negative delta. Forward
    /// advancement must be non-negative; rewinding is a separate
    /// timeline operation handled outside `Clock` (see `rewind.rs`).
    NegativeAdvance,

    /// The requested advancement would push the current time beyond
    /// the representable `SimTime` range. The clock is left unchanged.
    TimeOverflow,
}

impl fmt::Display for ClockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeAdvance => {
                f.write_str("clock cannot advance by a negative delta")
            }
            Self::TimeOverflow => {
                f.write_str("clock advancement would overflow the representable time range")
            }
        }
    }
}

impl std::error::Error for ClockError {}

// ============================================================================
// Clock
// ============================================================================

/// A deterministic logical clock for a single HELQIX execution timeline.
///
/// The clock owns exactly two pieces of information:
///
/// * `origin`  — the temporal position the timeline was created at,
/// * `current` — the current temporal position.
///
/// Both are [`SimTime`] values. The clock never reads wall-clock time,
/// never allocates, never locks, and never touches any global state.
///
/// # Invariants
///
/// 1. `current >= origin` at all times.
/// 2. `advance_by(d)` for `d >= 0` sets `current := current + d`, or
///    fails with [`ClockError::TimeOverflow`] and leaves `current`
///    unchanged.
/// 3. `advance_by(d)` for `d < 0` fails with
///    [`ClockError::NegativeAdvance`] and leaves `current` unchanged.
/// 4. `reset()` sets `current := origin`.
///
/// These are the only state transitions. No other API mutates the
/// clock.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Clock {
    origin: SimTime,
    current: SimTime,
}

impl Clock {
    /// Create a new clock whose origin and current position are both
    /// [`SimTime::ZERO`].
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            origin: SimTime::ZERO,
            current: SimTime::ZERO,
        }
    }

    /// Create a new clock whose origin and current position are both
    /// `origin`.
    ///
    /// This is the constructor used by replay, rewind, and branch
    /// timelines: they re-establish a logical position without
    /// mutating any prior clock.
    #[inline]
    #[must_use]
    pub const fn starting_at(origin: SimTime) -> Self {
        Self { origin, current: origin }
    }

    /// The current logical time.
    #[inline]
    #[must_use]
    pub const fn now(&self) -> SimTime {
        self.current
    }

    /// The origin the clock was created at.
    #[inline]
    #[must_use]
    pub const fn origin(&self) -> SimTime {
        self.origin
    }

    /// Total logical time elapsed since this clock's origin.
    ///
    /// Because advancement is forward-only, this is always
    /// non-negative. In the extreme case where the origin is very
    /// negative and the current position very positive, the exact
    /// difference may exceed the `i128` range; this method then
    /// saturates at [`SimDelta::MAX`]. Callers needing the exact
    /// difference can use [`SimTime::checked_duration_since`].
    #[inline]
    #[must_use]
    pub fn elapsed(&self) -> SimDelta {
        SimDelta(self.current.0.saturating_sub(self.origin.0))
    }

    /// Advance the clock by `delta`.
    ///
    /// * `delta == 0` is accepted and is a no-op.
    /// * `delta > 0`  moves `current` forward by exactly `delta`.
    /// * `delta < 0`  is rejected with [`ClockError::NegativeAdvance`].
    ///
    /// Returns the new current time on success. On failure the clock
    /// is left unchanged.
    pub fn advance_by(&mut self, delta: SimDelta) -> Result<SimTime, ClockError> {
        if delta.is_negative() {
            return Err(ClockError::NegativeAdvance);
        }
        let next = self
            .current
            .checked_add(delta)
            .ok_or(ClockError::TimeOverflow)?;
        self.current = next;
        Ok(next)
    }

    /// Reset `current` back to `origin`.
    ///
    /// This is a temporal operation only. It does **not** touch
    /// engineering state, execution history, events, snapshots, or
    /// branches — those belong to Layers 04–11.
    #[inline]
    pub fn reset(&mut self) {
        self.current = self.origin;
    }
}

impl Default for Clock {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // SimTime construction
    // ------------------------------------------------------------------

    #[test]
    fn sim_time_zero_is_origin() {
        assert_eq!(SimTime::ZERO.as_nanos(), 0);
        assert!(SimTime::ZERO.is_zero());
    }

    #[test]
    fn sim_time_from_units_is_exact() {
        assert_eq!(SimTime::from_micros(1).unwrap().as_nanos(), 1_000);
        assert_eq!(SimTime::from_millis(1).unwrap().as_nanos(), 1_000_000);
        assert_eq!(SimTime::from_secs(1).unwrap().as_nanos(), 1_000_000_000);
        assert_eq!(SimTime::from_nanos(42).as_nanos(), 42);
    }

    #[test]
    fn sim_time_from_secs_nanos_uses_math_interpretation() {
        let t = SimTime::from_secs_nanos(-2, 500_000_000).unwrap();
        assert_eq!(t.as_nanos(), -1_500_000_000);
    }

    #[test]
    fn sim_time_rejects_invalid_nanos_component() {
        assert!(SimTime::from_secs_nanos(0, 1_000_000_000).is_none());
        assert!(SimTime::from_secs_nanos(0, u32::MAX).is_none());
    }

    #[test]
    fn sim_time_rejects_overflowing_construction() {
        assert!(SimTime::from_secs(i128::MAX).is_none());
        assert!(SimTime::from_millis(i128::MAX).is_none());
        assert!(SimTime::from_micros(i128::MAX).is_none());
    }

    #[test]
    fn sim_time_ordering_is_numeric() {
        let a = SimTime::from_nanos(-10);
        let b = SimTime::from_nanos(0);
        let c = SimTime::from_nanos(10);
        assert!(a < b);
        assert!(b < c);
        assert!(a < c);
    }

    // ------------------------------------------------------------------
    // SimDelta
    // ------------------------------------------------------------------

    #[test]
    fn sim_delta_sign_predicates() {
        assert!(SimDelta::ZERO.is_zero());
        assert!(SimDelta::from_nanos(5).is_positive());
        assert!(SimDelta::from_nanos(-5).is_negative());
        assert!(!SimDelta::from_nanos(0).is_positive());
        assert!(!SimDelta::from_nanos(0).is_negative());
    }

    // ------------------------------------------------------------------
    // Clock construction
    // ------------------------------------------------------------------

    #[test]
    fn new_clock_starts_at_zero() {
        let c = Clock::new();
        assert_eq!(c.now(), SimTime::ZERO);
        assert_eq!(c.origin(), SimTime::ZERO);
        assert_eq!(c.elapsed(), SimDelta::ZERO);
    }

    #[test]
    fn starting_at_preserves_origin() {
        let t0 = SimTime::from_secs(100).unwrap();
        let c = Clock::starting_at(t0);
        assert_eq!(c.now(), t0);
        assert_eq!(c.origin(), t0);
        assert_eq!(c.elapsed(), SimDelta::ZERO);
    }

    // ------------------------------------------------------------------
    // Advancement semantics
    // ------------------------------------------------------------------

    #[test]
    fn advance_by_positive_moves_now_only() {
        let mut c = Clock::new();
        let d = SimDelta::from_millis(10).unwrap();
        let t1 = c.advance_by(d).unwrap();
        assert_eq!(t1.as_nanos(), 10_000_000);
        assert_eq!(c.now(), t1);
        assert_eq!(c.origin(), SimTime::ZERO);
        assert_eq!(c.elapsed(), d);
    }

    #[test]
    fn advance_by_zero_is_noop() {
        let mut c = Clock::starting_at(SimTime::from_secs(5).unwrap());
        let before = c.now();
        let after = c.advance_by(SimDelta::ZERO).unwrap();
        assert_eq!(before, after);
        assert_eq!(c.now(), before);
        assert_eq!(c.elapsed(), SimDelta::ZERO);
    }

    #[test]
    fn advance_by_negative_is_rejected_and_leaves_state_unchanged() {
        let mut c = Clock::new();
        c.advance_by(SimDelta::from_secs(1).unwrap()).unwrap();
        let before = c.now();
        let err = c.advance_by(SimDelta::from_nanos(-1)).unwrap_err();
        assert_eq!(err, ClockError::NegativeAdvance);
        assert_eq!(c.now(), before);
    }

    #[test]
    fn advance_by_overflow_is_rejected_and_leaves_state_unchanged() {
        let mut c = Clock::starting_at(SimTime::MAX);
        let err = c.advance_by(SimDelta::from_nanos(1)).unwrap_err();
        assert_eq!(err, ClockError::TimeOverflow);
        assert_eq!(c.now(), SimTime::MAX);
    }

    #[test]
    fn repeated_advancement_is_additive() {
        let mut c = Clock::new();
        c.advance_by(SimDelta::from_millis(1).unwrap()).unwrap();
        c.advance_by(SimDelta::from_millis(2).unwrap()).unwrap();
        c.advance_by(SimDelta::from_millis(3).unwrap()).unwrap();
        assert_eq!(c.now().as_nanos(), 6_000_000);
        assert_eq!(c.elapsed().as_nanos(), 6_000_000);
    }

    // ------------------------------------------------------------------
    // Reset
    // ------------------------------------------------------------------

    #[test]
    fn reset_returns_to_origin() {
        let t0 = SimTime::from_secs(7).unwrap();
        let mut c = Clock::starting_at(t0);
        c.advance_by(SimDelta::from_secs(3).unwrap()).unwrap();
        assert_eq!(c.now().as_nanos(), 10_000_000_000);
        c.reset();
        assert_eq!(c.now(), t0);
        assert_eq!(c.elapsed(), SimDelta::ZERO);
    }

    // ------------------------------------------------------------------
    // Independence between clock instances
    // ------------------------------------------------------------------

    #[test]
    fn separate_clocks_are_independent() {
        let mut a = Clock::new();
        let mut b = Clock::new();
        a.advance_by(SimDelta::from_secs(1).unwrap()).unwrap();
        assert_eq!(b.now(), SimTime::ZERO);
        b.advance_by(SimDelta::from_secs(2).unwrap()).unwrap();
        assert_eq!(a.now().as_nanos(), 1_000_000_000);
        assert_eq!(b.now().as_nanos(), 2_000_000_000);
    }

    // ------------------------------------------------------------------
    // Determinism / replay-equivalence
    // ------------------------------------------------------------------

    #[test]
    fn deterministic_replay_sequence_produces_identical_state() {
        let seq = [
            SimDelta::from_millis(10).unwrap(),
            SimDelta::from_millis(10).unwrap(),
            SimDelta::from_millis(10).unwrap(),
            SimDelta::from_millis(10).unwrap(),
        ];

        let mut c1 = Clock::new();
        let mut c2 = Clock::new();
        let mut c3 = Clock::new();

        for d in &seq {
            c1.advance_by(*d).unwrap();
        }
        for d in &seq {
            c2.advance_by(*d).unwrap();
        }
        for d in &seq {
            c3.advance_by(*d).unwrap();
        }

        assert_eq!(c1.now(), c2.now());
        assert_eq!(c2.now(), c3.now());
        assert_eq!(c1.now().as_nanos(), 40_000_000);
    }

    // ------------------------------------------------------------------
    // Boundaries
    // ------------------------------------------------------------------

    #[test]
    fn advance_to_exact_max_is_allowed() {
        let start = SimTime::from_nanos(i128::MAX - 1);
        let mut c = Clock::starting_at(start);
        let t = c.advance_by(SimDelta::from_nanos(1)).unwrap();
        assert_eq!(t, SimTime::MAX);
    }

    #[test]
    fn advance_past_max_fails_without_state_change() {
        let mut c = Clock::starting_at(SimTime::MAX);
        let before = c.now();
        assert_eq!(
            c.advance_by(SimDelta::from_nanos(1)),
            Err(ClockError::TimeOverflow)
        );
        assert_eq!(c.now(), before);
    }

    // ------------------------------------------------------------------
    // Display formatting
    // ------------------------------------------------------------------

    #[test]
    fn display_is_exact_and_stable() {
        assert_eq!(SimTime::ZERO.to_string(), "0.000000000s");
        assert_eq!(SimTime::from_secs(1).unwrap().to_string(), "1.000000000s");
        assert_eq!(SimTime::from_nanos(1_500_000_000).to_string(), "1.500000000s");
        assert_eq!(SimTime::from_nanos(-1_500_000_000).to_string(), "-1.500000000s");
        assert_eq!(SimDelta::from_nanos(-1).to_string(), "-0.000000001s");
    }

    // ------------------------------------------------------------------
    // Cross-type consistency
    // ------------------------------------------------------------------

    #[test]
    fn checked_duration_since_is_exact_and_signed() {
        let a = SimTime::from_secs(2).unwrap();
        let b = SimTime::from_secs(5).unwrap();
        assert_eq!(b.checked_duration_since(a).unwrap().as_nanos(), 3_000_000_000);
        assert_eq!(a.checked_duration_since(b).unwrap().as_nanos(), -3_000_000_000);
    }

    #[test]
    fn clock_advanced_by_delta_matches_origin_plus_delta() {
        let origin = SimTime::from_secs(100).unwrap();
        let mut c = Clock::starting_at(origin);
        let d = SimDelta::from_millis(250).unwrap();
        let now = c.advance_by(d).unwrap();
        assert_eq!(now, origin.checked_add(d).unwrap());
        assert_eq!(c.elapsed(), d);
    }
}