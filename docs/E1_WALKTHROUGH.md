# E1: browser walkthrough of the real-engine Studio (2026-11-05)

Setup: API built and run in WSL Ubuntu (debug build, PostgreSQL 18, local dev database), Studio dev server in real mode. Driven in the built-in browser. Labels: MEASURED = seen in this run.

## Flow checked (MEASURED)

| Step | Result |
|---|---|
| New project (real API, project id returned) | Works |
| Define a part, set density, Create model r1 | Works (the "Add part" mouse click did not register in the automation tool; a script click did: tooling, not confirmed as a product bug, UNKNOWN) |
| Simulate: Run my model (with density) / Run a free body | Works after fixes 1 to 3 |
| Freeze, Branch, play the branch | Works (branch run id returned, plays) |
| Understand, Improve, Verify | No crash; messages corrected (fixes 4, 5) |
| Verify: Determinism, evidence package | Works after fix 4 (replay re-reads 50 of 2001 frames) |

## Defects found and fixed (each: test red first, then green)

1. A stored run of a model without bindings was hidden behind "could not be read" (`runForModel`).
2. `SimulateStage` declared a hook after an early return: blank screen when the run arrived (render test).
3. A run of an unbound model showed no readings and an empty graph: now its own variables.
4. The replay check always asked for the mock run (404 on the real engine), so Verify said "No run to replay" and the evidence package had no run items. Now it uses the run's id and samples a real run (4 tests).
5. Improve said the real engine cannot fork a run (false: Simulate branches). Message corrected; a side-by-side verdict screen is E7.
6. A model could be committed with a part that has no density, giving an immutable revision that "Run my model" refuses. Define now warns before the commit (4 tests).

## Not done / still open

- Sign and export (E8), Observe and Build stages are PREVIEW by design.
- `useRun` still swallows a frame-load error into "could not be read" (review finding, skipped).
- The real-mode branches of SimulateStage are not covered by the hook-order test (it runs in mock mode).
- Mass from volume x density was not re-checked in the browser (no single-run endpoint); it is covered by the Rust tests in CI.
- Accessibility keyboard walk, mobile layouts: not done.
