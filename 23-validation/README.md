# Layer 23 — Cross-layer product checks

`tests/verify_local_product.py` exercises the local UI API boundary using only Python's standard library. It checks CORS preflight, account/session behavior, explicit intent extraction, project validation, project-scoped scenario variants, rejection of invalid inputs, and the installed Blender export plus reopen/inspection path when Blender is available.

Run it only against a throwaway local development database. It creates a one-off account/project and writes a generated Blender file under `data/validation/`; those local smoke-test records are not production certification data.

Example, with the gateway pointed at an isolated SQLite file:

```powershell
$env:HELQIX_INTEGRATION_RUNS_DB = (Join-Path (Get-Location) 'data\validation_smoke.sqlite3')
& .\17-integrations\gateway\.venv\Scripts\python.exe -m uvicorn app:app --app-dir 17-integrations\gateway --host 127.0.0.1 --port 8092
```

In a second terminal, run:

```powershell
& .\17-integrations\gateway\.venv\Scripts\python.exe .\23-validation\tests\verify_local_product.py --base-url http://127.0.0.1:8092 --ui-origin http://127.0.0.1:5178
```

This suite does not validate flight physics, structural safety, airworthiness, or regulatory compliance. HELQIX has no design-specific flight simulation in this checkout, so no Layer 23 drone performance benchmark is claimed.

## Product readiness gate

Layer 23 is the release gate for cross-layer behavior. A release candidate must keep the Rust workspace, API, integration gateway, and Reality Studio build green, and must not treat architectural scaffolding as production capability. External solvers, QPU execution, AI proposals, durable workers, and security controls are only certified when their live lifecycle paths and failure modes are exercised by tests.
