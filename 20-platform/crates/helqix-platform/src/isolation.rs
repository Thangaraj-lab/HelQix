//! Execution isolation policy contracts.
//!
//! This module is policy only: it does not spawn processes or containers. A
//! worker runtime must consume this manifest and enforce every restriction
//! before starting an external solver/tool.

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsolationPolicy {
    pub network_enabled: bool,
    pub allowed_hosts: BTreeSet<String>,
    pub read_only_root: bool,
    pub writable_paths: BTreeSet<String>,
    pub max_processes: u32,
    pub max_memory_bytes: u64,
    pub max_cpu_millis: u64,
    pub secret_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsolationError {
    InvalidProcessLimit,
    InvalidMemoryLimit,
    InvalidCpuLimit,
    WritableRoot,
    SecretWithoutNetworkPolicy,
    InvalidWritablePath,
    HostNotAllowed,
}

impl IsolationPolicy {
    pub fn validate(&self) -> Result<(), IsolationError> {
        if self.max_processes == 0 { return Err(IsolationError::InvalidProcessLimit); }
        if self.max_memory_bytes == 0 { return Err(IsolationError::InvalidMemoryLimit); }
        if self.max_cpu_millis == 0 { return Err(IsolationError::InvalidCpuLimit); }
        if !self.read_only_root { return Err(IsolationError::WritableRoot); }
        if !self.network_enabled && !self.secret_ids.is_empty() { return Err(IsolationError::SecretWithoutNetworkPolicy); }
        if self.writable_paths.iter().any(|path| path.is_empty() || !path.starts_with('/')) { return Err(IsolationError::InvalidWritablePath); }
        if self.allowed_hosts.iter().any(|host| host.trim().is_empty()) { return Err(IsolationError::HostNotAllowed); }
        Ok(())
    }

    pub fn allows_host(&self, host: &str) -> bool {
        self.network_enabled && self.allowed_hosts.contains(host)
    }

    pub fn allows_write(&self, path: &str) -> bool {
        self.writable_paths.iter().any(|root| path == root || path.starts_with(&format!("{root}/")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn policy() -> IsolationPolicy {
        IsolationPolicy { network_enabled: false, allowed_hosts: BTreeSet::new(), read_only_root: true, writable_paths: ["/workspace".into()].into_iter().collect(), max_processes: 16, max_memory_bytes: 1_073_741_824, max_cpu_millis: 60_000, secret_ids: BTreeSet::new() }
    }
    #[test] fn denies_network_by_default() { assert!(!policy().allows_host("example.com")); }
    #[test] fn limits_writes_to_declared_paths() { assert!(policy().allows_write("/workspace/case/input.json")); assert!(!policy().allows_write("/etc/passwd")); }
    #[test] fn requires_read_only_root() { let mut p=policy(); p.read_only_root=false; assert_eq!(p.validate(),Err(IsolationError::WritableRoot)); }
}
