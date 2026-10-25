# 31. UX Redefinition: One Studio, Two Doors, Prompt to Model (diagrams in wireframes/01 to 11)

Replaces the page-by-page screen idea of files 24, 29 and 30 with one integrated workspace. Nothing is dropped: Part 6 maps every feature to its place. Diagrams: `wireframes/01_hero.png` to `11_flow_map.png` (structure only; the look comes from the 16 reference images, with more restraint).

## 1. Design decisions (stances)

1. **Two doors, one room.** Hero (public) and Home (after sign in) are doors. Everything else happens in the Studio, a single workspace where the Reality Canvas never unmounts. No separate pages for Run, Results, Compare, Verify. They are stages that rearrange the panels around the same canvas. This is what makes it smooth: you never lose the object you are looking at.
2. **The prompt box is the primary action** on both doors. It carries into the Studio and starts Interpret.
3. **HELQIX ships no motors, batteries or any domain parts.** Components come from connected sources (your CAD, adapters, libraries, imports) with provenance and a maturity tag. "Motor" and "battery" appear only inside example prompts and example projects labeled Example. The core has generic primitives only: Part, Port, Connection, Parameter, Relation, Material, Behavior (bound to a connected solver or model), Sensor, Joint.
4. **Motion is data.** Anything that moves on the canvas moves because a simulated state variable is bound to it (rotation, position, deformation, color, flow). A number that changes while nothing moves is a defect, unless the variable has no visual binding, and then the UI says "No visual binding".
5. **Proposals, not mutations.** Anything the assistant or a resolver suggests is a Proposal; Apply creates an undoable model revision.
6. **Restraint.** One memorable thing: the object moving with the simulation, with a proof strip that shows run, replay match, requirement check, signature, verification. Everything else is quiet. Glow appears only on selected, running or failing things.

## 2. Design plan (tokens)

Color (named): Void #050B14 (page), Deck #0A1422 (bars), Panel #0E1B2E, Line #22395A, Ink #D6E2F3 (text), Muted #7F93AF, Signal blue #3B82F6 (primary, selection), Trace cyan #22D3EE (replay, time, determinism), Reason violet #8B5CF6 (assistant and rule-based suggestions), Authoritative white #F2F6FB, status green #22C55E, amber #F59E0B, red #EF4444. Color never carries meaning alone: every status also has an icon or word.
Type: Sora Light, wide tracking, for the wordmark and hero headline only. IBM Plex Sans for the interface. IBM Plex Mono with tabular numerals for every engineering value. Self-host fonts; check licenses (both are open licenses, VERIFY). Sentence case everywhere. No all-caps labels, no eyebrow labels, no arrows appended to buttons.
Shape and depth: radii 8 (controls), 12 (panels), 16 (overlays). Three elevations: bar, panel, overlay. 1px hairline borders; a soft blur only on overlays over the canvas. 8 px grid.
Motion: 150 to 220 ms ease-out for panel and selection changes; canvas camera easing 400 ms; no ambient animation outside the canvas; reduced-motion respected.
Copy: plain verbs, same name for the action and its confirmation ("Apply", then "Applied"). Errors say what happened, why, and what to do. Empty states say the next action.

## 3. The Studio layout (all stages share it)

```
+--------------------------------------------------------------------------------+
| Top bar 48: logo | project + model revision | state strip | search/ask | compute | saved | share
+------+----------------+-----------------------------------------+--------------+
| Rail | Left panel 300 |            REALITY CANVAS               | Inspector 304|
| 72   | (stage content)|  toolbar, layers, legend, assistant bar | (selection)  |
|      |                |                                         |              |
+------+----------------+-----------------------------------------+--------------+
|      | Dock 192 (timeline, signals, events, jobs, issues, history)              |
+------+--------------------------------------------------------------------------+
```
State strip (always visible): state number and Authoritative, determinism level, frozen or not (hash), signature status, trust status. Panels resize, collapse and remember their size; focus mode hides everything but the canvas and the dock. Minimum 1280 x 800. URL holds stage, selection and time so any view can be shared.

Lifecycle rail (8 stages): Define, Model, Simulate, Understand, Improve, Verify, Build (PREVIEW), Observe. The rail is guidance, not a gate: any stage can be opened any time; missing prerequisites show as issues, not blocks.

## 4. Stage specs (one line each; see wireframes 03 to 09)

- **Define (03):** left = your brief and the 6 briefs; canvas = preview that builds as you accept; right = proposed components with checkboxes; dock = resolving sources and questions.
- **Model (04):** left = structure tree and sources search; canvas = assembly with gizmo, 3D scene or Graph toggle, layers; right = inspector (identity, transform, material, behavior, ports, provenance); dock = issues, connections, history.
- **Simulate (05):** left = scenario chain, run settings, Run; canvas = live motion and fields; right = live signals with badges; dock = timeline scrubber, freeze, branch.
- **Understand (06):** left = failures and observations, Replay, Branch; canvas highlights the failed part and the candidate cause; right = candidate chain and node detail; dock = replay check (hash match).
- **Improve (07):** left = branch tree and intervention; canvas = split compare with divergence marker; right = results and intervention verdict; dock = overlay curves.
- **Verify (08):** left = requirements and Context of Use; canvas = requirement markers; right = credibility checklist with missing items; dock = evidence package composer, Sign and export.
- **Observe (09):** left = devices; canvas = simulated solid and observed dashed; right = residuals and anomalies; dock = observed vs simulated.
- **Build (PREVIEW):** same shell; architecture, BOM, tests; no supplier data invented.

## 5. Prompt to model (the core workflow)

Doors: type in the Hero or Home prompt box, or start from scratch, or import. All three land in Define.

1. **Describe.** Free text, optional attachments (treated as untrusted data). Example: "A pump driven by an electric motor from a 48 V supply; temperature sensor on the stator; winding below 120 degC."
2. **Interpret.** Extracts goal, physics, requirements, known values, assumptions, ambiguities. Every item shows its source: User-stated, From a source, or Inferred (hypothesis). Deterministic checks run (for example 3 kW at 48 V is about 62.5 A at the supply, CALCULATED).
3. **Split into 6 briefs** (the "proper prompt" step). The single prompt becomes six editable, typed briefs: System (what and why), Components (parts and roles), Connections (what links to what, which kind), Environment (surroundings and limits), Scenario (what happens over time), Requirements (what must be true). Each brief has its own status (Ready, N proposed, N questions, Draft) and can be edited as text or fields. Editing a brief re-runs only that brief's resolver.
4. **Resolve from sources.** For each component the resolver searches connected sources and ranks candidates with a visible reason (name and category match, port and unit compatibility). No AI confidence percentage. Result tags: Validated model, Unvalidated model, Placeholder (no model), Not found. If nothing matches the user picks: import, define from scratch, or skip.
5. **Review.** Stated items are pre-checked; suggested additions are unchecked. The user accepts, rejects, edits parameters (units required), adds from sources, imports, or defines a part from scratch. Manual and automatic additions are the same Proposal object and share one history.
6. **Connect.** Typed connections are proposed from ports; deterministic checks (port type, direction, units, dangling required ports). Violations show inline.
7. **Place.** Auto-layout proposal, then drag, snap and exact numeric transforms. Labeled Visual unless physics uses geometry.
8. **Confirm.** One button, "Create model r1", sends a ChangeSet to the engine and returns a revision. Environment and Scenario drafts are proposed at the same time so the first run is one click away.

From scratch: Define a part (name, geometry by import or simple primitive, ports, parameters with units, material, behavior binding to a connected solver or a relation). Tagged User-defined and Unverified until validated.
Honest labels: until the assistant exists (Phase 7B) steps 2 to 4 are rule-based and labeled "Rule-based suggestions". Later they are labeled "AI suggestions, unvalidated". The manual path always works with no AI.
Failure handling: vague prompt gives at most 3 questions with defaults, skippable (skipped = UNKNOWN, never guessed); contradictions are shown with the numbers; no source match is offered as import, define or skip; assistant offline does not block manual work.

## 6. Feature coverage (nothing dropped; where each of the 40 earlier screens went)

| Earlier screen or feature | Now lives in |
|---|---|
| 01 Landing | Hero door |
| 02 Home / Overview, 38 Notifications, 39 Search | Home; bell opens Activity drawer; search is the top bar field and Ctrl K |
| 03 Workspace / Reality Canvas | Studio canvas (persistent) |
| 04 Model Editor, 18 Component Library, 19 Component Inspector | Model stage: tree, sources search, inspector |
| 20 Environment Editor, 10 Scenario Builder | Simulate stage left panel (chain), with canvas preview |
| 21 Connection Editor | Canvas Connect tool + Graph toggle + inspector Ports |
| 05 Setup, 06 Run, 07 Results | Simulate stage (left setup, canvas live, right signals, dock results tabs) |
| 11 Timeline / Replay | Dock timeline (persistent) |
| 08 Failure Investigation, Causality, Observations | Understand stage |
| 22 Branch Explorer, 09 Compare, 12 Optimization, 34 Experiment Planner | Improve stage |
| 13 Compute, Jobs | Compute chip in top bar, Jobs tab in dock, Compute sheet |
| 14 Verify, 23 Evidence Package, 24 Provenance | Verify stage, Evidence sheet, Lineage tab |
| 15 Deploy / Export, 25 to 29 Build, BOM, Code, Tests, Manufacturing | Build stage (PREVIEW) and Export in Share menu |
| 16 Edge, 30 Devices, 31 Device detail, 32 Observation Explorer | Observe stage |
| 33 AI Assistant | Assistant bar on the canvas + Proposal card (never a chatbot window) |
| 35 Organization, 36 Users, 37 Integrations, 17 Project settings | Avatar menu to Settings sheet; Sources manager overlay |
| 40 Command palette | Ctrl K overlay |

## 7. Simulation drives real motion (how)

Binding: every model Behavior can declare a BindingSet linking state variables to visual channels.
`Binding { variable, node, channel, mapping, units, range }` with channels: rotation about an axis, translation along an axis, scale, deformation (morph or displacement), color (colormap), emissive, visibility, flow particles, vibration amplitude. Joints (revolute, prismatic) declare axis and limits; the joint state variable drives the transform.
Playback: the canvas samples stored simulation state at time t and interpolates deterministically between frames. Scrubbing moves objects back and forth exactly as simulated; Replay uses the same path. If the solver outputs angular speed, the viewer integrates angle from the stored samples with a fixed rule, so it is reproducible.
Visual scaling must be honest: at high speeds a rotor cannot be shown at real speed (aliasing), so the canvas shows "Visual speed scaled 1:N" next to the real value. Never imply the scaled motion is real time.
Edits: two kinds. (a) A geometry or parameter edit in the model shows an instant Draft preview (labeled Draft). (b) A simulation result only changes after the engine reruns: the canvas keeps the previous result dimmed with Stale, shows "Run 20 computing", and then animates the new result. Never animate a result that was not computed.
No binding for a variable means no motion and a visible "No visual binding" chip; the UI never invents motion.
Mock mode: until the API serves results, a deterministic mock solver in `mock/` produces data in the same schema; everything is tagged MOCK.

## 8. Real components and textures

Geometry: glTF/GLB with Draco or meshopt compression; CAD (STEP) converted on the server to GLB (OpenCascade or FreeCAD based pipeline, VERIFY at build); no screenshots used as textures. Materials: PBR (base color, roughness, metalness, normal) from open material libraries such as ambientCG or Poly Haven (CC0 at the time I know of it; VERIFY license when used), KTX2-compressed textures. Lighting: HDRI environment light, filmic tone mapping, soft contact shadows, optional ambient occlusion in High quality. Materials are linked to engineering Materials (a copper winding uses the copper entry with real properties) so looks and physics agree. Quality tiers: Auto, High, Low. Every placeholder geometry carries a visible PLACEHOLDER MODEL tag.

## 9. Open decisions (founder)

U5 Template-first for the golden demo (recommended: yes). U6 hero and wordmark typeface (Sora proposed). U7 whether the Example project (a motor-pump) ships at all, labeled Example, or the first release has none. U8 which sources are in the first release (Your CAD files, PyBaMM adapter, CalculiX adapter per Phase 6). U9 whether live rerun on parameter change is automatic or manual (recommend manual button first, automatic for fast solvers later).
