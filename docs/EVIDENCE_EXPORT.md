# Evidence export (E8)

`POST /v1/runs/{id}/evidence` builds a signed evidence package for a stored run and returns it as a ZIP. The server builds and signs; the browser never holds a key.

## Request

```json
{ "requirement": { "name": "stay above ground", "variable": "y", "comparator": "atLeast", "threshold": 0 }, "subject": "optional one line" }
```

`variable` is one of the run's variables (`x y z vx vy vz` for the built-in engine); `comparator` is `atLeast` (>=), `atMost` (<=), `above` (>) or `below` (<): a strict limit is broken by a value that only equals it. A value that is not a number breaks any requirement. An unknown variable answers 422 and names the known ones.

## What a run must be

- A run of the built-in engine (point mass). It is **re-executed from its stored parameters** and must end on the stored final state hash; if it does not, no evidence is issued (409).
- Runs made with an FMU or the battery pack are **not covered yet** (422): their package shape (Tolerance, invocation record, tool output) is the next step (E8.5).
- A requirement that fails: the package is frozen at the first failing sample. A requirement that holds: frozen at the last sample, no failure index, and the verifier accepts that shape.
- The verdict inside is always `NotValidated` with no causal claim: no measured data was compared.

## What comes back

```
evidence-<run>.zip
  evidence-<run>/                    manifest, signature, freeze, state chain, requirement outcome, verdict, frames (as stored)
  evidence-<run>.intoto.dsse.json    signed in-toto attestation of the package
  trusted-public-key.txt             the public key to tell the verifier to trust
  README.txt
```

To check it, with no network and no HELQIX service:

```
helqix-evidence-verify evidence-<run> --trusted-key-file trusted-public-key.txt --attestation evidence-<run>.intoto.dsse.json
```

The response also carries the key that signed this very file in headers (`x-helqix-key-id`, `x-helqix-public-key`, `x-helqix-key-kind`, `x-helqix-signer`), so a reader never has to ask the service afterwards; the key in `trusted-public-key.txt` is the same one.

At most two exports run at once; another answers **429**. Each one re-runs the simulation, reads the frames and signs.

PASS means the files are exactly what the holder of that key signed. It does not mean the physics is right. `signed_at` is the server's clock, a claim by the signer.

## Who may export

The operator's service key may export any run. A user may export only a run they own (one they started, or one made from their project); another user's run answers 404.

## Signing key (operator)

| Variable | Meaning |
|---|---|
| `HELQIX_EVIDENCE_KEY_FILE` | A file with 64 hex characters: an Ed25519 seed. Without it export answers 503. A malformed file stops the start. |
| `HELQIX_EVIDENCE_SIGNER` | Name written into the signature (default `helqix-evidence-service`); plain printable ASCII, one line, at most 120 characters (it is sent in a header). |
| `HELQIX_EVIDENCE_KEY_KIND` | `test` (default) or `production`. Reported by `GET /v1/evidence/key`. |

`GET /v1/evidence/key` returns the public key, key id and kind. The private key never appears in a response, a log or a package. **Until the founder decides key custody (F4) the key is a test key.**
