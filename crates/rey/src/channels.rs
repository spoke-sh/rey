#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
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
pub const CHANNEL_ADMISSION_INDEX_SCHEMA: &str = "rey.channel-admission-index.v1";
pub const CHANNEL_ADD_RESULT_SCHEMA: &str = "rey.channel-add-result.v1";
pub const CHANNEL_COMMIT_SCHEMA: &str = "rey.channel-commit.v1";
pub const CHANNEL_COMMIT_RESULT_SCHEMA: &str = "rey.channel-commit-result.v1";
pub const CHANNEL_LOG_SCHEMA: &str = "rey.channel-log.v1";
pub const CHANNEL_MESSAGE_SCHEMA: &str = "rey.channel-message.v1";
pub const CHANNEL_MESSAGE_ADMISSION_SCHEMA: &str = "rey.channel-message-admission.v1";
pub const GITHUB_POLL_RECEIPT_SCHEMA: &str = "rey.github-channel-poll-receipt.v1";
pub const GITHUB_POLL_ADMISSION_SCHEMA: &str = "rey.github-channel-poll-admission.v1";
pub const CHANNEL_MAILBOX_SCHEMA: &str = "rey.channel-mailbox.v1";
pub const CHANNEL_RELAY_ATTEMPT_SCHEMA: &str = "rey.channel-relay-attempt.v1";
pub const POLLING_BEACON_TICK_SCHEMA: &str = "rey.polling-beacon-tick.v1";
pub const LOCAL_CHANNEL_HISTORY_SCHEMA: &str = "rey.local-channel-history.v1";
pub const MAX_CHANNEL_GRAPH_INPUT_BYTES: u64 = 1_024 * 1_024;
pub const MAX_CHANNEL_STATE_BYTES: u64 = 4 * 1_024 * 1_024;
pub const MAX_CHANNEL_COMMITS: usize = 256;
pub const MAX_CHANNEL_MESSAGE_BYTES: usize = 4_096;

const MAX_CHANNELS: usize = 32;
const MAX_SUBSCRIPTIONS: usize = 32;
const MAX_STREAMS: usize = 8;
const MAX_RELAYS: usize = 32;
const MAX_CHANNEL_APPLICATIONS: usize = 16;
const MAX_POLLING_BEACONS: usize = 16;
const MAX_RELAY_ARGUMENTS: usize = 32;
const MAX_RELAY_ARGUMENT_BYTES: usize = 4_096;
const MAX_BEACON_BATCH: u64 = 64;
const MIN_BEACON_INTERVAL_SECONDS: u64 = 5;
const MAX_BEACON_INTERVAL_SECONDS: u64 = 86_400;
const MAX_NAME_CHARS: usize = 80;
const MAX_IDENTIFIER_CHARS: usize = 80;
const MAX_LOCATOR_BYTES: usize = 4_096;
const MAX_SUBSCRIPTION_LIMIT: u64 = 256;
const MAX_RELAY_HOPS: u64 = 16;
const WORKING_FILE_NAME: &str = "working.json";
const STATE_FILE_NAME: &str = "state.json";
const MESSAGES_FILE_NAME: &str = "messages.json";
const ATTEMPTS_FILE_NAME: &str = "relay-attempts.json";
const LOCK_FILE_NAME: &str = "channels.lock";
const MAX_CHANNEL_MESSAGES: usize = 1_024;
const MAX_GITHUB_POLL_RECEIPTS: usize = 256;
const MAX_RELAY_ATTEMPTS: usize = 4_096;
const MAX_CHANNEL_MESSAGE_BODY_BYTES: usize = 16 * 1_024;
const MAX_GITHUB_NOTIFICATION_LIMIT: u64 = 50;
const MAX_GITHUB_PULL_REQUEST_LIMIT: u64 = 16;
const MAX_GITHUB_COMMENT_LIMIT: u64 = 50;
pub const MAX_GITHUB_POLL_MESSAGES: usize = 128;

pub const GITHUB_API_VERSION: &str = "2026-03-10";

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
pub struct ChannelApplicationDeclaration {
    pub id: String,
    pub revision: u64,
    pub environment_capability_id: String,
    pub executable_path: String,
    pub executable_version: Option<String>,
    pub executable_digest: String,
    #[serde(default)]
    pub relay_argv: Vec<String>,
    #[serde(default)]
    pub github_inbox: Option<GitHubInboxDeclaration>,
    pub timeout_ms: u64,
    pub max_output_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubInboxDeclaration {
    pub channel_id: String,
    pub hostname: String,
    pub poll_interval_seconds: u64,
    pub notification_limit: u64,
    pub pull_request_limit: u64,
    pub comment_limit: u64,
    pub credential_environment: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PollingBeaconDefinition {
    pub id: String,
    pub revision: u64,
    pub application_id: String,
    pub relay_ids: Vec<String>,
    pub interval_seconds: u64,
    pub batch_limit: u64,
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
    pub applications: Vec<ChannelApplicationDeclaration>,
    #[serde(default)]
    pub relays: Vec<ChannelRelayDeclaration>,
    #[serde(default)]
    pub beacons: Vec<PollingBeaconDefinition>,
}

impl ChannelGraph {
    pub fn canonicalize(mut self) -> Result<Self, ChannelGraphError> {
        validate_graph_members(&self)?;
        self.channels.sort_by(|left, right| left.id.cmp(&right.id));
        self.subscriptions
            .sort_by(|left, right| left.id.cmp(&right.id));
        self.streams.sort_by(|left, right| left.id.cmp(&right.id));
        self.relays.sort_by(|left, right| left.id.cmp(&right.id));
        self.applications
            .sort_by(|left, right| left.id.cmp(&right.id));
        self.beacons.sort_by(|left, right| left.id.cmp(&right.id));
        for channel in &mut self.channels {
            channel.accepted_observation_kinds.sort();
        }
        for subscription in &mut self.subscriptions {
            subscription.channel_ids.sort();
            subscription.observation_kinds.sort();
        }
        for application in &mut self.applications {
            if let Some(inbox) = &mut application.github_inbox {
                inbox.credential_environment.sort();
            }
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
            applications: Vec::new(),
            relays: Vec::new(),
            beacons: Vec::new(),
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
    pub max_applications: u64,
    pub max_polling_beacons: u64,
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
            max_applications: MAX_CHANNEL_APPLICATIONS as u64,
            max_polling_beacons: MAX_POLLING_BEACONS as u64,
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
    Staged,
    Mixed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelObjectKind {
    Channel,
    Subscription,
    Stream,
    Layout,
    Relay,
    Application,
    Beacon,
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
            Self::Application => "application",
            Self::Beacon => "beacon",
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
    pub head_commit: Option<ChannelCommit>,
    pub head: ChannelGraphSnapshot,
    pub index: Option<ChannelGraphSnapshot>,
    pub working: ChannelGraphSnapshot,
    pub staged: ChannelGraphDelta,
    pub unstaged: ChannelGraphDelta,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelCommit {
    pub schema: String,
    pub commit_id: SemanticDigest,
    pub sequence: u64,
    pub parent_commit_id: Option<SemanticDigest>,
    pub committed_at_unix: i64,
    pub message: String,
    pub snapshot: ChannelGraphSnapshot,
    pub delta: ChannelGraphDelta,
}

impl ChannelCommit {
    fn new(
        sequence: u64,
        parent_commit_id: Option<SemanticDigest>,
        message: String,
        source: &ChannelGraphSnapshot,
        snapshot: ChannelGraphSnapshot,
    ) -> Result<Self, ChannelGraphError> {
        let message = normalize_commit_message(message)?;
        let committed_at_unix = Utc::now().timestamp();
        validate_commit_timestamp(committed_at_unix)?;
        let delta = ChannelGraphDelta::derive(
            if sequence == 1 {
                "BUILT-IN".to_owned()
            } else {
                format!("CHANNEL@{}", sequence - 1)
            },
            source,
            format!("CHANNEL@{sequence}"),
            &snapshot,
        )?;
        let commit_id = channel_commit_identity(
            sequence,
            parent_commit_id.as_ref(),
            committed_at_unix,
            &message,
            &snapshot.snapshot_id,
            &delta.delta_id,
        );
        let commit = Self {
            schema: CHANNEL_COMMIT_SCHEMA.to_owned(),
            commit_id,
            sequence,
            parent_commit_id,
            committed_at_unix,
            message,
            snapshot,
            delta,
        };
        commit.verify(source)?;
        Ok(commit)
    }

    fn verify(&self, source: &ChannelGraphSnapshot) -> Result<(), ChannelGraphError> {
        if self.schema != CHANNEL_COMMIT_SCHEMA {
            return Err(ChannelGraphError::Schema {
                expected: CHANNEL_COMMIT_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        if self.sequence == 0 {
            return Err(ChannelGraphError::CommitSequence {
                expected: 1,
                actual: 0,
            });
        }
        validate_commit_timestamp(self.committed_at_unix)?;
        if normalize_commit_message(self.message.clone())? != self.message {
            return Err(ChannelGraphError::NonCanonicalCommitMessage);
        }
        self.snapshot.verify()?;
        let expected_delta = ChannelGraphDelta::derive(
            if self.sequence == 1 {
                "BUILT-IN".to_owned()
            } else {
                format!("CHANNEL@{}", self.sequence - 1)
            },
            source,
            format!("CHANNEL@{}", self.sequence),
            &self.snapshot,
        )?;
        if self.delta != expected_delta {
            return Err(ChannelGraphError::CommitDelta(self.sequence));
        }
        let actual = channel_commit_identity(
            self.sequence,
            self.parent_commit_id.as_ref(),
            self.committed_at_unix,
            &self.message,
            &self.snapshot.snapshot_id,
            &self.delta.delta_id,
        );
        if self.commit_id != actual {
            return Err(ChannelGraphError::CommitIdentity(self.sequence));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct LocalChannelHistory {
    schema: String,
    commits: Vec<ChannelCommit>,
    index: Option<ChannelAdmissionIndex>,
}

impl Default for LocalChannelHistory {
    fn default() -> Self {
        Self {
            schema: LOCAL_CHANNEL_HISTORY_SCHEMA.to_owned(),
            commits: Vec::new(),
            index: None,
        }
    }
}

impl LocalChannelHistory {
    fn verify(&self) -> Result<(), ChannelGraphError> {
        if self.schema != LOCAL_CHANNEL_HISTORY_SCHEMA {
            return Err(ChannelGraphError::Schema {
                expected: LOCAL_CHANNEL_HISTORY_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        if self.commits.len() > MAX_CHANNEL_COMMITS {
            return Err(ChannelGraphError::CommitLimit(MAX_CHANNEL_COMMITS));
        }
        let mut source = ChannelGraphSnapshot::built_in()?;
        let mut parent = None;
        let mut identities = BTreeSet::new();
        for (position, commit) in self.commits.iter().enumerate() {
            let expected = position as u64 + 1;
            if commit.sequence != expected {
                return Err(ChannelGraphError::CommitSequence {
                    expected,
                    actual: commit.sequence,
                });
            }
            if commit.parent_commit_id != parent {
                return Err(ChannelGraphError::CommitParent(commit.sequence));
            }
            commit.verify(&source)?;
            if commit.delta.assessment != DeltaAssessment::Different {
                return Err(ChannelGraphError::UnchangedCommit(commit.sequence));
            }
            if !identities.insert(commit.commit_id.clone()) {
                return Err(ChannelGraphError::DuplicateCommit(commit.commit_id.clone()));
            }
            parent = Some(commit.commit_id.clone());
            source = commit.snapshot.clone();
        }
        if let Some(index) = &self.index {
            index.verify(self)?;
        }
        Ok(())
    }

    fn head(&self) -> Option<&ChannelCommit> {
        self.commits.last()
    }

    fn head_snapshot(&self) -> Result<ChannelGraphSnapshot, ChannelGraphError> {
        Ok(self
            .head()
            .map_or(ChannelGraphSnapshot::built_in()?, |commit| {
                commit.snapshot.clone()
            }))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelAdmissionIndex {
    pub schema: String,
    pub index_id: SemanticDigest,
    pub base_commit_id: Option<SemanticDigest>,
    pub base_graph_id: SemanticDigest,
    pub snapshot: ChannelGraphSnapshot,
}

impl ChannelAdmissionIndex {
    fn new(
        history: &LocalChannelHistory,
        snapshot: ChannelGraphSnapshot,
    ) -> Result<Self, ChannelGraphError> {
        let head = history.head_snapshot()?;
        let base_commit_id = history.head().map(|commit| commit.commit_id.clone());
        let base_graph_id = head.graph_id;
        let index_id = channel_index_identity(
            base_commit_id.as_ref(),
            &base_graph_id,
            &snapshot.snapshot_id,
        );
        let index = Self {
            schema: CHANNEL_ADMISSION_INDEX_SCHEMA.to_owned(),
            index_id,
            base_commit_id,
            base_graph_id,
            snapshot,
        };
        index.verify(history)?;
        Ok(index)
    }

    fn verify(&self, history: &LocalChannelHistory) -> Result<(), ChannelGraphError> {
        if self.schema != CHANNEL_ADMISSION_INDEX_SCHEMA {
            return Err(ChannelGraphError::Schema {
                expected: CHANNEL_ADMISSION_INDEX_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        self.snapshot.verify()?;
        let expected_commit = history.head().map(|commit| commit.commit_id.clone());
        let expected_graph = history.head_snapshot()?.graph_id;
        if self.base_commit_id != expected_commit || self.base_graph_id != expected_graph {
            return Err(ChannelGraphError::StaleIndex);
        }
        let actual = channel_index_identity(
            self.base_commit_id.as_ref(),
            &self.base_graph_id,
            &self.snapshot.snapshot_id,
        );
        if self.index_id != actual {
            return Err(ChannelGraphError::IndexIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelAddResult {
    pub schema: String,
    pub index: ChannelAdmissionIndex,
    pub staged: ChannelGraphDelta,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelCommitResult {
    pub schema: String,
    pub commit: ChannelCommit,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelLog {
    pub schema: String,
    pub head_commit_id: Option<SemanticDigest>,
    pub total_commits: u64,
    pub selected_commits: u64,
    pub patch: bool,
    pub commits: Vec<ChannelCommit>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelMessageProposal {
    pub schema: String,
    pub channel_id: String,
    pub kind: ChannelObservationKind,
    pub body: String,
    #[serde(default)]
    pub evidence_locators: Vec<String>,
}

impl ChannelMessageProposal {
    pub fn verify(&self) -> Result<(), ChannelGraphError> {
        if self.schema != CHANNEL_MESSAGE_SCHEMA {
            return Err(ChannelGraphError::Schema {
                expected: CHANNEL_MESSAGE_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_identifier("message channel", &self.channel_id)?;
        if self.body.is_empty()
            || self.body.len() > MAX_CHANNEL_MESSAGE_BODY_BYTES
            || self.body.trim() != self.body
            || self.body.contains('\0')
        {
            return Err(ChannelGraphError::MessageBody);
        }
        if self.evidence_locators.len() > 32 {
            return Err(ChannelGraphError::MessageEvidenceLimit);
        }
        let mut locators = BTreeSet::new();
        for locator in &self.evidence_locators {
            validate_locator("message evidence", locator)?;
            if !locators.insert(locator) {
                return Err(ChannelGraphError::MessageEvidenceDuplicate(locator.clone()));
            }
        }
        Ok(())
    }

    fn identity(&self, source: &ChannelMessageSource) -> Result<SemanticDigest, ChannelGraphError> {
        self.verify()?;
        source.verify()?;
        let mut hasher = SemanticHasher::new(CHANNEL_MESSAGE_SCHEMA);
        hasher.add_bytes(&serde_json::to_vec(self)?);
        hasher.add_bytes(&serde_json::to_vec(source)?);
        Ok(hasher.finish())
    }
}

impl ChannelMessage {
    #[must_use]
    pub fn relay_payload(&self) -> &str {
        &self.proposal.body
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelMessage {
    pub schema: String,
    pub message_id: SemanticDigest,
    pub sequence: u64,
    pub admitted_at_unix: i64,
    pub channel_head_commit_id: Option<SemanticDigest>,
    pub channel_graph_id: SemanticDigest,
    pub proposal: ChannelMessageProposal,
    pub source: ChannelMessageSource,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChannelMessageSource {
    LocalAdmission,
    GitHubNotification {
        application_id: String,
        application_revision: u64,
        external_id: String,
        source_revision: String,
        repository: String,
        subject_type: String,
        subject_title: String,
        reason: String,
        provider_unread: bool,
        occurred_at_unix: i64,
        html_url: String,
    },
    GitHubIssueComment {
        application_id: String,
        application_revision: u64,
        external_id: String,
        source_revision: String,
        repository: String,
        pull_number: u64,
        author: String,
        occurred_at_unix: i64,
        html_url: String,
    },
    GitHubReviewComment {
        application_id: String,
        application_revision: u64,
        external_id: String,
        source_revision: String,
        repository: String,
        pull_number: u64,
        author: String,
        occurred_at_unix: i64,
        html_url: String,
        path: String,
    },
}

impl ChannelMessageSource {
    pub fn verify(&self) -> Result<(), ChannelGraphError> {
        match self {
            Self::LocalAdmission => Ok(()),
            Self::GitHubNotification {
                application_id,
                application_revision,
                external_id,
                source_revision,
                repository,
                subject_type,
                subject_title,
                reason,
                occurred_at_unix,
                html_url,
                ..
            } => {
                validate_github_source_common(
                    application_id,
                    *application_revision,
                    external_id,
                    source_revision,
                    repository,
                    *occurred_at_unix,
                    html_url,
                )?;
                validate_name("GitHub subject type", subject_type)?;
                validate_locator("GitHub subject title", subject_title)?;
                validate_name("GitHub notification reason", reason)
            }
            Self::GitHubIssueComment {
                application_id,
                application_revision,
                external_id,
                source_revision,
                repository,
                pull_number,
                author,
                occurred_at_unix,
                html_url,
            } => validate_github_comment_source(
                application_id,
                *application_revision,
                external_id,
                source_revision,
                repository,
                *pull_number,
                author,
                *occurred_at_unix,
                html_url,
            ),
            Self::GitHubReviewComment {
                application_id,
                application_revision,
                external_id,
                source_revision,
                repository,
                pull_number,
                author,
                occurred_at_unix,
                html_url,
                path,
            } => {
                validate_github_comment_source(
                    application_id,
                    *application_revision,
                    external_id,
                    source_revision,
                    repository,
                    *pull_number,
                    author,
                    *occurred_at_unix,
                    html_url,
                )?;
                validate_locator("GitHub review path", path)
            }
        }
    }

    #[must_use]
    pub const fn occurred_at_unix(&self) -> Option<i64> {
        match self {
            Self::LocalAdmission => None,
            Self::GitHubNotification {
                occurred_at_unix, ..
            }
            | Self::GitHubIssueComment {
                occurred_at_unix, ..
            }
            | Self::GitHubReviewComment {
                occurred_at_unix, ..
            } => Some(*occurred_at_unix),
        }
    }

    pub fn github_application(&self) -> Option<(&str, u64)> {
        match self {
            Self::LocalAdmission => None,
            Self::GitHubNotification {
                application_id,
                application_revision,
                ..
            }
            | Self::GitHubIssueComment {
                application_id,
                application_revision,
                ..
            }
            | Self::GitHubReviewComment {
                application_id,
                application_revision,
                ..
            } => Some((application_id, *application_revision)),
        }
    }
}

impl ChannelMessage {
    fn verify(&self) -> Result<(), ChannelGraphError> {
        if self.schema != CHANNEL_MESSAGE_ADMISSION_SCHEMA {
            return Err(ChannelGraphError::Schema {
                expected: CHANNEL_MESSAGE_ADMISSION_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        self.proposal.verify()?;
        validate_commit_timestamp(self.admitted_at_unix)?;
        if self.sequence == 0 {
            return Err(ChannelGraphError::MessageSequence);
        }
        self.source.verify()?;
        if self.message_id != self.proposal.identity(&self.source)? {
            return Err(ChannelGraphError::MessageIdentity);
        }
        validate_semantic_digest("message channel graph", &self.channel_graph_id)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ChannelMessageLog {
    #[serde(default)]
    messages: Vec<ChannelMessage>,
    #[serde(default)]
    github_polls: Vec<GitHubPollReceipt>,
}

fn github_new_message_count(
    log: &ChannelMessageLog,
    poll: &GitHubPollProposal,
) -> Result<usize, ChannelGraphError> {
    let retained = log
        .messages
        .iter()
        .map(|message| &message.message_id)
        .collect::<BTreeSet<_>>();
    poll.messages.iter().try_fold(0_usize, |count, message| {
        let message_id = message.proposal.identity(&message.source)?;
        Ok(count + usize::from(!retained.contains(&message_id)))
    })
}

fn prune_unreferenced_github_messages(log: &mut ChannelMessageLog) {
    let referenced = log
        .github_polls
        .iter()
        .flat_map(|receipt| receipt.current_message_ids.iter())
        .collect::<BTreeSet<_>>();
    log.messages.retain(|message| {
        message.source.github_application().is_none() || referenced.contains(&message.message_id)
    });
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelMessageAdmission {
    pub schema: String,
    pub admitted: bool,
    pub message: ChannelMessage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitHubPolledMessage {
    pub proposal: ChannelMessageProposal,
    pub source: ChannelMessageSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitHubPollProposal {
    pub polled_at_unix: i64,
    pub expected_channel_head_commit_id: SemanticDigest,
    pub expected_channel_graph_id: SemanticDigest,
    pub application_id: String,
    pub application_revision: u64,
    pub environment_commit_id: SemanticDigest,
    pub environment_capability_id: String,
    pub hostname: String,
    pub request_count: u64,
    pub notification_count: u64,
    pub pull_request_count: u64,
    pub issue_comment_count: u64,
    pub review_comment_count: u64,
    pub complete: bool,
    pub omissions: Vec<String>,
    pub responses: Vec<GitHubApiResponseEvidence>,
    pub messages: Vec<GitHubPolledMessage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubApiResponseEvidence {
    pub endpoint: String,
    pub content_digest: SemanticDigest,
    pub bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubPollReceipt {
    pub schema: String,
    pub poll_id: SemanticDigest,
    pub sequence: u64,
    pub polled_at_unix: i64,
    pub channel_head_commit_id: SemanticDigest,
    pub channel_graph_id: SemanticDigest,
    pub application_id: String,
    pub application_revision: u64,
    pub environment_commit_id: SemanticDigest,
    pub environment_capability_id: String,
    pub hostname: String,
    pub api_version: String,
    pub request_count: u64,
    pub notification_count: u64,
    pub pull_request_count: u64,
    pub issue_comment_count: u64,
    pub review_comment_count: u64,
    pub admitted_message_count: u64,
    pub reused_message_count: u64,
    pub current_message_ids: Vec<SemanticDigest>,
    pub complete: bool,
    pub omissions: Vec<String>,
    pub responses: Vec<GitHubApiResponseEvidence>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubPollAdmission {
    pub schema: String,
    pub receipt: GitHubPollReceipt,
    pub messages: Vec<ChannelMessage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelMailboxProjection {
    pub schema: String,
    pub ordering: String,
    pub messages: Vec<ChannelMessage>,
    pub polls: Vec<GitHubPollReceipt>,
    pub complete: bool,
    pub omissions: Vec<String>,
    pub max_messages: u64,
}

impl GitHubPollProposal {
    fn verify(&self) -> Result<(), ChannelGraphError> {
        validate_commit_timestamp(self.polled_at_unix)?;
        validate_semantic_digest(
            "GitHub poll Channel HEAD",
            &self.expected_channel_head_commit_id,
        )?;
        validate_semantic_digest("GitHub poll graph", &self.expected_channel_graph_id)?;
        validate_identifier("GitHub poll application", &self.application_id)?;
        validate_revision(
            "GitHub poll application",
            &self.application_id,
            self.application_revision,
        )?;
        validate_semantic_digest("GitHub poll environment", &self.environment_commit_id)?;
        validate_identifier(
            "GitHub poll environment capability",
            &self.environment_capability_id,
        )?;
        if self.hostname != "github.com" {
            return Err(ChannelGraphError::GitHubHostname(self.hostname.clone()));
        }
        if self.request_count == 0 || self.request_count > 1 + MAX_GITHUB_PULL_REQUEST_LIMIT * 2 {
            return Err(ChannelGraphError::GitHubPollRequestLimit(
                self.request_count,
            ));
        }
        if self.notification_count > MAX_GITHUB_NOTIFICATION_LIMIT
            || self.pull_request_count > MAX_GITHUB_PULL_REQUEST_LIMIT
            || self.issue_comment_count > MAX_GITHUB_PULL_REQUEST_LIMIT * MAX_GITHUB_COMMENT_LIMIT
            || self.review_comment_count > MAX_GITHUB_PULL_REQUEST_LIMIT * MAX_GITHUB_COMMENT_LIMIT
            || self.messages.len() > MAX_GITHUB_POLL_MESSAGES
        {
            return Err(ChannelGraphError::GitHubPollResultLimit);
        }
        if self.omissions.len() > 64 || self.responses.len() > self.request_count as usize {
            return Err(ChannelGraphError::GitHubPollResultLimit);
        }
        for omission in &self.omissions {
            validate_locator("GitHub poll omission", omission)?;
        }
        for response in &self.responses {
            response.verify()?;
        }
        let mut message_ids = BTreeSet::new();
        for message in &self.messages {
            message.proposal.verify()?;
            message.source.verify()?;
            if !message_ids.insert(message.proposal.identity(&message.source)?) {
                return Err(ChannelGraphError::GitHubPollDuplicateMessage);
            }
        }
        if self.complete && !self.omissions.is_empty() {
            return Err(ChannelGraphError::GitHubPollCompleteness);
        }
        Ok(())
    }
}

impl GitHubApiResponseEvidence {
    fn verify(&self) -> Result<(), ChannelGraphError> {
        validate_locator("GitHub API endpoint", &self.endpoint)?;
        validate_semantic_digest("GitHub API response", &self.content_digest)?;
        if self.bytes == 0 || self.bytes > MAX_CHANNEL_STATE_BYTES {
            return Err(ChannelGraphError::GitHubPollResultLimit);
        }
        Ok(())
    }
}

impl GitHubPollReceipt {
    fn new(
        poll: GitHubPollProposal,
        sequence: u64,
        admitted_message_count: u64,
        reused_message_count: u64,
        current_message_ids: Vec<SemanticDigest>,
    ) -> Result<Self, ChannelGraphError> {
        let mut receipt = Self {
            schema: GITHUB_POLL_RECEIPT_SCHEMA.to_owned(),
            poll_id: SemanticHasher::new("rey.github-channel-poll-placeholder.v1").finish(),
            sequence,
            polled_at_unix: poll.polled_at_unix,
            channel_head_commit_id: poll.expected_channel_head_commit_id,
            channel_graph_id: poll.expected_channel_graph_id,
            application_id: poll.application_id,
            application_revision: poll.application_revision,
            environment_commit_id: poll.environment_commit_id,
            environment_capability_id: poll.environment_capability_id,
            hostname: poll.hostname,
            api_version: GITHUB_API_VERSION.to_owned(),
            request_count: poll.request_count,
            notification_count: poll.notification_count,
            pull_request_count: poll.pull_request_count,
            issue_comment_count: poll.issue_comment_count,
            review_comment_count: poll.review_comment_count,
            admitted_message_count,
            reused_message_count,
            current_message_ids,
            complete: poll.complete,
            omissions: poll.omissions,
            responses: poll.responses,
        };
        receipt.poll_id = receipt.identity()?;
        receipt.verify()?;
        Ok(receipt)
    }

    fn identity(&self) -> Result<SemanticDigest, ChannelGraphError> {
        let mut hasher = SemanticHasher::new(GITHUB_POLL_RECEIPT_SCHEMA);
        let mut value = serde_json::to_value(self)?;
        value["poll_id"] = serde_json::Value::Null;
        hasher.add_bytes(&serde_json::to_vec(&value)?);
        Ok(hasher.finish())
    }

    fn verify(&self) -> Result<(), ChannelGraphError> {
        if self.schema != GITHUB_POLL_RECEIPT_SCHEMA {
            return Err(ChannelGraphError::Schema {
                expected: GITHUB_POLL_RECEIPT_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_commit_timestamp(self.polled_at_unix)?;
        validate_semantic_digest("GitHub poll", &self.poll_id)?;
        if self.sequence == 0 {
            return Err(ChannelGraphError::GitHubPollSequence);
        }
        validate_semantic_digest("GitHub poll Channel HEAD", &self.channel_head_commit_id)?;
        validate_semantic_digest("GitHub poll graph", &self.channel_graph_id)?;
        validate_identifier("GitHub poll application", &self.application_id)?;
        validate_revision(
            "GitHub poll application",
            &self.application_id,
            self.application_revision,
        )?;
        validate_semantic_digest("GitHub poll environment", &self.environment_commit_id)?;
        validate_identifier(
            "GitHub poll environment capability",
            &self.environment_capability_id,
        )?;
        if self.hostname != "github.com" || self.api_version != GITHUB_API_VERSION {
            return Err(ChannelGraphError::GitHubHostname(self.hostname.clone()));
        }
        if self.request_count == 0
            || self.request_count > 1 + MAX_GITHUB_PULL_REQUEST_LIMIT * 2
            || self.notification_count > MAX_GITHUB_NOTIFICATION_LIMIT
            || self.pull_request_count > MAX_GITHUB_PULL_REQUEST_LIMIT
            || self.issue_comment_count > MAX_GITHUB_PULL_REQUEST_LIMIT * MAX_GITHUB_COMMENT_LIMIT
            || self.review_comment_count > MAX_GITHUB_PULL_REQUEST_LIMIT * MAX_GITHUB_COMMENT_LIMIT
            || self.current_message_ids.len()
                != (self.admitted_message_count + self.reused_message_count) as usize
            || self.current_message_ids.len() > MAX_GITHUB_POLL_MESSAGES
            || self.responses.len() > self.request_count as usize
            || self.omissions.len() > 64
        {
            return Err(ChannelGraphError::GitHubPollResultLimit);
        }
        let mut identities = BTreeSet::new();
        for message_id in &self.current_message_ids {
            validate_semantic_digest("GitHub poll message", message_id)?;
            if !identities.insert(message_id) {
                return Err(ChannelGraphError::GitHubPollDuplicateMessage);
            }
        }
        for response in &self.responses {
            response.verify()?;
        }
        if self.complete && !self.omissions.is_empty() {
            return Err(ChannelGraphError::GitHubPollCompleteness);
        }
        if self.identity()? != self.poll_id {
            return Err(ChannelGraphError::GitHubPollIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayAttemptOutcome {
    Delivered,
    Failed,
    SkippedAlreadyDelivered,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RelayAttempt {
    pub schema: String,
    pub attempt_id: SemanticDigest,
    pub attempted_at_unix: i64,
    pub channel_commit_id: SemanticDigest,
    pub graph_id: SemanticDigest,
    pub relay_id: String,
    pub relay_revision: u64,
    pub application_id: String,
    pub application_revision: u64,
    pub environment_commit_id: SemanticDigest,
    pub environment_capability_id: String,
    pub message_id: SemanticDigest,
    pub target_channel_locator: String,
    pub outcome: RelayAttemptOutcome,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub stdout_digest: Option<SemanticDigest>,
    pub stderr_digest: Option<SemanticDigest>,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PollingBeaconTick {
    pub schema: String,
    pub beacon_id: String,
    pub beacon_revision: u64,
    pub checked_messages: u64,
    pub attempted: u64,
    pub delivered: u64,
    pub failed: u64,
    pub skipped: u64,
    pub attempts: Vec<RelayAttempt>,
}

impl RelayAttempt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        channel_commit_id: SemanticDigest,
        graph_id: SemanticDigest,
        relay: &ChannelRelayDeclaration,
        application: &ChannelApplicationDeclaration,
        environment_commit_id: SemanticDigest,
        message_id: SemanticDigest,
        outcome: RelayAttemptOutcome,
        exit_code: Option<i32>,
        timed_out: bool,
        stdout_digest: Option<SemanticDigest>,
        stderr_digest: Option<SemanticDigest>,
        detail: String,
    ) -> Self {
        let attempted_at_unix = Utc::now().timestamp();
        let mut hasher = SemanticHasher::new(CHANNEL_RELAY_ATTEMPT_SCHEMA);
        hasher.add_str(channel_commit_id.as_str());
        hasher.add_str(graph_id.as_str());
        hasher.add_str(&relay.id);
        hasher.add_u64(relay.revision);
        hasher.add_str(&application.id);
        hasher.add_u64(application.revision);
        hasher.add_str(environment_commit_id.as_str());
        hasher.add_str(message_id.as_str());
        hasher.add_str(&relay.target_channel_locator);
        hasher.add_str(&attempted_at_unix.to_string());
        hasher.add_str(match outcome {
            RelayAttemptOutcome::Delivered => "delivered",
            RelayAttemptOutcome::Failed => "failed",
            RelayAttemptOutcome::SkippedAlreadyDelivered => "skipped_already_delivered",
        });
        hasher.add_optional_str(exit_code.map(|value| value.to_string()).as_deref());
        hasher.add_str(if timed_out { "true" } else { "false" });
        hasher.add_optional_str(stdout_digest.as_ref().map(SemanticDigest::as_str));
        hasher.add_optional_str(stderr_digest.as_ref().map(SemanticDigest::as_str));
        hasher.add_str(&detail);
        Self {
            schema: CHANNEL_RELAY_ATTEMPT_SCHEMA.to_owned(),
            attempt_id: hasher.finish(),
            attempted_at_unix,
            channel_commit_id,
            graph_id,
            relay_id: relay.id.clone(),
            relay_revision: relay.revision,
            application_id: application.id.clone(),
            application_revision: application.revision,
            environment_commit_id,
            environment_capability_id: application.environment_capability_id.clone(),
            message_id,
            target_channel_locator: relay.target_channel_locator.clone(),
            outcome,
            exit_code,
            timed_out,
            stdout_digest,
            stderr_digest,
            detail,
        }
    }

    fn verify(&self) -> Result<(), ChannelGraphError> {
        if self.schema != CHANNEL_RELAY_ATTEMPT_SCHEMA {
            return Err(ChannelGraphError::Schema {
                expected: CHANNEL_RELAY_ATTEMPT_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_commit_timestamp(self.attempted_at_unix)?;
        validate_semantic_digest("relay channel commit", &self.channel_commit_id)?;
        validate_semantic_digest("relay graph", &self.graph_id)?;
        validate_identifier("relay id", &self.relay_id)?;
        validate_revision("relay", &self.relay_id, self.relay_revision)?;
        validate_identifier("relay application", &self.application_id)?;
        validate_revision(
            "relay application",
            &self.application_id,
            self.application_revision,
        )?;
        validate_semantic_digest("relay environment commit", &self.environment_commit_id)?;
        validate_identifier(
            "relay environment capability",
            &self.environment_capability_id,
        )?;
        validate_semantic_digest("relay message", &self.message_id)?;
        validate_locator("relay target", &self.target_channel_locator)?;
        if self.detail.is_empty()
            || self.detail.len() > MAX_RELAY_ARGUMENT_BYTES
            || self.detail.chars().any(char::is_control)
        {
            return Err(ChannelGraphError::RelayAttemptDetail);
        }
        let mut hasher = SemanticHasher::new(CHANNEL_RELAY_ATTEMPT_SCHEMA);
        hasher.add_str(self.channel_commit_id.as_str());
        hasher.add_str(self.graph_id.as_str());
        hasher.add_str(&self.relay_id);
        hasher.add_u64(self.relay_revision);
        hasher.add_str(&self.application_id);
        hasher.add_u64(self.application_revision);
        hasher.add_str(self.environment_commit_id.as_str());
        hasher.add_str(self.message_id.as_str());
        hasher.add_str(&self.target_channel_locator);
        hasher.add_str(&self.attempted_at_unix.to_string());
        hasher.add_str(match self.outcome {
            RelayAttemptOutcome::Delivered => "delivered",
            RelayAttemptOutcome::Failed => "failed",
            RelayAttemptOutcome::SkippedAlreadyDelivered => "skipped_already_delivered",
        });
        hasher.add_optional_str(self.exit_code.map(|value| value.to_string()).as_deref());
        hasher.add_str(if self.timed_out { "true" } else { "false" });
        hasher.add_optional_str(self.stdout_digest.as_ref().map(SemanticDigest::as_str));
        hasher.add_optional_str(self.stderr_digest.as_ref().map(SemanticDigest::as_str));
        hasher.add_str(&self.detail);
        if self.attempt_id != hasher.finish() {
            return Err(ChannelGraphError::RelayAttemptIdentity);
        }
        Ok(())
    }
}

#[must_use]
pub fn relay_output_digest(stream: &str, bytes: &[u8]) -> Option<SemanticDigest> {
    if bytes.is_empty() {
        return None;
    }
    let mut hasher = SemanticHasher::new(CHANNEL_RELAY_ATTEMPT_SCHEMA);
    hasher.add_str(stream);
    hasher.add_bytes(bytes);
    Some(hasher.finish())
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
        let history = self.load_history()?;
        let head = history.head_snapshot()?;
        let working = self.load_working(&head)?.unwrap_or_else(|| head.clone());
        let working_present = self.working_path().exists();
        let index = history.index.as_ref().map(|index| index.snapshot.clone());
        let effective_index = index.as_ref().unwrap_or(&head);
        let head_label = history.head().map_or_else(
            || "BUILT-IN".to_owned(),
            |commit| format!("CHANNEL@{}", commit.sequence),
        );
        let staged = ChannelGraphDelta::derive(head_label, &head, "INDEX", effective_index)?;
        let unstaged = ChannelGraphDelta::derive("INDEX", effective_index, "WORKING", &working)?;
        let state = match (
            staged.assessment == DeltaAssessment::Different,
            unstaged.assessment == DeltaAssessment::Different,
        ) {
            (false, false) => ChannelWorkingState::Clean,
            (false, true) => ChannelWorkingState::Working,
            (true, false) => ChannelWorkingState::Staged,
            (true, true) => ChannelWorkingState::Mixed,
        };
        Ok(ChannelStatus {
            schema: CHANNEL_STATUS_SCHEMA.to_owned(),
            state,
            working_present,
            head_commit: history.head().cloned(),
            head,
            index,
            working,
            staged,
            unstaged,
        })
    }

    pub fn diff(&self, staged: bool) -> Result<ChannelDiff, ChannelGraphError> {
        let status = self.status()?;
        let (source, target, delta) = if staged {
            let target = status.index.clone().unwrap_or_else(|| status.head.clone());
            (status.head, target, status.staged)
        } else {
            (
                status.index.unwrap_or_else(|| status.head.clone()),
                status.working,
                status.unstaged,
            )
        };
        Ok(ChannelDiff {
            schema: CHANNEL_DIFF_SCHEMA.to_owned(),
            source,
            target,
            delta,
        })
    }

    pub fn add(&self) -> Result<ChannelAddResult, ChannelGraphError> {
        self.with_lock(|| {
            let mut history = self.load_history()?;
            let head = history.head_snapshot()?;
            let working = self.load_working(&head)?.unwrap_or_else(|| head.clone());
            if working.graph_id == head.graph_id {
                return Err(ChannelGraphError::NothingToAdd);
            }
            validate_revision_progress(&head.graph, &working.graph)?;
            let index = ChannelAdmissionIndex::new(&history, working)?;
            let staged = ChannelGraphDelta::derive("HEAD", &head, "INDEX", &index.snapshot)?;
            history.index = Some(index.clone());
            self.save_history(&history)?;
            Ok(ChannelAddResult {
                schema: CHANNEL_ADD_RESULT_SCHEMA.to_owned(),
                index,
                staged,
            })
        })
    }

    pub fn commit(&self, message: String) -> Result<ChannelCommitResult, ChannelGraphError> {
        let message = normalize_commit_message(message)?;
        self.with_lock(|| {
            let mut history = self.load_history()?;
            if history.commits.len() >= MAX_CHANNEL_COMMITS {
                return Err(ChannelGraphError::CommitLimit(MAX_CHANNEL_COMMITS));
            }
            let index = history
                .index
                .clone()
                .ok_or(ChannelGraphError::NothingStaged)?;
            index.verify(&history)?;
            let source = history.head_snapshot()?;
            if source.graph_id == index.snapshot.graph_id {
                return Err(ChannelGraphError::NothingToCommit);
            }
            validate_revision_progress(&source.graph, &index.snapshot.graph)?;
            let sequence = history.commits.len() as u64 + 1;
            let commit = ChannelCommit::new(
                sequence,
                history.head().map(|commit| commit.commit_id.clone()),
                message,
                &source,
                index.snapshot,
            )?;
            history.commits.push(commit.clone());
            history.index = None;
            self.save_history(&history)?;
            self.clear_working_if_matches(&commit.snapshot)?;
            Ok(ChannelCommitResult {
                schema: CHANNEL_COMMIT_RESULT_SCHEMA.to_owned(),
                commit,
            })
        })
    }

    pub fn log(&self, max_count: usize, patch: bool) -> Result<ChannelLog, ChannelGraphError> {
        if max_count == 0 || max_count > MAX_CHANNEL_COMMITS {
            return Err(ChannelGraphError::LogLimit {
                limit: MAX_CHANNEL_COMMITS,
                actual: max_count,
            });
        }
        let history = self.load_history()?;
        let commits = history
            .commits
            .iter()
            .rev()
            .take(max_count)
            .cloned()
            .collect::<Vec<_>>();
        Ok(ChannelLog {
            schema: CHANNEL_LOG_SCHEMA.to_owned(),
            head_commit_id: history.head().map(|commit| commit.commit_id.clone()),
            total_commits: history.commits.len() as u64,
            selected_commits: commits.len() as u64,
            patch,
            commits,
        })
    }

    pub fn admit_message(
        &self,
        proposal: ChannelMessageProposal,
    ) -> Result<ChannelMessageAdmission, ChannelGraphError> {
        proposal.verify()?;
        let source = ChannelMessageSource::LocalAdmission;
        self.with_lock(|| {
            let history = self.load_history()?;
            let head_commit = history
                .head()
                .ok_or(ChannelGraphError::NoAdmittedChannelHead)?;
            let channel = head_commit
                .snapshot
                .graph
                .channels
                .iter()
                .find(|channel| channel.id == proposal.channel_id)
                .ok_or_else(|| ChannelGraphError::UnknownChannel(proposal.channel_id.clone()))?;
            if !channel.accepted_observation_kinds.contains(&proposal.kind) {
                return Err(ChannelGraphError::RejectedMessageKind);
            }
            let mut log = self.load_messages()?;
            let message_id = proposal.identity(&source)?;
            if let Some(message) = log
                .messages
                .iter()
                .find(|message| message.message_id == message_id)
            {
                return Ok(ChannelMessageAdmission {
                    schema: CHANNEL_MESSAGE_ADMISSION_SCHEMA.to_owned(),
                    admitted: false,
                    message: message.clone(),
                });
            }
            if log.messages.len() >= MAX_CHANNEL_MESSAGES {
                return Err(ChannelGraphError::MessageLimit(MAX_CHANNEL_MESSAGES));
            }
            let message = ChannelMessage {
                schema: CHANNEL_MESSAGE_ADMISSION_SCHEMA.to_owned(),
                message_id,
                sequence: log
                    .messages
                    .last()
                    .map_or(1, |message| message.sequence.saturating_add(1)),
                admitted_at_unix: Utc::now().timestamp(),
                channel_head_commit_id: Some(head_commit.commit_id.clone()),
                channel_graph_id: head_commit.snapshot.graph_id.clone(),
                proposal,
                source,
            };
            message.verify()?;
            log.messages.push(message.clone());
            self.save_json(MESSAGES_FILE_NAME, &log)?;
            Ok(ChannelMessageAdmission {
                schema: CHANNEL_MESSAGE_ADMISSION_SCHEMA.to_owned(),
                admitted: true,
                message,
            })
        })
    }

    pub fn messages(&self) -> Result<Vec<ChannelMessage>, ChannelGraphError> {
        Ok(self.load_messages()?.messages)
    }

    pub fn admit_github_poll(
        &self,
        poll: GitHubPollProposal,
    ) -> Result<GitHubPollAdmission, ChannelGraphError> {
        poll.verify()?;
        self.with_lock(|| {
            let history = self.load_history()?;
            let head = history
                .head()
                .ok_or(ChannelGraphError::NoAdmittedChannelHead)?;
            if head.commit_id != poll.expected_channel_head_commit_id
                || head.snapshot.graph_id != poll.expected_channel_graph_id
            {
                return Err(ChannelGraphError::StaleGitHubPoll);
            }
            let application = head
                .snapshot
                .graph
                .applications
                .iter()
                .find(|application| application.id == poll.application_id)
                .ok_or_else(|| {
                    ChannelGraphError::UnknownGitHubApplication(poll.application_id.clone())
                })?;
            if application.revision != poll.application_revision
                || application.environment_capability_id != poll.environment_capability_id
                || application.github_inbox.is_none()
            {
                return Err(ChannelGraphError::StaleGitHubPoll);
            }
            let inbox = application
                .github_inbox
                .as_ref()
                .expect("GitHub inbox presence was checked");
            for message in &poll.messages {
                if message.proposal.channel_id != inbox.channel_id
                    || message.source.github_application()
                        != Some((application.id.as_str(), application.revision))
                {
                    return Err(ChannelGraphError::GitHubPollMessageBinding);
                }
            }

            let mut log = self.load_messages()?;
            let next_message_sequence = log
                .messages
                .last()
                .map_or(1, |message| message.sequence.saturating_add(1));
            let next_poll_sequence = log
                .github_polls
                .last()
                .map_or(1, |receipt| receipt.sequence.saturating_add(1));
            while log.github_polls.len() >= MAX_GITHUB_POLL_RECEIPTS {
                log.github_polls.remove(0);
                prune_unreferenced_github_messages(&mut log);
            }
            while log.messages.len() + github_new_message_count(&log, &poll)? > MAX_CHANNEL_MESSAGES
            {
                if log.github_polls.is_empty() {
                    return Err(ChannelGraphError::MessageLimit(MAX_CHANNEL_MESSAGES));
                }
                log.github_polls.remove(0);
                prune_unreferenced_github_messages(&mut log);
            }
            let mut selected = Vec::with_capacity(poll.messages.len());
            let mut current_message_ids = Vec::with_capacity(poll.messages.len());
            let mut admitted_message_count = 0_u64;
            let mut reused_message_count = 0_u64;
            let mut sequence = next_message_sequence;
            for polled in &poll.messages {
                let message_id = polled.proposal.identity(&polled.source)?;
                if let Some(message) = log
                    .messages
                    .iter()
                    .find(|message| message.message_id == message_id)
                    .cloned()
                {
                    reused_message_count += 1;
                    current_message_ids.push(message.message_id.clone());
                    selected.push(message);
                    continue;
                }
                if log.messages.len() >= MAX_CHANNEL_MESSAGES {
                    return Err(ChannelGraphError::MessageLimit(MAX_CHANNEL_MESSAGES));
                }
                let message = ChannelMessage {
                    schema: CHANNEL_MESSAGE_ADMISSION_SCHEMA.to_owned(),
                    message_id,
                    sequence,
                    admitted_at_unix: poll.polled_at_unix,
                    channel_head_commit_id: Some(head.commit_id.clone()),
                    channel_graph_id: head.snapshot.graph_id.clone(),
                    proposal: polled.proposal.clone(),
                    source: polled.source.clone(),
                };
                message.verify()?;
                admitted_message_count += 1;
                current_message_ids.push(message.message_id.clone());
                selected.push(message.clone());
                log.messages.push(message);
                sequence = sequence.saturating_add(1);
            }
            let receipt = GitHubPollReceipt::new(
                poll,
                next_poll_sequence,
                admitted_message_count,
                reused_message_count,
                current_message_ids,
            )?;
            log.github_polls.push(receipt.clone());
            self.save_json(MESSAGES_FILE_NAME, &log)?;
            Ok(GitHubPollAdmission {
                schema: GITHUB_POLL_ADMISSION_SCHEMA.to_owned(),
                receipt,
                messages: selected,
            })
        })
    }

    pub fn mailbox(
        &self,
        status: &ChannelStatus,
    ) -> Result<ChannelMailboxProjection, ChannelGraphError> {
        status.head.verify()?;
        let Some(head) = status.head_commit.as_ref() else {
            return Ok(ChannelMailboxProjection {
                schema: CHANNEL_MAILBOX_SCHEMA.to_owned(),
                ordering: "provider_updated_desc".to_owned(),
                messages: Vec::new(),
                polls: Vec::new(),
                complete: true,
                omissions: Vec::new(),
                max_messages: MAX_GITHUB_POLL_MESSAGES as u64,
            });
        };
        let log = self.load_messages()?;
        let github_applications = status
            .head
            .graph
            .applications
            .iter()
            .filter(|application| application.github_inbox.is_some())
            .collect::<Vec<_>>();
        let mut polls = Vec::new();
        let mut omissions = Vec::new();
        for application in github_applications {
            if let Some(receipt) = log.github_polls.iter().rev().find(|receipt| {
                receipt.channel_head_commit_id == head.commit_id
                    && receipt.channel_graph_id == status.head.graph_id
                    && receipt.application_id == application.id
                    && receipt.application_revision == application.revision
            }) {
                polls.push(receipt.clone());
            } else {
                omissions.push(format!(
                    "GitHub application {}@{} has no retained poll for current Channel HEAD",
                    application.id, application.revision
                ));
            }
        }
        let mut identities = BTreeSet::new();
        let mut messages = polls
            .iter()
            .flat_map(|poll| poll.current_message_ids.iter())
            .filter(|message_id| identities.insert((*message_id).clone()))
            .filter_map(|message_id| {
                log.messages
                    .iter()
                    .find(|message| &message.message_id == message_id)
                    .cloned()
            })
            .collect::<Vec<_>>();
        messages.sort_by(|left, right| {
            right
                .source
                .occurred_at_unix()
                .cmp(&left.source.occurred_at_unix())
                .then_with(|| right.sequence.cmp(&left.sequence))
        });
        if messages.len() > MAX_GITHUB_POLL_MESSAGES {
            messages.truncate(MAX_GITHUB_POLL_MESSAGES);
            omissions.push(format!(
                "mailbox projection retained only the newest {MAX_GITHUB_POLL_MESSAGES} GitHub messages"
            ));
        }
        let complete = omissions.is_empty() && polls.iter().all(|poll| poll.complete);
        for poll in &polls {
            omissions.extend(poll.omissions.iter().cloned());
        }
        Ok(ChannelMailboxProjection {
            schema: CHANNEL_MAILBOX_SCHEMA.to_owned(),
            ordering: "provider_updated_desc".to_owned(),
            messages,
            polls,
            complete,
            omissions,
            max_messages: MAX_GITHUB_POLL_MESSAGES as u64,
        })
    }

    pub fn relay_attempts(&self) -> Result<Vec<RelayAttempt>, ChannelGraphError> {
        self.load_attempts()
    }

    pub fn admitted_head(&self) -> Result<ChannelCommit, ChannelGraphError> {
        self.load_history()?
            .head()
            .cloned()
            .ok_or(ChannelGraphError::NoAdmittedChannelHead)
    }

    pub fn apply(
        &self,
        graph: ChannelGraph,
        source: ChannelGraphSource,
    ) -> Result<ChannelApplyResult, ChannelGraphError> {
        let target = ChannelGraphSnapshot::new(graph, source)?;
        self.with_lock(|| {
            let status = self.status()?;
            self.apply_from_status(status, target)
        })
    }

    pub fn apply_if_current(
        &self,
        graph: ChannelGraph,
        source: ChannelGraphSource,
        expected_head_snapshot_id: &SemanticDigest,
        expected_working_snapshot_id: &SemanticDigest,
    ) -> Result<ChannelApplyResult, ChannelGraphError> {
        let target = ChannelGraphSnapshot::new(graph, source)?;
        self.with_lock(|| {
            let status = self.status()?;
            if &status.head.snapshot_id != expected_head_snapshot_id {
                return Err(ChannelGraphError::WritePrecondition {
                    plane: "HEAD",
                    expected: expected_head_snapshot_id.clone(),
                    actual: status.head.snapshot_id,
                });
            }
            if &status.working.snapshot_id != expected_working_snapshot_id {
                return Err(ChannelGraphError::WritePrecondition {
                    plane: "WORKING",
                    expected: expected_working_snapshot_id.clone(),
                    actual: status.working.snapshot_id,
                });
            }
            self.apply_from_status(status, target)
        })
    }

    fn apply_from_status(
        &self,
        status: ChannelStatus,
        target: ChannelGraphSnapshot,
    ) -> Result<ChannelApplyResult, ChannelGraphError> {
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
        let head = self.load_history()?.head_snapshot()?;
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
        let (temporary, mut file) = self.create_temporary(WORKING_FILE_NAME)?;
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

    fn load_history(&self) -> Result<LocalChannelHistory, ChannelGraphError> {
        self.verify_directory_boundary()?;
        let path = self.directory.join(STATE_FILE_NAME);
        let Some(bytes) = self.read_optional_state(&path)? else {
            return Ok(LocalChannelHistory::default());
        };
        let history: LocalChannelHistory = serde_json::from_slice(&bytes)?;
        history.verify()?;
        Ok(history)
    }

    fn load_messages(&self) -> Result<ChannelMessageLog, ChannelGraphError> {
        self.verify_directory_boundary()?;
        let path = self.directory.join(MESSAGES_FILE_NAME);
        let Some(bytes) = self.read_optional_state(&path)? else {
            return Ok(ChannelMessageLog::default());
        };
        let log: ChannelMessageLog = serde_json::from_slice(&bytes)?;
        if log.messages.len() > MAX_CHANNEL_MESSAGES {
            return Err(ChannelGraphError::MessageLimit(MAX_CHANNEL_MESSAGES));
        }
        if log.github_polls.len() > MAX_GITHUB_POLL_RECEIPTS {
            return Err(ChannelGraphError::GitHubPollLimit(MAX_GITHUB_POLL_RECEIPTS));
        }
        let mut identities = BTreeSet::new();
        let mut prior_message_sequence = 0_u64;
        for message in &log.messages {
            message.verify()?;
            if message.sequence <= prior_message_sequence {
                return Err(ChannelGraphError::MessageSequence);
            }
            prior_message_sequence = message.sequence;
            if !identities.insert(message.message_id.clone()) {
                return Err(ChannelGraphError::MessageIdentity);
            }
        }
        let mut poll_identities = BTreeSet::new();
        let mut prior_poll_sequence = 0_u64;
        for poll in &log.github_polls {
            poll.verify()?;
            if poll.sequence <= prior_poll_sequence {
                return Err(ChannelGraphError::GitHubPollSequence);
            }
            prior_poll_sequence = poll.sequence;
            if !poll_identities.insert(poll.poll_id.clone()) {
                return Err(ChannelGraphError::GitHubPollIdentity);
            }
            for message_id in &poll.current_message_ids {
                if !identities.contains(message_id) {
                    return Err(ChannelGraphError::GitHubPollMessageBinding);
                }
            }
        }
        Ok(log)
    }

    fn load_attempts(&self) -> Result<Vec<RelayAttempt>, ChannelGraphError> {
        self.verify_directory_boundary()?;
        let path = self.directory.join(ATTEMPTS_FILE_NAME);
        let Some(bytes) = self.read_optional_state(&path)? else {
            return Ok(Vec::new());
        };
        let attempts: Vec<RelayAttempt> = serde_json::from_slice(&bytes)?;
        if attempts.len() > MAX_RELAY_ATTEMPTS {
            return Err(ChannelGraphError::RelayAttemptLimit(MAX_RELAY_ATTEMPTS));
        }
        let mut identities = BTreeSet::new();
        for attempt in &attempts {
            attempt.verify()?;
            if !identities.insert(attempt.attempt_id.clone()) {
                return Err(ChannelGraphError::RelayAttemptIdentity);
            }
        }
        Ok(attempts)
    }

    pub fn retain_relay_attempt(&self, attempt: RelayAttempt) -> Result<(), ChannelGraphError> {
        attempt.verify()?;
        self.with_lock(|| {
            let mut attempts = self.load_attempts()?;
            if attempts.len() >= MAX_RELAY_ATTEMPTS {
                return Err(ChannelGraphError::RelayAttemptLimit(MAX_RELAY_ATTEMPTS));
            }
            attempts.push(attempt);
            self.save_json(ATTEMPTS_FILE_NAME, &attempts)
        })
    }

    fn save_history(&self, history: &LocalChannelHistory) -> Result<(), ChannelGraphError> {
        history.verify()?;
        self.save_json(STATE_FILE_NAME, history)
    }

    fn read_optional_state(&self, path: &Path) -> Result<Option<Vec<u8>>, ChannelGraphError> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(ChannelGraphError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(ChannelGraphError::UnsafePath(path.to_owned()));
        }
        if metadata.len() > MAX_CHANNEL_STATE_BYTES {
            return Err(ChannelGraphError::StateByteLimit(MAX_CHANNEL_STATE_BYTES));
        }
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|source| ChannelGraphError::Read {
                path: path.to_owned(),
                source,
            })?
            .take(MAX_CHANNEL_STATE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| ChannelGraphError::Read {
                path: path.to_owned(),
                source,
            })?;
        if bytes.len() as u64 > MAX_CHANNEL_STATE_BYTES {
            return Err(ChannelGraphError::StateByteLimit(MAX_CHANNEL_STATE_BYTES));
        }
        Ok(Some(bytes))
    }

    fn save_json<T: Serialize>(&self, file_name: &str, value: &T) -> Result<(), ChannelGraphError> {
        let bytes = serde_json::to_vec_pretty(value)?;
        if bytes.len().saturating_add(1) as u64 > MAX_CHANNEL_STATE_BYTES {
            return Err(ChannelGraphError::StateByteLimit(MAX_CHANNEL_STATE_BYTES));
        }
        self.prepare_directory()?;
        let target = self.directory.join(file_name);
        if let Ok(metadata) = fs::symlink_metadata(&target)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(ChannelGraphError::UnsafePath(target));
        }
        let (temporary, mut file) = self.create_temporary(file_name)?;
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

    fn clear_working_if_matches(
        &self,
        head: &ChannelGraphSnapshot,
    ) -> Result<(), ChannelGraphError> {
        let path = self.working_path();
        let Some(bytes) = self.read_optional_state(&path)? else {
            return Ok(());
        };
        let document: ChannelWorkingDocument = serde_json::from_slice(&bytes)?;
        if document.snapshot.graph_id == head.graph_id {
            fs::remove_file(&path).map_err(|source| ChannelGraphError::Write { path, source })?;
        }
        Ok(())
    }

    fn with_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, ChannelGraphError>,
    ) -> Result<T, ChannelGraphError> {
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
        let result = operation();
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

    fn create_temporary(&self, file_name: &str) -> Result<(PathBuf, File), ChannelGraphError> {
        for attempt in 0..32_u8 {
            let path = self
                .directory
                .join(format!(".{file_name}.tmp-{}-{attempt}", std::process::id()));
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
    validate_count(
        "applications",
        graph.applications.len(),
        0,
        MAX_CHANNEL_APPLICATIONS,
    )?;
    validate_count("relays", graph.relays.len(), 0, MAX_RELAYS)?;
    validate_count("beacons", graph.beacons.len(), 0, MAX_POLLING_BEACONS)?;

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

    let mut application_ids = BTreeSet::new();
    for application in &graph.applications {
        validate_identifier("channel application id", &application.id)?;
        validate_revision("channel application", &application.id, application.revision)?;
        validate_identifier(
            "environment application capability",
            &application.environment_capability_id,
        )?;
        validate_locator(
            "channel application executable",
            &application.executable_path,
        )?;
        if !Path::new(&application.executable_path).is_absolute() {
            return Err(ChannelGraphError::ApplicationExecutable(
                application.id.clone(),
            ));
        }
        if let Some(version) = &application.executable_version {
            validate_name("channel application version", version)?;
        }
        validate_semantic_digest_str(
            "channel application executable",
            &application.executable_digest,
        )?;
        if application.relay_argv.len() > MAX_RELAY_ARGUMENTS
            || application.relay_argv.iter().any(|argument| {
                argument.is_empty()
                    || argument.len() > MAX_RELAY_ARGUMENT_BYTES
                    || argument.contains('\0')
            })
        {
            return Err(ChannelGraphError::RelayArguments(application.id.clone()));
        }
        let target_placeholders = application
            .relay_argv
            .iter()
            .filter(|argument| argument.as_str() == "{target}")
            .count();
        let message_placeholders = application
            .relay_argv
            .iter()
            .filter(|argument| argument.as_str() == "{message}")
            .count();
        if !application.relay_argv.is_empty()
            && (target_placeholders != 1
                || message_placeholders != 1
                || application.relay_argv.iter().any(|argument| {
                    (argument.contains("{target}") && argument != "{target}")
                        || (argument.contains("{message}") && argument != "{message}")
                }))
        {
            return Err(ChannelGraphError::RelayArguments(application.id.clone()));
        }
        if let Some(inbox) = &application.github_inbox {
            if application.environment_capability_id != "comms.application.github.identity" {
                return Err(ChannelGraphError::GitHubCapability(
                    application.environment_capability_id.clone(),
                ));
            }
            validate_identifier("GitHub inbox channel", &inbox.channel_id)?;
            if inbox.hostname != "github.com" {
                return Err(ChannelGraphError::GitHubHostname(inbox.hostname.clone()));
            }
            if inbox.poll_interval_seconds == 0
                || inbox.poll_interval_seconds > 3_600
                || inbox.notification_limit == 0
                || inbox.notification_limit > MAX_GITHUB_NOTIFICATION_LIMIT
                || inbox.pull_request_limit == 0
                || inbox.pull_request_limit > MAX_GITHUB_PULL_REQUEST_LIMIT
                || inbox.comment_limit == 0
                || inbox.comment_limit > MAX_GITHUB_COMMENT_LIMIT
            {
                return Err(ChannelGraphError::GitHubInboxLimit(application.id.clone()));
            }
            if inbox.credential_environment.is_empty() || inbox.credential_environment.len() > 4 {
                return Err(ChannelGraphError::GitHubCredentialEnvironment(
                    application.id.clone(),
                ));
            }
            let mut credential_environment = BTreeSet::new();
            for name in &inbox.credential_environment {
                if !matches!(
                    name.as_str(),
                    "HOME" | "GH_CONFIG_DIR" | "GH_TOKEN" | "GITHUB_TOKEN"
                ) || !credential_environment.insert(name)
                {
                    return Err(ChannelGraphError::GitHubCredentialEnvironment(
                        application.id.clone(),
                    ));
                }
            }
        }
        if application.timeout_ms == 0 || application.timeout_ms > 60_000 {
            return Err(ChannelGraphError::ApplicationTimeout(
                application.id.clone(),
            ));
        }
        if application.max_output_bytes == 0 || application.max_output_bytes > 1_048_576 {
            return Err(ChannelGraphError::ApplicationOutputLimit(
                application.id.clone(),
            ));
        }
        if !application_ids.insert(application.id.as_str()) {
            return Err(ChannelGraphError::Duplicate {
                kind: "application",
                id: application.id.clone(),
            });
        }
    }

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

    let mut beacon_ids = BTreeSet::new();
    for beacon in &graph.beacons {
        validate_identifier("polling beacon id", &beacon.id)?;
        validate_revision("polling beacon", &beacon.id, beacon.revision)?;
        validate_identifier("beacon application", &beacon.application_id)?;
        if beacon.relay_ids.is_empty() {
            return Err(ChannelGraphError::EmptyReferenceSet {
                kind: "beacon relays",
                id: beacon.id.clone(),
            });
        }
        validate_unique_strings("beacon relay", &beacon.id, &beacon.relay_ids)?;
        if !(MIN_BEACON_INTERVAL_SECONDS..=MAX_BEACON_INTERVAL_SECONDS)
            .contains(&beacon.interval_seconds)
        {
            return Err(ChannelGraphError::BeaconInterval(beacon.id.clone()));
        }
        if beacon.batch_limit == 0 || beacon.batch_limit > MAX_BEACON_BATCH {
            return Err(ChannelGraphError::BeaconBatch(beacon.id.clone()));
        }
        if !beacon_ids.insert(beacon.id.as_str()) {
            return Err(ChannelGraphError::Duplicate {
                kind: "beacon",
                id: beacon.id.clone(),
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
    let applications = graph
        .applications
        .iter()
        .map(|application| (application.id.as_str(), application))
        .collect::<BTreeMap<_, _>>();
    let relays = graph
        .relays
        .iter()
        .map(|relay| (relay.id.as_str(), relay))
        .collect::<BTreeMap<_, _>>();

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
        let Some(application) = applications.get(relay.provider_id.as_str()) else {
            return Err(ChannelGraphError::MissingReference {
                owner_kind: "relay",
                owner_id: relay.id.clone(),
                target_kind: "application",
                target_id: relay.provider_id.clone(),
            });
        };
        if application.relay_argv.is_empty() {
            return Err(ChannelGraphError::RelayArguments(application.id.clone()));
        }
    }
    for application in &graph.applications {
        if let Some(inbox) = &application.github_inbox
            && !channel_ids.contains(inbox.channel_id.as_str())
        {
            return Err(ChannelGraphError::MissingReference {
                owner_kind: "GitHub inbox",
                owner_id: application.id.clone(),
                target_kind: "channel",
                target_id: inbox.channel_id.clone(),
            });
        }
    }
    for beacon in &graph.beacons {
        if !applications.contains_key(beacon.application_id.as_str()) {
            return Err(ChannelGraphError::MissingReference {
                owner_kind: "beacon",
                owner_id: beacon.id.clone(),
                target_kind: "application",
                target_id: beacon.application_id.clone(),
            });
        }
        for relay_id in &beacon.relay_ids {
            let relay = relays.get(relay_id.as_str()).ok_or_else(|| {
                ChannelGraphError::MissingReference {
                    owner_kind: "beacon",
                    owner_id: beacon.id.clone(),
                    target_kind: "relay",
                    target_id: relay_id.clone(),
                }
            })?;
            if relay.provider_id != beacon.application_id {
                return Err(ChannelGraphError::BeaconApplicationMismatch {
                    beacon_id: beacon.id.clone(),
                    relay_id: relay.id.clone(),
                });
            }
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
        "application",
        &source.applications,
        &target.applications,
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
    validate_object_revision_progress(
        "beacon",
        &source.beacons,
        &target.beacons,
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
    diff_applications(&mut changes, &source.applications, &target.applications);
    diff_relays(&mut changes, &source.relays, &target.relays);
    diff_beacons(&mut changes, &source.beacons, &target.beacons);
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

fn diff_applications(
    changes: &mut Vec<ChannelGraphChange>,
    source: &[ChannelApplicationDeclaration],
    target: &[ChannelApplicationDeclaration],
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
        ChannelObjectKind::Application,
        &before,
        &after,
        |value| value.environment_capability_id.clone(),
    );
    for (id, left) in &before {
        let Some(right) = after.get(id) else { continue };
        if left != right {
            push_change(
                changes,
                ChannelChangeKind::Modified,
                ChannelObjectKind::Application,
                id,
                Some(format!(
                    "{}@{}",
                    left.environment_capability_id, left.revision
                )),
                Some(format!(
                    "{}@{}",
                    right.environment_capability_id, right.revision
                )),
                format!(
                    "revision {} → {} · executable or bounded relay adapter changed",
                    left.revision, right.revision
                ),
            );
        }
    }
}

fn diff_beacons(
    changes: &mut Vec<ChannelGraphChange>,
    source: &[PollingBeaconDefinition],
    target: &[PollingBeaconDefinition],
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
        ChannelObjectKind::Beacon,
        &before,
        &after,
        |value| value.application_id.clone(),
    );
    for (id, left) in &before {
        let Some(right) = after.get(id) else { continue };
        if left != right {
            push_change(
                changes,
                ChannelChangeKind::Modified,
                ChannelObjectKind::Beacon,
                id,
                Some(format!("{}@{}", left.application_id, left.revision)),
                Some(format!("{}@{}", right.application_id, right.revision)),
                format!(
                    "revision {} → {} · polling cadence, batch, or relay set changed",
                    left.revision, right.revision
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

fn channel_commit_identity(
    sequence: u64,
    parent_commit_id: Option<&SemanticDigest>,
    committed_at_unix: i64,
    message: &str,
    snapshot_id: &SemanticDigest,
    delta_id: &SemanticDigest,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(CHANNEL_COMMIT_SCHEMA);
    hasher.add_u64(sequence);
    hasher.add_optional_str(parent_commit_id.map(SemanticDigest::as_str));
    hasher.add_str(&committed_at_unix.to_string());
    hasher.add_str(message);
    hasher.add_str(snapshot_id.as_str());
    hasher.add_str(delta_id.as_str());
    hasher.finish()
}

fn channel_index_identity(
    base_commit_id: Option<&SemanticDigest>,
    base_graph_id: &SemanticDigest,
    snapshot_id: &SemanticDigest,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(CHANNEL_ADMISSION_INDEX_SCHEMA);
    hasher.add_optional_str(base_commit_id.map(SemanticDigest::as_str));
    hasher.add_str(base_graph_id.as_str());
    hasher.add_str(snapshot_id.as_str());
    hasher.finish()
}

fn normalize_commit_message(message: String) -> Result<String, ChannelGraphError> {
    let message = message.trim().to_owned();
    if message.is_empty() {
        return Err(ChannelGraphError::EmptyCommitMessage);
    }
    if message.len() > MAX_CHANNEL_MESSAGE_BYTES {
        return Err(ChannelGraphError::CommitMessageLimit(
            MAX_CHANNEL_MESSAGE_BYTES,
        ));
    }
    if message.contains('\0') {
        return Err(ChannelGraphError::CommitMessageNul);
    }
    Ok(message)
}

fn validate_commit_timestamp(committed_at_unix: i64) -> Result<(), ChannelGraphError> {
    DateTime::<Utc>::from_timestamp(committed_at_unix, 0)
        .ok_or(ChannelGraphError::CommitTimestamp(committed_at_unix))?;
    Ok(())
}

fn validate_github_source_common(
    application_id: &str,
    application_revision: u64,
    external_id: &str,
    source_revision: &str,
    repository: &str,
    occurred_at_unix: i64,
    html_url: &str,
) -> Result<(), ChannelGraphError> {
    validate_identifier("GitHub source application", application_id)?;
    validate_revision(
        "GitHub source application",
        application_id,
        application_revision,
    )?;
    validate_locator("GitHub external id", external_id)?;
    validate_locator("GitHub source revision", source_revision)?;
    validate_locator("GitHub repository", repository)?;
    validate_commit_timestamp(occurred_at_unix)?;
    if !html_url.starts_with("https://github.com/") {
        return Err(ChannelGraphError::GitHubSourceUrl(html_url.to_owned()));
    }
    validate_locator("GitHub source URL", html_url)
}

#[allow(clippy::too_many_arguments)]
fn validate_github_comment_source(
    application_id: &str,
    application_revision: u64,
    external_id: &str,
    source_revision: &str,
    repository: &str,
    pull_number: u64,
    author: &str,
    occurred_at_unix: i64,
    html_url: &str,
) -> Result<(), ChannelGraphError> {
    validate_github_source_common(
        application_id,
        application_revision,
        external_id,
        source_revision,
        repository,
        occurred_at_unix,
        html_url,
    )?;
    if pull_number == 0 {
        return Err(ChannelGraphError::GitHubPullNumber);
    }
    validate_name("GitHub comment author", author)
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

fn validate_semantic_digest_str(field: &'static str, value: &str) -> Result<(), ChannelGraphError> {
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
    #[error("channel application {0} executable must be an exact absolute path")]
    ApplicationExecutable(String),
    #[error(
        "channel application {0} relay argv must contain separate exact {{target}} and {{message}} arguments"
    )]
    RelayArguments(String),
    #[error("channel application {0} timeout is outside 1..=60000ms")]
    ApplicationTimeout(String),
    #[error("channel application {0} output limit is outside 1..=1048576 bytes")]
    ApplicationOutputLimit(String),
    #[error("GitHub inbox requires comms.application.github.identity, found {0}")]
    GitHubCapability(String),
    #[error("GitHub inbox supports only github.com in this revision, found {0}")]
    GitHubHostname(String),
    #[error("GitHub inbox limits are invalid for application {0}")]
    GitHubInboxLimit(String),
    #[error("GitHub credential environment is invalid for application {0}")]
    GitHubCredentialEnvironment(String),
    #[error("polling beacon {0} interval is outside 5..=86400 seconds")]
    BeaconInterval(String),
    #[error("polling beacon {0} batch limit is outside 1..=64")]
    BeaconBatch(String),
    #[error("polling beacon {beacon_id} application does not own relay {relay_id}")]
    BeaconApplicationMismatch { beacon_id: String, relay_id: String },
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
        "channel {plane} changed before the WORKING write: expected {expected}, found {actual}"
    )]
    WritePrecondition {
        plane: &'static str,
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
    #[error("channel commit sequence must be {expected}, got {actual}")]
    CommitSequence { expected: u64, actual: u64 },
    #[error("channel commit {0} does not name its exact predecessor")]
    CommitParent(u64),
    #[error("channel commit {0} repeats its parent graph")]
    UnchangedCommit(u64),
    #[error("channel commit {0} has a non-replayable semantic delta")]
    CommitDelta(u64),
    #[error("channel commit {0} identity does not match its content")]
    CommitIdentity(u64),
    #[error("duplicate channel commit {0}")]
    DuplicateCommit(SemanticDigest),
    #[error("channel admission index is not based on current HEAD")]
    StaleIndex,
    #[error("channel admission index identity does not match its content")]
    IndexIdentity,
    #[error("channel commit message must not be empty")]
    EmptyCommitMessage,
    #[error("channel commit message exceeds the {0}-byte limit")]
    CommitMessageLimit(usize),
    #[error("channel commit message must not contain NUL")]
    CommitMessageNul,
    #[error("channel commit message is not canonical")]
    NonCanonicalCommitMessage,
    #[error("channel commit timestamp {0} is outside the supported range")]
    CommitTimestamp(i64),
    #[error("working channel graph has no changes to add")]
    NothingToAdd,
    #[error("nothing staged in the channel admission index")]
    NothingStaged,
    #[error("nothing to commit; channel INDEX matches HEAD")]
    NothingToCommit,
    #[error("channel history exceeds the {0}-commit limit")]
    CommitLimit(usize),
    #[error("channel log count must be between 1 and {limit}, got {actual}")]
    LogLimit { limit: usize, actual: usize },
    #[error("a Channel HEAD commit is required before admitting or relaying messages")]
    NoAdmittedChannelHead,
    #[error("message targets unknown admitted channel {0}")]
    UnknownChannel(String),
    #[error("message kind is not accepted by its admitted channel")]
    RejectedMessageKind,
    #[error("channel message body is empty, non-canonical, or exceeds its byte limit")]
    MessageBody,
    #[error("channel message has too many evidence locators")]
    MessageEvidenceLimit,
    #[error("channel message repeats evidence locator {0}")]
    MessageEvidenceDuplicate(String),
    #[error("channel message retained sequence is not strictly increasing")]
    MessageSequence,
    #[error("channel message identity does not match its proposal")]
    MessageIdentity,
    #[error("GitHub source URL is not an exact github.com URL: {0}")]
    GitHubSourceUrl(String),
    #[error("GitHub pull request number must be positive")]
    GitHubPullNumber,
    #[error("channel message log exceeds the {0}-message limit")]
    MessageLimit(usize),
    #[error("GitHub poll receipt log exceeds the {0}-receipt limit")]
    GitHubPollLimit(usize),
    #[error("GitHub poll request count {0} exceeds its admitted bound")]
    GitHubPollRequestLimit(u64),
    #[error("GitHub poll result exceeds its admitted bound")]
    GitHubPollResultLimit,
    #[error("GitHub poll contains a duplicate message revision")]
    GitHubPollDuplicateMessage,
    #[error("GitHub poll completeness conflicts with retained omissions")]
    GitHubPollCompleteness,
    #[error("GitHub poll identity does not match its retained evidence")]
    GitHubPollIdentity,
    #[error("GitHub poll retained sequence is not strictly increasing")]
    GitHubPollSequence,
    #[error("GitHub poll message does not match its application or target channel")]
    GitHubPollMessageBinding,
    #[error("Channel HEAD changed before the GitHub poll could be retained")]
    StaleGitHubPoll,
    #[error("unknown admitted GitHub application {0}")]
    UnknownGitHubApplication(String),
    #[error("relay attempt log exceeds the {0}-attempt limit")]
    RelayAttemptLimit(usize),
    #[error("relay attempt identity does not match its retained evidence")]
    RelayAttemptIdentity,
    #[error("relay attempt detail is empty or invalid")]
    RelayAttemptDetail,
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
    fn expected_snapshot_write_rejects_stale_operator_state() {
        let root = TempDir::new().unwrap();
        let store = LocalChannelStore::default_for_workspace(root.path());
        let initial = store.status().unwrap();
        let mut graph = initial.working.graph.clone();
        let signals = graph
            .streams
            .iter_mut()
            .find(|stream| stream.id == "signals")
            .unwrap();
        signals.name = "Signal desk".to_owned();
        signals.revision = 2;
        let result = store
            .apply_if_current(
                graph,
                ChannelGraphSource::worktree("ui:///channels/working".to_owned(), b"operator"),
                &initial.head.snapshot_id,
                &initial.working.snapshot_id,
            )
            .unwrap();
        assert!(result.applied);

        let current = store.status().unwrap();
        assert_eq!(
            current.working.graph.stream("signals").unwrap().name,
            "Signal desk"
        );
        let stale = store.apply_if_current(
            current.working.graph.clone(),
            ChannelGraphSource::worktree("ui:///channels/working".to_owned(), b"stale"),
            &current.head.snapshot_id,
            &initial.working.snapshot_id,
        );
        assert!(matches!(
            stale,
            Err(ChannelGraphError::WritePrecondition {
                plane: "WORKING",
                ..
            })
        ));
        assert_eq!(store.status().unwrap(), current);
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
