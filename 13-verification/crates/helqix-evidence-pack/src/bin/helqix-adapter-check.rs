//! Phase 6 reference run: PyBaMM adapter conformance, measured variation, closed-form cross-check,
//! and a signed Tolerance-level evidence package of one reference invocation.
//!
//! Usage: `helqix-adapter-check <python-with-pybamm> <out-dir> --key <secret-key-file> --signed-at <claimed time>`
//! Exit 0 when conformance passes, the cross-check is within 1e-6 A.h, and the package is written.
use std::{path::{Path, PathBuf}, process::ExitCode, time::Duration};

use ed25519_dalek::SigningKey;
use helqix_adapters::{pybamm::{measure_variation, PybammAdapter}, solver::{conformance, SolverAdapter}};
use helqix_evidence_pack::{write_package, PackageFile, Statement};
use serde_json::{json, Value};

fn reference_input() -> Value {
    let time: Vec<f64> = (0..=1800).step_by(30).map(f64::from).collect();
    json!({"schema": "helqix.pybamm-input.v1", "time_s": time, "current_a": vec![5.0; time.len()], "initial_ocv_v": 4.18, "contact_resistance_ohm": 0.0})
}

fn load_key(path: &str) -> Result<SigningKey, String> {
    let t = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let t = t.trim();
    let b: Vec<u8> = (0..t.len()).step_by(2).map(|i| u8::from_str_radix(t.get(i..i + 2).unwrap_or("zz"), 16)).collect::<Result<_, _>>().map_err(|_| "key is not hex".to_owned())?;
    Ok(SigningKey::from_bytes(&b.try_into().map_err(|_| "key must be 32 bytes".to_owned())?))
}

fn main() -> ExitCode {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let [python, out, flag_k, key, flag_t, at] = a.as_slice() else { eprintln!("usage: helqix-adapter-check <python> <out-dir> --key <key-file> --signed-at <time>"); return ExitCode::from(1) };
    if flag_k != "--key" || flag_t != "--signed-at" { eprintln!("usage error"); return ExitCode::from(1); }
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../18-libraries/python");
    let adapter = PybammAdapter::new(PathBuf::from(python), package, Duration::from_secs(600));
    let result = (|| -> Result<bool, String> {
        let input = reference_input();
        let bad = [json!({"schema": "helqix.pybamm-input.v1", "time_s": [0, 1], "current_a": [1], "initial_ocv_v": 4.18}), json!({"schema": "other"})];
        let conf = conformance(&adapter, &input, &bad);
        println!("conformance: {} {:?}", if conf.pass { "PASS" } else { "FAIL" }, conf.failures);
        let variation = measure_variation(&adapter, &input, 5).map_err(|e| e.to_string())?;
        println!("variation over {} runs: max |dV| = {:e} V; declared tolerance {:e} V ({})", variation.runs, variation.max_abs_deviation_v, variation.declared_tolerance_v, variation.margin_rule);
        let run = adapter.run(&input).map_err(|e| e.to_string())?;
        let q = &run.outputs.iter().find(|o| o.name == "discharge_capacity").ok_or("no capacity output")?.values;
        let cross = q.iter().enumerate().map(|(i, v)| (v - 5.0 * (i as f64 * 30.0) / 3600.0).abs()).fold(0.0_f64, f64::max);
        println!("cross-check vs closed form (Q = I t): max error {cross:e} A.h");
        println!("invocation: tool {} {}, input {}, output {}, {} ms", run.record.tool, run.record.tool_version, run.record.input_sha256, run.record.output_sha256, run.record.wall_time_ms);
        let r = &run.record;
        let to = |v: &Value| serde_json::to_vec_pretty(v).map_err(|e| e.to_string());
        let files = vec![
            PackageFile { path: "invocation_record.json".into(), role: "invocation_record".into(), bytes: to(&json!({"tool": r.tool, "tool_version": r.tool_version, "input_sha256": r.input_sha256, "configuration": r.configuration, "wall_time_ms": r.wall_time_ms as u64, "output_sha256": r.output_sha256, "exit_status": r.exit_status, "stderr_summary": r.stderr_summary}))? },
            PackageFile { path: "tool_input.json".into(), role: "tool_input".into(), bytes: serde_json::to_vec(&input).map_err(|e| e.to_string())? },
            PackageFile { path: "tool_outputs.json".into(), role: "tool_outputs".into(), bytes: run.raw_output.clone() },
            PackageFile { path: "adapter_checks.json".into(), role: "adapter_checks".into(), bytes: to(&json!({"conformance": {"pass": conf.pass, "failures": conf.failures},
                "variation": {"runs": variation.runs, "max_abs_deviation_v": variation.max_abs_deviation_v, "declared_tolerance_v": variation.declared_tolerance_v, "margin_rule": variation.margin_rule},
                "cross_check": {"closed_form": "discharge capacity = I * t / 3600", "max_error_ah": cross}, "supports_state_restore": adapter.contract().supports_state_restore}))? },
            PackageFile { path: "verdict.json".into(), role: "verdict".into(), bytes: to(&json!({"label": "NotValidated", "determinism": "Tolerance",
                "reason": "External solver run, schema-checked and recorded. The battery model is NotVerified for its Context of Use (Phase 1, held-out C/10 RMSE 71-77 mV); this run is not validation evidence."}))? },
        ];
        let statement = Statement { subject: "PyBaMM SPM / Chen2020 adapter reference run: 5 A for 1800 s from 4.18 V".into(),
            context_of_use: "Adapter conformance and provenance demonstration; not a validated physical prediction.".into(), determinism: "Tolerance".into(),
            verdict: format!("Conformance {}; charge-conservation cross-check max error {cross:e} A.h; declared tolerance {:e} V", if conf.pass { "PASS" } else { "FAIL" }, variation.declared_tolerance_v),
            label: "NotValidated".into(), final_state_hash: r.output_sha256.clone(), failure_index: 0 };
        let s = write_package(Path::new(out), files, &statement, "helqix-dev-windows", at, &load_key(key)?).map_err(|e| e.to_string())?;
        println!("package {out}: merkle root {}, public key {}", s.merkle_root, s.public_key);
        Ok(conf.pass && cross < 1e-6)
    })();
    match result { Ok(true) => { println!("PASS"); ExitCode::SUCCESS } Ok(false) => { println!("FAIL"); ExitCode::from(2) } Err(e) => { eprintln!("{e}"); ExitCode::from(1) } }
}
