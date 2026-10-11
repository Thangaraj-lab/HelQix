//! Layer 07 of HELQIX — deterministic temporal control.
//!
//! `helqix-time` provides the temporal infrastructure that all other
//! HELQIX layers depend on to make an engineering execution
//! deterministic, observable, replayable, rewindable, branchable, and
//! verifiable.
//!
//! # What Layer 07 owns
//!
//! Layer 07 owns **time**, and only time. Its conceptual structure is:
//!
//! ```text
//!     CLOCK          deterministic logical time        (clock.rs)
//!       ↓
//!     TIMELINE       ordered execution history         (timeline.rs)
//!       ↓
//!     SNAPSHOT       recoverable temporal point        (snapshot.rs)
//!       ↓
//!     REWIND         navigation to an existing point   (rewind.rs)
//!     REPLAY         deterministic reconstruction      (replay.rs)
//! ```
//!
//! # What Layer 07 does NOT own
//!
//! Time is an infrastructure primitive. It must never become the
//! owner of engineering truth. The ownership boundaries are strict:
//!
//! * **Layer 04** owns authoritative engineering state.
//! * **Layer 05** owns actions and execution.
//! * **Layer 06** owns physical simulation.
//! * **Layer 11** owns branching.
//! * **Layer 13** owns verification.
//!
//! This crate re-exports **nothing** from those layers, and consumes
//! **nothing** from them. It is a leaf dependency: everything that
//! needs time depends on `helqix-time`; `helqix-time` depends on
//! neither state, execution, simulation, branching, nor verification.
//!
//! # Logical time is not wall-clock time
//!
//! All temporal values in this crate are **logical**. They advance
//! only because a HELQIX execution explicitly advances them. No
//! `SystemTime`, `Instant`, `sleep`, or thread-scheduling input is ever
//! consulted for simulation truth. The same initial state plus the
//! same sequence of advances produces the same temporal position, in
//! every run, process, and platform.
//!
//! # Quick start
//!
//! ```
//! use helqix_time::{Clock, SimDelta, SimTime};
//!
//! let mut clock = Clock::new();
//! assert_eq!(clock.now(), SimTime::ZERO);
//!
//! clock.advance_by(SimDelta::from_millis(10).unwrap()).unwrap();
//! assert_eq!(clock.now(), SimTime::from_millis(10).unwrap());
//! assert_eq!(clock.elapsed(), SimDelta::from_millis(10).unwrap());
//! ```
//!
//! # Module map
//!
//! * [`clock`] — deterministic logical time ([`Clock`], [`SimTime`],
//!   [`SimDelta`], [`ClockError`]).
//! * [`timeline`] — ordered temporal history ([`Timeline`],
//!   [`TimelineEntry`], [`TimelineError`]).
//! * [`snapshot`] — recoverable temporal point ([`TemporalSnapshot`]).
//! * [`rewind`] — navigation to an existing historical point
//!   ([`RewindRequest`], [`RewindTarget`], [`RewindDirection`],
//!   [`RewindError`]).
//! * [`replay`] — deterministic reconstruction of a recorded execution
//!   ([`ReplayPlan`], [`ReplayWindow`], [`ReplayOutcome`],
//!   [`ReplayStatus`], [`ReplayStep`], [`ReplayError`], and the
//!   [`replay`](fn@replay) driver).

#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// Modules
// ---------------------------------------------------------------------------

/// Deterministic logical time: [`Clock`], [`SimTime`], [`SimDelta`],
/// [`ClockError`].
pub mod clock;

/// Ordered temporal history: [`Timeline`], [`TimelineEntry`],
/// [`TimelineError`].
pub mod timeline;

/// Recoverable temporal point: [`TemporalSnapshot`].
pub mod snapshot;

/// Navigation to an existing historical point: [`RewindRequest`],
/// [`RewindTarget`], [`RewindDirection`], [`RewindError`].
pub mod rewind;

/// Deterministic reconstruction of a recorded execution:
/// [`ReplayPlan`], [`ReplayWindow`], [`ReplayOutcome`],
/// [`ReplayStatus`], [`ReplayStep`], [`ReplayError`], and the
/// [`replay`](fn@replay) driver.
pub mod replay;

// ---------------------------------------------------------------------------
// Crate-root re-exports
// ---------------------------------------------------------------------------
//
// Curated re-exports of the domain types each module exposes. No
// wildcards, no aliases, no invented names. Every symbol below is a
// real `pub` item from the module named on its `use`.

pub use crate::clock::{Clock, ClockError, SimDelta, SimTime};

pub use crate::timeline::{Timeline, TimelineEntry, TimelineError};

pub use crate::snapshot::TemporalSnapshot;

pub use crate::rewind::{RewindDirection, RewindError, RewindRequest, RewindTarget};

pub use crate::replay::{
    replay, ReplayEnd, ReplayError, ReplayOutcome, ReplayPlan, ReplayStart, ReplayStatus,
    ReplayStep, ReplayWindow,
};