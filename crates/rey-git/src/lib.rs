#![forbid(unsafe_code)]

use std::{
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitLimits {
    pub total_timeout_ms: u64,
    pub command_timeout_ms: u64,
    pub max_capture_bytes: u64,
    pub max_index_entries: u64,
}

impl Default for GitLimits {
    fn default() -> Self {
        Self {
            total_timeout_ms: 5_000,
            command_timeout_ms: 2_000,
            max_capture_bytes: 4 * 1_024 * 1_024,
            max_index_entries: 10_000,
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
            ]
            .contains(&0)
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
                "inspect_recent_commits".to_owned(),
                "inspect_repository".to_owned(),
                "inspect_repository_status".to_owned(),
            ],
            enforced_limits: vec![
                "capture_bytes".to_owned(),
                "direct_argv".to_owned(),
                "no_optional_locks".to_owned(),
                "wall_timeout".to_owned(),
            ],
            unsupported_limits: vec![
                "complete_index_flags".to_owned(),
                "process_sandbox".to_owned(),
            ],
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
            Self::IndexChanged => "index.changed",
            Self::IndexConflicted => "index.conflicted",
        }
    }

    const fn needs_complete_head(self) -> bool {
        matches!(
            self,
            Self::HeadRefChanged
                | Self::RefCreated
                | Self::RefDeleted
                | Self::RefFastForward
                | Self::RefRewound
                | Self::RefRewritten
                | Self::RefUnknown
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

impl GitPollTransition {
    fn derive(
        cursor: &GitPollCursor,
        target: &GitSnapshot,
        head_movement: GitRefMovement,
    ) -> Result<Self, GitError> {
        cursor.verify()?;
        target.verify()?;
        if cursor.repository_id != target.repository_id
            || cursor.worktree_id != target.worktree_id
            || cursor.object_format != target.object_format
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
        if !cursor.index_complete || !target_index_complete {
            omissions.push(
                "semantic index comparison omits unsupported index flags or extensions".to_owned(),
            );
        }
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
        if !self.source_index_complete || !self.target_index_complete {
            expected_omissions.push(
                "semantic index comparison omits unsupported index flags or extensions".to_owned(),
            );
        }
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

    fn event_complete(&self, event: GitActivationEventClass) -> bool {
        if event.needs_complete_head() {
            self.head_complete
        } else {
            self.source_index_complete && self.target_index_complete
        }
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
    pub complete: bool,
    pub omissions: Vec<String>,
    pub workload_id: String,
    pub graph: ContractIdentity,
    pub scenario_ids: Vec<String>,
    pub budget: GitActivationBudget,
    pub authority: String,
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
        let matched_events = trigger
            .event_classes
            .iter()
            .filter(|event| transition.events.contains(event))
            .copied()
            .collect::<Vec<_>>();
        if matched_events.is_empty() {
            return Ok(None);
        }
        let complete = matched_events
            .iter()
            .all(|event| transition.event_complete(*event));
        if trigger.require_complete && !complete {
            return Ok(None);
        }
        let omissions = if complete {
            Vec::new()
        } else {
            transition.omissions.clone()
        };
        let mut proposal = Self {
            schema: GIT_ACTIVATION_PROPOSAL_SCHEMA.to_owned(),
            activation_id: SemanticHasher::new("rey.git-activation-proposal.pending.v1").finish(),
            trigger_id: trigger.trigger_id.clone(),
            trigger_revision: trigger.revision,
            transition_id: transition.transition_id.clone(),
            source_snapshot_id: transition.source_snapshot_id.clone(),
            target_snapshot_id: transition.target_snapshot_id.clone(),
            matched_events,
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
            || self.complete != self.omissions.is_empty()
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

impl GitInspector {
    pub fn inspect(&self) -> Result<Option<GitSnapshot>, GitError> {
        self.inspect_until(Instant::now() + Duration::from_millis(self.limits.total_timeout_ms))
    }

    pub fn inspect_until(&self, outer_deadline: Instant) -> Result<Option<GitSnapshot>, GitError> {
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
        snapshot_hasher
            .add_optional_str(index.as_ref().map(|summary| summary.entry_digest.as_str()));
        snapshot_hasher.add_bool(complete);
        snapshot_hasher.add_u64(self.limits.total_timeout_ms);
        snapshot_hasher.add_u64(self.limits.command_timeout_ms);
        snapshot_hasher.add_u64(self.limits.max_capture_bytes);
        snapshot_hasher.add_u64(self.limits.max_index_entries);

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
        let Some(target) = self.inspect_until(deadline)? else {
            return Ok(None);
        };
        if cursor.repository_id != target.repository_id
            || cursor.worktree_id != target.worktree_id
            || cursor.object_format != target.object_format
        {
            return Err(GitError::RepositoryIdentityChanged);
        }
        let movement = self.classify_head_movement(cursor, &target, deadline)?;
        let transition = GitPollTransition::derive(cursor, &target, movement)?;
        Ok(Some((target, transition)))
    }

    fn classify_head_movement(
        &self,
        cursor: &GitPollCursor,
        target: &GitSnapshot,
        deadline: Instant,
    ) -> Result<GitRefMovement, GitError> {
        let (Some(source_oid), Some(target_oid)) = (
            cursor.head.commit_oid.as_deref(),
            target.head.commit_oid.as_deref(),
        ) else {
            return Ok(
                match (
                    cursor.head.commit_oid.is_some(),
                    target.head.commit_oid.is_some(),
                ) {
                    (false, false) => GitRefMovement::Unchanged,
                    (false, true) => GitRefMovement::Created,
                    (true, false) => GitRefMovement::Deleted,
                    (true, true) => unreachable!("both commit ids were destructured above"),
                },
            );
        };
        if source_oid == target_oid {
            return Ok(GitRefMovement::Unchanged);
        }
        if cursor.shallow || target.shallow {
            return Ok(GitRefMovement::Unknown);
        }
        let workspace = fs::canonicalize(&self.workspace).map_err(|source| GitError::Path {
            path: self.workspace.clone(),
            source,
        })?;
        match self.is_ancestor(&workspace, source_oid, target_oid, deadline)? {
            Some(true) => Ok(GitRefMovement::FastForward),
            None => Ok(GitRefMovement::Unknown),
            Some(false) => match self.is_ancestor(&workspace, target_oid, source_oid, deadline)? {
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
            .git(workspace, &["ls-files", "--stage", "-z"], deadline)?
            .success("read Git index entries")?;
        let mut entries = output
            .split(|byte| *byte == 0)
            .filter(|row| !row.is_empty());
        let mut count = 0_u64;
        let mut logical_entries = Vec::new();
        let mut hasher = SemanticHasher::new("rey.git-index-entries.v1");
        for row in &mut entries {
            count = count.saturating_add(1);
            if count > self.limits.max_index_entries {
                return Err(GitError::IndexEntryLimit(self.limits.max_index_entries));
            }
            let separator = row
                .iter()
                .position(|byte| *byte == b'\t')
                .ok_or(GitError::MalformedIndex("entry is missing path separator"))?;
            let (header, path_with_separator) = row.split_at(separator);
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
                || !oid.bytes().all(|byte| byte.is_ascii_hexdigit())
                || !matches!(stage, "0" | "1" | "2" | "3")
            {
                return Err(GitError::MalformedIndex("entry header has invalid fields"));
            }
            hasher.add_str(mode);
            hasher.add_str(object_format);
            hasher.add_str(oid);
            hasher.add_str(stage);
            hasher.add_bytes(path);
            logical_entries.push(GitIndexEntry {
                mode: mode.to_owned(),
                object_format: object_format.to_owned(),
                object_oid: oid.to_owned(),
                stage: stage
                    .parse()
                    .map_err(|_| GitError::MalformedIndex("entry stage is not numeric"))?,
                path: PathIdentity::from_bytes(path),
            });
        }
        hasher.add_u64(count);
        Ok(GitIndexSummary {
            entry_digest: hasher.finish(),
            entry_count: count,
            entries: logical_entries,
            complete: false,
            omitted_semantics: vec![
                "assume_unchanged".to_owned(),
                "intent_to_add".to_owned(),
                "skip_worktree".to_owned(),
                "sparse_index".to_owned(),
                "split_index".to_owned(),
            ],
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
        GitActivationTrigger, GitError, GitInspector, GitLimits, GitPollCursor, GitRefMovement,
        derive_activation_proposals, parse_repository_status,
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
                GitActivationEventClass::IndexChanged,
            ]
        );
        assert!(transition.head_complete);
        assert!(!transition.target_index_complete);

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
    }

    #[test]
    fn partial_semantic_index_change_requires_an_explicit_partial_trigger() {
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
        assert!(!changed.target_index_complete);

        let complete_trigger = trigger(&initial, GitActivationEventClass::IndexChanged, true);
        assert!(
            derive_activation_proposals(&changed, &[complete_trigger])
                .unwrap()
                .is_empty()
        );
        let partial_trigger = trigger(&initial, GitActivationEventClass::IndexChanged, false);
        let proposals = derive_activation_proposals(&changed, &[partial_trigger]).unwrap();
        assert_eq!(proposals.len(), 1);
        assert!(!proposals[0].complete);
        assert!(!proposals[0].omissions.is_empty());
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
