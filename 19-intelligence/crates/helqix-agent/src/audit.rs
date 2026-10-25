//! Append-only, hash-chained audit log. Each entry's hash covers every field and the previous
//! hash, so editing, removing or reordering any entry breaks [`verify`].
use serde::{Deserialize, Serialize};

use crate::sha256_hex;

pub const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEntry {
    pub seq: u64,
    pub at: String,
    /// `agent:<provider>`, `human:<name>` or `helqix`.
    pub actor: String,
    /// `provider_reply`, `tool_call`, `grant`, `revoke`, `approve`, `proposal`, `validation`.
    pub action: String,
    pub tool: Option<String>,
    pub required: Option<String>,
    pub input_sha256: Option<String>,
    pub output_sha256: Option<String>,
    /// `allowed` or `denied: <reason>`.
    pub decision: String,
    pub approver: Option<String>,
    pub detail: String,
    pub prev: String,
    pub hash: String,
}

/// The fields a caller supplies; sequence, links and hash are filled in by the log.
#[derive(Debug, Clone, Default)]
pub struct Record {
    pub actor: String,
    pub action: String,
    pub tool: Option<String>,
    pub required: Option<String>,
    pub input_sha256: Option<String>,
    pub output_sha256: Option<String>,
    pub decision: String,
    pub approver: Option<String>,
    pub detail: String,
}

fn entry_hash(e: &AuditEntry) -> String {
    let body = serde_json::to_string(&(e.seq, &e.at, &e.actor, &e.action, &e.tool, &e.required, &e.input_sha256, &e.output_sha256, &e.decision, &e.approver, &e.detail, &e.prev))
        .expect("audit fields serialize");
    sha256_hex(format!("helqix.audit.v1\n{body}").as_bytes())
}

#[derive(Debug, Clone, Default)]
pub struct AuditLog { entries: Vec<AuditEntry> }

impl AuditLog {
    pub fn append(&mut self, at: String, r: Record) -> &AuditEntry {
        let prev = self.entries.last().map_or_else(|| GENESIS.to_owned(), |e| e.hash.clone());
        let mut e = AuditEntry { seq: self.entries.len() as u64, at, actor: r.actor, action: r.action, tool: r.tool, required: r.required, input_sha256: r.input_sha256,
                                 output_sha256: r.output_sha256, decision: r.decision, approver: r.approver, detail: r.detail, prev, hash: String::new() };
        e.hash = entry_hash(&e);
        self.entries.push(e);
        self.entries.last().expect("just pushed")
    }
    pub fn entries(&self) -> &[AuditEntry] { &self.entries }
    pub fn head(&self) -> &str { self.entries.last().map_or(GENESIS, |e| e.hash.as_str()) }
    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(&serde_json::json!({"schema": "helqix.agent-audit.v1", "head": self.head(), "entries": self.entries})).expect("audit serializes")
    }
}

/// Checks sequence numbers, links and hashes. Returns the first problem found.
pub fn verify(entries: &[AuditEntry]) -> Result<(), String> {
    let mut prev = GENESIS.to_owned();
    for (i, e) in entries.iter().enumerate() {
        if e.seq != i as u64 { return Err(format!("entry {i}: sequence number {} out of order", e.seq)); }
        if e.prev != prev { return Err(format!("entry {i}: link to previous entry broken")); }
        if entry_hash(e) != e.hash { return Err(format!("entry {i}: hash does not match its contents")); }
        prev = e.hash.clone();
    }
    Ok(())
}

/// Parses an exported log and verifies it; `expected_head` pins the last hash (catches truncation).
pub fn verify_json(bytes: &[u8], expected_head: &str) -> Result<(), String> {
    #[derive(Deserialize)]
    struct Doc { schema: String, head: String, entries: Vec<AuditEntry> }
    let d: Doc = serde_json::from_slice(bytes).map_err(|e| format!("not an audit log: {e}"))?;
    if d.schema != "helqix.agent-audit.v1" { return Err("unknown audit schema".into()); }
    verify(&d.entries)?;
    let last = d.entries.last().map_or(GENESIS, |e| e.hash.as_str());
    if last != d.head || d.head != expected_head { return Err("audit head does not match the expected head (entries removed or replaced)".into()); }
    Ok(())
}
