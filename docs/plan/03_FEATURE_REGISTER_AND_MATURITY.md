# 03. Feature Register and Maturity (nothing cut; labels are test-based)

Rule: a feature is "Working" only if a test passes AND it is demonstrable. Labels reflect the audit of commit bf318f4 (12 Oct 2026) run on Linux on 25 Oct 2026. See 04 for raw results.

Label meanings: Working (library level) = tests pass but wiring into the app is unverified. Partial = some code and tests. Stub = very thin. Broken = does not compile or tests do not run. Claimed = stated in README or spec, not verified by me. Not evidenced = no code found. Planned / Research = intentionally later.

## Core

| Feature | Label today | Evidence |
|---|---|---|
| Model runtime (components, properties, constraints, revisions, snapshots) | Working (library level) | helqix-engineering 207 pass |
| Physical quantity and unit system | Broken tests | helqix-core test target does not compile; lib builds |
| Reality graph, environment | Broken tests | helqix-reality test target does not compile |
| Authoritative state, snapshots, diffs | Working (library level) | helqix-state 67 unit pass, 1 doctest fails |
| Deterministic simulation kernel | Partial | helixsim 33 pass; bounded Euler/RK4, point-mass with drag (per README) |
| Coupled motor physics (electrical, torque, mechanical, thermal) | Not evidenced | no torque or back-EMF in code; equations exist only in the spec |
| Scenario execution (prepare, run, pause, step) | Broken tests | helqix-execution test target does not compile |
| Time, checkpoints, replay | Working (library level) | helqix-time 99 pass; end-to-end wiring UNKNOWN |
| Freeze at failure (exact state) | Partial | snapshot code exists; failure-triggered freeze not traced |
| Branching (fork, fork_at, isolation) | Working (library level) | helqix-branch 11 pass |
| Compare | Claimed | README says UI compares battery curves; not run by me |
| Observation (probes, signals, anomalies) | Partial | helqix-observation 14 pass |
| Failure detection and propagation | Stub | helqix-failure 2 tests, 318 lines |
| Events and causality | Stub | helqix-events 2, helqix-causal 1 tests |
| Search / optimization | Partial | helqix-search 21 pass, 1 fail; bounded 2 to 5 point sweep, not general optimization |
| Verification (requirements, validators) | Stub | helqix-verify 1 test |
| Evidence (execution record, provenance) | Broken | helqix-evidence does not compile; sha256 only, no signing |
| Intent extraction (deterministic) | Claimed | 19-intelligence; Python tests not run |

## Interface

| Feature | Label today |
|---|---|
| Reality Canvas, 5 visual modes | Claimed (React/Three.js exists; modes not verified) |
| Compute Fabric (CPU, GPU, quantum, external routing) | Not evidenced (platform has in-memory policy, lease, audit primitives, 12 tests pass) |
| Quantum pathway (Qiskit, qBraid, Braket) | Not evidenced; spec claims a qBraid adapter foundation, I did not search the repo for it (UNKNOWN) |
| AI agent with provider abstraction and permission ladder | Not evidenced (only deterministic intent extraction) |
| External adapters (Blender, FreeCAD, OpenFOAM, CalculiX, PyBaMM, Rapier, ROS 2, Gazebo, PX4) | Partial: PyBaMM and Blender referenced; adapters crate 4 tests; others Not evidenced |
| SDK (Rust, Python) | Broken (Rust lib does not compile); Python tests not run |
| API | Partial: builds, 0 tests |

## Frontier (later, not claimed)

Build Spec (Requirement, Parameter, Simulation, Verification, Build decision lineage): Planned. Edge (sensor rig tier first, phone tier second): Planned. Digital twin: Research until validation exists. HELQIXSIM, Tensor-Warp, generalized time-travel: Research. FMI/FMU, STEP, SysML v2 interop: Planned. Collaboration and sign-off roles: Planned. Tenancy and on-prem deployment: Planned. Metering and billing: Planned. Control and firmware (SIL/HIL): Planned. Failure knowledge base: Planned.

## Missing layers added by this review (not in the original spec)

Validation and calibration; requirements and Context of Use; uncertainty and sensitivity; numerical integrity; benchmark and regression suite; validated model library; interoperability and PLM/SPDM integration; collaboration and sign-off; deployment and tenancy; control and firmware loop; metering; failure knowledge base. See 05.

## Rule for updating this file

Change a label only after a test run you can paste. Record the date and command in 12_STATUS_BOARD.
