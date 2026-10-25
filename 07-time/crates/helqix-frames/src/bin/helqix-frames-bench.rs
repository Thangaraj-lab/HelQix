//! K4 measurements: stored size per frame, encode time, load-and-verify time, getFrame latency,
//! runFrames throughput, for the point-mass reference run at two lengths.
//! Usage: `helqix-frames-bench [store-dir]` (default: a temp directory). Prints a table; numbers are
//! measured on the machine that runs it (build profile matters: use --release for timings).
use std::time::Instant;

use helqix_frames::{encode, point_mass_variables, FrameStore};
use helqix_freeze::{chain_genesis, run, PointMassSpec, RunSpec};

fn main() {
    let dir = std::env::args().nth(1).map(std::path::PathBuf::from).unwrap_or_else(|| std::env::temp_dir().join(format!("helqix-frames-bench-{}", std::process::id())));
    let _ = std::fs::remove_dir_all(&dir);
    let store = FrameStore::open(&dir).expect("store");
    println!("profile: {}", if cfg!(debug_assertions) { "debug (timings pessimistic)" } else { "release" });
    println!("{:>8} {:>10} {:>9} {:>10} {:>12} {:>14} {:>14}", "frames", "bytes", "B/frame", "encode ms", "load+verify ms", "getFrame us", "runFrames us/f");
    for end in [2.0, 20.0] {
        let spec = RunSpec {
            model: PointMassSpec { mass_kg: 0.5, gravity_m_s2: [0.0, -9.80665, 0.0], air_density_kg_m3: 1.225, wind_m_s: [0.0; 3], applied_force_n: [0.0; 3], drag_coefficient: 0.47, reference_area_m2: 0.01 },
            start_seconds: 0.0, end_seconds: end, step_seconds: 0.001, initial_state: vec![0.0, 2000.0, 0.0, 3.0, 0.0, 0.0], seeds: vec![],
        };
        let r = run(&spec).expect("run");
        let id = format!("bench-{}", r.samples.len());
        let t0 = Instant::now();
        let (bytes, _) = encode(&id, &r, &point_mass_variables(), &chain_genesis(&spec)).expect("encode");
        let encode_ms = t0.elapsed().as_secs_f64() * 1e3;
        store.put(&id, &r, &point_mass_variables(), &chain_genesis(&spec)).expect("put");
        let t1 = Instant::now();
        let loaded = store.load(&id).expect("load");
        let load_ms = t1.elapsed().as_secs_f64() * 1e3;
        // Deterministic pseudo-random query times across the run.
        let (first, last) = (r.samples[0].time_seconds, r.samples[r.samples.len() - 1].time_seconds);
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        let queries = 20_000;
        let t2 = Instant::now();
        let mut sink = 0.0;
        for _ in 0..queries {
            x ^= x << 13; x ^= x >> 7; x ^= x << 17;
            let t = first + (last - first) * ((x >> 11) as f64 / (1u64 << 53) as f64);
            sink += loaded.get_frame(t).expect("frame").state["y"].v;
        }
        let get_us = t2.elapsed().as_secs_f64() * 1e6 / queries as f64;
        let t3 = Instant::now();
        let n = loaded.run_frames().inspect(|f| sink += f.t.v).count();
        let iter_us = t3.elapsed().as_secs_f64() * 1e6 / n as f64;
        println!("{:>8} {:>10} {:>9.1} {:>10.2} {:>12.2} {:>14.3} {:>14.3}", r.samples.len(), bytes.len(), bytes.len() as f64 / r.samples.len() as f64, encode_ms, load_ms, get_us, iter_us);
        std::hint::black_box(sink);
    }
    println!("store: {}", dir.display());
}
