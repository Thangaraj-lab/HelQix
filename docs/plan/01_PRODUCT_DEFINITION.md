# 01. HELQIX Product Definition (LOCKED v1.0)

## Identity

- Name: HELQIX. Do not use "HELIX" (the code and some docs still do; see 05).
- Category: Engineering Executable-Reality Platform.
- Tagline: Engineering truth, with proof.
- One sentence: HELQIX turns an engineering model and its requirements into one authoritative, deterministic, observable, replayable and branchable reality, validates it against measured data, and delivers every conclusion with signed provenance and a stated confidence.
- Promise: Reproduce an engineering failure exactly, test the fix in a branch, and prove it with an evidence report.
- The paying moment: "Failure reproduced, fix proven in a branch, signed evidence in hand."

## What HELQIX is

The engineering reality layer that sits above solvers and tools. It owns the model, state, scenario, execution identity, lineage, verification and evidence. External tools provide specialized capability only.

## What HELQIX is not

- Not a simulator, a CAD replacement, a CFD replacement, a 3D viewer or a dashboard.
- Not an AI chatbot. AI proposes; HELQIX validates; a human approves.
- Not an EV, drone, robotics or quantum product. Those are reference systems or compute pathways.
- Not a digital twin until a model is validated against measured data.

## Why this matters (new scarcity hypothesis)

HYPOTHESIS (not yet validated): when AI makes design, code and simulation cheap, the scarce thing becomes proof that a result is true and reproducible. Trust, verification and lineage become the bottleneck. HELQIX sits on that bottleneck. Needs 5 engineer interviews to confirm (see 08).

## Domain-agnostic rule

The core must stay generic: System, Component, Property, Constraint, Environment, Physics, Scenario, State, Observation, Failure. Battery, motor, EV, drone are "domain packs" supplying physics models, parameters, validation data and adapters. No domain-specific logic in core layers. The first domain pack exists only to prove the platform (see 08 for the open decision).

## The 8 immutable principles

1. One authoritative engineering state. The 3D scene, AI output, external solver output and edge data are never automatically truth.
2. Three states: Authoritative (computational truth), Derived (display only), Observed (physical measurement). Observed data never silently overwrites authoritative state; it enters through the observation and validation path.
3. Two-axis label on every value: Origin (Observed, Simulated, Derived, AI-hypothesis) crossed with Status (Verified, Unverified).
4. Every run declares a determinism level: Bitwise (core runtime), Tolerance (external solvers), Statistical (stochastic or quantum).
5. Credibility is contextual. Each evidence package carries Context of Use, risk level, and credibility achieved. There is no blanket "Verified".
6. Causal claims have two grades: model-causal (branch intervention inside the model) and validated-causal (only after the model is validated against measurement).
7. AI proposes, HELQIX validates, a human approves. Permission ladder: Read, Analyze, Plan, Propose, Simulate, Test, Execute, Mutate. Risk decides how much approval is needed.
8. Never fake. No fake simulation results, quantum execution, state, evidence or maturity labels. A simulator is labelled a simulator; a QPU is labelled a QPU. Quantum advantage is a research question, not a product claim.

## Verification vs validation (kept separate)

- Verification: is the model solved correctly (code correctness, numerical error, deterministic replay, constraint checks, consistency with external solvers).
- Validation: does the model match physical reality (comparison with measured data, held-out split, uncertainty bounds).
- Calibration and validation must use different data. Optimizing on an unvalidated model yields "model-optimal, not yet real-optimal" and must be labelled so.

## Determinism is not correctness

A wrong model replayed perfectly reproduces the wrong answer. Determinism supports reproducibility and evidence; correctness comes from validation.

## Trust layer (the moat)

Requirements and Context of Use, numerical integrity, benchmarks, verification, calibration, validation, uncertainty and sensitivity, hashed and signed tamper-evident evidence, human sign-off. See 02 (layer 8) and 06.

## Branch = controlled experiment

A branch changes one thing from a frozen state and re-executes. This is an intervention, so causal claims can be tested rather than inferred from correlation. Grade: model-causal until the model is validated.

## Closed loop

Intent, requirements, model, simulate, understand, verify, validate, evidence, then either a Report path (analysis users) or a Build Spec path (physical product), then edge observation, calibration, and back to the model. The Build path is optional; many users only need the report.
