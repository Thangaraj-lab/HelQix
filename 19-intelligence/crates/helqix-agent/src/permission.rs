//! The permission ladder. A session holds a set of granted levels; a tool declares the one level
//! it needs. Levels are not cumulative (the default grants Propose without Plan).
use std::collections::BTreeSet;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum Permission { Read, Analyze, Plan, Propose, Simulate, Test, Execute, Mutate }

impl Permission {
    pub const ALL: [Permission; 8] = [Self::Read, Self::Analyze, Self::Plan, Self::Propose, Self::Simulate, Self::Test, Self::Execute, Self::Mutate];
    /// Execute and Mutate change reality: each call needs its own human approval.
    pub fn needs_per_action_approval(self) -> bool { matches!(self, Self::Execute | Self::Mutate) }
    pub fn name(self) -> &'static str {
        match self {
            Self::Read => "Read", Self::Analyze => "Analyze", Self::Plan => "Plan", Self::Propose => "Propose",
            Self::Simulate => "Simulate", Self::Test => "Test", Self::Execute => "Execute", Self::Mutate => "Mutate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grants(BTreeSet<Permission>);

impl Grants {
    /// Read + Analyze + Propose.
    pub fn default_session() -> Self { Self([Permission::Read, Permission::Analyze, Permission::Propose].into()) }
    pub fn none() -> Self { Self(BTreeSet::new()) }
    pub fn allows(&self, p: Permission) -> bool { self.0.contains(&p) }
    pub fn levels(&self) -> Vec<Permission> { self.0.iter().copied().collect() }
    pub(crate) fn insert(&mut self, p: Permission) { self.0.insert(p); }
    pub(crate) fn remove(&mut self, p: Permission) { self.0.remove(&p); }
}
