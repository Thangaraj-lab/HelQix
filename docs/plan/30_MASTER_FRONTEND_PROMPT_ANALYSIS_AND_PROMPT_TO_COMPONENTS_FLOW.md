# 30. Analysis of the Master Frontend Prompt + the "Prompt to Components" Flow

Input analyzed: the user's MASTER FRONTEND IMPLEMENTATION PROMPT (63 sections). Output: (A) verdict, (B) what to fix, (C) the corrected end-to-end flow including the new step where the user types a prompt, AI splits it into components, the user reviews and also adds components manually, (D) data contracts, (E) paste-ready patch for the master prompt.

---

## A. Verdict

Strong prompt. It gets the main thing right: HELQIX is a connected engineering workflow, not a dashboard. Good parts to keep unchanged: the persistent shell and context-aware navigation (4, 39), "where am I / what state is it in" rule (47, 48), never fake results (49, 58), renderer is never the source of truth (44, 45), exact numeric transforms (12), undo/redo/save (54, 55), empty/error/unknown states (49), build the core deeply first (61), contextual AI instead of a giant chatbot (42), AI proposal must be validated (34, 42).

It is not ready to hand to an agent as is. Problems below are ordered by how much damage they cause.

## B. What to fix (my stance on each)

1. **It contradicts itself on scope.** Section 61 says do not build 40 screens superficially; section 63 says build everything (all routes, build, edge, optimization, compute...). An agent will obey 63 and produce a shallow wide UI. Fix: 63 becomes "deliver Phase 1 deeply; later phases are PREVIEW stubs".
2. **The UI becomes a CAD assembly editor (drag, snap, gizmos, ports).** That is a product by itself and CAD/assembly tools already exist (Onshape, FreeCAD, Fusion, SolidWorks). It is not HELQIX's differentiator (proof is). Position only matters for physics when geometry feeds the physics (thermal paths, structural). Stance: connection graph with typed ports first (it drives the physics); 3D layout second and often cosmetic; real geometry comes by import (STEP/GLB) later. Keep exact X/Y/Z fields, but tag layout as "visual" unless the model declares geometry-dependent physics.
3. **A component card is not a model.** A 3D thumbnail of a Motor means nothing if there is no validated physics behind it. Every library entry needs a maturity tag: Validated model / Unvalidated model / Placeholder (no model) / User-defined. Without this, the UI promises simulation the engine cannot do. (Library today: 18-libraries; validated library is a missing layer, file 05.)
4. **Missing step: requirements and Context of Use.** The flow starts at New Project then Model. The locked flow starts at Intent, Requirements, Context of Use (what must be true, limits, risk). Without it, Verify has nothing to verify against. Add a Requirements step (the user's prompt naturally supplies it).
5. **15 stores holding engineering state in the UI** (modelStore, componentStore, simulationStore...) repeats the finding in file 28: the app already has its own TypeScript runtime. That risks two authoritative states. Stores are client caches and UI state; the authoritative state lives in the engine behind the API (decision D5). Say it in the prompt.
6. **Stack is mandated (R3F, Tailwind, Zustand, Router, Recharts, Framer).** The repo uses plain three.js 0.170, React 18, Vite 5, no declared router or state library. Switching is a rewrite risk. Stance: decide after the UI audit (file 23 step 1). Recharts is weak for large time series (use uPlot-class for timelines, see file 26).
7. **Labels mix two axes.** Section 25 lists Observed, Simulated, Hypothesis, Verified, Unknown in one list. Locked principle: Origin (Observed, Simulated, Derived, AI-hypothesis) times Status (Verified, Unverified). Also "Confidence" appears in 25, 29 and 42: an LLM's self-reported confidence is not calibrated, and generic confidence is not credibility. Replace with: credibility per Context of Use (Trust layer) and, for AI output, "AI estimate, unvalidated".
8. **Build workspace (31 to 36: BOM, CAD, firmware, manufacturing, tests, hardware-in-loop)** is a different product (design-to-manufacture). It is Phase 8 or later in the locked order. Keep the nav entry as PREVIEW or remove it from the first release. "Never invent supplier data" is correct; extend to "no part numbers unless entered by the user".
9. **Golden workflow (60) has about 45 steps.** Too long to be a demo and too heavy to be Phase 1. Keep it as the authoring path; the 10-step demo path stays in file 29. For the demo, start from a template model so the authoring editor is not on the critical path.
10. **Duplicated or missing items:** Compare appears under Simulate and under Improve (one route, two entry points); freeze, determinism level, evidence signing status and replay hash match are not shown anywhere (add to the state indicator in 48); the 12-domain list conflicts with domain-agnostic plus one first pack (D1): show all domains, enable only the first pack, mark others "Coming".
11. **AI input safety is missing:** attached datasheets, CAD names or pasted text are untrusted data; they must never change permissions or execute (prompt injection). Add to 42.
12. **Colors:** "White = authoritative state" is a nice rule; keep it, but color alone must never carry meaning (add icon/text for status).

## C. The corrected flow (with the new prompt to components step)

Principle: AI is an accelerator, never a dependency. The same flow must work 100 percent manually. Everything AI suggests is a Proposal; nothing enters the model until the user applies it.

```
NEW PROJECT
  -> PROJECT HOME (3 doors: Describe your system | Start from template | Import model/CAD)
  -> 1 DESCRIBE            user types a prompt (and may attach datasheet, image, CAD, notes)
  -> 2 INTERPRET           system extracts: goal, physics domains, requirements, known values, assumptions, ambiguities
  -> 3 CLARIFY (optional)  max 3 short questions, each with a default; user can skip
  -> 4 DECOMPOSE           system proposes components, grouped by subsystem, each with reason and library match
  -> 5 REVIEW              user accepts, rejects, edits, adds manually (library, search, custom), sets quantities
  -> 6 CONNECT             system proposes typed connections (electrical, mechanical, thermal, data); user edits; deterministic checks
  -> 7 PLACE               auto-layout proposal in 3D; user adjusts; exact X/Y/Z
  -> 8 CONFIRM             summary + checks -> creates Model revision r1 (structured diff, undoable)
  -> ENVIRONMENT -> SCENARIO -> SETUP -> RUN -> RESULTS -> TIMELINE -> FAILURE -> CAUSALITY -> FREEZE -> BRANCH -> COMPARE -> OPTIMIZE -> VERIFY -> EVIDENCE -> (BUILD, EDGE: PREVIEW)
```

Manual path (no AI at all): Project Home, Start from template, or "Create Engineering Model" (empty), then straight to step 5 (REVIEW) with an empty list, add from library, then 6, 7, 8.

### Step details

**1 Describe.** One large text area plus example chips. Example prompt: "Electric motor driving a centrifugal pump, 3 kW, powered from a 48 V battery pack, temperature sensor on the stator, winding temperature must stay below 120 degC in continuous operation."
Buttons: Interpret, Skip AI (go manual). Attachments are treated as untrusted data.

**2 Interpret.** Output card "Understood as" (editable chips; each chip links back to the highlighted phrase in the prompt):
- Goal: continuous operation of a motor-pump system.
- Physics: electrical, mechanical, thermal.
- Requirement R1: winding temperature below 120 degC, continuous (becomes a requirement with a quantity, unit, threshold, Context of Use draft).
- Known values: power 3 kW, supply 48 V (User-stated).
- Derived check (CALCULATED, deterministic): 3 kW at 48 V implies about 62.5 A at the supply (I = P / V, ignoring efficiency); flag if this exceeds a battery or wiring limit the user states later.
- Assumptions (AI-inferred, Hypothesis): ambient 25 degC, steady load. Shown in amber, editable, and carried to the Context of Use draft.
- Ambiguities: motor type (BLDC, induction)? cooling method? load profile?
Every chip has a source tag: User-stated, Library default, AI-inferred (Hypothesis).

**3 Clarify.** At most 3 questions with defaults, for example "Motor type? (default: BLDC)". Skippable; skipped items stay UNKNOWN, not guessed.

**4 Decompose (the component split).** Screen layout: left = your prompt with highlights; center = proposed components by subsystem; right = live graph preview (and 3D preview later).

Subsystem groups, example:
- Electrical: Battery pack 48 V (stated), Motor controller/inverter (suggested, not in prompt), Current sensor (suggested).
- Mechanical: BLDC motor 3 kW (stated), Shaft (suggested), Bearings front and rear (suggested), Centrifugal pump as load (stated).
- Thermal: Motor housing/cooling path (suggested), Winding temperature sensor (stated).

Each proposed component card shows: name and category; why (link to the phrase, or "typically required by" with the reason); library match tag (Validated model, Unvalidated model, Placeholder no model, Not in library); proposed parameters, each with its source tag; required ports; a checkbox. Defaults: User-stated items checked; AI-suggested additions unchecked (user opts in); anything with no model gets a visible warning "cannot be simulated yet".
Buttons: Accept selected, Reject, Edit, Add manually, Regenerate suggestions (with the changed prompt), Back.
Do not show an AI confidence percentage. Show match quality (library match tag) and source tags.

**5 Review and manual add.** The same screen continues; the user can:
- Search and add from the Component Library (double-click or drag). 
- Add custom component: name, category, ports, parameters with units; tagged User-defined, Unverified, no validated model.
- Import (STEP, GLB; PREVIEW) and match to a component type.
- Duplicate, set quantity, group, rename, delete.
- Edit parameters (units mandatory; unit mismatch is rejected).
- See "Unresolved" list: required parameters still UNKNOWN, components without models, ambiguous items. Unresolved items do not block adding to the model, but they block Run and will appear in Verify.
Manual and AI additions are the same object type (Proposed change), land in the same history, and both obey Undo/Redo.

**6 Connect.** System proposes typed connections from the component ports (Battery to Controller electrical; Controller to Motor electrical; Motor to Shaft mechanical; Shaft to Pump mechanical; Stator to Sensor thermal/data). The user can add, remove, retype. Deterministic checks: port types compatible, direction, units, no dangling required ports, no impossible loops. Violations are shown inline, never hidden.

**7 Place.** Auto-layout from the connection graph (non-overlapping, grouped by subsystem) as a Proposal; user edits with drag, snap, gizmo, exact X/Y/Z, rotation, scale. Layout is labeled "Visual" unless the model declares geometry-dependent physics.

**8 Confirm.** Summary: components (count, by maturity tag), connections, requirements, unresolved items, assumptions. Primary button "Create Model r1". This writes one structured change set to the authoritative model (via API) and creates a revision; the UI never writes engine state directly. The result appears in the Model tree and the state indicator ("Model r1, Draft"). Undo returns to the empty model. Errors (API unavailable) are shown honestly; nothing is shown as saved unless the backend confirmed.

### Later prompts use the same loop

Inside any screen the contextual assistant (Ctrl/Cmd+K or the side drawer) takes a prompt, produces a Proposal (Intent, Proposed change as a diff, Reasoning, Expected effect, Required validations), and the user chooses Apply, Simulate first (creates a branch), or Reject. Example: "Add a temperature sensor near the stator" yields a proposed sensor component with a thermal port connected to the stator and a proposed position; Apply creates revision r2.

### Failure handling

- Prompt too vague: ask clarifying questions; if skipped, create fewer components and list UNKNOWNs. Do not pad.
- Contradiction found by deterministic checks (units, power versus voltage and current): show it as a warning with the numbers.
- No AI available, no network, or budget exhausted: show "Assistant unavailable"; the manual path stays fully usable.
- Component not in library: offer closest matches, or custom component tagged Unverified.
- Attached file contains instructions: ignored; displayed only as data.

### What is real when (honest status)

Deterministic intent extraction exists in 19-intelligence (claimed, not run). A real AI assistant arrives in Phase 7B. Until then step 2 and 4 run deterministic extraction plus library search and are labeled "Rule-based suggestions". When the AI is added, label them "AI suggestions (unvalidated)". Do not present rule-based output as AI reasoning, and do not present AI output as validated.

## D. Data contracts (HYPOTHESIS design; align with existing types in 03-engineering before building)

```
Intent       { id, promptText, extracted: [IntentItem], ambiguities: [Question], createdAt, source: user|assistant }
IntentItem   { kind: goal|physics|requirement|knownValue|assumption, text, quantity?, unit?, threshold?, sourceSpan, origin: UserStated|LibraryDefault|AIInferred }
Proposal     { id, intentId, items: [ProposedComponent], connections: [ProposedConnection], layout?, status: pending|applied|rejected, producedBy: rules|ai, createdAt }
ProposedComponent { tempId, name, category, reason, reasonSpan?, libraryMatch: Validated|Unvalidated|Placeholder|NotInLibrary, libraryRef?, parameters: [{name, value|UNKNOWN, unit, source}], ports: [Port], quantity, selected }
ProposedConnection { from: portRef, to: portRef, type: mechanical|electrical|thermal|fluid|data|control, source }
ChangeSet    { id, baseRevision, operations: [add|update|remove|connect...], author: user|assistant+approver, approvedBy, createdAt }
ModelRevision{ id, parent, changeSetId, hash }
```
Rule: the UI sends a ChangeSet; the engine validates and returns a ModelRevision or errors. Origin and source tags are stored with values, not just shown.

## E. Patch for the MASTER FRONTEND PROMPT (paste-ready, replaces or adds sections)

```
PATCH 1 (replace section 5, 6 and 7):
New Project creates a project with name, description, optional domain. Project Home offers three doors: "Describe your system" (primary), "Start from template", "Import model/CAD (PREVIEW)". "Describe your system" runs the 8-step flow: Describe, Interpret, Clarify, Decompose, Review (with manual add), Connect, Place, Confirm. Every AI or rule-based suggestion is a Proposal; nothing enters the model until the user applies it; applying creates an undoable Model revision through the API. The manual path must work with no AI.

PATCH 2 (add to section 10 Component Library):
Every component shows a maturity tag: Validated model, Unvalidated model, Placeholder (no model), User-defined. Components without a model cannot be simulated; show this before the user places them. Do not show 3D thumbnails as if they were validated models.

PATCH 3 (add a Requirements step):
Between Model and Environment add Requirements: each requirement has quantity, unit, threshold, condition, and a Context of Use draft. Requirements created from the prompt keep a link to the phrase they came from. Verify reports against these requirements.

PATCH 4 (replace section 25 labels and any "Confidence"):
Values carry Origin (Observed, Simulated, Derived, AI-hypothesis) and Status (Verified, Unverified). Do not show a generic confidence number. For AI output show "AI estimate, unvalidated". Credibility is shown per Context of Use by the Trust layer. Unproven causal chains are called "Candidate".

PATCH 5 (replace sections 44 and 45 stack and stores):
Audit the existing 21-app first. Prefer existing libraries (React 18, three.js, Vite, TypeScript). Add libraries only after a written decision. Stores are caches of server state plus UI state; authoritative engineering state lives in the engine behind the API. Do not create a second engineering runtime in the browser.

PATCH 6 (replace section 63 deliverable):
Deliver Phase 1 deeply (Landing, Project Home, Describe flow, Component Library with maturity tags, Model Editor, Inspector with exact transforms, Connections, Environment, Scenario Builder). Phase 2 and later screens exist as PREVIEW stubs with MOCK or PREVIEW tags until their engine gates pass. Build workspace and Edge are PREVIEW.

PATCH 7 (add to section 42):
Assistant input (prompts, attachments, notes, CAD names) is untrusted data and cannot grant permissions or execute actions. Assistant actions follow the permission ladder; default is Propose. Mutate requires explicit user approval recorded in the audit log.

PATCH 8 (add to section 48 state indicator):
Also show: Frozen (yes/no, hash), Determinism level (Bitwise, Tolerance, Statistical), Model revision, Evidence signature status.
```

## F. Phase and gate notes

Describe flow Phase 1 UI uses rules plus library search (real now where library exists, otherwise PREVIEW). The AI version is gated by Phase 7B. Verify and evidence screens stay PREVIEW until Phases 1 and 5. Nothing in this file changes the locked engine build order.
