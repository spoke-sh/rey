#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
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

mod mapping;
mod source;

#[cfg(test)]
mod source_tests;

pub use mapping::*;
pub use source::*;

#[cfg(unix)]
use rustix::process::{Pid, Signal, kill_process_group};
#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub const CAPABILITY_RELATION: &str = "rey.capabilities";
pub const CAPABILITY_SCHEMA_VERSION: &str = "1";
pub const LOCAL_PROVIDER_REVISION: u64 = 1;
const DISCOVERY_APPLICATION_PROVIDER_REVISION: u64 = 2;
pub const DISCOVERY_SEED_PROVIDER_ID: &str = "rey.discovery-seed";
pub const DISCOVERY_SEED_SCHEMA: &str = "rey.discovery-seeds.v1";
pub const DISCOVERY_APPLICATION_SCHEMA: &str = "rey.discovery-application.v2";
pub const DISCOVERY_SEED_NAMES: [&str; 3] = ["HOME", "PWD", "PATH"];

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
    pub const fn as_str(self) -> &'static str {
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
        if let Some(duplicate) = capabilities
            .windows(2)
            .find(|rows| capability_key(&rows[0]) == capability_key(&rows[1]))
        {
            let row = &duplicate[0];
            return Err(DiscoveryError::DuplicateCapabilityKey {
                provider_id: row.provider_id.clone(),
                provider_revision: row.provider_revision,
                capability_id: row.capability_id.clone(),
            });
        }
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

    /// Parses and verifies a snapshot before it is admitted as runtime input.
    ///
    /// JSON map ordering is irrelevant, but relation schema, row/list ordering,
    /// semantic identity, completeness, and key uniqueness are all canonical.
    pub fn from_json_slice(bytes: &[u8], max_capabilities: u64) -> Result<Self, DiscoveryError> {
        let supplied: Self = serde_json::from_slice(bytes)?;
        if supplied.capabilities.len() as u64 > max_capabilities {
            return Err(DiscoveryError::CapabilityLimit {
                limit: max_capabilities,
                observed: supplied.capabilities.len() as u64,
            });
        }
        supplied.verify()?;
        Ok(supplied)
    }

    /// Recomputes all invariants needed to use this snapshot as semantic input.
    pub fn verify(&self) -> Result<(), DiscoveryError> {
        let expected_schema = format!("{CAPABILITY_RELATION}.v{CAPABILITY_SCHEMA_VERSION}");
        if self.schema != expected_schema {
            return Err(DiscoveryError::UnsupportedSnapshotSchema {
                expected: expected_schema,
                actual: self.schema.clone(),
            });
        }
        let recomputed = Self::new(
            self.profile.clone(),
            self.limits.clone(),
            self.capabilities.clone(),
        )?;
        if self.capabilities != recomputed.capabilities {
            return Err(DiscoveryError::NonCanonicalSnapshot);
        }
        if self.semantic_digest != recomputed.semantic_digest {
            return Err(DiscoveryError::SnapshotDigest {
                declared: self.semantic_digest.clone(),
                actual: recomputed.semantic_digest,
            });
        }
        if self.complete != recomputed.complete {
            return Err(DiscoveryError::SnapshotCompleteness {
                declared: self.complete,
                actual: recomputed.complete,
            });
        }
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
                attributes: Default::default(),
            },
        )?)
    }
}

fn capability_key(row: &CapabilityRecord) -> (&str, u64, &str) {
    (
        row.provider_id.as_str(),
        row.provider_revision,
        row.capability_id.as_str(),
    )
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
    pub seed_values: BTreeMap<String, OsString>,
    pub limits: DiscoveryLimits,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiscoverySeedProvenance {
    pub schema: String,
    pub name: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiscoveryApplicationProvenance {
    pub schema: String,
    pub name: String,
    pub groups: Vec<String>,
    pub purpose: String,
    pub required: bool,
    pub potential_capabilities: Vec<String>,
    pub search_path_count: u64,
}

impl LocalDiscovery {
    pub fn from_environment(workspace: PathBuf, limits: DiscoveryLimits) -> Self {
        let search_paths = std::env::var_os("PATH")
            .map(|path| std::env::split_paths(&path).collect())
            .unwrap_or_default();
        let seed_values = DISCOVERY_SEED_NAMES
            .into_iter()
            .filter_map(|name| std::env::var_os(name).map(|value| (name.to_owned(), value)))
            .collect();
        Self {
            workspace,
            search_paths,
            seed_values,
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

        let mut capabilities = Vec::new();
        for name in DISCOVERY_SEED_NAMES {
            capabilities.push(discovery_seed_capability(
                name,
                self.seed_values.get(name).map(OsString::as_os_str),
                self.limits.max_capture_bytes,
            )?);
        }
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
        let search_path_count = self.search_paths.len() as u64;
        let Some(program) = resolve_executable(adapter.executable, &self.search_paths) else {
            return unavailable_tool(
                adapter,
                search_path_count,
                "not_found",
                "not found in configured search paths",
            );
        };
        let Some(identity_args) = adapter.identity_args else {
            return available_tool(
                adapter,
                &program,
                None,
                Some(executable_digest(&program, self.limits.max_capture_bytes)),
                search_path_count,
                false,
            );
        };
        let total = Duration::from_millis(self.limits.total_timeout_ms);
        let Some(remaining) = total.checked_sub(started.elapsed()) else {
            return error_tool(
                adapter,
                &program,
                search_path_count,
                "discovery_deadline",
                "total discovery deadline elapsed",
            );
        };
        let timeout = remaining.min(Duration::from_millis(self.limits.probe_timeout_ms));
        let request = CommandRequest {
            program: program.clone(),
            args: identity_args.iter().map(OsString::from).collect(),
            cwd: workspace.to_owned(),
            timeout,
            max_capture_bytes: self.limits.max_capture_bytes,
            environment: vec![(OsString::from("LC_ALL"), OsString::from("C"))],
        };
        match run_bounded(&request) {
            Ok(output) if output.timed_out => error_tool(
                adapter,
                &program,
                search_path_count,
                "probe_timeout",
                "identity probe exceeded its deadline",
            ),
            Ok(output) if output.overflowed => error_tool(
                adapter,
                &program,
                search_path_count,
                "probe_output_limit",
                "identity probe exceeded its capture limit",
            ),
            Ok(output) if !output.status.success() => error_tool(
                adapter,
                &program,
                search_path_count,
                "probe_nonzero",
                &format!(
                    "identity probe exited with {}",
                    display_status(output.status)
                ),
            ),
            Ok(output) => match parse_version(&output.stdout) {
                Ok(version) => available_tool(
                    adapter,
                    &program,
                    Some(version),
                    Some(executable_digest(&program, self.limits.max_capture_bytes)),
                    search_path_count,
                    true,
                ),
                Err(detail) => error_tool(
                    adapter,
                    &program,
                    search_path_count,
                    "probe_malformed",
                    detail,
                ),
            },
            Err(error) => error_tool(
                adapter,
                &program,
                search_path_count,
                "probe_failed",
                &error.to_string(),
            ),
        }
    }
}

struct ToolAdapter {
    name: &'static str,
    executable: &'static str,
    capability_id: &'static str,
    groups: &'static [&'static str],
    purpose: &'static str,
    required: bool,
    identity_args: Option<&'static [&'static str]>,
}

const TOOL_ADAPTERS: &[ToolAdapter] = &[
    ToolAdapter {
        name: "agy",
        executable: "agy",
        capability_id: "agent.runtime.agy.identity",
        groups: &["agents"],
        purpose: "Potential agent runtime for bounded collaboration tasks",
        required: false,
        identity_args: None,
    },
    ToolAdapter {
        name: "claude",
        executable: "claude",
        capability_id: "agent.runtime.claude.identity",
        groups: &["agents"],
        purpose: "Potential agent runtime for bounded collaboration tasks",
        required: false,
        identity_args: None,
    },
    ToolAdapter {
        name: "codex",
        executable: "codex",
        capability_id: "agent.runtime.codex.identity",
        groups: &["agents"],
        purpose: "Potential agent runtime for bounded collaboration tasks",
        required: false,
        identity_args: None,
    },
    ToolAdapter {
        name: "copilot",
        executable: "copilot",
        capability_id: "agent.runtime.copilot.identity",
        groups: &["agents"],
        purpose: "Potential agent runtime for bounded collaboration tasks",
        required: false,
        identity_args: None,
    },
    ToolAdapter {
        name: "droid",
        executable: "droid",
        capability_id: "agent.runtime.droid.identity",
        groups: &["agents"],
        purpose: "Potential agent runtime for bounded collaboration tasks",
        required: false,
        identity_args: None,
    },
    ToolAdapter {
        name: "git",
        executable: "git",
        capability_id: "tool.git.identity",
        groups: &["code"],
        purpose: "Inspect repository identity and activation inputs",
        required: false,
        identity_args: Some(&["--version"]),
    },
    ToolAdapter {
        name: "grep",
        executable: "grep",
        capability_id: "tool.grep.identity",
        groups: &["retrieval"],
        purpose: "Extend bounded source mining with portable text search",
        required: false,
        identity_args: None,
    },
    ToolAdapter {
        name: "rg",
        executable: "rg",
        capability_id: "tool.ripgrep.identity",
        groups: &["retrieval"],
        purpose: "Extend bounded source mining with fast text search",
        required: false,
        identity_args: Some(&["--version"]),
    },
    ToolAdapter {
        name: "opencode",
        executable: "opencode",
        capability_id: "agent.runtime.opencode.identity",
        groups: &["agents"],
        purpose: "Potential agent runtime for bounded collaboration tasks",
        required: false,
        identity_args: None,
    },
    ToolAdapter {
        name: "slack-cli",
        executable: "slack-cli",
        capability_id: "comms.application.slack.identity",
        groups: &["communications"],
        purpose: "Potential Slack communications client; discovery grants no relay authority",
        required: false,
        identity_args: None,
    },
    ToolAdapter {
        name: "gh",
        executable: "gh",
        capability_id: "comms.application.github.identity",
        groups: &["code", "communications"],
        purpose: "Potential GitHub communications client; discovery grants no relay authority",
        required: false,
        identity_args: None,
    },
    ToolAdapter {
        name: "telegram-cli",
        executable: "telegram-cli",
        capability_id: "comms.application.telegram.identity",
        groups: &["communications"],
        purpose: "Potential Telegram communications client; discovery grants no relay authority",
        required: false,
        identity_args: None,
    },
    ToolAdapter {
        name: "imsg",
        executable: "imsg",
        capability_id: "comms.application.imessage.identity",
        groups: &["communications"],
        purpose: "Potential iMessage communications client; discovery grants no relay authority",
        required: false,
        identity_args: None,
    },
    ToolAdapter {
        name: "teams",
        executable: "teams",
        capability_id: "comms.application.microsoft-teams.identity",
        groups: &["communications"],
        purpose: "Potential Teams communications client; discovery grants no relay authority",
        required: false,
        identity_args: None,
    },
    ToolAdapter {
        name: "signal-cli",
        executable: "signal-cli",
        capability_id: "comms.application.signal.identity",
        groups: &["communications"],
        purpose: "Potential Signal communications client; discovery grants no relay authority",
        required: false,
        identity_args: None,
    },
];

fn discovery_seed_capability(
    name: &str,
    value: Option<&OsStr>,
    max_capture_bytes: u64,
) -> Result<CapabilityRecord, DiscoveryError> {
    let (availability, captured_value, content_digest, error_code) = match value {
        None => (Availability::Unavailable, None, None, None),
        Some(value) => match value.to_str() {
            None => (
                Availability::Error,
                None,
                None,
                Some("seed_not_utf8".to_owned()),
            ),
            Some(value) if value.len() as u64 > max_capture_bytes => (
                Availability::Error,
                None,
                None,
                Some("seed_capture_limit".to_owned()),
            ),
            Some(value) => {
                let mut hasher = SemanticHasher::new("rey.discovery-seed-value.v1");
                hasher.add_str(name);
                hasher.add_str(value);
                (
                    Availability::Available,
                    Some(value.to_owned()),
                    Some(hasher.finish().to_string()),
                    None,
                )
            }
        },
    };
    let provenance = DiscoverySeedProvenance {
        schema: DISCOVERY_SEED_SCHEMA.to_owned(),
        name: name.to_owned(),
        value: captured_value,
    };
    Ok(CapabilityRecord {
        provider_id: DISCOVERY_SEED_PROVIDER_ID.to_owned(),
        provider_revision: LOCAL_PROVIDER_REVISION,
        provider_kind: "process_discovery".to_owned(),
        capability_id: format!("env.seed.{}", name.to_ascii_lowercase()),
        capability_kind: "environment_seed".to_owned(),
        resolved_location: Some(format!("env://{name}")),
        version: Some(DISCOVERY_SEED_SCHEMA.to_owned()),
        content_digest,
        provenance: Some(serde_json::to_string(&provenance)?),
        availability,
        trust_class: TrustClass::DiscoveredLocal,
        operations: vec!["observe_seed_value".to_owned()],
        enforced_limits: vec![
            "fixed_seed_set".to_owned(),
            format!("max_bytes={max_capture_bytes}"),
            "no_shell_profile_loading".to_owned(),
        ],
        unsupported_limits: Vec::new(),
        observed_at: None,
        error_code,
        error_detail: None,
    })
}

fn application_provenance(adapter: &ToolAdapter, search_path_count: u64) -> String {
    serde_json::json!(DiscoveryApplicationProvenance {
        schema: DISCOVERY_APPLICATION_SCHEMA.to_owned(),
        name: adapter.name.to_owned(),
        groups: adapter
            .groups
            .iter()
            .map(|group| (*group).to_owned())
            .collect(),
        purpose: adapter.purpose.to_owned(),
        required: adapter.required,
        potential_capabilities: vec![adapter.capability_id.to_owned()],
        search_path_count,
    })
    .to_string()
}

fn available_tool(
    adapter: &ToolAdapter,
    program: &Path,
    version: Option<String>,
    content_digest: Option<String>,
    search_path_count: u64,
    identity_probe_executed: bool,
) -> CapabilityRecord {
    let (operations, enforced_limits, mut unsupported_limits) = if identity_probe_executed {
        (
            vec!["inspect_identity".to_owned()],
            vec![
                "capture_bytes".to_owned(),
                "cleared_environment".to_owned(),
                "direct_argv".to_owned(),
                "wall_timeout".to_owned(),
            ],
            vec!["process_sandbox".to_owned()],
        )
    } else {
        (
            vec!["resolve_executable_presence".to_owned()],
            vec![
                "absolute_search_paths".to_owned(),
                "executable_permission".to_owned(),
                "no_process_execution".to_owned(),
            ],
            vec!["version_identity".to_owned(), "task_execution".to_owned()],
        )
    };
    if adapter.capability_id.starts_with("comms.application.") {
        unsupported_limits.extend([
            "message_admission".to_owned(),
            "polling_beacon".to_owned(),
            "relay_authority".to_owned(),
            "transport_adapter".to_owned(),
        ]);
    }
    CapabilityRecord {
        provider_id: format!("rey.tool.{}", adapter.name),
        provider_revision: DISCOVERY_APPLICATION_PROVIDER_REVISION,
        provider_kind: "known_tool".to_owned(),
        capability_id: adapter.capability_id.to_owned(),
        capability_kind: "identity_probe".to_owned(),
        resolved_location: Some(program.display().to_string()),
        version,
        content_digest,
        provenance: Some(application_provenance(adapter, search_path_count)),
        availability: Availability::Available,
        trust_class: TrustClass::DiscoveredLocal,
        operations,
        enforced_limits,
        unsupported_limits,
        observed_at: None,
        error_code: None,
        error_detail: None,
    }
}

fn unavailable_tool(
    adapter: &ToolAdapter,
    search_path_count: u64,
    code: &str,
    detail: &str,
) -> CapabilityRecord {
    failed_tool(
        adapter,
        None,
        search_path_count,
        Availability::Unavailable,
        code,
        detail,
    )
}

fn error_tool(
    adapter: &ToolAdapter,
    program: &Path,
    search_path_count: u64,
    code: &str,
    detail: &str,
) -> CapabilityRecord {
    failed_tool(
        adapter,
        Some(program),
        search_path_count,
        Availability::Error,
        code,
        detail,
    )
}

fn failed_tool(
    adapter: &ToolAdapter,
    program: Option<&Path>,
    search_path_count: u64,
    availability: Availability,
    code: &str,
    detail: &str,
) -> CapabilityRecord {
    let (enforced_limits, mut unsupported_limits) = if adapter.identity_args.is_some() {
        (
            vec![
                "capture_bytes".to_owned(),
                "cleared_environment".to_owned(),
                "direct_argv".to_owned(),
                "wall_timeout".to_owned(),
            ],
            vec!["process_sandbox".to_owned()],
        )
    } else {
        (
            vec![
                "absolute_search_paths".to_owned(),
                "executable_permission".to_owned(),
                "no_process_execution".to_owned(),
            ],
            vec!["version_identity".to_owned(), "task_execution".to_owned()],
        )
    };
    if adapter.capability_id.starts_with("comms.application.") {
        unsupported_limits.extend([
            "message_admission".to_owned(),
            "polling_beacon".to_owned(),
            "relay_authority".to_owned(),
            "transport_adapter".to_owned(),
        ]);
    }
    CapabilityRecord {
        provider_id: format!("rey.tool.{}", adapter.name),
        provider_revision: DISCOVERY_APPLICATION_PROVIDER_REVISION,
        provider_kind: "known_tool".to_owned(),
        capability_id: adapter.capability_id.to_owned(),
        capability_kind: "identity_probe".to_owned(),
        resolved_location: program.map(|path| path.display().to_string()),
        version: None,
        content_digest: None,
        provenance: Some(application_provenance(adapter, search_path_count)),
        availability,
        trust_class: TrustClass::DiscoveredLocal,
        operations: Vec::new(),
        enforced_limits,
        unsupported_limits,
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

fn executable_digest(path: &Path, max_bytes: u64) -> String {
    let mut hasher = SemanticHasher::new("rey.executable-identity.v1");
    hasher.add_str(&path.display().to_string());
    match fs::metadata(path) {
        Ok(metadata) => {
            hasher.add_u64(metadata.len());
            if metadata.len() <= max_bytes {
                match fs::read(path) {
                    Ok(bytes) => hasher.add_bytes(&bytes),
                    Err(error) => hasher.add_str(&format!("read-error:{:?}", error.kind())),
                }
            } else {
                hasher.add_str("content-over-capture-bound");
            }
        }
        Err(error) => hasher.add_str(&format!("metadata-error:{:?}", error.kind())),
    }
    hasher.finish().to_string()
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
    #[error("duplicate capability key ({provider_id}, {provider_revision}, {capability_id})")]
    DuplicateCapabilityKey {
        provider_id: String,
        provider_revision: u64,
        capability_id: String,
    },
    #[error("unsupported capability snapshot schema {actual}; expected {expected}")]
    UnsupportedSnapshotSchema { expected: String, actual: String },
    #[error("capability snapshot is not in canonical row and list order")]
    NonCanonicalSnapshot,
    #[error("capability snapshot digest {declared} does not match recomputed {actual}")]
    SnapshotDigest {
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("capability snapshot completeness {declared} does not match recomputed {actual}")]
    SnapshotCompleteness { declared: bool, actual: bool },
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error("capability JSON encoding failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("capability dataframe failed: {0}")]
    Polars(#[from] polars::error::PolarsError),
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, ffi::OsString, fs, path::PathBuf, time::Duration};

    use tempfile::TempDir;

    use super::{
        Availability, CapabilityRecord, CapabilitySnapshot, CommandRequest,
        DiscoveryApplicationProvenance, DiscoveryError, DiscoveryLimits, LOCAL_PROVIDER_REVISION,
        LocalDiscovery, TOOL_ADAPTERS, TrustClass, resolve_executable, run_bounded,
    };

    #[test]
    fn zero_tool_discovery_contains_only_environment_observations() {
        let workspace = TempDir::new().unwrap();
        let snapshot = LocalDiscovery {
            workspace: workspace.path().to_owned(),
            search_paths: Vec::new(),
            seed_values: BTreeMap::new(),
            limits: DiscoveryLimits::default(),
        }
        .inspect()
        .unwrap();

        assert_eq!(snapshot.profile, "standalone");
        assert_eq!(snapshot.capabilities.len(), 18);
        assert_eq!(
            snapshot
                .capabilities
                .iter()
                .filter(|row| row.availability == Availability::Available)
                .count(),
            0
        );
        assert_eq!(snapshot.to_frame().unwrap().dataframe().height(), 18);
        assert!(snapshot.capabilities.iter().all(|capability| !matches!(
            capability.capability_id.as_str(),
            "frame.arrow-stream" | "source.search.literal-utf8" | "workspace.metadata"
        )));
        let repeated = LocalDiscovery {
            workspace: workspace.path().to_owned(),
            search_paths: Vec::new(),
            seed_values: BTreeMap::new(),
            limits: DiscoveryLimits::default(),
        }
        .inspect()
        .unwrap();
        assert_eq!(snapshot.semantic_digest, repeated.semantic_digest);
    }

    #[test]
    fn process_owned_discovery_records_only_home_pwd_and_path_seeds() {
        let workspace = TempDir::new().unwrap();
        let seed_values = [
            ("HOME".to_owned(), OsString::from("/home/operator")),
            ("PWD".to_owned(), OsString::from("/workspace/project")),
            ("PATH".to_owned(), OsString::from("/bin:/usr/bin")),
            (
                "PRIVATE_TOKEN".to_owned(),
                OsString::from("must-not-be-read"),
            ),
        ]
        .into_iter()
        .collect();
        let snapshot = LocalDiscovery {
            workspace: workspace.path().to_owned(),
            search_paths: Vec::new(),
            seed_values,
            limits: DiscoveryLimits::default(),
        }
        .inspect()
        .unwrap();

        let seeds = snapshot
            .capabilities
            .iter()
            .filter(|row| row.capability_kind == "environment_seed")
            .collect::<Vec<_>>();
        assert_eq!(seeds.len(), 3);
        assert!(
            seeds
                .iter()
                .all(|row| row.provider_id == "rey.discovery-seed")
        );
        assert!(seeds.iter().all(|row| {
            !row.provenance
                .as_deref()
                .unwrap_or_default()
                .contains("PRIVATE_TOKEN")
        }));
        assert_eq!(
            seeds
                .iter()
                .map(|row| row.capability_id.as_str())
                .collect::<Vec<_>>(),
            ["env.seed.home", "env.seed.path", "env.seed.pwd"]
        );
    }

    #[test]
    fn process_owned_discovery_declares_major_agent_runtime_options() {
        let workspace = TempDir::new().unwrap();
        let snapshot = LocalDiscovery {
            workspace: workspace.path().to_owned(),
            search_paths: Vec::new(),
            seed_values: BTreeMap::new(),
            limits: DiscoveryLimits::default(),
        }
        .inspect()
        .unwrap();

        let agent_runtimes = snapshot
            .capabilities
            .iter()
            .filter(|row| row.capability_id.starts_with("agent.runtime."))
            .map(|row| row.capability_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            agent_runtimes,
            [
                "agent.runtime.agy.identity",
                "agent.runtime.claude.identity",
                "agent.runtime.codex.identity",
                "agent.runtime.copilot.identity",
                "agent.runtime.droid.identity",
                "agent.runtime.opencode.identity",
            ]
        );
        assert!(
            snapshot
                .capabilities
                .iter()
                .filter(|row| {
                    row.capability_id.starts_with("agent.runtime.")
                        && row.availability == Availability::Unavailable
                        && row.error_code.as_deref() == Some("not_found")
                })
                .count()
                == 6
        );
    }

    #[test]
    fn process_owned_discovery_declares_communications_application_candidates() {
        let workspace = TempDir::new().unwrap();
        let snapshot = LocalDiscovery {
            workspace: workspace.path().to_owned(),
            search_paths: Vec::new(),
            seed_values: BTreeMap::new(),
            limits: DiscoveryLimits::default(),
        }
        .inspect()
        .unwrap();

        let applications = snapshot
            .capabilities
            .iter()
            .filter(|row| row.capability_id.starts_with("comms.application."))
            .map(|row| row.capability_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            applications,
            [
                "comms.application.github.identity",
                "comms.application.imessage.identity",
                "comms.application.signal.identity",
                "comms.application.slack.identity",
                "comms.application.microsoft-teams.identity",
                "comms.application.telegram.identity",
            ]
        );
        for application in snapshot
            .capabilities
            .iter()
            .filter(|row| row.capability_id.starts_with("comms.application."))
        {
            assert_eq!(application.availability, Availability::Unavailable);
            assert!(application.operations.is_empty());
            assert!(
                application
                    .unsupported_limits
                    .contains(&"relay_authority".to_owned())
            );
            let provenance: DiscoveryApplicationProvenance =
                serde_json::from_str(application.provenance.as_deref().unwrap()).unwrap();
            assert!(provenance.groups.contains(&"communications".to_owned()));
            let expected_name = match application.capability_id.as_str() {
                "comms.application.github.identity" => "gh",
                "comms.application.imessage.identity" => "imsg",
                "comms.application.microsoft-teams.identity" => "teams",
                "comms.application.signal.identity" => "signal-cli",
                "comms.application.slack.identity" => "slack-cli",
                "comms.application.telegram.identity" => "telegram-cli",
                capability_id => panic!("unexpected communications application {capability_id}"),
            };
            assert_eq!(provenance.name, expected_name);
        }
    }

    #[test]
    fn process_owned_application_groups_are_many_to_many_without_duplicate_search_rows() {
        let workspace = TempDir::new().unwrap();
        let snapshot = LocalDiscovery {
            workspace: workspace.path().to_owned(),
            search_paths: Vec::new(),
            seed_values: BTreeMap::new(),
            limits: DiscoveryLimits::default(),
        }
        .inspect()
        .unwrap();

        let applications = snapshot
            .capabilities
            .iter()
            .filter(|row| row.capability_kind == "identity_probe")
            .collect::<Vec<_>>();
        assert_eq!(applications.len(), 15);
        assert!(
            TOOL_ADAPTERS
                .iter()
                .all(|adapter| adapter.name == adapter.executable)
        );

        let groups = |capability_id: &str| {
            let application = applications
                .iter()
                .find(|row| row.capability_id == capability_id)
                .unwrap();
            serde_json::from_str::<DiscoveryApplicationProvenance>(
                application.provenance.as_deref().unwrap(),
            )
            .unwrap()
            .groups
        };
        assert_eq!(groups("tool.grep.identity"), ["retrieval"]);
        assert_eq!(groups("tool.ripgrep.identity"), ["retrieval"]);
        assert_eq!(
            groups("comms.application.github.identity"),
            ["code", "communications"]
        );
        assert_eq!(groups("agent.runtime.codex.identity"), ["agents"]);
    }

    #[cfg(unix)]
    #[test]
    fn agent_runtime_presence_discovery_does_not_start_the_application() {
        use std::os::unix::fs::PermissionsExt;

        let workspace = TempDir::new().unwrap();
        let bin = TempDir::new().unwrap();
        let marker = bin.path().join("application-started");
        let application = bin.path().join("claude");
        fs::write(
            &application,
            format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        )
        .unwrap();
        fs::set_permissions(&application, fs::Permissions::from_mode(0o755)).unwrap();

        let snapshot = LocalDiscovery {
            workspace: workspace.path().to_owned(),
            search_paths: vec![bin.path().to_owned()],
            seed_values: BTreeMap::new(),
            limits: DiscoveryLimits::default(),
        }
        .inspect()
        .unwrap();
        let claude = snapshot
            .capabilities
            .iter()
            .find(|row| row.capability_id == "agent.runtime.claude.identity")
            .unwrap();

        assert_eq!(claude.availability, Availability::Available);
        assert!(claude.version.is_none());
        assert_eq!(claude.operations, ["resolve_executable_presence"]);
        assert!(!marker.exists());
    }

    #[test]
    fn snapshot_json_is_recomputed_before_admission() {
        let snapshot = CapabilitySnapshot::new(
            "fixture",
            DiscoveryLimits::default(),
            vec![fixture_capability("one"), fixture_capability("two")],
        )
        .unwrap();
        let canonical = serde_json::to_vec(&snapshot).unwrap();
        assert_eq!(
            CapabilitySnapshot::from_json_slice(&canonical, 64).unwrap(),
            snapshot
        );

        let mut tampered: serde_json::Value = serde_json::from_slice(&canonical).unwrap();
        tampered["capabilities"][0]["version"] = "changed".into();
        let error =
            CapabilitySnapshot::from_json_slice(&serde_json::to_vec(&tampered).unwrap(), 64)
                .unwrap_err();
        assert!(matches!(error, DiscoveryError::SnapshotDigest { .. }));

        let mut noncanonical = snapshot.clone();
        noncanonical.capabilities.reverse();
        let error =
            CapabilitySnapshot::from_json_slice(&serde_json::to_vec(&noncanonical).unwrap(), 64)
                .unwrap_err();
        assert!(matches!(error, DiscoveryError::NonCanonicalSnapshot));
    }

    #[test]
    fn duplicate_capability_keys_are_rejected() {
        let row = fixture_capability("duplicate");
        let error = CapabilitySnapshot::new(
            "fixture",
            DiscoveryLimits::default(),
            vec![row.clone(), row],
        )
        .unwrap_err();

        assert!(matches!(
            error,
            DiscoveryError::DuplicateCapabilityKey { .. }
        ));
    }

    fn fixture_capability(id: &str) -> CapabilityRecord {
        CapabilityRecord {
            provider_id: "fixture".to_owned(),
            provider_revision: LOCAL_PROVIDER_REVISION,
            provider_kind: "fixture".to_owned(),
            capability_id: id.to_owned(),
            capability_kind: "identity".to_owned(),
            resolved_location: None,
            version: Some("1".to_owned()),
            content_digest: None,
            provenance: Some("fixture".to_owned()),
            availability: Availability::Available,
            trust_class: TrustClass::BuiltIn,
            operations: Vec::new(),
            enforced_limits: Vec::new(),
            unsupported_limits: Vec::new(),
            observed_at: None,
            error_code: None,
            error_detail: None,
        }
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
