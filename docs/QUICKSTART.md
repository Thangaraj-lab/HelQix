# Quick start

Everything below was run on this repository. Linux CI is the authority for Rust tests (see [STATUS.md](STATUS.md)).

## 1. Try the Studio with the mock engine (no setup)

```bash
cd 21-app/reality-studio
npm ci
npm run dev
```

Open the address it prints. Everything is labelled Mock or Example. `npm test` runs the plan-rule, scene, import and accessibility tests.

## 2. Run it against the real engine

You need PostgreSQL, a Rust toolchain, and the variables listed in `scripts/k2_curl_demo.sh`.

```bash
cargo run -p helqix-frames --bin helqix-frames-record /tmp/helqix-frames   # optional: a recorded reference run
HELQIX_FRAME_STORE=/tmp/helqix-frames cargo run -p helqix-api --bin helqix-api
```

In another terminal copy `21-app/reality-studio/.env.example` to `.env.local`, fill in the key the API was started with, then:

```bash
cd 21-app/reality-studio
VITE_ENGINE=real npm run dev
```

What you can do: build a model from scratch, state a **material density** for each part (Define, select a part), commit it, then in **Simulate** choose **Run my model**. The engine works out the mass and footprint from your parts and runs the body; a part without a density is named and nothing is guessed. **Freeze** at any time, **Branch** by changing one number, and play the result. Every run is linked to its project and revision.

## 3. Describe a system with a local open-source model

```bash
ollama pull qwen2.5:7b-instruct
HELQIX_OLLAMA_MODEL=qwen2.5:7b-instruct HELQIX_API_TIMEOUT_SECONDS=300 HELQIX_FRAME_STORE=/tmp/helqix-frames cargo run -p helqix-api --bin helqix-api
```

Type a description on the Hero. The model proposes parts; the API checks them; they appear labelled as AI hypotheses with placeholder sizes. One model call runs at a time (a second one gets "busy").

## 4. Verify evidence independently

See [EVIDENCE.md](EVIDENCE.md): `helqix-evidence-verify <package-dir> --trusted-key <public key>` needs no HELQIX runtime.

## 5. Checks you can run

```bash
python -I scripts/secret_scan.py .        # no secrets in tracked files
python -m pytest -q scripts/tests         # scanner and licence-gate tests
cargo test -p helqix-modelbuild           # includes the lumped-body translation
```
