use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use rey_core::SemanticDigest;
use rey_git::{
    GitActivationProposal, GitActivationTrigger, GitError, GitPollCursor, GitPollTransition,
    GitSnapshot, derive_activation_proposals,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const LOCAL_GIT_STATE_SCHEMA: &str = "rey.local-git-state.v1";
pub const GIT_POLL_RECORD_SCHEMA: &str = "rey.git-poll-record.v1";
pub const GIT_OPERATOR_STATUS_SCHEMA: &str = "rey.git-operator-status.v1";
pub const GIT_POLL_OUTCOME_SCHEMA: &str = "rey.git-poll-outcome.v1";
pub const GIT_ACKNOWLEDGEMENT_SCHEMA: &str = "rey.git-acknowledgement.v1";
const STATE_FILE_NAME: &str = "state.json";
const MAX_GIT_STATE_BYTES: u64 = 16 * 1_024 * 1_024;
const MAX_RETAINED_GIT_TRANSITIONS: usize = 1_024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitOperatorStatus {
    pub schema: String,
    pub observed_snapshot: GitSnapshot,
    pub state: LocalGitState,
    pub changed_since_cursor: Option<bool>,
    pub repository_authority: String,
    pub next: String,
}

impl GitOperatorStatus {
    pub fn new(
        observed_snapshot: GitSnapshot,
        state: LocalGitState,
    ) -> Result<Self, LocalGitStateError> {
        observed_snapshot.verify()?;
        state.verify()?;
        let changed_since_cursor = state
            .cursor
            .as_ref()
            .map(|cursor| cursor.snapshot_id != observed_snapshot.snapshot_id);
        Ok(Self {
            schema: GIT_OPERATOR_STATUS_SCHEMA.to_owned(),
            observed_snapshot,
            changed_since_cursor,
            state,
            repository_authority: "read_only_observation; no Git mutation or workload execution"
                .to_owned(),
            next: "Initialize a retained cursor, poll a transition, or acknowledge exact retained transition evidence"
                .to_owned(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitPollOutcome {
    pub schema: String,
    pub changed: bool,
    pub retained: bool,
    pub record: GitPollRecord,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitAcknowledgement {
    pub schema: String,
    pub acknowledged_transition_id: SemanticDigest,
    pub cursor: GitPollCursor,
    pub retained_transition_count: u64,
    pub authority: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitPollRecord {
    pub schema: String,
    pub target_snapshot: GitSnapshot,
    pub transition: GitPollTransition,
    pub triggers: Vec<GitActivationTrigger>,
    pub proposals: Vec<GitActivationProposal>,
}

impl GitPollRecord {
    pub fn new(
        target_snapshot: GitSnapshot,
        transition: GitPollTransition,
        triggers: Vec<GitActivationTrigger>,
    ) -> Result<Self, LocalGitStateError> {
        let proposals = derive_activation_proposals(&transition, &triggers)?;
        let record = Self {
            schema: GIT_POLL_RECORD_SCHEMA.to_owned(),
            target_snapshot,
            transition,
            triggers,
            proposals,
        };
        record.verify()?;
        Ok(record)
    }

    pub fn verify(&self) -> Result<(), LocalGitStateError> {
        self.target_snapshot.verify()?;
        self.transition.verify()?;
        for trigger in &self.triggers {
            trigger.verify()?;
        }
        for proposal in &self.proposals {
            proposal.verify()?;
        }
        if self.schema != GIT_POLL_RECORD_SCHEMA
            || self.transition.target_snapshot_id != self.target_snapshot.snapshot_id
            || self.transition.repository_id != self.target_snapshot.repository_id
            || self.transition.worktree_id != self.target_snapshot.worktree_id
            || self.transition.object_format != self.target_snapshot.object_format
            || derive_activation_proposals(&self.transition, &self.triggers)? != self.proposals
        {
            return Err(LocalGitStateError::InvalidState);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalGitState {
    pub schema: String,
    pub cursor_snapshot: Option<GitSnapshot>,
    pub cursor: Option<GitPollCursor>,
    pub pending: Option<GitPollRecord>,
    pub retained_polls: Vec<GitPollRecord>,
}

impl Default for LocalGitState {
    fn default() -> Self {
        Self {
            schema: LOCAL_GIT_STATE_SCHEMA.to_owned(),
            cursor_snapshot: None,
            cursor: None,
            pending: None,
            retained_polls: Vec::new(),
        }
    }
}

impl LocalGitState {
    pub fn verify(&self) -> Result<(), LocalGitStateError> {
        if self.schema != LOCAL_GIT_STATE_SCHEMA
            || self.retained_polls.len() > MAX_RETAINED_GIT_TRANSITIONS
            || self.cursor.is_some() != self.cursor_snapshot.is_some()
        {
            return Err(LocalGitStateError::InvalidState);
        }
        let (Some(cursor), Some(snapshot)) = (&self.cursor, &self.cursor_snapshot) else {
            if self.pending.is_some() || !self.retained_polls.is_empty() {
                return Err(LocalGitStateError::InvalidState);
            }
            return Ok(());
        };
        cursor.verify()?;
        snapshot.verify()?;
        if cursor.snapshot_id != snapshot.snapshot_id
            || cursor.repository_id != snapshot.repository_id
            || cursor.worktree_id != snapshot.worktree_id
            || cursor.object_format != snapshot.object_format
        {
            return Err(LocalGitStateError::InvalidState);
        }
        let mut replayed_cursor = self.retained_polls.first().map(|poll| {
            let transition = &poll.transition;
            GitPollCursor {
                schema: rey_git::GIT_POLL_CURSOR_SCHEMA.to_owned(),
                cursor_id: transition.source_cursor_id.clone(),
                repository_id: transition.repository_id.clone(),
                worktree_id: transition.worktree_id.clone(),
                snapshot_id: transition.source_snapshot_id.clone(),
                object_format: transition.object_format.clone(),
                shallow: transition.source_shallow,
                head: transition.source_head.clone(),
                index_digest: transition.source_index_digest.clone(),
                index_complete: transition.source_index_complete,
                index_conflicted: transition.source_index_conflicted,
                provider_revision: rey_environment::LOCAL_PROVIDER_REVISION,
                retained_evidence_id: transition.source_snapshot_id.clone(),
            }
        });
        for poll in &self.retained_polls {
            poll.verify()?;
            let transition = &poll.transition;
            transition.verify()?;
            let expected = replayed_cursor
                .as_ref()
                .ok_or(LocalGitStateError::InvalidState)?;
            expected.verify()?;
            if expected.cursor_id != transition.source_cursor_id
                || expected.snapshot_id != transition.source_snapshot_id
            {
                return Err(LocalGitStateError::InvalidState);
            }
            replayed_cursor = Some(expected.advance(transition, transition.transition_id.clone())?);
        }
        if let Some(replayed) = replayed_cursor {
            if &replayed != cursor {
                return Err(LocalGitStateError::InvalidState);
            }
        } else if cursor.retained_evidence_id != snapshot.snapshot_id {
            return Err(LocalGitStateError::InvalidState);
        }
        if let Some(pending) = &self.pending {
            pending.verify()?;
            if pending.transition.source_cursor_id != cursor.cursor_id
                || pending.transition.source_snapshot_id != cursor.snapshot_id
            {
                return Err(LocalGitStateError::InvalidState);
            }
        }
        Ok(())
    }

    pub fn acknowledged_activation(
        &self,
        activation_id: &str,
    ) -> Result<GitActivationProposal, LocalGitStateError> {
        self.verify()?;
        if let Some(proposal) = self
            .retained_polls
            .iter()
            .flat_map(|poll| &poll.proposals)
            .find(|proposal| proposal.activation_id.as_str() == activation_id)
        {
            return Ok(proposal.clone());
        }
        if self.pending.as_ref().is_some_and(|pending| {
            pending
                .proposals
                .iter()
                .any(|proposal| proposal.activation_id.as_str() == activation_id)
        }) {
            return Err(LocalGitStateError::ActivationNotAcknowledged(
                activation_id.to_owned(),
            ));
        }
        Err(LocalGitStateError::UnknownActivation(
            activation_id.to_owned(),
        ))
    }
}

#[derive(Clone, Debug)]
pub struct LocalGitStore {
    directory: PathBuf,
}

impl LocalGitStore {
    #[must_use]
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    #[must_use]
    pub fn default_for_workspace(workspace: &Path) -> Self {
        Self::new(workspace.join(".rey").join("git"))
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.directory.join(STATE_FILE_NAME)
    }

    pub fn load(&self) -> Result<LocalGitState, LocalGitStateError> {
        self.verify_directory_boundary()?;
        let path = self.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(LocalGitState::default());
            }
            Err(source) => return Err(LocalGitStateError::Read { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(LocalGitStateError::UnsafePath(path));
        }
        if metadata.len() > MAX_GIT_STATE_BYTES {
            return Err(LocalGitStateError::ByteLimit(MAX_GIT_STATE_BYTES));
        }
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|source| LocalGitStateError::Read {
                path: path.clone(),
                source,
            })?
            .take(MAX_GIT_STATE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| LocalGitStateError::Read {
                path: path.clone(),
                source,
            })?;
        if bytes.len() as u64 > MAX_GIT_STATE_BYTES {
            return Err(LocalGitStateError::ByteLimit(MAX_GIT_STATE_BYTES));
        }
        let state = serde_json::from_slice::<LocalGitState>(&bytes).map_err(|source| {
            LocalGitStateError::Json {
                path: path.clone(),
                source,
            }
        })?;
        state.verify()?;
        Ok(state)
    }

    pub fn initialize(&self, snapshot: GitSnapshot) -> Result<LocalGitState, LocalGitStateError> {
        let mut state = self.load()?;
        if state.cursor.is_some() {
            return Err(LocalGitStateError::AlreadyInitialized);
        }
        let cursor =
            GitPollCursor::from_retained_snapshot(&snapshot, snapshot.snapshot_id.clone())?;
        state.cursor_snapshot = Some(snapshot);
        state.cursor = Some(cursor);
        self.save(&state)?;
        Ok(state)
    }

    pub fn retain_poll(&self, record: GitPollRecord) -> Result<LocalGitState, LocalGitStateError> {
        let mut state = self.load()?;
        let cursor = state
            .cursor
            .as_ref()
            .ok_or(LocalGitStateError::Uninitialized)?;
        if record.transition.source_cursor_id != cursor.cursor_id {
            return Err(LocalGitStateError::StalePoll);
        }
        if let Some(pending) = &state.pending {
            if pending == &record {
                return Ok(state);
            }
            return Err(LocalGitStateError::PendingPoll(
                pending.transition.transition_id.clone(),
            ));
        }
        state.pending = Some(record);
        self.save(&state)?;
        Ok(state)
    }

    pub fn acknowledge(
        &self,
        expected_transition_id: &str,
    ) -> Result<LocalGitState, LocalGitStateError> {
        let mut state = self.load()?;
        let cursor = state
            .cursor
            .as_ref()
            .ok_or(LocalGitStateError::Uninitialized)?;
        let pending = state
            .pending
            .take()
            .ok_or(LocalGitStateError::NoPendingPoll)?;
        if pending.transition.transition_id.as_str() != expected_transition_id {
            return Err(LocalGitStateError::StaleAcknowledgement {
                expected: pending.transition.transition_id,
                actual: expected_transition_id.to_owned(),
            });
        }
        if state.retained_polls.len() >= MAX_RETAINED_GIT_TRANSITIONS {
            return Err(LocalGitStateError::TransitionLimit(
                MAX_RETAINED_GIT_TRANSITIONS,
            ));
        }
        let advanced = cursor.advance(
            &pending.transition,
            pending.transition.transition_id.clone(),
        )?;
        state.cursor_snapshot = Some(pending.target_snapshot.clone());
        state.retained_polls.push(pending);
        state.cursor = Some(advanced);
        self.save(&state)?;
        Ok(state)
    }

    pub fn save(&self, state: &LocalGitState) -> Result<(), LocalGitStateError> {
        state.verify()?;
        let mut bytes =
            serde_json::to_vec_pretty(state).map_err(|source| LocalGitStateError::Json {
                path: self.path(),
                source,
            })?;
        bytes.push(b'\n');
        if bytes.len() as u64 > MAX_GIT_STATE_BYTES {
            return Err(LocalGitStateError::ByteLimit(MAX_GIT_STATE_BYTES));
        }
        self.prepare_directory()?;
        let target = self.path();
        if let Ok(metadata) = fs::symlink_metadata(&target)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(LocalGitStateError::UnsafePath(target));
        }
        let (temporary, mut file) = self.create_temporary()?;
        let publication = (|| {
            file.write_all(&bytes).and_then(|()| file.flush())?;
            drop(file);
            fs::rename(&temporary, &target)
        })();
        if let Err(source) = publication {
            let _ = fs::remove_file(&temporary);
            return Err(LocalGitStateError::Write {
                path: target,
                source,
            });
        }
        Ok(())
    }

    fn prepare_directory(&self) -> Result<(), LocalGitStateError> {
        self.verify_directory_boundary()?;
        match fs::symlink_metadata(&self.directory) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                Err(LocalGitStateError::UnsafePath(self.directory.clone()))
            }
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.directory).map_err(|source| LocalGitStateError::Write {
                    path: self.directory.clone(),
                    source,
                })
            }
            Err(source) => Err(LocalGitStateError::Write {
                path: self.directory.clone(),
                source,
            }),
        }
    }

    fn verify_directory_boundary(&self) -> Result<(), LocalGitStateError> {
        for ancestor in self.directory.ancestors() {
            match fs::symlink_metadata(ancestor) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(LocalGitStateError::UnsafePath(ancestor.to_owned()));
                }
                Ok(metadata) if ancestor == self.directory && !metadata.is_dir() => {
                    return Err(LocalGitStateError::UnsafePath(ancestor.to_owned()));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(LocalGitStateError::Read {
                        path: ancestor.to_owned(),
                        source,
                    });
                }
            }
        }
        Ok(())
    }

    fn create_temporary(&self) -> Result<(PathBuf, File), LocalGitStateError> {
        for attempt in 0..32_u8 {
            let path = self
                .directory
                .join(format!(".state.json.tmp-{}-{attempt}", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(LocalGitStateError::Write { path, source }),
            }
        }
        Err(LocalGitStateError::TemporaryLimit(self.directory.clone()))
    }
}

#[derive(Debug, Error)]
pub enum LocalGitStateError {
    #[error(transparent)]
    Git(#[from] GitError),
    #[error("local Git state is invalid or semantically tampered")]
    InvalidState,
    #[error("local Git state path is symlinked or has the wrong file type: {0}")]
    UnsafePath(PathBuf),
    #[error("local Git state at {path} could not be read: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("local Git state at {path} could not be written: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("local Git state JSON at {path} is invalid: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("local Git state exceeds the {0}-byte limit")]
    ByteLimit(u64),
    #[error("local Git state temporary-file attempts were exhausted in {0}")]
    TemporaryLimit(PathBuf),
    #[error("Git polling is already initialized")]
    AlreadyInitialized,
    #[error("Git polling has no retained cursor; run `rey git init` first")]
    Uninitialized,
    #[error("Git poll was derived from a stale cursor")]
    StalePoll,
    #[error("Git transition {0} is already pending acknowledgement")]
    PendingPoll(SemanticDigest),
    #[error("there is no retained Git transition awaiting acknowledgement")]
    NoPendingPoll,
    #[error("Git acknowledgement expected {expected}, not {actual}")]
    StaleAcknowledgement {
        expected: SemanticDigest,
        actual: String,
    },
    #[error("retained Git transition history exceeds {0}")]
    TransitionLimit(usize),
    #[error("Git activation {0} is pending and must be acknowledged before workload admission")]
    ActivationNotAcknowledged(String),
    #[error("unknown acknowledged Git activation {0}")]
    UnknownActivation(String),
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, process::Command};

    use rey_environment::resolve_executable;
    use rey_git::{GitInspector, GitLimits};
    use tempfile::TempDir;

    use super::{GitPollRecord, LocalGitStateError, LocalGitStore};

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
        directory
    }

    fn inspector(directory: &Path) -> GitInspector {
        let paths = std::env::split_paths(&std::env::var_os("PATH").unwrap()).collect::<Vec<_>>();
        GitInspector {
            git_program: resolve_executable("git", &paths).unwrap(),
            workspace: directory.to_owned(),
            limits: GitLimits::default(),
        }
    }

    #[test]
    fn local_store_retains_pending_evidence_before_advancing_the_cursor() {
        let directory = repository();
        let inspect = inspector(directory.path());
        let store = LocalGitStore::default_for_workspace(directory.path());
        let initial = inspect.inspect().unwrap().unwrap();
        let state = store.initialize(initial).unwrap();
        let initial_cursor = state.cursor.unwrap();

        fs::write(directory.path().join("tracked"), "two\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        git(directory.path(), &["commit", "-q", "-m", "second"]);
        let (target, transition) = inspect
            .inspect_transition(&initial_cursor)
            .unwrap()
            .unwrap();
        let record = GitPollRecord::new(target, transition.clone(), Vec::new()).unwrap();
        let retained = store.retain_poll(record.clone()).unwrap();
        assert_eq!(retained.pending, Some(record.clone()));
        assert_eq!(retained.cursor, Some(initial_cursor));
        assert_eq!(store.retain_poll(record).unwrap(), retained);

        assert!(matches!(
            store.acknowledge(
                "blake3:0000000000000000000000000000000000000000000000000000000000000000"
            ),
            Err(LocalGitStateError::StaleAcknowledgement { .. })
        ));
        assert!(store.load().unwrap().pending.is_some());

        let advanced = store
            .acknowledge(transition.transition_id.as_str())
            .unwrap();
        assert!(advanced.pending.is_none());
        assert_eq!(advanced.retained_polls.len(), 1);
        assert_eq!(advanced.retained_polls[0].transition, transition);
        assert_eq!(store.load().unwrap(), advanced);

        fs::write(directory.path().join("tracked"), "staged\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        let cursor = advanced.cursor.as_ref().unwrap();
        let (target, transition) = inspect.inspect_transition(cursor).unwrap().unwrap();
        store
            .retain_poll(GitPollRecord::new(target, transition.clone(), Vec::new()).unwrap())
            .unwrap();
        let advanced = store
            .acknowledge(transition.transition_id.as_str())
            .unwrap();
        assert_eq!(advanced.retained_polls.len(), 2);
        advanced.verify().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_local_git_state_ancestor_fails_closed() {
        use std::os::unix::fs::symlink;

        let directory = repository();
        let outside = TempDir::new().unwrap();
        symlink(outside.path(), directory.path().join(".rey")).unwrap();
        let store = LocalGitStore::default_for_workspace(directory.path());
        assert!(matches!(
            store.load(),
            Err(LocalGitStateError::UnsafePath(_))
        ));
    }
}
