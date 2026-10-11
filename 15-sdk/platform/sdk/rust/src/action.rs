use serde::{Deserialize, Serialize};

use crate::reality::{is_iso_datetime, RealityId, RealityValidationError, RealityValidationIssue};

/// Reference to an action available on a reality document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionRef {
    pub id: String,
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
}

impl ActionRef {
    pub fn validate(&self) -> Result<(), RealityValidationError> {
        let mut issues = Vec::new();
        validate_token("id", &self.id, &mut issues);
        validate_token("type", &self.action_type, &mut issues);
        if issues.is_empty() { Ok(()) } else { Err(RealityValidationError { issues }) }
    }
}

/// Request to invoke an action. `idempotency_key` remains stable across retries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionInvocation {
    pub invocation_id: String,
    pub reality_id: RealityId,
    pub action_id: String,
    pub input: serde_json::Value,
    pub idempotency_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl ActionInvocation {
    pub fn validate(&self) -> Result<(), RealityValidationError> {
        let mut issues = Vec::new();
        validate_token("invocation_id", &self.invocation_id, &mut issues);
        validate_token("reality_id", self.reality_id.as_str(), &mut issues);
        validate_token("action_id", &self.action_id, &mut issues);
        validate_token("idempotency_key", &self.idempotency_key, &mut issues);
        if let Some(trace_id) = &self.trace_id { validate_token("trace_id", trace_id, &mut issues); }
        if let Some(timestamp) = &self.requested_at {
            if !is_timestamp(timestamp) { issues.push(issue("requested_at", "must be an ISO-8601 timestamp with timezone")); }
        }
        if issues.is_empty() { Ok(()) } else { Err(RealityValidationError { issues }) }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionFailure {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum ActionOutcome {
    Succeeded { output: serde_json::Value, completed_at: String },
    Accepted { operation_id: String, accepted_at: String },
    Failed { error: ActionFailure, completed_at: String },
}

impl ActionOutcome {
    pub fn validate(&self) -> Result<(), RealityValidationError> {
        let mut issues = Vec::new();
        match self {
            Self::Succeeded { completed_at, .. } | Self::Failed { completed_at, .. } => {
                if !is_timestamp(completed_at) { issues.push(issue("completed_at", "must be an ISO-8601 timestamp with timezone")); }
            }
            Self::Accepted { operation_id, accepted_at } => {
                validate_token("operation_id", operation_id, &mut issues);
                if !is_timestamp(accepted_at) { issues.push(issue("accepted_at", "must be an ISO-8601 timestamp with timezone")); }
            }
        }
        if let Self::Failed { error, .. } = self {
            validate_token("error.code", &error.code, &mut issues);
            if error.message.trim().is_empty() { issues.push(issue("error.message", "must not be empty")); }
        }
        if issues.is_empty() { Ok(()) } else { Err(RealityValidationError { issues }) }
    }
}

fn validate_token(path: &str, value: &str, issues: &mut Vec<RealityValidationIssue>) {
    if value.trim().is_empty() || value.trim() != value {
        issues.push(issue(path, "must be a non-empty trimmed string"));
    }
}

fn is_timestamp(value: &str) -> bool {
    is_iso_datetime(value)
}

fn issue(path: impl Into<String>, message: impl Into<String>) -> RealityValidationIssue {
    RealityValidationIssue { path: path.into(), message: message.into() }
}
