# HELQIX implementation migration map

**Reviewed:** 13 October 2026  
**Product boundary:** [`PRODUCT_DEFINITION.md`](./PRODUCT_DEFINITION.md)

This map records the repository-wide integration from the local `main` baseline
and the engineering-core handoff branch. The repository is still a prototype;
layer files and UI controls are not counted as complete unless a supported
workflow actually crosses them.

## Product entry and shared shell

| Concern | Current implementation | Decision |
|---|---|---|
| Browser entry | `21-app/reality-studio/src/main.tsx` mounts `App` | Keep as the only product entry. |
| Workspace access | Active `App` falls back from session lookup to the local-workspace gateway; no login page is shown | Keep, matching the local prototype workflow. This is not production identity/security. |
| Product navigation | One Reality Studio shell: Home, Build, Simulate, Run, Understand, Improve, Compute, Verify, Evidence | Keep one project context and one central reality viewport. |
| Alternate route stack | `src/app/routes.tsx` and `src/pages/**` were not imported by `main.tsx`; they contained authentication routes and screens with unavailable API calls | Removed the disconnected route, page, auth-state, hook, legacy-component, and duplicate API-client trees. Their product workflows now belong to the single active Reality Studio shell. |

## Cross-layer vertical paths

| Path | Connected behavior | Limit that remains visible |
|---|---|---|
| Build → Reality / API | Components, interfaces, connections, properties, requirements, and model bindings are edited in the project reality and saved through Layer 22. | Proxy connection lines are topology only; they do not claim circuit, fluid, or CAD solver behavior. |
| Reality → State → Run → Visual runtime | The Run workspace opens a deterministic scenario board. It advances authoritative state, supports step/run/pause, and renders motion from recorded state signals and logical time. | This is a deterministic state demonstration, not a physical solver. |
| State → Freeze → Replay → Branch/Compare | A run can be frozen with reality/scenario/state digests, replayed twice for determinism comparison, and branched for controlled state changes. | Branch deltas do not become solver results unless a compatible solver is actually invoked. |
| Simulate → Layer 17 integrations → saved run | The app checks real local integration health and invokes only the supported PyBaMM single-cell and prescribed EV drive-cycle operations through the gateway. | The EV result has the scoped limits in `PRODUCT_DEFINITION.md`; unavailable tools remain disabled. |
| Run → Understand | Recorded run outputs drive the displayed series, metrics, timeline, and object movement where source values exist. | No invented geometry or physical motion is presented when the run has no applicable recorded signal. |
| Requirements → Verification → Evidence | Accepted numeric requirements are checked against compatible saved run records; evidence export is backed by the gateway. | This does not provide certification or general-purpose verification for arbitrary domains. |
| Compute / provider integration | Compute fabric and the qBraid adapter expose typed provider contracts; the app shows actual gateway tool health in Integrations/Activity. | No configured quantum provider or generic remote worker is claimed as available by default. |

## Shared-code integration choices

- Merged the engineering-core handoff into the current `main` worktree without
  discarding the three local commits. The integration adds deterministic
  simulation, state replay/freeze, visual motion, branching, physics/observation
  contracts, compute-provider abstractions, evidence helpers, and worker/API
  changes.
- Resolved the overlapping state-freeze and condition contracts in favor of the
  newer validated, digest-bearing contracts; kept the execution transition
  accessors and full contract tests from the handoff.
- Preserved local workspace access, gateway CORS/run-script changes, the
  New Project flow, and local app type fixes from the saved worktree.
- Added the scenario board to the existing Run workspace with a return path to
  recorded runs. The scenario visual is driven by authoritative state and is
  labeled as a state scenario rather than a physical simulation.
- Added Home, Compute, and Evidence to the same project shell. The overview
  reads project/run/check records, Compute displays gateway-reported provider
  health, and Evidence lists saved run provenance and exports through Layer 22.
- Connected the state-scenario board to `ObservationFabric` and
  `CausalityEngine`. Component faults become state-backed anomalies; causality
  reports insufficient upstream evidence instead of inventing a physical cause.
- Corrected replay fixtures to pass immutable state snapshots, normalized
  runtime/model quantity provenance into the supported metadata vocabulary, and
  made the visual temperature scale use a stable default limit.

## Validation and release gaps

- Frontend typecheck and production build pass. Production output still emits a
  large Three.js chunk warning.
- The active frontend suite passes 16 tests, including local workspace startup
  and navigation coverage. Obsolete authentication-route tests were removed
  with the unmounted alternate app.
- Rust workspace tests are currently blocked on this Windows host because the
  MSVC linker (`link.exe`) is unavailable. This is an environment limitation,
  not a passing Rust result.
- Scenario state/replay is currently held in the client session; persistence,
  distributed job execution, identity, isolation, recovery, production
  observability, and broad domain-specific solver coverage remain release work.
- General physical simulation is not complete. The runnable solver-backed
  vertical is the explicitly scoped battery-cell/EV workflow described in the
  product definition; the deterministic motion scenario is not evidence of a
  physical model.
