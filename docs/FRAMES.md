# Run frames, content addressing and replay (K4)

Crate: `07-time/crates/helqix-frames`. Routes in `22-api/helqix-api/src/frames.rs`.

## Storage

A recorded run is stored as **one Parquet file** (Apache Parquet via parquet-rs 60, Apache-2.0; no compression codecs, so no C libraries in the build). Each row is one frame:

| Column | Type | Meaning |
|---|---|---|
| `index` | u64 | grid index |
| `t` | f64 | time, s |
| `state.<name>` | f64 | one column per state variable, stored as exact bits |
| `link` | text | the state-chain link of this frame (SHA-256, `helqix.chain.v1`, same as Phase 2) |

File metadata (`helqix.frames.v1`) holds the run id, variable names and units, determinism level, row count, the link before the first frame (`prior_link`), and the final link.

**Content addressing:** the file is stored as `frames/<BLAKE3 of its bytes>.parquet`. A run index `runs/<run id>.json` points to it. The same run always produces the same bytes (fixed writer name, no timestamps), so storing it twice is a no-op. Rebinding a run id to different frames is refused.

**Every read verifies:** the file's BLAKE3 must equal its address and the index; the metadata must equal the index; and the whole state chain is recomputed from `prior_link`, frame by frame, and must end at `final_link`. Frames must be consecutive in index and strictly increasing in time.

## getFrame rule (deterministic)

| Requested `t` | Result |
|---|---|
| bit-equal to a stored time | that frame; values Origin `simulated`; `stateSha256` = its chain link |
| strictly between two stored times | per variable `a + (b - a) * ((t - t0) / (t1 - t0))` in f64; values Origin **`derived`** (the solver did not compute it); no chain link; `index` = the earlier frame |
| before the first, after the last, NaN or infinite | error (HTTP 422) |

Scrubbing is a pure read: the same `t` always returns the same bytes, backwards or forwards, and reading never changes the stored run (tested).

## API

`HELQIX_FRAME_STORE` sets the store directory; without it the routes answer 503.

| Route | Does |
|---|---|
| `GET /v1/runs/{id}/frames?from=&limit=` | `runFrames`: stored frames in order, default 1000, max 10 000, with `next` for paging |
| `GET /v1/runs/{id}/frame?t=` | `getFrame` by the rule above |

A tampered file answers 409 ("stored frames failed verification"), never stale or partial data. Run ids are letters, digits, `-` and `_` only (no path tricks).

`helqix-frames-record <store> [run-id]` records the Phase 2 reference drop (0.5 kg from 20 m, 2 s at 1 ms) into a store.

## Replay hash-match

Test `a_replay_from_a_freeze_matches_the_stored_frames_hash_for_hash`: run, store, freeze at index 500, replay from the freeze, store the replay. Every replayed frame equals the stored frame in index, time bits, state bits and chain link; the final links match. Replaying from index 0 gives a frame file with the **identical BLAKE3** to the original.

## Measurements

Release build, founder's PC (AMD laptop, Windows 11), point-mass reference run with six state variables. MEASURED with `cargo run --release -p helqix-frames --bin helqix-frames-bench`; CI publishes the Linux runner's numbers as a notice.

| Frames | File | Per frame | Encode | Load + verify | getFrame | runFrames |
|---|---|---|---|---|---|---|
| 2 001 | 255 KB | 127 B | 1.6 ms | 4.7 ms | 0.65 µs | 0.59 µs per frame |
| 20 001 | 2.58 MB | 129 B | 10.5 ms | 40.8 ms | 0.72 µs | 0.66 µs per frame |

About half of each frame is the chain link stored as 64 hex characters; storing it as 32 raw bytes and adding a pure-Rust codec would roughly halve the size (not done).

## Tests

`helqix-frames`: 8 tests (bit-exact round trip, stable address, changed byte refused, address-only change refused, rehashed forgery fails the chain, getFrame rule, runFrames order, replay hash-match). Three mutations caught: skipping chain verification, skipping the BLAKE3 check, changing the interpolation weight. `helqix-api/tests/frames_api.rs`: routes through the real router (paging, exact and derived frames, 422, 404, 400, tampered file 409, 503 without a store); needs no database.

## Not done in K4

Object storage in SeaweedFS (frames are on a local directory), run metadata in PostgreSQL, DuckDB analysis queries, Zarr for fields, and a run endpoint that records frames from an API request. Frames are stored after a run completes, not streamed while it runs.
