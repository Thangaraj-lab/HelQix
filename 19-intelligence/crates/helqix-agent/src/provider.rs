//! Provider abstraction. No model vendor is built in: the founder chooses one, and its current API
//! and pricing are checked when that adapter is written. [`ScriptedProvider`] replays fixed
//! replies for tests and demos; it is not an AI model.
use std::collections::VecDeque;

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ToolRequest { pub tool: String, pub input: Value }

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage { pub input_tokens: u64, pub output_tokens: u64, pub cost_microdollars: u64 }

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderReply { pub text: String, pub requests: Vec<ToolRequest>, pub usage: Usage }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role { Instructions, Agent, ToolResult }

/// One transcript message. `ToolResult` content is untrusted data and is labelled as such.
#[derive(Debug, Clone, PartialEq)]
pub struct Message { pub role: Role, pub content: String }

pub trait AgentProvider {
    fn name(&self) -> &str;
    /// Sees the transcript only; has no access to the session, its grants or its approvals.
    fn reply(&mut self, transcript: &[Message]) -> Result<ProviderReply, String>;
}

pub struct ScriptedProvider { replies: VecDeque<ProviderReply> }

impl ScriptedProvider {
    pub fn new(replies: Vec<ProviderReply>) -> Self { Self { replies: replies.into() } }
}

impl AgentProvider for ScriptedProvider {
    fn name(&self) -> &str { "scripted" }
    fn reply(&mut self, _transcript: &[Message]) -> Result<ProviderReply, String> { self.replies.pop_front().ok_or_else(|| "script exhausted".into()) }
}
