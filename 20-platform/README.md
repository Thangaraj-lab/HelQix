# Layer 20 — Platform

Layer 20 contains policy and lifecycle primitives shared by the API and worker
services: workspace/project access decisions, bounded asynchronous job
admission with expiring fencing-token leases, and append-only audit events.

The crate is deliberately a policy core. Its queue and audit log are in-memory
implementations for deterministic local use and conformance tests; they do not
provide durable or distributed guarantees.

The local Python gateway now supplies a SQLite-backed, append-only project
activity adapter. It records resource create/update/delete actions and
project-scoped tool-run records using a minimal safe field set, enforces
immutability with database triggers, and exposes owner-scoped read access to
Reality Studio. This is a local single-database adapter, not production
distributed audit storage, job coordination, identity verification, or worker
process isolation. Production deployment still needs those services and their
operational controls.
