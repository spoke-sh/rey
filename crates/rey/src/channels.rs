#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use rey_core::{SemanticDigest, SemanticHasher};
use rey_diff::DeltaAssessment;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CHANNEL_GRAPH_SCHEMA: &str = "rey.channel-graph.v1";
pub const CHANNEL_GRAPH_SNAPSHOT_SCHEMA: &str = "rey.channel-graph-snapshot.v1";
pub const CHANNEL_WORKING_SCHEMA: &str = "rey.channel-working.v1";
pub const CHANNEL_STATUS_SCHEMA: &str = "rey.channel-status.v1";
pub const CHANNEL_DIFF_SCHEMA: &str = "rey.channel-diff.v1";
pub const CHANNEL_GRAPH_DELTA_SCHEMA: &str = "rey.channel-graph-delta.v1";
pub const CHANNEL_APPLY_RESULT_SCHEMA: &str = "rey.channel-apply-result.v1";
pub const MAX_CHANNEL_GRAPH_INPUT_BYTES: u64 = 1_024 * 1_024;
pub const MAX_CHANNEL_STATE_BYTES: u64 = 4 * 1_024 * 1_024;

const MAX_CHANNELS: usize = 32;
const MAX_SUBSCRIPTIONS: usize = 32;
const MAX_STREAMS: usize = 8;
const MAX_RELAYS: usize = 32;
const MAX_NAME_CHARS: usize = 80;
const MAX_IDENTIFIER_CHARS: usize = 80;
const MAX_LOCATOR_BYTES: usize = 4_096;
const MAX_SUBSCRIPTION_LIMIT: u64 = 256;
const MAX_RELAY_HOPS: u64 = 16;
const WORKING_FILE_NAME: &str = "working.json";
const LOCK_FILE_NAME: &str = "channels.lock";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelObservationKind {
    Finding,
    Question,
    Progress,
    Blocker,
    Handoff,
}

impl ChannelObservationKind {
    fn all() -> Vec<Self> {
        vec![
            Self::Finding,
            Self::Question,
            Self::Progress,
            Self::Blocker,
            Self::Handoff,
        ]
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Finding => "finding",
            Self::Question => "question",
            Self::Progress => "progress",
            Self::Blocker => "blocker",
            Self::Handoff => "handoff",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelScope {
    WorkspaceLocal,
}

impl ChannelScope {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::WorkspaceLocal => "workspace local",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelDefinition {
    pub id: String,
    pub revision: u64,
    pub name: String,
    pub scope: ChannelScope,
    pub accepted_observation_kinds: Vec<ChannelObservationKind>,
    pub broadcast_default: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelSubscription {
    pub id: String,
    pub revision: u64,
    pub channel_ids: Vec<String>,
    pub observation_kinds: Vec<ChannelObservationKind>,
    #[serde(default)]
    pub filters: BTreeMap<String, String>,
    pub limit: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FeedStreamDefinition {
    pub id: String,
    pub revision: u64,
    pub name: String,
    pub subscription_id: String,
    pub lens: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FeedLayout {
    pub id: String,
    pub revision: u64,
    pub stream_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelRelayDeclaration {
    pub id: String,
    pub revision: u64,
    pub source_channel_id: String,
    pub target_channel_locator: String,
    pub provider_id: String,
    pub hop_limit: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelGraph {
    pub schema: String,
    pub channels: Vec<ChannelDefinition>,
    pub subscriptions: Vec<ChannelSubscription>,
    pub streams: Vec<FeedStreamDefinition>,
    pub layout: FeedLayout,
    #[serde(default)]
    pub relays: Vec<ChannelRelayDeclaration>,
}

impl ChannelGraph {
    pub fn canonicalize(mut self) -> Result<Self, ChannelGraphError> {
        validate_graph_members(&self)?;
        self.channels.sort_by(|left, right| left.id.cmp(&right.id));
        self.subscriptions
            .sort_by(|left, right| left.id.cmp(&right.id));
        self.streams.sort_by(|left, right| left.id.cmp(&right.id));
        self.relays.sort_by(|left, right| left.id.cmp(&right.id));
        for channel in &mut self.channels {
            channel.accepted_observation_kinds.sort();
        }
        for subscription in &mut self.subscriptions {
            subscription.channel_ids.sort();
            subscription.observation_kinds.sort();
        }
        validate_graph_references(&self)?;
        Ok(self)
    }

    pub fn verify(&self) -> Result<(), ChannelGraphError> {
        let canonical = self.clone().canonicalize()?;
        if canonical != *self {
            return Err(ChannelGraphError::NonCanonicalGraph);
        }
        Ok(())
    }

    pub fn identity(&self) -> Result<SemanticDigest, ChannelGraphError> {
        self.verify()?;
        let mut hasher = SemanticHasher::new(CHANNEL_GRAPH_SCHEMA);
        hasher.add_bytes(&serde_json::to_vec(self)?);
        Ok(hasher.finish())
    }

    pub fn built_in() -> Result<Self, ChannelGraphError> {
        let observation_kinds = ChannelObservationKind::all();
        Self {
            schema: CHANNEL_GRAPH_SCHEMA.to_owned(),
            channels: vec![ChannelDefinition {
                id: "workspace".to_owned(),
                revision: 1,
                name: "Workspace".to_owned(),
                scope: ChannelScope::WorkspaceLocal,
                accepted_observation_kinds: observation_kinds.clone(),
                broadcast_default: true,
            }],
            subscriptions: vec![ChannelSubscription {
                id: "workspace".to_owned(),
                revision: 1,
                channel_ids: vec!["workspace".to_owned()],
                observation_kinds,
                filters: BTreeMap::new(),
                limit: 64,
            }],
            streams: vec![
                FeedStreamDefinition {
                    id: "signals".to_owned(),
                    revision: 1,
                    name: "Signals".to_owned(),
                    subscription_id: "workspace".to_owned(),
                    lens: "signals".to_owned(),
                },
                FeedStreamDefinition {
                    id: "admission".to_owned(),
                    revision: 1,
                    name: "Admission".to_owned(),
                    subscription_id: "workspace".to_owned(),
                    lens: "admission".to_owned(),
                },
                FeedStreamDefinition {
                    id: "flow".to_owned(),
                    revision: 1,
                    name: "Flow".to_owned(),
                    subscription_id: "workspace".to_owned(),
                    lens: "flow".to_owned(),
                },
            ],
            layout: FeedLayout {
                id: "feed".to_owned(),
                revision: 1,
                stream_ids: vec![
                    "signals".to_owned(),
                    "admission".to_owned(),
                    "flow".to_owned(),
                ],
            },
            relays: Vec::new(),
        }
        .canonicalize()
    }

    #[must_use]
    pub fn stream(&self, stream_id: &str) -> Option<&FeedStreamDefinition> {
        self.streams.iter().find(|stream| stream.id == stream_id)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelGraphLimits {
    pub max_channels: u64,
    pub max_subscriptions: u64,
    pub max_streams: u64,
    pub max_relays: u64,
    pub max_subscription_records: u64,
    pub max_relay_hops: u64,
}

impl Default for ChannelGraphLimits {
    fn default() -> Self {
        Self {
            max_channels: MAX_CHANNELS as u64,
            max_subscriptions: MAX_SUBSCRIPTIONS as u64,
            max_streams: MAX_STREAMS as u64,
            max_relays: MAX_RELAYS as u64,
            max_subscription_records: MAX_SUBSCRIPTION_LIMIT,
            max_relay_hops: MAX_RELAY_HOPS,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelGraphSourceKind {
    BuiltIn,
    Worktree,
}

impl ChannelGraphSourceKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::BuiltIn => "BUILT-IN",
            Self::Worktree => "WORKING",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelGraphSource {
    pub kind: ChannelGraphSourceKind,
    pub locator: String,
    pub content_digest: SemanticDigest,
}

impl ChannelGraphSource {
    fn built_in(graph_id: &SemanticDigest) -> Self {
        Self {
            kind: ChannelGraphSourceKind::BuiltIn,
            locator: "builtin://rey/channel-graph/default".to_owned(),
            content_digest: graph_id.clone(),
        }
    }

    #[must_use]
    pub fn worktree(locator: String, bytes: &[u8]) -> Self {
        let mut hasher = SemanticHasher::new("rey.channel-graph-source.v1");
        hasher.add_bytes(bytes);
        Self {
            kind: ChannelGraphSourceKind::Worktree,
            locator,
            content_digest: hasher.finish(),
        }
    }

    fn verify(&self) -> Result<(), ChannelGraphError> {
        validate_locator("channel graph source", &self.locator)?;
        validate_semantic_digest("channel graph source content", &self.content_digest)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelGraphSnapshot {
    pub schema: String,
    pub snapshot_id: SemanticDigest,
    pub graph_id: SemanticDigest,
    pub source: ChannelGraphSource,
    pub limits: ChannelGraphLimits,
    pub graph: ChannelGraph,
}

impl ChannelGraphSnapshot {
    pub fn new(graph: ChannelGraph, source: ChannelGraphSource) -> Result<Self, ChannelGraphError> {
        let graph = graph.canonicalize()?;
        let graph_id = graph.identity()?;
        source.verify()?;
        let limits = ChannelGraphLimits::default();
        let snapshot_id = channel_snapshot_identity(&graph_id, &source, &limits)?;
        let snapshot = Self {
            schema: CHANNEL_GRAPH_SNAPSHOT_SCHEMA.to_owned(),
            snapshot_id,
            graph_id,
            source,
            limits,
            graph,
        };
        snapshot.verify()?;
        Ok(snapshot)
    }

    pub fn built_in() -> Result<Self, ChannelGraphError> {
        let graph = ChannelGraph::built_in()?;
        let graph_id = graph.identity()?;
        Self::new(graph, ChannelGraphSource::built_in(&graph_id))
    }

    pub fn verify(&self) -> Result<(), ChannelGraphError> {
        if self.schema != CHANNEL_GRAPH_SNAPSHOT_SCHEMA {
            return Err(ChannelGraphError::Schema {
                expected: CHANNEL_GRAPH_SNAPSHOT_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        self.source.verify()?;
        if self.limits != ChannelGraphLimits::default() {
            return Err(ChannelGraphError::LimitEnvelope);
        }
        let actual = self.graph.identity()?;
        if actual != self.graph_id {
            return Err(ChannelGraphError::GraphIdentity {
                declared: self.graph_id.clone(),
                actual,
            });
        }
        let actual = channel_snapshot_identity(&self.graph_id, &self.source, &self.limits)?;
        if actual != self.snapshot_id {
            return Err(ChannelGraphError::SnapshotIdentity {
                declared: self.snapshot_id.clone(),
                actual,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ChannelWorkingDocument {
    schema: String,
    base_graph_id: SemanticDigest,
    snapshot: ChannelGraphSnapshot,
}

impl ChannelWorkingDocument {
    fn verify(&self, expected_base: &SemanticDigest) -> Result<(), ChannelGraphError> {
        if self.schema != CHANNEL_WORKING_SCHEMA {
            return Err(ChannelGraphError::Schema {
                expected: CHANNEL_WORKING_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        if &self.base_graph_id != expected_base {
            return Err(ChannelGraphError::StaleWorking {
                expected: expected_base.clone(),
                actual: self.base_graph_id.clone(),
            });
        }
        self.snapshot.verify()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelWorkingState {
    Clean,
    Working,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelObjectKind {
    Channel,
    Subscription,
    Stream,
    Layout,
    Relay,
}

impl ChannelObjectKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Channel => "channel",
            Self::Subscription => "subscription",
            Self::Stream => "stream",
            Self::Layout => "layout",
            Self::Relay => "relay",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelChangeKind {
    Added,
    Removed,
    Modified,
    Renamed,
    Retargeted,
    Moved,
}

impl ChannelChangeKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Modified => "modified",
            Self::Renamed => "renamed",
            Self::Retargeted => "retargeted",
            Self::Moved => "moved",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelGraphChange {
    pub kind: ChannelChangeKind,
    pub object_kind: ChannelObjectKind,
    pub object_id: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelGraphDeltaSummary {
    pub added: u64,
    pub removed: u64,
    pub modified: u64,
    pub renamed: u64,
    pub retargeted: u64,
    pub moved: u64,
    pub total: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelGraphDelta {
    pub schema: String,
    pub delta_id: SemanticDigest,
    pub source_label: String,
    pub target_label: String,
    pub source_graph_id: SemanticDigest,
    pub target_graph_id: SemanticDigest,
    pub assessment: DeltaAssessment,
    pub summary: ChannelGraphDeltaSummary,
    pub changes: Vec<ChannelGraphChange>,
}

impl ChannelGraphDelta {
    pub fn derive(
        source_label: impl Into<String>,
        source: &ChannelGraphSnapshot,
        target_label: impl Into<String>,
        target: &ChannelGraphSnapshot,
    ) -> Result<Self, ChannelGraphError> {
        source.verify()?;
        target.verify()?;
        let source_label = source_label.into();
        let target_label = target_label.into();
        let changes = graph_changes(&source.graph, &target.graph);
        let summary = summarize_changes(&changes);
        let assessment = if changes.is_empty() {
            DeltaAssessment::Equal
        } else {
            DeltaAssessment::Different
        };
        let mut hasher = SemanticHasher::new(CHANNEL_GRAPH_DELTA_SCHEMA);
        hasher.add_str(&source_label);
        hasher.add_str(source.graph_id.as_str());
        hasher.add_str(&target_label);
        hasher.add_str(target.graph_id.as_str());
        hasher.add_bytes(&serde_json::to_vec(&changes)?);
        Ok(Self {
            schema: CHANNEL_GRAPH_DELTA_SCHEMA.to_owned(),
            delta_id: hasher.finish(),
            source_label,
            target_label,
            source_graph_id: source.graph_id.clone(),
            target_graph_id: target.graph_id.clone(),
            assessment,
            summary,
            changes,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelStatus {
    pub schema: String,
    pub state: ChannelWorkingState,
    pub working_present: bool,
    pub head: ChannelGraphSnapshot,
    pub index: Option<ChannelGraphSnapshot>,
    pub working: ChannelGraphSnapshot,
    pub delta: ChannelGraphDelta,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelDiff {
    pub schema: String,
    pub source: ChannelGraphSnapshot,
    pub target: ChannelGraphSnapshot,
    pub delta: ChannelGraphDelta,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelApplyResult {
    pub schema: String,
    pub applied: bool,
    pub snapshot: ChannelGraphSnapshot,
    pub delta: ChannelGraphDelta,
}

#[derive(Clone, Debug)]
pub struct LocalChannelStore {
    directory: PathBuf,
}

impl LocalChannelStore {
    #[must_use]
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    #[must_use]
    pub fn default_for_workspace(workspace: &Path) -> Self {
        Self::new(workspace.join(".rey").join("channels"))
    }

    #[must_use]
    pub fn working_path(&self) -> PathBuf {
        self.directory.join(WORKING_FILE_NAME)
    }

    pub fn status(&self) -> Result<ChannelStatus, ChannelGraphError> {
        let head = ChannelGraphSnapshot::built_in()?;
        let working = self.load_working(&head)?.unwrap_or_else(|| head.clone());
        let working_present = self.working_path().exists();
        let delta = ChannelGraphDelta::derive("BUILT-IN", &head, "WORKING", &working)?;
        let state = if delta.assessment == DeltaAssessment::Equal {
            ChannelWorkingState::Clean
        } else {
            ChannelWorkingState::Working
        };
        Ok(ChannelStatus {
            schema: CHANNEL_STATUS_SCHEMA.to_owned(),
            state,
            working_present,
            head,
            index: None,
            working,
            delta,
        })
    }

    pub fn diff(&self) -> Result<ChannelDiff, ChannelGraphError> {
        let status = self.status()?;
        Ok(ChannelDiff {
            schema: CHANNEL_DIFF_SCHEMA.to_owned(),
            source: status.head,
            target: status.working,
            delta: status.delta,
        })
    }

    pub fn apply(
        &self,
        graph: ChannelGraph,
        source: ChannelGraphSource,
    ) -> Result<ChannelApplyResult, ChannelGraphError> {
        let target = ChannelGraphSnapshot::new(graph, source)?;
        self.prepare_directory()?;
        let lock_path = self.directory.join(LOCK_FILE_NAME);
        if let Ok(metadata) = fs::symlink_metadata(&lock_path)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(ChannelGraphError::UnsafePath(lock_path));
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| ChannelGraphError::Write {
                path: lock_path.clone(),
                source,
            })?;
        File::lock(&lock).map_err(|source| ChannelGraphError::Lock {
            path: lock_path.clone(),
            source,
        })?;
        let result = (|| {
            let status = self.status()?;
            validate_revision_progress(&status.working.graph, &target.graph)?;
            let delta = ChannelGraphDelta::derive("WORKING", &status.working, "PROPOSAL", &target)?;
            let applied = delta.assessment == DeltaAssessment::Different;
            if applied {
                self.save_working(&ChannelWorkingDocument {
                    schema: CHANNEL_WORKING_SCHEMA.to_owned(),
                    base_graph_id: status.head.graph_id,
                    snapshot: target.clone(),
                })?;
            }
            Ok(ChannelApplyResult {
                schema: CHANNEL_APPLY_RESULT_SCHEMA.to_owned(),
                applied,
                snapshot: target,
                delta,
            })
        })();
        let unlock = File::unlock(&lock).map_err(|source| ChannelGraphError::Lock {
            path: lock_path,
            source,
        });
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    fn load_working(
        &self,
        head: &ChannelGraphSnapshot,
    ) -> Result<Option<ChannelGraphSnapshot>, ChannelGraphError> {
        self.verify_directory_boundary()?;
        let path = self.working_path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(ChannelGraphError::Read { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(ChannelGraphError::UnsafePath(path));
        }
        if metadata.len() > MAX_CHANNEL_STATE_BYTES {
            return Err(ChannelGraphError::StateByteLimit(MAX_CHANNEL_STATE_BYTES));
        }
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|source| ChannelGraphError::Read {
                path: path.clone(),
                source,
            })?
            .take(MAX_CHANNEL_STATE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| ChannelGraphError::Read {
                path: path.clone(),
                source,
            })?;
        if bytes.len() as u64 > MAX_CHANNEL_STATE_BYTES {
            return Err(ChannelGraphError::StateByteLimit(MAX_CHANNEL_STATE_BYTES));
        }
        let document: ChannelWorkingDocument = serde_json::from_slice(&bytes)?;
        document.verify(&head.graph_id)?;
        Ok(Some(document.snapshot))
    }

    fn save_working(&self, document: &ChannelWorkingDocument) -> Result<(), ChannelGraphError> {
        let head = ChannelGraphSnapshot::built_in()?;
        document.verify(&head.graph_id)?;
        let bytes = serde_json::to_vec_pretty(document)?;
        if bytes.len().saturating_add(1) as u64 > MAX_CHANNEL_STATE_BYTES {
            return Err(ChannelGraphError::StateByteLimit(MAX_CHANNEL_STATE_BYTES));
        }
        let target = self.working_path();
        if let Ok(metadata) = fs::symlink_metadata(&target)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(ChannelGraphError::UnsafePath(target));
        }
        let (temporary, mut file) = self.create_temporary()?;
        let publication = (|| {
            file.write_all(&bytes)
                .and_then(|()| file.write_all(b"\n"))
                .and_then(|()| file.flush())?;
            drop(file);
            fs::rename(&temporary, &target)
        })();
        if let Err(source) = publication {
            let _ = fs::remove_file(&temporary);
            return Err(ChannelGraphError::Write {
                path: target,
                source,
            });
        }
        Ok(())
    }

    fn prepare_directory(&self) -> Result<(), ChannelGraphError> {
        self.verify_directory_boundary()?;
        match fs::symlink_metadata(&self.directory) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                Err(ChannelGraphError::UnsafePath(self.directory.clone()))
            }
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.directory).map_err(|source| ChannelGraphError::Write {
                    path: self.directory.clone(),
                    source,
                })
            }
            Err(source) => Err(ChannelGraphError::Write {
                path: self.directory.clone(),
                source,
            }),
        }
    }

    fn verify_directory_boundary(&self) -> Result<(), ChannelGraphError> {
        for ancestor in self.directory.ancestors() {
            match fs::symlink_metadata(ancestor) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(ChannelGraphError::UnsafePath(ancestor.to_owned()));
                }
                Ok(metadata) if ancestor == self.directory && !metadata.is_dir() => {
                    return Err(ChannelGraphError::UnsafePath(ancestor.to_owned()));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(ChannelGraphError::Read {
                        path: ancestor.to_owned(),
                        source,
                    });
                }
            }
        }
        Ok(())
    }

    fn create_temporary(&self) -> Result<(PathBuf, File), ChannelGraphError> {
        for attempt in 0..32_u8 {
            let path = self.directory.join(format!(
                ".{WORKING_FILE_NAME}.tmp-{}-{attempt}",
                std::process::id()
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(ChannelGraphError::Write { path, source }),
            }
        }
        Err(ChannelGraphError::TemporaryLimit(self.directory.clone()))
    }
}

fn validate_graph_members(graph: &ChannelGraph) -> Result<(), ChannelGraphError> {
    if graph.schema != CHANNEL_GRAPH_SCHEMA {
        return Err(ChannelGraphError::Schema {
            expected: CHANNEL_GRAPH_SCHEMA,
            actual: graph.schema.clone(),
        });
    }
    validate_count("channels", graph.channels.len(), 1, MAX_CHANNELS)?;
    validate_count(
        "subscriptions",
        graph.subscriptions.len(),
        1,
        MAX_SUBSCRIPTIONS,
    )?;
    validate_count("streams", graph.streams.len(), 1, MAX_STREAMS)?;
    validate_count("relays", graph.relays.len(), 0, MAX_RELAYS)?;

    let mut channel_ids = BTreeSet::new();
    for channel in &graph.channels {
        validate_identifier("channel id", &channel.id)?;
        validate_revision("channel", &channel.id, channel.revision)?;
        validate_name("channel name", &channel.name)?;
        if !channel_ids.insert(channel.id.as_str()) {
            return Err(ChannelGraphError::Duplicate {
                kind: "channel",
                id: channel.id.clone(),
            });
        }
        validate_unique_kinds(
            "channel accepted observation kinds",
            &channel.id,
            &channel.accepted_observation_kinds,
        )?;
    }

    let mut subscription_ids = BTreeSet::new();
    for subscription in &graph.subscriptions {
        validate_identifier("subscription id", &subscription.id)?;
        validate_revision("subscription", &subscription.id, subscription.revision)?;
        if !subscription_ids.insert(subscription.id.as_str()) {
            return Err(ChannelGraphError::Duplicate {
                kind: "subscription",
                id: subscription.id.clone(),
            });
        }
        if subscription.channel_ids.is_empty() {
            return Err(ChannelGraphError::EmptyReferenceSet {
                kind: "subscription channels",
                id: subscription.id.clone(),
            });
        }
        validate_unique_strings(
            "subscription channel",
            &subscription.id,
            &subscription.channel_ids,
        )?;
        validate_unique_kinds(
            "subscription observation kinds",
            &subscription.id,
            &subscription.observation_kinds,
        )?;
        if subscription.limit == 0 || subscription.limit > MAX_SUBSCRIPTION_LIMIT {
            return Err(ChannelGraphError::SubscriptionLimit {
                id: subscription.id.clone(),
                actual: subscription.limit,
                limit: MAX_SUBSCRIPTION_LIMIT,
            });
        }
        for (name, value) in &subscription.filters {
            validate_identifier("subscription filter", name)?;
            validate_name("subscription filter value", value)?;
        }
    }

    let mut stream_ids = BTreeSet::new();
    for stream in &graph.streams {
        validate_identifier("stream id", &stream.id)?;
        validate_revision("stream", &stream.id, stream.revision)?;
        validate_name("stream name", &stream.name)?;
        validate_identifier("stream subscription id", &stream.subscription_id)?;
        validate_identifier("stream lens", &stream.lens)?;
        if !stream_ids.insert(stream.id.as_str()) {
            return Err(ChannelGraphError::Duplicate {
                kind: "stream",
                id: stream.id.clone(),
            });
        }
    }

    validate_identifier("layout id", &graph.layout.id)?;
    validate_revision("layout", &graph.layout.id, graph.layout.revision)?;
    if graph.layout.stream_ids.len() != graph.streams.len() {
        return Err(ChannelGraphError::LayoutCoverage {
            streams: graph.streams.len(),
            positions: graph.layout.stream_ids.len(),
        });
    }
    validate_unique_strings("layout stream", &graph.layout.id, &graph.layout.stream_ids)?;

    let mut relay_ids = BTreeSet::new();
    for relay in &graph.relays {
        validate_identifier("relay id", &relay.id)?;
        validate_revision("relay", &relay.id, relay.revision)?;
        validate_identifier("relay source channel", &relay.source_channel_id)?;
        validate_locator("relay target channel", &relay.target_channel_locator)?;
        validate_identifier("relay provider", &relay.provider_id)?;
        if relay.hop_limit == 0 || relay.hop_limit > MAX_RELAY_HOPS {
            return Err(ChannelGraphError::RelayHopLimit {
                id: relay.id.clone(),
                actual: relay.hop_limit,
                limit: MAX_RELAY_HOPS,
            });
        }
        if !relay_ids.insert(relay.id.as_str()) {
            return Err(ChannelGraphError::Duplicate {
                kind: "relay",
                id: relay.id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_graph_references(graph: &ChannelGraph) -> Result<(), ChannelGraphError> {
    let channel_ids = graph
        .channels
        .iter()
        .map(|channel| channel.id.as_str())
        .collect::<BTreeSet<_>>();
    let subscription_ids = graph
        .subscriptions
        .iter()
        .map(|subscription| subscription.id.as_str())
        .collect::<BTreeSet<_>>();
    let stream_ids = graph
        .streams
        .iter()
        .map(|stream| stream.id.as_str())
        .collect::<BTreeSet<_>>();

    for subscription in &graph.subscriptions {
        for channel_id in &subscription.channel_ids {
            if !channel_ids.contains(channel_id.as_str()) {
                return Err(ChannelGraphError::MissingReference {
                    owner_kind: "subscription",
                    owner_id: subscription.id.clone(),
                    target_kind: "channel",
                    target_id: channel_id.clone(),
                });
            }
        }
    }
    for stream in &graph.streams {
        if !subscription_ids.contains(stream.subscription_id.as_str()) {
            return Err(ChannelGraphError::MissingReference {
                owner_kind: "stream",
                owner_id: stream.id.clone(),
                target_kind: "subscription",
                target_id: stream.subscription_id.clone(),
            });
        }
    }
    for stream_id in &graph.layout.stream_ids {
        if !stream_ids.contains(stream_id.as_str()) {
            return Err(ChannelGraphError::MissingReference {
                owner_kind: "layout",
                owner_id: graph.layout.id.clone(),
                target_kind: "stream",
                target_id: stream_id.clone(),
            });
        }
    }
    for relay in &graph.relays {
        if !channel_ids.contains(relay.source_channel_id.as_str()) {
            return Err(ChannelGraphError::MissingReference {
                owner_kind: "relay",
                owner_id: relay.id.clone(),
                target_kind: "channel",
                target_id: relay.source_channel_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_revision_progress(
    source: &ChannelGraph,
    target: &ChannelGraph,
) -> Result<(), ChannelGraphError> {
    validate_object_revision_progress(
        "channel",
        &source.channels,
        &target.channels,
        |value| &value.id,
        |value| value.revision,
    )?;
    validate_object_revision_progress(
        "subscription",
        &source.subscriptions,
        &target.subscriptions,
        |value| &value.id,
        |value| value.revision,
    )?;
    validate_object_revision_progress(
        "stream",
        &source.streams,
        &target.streams,
        |value| &value.id,
        |value| value.revision,
    )?;
    validate_object_revision_progress(
        "relay",
        &source.relays,
        &target.relays,
        |value| &value.id,
        |value| value.revision,
    )?;
    if source.layout.id == target.layout.id
        && source.layout != target.layout
        && target.layout.revision <= source.layout.revision
    {
        return Err(ChannelGraphError::RevisionNotAdvanced {
            kind: "layout",
            id: target.layout.id.clone(),
            previous: source.layout.revision,
            proposed: target.layout.revision,
        });
    }
    Ok(())
}

fn validate_object_revision_progress<T: Eq>(
    kind: &'static str,
    source: &[T],
    target: &[T],
    id: impl Fn(&T) -> &String,
    revision: impl Fn(&T) -> u64,
) -> Result<(), ChannelGraphError> {
    for proposed in target {
        if let Some(previous) = source
            .iter()
            .find(|candidate| id(candidate) == id(proposed))
            && previous != proposed
            && revision(proposed) <= revision(previous)
        {
            return Err(ChannelGraphError::RevisionNotAdvanced {
                kind,
                id: id(proposed).clone(),
                previous: revision(previous),
                proposed: revision(proposed),
            });
        }
    }
    Ok(())
}

fn graph_changes(source: &ChannelGraph, target: &ChannelGraph) -> Vec<ChannelGraphChange> {
    let mut changes = Vec::new();
    diff_channels(&mut changes, &source.channels, &target.channels);
    diff_subscriptions(&mut changes, &source.subscriptions, &target.subscriptions);
    diff_streams(&mut changes, &source.streams, &target.streams);
    diff_layout(&mut changes, &source.layout, &target.layout);
    diff_relays(&mut changes, &source.relays, &target.relays);
    changes
}

fn diff_channels(
    changes: &mut Vec<ChannelGraphChange>,
    source: &[ChannelDefinition],
    target: &[ChannelDefinition],
) {
    let before = source
        .iter()
        .map(|value| (value.id.as_str(), value))
        .collect::<BTreeMap<_, _>>();
    let after = target
        .iter()
        .map(|value| (value.id.as_str(), value))
        .collect::<BTreeMap<_, _>>();
    diff_presence(
        changes,
        ChannelObjectKind::Channel,
        &before,
        &after,
        |value| value.name.clone(),
    );
    for (id, left) in &before {
        let Some(right) = after.get(id) else { continue };
        if left.name != right.name {
            push_change(
                changes,
                ChannelChangeKind::Renamed,
                ChannelObjectKind::Channel,
                id,
                Some(left.name.clone()),
                Some(right.name.clone()),
                format!("name {:?} → {:?}", left.name, right.name),
            );
        }
        if left.revision != right.revision
            || left.scope != right.scope
            || left.accepted_observation_kinds != right.accepted_observation_kinds
            || left.broadcast_default != right.broadcast_default
        {
            push_change(
                changes,
                ChannelChangeKind::Modified,
                ChannelObjectKind::Channel,
                id,
                Some(left.revision.to_string()),
                Some(right.revision.to_string()),
                format!(
                    "revision {} → {} · routing posture changed",
                    left.revision, right.revision
                ),
            );
        }
    }
}

fn diff_subscriptions(
    changes: &mut Vec<ChannelGraphChange>,
    source: &[ChannelSubscription],
    target: &[ChannelSubscription],
) {
    let before = source
        .iter()
        .map(|value| (value.id.as_str(), value))
        .collect::<BTreeMap<_, _>>();
    let after = target
        .iter()
        .map(|value| (value.id.as_str(), value))
        .collect::<BTreeMap<_, _>>();
    diff_presence(
        changes,
        ChannelObjectKind::Subscription,
        &before,
        &after,
        |value| value.id.clone(),
    );
    for (id, left) in &before {
        let Some(right) = after.get(id) else { continue };
        if left.channel_ids != right.channel_ids {
            push_change(
                changes,
                ChannelChangeKind::Retargeted,
                ChannelObjectKind::Subscription,
                id,
                Some(left.channel_ids.join(", ")),
                Some(right.channel_ids.join(", ")),
                format!(
                    "channels [{}] → [{}]",
                    left.channel_ids.join(", "),
                    right.channel_ids.join(", ")
                ),
            );
        }
        if left.revision != right.revision
            || left.observation_kinds != right.observation_kinds
            || left.filters != right.filters
            || left.limit != right.limit
        {
            push_change(
                changes,
                ChannelChangeKind::Modified,
                ChannelObjectKind::Subscription,
                id,
                Some(left.revision.to_string()),
                Some(right.revision.to_string()),
                format!(
                    "revision {} → {} · selection or limit changed",
                    left.revision, right.revision
                ),
            );
        }
    }
}

fn diff_streams(
    changes: &mut Vec<ChannelGraphChange>,
    source: &[FeedStreamDefinition],
    target: &[FeedStreamDefinition],
) {
    let before = source
        .iter()
        .map(|value| (value.id.as_str(), value))
        .collect::<BTreeMap<_, _>>();
    let after = target
        .iter()
        .map(|value| (value.id.as_str(), value))
        .collect::<BTreeMap<_, _>>();
    diff_presence(
        changes,
        ChannelObjectKind::Stream,
        &before,
        &after,
        |value| value.name.clone(),
    );
    for (id, left) in &before {
        let Some(right) = after.get(id) else { continue };
        if left.name != right.name {
            push_change(
                changes,
                ChannelChangeKind::Renamed,
                ChannelObjectKind::Stream,
                id,
                Some(left.name.clone()),
                Some(right.name.clone()),
                format!("name {:?} → {:?}", left.name, right.name),
            );
        }
        if left.subscription_id != right.subscription_id {
            push_change(
                changes,
                ChannelChangeKind::Retargeted,
                ChannelObjectKind::Stream,
                id,
                Some(left.subscription_id.clone()),
                Some(right.subscription_id.clone()),
                format!(
                    "subscription {} → {}",
                    left.subscription_id, right.subscription_id
                ),
            );
        }
        if left.revision != right.revision || left.lens != right.lens {
            push_change(
                changes,
                ChannelChangeKind::Modified,
                ChannelObjectKind::Stream,
                id,
                Some(format!("{}@{}", left.lens, left.revision)),
                Some(format!("{}@{}", right.lens, right.revision)),
                format!(
                    "lens {}@{} → {}@{}",
                    left.lens, left.revision, right.lens, right.revision
                ),
            );
        }
    }
}

fn diff_layout(changes: &mut Vec<ChannelGraphChange>, source: &FeedLayout, target: &FeedLayout) {
    if source.id != target.id {
        push_change(
            changes,
            ChannelChangeKind::Removed,
            ChannelObjectKind::Layout,
            &source.id,
            Some(source.id.clone()),
            None,
            format!("removed layout {}", source.id),
        );
        push_change(
            changes,
            ChannelChangeKind::Added,
            ChannelObjectKind::Layout,
            &target.id,
            None,
            Some(target.id.clone()),
            format!("added layout {}", target.id),
        );
        return;
    }
    if source.revision != target.revision {
        push_change(
            changes,
            ChannelChangeKind::Modified,
            ChannelObjectKind::Layout,
            &source.id,
            Some(source.revision.to_string()),
            Some(target.revision.to_string()),
            format!("revision {} → {}", source.revision, target.revision),
        );
    }
    for (target_index, stream_id) in target.stream_ids.iter().enumerate() {
        let Some(source_index) = source
            .stream_ids
            .iter()
            .position(|candidate| candidate == stream_id)
        else {
            continue;
        };
        if source_index != target_index {
            push_change(
                changes,
                ChannelChangeKind::Moved,
                ChannelObjectKind::Stream,
                stream_id,
                Some((source_index + 1).to_string()),
                Some((target_index + 1).to_string()),
                format!("position {} → {}", source_index + 1, target_index + 1),
            );
        }
    }
}

fn diff_relays(
    changes: &mut Vec<ChannelGraphChange>,
    source: &[ChannelRelayDeclaration],
    target: &[ChannelRelayDeclaration],
) {
    let before = source
        .iter()
        .map(|value| (value.id.as_str(), value))
        .collect::<BTreeMap<_, _>>();
    let after = target
        .iter()
        .map(|value| (value.id.as_str(), value))
        .collect::<BTreeMap<_, _>>();
    diff_presence(
        changes,
        ChannelObjectKind::Relay,
        &before,
        &after,
        |value| value.target_channel_locator.clone(),
    );
    for (id, left) in &before {
        let Some(right) = after.get(id) else { continue };
        if left != right {
            push_change(
                changes,
                ChannelChangeKind::Modified,
                ChannelObjectKind::Relay,
                id,
                Some(format!("{}@{}", left.provider_id, left.revision)),
                Some(format!("{}@{}", right.provider_id, right.revision)),
                format!(
                    "{} → {} · provider {}@{} → {}@{}",
                    left.target_channel_locator,
                    right.target_channel_locator,
                    left.provider_id,
                    left.revision,
                    right.provider_id,
                    right.revision
                ),
            );
        }
    }
}

fn diff_presence<'a, T>(
    changes: &mut Vec<ChannelGraphChange>,
    object_kind: ChannelObjectKind,
    source: &BTreeMap<&'a str, &'a T>,
    target: &BTreeMap<&'a str, &'a T>,
    label: impl Fn(&T) -> String,
) {
    for (id, value) in source {
        if !target.contains_key(id) {
            let value = label(value);
            push_change(
                changes,
                ChannelChangeKind::Removed,
                object_kind,
                id,
                Some(value.clone()),
                None,
                format!("removed {value:?}"),
            );
        }
    }
    for (id, value) in target {
        if !source.contains_key(id) {
            let value = label(value);
            push_change(
                changes,
                ChannelChangeKind::Added,
                object_kind,
                id,
                None,
                Some(value.clone()),
                format!("added {value:?}"),
            );
        }
    }
}

fn push_change(
    changes: &mut Vec<ChannelGraphChange>,
    kind: ChannelChangeKind,
    object_kind: ChannelObjectKind,
    object_id: &str,
    before: Option<String>,
    after: Option<String>,
    detail: String,
) {
    changes.push(ChannelGraphChange {
        kind,
        object_kind,
        object_id: object_id.to_owned(),
        before,
        after,
        detail,
    });
}

fn summarize_changes(changes: &[ChannelGraphChange]) -> ChannelGraphDeltaSummary {
    let mut summary = ChannelGraphDeltaSummary::default();
    for change in changes {
        match change.kind {
            ChannelChangeKind::Added => summary.added += 1,
            ChannelChangeKind::Removed => summary.removed += 1,
            ChannelChangeKind::Modified => summary.modified += 1,
            ChannelChangeKind::Renamed => summary.renamed += 1,
            ChannelChangeKind::Retargeted => summary.retargeted += 1,
            ChannelChangeKind::Moved => summary.moved += 1,
        }
    }
    summary.total = changes.len() as u64;
    summary
}

fn channel_snapshot_identity(
    graph_id: &SemanticDigest,
    source: &ChannelGraphSource,
    limits: &ChannelGraphLimits,
) -> Result<SemanticDigest, ChannelGraphError> {
    let mut hasher = SemanticHasher::new(CHANNEL_GRAPH_SNAPSHOT_SCHEMA);
    hasher.add_str(graph_id.as_str());
    hasher.add_bytes(&serde_json::to_vec(source)?);
    hasher.add_bytes(&serde_json::to_vec(limits)?);
    Ok(hasher.finish())
}

fn validate_count(
    field: &'static str,
    actual: usize,
    minimum: usize,
    maximum: usize,
) -> Result<(), ChannelGraphError> {
    if actual < minimum || actual > maximum {
        return Err(ChannelGraphError::CountLimit {
            field,
            actual,
            minimum,
            maximum,
        });
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), ChannelGraphError> {
    let valid = !value.is_empty()
        && value.chars().count() <= MAX_IDENTIFIER_CHARS
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric);
    if !valid {
        return Err(ChannelGraphError::Identifier {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_name(field: &'static str, value: &str) -> Result<(), ChannelGraphError> {
    let count = value.chars().count();
    if count == 0
        || count > MAX_NAME_CHARS
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(ChannelGraphError::Name {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_locator(field: &'static str, value: &str) -> Result<(), ChannelGraphError> {
    if value.is_empty()
        || value.len() > MAX_LOCATOR_BYTES
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(ChannelGraphError::Locator {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_semantic_digest(
    field: &'static str,
    value: &SemanticDigest,
) -> Result<(), ChannelGraphError> {
    let value = value.as_str();
    let valid = value.len() == 71
        && value.starts_with("blake3:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit());
    if !valid {
        return Err(ChannelGraphError::Digest {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_revision(kind: &'static str, id: &str, revision: u64) -> Result<(), ChannelGraphError> {
    if revision == 0 {
        return Err(ChannelGraphError::Revision {
            kind,
            id: id.to_owned(),
        });
    }
    Ok(())
}

fn validate_unique_strings(
    kind: &'static str,
    owner: &str,
    values: &[String],
) -> Result<(), ChannelGraphError> {
    let mut unique = BTreeSet::new();
    for value in values {
        validate_identifier(kind, value)?;
        if !unique.insert(value.as_str()) {
            return Err(ChannelGraphError::DuplicateReference {
                kind,
                owner: owner.to_owned(),
                value: value.clone(),
            });
        }
    }
    Ok(())
}

fn validate_unique_kinds(
    kind: &'static str,
    owner: &str,
    values: &[ChannelObservationKind],
) -> Result<(), ChannelGraphError> {
    if values.is_empty() {
        return Err(ChannelGraphError::EmptyReferenceSet {
            kind,
            id: owner.to_owned(),
        });
    }
    let unique = values.iter().collect::<BTreeSet<_>>();
    if unique.len() != values.len() {
        return Err(ChannelGraphError::DuplicateObservationKind {
            kind,
            owner: owner.to_owned(),
        });
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum ChannelGraphError {
    #[error("expected schema {expected}, found {actual}")]
    Schema {
        expected: &'static str,
        actual: String,
    },
    #[error("channel graph {field} count {actual} is outside {minimum}..={maximum}")]
    CountLimit {
        field: &'static str,
        actual: usize,
        minimum: usize,
        maximum: usize,
    },
    #[error("invalid {field} {value:?}")]
    Identifier { field: &'static str, value: String },
    #[error("invalid bounded {field} {value:?}")]
    Name { field: &'static str, value: String },
    #[error("invalid bounded {field} locator {value:?}")]
    Locator { field: &'static str, value: String },
    #[error("invalid {field} digest {value:?}")]
    Digest { field: &'static str, value: String },
    #[error("{kind} {id} must use a positive revision")]
    Revision { kind: &'static str, id: String },
    #[error("duplicate {kind} id {id}")]
    Duplicate { kind: &'static str, id: String },
    #[error("duplicate {kind} reference {value} in {owner}")]
    DuplicateReference {
        kind: &'static str,
        owner: String,
        value: String,
    },
    #[error("duplicate {kind} in {owner}")]
    DuplicateObservationKind { kind: &'static str, owner: String },
    #[error("{kind} {id} requires at least one value")]
    EmptyReferenceSet { kind: &'static str, id: String },
    #[error("{owner_kind} {owner_id} references missing {target_kind} {target_id}")]
    MissingReference {
        owner_kind: &'static str,
        owner_id: String,
        target_kind: &'static str,
        target_id: String,
    },
    #[error("layout has {positions} positions for {streams} streams")]
    LayoutCoverage { streams: usize, positions: usize },
    #[error("subscription {id} limit {actual} is outside 1..={limit}")]
    SubscriptionLimit { id: String, actual: u64, limit: u64 },
    #[error("relay {id} hop limit {actual} is outside 1..={limit}")]
    RelayHopLimit { id: String, actual: u64, limit: u64 },
    #[error("channel graph is not in canonical object order")]
    NonCanonicalGraph,
    #[error("channel graph uses an unsupported effective limit envelope")]
    LimitEnvelope,
    #[error("channel graph identity mismatch: declared {declared}, actual {actual}")]
    GraphIdentity {
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("channel graph snapshot identity mismatch: declared {declared}, actual {actual}")]
    SnapshotIdentity {
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("working channel graph is stale: expected base {expected}, found {actual}")]
    StaleWorking {
        expected: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error(
        "{kind} {id} changed without advancing its revision beyond {previous}; proposed {proposed}"
    )]
    RevisionNotAdvanced {
        kind: &'static str,
        id: String,
        previous: u64,
        proposed: u64,
    },
    #[error("channel state exceeds {0} bytes")]
    StateByteLimit(u64),
    #[error("unsafe symlink or file type in channel state path {0}")]
    UnsafePath(PathBuf),
    #[error("could not read channel state {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not write channel state {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not lock channel state {path}: {source}")]
    Lock {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not allocate a temporary channel state file in {0}")]
    TemporaryLimit(PathBuf),
    #[error("invalid channel state JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{
        ChannelChangeKind, ChannelGraph, ChannelGraphError, ChannelGraphSnapshot,
        ChannelGraphSource, ChannelWorkingState, LocalChannelStore,
    };

    #[test]
    fn built_in_graph_is_canonical_and_preserves_feed_order() {
        let graph = ChannelGraph::built_in().unwrap();
        graph.verify().unwrap();
        assert_eq!(graph.channels.len(), 1);
        assert_eq!(graph.subscriptions.len(), 1);
        assert_eq!(graph.streams.len(), 3);
        assert_eq!(graph.layout.stream_ids, ["signals", "admission", "flow"]);
        assert_eq!(graph.channels[0].id, "workspace");
        assert!(graph.channels[0].broadcast_default);
    }

    #[test]
    fn graph_rejects_duplicate_and_dangling_references() {
        let mut duplicate = ChannelGraph::built_in().unwrap();
        duplicate.channels.push(duplicate.channels[0].clone());
        assert!(matches!(
            duplicate.canonicalize(),
            Err(ChannelGraphError::Duplicate {
                kind: "channel",
                ..
            })
        ));

        let mut dangling = ChannelGraph::built_in().unwrap();
        dangling.streams[0].subscription_id = "missing".to_owned();
        assert!(matches!(
            dangling.canonicalize(),
            Err(ChannelGraphError::MissingReference {
                owner_kind: "stream",
                ..
            })
        ));
    }

    #[test]
    fn graph_rejects_noncanonical_replay_and_hard_limit_overflow() {
        let mut noncanonical = ChannelGraph::built_in().unwrap();
        noncanonical.streams.swap(0, 1);
        assert!(matches!(
            noncanonical.verify(),
            Err(ChannelGraphError::NonCanonicalGraph)
        ));

        let mut oversized = ChannelGraph::built_in().unwrap();
        for index in 0..6 {
            let mut stream = oversized.streams[0].clone();
            stream.id = format!("extra-{index}");
            stream.name = format!("Extra {index}");
            oversized.layout.stream_ids.push(stream.id.clone());
            oversized.streams.push(stream);
        }
        assert!(matches!(
            oversized.canonicalize(),
            Err(ChannelGraphError::CountLimit {
                field: "streams",
                ..
            })
        ));
    }

    #[test]
    fn delta_names_renames_and_moves_instead_of_serialized_blobs() {
        let source = ChannelGraphSnapshot::built_in().unwrap();
        let mut graph = source.graph.clone();
        let admission = graph
            .streams
            .iter_mut()
            .find(|stream| stream.id == "admission")
            .unwrap();
        admission.name = "Review".to_owned();
        admission.revision = 2;
        graph.layout.stream_ids.swap(0, 1);
        graph.layout.revision = 2;
        let target = ChannelGraphSnapshot::new(
            graph,
            ChannelGraphSource::worktree("worktree:///channels.yaml".to_owned(), b"fixture"),
        )
        .unwrap();
        let delta =
            super::ChannelGraphDelta::derive("BUILT-IN", &source, "WORKING", &target).unwrap();
        assert!(
            delta
                .changes
                .iter()
                .any(|change| change.kind == ChannelChangeKind::Renamed)
        );
        assert_eq!(
            delta
                .changes
                .iter()
                .filter(|change| change.kind == ChannelChangeKind::Moved)
                .count(),
            2
        );
    }

    #[test]
    fn local_store_is_empty_by_default_and_replays_working_state() {
        let root = TempDir::new().unwrap();
        let store = LocalChannelStore::default_for_workspace(root.path());
        let clean = store.status().unwrap();
        assert_eq!(clean.state, ChannelWorkingState::Clean);
        assert!(!clean.working_present);
        assert!(!root.path().join(".rey").exists());

        let mut graph = clean.working.graph.clone();
        let flow = graph
            .streams
            .iter_mut()
            .find(|stream| stream.id == "flow")
            .unwrap();
        flow.name = "Outcomes".to_owned();
        flow.revision = 2;
        let result = store
            .apply(
                graph,
                ChannelGraphSource::worktree("worktree:///channels.yaml".to_owned(), b"fixture"),
            )
            .unwrap();
        assert!(result.applied);
        assert!(store.working_path().is_file());

        let replayed = LocalChannelStore::default_for_workspace(root.path())
            .status()
            .unwrap();
        assert_eq!(replayed.state, ChannelWorkingState::Working);
        assert_eq!(
            replayed.working.graph.stream("flow").unwrap().name,
            "Outcomes"
        );
    }

    #[test]
    fn tampered_working_identity_fails_closed() {
        let root = TempDir::new().unwrap();
        let store = LocalChannelStore::default_for_workspace(root.path());
        let mut graph = ChannelGraph::built_in().unwrap();
        graph.streams[0].name = "Changed".to_owned();
        graph.streams[0].revision = 2;
        store
            .apply(
                graph,
                ChannelGraphSource::worktree("worktree:///channels.yaml".to_owned(), b"one"),
            )
            .unwrap();
        let path = store.working_path();
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["snapshot"]["graph"]["streams"][0]["name"] = "Tampered".into();
        fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(matches!(
            store.status(),
            Err(ChannelGraphError::GraphIdentity { .. })
        ));
    }

    #[test]
    fn tampered_working_source_binding_fails_closed() {
        let root = TempDir::new().unwrap();
        let store = LocalChannelStore::default_for_workspace(root.path());
        let mut graph = ChannelGraph::built_in().unwrap();
        graph.streams[0].name = "Changed".to_owned();
        graph.streams[0].revision = 2;
        store
            .apply(
                graph,
                ChannelGraphSource::worktree("worktree:///channels.yaml".to_owned(), b"one"),
            )
            .unwrap();
        let path = store.working_path();
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["snapshot"]["source"]["locator"] = "worktree:///other.yaml".into();
        fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(matches!(
            store.status(),
            Err(ChannelGraphError::SnapshotIdentity { .. })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_working_state_fails_closed() {
        use std::os::unix::fs::symlink;

        let root = TempDir::new().unwrap();
        let directory = root.path().join(".rey/channels");
        fs::create_dir_all(&directory).unwrap();
        let target = root.path().join("outside.json");
        fs::write(&target, b"{}\n").unwrap();
        symlink(&target, directory.join("working.json")).unwrap();
        let store = LocalChannelStore::default_for_workspace(root.path());
        assert!(matches!(
            store.status(),
            Err(ChannelGraphError::UnsafePath(_))
        ));
    }
}
