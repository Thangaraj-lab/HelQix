use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::adapter::AdapterError;

/// Capability advertised by an adapter. `operations` names the accepted
/// request operations; the adapter remains responsible for their semantics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterCapability {
    pub name: String,
    pub operations: Vec<String>,
    #[serde(default)]
    pub supports_idempotency: bool,
    #[serde(default)]
    pub supports_cancellation: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrency: Option<usize>,
}

impl AdapterCapability {
    pub fn validate(&self) -> Result<(), AdapterError> {
        validate_name("capability name", &self.name)?;
        if self.operations.is_empty() {
            return Err(AdapterError::InvalidDescriptor(format!("capability {} must list at least one operation", self.name)));
        }
        if self.operations.len() > 256 {
            return Err(AdapterError::InvalidDescriptor(format!("capability {} cannot advertise more than 256 operations", self.name)));
        }
        let mut seen = HashSet::new();
        for operation in &self.operations {
            validate_name("operation", operation)?;
            if !seen.insert(operation) {
                return Err(AdapterError::InvalidDescriptor(format!("capability {} repeats operation {operation:?}", self.name)));
            }
        }
        if self.max_concurrency == Some(0) {
            return Err(AdapterError::InvalidDescriptor(format!("capability {} max_concurrency must be greater than zero", self.name)));
        }
        Ok(())
    }

    pub fn supports(&self, operation: &str) -> bool {
        self.operations.iter().any(|candidate| candidate == operation)
    }
}

/// Immutable identity and discovery data for a registered adapter.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<AdapterCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl AdapterDescriptor {
    pub fn validate(&self) -> Result<(), AdapterError> {
        validate_name("adapter id", &self.id)?;
        validate_name("adapter name", &self.name)?;
        if !is_semver(&self.version) {
            return Err(AdapterError::InvalidDescriptor("adapter version must be semantic versioning (MAJOR.MINOR.PATCH)".into()));
        }
        if self.capabilities.is_empty() {
            return Err(AdapterError::InvalidDescriptor("adapter must advertise at least one capability".into()));
        }
        let mut names = HashSet::new();
        for capability in &self.capabilities {
            capability.validate()?;
            if !names.insert(&capability.name) {
                return Err(AdapterError::InvalidDescriptor(format!("duplicate capability name {:?}", capability.name)));
            }
        }
        if let Some(provider) = &self.provider { validate_name("provider", provider)?; }
        if let Some(description) = &self.description {
            if description.trim().is_empty() {
                return Err(AdapterError::InvalidDescriptor("description must not be empty when provided".into()));
            }
        }
        Ok(())
    }

    pub fn supports(&self, capability: &str, operation: &str) -> bool {
        self.capabilities.iter().any(|item| item.name == capability && item.supports(operation))
    }
}

fn validate_name(label: &str, value: &str) -> Result<(), AdapterError> {
    if value.trim().is_empty() || value.trim() != value || value.len() > 128 {
        Err(AdapterError::InvalidDescriptor(format!("{label} must be non-empty, trimmed, and at most 128 bytes")))
    } else {
        Ok(())
    }
}

fn is_semver(value: &str) -> bool {
    if value.len() > 64 { return false; }
    let (without_build, build) = match value.split_once('+') {
        Some((core, metadata)) if !metadata.is_empty() && !metadata.contains('+') => (core, Some(metadata)),
        Some(_) => return false,
        None => (value, None),
    };
    if let Some(build) = build {
        if !valid_identifiers(build, false) { return false; }
    }
    let (core, prerelease) = match without_build.split_once('-') {
        Some((core, metadata)) if !metadata.is_empty() => (core, Some(metadata)),
        Some(_) => return false,
        None => (without_build, None),
    };
    if let Some(prerelease) = prerelease {
        if !valid_identifiers(prerelease, true) { return false; }
    }
    let parts: Vec<_> = core.split('.').collect();
    parts.len() == 3 && parts.iter().all(|part| {
        !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()) &&
            (*part == "0" || !part.starts_with('0'))
    })
}

fn valid_identifiers(value: &str, reject_numeric_leading_zero: bool) -> bool {
    value.split('.').all(|identifier| {
        !identifier.is_empty() && identifier.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-') &&
            !(reject_numeric_leading_zero && identifier.len() > 1 && identifier.starts_with('0') &&
                identifier.bytes().all(|byte| byte.is_ascii_digit()))
    })
}

#[cfg(test)]
mod tests {
    use super::is_semver;

    #[test]
    fn accepts_semver_and_rejects_ambiguous_versions() {
        assert!(is_semver("1.2.3"));
        assert!(is_semver("0.4.0-rc.1+build.7"));
        assert!(!is_semver("1.2"));
        assert!(!is_semver("01.2.3"));
        assert!(!is_semver("1.2.3-01"));
    }
}
