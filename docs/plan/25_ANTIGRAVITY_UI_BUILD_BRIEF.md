# 25. Antigravity UI Build Brief (paste after file 13 preamble; attach the screen images)

Scope: 21-app (Reality Studio) only. Engine crates are not modified. Read files 13, 23, 24 first. If the repo has docs/ENGINEERING_RULES.md, read that too.

## Task

Rebuild the attached HELQIX screens as a real, working UI that matches the reference images in look and layout, with honest data handling.

## Rules

1. Reference images are visual targets. Rebuild text and data in code; do not embed screenshots as UI.
2. Data: use the real API or gateway where it exists. Where it does not, use a clearly named mock layer (src/mock) and show a visible MOCK or PREVIEW badge on any screen or value using it. Never show mock values as Verified.
3. Every value renders Origin x Status badges. Determinism level chip on every run header. "Candidate root cause" wording. A failing requirement can never render Verified (add UI test).
4. Design tokens first (colors, type, spacing, radius, glow, motion), light and dark. Components: Sidebar, TopBar, Panel, Badge, Value, Slider, Timeline, Chart, Drawer, Tabs, EmptyState, ErrorState, Toast.
5. 3D: Three.js with WebGL2 baseline; WebGPU only behind feature detection. Procedural stand-in models are fine; no fake claims.
6. States for every screen: loading, empty, error, disconnected. Keyboard navigation, visible focus, WCAG 2.2 AA contrast, reduced motion.
7. Performance targets (ASSUMPTION, measure and report actuals): 60 fps reference scene, first render under 3 s.
8. Minimal dependencies; list every added package with reason and license.
9. No secrets in git. No pushing. Small commits.

## Build order

1. Audit current 21-app, write docs/UI_AUDIT.md (what runs, what is mock, errors). Do not change anything.
2. Tokens and component library with tests.
3. Shell: sidebar, top bar, breadcrumb, mode tabs, run header.
4. Screens in this order: 03 Workspace, 06 Run, 07 Results, 09 Compare, 11 Timeline, 08 Failure/Causality (PREVIEW until Phase 3), 14 Verify (PREVIEW until Phase 5), then 02, 04, 05, 10, 12, 13, 15, 16, 01.
5. Tests: honesty tests, component tests, visual regression screenshots of key screens, accessibility and performance checks.

## Report format

Screens done vs images (side-by-side screenshots), what is real vs MOCK, test outputs, accessibility and performance numbers, deviations from the images and why. Stop at the end of each numbered step and wait.

## Review (by a different agent)

Have a second agent attempt to find any place where mock data appears without a badge, or a failing state appears Verified.
