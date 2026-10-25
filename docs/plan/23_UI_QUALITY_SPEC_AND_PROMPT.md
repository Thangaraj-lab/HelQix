# 23. UI Quality Spec and Claude Code Prompt (Reality Studio, 21-app)

Status: DRAFT for founder approval. Runs as a parallel UI track; it never overrides engine gates. Paste 13 (preamble) first.

## Principle

The UI is a window onto the engine, not a second source of truth. High quality means: trustworthy, fast, clear, and beautiful, in that order. A beautiful screen that shows an unverified number as Verified is a defect, not a feature.

## Quality bar (measurable where possible)

1. Honesty: every value shows Origin x Status badge; Verified appears only when Trust criteria pass; "candidate" wording for unproven causes. (UI test enforces.)
2. Performance: target 60 fps on a mid laptop for the reference scene; time-to-first-render under 3 s on local; scrubbing the timeline never blocks the UI thread for more than 50 ms. These are TARGETS (ASSUMPTION); measure and record actuals.
3. Clarity: one primary action per screen; the engineer can answer "what failed, why (candidate), what did I change, what happened" within 30 seconds on the reference scenario (test with 3 real users; HYPOTHESIS).
4. Accessibility: keyboard navigation for all primary actions, visible focus, contrast meeting WCAG 2.2 AA, no color-only meaning (status also by icon/text), reduced-motion respected.
5. Resilience: loading, empty, error, partial-data and disconnected states designed for every view. No blank screens, no silent failures.
6. Visual consistency: one design-token set (color, type, spacing, radius, elevation, motion), light and dark, used everywhere. No one-off styles.
7. Rendering: Three.js with WebGL2 as the baseline; WebGPU optional and feature-detected (experimental). Charts are 2D, crisp, with units on every axis.
8. Responsiveness: desktop first (engineers use laptops/monitors); tablet usable; phone only for read-only status. Do not promise full phone editing.

## Information architecture (5 modes, one shared state)

Reality (3D + live values) / Time (timeline, checkpoints, freeze markers, scrub, replay) / Causality (chain, node detail drawer) / Branch (tree, interventions, Compare) / Verification (requirements, Context of Use, credibility, uncertainty, determinism level, Export evidence). A persistent header shows: model revision, run id, determinism level, trust status. Mode switch never loses selection or time position.

## Key screens and components

Run header; timeline scrubber with event/failure/freeze markers; value inspector with badges; requirement panel (pass/fail/margin); causal chain view with node drawer (equation, parameters, previous/current value, observation, snapshot link); branch tree and side-by-side Compare (overlaid curves, delta table, first divergence marker); evidence export dialog (what is included, signature status, verifier command); audit/activity panel; settings for units.

## Sequencing (do not build ahead of the engine)

- Now (parallel to Phase 0): design tokens, component library, screen skeletons using real API shapes or clearly marked mock data (MOCK badge visible). No claims.
- With Phase 3: Causality view. With Phase 4: Branch + Compare. With Phase 5: Verification workspace + polish pass + accessibility audit + performance pass.
- Anything shown before its engine gate passes is labeled PREVIEW in the UI.

## Process for quality (since the founder does not code)

1. Design first: produce screen designs (use the Design artifact type for visual options) and get founder approval before implementation.
2. Implement from approved designs against the token set.
3. Review by running the app and recording a screen walkthrough; compare to design.
4. Automated checks: component tests, accessibility checks (axe-style), visual regression screenshots on key screens, and a performance budget check in CI.
5. User test with 3 engineers (Track B) on the reference scenario; record task success and confusion points.

## Claude Code prompt (UI track)

Role and rules: as in 13. Scope: 21-app only; do not change engine crates except through documented API additions requested by a gate.

Tasks:
1. Audit current app: build, run, list what screens work, what is mock, what is broken, with screenshots and console errors. Record in docs/UI_AUDIT.md. Do not fix yet.
2. Create docs/UI_TOKENS (design tokens file) and a minimal component library (Badge for Origin x Status, Value, Panel, Timeline, Chart, Drawer, EmptyState, ErrorState) with tests.
3. Implement the persistent run header and mode switcher with shared selection and time state.
4. Implement states (loading, empty, error, disconnected) for each existing view.
5. Add UI tests: a failing requirement never renders Verified; unproven chain never says "root cause"; MOCK data always badged.
6. Add accessibility checks and a performance budget check to CI; report actual numbers.
7. Stop at the UI gate: audit doc, screenshots, test and CI output pasted. Do not start engine-dependent views until their gates pass.

UI Gate: audit complete, tokens and components merged with tests, state coverage complete for existing views, honesty tests pass, accessibility and performance numbers recorded.

## Open decisions (founder)

U1 approve design direction (after seeing options). U2 dark-only vs light+dark (recommend both). U3 brand identity link to HelQix brand work (film project) so the product matches. U4 who does visual design review.
