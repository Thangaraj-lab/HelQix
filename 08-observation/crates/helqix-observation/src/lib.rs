//! HELQIX Layer 08 — Observation.
//!
//! Provides deterministic, read-only observations of runtime engineering state:
//! typed metrics, explicit probes, immutable observation frames, and bounded
//! telemetry collection. Observation never changes state, advances time, runs
//! physics, or decides whether a condition is a failure.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod metric;
pub mod observer;
pub mod probe;
pub mod telemetry;

pub use metric::{Metric, MetricError, MetricId, MetricKind, MetricValue, MetricUnit};
pub use observer::{ObservationFrame, ObservationSource, Observer, ObserverError};
pub use probe::{Probe, ProbeError, ProbeId, ProbeTarget};
pub use telemetry::{TelemetryBuffer, TelemetryError, TelemetryPolicy};
