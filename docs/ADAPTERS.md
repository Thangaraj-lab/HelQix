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
| FMI 2.0 co-simulation via FMPy 0.3.32 | **Working** (K3): 7 tests incl. regression benchmark and conformance PASS | FMPy BSD-3, Pint BSD-3; test FMUs BSD-2 | Tolerance 1e-12 (ASSUMPTION floor) | No |
| PyBaMM 26.10 (SPM, Chen2020) | **Working**: 6 adapter tests + conformance PASS | BSD-3-Clause (used as an external process) | Tolerance 4.2e-6 V | No |
| CalculiX ccx (Ubuntu `calculix-ccx`) | **Built** (K3): cantilever benchmark, linearity, timeout, invalid input, conformance; verified in Linux CI only (no ccx on the founder's PC) | GPL-2.0, run only as a separate program | Tolerance 1e-12 m (ASSUMPTION floor) | No |
| OpenModelica, FMI/FMU, Ansys, COMSOL, others | Planned, not built | varies | — | — |

### PyBaMM

`PybammAdapter` runs `18-libraries/python/helqix_battery/adapter_cli.py` (stdin `helqix.pybamm-input.v1`, stdout `helqix.pybamm-output.v1`; exit 2 invalid input, exit 3 simulation incomplete). The package directory is passed as an argument, not pasted into code. It is also registered as a platform `Adapter` (`battery.cell.simulate`).

**Declared tolerance rule:** declared = max(10 × measured run-to-run maximum deviation, solver rtol 1e-6 × 4.2 V). Measured on Windows over 5 runs: maximum deviation **0 V**, so the floor applies: **4.2e-6 V**. Zero variation on one machine is not proof of zero variation elsewhere, which is why the floor exists and why the level is Tolerance, not Bitwise.

### FMI (K3)

`FmiAdapter` runs `18-libraries/python/helqix_fmi/cli.py` (stdin `helqix.fmi-input.v1`; exit 2 invalid input, exit 3 simulation failed) through the same sandboxed runner. The caller passes parameters and times only. The adapter fixes the FMU path and output list, and adds the **FMU file's SHA-256** to the tool input, so the invocation record identifies the exact model binary. Units come from the FMU's model description and are normalized to SI base units with Pint; a variable without a unit is `"1"`. An output whose unit differs from the contract is rejected.

**Benchmark** (Modelica Association Reference FMUs v0.0.42, BSD-2-Clause, fetched with a pinned SHA-256 by `tools/fixtures/fetch_reference_fmus.py`; Example data, not committed):

| Check | Result |
|---|---|
| Dahlquist (dx/dt = -k x) against the reference output shipped inside the FMU | max deviation 0, within the declared 1e-12 |
| Same model against the closed form of its scheme, x_n = (1 - k h)^n, for k = 0.5, 1, 2 | within 1e-12 (CALCULATED) |
| Same model against the exact ODE solution e^(-k t) | differs by more than 1e-3 (the FMU uses fixed-step Euler); the test asserts the gap stays visible |
| BouncingBall height against its shipped reference; units `s`, `m`, `m/s` | within 1e-12; asking for `mm` is a unit mismatch |
| Two runs on this machine | byte-identical output |
| Invalid inputs (caller-supplied FMU path, unknown variable, negative step, missing FMU), 1.5 s timeout on a long run, conformance kit | typed errors; PASS |

A 1e-9 relative perturbation of the tool output fails 3 of the 7 tests (mutation check).

Limits: the Reference FMUs integrate with a compiled-in fixed solver step (0.1 s), so a smaller requested step does not refine the solution. Only FMI 2.0 co-simulation is wired. An FMU contains native code, so running one runs that code: only run FMUs from trusted sources. Status and Cancel are synchronous (hard timeout), not an asynchronous job API. The invocation record contains the FMU's local path.

The adapter trait gained `tool_input()`: the exact document sent to the tool. The conformance kit checks the recorded input hash against those bytes.

### CalculiX (K3)

`CalculixAdapter` runs `18-libraries/python/helqix_calculix/cli.py`, which writes a CalculiX input deck and starts `ccx` as a **separate program** (GPL boundary: no linking, not shipped with HELQIX; CI installs Ubuntu's `calculix-ccx`). The ccx path is fixed when the adapter is built and passed as a launcher argument; a request cannot choose which program runs. ccx runs single-threaded with a 300 s cap of its own.

Case (Example, a benchmark not a design): cantilever, 1 m long, 50 mm x 50 mm, E = 210 GPa, nu = 0.3, 1 kN tip load, clamped end, 20 x 2 x 2 C3D8I bricks (bending without shear locking). Output: mean tip deflection of the end-face nodes, plus two closed forms computed by the tool (CALCULATED): Euler-Bernoulli F L^3 / (3 E I) and Timoshenko (adds F L / (k G A), k = 5/6).

| Check | Pass rule |
|---|---|
| FE tip deflection vs Timoshenko closed form | within 2 % (ASSUMPTION for this mesh); the measured number is published as a CI notice |
| Doubling the load, doubling E | deflection ratio 2 within 1e-6 (linear static) |
| Two runs | identical output hash |
| Request names a program; Poisson ratio 0.7; missing ccx; 20 000-element run with a 0.8 s limit | typed errors (invalid input, exit 2, exit 3, timeout) |
| Conformance kit | PASS |

Python unit tests (no ccx needed, run everywhere): input validation, mesh counts, every brick has positive volume, the load sums to -1 kN on the end face, the `.dat` parser.

Limits: one benchmark geometry only; no general mesh import yet; the result is a model of a beam, not validation of a real part.

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
