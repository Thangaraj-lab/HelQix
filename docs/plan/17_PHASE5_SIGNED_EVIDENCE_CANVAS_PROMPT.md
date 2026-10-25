# 17. Phase 5 Prompt: Signed Evidence, Verification Workspace, Canvas Polish

Prerequisites: Gate 4 passed. Paste 13 first. Branch: `phase-5-evidence`.

## Goal

Every conclusion HELQIX shows can be exported as an evidence package that a third party can verify independently, offline, without trusting HELQIX's running process. The UI shows trust status honestly. This is the product's differentiator; treat correctness here as critical.

## Background to verify

helqix-evidence did not compile at audit (fixed in Phase 0). Provenance today is sha256 only; no signing, no Merkle structure. Check the repo for any prior crypto before adding; reuse vetted crates and never hand-roll cryptographic primitives.

## Tasks

1. Evidence package contents (a single directory or archive plus manifest): model revision hash, scenario, parameters, environment and tool versions, determinism level, freeze(s), state-hash chain, results, requirement outcomes, Context of Use, validation evidence references (Phase 1), uncertainty statement (Phase 1), causal chains with grades (Phases 3 and 4), branch/intervention records, and the verdict with its label.
2. Integrity structure: content-addressed files and a Merkle tree (or equivalent hash tree) over manifest entries; one root hash.
3. Signing: sign the root with an asymmetric signature (Ed25519 via a vetted library). Key handling: development keys are generated locally and never committed; production key custody is an open decision (record as a decision, do not invent). Include signer identity, key id, timestamp field. Be explicit that a timestamp inside the package is a claim, not proof, unless an external timestamp authority is added (optional, design only).
4. Independent verifier: a separate small command-line tool (own crate/binary, minimal dependencies, no import of the runtime) that takes a package and prints PASS or FAIL with reasons. It must check: manifest completeness, every hash, Merkle root, signature, and internal consistency (for example, the result hash matches the state-hash chain end).
5. Re-execution check (where determinism permits): the verifier or a companion command can replay from the package and compare. Report level (Bitwise or Tolerance) honestly; for Tolerance state the tolerance.
6. Tamper tests are first-class: modify one byte of any file, remove a file, reorder manifest, replace signature, swap in a different valid-looking package: every case must FAIL with the right reason.
7. Verification workspace in the UI: shows requirement status, Context of Use, credibility statement, uncertainty, determinism level, origin x status badges, and an "Export evidence" action. No green "Verified" badge unless the Trust layer criteria are met; otherwise show the exact missing criteria.
8. Canvas polish (only after the above): make the five modes (Reality, Time, Causality, Branch, Verification) consistent in state and labels. Do not add visual features that outrun the engine. If WebGPU is used, keep the WebGL2 fallback and treat WebGPU as experimental.

## Required tests

- Round trip: build, sign, verify = PASS.
- Every tamper case above = FAIL with a specific reason.
- Verifier binary builds and runs with no access to runtime crates (dependency check in CI).
- Signature with wrong key fails.
- Deterministic package build: same inputs give same Merkle root (excluding signature timestamp field, which is documented).
- UI: a failing requirement never renders as Verified (UI test).
- Mutation sanity check as in the preamble, especially on the verifier (break a hash check and show a tamper test catches it).

## Gate 5

A package produced on one machine is verified PASS by the standalone verifier on a second clean environment, and all tamper cases FAIL correctly. Raw output pasted. Security review notes written (threat model: what is and is not protected). STATUS updated.

## Security notes to include in docs

Signing proves origin and integrity of the package, not correctness of the physics. Validation evidence supplies correctness claims. Say this in the verifier output text.

## Out of scope

Adapters, edge hardware, agent, quantum.
