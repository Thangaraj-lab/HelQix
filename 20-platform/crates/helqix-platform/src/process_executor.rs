//! Development/local process execution adapter.
//!
//! This is a real process runner, but it is intentionally NOT the production
//! sandbox. OS/container enforcement of memory, CPU, filesystem and network
//! policy must be supplied by a hardened worker backend. The adapter therefore
//! rejects network-enabled policies instead of pretending it can enforce them.
use crate::isolation::IsolationPolicy;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSpec {
    pub executable: String,
    pub args: Vec<String>,
    pub working_directory: PathBuf,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessExecutionError {
    EmptyExecutable,
    ZeroTimeout,
    NetworkPolicyUnsupported,
    InvalidWorkingDirectory,
    WorkingDirectoryNotWritable,
    SpawnFailed(String),
    WaitFailed(String),
    OutputFailed(String),
}

pub struct LocalProcessExecutor;

impl LocalProcessExecutor {
    pub fn validate(spec: &ProcessSpec, policy: &IsolationPolicy) -> Result<(), ProcessExecutionError> {
        if spec.executable.trim().is_empty() { return Err(ProcessExecutionError::EmptyExecutable); }
        if spec.timeout_ms == 0 { return Err(ProcessExecutionError::ZeroTimeout); }
        // A plain child process cannot enforce a network deny-list. Refusing
        // the request is safer than executing under a false isolation claim.
        if policy.network_enabled { return Err(ProcessExecutionError::NetworkPolicyUnsupported); }
        if !spec.working_directory.is_absolute() { return Err(ProcessExecutionError::InvalidWorkingDirectory); }
        let path = spec.working_directory.to_string_lossy();
        if !policy.allows_write(&path) { return Err(ProcessExecutionError::WorkingDirectoryNotWritable); }
        Ok(())
    }

    pub fn execute(spec: &ProcessSpec, policy: &IsolationPolicy) -> Result<ProcessOutput, ProcessExecutionError> {
        Self::validate(spec, policy)?;
        let mut child = Command::new(&spec.executable)
            .args(&spec.args)
            .current_dir(&spec.working_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ProcessExecutionError::SpawnFailed(e.to_string()))?;

        let deadline = Instant::now() + Duration::from_millis(spec.timeout_ms);
        loop {
            match child.try_wait().map_err(|e| ProcessExecutionError::WaitFailed(e.to_string()))? {
                Some(status) => return Self::collect(child, Some(status.code()), false),
                None if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let status = child.wait().map_err(|e| ProcessExecutionError::WaitFailed(e.to_string()))?;
                    return Self::collect(child, status.code(), true);
                }
                None => thread::sleep(Duration::from_millis(5)),
            }
        }
    }

    fn collect(mut child: Child, exit_code: Option<i32>, timed_out: bool) -> Result<ProcessOutput, ProcessExecutionError> {
        let output = child.wait_with_output().map_err(|e| ProcessExecutionError::OutputFailed(e.to_string()))?;
        Ok(ProcessOutput {
            exit_code: if timed_out { exit_code } else { output.status.code().or(exit_code) },
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            timed_out,
        })
    }

    pub fn is_within(path: &Path, root: &Path) -> bool {
        path == root || path.starts_with(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn policy() -> IsolationPolicy {
        IsolationPolicy {
            network_enabled: false,
            allowed_hosts: BTreeSet::new(),
            read_only_root: true,
            writable_paths: ["/workspace".into()].into_iter().collect(),
            max_processes: 2,
            max_memory_bytes: 1024 * 1024,
            max_cpu_millis: 1000,
            secret_ids: BTreeSet::new(),
        }
    }

    #[test]
    fn refuses_network_policy_it_cannot_enforce() {
        let mut p = policy(); p.network_enabled = true;
        let spec = ProcessSpec { executable: "solver".into(), args: vec![], working_directory: "/workspace/case".into(), timeout_ms: 1000 };
        assert_eq!(LocalProcessExecutor::validate(&spec, &p), Err(ProcessExecutionError::NetworkPolicyUnsupported));
    }

    #[test]
    fn rejects_working_directory_outside_declared_write_root() {
        let spec = ProcessSpec { executable: "solver".into(), args: vec![], working_directory: "/etc".into(), timeout_ms: 1000 };
        assert_eq!(LocalProcessExecutor::validate(&spec, &policy()), Err(ProcessExecutionError::WorkingDirectoryNotWritable));
    }

    #[test]
    fn validates_basic_process_spec() {
        let spec = ProcessSpec { executable: "solver".into(), args: vec!["--version".into()], working_directory: "/workspace/case".into(), timeout_ms: 1000 };
        assert!(LocalProcessExecutor::validate(&spec, &policy()).is_ok());
    }
}
