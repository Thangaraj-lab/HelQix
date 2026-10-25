# 20. Phase 8 Prompt: Build Spec and Research Track

Prerequisites: Gate 5 passed at minimum (Gate 7 not required for 8A). Research items (8B) are time-boxed and never block product work. Paste 13 first. Branches: `phase-8a-buildspec`, research in `research/<name>`.

---

## 8A. Build Spec (decision lineage)

Goal: connect Requirement -> Parameter -> Simulation -> Verification -> Build decision, so every design decision has traceable evidence. This is the bridge from "analysis tool" to "engineering record".

Tasks:
1. Data model: Requirement, Parameter, Simulation (run reference), Verification (outcome), Decision (who, when, rationale, status), with links. Reuse existing types; do not create parallel structures.
2. Traceability queries: "which decisions depend on this model revision?", "what is invalidated if this parameter changes?", "show evidence for decision D".
3. Impact analysis: when a model revision changes, mark dependent verifications as stale (not deleted).
4. Sign-off roles (author, reviewer, approver): minimal workflow; each approval is a signed record (reuse Phase 5 signing).
5. Export: Build Spec document (generated from records; every statement links to evidence ids). Interop (SysML v2, STEP, PLM/SPDM, FMI) is mapped as export/import design notes only; build none without a pilot request. Note that SPDM tools already provide traceability; HELQIX differentiator is executable, replayable evidence, not the traceability graph alone.

Required tests: stale propagation; trace queries on a reference project; unsigned approval rejected; export statements all resolve to existing evidence.

Gate 8A: a reference project shows a full lineage from requirement to decision with signed approvals and stale propagation after a model change.

---

## 8B. Research track (clearly labeled HYPOTHESIS; never marketed as product features)

Each item: write a one-page hypothesis, the falsifying experiment, success metric, and a time-box (suggest 2 to 4 weeks of part-time effort). Stop when the box ends.

1. HELQIXSIM: a determinism-first simulation kernel with formal reproducibility guarantees. Experiment: compare bitwise reproducibility under platform/compiler variations; measure the cost of enforcing it.
2. Tensor-Warp: tensorized batch simulation for many branches in parallel (GPU). Experiment: speedup vs CPU on 1,000 branch sweeps with the same results within tolerance. Kill if speedup under 5x or if tolerance cannot be met.
3. Generalized time-travel debugging for physical simulation: lineage-aware reverse stepping. Experiment: reverse-step and compare with stored snapshots.
4. Digital twin continuity: only after validation exists and real data streams; test drift detection against a held-out dataset.
5. Surrogate models (learned) with uncertainty, as accelerators: must be validated against the physics model; label as Derived/Unverified until validated.

Rules: research code lives apart from product code; no research claim enters docs as a capability until its experiment passes and a test proves it.

## Gate 8B (per item)

Hypothesis page, experiment result with raw numbers, decision (continue, park, kill). No result is "done" without the falsifier having been run.
