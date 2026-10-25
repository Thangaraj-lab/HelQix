# 19. Phase 7 Prompt: Edge Observation, Agent Permissions, Quantum Pathway

Prerequisites: Gate 6 passed. Split into three independent sub-phases (7A, 7B, 7C); run one at a time, each with its own gate. Paste 13 first. Branches: `phase-7a-edge`, `phase-7b-agent`, `phase-7c-quantum`.

---

## 7A. Edge observation (sensor rig first, phone second)

Goal: real measurements enter HELQIX as Observed values, never overwriting Authoritative state, and feed validation.

Safety first (hard rules): any hardware using lithium cells must have fuse or current limiting, a temperature cutoff, a non-flammable enclosure or tray, and a written test procedure. The assistant must not design high-energy cell abuse tests. The founder decides the rig; software must enforce limits (see 08 for the draft rig and safety notes).

Tasks:
1. Ingest protocol: a documented schema for measurement streams (sensor id, quantity, unit, timestamp, sequence number, calibration reference, device id). Reject malformed, out-of-order beyond a window, or unit-ambiguous data with a typed error.
2. Observed channel: stored as Observed, with timestamps from the device and receive time kept separately; clock-offset handling documented.
3. Data quality: spike detection, dropout flags, saturation flags (for example, the INA219 limits: about 26 V bus, about 3.2 A with the common shunt, 12-bit; verify against the datasheet of the actual part chosen).
4. Replayable recordings: a recording of observations is itself a content-addressed artifact that can be re-fed.
5. Reconciliation (not overwrite): compare Observed vs Simulated; the difference feeds Phase 1 validation metrics and anomaly events. Never write observed values into Authoritative state automatically.
6. Phone tier: out of scope until the rig tier gate passes; document the sensor limitations only.

Required tests: malformed data rejected; out-of-order handling; recording round trip; observed value cannot overwrite authoritative (negative test); unit mismatch rejected; saturation flagged.

Gate 7A: recorded real rig data (founder-supplied) is ingested, compared to simulation, and a validation metric is produced and included in an evidence package. If no real data exists yet, the gate is NOT passed; synthetic data cannot pass it.

---

## 7B. Agent with permissions

Goal: an AI agent can read, analyze, plan and propose, but cannot change reality without validation and human approval.

Tasks:
1. Provider abstraction (interface only; provider chosen by the founder; check current API and pricing at build time, do not hard-code assumptions).
2. Permission ladder enforced in code, not in prompts: Read, Analyze, Plan, Propose, Simulate, Test, Execute, Mutate. Each tool call is checked against the session's granted level. Default is Read plus Analyze plus Propose. Execute and Mutate need explicit per-action human approval recorded in the audit log.
3. Proposal object: structured (intervention diff, rationale, expected effect, required validations). Agent output is Origin: AI-hypothesis, Status: Unverified until HELQIX validates it (run branch, check requirements).
4. Audit log: every agent action, input, output, approver, timestamp; append-only and hash-chained, included in evidence packages when the agent contributed.
5. Prompt-injection hygiene: tool outputs, file contents and model text are untrusted data and never grant permissions. Test with adversarial strings inside model notes and solver logs.
6. Cost and rate limits per session.
7. Do not describe this as ASI or AGI anywhere.

Required tests: attempt to call a Mutate tool at Propose level is denied; injection strings do not change permission; approval is required and logged; proposals are labeled AI-hypothesis; audit chain tamper detection.

Gate 7B: adversarial test-suite passes; a demo shows agent proposing a fix that HELQIX validates by branch simulation and a human approves, with a complete audit trail.

---

## 7C. Quantum pathway (honest and minimal)

Rule: quantum is a compute capability, used only if it gives a real advantage. Most engineering problems here do not need it.

Tasks:
1. Define the routing interface (CPU, GPU, quantum simulator, quantum hardware, external). Routing decision recorded with reason.
2. A single small, real example that is classically checkable (for instance a small optimization or sampling task) run on a simulator through a mainstream SDK. Compare against the classical answer.
3. Labeling: results are marked `quantum-simulator` or `quantum-hardware` with backend name, shots, noise model, and SDK version. A simulator result must never be labeled as hardware.
4. Hardware access via a provider is optional, needs the founder's account and budget, and queue/cost must be checked at the time. Do not claim a qBraid or Braket integration unless it was run (current register status: UNKNOWN / not evidenced).
5. Document in one page why or why not quantum helps for HELQIX use cases today (expected answer: mostly not yet; keep the interface, avoid claims).

Gate 7C: simulator example runs, compared to classical, labeled correctly, one-page assessment written. No hardware claims without hardware logs.

## Out of scope for all of Phase 7

Build Spec, research items.
