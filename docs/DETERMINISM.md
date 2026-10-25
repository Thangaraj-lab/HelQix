# Determinism: what is guaranteed, and what is not (Phase 2)

Every run declares a level (`helqix_core::determinism::DeterminismLevel`, locked principle 4).

| Level | Who | Meaning | How it is checked |
|---|---|---|---|
| **Bitwise** | Core runtime: `helixsim` fixed-step kernel, `helqix-freeze` | Same build, same platform, same inputs give bit-identical states | Hash chain over every sample (grid index, time bits, state bits); `helqix-replay-check` |
| **Tolerance** | External solvers (PyBaMM battery pack, future adapters) | Results agree within a stated absolute tolerance | Re-run and compare within the tolerance; record tool, version, configuration, input and output hashes |
| **Statistical** | Stochastic or quantum backends (none yet) | Only distributions reproduce | Not decidable sample by sample |

## Scope of the Bitwise claim (VERIFIED by tests and by the run-twice command)

Guaranteed: the same binary on the same platform, given the same `RunSpec`, produces the same sample bits and the same hash chain, and a replay from a freeze reproduces the original continuation bit for bit, including the failure event at the same grid index and time bits.

**Not claimed:**
- Cross-platform or cross-compiler bit identity (different CPUs, operating systems, Rust versions, optimisation levels). The Linux CI job runs the same check; matching hashes between Windows and Linux would be evidence for those two builds only, not a general guarantee.
- Anything about physical correctness. "Determinism is not correctness" (plan/01): a wrong model replays its wrong answer exactly.

## Sources of nondeterminism audited (helixsim, helqix-freeze)

| Source | Status |
|---|---|
| Wall-clock time, environment variables, global mutable state | Not used (search: no `SystemTime`, `Instant`, `env::var`, statics with mutation) |
| Hash-map iteration order | Not used in the run path (no `HashMap`); freeze encoding uses fixed key order |
| Threads and scheduling order | The kernel is single-threaded; a test runs the same spec on 4 threads concurrently and gets identical chains |
| Randomness | None; `RunSpec.seeds` is recorded (empty for the core runtime) |
| Time-grid drift | Every timestamp is `start + index * step` (never accumulated). Restarting a fresh run at a frozen time would use `t_k + j * step`, which a test shows drifts; replay therefore uses `helixsim::kernel::resume` on the original grid |
| Floating-point contraction (fused multiply-add) | Rust does not contract `a * b + c` into FMA unless `mul_add` is called; the kernel does not call it |
| Reduction order | Fixed sequential loops; no parallel sums |
| Platform maths library (`libm`) | `sqrt` is correctly rounded (IEEE 754). The kernel uses no `exp`, `ln`, `powf`, `sin`, `cos` in the point-mass path; any model that does would depend on the platform `libm` and is not covered across platforms |
| Serialisation of floats | Freezes store floats as their 64-bit patterns in hex; JSON number parsing in the Rust trust layer uses `float_roundtrip` (Phase 1 finding) |

## Freeze content (content-addressed, immutable)

Model parameters and model hash, solver name, version, method and step bound, scenario time grid, initial state, seeds, grid index and time of the frozen state, the state itself, the hash-chain link before it, and the trigger (failure event, requirement violation, manual, time marker). `load_freeze` recomputes the SHA-256 and refuses a file whose content changed.

## External solvers (Tolerance)

The battery pack (Phase 1) records the PyBaMM version, options, tolerances, mesh and parameter overrides in a model identity hash, the dataset file hashes, and the split hash; its declared level is Tolerance. Two complete runs on 2026-10-25 produced byte-identical reports on one machine, which is consistent with but does not upgrade the declared level. FMI state save and restore is optional in practice and is not relied on.

## Reproduce

```
cargo run -p helqix-freeze --bin helqix-replay-check
```
Expected: two identical final hashes, the failure at index 1793 (t = 1.793 s), an identical replay, and `PASS`.
