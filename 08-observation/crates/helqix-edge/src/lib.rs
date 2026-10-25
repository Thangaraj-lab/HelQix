//! Edge observation (Phase 7A): real measurements enter HELQIX as **Observed** values.
//!
//! - [`ingest`]: the `helqix.measurement.v1` line protocol, typed rejections, quality flags.
//! - [`recording`]: a content-addressed recording of raw lines that re-feeds to the same result.
//! - [`reconcile`]: Observed vs Simulated residual metrics and anomaly events.
//! - [`store`]: Observed values are kept apart; the only write path into Authoritative values
//!   refuses anything that came from observation.
//!
//! Synthetic data can exercise this code but cannot pass Gate 7A; that needs real rig data.
pub mod ingest;
pub mod reconcile;
pub mod recording;
pub mod store;
