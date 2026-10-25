# 37. Antigravity: Full Frontend Prompt (REFACTOR of what you already built to the corrected flow)

Paste everything under "PROMPT" into Antigravity. It is self-contained (Antigravity does not need the other docs). It supersedes file 32 wherever they differ, because file 35 corrected the flow: New Project goes straight to a Prompt screen, then a live build in one Studio, and there are no built-in parts.

Use with: file 36 is the Claude Code (engine and contracts) prompt. This one is frontend only and runs fully on a mock engine, so you can see everything now.

---

## PROMPT

### 0. Situation and what I want from you
You already built screens for HELQIX from my earlier master prompt. **Do not start over and do not delete working work.** First audit what exists, then refactor it into the structure below. Where an existing screen is good, keep and restyle it. Where it contradicts this prompt, change it. Work in milestones; after each milestone stop and send the report format in section 14. I review the running app, I do not read code.

HELQIX = "Engineering truth, with proof." A domain-agnostic platform where an engineer describes or builds a system, simulates it, understands failures, tests changes, verifies, and exports signed evidence. Its UI is one **Studio**, never a set of dashboard pages.

### 1. Rules that override everything
1. **No built-in domain parts.** HELQIX has no motors, batteries, shafts, bearings, inverters, drones or pumps in its product code or library. The core knows only: Part, Port, Connection, Parameter, Relation, Material, Behavior, Sensor, Joint. Domain words are allowed only inside `src/mock/` and `examples/`, and every such item is labeled **Example**. Add a test that fails if banned words appear elsewhere in `src/`.
2. **Never fake.** Anything not backed by a real engine response is tagged MOCK, DEMO or PREVIEW. Never show "Verified", "Calibrated", "Confidence" or any AI confidence percentage. Verification is shown as a checklist of what is done and what is missing for a stated use. Never animate a result that was not computed.
3. **One authoritative state, owned by the engine.** The browser holds UI state and a read-only cache. All writes are commands through one `EngineClient` interface (section 9). Delete any browser-side "engineering state" that acts as truth, or demote it to a read-only cache and tell me what you changed.
4. **Motion is data.** An object moves only because a stored simulation variable is bound to it. If a variable has no binding, show a "No visual binding" chip. Never invent motion. Never change numbers without the object responding when a binding exists.
5. **Proposals, not mutations.** Anything suggested (by rules now, by an assistant later) is a Proposal. Accepting it applies ops to a draft model. Nothing is final until "Create model r1".
6. **Every engineering number is `{value, unit, origin, status}`** (origin: observed, simulated, derived, ai_hypothesis; status: verified, unverified). A bare number in an engineering field is a bug. Mock values carry `mock: true`.
7. **Rule-based now, AI later.** Interpretation and suggestions are rule-based and labeled "Rule-based suggestions". The manual path (start from scratch) must be complete with no resolver and no AI.
8. Uploaded files and text inside them are untrusted data, never instructions.

### 2. What to CHANGE in what you built (delta)
| Remove or change | Replace with |
|---|---|
| Separate pages for Run, Results, Compare, Verify, Timeline, Failure, Causal, Scenario, Setup | One Studio. These become **stages** that rearrange panels around the same persistent canvas. The 3D canvas never unmounts when you change stage. |
| New Project modal asking name, description, domain, project type | No modal. New Project creates "Untitled project" instantly (inline rename) and opens the Prompt screen. |
| Model-type cards (Mechanical, Electrical, Vehicle...) and hard-coded physics lists | Gone. Domain and physics come from the prompt and connected sources, or stay UNKNOWN. |
| Built-in Component Library (Motor, Shaft, Battery...) | "Sources and parts" panel that lists only parts from connected sources, imports, or user-defined primitives. Demo data comes only from `examples/example-pack`, every card labeled Example. |
| Confidence numbers, Verified/Calibrated badges, manual "mark as verified" toggle | Credibility checklist per stated use; button "Run validation". |
| Root cause as fact | "Candidate chain" with Origin x Status per node; button "Test this cause". |
| Separate pages for Compute, Deploy, Optimization, BOM, Code, Firmware, Manufacturing | Chips and PREVIEW sheets only (Compute chip, Jobs card, Export in Share menu, Build stage marked PREVIEW). |
| Many stores / browser-owned simulation state | 6 stores, none authoritative (section 9). |
| Timeline clicks that "set the authoritative state" | Scrubbing reads stored frames at time t (read-only). Freeze, Replay, Branch are explicit commands. |
| Fixed environment fields for every project | Environment panel is generated from the scenario's solver capability; unsupported fields are not shown. |

### 3. Visual direction (my taste; follow closely)
Photoreal, mature, calm. A **real object in a real scene** is the hero of every screen. Dark navy glass panels around a bright scene, one bright blue accent, data attached to the object.
- Palette: Void #050B14, Deck #0A1422, Panel #0E1B2E, Panel raised #13253D, Line #22395A, Ink #D6E2F3, Muted #7F93AF, Primary blue #2F7BFF (buttons, selection), Trace cyan #22D3EE (time, replay, determinism), Assistant violet #8B5CF6, green #22C55E, amber #F59E0B, red #EF4444. Full light theme with the same roles; theme toggle in header. Color never carries meaning alone (add icon or word).
- Type: IBM Plex Sans (UI), IBM Plex Mono with tabular figures for every value. Hero headline and wordmark in a light wide-tracked geometric face (Sora Light). Sentence case. Self-host fonts. No all-caps eyebrow labels, no arrows appended to buttons.
- Shape: radii 8 (controls), 14 (panels), 16 (overlays); 1 px hairline borders; three elevations; blur only on overlays that sit over the scene, solid panels elsewhere (readability first). 8 px grid. Minimum text 11 px, contrast AA.
- Scene: real glTF/GLB objects (Draco or meshopt), PBR materials (base color, roughness, metalness, normal) with KTX2 textures, HDRI lighting and background (mountain valley, hangar, lab, road), filmic tone mapping, soft contact shadows. Use CC0 libraries (ambientCG, Poly Haven) and **verify and report each license**. Never use screenshots as textures. Two looks with one toggle: **Realistic** (default for Simulate) and **Engineering** (neutral studio floor with grid; default for Model and Verify). Quality tiers Auto, High, Low.
- Motion: UI transitions 150 to 220 ms ease-out; camera easing 400 ms; no ambient animation outside the canvas; one orchestrated moment on Hero load (object settles, proof strip fills in); reduced motion respected. Glow only on selected, running or failing things.
- Brand: hex-arrow logomark + HELQIX + "Engineering studio".
- Never: neon overload, floating decorative particles, gaming RGB, generic admin dashboard, charts bigger than the object, placeholder-heavy layouts.

### 4. App structure and routes
Routes (flat): `/` Hero, `/home`, `/new` (creates project, redirects), `/p/:projectId` Studio. Studio view state lives in the URL query so back/forward and sharing work: `stage=define|model|simulate|understand|improve|verify|observe|build`, `step=intent|requirements|architecture|components|assemble` (only while defining), `sel=<nodeId>`, `t=<time>`, `run=<id>`, `rev=<rN>`. Overlays (Evidence sheet, Sources manager, Settings, Activity, Compute sheet, Command palette Ctrl/Cmd K) open over the Studio, not as pages.

### 5. The Studio shell (all stages)
```
+--------------------------------------------------------------------------------------+
| Left nav 176 | Header card: thumbnail, project name, Active chip, "Model r3 · Saved 2 min ago",
| (icons-only  | count chips (parts, scenarios, runs, branches), state chips (determinism, signed),
|  when narrow)| PRIMARY SPLIT BUTTON, theme toggle, bell, avatar
|--------------+------------------------------------------------------------------------|
| Stages:      | Left panel (list)  |  CENTER: Reality Canvas        | Right: Details +     |
| Studio       | components /       |  tool strip, HUD card, camera  | Environment          |
| Define Model | scenarios /        |  presets, Realistic|Engineering|                      |
| Simulate     | branches           |  layer toggles                 |                      |
| Understand   |--------------------+--------------------------------+----------------------|
| Improve      | Bottom: 4 cards (Live telemetry | Graphs | Event log | State inspector/map) |
| Verify       |                                                                           |
| Observe      | Violet "Assistant" button bottom-left (rule-based suggestions, PREVIEW)  |
| Build PREVIEW|                                                                           |
+--------------+---------------------------------------------------------------------------+
```
- **One primary action**, top right, always the next best step: Describe it, then Create model r1, then Run simulation, then Investigate failure, then Test this cause, then Compare, then Verify, then Export evidence. Label and target change with project state; other choices are in the dropdown.
- Nav items show small status (Simulate: Run 19 running; Verify: 2 gaps; Improve: 2 branches). Any stage opens any time; missing prerequisites appear as Issues, never walls.
- Stepper (Intent, Requirements, Architecture, Components, Assemble) is visible only while defining. After model r1 exists it disappears.
- Where-am-I is answered by the highlighted nav item, header card and selected object. Every screen answers: where am I, what am I editing, what state, what happened, what next.
- Bottom four cards can be expanded, replaced (Event log becomes Jobs) or collapsed. Panels resize, collapse and remember size. Focus mode shows only canvas and timeline. Minimum 1280 x 800.

### 6. Screens (build in this order)

**6.1 Hero (public door).** Full-bleed photoreal object in a cinematic scene (use one Example scene). Headline "Engineering truth, with proof." One line subtext. Large glass prompt box "Describe what you want to engineer…" with Attach files, Start from scratch, Import from tools and a blue "Build model" button. A thin **proof strip**: Run, Replay matches, Requirement checked, Signed, Verified by anyone (these are the product's promise, shown as a static explainer, not as live claims). "Connects to the tools you already trust" chip row (Example). Top bar: logo, Product, Integrations, Docs, Pricing, Sign in, Start free. Typing in the box and pressing Build model carries the text into the Prompt screen.

**6.2 Home (after sign in).** Greeting, the same big prompt box with three options, project cards (real rendered thumbnail of the object in its scene, name, last run state, one trust chip that states facts like "2 gaps"), Recent runs, right column: Needs attention, Connected sources, Trust summary.

**6.3 New Project to Prompt screen (Define: Intent).** Click New Project: `createProject()` returns instantly, URL becomes `/p/:id?stage=define&step=intent`. Studio shell with the stepper. Center: a calm panel over a blurred photoreal scene:
- Title "What do you want to engineer?"
- Mode tabs: Text, Reference, Voice (PREVIEW), Blank. Large text area, example prompt chips (labeled Example), attach, character counter.
- Buttons: blue **Describe it**, secondary **Start from scratch**, **Import**.
- "Connected sources" strip below: chips for each connected source with status; if none: "No sources connected. Parts will be placeholders or defined from scratch." with Connect, Import, Define buttons.
Nothing else is asked.

**6.4 Define: Requirements.** After Describe it, "Interpreting…" shows real steps from events. Left: your brief text. Center: **six brief cards** (System, Components, Connections, Environment, Scenario, Requirements) each with status chip (Ready, N proposed, N questions, Draft), editable as text or fields; editing re-runs only that brief. Right: "Extracted requirements" list with value + unit + source tag (User-stated, From a source, Inferred) and CALCULATED checks (for example "3 kW at 48 V is about 62.5 A"). Bottom: up to **3 question cards** with Use default, Answer, Skip (Skip = UNKNOWN). Label "Rule-based suggestions". Primary: Continue.

**6.5 Define: Architecture.** Center: a **block diagram** of subsystems over a faint translucent render, blocks grouped in dashed subsystem boxes, links colored by type (electrical, mechanical, thermal, data) with legend. It is generated from the briefs and has no physics claims. Primary: Continue to components.

**6.6 Define: Components.** For each block: candidate cards from connected sources with product-style render, name, key specs with units, source label with as-of date, maturity tag (Validated model, Unvalidated model, Placeholder, Not found), visible ranking reason ("matches voltage and port types"), Select. Always present: "Define from scratch" and "Import file" cards. No vendor names unless a source supplied them; demo cards say Example.

**6.7 Define: Assemble (LIVE BUILD).** The most important screen. Layout: left Build log (checklist driven by real events: Resolving sources, Importing geometry, Placing parts, Connecting ports, Checking units, with counts like "6 of 9 placed") above the model tree that grows; center Reality Canvas where parts appear; right Proposals, Issues and Inspector; bottom cards Issues, Connections, Sources, History (op log). Header chip "Draft - not committed". Behavior per event:
- `part.add`: ghost appears, becomes a dashed Draft part (PLACEHOLDER MODEL tag if no model). Tree node appears.
- `part.place`: part tweens 300 to 500 ms to its exact transform.
- `connection.add`: typed colored line draws between ports.
- `material.assign`: PBR material applies.
- `issue.raised`: the part/port pulses once; Issue card with Fix button.
- `staged.ready`: camera frames the model; **Create model r1** enables (disabled while blocking Issues).
- `committed`: dashed becomes solid, chip becomes "r1", stepper disappears, primary becomes **Run simulation**, Environment and Scenario drafts appear as Proposals.
Controls: Pause, Step, Skip to end, "Build log playback 1x/2x/4x" (label it as playback of the real op log; never a fake progress bar), Reject Proposal, Undo, Redo. The user can take over manually at any time: drag a part, edit X/Y/Z, add a connection. Manual and automatic edits produce the same ops and share one history.

**6.8 Scratch path (inside Assemble).** "Start from scratch" opens the same Studio with an empty canvas. **Define a part** panel: name, geometry (import glTF/STEP or primitive box/cylinder/sphere/plate with dimensions and units), ports (name, kind, direction, unit), parameters (value + unit), material (generic with entered properties, or from a source), behavior/relation binding, optional visual Binding. Tagged User-defined and Unverified. Placement: drag from the parts list or double-click; ghost follows cursor; grid snap (default 1 mm, show units) and rotation snap 15 degrees; click places; Esc cancels; G/R/S/F keys; gizmo plus numeric position/rotation/scale always editable. Connect tool: click output port then input port; deterministic checks run immediately. Undo/redo for add, move, rotate, scale, delete, rename, connect, disconnect, parameter change.

**6.9 Model stage.** Neutral Engineering view. Left component list with thumbnails; center assembly with selection outline, bounding box, gizmo, layers (Structure, Physics, Environment, State, Flow, Sensors, Connections, Failures, Observations); right Component details (photo, tabs Parameters, Behavior, Ports, Source; sliders with units; Material card with real PBR swatch); bottom cards Issues, Connections, History, Sources. 3D or Graph toggle for connections. Tree operations: rename, duplicate, delete, hide, lock, isolate, group, reorder.

**6.10 Simulate stage.** Realistic view with follow camera. Left: Scenario cards numbered 01 to 05 with tags (Current, Baseline) and play buttons; Simulation controls (Pause, Stop, speed 0.5x 1x 2x 4x, time "32.4 s of 120 s"); run settings summary (solver, time step, duration, seed, determinism level). Right: Environment card (only fields the scenario supports; weather chips Clear/Cloudy/Rain/Fog/Night only if supported) and scenario parameters. Center: object moving per bindings, floating HUD card (four live values with units and badges Simulated, Unverified, MOCK), compass, camera presets (Perspective, Top, Front, Right; Follow, Orbit, FPV). Bottom: live telemetry tiles with sparklines, real-time graphs with cursor tooltip, event stream, state inspector.
- Edits: a parameter edit shows an instant **Draft** preview; a result changes only after rerun; the previous result stays dimmed with a **Stale** chip while "Run N computing"; then it animates.
- If a variable has no binding: "No visual binding". If speed is visually scaled: "Visual speed scaled 1:N".
- Timeline (dock) is **state replay**, not video: Play, Pause, Step -1/+1, Jump to event, Freeze state, Branch. Scrubbing reads stored frames; the object moves back and forth exactly.

**6.11 Understand stage.** On failure the camera flies to the failed part, it glows with a heat gradient, a red banner states what failed and when ("Winding temperature above limit at 8.42 s"), and the primary action becomes **Investigate failure**. Left list of failures and observations; center highlights the candidate cause with thin leader lines; right vertical **Candidate chain** (5 rounded nodes) and a node detail card (relation, previous value, value at failure, Origin x Status); label "Candidate: structural and temporal"; button **Test this cause** (creates an intervention branch). Bottom: Replay check card ("Replay matches original" with hash).

**6.12 Improve stage.** Tabs Original, Cooling +20 %, Load -10 %, New branch (example names; real names come from user interventions). Left intervention sliders with units and "Forked from freeze at 8.0 s". Center two photoreal views side by side with divergence marker. Right metric deltas and a verdict card "Meets R1 in the model. Not yet validated." (both facts). Bottom overlay curves and a branch strip R0 to R3 with thumbnails. Primary **Compare**. Optimization is an automated version of branching: a drawer inside Improve (PREVIEW).

**6.13 Verify stage.** Left requirements with pass/warning/not-run states; center object with requirement markers; right **credibility checklist** (validation error against reference data, uncertainty, determinism, numerical checks, independent review) listing what is missing and a card "Not yet verified for this use"; bottom evidence package composer (six included items) and **Sign and export**. Primary **Run validation** then **Export evidence**. PREVIEW until the engine supports it.

**6.14 Evidence sheet (overlay).** Right slide-over: package contents tree, root hash, signature status, verifier command with Copy, note "Signing proves origin and integrity, not correct physics", buttons Export evidence and Generate report.

**6.15 Observe stage.** Real device on a test-bench photo blended with its digital twin overlay (solid simulated, dashed observed). Left device list with status; right residual cards, anomaly events, note "Observed data never overwrites the model", button **Propose model update**; bottom observed vs simulated chart. PREVIEW.

**6.16 Build stage (PREVIEW).** Architecture, BOM, tests. Never invent supplier data; empty states explain what is not available.

**6.17 Overlays.** Command palette (Create project, Add part, Find part, Run simulation, Pause, Freeze, Branch, Compare, Verify, Open evidence, Open sources). Sources manager. Settings sheet (avatar menu). Activity drawer (bell). Assistant bar on the canvas (rule-based suggestions; PREVIEW) that shows Intent, Proposal, Reasoning, Expected effect, and actions Apply, Simulate, Create branch, Reject. It is never a chatbot window.

### 7. States (design every one)
Loading: technical skeleton, not a spinner. Empty: say what to do next with a button. Error: what happened, why, what can be done, Retry, View details. Unknown: show `UNKNOWN` and "Data unavailable"; never fill. Engine unavailable: banner; local pending ops marked "Not sent"; never fake a commit. Status vocabulary: Draft, Ready, Running, Paused, Frozen, Stale, Outdated, Syncing, Warning, Failed, Unknown. Save indicator: Draft, Committing, Committed rN, Stale.

### 8. 3D and runtime requirements
- Keep the existing three.js 0.170 renderer and visual-runtime layer (Scene, SceneManager, StateMapper, TransformSync, RealitySynchronizer, InteractionController). Do not replace it with another engine. Add packages only after you list them with a reason and I approve (candidates: react-router, a small state library, a light chart/uPlot library, GLTF/Draco/KTX2 loaders already in three/examples).
- The renderer never imports stores and is never the source of truth. Pipeline: EngineClient data, read-only cache, binding layer, renderer.
- Binding engine: `Binding {variable, node, channel, axis?, mapping, units, range}` with channels rotation, translation, scale, deformation, color, emissive, visibility, flow, vibration. Angular speed is integrated from stored samples with a fixed rule so scrubbing is reproducible. Flow streaks only from a solved field; otherwise an "Environment preview, not solved" label.
- Placement: ghost, grid and rotation snap, gizmo, selection outline, bounding box, numeric transforms. Label geometry "Visual" unless a solver reads it.
- Performance: 60 fps with 200 parts on a mid laptop; instancing for repeats; frustum culling; lazy-load assets; Draco/meshopt; KTX2; Auto/High/Low tiers with an on-screen FPS debug toggle.
- Keyboard: G move, R rotate, S scale, F focus, Delete, Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z, Esc, Ctrl/Cmd+K. Mouse: left drag orbit or select by mode, right drag pan, scroll zoom.

### 9. Data layer (build this before the screens depend on it)
Stores (6, none authoritative): `sessionStore` (panels, tool, camera, layers, quality), `selectionStore`, `playbackStore` (time cursor, speed), `draftStore` (pending ops until acknowledged), `queryCache` (read-only engine projections), `assistantStore` (Proposals; rule-based; PREVIEW).
Contracts (TypeScript, in a shared `contracts/` package so the real engine can implement them later):
```ts
type Value = { v:number; unit:string; origin:'observed'|'simulated'|'derived'|'ai_hypothesis'; status:'verified'|'unverified'; mock?:boolean };
type Provenance = { kind:'user_stated'|'from_source'|'inferred'; sourceId?:string; asOf?:string; note?:string };
type Maturity = 'validated_model'|'unvalidated_model'|'placeholder'|'user_defined';
type Transform = { pos:[Value,Value,Value]; rot:[Value,Value,Value]; scale:[number,number,number]; frame:string };
type ModelOp =
 | {op:'part.add'; id:string; name:string; geometry:{kind:'primitive';shape:string;dims:Value[]}|{kind:'import';assetId:string}; maturity:Maturity; prov:Provenance}
 | {op:'part.setParam'; id:string; key:string; value:Value}
 | {op:'part.place'; id:string; transform:Transform}
 | {op:'port.add'; part:string; id:string; kind:string; direction:'in'|'out'|'bi'; unit?:string}
 | {op:'connection.add'; id:string; from:PortRef; to:PortRef; kind:string}
 | {op:'material.assign'; part:string; materialId:string}
 | {op:'binding.add'; binding:Binding}
 | {op:'requirement.add'; id:string; text:string; expr?:string; prov:Provenance}
 | {op:'part.remove'|'connection.remove'; id:string};
type BuildEvent = {seq:number; buildId:string; ts:string;
 kind:'step.start'|'step.done'|'op.applied'|'issue.raised'|'issue.cleared'|'question.asked'|'staged.ready'|'committed'|'failed';
 step?:'interpret'|'resolve'|'import'|'place'|'connect'|'check'; op?:ModelOp; issue?:Issue; question?:Question; note?:string};
interface EngineClient {
  createProject():Promise<Project>;
  startBuild(pid:string, input:{prompt:string;attachments?:AssetRef[]}|{scratch:true}|{importAsset:AssetRef}):Promise<{buildId:string}>;
  buildEvents(buildId:string):AsyncIterable<BuildEvent>;
  answerQuestion(buildId:string, qid:string, answer:string|'default'|'skip'):Promise<void>;
  applyOps(buildId:string, ops:ModelOp[]):Promise<BuildEvent[]>;
  undo(buildId:string):Promise<void>; redo(buildId:string):Promise<void>;
  commit(buildId:string):Promise<ModelRevision>;
  listSources(pid:string):Promise<Source[]>;
  run(pid:string, scenarioId:string, opts?:{seed?:number}):Promise<{runId:string}>;
  runFrames(runId:string):AsyncIterable<RunFrame>;
  getFrame(runId:string, t:number):Promise<RunFrame>;
  freeze(runId:string, t:number):Promise<Checkpoint>;
  branch(cp:string, intervention:Intervention):Promise<{runId:string}>;
}
```
**MockEngineClient** implements all of it deterministically (seeded), tags every Value `mock:true`, persists the staged build so a page refresh restores it, and lives in `src/mock/`. A `RealEngineClient` stub throws "not implemented" (never pretends). The UI imports only the interface.

**Mock build script for the Example prompt** ("A pump driven by an electric motor from a 48 V supply; temperature sensor on the stator; winding below 120 degC."): events in this order: `step.start interpret` then `question.asked` (load) then `step.done interpret`; `step.start resolve` then `part.add` for "Power supply (Example)", "Electric motor (Example)", "Temperature sensor (Example)", "Pump (no model)" with maturity placeholder; `step.start place` with `part.place` for each; `step.start connect` with `connection.add` electrical, mechanical, data; one `issue.raised` (supply voltage unit vs motor input mismatch, with a Fix that emits a corrected op); `step.start check`; `staged.ready`. Then `commit()` returns r1. The mock solver for run 1 produces a rotor angular speed and a rising winding temperature that crosses 120 degC at 8.42 s, so the failure flow can be shown (tagged MOCK).
**Mock script for Start from scratch:** empty staged revision; every user action calls `applyOps` and returns the resulting events.

### 10. Honesty and copy rules (checked by tests)
Plain verbs; same word for action and confirmation ("Apply", then "Applied"). Never the strings "Verified", "Calibrated", "Confidence" unless the criteria are on screen. "Meets R1 in the model. Not yet validated." style verdicts. Every value shows a unit; every mock shows MOCK; every placeholder shows PLACEHOLDER MODEL; every example shows Example. Voice input and Build stage show PREVIEW. No text-to-3D claims. No invented vendor data, prices or part numbers.

### 11. Milestones (stop and report after each; do not continue until I say "continue")
- **A0 Audit (no features):** list every existing screen/route/component from my earlier prompt, map each to the new structure (keep, restyle, merge, remove), list browser-side engineering state you found, list packages used, list tests and whether they run. Show the plan; wait.
- **A1 Foundation:** contracts package, EngineClient + MockEngineClient, tokens (dark and light), fonts, UI kit (Panel, Value, MockTag, PreviewTag, PlaceholderTag, ExampleTag, Tabs, Drawer, Slider with units, Toast, EmptyState, ErrorState, Skeleton), Studio shell with one route and URL-held state, domain-word and honesty tests.
- **A2 Doors and Prompt screen:** Hero, Home, New Project flow, Prompt screen with sources strip, Interpret, Clarify, six briefs, Architecture diagram, Components cards, Review.
- **A3 Live build:** Assemble with Build log, canvas events, model tree, Issues and Fix, manual and scratch path, placement tools, undo/redo, Commit r1.
- **A4 Simulate:** bindings, playback, scrub, Draft/Stale rules, HUD, telemetry cards, environment panel, real assets and HDRI scene, quality tiers.
- **A5 Understand and Improve:** failure focus, candidate chain, Test this cause, freeze, branch, compare, branch strip.
- **A6 Verify, Evidence, Observe, Build (PREVIEW), overlays:** credibility checklist, evidence sheet, observe, command palette, settings, activity, assistant bar.
- **A7 Polish and hardening:** accessibility, reduced motion, light theme, performance pass, visual regression screenshots, empty/error states for every screen.

### 12. Acceptance (automated where possible)
1. **Golden path A (prompt):** New Project, Prompt screen (no modal) in under 300 ms, paste the Example prompt, see requirements with CALCULATED check, skip one question (becomes UNKNOWN), edit the Environment brief and see only that brief re-resolve, choose components (one shows PLACEHOLDER MODEL), watch the live build, fix the Issue, Create model r1, Run simulation (MOCK), rotor rotates, "Visual speed scaled 1:N" shows, scrub backward and it rotates backward, failure at 8.42 s focuses the camera and the primary action becomes Investigate failure.
2. **Golden path B (scratch, no resolver, no source, no AI):** define three primitive parts with ports and units, place them with drag, snap and an exact numeric Z, undo and redo, connect them with a unit check, add a rotation Binding, Create model r1, Run (MOCK), the bound part rotates.
3. Tests that must pass: no banned domain words outside mock/examples; no bare numbers in engineering types; forbidden-strings test; MOCK/PLACEHOLDER/Example badge presence; determinism (same seed gives identical frames; scrubbing t then t gives identical transforms); renderer and stores never write engine data; build log order equals applied ops; undo removes exactly one op; refresh restores the staged build; no timer-driven progress in the build log.
4. Screenshots at 1440 x 900 for every stage in Realistic and Engineering view, dark and light.
5. 60 fps with 200 parts on the Auto tier on a mid laptop (report measured numbers).

### 13. Non-goals
Built-in component libraries; model-type wizards; separate pages per stage; Compute/Deploy/Optimization/BOM/Code/Firmware/Manufacturing screens; quantum UI; multi-user editing; text-to-3D generation; real backend changes (Rust, API) in this task; pushing to git or deploying without my explicit permission.

### 14. Report format after every milestone (plain English)
1. What now works, with the exact steps to click through and the command to run the app.
2. What is MOCK or PREVIEW.
3. Test results (counts and the failures, honestly).
4. What you kept, changed, removed from my earlier build.
5. Packages added (with reasons) and licenses of any assets.
6. Contradictions or surprises, and decisions I must make with your recommendation.
7. What you did not verify.
8. Screenshots or a screen recording of the milestone.
