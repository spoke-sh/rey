#![forbid(unsafe_code)]

use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rey_core::{SemanticDigest, SemanticHasher};
use rey_environment::{
    Availability, CapabilityRecord, CommandError, CommandOutput, CommandRequest,
    LOCAL_PROVIDER_REVISION, TrustClass, run_bounded,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const GIT_SNAPSHOT_SCHEMA: &str = "rey.git-repository.v1";

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
                "inspect_repository".to_owned(),
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

        Ok(Some(GitSnapshot {
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
        }))
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
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, process::Command};

    use tempfile::TempDir;

    use rey_environment::resolve_executable;

    use super::{GitInspector, GitLimits};

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
        let search_paths =
            std::env::split_paths(&std::env::var_os("PATH").unwrap()).collect::<Vec<_>>();
        GitInspector {
            git_program: resolve_executable("git", &search_paths).unwrap(),
            workspace: directory.to_owned(),
            limits: GitLimits::default(),
        }
    }
    #[test]
    fn non_repository_is_absent_not_an_error() {
        let directory = TempDir::new().unwrap();
        assert!(inspector(directory.path()).inspect().unwrap().is_none());
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
