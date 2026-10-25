//! K4 tests: Parquet round trip is bit-exact, content addresses are stable, tampering is refused,
//! getFrame follows the documented rule, and a replay from a freeze matches the stored frames hash for hash.
use helqix_contracts::Origin;
use helqix_frames::{blake3_hex, decode, encode, point_mass_variables, FrameError, FrameStore};
use helqix_freeze::{chain_genesis, freeze, replay, run, FreezeTrigger, PointMassSpec, RunSpec};

fn spec() -> RunSpec {
    RunSpec {
        model: PointMassSpec { mass_kg: 0.5, gravity_m_s2: [0.0, -9.80665, 0.0], air_density_kg_m3: 1.225, wind_m_s: [0.0; 3], applied_force_n: [0.0; 3], drag_coefficient: 0.47, reference_area_m2: 0.01 },
        start_seconds: 0.0, end_seconds: 2.0, step_seconds: 0.001, initial_state: vec![0.0, 20.0, 0.0, 3.0, 0.0, 0.0], seeds: vec![],
    }
}

fn store(name: &str) -> FrameStore {
    let dir = std::env::temp_dir().join(format!("helqix-frames-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    FrameStore::open(dir).unwrap()
}

#[test]
fn a_stored_run_reads_back_bit_exact_with_its_chain() {
    let s = spec();
    let r = run(&s).unwrap();
    let st = store("roundtrip");
    let idx = st.put("ref", &r, &point_mass_variables(), &chain_genesis(&s)).unwrap();
    assert_eq!(idx.meta.rows as usize, r.samples.len());
    let loaded = st.load("ref").unwrap();
    for (f, s) in loaded.frames().iter().zip(&r.samples) {
        assert_eq!((f.index as usize, f.t.to_bits()), (s.index, s.time_seconds.to_bits()));
        assert_eq!(f.state.iter().map(|v| v.to_bits()).collect::<Vec<_>>(), s.state.iter().map(|v| v.to_bits()).collect::<Vec<_>>());
    }
    assert_eq!(loaded.meta.final_link, *r.chain.last().unwrap());
    assert_eq!(loaded.meta.determinism, "Bitwise");
}

#[test]
fn the_content_address_is_stable_and_names_the_file() {
    let s = spec();
    let r = run(&s).unwrap();
    let (a, _) = encode("ref", &r, &point_mass_variables(), &chain_genesis(&s)).unwrap();
    let (b, _) = encode("ref", &run(&s).unwrap(), &point_mass_variables(), &chain_genesis(&s)).unwrap();
    assert_eq!(blake3_hex(&a), blake3_hex(&b), "same run, same bytes");
    let st = store("address");
    let idx = st.put("ref", &r, &point_mass_variables(), &chain_genesis(&s)).unwrap();
    assert_eq!(idx.frames_blake3, blake3_hex(&a));
    assert!(st.frames_path(&idx.frames_blake3).exists());
    assert_eq!(st.put("ref", &r, &point_mass_variables(), &chain_genesis(&s)).unwrap(), idx, "putting the same run again is a no-op");
    let mut other = spec();
    other.model.drag_coefficient = 1.0;
    assert!(matches!(st.put("ref", &run(&other).unwrap(), &point_mass_variables(), &chain_genesis(&other)), Err(FrameError::Integrity(_))), "a run id cannot be silently rebound");
}

#[test]
fn a_changed_byte_is_refused() {
    let s = spec();
    let st = store("byte");
    let idx = st.put("ref", &run(&s).unwrap(), &point_mass_variables(), &chain_genesis(&s)).unwrap();
    let path = st.frames_path(&idx.frames_blake3);
    let mut bytes = std::fs::read(&path).unwrap();
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0x01;
    std::fs::write(&path, bytes).unwrap();
    assert!(matches!(st.load("ref"), Err(FrameError::Integrity(_))));
}

#[test]
fn a_valid_file_with_different_bytes_is_refused_by_its_address() {
    // Same frames and chain, still valid Parquet, only the writer name in the footer changed:
    // nothing but the content address can notice.
    let s = spec();
    let st = store("address-only");
    let idx = st.put("ref", &run(&s).unwrap(), &point_mass_variables(), &chain_genesis(&s)).unwrap();
    let path = st.frames_path(&idx.frames_blake3);
    let bytes = std::fs::read(&path).unwrap();
    let needle = b"helqix-frames 0.1";
    let at = bytes.windows(needle.len()).rposition(|w| w == needle).expect("writer name in the footer");
    let mut changed = bytes.clone();
    changed[at] = b'H';
    assert!(decode(&changed).is_ok(), "the changed file is still a valid, chain-consistent frame file");
    std::fs::write(&path, changed).unwrap();
    match st.load("ref") {
        Err(FrameError::Integrity(m)) => assert!(m.contains("does not match its address"), "{m}"),
        other => panic!("accepted: {other:?}"),
    }
}

#[test]
fn a_consistently_rehashed_forgery_fails_the_chain() {
    let s = spec();
    let mut r = run(&s).unwrap();
    r.samples[700].state[1] += 1e-9; // change a state value but keep the original chain links
    let (bytes, _) = encode("forged", &r, &point_mass_variables(), &chain_genesis(&s)).unwrap();
    match decode(&bytes) {
        Err(FrameError::Integrity(m)) => assert!(m.contains("700"), "{m}"),
        other => panic!("forgery accepted: {other:?}"),
    }
}

#[test]
fn get_frame_returns_stored_frames_on_the_grid_and_derived_values_between() {
    let s = spec();
    let r = run(&s).unwrap();
    let st = store("getframe");
    st.put("ref", &r, &point_mass_variables(), &chain_genesis(&s)).unwrap();
    let run_ = st.load("ref").unwrap();
    let on = &r.samples[1000];
    let f = run_.get_frame(on.time_seconds).unwrap();
    assert_eq!(f.index, 1000);
    assert_eq!(f.state["y"].v.to_bits(), on.state[1].to_bits());
    assert_eq!((f.state["y"].origin, f.state["y"].unit.as_str()), (Origin::Simulated, "m"));
    assert_eq!(f.state_sha256.as_deref(), Some(r.chain[1000].as_str()));

    let (a, b) = (&r.samples[1000], &r.samples[1001]);
    let t = a.time_seconds + 0.25 * (b.time_seconds - a.time_seconds);
    let g = run_.get_frame(t).unwrap();
    let w = (t - a.time_seconds) / (b.time_seconds - a.time_seconds);
    assert_eq!(g.state["vy"].v.to_bits(), (a.state[4] + (b.state[4] - a.state[4]) * w).to_bits(), "documented interpolation, bit for bit");
    assert_eq!((g.index, g.state["vy"].origin, g.state_sha256.clone()), (1000, Origin::Derived, None));
    assert!(g.state["y"].v < a.state[1] && g.state["y"].v > b.state[1], "falling body: between the neighbours");

    // Scrubbing is a pure read: t, then t again, then backwards gives identical frames.
    let again = run_.get_frame(t).unwrap();
    assert_eq!(serde_json::to_vec(&g).unwrap(), serde_json::to_vec(&again).unwrap());
    let back = run_.get_frame(r.samples[10].time_seconds).unwrap();
    assert_eq!(back.index, 10);
    assert_eq!(st.load("ref").unwrap(), run_, "reads never change the stored run");

    for bad in [-0.001, 2.5, f64::NAN, f64::INFINITY] { assert!(matches!(run_.get_frame(bad), Err(FrameError::OutOfRange(_))), "{bad}"); }
}

#[test]
fn run_frames_streams_every_stored_frame_in_order() {
    let s = spec();
    let r = run(&s).unwrap();
    let st = store("runframes");
    st.put("ref", &r, &point_mass_variables(), &chain_genesis(&s)).unwrap();
    let frames: Vec<_> = st.load("ref").unwrap().run_frames().collect();
    assert_eq!(frames.len(), r.samples.len());
    assert!(frames.windows(2).all(|w| w[1].index == w[0].index + 1));
    assert!(frames.iter().all(|f| f.state.values().all(|v| v.origin == Origin::Simulated) && f.state.len() == 6));
}

#[test]
fn a_replay_from_a_freeze_matches_the_stored_frames_hash_for_hash() {
    let s = spec();
    let original = run(&s).unwrap();
    let st = store("replay");
    st.put("original", &original, &point_mass_variables(), &chain_genesis(&s)).unwrap();
    let stored = st.load("original").unwrap();

    let fz = freeze(&s, &original, 500, FreezeTrigger::TimeMarker(0.5)).unwrap();
    let replayed = replay(&fz).unwrap();
    st.put("replay-500", &replayed, &point_mass_variables(), fz.prior_link()).unwrap();
    let back = st.load("replay-500").unwrap();

    assert_eq!(back.frames().len(), stored.frames().len() - 500);
    for (a, b) in stored.frames()[500..].iter().zip(back.frames()) {
        assert_eq!((a.index, a.t.to_bits(), &a.link), (b.index, b.t.to_bits(), &b.link), "frame {} differs", a.index);
        assert_eq!(a.state.iter().map(|v| v.to_bits()).collect::<Vec<_>>(), b.state.iter().map(|v| v.to_bits()).collect::<Vec<_>>());
    }
    assert_eq!(back.meta.final_link, stored.meta.final_link, "replay hash matches the original");
    // The same run replayed from index 0 produces the identical frame file.
    let fz0 = freeze(&s, &original, 0, FreezeTrigger::TimeMarker(0.0)).unwrap();
    let (bytes0, _) = encode("original", &replay(&fz0).unwrap(), &point_mass_variables(), fz0.prior_link()).unwrap();
    assert_eq!(blake3_hex(&bytes0), stored.frames_blake3);
}
