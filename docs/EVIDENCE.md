# Signed evidence and independent verification (Phase 5)

Date 2026-10-25. Crates: `helqix-evidence-pack` (builder: `keygen`, `build`, `reexecute`) and `helqix-evidence-verify` (independent verifier; no HELQIX runtime dependency, checked in CI). UI work (verification workspace, canvas polish) is out of scope of this engine work.

## Package

A directory with one file per evidence item, plus `manifest.json` and `signature.json`:

| File | Role | Content |
|---|---|---|
| freeze.json | freeze | Phase 2 freeze at the failure (content-addressed) |
| state_chain.json | state_chain | every hash-chain link of the run, final link, determinism level |
| requirement_outcome.json | requirement_outcome | requirement, failure index, time, value |
| investigation.json | causal_chains | Phase 3 candidate chains with grades and click-through details |
| interventions.json | branch_records | Phase 4 intervention verdicts with lineage |
| verdict.json | verdict | label `NotValidated`, causal grade `model-causal`, reason |

The manifest lists every file (path, role, SHA-256) in path order, a Merkle root over them, and a statement (subject, context of use, determinism, verdict, label, final state hash, failure index). The signature is Ed25519 (`ed25519-dalek` 2.2) over `helqix.evidence.v1\n<manifest sha256>\n<signer>\n<signed_at>`.

## What the verifier checks

Manifest schema; canonical entry order; plain file names; required roles; every file hash; no unlisted files; Merkle root; signing key equals the key the user trusts; signature covers this exact manifest; signature valid; final state hash consistent across the chain and the statement; failure index consistent across outcome, freeze and statement. Every result prints: "Signing proves origin and integrity of the package, not correctness of the physics."

Companion re-execution (`helqix-evidence-pack reexecute`) replays from the package's freeze on the core runtime and compares the final state hash (Bitwise).

## Evidence (VERIFIED)

- 10 tamper tests on the verifier, with packages built by hand: byte change, removed file, unlisted file, reordered manifest, replaced signature, wrong key, mixed packages, signed but inconsistent, Merkle sensitivity, and a good package passing.
- 5 builder tests: independent round trip, deterministic manifest and Merkle root, re-execution passes and fails after a freeze change, honest label, refuses a non-empty directory.
- Sample package `docs/evidence/sample-package`, built on Windows with a development key; public key in `docs/evidence/sample-package.public-key.txt`. Locally: verifier PASS (19 checks) and re-execution PASS. CI verifies and re-executes the same files on Linux (second clean environment) once the branch is pushed.
- Mutations caught: disabling the verifier's file-hash check (2 tests fail); changing the builder's Merkle leaf format (round trip fails).

## Threat model

Protected:
- Any change, removal, addition or reordering of package files after signing.
- Substituting a package signed by a key the user does not trust.
- Mixing parts of two packages.
- A package whose statement contradicts its own files.

Not protected (stated):
- **Correctness of the physics.** A signed wrong model is still wrong; validation evidence (Phase 1) or its absence speaks to correctness. This package says `NotValidated`.
- **Key compromise and key custody.** Development keys only; production key custody is an open founder decision. The private key is never written into a package; `.keys/` and `*.key` are git-ignored.
- **Time.** `signed_at` is a claim by the signer. Proof of time needs a trusted timestamp authority (design only).
- **Trust anchor distribution.** The verifier trusts whatever public key it is given; how users obtain the right key is outside this phase.
- **Re-execution scope.** Bitwise re-execution is guaranteed on the same build and platform; cross-platform agreement is tested by CI (Windows-built package, Linux replay) but is not a general guarantee.

## Reproduce

```
cargo run -p helqix-evidence-pack --bin helqix-evidence-pack -- keygen .keys/my-dev.key
cargo run -p helqix-evidence-pack --bin helqix-evidence-pack -- build .data/package --key .keys/my-dev.key --signer me --signed-at 2026-10-25T00:00:00Z
cargo run -p helqix-evidence-verify --bin helqix-evidence-verify -- .data/package --trusted-key <public key printed by build>
cargo run -p helqix-evidence-pack --bin helqix-evidence-pack -- reexecute .data/package
```

## K5: in-toto attestation and cosign

**Signing proves origin and integrity, not correct physics.**

### in-toto attestation (HELQIX, Ed25519)

`helqix-evidence-pack attest <package> --key <key> --out <package>.intoto.dsse.json` writes an **in-toto Statement v1** in a **DSSE envelope** (`application/vnd.in-toto+json`), signed with the package's Ed25519 key over the DSSE pre-authentication encoding.

- Subjects: every file of the package, including `manifest.json` and `signature.json`, by name and SHA-256.
- Predicate type `https://helqix.dev/attestations/evidence-package/v1`. The predicate holds the builder, the Merkle root, the manifest statement and the scope note.
- The envelope is stored **beside** the package, so the package's own file list is unchanged.

`helqix-evidence-verify <package> --trusted-key-file <pub> --attestation <envelope>` checks the package (Phase 5) and then the attestation. The trusted key must have signed the envelope. The attested files must equal the package files exactly (a changed, added or removed file fails). The Merkle root must match the manifest, and the scope note must be present. The verifier still has no HELQIX runtime dependency and no network code. CI runs it inside a Linux network namespace with no network (`unshare --net`).

Samples: `docs/evidence/sample-package.intoto.dsse.json` and `docs/evidence/pybamm-adapter-package.intoto.dsse.json`, signed with the same dev key as the packages; both verify.

Tests: DSSE pre-authentication encoding and base64 against the spec and RFC 4648 vectors (2); attestation of a real package verifies; changed, added and removed files fail; altered payload and altered payload type fail; wrong trusted key, an envelope signed by an untrusted key, and another package's attestation fail (4). Mutations: removing the file-set check and accepting any signature are both caught. Signing over the fixed payload type instead of the stated one survives, but it is equivalent: the type check already requires the two to be the same.

### cosign (Sigstore)

`scripts/cosign_package.sh <cosign> <package> <work-dir>` uses cosign v3.1.4 (Apache-2.0, official release, SHA-256 pinned in CI) on top of a package:

1. `cosign sign-blob` signs `manifest.json`, which covers every file by hash and Merkle root.
2. `cosign attest-blob` attests the same predicate HELQIX writes, with the same predicate type.
3. `cosign verify-blob` and `cosign verify-blob-attestation` verify both offline with the public key. A tampered manifest must fail both.

**Nothing is uploaded.** The signing config lists no transparency log and no timestamp service, and every HTTP(S) request goes to a dead proxy, so any attempted upload fails instead of publishing. Verification uses `--insecure-ignore-tlog` because there is deliberately no log entry. The cosign key pair in CI is generated for each run. Where a real cosign key lives (file, KMS, hardware), and whether to publish to the public Rekor log, are **founder decisions**: a public log entry is permanent and reveals the artifact hash, signer key and time.

Not done: SLSA build provenance for HELQIX's own binaries, keyless (Fulcio) signing, Rekor upload, and verifying cosign bundles inside `helqix-evidence-verify` (cosign verifies its own bundles).
