//! Adapter contracts, lifecycle state, and capability-based routing for HELQIX.

pub mod adapter;
pub mod capability;
pub mod lifecycle;
pub mod registry;

pub use adapter::{
    Adapter, AdapterContext, AdapterError, AdapterRequest, AdapterResponse,
    CancellationToken, HealthReport, HealthStatus,
};
pub use capability::{AdapterCapability, AdapterDescriptor};
pub use lifecycle::{AdapterLifecycle, AdapterState, LifecycleEvent};
pub use registry::AdapterRegistry;
