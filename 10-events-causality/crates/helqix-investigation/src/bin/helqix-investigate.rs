//! Investigates a failure from a JSON input (helqix.investigation-input.v1) and prints the
//! report (helqix.investigation-report.v1). Usage: `helqix-investigate <input.json>`.
//! Exit status: 0 = report written, 1 = unreadable or invalid input.
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else { eprintln!("usage: helqix-investigate <input.json>"); return ExitCode::from(1) };
    let parsed = std::fs::read_to_string(&path).map_err(|e| e.to_string())
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).map_err(|e| e.to_string()))
        .and_then(|d| helqix_investigation::input_from_json(&d).map_err(|e| e.to_string()));
    let input = match parsed { Ok(i) => i, Err(e) => { eprintln!("invalid input {path}: {e}"); return ExitCode::from(1) } };
    match helqix_investigation::investigate(&input) {
        Ok(inv) => { println!("{}", serde_json::to_string_pretty(&helqix_investigation::report_json(&input, &inv)).unwrap_or_default()); ExitCode::SUCCESS }
        Err(e) => { eprintln!("{e}"); ExitCode::from(1) }
    }
}
