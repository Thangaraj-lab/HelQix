//! Validation-report boundary: parses an untrusted JSON report produced by a
//! domain pack (for example a Python solver adapter) into a [`ModelValidation`]
//! and turns the Rust assessment into a model-library entry. Nothing in the
//! report is trusted as a verdict: errors are recomputed and the split re-hashed.
use helqix_core::{id::Identifier, quantity::Quantity, units::Unit};
use serde_json::{json, Value};
use crate::{credibility::{assess, CaseRole, ComparisonCase, ContextOfUse, CredibilityAssessment, CredibilityLabel, ModelValidation, RiskLevel, SplitRecord}, requirement::{Comparison, Requirement, RequirementPriority}};

/// Report schema this parser accepts.
pub const REPORT_SCHEMA: &str = "helqix.validation-report.v1";
/// Library-entry schema this module writes.
pub const LIBRARY_ENTRY_SCHEMA: &str = "helqix.model-library-entry.v1";

/// A parsed report: the validation to judge plus the reported numerical-integrity outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedReport { pub validation: ModelValidation, pub numerical_integrity_passed: bool }

/// Parses and validates the structure of a report. Unknown or missing fields are errors.
pub fn parse_report(report: &Value) -> Result<ParsedReport, String> {
    if text(report, "schema")? != REPORT_SCHEMA { return Err(format!("unsupported report schema; expected {REPORT_SCHEMA}")); }
    let requirement = parse_requirement(field(report, "requirement")?)?;
    let context = field(report, "context_of_use")?;
    let risk = match text(context, "risk")? { "Low" => RiskLevel::Low, "Medium" => RiskLevel::Medium, "High" => RiskLevel::High, other => return Err(format!("unknown risk level {other}")) };
    let context = ContextOfUse::new(text(context, "statement")?, risk).map_err(|e| e.to_string())?;
    let output_unit = Unit::parse(text(report, "output_unit")?).map_err(|e| e.to_string())?;
    let split = field(report, "split")?;
    let fitted_on = field(split, "fitted_on")?.as_array().ok_or("split.fitted_on must be an array")?
        .iter().map(|v| v.as_str().ok_or_else(|| "fitted_on entries must be strings".to_owned()).and_then(identifier)).collect::<Result<Vec<_>, _>>()?;
    let split = SplitRecord { split_sha256: text(split, "sha256")?.to_owned(), fitted_on };
    let cases = field(report, "cases")?.as_array().ok_or("cases must be an array")?.iter().map(parse_case).collect::<Result<Vec<_>, _>>()?;
    let model_sha256 = text(field(report, "model")?, "sha256")?.to_owned();
    let numerical_integrity_passed = report.get("numerical_integrity").and_then(|n| n.get("passed")).and_then(Value::as_bool).unwrap_or(false);
    Ok(ParsedReport { validation: ModelValidation { requirement, context, model_sha256, output_unit, split, cases }, numerical_integrity_passed })
}

/// Assesses a parsed report. A failed or missing numerical-integrity check refuses verification.
pub fn assess_report(parsed: &ParsedReport) -> CredibilityAssessment {
    let mut assessment = assess(&parsed.validation);
    if !parsed.numerical_integrity_passed {
        assessment.reasons.push("numerical integrity check failed or is missing: solution accuracy is not established".to_owned());
        assessment.label = CredibilityLabel::NotVerified;
    }
    assessment
}

/// Builds the model-library entry: identity, data and split hashes, achieved error, context, label.
pub fn library_entry(report: &Value, assessment: &CredibilityAssessment) -> Value {
    let errors = |role: CaseRole| -> Vec<Value> {
        assessment.outcomes.iter().filter(|o| o.role == role)
            .map(|o| json!({"case": o.case_id.as_str(), "rmse": o.rmse, "peak_abs_error": o.peak_abs_error})).collect()
    };
    json!({
        "schema": LIBRARY_ENTRY_SCHEMA,
        "label": match assessment.label { CredibilityLabel::VerifiedForContext => "VerifiedForContext", CredibilityLabel::NotVerified => "NotVerified" },
        "reasons": assessment.reasons,
        "model": report.get("model").cloned().unwrap_or(Value::Null),
        "dataset": report.get("dataset").cloned().unwrap_or(Value::Null),
        "split_sha256": report.get("split").and_then(|s| s.get("sha256")).cloned().unwrap_or(Value::Null),
        "fitted_on": report.get("split").and_then(|s| s.get("fitted_on")).cloned().unwrap_or(Value::Null),
        "requirement": report.get("requirement").cloned().unwrap_or(Value::Null),
        "context_of_use": report.get("context_of_use").cloned().unwrap_or(Value::Null),
        "output_unit": report.get("output_unit").cloned().unwrap_or(Value::Null),
        "held_out_errors": errors(CaseRole::Validation),
        "calibration_errors": errors(CaseRole::Calibration),
        "assessed_by": format!("helqix-verify {}", env!("CARGO_PKG_VERSION")),
        "report_date": report.get("date").cloned().unwrap_or(Value::Null),
    })
}

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a Value, String> { value.get(name).ok_or_else(|| format!("missing field {name}")) }
fn text<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> { field(value, name)?.as_str().ok_or_else(|| format!("field {name} must be a string")) }
fn identifier(value: &str) -> Result<Identifier, String> { Identifier::parse(value).map_err(|e| format!("invalid identifier {value:?}: {e}")) }

fn numbers(value: &Value, name: &str) -> Result<Vec<f64>, String> {
    field(value, name)?.as_array().ok_or_else(|| format!("field {name} must be an array"))?
        .iter().map(|v| v.as_f64().ok_or_else(|| format!("field {name} must contain only numbers"))).collect()
}

fn parse_case(value: &Value) -> Result<ComparisonCase, String> {
    let role = match text(value, "role")? { "calibration" => CaseRole::Calibration, "validation" => CaseRole::Validation, other => return Err(format!("unknown case role {other}")) };
    Ok(ComparisonCase { id: identifier(text(value, "id")?)?, role, time_s: numbers(value, "time_s")?, input: numbers(value, "input")?, measured: numbers(value, "measured")?, simulated: numbers(value, "simulated")? })
}

fn parse_requirement(value: &Value) -> Result<Requirement, String> {
    let comparison = match text(value, "comparison")? {
        "LessThan" => Comparison::LessThan, "LessOrEqual" => Comparison::LessOrEqual, "Equal" => Comparison::Equal,
        "GreaterOrEqual" => Comparison::GreaterOrEqual, "GreaterThan" => Comparison::GreaterThan, other => return Err(format!("unknown comparison {other}")),
    };
    let threshold = field(value, "threshold")?;
    let unit = Unit::parse(text(threshold, "unit")?).map_err(|e| e.to_string())?;
    let amount = field(threshold, "value")?.as_f64().ok_or("threshold.value must be a number")?;
    let threshold = Quantity::new(amount, unit).map_err(|e| e.to_string())?;
    let mut requirement = Requirement::draft(identifier(text(value, "id")?)?, text(value, "statement")?, RequirementPriority::Must, identifier(text(value, "metric")?)?, comparison, threshold).map_err(|e| e.to_string())?;
    requirement.set_accepted(field(value, "accepted")?.as_bool().ok_or("accepted must be a boolean")?).map_err(|e| e.to_string())?;
    Ok(requirement)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credibility::split_sha256;

    fn series_case(name: &str, role: &str, offset: f64) -> Value {
        json!({"id": name, "role": role, "time_s": [0.0, 10.0, 20.0], "input": [0.5, 0.5, 0.5], "measured": [4.1, 4.0, 3.9], "simulated": [4.1 + offset, 4.0 + offset, 3.9 + offset]})
    }

    fn report(offset: f64, integrity: bool) -> Value {
        let cases = vec![series_case("c1_c10", "validation", offset), series_case("c1_1c", "calibration", 0.2)];
        let mut r = json!({
            "schema": REPORT_SCHEMA,
            "model": {"sha256": "b".repeat(64), "name": "test model"},
            "requirement": {"id": "voltage_rmse_limit", "statement": "RMSE below 46 mV", "metric": "voltage_rmse", "comparison": "LessThan", "threshold": {"value": 46.0, "unit": "mV"}, "accepted": true},
            "context_of_use": {"statement": "Predict voltage at C/10", "risk": "Low"},
            "output_unit": "V",
            "split": {"sha256": "", "fitted_on": ["c1_1c"]},
            "numerical_integrity": {"passed": integrity},
            "dataset": {"doi": "10.0/example"},
            "cases": cases
        });
        let parsed = parse_report(&r).unwrap();
        r["split"]["sha256"] = json!(split_sha256(&parsed.validation.cases));
        r
    }

    #[test]
    fn parses_and_verifies_a_good_report() {
        let parsed = parse_report(&report(0.01, true)).unwrap();
        assert_eq!(parsed.validation.cases.len(), 2);
        assert_eq!(parsed.validation.cases[0].role, CaseRole::Validation);
        assert_eq!(assess_report(&parsed).label, CredibilityLabel::VerifiedForContext);
    }

    #[test]
    fn failed_numerical_integrity_refuses_verification() {
        let parsed = parse_report(&report(0.01, false)).unwrap();
        let a = assess_report(&parsed);
        assert_eq!(a.label, CredibilityLabel::NotVerified);
        assert!(a.reasons.iter().any(|r| r.contains("numerical integrity")));
    }

    #[test]
    fn high_error_is_not_verified() {
        assert_eq!(assess_report(&parse_report(&report(0.08, true)).unwrap()).label, CredibilityLabel::NotVerified);
    }

    #[test]
    fn wrong_schema_unknown_role_and_bad_threshold_unit_are_rejected() {
        let mut r = report(0.01, true); r["schema"] = json!("other.v9");
        assert!(parse_report(&r).is_err());
        let mut r = report(0.01, true); r["cases"][0]["role"] = json!("training");
        assert!(parse_report(&r).is_err());
        let mut r = report(0.01, true); r["requirement"]["threshold"]["unit"] = json!("furlong");
        assert!(parse_report(&r).is_err());
        let mut r = report(0.01, true); r["cases"][0]["measured"] = json!([4.1, "x", 3.9]);
        assert!(parse_report(&r).is_err());
        let mut r = report(0.01, true); r["context_of_use"]["risk"] = json!("Tiny");
        assert!(parse_report(&r).is_err());
    }

    #[test]
    fn a_claimed_verdict_in_the_report_is_ignored() {
        let mut r = report(0.08, true);
        r["label"] = json!("VerifiedForContext");
        r["cases"][0]["rmse"] = json!(0.001);
        assert_eq!(assess_report(&parse_report(&r).unwrap()).label, CredibilityLabel::NotVerified);
    }

    #[test]
    fn library_entry_carries_identity_hashes_error_context_and_label() {
        let r = report(0.01, true);
        let a = assess_report(&parse_report(&r).unwrap());
        let entry = library_entry(&r, &a);
        assert_eq!(entry["schema"], LIBRARY_ENTRY_SCHEMA);
        assert_eq!(entry["label"], "VerifiedForContext");
        assert_eq!(entry["model"]["sha256"], "b".repeat(64));
        assert_eq!(entry["split_sha256"], r["split"]["sha256"]);
        assert_eq!(entry["dataset"]["doi"], "10.0/example");
        assert_eq!(entry["context_of_use"]["risk"], "Low");
        let held_out = entry["held_out_errors"].as_array().unwrap();
        assert_eq!(held_out.len(), 1);
        assert!((held_out[0]["rmse"].as_f64().unwrap() - 0.01).abs() < 1e-9);
    }
}
