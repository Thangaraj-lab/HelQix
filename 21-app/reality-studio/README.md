# HELQIX Studio (21-app/reality-studio)

React 18, Vite 5, Tailwind 4, three.js. One Studio: Hero, Home, and `/p/:projectId` with stages
Define, Model, Simulate, Understand, Improve, Verify, Observe, Build (the last three are PREVIEW).

## Run it

```bash
npm ci
npm run dev          # mock engine: everything is labelled Mock or Example
npm test             # plan rules, bindings, import checks, engine client wire format
npm run typecheck
npm run build
```

## Run it against the real API

Copy `.env.example` to `.env.local` (git-ignored) and fill in the key the API was started with.

```bash
# in the repo root, with PostgreSQL running and the variables from scripts/k2_curl_demo.sh
cargo run -p helqix-api --bin helqix-api
# in this folder
VITE_ENGINE=real npm run dev
```

The browser calls `/api/*`; the dev server forwards it to `HELQIX_API_URL` and adds the key on the
server side. The key is never in the bundle. For a recorded run to play in Simulate, start the API
with `HELQIX_FRAME_STORE=<dir>` after `cargo run -p helqix-frames --bin helqix-frames-record <dir>`.

## Describe a system (local open-source model)

Install [Ollama](https://ollama.com) (MIT), pull an open-weights model (for example `ollama pull qwen2.5:7b-instruct`, Apache-2.0)
and start the API with `HELQIX_OLLAMA_MODEL=qwen2.5:7b-instruct` (optional `HELQIX_OLLAMA_HOST`, loopback only; set
`HELQIX_API_TIMEOUT_SECONDS=300` on a CPU-only machine). A description typed on the Hero then becomes parts labelled
as AI hypotheses with placeholder sizes; you edit and commit them like any other part. Without a model the Studio says so
and Start from scratch still works.

## What the real engine does

Simulate starts the point-mass model from the numbers you type (`POST /v1/runs`), can freeze any time and branch with one changed
number, and plays recorded runs from the frame store. A Studio project's parts are not yet translated into a solver model, so
you bind parts to the run's variables yourself.

## Rules the tests enforce

No built-in domain parts outside `src/mock`; no "Verified", "Calibrated" or "Confidence" claims;
no invented history or computed effects; every mock value tagged; motion only from a bound variable.
Imported glTF and HDRI files are structure-checked and carry a recorded licence.
