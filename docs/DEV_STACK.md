# Development stack and gates (K1)

## Gates: `make check`

```bash
make check          # Linux/macOS, or Git Bash on Windows: bash scripts/check.sh
```

Runs, stopping at the first failure: Rust build, clippy, Rust tests, **cargo-deny** (licences, bans, sources, advisories; policy in `deny.toml`), the **JS licence gate** (`scripts/js-license-check.sh`, production dependencies of `21-app/reality-studio`), the **tool register gate** (`scripts/check_tools_lock.py`: every tool in `tools/TOOLS.lock.json` has a verified licence; no forbidden tool appears as an image in the dev stack), and the Python suites. PyBaMM suites run when `HELQIX_PYTHON` has PyBaMM.

Licence policy is in [TOOLS.md](TOOLS.md). Unlisted or missing licences fail. HELQIX's own crates (`publish = false`) and `@helqix/*` packages are excluded because their licence is an open founder decision: the root `LICENSE` file is empty; 20 crates say MIT, 4 say UNLICENSED, `helqix-state` says nothing.

Both licence gates were checked to fail: a MinIO image in the compose file fails the register gate; removing MIT from the JS allowlist fails the JS gate.

## Dev stack

Needs Docker with Compose v2 (not installed on the founder's PC as of 2026-10-25; CI runs it on Linux).

```bash
cp deploy/dev/.env.example deploy/dev/.env      # then change the password; .env is git-ignored
make dev-up                                      # waits until every service is up
make dev-down                                    # stop (volumes kept); add -v by hand to delete data
```

| Service | Image (pinned) | Licence | Port (localhost only) | Volume |
|---|---|---|---|---|
| PostgreSQL 17 + pgvector 0.8.7 | `pgvector/pgvector:0.8.7-pg17-trixie` | PostgreSQL | 5432 | `postgres-data` |
| NATS with JetStream | `nats:2.15.1` | Apache-2.0 | 4222, monitor 8222 | `nats-data` |
| SeaweedFS (S3 API) | `chrislusf/seaweedfs:4.50` | Apache-2.0 | 8333 | `seaweedfs-data` |
| Temporal dev server | `temporalio/temporal:1.9.1` (`server start-dev`, in memory) | MIT | 7233, UI 8233 | none: history is lost on restart (a file in a named volume failed in CI) |
| OpenTelemetry Collector | `otel/opentelemetry-collector:0.163.2` | Apache-2.0 | OTLP 4317 gRPC, 4318 HTTP | — |

Ports bind to 127.0.0.1 only. The Temporal dev server and the debug-only collector are development settings, not production. SeaweedFS replaces Garage, which is AGPL-3.0 (forbidden).

CI job `dev-stack` brings the stack up, probes each service (pgvector extension created, NATS health, S3 endpoint, Temporal UI, OTLP endpoint) and tears it down.
