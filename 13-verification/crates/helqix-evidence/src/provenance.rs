//! Immutable source identity and content-digest metadata for a tool execution.
use helqix_core::id::Identifier;

/// Source tuple needed to identify and reproduce one engineering execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub project_id: Identifier, pub project_revision: u64, pub run_id: Identifier,
    pub tool_id: Identifier, pub tool_version: String, pub model_id: Identifier, pub model_version: String,
    pub model_sha256: String, pub inputs_sha256: String, pub outputs_sha256: String,
}
impl Provenance {
    /// Validates nonempty versions and three canonical SHA-256 digests.
    #[allow(clippy::too_many_arguments)]
    pub fn new(project_id: Identifier, project_revision: u64, run_id: Identifier, tool_id: Identifier, tool_version: impl Into<String>, model_id: Identifier, model_version: impl Into<String>, model_sha256: impl Into<String>, inputs_sha256: impl Into<String>, outputs_sha256: impl Into<String>) -> Result<Self, ProvenanceError> {
        if project_revision == 0 { return Err(ProvenanceError::InvalidProjectRevision); }
        let tool_version = tool_version.into(); let model_version = model_version.into();
        if tool_version.trim().is_empty() || model_version.trim().is_empty() { return Err(ProvenanceError::EmptyVersion); }
        let model_sha256 = model_sha256.into(); let inputs_sha256 = inputs_sha256.into(); let outputs_sha256 = outputs_sha256.into();
        if !valid_sha256(&model_sha256) || !valid_sha256(&inputs_sha256) || !valid_sha256(&outputs_sha256) { return Err(ProvenanceError::InvalidDigest); }
        Ok(Self { project_id, project_revision, run_id, tool_id, tool_version, model_id, model_version, model_sha256, inputs_sha256, outputs_sha256 })
    }
}
/// Provenance invariant violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceError { InvalidProjectRevision, EmptyVersion, InvalidDigest }
impl core::fmt::Display for ProvenanceError { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self { Self::InvalidProjectRevision => f.write_str("project revision must be positive"), Self::EmptyVersion => f.write_str("tool and model versions must be explicit"), Self::InvalidDigest => f.write_str("provenance digests must be 64 lowercase hexadecimal characters") } } }
impl std::error::Error for ProvenanceError {}
pub(crate) fn valid_sha256(value: &str) -> bool { value.len() == 64 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) }
