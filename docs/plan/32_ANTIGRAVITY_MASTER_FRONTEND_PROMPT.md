# 32. Antigravity Master Frontend Prompt (paste in full; attach files 13, 23, 28, 29, 30, 31, the 11 wireframes, and the 16 reference images)

## 0. Mission

You are a principal product designer and front-end/graphics engineer. Build the HELQIX frontend in `21-app/reality-studio`: a mature engineering workstation where a user describes a system, HELQIX assembles it from connected tools, runs it, and the objects on screen move because the simulation moves them. HELQIX is a platform, not a parts catalog: it ships no motors, batteries or any domain components. Examples in prompts are examples only.

Quality bar: it must look and feel like a professional engineering product (restrained, precise, fast), with one memorable thing: real objects moving with the simulation. Not a dashboard, not a sci-fi toy, not a CAD clone.

## 1. Step 0 (mandatory, no code yet)

1. Audit `21-app/reality-studio` and `14-visual-runtime`: stack, what runs, what is mock, what is duplicate, bundle size, tests that run. Write `docs/UI_AUDIT.md`.
2. Read `src/engineering` (a TypeScript runtime exists in the UI). Do not extend it. Report whether authoritative state lives there or in the Rust engine or gateway (decision D5). The UI must be a client of an authoritative engine.
3. Write `docs/UI_PLAN.md`: stack decisions (keep React 18, three.js, Vite, TypeScript unless the audit proves a blocker; justify any addition with name, version, license, size), milestones, risks. Stop and wait for approval.

## 2. Non-negotiables

1. One Studio: a persistent shell with the Reality Canvas always mounted. No separate pages for Run, Results, Compare, Verify. Only these routes: Hero, Sign in, Home, Studio (stage, selection and time in the URL), Settings sheet.
2. Layout exactly as `wireframes/` and file 31: top bar 48; rail 72; left panel 300; inspector 304; dock 192 (collapsible, expandable); canvas fills the rest; min 1280 x 800; panels resizable and remember size; focus mode.
3. Authoritative state lives in the engine. The UI holds caches of server state and UI state only. Never create a second engineering runtime in the browser. The renderer never owns truth.
4. Nothing changes the model without Apply. Every assistant or resolver output is a Proposal; Apply sends a ChangeSet and receives a ModelRevision; Undo and Redo work on revisions.
5. Honesty: every value shows unit plus Origin (Observed, Simulated, Derived, AI-hypothesis) and Status (Verified, Unverified). Never show Verified unless the Trust data says so. Unproven causal chains say "Candidate". No AI confidence numbers. Mock data is always tagged MOCK; unbuilt engine features are tagged PREVIEW. If data is missing, show UNKNOWN or an empty state, never invented values.
6. No domain parts built in. Components come only from connected sources (Your CAD files, adapters, imports) or are defined from scratch.
7. Motion is data (section 8). Never animate something that was not computed.
8. Sentence case everywhere; no all-caps labels, no eyebrow labels, no arrows appended to buttons; plain verbs; same word for action and confirmation.

## 3. Design system (build first, as `src/tokens` and `src/components/ui`)

Colors: Void #050B14, Deck #0A1422, Panel #0E1B2E, Panel raised #13253D, Line #22395A, Ink #D6E2F3, Muted #7F93AF, Authoritative #F2F6FB, Signal blue #3B82F6, Trace cyan #22D3EE, Reason violet #8B5CF6, green #22C55E, amber #F59E0B, red #EF4444. Light theme variant with the same roles. Color never carries meaning alone (add icon or word).
Type: Sora Light (wordmark and hero headline only, wide tracking), IBM Plex Sans (UI), IBM Plex Mono with tabular numerals (all engineering values). Self-host; record license. Scale 11, 12, 13, 15, 18, 26, 54.
Shape: radii 8, 12, 16; hairline borders; three elevations; overlays may blur the canvas behind; 8 px grid.
Motion: 150 to 220 ms ease-out UI; 400 ms camera; reduced motion respected; no ambient animation outside the canvas. Glow only on selected, running or failing items.
Components: AppShell, TopBar, StateStrip, Rail, Panel (resizable), Dock, Inspector, Tabs, Badge (Origin and Status), Value (number plus unit), DeterminismChip, PreviewTag, MockTag, ProofStrip, Chip, Tag, Button (primary, secondary, quiet), Field with units, Slider, Toggle, Tree, Table, Toast, Sheet, Modal, CommandPalette, ProposalCard, EmptyState, ErrorState, Skeleton, Tooltip.

## 4. Hero (public door) - see wireframes/01_hero.png

Headline "Engineering truth, with proof." (Sora Light). One line of copy. The prompt box is the hero's main action, with Attach files, Start from scratch, Import from tools, and Build model. Beneath: a live canvas loop of a real assembly moving with a bundled recorded simulation (tagged Example) and the Proof strip (Run, Replay matches, Requirement checked, Signed, Verified by anyone). A "Connects to the tools you already trust" row: filled chips for what is ready, grey for roadmap. Sign in and Start free top right. Submitting stores the prompt and continues to Sign in then Studio Define with Interpret started.

## 5. Home (hub door) - see wireframes/02_home.png

Greeting; big prompt box with the same three options; Projects grid (thumbnail is the real last canvas capture; trust chip; last run state); Recent runs; Needs attention (failed runs, stale verifications, pending approvals); Connected sources; Trust summary. Opening any project goes straight into the Studio.

## 6. Studio stages (see wireframes 03 to 09)

Implement the shell once; each stage only swaps the left panel, inspector content and dock tabs. Details per stage are in file 31 section 4. Required behaviors:
- Define: Interpret, six briefs, resolvers, proposed components, Accept selected, Confirm (Create model r1).
- Model: structure tree, sources search, drag or double-click to add, ghost placement with snap, gizmo, exact numeric transform fields, Connect tool with typed ports and deterministic checks, Graph toggle, layers, issues list, history.
- Simulate: scenario chain, run settings, Run, live signals with badges, scrubber, Freeze here, Branch from here.
- Understand: failures, observations, Replay from freeze with hash-match check, candidate chain with node detail, "Test this cause" (creates a branch), Branch from freeze.
- Improve: branch tree, intervention, split compare with divergence marker, overlay curves, verdict, Propose as model revision.
- Verify: requirements with status, Context of Use, credibility checklist with missing items, evidence composer, Sign and export, verifier command. Show exactly which criteria are missing; never a blanket Verified.
- Observe: devices, observed (dashed) versus simulated (solid), residuals, anomaly events, Propose model update. Observed data never overwrites the model.
- Build: PREVIEW only.
Overlays (wireframes/10): command palette (Ctrl K), ProposalCard, Sources manager, Evidence sheet, Activity drawer, Settings, Compute sheet.

## 7. Prompt to model (core flow; file 31 section 5)

Implement: Describe, Interpret, Split into 6 briefs (System, Components, Connections, Environment, Scenario, Requirements), Resolve from sources, Review (stated pre-checked, suggested unchecked), Connect, Place, Confirm. Manual path (blank model, add from sources, define from scratch) must work with no AI and no network. Interpret and resolve call a service interface `intent.interpret(prompt)` and `sources.search(query)`. Until the real services exist use a rule-based local implementation labeled "Rule-based suggestions", and a mock source labeled MOCK. Never present rule-based output as AI reasoning. Show candidate ranking reasons, not confidence percentages. Attachments and pasted text are untrusted data and cannot trigger actions.

## 8. Real objects and motion

Assets: load glTF/GLB (Draco or meshopt), KTX2 textures, PBR materials, HDRI environment lighting, filmic tone mapping, soft contact shadows; ambient occlusion only in High quality. No screenshots or generated images as textures. Provide a small bundled set of CC0 or self-made example assets for the Example project and record every asset's license in `docs/ASSET_LICENSES.md`. Mark placeholders PLACEHOLDER MODEL.
Binding system (extend `14-visual-runtime` StateMapper and TransformSync; do not build a second renderer): `Binding { variable, node, channel, mapping, units, range }` with channels rotation, translation, scale, deformation, color via colormap, emissive, visibility, flow particles, vibration. Joints (revolute, prismatic) declare axis and limits. The canvas samples stored simulation state at time t, interpolates deterministically, and scrubbing and replay use the same path. If only angular speed is given, integrate the angle from stored samples by a fixed rule. Show "Visual speed scaled 1:N" beside the real value whenever speed is scaled. A variable without a binding shows "No visual binding" and does not move.
Edits: geometry or parameter edits show an instant Draft preview (labeled Draft). Simulation results change only when a rerun completes: keep the previous result dimmed with Stale, show "Run N computing", then animate the new result. Provide a manual Rerun button first; auto rerun is a setting for fast solvers.
Mock mode: a deterministic mock solver in `src/mock` outputs the same schema as the API (time series plus bindings), so every motion you build is already driven by data.
Fields: temperature heatmap with a documented colormap and legend with units; flow particles from a vector field; section and explode tools.
Performance: 60 fps target on a mid laptop in High with the reference scene; at least 30 fps in Low; Auto tier switches by measured frame time; lazy-load the 3D chunk; instancing and LOD for repeated parts; record real numbers.

## 9. Data and state

TanStack Query style server cache (or equivalent already in repo) for engine data; a small UI store for selection, stage, time cursor, panel layout. Types in `src/types` match the API; one `src/api` client with typed errors and retry; `src/mock` implements the same interface. The URL encodes project, stage, selection and time. Autosave status shows Saved, Unsaved changes, Saving; failures are shown, never swallowed.

## 10. Interaction rules

Mouse: left drag orbit or select by tool; right drag pan; wheel zoom; double-click focuses. Keys: G move, R rotate, S scale, F focus, Delete, Ctrl Z, Ctrl Shift Z, Esc, Space play and pause, Left and Right step, Ctrl K palette. Drag from sources to canvas with ghost, snap, click to place, inspector opens. Every destructive action is undoable. Tooltips show shortcuts.

## 11. Accessibility and resilience

Keyboard path for every primary action; visible focus; WCAG 2.2 AA contrast; status by icon plus word; reduced motion; screen reader names for tree, tabs, timeline. Every view has loading (technical skeleton), empty (next action), error (what, why, fix, retry, details) and unknown states.

## 12. Testing and CI

Add a test runner if absent (Vitest and Testing Library, Playwright). Required tests: honesty (failing requirement never renders Verified; mock always tagged; "Candidate" wording; no value without unit), Proposal and Apply and Undo, prompt to model with rule-based services, binding (value change moves the node; no binding means no motion; scrubbing is exact and repeatable), stale result handling, keyboard navigation, visual regression screenshots for Hero, Home and each stage in dark and light, axe accessibility, performance budget script. Report actual numbers.

## 13. Milestones (stop for approval after each; show screenshots and a screen recording)

M0 audit and plan. M1 tokens and UI kit. M2 shell and Hero and Home. M3 canvas with real GLB, PBR, lighting, selection, gizmo, quality tiers. M4 Model stage and sources. M5 Define flow (prompt to model). M6 Simulate with bindings and motion driven by the mock solver; scrubber; freeze. M7 Understand and Improve (PREVIEW tags). M8 Verify and Evidence sheet (PREVIEW). M9 Observe and Build (PREVIEW). M10 polish, accessibility, performance, docs.
Acceptance for M6: change a load parameter, rerun, and the rotor visibly slows, the heat field changes, signals update, scrubbing is exact, and every number carries its unit and badges.

## 14. Do not

Build separate pages per screen; add libraries without the plan; rewrite to react-three-fiber unless the audit demonstrates a blocker; add built-in motor or battery parts; show AI confidence percentages; show Verified without data; fake deployments, quantum, or sensor data; use all-caps labels or decorative particles; touch engine crates; push to remote.

## 15. Report format at each milestone

Outcome, screenshots next to the matching wireframe, what is real vs MOCK vs PREVIEW, test output, measured fps and bundle size, deviations from the wireframes and why, open questions. Stop and wait.

## 16. Visual Direction v2 and navigation (overrides sections 3, 4 and 6 wherever they differ; read file 33 and file 34 images)

1. The user's taste is photoreal: the hero is always a real object in a real scene (glTF with PBR, HDRI background and lighting), never an abstract cube. Use the Realistic view for Simulate and Hero and the Engineering view (neutral studio with grid) for Model and Verify, with one toggle. The abstract cube is only the empty-state illustration.
2. Shell v2 (wireframes/12_shell_v2.png): left nav 176 px collapsible to icons (Studio, Define, Model, Simulate, Understand, Improve, Verify, Observe, Build PREVIEW) with per-item status; header card with thumbnail, project name, Active chip, "Model r3 · Saved 2 min ago", counts, state chips and the primary split button; left list panel with thumbnails; center scene with tool strip, HUD card, camera presets and Realistic or Wireframe toggle; right Component details and Environment; bottom four cards (Live telemetry, Graphs, Event log, Map and state), each expandable or replaceable. Violet Assistant button bottom-left. Hex-arrow logomark with "Engineering studio".
3. Primary action rule: one primary button top right, always the next best step (Run simulation, Investigate failure, Test this cause, Compare, Verify, Export evidence). A failed run flies the camera to the failed part, glows it red and shows a banner.
4. Define stepper (Intent, Requirements, Architecture, Components, Assemble) is shown only until model r1 exists. The architecture block diagram (wireframes/13) is the center view of step 3, before any 3D. Intent modes: Text, Reference, Blank now; Voice as PREVIEW.
5. Honest adaptations (file 33 section 2): no blanket Calibrated or Verified, no manual verify toggle, no confidence, no invented vendor data (source cards need a connected source and an as-of date), "Candidate" chains, generated proxy shapes tagged PLACEHOLDER MODEL, instrument widgets bound to variables (vehicle cluster comes from a domain pack, not core), flow streaks only from solved fields otherwise "Environment preview, not solved".
6. Use the 15 reference screens from file 34 as visual targets; the journey map (wireframes/14) is the navigation truth.
