//! HELQIX Layer 20 — platform control plane contracts.
//!
//! This crate owns policy and lifecycle rules shared by APIs and workers:
//! tenant-scoped authorization, quota-aware job admission, worker capability
//! contracts and immutable audit events. Durable storage, distributed
//! coordination, authentication and process isolation are supplied by
//! production adapters; this crate does not pretend its in-memory primitives
//! provide those guarantees.

#![forbid(unsafe_code)]

pub mod access;
pub mod audit;
pub mod jobs;
pub mod limits;
pub mod workers;

pub use access::{AccessDecision, AccessPolicy, Action, Principal, ResourceScope};
pub use audit::{AuditEvent, AuditLog, AuditOutcome, AuditSubject};
pub use jobs::{Job, JobId, JobQueue, JobState, Lease, QueueError};
pub use limits::{AdmissionLimits, AdmissionUsage, LimitError};
pub use workers::{WorkerCapabilities, WorkerError, WorkerRegistration, WorkerState};
