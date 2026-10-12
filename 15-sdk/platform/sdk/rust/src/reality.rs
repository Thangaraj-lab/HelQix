use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{action::ActionRef, behaviour::BehaviourRef, component::ComponentRef};

pub const REALITY_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RealityId(String);

impl RealityId {
    pub fn new(value: impl Into<String>) -> Result<Self, RealityValidationError> {
        let value = value.into();
        validate_token("id", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealityMetadata {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

impl RealityMetadata {
    pub fn validate(&self) -> Result<(), RealityValidationError> {
        let mut errors = Vec::new();
        collect_token_error("metadata.name", &self.name, &mut errors);
        for (index, tag) in self.tags.iter().enumerate() {
            collect_token_error(&format!("metadata.tags[{index}]"), tag, &mut errors);
        }
        for (field, value) in [("created_at", &self.created_at), ("updated_at", &self.updated_at)] {
            if let Some(value) = value {
                if !is_iso_datetime(value) {
                    errors.push(issue(format!("metadata.{field}"), "must be an ISO-8601 date-time with timezone"));
                }
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(RealityValidationError { issues: errors }) }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealityDocument {
    pub schema_version: u16,
    pub id: RealityId,
    pub metadata: RealityMetadata,
    pub components: Vec<ComponentRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behaviours: Vec<BehaviourRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionRef>,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

impl RealityDocument {
    pub fn new(id: RealityId, metadata: RealityMetadata) -> Self {
        Self { schema_version: REALITY_SCHEMA_VERSION, id, metadata, components: Vec::new(), behaviours: Vec::new(), actions: Vec::new(), extensions: serde_json::Map::new() }
    }

    pub fn validate(&self) -> Result<(), RealityValidationError> {
        let mut errors = Vec::new();
        if self.schema_version != REALITY_SCHEMA_VERSION {
            errors.push(issue("schema_version", format!("must equal {REALITY_SCHEMA_VERSION}")));
        }
        if let Err(error) = self.metadata.validate() { errors.extend(error.issues); }
        validate_unique_refs("components", self.components.iter().map(|item| item.id.as_str()), &mut errors);
        validate_unique_refs("behaviours", self.behaviours.iter().map(|item| item.id.as_str()), &mut errors);
        validate_unique_refs("actions", self.actions.iter().map(|item| item.id.as_str()), &mut errors);
        for (index, component) in self.components.iter().enumerate() {
            if let Err(error) = component.validate() { errors.extend(prefix_issues(format!("components[{index}]"), error)); }
        }
        for (index, behaviour) in self.behaviours.iter().enumerate() {
            if let Err(error) = behaviour.validate() { errors.extend(prefix_issues(format!("behaviours[{index}]"), error)); }
        }
        for (index, action) in self.actions.iter().enumerate() {
            if let Err(error) = action.validate() { errors.extend(prefix_issues(format!("actions[{index}]"), error)); }
        }
        if errors.is_empty() { Ok(()) } else { Err(RealityValidationError { issues: errors }) }
    }

    pub fn to_json(&self) -> Result<String, RealityCodecError> {
        self.validate().map_err(RealityCodecError::Validation)?;
        serde_json::to_string(self).map_err(RealityCodecError::Json)
    }

    pub fn from_json(input: &str) -> Result<Self, RealityCodecError> {
        let reality: Self = serde_json::from_str(input).map_err(RealityCodecError::Json)?;
        reality.validate().map_err(RealityCodecError::Validation)?;
        Ok(reality)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RealityValidationIssue {
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RealityValidationError {
    pub issues: Vec<RealityValidationIssue>,
}

impl std::fmt::Display for RealityValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid reality document: ")?;
        for (index, item) in self.issues.iter().enumerate() {
            if index > 0 { write!(formatter, "; ")?; }
            write!(formatter, "{}: {}", item.path, item.message)?;
        }
        Ok(())
    }
}
impl std::error::Error for RealityValidationError {}

#[derive(Debug)]
pub enum RealityCodecError { Validation(RealityValidationError), Json(serde_json::Error) }
impl std::fmt::Display for RealityCodecError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::Validation(error) => error.fmt(formatter), Self::Json(error) => write!(formatter, "reality JSON error: {error}") }
    }
}
impl std::error::Error for RealityCodecError {}

fn validate_unique_refs<'a>(path: &str, ids: impl Iterator<Item = &'a str>, errors: &mut Vec<RealityValidationIssue>) {
    let mut seen = HashSet::new();
    for (index, id) in ids.enumerate() {
        if !seen.insert(id) { errors.push(issue(format!("{path}[{index}].id"), format!("duplicate id {id:?}"))); }
    }
}

fn validate_token(path: &str, value: &str) -> Result<(), RealityValidationError> {
    if value.trim().is_empty() || value.trim() != value {
        Err(RealityValidationError { issues: vec![issue(path, "must be a non-empty trimmed string")] })
    } else { Ok(()) }
}

fn collect_token_error(path: &str, value: &str, errors: &mut Vec<RealityValidationIssue>) {
    if let Err(error) = validate_token(path, value) { errors.extend(error.issues); }
}

pub(crate) fn is_iso_datetime(value: &str) -> bool {
    let (timestamp, timezone) = if let Some(timestamp) = value.strip_suffix('Z') {
        (timestamp, true)
    } else if let Some(index) = value.rfind(|character| character == '+' || character == '-') {
        let offset = &value[index..];
        let bytes = offset.as_bytes();
        if index <= 10 || bytes.len() != 6 || bytes[3] != b':' || !bytes[..2].iter().chain(bytes[4..].iter()).all(|byte| byte.is_ascii_digit()) { return false; }
        (&value[..index], true)
    } else { (value, false) };
    if !timezone { return false; }
    let base = timestamp.split_once('.').map_or(timestamp, |(base, fraction)| {
        if fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()) { "" } else { base }
    });
    let bytes = base.as_bytes();
    bytes.len() == 19 && bytes[4] == b'-' && bytes[7] == b'-' && bytes[10] == b'T' && bytes[13] == b':' && bytes[16] == b':' && [0..4, 5..7, 8..10, 11..13, 14..16, 17..19].iter().all(|range| bytes[range.clone()].iter().all(|byte| byte.is_ascii_digit())) && value.parse::<chrono::DateTime<chrono::FixedOffset>>().is_ok()
}

fn issue(path: impl Into<String>, message: impl Into<String>) -> RealityValidationIssue { RealityValidationIssue { path: path.into(), message: message.into() } }
fn prefix_issues(prefix: String, error: RealityValidationError) -> Vec<RealityValidationIssue> {
    error.issues.into_iter().map(|mut item| { item.path = format!("{prefix}.{}", item.path); item }).collect()
}

#[cfg(test)]
mod tests {
    use super::{RealityDocument, RealityId, RealityMetadata, REALITY_SCHEMA_VERSION};
    #[test]
    fn metadata_validation_does_not_require_a_reality_id() {
        let metadata = RealityMetadata { name: "Battery study".into(), description: None, tags: Vec::new(), created_at: None, updated_at: None, extensions: Default::default() };
        assert!(metadata.validate().is_ok());
    }
    #[test]
    fn reality_wire_decoder_requires_components_and_rejects_unknown_fields() {
        let id = RealityId::new("reality-1").unwrap();
        let metadata = RealityMetadata { name: "Battery study".into(), description: None, tags: Vec::new(), created_at: None, updated_at: None, extensions: Default::default() };
        let document = RealityDocument::new(id, metadata);
        let wire = document.to_json().unwrap();
        assert!(RealityDocument::from_json(&wire).is_ok());
        let missing_components = wire.replace("\"components\":[]", "\"removedComponents\":[]");
        assert!(RealityDocument::from_json(&missing_components).is_err());
        let unknown = wire.replace("\"components\":[]", "\"components\":[],\"unexpected\":true");
        assert!(RealityDocument::from_json(&unknown).is_err());
        assert_eq!(document.schema_version, REALITY_SCHEMA_VERSION);
    }
}
