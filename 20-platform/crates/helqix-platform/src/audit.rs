//! Append-only audit records with monotonic sequence numbers.
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuditSubject { pub kind: String, pub id: String }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOutcome { Accepted, Rejected, Completed, Failed }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    pub sequence: u64,
    pub occurred_at_unix_ms: u64,
    pub actor_id: String,
    pub workspace_id: String,
    pub action: String,
    pub subject: AuditSubject,
    pub outcome: AuditOutcome,
    /// Safe, non-secret, bounded metadata. Credentials and payloads belong in neither this map nor logs.
    pub metadata: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditError { EmptyField, ClockReversed, SequenceOverflow, MetadataTooLarge, SensitiveMetadata }
#[derive(Debug, Clone, Default)]
pub struct AuditLog { events: Vec<AuditEvent>, last_time_ms: Option<u64>, max_metadata_bytes: usize }
impl AuditLog {
    pub fn new(max_metadata_bytes: usize) -> Self { Self { events: Vec::new(), last_time_ms: None, max_metadata_bytes } }
    pub fn append(&mut self, occurred_at_unix_ms: u64, actor_id: impl Into<String>, workspace_id: impl Into<String>, action: impl Into<String>, subject: AuditSubject, outcome: AuditOutcome, metadata: BTreeMap<String,String>) -> Result<&AuditEvent, AuditError> {
        let actor_id=actor_id.into(); let workspace_id=workspace_id.into(); let action=action.into();
        if actor_id.trim().is_empty() || workspace_id.trim().is_empty() || action.trim().is_empty() || subject.kind.trim().is_empty() || subject.id.trim().is_empty() { return Err(AuditError::EmptyField); }
        if self.last_time_ms.is_some_and(|last| occurred_at_unix_ms < last) { return Err(AuditError::ClockReversed); }
        let metadata_bytes = metadata.iter().map(|(k,v)| k.len().saturating_add(v.len())).sum::<usize>();
        if metadata_bytes > self.max_metadata_bytes { return Err(AuditError::MetadataTooLarge); }
        const SENSITIVE_KEY_FRAGMENTS: [&str; 7] = ["password", "secret", "token", "credential", "authorization", "api_key", "private_key"];
        if metadata.keys().any(|key| { let normalized=key.to_ascii_lowercase(); SENSITIVE_KEY_FRAGMENTS.iter().any(|fragment| normalized.contains(fragment)) }) { return Err(AuditError::SensitiveMetadata); }
        let sequence = (self.events.len() as u64).checked_add(1).ok_or(AuditError::SequenceOverflow)?;
        self.events.push(AuditEvent { sequence, occurred_at_unix_ms, actor_id, workspace_id, action, subject, outcome, metadata });
        self.last_time_ms=Some(occurred_at_unix_ms);
        Ok(self.events.last().expect("event appended"))
    }
    pub fn events(&self) -> &[AuditEvent] { &self.events }
    pub fn len(&self) -> usize { self.events.len() }
    pub fn is_empty(&self) -> bool { self.events.is_empty() }
}
impl std::fmt::Display for AuditError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(match self { Self::EmptyField=>"audit event has an empty required field", Self::ClockReversed=>"audit event time moved backwards", Self::SequenceOverflow=>"audit sequence overflow", Self::MetadataTooLarge=>"audit event metadata exceeds its configured size limit", Self::SensitiveMetadata=>"audit metadata key is reserved for sensitive credentials and secrets" }) } }
impl std::error::Error for AuditError {}

#[cfg(test)]
mod tests { use super::*; fn subject()->AuditSubject{AuditSubject{kind:"simulation_run".into(),id:"run-1".into()}} #[test] fn sequence_is_monotonic_and_append_only(){let mut log=AuditLog::new(64);let first=log.append(10,"u1","w1","run",subject(),AuditOutcome::Accepted,BTreeMap::new()).unwrap().sequence;let second=log.append(10,"u1","w1","finish",subject(),AuditOutcome::Completed,BTreeMap::new()).unwrap().sequence;assert_eq!((first,second),(1,2));assert_eq!(log.len(),2);} #[test] fn reversed_clock_is_rejected_without_mutation(){let mut log=AuditLog::new(64);log.append(10,"u1","w1","run",subject(),AuditOutcome::Accepted,BTreeMap::new()).unwrap();assert_eq!(log.append(9,"u1","w1","finish",subject(),AuditOutcome::Completed,BTreeMap::new()).unwrap_err(),AuditError::ClockReversed);assert_eq!(log.len(),1);} #[test] fn credential_metadata_is_rejected(){let mut log=AuditLog::new(128);let mut metadata=BTreeMap::new();metadata.insert("api_key_fingerprint".into(),"value".into());assert_eq!(log.append(1,"u","w","run",subject(),AuditOutcome::Accepted,metadata).unwrap_err(),AuditError::SensitiveMetadata);assert!(log.is_empty());} }
