# HELQIX product definition

> **SUPERSEDED (2026-10-25).** The locked definition is now [docs/LOCKED_PLAN.md](LOCKED_PLAN.md) and the full document set in [docs/plan/](plan/00_README_INDEX.md). The 2026-10-25 audit found claims in this file that the code contradicts (see [docs/plan/05_GAPS_MISTAKES_AND_FIXES.md](plan/05_GAPS_MISTAKES_AND_FIXES.md), item B12). Kept unchanged below for history.

**Status:** product boundary and implementation target, 10 October 2026  
**Product:** HELQIX — Engineering Reality

## Product in one sentence

HELQIX is a layered engineering workspace for creating a traceable, executable digital representation of an engineered product or system, connecting its models and tools, exploring and comparing possible states, and preserving the evidence behind engineering decisions.

## What the product is

HELQIX is the **engineering environment and orchestration platform**. It connects product intent, requirements, architecture, components, models, state, execution, simulation, observations, failures, causal analysis, alternatives, verification, and evidence in one project. Its defining outcome is a navigable engineering reality: a user can see what a product is made of, which assumptions and models describe it, what was actually executed, how the system behaved, how alternatives differ, and which evidence supports a conclusion.

The product is domain-extensible. A domain supplies typed component catalogs, interfaces, constraints, engineering models, units, tool adapters, and validation rules. HELQIX's common layers provide the shared reality graph, state and execution semantics, simulation orchestration, time and observation, causal and failure analysis, branching and search, verification, visualization, APIs, and project evidence.

## What the product is not

- HELQIX is not a single-product simulator. The current demonstrator uses an electric vehicle and its battery as one domain pack for the domain-extensible engineering workflow.
- HELQIX is not a chatbot that invents component choices or performance figures. AI can extract, explain, and propose; domain rules and engineering tools must support technical conclusions, with assumptions and provenance visible.
- A 3D visualization is not automatically manufacturing CAD, a validated physics model, or an executed simulation.
- Connecting a third-party tool does not make its output valid for every design. Inputs, tool/version, configuration, output, and applicability must be recorded.
- A scenario or branch is not an engineering result until a compatible model has actually executed and returned a validated result.

## Primary user outcome

An engineer starts from a product need, turns it into a reviewed system definition, builds or imports the product's component-and-model architecture, connects suitable engineering tools, executes the analyses that are supported, inspects observed behavior and failures, compares controlled alternatives, and creates a reproducible evidence package for the decision. HELQIX should keep the boundary between **declared inputs**, **derived values**, **tool outputs**, **engineering interpretations**, and **verified claims** explicit.

## Canonical project workflow

1. **Define intent and requirements.** Capture the user brief, source text, units, constraints, target outcomes, and unresolved questions. Extracted candidates remain reviewable and linked to their source; unsupported fields stay unresolved.
2. **Establish the engineering reality.** Create a typed product/system model: components, properties, interfaces, behavior, environment, dependencies, and constraints. Validate structure and identify missing inputs without silently filling them.
3. **Analyze and resolve the architecture.** Use domain catalogs and engineering models to evaluate candidate components and compatibility. Recommendations must state the criteria, assumptions, uncertainty, and evidence; a human approves consequential selections.
4. **Create and connect representations.** Link visual scenes, CAD, meshes, equations, and external solver models to the same versioned engineering entities. Each representation declares what it can and cannot establish.
5. **Execute analyses.** Select a compatible adapter and explicit inputs; validate units and preconditions; run in a bounded environment; store immutable inputs, tool/model identity, logs, outputs, status, and provenance.
6. **Inspect behavior over time.** View state, telemetry, events, anomalies, and causal relationships against the executed run and its time axis.
7. **Explore alternatives.** Fork the same approved baseline, change specified inputs, and run compatible analyses. Compare only like-for-like outputs. Optimization may recommend candidates only when its objective, constraints, search space, and evaluation method are recorded.
8. **Verify and report.** Evaluate explicit requirements against applicable evidence, label inconclusive or failed checks honestly, and export traceable, reproducible project evidence.

## Shared platform layer map

| Layer | Responsibility in HELQIX |
|---|---|
| 01 Foundation | Identifiers, units, quantities, properties, components, behavior, interfaces, constraints, and shared errors. |
| 02 Reality | The product/system reality graph, component registry, topology, environment, and structural validation. |
| 03 Engineering | Engineering models, equations, parameter sets, dependencies, constraints, and model compatibility. |
| 04 State | Typed state, snapshots, diffs, persistence, and state validation. |
| 05 Execution | Actions, scheduling, runtime, execution graph, and controlled transitions. |
| 06 Simulation | Model execution and simulation orchestration behind explicit solver contracts. |
| 07 Time | Simulation clocks, timelines, snapshots, replay, and rewind semantics. |
| 08 Observation | Metrics, probes, telemetry, and observers over executions. |
| 09 Failure | Fault definitions, detection, propagation, and failure-state handling. |
| 10 Events and causality | Event delivery, triggers, propagation, causal graphs, and root-cause analysis. |
| 11 Branching | Versioned forks of a reality/state for controlled what-if experiments. |
| 12 Search | Bounded exploration and optimization over declared variables, constraints, objectives, and evaluators. |
| 13 Verification | Requirements, validators, constraint checks, results, and reproducibility evidence. |
| 14 Visual runtime | Interactive visual representation of the engineering reality and its live or recorded state. |
| 15 SDK | Stable developer interfaces for domain packs, models, components, actions, and realities. |
| 16 Adapters | Typed boundaries that translate HELQIX contracts to external CAD, simulation, data, and engineering tools. |
| 17 Integrations | Concrete tool/provider connections, execution, health, provenance, and operational limits. |
| 18 Libraries | Reusable domain schemas, catalogs, model assets, and engineering knowledge packages. |
| 19 Intelligence | Intent interpretation, assistance, explanation, and recommendation with review and provenance. |
| 20 Platform | Compute scheduling, storage, permissions, audit, isolation, and deployment capabilities. |
| 21 Application | The user-facing HELQIX workspace and cross-layer project workflows. |
| 22 API | Authenticated, versioned service contracts between app, platform, and integrations. |
| 23 Validation | Unit, integration, conformance, reproducibility, and product-level quality checks. |

These layers are one product architecture, not 23 separate end-user products. A workflow is complete only when these contracts connect end to end for a declared domain and the actual evidence supports the displayed conclusion.

## First product vertical: user-defined engineered systems

The first demonstrator is a **battery-electric vehicle study** because the current workspace has explicit vehicle, battery-pack, and drive-cycle inputs and a model-backed battery workflow. It is a domain pack and example, not HELQIX's product boundary. The vehicle adapter currently computes longitudinal road load for a prescribed speed trace and couples that load to a Chen2020 single-cell model scaled across identical cell counts. Its 3D trajectory follows the recorded distance samples. It does not predict free-running acceleration, braking or regeneration, cell mismatch, thermal gradients, BMS behavior, crashworthiness, or a validated vehicle design. HELQIX must show those limits with each result and must not label this scoped analysis as full-vehicle simulation. Other products, including drones, can be added as separate domain packs when their compatible models and tool adapters are implemented.

The same platform contracts should later support other engineered products by adding domain data and adapters, not by rewriting the common reality, state, execution, time, branch, verification, or evidence layers.

## Product-wide acceptance criteria

- Every project has a versioned, typed engineering reality with stable component identity and explicit units.
- Every extracted, entered, calculated, imported, or simulated value carries an origin and can be distinguished from the others.
- Every execution is reproducible from saved model/tool versions and immutable run inputs, or is marked non-reproducible with the reason.
- No result is labeled simulated, verified, or recommended unless the required compatible execution/check completed successfully.
- A branch records its baseline and delta; comparisons disclose incompatible or missing data instead of inventing a winner.
- Visual representations identify their source model and fidelity; previews cannot masquerade as validated geometry or physics.
- Project evidence can be traced from requirements through the selected reality, models, execution, results, and verification outcomes.
- A domain pack can add a product type without changing the meaning of shared platform contracts.
- Security, project isolation, failure recovery, and validation are part of the product, not optional finishing work.

## Current repository boundary

The repository contains Rust source across Layers 01–16 and 22, a React/Three.js Reality Studio, a local Python API and tool gateway, a shared engineering-reality contract, a limited EV/battery domain workflow, a starter structural component-type library, and a separate drone example contract. This remains an early local prototype, not a complete HELQIX release. The EV battery adapter is the only product-level workflow with a supported physical model; Layer 06's generic point-mass kernel is a separate primitive and is not yet wired into the EV adapter or UI. Layer 14 now has working material-safe health/selection/thermal visualization and camera behavior, but still needs broader model-to-view and end-to-end workflow coverage. Layer 17 now persists failed process attempts and malformed reports with bounded logs and exposes them separately from usable results; installed tool coverage and operational deployment remain limited. Layer 19 has opt-in proposal-only brief review through a configured provider; validated domain knowledge, supplier-backed catalogs, and broad compatible tool chains are not complete. Layers 20, 21, and 23 still have important workflow, integration, or validation gaps. Layer 20's local SQLite-backed append-only project activity adapter does not provide distributed audit storage, durable job coordination, identity verification, or worker process isolation. Existing integrations must be represented at their actual fidelity and applicability.

The current tested model-backed slice adds a scoped battery-cell example: a saved `battery_cell` or `lithium_ion_cell` may bind to PyBaMM SPM / Chen2020 only when the physical cell matches that parameterization, then run with explicit cell current and cutoff properties, retain a bounded solver time series and project-model digest, and evaluate accepted explicit thresholds against that exact model revision. A saved branch can change a supported cell property, rerun the solver, and be compared to a baseline only when the binding and model revision match; stale branches are rejected, and the evidence export includes the run records. The application plots their recorded voltage curves. Layer 12 also supports a bounded two-to-five-point cell-current sweep through those real branch runs, with an optional minimum-duration constraint, persisted candidate/run provenance, and a best-evaluated candidate ranked by measured discharged capacity. EV drive-cycle runs retain the exact validated vehicle, pack, and drive-cycle configuration alongside its model digest. Reality Studio can compare completed EV runs only when their drive-cycle points, solver parameterization, tool, and provenance match; its plots and metric deltas use recorded outputs. Vehicle speed remains prescribed, so this comparison does not model free-running acceleration or select a vehicle-performance winner. The Build screen now edits component interfaces and direct, matching-domain connections in the shared reality contract, and shows those declared links as visual proxy wires. These are topology records, not electrical/fluid solvers or CAD geometry. This is not a global optimizer, battery-pack or product-system simulation, and does not close general execution, branching, search, or verification support for other domains.

The initial **ProofLoop** slice adds a deterministic, evidence-aware next-step policy to the project evidence screen. It checks the saved engineering-reality digest, scoped single-cell solver runs, accepted numeric requirements, and revision-matched verification records; it then identifies the next required proof obligation or, after supported checks pass, proposes a supervised cell bench-validation plan. Users can record that proposed step in the project evidence bundle. Recommendations disclose their triggering record IDs and scope limitations. This is a transparent rules policy, not autonomous engineering judgment; it does not ingest live hardware data, create build instructions, or verify a battery pack or vehicle.

## Implementation rule

Build in vertical slices through the shared architecture: define a stable contract, connect it to the SDK/API, implement one useful domain pack and real adapter, make it inspectable in the application, record provenance, and validate its behavior. Do not mark a layer complete because files exist or a mock screen renders. Product completion means the acceptance criteria above are demonstrated by runnable, cross-layer workflows.
