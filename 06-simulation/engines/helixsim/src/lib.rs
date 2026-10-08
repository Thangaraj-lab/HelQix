//! HELIXSIM — HELQIX deterministic physical consequence engine (Layer 06).
#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Constraint evaluation against simulation measurements.
pub mod constraints;
/// Equations of motion (state → derivative).
pub mod dynamics;
/// Environmental conditions influencing simulation.
pub mod environment;
/// Physical force models.
pub mod forces;
/// Deterministic numerical integration schemes.
pub mod integration;
/// Deterministic timestep policy (no clock ownership — Layer 07 owns time).
pub mod timestep;
