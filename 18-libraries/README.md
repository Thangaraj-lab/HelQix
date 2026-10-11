# Layer 18 — Shared reality and domain contracts

The Python package at `python/helqix_drone_domain` currently contains a domain-neutral engineering-reality contract plus the first drone-domain contracts. The shared reality schema supports typed components, declared properties with origin/provenance, interfaces, connections, requirements, and model bindings:

- `helqix.engineering-reality.v1`: product/system structure and references for different domain packs. Structural validation checks stable IDs, rejects dangling component/interface/model references, enforces output-to-input direction, and requires an exact interface-domain match for a direct connection. Cross-domain converters need their own explicit contract and are not inferred. This validation does not establish physical validity.
- `helqix.engineering-branch.v1`: revision-bound, explicit component-property changes. A compatible adapter must still execute a branch before it has result data.
- `helqix.bounded-search.v1`: persisted two-to-five-point battery-cell current sweeps, requiring a real solver run for each completed candidate and checking that candidate order, objective ranking, constraint, and stored metrics agree.
- `helqix.verification-check.v1`: a numeric observation, explicit rule, verdict, source-run ID, and source/current model digests. The contract rejects a verdict inconsistent with its observation or a conclusive result whose model digests differ.

The initial drone example remains versioned separately:

- `helqix.drone-specification.v1`: quadrotor design inputs persisted with a project.
- `helqix.drone-scenario.v1`: a saved configuration variant with battery mass, capacity, voltage, payload, and wind inputs; records remain `execution_state: not_run` until a run workflow exists.

The bounds reject malformed values and unmodeled fields. Valid legacy draft specifications without a schema marker are upgraded without changing their values. Passing validation means a record is structurally usable, not physically feasible, airworthy, or performance-validated. The search and verification contracts currently support the tested PyBaMM single-cell path; supplier-backed catalogs, other engineering models, and validated product recommendations remain separate work.

## Component type library

`helqix_drone_domain.catalog` provides a versioned, domain-aware starter type catalog through `component_type_catalog(domain)`. It classifies structural types for general systems and the current vehicle, battery, drone, and industrial examples. The authenticated gateway exposes it at `GET /v1/libraries/component-types?domain=...`; Reality Studio's Add component form uses this endpoint and still permits a user-defined type if the library is unavailable.

These are **type classifications**, not manufacturer parts, engineering specifications, or component recommendations. They contain no inferred property values. `battery_cell` and the more specific `lithium_ion_cell` may bind to the currently implemented PyBaMM SPM / Chen2020 path with explicit current and cutoff-voltage inputs. That path is a single-cell discharge model and is applicable only when the physical cell matches the Chen2020 parameterization; cell chemistry and compatibility are not inferred from the type name. It provides no pack, thermal, or product-level coupling. All other catalog types assert no solver support. In particular, drone propeller and airframe types do not imply flight simulation, performance, or airworthiness support. Supplier-backed catalogs, validated domain parameters, and broader model bindings remain future work.
