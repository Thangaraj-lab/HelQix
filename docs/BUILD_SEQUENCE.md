# HELQIX Build Sequence

Status date: 2026-10-12  
Scope: continue the existing repository; do not replace established foundations or build new domain-specific products.

## Product invariant

HELQIX is an Engineering Executable-Reality Platform. The saved Engineering Reality is the model contract; Authoritative State is the execution truth; the 3D runtime is a projection of that state. External solvers and providers run behind adapters. Results must normalize into HELQIX records with revision, input, model, scenario, and provenance links.

The target vertical slice is:

```text
Engineering Reality
  -> Executable Scenario
  -> Execution Controller
  -> Authoritative State
  -> State Timeline / Observation
  -> State-bound 3D Runtime
  -> Freeze / Replay / Branch
  -> Verification + Evidence + Provenance
```

## Build gates

A layer is not complete because a file exists. It is complete only when its behavior is wired into the Studio, invalid inputs fail safely, automated checks pass, and evidence shows the intended state/result is produced.

### Gate 0 — Restore a trustworthy build baseline

**Do first.**
- Fix type/runtime contract mismatches in state-to-visual binding.
- Run Reality Studio typecheck and production build; fix failures before adding features.
- Add or update focused tests for state mapping, scenario validation, controller stepping, and timeline output.
- Record actual check results; never label a build as passing without running it.

**Exit criteria:** typecheck and build pass; one scenario-controller test proves that a state transition and logical time change are represented in the resulting state/events.

### Gate 1 — Wire the existing scenario runtime into Studio

- Add the existing Scenario Execution Board to the existing Run workspace without removing the saved external model-run list.
- Keep the local scenario runtime visibly distinct from external solver/hardware execution and from persisted backend run records.
- Make the state-binding function the only path from Authoritative State to runtime entity status/signals/fault annotations.
- Pass the controller's current state to RuntimeCanvas so the 3D projection changes when a scenario step changes state.
- Keep selection, scene, and inspector aligned to the same component IDs.

**Exit criteria:** in a saved project, prepare an inspection scenario, step/run it, observe logical time/state revision/event changes, and see corresponding supported state indicators in the 3D viewport. Reset/checkpoint restore must not mutate the saved Engineering Reality. No claim of physical failure or solver result is made from a simulated/injected state alone.

### Gate 2 — Complete deterministic scenario semantics

- Treat scenario actions as a validated executable contract.
- Define and test ordering, same-time actions, maximum action count, cancellation/failed action behavior, and state-revision rules.
- Add condition definitions/evaluation and explicit true/false scheduling semantics; do not evaluate arbitrary user code.
- Validate component/property/environment/signal references before execution.
- Define whether a checkpoint is an event, a snapshot, or both, consistently.

**Exit criteria:** deterministic fixtures reproduce the same state/events for the same reality, scenario, initial state, and execution settings; invalid scenarios fail before partial execution.

### Gate 3 — Unify observation, timeline, freeze, replay, and branching

- Use one revision-aware timeline for state events, telemetry, model output, runtime events, and external-tool events.
- Capture a complete freeze manifest: reality/model revision, initial/current state, logical time, scenario/actions, environment, execution settings, solver/model versions, input digests, and external references.
- Replay from a frozen state with explicit compatibility checks; never silently replay against changed geometry/model versions.
- Branch from an exact snapshot with parent lineage and explicit differences.
- Show event-to-state-to-snapshot links in the UI.

**Exit criteria:** select an event, freeze its exact state, restore it, and create two branches. Each branch retains its parent, changed inputs, resulting state, and reproducibility limitations.

### Gate 4 — Failure investigation and evidence chain

- Join failures, events, telemetry, state revisions, causal edges, and solver output using stable IDs.
- Separate observed facts from hypotheses; every root-cause candidate must show supporting evidence and confidence/source.
- Link requirement -> constraint -> test -> scenario -> execution -> observation -> evidence -> verification.
- Include provenance manifests and content hashes where meaningful.

**Exit criteria:** a user can trace an observed failure to exact state and source events, replay it, and export a verification/evidence bundle. Fault injection alone is labeled as injected state, not proof of a physical phenomenon.

### Gate 5 — Simulation and external model lifecycle

- Normalize the lifecycle for prepare, validate, execute, stream, cancel, timeout, retry, fail, and collect results.
- Strengthen existing PyBaMM/OpenFOAM/PX4+Gazebo integrations; keep their scope and solver applicability explicit.
- Implement geometry import/export and stable component identity for FreeCAD/Blender/STEP/IGES through adapters.
- Normalize tool results into HELQIX telemetry/state/evidence without letting adapters silently invent model facts.

**Exit criteria:** each supported integration has a declared contract, a tested happy path, tested failure/cancellation paths, versioned inputs, normalized outputs, and provenance.

### Gate 6 — Secure production execution fabric

- Put untrusted tools and workloads behind isolated workers with resource/time limits, filesystem and network policies, secret isolation, and guaranteed process cleanup.
- Implement durable job lifecycle, queue, scheduler, retries, cancellation, worker health/capabilities, and result storage.
- Apply authorization, least privilege, audit logging, and tenant isolation before exposing shared/distributed execution.
- Preserve the current API/database foundations instead of rebuilding them.

**Exit criteria:** a job can be accepted, queued, scheduled, cancelled/timed out, failed safely, retried under policy, and linked to durable outputs and provenance. Cross-tenant access and sandbox escape tests are part of the release gate.

### Gate 7 — Search, optimization, and classical compute

- Formalize variables, bounds, constraints, objective(s), budget, baseline, and scoring.
- Reuse the same execution/result/evidence contract for every candidate.
- Add a classical CPU baseline and then bounded grid/random search, followed by more advanced strategies only when measured value warrants them.
- Report feasibility, units, objective scores, resource cost, and candidate lineage.

**Exit criteria:** repeated runs produce comparable rankings with traceable inputs, and optimized results are checked against a classical baseline and declared constraints.

### Gate 8 — Quantum adapter, qBraid first

- Integrate HELQIX -> qBraid Runtime -> backend discovery/submission/status/result -> HELQIX normalization/provenance.
- Start with a simulator and a bounded optimization workload; compare with a classical baseline before claiming benefit.
- Handle credentials, backend availability/capabilities, queueing, quota, timeout, cancellation, provider errors, and result-schema differences.
- Add direct QPU-provider adapters only for capabilities the qBraid abstraction does not expose or HELQIX demonstrably needs.

**Exit criteria:** a real authenticated job can be submitted and polled, its output retrieved and normalized, errors represented, backend/job IDs recorded, and the answer compared with a classical baseline. qBraid is a runtime/access layer, not itself a QPU manufacturer.

### Gate 9 — Engineering AI and provider abstraction

- Treat AI output as a proposal, never authoritative state.
- Validate proposed changes/scenarios against reality contracts and constraints; require approval before executing consequential changes.
- Normalize provider APIs for supported hosted and local models, including explicit timeout/error/budget controls.
- Persist prompt/model/version and accepted/rejected proposal provenance as required by the configured privacy policy.

**Exit criteria:** an AI suggestion cannot bypass validation, access control, deterministic execution, or evidence collection.

### Gate 10 — Production product integration

- Complete durable object storage, backup/restore, retention, observability, and operational recovery.
- Finish RBAC, secrets management, tenant boundaries, collaboration, review/approval, revision locking, and audit exports.
- Complete Studio workflows, accessible empty/error/loading states, responsive layout, keyboard operation, and user documentation.

**Exit criteria:** end-to-end project workflow passes in a deployed environment with migrations, restore drills, security tests, and operational monitoring.

## Immediate implementation order

The first pull request should do only the work needed for Gate 0 and the smallest coherent slice of Gate 1:

1. Fix visual-state binding's runtime-state shape/status mappings.
2. Integrate Scenario Execution Board as a separate view inside Run, preserving existing external model records.
3. Bind controller state into RuntimeCanvas and clearly label this as a local deterministic scenario run, not an external solver/hardware run and not a persisted backend execution.
4. Add focused checks where the repository's test setup permits.
5. Run typecheck/build and record the actual results.
6. Merge only after the checks pass.

Do not start qBraid, a broad UI redesign, generic AI agent workflows, or distributed execution ahead of this slice. These depend on trustworthy state, action, replay, and evidence contracts.

## Working rule

Every pull request should have one vertical-slice outcome, explicit acceptance criteria, a scope/limitation note, and recorded validation. Prefer completing and integrating existing modules over adding parallel abstractions.
