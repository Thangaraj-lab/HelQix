//! Layer 05 of HELQIX — Execution.
//!
//! `helqix-execution` coordinates **explicit engineering actions**
//! against HELQIX's existing engineering reality, using deterministic
//! scheduling and a fixed three-stage execution pipeline.
//!
//! # What Layer 05 owns
//!
//! Layer 05 owns the *execution orchestration* of HELQIX:
//!
//! ```text
//!     Action            a domain-independent, immutable request
//!       ↓
//!     ExecutionGraph    dependency structure between action identities
//!       ↓
//!     Scheduler         deterministic ordering (scheduled_at, ActionId)
//!       ↓
//!     Executor          validate → execute → apply pipeline
//!       ↓
//!     Transition        the execution-side descriptor of the state change
//!       ↓
//!     Runtime           the top-level loop that ties the above together
//! ```
//!
//! # What Layer 05 does NOT own
//!
//! HELQIX's authoritative engineering state is **not** owned here. Nor
//! is physical simulation, nor logical time. Those belong to Layers 04,
//! 06, and 07 respectively, and Layer 05 reaches them only through the
//! caller-supplied [`ExecutionBackend`] trait and the
//! [`helqix_time::Clock`] dependency.
//!
//! The ownership boundaries are strict:
//!
//! * **Layer 04** owns authoritative engineering state.
//! * **Layer 06** owns physical simulation.
//! * **Layer 07** owns deterministic logical time.
//! * **Layer 03** owns engineering models and dependencies.
//! * **Layer 11** owns branching.
//! * **Layer 13** owns verification.
//!
//! Layer 05 does not re-export anything from those layers, and does not
//! duplicate any of their types.
//!
//! # Determinism
//!
//! Every ordering decision Layer 05 makes is deterministic by
//! construction: the scheduler uses `(scheduled_at, ActionId)`, the
//! dependency graph uses `BTreeSet` / `BTreeMap`, and the runtime
//! iterates in scheduler order. No wall clock, no randomness, no
//! threads, no interior mutability, no `unsafe`.
//!
//! # Quick start
//!
//! The public boundary is usable without any cross-layer knowledge —
//! `Action` and its identity are entirely Layer 05-local:
//!
//! ```
//! use helqix_execution::{Action, ActionError, ActionId, ActionPayload};
//!
//! #[derive(Debug, Eq, PartialEq)]
//! struct MyPayload(u32);
//!
//! impl ActionPayload for MyPayload {
//!     fn validate(&self) -> Result<(), ActionError> {
//!         if self.0 == 0 {
//!             return Err(ActionError::new("value must be non-zero"));
//!         }
//!         Ok(())
//!     }
//! }
//!
//! let id = ActionId::from_raw(1);
//! let action = Action::try_new(id, MyPayload(7)).unwrap();
//! assert_eq!(action.id(), id);
//!
//! // Validation is enforced at construction time.
//! let err = Action::try_new(id, MyPayload(0)).unwrap_err();
//! assert_eq!(err.reason(), "value must be non-zero");
//! ```
//!
//! Full end-to-end execution (scheduling, dependencies, state mutation)
//! requires a caller-supplied [`ExecutionBackend`] implementation that
//! delegates to Layers 04 and 06, plus a [`helqix_time::Clock`].
//!
//! # Module map
//!
//! * [`action`] — the immutable [`Action`] value, its [`ActionId`], the
//!   [`ActionPayload`] trait, and [`ActionError`].
//! * [`execution_graph`] — the [`ExecutionGraph`] of dependency edges
//!   between [`ActionId`]s, plus [`ExecutionGraphError`].
//! * [`scheduler`] — the deterministic [`Scheduler`] of pending
//!   [`ScheduledAction`]s, plus [`ScheduleError`].
//! * [`executor`] — the [`execute`] pipeline, the [`ExecutionBackend`]
//!   trait, and the stage-aware [`ExecutionError`] / [`ExecutionStage`].
//! * [`transition`] — the [`Transition`] descriptor binding an action,
//!   a logical time, and a canonical Layer 04 change.
//! * [`runtime`] — the top-level [`Runtime`] loop and its
//!   [`RuntimeOutcome`] / [`RuntimeSubmitError`].

#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// Modules
// ---------------------------------------------------------------------------

/// Domain-independent action contract: [`Action`], [`ActionId`],
/// [`ActionPayload`], [`ActionError`].
pub mod action;

/// Execution dependency graph: [`ExecutionGraph`],
/// [`ExecutionGraphError`].
pub mod execution_graph;

/// Deterministic scheduling: [`Scheduler`], [`ScheduledAction`],
/// [`ScheduleError`].
pub mod scheduler;

/// Execution-side transition descriptor: [`Transition`].
pub mod transition;

/// The execution pipeline: [`execute`], [`ExecutionBackend`],
/// [`ExecutionError`], [`ExecutionStage`].
pub mod executor;

/// Top-level runtime loop: [`Runtime`], [`RuntimeOutcome`],
/// [`RuntimeSubmitError`].
pub mod runtime;

// ---------------------------------------------------------------------------
// Crate-root re-exports
// ---------------------------------------------------------------------------
//
// Curated re-exports of the domain types each module exposes. No
// wildcards, no aliases, no invented names. Every symbol below is a
// real `pub` item from the module named on its `use`.

pub use crate::action::{Action, ActionError, ActionId, ActionPayload};

pub use crate::execution_graph::{ExecutionGraph, ExecutionGraphError};

pub use crate::scheduler::{ScheduleError, ScheduledAction, Scheduler};

pub use crate::transition::Transition;

pub use crate::executor::{execute, ExecutionBackend, ExecutionError, ExecutionStage};

pub use crate::runtime::{Runtime, RuntimeOutcome, RuntimeSubmitError};