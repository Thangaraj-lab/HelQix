//! Linux Podman-backed execution sandbox.
//!
//! This backend runs allow-listed commands inside an OCI container and asks
//! Podman to enforce read-only root, no network, memory and PID limits, and
//! explicitly declared writable bind mounts. Network allow-lists are
//! intentionally unsupported because they require a separately enforced
//! egress proxy/firewall.

use crate::isolation::IsolationPolicy;
use crate::worker_runtime::{SandboxedExecutor, WorkerExecutionSpec};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PodmanCommand {
    pub image: String,
    pub executable: String,
    pub args: Vec<String>,
    pub working_directory: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub timed_out: bool,
    pub cancelled: bool,
}

impl SandboxOutput {
    pub fn succeeded(&self) -> bool {
        self.exit_code == Some(0) && !self.timed_out && !self.cancelled
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PodmanError {
    UnsupportedPlatform,
    EmptyCommandMap,
    ZeroOutputLimit,
    InvalidOperation,
    MissingCommandTemplate(String),
    InvalidImage,
    InvalidExecutable,
    InvalidWorkingDirectory,
    WorkingDirectoryNotWritable,
    InvalidWritablePath(String),
    UnsupportedNetworkPolicy,
    SubsecondCpuBudgetUnsupported,
    InvalidIsolation(String),
    SpawnFailed(String),
    WaitFailed(String),
    CancelledBeforeStart,
}

impl std::fmt::Display for PodmanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for PodmanError {}

/// Commands are trusted administrator configuration, never user-provided shell
/// fragments. Each operation maps to one fixed image/executable/argument list.
pub struct PodmanSandboxExecutor {
    podman_binary: PathBuf,
    commands: BTreeMap<String, PodmanCommand>,
    max_output_bytes_per_stream: usize,
}

impl PodmanSandboxExecutor {
    pub fn new(commands: BTreeMap<String, PodmanCommand>, max_output_bytes_per_stream: usize) -> Result<Self, PodmanError> {
        if commands.is_empty() { return Err(PodmanError::EmptyCommandMap); }
        if max_output_bytes_per_stream == 0 { return Err(PodmanError::ZeroOutputLimit); }
        if commands.keys().any(|operation| operation.trim().is_empty()) { return Err(PodmanError::InvalidOperation); }
        Ok(Self { podman_binary: PathBuf::from("podman"), commands, max_output_bytes_per_stream })
    }

    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.podman_binary = binary.into();
        self
    }

    pub fn validate_command(command: &PodmanCommand, policy: &IsolationPolicy) -> Result<(), PodmanError> {
        policy.validate().map_err(|e| PodmanError::InvalidIsolation(e.to_string()))?;
        if policy.network_enabled || !policy.allowed_hosts.is_empty() { return Err(PodmanError::UnsupportedNetworkPolicy); }
        if command.image.trim().is_empty() || command.image.chars().any(char::is_whitespace) || command.image.starts_with('-') { return Err(PodmanError::InvalidImage); }
        if !command.executable.starts_with('/') || command.executable.contains('\0') { return Err(PodmanError::InvalidExecutable); }
        if !command.working_directory.is_absolute() { return Err(PodmanError::InvalidWorkingDirectory); }
        let directory = fs::canonicalize(&command.working_directory).map_err(|_| PodmanError::InvalidWorkingDirectory)?;
        if !directory.is_dir() { return Err(PodmanError::InvalidWorkingDirectory); }
        let mut roots = Vec::new();
        for root in &policy.writable_paths {
            let root_path = Path::new(root);
            if !root_path.is_absolute() || root.contains(':') || root.contains(',') { return Err(PodmanError::InvalidWritablePath(root.clone())); }
            let canonical = fs::canonicalize(root_path).map_err(|_| PodmanError::InvalidWritablePath(root.clone()))?;
            if !canonical.is_dir() || canonical.to_string_lossy() != root.as_str() { return Err(PodmanError::InvalidWritablePath(root.clone())); }
            roots.push(canonical);
        }
        if !roots.iter().any(|root| directory == *root || directory.starts_with(root)) { return Err(PodmanError::WorkingDirectoryNotWritable); }
        Ok(())
    }

    fn execute_inner(&self, spec: &WorkerExecutionSpec, cancellation: &AtomicBool) -> Result<SandboxOutput, PodmanError> {
        let command = self.commands.get(&spec.operation).ok_or_else(|| PodmanError::MissingCommandTemplate(spec.operation.clone()))?;
        Self::validate_command(command, &spec.isolation)?;
        if cancellation.load(Ordering::Relaxed) { return Err(PodmanError::CancelledBeforeStart); }
        if spec.limits.timeout_ms == 0 || spec.request.timeout_ms != spec.limits.timeout_ms { return Err(PodmanError::InvalidOperation); }
        if spec.isolation.max_cpu_millis < 1_000 { return Err(PodmanError::SubsecondCpuBudgetUnsupported); }
        let cpu_seconds = spec.isolation.max_cpu_millis / 1_000 + u64::from(spec.isolation.max_cpu_millis % 1_000 != 0);
        let cidfile = CidFile::new()?;
        let mut process = Command::new(&self.podman_binary);
        process.arg("run").arg("--rm").arg("--pull=never")
            .arg(format!("--cidfile={}", cidfile.path.display()))
            .arg("--network=none").arg("--read-only")
            .arg("--security-opt=no-new-privileges").arg("--cap-drop=ALL")
            .arg("--pids-limit").arg(spec.isolation.max_processes.to_string())
            .arg("--memory").arg(format!("{}b", spec.isolation.max_memory_bytes))
            .arg(format!("--ulimit=cpu={cpu_seconds}:{cpu_seconds}"))
            .arg("--userns=keep-id");
        for root in &spec.isolation.writable_paths { process.arg("--volume").arg(format!("{root}:{root}:rw")); }
        process.arg("--workdir").arg(&command.working_directory).arg(&command.image).arg(&command.executable).args(&command.args)
            .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = process.spawn().map_err(|e| PodmanError::SpawnFailed(e.to_string()))?;
        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");
        let limit = self.max_output_bytes_per_stream;
        let stdout_reader = thread::spawn(move || read_bounded(stdout, limit));
        let stderr_reader = thread::spawn(move || read_bounded(stderr, limit));
        let deadline = Instant::now() + Duration::from_millis(spec.limits.timeout_ms);
        let mut timed_out = false;
        let mut cancelled = false;
        loop {
            if cancellation.load(Ordering::Relaxed) { cancelled = true; stop_container(&self.podman_binary, &cidfile.path); let _ = child.kill(); break; }
            match child.try_wait().map_err(|e| PodmanError::WaitFailed(e.to_string()))? {
                Some(_) => break,
                None if Instant::now() >= deadline => { timed_out = true; stop_container(&self.podman_binary, &cidfile.path); let _ = child.kill(); break; }
                None => thread::sleep(Duration::from_millis(10)),
            }
        }
        let status = child.wait().map_err(|e| PodmanError::WaitFailed(e.to_string()))?;
        let stdout = stdout_reader.join().map_err(|_| PodmanError::WaitFailed("stdout reader panicked".into()))?;
        let stderr = stderr_reader.join().map_err(|_| PodmanError::WaitFailed("stderr reader panicked".into()))?;
        Ok(SandboxOutput { exit_code: status.code(), stdout: String::from_utf8_lossy(&stdout.bytes).into_owned(), stderr: String::from_utf8_lossy(&stderr.bytes).into_owned(), stdout_truncated: stdout.truncated, stderr_truncated: stderr.truncated, timed_out, cancelled })
    }
}

impl SandboxedExecutor for PodmanSandboxExecutor {
    type Output = SandboxOutput;
    type Error = PodmanError;
    fn execute(&self, spec: &WorkerExecutionSpec, cancellation: &AtomicBool) -> Result<Self::Output, Self::Error> {
        if !cfg!(target_os = "linux") { return Err(PodmanError::UnsupportedPlatform); }
        self.execute_inner(spec, cancellation)
    }
}

struct BoundedBytes { bytes: Vec<u8>, truncated: bool }
fn read_bounded<R: Read>(mut reader: R, limit: usize) -> BoundedBytes {
    let mut kept = Vec::with_capacity(limit.min(8192));
    let mut buffer = [0_u8; 8192];
    let mut truncated = false;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(count) => { let remaining = limit.saturating_sub(kept.len()); let keep = remaining.min(count); kept.extend_from_slice(&buffer[..keep]); truncated |= keep < count; }
        }
    }
    BoundedBytes { bytes: kept, truncated }
}

fn stop_container(podman_binary: &Path, cidfile: &Path) {
    let Ok(id) = fs::read_to_string(cidfile) else { return; };
    let id = id.trim();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_hexdigit()) { return; }
    let _ = Command::new(podman_binary).arg("kill").arg(id).status();
}

struct CidFile { path: PathBuf }
impl CidFile {
    fn new() -> Result<Self, PodmanError> {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let path = std::env::temp_dir().join(format!("helqix-{}-{nonce}.cid", std::process::id()));
        if path.exists() { return Err(PodmanError::SpawnFailed("unable to allocate unique cidfile".into())); }
        Ok(Self { path })
    }
}
impl Drop for CidFile { fn drop(&mut self) { let _ = fs::remove_file(&self.path); } }

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    fn policy() -> IsolationPolicy { IsolationPolicy { network_enabled:false, allowed_hosts:BTreeSet::new(), read_only_root:true, writable_paths:[std::env::temp_dir().to_string_lossy().into_owned()].into_iter().collect(), max_processes:8, max_memory_bytes:512*1024*1024, max_cpu_millis:2_000, secret_ids:BTreeSet::new() } }
    fn command() -> PodmanCommand { PodmanCommand { image:"localhost/helqix-test:latest".into(), executable:"/usr/bin/true".into(), args:vec![], working_directory:std::env::temp_dir() } }
    #[test] fn rejects_network_policy() { let mut p=policy(); p.network_enabled=true; p.allowed_hosts.insert("solver.example".into()); assert_eq!(PodmanSandboxExecutor::validate_command(&command(),&p),Err(PodmanError::UnsupportedNetworkPolicy)); }
    #[test] fn rejects_ungrounded_workdir() { let mut p=policy(); p.writable_paths.clear(); assert_eq!(PodmanSandboxExecutor::validate_command(&command(),&p),Err(PodmanError::WorkingDirectoryNotWritable)); }
    #[test] fn rejects_relative_executable() { let mut c=command(); c.executable="python".into(); assert_eq!(PodmanSandboxExecutor::validate_command(&c,&policy()),Err(PodmanError::InvalidExecutable)); }
    #[test] fn bounds_output() { let output=read_bounded(b"abcdefghij".as_slice(),4); assert_eq!(output.bytes,b"abcd"); assert!(output.truncated); }
    #[test] fn requires_allowlisted_template() { let e=PodmanSandboxExecutor::new([("simulation".into(),command())].into_iter().collect(),4096).unwrap(); assert!(!e.commands.contains_key("quantum")); }
}
