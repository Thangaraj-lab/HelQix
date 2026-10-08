//! HELIXSIM — HELQIX deterministic physical consequence engine (Layer 06).
#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Environmental conditions influencing simulation.
pub mod environment;
/// Physical force models.
pub mod forces;
/// Equations of motion (state → derivative).
pub mod dynamics;
/// Deterministic numerical integration schemes.
pub mod integration;
