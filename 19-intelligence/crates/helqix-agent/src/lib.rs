//! Agent sessions with permissions (Phase 7B).
//!
//! An agent may read, analyze, plan and propose. It cannot change anything without the right
//! grant, and Execute or Mutate additionally need a human approval for that exact action.
//! Permissions live in [`session::Session`], which the agent never touches: the provider only
//! returns text and tool requests, so nothing an agent, a tool, a file or a solver log says can
//! grant a permission or an approval.
//!
//! Everything the agent proposes is `Origin: AI-hypothesis, Status: Unverified` until HELQIX
//! checks it. This is an engineering assistant with guard rails, not a general intelligence.
pub mod audit;
pub mod permission;
pub mod proposal;
pub mod provider;
pub mod session;

use sha2::{Digest, Sha256};

pub(crate) fn sha256_hex(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
