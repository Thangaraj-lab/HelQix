//! Evidence package builder: files + Merkle manifest + Ed25519 signature, and a re-execution check.
//!
//! The package format is specified in the independent verifier (`helqix-evidence-verify`). This
//! crate implements the writing side separately on purpose: if builder and verifier ever disagree,
//! the round-trip test fails.
//!
//! Keys: [`generate_key`] makes a **development** key from the operating system's random source.
//! Keys are never written into a package or committed; production key custody is an open founder
//! decision. `signed_at` is a claim supplied by the signer, not a proven time.
#![forbid(unsafe_code)]

use std::{fs, path::Path};

use ed25519_dalek::{Signer, SigningKey};
use helqix_branch::intervention::{intervention_test, Budget, Experiments, Intervention, Parameter};
use helqix_causal::CandidateMethod;
use helqix_core::id::Identifier;
use helqix_freeze::{detect_failure, freeze, load_freeze, replay, run, save_freeze, Comparator, FreezeTrigger, PointMassSpec, RunSpec, StateLimit};
use helqix_investigation::{investigate, report_json, Comparator as InvComparator, InvestigationInput, NodeSeries, Relation, Requirement, StateReference};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// One file of a package.
#[derive(Debug, Clone, PartialEq)]
pub struct PackageFile { pub path: String, pub role: String, pub bytes: Vec<u8> }

/// What the package claims (written into the manifest and therefore signed).
#[derive(Debug, Clone, PartialEq)]
pub struct Statement { pub subject: String, pub context_of_use: String, pub determinism: String, pub verdict: String, pub label: String, pub final_state_hash: String, pub failure_index: usize }

/// Result of writing a package.
#[derive(Debug, Clone, PartialEq)]
pub struct PackageSummary { pub merkle_root: String, pub manifest_sha256: String, pub public_key: String }

/// Builder error.
#[derive(Debug, Clone, PartialEq)]
pub enum PackError { Io(String), NotEmpty(String), Build(String) }

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::Io(m) => write!(f, "I/O error: {m}"), Self::NotEmpty(d) => write!(f, "output directory {d} is not empty"), Self::Build(m) => write!(f, "cannot build package: {m}") }
    }
}
impl std::error::Error for PackError {}

fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }
fn sha(b: &[u8]) -> String { hex(&Sha256::digest(b)) }
fn io(e: impl std::fmt::Display) -> PackError { PackError::Io(e.to_string()) }
fn build(e: impl std::fmt::Display) -> PackError { PackError::Build(e.to_string()) }
fn id(s: &str) -> Identifier { Identifier::parse(s).expect("static identifier") }

fn merkle_root(entries: &[(String, String)]) -> String {
    let mut level: Vec<String> = entries.iter().map(|(p, h)| sha(format!("helqix.leaf\0{p}\0{h}").as_bytes())).collect();
    if level.is_empty() { return sha(b"helqix.empty"); }
    while level.len() > 1 {
        level = level.chunks(2).map(|c| if c.len() == 2 { sha(format!("helqix.node\0{}{}", c[0], c[1]).as_bytes()) } else { c[0].clone() }).collect();
    }
    level.remove(0)
}

/// Development signing key from the operating system's random source.
pub fn generate_key() -> Result<SigningKey, PackError> {
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed).map_err(build)?;
    Ok(SigningKey::from_bytes(&seed))
}

/// Hex public key (what a verifier must be told to trust).
pub fn public_key_hex(key: &SigningKey) -> String { hex(key.verifying_key().as_bytes()) }

/// Writes `files`, the manifest and the signature into an empty or new directory.
pub fn write_package(dir: &Path, mut files: Vec<PackageFile>, statement: &Statement, signer: &str, signed_at: &str, key: &SigningKey) -> Result<PackageSummary, PackError> {
    if dir.exists() && fs::read_dir(dir).map_err(io)?.next().is_some() { return Err(PackError::NotEmpty(dir.display().to_string())); }
    fs::create_dir_all(dir).map_err(io)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    if files.windows(2).any(|w| w[0].path == w[1].path) { return Err(build("duplicate file path")); }
    let mut entries = Vec::new();
    for f in &files {
        fs::write(dir.join(&f.path), &f.bytes).map_err(io)?;
        entries.push((f.path.clone(), f.role.clone(), sha(&f.bytes)));
    }
    let pairs: Vec<(String, String)> = entries.iter().map(|e| (e.0.clone(), e.2.clone())).collect();
    let root = merkle_root(&pairs);
    let manifest = json!({
        "schema": "helqix.evidence-manifest.v1",
        "entries": entries.iter().map(|(p, r, h)| json!({"path": p, "role": r, "sha256": h})).collect::<Vec<_>>(),
        "merkle_root": root,
        "statement": {"subject": statement.subject, "context_of_use": statement.context_of_use, "determinism": statement.determinism, "verdict": statement.verdict,
                      "label": statement.label, "final_state_hash": statement.final_state_hash, "failure_index": statement.failure_index},
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(build)?;
    fs::write(dir.join("manifest.json"), &manifest_bytes).map_err(io)?;
    let manifest_sha = sha(&manifest_bytes);
    let message = format!("helqix.evidence.v1\n{manifest_sha}\n{signer}\n{signed_at}");
    let signature = key.sign(message.as_bytes());
    let public = public_key_hex(key);
    let sig = json!({"schema": "helqix.evidence-signature.v1", "algorithm": "Ed25519", "public_key": public, "key_id": &sha(key.verifying_key().as_bytes())[..16],
                     "signer": signer, "signed_at": signed_at, "signed_at_note": "claim by the signer; not a trusted timestamp",
                     "manifest_sha256": manifest_sha, "signature": hex(&signature.to_bytes())});
    fs::write(dir.join("signature.json"), serde_json::to_vec_pretty(&sig).map_err(build)?).map_err(io)?;
    Ok(PackageSummary { merkle_root: root, manifest_sha256: manifest_sha, public_key: public })
}

fn reference_spec() -> RunSpec {
    RunSpec {
        model: PointMassSpec { mass_kg: 0.5, gravity_m_s2: [0.0, -9.80665, 0.0], air_density_kg_m3: 1.225, wind_m_s: [0.0; 3], applied_force_n: [0.0; 3], drag_coefficient: 0.47, reference_area_m2: 0.01 },
        start_seconds: 0.0, end_seconds: 4.0, step_seconds: 0.001, initial_state: vec![0.0, 20.0, 0.0, 3.0, 0.0, 0.0], seeds: vec![],
    }
}

fn freeze_bytes(f: &helqix_freeze::Freeze) -> Result<Vec<u8>, PackError> {
    // One path per call: concurrent builds of the same freeze must not delete each other's file.
    static CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("helqix-pack-freeze-{}-{n}-{}.json", std::process::id(), &f.sha256()[..16]));
    save_freeze(f, &path).map_err(build)?;
    let bytes = fs::read(&path).map_err(io)?;
    let _ = fs::remove_file(&path);
    Ok(bytes)
}

/// The core-runtime reference evidence chain (Phases 2 to 4): body dropped from 20 m.
pub fn reference_files() -> Result<(Vec<PackageFile>, Statement), PackError> {
    let spec = reference_spec();
    let limit = StateLimit { name: "stay above ground".into(), variable: 1, comparator: Comparator::AtLeast, threshold: 0.0 };
    let parent = run(&spec).map_err(build)?;
    let failure = detect_failure(&parent, &limit).ok_or_else(|| build("reference scenario has no failure"))?;
    let at_failure = freeze(&spec, &parent, failure.index, FreezeTrigger::Failure(failure.clone())).map_err(build)?;
    let final_hash = parent.chain.last().cloned().ok_or_else(|| build("empty run"))?;

    let series = |k: usize| parent.samples.iter().map(|s| s.state[k]).collect::<Vec<_>>();
    let input = InvestigationInput {
        times_seconds: parent.samples.iter().map(|s| s.time_seconds).collect(),
        nodes: vec![NodeSeries { id: id("velocity_x"), values: series(3) }, NodeSeries { id: id("velocity_y"), values: series(4) }, NodeSeries { id: id("height"), values: series(1) }],
        relations: vec![
            Relation { cause: id("velocity_x"), effect: id("velocity_y"), relation: id("quadratic_drag"), parameters: vec![("drag_coefficient".into(), 0.47), ("reference_area_m2".into(), 0.01), ("air_density_kg_m3".into(), 1.225), ("mass_kg".into(), 0.5)] },
            Relation { cause: id("velocity_y"), effect: id("height"), relation: id("kinematics_y"), parameters: vec![] },
        ],
        requirement: Requirement { id: id("stay_above_ground"), node: id("height"), comparator: InvComparator::AtLeast, threshold: 0.0 },
        state_reference: Some(StateReference { freeze_sha256: at_failure.sha256(), index: failure.index }),
        method: CandidateMethod { min_relative_change: 0.05, onset_fraction: 0.1, path_limit: 64 },
    };
    let investigation = investigate(&input).map_err(build)?;

    let early = freeze(&spec, &parent, 500, FreezeTrigger::TimeMarker(0.5)).map_err(build)?;
    let mut ex = Experiments::new(Budget { max_branches: 10, max_total_steps: 100_000 });
    let mut interventions = Vec::new();
    for (label, iv) in [("stop_vertical_velocity", Intervention::State { variable: 4, value: 0.0 }), ("move_out_of_plane", Intervention::State { variable: 2, value: 5.0 }), ("more_drag", Intervention::Parameter { parameter: Parameter::DragCoefficient, value: 1.2 })] {
        let r = intervention_test(&mut ex, &parent, &early, &limit, id(label), iv.clone(), 0.005).map_err(build)?;
        interventions.push(json!({"branch": label, "intervention": format!("{iv:?}"), "parent_failure_seconds": r.parent_failure_seconds, "branch_failure_seconds": r.branch_failure_seconds,
                                   "verdict": format!("{:?}", r.verdict), "grade": r.grade, "parent_freeze_sha256": early.sha256()}));
    }

    let to_bytes = |v: &Value| serde_json::to_vec_pretty(v).map_err(build);
    let files = vec![
        PackageFile { path: "freeze.json".into(), role: "freeze".into(), bytes: freeze_bytes(&at_failure)? },
        PackageFile { path: "state_chain.json".into(), role: "state_chain".into(), bytes: to_bytes(&json!({"genesis": parent.genesis, "samples": parent.samples.len(), "determinism": parent.determinism.label(), "links": parent.chain, "final": final_hash}))? },
        PackageFile { path: "requirement_outcome.json".into(), role: "requirement_outcome".into(), bytes: to_bytes(&json!({"requirement": "stay_above_ground", "node": "height", "comparator": "at_least", "threshold": 0.0,
            "failure": {"index": failure.index, "time_seconds": failure.time_seconds, "value": failure.value}}))? },
        PackageFile { path: "investigation.json".into(), role: "causal_chains".into(), bytes: to_bytes(&report_json(&input, &investigation))? },
        PackageFile { path: "interventions.json".into(), role: "branch_records".into(), bytes: to_bytes(&json!({"freeze_index": 500, "freeze_time_seconds": 0.5, "results": interventions}))? },
        PackageFile { path: "verdict.json".into(), role: "verdict".into(), bytes: to_bytes(&json!({
            "label": "NotValidated", "causal_grade": "model-causal", "candidate": "velocity_y -> height",
            "reason": "The requirement is violated in the model and the candidate cause is supported by an intervention inside the model. No measured data exists for this model, so nothing here is validated.",
            "validation_evidence": null}))? },
    ];
    let statement = Statement {
        subject: "core-runtime reference: 0.5 kg body dropped from 20 m with quadratic drag".into(),
        context_of_use: "Demonstration of the HELQIX evidence chain on the deterministic core runtime; not a validated physical prediction.".into(),
        determinism: "Bitwise".into(),
        verdict: format!("Requirement 'stay above ground' violated at t = {} s in the model; candidate cause velocity_y -> height supported by intervention (model-causal).", failure.time_seconds),
        label: "NotValidated".into(),
        final_state_hash: final_hash,
        failure_index: failure.index,
    };
    Ok((files, statement))
}

/// Re-executes from the package's freeze and checks the replay ends on the recorded final hash (Bitwise).
pub fn reexecute(dir: &Path) -> Result<bool, PackError> {
    let frozen = load_freeze(&dir.join("freeze.json")).map_err(build)?;
    let chain: Value = serde_json::from_slice(&fs::read(dir.join("state_chain.json")).map_err(io)?).map_err(build)?;
    let replayed = replay(&frozen).map_err(build)?;
    Ok(replayed.chain.last().map(String::as_str) == chain["final"].as_str())
}
