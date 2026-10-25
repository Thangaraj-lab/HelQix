# 27. HELQIX Folder Architecture (actual + proposed)

Source: `git ls-files` on commit bf318f4 (12 Oct 2026), VERIFIED by running. Folders are listed in the locked layer order (01 to 23). `target/` (656 tracked build files, about 333 MB) and `21-app/reality-studio/node_modules/` (8,724 tracked files) are tracked in git by mistake and are excluded from the tree below. Both are Phase 0 hygiene fixes (untrack, add to .gitignore). The node_modules finding is NEW since the earlier audit (file 04).

## Part 1. Layer to folder map (build order follows the phases)

| Folder | Layer | Role | First phase that touches it |
|---|---|---|---|
| 01-foundation | shared core | ids, quantities, units, components, constraints | 0 (fix tests), 1 (numerical integrity) |
| 02-reality | 3 Scenario | reality graph, environment, topology | 0 |
| 03-engineering | 2 Model | components, equations, parameters, revisions | 0, 1 |
| 04-state | 5 State | authoritative state, snapshots, diffs | 0, 2 |
| 05-execution | 4 Execution | scenario run lifecycle | 0, 2 |
| 06-simulation | 3 Physics | helixsim deterministic kernel | 1, 2 |
| 07-time | 6 Time | checkpoints, replay, snapshots | 2 |
| 08-observation | 7 Observation | probes, signals, anomalies | 3, 7A |
| 09-failure | 7 Failure | failure detection and propagation | 3 |
| 10-events-causality | 7 Causality | events, causal graph | 3, 4 |
| 11-branching | 6 Lineage | fork, isolation, compare | 4 |
| 12-search | 6 Search | sweeps, optimization | 4 |
| 13-verification | 8 Trust | requirements, validators, evidence | 1, 5 |
| 14-visual-runtime | 10 Presentation | visual runtime package used by the UI | 5 (UI track earlier) |
| 15-sdk | 10 Integration | Rust and Python SDK | 0 (fix), later |
| 16-adapters | 4 Execution | external tool adapters | 6 |
| 17-integrations | 4 / 10 | gateway (Python, SQLite), integrations | 0 (trace), 6 |
| 18-libraries | 2 Library | validated component and material libraries | 1 |
| 19-intelligence | 1 / 9 | intent extraction, later agent | 1, 7B |
| 20-platform | 4 Fabric | compute policy, leases, audit | 6, 7C |
| 21-app | 10 UI | Reality Studio (React 18, three 0.170, Vite 5, TypeScript, lucide-react; VERIFIED from package.json; no react-three-fiber) | UI track |
| 22-api | 10 | API service | 5 |
| 23-validation | 8 Trust | benchmarks and validation data (empty today) | 1 |

## Part 2. ACTUAL tree (tracked files only)

```
helqix/
├── .gitignore
├── 01-foundation/
│   └── crates/
│       └── helqix-core/
│           ├── Cargo.toml
│           └── src/
│               ├── behaviour.rs
│               ├── component.rs
│               ├── constraint.rs
│               ├── error.rs
│               ├── id.rs
│               ├── interface.rs
│               ├── lib.rs
│               ├── property.rs
│               ├── quantity.rs
│               └── units.rs
├── 02-reality/
│   └── crates/
│       └── helqix-reality/
│           ├── Cargo.toml
│           └── src/
│               ├── component_registry.rs
│               ├── environment.rs
│               ├── lib.rs
│               ├── model.rs
│               ├── reality.rs
│               ├── reality_graph.rs
│               ├── topology.rs
│               └── validation.rs
├── 03-engineering/
│   └── crates/
│       └── helqix-engineering/
│           ├── Cargo.toml
│           ├── src/
│           │   ├── behaviour.rs
│           │   ├── constraint.rs
│           │   ├── constraint_set.rs
│           │   ├── dependency.rs
│           │   ├── dependency_graph.rs
│           │   ├── engineering.rs
│           │   ├── equation.rs
│           │   ├── equation_set.rs
│           │   ├── interface.rs
│           │   ├── lib.rs
│           │   ├── model.rs
│           │   ├── model_registry.rs
│           │   ├── parameter.rs
│           │   ├── parameter_set.rs
│           │   └── validation.rs
│           └── tests/
│               └── integration.rs
├── 04-state/
│   └── crates/
│       └── helqix-state/
│           ├── Cargo.toml
│           └── src/
│               ├── component_state.rs
│               ├── lib.rs
│               ├── physical_state.rs
│               ├── property_state.rs
│               ├── serialization.rs
│               ├── snapshot.rs
│               ├── spatial_state.rs
│               ├── state.rs
│               ├── state_diff.rs
│               ├── state_registry.rs
│               ├── state_snapshot.rs
│               ├── state_store.rs
│               ├── transition.rs
│               └── validation.rs
├── 05-execution/
│   └── crates/
│       └── helqix-execution/
│           ├── Cargo.toml
│           └── src/
│               ├── action.rs
│               ├── execution_graph.rs
│               ├── executor.rs
│               ├── lib.rs
│               ├── runtime.rs
│               ├── scheduler.rs
│               └── transition.rs
├── 06-simulation/
│   ├── README.md
│   └── engines/
│       └── helixsim/
│           ├── Cargo.toml
│           └── src/
│               ├── collision.rs
│               ├── constraints.rs
│               ├── dynamics.rs
│               ├── environment.rs
│               ├── forces.rs
│               ├── integration.rs
│               ├── kernel.rs
│               ├── lib.rs
│               ├── solver.rs
│               └── timestep.rs
├── 07-time/
│   └── crates/
│       └── helqix-time/
│           ├── Cargo.toml
│           └── src/
│               ├── clock.rs
│               ├── lib.rs
│               ├── replay.rs
│               ├── rewind.rs
│               ├── snapshot.rs
│               └── timeline.rs
├── 08-observation/
│   └── crates/
│       └── helqix-observation/
│           ├── Cargo.toml
│           └── src/
│               ├── lib.rs
│               ├── metric.rs
│               ├── observer.rs
│               ├── probe.rs
│               └── telemetry.rs
├── 09-failure/
│   └── crates/
│       └── helqix-failure/
│           ├── Cargo.toml
│           └── src/
│               ├── detector.rs
│               ├── failure.rs
│               ├── fault.rs
│               ├── lib.rs
│               └── propagation.rs
├── 10-events-causality/
│   └── crates/
│       ├── helqix-causal/
│       │   ├── Cargo.toml
│       │   └── src/
│       │       ├── causal_graph.rs
│       │       ├── cause.rs
│       │       ├── effect.rs
│       │       ├── lib.rs
│       │       ├── propagation.rs
│       │       └── root_cause.rs
│       └── helqix-events/
│           ├── Cargo.toml
│           └── src/
│               ├── event.rs
│               ├── event_bus.rs
│               ├── lib.rs
│               ├── propagation.rs
│               └── trigger.rs
├── 11-branching/
│   └── crates/
│       └── helqix-branch/
│           ├── Cargo.toml
│           └── src/
│               ├── branch.rs
│               ├── branch_point.rs
│               ├── branch_tree.rs
│               ├── comparison.rs
│               ├── lib.rs
│               └── state_fork.rs
├── 12-search/
│   └── engines/
│       └── search-engine/
│           ├── Cargo.toml
│           └── src/
│               ├── candidate.rs
│               ├── constraints.rs
│               ├── execution_scheduler.rs
│               ├── lib.rs
│               ├── objective.rs
│               ├── optimizer.rs
│               ├── ranking.rs
│               ├── sampler.rs
│               ├── search_space.rs
│               └── variable.rs
├── 13-verification/
│   └── crates/
│       ├── helqix-evidence/
│       │   ├── Cargo.toml
│       │   └── src/
│       │       ├── audit_trail.rs
│       │       ├── evidence.rs
│       │       ├── execution_record.rs
│       │       ├── lib.rs
│       │       ├── provenance.rs
│       │       └── reproducibility.rs
│       └── helqix-verify/
│           ├── Cargo.toml
│           └── src/
│               ├── constraint_check.rs
│               ├── lib.rs
│               ├── requirement.rs
│               ├── validator.rs
│               └── verification_result.rs
├── 14-visual-runtime/
│   └── engines/
│       └── visual-runtime/
│           ├── package-lock.json
│           ├── package.json
│           ├── src/
│           │   ├── index.ts
│           │   ├── interaction/
│           │   │   └── InteractionController.ts
│           │   ├── objects/
│           │   │   ├── EngineeringObject.ts
│           │   │   └── ObjectRegistry.ts
│           │   ├── scene/
│           │   │   ├── Scene.ts
│           │   │   └── SceneManager.ts
│           │   ├── state/
│           │   │   ├── StateMapper.ts
│           │   │   └── VisualState.ts
│           │   ├── synchronization/
│           │   │   └── RealitySynchronizer.ts
│           │   └── transforms/
│           │       └── TransformSync.ts
│           └── tsconfig.json
├── 15-sdk/
│   └── platform/
│       └── sdk/
│           ├── python/
│           │   ├── helqix/
│           │   │   ├── __init__.py
│           │   │   ├── action.py
│           │   │   ├── behaviour.py
│           │   │   ├── component.py
│           │   │   └── reality.py
│           │   ├── pyproject.toml
│           │   └── tests/
│           │       ├── __init__.py
│           │       └── test_sdk.py
│           ├── rust/
│           │   ├── Cargo.toml
│           │   └── src/
│           │       ├── action.rs
│           │       ├── behaviour.rs
│           │       ├── component.rs
│           │       ├── environment.rs
│           │       ├── lib.rs
│           │       └── reality.rs
│           └── typescript/
│               ├── package-lock.json
│               ├── package.json
│               ├── src/
│               │   ├── action.ts
│               │   ├── component.ts
│               │   ├── index.ts
│               │   └── reality.ts
│               └── tests/
│                   └── reality.test.mjs
├── 16-adapters/
│   └── platform/
│       └── adapters/
│           ├── Cargo.toml
│           └── src/
│               ├── adapter.rs
│               ├── capability.rs
│               ├── lib.rs
│               ├── lifecycle.rs
│               └── registry.rs
├── 17-integrations/
│   ├── README.md
│   ├── battery/
│   │   └── pybamm/
│   │       └── .gitkeep
│   ├── cad/
│   │   ├── freecad/
│   │   │   └── .gitkeep
│   │   └── opencascade/
│   │       └── .gitkeep
│   ├── cfd/
│   │   └── openfoam/
│   │       └── .gitkeep
│   ├── electrical/
│   │   └── ngspice/
│   │       └── .gitkeep
│   ├── gateway/
│   │   ├── __init__.py
│   │   ├── app.py
│   │   ├── ev_vehicle_solver.py
│   │   ├── install.ps1
│   │   ├── proofloop.py
│   │   ├── requirements.txt
│   │   └── run.ps1
│   ├── physics/
│   │   └── chrono/
│   │       └── .gitkeep
│   └── quantum/
│       ├── providers/
│       │   └── .gitkeep
│       └── qiskit/
│           └── .gitkeep
├── 18-libraries/
│   ├── .gitkeep
│   ├── README.md
│   ├── behaviours/
│   │   └── .gitkeep
│   ├── components/
│   │   └── .gitkeep
│   ├── environments/
│   │   └── .gitkeep
│   ├── failure-models/
│   │   └── .gitkeep
│   ├── materials/
│   │   └── .gitkeep
│   ├── python/
│   │   ├── helqix_drone_domain/
│   │   │   ├── __init__.py
│   │   │   ├── catalog.py
│   │   │   ├── models.py
│   │   │   └── reality.py
│   │   └── tests/
│   │       ├── test_catalog.py
│   │       └── test_models.py
│   └── validation-data/
│       └── .gitkeep
├── 19-intelligence/
│   ├── README.md
│   ├── ai/
│   │   ├── orchestration/
│   │   │   └── .gitkeep
│   │   ├── prompts/
│   │   │   └── .gitkeep
│   │   ├── provider/
│   │   │   └── .gitkeep
│   │   └── tools/
│   │       └── .gitkeep
│   ├── intent/
│   │   ├── compiler/
│   │   │   └── .gitkeep
│   │   ├── parser/
│   │   │   └── .gitkeep
│   │   └── schema/
│   │       └── .gitkeep
│   ├── optimization/
│   │   ├── planners/
│   │   │   └── .gitkeep
│   │   └── strategies/
│   │       └── .gitkeep
│   └── python/
│       ├── helqix_intelligence/
│       │   ├── __init__.py
│       │   └── intent.py
│       └── tests/
│           └── test_intent.py
├── 20-platform/
│   ├── README.md
│   ├── compute/
│   │   ├── cloud/
│   │   │   └── .gitkeep
│   │   ├── cpu/
│   │   │   └── .gitkeep
│   │   ├── gpu/
│   │   │   └── .gitkeep
│   │   ├── qpu/
│   │   │   └── .gitkeep
│   │   └── scheduler/
│   │       └── .gitkeep
│   ├── crates/
│   │   └── helqix-platform/
│   │       ├── Cargo.toml
│   │       └── src/
│   │           ├── access.rs
│   │           ├── audit.rs
│   │           ├── jobs.rs
│   │           ├── lib.rs
│   │           └── limits.rs
│   ├── security/
│   │   ├── audit/
│   │   │   └── .gitkeep
│   │   ├── permissions/
│   │   │   └── .gitkeep
│   │   └── sandbox/
│   │       └── .gitkeep
│   └── storage/
│       ├── cache/
│       │   └── .gitkeep
│       ├── interface/
│       │   └── .gitkeep
│       ├── object-storage/
│       │   └── .gitkeep
│       └── postgres/
│           └── .gitkeep
├── 21-app/
│   └── reality-studio/
│       ├── .eslintrc.json
│       ├── dist/
│       │   ├── assets/
│       │   │   ├── index-CXLyIvkh.css
│       │   │   └── index-s-7kS9OJ.js
│       │   └── index.html
│       ├── index.html
│       ├── package-lock.json
│       ├── package.json
│       ├── src/
│       │   ├── App.tsx
│       │   ├── RuntimeCanvas.tsx
│       │   ├── ScenarioExecutionBoard.tsx
│       │   ├── api/
│       │   │   ├── client.test.ts
│       │   │   ├── client.ts
│       │   │   ├── errors.ts
│       │   │   ├── resources.ts
│       │   │   └── types.ts
│       │   ├── api.ts
│       │   ├── app/
│       │   │   ├── AppShell.tsx
│       │   │   ├── ProtectedRoute.tsx
│       │   │   ├── auth-routing.test.tsx
│       │   │   ├── routes.tsx
│       │   │   └── shell.css
│       │   ├── components/
│       │   │   ├── branch-view/
│       │   │   │   ├── BranchTree.tsx
│       │   │   │   ├── CreateBranchPanel.tsx
│       │   │   │   └── branch.css
│       │   │   ├── causal-view/
│       │   │   │   ├── CausalGraph.test.tsx
│       │   │   │   └── CausalGraph.tsx
│       │   │   ├── common.tsx
│       │   │   ├── component-tree/
│       │   │   │   └── ComponentTree.tsx
│       │   │   ├── control-panel/
│       │   │   │   ├── ControlPanel.tsx
│       │   │   │   └── control.css
│       │   │   ├── failure-view/
│       │   │   │   └── FailureView.tsx
│       │   │   ├── reality-view/
│       │   │   │   ├── RealityView.tsx
│       │   │   │   ├── projection.test.ts
│       │   │   │   ├── projection.ts
│       │   │   │   └── reality.css
│       │   │   ├── state-panel/
│       │   │   │   └── StatePanel.tsx
│       │   │   └── timeline/
│       │   │       ├── Timeline.test.tsx
│       │   │       ├── Timeline.tsx
│       │   │       └── timeline.css
│       │   ├── engineering/
│       │   │   ├── authoritativeState.ts
│       │   │   ├── branchEngine.ts
│       │   │   ├── conditionEngine.ts
│       │   │   ├── engineeringReality.ts
│       │   │   ├── engineeringRuntime.ts
│       │   │   ├── executionController.ts
│       │   │   ├── index.ts
│       │   │   ├── scenarioEngine.ts
│       │   │   ├── scenarioTemplates.ts
│       │   │   ├── stateFreeze.ts
│       │   │   ├── stateTimeline.ts
│       │   │   └── visualStateBinding.ts
│       │   ├── hooks/
│       │   │   ├── useAsyncResource.ts
│       │   │   ├── useRealityData.ts
│       │   │   └── useReveal.ts
│       │   ├── main.tsx
│       │   ├── pages/
│       │   │   ├── LandingPage.tsx
│       │   │   ├── LoginPage.tsx
│       │   │   ├── PasswordRecoveryPage.tsx
│       │   │   ├── SignupPage.tsx
│       │   │   ├── architecture/
│       │   │   │   ├── ArchitecturePage.tsx
│       │   │   │   └── architecture.css
│       │   │   ├── auth.css
│       │   │   ├── branch/
│       │   │   │   └── BranchesPage.tsx
│       │   │   ├── causal/
│       │   │   │   ├── CausalPage.tsx
│       │   │   │   └── causal.css
│       │   │   ├── compare/
│       │   │   │   ├── ComparePage.tsx
│       │   │   │   └── compare.css
│       │   │   ├── evidence/
│       │   │   │   ├── EvidencePage.tsx
│       │   │   │   └── evidence.css
│       │   │   ├── execution/
│       │   │   │   ├── ExecutionPage.tsx
│       │   │   │   ├── ObservationPage.tsx
│       │   │   │   ├── TimelinePage.tsx
│       │   │   │   ├── charts.css
│       │   │   │   └── execution.css
│       │   │   ├── failure/
│       │   │   │   ├── FailurePage.tsx
│       │   │   │   └── failure.css
│       │   │   ├── home/
│       │   │   │   ├── CreateProjectModal.tsx
│       │   │   │   ├── HomePage.tsx
│       │   │   │   ├── ProjectOverviewPage.tsx
│       │   │   │   ├── ProjectsPage.tsx
│       │   │   │   ├── SettingsPage.tsx
│       │   │   │   ├── WorkspaceHomePage.tsx
│       │   │   │   ├── home.css
│       │   │   │   ├── home.test.tsx
│       │   │   │   └── workspace.css
│       │   │   ├── intent/
│       │   │   │   ├── IntentPage.tsx
│       │   │   │   └── intent.css
│       │   │   ├── landing.css
│       │   │   ├── model/
│       │   │   │   ├── ModelPage.tsx
│       │   │   │   └── model.css
│       │   │   ├── reality/
│       │   │   │   ├── RealityStudioPage.tsx
│       │   │   │   └── reality.css
│       │   │   ├── search/
│       │   │   │   ├── SearchPage.tsx
│       │   │   │   └── search.css
│       │   │   └── verify/
│       │   │       ├── VerifyPage.tsx
│       │   │       └── verify.css
│       │   ├── runtime.css
│       │   ├── state/
│       │   │   ├── AppState.tsx
│       │   │   └── auth.tsx
│       │   ├── styles/
│       │   │   ├── base.css
│       │   │   ├── tokens.css
│       │   │   └── ui.css
│       │   ├── styles.css
│       │   ├── test/
│       │   │   ├── fetch-mock.ts
│       │   │   └── setup.ts
│       │   ├── utils/
│       │   │   ├── format.test.ts
│       │   │   └── format.ts
│       │   ├── visualRuntime.ts
│       │   └── vite-env.d.ts
│       ├── tsconfig.app.json
│       ├── tsconfig.json
│       ├── tsconfig.node.json
│       └── vite.config.ts
├── 22-api/
│   └── helqix-api/
│       ├── Cargo.toml
│       ├── migrations/
│       │   └── 0001_resources.sql
│       └── src/
│           ├── auth.rs
│           ├── branches/
│           │   └── .gitkeep
│           ├── config.rs
│           ├── error.rs
│           ├── evidence/
│           │   └── .gitkeep
│           ├── executions/
│           │   └── .gitkeep
│           ├── main.rs
│           ├── projects/
│           │   └── .gitkeep
│           ├── realities/
│           │   └── .gitkeep
│           ├── request_id.rs
│           ├── resources/
│           │   └── mod.rs
│           ├── routes/
│           │   ├── .gitkeep
│           │   └── mod.rs
│           ├── searches/
│           │   └── .gitkeep
│           ├── state.rs
│           ├── store.rs
│           └── verification/
│               └── .gitkeep
├── 23-validation/
│   ├── README.md
│   ├── benchmarks/
│   │   └── .gitkeep
│   ├── domains/
│   │   ├── _template/
│   │   │   ├── behaviours/
│   │   │   │   └── .gitkeep
│   │   │   ├── components/
│   │   │   │   └── .gitkeep
│   │   │   ├── constraints/
│   │   │   │   └── .gitkeep
│   │   │   ├── environments/
│   │   │   │   └── .gitkeep
│   │   │   └── scenarios/
│   │   │       └── .gitkeep
│   │   └── drone/
│   │       ├── behaviours/
│   │       │   └── .gitkeep
│   │       ├── components/
│   │       │   └── .gitkeep
│   │       ├── constraints/
│   │       │   └── .gitkeep
│   │       ├── environments/
│   │       │   └── .gitkeep
│   │       └── scenarios/
│   │           └── .gitkeep
│   └── tests/
│       ├── .gitkeep
│       ├── verify_api_key_project_scope.py
│       └── verify_local_product.py
├── Cargo.lock
├── Cargo.toml
├── LICENSE
├── README.md
├── configs/
│   ├── execution.toml
│   ├── helqix.toml
│   ├── integrations.toml
│   ├── physics.toml
│   └── visualization.toml
├── docs/
│   ├── .gitkeep
│   └── PRODUCT_DEFINITION.md
├── rust-toolchain.toml
└── scripts/
    ├── build.ps1
    ├── build.sh
    ├── run.ps1
    ├── run.sh
    ├── setup.ps1
    └── setup.sh
```

## Part 3. PROPOSED additions (HYPOTHESIS: a design, not existing code; approve before creating)

Rule: no renames of the 23 folders. Only add.

```
helqix/
├── .github/
│   └── workflows/
│       ├── rust.yml              # build + test all crates on every push (Phase 0)
│       ├── ui.yml                # typecheck, test, a11y, perf budget (UI track)
│       └── determinism.yml       # run-twice replay check (Phase 2)
├── docs/
│   ├── LOCKED_PLAN.md            # files 01, 02, 06 condensed
│   ├── STATUS.md                 # gate tracker and layer scores (file 12)
│   ├── RUNTIME_TRACE.md          # who owns authoritative state (Phase 0, decision D5)
│   ├── ENGINEERING_RULES.md      # file 13 preamble (every agent reads this)
│   ├── DECISIONS.md              # decision log (file 08)
│   ├── UI_AUDIT.md               # UI track step 1
│   ├── UI_STACK_DECISION.md      # UI track, packages approved or rejected
│   ├── determinism.md            # exact scope of replay guarantee (Phase 2)
│   ├── security-threat-model.md  # evidence signing (Phase 5)
│   └── phase-reports/            # one report per gate (phase0.md ... phase8.md)
├── 13-verification/
│   └── crates/
│       ├── helqix-verify/        # requirements, validators, credibility (Phase 1)
│       ├── helqix-evidence/      # packages, Merkle tree, signing (Phase 5)
│       └── helqix-evidence-verifier/   # standalone verifier binary, no runtime deps (Phase 5)
├── 23-validation/
│   ├── benchmarks/
│   │   ├── analytic/             # problems with known closed-form answers
│   │   ├── reference-solvers/    # cross-check cases (PyBaMM, CalculiX)
│   │   └── regression/           # frozen golden outputs with hashes
│   ├── datasets/
│   │   ├── README.md             # protocol (decision D4); raw data NOT committed unless small and licensed
│   │   └── manifests/            # hashes, provenance, calibration references
│   └── cases/                    # reference scenarios used by gates (battery or motor after D1)
├── 16-adapters/
│   ├── conformance/              # adapter contract test-suite (Phase 6)
│   ├── pybamm/
│   └── calculix/
├── 19-intelligence/
│   └── agent/                    # permission ladder, audit log, provider interface (Phase 7B)
├── 20-platform/
│   └── routing/                  # CPU, GPU, quantum-simulator routing (Phase 7C)
├── 24-edge/                      # NEW folder only if approved (Phase 7A): ingest schema, recordings, rig docs
├── 25-build-spec/                # NEW folder only if approved (Phase 8A): requirement to decision lineage
├── 99-research/                  # NEW, isolated (Phase 8B): HELQIXSIM, Tensor-Warp, time-travel experiments
└── 21-app/reality-studio/src/    # target UI structure (UI track, files 23 to 26)
    ├── app/                      # shell, routes, providers
    ├── tokens/                   # design tokens (colors, type, spacing, glow, motion)
    ├── components/               # Panel, Badge, Value, Tabs, Drawer, Slider, Toast, EmptyState, ErrorState
    ├── state/                    # one store: time, selection, mode (engine state stays authoritative)
    ├── api/                      # typed client for gateway or api; mock layer in ../mock
    ├── mock/                     # MOCK data, always badged
    ├── scene/                    # HeroScene, overlays (heatmap, flow), quality tiers
    ├── screens/
    │   ├── landing/              # 01
    │   ├── overview/             # 02
    │   ├── workspace/            # 03 Reality canvas
    │   ├── model/                # 04
    │   ├── simulate/             # 05 setup, 06 run, 07 results, 09 compare, 10 scenario builder
    │   ├── understand/           # 08 failure and causality, 11 timeline and replay
    │   ├── improve/              # 12 optimization
    │   ├── compute/              # 13
    │   ├── verify/               # 14 verification workspace
    │   ├── export/               # 15
    │   └── edge/                 # 16
    ├── tests/                    # honesty tests, visual regression, accessibility, performance
    └── assets/                   # fonts, GLB models (compressed), icons

    (NOTE: file 28 supersedes this UI structure; it maps onto the files that already exist.)
```

## Part 4. Order in which folders get touched (matches the phases)

0: 01, 02, 03, 04, 05, 15, 17 (fix builds, trace), repo root (.gitignore, CI, docs). 1: 13, 18, 19, 23, 01. 2: 06, 07, 04, 05. 3: 08, 09, 10. 4: 11, 12, 10. 5: 13 (evidence, verifier), 22, 21, 14. 6: 16, 17. 7A: 08, 24. 7B: 19. 7C: 20. 8A: 25. 8B: 99. UI track runs in 21-app in parallel from Phase 0, but each screen stays PREVIEW until its engine gate passes.

## Notes

1. Some crate folders contain extra crates (for example 12-search/engines/search-engine is the crate named helqix-search). Use `cargo metadata` for the authoritative crate list.
2. Whether Rust crates are actually called by the gateway or UI is still UNKNOWN (decision D5, Phase 0 trace).
3. Do not create the proposed folders until the phase that needs them is approved.
