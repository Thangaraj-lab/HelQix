//! Structured proposals. Built from untrusted agent JSON: only the four content fields are read;
//! origin and status are always set here, whatever the agent wrote.
use serde::Serialize;
use serde_json::Value;

pub const ORIGIN_AI: &str = "AI-hypothesis";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ProposalStatus {
    /// As proposed by the agent; nothing checked.
    Unverified,
    /// HELQIX ran the required validations and they passed (simulation evidence, not reality).
    SupportedBySimulation,
    /// HELQIX ran the required validations and they failed.
    RejectedBySimulation,
    /// A human applied it after a passing validation.
    Applied,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Proposal {
    pub id: String,
    /// The change, e.g. `{"parameter": "drag_coefficient", "from": 0.47, "to": 1.2}`.
    pub intervention: Value,
    pub rationale: String,
    pub expected_effect: String,
    pub required_validations: Vec<String>,
    pub origin: &'static str,
    status: ProposalStatus,
    validation: Option<Value>,
}

impl Proposal {
    pub fn from_agent(id: String, v: &Value) -> Result<Self, String> {
        let text = |k: &str| v[k].as_str().filter(|s| !s.trim().is_empty()).map(str::to_owned).ok_or(format!("proposal field {k} must be non-empty text"));
        let intervention = v["intervention"].clone();
        if !intervention.is_object() { return Err("proposal field intervention must be an object".into()); }
        let required_validations: Vec<String> = v["required_validations"].as_array().ok_or("proposal field required_validations must be a list")?
            .iter().map(|x| x.as_str().map(str::to_owned).ok_or("required_validations must be text")).collect::<Result<_, _>>()?;
        if required_validations.is_empty() { return Err("a proposal must name at least one validation".into()); }
        Ok(Self { id, intervention, rationale: text("rationale")?, expected_effect: text("expected_effect")?, required_validations, origin: ORIGIN_AI, status: ProposalStatus::Unverified, validation: None })
    }
    pub fn status(&self) -> ProposalStatus { self.status }
    pub fn validation(&self) -> Option<&Value> { self.validation.as_ref() }
    pub(crate) fn set_validation(&mut self, passed: bool, evidence: Value) {
        self.status = if passed { ProposalStatus::SupportedBySimulation } else { ProposalStatus::RejectedBySimulation };
        self.validation = Some(evidence);
    }
    pub(crate) fn set_applied(&mut self) { self.status = ProposalStatus::Applied; }
}
