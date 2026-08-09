#![forbid(unsafe_code)]

pub mod env;
pub mod workloads;

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use rey_environment::{
    Availability, CapabilityRecord, CapabilitySnapshot, DiscoveryError, DiscoveryLimits,
    EnvironmentMapError, EnvironmentMapInputs, EnvironmentMapLimits, EnvironmentMapObservation,
    LOCAL_PROVIDER_REVISION, LocalDiscovery, TrustClass,
};
use rey_git::{GitError, GitInspector, GitLimits};
use thiserror::Error;

pub fn inspect_environment(
    workspace: &Path,
    discovery_limits: DiscoveryLimits,
    git_limits: GitLimits,
) -> Result<CapabilitySnapshot, ReyError> {
    let deadline = Instant::now() + Duration::from_millis(discovery_limits.total_timeout_ms);
    let discovery = LocalDiscovery::from_environment(workspace.to_owned(), discovery_limits);
    let mut snapshot = discovery.inspect()?;
    let git_program = snapshot.capabilities.iter().find_map(|row| {
        (row.capability_id == "tool.git.identity" && row.availability == Availability::Available)
            .then_some(row.resolved_location.as_deref())
            .flatten()
            .map(PathBuf::from)
    });
    if let Some(git_program) = git_program {
        let inspector = GitInspector {
            git_program,
            workspace: workspace.to_owned(),
            limits: git_limits,
        };
        match inspector.inspect_until(deadline) {
            Ok(Some(git)) => snapshot.push(git.capability_record())?,
            Ok(None) => {}
            Err(error) => snapshot.push(git_error_capability(workspace, &error))?,
        }
    }
    Ok(snapshot)
}

pub fn inspect_environment_with_mapping(
    workspace: &Path,
    discovery_limits: DiscoveryLimits,
    git_limits: GitLimits,
    map_path: Option<&Path>,
    map_limits: EnvironmentMapLimits,
) -> Result<CapabilitySnapshot, ReyError> {
    let mut snapshot = inspect_environment(workspace, discovery_limits, git_limits)?;
    if let Some(observation) = EnvironmentMapObservation::load(
        workspace,
        map_path,
        &EnvironmentMapInputs::from_environment(),
        map_limits,
    )? {
        for capability in observation.capabilities {
            snapshot.push(capability)?;
        }
    }
    Ok(snapshot)
}

fn git_error_capability(workspace: &Path, error: &GitError) -> CapabilityRecord {
    CapabilityRecord {
        provider_id: "rey.git".to_owned(),
        provider_revision: LOCAL_PROVIDER_REVISION,
        provider_kind: "git_repository".to_owned(),
        capability_id: "git.repository.inspect".to_owned(),
        capability_kind: "context_surface".to_owned(),
        resolved_location: Some(workspace.display().to_string()),
        version: None,
        content_digest: None,
        provenance: None,
        availability: Availability::Error,
        trust_class: TrustClass::ExplicitLocal,
        operations: Vec::new(),
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
        error_code: Some("git_inspection_failed".to_owned()),
        error_detail: Some(error.to_string()),
    }
}

#[derive(Debug, Error)]
pub enum ReyError {
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
    #[error(transparent)]
    EnvironmentMap(#[from] EnvironmentMapError),
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use rey_environment::DiscoveryLimits;
    use rey_git::GitLimits;
    use tempfile::TempDir;

    use super::inspect_environment;

    #[test]
    fn inspection_succeeds_without_spoke_or_git_repository() {
        let workspace = TempDir::new().unwrap();
        let snapshot = inspect_environment(
            workspace.path(),
            DiscoveryLimits::default(),
            GitLimits::default(),
        )
        .unwrap();

        assert_eq!(snapshot.profile, "standalone");
        assert!(
            snapshot
                .capabilities
                .iter()
                .all(|row| !row.provider_id.contains("spoke"))
        );
    }

    #[test]
    fn git_repository_is_projected_into_the_common_capability_relation() {
        let workspace = TempDir::new().unwrap();
        assert!(
            Command::new("git")
                .args(["-C", workspace.path().to_str().unwrap(), "init", "-q"])
                .status()
                .unwrap()
                .success()
        );
        let snapshot = inspect_environment(
            workspace.path(),
            DiscoveryLimits::default(),
            GitLimits::default(),
        )
        .unwrap();

        assert!(
            snapshot
                .capabilities
                .iter()
                .any(|row| row.capability_id == "git.repository.inspect")
        );
    }
}
