# HELQIX Document Set (v1.0 locked definition, audit dated 2026-10-25)

Purpose: one place that holds everything decided, verified, wrong, and still open about HELQIX, so no conversation has to be repeated. Any person, any new chat, and Claude Code can start from these files.

## Reading order

| # | File | What it answers |
|---|---|---|
| 01 | PRODUCT_DEFINITION | What HELQIX is and is not. The 8 immutable principles. |
| 02 | ARCHITECTURE_LAYERS_AND_FLOW | The 10 locked layers, mapping to the repo's 23 folders, the locked flow, the Reality Canvas. |
| 03 | FEATURE_REGISTER_AND_MATURITY | Every feature (none cut) with its test-based maturity label today. |
| 04 | REPO_AUDIT_2026-10-25 | What the code actually contains: builds, tests, what is missing. |
| 05 | GAPS_MISTAKES_AND_FIXES | What is wrong or missing, including mistakes made in the discussion, and how to fix each. |
| 06 | BUILD_ROADMAP_AND_GATES | Locked build order, gates, kill criteria, parallel tracks, cadence. |
| 07 | BUSINESS_AND_GTM | Wedge, buyers (hypotheses), competitors, business model, risks. |
| 08 | OPEN_DECISIONS_AND_HYPOTHESES | What is NOT locked, decision log, domain scoring, validation rig, safety. |
| 09 | CLAUDE_CODE_PHASE0_PROMPT | Paste-ready prompt for Phase 0 (hygiene + runtime trace). |
| 10 | CLAUDE_CODE_PHASE1_PROMPT_TEMPLATE | Template for Phase 1, blocked until decisions are made. |
| 11 | VERIFICATION_SOURCES | Facts checked on the web, with links, and facts still unverified. |
| 12 | STATUS_BOARD | Weighted completion score, per layer, with the method. |
| 13 | COMMON_PREAMBLE | Rules block pasted BEFORE every phase prompt. |
| 14 | PHASE2_REPLAY_DETERMINISM_PROMPT | Freeze, exact replay, determinism levels. |
| 15 | PHASE3_CAUSALITY_PROMPT | Failure events, candidate causal chains, click-through. |
| 16 | PHASE4_BRANCH_COMPARE_PROMPT | Counterfactual branches, intervention tests, Compare. |
| 17 | PHASE5_SIGNED_EVIDENCE_CANVAS_PROMPT | Signed evidence, independent verifier, verification workspace. |
| 18 | PHASE6_ADAPTERS_PROMPT | Adapter contract, PyBaMM, CalculiX. |
| 19 | PHASE7_EDGE_AGENT_QUANTUM_PROMPT | 7A edge, 7B agent permissions, 7C quantum (each own gate). |
| 20 | PHASE8_BUILD_SPEC_RESEARCH_PROMPT | Build Spec lineage and time-boxed research. |
| 21 | HOW_TO_RUN_PHASES_AND_REVIEW | Run loop, decisions needed per phase, review checklist, red flags. |
| 22 | FOUNDER_TRACK_B_TOOLKIT | Interviews, pilots, prior art, rig, naming, funding (what Claude Code cannot do). |
| 23 | UI_QUALITY_SPEC_AND_PROMPT | High-quality UI bar, sequencing, review process, UI-track Claude Code prompt. |
| 24 | UI_SCREEN_IMAGE_PROMPTS | Style block and 16 image prompts to regenerate the uploaded screens, with honesty fixes. |
| 25 | ANTIGRAVITY_UI_BUILD_BRIEF | Build brief for Antigravity (21-app UI track). |
| 26 | UI_VISUAL_QUALITY_PACKAGES_AND_PROMPT | Recommended packages, visual recipe, performance budget, prompt for high visual quality. |
| 27 | FOLDER_ARCHITECTURE | Actual repo tree (verified), layer map, proposed additions, folder order by phase. |
| 28 | UI_LAYERS_FOLDER_ARCHITECTURE | UI-only layers (22-api, 15-sdk, 14-visual-runtime, 21-app): actual tree, screen mapping, proposed additions, new findings. |
| 29 | USER_WORKFLOW_PER_SCREEN | One user story across all 16 screens with example prompts, engine dependency, golden demo path. |
| 30 | MASTER_FRONTEND_PROMPT_ANALYSIS_AND_PROMPT_TO_COMPONENTS_FLOW | Critique of the 63-section master prompt, corrected flow with AI split-into-components step and manual add, data contracts, paste-ready patches. |
| 31 | UX_REDEFINITION_STUDIO_AND_PROMPT_TO_MODEL | One Studio, two doors, prompt to 6 briefs to model, feature coverage map, motion binding, real assets. Replaces page-by-page screens. |
| 32 | ANTIGRAVITY_MASTER_FRONTEND_PROMPT | Paste-ready master prompt for Antigravity with milestones and acceptance. |
| 33 | TASTE_ANALYSIS_AND_VISUAL_DIRECTION_V2 | Your taste read from the images, adopt/adapt/drop table, Visual Direction v2, Studio shell v2, navigation rules. |
| 34 | IMAGE_PROMPTS_V2_PHOTOREAL_SCREENS | 15 photoreal image prompts matching your taste. |
| 35 | CHATGPT_HANDOFF_ANALYSIS_AND_CORRECTED_END_TO_END_FLOW | ChatGPT plan analyzed (keep/mistakes), corrected pipeline P0 to P17, layers, live-build design, contracts, golden path and tests. |
| 36 | CLAUDE_CODE_PROMPT_NEW_PROJECT_TO_LIVE_STUDIO | Paste-ready Claude Code prompt M0 to M4: New Project, Prompt screen, live model build, bound-motion simulate. |
| 37 | ANTIGRAVITY_FULL_FRONTEND_REFACTOR_PROMPT | Self-contained Antigravity prompt: refactor existing screens to one Studio, New Project to Prompt screen to live build, visual direction, contracts, mock engine, milestones A0 to A7. Supersedes 32 where different. |
| 38 | FINAL_OPEN_SOURCE_STACK_RENDER_LADDER_AND_USER_TYPES | Final open-source tool stack, render ladder T0 to T3 (low-end to cinematic), user types, free-tier plan and cost model, prompts A (Antigravity), B (T2 stream spike), C (Blender pipeline). |
| 39 | CLAUDE_CODE_PROMPT_BUILD_THE_STACK | Paste-ready Claude Code prompt K0 to K9: tool register and license gate, dev stack, real API, adapters, frames and replay, signed evidence, Blender pipeline, stream spike, metering. Splits work from Antigravity. |
| wireframes/ | Not stored in git (about 10 MB of PNG); kept in the founder's helqix-docs zip. 11 layout diagrams (PNG) + generator source | Hero, Home, 7 Studio stages, overlays, flow map. |

Prompt order: 13 (always first) + 09 (Phase 0) -> 10 (Phase 1) -> 14 -> 15 -> 16 -> 17 -> 18 -> 19 -> 20. Never skip a gate.

## Labels used everywhere

- VERIFIED: checked against a primary source or by running code.
- CALCULATED: derived by a stated method.
- ASSUMPTION: believed but not checked.
- HYPOTHESIS: a bet that needs a test or interviews.
- UNKNOWN: not traced or not found.
- Maturity labels are test-based (see 03): Working, Partial, Stub, Broken, Claimed (audit pending), Not evidenced, Planned, Research.

## Context loader (paste at the top of any new chat)

> I am Raj, founder of HELQIX ("Engineering truth, with proof"), a domain-agnostic Engineering Executable-Reality Platform. The locked definition, architecture, build order, audit and open decisions are in the HELQIX document set (files 01 to 12). Rules: never fake results or maturity labels; label claims VERIFIED / CALCULATED / ASSUMPTION / HYPOTHESIS / UNKNOWN; do not change locked sections without evidence; the first domain pack (battery vs motor) is still undecided. Answer me in Tanglish. Be direct and take stances.

## Change control

Locked sections (files 01, 02, 06) change only with evidence: a failing test, pilot feedback, or a verified source. Opinion alone does not change them. Open items (file 08) update as evidence arrives. Every change is logged in the decision log in file 08.

## Language note

These files are written in English so they can be pasted directly into Claude Code and shared with engineers. Conversation with Claude stays in Tanglish.
