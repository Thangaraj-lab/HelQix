# 28. HELQIX UI Layers: Folder Architecture (UI only)

Source: `git ls-files` at commit bf318f4 (VERIFIED by running). Scope: only the folders that make up the user-interface side of layer 10 (Presentation and Integration): 22-api, 15-sdk (TypeScript part), 14-visual-runtime, 21-app. Excludes tracked `node_modules/` (8,724 files) and `dist/`. Based on file names only; I did NOT run the UI, so what each file does is HYPOTHESIS until the UI audit (file 23, step 1).

## Part 1. UI layers, bottom to top (build and dependency order)

| # | UI layer | Folder | What it is |
|---|---|---|---|
| U1 | Data service | 22-api/helqix-api (Rust: auth, config, store, routes, migrations; resource folders branches, evidence, executions, projects, realities, searches, verification contain only .gitkeep placeholders, so they are empty) | Backend the UI talks to |
| U2 | Client SDK | 15-sdk/platform/sdk/typescript (action, component, reality) | Typed client library (Rust and Python SDKs exist too but are not UI) |
| U3 | 3D visual runtime | 14-visual-runtime/engines/visual-runtime (Scene, SceneManager, ObjectRegistry, StateMapper, RealitySynchronizer, TransformSync, InteractionController) | Three.js layer that maps state to visuals |
| U4 | In-browser engineering runtime | 21-app/reality-studio/src/engineering | TypeScript runtime: authoritativeState, branchEngine, stateFreeze, stateTimeline, scenarioEngine, conditionEngine, executionController |
| U5 | API client and data hooks | src/api, src/hooks | client, errors, resources, types; useRealityData, useAsyncResource |
| U6 | State | src/state | AppState, auth |
| U7 | Components | src/components | branch-view, causal-view, component-tree, control-panel, failure-view, reality-view, state-panel, timeline |
| U8 | Pages (screens) | src/pages | landing, auth, home, model, reality, execution, failure, causal, branch, compare, search, verify, evidence, intent, architecture |
| U9 | App shell and routing | src/app | AppShell, routes, ProtectedRoute |
| U10 | Styling | src/styles, per-page css | tokens.css, base.css, ui.css |

## Part 2. ACTUAL tree of the UI layers (tracked files, node_modules and dist excluded)

```
helqix/
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
```

(The 21-app `dist/` folder is a tracked build output: 3 files; untrack in Phase 0.)

## Part 3. Mapping the 16 reference screens (files 24 and 25) to existing files (HYPOTHESIS from file names)

| Screen | Existing file(s) | Action |
|---|---|---|
| 01 Landing | pages/LandingPage.tsx, landing.css | restyle |
| 02 Home / Overview | pages/home/HomePage, ProjectOverviewPage, WorkspaceHomePage | restyle, add Trust Summary |
| 03 Workspace / Reality Canvas | pages/reality/RealityStudioPage, components/reality-view, RuntimeCanvas.tsx | restyle with HeroScene, quality tiers |
| 04 Model Editor | pages/model/ModelPage, components/component-tree | extend |
| 05 Simulation Setup | ScenarioExecutionBoard.tsx, engineering/scenarioTemplates.ts | NEW page simulate/SetupPage |
| 06 Simulation Run | pages/execution/ExecutionPage | restyle, add Freeze button |
| 07 Results | pages/execution/ObservationPage | NEW ResultsPage or extend |
| 08 Failure / Causality | pages/failure, pages/causal, components/failure-view, causal-view | restyle, "Candidate" wording, PREVIEW until Phase 3 |
| 09 Compare | pages/compare/ComparePage | restyle, PREVIEW until Phase 4 |
| 10 Scenario Builder | engineering/scenarioEngine, scenarioTemplates | NEW ScenarioBuilderPage |
| 11 Timeline / Replay | pages/execution/TimelinePage, components/timeline | restyle, add replay hash check (Phase 2) |
| 12 Build / Optimization | pages/search/SearchPage | restyle, label proposals Unverified |
| 13 Compute / Resources | none | NEW, PREVIEW |
| 14 Verify / Validation | pages/verify/VerifyPage | restyle, honesty rules, PREVIEW until Phase 5 |
| 15 Deploy / Export | pages/evidence/EvidencePage | NEW ExportPage, PREVIEW until Phase 5 |
| 16 Edge / Monitoring | none | NEW, PREVIEW until Phase 7A |

## Part 4. PROPOSED additions inside 21-app (additive; HYPOTHESIS; approve first)

This replaces the generic UI structure in file 27. Reuse existing folders; add only what is missing.

```
21-app/reality-studio/src/
├── styles/
│   ├── tokens.css            # EXISTS: extend with glow, glass, motion, quality-tier tokens; light and dark
│   ├── base.css  ui.css      # EXISTS
│   └── effects.css           # NEW: glass panels, glow, reduced-motion rules
├── components/
│   ├── ui/                   # NEW kit: Panel, Badge (Origin x Status), Value (with unit), Tabs, Drawer, Slider, Toast, EmptyState, ErrorState, DeterminismChip, PreviewTag, MockTag
│   └── (existing folders stay)
├── scene/                    # NEW (thin): HeroScene, overlays (heatmap, flow), quality tiers; built on 14-visual-runtime, not a second 3D engine
├── mock/                     # NEW: all mock data lives here; always badged
├── pages/
│   ├── simulate/             # NEW: SetupPage, ResultsPage, ScenarioBuilderPage
│   ├── compute/              # NEW: ComputePage (PREVIEW)
│   ├── export/               # NEW: ExportPage (PREVIEW)
│   └── edge/                 # NEW: EdgePage (PREVIEW)
└── test/                     # EXISTS: add honesty tests, visual regression, accessibility, performance
```

## Part 5. Findings that change the UI plan (VERIFIED from file listing unless marked)

1. **The UI contains its own TypeScript engineering runtime** (src/engineering: authoritativeState, branchEngine, stateFreeze, stateTimeline, executionController...). This may mean authoritative state lives in the browser, not in the Rust engine. This is decision D5. If true, it violates principle 1 (one authoritative state) because two runtimes exist (Rust and TypeScript). Phase 0 trace must settle it before UI work grows. Do not build new screens on top of the second runtime until D5 is decided. HYPOTHESIS until traced.
2. **22-api is a scaffold**: only auth, config, store, request_id, routes/mod.rs and one resources module exist; branches, evidence, executions, projects, realities, searches, verification are empty placeholders. So the UI probably talks to a mock or to the Python gateway. UNKNOWN until traced.
3. **Tests exist but no test runner is declared in package.json** (no vitest or jest in devDependencies, no "test" script), while files like `client.test.ts`, `Timeline.test.tsx`, `home.test.tsx` and `test/setup.ts` exist. Likely the tests cannot run today. VERIFIED from package.json and file names; not run.
4. **Stack is plain three.js (0.170), React 18, Vite 5, TypeScript, lucide-react; no router or state or chart library is declared** (routes.tsx exists; check how routing is done, UNKNOWN). Charts are likely hand-drawn CSS/SVG (charts.css). This affects file 26: add packages only after the audit.
5. **Duplicate or legacy files**: `src/api.ts` and `src/api/`, `src/styles.css` and `src/styles/`, `src/runtime.css`, `RuntimeCanvas.tsx` and `visualRuntime.ts` at the top level. Clean up only after the audit shows which are used.
6. `dist/` and `node_modules/` tracked in git (Phase 0).

## Part 6. Order of UI work

0 Audit and cleanup (untrack dist, node_modules; add test runner; settle D5) -> tokens and UI kit -> shell restyle -> Workspace, Run, Results, Timeline, Compare restyle -> new simulate pages -> Failure and Verify (PREVIEW until engine gates) -> Compute, Export, Edge (PREVIEW) -> landing polish. Each screen keeps a PREVIEW or MOCK tag until its engine gate passes.
