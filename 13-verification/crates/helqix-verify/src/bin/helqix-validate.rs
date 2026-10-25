//! Judges a validation report and optionally writes a model-library entry.
//!
//! Usage: `helqix-validate <report.json> [--library-entry <entry.json>]`
//! Exit status: 0 = VerifiedForContext, 2 = NotVerified, 1 = unreadable or malformed report.
use std::{env, fs, process::ExitCode};
use helqix_verify::{credibility::{CaseRole, CredibilityLabel}, report::{assess_report, library_entry, parse_report}};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let (report_path, entry_path) = match args.as_slice() {
        [report] => (report.clone(), None),
        [report, flag, entry] if flag == "--library-entry" => (report.clone(), Some(entry.clone())),
        _ => { eprintln!("usage: helqix-validate <report.json> [--library-entry <entry.json>]"); return ExitCode::from(1); }
    };
    let report: serde_json::Value = match fs::read_to_string(&report_path).map_err(|e| e.to_string()).and_then(|t| serde_json::from_str(&t).map_err(|e| e.to_string())) {
        Ok(value) => value,
        Err(error) => { eprintln!("cannot read report {report_path}: {error}"); return ExitCode::from(1); }
    };
    let parsed = match parse_report(&report) { Ok(p) => p, Err(error) => { eprintln!("malformed report: {error}"); return ExitCode::from(1); } };
    let assessment = assess_report(&parsed);

    let unit = parsed.validation.output_unit;
    println!("Context of use: {}", parsed.validation.context.statement);
    println!("Requirement: {}", parsed.validation.requirement.statement);
    for outcome in &assessment.outcomes {
        let role = match outcome.role { CaseRole::Validation => "held-out", CaseRole::Calibration => "calibration" };
        println!("  {:<12} {:<20} RMSE {:>10.6} {unit}  peak {:>10.6} {unit}", role, outcome.case_id.as_str(), outcome.rmse, outcome.peak_abs_error);
    }
    for reason in &assessment.reasons { println!("  reason: {reason}"); }
    let verified = assessment.label == CredibilityLabel::VerifiedForContext;
    println!("LABEL: {}", if verified { "VerifiedForContext" } else { "NotVerified" });
    println!("Note: a label covers only the stated context of use. Signing (later phase) would prove origin and integrity, not physical correctness.");

    if let Some(path) = entry_path {
        let entry = library_entry(&report, &assessment);
        if let Err(error) = fs::write(&path, serde_json::to_string_pretty(&entry).unwrap_or_default() + "\n") { eprintln!("cannot write library entry {path}: {error}"); return ExitCode::from(1); }
    }
    if verified { ExitCode::SUCCESS } else { ExitCode::from(2) }
}
