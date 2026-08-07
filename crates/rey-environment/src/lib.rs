#![forbid(unsafe_code)]

use std::{
    ffi::OsString,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use polars::df;
use rey_core::{SemanticDigest, SemanticHasher};
use rey_dataframe::{Frame, FrameError, FrameMetadata};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(unix)]
use rustix::process::{Pid, Signal, kill_process_group};
#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub const CAPABILITY_RELATION: &str = "rey.capabilities";
pub const CAPABILITY_SCHEMA_VERSION: &str = "1";
pub const LOCAL_PROVIDER_REVISION: u64 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Available,
    Unavailable,
    Error,
}

impl Availability {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable => "unavailable",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustClass {
    BuiltIn,
    ExplicitLocal,
    DiscoveredLocal,
}

impl TrustClass {
    const fn as_str(self) -> &'static str {
        match self {
            Self::BuiltIn => "built_in",
            Self::ExplicitLocal => "explicit_local",
            Self::DiscoveredLocal => "discovered_local",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityRecord {
    pub provider_id: String,
    pub provider_revision: u64,
    pub provider_kind: String,
    pub capability_id: String,
    pub capability_kind: String,
    pub resolved_location: Option<String>,
    pub version: Option<String>,
    pub content_digest: Option<String>,
    pub provenance: Option<String>,
    pub availability: Availability,
    pub trust_class: TrustClass,
    pub operations: Vec<String>,
    pub enforced_limits: Vec<String>,
    pub unsupported_limits: Vec<String>,
    pub observed_at: Option<String>,
    pub error_code: Option<String>,
    pub error_detail: Option<String>,
}

impl CapabilityRecord {
    fn normalize(&mut self) {
        for values in [
            &mut self.operations,
            &mut self.enforced_limits,
            &mut self.unsupported_limits,
        ] {
            values.sort();
            values.dedup();
        }
    }

    fn add_semantics(&self, hasher: &mut SemanticHasher) {
        hasher.add_str(&self.provider_id);
        hasher.add_u64(self.provider_revision);
        hasher.add_str(&self.provider_kind);
        hasher.add_str(&self.capability_id);
        hasher.add_str(&self.capability_kind);
        hasher.add_optional_str(self.resolved_location.as_deref());
        hasher.add_optional_str(self.version.as_deref());
        hasher.add_optional_str(self.content_digest.as_deref());
        hasher.add_optional_str(self.provenance.as_deref());
        hasher.add_str(self.availability.as_str());
        hasher.add_str(self.trust_class.as_str());
        add_strings(hasher, &self.operations);
        add_strings(hasher, &self.enforced_limits);
        add_strings(hasher, &self.unsupported_limits);
        hasher.add_optional_str(self.error_code.as_deref());
    }
}

fn add_strings(hasher: &mut SemanticHasher, values: &[String]) {
    hasher.add_u64(values.len() as u64);
    for value in values {
        hasher.add_str(value);
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiscoveryLimits {
    pub total_timeout_ms: u64,
    pub probe_timeout_ms: u64,
    pub max_capture_bytes: u64,
    pub max_capabilities: u64,
}

impl Default for DiscoveryLimits {
    fn default() -> Self {
        Self {
            total_timeout_ms: 5_000,
            probe_timeout_ms: 1_000,
            max_capture_bytes: 65_536,
            max_capabilities: 64,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilitySnapshot {
    pub schema: String,
    pub profile: String,
    pub semantic_digest: SemanticDigest,
    pub complete: bool,
    pub limits: DiscoveryLimits,
    pub capabilities: Vec<CapabilityRecord>,
}

impl CapabilitySnapshot {
    pub fn new(
        profile: impl Into<String>,
        limits: DiscoveryLimits,
        mut capabilities: Vec<CapabilityRecord>,
    ) -> Result<Self, DiscoveryError> {
        if capabilities.len() as u64 > limits.max_capabilities {
            return Err(DiscoveryError::CapabilityLimit {
                limit: limits.max_capabilities,
                observed: capabilities.len() as u64,
            });
        }
        for capability in &mut capabilities {
            capability.normalize();
        }
        capabilities.sort_by(|left, right| {
            (
                &left.provider_id,
                left.provider_revision,
                &left.capability_id,
            )
                .cmp(&(
                    &right.provider_id,
                    right.provider_revision,
                    &right.capability_id,
                ))
        });
        let profile = profile.into();
        let mut hasher = SemanticHasher::new("rey.capability-snapshot.v1");
        hasher.add_str(CAPABILITY_SCHEMA_VERSION);
        hasher.add_str(&profile);
        hasher.add_u64(limits.total_timeout_ms);
        hasher.add_u64(limits.probe_timeout_ms);
        hasher.add_u64(limits.max_capture_bytes);
        hasher.add_u64(limits.max_capabilities);
        hasher.add_u64(capabilities.len() as u64);
        for capability in &capabilities {
            capability.add_semantics(&mut hasher);
        }
        let complete = capabilities
            .iter()
            .all(|row| row.availability != Availability::Error);
        Ok(Self {
            schema: format!("{CAPABILITY_RELATION}.v{CAPABILITY_SCHEMA_VERSION}"),
            profile,
            semantic_digest: hasher.finish(),
            complete,
            limits,
            capabilities,
        })
    }

    pub fn push(&mut self, capability: CapabilityRecord) -> Result<(), DiscoveryError> {
        let mut capabilities = self.capabilities.clone();
        capabilities.push(capability);
        *self = Self::new(self.profile.clone(), self.limits.clone(), capabilities)?;
        Ok(())
    }

    pub fn to_frame(&self) -> Result<Frame, DiscoveryError> {
        let rows = &self.capabilities;
        let operations = canonical_arrays(rows.iter().map(|row| &row.operations))?;
        let enforced = canonical_arrays(rows.iter().map(|row| &row.enforced_limits))?;
        let unsupported = canonical_arrays(rows.iter().map(|row| &row.unsupported_limits))?;
        let dataframe = df!(
            "provider_id" => rows.iter().map(|row| row.provider_id.as_str()).collect::<Vec<_>>(),
            "provider_revision" => rows.iter().map(|row| row.provider_revision).collect::<Vec<_>>(),
            "provider_kind" => rows.iter().map(|row| row.provider_kind.as_str()).collect::<Vec<_>>(),
            "capability_id" => rows.iter().map(|row| row.capability_id.as_str()).collect::<Vec<_>>(),
            "capability_kind" => rows.iter().map(|row| row.capability_kind.as_str()).collect::<Vec<_>>(),
            "resolved_location" => rows.iter().map(|row| row.resolved_location.as_deref()).collect::<Vec<_>>(),
            "version" => rows.iter().map(|row| row.version.as_deref()).collect::<Vec<_>>(),
            "content_digest" => rows.iter().map(|row| row.content_digest.as_deref()).collect::<Vec<_>>(),
            "provenance" => rows.iter().map(|row| row.provenance.as_deref()).collect::<Vec<_>>(),
            "availability" => rows.iter().map(|row| row.availability.as_str()).collect::<Vec<_>>(),
            "trust_class" => rows.iter().map(|row| row.trust_class.as_str()).collect::<Vec<_>>(),
            "operations" => operations,
            "enforced_limits" => enforced,
            "unsupported_limits" => unsupported,
            "observed_at" => rows.iter().map(|row| row.observed_at.as_deref()).collect::<Vec<_>>(),
            "error_code" => rows.iter().map(|row| row.error_code.as_deref()).collect::<Vec<_>>(),
            "error_detail" => rows.iter().map(|row| row.error_detail.as_deref()).collect::<Vec<_>>(),
        )?;
        Ok(Frame::new(
            dataframe,
            FrameMetadata {
                relation: CAPABILITY_RELATION.to_owned(),
                schema_version: CAPABILITY_SCHEMA_VERSION.to_owned(),
                semantic_digest: self.semantic_digest.to_string(),
                row_count: rows.len() as u64,
                complete: self.complete,
                key_columns: vec![
                    "provider_id".to_owned(),
                    "provider_revision".to_owned(),
                    "capability_id".to_owned(),
                ],
            },
        )?)
    }
}

fn canonical_arrays<'a>(
    values: impl Iterator<Item = &'a Vec<String>>,
) -> Result<Vec<String>, serde_json::Error> {
    values.map(serde_json::to_string).collect()
}

#[derive(Clone, Debug)]
pub struct LocalDiscovery {
    pub workspace: PathBuf,
    pub search_paths: Vec<PathBuf>,
    pub limits: DiscoveryLimits,
}

impl LocalDiscovery {
    pub fn from_environment(workspace: PathBuf, limits: DiscoveryLimits) -> Self {
        let search_paths = std::env::var_os("PATH")
            .map(|path| std::env::split_paths(&path).collect())
            .unwrap_or_default();
        Self {
            workspace,
            search_paths,
            limits,
        }
    }

    pub fn inspect(&self) -> Result<CapabilitySnapshot, DiscoveryError> {
        let started = Instant::now();
        let workspace =
            fs::canonicalize(&self.workspace).map_err(|source| DiscoveryError::Workspace {
                path: self.workspace.clone(),
                source,
            })?;
        if !workspace.is_dir() {
            return Err(DiscoveryError::WorkspaceNotDirectory(workspace));
        }

        let mut capabilities = vec![builtin_capability(), workspace_capability(&workspace)];
        for adapter in TOOL_ADAPTERS {
            capabilities.push(self.inspect_tool(adapter, &workspace, started));
        }
        CapabilitySnapshot::new("standalone", self.limits.clone(), capabilities)
    }

    fn inspect_tool(
        &self,
        adapter: &ToolAdapter,
        workspace: &Path,
        started: Instant,
    ) -> CapabilityRecord {
        let Some(program) = resolve_executable(adapter.name, &self.search_paths) else {
            return unavailable_tool(adapter, "not_found", "not found in configured search paths");
        };
        let total = Duration::from_millis(self.limits.total_timeout_ms);
        let Some(remaining) = total.checked_sub(started.elapsed()) else {
            return error_tool(
                adapter,
                &program,
                "discovery_deadline",
                "total discovery deadline elapsed",
            );
        };
        let timeout = remaining.min(Duration::from_millis(self.limits.probe_timeout_ms));
        let request = CommandRequest {
            program: program.clone(),
            args: adapter.args.iter().map(OsString::from).collect(),
            cwd: workspace.to_owned(),
            timeout,
            max_capture_bytes: self.limits.max_capture_bytes,
            environment: vec![(OsString::from("LC_ALL"), OsString::from("C"))],
        };
        match run_bounded(&request) {
            Ok(output) if output.timed_out => error_tool(
                adapter,
                &program,
                "probe_timeout",
                "identity probe exceeded its deadline",
            ),
            Ok(output) if output.overflowed => error_tool(
                adapter,
                &program,
                "probe_output_limit",
                "identity probe exceeded its capture limit",
            ),
            Ok(output) if !output.status.success() => error_tool(
                adapter,
                &program,
                "probe_nonzero",
                &format!(
                    "identity probe exited with {}",
                    display_status(output.status)
                ),
            ),
            Ok(output) => match parse_version(&output.stdout) {
                Ok(version) => available_tool(adapter, &program, version),
                Err(detail) => error_tool(adapter, &program, "probe_malformed", detail),
            },
            Err(error) => error_tool(adapter, &program, "probe_failed", &error.to_string()),
        }
    }
}

struct ToolAdapter {
    name: &'static str,
    capability_id: &'static str,
    args: &'static [&'static str],
}

const TOOL_ADAPTERS: &[ToolAdapter] = &[
    ToolAdapter {
        name: "git",
        capability_id: "tool.git.identity",
        args: &["--version"],
    },
    ToolAdapter {
        name: "rg",
        capability_id: "tool.ripgrep.identity",
        args: &["--version"],
    },
];

fn builtin_capability() -> CapabilityRecord {
    CapabilityRecord {
        provider_id: "rey.builtin".to_owned(),
        provider_revision: LOCAL_PROVIDER_REVISION,
        provider_kind: "builtin".to_owned(),
        capability_id: "frame.arrow-stream".to_owned(),
        capability_kind: "typed_frame".to_owned(),
        resolved_location: None,
        version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        content_digest: None,
        provenance: Some("compiled_rey_runtime".to_owned()),
        availability: Availability::Available,
        trust_class: TrustClass::BuiltIn,
        operations: vec!["encode_arrow_stream".to_owned(), "render_table".to_owned()],
        enforced_limits: vec!["bounded_input_rows".to_owned()],
        unsupported_limits: Vec::new(),
        observed_at: None,
        error_code: None,
        error_detail: None,
    }
}

fn workspace_capability(workspace: &Path) -> CapabilityRecord {
    CapabilityRecord {
        provider_id: "rey.workspace".to_owned(),
        provider_revision: LOCAL_PROVIDER_REVISION,
        provider_kind: "workspace".to_owned(),
        capability_id: "workspace.metadata".to_owned(),
        capability_kind: "context_surface".to_owned(),
        resolved_location: Some(workspace.display().to_string()),
        version: None,
        content_digest: None,
        provenance: Some("explicit_canonical_root".to_owned()),
        availability: Availability::Available,
        trust_class: TrustClass::ExplicitLocal,
        operations: vec!["inspect_metadata".to_owned()],
        enforced_limits: vec!["canonical_workspace_root".to_owned()],
        unsupported_limits: vec!["filesystem_sandbox".to_owned()],
        observed_at: None,
        error_code: None,
        error_detail: None,
    }
}

fn available_tool(adapter: &ToolAdapter, program: &Path, version: String) -> CapabilityRecord {
    CapabilityRecord {
        provider_id: format!("rey.tool.{}", adapter.name),
        provider_revision: LOCAL_PROVIDER_REVISION,
        provider_kind: "known_tool".to_owned(),
        capability_id: adapter.capability_id.to_owned(),
        capability_kind: "identity_probe".to_owned(),
        resolved_location: Some(program.display().to_string()),
        version: Some(version),
        content_digest: None,
        provenance: Some("configured_search_path_and_fixed_version_probe".to_owned()),
        availability: Availability::Available,
        trust_class: TrustClass::DiscoveredLocal,
        operations: vec!["inspect_identity".to_owned()],
        enforced_limits: vec![
            "capture_bytes".to_owned(),
            "cleared_environment".to_owned(),
            "direct_argv".to_owned(),
            "wall_timeout".to_owned(),
        ],
        unsupported_limits: vec!["process_sandbox".to_owned()],
        observed_at: None,
        error_code: None,
        error_detail: None,
    }
}

fn unavailable_tool(adapter: &ToolAdapter, code: &str, detail: &str) -> CapabilityRecord {
    failed_tool(adapter, None, Availability::Unavailable, code, detail)
}

fn error_tool(adapter: &ToolAdapter, program: &Path, code: &str, detail: &str) -> CapabilityRecord {
    failed_tool(adapter, Some(program), Availability::Error, code, detail)
}

fn failed_tool(
    adapter: &ToolAdapter,
    program: Option<&Path>,
    availability: Availability,
    code: &str,
    detail: &str,
) -> CapabilityRecord {
    CapabilityRecord {
        provider_id: format!("rey.tool.{}", adapter.name),
        provider_revision: LOCAL_PROVIDER_REVISION,
        provider_kind: "known_tool".to_owned(),
        capability_id: adapter.capability_id.to_owned(),
        capability_kind: "identity_probe".to_owned(),
        resolved_location: program.map(|path| path.display().to_string()),
        version: None,
        content_digest: None,
        provenance: None,
        availability,
        trust_class: TrustClass::DiscoveredLocal,
        operations: Vec::new(),
        enforced_limits: vec![
            "capture_bytes".to_owned(),
            "cleared_environment".to_owned(),
            "direct_argv".to_owned(),
            "wall_timeout".to_owned(),
        ],
        unsupported_limits: vec!["process_sandbox".to_owned()],
        observed_at: None,
        error_code: Some(code.to_owned()),
        error_detail: Some(detail.to_owned()),
    }
}

fn parse_version(stdout: &[u8]) -> Result<String, &'static str> {
    let stdout = std::str::from_utf8(stdout).map_err(|_| "identity output is not UTF-8")?;
    let version = stdout
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(str::trim);
    match version {
        Some(version) if version.len() <= 256 => Ok(version.to_owned()),
        Some(_) => Err("identity line exceeds 256 bytes"),
        None => Err("identity output has no non-empty line"),
    }
}

#[must_use]
pub fn resolve_executable(name: &str, search_paths: &[PathBuf]) -> Option<PathBuf> {
    search_paths.iter().find_map(|directory| {
        if directory.as_os_str().is_empty() || !directory.is_absolute() {
            return None;
        }
        let candidate = directory.join(name);
        if is_executable(&candidate) {
            // Preserve the invoked basename. Some tool distributions use one
            // multi-call executable behind several symlinks and select the
            // operation from argv[0].
            Some(candidate)
        } else {
            None
        }
    })
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[derive(Clone, Debug)]
pub struct CommandRequest {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
    pub timeout: Duration,
    pub max_capture_bytes: u64,
    pub environment: Vec<(OsString, OsString)>,
}

#[derive(Debug)]
pub struct CommandOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
    pub overflowed: bool,
}

pub fn run_bounded(request: &CommandRequest) -> Result<CommandOutput, CommandError> {
    let mut command = Command::new(&request.program);
    command
        .args(&request.args)
        .current_dir(&request.cwd)
        .env_clear()
        .envs(request.environment.iter().cloned())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().map_err(CommandError::Spawn)?;
    #[cfg(unix)]
    let process_group = i32::try_from(child.id())
        .ok()
        .and_then(Pid::from_raw)
        .ok_or(CommandError::InvalidProcessId)?;
    let stdout = child
        .stdout
        .take()
        .ok_or(CommandError::MissingPipe("stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or(CommandError::MissingPipe("stderr"))?;
    let overflowed = Arc::new(AtomicBool::new(false));
    let stdout_thread = drain_bounded(stdout, request.max_capture_bytes, Arc::clone(&overflowed));
    let stderr_thread = drain_bounded(stderr, request.max_capture_bytes, Arc::clone(&overflowed));
    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(CommandError::Wait)? {
            break status;
        }
        if overflowed.load(Ordering::Relaxed) || started.elapsed() >= request.timeout {
            timed_out = started.elapsed() >= request.timeout;
            #[cfg(unix)]
            terminate_process_group(process_group)?;
            #[cfg(not(unix))]
            child.kill().map_err(CommandError::Kill)?;
            break child.wait().map_err(CommandError::Wait)?;
        }
        thread::sleep(Duration::from_millis(2));
    };
    #[cfg(unix)]
    terminate_process_group(process_group)?;
    let stdout = stdout_thread
        .join()
        .map_err(|_| CommandError::ReaderPanic("stdout"))??;
    let stderr = stderr_thread
        .join()
        .map_err(|_| CommandError::ReaderPanic("stderr"))??;
    Ok(CommandOutput {
        status,
        stdout,
        stderr,
        timed_out,
        overflowed: overflowed.load(Ordering::Relaxed),
    })
}

#[cfg(unix)]
fn terminate_process_group(process_group: Pid) -> Result<(), CommandError> {
    match kill_process_group(process_group, Signal::KILL) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(CommandError::KillGroup(error)),
    }
}

fn drain_bounded(
    mut input: impl Read + Send + 'static,
    max_bytes: u64,
    overflowed: Arc<AtomicBool>,
) -> thread::JoinHandle<Result<Vec<u8>, CommandError>> {
    thread::spawn(move || {
        let mut captured = Vec::new();
        let mut buffer = [0_u8; 8_192];
        loop {
            let count = input.read(&mut buffer).map_err(CommandError::Capture)?;
            if count == 0 {
                break;
            }
            let remaining = max_bytes.saturating_sub(captured.len() as u64) as usize;
            captured.extend_from_slice(&buffer[..count.min(remaining)]);
            if count > remaining {
                overflowed.store(true, Ordering::Relaxed);
            }
        }
        Ok(captured)
    })
}

fn display_status(status: ExitStatus) -> String {
    status
        .code()
        .map_or_else(|| "a signal".to_owned(), |code| code.to_string())
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("could not start process: {0}")]
    Spawn(io::Error),
    #[error("process output pipe {0} was unavailable")]
    MissingPipe(&'static str),
    #[error("could not observe process: {0}")]
    Wait(io::Error),
    #[error("could not terminate bounded process: {0}")]
    Kill(io::Error),
    #[cfg(unix)]
    #[error("could not terminate bounded process group: {0}")]
    KillGroup(rustix::io::Errno),
    #[cfg(unix)]
    #[error("child process identifier was outside the supported range")]
    InvalidProcessId,
    #[error("could not capture process output: {0}")]
    Capture(io::Error),
    #[error("{0} capture worker panicked")]
    ReaderPanic(&'static str),
}

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("workspace {path} cannot be resolved: {source}")]
    Workspace { path: PathBuf, source: io::Error },
    #[error("workspace {0} is not a directory")]
    WorkspaceNotDirectory(PathBuf),
    #[error("capability snapshot contains {observed} rows, exceeding limit {limit}")]
    CapabilityLimit { limit: u64, observed: u64 },
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error("capability JSON encoding failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("capability dataframe failed: {0}")]
    Polars(#[from] polars::error::PolarsError),
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::Duration};

    use tempfile::TempDir;

    use super::{
        Availability, CommandRequest, DiscoveryLimits, LocalDiscovery, resolve_executable,
        run_bounded,
    };

    #[test]
    fn zero_tool_discovery_is_still_a_useful_standalone_snapshot() {
        let workspace = TempDir::new().unwrap();
        let snapshot = LocalDiscovery {
            workspace: workspace.path().to_owned(),
            search_paths: Vec::new(),
            limits: DiscoveryLimits::default(),
        }
        .inspect()
        .unwrap();

        assert_eq!(snapshot.profile, "standalone");
        assert_eq!(snapshot.capabilities.len(), 4);
        assert_eq!(
            snapshot
                .capabilities
                .iter()
                .filter(|row| row.availability == Availability::Available)
                .count(),
            2
        );
        assert_eq!(snapshot.to_frame().unwrap().dataframe().height(), 4);
        let repeated = LocalDiscovery {
            workspace: workspace.path().to_owned(),
            search_paths: Vec::new(),
            limits: DiscoveryLimits::default(),
        }
        .inspect()
        .unwrap();
        assert_eq!(snapshot.semantic_digest, repeated.semantic_digest);
    }

    #[cfg(unix)]
    #[test]
    fn command_capture_is_bounded() {
        let directory = TempDir::new().unwrap();
        let input = directory.path().join("oversized");
        fs::write(&input, vec![b'x'; 1_024]).unwrap();
        let output = run_bounded(&CommandRequest {
            program: test_tool("cat"),
            args: vec![input.into_os_string()],
            cwd: directory.path().to_owned(),
            timeout: Duration::from_secs(2),
            max_capture_bytes: 64,
            environment: Vec::new(),
        })
        .unwrap();

        assert!(output.overflowed);
        assert!(output.stdout.len() <= 64);
    }

    #[cfg(unix)]
    #[test]
    fn command_deadline_is_enforced() {
        let directory = TempDir::new().unwrap();
        let output = run_bounded(&CommandRequest {
            program: test_tool("sleep"),
            args: vec!["10".into()],
            cwd: PathBuf::from(directory.path()),
            timeout: Duration::from_millis(20),
            max_capture_bytes: 64,
            environment: Vec::new(),
        })
        .unwrap();

        assert!(
            output.timed_out,
            "status={:?} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[cfg(unix)]
    #[test]
    fn descendants_cannot_hold_capture_open_after_the_probe_exits() {
        let directory = TempDir::new().unwrap();
        let command = format!("{} 10 &", test_tool("sleep").display());
        let started = std::time::Instant::now();
        let output = run_bounded(&CommandRequest {
            program: test_tool("sh"),
            args: vec!["-c".into(), command.into()],
            cwd: directory.path().to_owned(),
            timeout: Duration::from_secs(1),
            max_capture_bytes: 64,
            environment: Vec::new(),
        })
        .unwrap();

        assert!(output.status.success());
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[cfg(unix)]
    fn test_tool(name: &str) -> PathBuf {
        let paths = std::env::split_paths(&std::env::var_os("PATH").unwrap()).collect::<Vec<_>>();
        resolve_executable(name, &paths).unwrap()
    }
}
