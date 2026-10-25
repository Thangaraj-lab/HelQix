# Solver adapters (Phase 6)

HELQIX does not trust external tools. An adapter runs a tool in a box, checks what comes back, records exactly what happened, and labels every output **Simulated / Unverified**. A tool run is never validation evidence by itself.

## The contract

Every adapter declares a `SolverContract` (`16-adapters/platform/adapters/src/solver.rs`):

| Field | Meaning |
|---|---|
| `tool`, `fragment_types` | which tool, and which model fragments it may run |
| `inputs`, `outputs` | named fields with units; an output with the wrong unit is rejected |
| `determinism` | `Bitwise`, `Tolerance{absolute}` or `Statistical`, see [DETERMINISM.md](DETERMINISM.md) |
| `supports_state_restore` | whether a frozen state can be handed back to the tool exactly |
| `required_environment` | what must be installed |
| `timeout`, `max_output_bytes` | hard limits |
| `failure_modes` | the typed errors the adapter can return |

## How a tool is run (`run_tool`)

- **Fresh empty working directory** per run, deleted afterwards.
- **Cleared environment.** Only a short allowlist (`PATH`, `SYSTEMROOT`, temp dirs and similar) is passed; secrets in the parent environment do not reach the tool (tested).
- **Argument list, never a shell.** Input goes to stdin as JSON; shell characters in input are data (tested).
- **Timeout** kills the tool; **output cap** rejects oversized output.
- **Untrusted output.** JSON must parse, every declared output must be present with the declared unit and the same sample count, and every value must be finite.
- **Typed errors:** `ToolMissing`, `Timeout`, `Crash` (exit status + stderr summary), `InvalidInput`, `MalformedOutput`, `UnitMismatch`, `NonFinite`, `OutputTooLarge`.
- **Invocation record:** tool, tool version (reported by the tool), SHA-256 of the input, configuration, wall time, SHA-256 of the exact stdout, exit status, stderr summary. The raw stdout is kept so it can be packaged.

`conformance()` is the conformance kit: a good input must succeed and satisfy the contract, every bad input must be refused with a typed error, and an adapter that claims `Bitwise` while two runs differ fails.

## Adapters

| Tool | Status | Licence | Determinism | State restore |
|---|---|---|---|---|
| PyBaMM 26.10 (SPM, Chen2020) | **Working**: 6 adapter tests + conformance PASS | BSD-3-Clause (used as an external process) | Tolerance 4.2e-6 V | No |
| CalculiX | **Not built.** Not installed on this machine; licence is GPL-2.0. Running it as a separate process does not by itself make HELQIX GPL, but shipping it inside a HELQIX installer would need a licence review. **Founder decision.** | GPL-2.0 | — | — |
| OpenModelica, FMI/FMU, Ansys, COMSOL, others | Planned, not built | varies | — | — |

### PyBaMM

`PybammAdapter` runs `18-libraries/python/helqix_battery/adapter_cli.py` (stdin `helqix.pybamm-input.v1`, stdout `helqix.pybamm-output.v1`; exit 2 invalid input, exit 3 simulation incomplete). The package directory is passed as an argument, not pasted into code. It is also registered as a platform `Adapter` (`battery.cell.simulate`).

**Declared tolerance rule:** declared = max(10 × measured run-to-run maximum deviation, solver rtol 1e-6 × 4.2 V). Measured on Windows over 5 runs: maximum deviation **0 V**, so the floor applies: **4.2e-6 V**. Zero variation on one machine is not proof of zero variation elsewhere, which is why the floor exists and why the level is Tolerance, not Bitwise.

## Reference run and evidence package

```bash
cargo run -p helqix-evidence-pack --bin helqix-adapter-check -- <python-with-pybamm> <out-dir> --key <secret-key-file> --signed-at <time>
```

It runs conformance, measures variation (5 runs), checks discharge capacity against the closed form Q = I·t (max error 8.9e-16 A·h), and writes a signed **Tolerance** evidence package. Sample: [evidence/pybamm-adapter-package](evidence/pybamm-adapter-package), signed with the same dev key as the Phase 5 sample.

The independent verifier picks its profile from the statement's determinism level. A Tolerance or Statistical package must contain `invocation_record`, `tool_outputs` and `verdict`, and the tool output's SHA-256 must equal the hash in its invocation record (tested; mutation-checked).

```bash
cargo run -p helqix-evidence-verify -- docs/evidence/pybamm-adapter-package --trusted-key-file docs/evidence/sample-package.public-key.txt
```

## Limits

- The package proves what the tool was given and what it returned. It does not prove the tool is right; the battery model is still **NotVerified** for its Context of Use (Gate 1).
- Re-execution of a Tolerance package (re-running PyBaMM and comparing within tolerance) is not built yet; only the Bitwise sample can be re-executed.
- The sandbox is process-level (clean directory, clean environment, limits). It is not an OS container: the tool can still read files the user can read and use the network.
