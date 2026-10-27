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

## What the real engine does not do yet

It cannot interpret a typed description (use Start from scratch), start a run, freeze or branch.
Simulate plays a recorded run from the frame store and you bind model parts to its variables.

## Rules the tests enforce

No built-in domain parts outside `src/mock`; no "Verified", "Calibrated" or "Confidence" claims;
no invented history or computed effects; every mock value tagged; motion only from a bound variable.
Imported glTF and HDRI files are structure-checked and carry a recorded licence.
