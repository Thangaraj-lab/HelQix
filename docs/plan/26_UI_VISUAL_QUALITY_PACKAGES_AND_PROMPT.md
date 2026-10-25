> CORRECTION (file 35, decision N2): the React Three Fiber recommendation below is superseded. The repo uses plain three.js 0.170; keep it and decide on any new package only after the Phase 0 audit.

# 26. High-Visual-Quality Packages and Prompt (for Antigravity or Claude Code, 21-app only)

Paste after file 13, with files 23, 24, 25 and the screen images. Rule from file 23 still wins: honesty before beauty, performance budget before effects.

## A. Recommended stack (my judgement; VERIFY current versions, licenses and maintenance at build time, I did not check today's versions)

Pick what the repo already uses first. The audit says React + Three.js; whether it uses react-three-fiber is UNKNOWN. Do not rewrite to switch libraries unless the UI audit shows a blocker.

| Need | Package | Why | Watch out |
|---|---|---|---|
| 3D scene in React | three, @react-three/fiber, @react-three/drei | Standard React way for Three.js; drei gives controls, environment, helpers, text, Html overlays | Keep scene objects few; use instancing |
| Glow, depth, polish | @react-three/postprocessing (Bloom, SelectiveBloom, Vignette, SSAO sparingly) | The glow look in the screens | Costly on weak GPUs; provide quality tiers and a "Low effects" toggle |
| Glass and holographic look | Custom shaders / MeshPhysicalMaterial (transmission) via drei MeshTransmissionMaterial | Translucent engineering models | Transmission is expensive; use only on the hero model |
| Heatmaps, fields, particles | Custom GLSL, Points/InstancedMesh, GPU textures | Thermal maps, flow streams | Values must come from the data, not decoration |
| Real engineering models | glTF/GLB via drei useGLTF with Draco/meshopt compression | Fast loading | Model conversion from CAD is a separate task |
| Text in 3D | troika-three-text (via drei Text) | Crisp labels | Keep counts low |
| UI primitives | Tailwind CSS + Radix UI primitives (shadcn/ui as a starting kit) | Accessible, themeable, fast to build | Customize heavily or it looks generic |
| Icons | lucide-react (or phosphor) | Consistent line icons like the screens | One icon family only |
| Animation (UI) | motion (Framer Motion) | Panels, tabs, drawers, shared layout transitions | Respect prefers-reduced-motion |
| Timeline choreography | optional: GSAP or Theatre.js for cinematic landing only | Landing/entry polish | Check license terms; do not use for core UI |
| Time-series charts (large data) | uPlot (very fast) for live/timeline; visx or D3 for custom charts | Smooth 60 fps scrubbing on many points | Recharts is fine for small static charts but slower on big data |
| State | zustand | Small, fast shared state (time position, selection, mode) shared by 3D and 2D | One store, no duplicated truth; engine state stays authoritative on the backend |
| Data fetching | TanStack Query | Caching, retries, loading/error states | Cache must never present stale data as live |
| Virtual lists | @tanstack/react-virtual | Large event/log lists | |
| Fonts | Inter (UI), JetBrains Mono (numbers/code); self-host | Clean technical look, tabular numerals | Use tabular-nums for values |
| Performance tooling | r3f-perf (dev), Lighthouse, Playwright screenshots, axe-core | Measure fps, accessibility, visual regression | Numbers must be recorded in docs |
| Testing | Vitest + Testing Library, Playwright | Honesty tests and visual regression | |

Not recommended now: heavy game engines, physics engines in the UI, a second charting library per screen, WebGPU as the only path, many ad hoc CSS-in-JS libraries.

## B. Visual recipe (how to get the look without hurting performance)

1. Background: layered CSS gradient (deep navy) plus a single low-cost animated gradient or static noise texture; stars/particles only on the landing screen.
2. Glass panels: backdrop-filter blur on few large panels only (backdrop-filter is costly); inner 1px border with a blue gradient; soft glow via box-shadow tokens.
3. Hero model: translucent shell (physical material, low opacity), inner components with emissive color, thin edge lines (edges geometry), a reflective circular platform (cheap fake reflection using a gradient disc, not real-time reflections), Bloom only on emissive parts via selective bloom.
4. Data-driven color: heatmap color comes from the actual field values through a defined color scale (document the scale and units). No decorative heat.
5. Motion: 150 to 250 ms eases for UI, slower camera easing for the canvas; one motion token set; disable on reduced motion.
6. Number display: tabular numerals, units always, Origin x Status badges, never animate a number in a way that implies a value that was not computed.
7. Quality tiers: Auto / High / Low. Low disables bloom, transmission, particles and uses simple materials. Auto picks by measured frame time.
8. Budgets (ASSUMPTION, measure): 60 fps on a mid laptop with integrated or entry GPU on the reference scene in High; at least 30 fps in Low; JS bundle split by route; 3D chunk lazy-loaded; first render under 3 s locally.

## C. The prompt

ROLE: Senior front-end and real-time graphics engineer. Scope: 21-app only; no engine changes. Follow file 13 rules and file 23 honesty rules.

TASKS
1. Audit: list the current UI stack, versions, bundle size, fps on the reference scene, and which of the packages above are already present. Output docs/UI_STACK_DECISION.md with keep/add/avoid per row of table A and the reason. Do not install anything before this is approved.
2. Install only approved packages, pinned, with license check recorded. Remove nothing unrequested.
3. Implement the design tokens and glass/glow component kit (Panel, Badge, Value with units, Tabs, Drawer, Slider, Toast).
4. Implement the HeroScene component: translucent model shell, emissive internals, edge lines, circular platform, controlled bloom, quality tiers (Auto/High/Low), camera easing, selection highlight. Use a procedural or supplied GLB model; mark placeholder models with a visible PLACEHOLDER MODEL tag.
5. Implement data-driven overlays: heatmap from a scalar field with a documented color scale and legend with units, flow particles driven by a vector field, failure marker linked to the timeline.
6. Implement the timeline scrubber with uPlot-class performance: 100k points scrub without dropping below 30 fps in High (measure and report), event markers, freeze markers, keyboard control.
7. Implement the mode switcher (Reality, Time, Causality, Branch, Verification) with shared state in one store; switching never loses selection or time.
8. Accessibility and reduced motion; contrast check; keyboard navigation.
9. Tests: honesty tests (failing requirement never Verified; mock always badged; "Candidate root cause" wording), visual regression screenshots for each finished screen in both themes, performance check script that records fps and bundle size.
10. Report with before/after screenshots, fps numbers on named hardware, bundle sizes, and a list of effects you disabled in Low and why. Stop after each numbered step for approval.

DO NOT: add decorative data, fake results, unbadged mock values, huge dependencies without approval, or effects that break the performance budget. If a visual effect conflicts with the budget, keep the budget and report it.

## D. Founder review checklist (no code needed)

1. Run it on your own laptop. Does it feel smooth? Record screen. Try Low quality.
2. Compare each screen side by side with the reference image; note differences.
3. Check badges: every number has a unit and Origin x Status. Anything MOCK says MOCK.
4. Open on a second, weaker device. If it stutters in Auto, performance work is not done.
5. Ask 3 engineers (Track B): can you tell what failed and what is proven within 30 seconds?
