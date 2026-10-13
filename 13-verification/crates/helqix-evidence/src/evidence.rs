//! Deterministic evidence bundle and SHA-256 integrity digest.
use std::collections::BTreeMap;
use helqix_core::id::Identifier;
use sha2::{Digest, Sha256};
use helqix_verify::VerificationResult;
use crate::execution_record::ExecutionRecord;

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceBundle { pub project_id: Identifier, pub project_revision: u64, executions: BTreeMap<Identifier, ExecutionRecord>, verifications: Vec<VerificationResult> }
impl EvidenceBundle {
    pub fn new(project_id: Identifier, project_revision: u64) -> Result<Self, EvidenceError> { if project_revision == 0 { return Err(EvidenceError::InvalidRevision); } Ok(Self { project_id, project_revision, executions: BTreeMap::new(), verifications: Vec::new() }) }
    pub fn add_execution(&mut self, record: ExecutionRecord) -> Result<(), EvidenceError> { if record.provenance.project_id != self.project_id { return Err(EvidenceError::WrongProject); } if self.executions.contains_key(&record.id) { return Err(EvidenceError::DuplicateExecution(record.id)); } self.executions.insert(record.id.clone(), record); Ok(()) }
    pub fn add_verification(&mut self, result: VerificationResult) { self.verifications.push(result); }
    pub fn executions(&self) -> impl Iterator<Item = &ExecutionRecord> { self.executions.values() }
    pub fn verifications(&self) -> &[VerificationResult] { &self.verifications }
    pub fn execution_count(&self) -> usize { self.executions.len() }
    pub fn digest_sha256(&self) -> String {
        let mut hash = Sha256::new(); put_text(&mut hash, "helqix.evidence-bundle.v1"); put_text(&mut hash, self.project_id.as_str()); put_u64(&mut hash, self.project_revision); put_u64(&mut hash, self.executions.len() as u64);
        for record in self.executions.values() { put_text(&mut hash, record.id.as_str()); let p = &record.provenance; put_text(&mut hash, p.run_id.as_str()); put_text(&mut hash, p.tool_id.as_str()); put_text(&mut hash, &p.tool_version); put_text(&mut hash, p.model_id.as_str()); put_text(&mut hash, &p.model_version); put_text(&mut hash, &p.model_sha256); put_text(&mut hash, &p.inputs_sha256); put_text(&mut hash, &p.outputs_sha256); put_u64(&mut hash, p.project_revision); put_u8(&mut hash, record.status as u8); put_i128(&mut hash, record.started_at.as_nanos()); put_bool(&mut hash, record.finished_at.is_some()); if let Some(end) = record.finished_at { put_i128(&mut hash, end.as_nanos()); } put_text(&mut hash, record.failure_reason.as_deref().unwrap_or("")); }
        let mut checks: Vec<_> = self.verifications.iter().collect(); checks.sort_by(|a, b| a.requirement_id.cmp(&b.requirement_id).then_with(|| a.source_run_id.cmp(&b.source_run_id)).then_with(|| a.verdict.cmp(&b.verdict))); put_u64(&mut hash, checks.len() as u64);
        for check in checks { put_text(&mut hash, check.requirement_id.as_str()); put_u8(&mut hash, check.verdict as u8); put_quantity(&mut hash, check.expected); put_bool(&mut hash, check.observed.is_some()); if let Some(value) = check.observed { put_quantity(&mut hash, value); } put_text(&mut hash, check.source_run_id.as_ref().map_or("", Identifier::as_str)); put_text(&mut hash, check.source_model_sha256.as_deref().unwrap_or("")); put_text(&mut hash, &check.message); }
        to_hex(hash.finalize().as_slice())
    }
}
fn put_u8(hash: &mut Sha256, value: u8) { hash.update([value]); }
fn put_bool(hash: &mut Sha256, value: bool) { put_u8(hash, u8::from(value)); }
fn put_u64(hash: &mut Sha256, value: u64) { hash.update(value.to_le_bytes()); }
fn put_i128(hash: &mut Sha256, value: i128) { hash.update(value.to_le_bytes()); }
fn put_text(hash: &mut Sha256, value: &str) { put_u64(hash, value.len() as u64); hash.update(value.as_bytes()); }
fn put_quantity(hash: &mut Sha256, value: helqix_core::Quantity) { hash.update(value.value().to_bits().to_le_bytes()); put_text(hash, value.unit().symbol()); }
fn to_hex(bytes: impl AsRef<[u8]>) -> String { const HEX: &[u8; 16] = b"0123456789abcdef"; bytes.as_ref().iter().flat_map(|b| [HEX[(b >> 4) as usize] as char, HEX[(b & 0x0f) as usize] as char]).collect() }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceError { InvalidRevision, WrongProject, DuplicateExecution(Identifier) }
impl core::fmt::Display for EvidenceError { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self { Self::InvalidRevision => f.write_str("evidence project revision must be positive"), Self::WrongProject => f.write_str("execution provenance belongs to a different project"), Self::DuplicateExecution(id) => write!(f, "execution already exists in evidence bundle: {id}") } } }
impl std::error::Error for EvidenceError {}

#[cfg(test)]
mod tests {
    use super::*; use helqix_core::id::Identifier; use helqix_time::SimTime; use crate::{execution_record::{ExecutionRecord, ExecutionStatus}, provenance::Provenance};
    fn id(s: &str) -> Identifier { Identifier::parse(s).unwrap() }
    fn record(id_value: &str, run: &str) -> ExecutionRecord { let p = Provenance::new(id("project"), 2, id(run), id("pybamm"), "1.0", id("spm"), "Chen2020", "a".repeat(64), "b".repeat(64), "c".repeat(64)).unwrap(); ExecutionRecord::new(id(id_value), p, ExecutionStatus::Completed, SimTime::ZERO, Some(SimTime::from_secs(1).unwrap()), None).unwrap() }
    #[test]
    fn bundle_digest_is_order_independent_and_project_scoped() { let mut one = EvidenceBundle::new(id("project"), 3).unwrap(); one.add_execution(record("execution_a", "run_a")).unwrap(); one.add_execution(record("execution_b", "run_b")).unwrap(); let mut two = EvidenceBundle::new(id("project"), 3).unwrap(); two.add_execution(record("execution_b", "run_b")).unwrap(); two.add_execution(record("execution_a", "run_a")).unwrap(); assert_eq!(one.digest_sha256(), two.digest_sha256()); assert_eq!(one.digest_sha256().len(), 64); }
}
