use serde::{Deserialize, Serialize};

use crate::reality::{RealityValidationError, RealityValidationIssue};

/// Component reference embedded in a reality document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentRef {
    pub id: String,
    #[serde(rename = "type")]
    pub component_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl ComponentRef {
    pub fn validate(&self) -> Result<(), RealityValidationError> {
        let mut issues = Vec::new();
        validate_token("id", &self.id, &mut issues);
        validate_token("type", &self.component_type, &mut issues);
        if issues.is_empty() { Ok(()) } else { Err(RealityValidationError { issues }) }
    }
}

/// Full component state attached to an entity or reality.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Component {
    pub id: String,
    #[serde(rename = "type")]
    pub component_type: String,
    pub version: u32,
    pub data: serde_json::Value,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl Component {
    pub fn validate(&self) -> Result<(), RealityValidationError> {
        let mut issues = Vec::new();
        validate_token("id", &self.id, &mut issues);
        validate_token("type", &self.component_type, &mut issues);
        if self.version == 0 { issues.push(issue("version", "must be greater than zero")); }
        if issues.is_empty() { Ok(()) } else { Err(RealityValidationError { issues }) }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentKey {
    pub owner_id: String,
    pub component_id: String,
}

impl ComponentKey {
    pub fn new(owner_id: impl Into<String>, component_id: impl Into<String>) -> Result<Self, RealityValidationError> {
        let key = Self { owner_id: owner_id.into(), component_id: component_id.into() };
        let mut issues = Vec::new();
        validate_token("owner_id", &key.owner_id, &mut issues);
        validate_token("component_id", &key.component_id, &mut issues);
        if issues.is_empty() { Ok(key) } else { Err(RealityValidationError { issues }) }
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
