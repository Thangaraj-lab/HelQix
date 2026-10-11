use serde::{Deserialize, Serialize};

use crate::reality::{RealityValidationError, RealityValidationIssue};

/// Behaviour registered with a reality. Configuration is JSON data; execution
/// is intentionally handled by the behaviour/runtime layers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BehaviourRef {
    pub id: String,
    #[serde(rename = "type")]
    pub behaviour_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

impl BehaviourRef {
    pub fn validate(&self) -> Result<(), RealityValidationError> {
        let mut issues = Vec::new();
        validate_token("id", &self.id, &mut issues);
        validate_token("type", &self.behaviour_type, &mut issues);
        if issues.is_empty() { Ok(()) } else { Err(RealityValidationError { issues }) }
    }
}

/// A concrete behaviour configuration can be versioned independently from
/// the root reality schema.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BehaviourDefinition {
    pub id: String,
    #[serde(rename = "type")]
    pub behaviour_type: String,
    pub version: u32,
    pub config: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl BehaviourDefinition {
    pub fn validate(&self) -> Result<(), RealityValidationError> {
        let mut issues = Vec::new();
        validate_token("id", &self.id, &mut issues);
        validate_token("type", &self.behaviour_type, &mut issues);
        if self.version == 0 { issues.push(issue("version", "must be greater than zero")); }
        if let Some(description) = &self.description {
            if description.trim().is_empty() { issues.push(issue("description", "must not be empty when provided")); }
        }
        if issues.is_empty() { Ok(()) } else { Err(RealityValidationError { issues }) }
    }
}

fn validate_token(path: &str, value: &str, issues: &mut Vec<RealityValidationIssue>) {
    if value.trim().is_empty() || value.trim() != value {
        issues.push(issue(path, "must be a non-empty trimmed string"));
    }
}

fn issue(path: impl Into<String>, message: impl Into<String>) -> RealityValidationIssue {
    RealityValidationIssue { path: path.into(), message: message.into() }
}
