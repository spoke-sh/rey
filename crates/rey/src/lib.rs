#![forbid(unsafe_code)]

pub mod env;
pub mod journal;
pub mod workloads;

use std::path::Path;

use rey_environment::{
    CapabilitySnapshot, DiscoveryError, DiscoveryLimits, EnvironmentMapError, EnvironmentMapInputs,
    EnvironmentMapLimits, EnvironmentMapObservation, LocalDiscovery,
};
use thiserror::Error;

use crate::env::{EnvironmentStatus, LocalEnvironmentStore};

pub fn inspect_environment(
    workspace: &Path,
    discovery_limits: DiscoveryLimits,
) -> Result<CapabilitySnapshot, ReyError> {
    let discovery = LocalDiscovery::from_environment(workspace.to_owned(), discovery_limits);
    Ok(discovery.inspect()?)
}

pub fn inspect_environment_with_mapping(
    workspace: &Path,
    discovery_limits: DiscoveryLimits,
    map_path: Option<&Path>,
    map_limits: EnvironmentMapLimits,
) -> Result<CapabilitySnapshot, ReyError> {
    let mut snapshot = inspect_environment(workspace, discovery_limits)?;
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

pub fn current_environment_status(
    store: &LocalEnvironmentStore,
    workspace: &Path,
    discovery_limits: DiscoveryLimits,
    map_path: Option<&Path>,
    max_changes: u64,
) -> Result<EnvironmentStatus, ReyError> {
    let history = store.load()?;
    let index = store.load_index(&history)?;
    let snapshot = inspect_environment_with_mapping(
        workspace,
        discovery_limits,
        map_path,
        EnvironmentMapLimits::default(),
    )?;
    Ok(EnvironmentStatus::derive(
        &history,
        index,
        snapshot,
        max_changes,
    )?)
}

#[derive(Debug, Error)]
pub enum ReyError {
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
    #[error(transparent)]
    EnvironmentMap(#[from] EnvironmentMapError),
    #[error(transparent)]
    EnvironmentState(#[from] crate::env::LocalEnvironmentHistoryError),
}

#[cfg(test)]
mod tests {
    use std::{fs, process::Command};

    use rey_environment::DiscoveryLimits;
    use tempfile::TempDir;

    use super::inspect_environment;

    #[test]
    fn inspection_succeeds_without_spoke_or_git_repository() {
        let workspace = TempDir::new().unwrap();
        let snapshot = inspect_environment(workspace.path(), DiscoveryLimits::default()).unwrap();

        assert_eq!(snapshot.profile, "standalone");
        assert!(
            snapshot
                .capabilities
                .iter()
                .all(|row| !row.provider_id.contains("spoke"))
        );
    }

    #[test]
    fn environment_discovery_keeps_git_repository_state_out_of_admission() {
        let workspace = TempDir::new().unwrap();
        assert!(
            Command::new("git")
                .args(["-C", workspace.path().to_str().unwrap(), "init", "-q"])
                .status()
                .unwrap()
                .success()
        );
        let before = inspect_environment(workspace.path(), DiscoveryLimits::default()).unwrap();
        fs::write(
            workspace.path().join("tracked.txt"),
            "semantic index drift\n",
        )
        .unwrap();
        assert!(
            Command::new("git")
                .args([
                    "-C",
                    workspace.path().to_str().unwrap(),
                    "add",
                    "--",
                    "tracked.txt",
                ])
                .status()
                .unwrap()
                .success()
        );
        let after = inspect_environment(workspace.path(), DiscoveryLimits::default()).unwrap();

        assert!(
            before
                .capabilities
                .iter()
                .all(|row| row.capability_id != "git.repository.inspect")
        );
        assert!(
            before
                .capabilities
                .iter()
                .any(|row| row.capability_id == "tool.git.identity")
        );
        assert_eq!(before.semantic_digest, after.semantic_digest);
    }
}
