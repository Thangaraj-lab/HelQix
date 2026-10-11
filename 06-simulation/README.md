# Layer 06 — Simulation

Layer 06 owns numerical integration and model execution. It accepts an explicit model, initial state, logical interval, integration method, and resource bounds. It does not infer physics or certify that a model applies to a product.

## HELIXSIM core

The `helixsim` Rust crate provides:

- validated fixed-step timelines with a caller-declared step limit;
- explicit Euler and classical RK4 integration for caller-supplied ODEs;
- finite-value and state-dimension checks at model boundaries;
- model ID, model version, integration method, time samples, and bounded output in each successful result;
- an SI point-mass model for translational motion with gravity, constant applied force, and optional quadratic aerodynamic drag.

Point-mass state is `[x, y, z, vx, vy, vz]` in metres and metres per second. The model uses kilograms, newtons, a declared environment, drag coefficient, and reference area. Its trajectory is real numerical motion for those stated equations and inputs; it is not a rigid-body, full-vehicle, battery, structural, contact, collision, or flight model.

## Numerical boundaries

- Euler and fixed-step RK4 are explicit choices; HELIXSIM does not claim they are suitable for every model. There is no adaptive or stiff-system solver in this crate.
- The solver bounds interval count and retained output size. It rejects non-finite states, derivatives, or results rather than returning them as successful output.
- State vectors are caller-defined scalar channels. The generic solver cannot validate their physical units; the point-mass model validates its SI vector and force signatures.
- A model author remains responsible for equations, parameter provenance, validation data, uncertainty, and applicability. A successful integration is not itself engineering verification.
- HELIXSIM results are not yet connected to the Python tool gateway, the Layer 07 timeline, persisted project evidence, or Reality Studio playback. External adapters remain the model-backed workflow used by the current UI.

The Layer 06 crate has focused tests for bounded time grids, RK4 accuracy, deterministic endpoint sampling, invalid numeric data, force composition, gravity, drag, and point-mass motion. These tests must pass with the Rust toolchain before this layer can be considered verified.
