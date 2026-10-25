//! Usage: `helqix-evidence-verify <package-dir> --trusted-key <hex> | --trusted-key-file <path> [--attestation <envelope.json>]`
//! Prints PASS or FAIL with every reason. Exit status 0 = PASS, 2 = FAIL, 1 = usage error.
//! With `--attestation`, the in-toto DSSE attestation beside the package is verified too (offline).
use std::process::ExitCode;

use helqix_evidence_verify::{attestation::verify_attestation, verify_package, SCOPE_NOTE};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = || { eprintln!("usage: helqix-evidence-verify <package-dir> --trusted-key <hex> | --trusted-key-file <path> [--attestation <envelope.json>]"); ExitCode::from(1) };
    let (key, attestation) = match args.as_slice() {
        [_, flag, k, rest @ ..] if flag == "--trusted-key" => (k.trim().to_owned(), rest),
        [_, flag, path, rest @ ..] if flag == "--trusted-key-file" => match std::fs::read_to_string(path) { Ok(k) => (k.trim().to_owned(), rest), Err(e) => { eprintln!("cannot read key file: {e}"); return ExitCode::from(1) } },
        _ => return usage(),
    };
    let attestation = match attestation {
        [] => None,
        [flag, path] if flag == "--attestation" => match std::fs::read(path) { Ok(b) => Some(b), Err(e) => { eprintln!("cannot read attestation: {e}"); return ExitCode::from(1) } },
        _ => return usage(),
    };
    let dir = std::path::Path::new(&args[0]);
    let report = verify_package(dir, &key);
    for (name, ok) in &report.checks { println!("{} {name}", if *ok { "ok  " } else { "FAIL" }); }
    for reason in &report.reasons { println!("reason: {reason}"); }
    let mut pass = report.pass;
    if let Some(bytes) = attestation {
        let a = verify_attestation(dir, &bytes, &key);
        for (name, ok) in &a.checks { println!("{} attestation: {name}", if *ok { "ok  " } else { "FAIL" }); }
        for reason in &a.reasons { println!("reason: attestation: {reason}"); }
        pass &= a.pass;
    }
    println!("{}", if pass { "PASS" } else { "FAIL" });
    println!("{SCOPE_NOTE}");
    if pass { ExitCode::SUCCESS } else { ExitCode::from(2) }
}
