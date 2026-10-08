//! HELQIX Layer 10 — deterministic event infrastructure.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Canonical event representation: what happened.
pub mod event;
/// Deterministic condition triggers: when an event should be produced.
pub mod trigger;
/// Deterministic event publication and dispatch.
pub mod event_bus;
/// Deterministic event forwarding/propagation (delivery, not causality).
pub mod propagation;
