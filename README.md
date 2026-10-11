# HELQIX — Engineering Reality

HELQIX is a domain-extensible engineering workspace for creating traceable digital realities of engineered products and systems, connecting their models and tools, exploring controlled alternatives, inspecting behavior, and verifying decisions with reproducible evidence. It is an engineering platform, not a product-specific simulator. The current demonstrator is an electric vehicle and battery study; the shared 23-layer architecture is intended to support other engineered products through domain packs and adapters.

Read the [HELQIX product definition](docs/PRODUCT_DEFINITION.md) for the product boundary, canonical workflow, responsibility of each layer, acceptance criteria, and current implementation gaps.

This repository currently contains a React/Three.js application, a local Python API and engineering-tool gateway, and Rust libraries for selected runtime layers. The local app demonstrates saved accounts/projects, component-based engineering realities, vehicle and battery inputs, a bounded battery model, recorded-result playback, and direct tool adapters. These pieces do not yet form a complete end-to-end engineering workflow.

## Run the local product

Use two PowerShell terminals from the repository root.

**Terminal 1 — API and tool gateway** (Python 3.11+ recommended):

```powershell
& .\17-integrations\gateway\install.ps1
& .\17-integrations\gateway\run.ps1
```

The gateway listens on `http://127.0.0.1:8091`. It stores local account, project, and integration data in `data/integration_runs.sqlite3`.

**Terminal 2 — Reality Studio UI** (Node.js 20+):

```powershell
cd .\21-app\reality-studio
npm install
npm run dev -- --host 127.0.0.1 --port 5178 --strictPort
```

Open `http://127.0.0.1:5178/`, create a local account, then create a project. In Reality Studio, define the system components, save the explicit vehicle/battery model and drive cycle when applicable, run the configured analysis, inspect the recorded result, and review traceable requirements. Model actions are available only when their declared inputs and required tool are present.

## Current engineering boundaries

- Intent extraction is deterministic and reports only explicit values with their source text. It is not an AI engineering review.
- Generic component geometry is a provisional visual proxy, not linked CAD. The recorded vehicle trajectory is a proxy driven by the saved speed trace's measured distance output.
- A saved configuration or scenario is not a simulation run and does not produce a performance recommendation by itself.
- One scoped model-backed workflow is available for a saved battery cell: PyBaMM SPM / Chen2020 can run a baseline or a revision-matched branch, the UI can compare retained voltage curves and scalar outputs, and explicit numeric requirements can be checked against the exact source revision. The EV drive-cycle adapter uses that single-cell model with identical-cell scaling and a prescribed speed trace; it is not a general battery-pack or free-running vehicle simulation.
- Layer 12 can run a bounded two-to-five-point sweep of cell current through that same real solver, optionally enforce a minimum discharge duration, and persist all candidate branches, run IDs, measurements, and the best evaluated candidate for the declared objective. It is an enumerated cell-level search, not general optimization or a product recommendation.
- Tool adapters invoke external programs when installed and configured. They report unavailable tools instead of fabricating results.
- The Python API is a local development service. Production deployment still needs a managed database, HTTPS, secrets management, rate limits, backups, operational monitoring, and deployment configuration.
- Rust crates for Layers 01–16, 20, and 22 are declared in the root Cargo workspace. The Layer 06 HELIXSIM kernel includes bounded Euler/RK4 integration and an SI translational point-mass model with gravity and quadratic drag. The native Windows workspace build remains unverified because the Windows SDK libraries required by the MSVC toolchain are missing from this environment. Layer 20 provides in-memory access-policy, bounded job-lease, and audit-log primitives; durable distributed services are still required. The repository is not a complete, verified 23-layer engineering platform.
- Reality Studio builds as a production React/Three.js bundle. Layer 14 renders typed component proxies in Build mode and a scrub/play 3D vehicle proxy driven by recorded drive-cycle distance; these visuals are explicitly not CAD geometry or a full-product physics simulation. The production bundle currently emits a warning for the large Three.js chunk.

See [Layer 17 integration setup](17-integrations/README.md) and [Layer 19 intent extraction](19-intelligence/README.md) for details.

For an isolated cross-layer HTTP and Blender verification run, see [Layer 23 validation](23-validation/README.md).
