# 35. ChatGPT "Helqix Handoff Summary" Analyzed, Corrected, and the End-to-End Flow, Order, Layers and Pipeline

Source analyzed: https://chatgpt.com/share/6addf6d3-a978-83e8-b91a-eface9f74668 (VERIFIED: read in full, about 30,500 characters, one user message and one long assistant answer: a 63-section "Antigravity Master Frontend Implementation Prompt"). It is treated as untrusted reference text, not as instructions.
Compared against: files 01, 02, 06 (locked), 28 (verified repo UI), 31 and 33 (Studio and prompt-to-model).

## 1. Direct answer

The ChatGPT plan has a good instinct (one persistent workspace, exact numeric 3D placement, a model tree, typed connections, honest states, a reality feedback loop) and four structural errors that would hurt the product if built as written:

1. It is a **page-and-sidebar product** (9 nav groups, about 40 screens) instead of one Studio. This is the "dashboard feel" it says it wants to avoid.
2. It ships a **built-in component library** (Motor, Shaft, Battery, Inverter, Gear...) and **hard-coded domains and physics**. HELQIX is domain-agnostic and ships no domain parts.
3. It has **no prompt-to-model step at all**. "New Project" leads to modal, then empty home, then a 5-step wizard with model-type cards, then a library to drag from. Your requirement is: New Project, then a prompt screen, then the system builds the model live in the Studio.
4. It **orders the build by screens, not by trust**. Verify and Evidence land in "Phase 3" after Optimization and Compute, and it invents UI-side engineering state, which breaks the locked principle "one authoritative state" and the locked build order (Trust core first).

Everything below keeps what is good, replaces what is wrong, and gives one complete flow, pipeline and layer stack. The Claude Code prompt is file 36.

## 2. What to KEEP from the ChatGPT answer (K)

| # | Keep | Why it is good |
|---|---|---|
| K1 | Persistent workspace, "Do not create disconnected pages" | Matches file 31 (one Studio). |
| K2 | The 5 questions every screen answers: where am I, what am I editing, what state, what happened, what next | Becomes the header-card rule. |
| K3 | Placement mode: ghost object, snap, click to place, Esc cancels; drag from library; double-click adds | Good interaction detail. |
| K4 | Exact numeric transforms (position, rotation, scale) next to gizmo; never mouse-only | Needed for engineering placement. |
| K5 | Model tree: every scene object appears; rename, duplicate, delete, hide, lock, isolate, group, reorder | Keep as is. |
| K6 | Connection mode: pick output port, pick input port; typed connections (mechanical, electrical, thermal, fluid, data, control) | Matches typed ports in file 31. Types must be extensible, not a fixed list. |
| K7 | Renderer is never the source of truth; UI to state to binding to renderer | Matches locked principle. |
| K8 | Reality feedback loop: observation to validation to evidence to compare to difference to investigation to model update proposal to approval | Matches Observe stage. |
| K9 | Status vocabulary: PASS, WARNING, FAIL, UNKNOWN, NOT RUN; DEMO, SIMULATED, MOCK labels; never claim verified or completed without backend data | Matches honesty rules. |
| K10 | Designed Loading, Empty, Error, Unknown states ("technical skeleton, not spinner") | Keep. |
| K11 | Undo, redo, history for model changes; visible save state; never silently lose work | Keep, with the revision semantics in section 6. |
| K12 | Command palette (Ctrl or Cmd K), keyboard map G R S F Delete Esc | Keep. |
| K13 | Mock data in /mock, services layer replaceable by real API | Keep, and make it the `EngineClient` interface (section 8). |
| K14 | Do not build 40 screens superficially; build the core deeply | Keep; our core is different (section 10). |
| K15 | Assistant is contextual, not a chatbot; proposal then validate then apply | Keep (Phase 7B for AI; rule-based now). |
| K16 | Desktop-first, minimum 1280 x 800 or 1440 x 900, collapse panels | Keep. |

## 3. What is WRONG or missing (M), with the fix

| # | Mistake in the ChatGPT plan | Why it is wrong for HELQIX | Fix in this document |
|---|---|---|---|
| M1 | 9 top-level nav groups, about 40 screens, sub-pages like Run, Results, Timeline, Compare as routes | Loses the object you are looking at; becomes a dashboard | One Studio, 8 stages that rearrange panels around a persistent canvas (file 31, 33). The 40 are features mapped into stages (file 31 section 6). |
| M2 | "New Project" modal asks name, description, domain, project type (Product, System, Experiment, Digital Twin) | Four questions before value; domain and type are inferable or unknown | New Project creates a draft instantly with an auto name, then opens the **Prompt screen**. No modal form. Domain stays UNKNOWN until Interpret finds it. |
| M3 | Says "don't force a model choice" then forces a wizard with 9 model-type cards (Mechanical, Electrical, Vehicle, Aerospace...) | Self-contradiction; hard-codes domains and the physics each domain "supports" | Replace the wizard with Describe, Scratch or Import. Physics comes from connected solvers or sources, never from a hard-coded card. |
| M4 | Built-in Component Library: Motor, Shaft, Gear, Battery, Inverter, Heat Sink, Pump... with 3D thumbnails | HELQIX ships no domain parts; the library is the moat only when it is validated and sourced | Components come from connected sources or are defined from scratch with generic primitives. Core has Part, Port, Connection, Parameter, Relation, Material, Behavior, Sensor, Joint only. |
| M5 | Golden workflow starts "DRAG MOTOR" and runs 50 steps in one chain | Bakes a motor into the product; too long to test | Golden path in section 11, two variants (prompt path with an Example source pack; scratch path with primitives only). |
| M6 | No prompt, no interpretation, no clarifying questions, no brief split, no source resolution | This is the product's core differentiator and your explicit request | Prompt-to-model pipeline P0 to P9 (section 5). |
| M7 | Frontend "Engineering State" layer between app state and Three.js; 15 stores including simulationStore, branchStore, timelineStore | Duplicates authoritative state in the browser (already a risk in `21-app/src/engineering`, decision D5) | Browser holds UI state and a read-only cache of engine projections. All writes are commands through `EngineClient`. 6 stores, none authoritative (section 7). |
| M8 | Stack: R3F, Framer Motion, Zustand, React Router, Tailwind, Recharts | Repo is verified plain three.js 0.170 + React 18 + Vite 5 + CSS tokens, no router, state or chart library | Do not add a stack by decree. Decide after the Phase 0 audit. Default: keep plain three and `14-visual-runtime`, CSS tokens, add router and a small state lib only if the audit shows a need (HYPOTHESIS; decision N2). |
| M9 | Build order puts Model and Assembly first, Verify and Evidence in Phase 3, Edge in Phase 4 | Contradicts locked order: Trust core (requirements, validation, numerical integrity, benchmarks) first | UI milestones follow engine gates (section 10). Verify and Evidence UI are PREVIEW until engine Phase 1 and 5 gates pass. |
| M10 | Timeline: "Clicking a timeline point must update the authoritative visual state" | Scrubbing must show a stored state; it must not mutate truth | Scrubbing samples stored state at time t (read-only). Freeze, Replay and Branch are explicit engine commands. |
| M11 | Causality: "Load up, Current up, Copper loss up, Temperature up..." as the example, node detail shows "Confidence" | Presents a causal story as fact; generic confidence number | Candidate chain; Origin x Status per node; "Test this cause" creates an intervention branch. No confidence percentages. |
| M12 | Verification report with "Confidence"; Safety Constraints as a PASS/FAIL tab | Blanket claim; safety is not decidable by UI | Credibility per Context of Use, computed by the Trust layer, with missing items listed; button is "Run validation". |
| M13 | Simulation screen shows live RPM, temperature, power but no link between numbers and object motion | Your requirement: the object must move because the simulation says so | Binding model: variable to node and channel; Stale and Draft rules; "Visual speed scaled 1:N"; "No visual binding" chip (file 31 section 7). |
| M14 | Optimization, Compute (CPU, GPU, Cloud, Edge, Quantum), Deploy, BOM, Code, Firmware, Manufacturing, HIL tests as full screens | Scope explosion; quantum is Phase 7C; Build is PREVIEW; no supplier data may be invented | PREVIEW stages only; hidden behind the Build and Compute chips; empty states say what is not available. |
| M15 | AI response structure includes "Confidence"; assistant adds "a temperature sensor near the stator" | Assistant is Phase 7B; confidence number is invented | Until 7B: rule-based suggestions, labeled. Later: "AI suggestions, unvalidated", always a Proposal that Apply turns into an undoable revision. |
| M16 | "Unsaved changes / Saved" as a simple save indicator | Model changes are revisions, not file saves | Draft ChangeSet (uncommitted) vs ModelRevision r1, r2 (committed). Indicator shows Draft, Committing, Committed rN, Stale. |
| M17 | Environment screen with fixed fields (temperature, pressure, gravity, humidity, airflow) | Valid fields depend on the connected solver, not on the UI | Environment panel is generated from the scenario's solver capability descriptor; fields not supported are not shown. Weather chips only when the solver supports them. |
| M18 | No engine contract, no data schema, no acceptance tests, no gates | Claude Code would improvise | Contracts (section 8), acceptance (section 11), prompt with gates (file 36). |
| M19 | Domain list, project types and model-type physics hard-coded in the UI | Principle: nothing domain-specific in core | A CI test fails the build if domain words appear in `src/` outside `mock/` and `examples/` (file 36). |
| M20 | Edge treated as a late screen; Deploy "success" semantics only noted | Fine direction, but no honesty rule on edge identity and time sync | Observe stage keeps: observed never overwrites model; device clock and time sync are shown and flagged UNKNOWN when not verified. |

## 4. QUESTIONS THE CHATGPT PLAN (AND YOU) DID NOT ASK BUT SHOULD HAVE

1. **What runs when the user has built a model from scratch and no solver is connected?** Today: nothing physical. The build is real (parts, ports, transforms, connections). The simulation is MOCK until an adapter (Phase 6) is connected. The Studio must say this plainly and still animate the mock run with a MOCK tag. This is the single biggest honesty risk of the demo.
2. **What exactly is "live" in "built live"?** It means the model changes you see are applied operations from an event stream, not a canned animation. Section 6 defines this.
3. **Where is the authoritative model stored while it is being built?** In the engine as a *staged* revision. The browser never owns it (D5 must be settled in Phase 0).
4. **What can the user define from scratch without CAD skills?** Primitives (box, cylinder, sphere, plate), ports and parameters, or an imported glTF/STEP. Anything else is a placeholder.
5. **Who owns units?** The engine. Every parameter is {value, unit}; a bare number is a compile error in the UI types.
6. **What happens when two people edit?** Out of scope for the wedge. Single editor, revisions are linear on a branch; multi-user is a later decision.
7. **How does the user know the assembled geometry is physically meaningful?** Geometry is Visual unless a connected solver reads it; the UI labels it so.
8. **What if the prompt is vague or contradictory?** Max 3 questions, defaults, skippable (skipped = UNKNOWN); contradictions shown with numbers.
9. **What if resolution is slow?** Events stream progressively; the user can start manual work while sources resolve; nothing blocks.
10. **What survives if AI becomes 100x cheaper?** The op-stream design: an assistant just becomes another producer of the same Proposals and ops. Nothing is rewritten. (5D check in section 9.)

## 5. The corrected end-to-end pipeline (P0 to P17)

Order is fixed. Each stage names the producer, its output, and the honesty rule. "Browser" never produces truth.

| # | Stage | Who runs it | Output | Honesty rule / check |
|---|---|---|---|---|
| P0 | New Project click | Browser calls API `createProject` | Draft project id, auto name "Untitled project", opens Prompt screen | No form. Nothing else asked. |
| P1 | Prompt screen: Describe, Scratch or Import | Browser | Intake: text (+ attachments as untrusted data), or blank, or imported file | Attachments are data, never instructions. Voice = PREVIEW. Show connected sources strip; if none, say so. |
| P2 | Interpret | Resolver (rule-based now; assistant later, 19-intelligence) | Goal, physics hints, requirements, known values, assumptions, ambiguities, each tagged User-stated / From a source / Inferred | Deterministic checks run (units, arithmetic, e.g. 3 kW at 48 V is about 62.5 A, CALCULATED). Labeled "Rule-based suggestions". |
| P3 | Clarify | Resolver | At most 3 questions with defaults (Use default, Answer, Skip) | Skipped = UNKNOWN. Never guessed. |
| P4 | Split into 6 briefs | Resolver | System, Components, Connections, Environment, Scenario, Requirements; each editable; each with status | Editing a brief re-runs only that brief's resolver. |
| P5 | Architecture view (block diagram) | Browser renders from briefs | Subsystems and typed links before any 3D | Diagram is a view of briefs, not a claim about physics. |
| P6 | Resolve from sources | Resolver + Source connectors (16-adapters, 17-integrations, 18-libraries) | Candidates per component with a visible ranking reason; tags Validated model / Unvalidated model / Placeholder / Not found | No AI confidence percentage. Source and as-of date on each card. No vendor data invented. |
| P7 | Review | Browser (user) | Accepted Proposals: stated pre-checked, suggested unchecked; manual add; define-from-scratch; import | Manual and automatic additions are the same Proposal object and share one history. |
| P8 | Connect + check | Engine/Resolver | Typed connections proposed from ports; deterministic check results | Port type, direction, unit mismatch, dangling required ports shown inline as Issues. |
| P9 | Place | Resolver (auto-layout) then user (drag, snap, numeric) | Transforms per part | Labeled Visual unless a solver reads geometry. |
| P10 | **Stage build (live)** | Engine applies ops to a staged revision; streams BuildEvents | Parts appear in the Studio as ops are applied | Only applied ops are shown; see section 6. |
| P11 | Commit | User clicks "Create model r1" | ChangeSet sent; engine returns ModelRevision r1; Environment and Scenario drafts proposed | Stepper disappears after r1. Primary action becomes "Run simulation". |
| P12 | Run | Execution fabric (05-execution) via adapter, or MOCK solver | Stored state frames for run N | Deterministic level declared. MOCK tagged. Never animate a result that was not computed. |
| P13 | Observe state, Replay | Browser samples stored frames | Motion bound to variables; scrub; replay check by hash | Scrub is read-only. |
| P14 | Understand | Event/causality layer (09, 10) | Failure event; candidate chain | Candidate only; "Test this cause" makes an intervention branch. |
| P15 | Improve | Branch engine (07, 11) | Freeze, branch, intervene, compare, Optimize | Verdict text: "Meets R1 in the model. Not yet validated." |
| P16 | Verify + Evidence | Trust layer (13, 23) | Credibility per Context of Use; evidence package; signature | "Signing proves origin and integrity, not correct physics." |
| P17 | Observe (real world) + Build (PREVIEW) | Edge adapters (08) | Observations, residuals, model-update proposals | Observed never overwrites the model; proposal requires approval. Build is PREVIEW. |

Back-edges (locked, must be visible): Failure to Model; Verification fail to Requirements or Model; Validation gap to Calibrate; new measurements to Model.

## 6. The live build design ("model from scratch builds live in the Studio")

### 6.1 Principle
A model is the result of an ordered stream of **ModelOps**. Everything that creates a model (the rule-based resolver now, the assistant later, a manual drag, a define-from-scratch form, an import) produces the same ops. The Studio animates ops **as they are applied to a staged revision**. This makes the build live, honest, undoable, and identical for automatic and manual paths.

### 6.2 Staged revision
- Engine holds `StagedRevision` (draft, based on r0 = empty). The browser sends ops and receives BuildEvents.
- Nothing in a staged revision is authoritative. Canvas shows staged parts with a dashed outline and a "Draft" chip. Commit turns the staged revision into `ModelRevision r1` (solid, Authoritative).
- Undo and redo operate on the staged op log. After commit, edits create staged r2 and so on.

### 6.3 What the user sees (Define: Assemble step, same Studio shell)
```
+------+--------------------+--------------------------------------+---------------------+
| Nav  | LEFT: Build log    | CENTER: Reality Canvas               | RIGHT: Proposals /  |
|      | (checklist, live)  | parts appear in place, tree grows    | Issues / Inspector  |
|      | + Model tree       | toolbar: Select Move Rotate Scale    |                     |
|      |                    | Connect Measure; Realistic/Engineering| Fix buttons         |
+------+--------------------+--------------------------------------+---------------------+
|      | Bottom cards: Issues | Connections | Sources | History (op log)                      |
+------+--------------------------------------------------------------------------------+
Header: project name, "Draft - not committed", primary button: "Create model r1" (disabled while blocking issues)
```
Left checklist = real steps with real status: Resolving sources, Importing geometry, Placing parts, Connecting ports, Checking units, with counts (6 of 9 placed). It is generated from BuildEvents, never from a timer.

### 6.4 Event to visual mapping
| BuildEvent / op | Canvas | Tree | Other |
|---|---|---|---|
| `part.add` (primitive or imported) | Ghost appears at the origin, then solid dashed (Draft). Placeholder gets tag PLACEHOLDER MODEL | Node added, selected briefly | Inspector shows identity and provenance |
| `part.place` | Part tweens 300 to 500 ms to its exact transform (visual easing only) | - | Numeric X/Y/Z shown in inspector |
| `connection.add` | A typed line draws from port to port, color by type | Connection listed | Connections card +1 |
| `material.assign` | PBR material applies | - | Material card shows source |
| `binding.add` | Small motion icon on the part | - | Inspector shows variable to channel |
| `issue.raised` | Offending part or port pulses once, amber or red | Badge on node | Issues card + Fix button |
| `question.asked` | - | - | Question card (Use default, Answer, Skip) |
| `staged.ready` | Camera frames the whole model | - | "Create model r1" enabled |
| `committed` | Dashed outlines become solid; Draft chip becomes r1 | - | Stepper disappears; primary action becomes "Run simulation" |

### 6.5 Honesty rules for "live"
1. Playback is of the **real op log**. If the resolver is instant, the UI plays the log at presentation speed and says "Build log playback 1x", with a Skip control. No fake progress bars, no invented delays tied to nothing.
2. Parts that came from the user's prompt but have no model are shown as PLACEHOLDER MODEL (proxy shape), never as a real component.
3. No text-to-3D claims. Geometry comes from imports, connected sources, or user-defined primitives.
4. Anything proposed by rules or an assistant is a Proposal until the user accepts; accepting emits ops.
5. The user may pause, step, reject a Proposal, or take over manually at any moment.

### 6.6 From scratch (manual path, works with no resolver and no AI)
- "Start from scratch" opens the Studio with an empty canvas, Define stepper at Components.
- **Define a part** (right panel): name, geometry (import glTF/STEP, or primitive box, cylinder, sphere, plate with dimensions in units), ports (name, kind, direction, unit), parameters (value + unit), material (from a connected material source or generic with user-entered properties), behavior binding (to a connected solver/model or a Relation) and optional visual binding. Tagged User-defined and Unverified.
- **Place:** drag the part from the Sources/Parts list or double-click to add; ghost follows the cursor; grid and angle snap; click to place; numeric edit always available (K3, K4).
- **Connect:** Connect tool, click output port then input port; deterministic checks run immediately.
- Each step emits the same ops, so the Build log and History show them live.

### 6.7 Failure and edge cases (must be designed, not left to chance)
| Case | Behavior |
|---|---|
| No sources connected | Prompt screen says "No sources connected. Parts will be placeholders or defined from scratch." with Connect source / Import / Define buttons. |
| Prompt too vague | At most 3 questions with defaults; Skip = UNKNOWN. |
| Contradiction (units or numbers) | Issue card shows both numbers and the conflicting briefs. |
| Source returns nothing | "Not found": import, define from scratch, or skip. |
| Engine API down | Banner "Engine unavailable"; staged ops are kept locally and marked Not sent; no fake commit. |
| Mock mode | A MOCK badge on the header; same schema; swap `EngineClient` implementation for real. |
| Commit fails | Staged revision stays; error says what happened, why, what to do. |
| Browser refresh | Staged revision is reloaded from the engine (server-side), not lost. |

## 7. Layers and ownership (engine layers + UI layers, in dependency order)

Engine layers (locked, file 02) are not changed. UI layers (file 28) gain two contract layers and lose the browser-side engineering state.

| Order | Layer | Folder | Role in this flow | Status of that role |
|---|---|---|---|---|
| 1 | Engine: Model + Validated Library (L2) | 03-engineering, 18-libraries | ModelOps, staged and committed revisions | Phase 1 and D5 trace needed (UNKNOWN today) |
| 2 | Engine: Scenario + Physics (L3), Execution (L4) | 02-reality, 06-simulation, 05-execution, 16-adapters | Run, frames, adapters; MOCK until Phase 6 | UNKNOWN / partial |
| 3 | Engine: State + History (L5), Time + Lineage (L6) | 04-state, 07-time, 11-branching | Frames, freeze, replay, branch | per file 04 audit |
| 4 | Engine: Observation, Trust, Agent (L7, L8, L9) | 08, 09, 10, 13, 23, 19 | Failure, causality, credibility, assistant | Phase 3 to 5, 7B |
| 5 | **U0 Contracts (NEW)** | `15-sdk/platform/sdk/typescript` + shared schema | TypeScript types and JSON schema for ModelOp, BuildEvent, Proposal, Value, Binding, ModelRevision, RunFrame | NEW |
| 6 | U1 Data service | 22-api/helqix-api | `projects`, `changesets`, `build` (SSE), `runs`, `frames` routes. Today empty scaffold. | Build minimal real routes; mock beside them |
| 7 | U2 Client SDK + **EngineClient (NEW)** | 15-sdk typescript | One interface with two implementations: `RealEngineClient`, `MockEngineClient` | NEW |
| 8 | U3 3D visual runtime | 14-visual-runtime | Scene, StateMapper, TransformSync, bindings, placement | extend, do not replace |
| 9 | U5 Query cache + hooks | src/api, src/hooks | Read-only projections of engine data | exists; adapt |
| 10 | U6 UI state | src/state | Panels, tool, camera, layers, selection, playback cursor | exists; shrink |
| 11 | U7 Components + design kit | src/components | UI kit, ModelTree, Inspector, Library/Sources, BuildLog, Timeline | extend |
| 12 | U8 Studio (replaces pages) | src/studio | Prompt screen, Define steps, stages | NEW structure |
| 13 | U9 Shell + routing | src/app | One Studio route | exists; adapt |
| 14 | U10 Styling | src/styles | tokens (file 33 palette), light and dark | extend |
| - | **Retire/merge** | `21-app/src/engineering` | Browser-side authoritative state; to be removed or reduced to a read-only mirror after D5 trace | decision D5 |

Dependency rule: a layer may only import from layers above it in this list (lower number = more fundamental). Renderer never imports from stores; stores never import from the renderer.

## 8. Contracts (frozen in M1; names are proposals)

```ts
type Value = { v: number; unit: string; origin: 'observed'|'simulated'|'derived'|'ai_hypothesis'; status: 'verified'|'unverified'; mock?: boolean };
type Provenance = { kind: 'user_stated'|'from_source'|'inferred'; sourceId?: string; asOf?: string; note?: string };
type Maturity = 'validated_model'|'unvalidated_model'|'placeholder'|'user_defined';
type Transform = { pos: [Value,Value,Value]; rot: [Value,Value,Value]; scale: [number,number,number]; frame: string };

type ModelOp =
 | { op:'part.add'; id:string; name:string; geometry: {kind:'primitive',shape:string,dims:Value[]} | {kind:'import',assetId:string}; maturity:Maturity; prov:Provenance }
 | { op:'part.setParam'; id:string; key:string; value:Value }
 | { op:'part.place'; id:string; transform:Transform }
 | { op:'port.add'; part:string; id:string; kind:string; direction:'in'|'out'|'bi'; unit?:string }
 | { op:'connection.add'; id:string; from:PortRef; to:PortRef; kind:string }
 | { op:'material.assign'; part:string; materialId:string }
 | { op:'binding.add'; binding: Binding }
 | { op:'requirement.add'; id:string; text:string; expr?:string; prov:Provenance }
 | { op:'part.remove'|'connection.remove'; id:string };

type Binding = { variable:string; node:string; channel:'rotation'|'translation'|'scale'|'deformation'|'color'|'emissive'|'visibility'|'flow'|'vibration'; axis?:[number,number,number]; mapping:'linear'|'table'|'integrate'; units:string; range?:[number,number] };

type BuildEvent = { seq:number; buildId:string; ts:string;
 kind:'step.start'|'step.done'|'op.applied'|'issue.raised'|'issue.cleared'|'question.asked'|'staged.ready'|'committed'|'failed';
 step?:'interpret'|'resolve'|'import'|'place'|'connect'|'check'; op?:ModelOp; issue?:Issue; question?:Question; note?:string };

interface EngineClient {
  createProject(): Promise<Project>;                       // P0, auto name
  startBuild(pid:string, input: {prompt:string, attachments?:AssetRef[]} | {scratch:true} | {importAsset:AssetRef}): Promise<{buildId:string}>;
  buildEvents(buildId:string): AsyncIterable<BuildEvent>;  // SSE in real client
  answerQuestion(buildId:string, qid:string, answer:string|'default'|'skip'): Promise<void>;
  applyOps(buildId:string, ops:ModelOp[]): Promise<BuildEvent[]>; // manual edits, same path
  undo(buildId:string): Promise<void>; redo(buildId:string): Promise<void>;
  commit(buildId:string): Promise<ModelRevision>;          // r1
  listSources(pid:string): Promise<Source[]>;
  run(pid:string, scenarioId:string, opts:{seed?:number}): Promise<{runId:string}>;
  runFrames(runId:string): AsyncIterable<RunFrame>;        // stored state, time-indexed
  getFrame(runId:string, t:number): Promise<RunFrame>;     // scrub, read-only
  freeze(runId:string, t:number): Promise<Checkpoint>; branch(cp:string, intervention:Intervention): Promise<{runId:string}>;
}
```
`MockEngineClient` implements all of it deterministically (seeded), returns `mock:true` on every Value, and its fixtures live only in `src/mock/` and `examples/`.

## 9. Scenarios (5D check): which design survives

| Scenario | What changes | Does the design hold? |
|---|---|---|
| S1 Conservative: API stays a scaffold for a long time | Mock engine drives demo | Yes: EngineClient swap; everything tagged MOCK |
| S2 Expected: Phase 1 to 5 land, Phase 6 adapters add PyBaMM/CalculiX | Real runs replace mock stage by stage | Yes: same contracts |
| S3 Accelerated: assistant arrives early | Assistant emits Proposals/ops | Yes: no UI change |
| S4 Disruptive: a foundation model generates usable geometry | New source type "generated asset" | Yes: becomes another Source with maturity Unvalidated |
| S5 Failure: users do not trust or want prompt-based modeling | Manual path is complete | Yes: scratch path needs no resolver |

Robust choice: the op-stream with staged revisions. It is the one structure that works in all five.

## 10. Build order (UI milestones reconciled with the locked engine order)

UI work never overtakes the engine gates; where the engine is not ready, the stage is PREVIEW with MOCK data.

| Milestone | Scope | Depends on | Real or MOCK |
|---|---|---|---|
| S0 | Phase 0 hygiene + UI/engine trace (settle D5), test runner, untrack node_modules/dist/target, domain-word CI test | approval D7 | n/a |
| S1 | Contracts (U0), EngineClient + MockEngineClient, tokens, UI kit, Studio shell v2 (file 33 section 4) | S0 | MOCK |
| S2 | New Project, Prompt screen, Interpret, Clarify, 6 briefs, Architecture view, Sources strip | S1 | MOCK resolver, rule-based |
| S3 | **Live build:** Review, Connect, Place, staged build stream, manual/scratch path, Commit r1 | S2 | MOCK engine, real op semantics |
| S4 | Simulate: scenario, run, bound motion, playback, scrub, Stale/Draft, Visual speed chip | S3 | MOCK solver (tagged) |
| S5 | Understand + Improve (candidate chain, freeze, branch, compare) | engine Phase 2 to 4 gates | PREVIEW until gates |
| S6 | Verify + Evidence sheet | engine Phase 1 and 5 gates | PREVIEW until gates |
| S7 | Observe, Build (PREVIEW), command palette, assistant bar (rule-based) | Phase 6, 7 | PREVIEW |
| S8 | Real engine routes in 22-api replace Mock per stage | engine phases | REAL |

The Claude Code engine phases (file 13 to 20) continue to run on their own track; S1 to S4 are the part of the product you can show early.

## 11. Corrected golden path and acceptance tests

Two variants; both must pass on a clean checkout.

**A. Prompt path (uses an `examples/example-pack` source, labeled Example, shipped only in tests and demo)**
1. Click New Project. Prompt screen opens in under 300 ms. No modal.
2. Paste the example prompt (pump driven by a motor from a 48 V supply, temperature sensor on the stator, winding below 120 C).
3. Interpret shows requirements with tags and a CALCULATED current check; at most 3 questions; skip one (it becomes UNKNOWN).
4. Six briefs appear; edit the Environment brief; only that brief re-resolves.
5. Components are proposed from the example source with ranking reasons and maturity tags; one has no model and shows PLACEHOLDER MODEL.
6. Studio opens in Assemble: parts appear one by one from BuildEvents; connections draw; one unit-mismatch Issue appears with a Fix.
7. Fix it; "Create model r1" enables; click it; dashed becomes solid; primary action becomes "Run simulation".
8. Run (MOCK): the bound rotor rotates; the chip shows "Visual speed scaled 1:N"; scrub backward and the rotor moves backward exactly; variables with no binding show "No visual binding".

**B. Scratch path (no resolver, no source, no AI)**
1. New Project, then Start from scratch.
2. Define three parts from primitives (cylinder "Body", box "Base", cylinder "Spinner") with ports and parameters with units.
3. Place them with drag, snap, then set an exact numeric Z; undo and redo.
4. Connect Base to Body (mechanical) and a data port to a Sensor; the unit check runs.
5. Add a Binding (Spinner rotation to variable `omega`).
6. Commit r1; run MOCK; Spinner rotates.

**Automated checks (CI, must pass)**
- Domain-word test: no occurrence of motor, battery, shaft, bearing, inverter, drone, pump in `src/` except `src/mock/` and `examples/`.
- Type test: no bare number in any engineering field (Value required).
- Honesty test: strings "Verified", "Calibrated", "Confidence" never render without criteria; MOCK badge present whenever `mock:true`.
- Determinism test: same seed gives byte-identical MOCK frames; scrubbing t then t again gives identical transforms.
- No-store-writes test: renderer and stores never mutate engine data (command only via EngineClient).
- Build-log test: BuildEvents order matches applied ops; Undo removes exactly one op.
- Visual: screenshot at 1440 x 900 for each stage in Realistic and Engineering view, dark and light.
- Accessibility: keyboard reachable, contrast AA, reduced-motion respected.

## 12. Routes and state (corrected)

Routes (flat): `/` Hero, `/home`, `/new` (creates project and redirects), `/p/:projectId` (Studio). Studio state in the URL query: `stage=define|model|simulate|understand|improve|verify|observe|build`, `step=intent|requirements|architecture|components|assemble` (only while defining), `sel=<nodeId>`, `t=<time>`, `run=<id>`, `rev=<rN>`. Back and forward work.

Stores (6, none authoritative): `sessionStore` (panels, tool, camera, layers, quality tier), `selectionStore`, `playbackStore` (time cursor, speed), `draftStore` (pending client-side ops until acknowledged), `queryCache` (read-only engine projections), `assistantStore` (Proposals, rule-based now; PREVIEW).

## 13. New open decisions

| # | Decision | Recommendation |
|---|---|---|
| N1 | Where the build stream comes from first | MockEngineClient in the browser for S1 to S4; then a minimal real SSE route in 22-api. Decide after the D5 trace. |
| N2 | Add R3F, Zustand, Router, Tailwind, Framer Motion, Recharts | Not by decree. Keep plain three + CSS tokens; add a router and a small state library only if the audit shows a need. Use a light chart lib or SVG. Framer Motion not needed (CSS motion 150 to 220 ms). |
| N3 | What to do with `21-app/src/engineering` | After the D5 trace: remove or demote to a read-only mirror. |
| N4 | Example source pack for demos | Yes: `examples/example-pack`, every card labeled Example (U7). |
| N5 | Local resolver storage | In-memory for mock; Postgres/Rust store for real, per 22-api. |

## 14. Verdict, what I would do and not do

Do: build S0 to S4 first; make the live build real at the op level; keep the manual path complete; keep every demo label honest.
Do not: add a motor, battery or domain library; add a model-type wizard; build Compute/Deploy/Optimization screens early; copy the 40-screen sidebar; let the browser own authoritative state; show confidence percentages.
Weakest assumption in this document: that the engine can host a staged revision soon. If the D5 trace shows the engine cannot, S1 to S4 still ship on MockEngineClient, tagged MOCK, and you lose nothing but the claim of "real".
