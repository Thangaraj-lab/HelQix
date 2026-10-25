//! A session: granted levels, registered tools, human approvals, limits, proposals, audit log.
//!
//! Only host code holding `&mut Session` can grant, revoke, approve or record a validation, and
//! each of those is audited with the human or system that did it. The agent's provider sees only
//! the transcript. Two tools are built in:
//! - `propose` (Propose): stores a [`Proposal`] built from the request input.
//! - `apply_proposal` (Mutate): input `{"proposal_id": ...}`; refused unless HELQIX has recorded a
//!   passing validation for that proposal; then runs the host's registered `apply_proposal` tool
//!   with the proposal's intervention.
use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::{
    audit::{AuditLog, Record},
    permission::{Grants, Permission},
    proposal::{Proposal, ProposalStatus},
    provider::{AgentProvider, Message, Role, ToolRequest, Usage},
    sha256_hex,
};

pub type ToolFn = Box<dyn FnMut(&Value) -> Result<Value, String>>;

#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpec { pub name: String, pub required: Permission, pub description: String }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits { pub max_provider_calls: u32, pub max_tool_calls: u32, pub max_tokens: u64, pub max_cost_microdollars: u64 }

#[derive(Debug, Clone, PartialEq)]
pub enum AgentError {
    UnknownTool(String),
    Denied { tool: String, required: Permission },
    ApprovalRequired { tool: String, action_sha256: String },
    LimitExceeded(String),
    InvalidProposal(String),
    NotValidated(String),
    Tool(String),
    Provider(String),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTool(t) => write!(f, "unknown tool {t}"),
            Self::Denied { tool, required } => write!(f, "{tool} needs {} which this session does not have", required.name()),
            Self::ApprovalRequired { tool, action_sha256 } => write!(f, "{tool} needs a human approval for action {action_sha256}"),
            Self::LimitExceeded(m) => write!(f, "session limit exceeded: {m}"),
            Self::InvalidProposal(m) => write!(f, "invalid proposal: {m}"),
            Self::NotValidated(m) => write!(f, "not validated: {m}"),
            Self::Tool(m) => write!(f, "tool failed: {m}"),
            Self::Provider(m) => write!(f, "provider failed: {m}"),
        }
    }
}

/// Identity of one action: the tool and its exact input. An approval covers this hash only.
pub fn action_sha256(tool: &str, input: &Value) -> String {
    sha256_hex(format!("helqix.action.v1\n{tool}\n{}", serde_json::to_string(input).expect("json value serializes")).as_bytes())
}

pub struct Session {
    pub id: String,
    grants: Grants,
    limits: Limits,
    tools: BTreeMap<String, (ToolSpec, Option<ToolFn>)>,
    approvals: BTreeMap<String, String>,
    proposals: Vec<Proposal>,
    audit: AuditLog,
    clock: Box<dyn FnMut() -> String>,
    provider_calls: u32,
    tool_calls: u32,
    usage: Usage,
}

impl Session {
    pub fn new(id: &str, grants: Grants, limits: Limits, clock: Box<dyn FnMut() -> String>) -> Self {
        let mut s = Self { id: id.into(), grants, limits, tools: BTreeMap::new(), approvals: BTreeMap::new(), proposals: Vec::new(), audit: AuditLog::default(), clock,
                           provider_calls: 0, tool_calls: 0, usage: Usage::default() };
        s.tools.insert("propose".into(), (ToolSpec { name: "propose".into(), required: Permission::Propose, description: "submit a structured proposal".into() }, None));
        s.tools.insert("apply_proposal".into(), (ToolSpec { name: "apply_proposal".into(), required: Permission::Mutate, description: "apply a validated proposal".into() }, None));
        let levels = s.grants.levels().iter().map(|p| p.name()).collect::<Vec<_>>().join(", ");
        s.log(Record { actor: "helqix".into(), action: "session_start".into(), decision: "allowed".into(), detail: format!("granted: {levels}"), ..Record::default() });
        s
    }

    fn log(&mut self, r: Record) { let at = (self.clock)(); self.audit.append(at, r); }

    /// Registers a host tool. For `apply_proposal`, `f` performs the change (it receives the intervention).
    pub fn register_tool(&mut self, spec: ToolSpec, f: ToolFn) {
        let entry = self.tools.entry(spec.name.clone()).or_insert((spec.clone(), None));
        if entry.0.name != "propose" && entry.0.name != "apply_proposal" { entry.0 = spec; }
        entry.1 = Some(f);
    }

    pub fn grants(&self) -> &Grants { &self.grants }
    pub fn audit(&self) -> &AuditLog { &self.audit }
    pub fn proposals(&self) -> &[Proposal] { &self.proposals }
    pub fn usage(&self) -> Usage { self.usage }

    pub fn grant(&mut self, p: Permission, by_human: &str) {
        self.grants.insert(p);
        self.log(Record { actor: format!("human:{by_human}"), action: "grant".into(), required: Some(p.name().into()), decision: "allowed".into(), ..Record::default() });
    }
    pub fn revoke(&mut self, p: Permission, by_human: &str) {
        self.grants.remove(p);
        self.log(Record { actor: format!("human:{by_human}"), action: "revoke".into(), required: Some(p.name().into()), decision: "allowed".into(), ..Record::default() });
    }

    /// A human approves exactly this tool call. Single use. Returns the action hash.
    pub fn approve(&mut self, tool: &str, input: &Value, approver: &str, note: &str) -> String {
        let h = action_sha256(tool, input);
        self.approvals.insert(h.clone(), approver.into());
        self.log(Record { actor: format!("human:{approver}"), action: "approve".into(), tool: Some(tool.into()), input_sha256: Some(h.clone()), decision: "allowed".into(), approver: Some(approver.into()), detail: note.into(), ..Record::default() });
        h
    }

    /// HELQIX (not the agent) records the outcome of a proposal's required validations.
    pub fn record_validation(&mut self, proposal_id: &str, passed: bool, evidence: Value) -> Result<(), AgentError> {
        let p = self.proposals.iter_mut().find(|p| p.id == proposal_id).ok_or_else(|| AgentError::InvalidProposal(format!("no proposal {proposal_id}")))?;
        let digest = sha256_hex(evidence.to_string().as_bytes());
        p.set_validation(passed, evidence);
        self.log(Record { actor: "helqix".into(), action: "validation".into(), output_sha256: Some(digest), decision: "allowed".into(),
                          detail: format!("proposal {proposal_id}: {}", if passed { "SupportedBySimulation" } else { "RejectedBySimulation" }), ..Record::default() });
        Ok(())
    }

    pub fn call_tool(&mut self, req: &ToolRequest, actor: &str) -> Result<Value, AgentError> {
        let action = action_sha256(&req.tool, &req.input);
        let base = Record { actor: actor.into(), action: "tool_call".into(), tool: Some(req.tool.clone()), input_sha256: Some(action.clone()), ..Record::default() };
        let deny = |s: &mut Self, mut r: Record, e: AgentError| { r.decision = format!("denied: {e}"); s.log(r); Err(e) };
        if self.tool_calls >= self.limits.max_tool_calls { return deny(self, base, AgentError::LimitExceeded(format!("{} tool calls", self.limits.max_tool_calls))); }
        self.tool_calls += 1;
        let Some((spec, _)) = self.tools.get(&req.tool) else { return deny(self, base, AgentError::UnknownTool(req.tool.clone())) };
        let required = spec.required;
        let mut base = Record { required: Some(required.name().into()), ..base };
        if !self.grants.allows(required) { return deny(self, base, AgentError::Denied { tool: req.tool.clone(), required }); }
        if required.needs_per_action_approval() {
            match self.approvals.remove(&action) {
                Some(approver) => base.approver = Some(approver),
                None => return deny(self, base, AgentError::ApprovalRequired { tool: req.tool.clone(), action_sha256: action }),
            }
        }
        let result = match req.tool.as_str() {
            "propose" => {
                let id = format!("proposal-{}", self.proposals.len() + 1);
                match Proposal::from_agent(id.clone(), &req.input) {
                    Ok(p) => { self.proposals.push(p); Ok(json!({"proposal_id": id, "origin": crate::proposal::ORIGIN_AI, "status": "Unverified"})) }
                    Err(m) => Err(AgentError::InvalidProposal(m)),
                }
            }
            "apply_proposal" => self.apply(&req.input),
            _ => match self.tools.get_mut(&req.tool).and_then(|t| t.1.as_mut()) {
                Some(f) => f(&req.input).map_err(AgentError::Tool),
                None => Err(AgentError::Tool("tool has no implementation".into())),
            },
        };
        match result {
            Ok(out) => { base.decision = "allowed".into(); base.output_sha256 = Some(sha256_hex(out.to_string().as_bytes())); self.log(base); Ok(out) }
            Err(e) => deny(self, base, e),
        }
    }

    fn apply(&mut self, input: &Value) -> Result<Value, AgentError> {
        let id = input["proposal_id"].as_str().ok_or_else(|| AgentError::InvalidProposal("proposal_id must be text".into()))?;
        let idx = self.proposals.iter().position(|p| p.id == id).ok_or_else(|| AgentError::InvalidProposal(format!("no proposal {id}")))?;
        if self.proposals[idx].status() != ProposalStatus::SupportedBySimulation {
            return Err(AgentError::NotValidated(format!("proposal {id} is {:?}", self.proposals[idx].status())));
        }
        let intervention = self.proposals[idx].intervention.clone();
        let f = self.tools.get_mut("apply_proposal").and_then(|t| t.1.as_mut()).ok_or_else(|| AgentError::Tool("no apply_proposal implementation registered".into()))?;
        let out = f(&intervention).map_err(AgentError::Tool)?;
        self.proposals[idx].set_applied();
        Ok(out)
    }

    /// One agent turn: ask the provider, enforce limits, run its tool requests, and append the
    /// results to the transcript as untrusted data.
    pub fn step(&mut self, provider: &mut dyn AgentProvider, transcript: &mut Vec<Message>) -> Result<Vec<Result<Value, AgentError>>, AgentError> {
        let actor = format!("agent:{}", provider.name());
        if self.provider_calls >= self.limits.max_provider_calls {
            let e = AgentError::LimitExceeded(format!("{} provider calls", self.limits.max_provider_calls));
            self.log(Record { actor, action: "provider_reply".into(), decision: format!("denied: {e}"), ..Record::default() });
            return Err(e);
        }
        self.provider_calls += 1;
        let reply = provider.reply(transcript).map_err(AgentError::Provider)?;
        self.usage.input_tokens += reply.usage.input_tokens;
        self.usage.output_tokens += reply.usage.output_tokens;
        self.usage.cost_microdollars += reply.usage.cost_microdollars;
        let over = self.usage.input_tokens + self.usage.output_tokens > self.limits.max_tokens || self.usage.cost_microdollars > self.limits.max_cost_microdollars;
        let decision = if over { "denied: session token or cost limit exceeded; tool requests dropped".to_owned() } else { "allowed".to_owned() };
        self.log(Record { actor: actor.clone(), action: "provider_reply".into(), output_sha256: Some(sha256_hex(reply.text.as_bytes())), decision, detail: format!("{} tool requests; usage {:?}", reply.requests.len(), reply.usage), ..Record::default() });
        transcript.push(Message { role: Role::Agent, content: reply.text });
        if over { return Err(AgentError::LimitExceeded("session token or cost limit".into())); }
        let mut results = Vec::new();
        for req in &reply.requests {
            let r = self.call_tool(req, &actor);
            let shown = match &r { Ok(v) => v.to_string(), Err(e) => format!("error: {e}") };
            transcript.push(Message { role: Role::ToolResult, content: format!("[untrusted tool output from {}] {shown}", req.tool) });
            results.push(r);
        }
        Ok(results)
    }
}
