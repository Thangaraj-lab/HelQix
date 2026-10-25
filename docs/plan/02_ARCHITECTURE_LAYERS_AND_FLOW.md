# 02. Architecture: 10 Locked Layers, Repo Mapping, Flow (LOCKED)

## The 10 logical layers

1. Intent, Requirements, Context of Use. Machine-checkable criteria, e.g. "Temperature below 90 C for 30 min at 80% load". Context of Use and risk level decide how much credibility is required.
2. Model and Validated Library. Components, connections, properties, constraints, SI quantity system; library of models with parameters and credibility metadata. The validated library is a real moat, not the solver.
3. Scenario and Physics. Scenario = Model + Environment + Inputs + Configuration. Physics defines equations; Environment is a scenario input.
4. Execution Fabric. Solvers, adapters, compute routing (CPU, GPU, quantum simulator or QPU, external), metering and cost. Adapter contract: Capabilities, Validate, Prepare, Execute, Status, Cancel, Collect, Normalize, Provenance.
5. Authoritative State and History. Typed state, snapshots, diffs, event log. "Reality" is the composite of model, state and history, not a separate layer.
6. Time and Lineage. Freeze = checkpoint, Replay = same inputs, Branch = changed inputs, plus Compare and Optimize. Optimization reuses the branch engine (automated branching).
7. Observation and Events. Probes, signals, anomalies, failure as an event type, causality as analysis over the event graph, edge sources as observation adapters.
8. Trust. Numerical integrity, code and solution verification, calibration, validation, uncertainty and sensitivity, benchmark and regression suite, signed evidence, sign-off.
9. Agent. One permissioned AI layer (cognition, build intelligence, explanation). Provider-agnostic. Never writes authoritative state silently.
10. Presentation and Integration. Reality Canvas, workspaces, SDK and API, interoperability (FMI/FMU, STEP, SysML v2, PLM and SPDM connectors).

Cross-cutting: security, tenancy, on-prem or air-gapped deployment, auditability.

## Mapping to the repository (do not rename folders)

| Logical layer | Repo folders |
|---|---|
| 1 Intent, Requirements, CoU | 19-intelligence, 13-verification |
| 2 Model and Validated Library | 03-engineering, 18-libraries |
| 3 Scenario and Physics | 02-reality, 06-simulation |
| 4 Execution Fabric | 05-execution, 16-adapters, 17-integrations, 20-platform |
| 5 State and History | 04-state |
| 6 Time and Lineage | 07-time, 11-branching, 12-search |
| 7 Observation and Events | 08-observation, 09-failure, 10-events-causality |
| 8 Trust | 13-verification, 23-validation |
| 9 Agent | 19-intelligence |
| 10 Presentation and Integration | 14-visual-runtime, 15-sdk, 21-app, 22-api |

Reason for mapping instead of renaming: renaming 23 folders has high risk and no product benefit. Names merge in documentation and marketing: three tiers to customers (Core, Interface, Frontier), not 12 modules.

## Locked flow (v3)

Intent
-> Requirements + Context of Use + Risk
-> Model
-> Code and Solution Verification
-> Simulate (deterministic, level declared)
-> Observe -> Failure -> Causality
-> Freeze -> Replay -> Branch -> Compare
-> Calibrate (training data)
-> Validate (held-out data)
-> Optimize (carrying validation status)
-> Signed Evidence
-> Report path, or Build Spec path -> Real product -> Edge observation -> new data -> back to Model

Back-edges (must be shown): Failure to Model; Verification fail to Requirements or Model; Validation gap to Calibrate; new measurements to Model.

## Three states and the truth hierarchy

- Authoritative state (computational truth) drives everything downstream.
- Derived state (3D scene, charts, metrics) is display only.
- Observed state (sensors, edge) enters evidence and engineering interpretation, never overwrites authoritative state silently.

Never: 3D animation to truth. Never: AI response to truth.

## Reality Canvas (UI heart)

Five visual modes over the same reality:
1. Structure: components, topology, interfaces.
2. Physics: force, torque, temperature, current, flow, energy.
3. State: current computational state, constraints, fault state.
4. Causality: clickable causal chain; each node shows equation, parameter, previous and current value, observation, simulation evidence, affected components, related branches.
5. Evidence: every value carries Origin and Status badges.

Time bar: state replay, not video. Play, pause, step, rewind, jump to event, freeze, replay. Counterfactual view shows Baseline vs Branch inside the same canvas.

Rendering: Three.js. WebGPURenderer falls back to WebGL 2 automatically but is still described as experimental by the Three.js manual; WebGLRenderer remains the stable choice. The renderer must never touch engineering truth (see 11).

## Workstation UI

Top bar: HELQIX | Project | AI | Search | Save | Run Status. Navigation: Home, Build, Simulate, Run, Understand, Improve, Compute, Verify, Evidence. Center: Engineering Reality. Right: inspector (Properties, State, Parameters, Constraints, Physics, Evidence). Bottom: Reality Revision, State, Determinism level, Logical Time, Run, Freeze.

## Data and lineage

Result is traceable to: Project, Model Revision, Scenario, Environment, Execution, State, Branch, Result, Evidence. Persistence: relational database plus object storage plus execution metadata. At scale: delta checkpoints, copy-on-write branching, content-addressed storage, branch pruning policy (scheduled later, see 06).

## Evidence package (target content)

Project, model revision, scenario, parameters, environment, execution identity, determinism level, logical time, observations, failures, branch lineage, optimization, compute backend, provider metadata, verification results, validation results with uncertainty, Context of Use and risk, sign-off, hashes and signatures.
