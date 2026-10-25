# 18. Phase 6 Prompt: External Adapters (PyBaMM and CalculiX first)

Prerequisites: Gate 5 passed. Paste 13 first. Branch: `phase-6-adapters`.

## Goal

External solvers become capabilities HELQIX orchestrates under one adapter contract, with honest determinism levels, recorded tool identity, and results flowing through the same state, evidence and verification path. HELQIX owns identity, state and provenance; the tool only computes.

## Background to verify

16-adapters has 4 passing tests; PyBaMM and Blender are referenced; the PyBaMM SPM/Chen2020 workflow is README-claimed and was not run. Confirm what exists before writing anything.

## Adapter contract (define once, reuse)

An adapter declares: capabilities, supported model fragment types, input schema, output schema, tool name, tool version, determinism level it can honestly offer (usually Tolerance or Statistical), required environment, resource needs, and failure modes. Every invocation records: input hash, configuration, tool version, wall time, output hash, exit status, and stderr summary. Tool output is untrusted data: validate against the output schema before it enters state; it enters as Derived (Origin: Simulated), never Authoritative without the Trust path.

## Tasks

1. Adapter contract in code (trait or interface) and a contract test-suite any adapter must pass (a conformance kit).
2. PyBaMM adapter: map the HELQIX battery model fragment to a PyBaMM model, run, convert outputs to HELQIX quantities with units. Pin versions. Record solver options. Declare level Tolerance and measure the actual run-to-run variation; do not assume it is zero.
3. CalculiX adapter (structural/thermal-structural): input deck generation from a HELQIX model fragment; run; parse results. Check the installation and licensing terms before bundling anything; do not ship third-party binaries without checking the license (record the result).
4. Failure handling: tool missing, timeout, crash, malformed output, NaN/Inf, unit mismatch. Each produces a typed error; never a silent partial result.
5. Sandboxing: run tools in a controlled working directory with resource limits; no shell string concatenation with user data.
6. Cross-check: for at least one problem with a known analytic or benchmark answer (from Phase 1 benchmarks), run both the native kernel and the adapter and compare; record error.
7. Other adapters (OpenFOAM, Blender, FreeCAD, Rapier, ROS 2, Gazebo, PX4, FMI/FMU) are NOT built here. Add stub entries to the register as "Planned" only. FMI: note that state save/restore is optional and often commercial, so FMU co-simulation may not support exact freeze/branch; adapters must declare `supports_state_restore` honestly.

## Required tests

- Conformance kit passes for each adapter.
- Missing-tool and malformed-output tests produce typed errors.
- Unit mismatch rejected.
- Provenance record complete and hash-consistent.
- Variation measurement: N repeated runs, report max deviation; set the declared tolerance from it with margin, and justify the margin.
- Mutation sanity check as in the preamble.

## Gate 6

Both adapters run a reference problem end to end into the evidence package (Phase 5) with correct level labels, cross-check error recorded, failure-handling tests pass, license notes written, STATUS updated.

## Out of scope

Edge hardware, agent, quantum, additional adapters.
