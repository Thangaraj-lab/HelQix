# 07. Business and Go-To-Market (all items are HYPOTHESES until pilots)

## Wedge

Failure root-cause and what-if analysis for a physics-coupled system (battery or motor thermal and electrical failure; choice open, see 08). Deliverable: an evidence report, not software access.

## Buyer and user hypotheses

- Buyers: R&D or reliability leads at EV and drone startups, battery and motor makers, university and industrial research labs, test and certification teams.
- Users: simulation engineers, reliability engineers, systems engineers.
- Job to be done: "Reproduce this failure, prove why it happened, and prove the fix works."
- Pain to test in interviews: time to reproduce a failure; how results are shared; what audit evidence they must produce.

## Positioning

Not a solver replacement. A system of record above or beside solvers: freeze, branch, compare, prove. Claim to test: "trust and verification become the scarce thing in the AI era."

## Competitors (VERIFIED where marked, else ASSUMPTION)

| Group | Names | Note |
|---|---|---|
| Simulation suites | Ansys, Siemens Simcenter, Dassault, MathWorks Simulink, OpenModelica | Strong solvers, validated physics, certification history (ASSUMPTION) |
| Platforms | NVIDIA Omniverse | Digital twin and visualization (ASSUMPTION) |
| SPDM (closest threat) | Ansys Minerva (powered by Aras), Aras Simulation Management, Teamcenter | Requirement tracking, version control, traceability, audit trails, digital thread (VERIFIED for Ansys Minerva and Aras, see 11; Teamcenter ASSUMPTION) |
| Standards | FMI/FMU | State save and restore exists but is optional and found mostly in commercial tools (VERIFIED, 11) |
| Workflow and ML tooling | MLflow, DVC, Git-based flows | Partial provenance (ASSUMPTION) |

Defensibility candidates: lineage and evidence data, validated model library, adapter ecosystem, reputation for never faking results, open adapters. Incumbent attack: they can add snapshot or branch features; defense is evidence ecosystem, speed, and being the neutral layer across solvers. Also prepare for AI surrogates making simulation cheap.

Novelty is NOT proven. Required: patent search and a feature-by-feature competitor check (08).

## Business model (to test)

Paid failure-analysis pilot with evidence report; per-seat workstation license; compute-credit pricing for GPU, external or quantum jobs; premium evidence and compliance package. Pricing is not set until pilot data exists.

## Unit economics to model (after pilots)

Customers x ARPU; gross margin; CAC; LTV; LTV to CAC; compute cost per run for CPU, GPU, external, quantum; support load; break-even seats. Do not publish numbers before data exists.

## Go-to-market

1. Pick one wedge and one domain pack.
2. Publish a validated reference case with real measured data (error bound shown).
3. Run 3 design-partner pilots; the report is the product.
4. Convert pilots to subscriptions.
5. Expand domains and adapters from pilot demand.

## Customer discovery (founder task, no code)

5 interviews with engineers who debug thermal or electrical failures. Ask: how long does reproducing a failure take; what tools; what evidence do regulators or customers require; what would they pay for; who signs off. Success test for the "trust is scarce" hypothesis: at least 3 of 5 describe reproducibility or traceability as a costly problem.

## Risks

| Risk | Mitigation |
|---|---|
| Scope too wide | Gates, one wedge |
| Model accuracy challenged | Validate before "Verified" |
| External solvers break determinism | Declared determinism levels |
| Storage and branch explosion | Delta storage, pruning |
| Incumbents copy features | Evidence ecosystem, neutral layer |
| Overclaiming (quantum, twin, time-travel) | Maturity labels and never-fake rule |
| Demand unproven | Pilot-first |
| Team capacity (founder does not code, review bottleneck) | External senior reviewer, strict test gates |
| Licensing, naming, trademark | Check early (08) |

## Funding and runway

UNKNOWN. Not planned in earlier discussion. Decide a monthly budget before buying a rig.
