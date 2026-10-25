//! Contextual model credibility: held-out validation against measured data.
//!
//! A model earns [`CredibilityLabel::VerifiedForContext`] only when every
//! held-out (validation) case meets an accepted requirement, the split between
//! calibration and validation data is intact (re-hashed here), and no fitting
//! touched held-out data. Errors are recomputed here from the raw series; a
//! caller-supplied error value is never trusted. Domain-agnostic: a case is
//! any measured-versus-simulated time series.
use helqix_core::{id::Identifier, quantity::Quantity, units::Unit};
use sha2::{Digest, Sha256};
use crate::{requirement::Requirement, validator::{Measurement, Validator}, verification_result::{Verdict, VerificationResult}};

/// Consequence of a wrong prediction; decides how much credibility is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel { Low, Medium, High }

/// The decision a model supports and the conditions it is trusted for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextOfUse { pub statement: String, pub risk: RiskLevel }
impl ContextOfUse {
    /// Creates a context of use; the statement must say what the model is used for.
    pub fn new(statement: impl Into<String>, risk: RiskLevel) -> Result<Self, CredibilityError> {
        let statement = statement.into();
        if statement.trim().is_empty() { return Err(CredibilityError::EmptyContext); }
        if statement.len() > 4_000 { return Err(CredibilityError::ContextTooLong); }
        Ok(Self { statement, risk })
    }
}

/// Whether a case may be used for fitting (calibration) or only for judging (validation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CaseRole { Calibration, Validation }
impl CaseRole {
    fn as_str(self) -> &'static str { match self { Self::Calibration => "calibration", Self::Validation => "validation" } }
}

/// One measured experiment and the model's prediction of it, sample by sample.
#[derive(Debug, Clone, PartialEq)]
pub struct ComparisonCase { pub id: Identifier, pub role: CaseRole, pub time_s: Vec<f64>, pub input: Vec<f64>, pub measured: Vec<f64>, pub simulated: Vec<f64> }
impl ComparisonCase {
    /// SHA-256 of the measured data (time, input, measured output) as little-endian f64 bytes.
    pub fn data_sha256(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"helqix.case-data.v1\n");
        for (tag, series) in [(b"t", &self.time_s), (b"i", &self.input), (b"v", &self.measured)] {
            hasher.update(tag);
            for value in series { hasher.update(value.to_le_bytes()); }
        }
        hex(&hasher.finalize())
    }

    /// Checks equal lengths (at least two samples), finite values and strictly increasing time.
    fn series_problem(&self) -> Option<&'static str> {
        let n = self.time_s.len();
        if n < 2 || self.input.len() != n || self.measured.len() != n || self.simulated.len() != n { return Some("series must share one length of at least two samples"); }
        if [&self.time_s, &self.input, &self.measured, &self.simulated].iter().any(|s| s.iter().any(|v| !v.is_finite())) { return Some("series contain a non-finite value"); }
        if self.time_s.windows(2).any(|w| w[1] <= w[0]) { return Some("time must be strictly increasing"); }
        None
    }

    /// Root-mean-square and peak absolute difference between simulation and measurement.
    fn errors(&self) -> (f64, f64) {
        let diffs = self.simulated.iter().zip(&self.measured).map(|(s, m)| s - m);
        let (sum_sq, peak) = diffs.fold((0.0_f64, 0.0_f64), |(sq, peak), d| (sq + d * d, peak.max(d.abs())));
        ((sum_sq / self.measured.len() as f64).sqrt(), peak)
    }
}

/// SHA-256 over every case's id, role and data digest, sorted by id. Computed before any fitting.
pub fn split_sha256(cases: &[ComparisonCase]) -> String {
    let mut sorted: Vec<&ComparisonCase> = cases.iter().collect();
    sorted.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
    let mut text = String::from("helqix.split.v1\n");
    for case in sorted { text.push_str(&format!("{}\t{}\t{}\n", case.id.as_str(), case.role.as_str(), case.data_sha256())); }
    hex(&Sha256::digest(text.as_bytes()))
}

fn hex(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() }

/// Recorded split and fitting provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitRecord { pub split_sha256: String, pub fitted_on: Vec<Identifier> }

/// Everything needed to judge one model revision for one context of use.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelValidation { pub requirement: Requirement, pub context: ContextOfUse, pub model_sha256: String, pub output_unit: Unit, pub split: SplitRecord, pub cases: Vec<ComparisonCase> }

/// Contextual credibility label. There is no unqualified "Verified".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredibilityLabel { VerifiedForContext, NotVerified }

/// Recomputed error statistics and the requirement verdict for one case.
#[derive(Debug, Clone, PartialEq)]
pub struct CaseOutcome { pub case_id: Identifier, pub role: CaseRole, pub rmse: f64, pub peak_abs_error: f64, pub result: Option<VerificationResult> }

/// Outcome of a credibility assessment with the reasons for any refusal.
#[derive(Debug, Clone, PartialEq)]
pub struct CredibilityAssessment { pub label: CredibilityLabel, pub outcomes: Vec<CaseOutcome>, pub reasons: Vec<String> }

/// Judges a model against held-out data. Fail-closed: any doubt yields NotVerified.
pub fn assess(validation: &ModelValidation) -> CredibilityAssessment {
    let mut reasons = Vec::new();
    let cases = &validation.cases;
    if !validation.requirement.accepted { reasons.push("requirement has not been explicitly accepted".to_owned()); }

    let mut ids: Vec<&str> = cases.iter().map(|c| c.id.as_str()).collect();
    ids.sort_unstable();
    if ids.windows(2).any(|w| w[0] == w[1]) { reasons.push("case ids must be unique".to_owned()); }

    if split_sha256(cases) != validation.split.split_sha256 { reasons.push("split hash does not match the recorded split: data or roles changed after the split was fixed".to_owned()); }

    for fitted in &validation.split.fitted_on {
        match cases.iter().find(|c| &c.id == fitted) {
            Some(case) if case.role == CaseRole::Validation => reasons.push(format!("model was fitted on held-out case {}", fitted.as_str())),
            Some(_) => {}
            None => reasons.push(format!("fitting record names unknown case {}", fitted.as_str())),
        }
    }

    if !cases.iter().any(|c| c.role == CaseRole::Validation) { reasons.push("no held-out validation case was supplied".to_owned()); }

    let mut outcomes = Vec::new();
    for case in cases {
        if let Some(problem) = case.series_problem() {
            reasons.push(format!("case {}: {problem}", case.id.as_str()));
            continue;
        }
        let (rmse, peak_abs_error) = case.errors();
        let result = (case.role == CaseRole::Validation).then(|| judge(validation, case, rmse));
        if let Some(result) = &result {
            if result.verdict != Verdict::Pass { reasons.push(format!("held-out case {}: {} (RMSE {rmse:.6} {})", case.id.as_str(), result.message, validation.output_unit)); }
        }
        outcomes.push(CaseOutcome { case_id: case.id.clone(), role: case.role, rmse, peak_abs_error, result });
    }

    let label = if reasons.is_empty() { CredibilityLabel::VerifiedForContext } else { CredibilityLabel::NotVerified };
    CredibilityAssessment { label, outcomes, reasons }
}

/// Checks one held-out case's recomputed RMSE through the fail-closed requirement validator.
fn judge(validation: &ModelValidation, case: &ComparisonCase, rmse: f64) -> VerificationResult {
    let requirement = &validation.requirement;
    let measurement = Quantity::new(rmse, validation.output_unit).ok()
        .and_then(|value| Measurement::new(requirement.metric.clone(), value, case.id.clone(), validation.model_sha256.clone()).ok());
    match measurement {
        Some(m) => Validator.evaluate(requirement, Some(&m), &validation.model_sha256),
        None => VerificationResult::inconclusive(requirement, None, Some(case.id.clone()), None, "Error value or model digest is invalid"),
    }
}

/// Credibility input error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredibilityError { EmptyContext, ContextTooLong }
impl core::fmt::Display for CredibilityError { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self { Self::EmptyContext => f.write_str("context of use statement cannot be empty"), Self::ContextTooLong => f.write_str("context of use statement exceeds 4000 bytes") } } }
impl std::error::Error for CredibilityError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::requirement::{Comparison, RequirementPriority};

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn volts() -> Unit { Unit::parse("V").unwrap() }

    fn requirement(accepted: bool) -> Requirement {
        let threshold = Quantity::new(46.0, Unit::parse("mV").unwrap()).unwrap();
        let mut r = Requirement::draft(id("voltage_rmse_limit"), "Held-out voltage RMSE below 46 mV", RequirementPriority::Must, id("voltage_rmse"), Comparison::LessThan, threshold).unwrap();
        r.set_accepted(accepted).unwrap();
        r
    }

    /// A case whose simulation is offset from the measurement by `offset` volts everywhere (RMSE = |offset|).
    fn case(name: &str, role: CaseRole, offset: f64) -> ComparisonCase {
        let measured = vec![4.10, 4.00, 3.90, 3.80];
        ComparisonCase { id: id(name), role, time_s: vec![0.0, 10.0, 20.0, 30.0], input: vec![0.5; 4], simulated: measured.iter().map(|v| v + offset).collect(), measured }
    }

    fn validation(cases: Vec<ComparisonCase>, fitted_on: &[&str]) -> ModelValidation {
        let split = SplitRecord { split_sha256: split_sha256(&cases), fitted_on: fitted_on.iter().map(|n| id(n)).collect() };
        ModelValidation { requirement: requirement(true), context: ContextOfUse::new("Predict cell voltage at C/10", RiskLevel::Low).unwrap(), model_sha256: "a".repeat(64), output_unit: volts(), split, cases }
    }

    #[test]
    fn golden_vectors_match_the_python_implementation() {
        let a = ComparisonCase { id: id("cell_a_c10"), role: CaseRole::Validation, time_s: vec![0.0, 1.0], input: vec![0.5, 0.5], measured: vec![4.0, 3.9], simulated: vec![4.0, 3.9] };
        let b = ComparisonCase { id: id("cell_a_1c"), role: CaseRole::Calibration, time_s: vec![0.0, 2.0], input: vec![5.0, 5.0], measured: vec![4.0, 3.5], simulated: vec![4.0, 3.5] };
        assert_eq!(a.data_sha256(), "648fa04cd43ea396914ad5b01b64bdf9dd10ef3f3a52753597e80a335d47ac26");
        assert_eq!(b.data_sha256(), "b068e0e17017c6fc0282c459fd58acdeff43b556da9c889cde7b60e945f7514b");
        assert_eq!(split_sha256(&[a, b]), "eadf27faeff4c6511645295ab731e7e924ed568baf4a26999e9241ead16fa819");
    }

    #[test]
    fn verified_only_when_every_held_out_case_passes() {
        let v = validation(vec![case("c1_c10", CaseRole::Validation, 0.010), case("c2_c10", CaseRole::Validation, -0.020), case("c1_1c", CaseRole::Calibration, 0.005)], &["c1_1c"]);
        let a = assess(&v);
        assert_eq!(a.label, CredibilityLabel::VerifiedForContext, "{:?}", a.reasons);
        let held_out: Vec<_> = a.outcomes.iter().filter(|o| o.role == CaseRole::Validation).collect();
        assert_eq!(held_out.len(), 2);
        assert!((held_out[0].rmse - 0.010).abs() < 1e-12);
        assert!(held_out.iter().all(|o| o.result.as_ref().unwrap().verdict == Verdict::Pass));
    }

    #[test]
    fn one_failing_held_out_case_refuses_verification() {
        let a = assess(&validation(vec![case("c1_c10", CaseRole::Validation, 0.010), case("c2_c10", CaseRole::Validation, 0.050)], &[]));
        assert_eq!(a.label, CredibilityLabel::NotVerified);
        assert!(a.reasons.iter().any(|r| r.contains("c2_c10")));
    }

    #[test]
    fn error_exactly_at_threshold_is_not_below_it() {
        let a = assess(&validation(vec![case("c1_c10", CaseRole::Validation, 0.046)], &[]));
        assert_eq!(a.label, CredibilityLabel::NotVerified);
    }

    #[test]
    fn calibration_errors_do_not_gate_the_label() {
        let a = assess(&validation(vec![case("c1_c10", CaseRole::Validation, 0.010), case("c1_1c", CaseRole::Calibration, 0.300)], &["c1_1c"]));
        assert_eq!(a.label, CredibilityLabel::VerifiedForContext, "{:?}", a.reasons);
    }

    #[test]
    fn fitting_on_held_out_data_refuses_verification() {
        let a = assess(&validation(vec![case("c1_c10", CaseRole::Validation, 0.001), case("c1_1c", CaseRole::Calibration, 0.001)], &["c1_1c", "c1_c10"]));
        assert_eq!(a.label, CredibilityLabel::NotVerified);
        assert!(a.reasons.iter().any(|r| r.contains("held-out")));
    }

    #[test]
    fn fitting_on_an_unknown_case_refuses_verification() {
        let a = assess(&validation(vec![case("c1_c10", CaseRole::Validation, 0.001)], &["ghost"]));
        assert_eq!(a.label, CredibilityLabel::NotVerified);
    }

    #[test]
    fn tampered_measurement_breaks_the_split_hash() {
        let mut v = validation(vec![case("c1_c10", CaseRole::Validation, 0.001)], &[]);
        v.cases[0].measured[2] += 0.001;
        let a = assess(&v);
        assert_eq!(a.label, CredibilityLabel::NotVerified);
        assert!(a.reasons.iter().any(|r| r.contains("split")));
    }

    #[test]
    fn relabelled_role_breaks_the_split_hash() {
        let mut v = validation(vec![case("c1_c10", CaseRole::Validation, 0.001), case("c1_1c", CaseRole::Calibration, 0.001)], &[]);
        v.cases[1].role = CaseRole::Validation;
        assert_eq!(assess(&v).label, CredibilityLabel::NotVerified);
    }

    #[test]
    fn no_held_out_case_means_not_verified() {
        let a = assess(&validation(vec![case("c1_1c", CaseRole::Calibration, 0.001)], &["c1_1c"]));
        assert_eq!(a.label, CredibilityLabel::NotVerified);
    }

    #[test]
    fn unaccepted_requirement_means_not_verified() {
        let mut v = validation(vec![case("c1_c10", CaseRole::Validation, 0.001)], &[]);
        v.requirement = requirement(false);
        assert_eq!(assess(&v).label, CredibilityLabel::NotVerified);
    }

    #[test]
    fn malformed_series_are_rejected() {
        let mut short = validation(vec![case("c1_c10", CaseRole::Validation, 0.001)], &[]);
        short.cases[0].simulated.pop();
        assert_eq!(assess(&short).label, CredibilityLabel::NotVerified);

        let mut nan = validation(vec![case("c1_c10", CaseRole::Validation, 0.001)], &[]);
        nan.cases[0].simulated[1] = f64::NAN;
        assert_eq!(assess(&nan).label, CredibilityLabel::NotVerified);

        let mut backwards = validation(vec![case("c1_c10", CaseRole::Validation, 0.001)], &[]);
        backwards.cases[0].time_s = vec![0.0, 20.0, 10.0, 30.0];
        backwards.split.split_sha256 = split_sha256(&backwards.cases);
        assert_eq!(assess(&backwards).label, CredibilityLabel::NotVerified);
    }

    #[test]
    fn duplicate_case_ids_are_rejected() {
        let a = assess(&validation(vec![case("c1_c10", CaseRole::Validation, 0.001), case("c1_c10", CaseRole::Validation, 0.001)], &[]));
        assert_eq!(a.label, CredibilityLabel::NotVerified);
    }

    #[test]
    fn empty_context_is_rejected() {
        assert_eq!(ContextOfUse::new("  ", RiskLevel::Low), Err(CredibilityError::EmptyContext));
    }
}
