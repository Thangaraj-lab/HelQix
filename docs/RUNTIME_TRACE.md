# Runtime Trace (Phase 0, task G)

Date: 2026-10-25. Branch `phase-0-hygiene`, base commit `bf318f4`. Read-only trace; no runtime code was changed to produce it.
Method: reading source plus running the Python gateway locally on Windows (see "Runtime evidence"). Every claim cites file:line. Anything not confirmed is marked UNKNOWN.

## Summary (one paragraph)

Three separate systems exist and they do not call each other. (1) The **Rust workspace** (crates for state, time, branch, execution, evidence, ...) has no consumer: its only binary, `helqix-api`, depends on none of the `helqix-*` crates. (2) The **Python gateway** (FastAPI + SQLite) is what the running React app talks to; it stores projects, branches, runs and evidence bundles, and runs PyBaMM as a subprocess. (3) The **React app** contains its own TypeScript engine (authoritative state, execution controller, branch engine, freeze manifest), but the execution board and freeze code are not mounted in the app. Today there is no single authoritative state; the closest thing is the gateway's SQLite store of saved project payloads.

## Q1. Do the gateway or the app call any Rust crate (FFI, CLI, HTTP, WASM)?

**No (VERIFIED by search and by reading the call sites).**

- Gateway imports only Python packages from the repo: it puts `19-intelligence/python` and `18-libraries/python` on `sys.path` (`17-integrations/gateway/app.py:31-38`) and imports `helqix_intelligence` and `helqix_drone_domain` (`app.py:35`, `app.py:39-41`). A search of all gateway, intelligence, libraries and Python SDK code for `cargo`, `ctypes`, `cffi`, `pyo3`, `maturin`, `.dll`, `.so`, `wasm` and the Rust API port finds no Rust call (the only hit is the English word "cargo" in `19-intelligence/python/helqix_intelligence/intent.py:199`).
- Gateway subprocesses are external tools only: PyBaMM via the Python interpreter (`app.py:876-880`), Blender/FreeCAD (`app.py:287`, `app.py:291`), PX4 `make` (`app.py:1104`). None is a Rust binary.
- No Rust crate exposes FFI, PyO3 or wasm-bindgen: search for `pyo3`, `wasm_bindgen`, `#[no_mangle]`, `extern "C"` across `*.rs` and `Cargo.toml` returns nothing.
- The app's live API client targets the gateway: `21-app/reality-studio/src/api.ts:1` (`VITE_HELQIX_API_URL` or `http://127.0.0.1:8091`, the gateway port in `17-integrations/gateway/run.ps1`). `App.tsx:3` imports this client.
- A second client, `src/api/client.ts:16` (`API_BASE = "/api"`), is used only by `src/state/auth.tsx` and `src/pages/PasswordRecoveryPage.tsx`, which belong to the router shell in `src/app/routes.tsx:29-67`. That shell is **not mounted**: `src/main.tsx` renders `<App />` from `App.tsx`, and `routes.tsx` is imported only by tests. `vite.config.ts` defines no `/api` proxy. What `/api` would reach in a deployment is UNKNOWN.
- The only Rust binary in the workspace is `22-api/helqix-api` (`22-api/helqix-api/Cargo.toml:9`). Its `[dependencies]` (`Cargo.toml:13` onward) are axum, sqlx (Postgres), tokio etc.; **no `helqix-*` crate**. It listens on `127.0.0.1:8080` by default (`22-api/helqix-api/src/config.rs:25-26`) and requires `DATABASE_URL` (`config.rs:30`). Nothing in the app or gateway points at port 8080. It has 0 tests.

Consequence: the Rust crates (`helqix-state`, `helqix-time`, `helqix-branch`, `helqix-execution`, `helqix-evidence`, ...) are libraries with tests but no runtime caller. Only other Rust crates depend on them (`05-execution`, `08-observation`, `09-failure`, `10-events-causality`, `11-branching` Cargo.toml files).

## Q2. Where do run, state, branch and evidence data actually live?

| Data | Location | Evidence |
|---|---|---|
| Users, sessions | Gateway SQLite tables `helqix_users`, `helqix_sessions` | `app.py:165-172` |
| Projects, branches, searches, scenarios (JSON payloads) | Gateway SQLite `helqix_resources(payload_json, version)` | `app.py:175-181`, insert at `app.py:389`; app saves branches/searches via `api.createResource` (`App.tsx:464`, `App.tsx:502`, `api.ts:85`) |
| Tool runs (PyBaMM, Blender, reality validation) | Gateway SQLite `helqix_integration_runs(inputs_json, result_json, provenance, project_snapshot_json)` | `app.py:191-212`, insert at `app.py:361` |
| Audit trail | Gateway SQLite `helqix_audit_events`, with triggers blocking UPDATE and DELETE | `app.py:195-204`, insert at `app.py:378-379` |
| Evidence bundle | Built on request from the rows above, hashed with sha256, not stored, not signed | `app.py:1246-1294` (`bundle_sha256` at `app.py:1294`) |
| Database file | `data/integration_runs.sqlite3` under the workspace root, or `HELQIX_INTEGRATION_RUNS_DB` | `app.py:45` |
| In-app execution state (TypeScript) | React component memory only, never persisted | `ScenarioExecutionBoard.tsx:22-25` (and that component is not mounted, see Q4) |
| Rust state / snapshots / branches | In-process memory in library tests only | no runtime caller (Q1) |
| Rust API resources | Postgres via sqlx (`22-api/helqix-api/src/store.rs`) | not used by the app (Q1); not run in this trace |

## Q3. Which component is the authoritative state today?

**No component meets the "one authoritative state" principle today.** The closest is the **gateway SQLite store**: the saved project payload (including `engineering_reality`) in `helqix_resources` is what runs bind to, and runs record a sha256 digest of the model at run time (`project_snapshot`, `app.py:396-415`). A verifier compares source and current digests to detect a stale model (`app.py:658-706`).

But it is a JSON document store, not a typed state runtime with snapshots, logical time or event history. The typed authoritative-state implementations exist twice, in Rust (`04-state`) and in TypeScript (`21-app/reality-studio/src/engineering/authoritativeState.ts:3-4`, `:131`), and neither is used on the live path. **Decision D5 (runtime owner) is needed from the founder**; see "Decision needed" below.

## Q4. Where are freeze, replay, branch implemented end to end?

None is wired end to end. File:line per implementation:

**Freeze**
- Rust: no "freeze" or "checkpoint" type. Snapshots: `04-state/crates/helqix-state/src/snapshot.rs:28` (`capture`), `07-time/crates/helqix-time/src/snapshot.rs:146` (`capture`). No runtime caller.
- TypeScript: `21-app/reality-studio/src/engineering/stateFreeze.ts:53` (`createStateFreezeManifest`), `:77` (`verifyStateFreezeManifest`), digest at `:43`. **Imported by no other file** (not even `engineering/index.ts`). Not reachable from the UI.
- Gateway: no freeze. Runs store a model digest only (`app.py:396-415`); the comment at `app.py:907` says the snapshot stores a digest, not historical payloads.

**Replay**
- Rust: `07-time/crates/helqix-time/src/replay.rs:607` (`pub fn replay`). No runtime caller.
- TypeScript: `authoritativeState.ts:319` (`restoreSnapshot`), `:329` (`stateAtLogicalTime`), `executionController.ts:69` (`restoreExecutionCheckpoint`). Used only by `ScenarioExecutionBoard.tsx`, which **no file imports** (commit `910b89a` "wire executable scenario studio board" added the component file only; `App.tsx` never imports it).
- Gateway: no replay. Re-running a PyBaMM operation re-executes the tool.

**Branch**
- Rust: `11-branching/crates/helqix-branch/src/branch_tree.rs:43` (`fork`), `:53` (`fork_at`). No runtime caller.
- TypeScript: `engineering/branchEngine.ts:67` (`createBranch`), `:107` (`applyBranch`), `:168` (`compareBranches`). Branch creation in the UI goes through `App.tsx:464` (save to gateway).
- Gateway / Python: branch schema `18-libraries/python/helqix_drone_domain/reality.py:66-73` (`EngineeringBranchChange`, `EngineeringBranch`); a saved branch is loaded and checked against the current reality revision before a bound PyBaMM run (`app.py:965-980`). **This is the only branch path that runs end to end** (validated by `23-validation/tests/verify_local_product.py`, see below): saved branch -> PyBaMM run with branch changes -> stored run.

## Q5. Can visual state or AI output become authoritative anywhere?

**Visual state: No (VERIFIED for the code read).** `engineering/visualStateBinding.ts` projects authoritative state into display entities (doc comment at `:29-30`; it clones input and asserts validity). Search found no path from the visual runtime back into state. The 3D canvas receives entities as props (`App.tsx:746`).

**AI output: not automatically (VERIFIED for the code read), but there is no permission ladder.**
- AI review is optional and opt-in: the gateway calls an external LLM only if `assisted` is set and a provider is configured (`app.py:1209-1216`; provider configuration `19-intelligence/python/helqix_intelligence/intent.py:23-29`, `:33-45`). Seven providers are wired (OpenAI, Anthropic, Gemini, DeepSeek, Kimi, Qwen, GLM). **This contradicts audit 04, which said 19-intelligence is deterministic intent extraction only.**
- The AI result is returned in the HTTP response and not written to the database by that route (`app.py:1216-1217`).
- The UI labels it "PROPOSALS FROM <provider> ... REVIEW REQUIRED" and "AI PROPOSAL", and the user must tick each proposal (`App.tsx:1158`). What happens to accepted proposals after that (whether they are saved into the project payload with an AI origin label) was not traced: **UNKNOWN**.
- There is no Read/Analyze/Propose/.../Mutate permission enforcement in code (consistent with audit 04).

## Runtime evidence (commands run on Windows 11, Python 3.14.6, 2026-10-25)

- Gateway started with `python -m uvicorn app:app --host 127.0.0.1 --port 8091` from `17-integrations/gateway`, database in a scratch directory. `/health/live` returned `{"status":"live"}`.
- Integration inventory: `helqix_reality` ready, `blender` ready, `pybamm` (see below), `freecad`, `openfoam`, `px4_gazebo` unavailable (tools not installed).
- **PyBaMM through the gateway: WORKS (VERIFIED).** `POST /v1/integrations/pybamm/execute` with `simulate_discharge`, 5.0 A to 3.0 V, run twice. Both runs returned: model SPM, parameter set Chen2020, tool_version 26.10.1.1, duration 0.9335766219862074 h, discharged capacity 4.6678831099310365 Ah, final voltage 2.999999999999948 V, 65 samples. The two runs were identical in every returned field (same machine, same install; this is not a determinism guarantee).
- Finding: on a cold start the first `import pybamm` took 64 s; the gateway's readiness check times out at 20 s and caches the failure until restart (`app.py:329-352`), so the first gateway start reported PyBaMM "unavailable". Warm imports take about 2.5 s. Not fixed (out of scope); logged in STATUS Findings.
- `23-validation/tests/verify_api_key_project_scope.py`: PASS.
- `23-validation/tests/verify_local_product.py`: PASS (includes three bounded PyBaMM SPM branch runs, evidence export, Blender .blend round trip, cross-account isolation). The script itself states it does not run or certify flight simulation.

## UNKNOWN (not traced)

- What `/api` resolves to for the unmounted router shell in a deployment.
- Whether accepted AI proposals are persisted, and with what origin label.
- Whether the Rust `helqix-api` service is deployed anywhere.
- Rust behaviour at runtime on this machine: the Rust workspace does not link on this Windows machine (Windows SDK missing), see STATUS.

## Decision needed (D5)

Pick the authoritative runtime owner. Facts that bear on it: the Rust crates hold the most tested engine code but have no caller; the gateway is the only path that runs end to end and owns persistence; the TypeScript engine duplicates Rust concepts in the browser and is mostly unmounted. Options: (a) Rust core as authority, called by the gateway (CLI, PyO3 or HTTP via `helqix-api`); (b) Python gateway as authority, Rust used as a verified kernel; (c) TypeScript engine as authority (conflicts with "one authoritative state" if the browser holds it). Phase 2 (exact replay) cannot start until this is decided.
