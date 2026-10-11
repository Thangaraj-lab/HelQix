# HELQIX engineering tool integrations

The gateway exposes an HTTP boundary for the HELQIX Reality Studio UI and calls the installed engineering tools directly. It reports missing tools honestly; an unavailable connector cannot return fabricated simulation data. Completed outputs and failed attempts are saved in a local SQLite run history at `data/integration_runs.sqlite3`, with input and tool provenance. Failed attempts include a failure stage and bounded tool logs where available; the UI keeps these records inspectable without treating them as engineering results.

## Start locally

From PowerShell at the repository root, install the isolated gateway runtime once, then start it:

```powershell
& .\17-integrations\gateway\install.ps1
& .\17-integrations\gateway\run.ps1
```

The development gateway binds to loopback at `http://127.0.0.1:8091`. Install its Python requirements with `python -m pip install -r 17-integrations/gateway/requirements.txt` if FastAPI or Uvicorn is missing. Set `HELQIX_INTEGRATION_API_KEY` for machine-to-machine integrations and put the gateway behind a trusted server-side proxy; do not embed a production API key in the browser. When configured, the key authorizes tool access but does not identify an account: any operation that references a saved project or reads project run history also requires that project owner's authenticated session. Machine-only run history is restricted to ownerless runs. Set `HELQIX_ALLOWED_ORIGINS` to the exact UI origins in production.

The same service currently provides the local account and project API used by the UI: `POST /v1/auth/signup`, `POST /v1/auth/login`, `GET /v1/auth/me`, and `POST /v1/auth/logout`. It stores only a salted password hash, puts the opaque session in an HttpOnly cookie, and scopes project records to that account. Generic project resources use versioned `If-Match` updates and optional idempotency keys. This local SQLite-backed service is for development and single-host evaluation; production deployment still needs operational controls such as HTTPS, managed secrets, backups, migrations, rate limiting, and a production database.

## Tool adapters and operations

| Adapter | Operation | Required input | Actual tool invoked |
|---|---|---|---|
| `freecad` | `inspect_step` | `asset_path` within workspace | FreeCADCmd macro imports STEP and measures shape |
| `blender` | `create_quadrotor` | saved `project_id`, new `output_path` within workspace | Gateway loads the user's saved project specification; Blender builds a `.blend` parametric design scene from that authoritative record |
| `blender` | `inspect_asset` | `asset_path` within workspace | Blender background loads supported mesh/scene and reports mesh metrics |
| `pybamm` | `simulate_discharge` | `current_a`, `cutoff_voltage_v` | Single-cell Python PyBaMM SPM + Chen2020 benchmark |
| `pybamm` | `simulate_bound_cell` | owned `project_id`; optional saved `binding_id` and revision-matched `branch_id` | Runs PyBaMM SPM / Chen2020 from the saved cell's explicit current and cutoff properties; preserves a bounded voltage/capacity time series and model digest |
| `openfoam` | `run_case` | `case_dir`, optional `solver`, `timeout_s` | `blockMesh` then allowlisted OpenFOAM solver |
| `px4_gazebo` | `start_sitl` | optional allowlisted `model` | `make px4_sitl <model>`; sessions can be listed and stopped |

All file and case paths must resolve inside `HELQIX_WORKSPACE_ROOT`. PX4 requires `HELQIX_PX4_DIR` to point to a PX4 source checkout, and `make` plus modern Gazebo (`gz`) on PATH. The UI's multi-vehicle PX4 workflow requires Linux. OpenFOAM additionally requires a sourced/available OpenFOAM environment and solver binaries on PATH. PyBaMM runs under the configured Python interpreter (`PYTHON` or `python`) and must be installed into that environment.

The `simulate_discharge` operation is a standalone single-cell benchmark. The `simulate_bound_cell` operation is a tested, project-scoped single-cell path: saved branches can change only supported cell properties, stale-revision branches are rejected, and baseline/branch outputs are comparable only for the same cell binding and model digest. This does not claim to model a drone battery pack; pack topology, cell data, and validated electrical loads are required for pack analysis. The PX4 operation currently starts the stock x500 airframe. A user-designed drone needs a generated Gazebo model with validated mass, inertia, motor/propeller, and sensor data before its flight simulation can be treated as the user's design.

Reality Studio's Layer 12 search enumerates two to five explicit cell-current values by saving versioned branches and dispatching each through `simulate_bound_cell`. Each successful candidate therefore appears in ordinary PyBaMM run history and project evidence. The search resource records failed candidates as well as successful outputs and ranks only completed candidates that meet the optional minimum-duration constraint. This UI-orchestrated bounded sweep is limited to the supported single-cell model; it is not a general search service or global optimizer.

The Blender `create_quadrotor` operation creates a visual `.blend` asset in the workspace export folder. It uses the saved frame diagonal and propeller diameter; other geometry is illustrative. The export is not a manufacturing CAD model, verified mass-properties model, or PX4 flight airframe.

## HTTP endpoints

- `GET /health/live`
- `GET /v1/integrations`
- `GET /v1/integrations/runs?limit=20&project_id=<uuid>` (saved completed results and failed attempts, optionally scoped to a project)
- `GET /v1/projects/{project_id}/evidence/export` (download a SHA-256-addressed JSON traceability bundle)
- `POST /v1/integrations/{tool_id}/execute` with `{ "operation": "...", "inputs": { ... } }`
- `GET /v1/integrations/px4_gazebo/sessions`
- `DELETE /v1/integrations/px4_gazebo/sessions/{session_id}`
- `POST /v1/intelligence/intent/parse` (requires an account session cookie; see [Layer 19](../19-intelligence/README.md))
- `GET /v1/projects/{project_id}/proofloop/recommendation` (evidence-backed next engineering action for the saved project)
- `POST /v1/projects/{project_id}/proofloop/record` with `{"action_id":"..."}` (records the current recommendation as a planned project action; stale recommendations are rejected)

Use `Authorization: Bearer <HELQIX_INTEGRATION_API_KEY>` when a key is configured. Execution is intentionally constrained to known operations, safe argument lists, a workspace path boundary, and per-operation timeouts.

Tool runs retain their `project_id`, exact validated inputs, and bounded process output. Failed process starts, timeouts, non-zero exits, and missing or malformed machine reports are recorded with a failure stage and an attempt ID before the API returns an error. Successful Blender, PyBaMM, and FreeCAD outputs include the underlying tool version when reported by that tool. Blender generation requires an owned saved project and reads the specification from that record rather than trusting a browser-supplied copy. Existing integration history is migrated in place to add the project association.

The project evidence export includes the saved project, project-scoped resource records, and integration inputs/results with provenance. It refuses oversized exports instead of silently truncating them and includes a SHA-256 digest over the bundle content. The bundle is explicitly traceability-only: it does not certify verification, airworthiness, or custom-drone flight performance.

The ProofLoop recommendation is a deterministic policy over persisted project evidence. It prioritizes establishing a typed reality and compatible model, running the current revision, defining explicit measurable requirements, checking the run, investigating failed or inconclusive checks, and finally planning a supervised bench comparison. A recorded plan is not proof that the physical test happened. Live CAN/bench ingestion and battery-pack/vehicle models are not available in this prototype.

For API-key project-isolation verification, start a throwaway gateway with `HELQIX_INTEGRATION_API_KEY` and `HELQIX_INTEGRATION_RUNS_DB` set, then run `23-validation/tests/verify_api_key_project_scope.py --api-key <same-key> --base-url http://127.0.0.1:8093`.
