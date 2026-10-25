//! Usage: `helqix-evidence-verify <package-dir> --trusted-key <hex> | --trusted-key-file <path>`
//! Prints PASS or FAIL with every reason. Exit status 0 = PASS, 2 = FAIL, 1 = usage error.
use std::process::ExitCode;

use helqix_evidence_verify::{verify_package, SCOPE_NOTE};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let key = match args.as_slice() {
        [_, flag, k] if flag == "--trusted-key" => k.trim().to_owned(),
        [_, flag, path] if flag == "--trusted-key-file" => match std::fs::read_to_string(path) { Ok(k) => k.trim().to_owned(), Err(e) => { eprintln!("cannot read key file: {e}"); return ExitCode::from(1) } },
        _ => { eprintln!("usage: helqix-evidence-verify <package-dir> --trusted-key <hex> | --trusted-key-file <path>"); return ExitCode::from(1) }
    };
    let report = verify_package(std::path::Path::new(&args[0]), &key);
    for (name, ok) in &report.checks { println!("{} {name}", if *ok { "ok  " } else { "FAIL" }); }
    for reason in &report.reasons { println!("reason: {reason}"); }
    println!("{}", if report.pass { "PASS" } else { "FAIL" });
    println!("{SCOPE_NOTE}");
    if report.pass { ExitCode::SUCCESS } else { ExitCode::from(2) }
}
