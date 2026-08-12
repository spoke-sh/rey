#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_environment::{
    Availability, CapabilityRecord, CommandError, CommandOutput, CommandRequest,
    LOCAL_PROVIDER_REVISION, TrustClass, run_bounded,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const GIT_SNAPSHOT_SCHEMA: &str = "rey.git-repository.v1";
pub const GIT_COMMIT_SEQUENCE_SCHEMA: &str = "rey.git-commit-sequence.v1";
pub const GIT_REPOSITORY_STATUS_SCHEMA: &str = "rey.git-repository-status.v1";
pub const GIT_POLL_CURSOR_SCHEMA: &str = "rey.git-poll-cursor.v1";
pub const GIT_POLL_TRANSITION_SCHEMA: &str = "rey.git-poll-transition.v1";
pub const GIT_ACTIVATION_TRIGGER_SCHEMA: &str = "rey.git-activation-trigger.v1";
pub const GIT_ACTIVATION_PROPOSAL_SCHEMA: &str = "rey.git-activation-proposal.v1";
pub const MAX_GIT_COMMIT_SEQUENCE: usize = 256;
pub const MAX_GIT_ACTIVATION_TRIGGERS: usize = 256;
pub const MAX_GIT_ACTIVATION_SCENARIOS: usize = 256;
pub const MAX_GIT_WATCHED_REFS: usize = 256;
pub const MAX_GIT_REACHABLE_COMMITS_PER_DIRECTION: u64 = 4_096;
pub const MAX_GIT_PATH_CHANGES_PER_REF: u64 = 100_000;
pub const MAX_GIT_MATCHED_PATH_CHANGES: usize = 100_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitLimits {
    pub total_timeout_ms: u64,
    pub command_timeout_ms: u64,
    pub max_capture_bytes: u64,
    pub max_index_entries: u64,
    pub max_reachable_commits_per_direction: u64,
    pub max_path_changes_per_ref: u64,
}

impl Default for GitLimits {
    fn default() -> Self {
        Self {
            total_timeout_ms: 5_000,
            command_timeout_ms: 2_000,
            max_capture_bytes: 4 * 1_024 * 1_024,
            max_index_entries: 10_000,
            max_reachable_commits_per_direction: 256,
            max_path_changes_per_ref: 2_048,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PathIdentity {
    pub encoding: String,
    pub bytes: String,
    pub display: String,
}

impl PathIdentity {
    fn canonical(path: &Path) -> Result<Self, GitError> {
        let path = fs::canonicalize(path).map_err(|source| GitError::Path {
            path: path.to_owned(),
            source,
        })?;
        Ok(Self::from_path(&path))
    }

    fn from_path(path: &Path) -> Self {
        Self::from_bytes(path_bytes(path))
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            encoding: "base64url".to_owned(),
            bytes: URL_SAFE_NO_PAD.encode(bytes),
            display: String::from_utf8_lossy(bytes).into_owned(),
        }
    }

    fn add_semantics(&self, hasher: &mut SemanticHasher) {
        if let Ok(bytes) = URL_SAFE_NO_PAD.decode(&self.bytes) {
            hasher.add_bytes(&bytes);
        } else {
            hasher.add_str(&self.bytes);
        }
    }

    fn decoded_bytes(&self) -> Result<Vec<u8>, GitError> {
        if self.encoding != "base64url" {
            return Err(GitError::InvalidSnapshot);
        }
        URL_SAFE_NO_PAD
            .decode(&self.bytes)
            .map_err(|_| GitError::InvalidSnapshot)
    }

    fn verify(&self) -> Result<Vec<u8>, GitError> {
        let bytes = self.decoded_bytes()?;
        if bytes.is_empty() || self.display != String::from_utf8_lossy(&bytes) {
            return Err(GitError::InvalidSnapshot);
        }
        Ok(bytes)
    }
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> &[u8] {
    use std::os::unix::ffi::OsStrExt;

    path.as_os_str().as_bytes()
}

#[cfg(not(unix))]
fn path_bytes(path: &Path) -> &[u8] {
    path.to_str().unwrap_or_default().as_bytes()
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitHead {
    pub symbolic_ref: Option<String>,
    pub commit_oid: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitWatchedRef {
    pub name: String,
    pub target_oid: Option<String>,
}

impl GitWatchedRef {
    fn verify(&self, object_format: &str) -> Result<(), GitError> {
        if !valid_full_ref_name(&self.name)
            || self
                .target_oid
                .as_deref()
                .is_some_and(|oid| !valid_oid(oid, object_format))
        {
            return Err(GitError::InvalidSnapshot);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitCommitRecord {
    pub commit_oid: String,
    pub parent_oids: Vec<String>,
    pub committed_at_unix: i64,
    pub subject: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitCommitSequence {
    pub schema: String,
    pub sequence_id: SemanticDigest,
    pub object_format: String,
    pub head_oid: Option<String>,
    pub commits: Vec<GitCommitRecord>,
    pub complete: bool,
    pub shallow: bool,
    pub max_commits: u64,
    pub omissions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitWorkingTreeSummary {
    pub state: String,
    pub staged_entries: u64,
    pub unstaged_entries: u64,
    pub untracked_entries: u64,
    pub conflicted_entries: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitPublicationSummary {
    pub state: String,
    pub branch: Option<String>,
    pub head_oid: Option<String>,
    pub upstream: Option<String>,
    pub upstream_oid: Option<String>,
    pub ahead: Option<u64>,
    pub behind: Option<u64>,
    pub comparison_basis: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitRepositoryStatus {
    pub schema: String,
    pub status_id: SemanticDigest,
    pub working_tree: GitWorkingTreeSummary,
    pub publication: GitPublicationSummary,
    pub complete: bool,
    pub scope: String,
    pub omissions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitIndexSummary {
    pub entry_digest: SemanticDigest,
    pub entry_count: u64,
    pub entries: Vec<GitIndexEntry>,
    pub complete: bool,
    pub omitted_semantics: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitIndexEntry {
    pub mode: String,
    pub object_format: String,
    pub object_oid: String,
    pub stage: u8,
    pub path: PathIdentity,
    pub assume_unchanged: bool,
    pub skip_worktree: bool,
    pub intent_to_add: bool,
}

impl GitIndexSummary {
    fn verify(&self, object_format: &str) -> Result<(), GitError> {
        if self.entry_count != self.entries.len() as u64
            || self.complete != self.omitted_semantics.is_empty()
            || !is_canonical(&self.omitted_semantics)
        {
            return Err(GitError::InvalidSnapshot);
        }
        let mut hasher = SemanticHasher::new("rey.git-index-entries.v1");
        let mut previous_key: Option<(Vec<u8>, u8)> = None;
        for entry in &self.entries {
            let path = entry.path.verify()?;
            if entry.object_format != object_format
                || !valid_oid(&entry.object_oid, object_format)
                || entry.stage > 3
                || !entry.mode.bytes().all(|byte| byte.is_ascii_digit())
                || entry.stage != 0
                    && (entry.assume_unchanged || entry.skip_worktree || entry.intent_to_add)
            {
                return Err(GitError::InvalidSnapshot);
            }
            let key = (path.clone(), entry.stage);
            if previous_key
                .as_ref()
                .is_some_and(|previous| previous >= &key)
            {
                return Err(GitError::InvalidSnapshot);
            }
            previous_key = Some(key);
            hasher.add_str(&entry.mode);
            hasher.add_str(&entry.object_format);
            hasher.add_str(&entry.object_oid);
            hasher.add_str(&entry.stage.to_string());
            hasher.add_bytes(&path);
            hasher.add_bool(entry.assume_unchanged);
            hasher.add_bool(entry.skip_worktree);
            hasher.add_bool(entry.intent_to_add);
        }
        hasher.add_u64(self.entry_count);
        if self.entry_digest != hasher.finish() {
            return Err(GitError::InvalidSnapshot);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitSnapshot {
    pub schema: String,
    pub snapshot_id: SemanticDigest,
    pub repository_id: SemanticDigest,
    pub worktree_id: Option<SemanticDigest>,
    pub workspace_root: PathIdentity,
    pub common_directory: PathIdentity,
    pub git_directory: PathIdentity,
    pub worktree_root: Option<PathIdentity>,
    pub object_format: String,
    pub bare: bool,
    pub shallow: bool,
    pub head: GitHead,
    #[serde(default)]
    pub watched_refs: Vec<GitWatchedRef>,
    pub index: Option<GitIndexSummary>,
    pub complete: bool,
    pub limits: GitLimits,
}

impl GitSnapshot {
    pub fn verify(&self) -> Result<(), GitError> {
        if self.schema != GIT_SNAPSHOT_SCHEMA
            || !matches!(self.object_format.as_str(), "sha1" | "sha256")
            || [
                self.limits.total_timeout_ms,
                self.limits.command_timeout_ms,
                self.limits.max_capture_bytes,
                self.limits.max_index_entries,
                self.limits.max_reachable_commits_per_direction,
                self.limits.max_path_changes_per_ref,
            ]
            .contains(&0)
            || self.limits.max_reachable_commits_per_direction
                > MAX_GIT_REACHABLE_COMMITS_PER_DIRECTION
            || self.limits.max_path_changes_per_ref > MAX_GIT_PATH_CHANGES_PER_REF
            || self.head.symbolic_ref.as_deref().is_some_and(str::is_empty)
            || self
                .head
                .commit_oid
                .as_deref()
                .is_some_and(|oid| !valid_oid(oid, &self.object_format))
        {
            return Err(GitError::InvalidSnapshot);
        }
        self.workspace_root.verify()?;
        self.common_directory.verify()?;
        self.git_directory.verify()?;
        if let Some(root) = &self.worktree_root {
            root.verify()?;
        }
        let mut repository_hasher = SemanticHasher::new("rey.git-repository-id.v1");
        self.common_directory.add_semantics(&mut repository_hasher);
        repository_hasher.add_str(&self.object_format);
        if self.repository_id != repository_hasher.finish() {
            return Err(GitError::InvalidSnapshot);
        }
        let expected_worktree_id = self.worktree_root.as_ref().map(|root| {
            let mut hasher = SemanticHasher::new("rey.git-worktree-id.v1");
            root.add_semantics(&mut hasher);
            self.git_directory.add_semantics(&mut hasher);
            hasher.finish()
        });
        if self.worktree_id != expected_worktree_id
            || self.bare != self.worktree_root.is_none()
            || self.bare != self.index.is_none()
        {
            return Err(GitError::InvalidSnapshot);
        }
        if let Some(index) = &self.index {
            index.verify(&self.object_format)?;
        }
        verify_watched_refs(&self.watched_refs, &self.object_format)?;
        if self.complete != self.index.as_ref().is_none_or(|index| index.complete) {
            return Err(GitError::InvalidSnapshot);
        }
        let mut snapshot_hasher = SemanticHasher::new("rey.git-snapshot.v1");
        snapshot_hasher.add_str(self.repository_id.as_str());
        snapshot_hasher.add_optional_str(self.worktree_id.as_ref().map(SemanticDigest::as_str));
        snapshot_hasher.add_str(&self.object_format);
        snapshot_hasher.add_bool(self.bare);
        snapshot_hasher.add_bool(self.shallow);
        snapshot_hasher.add_optional_str(self.head.symbolic_ref.as_deref());
        snapshot_hasher.add_optional_str(self.head.commit_oid.as_deref());
        add_watched_refs(&mut snapshot_hasher, &self.watched_refs);
        snapshot_hasher.add_optional_str(
            self.index
                .as_ref()
                .map(|summary| summary.entry_digest.as_str()),
        );
        snapshot_hasher.add_bool(self.complete);
        snapshot_hasher.add_u64(self.limits.total_timeout_ms);
        snapshot_hasher.add_u64(self.limits.command_timeout_ms);
        snapshot_hasher.add_u64(self.limits.max_capture_bytes);
        snapshot_hasher.add_u64(self.limits.max_index_entries);
        snapshot_hasher.add_u64(self.limits.max_reachable_commits_per_direction);
        snapshot_hasher.add_u64(self.limits.max_path_changes_per_ref);
        if self.snapshot_id != snapshot_hasher.finish() {
            return Err(GitError::InvalidSnapshot);
        }
        Ok(())
    }

    #[must_use]
    pub fn capability_record(&self) -> CapabilityRecord {
        CapabilityRecord {
            provider_id: "rey.git".to_owned(),
            provider_revision: LOCAL_PROVIDER_REVISION,
            provider_kind: "git_repository".to_owned(),
            capability_id: "git.repository.inspect".to_owned(),
            capability_kind: "context_surface".to_owned(),
            resolved_location: self.worktree_root.as_ref().map_or_else(
                || Some(self.workspace_root.display.clone()),
                |root| Some(root.display.clone()),
            ),
            version: Some(self.object_format.clone()),
            content_digest: Some(self.snapshot_id.to_string()),
            provenance: serde_json::to_string(self).ok(),
            availability: Availability::Available,
            trust_class: TrustClass::ExplicitLocal,
            operations: vec![
                "inspect_head".to_owned(),
                "inspect_index_entries".to_owned(),
                "inspect_index_flags".to_owned(),
                "inspect_recent_commits".to_owned(),
                "inspect_repository".to_owned(),
                "inspect_repository_status".to_owned(),
                "inspect_watched_refs".to_owned(),
            ],
            enforced_limits: vec![
                "capture_bytes".to_owned(),
                "direct_argv".to_owned(),
                "exact_watched_ref_scope".to_owned(),
                "no_replace_objects".to_owned(),
                "path_changes_per_ref".to_owned(),
                "reachable_commits_per_direction".to_owned(),
                "no_optional_locks".to_owned(),
                "semantic_index_flags".to_owned(),
                "wall_timeout".to_owned(),
            ],
            unsupported_limits: vec!["process_sandbox".to_owned()],
            observed_at: None,
            error_code: None,
            error_detail: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GitRefMovement {
    Unchanged,
    Created,
    Deleted,
    FastForward,
    Rewound,
    Rewritten,
    Unknown,
}

impl GitRefMovement {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Created => "created",
            Self::Deleted => "deleted",
            Self::FastForward => "fast_forward",
            Self::Rewound => "rewound",
            Self::Rewritten => "rewritten",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GitActivationEventClass {
    HeadRefChanged,
    RefCreated,
    RefDeleted,
    RefFastForward,
    RefRewound,
    RefRewritten,
    RefUnknown,
    CommitReachableAdded,
    CommitReachableRemoved,
    PathAdded,
    PathDeleted,
    PathModified,
    PathTypeChanged,
    IndexChanged,
    IndexConflicted,
}

impl GitActivationEventClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HeadRefChanged => "head.ref_changed",
            Self::RefCreated => "ref.created",
            Self::RefDeleted => "ref.deleted",
            Self::RefFastForward => "ref.fast_forward",
            Self::RefRewound => "ref.rewound",
            Self::RefRewritten => "ref.rewritten",
            Self::RefUnknown => "ref.unknown",
            Self::CommitReachableAdded => "commit.reachable_added",
            Self::CommitReachableRemoved => "commit.reachable_removed",
            Self::PathAdded => "path.added",
            Self::PathDeleted => "path.deleted",
            Self::PathModified => "path.modified",
            Self::PathTypeChanged => "path.type_changed",
            Self::IndexChanged => "index.changed",
            Self::IndexConflicted => "index.conflicted",
        }
    }

    const fn is_ref_event(self) -> bool {
        matches!(
            self,
            Self::HeadRefChanged
                | Self::RefCreated
                | Self::RefDeleted
                | Self::RefFastForward
                | Self::RefRewound
                | Self::RefRewritten
                | Self::RefUnknown
                | Self::CommitReachableAdded
                | Self::CommitReachableRemoved
                | Self::PathAdded
                | Self::PathDeleted
                | Self::PathModified
                | Self::PathTypeChanged
        )
    }

    const fn is_path_event(self) -> bool {
        matches!(
            self,
            Self::PathAdded | Self::PathDeleted | Self::PathModified | Self::PathTypeChanged
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitPollCursor {
    pub schema: String,
    pub cursor_id: SemanticDigest,
    pub repository_id: SemanticDigest,
    pub worktree_id: Option<SemanticDigest>,
    pub snapshot_id: SemanticDigest,
    pub object_format: String,
    pub shallow: bool,
    pub head: GitHead,
    #[serde(default)]
    pub watched_refs: Vec<GitWatchedRef>,
    pub index_digest: Option<SemanticDigest>,
    pub index_complete: bool,
    pub index_conflicted: bool,
    pub provider_revision: u64,
    pub retained_evidence_id: SemanticDigest,
}

impl GitPollCursor {
    pub fn from_retained_snapshot(
        snapshot: &GitSnapshot,
        retained_evidence_id: SemanticDigest,
    ) -> Result<Self, GitError> {
        snapshot.verify()?;
        if retained_evidence_id != snapshot.snapshot_id {
            return Err(GitError::CursorRetentionMismatch);
        }
        let mut cursor = Self {
            schema: GIT_POLL_CURSOR_SCHEMA.to_owned(),
            cursor_id: SemanticHasher::new("rey.git-poll-cursor.pending.v1").finish(),
            repository_id: snapshot.repository_id.clone(),
            worktree_id: snapshot.worktree_id.clone(),
            snapshot_id: snapshot.snapshot_id.clone(),
            object_format: snapshot.object_format.clone(),
            shallow: snapshot.shallow,
            head: snapshot.head.clone(),
            watched_refs: snapshot.watched_refs.clone(),
            index_digest: snapshot
                .index
                .as_ref()
                .map(|index| index.entry_digest.clone()),
            index_complete: snapshot.index.as_ref().is_none_or(|index| index.complete),
            index_conflicted: snapshot
                .index
                .as_ref()
                .is_some_and(|index| index.entries.iter().any(|entry| entry.stage != 0)),
            provider_revision: LOCAL_PROVIDER_REVISION,
            retained_evidence_id,
        };
        cursor.cursor_id = git_cursor_digest(&cursor);
        cursor.verify()?;
        Ok(cursor)
    }

    pub fn advance(
        &self,
        transition: &GitPollTransition,
        retained_evidence_id: SemanticDigest,
    ) -> Result<Self, GitError> {
        self.verify()?;
        transition.verify()?;
        if transition.source_cursor_id != self.cursor_id
            || retained_evidence_id != transition.transition_id
        {
            return Err(GitError::CursorRetentionMismatch);
        }
        let mut cursor = Self {
            schema: GIT_POLL_CURSOR_SCHEMA.to_owned(),
            cursor_id: SemanticHasher::new("rey.git-poll-cursor.pending.v1").finish(),
            repository_id: transition.repository_id.clone(),
            worktree_id: transition.worktree_id.clone(),
            snapshot_id: transition.target_snapshot_id.clone(),
            object_format: transition.object_format.clone(),
            shallow: transition.target_shallow,
            head: transition.target_head.clone(),
            watched_refs: transition.target_watched_refs.clone(),
            index_digest: transition.target_index_digest.clone(),
            index_complete: transition.target_index_complete,
            index_conflicted: transition.target_index_conflicted,
            provider_revision: LOCAL_PROVIDER_REVISION,
            retained_evidence_id,
        };
        cursor.cursor_id = git_cursor_digest(&cursor);
        cursor.verify()?;
        Ok(cursor)
    }

    pub fn verify(&self) -> Result<(), GitError> {
        if self.schema != GIT_POLL_CURSOR_SCHEMA
            || self.provider_revision != LOCAL_PROVIDER_REVISION
            || !is_semantic_digest(&self.cursor_id)
            || !is_semantic_digest(&self.repository_id)
            || self
                .worktree_id
                .as_ref()
                .is_some_and(|digest| !is_semantic_digest(digest))
            || !is_semantic_digest(&self.snapshot_id)
            || !is_semantic_digest(&self.retained_evidence_id)
            || !matches!(self.object_format.as_str(), "sha1" | "sha256")
            || self.head.symbolic_ref.as_deref().is_some_and(str::is_empty)
            || self
                .index_digest
                .as_ref()
                .is_some_and(|digest| !is_semantic_digest(digest))
            || self.index_digest.is_none() && (!self.index_complete || self.index_conflicted)
            || self
                .head
                .commit_oid
                .as_deref()
                .is_some_and(|oid| !valid_oid(oid, &self.object_format))
            || verify_watched_refs(&self.watched_refs, &self.object_format).is_err()
            || self.cursor_id != git_cursor_digest(self)
        {
            return Err(GitError::InvalidPollCursor);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitPollTransition {
    pub schema: String,
    pub transition_id: SemanticDigest,
    pub source_cursor_id: SemanticDigest,
    pub repository_id: SemanticDigest,
    pub worktree_id: Option<SemanticDigest>,
    pub object_format: String,
    pub source_snapshot_id: SemanticDigest,
    pub target_snapshot_id: SemanticDigest,
    pub source_head: GitHead,
    pub target_head: GitHead,
    pub head_movement: GitRefMovement,
    pub head_complete: bool,
    #[serde(default)]
    pub source_watched_refs: Vec<GitWatchedRef>,
    #[serde(default)]
    pub target_watched_refs: Vec<GitWatchedRef>,
    #[serde(default)]
    pub watched_ref_changes: Vec<GitWatchedRefChange>,
    #[serde(default)]
    pub reachability_deltas: Vec<GitReachabilityDelta>,
    #[serde(default)]
    pub path_deltas: Vec<GitPathDelta>,
    pub source_index_digest: Option<SemanticDigest>,
    pub target_index_digest: Option<SemanticDigest>,
    pub source_index_complete: bool,
    pub target_index_complete: bool,
    pub source_index_conflicted: bool,
    pub target_index_conflicted: bool,
    pub source_shallow: bool,
    pub target_shallow: bool,
    pub events: Vec<GitActivationEventClass>,
    pub omissions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitWatchedRefChange {
    pub ref_name: String,
    pub source_oid: Option<String>,
    pub target_oid: Option<String>,
    pub movement: GitRefMovement,
    pub complete: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitReachabilityDelta {
    pub ref_name: String,
    pub source_oid: Option<String>,
    pub target_oid: Option<String>,
    pub added_commits: Vec<String>,
    pub removed_commits: Vec<String>,
    pub max_commits_per_direction: u64,
    pub complete: bool,
    pub omissions: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GitPathChangeKind {
    Added,
    Deleted,
    Modified,
    TypeChanged,
}

impl GitPathChangeKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Deleted => "deleted",
            Self::Modified => "modified",
            Self::TypeChanged => "type_changed",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitPathChange {
    pub path: PathIdentity,
    pub kind: GitPathChangeKind,
    pub source_mode: Option<String>,
    pub source_oid: Option<String>,
    pub target_mode: Option<String>,
    pub target_oid: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitPathDelta {
    pub ref_name: String,
    pub source_oid: Option<String>,
    pub target_oid: Option<String>,
    pub changes: Vec<GitPathChange>,
    pub max_changes: u64,
    pub complete: bool,
    pub omissions: Vec<String>,
}

impl GitPollTransition {
    fn derive(
        cursor: &GitPollCursor,
        target: &GitSnapshot,
        head_movement: GitRefMovement,
        watched_ref_changes: Vec<GitWatchedRefChange>,
        reachability_deltas: Vec<GitReachabilityDelta>,
        path_deltas: Vec<GitPathDelta>,
    ) -> Result<Self, GitError> {
        cursor.verify()?;
        target.verify()?;
        if cursor.repository_id != target.repository_id
            || cursor.worktree_id != target.worktree_id
            || cursor.object_format != target.object_format
            || cursor
                .watched_refs
                .iter()
                .map(|watched| &watched.name)
                .ne(target.watched_refs.iter().map(|watched| &watched.name))
        {
            return Err(GitError::RepositoryIdentityChanged);
        }
        let head_complete = head_movement != GitRefMovement::Unknown;
        let target_index_digest = target
            .index
            .as_ref()
            .map(|index| index.entry_digest.clone());
        let target_index_complete = target.index.as_ref().is_none_or(|index| index.complete);
        let target_index_conflicted = target
            .index
            .as_ref()
            .is_some_and(|index| index.entries.iter().any(|entry| entry.stage != 0));
        let mut events = Vec::new();
        if cursor.head.symbolic_ref != target.head.symbolic_ref {
            events.push(GitActivationEventClass::HeadRefChanged);
        }
        if cursor.head.commit_oid != target.head.commit_oid {
            events.push(match head_movement {
                GitRefMovement::Created => GitActivationEventClass::RefCreated,
                GitRefMovement::Deleted => GitActivationEventClass::RefDeleted,
                GitRefMovement::FastForward => GitActivationEventClass::RefFastForward,
                GitRefMovement::Rewound => GitActivationEventClass::RefRewound,
                GitRefMovement::Rewritten => GitActivationEventClass::RefRewritten,
                GitRefMovement::Unknown => GitActivationEventClass::RefUnknown,
                GitRefMovement::Unchanged => return Err(GitError::InvalidPollTransition),
            });
        } else if head_movement != GitRefMovement::Unchanged {
            return Err(GitError::InvalidPollTransition);
        }
        for change in &watched_ref_changes {
            events.push(ref_movement_event(change.movement)?);
        }
        for delta in &reachability_deltas {
            if !delta.added_commits.is_empty() {
                events.push(GitActivationEventClass::CommitReachableAdded);
            }
            if !delta.removed_commits.is_empty() {
                events.push(GitActivationEventClass::CommitReachableRemoved);
            }
        }
        for delta in &path_deltas {
            for change in &delta.changes {
                events.push(path_change_event(change.kind));
            }
        }
        if cursor.index_digest != target_index_digest {
            events.push(GitActivationEventClass::IndexChanged);
        }
        if target_index_conflicted
            && (!cursor.index_conflicted || cursor.index_digest != target_index_digest)
        {
            events.push(GitActivationEventClass::IndexConflicted);
        }
        events.sort();
        events.dedup();
        let mut omissions = Vec::new();
        if !head_complete {
            omissions.push(
                "HEAD movement is unknown because bounded history cannot establish ancestry"
                    .to_owned(),
            );
        }
        for change in &watched_ref_changes {
            if !change.complete {
                omissions.push(format!(
                    "watched ref {} movement is unknown because bounded history cannot establish ancestry",
                    change.ref_name
                ));
            }
        }
        for delta in &reachability_deltas {
            omissions.extend(delta.omissions.iter().cloned());
        }
        for delta in &path_deltas {
            omissions.extend(delta.omissions.iter().cloned());
        }
        if !cursor.index_complete || !target_index_complete {
            omissions
                .push("semantic index comparison omits unsupported persistent flags".to_owned());
        }
        omissions.sort();
        omissions.dedup();
        let mut transition = Self {
            schema: GIT_POLL_TRANSITION_SCHEMA.to_owned(),
            transition_id: SemanticHasher::new("rey.git-poll-transition.pending.v1").finish(),
            source_cursor_id: cursor.cursor_id.clone(),
            repository_id: cursor.repository_id.clone(),
            worktree_id: cursor.worktree_id.clone(),
            object_format: cursor.object_format.clone(),
            source_snapshot_id: cursor.snapshot_id.clone(),
            target_snapshot_id: target.snapshot_id.clone(),
            source_head: cursor.head.clone(),
            target_head: target.head.clone(),
            head_movement,
            head_complete,
            source_watched_refs: cursor.watched_refs.clone(),
            target_watched_refs: target.watched_refs.clone(),
            watched_ref_changes,
            reachability_deltas,
            path_deltas,
            source_index_digest: cursor.index_digest.clone(),
            target_index_digest,
            source_index_complete: cursor.index_complete,
            target_index_complete,
            source_index_conflicted: cursor.index_conflicted,
            target_index_conflicted,
            source_shallow: cursor.shallow,
            target_shallow: target.shallow,
            events,
            omissions,
        };
        transition.transition_id = git_transition_digest(&transition);
        transition.verify()?;
        Ok(transition)
    }

    pub fn verify(&self) -> Result<(), GitError> {
        let mut canonical_events = self.events.clone();
        canonical_events.sort();
        canonical_events.dedup();
        let expected_events = expected_transition_events(self)?;
        let mut expected_omissions = Vec::new();
        if !self.head_complete {
            expected_omissions.push(
                "HEAD movement is unknown because bounded history cannot establish ancestry"
                    .to_owned(),
            );
        }
        for change in &self.watched_ref_changes {
            if !change.complete {
                expected_omissions.push(format!(
                    "watched ref {} movement is unknown because bounded history cannot establish ancestry",
                    change.ref_name
                ));
            }
        }
        for delta in &self.reachability_deltas {
            expected_omissions.extend(delta.omissions.iter().cloned());
        }
        for delta in &self.path_deltas {
            expected_omissions.extend(delta.omissions.iter().cloned());
        }
        if !self.source_index_complete || !self.target_index_complete {
            expected_omissions
                .push("semantic index comparison omits unsupported persistent flags".to_owned());
        }
        expected_omissions.sort();
        expected_omissions.dedup();
        if self.schema != GIT_POLL_TRANSITION_SCHEMA
            || !is_semantic_digest(&self.transition_id)
            || !is_semantic_digest(&self.source_cursor_id)
            || !is_semantic_digest(&self.repository_id)
            || !is_semantic_digest(&self.source_snapshot_id)
            || !is_semantic_digest(&self.target_snapshot_id)
            || !matches!(self.object_format.as_str(), "sha1" | "sha256")
            || self
                .source_head
                .symbolic_ref
                .as_deref()
                .is_some_and(str::is_empty)
            || self
                .target_head
                .symbolic_ref
                .as_deref()
                .is_some_and(str::is_empty)
            || self
                .worktree_id
                .as_ref()
                .is_some_and(|digest| !is_semantic_digest(digest))
            || self
                .source_index_digest
                .as_ref()
                .is_some_and(|digest| !is_semantic_digest(digest))
            || self
                .target_index_digest
                .as_ref()
                .is_some_and(|digest| !is_semantic_digest(digest))
            || self
                .source_head
                .commit_oid
                .as_deref()
                .is_some_and(|oid| !valid_oid(oid, &self.object_format))
            || verify_watched_refs(&self.source_watched_refs, &self.object_format).is_err()
            || verify_watched_refs(&self.target_watched_refs, &self.object_format).is_err()
            || verify_watched_ref_changes(self).is_err()
            || verify_reachability_deltas(self).is_err()
            || verify_path_deltas(self).is_err()
            || self
                .target_head
                .commit_oid
                .as_deref()
                .is_some_and(|oid| !valid_oid(oid, &self.object_format))
            || self.source_index_digest.is_none()
                && (!self.source_index_complete || self.source_index_conflicted)
            || self.target_index_digest.is_none()
                && (!self.target_index_complete || self.target_index_conflicted)
            || canonical_events != self.events
            || expected_events != self.events
            || expected_omissions != self.omissions
            || self.head_complete != (self.head_movement != GitRefMovement::Unknown)
            || (self.source_shallow || self.target_shallow)
                && self.source_head.commit_oid != self.target_head.commit_oid
                && self.source_head.commit_oid.is_some()
                && self.target_head.commit_oid.is_some()
                && self.head_movement != GitRefMovement::Unknown
            || self.transition_id != git_transition_digest(self)
        {
            return Err(GitError::InvalidPollTransition);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitActivationBudget {
    pub max_scenarios: u64,
    pub max_actions: u64,
    pub max_evidence_bytes: u64,
}

impl Default for GitActivationBudget {
    fn default() -> Self {
        Self {
            max_scenarios: 64,
            max_actions: 1,
            max_evidence_bytes: 4 * 1_024 * 1_024,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitActivationTrigger {
    pub schema: String,
    pub trigger_id: String,
    pub revision: u64,
    pub repository_id: SemanticDigest,
    pub worktree_id: Option<SemanticDigest>,
    pub event_classes: Vec<GitActivationEventClass>,
    #[serde(default)]
    pub ref_names: Vec<String>,
    #[serde(default)]
    pub path_prefixes: Vec<PathIdentity>,
    pub require_complete: bool,
    pub workload_id: String,
    pub graph: ContractIdentity,
    pub scenario_ids: Vec<String>,
    pub budget: GitActivationBudget,
}

impl GitActivationTrigger {
    pub fn verify(&self) -> Result<(), GitError> {
        if self.schema != GIT_ACTIVATION_TRIGGER_SCHEMA
            || self.trigger_id.trim().is_empty()
            || self.revision == 0
            || !is_semantic_digest(&self.repository_id)
            || self
                .worktree_id
                .as_ref()
                .is_some_and(|digest| !is_semantic_digest(digest))
            || self.event_classes.is_empty()
            || !is_canonical(&self.event_classes)
            || !is_canonical(&self.ref_names)
            || self.ref_names.len() > MAX_GIT_WATCHED_REFS + 1
            || self
                .ref_names
                .iter()
                .any(|name| name != "HEAD" && !valid_full_ref_name(name))
            || verify_path_prefixes(&self.path_prefixes).is_err()
            || !self.path_prefixes.is_empty()
                && !self.event_classes.iter().any(|event| event.is_path_event())
            || self.workload_id.trim().is_empty()
            || self.graph.id.trim().is_empty()
            || self.graph.revision == 0
            || !is_semantic_digest(&self.graph.semantic_digest)
            || !is_canonical(&self.scenario_ids)
            || self
                .scenario_ids
                .iter()
                .any(|scenario| scenario.trim().is_empty())
            || self.scenario_ids.len() > MAX_GIT_ACTIVATION_SCENARIOS
            || self.scenario_ids.len() as u64 > self.budget.max_scenarios
            || self.budget.max_scenarios == 0
            || self.budget.max_actions == 0
            || self.budget.max_evidence_bytes == 0
        {
            return Err(GitError::InvalidActivationTrigger);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitActivationProposal {
    pub schema: String,
    pub activation_id: SemanticDigest,
    pub trigger_id: String,
    pub trigger_revision: u64,
    pub transition_id: SemanticDigest,
    pub source_snapshot_id: SemanticDigest,
    pub target_snapshot_id: SemanticDigest,
    pub matched_events: Vec<GitActivationEventClass>,
    #[serde(default)]
    pub matched_ref_names: Vec<String>,
    #[serde(default)]
    pub matched_path_changes: Vec<GitMatchedPathChange>,
    pub complete: bool,
    pub omissions: Vec<String>,
    pub workload_id: String,
    pub graph: ContractIdentity,
    pub scenario_ids: Vec<String>,
    pub budget: GitActivationBudget,
    pub authority: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitMatchedPathChange {
    pub ref_name: String,
    pub path: PathIdentity,
    pub kind: GitPathChangeKind,
}

impl GitActivationProposal {
    fn derive(
        trigger: &GitActivationTrigger,
        transition: &GitPollTransition,
    ) -> Result<Option<Self>, GitError> {
        trigger.verify()?;
        transition.verify()?;
        if trigger.repository_id != transition.repository_id
            || trigger
                .worktree_id
                .as_ref()
                .is_some_and(|worktree| Some(worktree) != transition.worktree_id.as_ref())
        {
            return Ok(None);
        }
        let selects_ref = |name: &str| {
            trigger.ref_names.is_empty()
                || trigger
                    .ref_names
                    .binary_search_by(|candidate| candidate.as_str().cmp(name))
                    .is_ok()
        };
        let path_prefixes = trigger_path_prefix_bytes(trigger)?;
        let selects_path = |path: &[u8]| {
            path_prefixes.is_empty() || path_prefixes.iter().any(|prefix| path.starts_with(prefix))
        };
        let mut matched_events = BTreeSet::new();
        let mut matched_ref_names = BTreeSet::new();
        let mut matched_path_changes = BTreeMap::new();
        let mut omissions = BTreeSet::new();
        let mut complete = true;
        let head_omission =
            "HEAD movement is unknown because bounded history cannot establish ancestry";
        if transition.source_head.symbolic_ref != transition.target_head.symbolic_ref
            && trigger
                .event_classes
                .contains(&GitActivationEventClass::HeadRefChanged)
            && selects_ref("HEAD")
        {
            matched_events.insert(GitActivationEventClass::HeadRefChanged);
            matched_ref_names.insert("HEAD".to_owned());
            if !transition.head_complete {
                complete = false;
                omissions.insert(head_omission.to_owned());
            }
        }
        if transition.source_head.commit_oid != transition.target_head.commit_oid {
            let event = ref_movement_event(transition.head_movement)?;
            if trigger.event_classes.contains(&event) && selects_ref("HEAD") {
                matched_events.insert(event);
                matched_ref_names.insert("HEAD".to_owned());
                if !transition.head_complete {
                    complete = false;
                    omissions.insert(head_omission.to_owned());
                }
            }
        }
        for change in &transition.watched_ref_changes {
            let event = ref_movement_event(change.movement)?;
            if trigger.event_classes.contains(&event) && selects_ref(&change.ref_name) {
                matched_events.insert(event);
                matched_ref_names.insert(change.ref_name.clone());
                if !change.complete {
                    complete = false;
                    omissions.insert(format!(
                        "watched ref {} movement is unknown because bounded history cannot establish ancestry",
                        change.ref_name
                    ));
                }
            }
        }
        for delta in &transition.reachability_deltas {
            for (event, commits) in [
                (
                    GitActivationEventClass::CommitReachableAdded,
                    &delta.added_commits,
                ),
                (
                    GitActivationEventClass::CommitReachableRemoved,
                    &delta.removed_commits,
                ),
            ] {
                if !commits.is_empty()
                    && trigger.event_classes.contains(&event)
                    && selects_ref(&delta.ref_name)
                {
                    matched_events.insert(event);
                    matched_ref_names.insert(delta.ref_name.clone());
                    if !delta.complete {
                        complete = false;
                        omissions.extend(delta.omissions.iter().cloned());
                    }
                }
            }
        }
        for delta in &transition.path_deltas {
            if !selects_ref(&delta.ref_name) {
                continue;
            }
            for change in &delta.changes {
                let path = change.path.decoded_bytes()?;
                let event = path_change_event(change.kind);
                if trigger.event_classes.contains(&event) && selects_path(&path) {
                    matched_events.insert(event);
                    matched_ref_names.insert(delta.ref_name.clone());
                    matched_path_changes.insert(
                        (delta.ref_name.clone(), path, change.kind),
                        GitMatchedPathChange {
                            ref_name: delta.ref_name.clone(),
                            path: change.path.clone(),
                            kind: change.kind,
                        },
                    );
                    if matched_path_changes.len() > MAX_GIT_MATCHED_PATH_CHANGES {
                        return Err(GitError::ActivationPathMatchLimit(
                            MAX_GIT_MATCHED_PATH_CHANGES,
                        ));
                    }
                    if !delta.complete {
                        complete = false;
                        omissions.extend(delta.omissions.iter().cloned());
                    }
                }
            }
        }
        for event in [
            GitActivationEventClass::IndexChanged,
            GitActivationEventClass::IndexConflicted,
        ] {
            if transition.events.contains(&event) && trigger.event_classes.contains(&event) {
                matched_events.insert(event);
                if !transition.source_index_complete || !transition.target_index_complete {
                    complete = false;
                    omissions.insert(
                        "semantic index comparison omits unsupported persistent flags".to_owned(),
                    );
                }
            }
        }
        if matched_events.is_empty() {
            return Ok(None);
        }
        if trigger.require_complete && !complete {
            return Ok(None);
        }
        let matched_events = matched_events.into_iter().collect::<Vec<_>>();
        let matched_ref_names = matched_ref_names.into_iter().collect::<Vec<_>>();
        let matched_path_changes = matched_path_changes.into_values().collect::<Vec<_>>();
        let omissions = omissions.into_iter().collect::<Vec<_>>();
        let mut proposal = Self {
            schema: GIT_ACTIVATION_PROPOSAL_SCHEMA.to_owned(),
            activation_id: SemanticHasher::new("rey.git-activation-proposal.pending.v1").finish(),
            trigger_id: trigger.trigger_id.clone(),
            trigger_revision: trigger.revision,
            transition_id: transition.transition_id.clone(),
            source_snapshot_id: transition.source_snapshot_id.clone(),
            target_snapshot_id: transition.target_snapshot_id.clone(),
            matched_events,
            matched_ref_names,
            matched_path_changes,
            complete,
            omissions,
            workload_id: trigger.workload_id.clone(),
            graph: trigger.graph.clone(),
            scenario_ids: trigger.scenario_ids.clone(),
            budget: trigger.budget.clone(),
            authority:
                "proposal_only; normal workload admission and runtime preconditions still apply"
                    .to_owned(),
        };
        proposal.activation_id = git_activation_digest(&proposal);
        proposal.verify()?;
        Ok(Some(proposal))
    }

    pub fn verify(&self) -> Result<(), GitError> {
        if self.schema != GIT_ACTIVATION_PROPOSAL_SCHEMA
            || !is_semantic_digest(&self.activation_id)
            || !is_semantic_digest(&self.transition_id)
            || !is_semantic_digest(&self.source_snapshot_id)
            || !is_semantic_digest(&self.target_snapshot_id)
            || self.trigger_id.trim().is_empty()
            || self.trigger_revision == 0
            || self.matched_events.is_empty()
            || !is_canonical(&self.matched_events)
            || !is_canonical(&self.matched_ref_names)
            || self.matched_ref_names.len() > MAX_GIT_WATCHED_REFS + 1
            || self
                .matched_ref_names
                .iter()
                .any(|name| name != "HEAD" && !valid_full_ref_name(name))
            || verify_matched_path_changes(&self.matched_path_changes).is_err()
            || self.matched_path_changes.iter().any(|change| {
                !self.matched_ref_names.contains(&change.ref_name)
                    || !self
                        .matched_events
                        .contains(&path_change_event(change.kind))
            })
            || self
                .matched_events
                .iter()
                .filter(|event| event.is_path_event())
                .any(|event| {
                    !self
                        .matched_path_changes
                        .iter()
                        .any(|change| path_change_event(change.kind) == *event)
                })
            || (self.matched_events.iter().any(|event| event.is_ref_event())
                != !self.matched_ref_names.is_empty())
            || (self
                .matched_events
                .iter()
                .any(|event| event.is_path_event())
                != !self.matched_path_changes.is_empty())
            || self.complete != self.omissions.is_empty()
            || !is_canonical(&self.omissions)
            || self.workload_id.trim().is_empty()
            || self.graph.id.trim().is_empty()
            || self.graph.revision == 0
            || !is_semantic_digest(&self.graph.semantic_digest)
            || !is_canonical(&self.scenario_ids)
            || self
                .scenario_ids
                .iter()
                .any(|scenario| scenario.trim().is_empty())
            || self.scenario_ids.len() as u64 > self.budget.max_scenarios
            || self.budget.max_scenarios == 0
            || self.budget.max_actions == 0
            || self.budget.max_evidence_bytes == 0
            || self.authority
                != "proposal_only; normal workload admission and runtime preconditions still apply"
            || self.activation_id != git_activation_digest(self)
        {
            return Err(GitError::InvalidActivationProposal);
        }
        Ok(())
    }
}

pub fn derive_activation_proposals(
    transition: &GitPollTransition,
    triggers: &[GitActivationTrigger],
) -> Result<Vec<GitActivationProposal>, GitError> {
    if triggers.len() > MAX_GIT_ACTIVATION_TRIGGERS {
        return Err(GitError::ActivationTriggerLimit(
            MAX_GIT_ACTIVATION_TRIGGERS,
        ));
    }
    let mut proposals = triggers
        .iter()
        .map(|trigger| GitActivationProposal::derive(trigger, transition))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    proposals.sort_by(|left, right| left.activation_id.cmp(&right.activation_id));
    if proposals
        .windows(2)
        .any(|window| window[0].activation_id == window[1].activation_id)
    {
        return Err(GitError::DuplicateActivationProposal);
    }
    Ok(proposals)
}

#[derive(Clone, Debug)]
pub struct GitInspector {
    pub git_program: PathBuf,
    pub workspace: PathBuf,
    pub limits: GitLimits,
}

enum ReachableSetRead {
    Complete(Vec<String>),
    Truncated(Vec<String>),
    Unavailable,
}

enum PathChangeRead {
    Complete(Vec<GitPathChange>),
    Truncated(Vec<GitPathChange>),
    Unavailable,
}

impl GitInspector {
    pub fn inspect(&self) -> Result<Option<GitSnapshot>, GitError> {
        self.inspect_with_watched_refs(&[])
    }

    pub fn inspect_with_watched_refs(
        &self,
        watched_ref_names: &[String],
    ) -> Result<Option<GitSnapshot>, GitError> {
        self.inspect_until_with_watched_refs(
            Instant::now() + Duration::from_millis(self.limits.total_timeout_ms),
            watched_ref_names,
        )
    }

    pub fn inspect_until(&self, outer_deadline: Instant) -> Result<Option<GitSnapshot>, GitError> {
        self.inspect_until_with_watched_refs(outer_deadline, &[])
    }

    fn inspect_until_with_watched_refs(
        &self,
        outer_deadline: Instant,
        watched_ref_names: &[String],
    ) -> Result<Option<GitSnapshot>, GitError> {
        let local_deadline = Instant::now() + Duration::from_millis(self.limits.total_timeout_ms);
        let deadline = local_deadline.min(outer_deadline);
        let workspace = fs::canonicalize(&self.workspace).map_err(|source| GitError::Path {
            path: self.workspace.clone(),
            source,
        })?;
        let git_directory_output =
            self.git(&workspace, &["rev-parse", "--absolute-git-dir"], deadline)?;
        if !git_directory_output.status.success() {
            return Ok(None);
        }
        let git_directory = canonical_output_path(&git_directory_output.stdout)?;
        let bare = self.git_bool(&workspace, &["rev-parse", "--is-bare-repository"], deadline)?;
        let common_directory = canonical_output_path(
            &self
                .git(
                    &workspace,
                    &["rev-parse", "--path-format=absolute", "--git-common-dir"],
                    deadline,
                )?
                .success("resolve Git common directory")?,
        )?;
        let worktree_root = if bare {
            None
        } else {
            Some(canonical_output_path(
                &self
                    .git(&workspace, &["rev-parse", "--show-toplevel"], deadline)?
                    .success("resolve Git worktree root")?,
            )?)
        };
        ensure_beneath(&git_directory, &workspace)?;
        ensure_beneath(&common_directory, &workspace)?;
        if let Some(root) = &worktree_root {
            ensure_beneath(root, &workspace)?;
        }

        let object_format =
            self.git_line(&workspace, &["rev-parse", "--show-object-format"], deadline)?;
        let shallow = self.git_bool(
            &workspace,
            &["rev-parse", "--is-shallow-repository"],
            deadline,
        )?;
        let symbolic = self.git(&workspace, &["symbolic-ref", "-q", "HEAD"], deadline)?;
        let symbolic_ref = if symbolic.status.success() {
            Some(parse_line(&symbolic.stdout)?.to_owned())
        } else {
            None
        };
        let head_oid = self.git(
            &workspace,
            &["rev-parse", "--verify", "HEAD^{commit}"],
            deadline,
        )?;
        let commit_oid = if head_oid.status.success() {
            Some(parse_line(&head_oid.stdout)?.to_owned())
        } else {
            None
        };
        let head = GitHead {
            symbolic_ref,
            commit_oid,
        };
        let watched_refs =
            self.inspect_watched_refs(&workspace, &object_format, watched_ref_names, deadline)?;
        let index = if bare {
            None
        } else {
            Some(self.inspect_index(&workspace, &object_format, deadline)?)
        };

        let workspace_identity = PathIdentity::canonical(&workspace)?;
        let common_identity = PathIdentity::canonical(&common_directory)?;
        let git_identity = PathIdentity::canonical(&git_directory)?;
        let worktree_identity = worktree_root
            .as_deref()
            .map(PathIdentity::canonical)
            .transpose()?;
        let mut repository_hasher = SemanticHasher::new("rey.git-repository-id.v1");
        common_identity.add_semantics(&mut repository_hasher);
        repository_hasher.add_str(&object_format);
        let repository_id = repository_hasher.finish();
        let worktree_id = worktree_identity.as_ref().map(|root| {
            let mut hasher = SemanticHasher::new("rey.git-worktree-id.v1");
            root.add_semantics(&mut hasher);
            git_identity.add_semantics(&mut hasher);
            hasher.finish()
        });
        let complete = index.as_ref().is_none_or(|summary| summary.complete);
        let mut snapshot_hasher = SemanticHasher::new("rey.git-snapshot.v1");
        snapshot_hasher.add_str(repository_id.as_str());
        snapshot_hasher.add_optional_str(worktree_id.as_ref().map(SemanticDigest::as_str));
        snapshot_hasher.add_str(&object_format);
        snapshot_hasher.add_bool(bare);
        snapshot_hasher.add_bool(shallow);
        snapshot_hasher.add_optional_str(head.symbolic_ref.as_deref());
        snapshot_hasher.add_optional_str(head.commit_oid.as_deref());
        add_watched_refs(&mut snapshot_hasher, &watched_refs);
        snapshot_hasher
            .add_optional_str(index.as_ref().map(|summary| summary.entry_digest.as_str()));
        snapshot_hasher.add_bool(complete);
        snapshot_hasher.add_u64(self.limits.total_timeout_ms);
        snapshot_hasher.add_u64(self.limits.command_timeout_ms);
        snapshot_hasher.add_u64(self.limits.max_capture_bytes);
        snapshot_hasher.add_u64(self.limits.max_index_entries);
        snapshot_hasher.add_u64(self.limits.max_reachable_commits_per_direction);
        snapshot_hasher.add_u64(self.limits.max_path_changes_per_ref);

        let snapshot = GitSnapshot {
            schema: GIT_SNAPSHOT_SCHEMA.to_owned(),
            snapshot_id: snapshot_hasher.finish(),
            repository_id,
            worktree_id,
            workspace_root: workspace_identity,
            common_directory: common_identity,
            git_directory: git_identity,
            worktree_root: worktree_identity,
            object_format,
            bare,
            shallow,
            head,
            watched_refs,
            index,
            complete,
            limits: self.limits.clone(),
        };
        snapshot.verify()?;
        Ok(Some(snapshot))
    }

    pub fn inspect_transition(
        &self,
        cursor: &GitPollCursor,
    ) -> Result<Option<(GitSnapshot, GitPollTransition)>, GitError> {
        cursor.verify()?;
        let deadline = Instant::now() + Duration::from_millis(self.limits.total_timeout_ms);
        let watched_ref_names = cursor
            .watched_refs
            .iter()
            .map(|watched| watched.name.clone())
            .collect::<Vec<_>>();
        let Some(target) = self.inspect_until_with_watched_refs(deadline, &watched_ref_names)?
        else {
            return Ok(None);
        };
        if cursor.repository_id != target.repository_id
            || cursor.worktree_id != target.worktree_id
            || cursor.object_format != target.object_format
        {
            return Err(GitError::RepositoryIdentityChanged);
        }
        let movement = self.classify_head_movement(cursor, &target, deadline)?;
        let watched_ref_changes = self.classify_watched_ref_changes(cursor, &target, deadline)?;
        let reachability_deltas = self.inspect_reachability_deltas(cursor, &target, deadline)?;
        let path_deltas = self.inspect_path_deltas(cursor, &target, deadline)?;
        let transition = GitPollTransition::derive(
            cursor,
            &target,
            movement,
            watched_ref_changes,
            reachability_deltas,
            path_deltas,
        )?;
        Ok(Some((target, transition)))
    }

    fn inspect_watched_refs(
        &self,
        workspace: &Path,
        object_format: &str,
        watched_ref_names: &[String],
        deadline: Instant,
    ) -> Result<Vec<GitWatchedRef>, GitError> {
        if watched_ref_names.len() > MAX_GIT_WATCHED_REFS
            || watched_ref_names
                .iter()
                .any(|name| !valid_full_ref_name(name))
        {
            return Err(GitError::InvalidWatchedRefScope);
        }
        let mut names = watched_ref_names.to_vec();
        names.sort();
        if names.windows(2).any(|window| window[0] == window[1]) {
            return Err(GitError::InvalidWatchedRefScope);
        }
        if names.is_empty() {
            return Ok(Vec::new());
        }
        let mut owned_args = vec![
            "for-each-ref".to_owned(),
            "--format=%(refname)%00%(objectname)".to_owned(),
        ];
        owned_args.extend(names.iter().cloned());
        let args = owned_args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self
            .git(workspace, &args, deadline)?
            .success("read exact watched Git refs")?;
        let mut observed = BTreeMap::new();
        for line in output.split(|byte| *byte == b'\n') {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            if line.is_empty() {
                continue;
            }
            let mut fields = line.split(|byte| *byte == 0);
            let name = fields
                .next()
                .and_then(|field| std::str::from_utf8(field).ok())
                .ok_or(GitError::MalformedRef("ref name is not UTF-8"))?;
            let oid = fields
                .next()
                .and_then(|field| std::str::from_utf8(field).ok())
                .ok_or(GitError::MalformedRef("ref target is not ASCII"))?;
            if fields.next().is_some() || !valid_oid(oid, object_format) {
                return Err(GitError::MalformedRef("ref record has invalid fields"));
            }
            if names
                .binary_search_by(|candidate| candidate.as_str().cmp(name))
                .is_ok()
                && observed.insert(name.to_owned(), oid.to_owned()).is_some()
            {
                return Err(GitError::MalformedRef("duplicate watched ref record"));
            }
        }
        Ok(names
            .into_iter()
            .map(|name| GitWatchedRef {
                target_oid: observed.remove(&name),
                name,
            })
            .collect())
    }

    fn classify_watched_ref_changes(
        &self,
        cursor: &GitPollCursor,
        target_snapshot: &GitSnapshot,
        deadline: Instant,
    ) -> Result<Vec<GitWatchedRefChange>, GitError> {
        let workspace = fs::canonicalize(&self.workspace).map_err(|source| GitError::Path {
            path: self.workspace.clone(),
            source,
        })?;
        cursor
            .watched_refs
            .iter()
            .zip(&target_snapshot.watched_refs)
            .filter(|(source, target_ref)| source.target_oid != target_ref.target_oid)
            .map(|(source, target_ref)| {
                if source.name != target_ref.name {
                    return Err(GitError::RepositoryIdentityChanged);
                }
                let movement = self.classify_oid_movement(
                    &workspace,
                    source.target_oid.as_deref(),
                    target_ref.target_oid.as_deref(),
                    cursor.shallow || target_snapshot.shallow,
                    deadline,
                )?;
                Ok(GitWatchedRefChange {
                    ref_name: source.name.clone(),
                    source_oid: source.target_oid.clone(),
                    target_oid: target_ref.target_oid.clone(),
                    movement,
                    complete: movement != GitRefMovement::Unknown,
                })
            })
            .collect()
    }

    fn inspect_reachability_deltas(
        &self,
        cursor: &GitPollCursor,
        target_snapshot: &GitSnapshot,
        deadline: Instant,
    ) -> Result<Vec<GitReachabilityDelta>, GitError> {
        let workspace = fs::canonicalize(&self.workspace).map_err(|source| GitError::Path {
            path: self.workspace.clone(),
            source,
        })?;
        let mut endpoints = Vec::new();
        if cursor.head.commit_oid != target_snapshot.head.commit_oid {
            endpoints.push((
                "HEAD".to_owned(),
                cursor.head.commit_oid.clone(),
                target_snapshot.head.commit_oid.clone(),
            ));
        }
        endpoints.extend(
            cursor
                .watched_refs
                .iter()
                .zip(&target_snapshot.watched_refs)
                .filter(|(source, target)| source.target_oid != target.target_oid)
                .map(|(source, target)| {
                    (
                        source.name.clone(),
                        source.target_oid.clone(),
                        target.target_oid.clone(),
                    )
                }),
        );
        endpoints
            .into_iter()
            .map(|(ref_name, source_oid, target_oid)| {
                self.inspect_reachability_delta(
                    &workspace,
                    &target_snapshot.object_format,
                    ref_name,
                    source_oid,
                    target_oid,
                    cursor.shallow || target_snapshot.shallow,
                    deadline,
                )
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn inspect_reachability_delta(
        &self,
        workspace: &Path,
        object_format: &str,
        ref_name: String,
        source_oid: Option<String>,
        target_oid: Option<String>,
        shallow: bool,
        deadline: Instant,
    ) -> Result<GitReachabilityDelta, GitError> {
        let max_commits = self.limits.max_reachable_commits_per_direction;
        let added = self.read_reachable_difference(
            workspace,
            object_format,
            target_oid.as_deref(),
            source_oid.as_deref(),
            max_commits,
            deadline,
        )?;
        let removed = self.read_reachable_difference(
            workspace,
            object_format,
            source_oid.as_deref(),
            target_oid.as_deref(),
            max_commits,
            deadline,
        )?;
        let mut omissions = Vec::new();
        if shallow {
            omissions.push(format!(
                "{ref_name} reachability is incomplete because repository history is shallow"
            ));
        }
        let added_commits = match added {
            ReachableSetRead::Complete(commits) => commits,
            ReachableSetRead::Truncated(commits) => {
                omissions.push(format!(
                    "{ref_name} added reachability exceeds the {max_commits}-commit per-direction limit"
                ));
                commits
            }
            ReachableSetRead::Unavailable => {
                omissions.push(format!(
                    "{ref_name} added reachability is unavailable because bounded Git history cannot resolve the comparison"
                ));
                Vec::new()
            }
        };
        let removed_commits = match removed {
            ReachableSetRead::Complete(commits) => commits,
            ReachableSetRead::Truncated(commits) => {
                omissions.push(format!(
                    "{ref_name} removed reachability exceeds the {max_commits}-commit per-direction limit"
                ));
                commits
            }
            ReachableSetRead::Unavailable => {
                omissions.push(format!(
                    "{ref_name} removed reachability is unavailable because bounded Git history cannot resolve the comparison"
                ));
                Vec::new()
            }
        };
        omissions.sort();
        omissions.dedup();
        Ok(GitReachabilityDelta {
            ref_name,
            source_oid,
            target_oid,
            added_commits,
            removed_commits,
            max_commits_per_direction: max_commits,
            complete: omissions.is_empty(),
            omissions,
        })
    }

    fn read_reachable_difference(
        &self,
        workspace: &Path,
        object_format: &str,
        include_oid: Option<&str>,
        exclude_oid: Option<&str>,
        max_commits: u64,
        deadline: Instant,
    ) -> Result<ReachableSetRead, GitError> {
        let Some(include_oid) = include_oid else {
            return Ok(ReachableSetRead::Complete(Vec::new()));
        };
        let mut owned_args = vec![
            "rev-list".to_owned(),
            "--topo-order".to_owned(),
            format!("--max-count={}", max_commits + 1),
            include_oid.to_owned(),
        ];
        if let Some(exclude_oid) = exclude_oid {
            owned_args.push(format!("^{exclude_oid}"));
        }
        owned_args.push("--".to_owned());
        let args = owned_args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.git(workspace, &args, deadline)?;
        if !output.status.success() {
            return match output.status.code() {
                Some(128) => Ok(ReachableSetRead::Unavailable),
                status => Err(GitError::Command {
                    operation: "derive bounded Git reachability",
                    status,
                    stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                }),
            };
        }
        let text = std::str::from_utf8(&output.stdout)
            .map_err(|_| GitError::MalformedReachability("commit ids are not ASCII"))?;
        let mut commits = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if commits
            .iter()
            .any(|commit| !valid_oid(commit, object_format))
        {
            return Err(GitError::MalformedReachability(
                "commit id has an invalid object format",
            ));
        }
        let truncated = commits.len() as u64 > max_commits;
        commits.truncate(max_commits as usize);
        commits.sort();
        if commits.windows(2).any(|window| window[0] == window[1]) {
            return Err(GitError::MalformedReachability(
                "duplicate commit id in reachability set",
            ));
        }
        Ok(if truncated {
            ReachableSetRead::Truncated(commits)
        } else {
            ReachableSetRead::Complete(commits)
        })
    }

    fn inspect_path_deltas(
        &self,
        cursor: &GitPollCursor,
        target_snapshot: &GitSnapshot,
        deadline: Instant,
    ) -> Result<Vec<GitPathDelta>, GitError> {
        let workspace = fs::canonicalize(&self.workspace).map_err(|source| GitError::Path {
            path: self.workspace.clone(),
            source,
        })?;
        let mut endpoints = Vec::new();
        if cursor.head.commit_oid != target_snapshot.head.commit_oid {
            endpoints.push((
                "HEAD".to_owned(),
                cursor.head.commit_oid.clone(),
                target_snapshot.head.commit_oid.clone(),
            ));
        }
        endpoints.extend(
            cursor
                .watched_refs
                .iter()
                .zip(&target_snapshot.watched_refs)
                .filter(|(source, target)| source.target_oid != target.target_oid)
                .map(|(source, target)| {
                    (
                        source.name.clone(),
                        source.target_oid.clone(),
                        target.target_oid.clone(),
                    )
                }),
        );
        endpoints
            .into_iter()
            .map(|(ref_name, source_oid, target_oid)| {
                self.inspect_path_delta(
                    &workspace,
                    &target_snapshot.object_format,
                    ref_name,
                    source_oid,
                    target_oid,
                    deadline,
                )
            })
            .collect()
    }

    fn inspect_path_delta(
        &self,
        workspace: &Path,
        object_format: &str,
        ref_name: String,
        source_oid: Option<String>,
        target_oid: Option<String>,
        deadline: Instant,
    ) -> Result<GitPathDelta, GitError> {
        let max_changes = self.limits.max_path_changes_per_ref;
        let read = match (source_oid.as_deref(), target_oid.as_deref()) {
            (Some(source), Some(target)) => self.read_tree_diff_paths(
                workspace,
                object_format,
                source,
                target,
                max_changes,
                deadline,
            )?,
            (None, Some(target)) => self.read_tree_inventory_paths(
                workspace,
                object_format,
                target,
                GitPathChangeKind::Added,
                max_changes,
                deadline,
            )?,
            (Some(source), None) => self.read_tree_inventory_paths(
                workspace,
                object_format,
                source,
                GitPathChangeKind::Deleted,
                max_changes,
                deadline,
            )?,
            (None, None) => return Err(GitError::InvalidPollTransition),
        };
        let (changes, omissions) = match read {
            PathChangeRead::Complete(changes) => (changes, Vec::new()),
            PathChangeRead::Truncated(changes) => (
                changes,
                vec![format!(
                    "{ref_name} path changes exceed the {max_changes}-change limit"
                )],
            ),
            PathChangeRead::Unavailable => (
                Vec::new(),
                vec![format!(
                    "{ref_name} path delta is unavailable because bounded Git trees cannot resolve the comparison"
                )],
            ),
        };
        Ok(GitPathDelta {
            ref_name,
            source_oid,
            target_oid,
            changes,
            max_changes,
            complete: omissions.is_empty(),
            omissions,
        })
    }

    fn read_tree_diff_paths(
        &self,
        workspace: &Path,
        object_format: &str,
        source_oid: &str,
        target_oid: &str,
        max_changes: u64,
        deadline: Instant,
    ) -> Result<PathChangeRead, GitError> {
        let output = self.git(
            workspace,
            &[
                "diff-tree",
                "--raw",
                "-z",
                "-r",
                "--full-index",
                "--no-renames",
                "--no-commit-id",
                source_oid,
                target_oid,
                "--",
            ],
            deadline,
        )?;
        if !output.status.success() {
            return match output.status.code() {
                Some(128) => Ok(PathChangeRead::Unavailable),
                status => Err(GitError::Command {
                    operation: "derive bounded Git tree path delta",
                    status,
                    stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                }),
            };
        }
        bounded_path_changes(
            parse_raw_path_changes(&output.stdout, object_format)?,
            max_changes,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn read_tree_inventory_paths(
        &self,
        workspace: &Path,
        object_format: &str,
        oid: &str,
        kind: GitPathChangeKind,
        max_changes: u64,
        deadline: Instant,
    ) -> Result<PathChangeRead, GitError> {
        let output = self.git(
            workspace,
            &["ls-tree", "-r", "-z", "--full-tree", oid, "--"],
            deadline,
        )?;
        if !output.status.success() {
            return match output.status.code() {
                Some(128) => Ok(PathChangeRead::Unavailable),
                status => Err(GitError::Command {
                    operation: "derive bounded Git tree inventory",
                    status,
                    stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                }),
            };
        }
        bounded_path_changes(
            parse_tree_inventory_changes(&output.stdout, object_format, kind)?,
            max_changes,
        )
    }

    fn classify_head_movement(
        &self,
        cursor: &GitPollCursor,
        target: &GitSnapshot,
        deadline: Instant,
    ) -> Result<GitRefMovement, GitError> {
        let workspace = fs::canonicalize(&self.workspace).map_err(|source| GitError::Path {
            path: self.workspace.clone(),
            source,
        })?;
        self.classify_oid_movement(
            &workspace,
            cursor.head.commit_oid.as_deref(),
            target.head.commit_oid.as_deref(),
            cursor.shallow || target.shallow,
            deadline,
        )
    }

    fn classify_oid_movement(
        &self,
        workspace: &Path,
        source_oid: Option<&str>,
        target_oid: Option<&str>,
        incomplete_history: bool,
        deadline: Instant,
    ) -> Result<GitRefMovement, GitError> {
        let (Some(source_oid), Some(target_oid)) = (source_oid, target_oid) else {
            return Ok(match (source_oid.is_some(), target_oid.is_some()) {
                (false, false) => GitRefMovement::Unchanged,
                (false, true) => GitRefMovement::Created,
                (true, false) => GitRefMovement::Deleted,
                (true, true) => unreachable!("both object ids were destructured above"),
            });
        };
        if source_oid == target_oid {
            return Ok(GitRefMovement::Unchanged);
        }
        if incomplete_history {
            return Ok(GitRefMovement::Unknown);
        }
        match self.is_ancestor(workspace, source_oid, target_oid, deadline)? {
            Some(true) => Ok(GitRefMovement::FastForward),
            None => Ok(GitRefMovement::Unknown),
            Some(false) => match self.is_ancestor(workspace, target_oid, source_oid, deadline)? {
                Some(true) => Ok(GitRefMovement::Rewound),
                Some(false) => Ok(GitRefMovement::Rewritten),
                None => Ok(GitRefMovement::Unknown),
            },
        }
    }

    fn is_ancestor(
        &self,
        workspace: &Path,
        ancestor: &str,
        descendant: &str,
        deadline: Instant,
    ) -> Result<Option<bool>, GitError> {
        let output = self.git(
            workspace,
            &["merge-base", "--is-ancestor", ancestor, descendant],
            deadline,
        )?;
        match output.status.code() {
            Some(0) => Ok(Some(true)),
            Some(1) => Ok(Some(false)),
            Some(128) => Ok(None),
            status => Err(GitError::Command {
                operation: "classify Git HEAD movement",
                status,
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            }),
        }
    }

    pub fn inspect_recent_commits(
        &self,
        max_commits: usize,
    ) -> Result<Option<GitCommitSequence>, GitError> {
        if max_commits == 0 || max_commits > MAX_GIT_COMMIT_SEQUENCE {
            return Err(GitError::HistoryLimit {
                actual: max_commits,
                limit: MAX_GIT_COMMIT_SEQUENCE,
            });
        }
        let deadline = Instant::now() + Duration::from_millis(self.limits.total_timeout_ms);
        let workspace = fs::canonicalize(&self.workspace).map_err(|source| GitError::Path {
            path: self.workspace.clone(),
            source,
        })?;
        let git_directory_output =
            self.git(&workspace, &["rev-parse", "--absolute-git-dir"], deadline)?;
        if !git_directory_output.status.success() {
            return Ok(None);
        }
        let git_directory = canonical_output_path(&git_directory_output.stdout)?;
        ensure_beneath(&git_directory, &workspace)?;
        let object_format =
            self.git_line(&workspace, &["rev-parse", "--show-object-format"], deadline)?;
        let shallow = self.git_bool(
            &workspace,
            &["rev-parse", "--is-shallow-repository"],
            deadline,
        )?;
        let head = self.git(
            &workspace,
            &["rev-parse", "--verify", "HEAD^{commit}"],
            deadline,
        )?;
        if !head.status.success() {
            let complete = !shallow;
            let mut hasher = SemanticHasher::new(GIT_COMMIT_SEQUENCE_SCHEMA);
            hasher.add_str(&object_format);
            hasher.add_bool(shallow);
            hasher.add_bool(complete);
            hasher.add_u64(max_commits as u64);
            return Ok(Some(GitCommitSequence {
                schema: GIT_COMMIT_SEQUENCE_SCHEMA.to_owned(),
                sequence_id: hasher.finish(),
                object_format,
                head_oid: None,
                commits: Vec::new(),
                complete,
                shallow,
                max_commits: max_commits as u64,
                omissions: if shallow {
                    vec!["repository history is shallow".to_owned()]
                } else {
                    Vec::new()
                },
            }));
        }
        let bounded_count = max_commits.saturating_add(1);
        let max_count = format!("--max-count={bounded_count}");
        let output = self
            .git(
                &workspace,
                &[
                    "log",
                    "-z",
                    max_count.as_str(),
                    "--format=%H%x00%P%x00%ct%x00%s",
                ],
                deadline,
            )?
            .success("read bounded Git commit sequence")?;
        let mut fields = output.split(|byte| *byte == 0).collect::<Vec<_>>();
        if fields.last().is_some_and(|field| field.is_empty()) {
            fields.pop();
        }
        if fields.len() % 4 != 0 {
            return Err(GitError::MalformedHistory(
                "commit sequence does not contain four fields per record",
            ));
        }
        let mut commits = Vec::with_capacity(fields.len() / 4);
        for record in fields.chunks_exact(4) {
            let commit_oid = parse_utf8_history_field(record[0])?;
            if !valid_oid(commit_oid, &object_format) {
                return Err(GitError::MalformedHistory("commit oid is invalid"));
            }
            let parents = parse_utf8_history_field(record[1])?;
            let parent_oids = if parents.is_empty() {
                Vec::new()
            } else {
                parents
                    .split(' ')
                    .map(|parent| {
                        if valid_oid(parent, &object_format) {
                            Ok(parent.to_owned())
                        } else {
                            Err(GitError::MalformedHistory("parent oid is invalid"))
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?
            };
            let committed_at_unix = parse_utf8_history_field(record[2])?
                .parse::<i64>()
                .map_err(|_| GitError::MalformedHistory("commit time is invalid"))?;
            let subject = parse_utf8_history_field(record[3])?.to_owned();
            commits.push(GitCommitRecord {
                commit_oid: commit_oid.to_owned(),
                parent_oids,
                committed_at_unix,
                subject,
            });
        }
        let truncated = commits.len() > max_commits;
        commits.truncate(max_commits);
        let complete = !truncated && !shallow;
        let mut omissions = Vec::new();
        if truncated {
            omissions.push(format!(
                "reachable history beyond the newest {max_commits} commits was not inspected"
            ));
        }
        if shallow {
            omissions.push("repository history is shallow".to_owned());
        }
        let mut hasher = SemanticHasher::new(GIT_COMMIT_SEQUENCE_SCHEMA);
        hasher.add_str(&object_format);
        hasher.add_bool(shallow);
        hasher.add_bool(complete);
        hasher.add_u64(max_commits as u64);
        for commit in &commits {
            hasher.add_str(&commit.commit_oid);
            hasher.add_u64(commit.parent_oids.len() as u64);
            for parent in &commit.parent_oids {
                hasher.add_str(parent);
            }
            hasher.add_str(&commit.committed_at_unix.to_string());
            hasher.add_str(&commit.subject);
        }
        Ok(Some(GitCommitSequence {
            schema: GIT_COMMIT_SEQUENCE_SCHEMA.to_owned(),
            sequence_id: hasher.finish(),
            object_format,
            head_oid: commits.first().map(|commit| commit.commit_oid.clone()),
            commits,
            complete,
            shallow,
            max_commits: max_commits as u64,
            omissions,
        }))
    }

    pub fn inspect_repository_status(&self) -> Result<Option<GitRepositoryStatus>, GitError> {
        let deadline = Instant::now() + Duration::from_millis(self.limits.total_timeout_ms);
        let workspace = fs::canonicalize(&self.workspace).map_err(|source| GitError::Path {
            path: self.workspace.clone(),
            source,
        })?;
        let git_directory_output =
            self.git(&workspace, &["rev-parse", "--absolute-git-dir"], deadline)?;
        if !git_directory_output.status.success() {
            return Ok(None);
        }
        let git_directory = canonical_output_path(&git_directory_output.stdout)?;
        ensure_beneath(&git_directory, &workspace)?;
        if self.git_bool(&workspace, &["rev-parse", "--is-bare-repository"], deadline)? {
            return Err(GitError::WorktreeUnavailable);
        }
        let object_format =
            self.git_line(&workspace, &["rev-parse", "--show-object-format"], deadline)?;
        let output = self
            .git(
                &workspace,
                &[
                    "status",
                    "--porcelain=v2",
                    "--branch",
                    "-z",
                    "--untracked-files=all",
                    "--ignore-submodules=none",
                ],
                deadline,
            )?
            .success("read Git repository status")?;
        let mut parsed = parse_repository_status(&output, &object_format)?;
        let upstream_oid = if parsed.upstream.is_some() {
            let output = self.git(
                &workspace,
                &["rev-parse", "--verify", "@{upstream}^{commit}"],
                deadline,
            )?;
            if output.status.success() {
                let oid = parse_line(&output.stdout)?;
                if !valid_oid(oid, &object_format) {
                    return Err(GitError::MalformedStatus("upstream oid is invalid"));
                }
                Some(oid.to_owned())
            } else {
                None
            }
        } else {
            None
        };
        if let (Some(head_oid), Some(upstream_oid)) =
            (parsed.head_oid.as_deref(), upstream_oid.as_deref())
        {
            let range = format!("{head_oid}...{upstream_oid}");
            let output = self
                .git(
                    &workspace,
                    &["rev-list", "--left-right", "--count", &range],
                    deadline,
                )?
                .success("compare exact Git HEAD and upstream revisions")?;
            let counts = parse_line(&output)?;
            let (ahead, behind) =
                counts
                    .split_once(char::is_whitespace)
                    .ok_or(GitError::MalformedStatus(
                        "exact branch divergence is incomplete",
                    ))?;
            parsed.ahead = Some(
                ahead
                    .parse()
                    .map_err(|_| GitError::MalformedStatus("exact ahead count is invalid"))?,
            );
            parsed.behind = Some(
                behind
                    .trim()
                    .parse()
                    .map_err(|_| GitError::MalformedStatus("exact behind count is invalid"))?,
            );
        } else if parsed.upstream.is_some() {
            parsed.ahead = None;
            parsed.behind = None;
        }
        Ok(Some(parsed.finish(upstream_oid)))
    }

    pub fn inspect_commit_publication(
        &self,
        commit_oids: &[String],
        upstream_oid: Option<&str>,
    ) -> Result<Option<Vec<String>>, GitError> {
        if commit_oids.len() > MAX_GIT_COMMIT_SEQUENCE {
            return Err(GitError::HistoryLimit {
                actual: commit_oids.len(),
                limit: MAX_GIT_COMMIT_SEQUENCE,
            });
        }
        let deadline = Instant::now() + Duration::from_millis(self.limits.total_timeout_ms);
        let workspace = fs::canonicalize(&self.workspace).map_err(|source| GitError::Path {
            path: self.workspace.clone(),
            source,
        })?;
        let repository = self.git(&workspace, &["rev-parse", "--git-dir"], deadline)?;
        if !repository.status.success() {
            return Ok(None);
        }
        let object_format =
            self.git_line(&workspace, &["rev-parse", "--show-object-format"], deadline)?;
        let Some(upstream_oid) = upstream_oid else {
            return Ok(Some(vec!["unknown".to_owned(); commit_oids.len()]));
        };
        if !valid_oid(upstream_oid, &object_format) {
            return Err(GitError::MalformedStatus("upstream oid is invalid"));
        }
        let mut states = Vec::with_capacity(commit_oids.len());
        for commit_oid in commit_oids {
            if !valid_oid(commit_oid, &object_format) {
                return Err(GitError::MalformedHistory("commit oid is invalid"));
            }
            let output = self.git(
                &workspace,
                &["merge-base", "--is-ancestor", commit_oid, upstream_oid],
                deadline,
            )?;
            match output.status.code() {
                Some(0) => states.push("pushed".to_owned()),
                Some(1) => states.push("local".to_owned()),
                _ => {
                    return Err(GitError::Command {
                        operation: "classify Git commit publication",
                        status: output.status.code(),
                        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                    });
                }
            }
        }
        Ok(Some(states))
    }

    fn inspect_index(
        &self,
        workspace: &Path,
        object_format: &str,
        deadline: Instant,
    ) -> Result<GitIndexSummary, GitError> {
        let output = self
            .git(
                workspace,
                &["ls-files", "--stage", "--debug", "--sparse", "-z"],
                deadline,
            )?
            .success("read Git index entries")?;
        let (logical_entries, omitted_semantics) =
            parse_debug_index_entries(&output, object_format, self.limits.max_index_entries)?;
        let mut hasher = SemanticHasher::new("rey.git-index-entries.v1");
        for entry in &logical_entries {
            hasher.add_str(&entry.mode);
            hasher.add_str(&entry.object_format);
            hasher.add_str(&entry.object_oid);
            hasher.add_str(&entry.stage.to_string());
            entry.path.add_semantics(&mut hasher);
            hasher.add_bool(entry.assume_unchanged);
            hasher.add_bool(entry.skip_worktree);
            hasher.add_bool(entry.intent_to_add);
        }
        let count = logical_entries.len() as u64;
        hasher.add_u64(count);
        Ok(GitIndexSummary {
            entry_digest: hasher.finish(),
            entry_count: count,
            entries: logical_entries,
            complete: omitted_semantics.is_empty(),
            omitted_semantics,
        })
    }

    fn git_bool(
        &self,
        workspace: &Path,
        args: &[&str],
        deadline: Instant,
    ) -> Result<bool, GitError> {
        match self.git_line(workspace, args, deadline)?.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(GitError::MalformedOutput("expected Git boolean")),
        }
    }

    fn git_line(
        &self,
        workspace: &Path,
        args: &[&str],
        deadline: Instant,
    ) -> Result<String, GitError> {
        let output = self
            .git(workspace, args, deadline)?
            .success("run Git inspection")?;
        Ok(parse_line(&output)?.to_owned())
    }

    fn git(
        &self,
        workspace: &Path,
        args: &[&str],
        deadline: Instant,
    ) -> Result<CommandOutput, GitError> {
        let mut fixed = vec![
            OsString::from("--no-pager"),
            OsString::from("--no-replace-objects"),
            OsString::from("--no-optional-locks"),
            OsString::from("-c"),
            OsString::from("core.hooksPath=/dev/null"),
            OsString::from("-c"),
            OsString::from("core.fsmonitor=false"),
            OsString::from("-C"),
            workspace.as_os_str().to_owned(),
        ];
        fixed.extend(args.iter().map(OsString::from));
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or(GitError::Deadline)?;
        let timeout = remaining.min(Duration::from_millis(self.limits.command_timeout_ms));
        let mut environment = vec![
            (OsString::from("GIT_CONFIG_NOSYSTEM"), OsString::from("1")),
            (
                OsString::from("GIT_CONFIG_GLOBAL"),
                OsString::from("/dev/null"),
            ),
            (OsString::from("GIT_OPTIONAL_LOCKS"), OsString::from("0")),
            (OsString::from("GIT_TERMINAL_PROMPT"), OsString::from("0")),
            (
                OsString::from("GIT_DISCOVERY_ACROSS_FILESYSTEM"),
                OsString::from("0"),
            ),
            (OsString::from("LC_ALL"), OsString::from("C")),
        ];
        if let Some(parent) = workspace.parent() {
            environment.push((
                OsString::from("GIT_CEILING_DIRECTORIES"),
                parent.as_os_str().to_owned(),
            ));
        }
        let output = run_bounded(&CommandRequest {
            program: self.git_program.clone(),
            args: fixed,
            cwd: workspace.to_owned(),
            timeout,
            max_capture_bytes: self.limits.max_capture_bytes,
            environment,
        })?;
        if output.timed_out {
            return Err(GitError::Deadline);
        }
        if output.overflowed {
            return Err(GitError::OutputLimit(self.limits.max_capture_bytes));
        }
        Ok(output)
    }
}

trait SuccessfulOutput {
    fn success(self, operation: &'static str) -> Result<Vec<u8>, GitError>;
}

impl SuccessfulOutput for CommandOutput {
    fn success(self, operation: &'static str) -> Result<Vec<u8>, GitError> {
        if self.status.success() {
            Ok(self.stdout)
        } else {
            Err(GitError::Command {
                operation,
                status: self.status.code(),
                stderr: String::from_utf8_lossy(&self.stderr).trim().to_owned(),
            })
        }
    }
}

const INDEX_STAGE_MASK: u32 = 0x0000_3000;
const INDEX_EXTENDED_FLAG: u32 = 0x0000_4000;
const INDEX_ASSUME_UNCHANGED_FLAG: u32 = 0x0000_8000;
const INDEX_SPLIT_REPLACEMENT_FLAG: u32 = 0x0800_0000;
const INDEX_INTENT_TO_ADD_FLAG: u32 = 0x2000_0000;
const INDEX_SKIP_WORKTREE_FLAG: u32 = 0x4000_0000;
const SUPPORTED_PERSISTENT_INDEX_FLAGS: u32 = INDEX_STAGE_MASK
    | INDEX_EXTENDED_FLAG
    | INDEX_ASSUME_UNCHANGED_FLAG
    | INDEX_SPLIT_REPLACEMENT_FLAG
    | INDEX_INTENT_TO_ADD_FLAG
    | INDEX_SKIP_WORKTREE_FLAG;

fn parse_debug_index_entries(
    output: &[u8],
    object_format: &str,
    max_entries: u64,
) -> Result<(Vec<GitIndexEntry>, Vec<String>), GitError> {
    let mut remaining = output;
    let mut entries = Vec::new();
    let mut omissions = BTreeSet::new();
    while !remaining.is_empty() {
        if entries.len() as u64 >= max_entries {
            return Err(GitError::IndexEntryLimit(max_entries));
        }
        let separator =
            remaining
                .iter()
                .position(|byte| *byte == 0)
                .ok_or(GitError::MalformedIndex(
                    "debug entry is missing its NUL separator",
                ))?;
        let row = &remaining[..separator];
        remaining = &remaining[separator + 1..];
        let path_separator = row
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or(GitError::MalformedIndex("entry is missing path separator"))?;
        let (header, path_with_separator) = row.split_at(path_separator);
        let path = &path_with_separator[1..];
        let header = std::str::from_utf8(header)
            .map_err(|_| GitError::MalformedIndex("entry header is not ASCII"))?;
        let mut fields = header.split(' ');
        let mode = fields
            .next()
            .ok_or(GitError::MalformedIndex("entry is missing mode"))?;
        let oid = fields
            .next()
            .ok_or(GitError::MalformedIndex("entry is missing object id"))?;
        let stage = fields
            .next()
            .ok_or(GitError::MalformedIndex("entry is missing stage"))?;
        if fields.next().is_some()
            || !mode.bytes().all(|byte| byte.is_ascii_digit())
            || !valid_oid(oid, object_format)
            || !matches!(stage, "0" | "1" | "2" | "3")
        {
            return Err(GitError::MalformedIndex("entry header has invalid fields"));
        }
        for prefix in [
            b"  ctime: ".as_slice(),
            b"  mtime: ",
            b"  dev: ",
            b"  uid: ",
        ] {
            let line = take_debug_index_line(&mut remaining)?;
            if !line.starts_with(prefix) {
                return Err(GitError::MalformedIndex(
                    "entry debug metadata has an unexpected shape",
                ));
            }
        }
        let flags_line = take_debug_index_line(&mut remaining)?;
        if !flags_line.starts_with(b"  size: ") {
            return Err(GitError::MalformedIndex(
                "entry debug metadata is missing size and flags",
            ));
        }
        let flags = flags_line
            .split(|byte| *byte == b'\t')
            .find_map(|field| field.strip_prefix(b"flags: "))
            .ok_or(GitError::MalformedIndex(
                "entry debug metadata is missing flags",
            ))?;
        let flags = std::str::from_utf8(flags)
            .ok()
            .and_then(|flags| u32::from_str_radix(flags, 16).ok())
            .ok_or(GitError::MalformedIndex("entry flags are not hexadecimal"))?;
        let stage = stage
            .parse::<u8>()
            .map_err(|_| GitError::MalformedIndex("entry stage is not numeric"))?;
        if ((flags & INDEX_STAGE_MASK) >> 12) as u8 != stage {
            return Err(GitError::MalformedIndex(
                "entry stage disagrees with its persistent flags",
            ));
        }
        let known_extended_flags = flags & (INDEX_INTENT_TO_ADD_FLAG | INDEX_SKIP_WORKTREE_FLAG);
        if known_extended_flags != 0 && flags & INDEX_EXTENDED_FLAG == 0 {
            return Err(GitError::MalformedIndex(
                "entry extended flags are internally inconsistent",
            ));
        }
        let unknown_flags = flags & !SUPPORTED_PERSISTENT_INDEX_FLAGS;
        if unknown_flags != 0 {
            omissions.insert(format!(
                "unsupported persistent index flags 0x{unknown_flags:08x}"
            ));
        } else if flags & INDEX_EXTENDED_FLAG != 0 && known_extended_flags == 0 {
            omissions.insert("unsupported empty extended index flags".to_owned());
        }
        entries.push(GitIndexEntry {
            mode: mode.to_owned(),
            object_format: object_format.to_owned(),
            object_oid: oid.to_owned(),
            stage,
            path: PathIdentity::from_bytes(path),
            assume_unchanged: flags & INDEX_ASSUME_UNCHANGED_FLAG != 0,
            skip_worktree: flags & INDEX_SKIP_WORKTREE_FLAG != 0,
            intent_to_add: flags & INDEX_INTENT_TO_ADD_FLAG != 0,
        });
    }
    Ok((entries, omissions.into_iter().collect()))
}

fn take_debug_index_line<'a>(remaining: &mut &'a [u8]) -> Result<&'a [u8], GitError> {
    let newline =
        remaining
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or(GitError::MalformedIndex(
                "entry debug metadata is incomplete",
            ))?;
    let line = &remaining[..newline];
    *remaining = &remaining[newline + 1..];
    Ok(line)
}

fn bounded_path_changes(
    mut changes: Vec<GitPathChange>,
    max_changes: u64,
) -> Result<PathChangeRead, GitError> {
    changes.sort_by(|left, right| {
        left.path
            .decoded_bytes()
            .expect("parsed path identity is reversible")
            .cmp(
                &right
                    .path
                    .decoded_bytes()
                    .expect("parsed path identity is reversible"),
            )
    });
    if path_changes_have_duplicate_paths(&changes)? {
        return Err(GitError::MalformedPathDelta(
            "duplicate path in one tree comparison",
        ));
    }
    let truncated = changes.len() as u64 > max_changes;
    changes.truncate(max_changes as usize);
    Ok(if truncated {
        PathChangeRead::Truncated(changes)
    } else {
        PathChangeRead::Complete(changes)
    })
}

fn parse_raw_path_changes(
    output: &[u8],
    object_format: &str,
) -> Result<Vec<GitPathChange>, GitError> {
    let mut records = output.split(|byte| *byte == 0).collect::<Vec<_>>();
    if records.last() == Some(&b"".as_slice()) {
        records.pop();
    } else if !records.is_empty() {
        return Err(GitError::MalformedPathDelta(
            "raw tree delta is not NUL terminated",
        ));
    }
    if records.len() % 2 != 0 {
        return Err(GitError::MalformedPathDelta(
            "raw tree delta is missing a path record",
        ));
    }
    records
        .chunks_exact(2)
        .map(|record| {
            let header = std::str::from_utf8(record[0])
                .map_err(|_| GitError::MalformedPathDelta("raw header is not ASCII"))?;
            let mut fields = header
                .strip_prefix(':')
                .ok_or(GitError::MalformedPathDelta("raw header is missing ':'"))?
                .split_whitespace();
            let source_mode = fields
                .next()
                .ok_or(GitError::MalformedPathDelta("source mode is missing"))?;
            let target_mode = fields
                .next()
                .ok_or(GitError::MalformedPathDelta("target mode is missing"))?;
            let source_oid = fields
                .next()
                .ok_or(GitError::MalformedPathDelta("source object id is missing"))?;
            let target_oid = fields
                .next()
                .ok_or(GitError::MalformedPathDelta("target object id is missing"))?;
            let status = fields
                .next()
                .ok_or(GitError::MalformedPathDelta("change status is missing"))?;
            if fields.next().is_some()
                || !valid_git_mode(source_mode)
                || !valid_git_mode(target_mode)
                || !valid_oid_or_zero(source_oid, object_format)
                || !valid_oid_or_zero(target_oid, object_format)
                || record[1].is_empty()
            {
                return Err(GitError::MalformedPathDelta(
                    "raw tree delta has invalid fields",
                ));
            }
            let source_mode = (source_mode != "000000").then(|| source_mode.to_owned());
            let target_mode = (target_mode != "000000").then(|| target_mode.to_owned());
            let source_oid = (!is_zero_oid(source_oid)).then(|| source_oid.to_owned());
            let target_oid = (!is_zero_oid(target_oid)).then(|| target_oid.to_owned());
            let kind = match status {
                "A" if source_mode.is_none()
                    && source_oid.is_none()
                    && target_mode.is_some()
                    && target_oid.is_some() =>
                {
                    GitPathChangeKind::Added
                }
                "D" if source_mode.is_some()
                    && source_oid.is_some()
                    && target_mode.is_none()
                    && target_oid.is_none() =>
                {
                    GitPathChangeKind::Deleted
                }
                "M" if source_mode.is_some()
                    && source_oid.is_some()
                    && target_mode.is_some()
                    && target_oid.is_some() =>
                {
                    GitPathChangeKind::Modified
                }
                "T" if source_mode.is_some()
                    && source_oid.is_some()
                    && target_mode.is_some()
                    && target_oid.is_some() =>
                {
                    GitPathChangeKind::TypeChanged
                }
                _ => {
                    return Err(GitError::MalformedPathDelta(
                        "unsupported or inconsistent raw change status",
                    ));
                }
            };
            Ok(GitPathChange {
                path: PathIdentity::from_bytes(record[1]),
                kind,
                source_mode,
                source_oid,
                target_mode,
                target_oid,
            })
        })
        .collect()
}

fn parse_tree_inventory_changes(
    output: &[u8],
    object_format: &str,
    kind: GitPathChangeKind,
) -> Result<Vec<GitPathChange>, GitError> {
    if !matches!(kind, GitPathChangeKind::Added | GitPathChangeKind::Deleted) {
        return Err(GitError::MalformedPathDelta(
            "tree inventory requires added or deleted direction",
        ));
    }
    let mut records = output.split(|byte| *byte == 0).collect::<Vec<_>>();
    if records.last() == Some(&b"".as_slice()) {
        records.pop();
    } else if !records.is_empty() {
        return Err(GitError::MalformedPathDelta(
            "tree inventory is not NUL terminated",
        ));
    }
    records
        .into_iter()
        .map(|record| {
            let separator = record.iter().position(|byte| *byte == b'\t').ok_or(
                GitError::MalformedPathDelta("tree inventory record is missing its path separator"),
            )?;
            let header = std::str::from_utf8(&record[..separator])
                .map_err(|_| GitError::MalformedPathDelta("tree header is not ASCII"))?;
            let path = &record[separator + 1..];
            let mut fields = header.split_whitespace();
            let mode = fields
                .next()
                .ok_or(GitError::MalformedPathDelta("tree mode is missing"))?;
            let object_kind = fields
                .next()
                .ok_or(GitError::MalformedPathDelta("tree object kind is missing"))?;
            let oid = fields
                .next()
                .ok_or(GitError::MalformedPathDelta("tree object id is missing"))?;
            if fields.next().is_some()
                || !valid_present_git_mode(mode)
                || !matches!(object_kind, "blob" | "commit")
                || !valid_oid(oid, object_format)
                || path.is_empty()
            {
                return Err(GitError::MalformedPathDelta(
                    "tree inventory has invalid fields",
                ));
            }
            let (source_mode, source_oid, target_mode, target_oid) = match kind {
                GitPathChangeKind::Added => {
                    (None, None, Some(mode.to_owned()), Some(oid.to_owned()))
                }
                GitPathChangeKind::Deleted => {
                    (Some(mode.to_owned()), Some(oid.to_owned()), None, None)
                }
                GitPathChangeKind::Modified | GitPathChangeKind::TypeChanged => unreachable!(),
            };
            Ok(GitPathChange {
                path: PathIdentity::from_bytes(path),
                kind,
                source_mode,
                source_oid,
                target_mode,
                target_oid,
            })
        })
        .collect()
}

fn path_changes_have_duplicate_paths(changes: &[GitPathChange]) -> Result<bool, GitError> {
    let paths = changes
        .iter()
        .map(|change| change.path.decoded_bytes())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(paths.windows(2).any(|window| window[0] == window[1]))
}

fn parse_line(output: &[u8]) -> Result<&str, GitError> {
    let line = std::str::from_utf8(output)
        .map_err(|_| GitError::MalformedOutput("Git output is not UTF-8"))?
        .trim();
    if line.is_empty() || line.contains('\n') {
        Err(GitError::MalformedOutput(
            "expected exactly one Git output line",
        ))
    } else {
        Ok(line)
    }
}

fn parse_utf8_history_field(field: &[u8]) -> Result<&str, GitError> {
    std::str::from_utf8(field)
        .map_err(|_| GitError::MalformedHistory("commit metadata is not UTF-8"))
}

fn git_cursor_digest(cursor: &GitPollCursor) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(GIT_POLL_CURSOR_SCHEMA);
    hasher.add_str(cursor.repository_id.as_str());
    hasher.add_optional_str(cursor.worktree_id.as_ref().map(SemanticDigest::as_str));
    hasher.add_str(cursor.snapshot_id.as_str());
    hasher.add_str(&cursor.object_format);
    hasher.add_bool(cursor.shallow);
    add_git_head(&mut hasher, &cursor.head);
    add_watched_refs(&mut hasher, &cursor.watched_refs);
    hasher.add_optional_str(cursor.index_digest.as_ref().map(SemanticDigest::as_str));
    hasher.add_bool(cursor.index_complete);
    hasher.add_bool(cursor.index_conflicted);
    hasher.add_u64(cursor.provider_revision);
    hasher.add_str(cursor.retained_evidence_id.as_str());
    hasher.finish()
}

fn git_transition_digest(transition: &GitPollTransition) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(GIT_POLL_TRANSITION_SCHEMA);
    hasher.add_str(transition.source_cursor_id.as_str());
    hasher.add_str(transition.repository_id.as_str());
    hasher.add_optional_str(transition.worktree_id.as_ref().map(SemanticDigest::as_str));
    hasher.add_str(&transition.object_format);
    hasher.add_str(transition.source_snapshot_id.as_str());
    hasher.add_str(transition.target_snapshot_id.as_str());
    add_git_head(&mut hasher, &transition.source_head);
    add_git_head(&mut hasher, &transition.target_head);
    hasher.add_str(transition.head_movement.as_str());
    hasher.add_bool(transition.head_complete);
    add_watched_refs(&mut hasher, &transition.source_watched_refs);
    add_watched_refs(&mut hasher, &transition.target_watched_refs);
    hasher.add_u64(transition.watched_ref_changes.len() as u64);
    for change in &transition.watched_ref_changes {
        hasher.add_str(&change.ref_name);
        hasher.add_optional_str(change.source_oid.as_deref());
        hasher.add_optional_str(change.target_oid.as_deref());
        hasher.add_str(change.movement.as_str());
        hasher.add_bool(change.complete);
    }
    hasher.add_u64(transition.reachability_deltas.len() as u64);
    for delta in &transition.reachability_deltas {
        hasher.add_str(&delta.ref_name);
        hasher.add_optional_str(delta.source_oid.as_deref());
        hasher.add_optional_str(delta.target_oid.as_deref());
        add_git_strings(&mut hasher, &delta.added_commits);
        add_git_strings(&mut hasher, &delta.removed_commits);
        hasher.add_u64(delta.max_commits_per_direction);
        hasher.add_bool(delta.complete);
        add_git_strings(&mut hasher, &delta.omissions);
    }
    hasher.add_u64(transition.path_deltas.len() as u64);
    for delta in &transition.path_deltas {
        hasher.add_str(&delta.ref_name);
        hasher.add_optional_str(delta.source_oid.as_deref());
        hasher.add_optional_str(delta.target_oid.as_deref());
        hasher.add_u64(delta.changes.len() as u64);
        for change in &delta.changes {
            change.path.add_semantics(&mut hasher);
            hasher.add_str(change.kind.as_str());
            hasher.add_optional_str(change.source_mode.as_deref());
            hasher.add_optional_str(change.source_oid.as_deref());
            hasher.add_optional_str(change.target_mode.as_deref());
            hasher.add_optional_str(change.target_oid.as_deref());
        }
        hasher.add_u64(delta.max_changes);
        hasher.add_bool(delta.complete);
        add_git_strings(&mut hasher, &delta.omissions);
    }
    hasher.add_optional_str(
        transition
            .source_index_digest
            .as_ref()
            .map(SemanticDigest::as_str),
    );
    hasher.add_optional_str(
        transition
            .target_index_digest
            .as_ref()
            .map(SemanticDigest::as_str),
    );
    hasher.add_bool(transition.source_index_complete);
    hasher.add_bool(transition.target_index_complete);
    hasher.add_bool(transition.source_index_conflicted);
    hasher.add_bool(transition.target_index_conflicted);
    hasher.add_bool(transition.source_shallow);
    hasher.add_bool(transition.target_shallow);
    hasher.add_u64(transition.events.len() as u64);
    for event in &transition.events {
        hasher.add_str(event.as_str());
    }
    add_git_strings(&mut hasher, &transition.omissions);
    hasher.finish()
}

fn expected_transition_events(
    transition: &GitPollTransition,
) -> Result<Vec<GitActivationEventClass>, GitError> {
    let mut events = Vec::new();
    if transition.source_head.symbolic_ref != transition.target_head.symbolic_ref {
        events.push(GitActivationEventClass::HeadRefChanged);
    }
    match (
        transition.source_head.commit_oid.as_ref(),
        transition.target_head.commit_oid.as_ref(),
        transition.head_movement,
    ) {
        (None, None, GitRefMovement::Unchanged) | (Some(_), Some(_), GitRefMovement::Unchanged)
            if transition.source_head.commit_oid == transition.target_head.commit_oid => {}
        (None, Some(_), GitRefMovement::Created) => {
            events.push(GitActivationEventClass::RefCreated);
        }
        (Some(_), None, GitRefMovement::Deleted) => {
            events.push(GitActivationEventClass::RefDeleted);
        }
        (Some(source), Some(target), movement) if source != target => {
            events.push(match movement {
                GitRefMovement::FastForward => GitActivationEventClass::RefFastForward,
                GitRefMovement::Rewound => GitActivationEventClass::RefRewound,
                GitRefMovement::Rewritten => GitActivationEventClass::RefRewritten,
                GitRefMovement::Unknown => GitActivationEventClass::RefUnknown,
                GitRefMovement::Unchanged | GitRefMovement::Created | GitRefMovement::Deleted => {
                    return Err(GitError::InvalidPollTransition);
                }
            });
        }
        _ => return Err(GitError::InvalidPollTransition),
    }
    for change in &transition.watched_ref_changes {
        events.push(ref_movement_event(change.movement)?);
    }
    for delta in &transition.reachability_deltas {
        if !delta.added_commits.is_empty() {
            events.push(GitActivationEventClass::CommitReachableAdded);
        }
        if !delta.removed_commits.is_empty() {
            events.push(GitActivationEventClass::CommitReachableRemoved);
        }
    }
    for delta in &transition.path_deltas {
        for change in &delta.changes {
            events.push(path_change_event(change.kind));
        }
    }
    if transition.source_index_digest != transition.target_index_digest {
        events.push(GitActivationEventClass::IndexChanged);
    }
    if transition.target_index_conflicted
        && (!transition.source_index_conflicted
            || transition.source_index_digest != transition.target_index_digest)
    {
        events.push(GitActivationEventClass::IndexConflicted);
    }
    events.sort();
    events.dedup();
    Ok(events)
}

fn git_activation_digest(proposal: &GitActivationProposal) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(GIT_ACTIVATION_PROPOSAL_SCHEMA);
    hasher.add_str(&proposal.trigger_id);
    hasher.add_u64(proposal.trigger_revision);
    hasher.add_str(proposal.transition_id.as_str());
    hasher.add_str(proposal.source_snapshot_id.as_str());
    hasher.add_str(proposal.target_snapshot_id.as_str());
    hasher.add_u64(proposal.matched_events.len() as u64);
    for event in &proposal.matched_events {
        hasher.add_str(event.as_str());
    }
    add_git_strings(&mut hasher, &proposal.matched_ref_names);
    hasher.add_u64(proposal.matched_path_changes.len() as u64);
    for change in &proposal.matched_path_changes {
        hasher.add_str(&change.ref_name);
        change.path.add_semantics(&mut hasher);
        hasher.add_str(change.kind.as_str());
    }
    hasher.add_bool(proposal.complete);
    add_git_strings(&mut hasher, &proposal.omissions);
    hasher.add_str(&proposal.workload_id);
    proposal.graph.add_semantics(&mut hasher);
    add_git_strings(&mut hasher, &proposal.scenario_ids);
    hasher.add_u64(proposal.budget.max_scenarios);
    hasher.add_u64(proposal.budget.max_actions);
    hasher.add_u64(proposal.budget.max_evidence_bytes);
    hasher.add_str(&proposal.authority);
    hasher.finish()
}

fn add_git_head(hasher: &mut SemanticHasher, head: &GitHead) {
    hasher.add_optional_str(head.symbolic_ref.as_deref());
    hasher.add_optional_str(head.commit_oid.as_deref());
}

fn add_watched_refs(hasher: &mut SemanticHasher, watched_refs: &[GitWatchedRef]) {
    hasher.add_u64(watched_refs.len() as u64);
    for watched in watched_refs {
        hasher.add_str(&watched.name);
        hasher.add_optional_str(watched.target_oid.as_deref());
    }
}

fn add_git_strings(hasher: &mut SemanticHasher, values: &[String]) {
    hasher.add_u64(values.len() as u64);
    for value in values {
        hasher.add_str(value);
    }
}

fn is_semantic_digest(value: &SemanticDigest) -> bool {
    value
        .as_str()
        .strip_prefix("blake3:")
        .is_some_and(|digest| {
            digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
}

fn is_canonical<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|window| window[0] < window[1])
}

#[derive(Default)]
struct ParsedRepositoryStatus {
    branch: Option<String>,
    head_oid: Option<String>,
    upstream: Option<String>,
    ahead: Option<u64>,
    behind: Option<u64>,
    staged_entries: u64,
    unstaged_entries: u64,
    untracked_entries: u64,
    conflicted_entries: u64,
}

impl ParsedRepositoryStatus {
    fn finish(self, upstream_oid: Option<String>) -> GitRepositoryStatus {
        let working_state = if self.staged_entries == 0
            && self.unstaged_entries == 0
            && self.untracked_entries == 0
            && self.conflicted_entries == 0
        {
            "clean"
        } else {
            "dirty"
        };
        let (publication_state, complete) = match (
            self.head_oid.as_ref(),
            self.branch.as_deref(),
            self.upstream.as_ref(),
            self.ahead,
            self.behind,
            upstream_oid.as_ref(),
        ) {
            (None, _, _, _, _, _) => ("unborn", true),
            (_, Some("(detached)"), _, _, _, _) | (_, None, _, _, _, _) => ("detached", true),
            (_, _, None, _, _, _) => ("no_upstream", true),
            (_, _, Some(_), Some(0), Some(0), Some(_)) => ("pushed", true),
            (_, _, Some(_), Some(ahead), Some(0), Some(_)) if ahead > 0 => ("unpushed", true),
            (_, _, Some(_), Some(0), Some(behind), Some(_)) if behind > 0 => ("behind", true),
            (_, _, Some(_), Some(ahead), Some(behind), Some(_)) if ahead > 0 && behind > 0 => {
                ("diverged", true)
            }
            _ => ("unknown", false),
        };
        let branch = self.branch.filter(|branch| branch != "(detached)");
        let working_tree = GitWorkingTreeSummary {
            state: working_state.to_owned(),
            staged_entries: self.staged_entries,
            unstaged_entries: self.unstaged_entries,
            untracked_entries: self.untracked_entries,
            conflicted_entries: self.conflicted_entries,
        };
        let publication = GitPublicationSummary {
            state: publication_state.to_owned(),
            branch,
            head_oid: self.head_oid,
            upstream: self.upstream,
            upstream_oid,
            ahead: self.ahead,
            behind: self.behind,
            comparison_basis: "local_tracking_ref".to_owned(),
        };
        let mut hasher = SemanticHasher::new(GIT_REPOSITORY_STATUS_SCHEMA);
        hasher.add_str(&working_tree.state);
        hasher.add_u64(working_tree.staged_entries);
        hasher.add_u64(working_tree.unstaged_entries);
        hasher.add_u64(working_tree.untracked_entries);
        hasher.add_u64(working_tree.conflicted_entries);
        hasher.add_str(&publication.state);
        hasher.add_optional_str(publication.branch.as_deref());
        hasher.add_optional_str(publication.head_oid.as_deref());
        hasher.add_optional_str(publication.upstream.as_deref());
        hasher.add_optional_str(publication.upstream_oid.as_deref());
        hasher.add_optional_str(publication.ahead.map(|value| value.to_string()).as_deref());
        hasher.add_optional_str(publication.behind.map(|value| value.to_string()).as_deref());
        hasher.add_bool(complete);
        GitRepositoryStatus {
            schema: GIT_REPOSITORY_STATUS_SCHEMA.to_owned(),
            status_id: hasher.finish(),
            working_tree,
            publication,
            complete,
            scope: "tracked_changes_and_untracked_files".to_owned(),
            omissions: vec![
                "remote transport was not contacted; publication is relative to the locally retained upstream ref"
                    .to_owned(),
                "ignored files are outside the working-tree status scope".to_owned(),
            ],
        }
    }
}

fn parse_repository_status(
    output: &[u8],
    object_format: &str,
) -> Result<ParsedRepositoryStatus, GitError> {
    let mut parsed = ParsedRepositoryStatus::default();
    let mut records = output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty());
    while let Some(record) = records.next() {
        if let Some(value) = record.strip_prefix(b"# branch.oid ") {
            if value != b"(initial)" {
                let oid = std::str::from_utf8(value)
                    .map_err(|_| GitError::MalformedStatus("head oid is not UTF-8"))?;
                if !valid_oid(oid, object_format) {
                    return Err(GitError::MalformedStatus("head oid is invalid"));
                }
                parsed.head_oid = Some(oid.to_owned());
            }
        } else if let Some(value) = record.strip_prefix(b"# branch.head ") {
            parsed.branch = Some(
                std::str::from_utf8(value)
                    .map_err(|_| GitError::MalformedStatus("branch name is not UTF-8"))?
                    .to_owned(),
            );
        } else if let Some(value) = record.strip_prefix(b"# branch.upstream ") {
            parsed.upstream = Some(
                std::str::from_utf8(value)
                    .map_err(|_| GitError::MalformedStatus("upstream ref is not UTF-8"))?
                    .to_owned(),
            );
        } else if let Some(value) = record.strip_prefix(b"# branch.ab ") {
            let value = std::str::from_utf8(value)
                .map_err(|_| GitError::MalformedStatus("branch divergence is not ASCII"))?;
            let (ahead, behind) = value
                .split_once(' ')
                .ok_or(GitError::MalformedStatus("branch divergence is incomplete"))?;
            parsed.ahead = Some(parse_divergence_count(ahead, '+')?);
            parsed.behind = Some(parse_divergence_count(behind, '-')?);
        } else {
            match record.first().copied() {
                Some(b'1' | b'2') => {
                    let status = record.get(2..4).ok_or(GitError::MalformedStatus(
                        "ordinary entry is missing XY state",
                    ))?;
                    if status[0] != b'.' {
                        parsed.staged_entries = parsed.staged_entries.saturating_add(1);
                    }
                    if status[1] != b'.' {
                        parsed.unstaged_entries = parsed.unstaged_entries.saturating_add(1);
                    }
                    if record[0] == b'2' && records.next().is_none() {
                        return Err(GitError::MalformedStatus(
                            "renamed entry is missing its original path",
                        ));
                    }
                }
                Some(b'u') => {
                    parsed.conflicted_entries = parsed.conflicted_entries.saturating_add(1);
                }
                Some(b'?') => {
                    parsed.untracked_entries = parsed.untracked_entries.saturating_add(1);
                }
                Some(b'!') => {}
                Some(b'#') => {
                    return Err(GitError::MalformedStatus("unknown branch header"));
                }
                _ => return Err(GitError::MalformedStatus("unknown status record")),
            }
        }
    }
    Ok(parsed)
}

fn parse_divergence_count(value: &str, prefix: char) -> Result<u64, GitError> {
    value
        .strip_prefix(prefix)
        .ok_or(GitError::MalformedStatus(
            "branch divergence has invalid sign",
        ))?
        .parse()
        .map_err(|_| GitError::MalformedStatus("branch divergence is not numeric"))
}

fn valid_oid(value: &str, object_format: &str) -> bool {
    let expected = match object_format {
        "sha1" => 40,
        "sha256" => 64,
        _ => return false,
    };
    value.len() == expected && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_zero_oid(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte == b'0')
}

fn valid_oid_or_zero(value: &str, object_format: &str) -> bool {
    valid_oid(value, object_format)
        || (is_zero_oid(value)
            && value.len()
                == match object_format {
                    "sha1" => 40,
                    "sha256" => 64,
                    _ => return false,
                })
}

fn valid_git_mode(value: &str) -> bool {
    value.len() == 6 && value.bytes().all(|byte| matches!(byte, b'0'..=b'7'))
}

fn valid_present_git_mode(value: &str) -> bool {
    value != "000000" && valid_git_mode(value)
}

fn valid_full_ref_name(name: &str) -> bool {
    name.len() <= 1_024
        && name.starts_with("refs/")
        && !name.ends_with('/')
        && !name.ends_with('.')
        && !name.contains("..")
        && !name.contains("@{")
        && !name.contains("//")
        && !name.contains('\\')
        && !name.bytes().any(|byte| {
            byte.is_ascii_control()
                || byte == b' '
                || byte == 0x7f
                || matches!(byte, b'~' | b'^' | b':' | b'?' | b'*' | b'[')
        })
        && name.split('/').all(|component| {
            !component.is_empty() && !component.starts_with('.') && !component.ends_with(".lock")
        })
}

fn verify_watched_refs(
    watched_refs: &[GitWatchedRef],
    object_format: &str,
) -> Result<(), GitError> {
    let names = watched_refs
        .iter()
        .map(|watched| &watched.name)
        .collect::<Vec<_>>();
    if watched_refs.len() > MAX_GIT_WATCHED_REFS || !is_canonical(&names) {
        return Err(GitError::InvalidSnapshot);
    }
    watched_refs
        .iter()
        .try_for_each(|watched| watched.verify(object_format))
}

fn ref_movement_event(movement: GitRefMovement) -> Result<GitActivationEventClass, GitError> {
    match movement {
        GitRefMovement::Created => Ok(GitActivationEventClass::RefCreated),
        GitRefMovement::Deleted => Ok(GitActivationEventClass::RefDeleted),
        GitRefMovement::FastForward => Ok(GitActivationEventClass::RefFastForward),
        GitRefMovement::Rewound => Ok(GitActivationEventClass::RefRewound),
        GitRefMovement::Rewritten => Ok(GitActivationEventClass::RefRewritten),
        GitRefMovement::Unknown => Ok(GitActivationEventClass::RefUnknown),
        GitRefMovement::Unchanged => Err(GitError::InvalidPollTransition),
    }
}

const fn path_change_event(kind: GitPathChangeKind) -> GitActivationEventClass {
    match kind {
        GitPathChangeKind::Added => GitActivationEventClass::PathAdded,
        GitPathChangeKind::Deleted => GitActivationEventClass::PathDeleted,
        GitPathChangeKind::Modified => GitActivationEventClass::PathModified,
        GitPathChangeKind::TypeChanged => GitActivationEventClass::PathTypeChanged,
    }
}

fn verify_watched_ref_changes(transition: &GitPollTransition) -> Result<(), GitError> {
    let change_names = transition
        .watched_ref_changes
        .iter()
        .map(|change| &change.ref_name)
        .collect::<Vec<_>>();
    if transition.source_watched_refs.len() != transition.target_watched_refs.len()
        || transition.watched_ref_changes.len() > MAX_GIT_WATCHED_REFS
        || !is_canonical(&change_names)
    {
        return Err(GitError::InvalidPollTransition);
    }
    let mut expected_changes = 0;
    for (source, target) in transition
        .source_watched_refs
        .iter()
        .zip(&transition.target_watched_refs)
    {
        if source.name != target.name {
            return Err(GitError::InvalidPollTransition);
        }
        if source.target_oid == target.target_oid {
            continue;
        }
        expected_changes += 1;
        let change = transition
            .watched_ref_changes
            .iter()
            .find(|change| change.ref_name == source.name)
            .ok_or(GitError::InvalidPollTransition)?;
        if change.source_oid != source.target_oid
            || change.target_oid != target.target_oid
            || change.complete != (change.movement != GitRefMovement::Unknown)
            || (transition.source_shallow || transition.target_shallow)
                && change.source_oid.is_some()
                && change.target_oid.is_some()
                && change.movement != GitRefMovement::Unknown
            || !matches!(
                (&change.source_oid, &change.target_oid, change.movement),
                (None, Some(_), GitRefMovement::Created)
                    | (Some(_), None, GitRefMovement::Deleted)
                    | (
                        Some(_),
                        Some(_),
                        GitRefMovement::FastForward
                            | GitRefMovement::Rewound
                            | GitRefMovement::Rewritten
                            | GitRefMovement::Unknown
                    )
            )
        {
            return Err(GitError::InvalidPollTransition);
        }
    }
    if expected_changes != transition.watched_ref_changes.len() {
        return Err(GitError::InvalidPollTransition);
    }
    Ok(())
}

fn verify_reachability_deltas(transition: &GitPollTransition) -> Result<(), GitError> {
    let mut endpoints = Vec::new();
    if transition.source_head.commit_oid != transition.target_head.commit_oid {
        endpoints.push((
            "HEAD",
            &transition.source_head.commit_oid,
            &transition.target_head.commit_oid,
            transition.head_movement,
        ));
    }
    endpoints.extend(transition.watched_ref_changes.iter().map(|change| {
        (
            change.ref_name.as_str(),
            &change.source_oid,
            &change.target_oid,
            change.movement,
        )
    }));
    if endpoints.len() != transition.reachability_deltas.len() {
        return Err(GitError::InvalidPollTransition);
    }
    for ((ref_name, source_oid, target_oid, movement), delta) in
        endpoints.into_iter().zip(&transition.reachability_deltas)
    {
        let shallow_omission =
            format!("{ref_name} reachability is incomplete because repository history is shallow");
        let added_limit_omission = format!(
            "{ref_name} added reachability exceeds the {}-commit per-direction limit",
            delta.max_commits_per_direction
        );
        let removed_limit_omission = format!(
            "{ref_name} removed reachability exceeds the {}-commit per-direction limit",
            delta.max_commits_per_direction
        );
        let added_unavailable_omission = format!(
            "{ref_name} added reachability is unavailable because bounded Git history cannot resolve the comparison"
        );
        let removed_unavailable_omission = format!(
            "{ref_name} removed reachability is unavailable because bounded Git history cannot resolve the comparison"
        );
        let allowed_omissions = [
            shallow_omission.as_str(),
            added_limit_omission.as_str(),
            removed_limit_omission.as_str(),
            added_unavailable_omission.as_str(),
            removed_unavailable_omission.as_str(),
        ];
        if delta.ref_name != ref_name
            || &delta.source_oid != source_oid
            || &delta.target_oid != target_oid
            || delta.max_commits_per_direction == 0
            || delta.max_commits_per_direction > MAX_GIT_REACHABLE_COMMITS_PER_DIRECTION
            || delta.added_commits.len() as u64 > delta.max_commits_per_direction
            || delta.removed_commits.len() as u64 > delta.max_commits_per_direction
            || !is_canonical(&delta.added_commits)
            || !is_canonical(&delta.removed_commits)
            || delta
                .added_commits
                .iter()
                .chain(&delta.removed_commits)
                .any(|oid| !valid_oid(oid, &transition.object_format))
            || delta
                .added_commits
                .iter()
                .any(|oid| delta.removed_commits.binary_search(oid).is_ok())
            || delta.complete != delta.omissions.is_empty()
            || !is_canonical(&delta.omissions)
            || delta
                .omissions
                .iter()
                .any(|omission| !allowed_omissions.contains(&omission.as_str()))
            || (transition.source_shallow || transition.target_shallow)
                != delta.omissions.contains(&shallow_omission)
            || delta.omissions.contains(&added_limit_omission)
                && delta.added_commits.len() as u64 != delta.max_commits_per_direction
            || delta.omissions.contains(&removed_limit_omission)
                && delta.removed_commits.len() as u64 != delta.max_commits_per_direction
            || delta.omissions.contains(&added_unavailable_omission)
                && !delta.added_commits.is_empty()
            || delta.omissions.contains(&removed_unavailable_omission)
                && !delta.removed_commits.is_empty()
            || delta.omissions.contains(&added_limit_omission)
                && delta.omissions.contains(&added_unavailable_omission)
            || delta.omissions.contains(&removed_limit_omission)
                && delta.omissions.contains(&removed_unavailable_omission)
            || movement == GitRefMovement::Unknown && delta.complete
            || matches!(
                movement,
                GitRefMovement::FastForward | GitRefMovement::Created
            ) && !delta.removed_commits.is_empty()
            || matches!(movement, GitRefMovement::Rewound | GitRefMovement::Deleted)
                && !delta.added_commits.is_empty()
        {
            return Err(GitError::InvalidPollTransition);
        }
    }
    Ok(())
}

fn verify_path_deltas(transition: &GitPollTransition) -> Result<(), GitError> {
    let mut endpoints = Vec::new();
    if transition.source_head.commit_oid != transition.target_head.commit_oid {
        endpoints.push((
            "HEAD",
            &transition.source_head.commit_oid,
            &transition.target_head.commit_oid,
        ));
    }
    endpoints.extend(transition.watched_ref_changes.iter().map(|change| {
        (
            change.ref_name.as_str(),
            &change.source_oid,
            &change.target_oid,
        )
    }));
    if endpoints.len() != transition.path_deltas.len() {
        return Err(GitError::InvalidPollTransition);
    }
    for ((ref_name, source_oid, target_oid), delta) in
        endpoints.into_iter().zip(&transition.path_deltas)
    {
        let limit_omission = format!(
            "{ref_name} path changes exceed the {}-change limit",
            delta.max_changes
        );
        let unavailable_omission = format!(
            "{ref_name} path delta is unavailable because bounded Git trees cannot resolve the comparison"
        );
        if delta.ref_name != ref_name
            || &delta.source_oid != source_oid
            || &delta.target_oid != target_oid
            || delta.max_changes == 0
            || delta.max_changes > MAX_GIT_PATH_CHANGES_PER_REF
            || delta.changes.len() as u64 > delta.max_changes
            || delta.complete != delta.omissions.is_empty()
            || !is_canonical(&delta.omissions)
            || delta
                .omissions
                .iter()
                .any(|omission| omission != &limit_omission && omission != &unavailable_omission)
            || delta.omissions.contains(&limit_omission)
                && delta.changes.len() as u64 != delta.max_changes
            || delta.omissions.contains(&unavailable_omission) && !delta.changes.is_empty()
            || delta.omissions.contains(&limit_omission)
                && delta.omissions.contains(&unavailable_omission)
        {
            return Err(GitError::InvalidPollTransition);
        }
        let mut previous_path: Option<Vec<u8>> = None;
        for change in &delta.changes {
            let path = change.path.verify()?;
            if previous_path
                .as_ref()
                .is_some_and(|previous| previous >= &path)
                || change
                    .source_mode
                    .as_deref()
                    .is_some_and(|mode| !valid_present_git_mode(mode))
                || change
                    .target_mode
                    .as_deref()
                    .is_some_and(|mode| !valid_present_git_mode(mode))
                || change
                    .source_oid
                    .as_deref()
                    .is_some_and(|oid| !valid_oid(oid, &transition.object_format))
                || change
                    .target_oid
                    .as_deref()
                    .is_some_and(|oid| !valid_oid(oid, &transition.object_format))
                || !matches!(
                    (
                        change.kind,
                        &change.source_mode,
                        &change.source_oid,
                        &change.target_mode,
                        &change.target_oid,
                    ),
                    (GitPathChangeKind::Added, None, None, Some(_), Some(_))
                        | (GitPathChangeKind::Deleted, Some(_), Some(_), None, None)
                        | (
                            GitPathChangeKind::Modified | GitPathChangeKind::TypeChanged,
                            Some(_),
                            Some(_),
                            Some(_),
                            Some(_)
                        )
                )
                || change.kind == GitPathChangeKind::Modified
                    && change.source_mode == change.target_mode
                    && change.source_oid == change.target_oid
                || change.kind == GitPathChangeKind::TypeChanged
                    && change.source_mode == change.target_mode
            {
                return Err(GitError::InvalidPollTransition);
            }
            previous_path = Some(path);
        }
    }
    Ok(())
}

fn verify_path_prefixes(prefixes: &[PathIdentity]) -> Result<(), GitError> {
    if prefixes.len() > MAX_GIT_WATCHED_REFS {
        return Err(GitError::InvalidActivationTrigger);
    }
    let decoded = prefixes
        .iter()
        .map(PathIdentity::verify)
        .collect::<Result<Vec<_>, _>>()?;
    if !is_canonical(&decoded) {
        return Err(GitError::InvalidActivationTrigger);
    }
    Ok(())
}

fn trigger_path_prefix_bytes(trigger: &GitActivationTrigger) -> Result<Vec<Vec<u8>>, GitError> {
    trigger
        .path_prefixes
        .iter()
        .map(PathIdentity::decoded_bytes)
        .collect()
}

fn verify_matched_path_changes(changes: &[GitMatchedPathChange]) -> Result<(), GitError> {
    if changes.len() > MAX_GIT_MATCHED_PATH_CHANGES {
        return Err(GitError::InvalidActivationProposal);
    }
    let mut previous: Option<(String, Vec<u8>, GitPathChangeKind)> = None;
    for change in changes {
        if change.ref_name != "HEAD" && !valid_full_ref_name(&change.ref_name) {
            return Err(GitError::InvalidActivationProposal);
        }
        let key = (change.ref_name.clone(), change.path.verify()?, change.kind);
        if previous.as_ref().is_some_and(|previous| previous >= &key) {
            return Err(GitError::InvalidActivationProposal);
        }
        previous = Some(key);
    }
    Ok(())
}

fn canonical_output_path(output: &[u8]) -> Result<PathBuf, GitError> {
    let path = output_path(output)?;
    fs::canonicalize(&path).map_err(|source| GitError::Path { path, source })
}

#[cfg(unix)]
fn output_path(output: &[u8]) -> Result<PathBuf, GitError> {
    use std::os::unix::ffi::OsStringExt;

    let bytes = output
        .strip_suffix(b"\r\n")
        .or_else(|| output.strip_suffix(b"\n"))
        .unwrap_or(output);
    if bytes.is_empty() || bytes.contains(&0) {
        return Err(GitError::MalformedOutput(
            "Git path output is empty or contains NUL",
        ));
    }
    Ok(PathBuf::from(OsString::from_vec(bytes.to_vec())))
}

#[cfg(not(unix))]
fn output_path(output: &[u8]) -> Result<PathBuf, GitError> {
    Ok(PathBuf::from(parse_line(output)?))
}

fn ensure_beneath(path: &Path, workspace: &Path) -> Result<(), GitError> {
    if path.starts_with(workspace) {
        Ok(())
    } else {
        Err(GitError::OutsideWorkspace {
            path: path.to_owned(),
            workspace: workspace.to_owned(),
        })
    }
}

#[derive(Debug, Error)]
pub enum GitError {
    #[error("Git path {path} cannot be resolved: {source}")]
    Path { path: PathBuf, source: io::Error },
    #[error("Git path {path} is outside explicit workspace {workspace}")]
    OutsideWorkspace { path: PathBuf, workspace: PathBuf },
    #[error(transparent)]
    Process(#[from] CommandError),
    #[error("Git inspection exceeded its command deadline")]
    Deadline,
    #[error("Git inspection exceeded its {0}-byte output limit")]
    OutputLimit(u64),
    #[error("could not {operation}; exit={status:?}; stderr={stderr}")]
    Command {
        operation: &'static str,
        status: Option<i32>,
        stderr: String,
    },
    #[error("malformed Git output: {0}")]
    MalformedOutput(&'static str),
    #[error("malformed Git index: {0}")]
    MalformedIndex(&'static str),
    #[error("malformed watched Git ref: {0}")]
    MalformedRef(&'static str),
    #[error("malformed Git reachability evidence: {0}")]
    MalformedReachability(&'static str),
    #[error("malformed Git path delta: {0}")]
    MalformedPathDelta(&'static str),
    #[error("Git watched-ref scope is invalid")]
    InvalidWatchedRefScope,
    #[error("Git index exceeds {0} logical entries")]
    IndexEntryLimit(u64),
    #[error("Git commit sequence limit {actual} is outside 1..={limit}")]
    HistoryLimit { actual: usize, limit: usize },
    #[error("malformed Git commit sequence: {0}")]
    MalformedHistory(&'static str),
    #[error("malformed Git repository status: {0}")]
    MalformedStatus(&'static str),
    #[error("Git repository does not expose a worktree status")]
    WorktreeUnavailable,
    #[error("Git snapshot is invalid or semantically tampered")]
    InvalidSnapshot,
    #[error("Git poll cursor is invalid or semantically tampered")]
    InvalidPollCursor,
    #[error("Git poll transition is invalid or semantically tampered")]
    InvalidPollTransition,
    #[error("Git poll cursor can advance only after its exact transition evidence is retained")]
    CursorRetentionMismatch,
    #[error("Git repository or worktree identity changed across the poll cursor")]
    RepositoryIdentityChanged,
    #[error("Git activation trigger is invalid")]
    InvalidActivationTrigger,
    #[error("Git activation proposal is invalid or semantically tampered")]
    InvalidActivationProposal,
    #[error("Git activation trigger count exceeds {0}")]
    ActivationTriggerLimit(usize),
    #[error("Git activation matched path count exceeds {0}")]
    ActivationPathMatchLimit(usize),
    #[error("duplicate Git activation proposals are not permitted")]
    DuplicateActivationProposal,
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, process::Command};

    use tempfile::TempDir;

    use rey_core::ContractIdentity;
    use rey_environment::resolve_executable;

    use super::{
        GIT_ACTIVATION_TRIGGER_SCHEMA, GitActivationBudget, GitActivationEventClass,
        GitActivationTrigger, GitError, GitInspector, GitLimits, GitPathChangeKind, GitPollCursor,
        GitRefMovement, PathIdentity, derive_activation_proposals, parse_repository_status,
    };

    fn git(directory: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(["-C", directory.to_str().unwrap()])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .status()
            .unwrap();
        assert!(status.success(), "git fixture command failed: {args:?}");
    }

    fn repository() -> TempDir {
        let directory = TempDir::new().unwrap();
        git(directory.path(), &["init", "-q"]);
        git(directory.path(), &["config", "user.name", "Rey Test"]);
        git(
            directory.path(),
            &["config", "user.email", "rey@example.invalid"],
        );
        fs::write(directory.path().join("tracked"), "one\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        git(directory.path(), &["commit", "-q", "-m", "initial"]);
        git(directory.path(), &["branch", "-M", "main"]);
        directory
    }

    fn push_to_local_remote(directory: &Path) -> TempDir {
        let remote = TempDir::new().unwrap();
        git(remote.path(), &["init", "--bare", "-q"]);
        git(
            directory,
            &["remote", "add", "origin", remote.path().to_str().unwrap()],
        );
        git(directory, &["push", "-q", "-u", "origin", "main"]);
        remote
    }

    fn inspector(directory: &Path) -> GitInspector {
        let search_paths =
            std::env::split_paths(&std::env::var_os("PATH").unwrap()).collect::<Vec<_>>();
        GitInspector {
            git_program: resolve_executable("git", &search_paths).unwrap(),
            workspace: directory.to_owned(),
            limits: GitLimits::default(),
        }
    }

    fn trigger(
        snapshot: &super::GitSnapshot,
        event: GitActivationEventClass,
        require_complete: bool,
    ) -> GitActivationTrigger {
        GitActivationTrigger {
            schema: GIT_ACTIVATION_TRIGGER_SCHEMA.to_owned(),
            trigger_id: format!("fixture.{}", event.as_str()),
            revision: 1,
            repository_id: snapshot.repository_id.clone(),
            worktree_id: snapshot.worktree_id.clone(),
            event_classes: vec![event],
            ref_names: Vec::new(),
            path_prefixes: Vec::new(),
            require_complete,
            workload_id: "fixture-workload".to_owned(),
            graph: ContractIdentity::new("fixture.graph", 1, "fixture graph"),
            scenario_ids: vec!["fixture-scenario".to_owned()],
            budget: GitActivationBudget::default(),
        }
    }

    #[test]
    fn non_repository_is_absent_not_an_error() {
        let directory = TempDir::new().unwrap();
        assert!(inspector(directory.path()).inspect().unwrap().is_none());
        assert!(
            inspector(directory.path())
                .inspect_recent_commits(8)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn unborn_repository_is_a_complete_typed_empty_sequence() {
        let directory = TempDir::new().unwrap();
        git(directory.path(), &["init", "-q"]);

        let sequence = inspector(directory.path())
            .inspect_recent_commits(8)
            .unwrap()
            .unwrap();

        assert!(sequence.complete);
        assert!(sequence.commits.is_empty());
        assert!(sequence.head_oid.is_none());
        assert!(sequence.omissions.is_empty());
    }

    #[test]
    fn recent_commit_sequence_is_newest_first_bounded_and_exact() {
        let directory = repository();
        fs::write(directory.path().join("tracked"), "two\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        git(directory.path(), &["commit", "-q", "-m", "second"]);

        let inspect = inspector(directory.path());
        let complete = inspect.inspect_recent_commits(8).unwrap().unwrap();
        assert_eq!(complete.schema, "rey.git-commit-sequence.v1");
        assert_eq!(complete.commits.len(), 2);
        assert_eq!(complete.commits[0].subject, "second");
        assert_eq!(complete.commits[1].subject, "initial");
        assert_eq!(
            complete.commits[0].parent_oids,
            vec![complete.commits[1].commit_oid.clone()]
        );
        assert_eq!(
            complete.head_oid,
            Some(complete.commits[0].commit_oid.clone())
        );
        assert!(complete.complete);
        assert!(complete.omissions.is_empty());

        let bounded = inspect.inspect_recent_commits(1).unwrap().unwrap();
        assert_eq!(bounded.commits.len(), 1);
        assert!(!bounded.complete);
        assert_eq!(bounded.omissions.len(), 1);
        assert!(bounded.omissions[0].contains("newest 1 commits"));
    }

    #[test]
    fn repository_status_separates_worktree_and_local_upstream_state() {
        let directory = repository();
        let _remote = push_to_local_remote(directory.path());
        let pushed = inspector(directory.path())
            .inspect_repository_status()
            .unwrap()
            .unwrap();
        assert_eq!(pushed.schema, "rey.git-repository-status.v1");
        assert_eq!(pushed.working_tree.state, "clean");
        assert_eq!(pushed.publication.state, "pushed");
        assert_eq!(pushed.publication.branch.as_deref(), Some("main"));
        assert_eq!(pushed.publication.upstream.as_deref(), Some("origin/main"));
        assert_eq!(pushed.publication.ahead, Some(0));
        assert_eq!(pushed.publication.behind, Some(0));
        assert!(pushed.complete);

        fs::write(directory.path().join("tracked"), "two\n").unwrap();
        fs::write(directory.path().join("staged"), "staged\n").unwrap();
        fs::write(directory.path().join("untracked"), "untracked\n").unwrap();
        git(directory.path(), &["add", "staged"]);
        let dirty = inspector(directory.path())
            .inspect_repository_status()
            .unwrap()
            .unwrap();
        assert_eq!(dirty.working_tree.state, "dirty");
        assert_eq!(dirty.working_tree.staged_entries, 1);
        assert_eq!(dirty.working_tree.unstaged_entries, 1);
        assert_eq!(dirty.working_tree.untracked_entries, 1);
        assert_eq!(dirty.working_tree.conflicted_entries, 0);
        assert_eq!(dirty.publication.state, "pushed");
    }

    #[test]
    fn recent_commits_are_classified_against_the_local_upstream_ref() {
        let directory = repository();
        let _remote = push_to_local_remote(directory.path());
        fs::write(directory.path().join("tracked"), "two\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        git(directory.path(), &["commit", "-q", "-m", "local"]);

        let inspect = inspector(directory.path());
        let status = inspect.inspect_repository_status().unwrap().unwrap();
        assert_eq!(status.publication.state, "unpushed");
        assert_eq!(status.publication.ahead, Some(1));
        assert_eq!(status.publication.behind, Some(0));
        let commits = inspect.inspect_recent_commits(8).unwrap().unwrap();
        let states = inspect
            .inspect_commit_publication(
                &commits
                    .commits
                    .iter()
                    .map(|commit| commit.commit_oid.clone())
                    .collect::<Vec<_>>(),
                status.publication.upstream_oid.as_deref(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(states, vec!["local", "pushed"]);
    }

    #[test]
    fn repository_without_upstream_is_explicit_not_unknown() {
        let directory = repository();
        let inspect = inspector(directory.path());
        let status = inspect.inspect_repository_status().unwrap().unwrap();
        assert_eq!(status.publication.state, "no_upstream");
        assert_eq!(status.publication.ahead, None);
        assert_eq!(status.publication.behind, None);
        assert!(status.complete);
        let commits = inspect.inspect_recent_commits(8).unwrap().unwrap();
        let states = inspect
            .inspect_commit_publication(
                &commits
                    .commits
                    .iter()
                    .map(|commit| commit.commit_oid.clone())
                    .collect::<Vec<_>>(),
                status.publication.upstream_oid.as_deref(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(states, vec!["unknown"]);
    }

    #[test]
    fn porcelain_v2_conflicts_remain_a_separate_attention_dimension() {
        let oid = "0".repeat(40);
        let input = format!(
            "# branch.oid {oid}\0# branch.head main\0u UU N... 100644 100644 100644 100644 {oid} {oid} {oid} conflict\0"
        );
        let parsed = parse_repository_status(input.as_bytes(), "sha1").unwrap();
        let status = parsed.finish(None);
        assert_eq!(status.working_tree.state, "dirty");
        assert_eq!(status.working_tree.conflicted_entries, 1);
        assert_eq!(status.working_tree.staged_entries, 0);
        assert_eq!(status.working_tree.unstaged_entries, 0);
    }

    #[test]
    fn repository_discovery_does_not_cross_the_explicit_workspace_root() {
        let directory = repository();
        let child = directory.path().join("bounded-child");
        fs::create_dir(&child).unwrap();

        assert!(inspector(&child).inspect().unwrap().is_none());
    }

    #[test]
    fn logical_index_digest_ignores_refresh_and_changes_for_staged_content() {
        let directory = repository();
        let inspect = inspector(directory.path());
        let initial = inspect.inspect().unwrap().unwrap();
        let index_path = directory.path().join(".git/index");
        let raw_before = fs::read(&index_path).unwrap();

        git(directory.path(), &["update-index", "--refresh"]);
        let refreshed = inspect.inspect().unwrap().unwrap();
        assert_eq!(initial.snapshot_id, refreshed.snapshot_id);
        assert_eq!(
            initial.index.as_ref().unwrap().entry_digest,
            refreshed.index.as_ref().unwrap().entry_digest
        );

        fs::write(directory.path().join("tracked"), "two\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        let changed = inspect.inspect().unwrap().unwrap();
        assert_ne!(
            refreshed.index.as_ref().unwrap().entry_digest,
            changed.index.as_ref().unwrap().entry_digest
        );
        assert!(!raw_before.is_empty());
    }

    #[test]
    fn logical_index_retains_behavior_flags_and_ignores_storage_details() {
        let directory = repository();
        fs::write(directory.path().join("skip"), "skip\n").unwrap();
        git(directory.path(), &["add", "skip"]);
        git(
            directory.path(),
            &["commit", "-q", "-m", "add skip fixture"],
        );
        git(
            directory.path(),
            &["update-index", "--assume-unchanged", "tracked"],
        );
        git(
            directory.path(),
            &["update-index", "--skip-worktree", "skip"],
        );
        fs::write(directory.path().join("intent"), "intent\n").unwrap();
        git(directory.path(), &["add", "-N", "intent"]);

        let inspect = inspector(directory.path());
        let flagged = inspect.inspect().unwrap().unwrap();
        let index = flagged.index.as_ref().unwrap();
        assert!(index.complete);
        assert!(index.omitted_semantics.is_empty());
        let entry = |path: &str| {
            index
                .entries
                .iter()
                .find(|entry| entry.path.display == path)
                .unwrap()
        };
        assert!(entry("tracked").assume_unchanged);
        assert!(!entry("tracked").skip_worktree);
        assert!(entry("skip").skip_worktree);
        assert!(!entry("skip").intent_to_add);
        assert!(entry("intent").intent_to_add);
        assert_eq!(entry("intent").stage, 0);

        let mut tampered = flagged.clone();
        let assume_unchanged = tampered.index.as_ref().unwrap().entries[0].assume_unchanged;
        tampered.index.as_mut().unwrap().entries[0].assume_unchanged = !assume_unchanged;
        assert!(matches!(tampered.verify(), Err(GitError::InvalidSnapshot)));

        git(
            directory.path(),
            &["update-index", "--no-assume-unchanged", "tracked"],
        );
        git(
            directory.path(),
            &["update-index", "--no-skip-worktree", "skip"],
        );
        git(directory.path(), &["add", "intent"]);
        let cleared = inspect.inspect().unwrap().unwrap();
        assert_ne!(flagged.snapshot_id, cleared.snapshot_id);
        assert_ne!(
            flagged.index.unwrap().entry_digest,
            cleared.index.unwrap().entry_digest
        );
    }

    #[test]
    fn unknown_persistent_index_flags_remain_explicit_omissions() {
        let oid = "0".repeat(40);
        let output = format!(
            "100644 {oid} 0\ttracked\0  ctime: 0:0\n  mtime: 0:0\n  dev: 0\tino: 0\n  uid: 0\tgid: 0\n  size: 0\tflags: 80000000\n"
        );
        let (entries, omissions) =
            super::parse_debug_index_entries(output.as_bytes(), "sha1", 8).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            omissions,
            vec!["unsupported persistent index flags 0x80000000"]
        );
    }

    #[test]
    fn split_and_sparse_storage_project_complete_logical_entries() {
        let split = repository();
        git(split.path(), &["update-index", "--split-index"]);
        let split_snapshot = inspector(split.path()).inspect().unwrap().unwrap();
        let split_index = split_snapshot.index.unwrap();
        assert!(split_index.complete);
        assert_eq!(split_index.entries.len(), 1);
        assert_eq!(split_index.entries[0].path.display, "tracked");

        let sparse = TempDir::new().unwrap();
        git(sparse.path(), &["init", "-q"]);
        git(sparse.path(), &["config", "user.name", "Rey Test"]);
        git(
            sparse.path(),
            &["config", "user.email", "rey@example.invalid"],
        );
        fs::create_dir_all(sparse.path().join("visible")).unwrap();
        fs::create_dir_all(sparse.path().join("hidden")).unwrap();
        fs::write(sparse.path().join("visible/file"), "visible\n").unwrap();
        fs::write(sparse.path().join("hidden/file"), "hidden\n").unwrap();
        git(sparse.path(), &["add", "."]);
        git(sparse.path(), &["commit", "-q", "-m", "sparse fixture"]);
        git(
            sparse.path(),
            &["sparse-checkout", "init", "--cone", "--sparse-index"],
        );
        git(sparse.path(), &["sparse-checkout", "set", "visible"]);
        let sparse_snapshot = inspector(sparse.path()).inspect().unwrap().unwrap();
        let sparse_index = sparse_snapshot.index.unwrap();
        assert!(sparse_index.complete);
        assert_eq!(sparse_index.entries.len(), 2);
        let hidden = sparse_index
            .entries
            .iter()
            .find(|entry| entry.path.display == "hidden/")
            .unwrap();
        assert_eq!(hidden.mode, "040000");
        assert!(hidden.skip_worktree);
    }

    #[test]
    fn poll_classifies_fast_forward_and_replays_one_activation_identity() {
        let directory = repository();
        let inspect = inspector(directory.path());
        let initial = inspect.inspect().unwrap().unwrap();
        let mut tampered_snapshot = initial.clone();
        tampered_snapshot.head.symbolic_ref = Some("refs/heads/other".to_owned());
        assert!(matches!(
            GitPollCursor::from_retained_snapshot(
                &tampered_snapshot,
                tampered_snapshot.snapshot_id.clone()
            ),
            Err(GitError::InvalidSnapshot)
        ));
        let cursor =
            GitPollCursor::from_retained_snapshot(&initial, initial.snapshot_id.clone()).unwrap();

        fs::write(directory.path().join("tracked"), "two\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        git(directory.path(), &["commit", "-q", "-m", "second"]);

        let (target, transition) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        assert_eq!(transition.head_movement, GitRefMovement::FastForward);
        assert_eq!(
            transition.events,
            vec![
                GitActivationEventClass::RefFastForward,
                GitActivationEventClass::CommitReachableAdded,
                GitActivationEventClass::PathModified,
                GitActivationEventClass::IndexChanged,
            ]
        );
        assert_eq!(transition.reachability_deltas.len(), 1);
        assert_eq!(transition.reachability_deltas[0].ref_name, "HEAD");
        assert_eq!(
            transition.reachability_deltas[0].added_commits,
            vec![target.head.commit_oid.clone().unwrap()]
        );
        assert!(transition.reachability_deltas[0].removed_commits.is_empty());
        assert!(transition.reachability_deltas[0].complete);
        assert_eq!(transition.path_deltas.len(), 1);
        assert_eq!(transition.path_deltas[0].changes.len(), 1);
        assert_eq!(
            transition.path_deltas[0].changes[0].kind,
            GitPathChangeKind::Modified
        );
        assert_eq!(transition.path_deltas[0].changes[0].path.display, "tracked");
        assert!(transition.path_deltas[0].complete);
        assert!(transition.head_complete);
        assert!(transition.target_index_complete);

        let trigger = trigger(&initial, GitActivationEventClass::RefFastForward, true);
        let proposals =
            derive_activation_proposals(&transition, std::slice::from_ref(&trigger)).unwrap();
        assert_eq!(proposals.len(), 1);
        assert!(proposals[0].complete);
        assert!(proposals[0].omissions.is_empty());
        assert_eq!(
            proposals[0].authority,
            "proposal_only; normal workload admission and runtime preconditions still apply"
        );

        let (_, replay) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        let replayed = derive_activation_proposals(&replay, &[trigger]).unwrap();
        assert_eq!(transition, replay);
        assert_eq!(proposals, replayed);

        assert!(matches!(
            cursor.advance(&transition, target.snapshot_id.clone()),
            Err(GitError::CursorRetentionMismatch)
        ));
        let advanced = cursor
            .advance(&transition, transition.transition_id.clone())
            .unwrap();
        let (_, unchanged) = inspect.inspect_transition(&advanced).unwrap().unwrap();
        assert_eq!(unchanged.head_movement, GitRefMovement::Unchanged);
        assert!(unchanged.events.is_empty());
    }

    #[test]
    fn watched_refs_retain_absence_classify_independently_and_scope_triggers() {
        let directory = repository();
        git(directory.path(), &["branch", "release"]);
        fs::write(directory.path().join("tracked"), "two\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        git(directory.path(), &["commit", "-q", "-m", "second"]);

        let inspect = inspector(directory.path());
        let watched_names = vec![
            "refs/heads/release".to_owned(),
            "refs/heads/future".to_owned(),
        ];
        let initial = inspect
            .inspect_with_watched_refs(&watched_names)
            .unwrap()
            .unwrap();
        assert_eq!(
            initial
                .watched_refs
                .iter()
                .map(|watched| watched.name.as_str())
                .collect::<Vec<_>>(),
            vec!["refs/heads/future", "refs/heads/release"]
        );
        assert!(initial.watched_refs[0].target_oid.is_none());
        assert!(initial.watched_refs[1].target_oid.is_some());
        let cursor =
            GitPollCursor::from_retained_snapshot(&initial, initial.snapshot_id.clone()).unwrap();

        git(directory.path(), &["branch", "-f", "release", "main"]);
        let (target, transition) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        assert_eq!(transition.head_movement, GitRefMovement::Unchanged);
        assert_eq!(transition.watched_ref_changes.len(), 1);
        let change = &transition.watched_ref_changes[0];
        assert_eq!(change.ref_name, "refs/heads/release");
        assert_eq!(change.movement, GitRefMovement::FastForward);
        assert!(change.complete);
        assert_eq!(
            transition.events,
            vec![
                GitActivationEventClass::RefFastForward,
                GitActivationEventClass::CommitReachableAdded,
                GitActivationEventClass::PathModified,
            ]
        );
        assert_eq!(transition.reachability_deltas.len(), 1);
        assert_eq!(
            transition.reachability_deltas[0].ref_name,
            "refs/heads/release"
        );
        assert_eq!(transition.reachability_deltas[0].added_commits.len(), 1);
        assert!(transition.reachability_deltas[0].complete);

        let mut release_trigger = trigger(
            &initial,
            GitActivationEventClass::CommitReachableAdded,
            true,
        );
        release_trigger.ref_names = vec!["refs/heads/release".to_owned()];
        let mut future_trigger = release_trigger.clone();
        future_trigger.trigger_id = "fixture.future-reachability".to_owned();
        future_trigger.ref_names = vec!["refs/heads/future".to_owned()];
        let proposals =
            derive_activation_proposals(&transition, &[release_trigger.clone(), future_trigger])
                .unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].matched_ref_names, vec!["refs/heads/release"]);

        let (_, replay) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        assert_eq!(transition, replay);
        let advanced = cursor
            .advance(&transition, transition.transition_id.clone())
            .unwrap();
        assert_eq!(advanced.watched_refs, target.watched_refs);
        git(directory.path(), &["branch", "future", "main"]);
        let (created_target, created) = inspect.inspect_transition(&advanced).unwrap().unwrap();
        assert_eq!(created.watched_ref_changes.len(), 1);
        assert_eq!(created.watched_ref_changes[0].ref_name, "refs/heads/future");
        assert_eq!(
            created.watched_ref_changes[0].movement,
            GitRefMovement::Created
        );
        let mut future_created = trigger(&initial, GitActivationEventClass::RefCreated, true);
        future_created.ref_names = vec!["refs/heads/future".to_owned()];
        assert_eq!(
            derive_activation_proposals(&created, &[future_created])
                .unwrap()
                .remove(0)
                .matched_ref_names,
            vec!["refs/heads/future"]
        );
        let created_cursor = advanced
            .advance(&created, created.transition_id.clone())
            .unwrap();
        assert_eq!(created_cursor.watched_refs, created_target.watched_refs);
        git(directory.path(), &["branch", "-D", "future"]);
        let (_, deleted) = inspect
            .inspect_transition(&created_cursor)
            .unwrap()
            .unwrap();
        assert_eq!(deleted.watched_ref_changes.len(), 1);
        assert_eq!(deleted.watched_ref_changes[0].ref_name, "refs/heads/future");
        assert_eq!(
            deleted.watched_ref_changes[0].movement,
            GitRefMovement::Deleted
        );

        let mut tampered = transition;
        tampered.target_watched_refs[1].target_oid = None;
        assert!(matches!(
            tampered.verify(),
            Err(GitError::InvalidPollTransition)
        ));
        assert!(matches!(
            inspect.inspect_with_watched_refs(&["main".to_owned()]),
            Err(GitError::InvalidWatchedRefScope)
        ));
    }

    #[test]
    fn path_deltas_are_exact_directional_and_selectable_by_byte_prefix() {
        let directory = repository();
        fs::create_dir_all(directory.path().join("src")).unwrap();
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        fs::write(directory.path().join("src/modified"), "before\n").unwrap();
        fs::write(directory.path().join("docs/renamed-from"), "same bytes\n").unwrap();
        git(directory.path(), &["add", "."]);
        git(directory.path(), &["commit", "-q", "-m", "path baseline"]);

        let inspect = inspector(directory.path());
        let initial = inspect.inspect().unwrap().unwrap();
        let cursor =
            GitPollCursor::from_retained_snapshot(&initial, initial.snapshot_id.clone()).unwrap();
        fs::write(directory.path().join("src/modified"), "after\n").unwrap();
        fs::rename(
            directory.path().join("docs/renamed-from"),
            directory.path().join("docs/renamed-to"),
        )
        .unwrap();
        git(directory.path(), &["add", "--all"]);
        git(directory.path(), &["commit", "-q", "-m", "path changes"]);

        let (_, transition) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        assert_eq!(transition.path_deltas.len(), 1);
        let delta = &transition.path_deltas[0];
        assert_eq!(delta.ref_name, "HEAD");
        assert!(delta.complete);
        assert!(delta.omissions.is_empty());
        assert_eq!(delta.changes.len(), 3);
        let change = |path: &str| {
            delta
                .changes
                .iter()
                .find(|change| change.path.display == path)
                .unwrap()
        };
        assert_eq!(change("docs/renamed-from").kind, GitPathChangeKind::Deleted);
        assert_eq!(change("docs/renamed-to").kind, GitPathChangeKind::Added);
        assert_eq!(change("src/modified").kind, GitPathChangeKind::Modified);
        assert_eq!(
            change("docs/renamed-from").source_oid,
            change("docs/renamed-to").target_oid
        );
        assert!(
            transition
                .events
                .contains(&GitActivationEventClass::PathAdded)
        );
        assert!(
            transition
                .events
                .contains(&GitActivationEventClass::PathDeleted)
        );
        assert!(
            transition
                .events
                .contains(&GitActivationEventClass::PathModified)
        );

        let mut source_trigger = trigger(&initial, GitActivationEventClass::PathModified, true);
        source_trigger.ref_names = vec!["HEAD".to_owned()];
        source_trigger.path_prefixes = vec![PathIdentity::from_bytes(b"src/")];
        let proposal = derive_activation_proposals(&transition, &[source_trigger.clone()])
            .unwrap()
            .remove(0);
        assert_eq!(proposal.matched_ref_names, vec!["HEAD"]);
        assert_eq!(proposal.matched_path_changes.len(), 1);
        assert_eq!(
            proposal.matched_path_changes[0].path.display,
            "src/modified"
        );
        assert_eq!(
            proposal.matched_path_changes[0].kind,
            GitPathChangeKind::Modified
        );

        let (_, replay) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        assert_eq!(transition, replay);
        assert_eq!(
            vec![proposal],
            derive_activation_proposals(&replay, &[source_trigger]).unwrap()
        );

        let mut invalid_trigger = trigger(&initial, GitActivationEventClass::RefFastForward, true);
        invalid_trigger.path_prefixes = vec![PathIdentity::from_bytes(b"src/")];
        assert!(matches!(
            invalid_trigger.verify(),
            Err(GitError::InvalidActivationTrigger)
        ));
        let mut tampered = transition;
        tampered.path_deltas[0].changes[0].path.display = "different".to_owned();
        assert!(matches!(
            tampered.verify(),
            Err(GitError::InvalidPollTransition | GitError::InvalidSnapshot)
        ));
    }

    #[test]
    fn path_change_bounds_are_explicit_and_gate_complete_triggers() {
        let directory = repository();
        let mut inspect = inspector(directory.path());
        inspect.limits.max_path_changes_per_ref = 1;
        let initial = inspect.inspect().unwrap().unwrap();
        let cursor =
            GitPollCursor::from_retained_snapshot(&initial, initial.snapshot_id.clone()).unwrap();
        fs::write(directory.path().join("added-a"), "a\n").unwrap();
        fs::write(directory.path().join("added-b"), "b\n").unwrap();
        git(directory.path(), &["add", "added-a", "added-b"]);
        git(directory.path(), &["commit", "-q", "-m", "two paths"]);

        let (_, transition) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        let delta = &transition.path_deltas[0];
        assert_eq!(delta.changes.len(), 1);
        assert_eq!(delta.max_changes, 1);
        assert!(!delta.complete);
        assert_eq!(
            delta.omissions,
            vec!["HEAD path changes exceed the 1-change limit"]
        );
        let complete = trigger(&initial, GitActivationEventClass::PathAdded, true);
        assert!(
            derive_activation_proposals(&transition, &[complete])
                .unwrap()
                .is_empty()
        );
        let partial = trigger(&initial, GitActivationEventClass::PathAdded, false);
        let proposal = derive_activation_proposals(&transition, &[partial])
            .unwrap()
            .remove(0);
        assert!(!proposal.complete);
        assert_eq!(proposal.matched_path_changes.len(), 1);
        assert_eq!(proposal.omissions, delta.omissions);
    }

    #[cfg(unix)]
    #[test]
    fn path_deltas_and_prefixes_preserve_non_utf8_bytes() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};

        let directory = repository();
        let inspect = inspector(directory.path());
        let initial = inspect.inspect().unwrap().unwrap();
        let cursor =
            GitPollCursor::from_retained_snapshot(&initial, initial.snapshot_id.clone()).unwrap();
        fs::create_dir(directory.path().join("weird")).unwrap();
        let filename = OsString::from_vec(vec![0xff]);
        fs::write(directory.path().join("weird").join(filename), "bytes\n").unwrap();
        git(directory.path(), &["add", "--all"]);
        git(directory.path(), &["commit", "-q", "-m", "non utf8 path"]);

        let (_, transition) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        let path = &transition.path_deltas[0].changes[0].path;
        assert_eq!(path.decoded_bytes().unwrap(), b"weird/\xff");
        assert_eq!(path.encoding, "base64url");
        let mut path_trigger = trigger(&initial, GitActivationEventClass::PathAdded, true);
        path_trigger.path_prefixes = vec![PathIdentity::from_bytes(b"weird/")];
        let proposal = derive_activation_proposals(&transition, &[path_trigger])
            .unwrap()
            .remove(0);
        assert_eq!(proposal.matched_path_changes[0].path, *path);
    }

    #[cfg(unix)]
    #[test]
    fn path_deltas_distinguish_type_changes_from_mode_only_modifications() {
        use std::os::unix::{fs::PermissionsExt, fs::symlink};

        let directory = repository();
        fs::write(directory.path().join("typed"), "target\n").unwrap();
        fs::write(directory.path().join("mode-only"), "same bytes\n").unwrap();
        git(directory.path(), &["add", "typed", "mode-only"]);
        git(directory.path(), &["commit", "-q", "-m", "mode baseline"]);
        let inspect = inspector(directory.path());
        let initial = inspect.inspect().unwrap().unwrap();
        let cursor =
            GitPollCursor::from_retained_snapshot(&initial, initial.snapshot_id.clone()).unwrap();

        fs::remove_file(directory.path().join("typed")).unwrap();
        symlink("target", directory.path().join("typed")).unwrap();
        let mode_path = directory.path().join("mode-only");
        let mut permissions = fs::metadata(&mode_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&mode_path, permissions).unwrap();
        git(directory.path(), &["add", "--all"]);
        git(directory.path(), &["commit", "-q", "-m", "mode changes"]);

        let (_, transition) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        let changes = &transition.path_deltas[0].changes;
        let mode_only = changes
            .iter()
            .find(|change| change.path.display == "mode-only")
            .unwrap();
        assert_eq!(mode_only.kind, GitPathChangeKind::Modified);
        assert_eq!(mode_only.source_oid, mode_only.target_oid);
        assert_ne!(mode_only.source_mode, mode_only.target_mode);
        let typed = changes
            .iter()
            .find(|change| change.path.display == "typed")
            .unwrap();
        assert_eq!(typed.kind, GitPathChangeKind::TypeChanged);
        assert_ne!(typed.source_mode, typed.target_mode);
        assert!(
            transition
                .events
                .contains(&GitActivationEventClass::PathModified)
        );
        assert!(
            transition
                .events
                .contains(&GitActivationEventClass::PathTypeChanged)
        );
    }

    #[test]
    fn reachable_commit_bounds_are_explicit_and_gate_complete_triggers() {
        let directory = repository();
        let mut inspect = inspector(directory.path());
        inspect.limits.max_reachable_commits_per_direction = 1;
        let initial = inspect.inspect().unwrap().unwrap();
        let cursor =
            GitPollCursor::from_retained_snapshot(&initial, initial.snapshot_id.clone()).unwrap();
        for revision in 2..=4 {
            fs::write(
                directory.path().join("tracked"),
                format!("revision {revision}\n"),
            )
            .unwrap();
            git(directory.path(), &["add", "tracked"]);
            git(
                directory.path(),
                &["commit", "-q", "-m", &format!("revision {revision}")],
            );
        }

        let (_, transition) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        let reachability = &transition.reachability_deltas[0];
        assert_eq!(reachability.ref_name, "HEAD");
        assert_eq!(reachability.added_commits.len(), 1);
        assert!(reachability.removed_commits.is_empty());
        assert!(!reachability.complete);
        assert_eq!(
            reachability.omissions,
            vec!["HEAD added reachability exceeds the 1-commit per-direction limit"]
        );
        assert_eq!(transition.omissions, reachability.omissions);

        let complete = trigger(
            &initial,
            GitActivationEventClass::CommitReachableAdded,
            true,
        );
        assert!(
            derive_activation_proposals(&transition, &[complete])
                .unwrap()
                .is_empty()
        );
        let partial = trigger(
            &initial,
            GitActivationEventClass::CommitReachableAdded,
            false,
        );
        let proposal = derive_activation_proposals(&transition, &[partial])
            .unwrap()
            .remove(0);
        assert!(!proposal.complete);
        assert_eq!(proposal.matched_ref_names, vec!["HEAD"]);
        assert_eq!(proposal.omissions, reachability.omissions);

        let mut tampered = transition;
        tampered.reachability_deltas[0].added_commits.clear();
        assert!(matches!(
            tampered.verify(),
            Err(GitError::InvalidPollTransition)
        ));
    }

    #[test]
    fn shallow_reachability_remains_partial_even_when_known_commits_are_retained() {
        let source = repository();
        fs::write(source.path().join("tracked"), "two\n").unwrap();
        git(source.path(), &["add", "tracked"]);
        git(source.path(), &["commit", "-q", "-m", "second"]);
        let clone_root = TempDir::new().unwrap();
        let shallow_path = clone_root.path().join("shallow");
        let status = Command::new("git")
            .args([
                "clone",
                "-q",
                "--depth=1",
                &format!("file://{}", source.path().display()),
                shallow_path.to_str().unwrap(),
            ])
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .status()
            .unwrap();
        assert!(status.success());
        git(&shallow_path, &["config", "user.name", "Rey Test"]);
        git(
            &shallow_path,
            &["config", "user.email", "rey@example.invalid"],
        );

        let inspect = inspector(&shallow_path);
        let initial = inspect.inspect().unwrap().unwrap();
        assert!(initial.shallow);
        let cursor =
            GitPollCursor::from_retained_snapshot(&initial, initial.snapshot_id.clone()).unwrap();
        fs::write(shallow_path.join("tracked"), "three\n").unwrap();
        git(&shallow_path, &["add", "tracked"]);
        git(&shallow_path, &["commit", "-q", "-m", "third"]);

        let (_, transition) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        assert_eq!(transition.head_movement, GitRefMovement::Unknown);
        let reachability = &transition.reachability_deltas[0];
        assert_eq!(reachability.added_commits.len(), 1);
        assert!(reachability.removed_commits.is_empty());
        assert!(!reachability.complete);
        assert_eq!(
            reachability.omissions,
            vec!["HEAD reachability is incomplete because repository history is shallow"]
        );
        assert!(
            transition
                .events
                .contains(&GitActivationEventClass::CommitReachableAdded)
        );
        let partial = trigger(
            &initial,
            GitActivationEventClass::CommitReachableAdded,
            false,
        );
        let proposal = derive_activation_proposals(&transition, &[partial])
            .unwrap()
            .remove(0);
        assert!(!proposal.complete);
        assert_eq!(proposal.omissions, reachability.omissions);
    }

    #[test]
    fn poll_distinguishes_rewind_from_rewrite() {
        let directory = repository();
        let inspect = inspector(directory.path());
        let initial = inspect.inspect().unwrap().unwrap();
        let initial_oid = initial.head.commit_oid.clone().unwrap();

        fs::write(directory.path().join("tracked"), "two\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        git(directory.path(), &["commit", "-q", "-m", "second"]);
        let second = inspect.inspect().unwrap().unwrap();
        let cursor =
            GitPollCursor::from_retained_snapshot(&second, second.snapshot_id.clone()).unwrap();

        git(directory.path(), &["reset", "--hard", "-q", &initial_oid]);
        let (_, rewind) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        assert_eq!(rewind.head_movement, GitRefMovement::Rewound);
        assert_eq!(
            rewind.events,
            vec![
                GitActivationEventClass::RefRewound,
                GitActivationEventClass::CommitReachableRemoved,
                GitActivationEventClass::PathModified,
                GitActivationEventClass::IndexChanged,
            ]
        );

        fs::write(directory.path().join("tracked"), "replacement\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        git(directory.path(), &["commit", "-q", "-m", "replacement"]);
        let (_, rewrite) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        assert_eq!(rewrite.head_movement, GitRefMovement::Rewritten);
        assert_eq!(
            rewrite.events,
            vec![
                GitActivationEventClass::RefRewritten,
                GitActivationEventClass::CommitReachableAdded,
                GitActivationEventClass::CommitReachableRemoved,
                GitActivationEventClass::PathModified,
                GitActivationEventClass::IndexChanged,
            ]
        );
    }

    #[test]
    fn poll_keeps_unborn_creation_and_ref_deletion_explicit() {
        let directory = TempDir::new().unwrap();
        git(directory.path(), &["init", "-q"]);
        git(directory.path(), &["config", "user.name", "Rey Test"]);
        git(
            directory.path(),
            &["config", "user.email", "rey@example.invalid"],
        );
        let inspect = inspector(directory.path());
        let unborn = inspect.inspect().unwrap().unwrap();
        let unborn_cursor =
            GitPollCursor::from_retained_snapshot(&unborn, unborn.snapshot_id.clone()).unwrap();

        fs::write(directory.path().join("tracked"), "one\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        git(directory.path(), &["commit", "-q", "-m", "initial"]);
        let (created_snapshot, created) =
            inspect.inspect_transition(&unborn_cursor).unwrap().unwrap();
        assert_eq!(created.head_movement, GitRefMovement::Created);
        assert!(
            created
                .events
                .contains(&GitActivationEventClass::RefCreated)
        );
        assert_eq!(created.path_deltas[0].changes.len(), 1);
        assert_eq!(
            created.path_deltas[0].changes[0].kind,
            GitPathChangeKind::Added
        );

        let created_cursor = GitPollCursor::from_retained_snapshot(
            &created_snapshot,
            created_snapshot.snapshot_id.clone(),
        )
        .unwrap();
        let selected_ref = created_snapshot.head.symbolic_ref.as_deref().unwrap();
        git(directory.path(), &["update-ref", "-d", selected_ref]);
        let (_, deleted) = inspect
            .inspect_transition(&created_cursor)
            .unwrap()
            .unwrap();
        assert_eq!(deleted.head_movement, GitRefMovement::Deleted);
        assert!(
            deleted
                .events
                .contains(&GitActivationEventClass::RefDeleted)
        );
        assert_eq!(deleted.path_deltas[0].changes.len(), 1);
        assert_eq!(
            deleted.path_deltas[0].changes[0].kind,
            GitPathChangeKind::Deleted
        );
    }

    #[test]
    fn complete_semantic_index_change_admits_a_complete_trigger() {
        let directory = repository();
        let inspect = inspector(directory.path());
        let initial = inspect.inspect().unwrap().unwrap();
        let cursor =
            GitPollCursor::from_retained_snapshot(&initial, initial.snapshot_id.clone()).unwrap();

        git(directory.path(), &["update-index", "--refresh"]);
        let (_, refresh) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        assert!(refresh.events.is_empty());
        assert_eq!(refresh.source_snapshot_id, refresh.target_snapshot_id);

        fs::write(directory.path().join("tracked"), "staged\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        let (_, changed) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        assert_eq!(changed.events, vec![GitActivationEventClass::IndexChanged]);
        assert!(changed.source_index_complete);
        assert!(changed.target_index_complete);
        assert!(changed.omissions.is_empty());

        let complete_trigger = trigger(&initial, GitActivationEventClass::IndexChanged, true);
        let proposals = derive_activation_proposals(&changed, &[complete_trigger]).unwrap();
        assert_eq!(proposals.len(), 1);
        assert!(proposals[0].complete);
        assert!(proposals[0].omissions.is_empty());
    }

    #[test]
    fn poll_cursor_and_activation_tampering_fail_closed() {
        let directory = repository();
        let inspect = inspector(directory.path());
        let initial = inspect.inspect().unwrap().unwrap();
        let cursor =
            GitPollCursor::from_retained_snapshot(&initial, initial.snapshot_id.clone()).unwrap();
        let mut tampered_cursor = cursor.clone();
        tampered_cursor.shallow = !tampered_cursor.shallow;
        assert!(matches!(
            tampered_cursor.verify(),
            Err(GitError::InvalidPollCursor)
        ));

        fs::write(directory.path().join("tracked"), "two\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        git(directory.path(), &["commit", "-q", "-m", "second"]);
        let (_, transition) = inspect.inspect_transition(&cursor).unwrap().unwrap();
        let mut tampered_transition = transition.clone();
        tampered_transition.head_movement = GitRefMovement::Rewound;
        assert!(matches!(
            tampered_transition.verify(),
            Err(GitError::InvalidPollTransition)
        ));
        let proposal = derive_activation_proposals(
            &transition,
            &[trigger(
                &initial,
                GitActivationEventClass::RefFastForward,
                true,
            )],
        )
        .unwrap()
        .remove(0);
        let mut tampered_proposal = proposal;
        tampered_proposal.scenario_ids = vec!["different".to_owned()];
        assert!(matches!(
            tampered_proposal.verify(),
            Err(GitError::InvalidActivationProposal)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn inspection_does_not_change_the_index() {
        use std::os::unix::fs::PermissionsExt;

        let directory = repository();
        let index_path = directory.path().join(".git/index");
        let marker = directory.path().join("fsmonitor-invoked");
        let monitor = directory.path().join("fsmonitor-hook");
        fs::write(
            &monitor,
            format!("#!/bin/sh\nprintf invoked > '{}'\n", marker.display()),
        )
        .unwrap();
        fs::set_permissions(&monitor, fs::Permissions::from_mode(0o755)).unwrap();
        git(
            directory.path(),
            &["config", "core.fsmonitor", monitor.to_str().unwrap()],
        );
        let before = fs::read(&index_path).unwrap();
        inspector(directory.path()).inspect().unwrap().unwrap();
        assert_eq!(fs::read(index_path).unwrap(), before);
        assert!(!marker.exists());
    }
}
