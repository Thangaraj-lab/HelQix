//! HELQIX Layer 04 — State.
//!
//! This crate defines the deterministic runtime state of an engineering
//! reality.
//!
//! Layer boundary:
//!
//! ```text
//! Layer 02 — Reality
//!     ↓
//! Layer 03 — Engineering
//!     ↓
//! Layer 04 — State
//!     ↓
//! Layer 05 — Execution
//! ```
//!
//! State represents the current condition of the engineering system.
//!
//! This layer intentionally does NOT contain:
//!
//! - physics execution;
//! - simulation time advancement;
//! - events;
//! - failure analysis;
//! - causality;
//! - branching;
//! - optimization;
//! - rendering;
//! - AI;
//! - quantum computation.
//!
//! The same engineering state must remain the source of truth for all
//! downstream execution and verification systems.

pub mod component_state;
pub mod physical_state;
pub mod property_state;
// NOTE: `spatial_state`, `validation`, and the remaining Layer 04 module
// files (`serialization`, `snapshot`, `state_store`, `transition`) are
// currently empty placeholders. Their public contracts (e.g. `SpatialState`,
// `StateValidator`) are planned but not yet implemented, so they are not
// declared as modules here — declaring them would break compilation of the
// whole layer. Re-add each `pub mod` + re-export as soon as its file is
// implemented.
pub mod state;
pub mod state_diff;
pub mod state_registry;
pub mod state_snapshot;

// -----------------------------------------------------------------------------
// Component State
// -----------------------------------------------------------------------------

pub use crate::component_state::{
    ComponentState,
    ComponentStateError,
};

// -----------------------------------------------------------------------------
// Physical State
// -----------------------------------------------------------------------------

pub use crate::physical_state::{
    PhysicalState,
    PhysicalStateError,
};

// -----------------------------------------------------------------------------
// Property State
// -----------------------------------------------------------------------------

pub use crate::property_state::{
    PropertyState,
    PropertyStateError,
};

// -----------------------------------------------------------------------------
// Spatial State (placeholder module — not yet implemented; see note above)
// -----------------------------------------------------------------------------

// -----------------------------------------------------------------------------
// State
// -----------------------------------------------------------------------------

pub use crate::state::{
    State,
    StateError,
};

// -----------------------------------------------------------------------------
// State Diff
// -----------------------------------------------------------------------------

pub use crate::state_diff::{
    ComponentStateChange,
    StateDiff,
};

// -----------------------------------------------------------------------------
// State Registry
// -----------------------------------------------------------------------------

pub use crate::state_registry::{
    StateRegistry,
    StateRegistryError,
};

// -----------------------------------------------------------------------------
// State Snapshot
// -----------------------------------------------------------------------------

pub use crate::state_snapshot::{
    StateSnapshot,
    StateSnapshotError,
};

// -----------------------------------------------------------------------------
// Validation (placeholder module — not yet implemented; see note above)
// -----------------------------------------------------------------------------