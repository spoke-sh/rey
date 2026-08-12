#![forbid(unsafe_code)]

pub mod channels;
pub mod editor;
pub mod env;
pub mod git;
pub mod ignore;
pub mod journal;
pub mod journal_opportunities;
pub mod journal_queries;
pub mod journal_seed;
pub mod observations;
pub mod workload_evidence;
pub mod workloads;

use std::path::Path;

use rey_environment::{
    CapabilitySnapshot, DiscoveryError, DiscoveryLimits, EnvironmentMapError, EnvironmentMapInputs,
    EnvironmentMapLimits, EnvironmentMapObservation, LocalDiscovery,
};
use thiserror::Error;

use crate::env::{EnvironmentStatus, LocalEnvironmentStore};
use crate::ignore::{ReyIgnoreFile, apply_environment_ignore};

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
    if let Some(ignore) = ReyIgnoreFile::load(workspace)? {
        snapshot = apply_environment_ignore(snapshot, &ignore)?.0;
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
    let mut status = EnvironmentStatus::derive(&history, index, snapshot, max_changes)?;
    status.apply_ignore_projection(workspace)?;
    Ok(status)
}

#[derive(Debug, Error)]
pub enum ReyError {
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
    #[error(transparent)]
    EnvironmentMap(#[from] EnvironmentMapError),
    #[error(transparent)]
    EnvironmentState(#[from] crate::env::LocalEnvironmentHistoryError),
    #[error(transparent)]
    Ignore(#[from] crate::ignore::ReyIgnoreError),
}

#[cfg(test)]
mod tests {
    use std::{fs, process::Command};

    use rey_environment::DiscoveryLimits;
    use tempfile::TempDir;

    use super::{inspect_environment, inspect_environment_with_mapping};

    #[test]
    fn inspection_succeeds_without_a_git_repository() {
        let workspace = TempDir::new().unwrap();
        let snapshot = inspect_environment(workspace.path(), DiscoveryLimits::default()).unwrap();

        assert_eq!(snapshot.profile, "standalone");
        assert!(!snapshot.capabilities.is_empty());
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

    #[test]
    fn environment_ignore_policy_is_retained_in_the_filtered_snapshot() {
        let workspace = TempDir::new().unwrap();
        fs::write(
            workspace.path().join(".reyignore"),
            "environment variable:*\napplication: definitely-missing\n",
        )
        .unwrap();
        let snapshot = inspect_environment_with_mapping(
            workspace.path(),
            DiscoveryLimits::default(),
            None,
            rey_environment::EnvironmentMapLimits::default(),
        )
        .unwrap();
        assert!(
            snapshot
                .capabilities
                .iter()
                .all(|record| { record.capability_kind != "environment_seed" })
        );
        let policy = snapshot
            .capabilities
            .iter()
            .find(|record| record.capability_kind == "ignore_policy")
            .unwrap();
        let projection: crate::ignore::ReyIgnoreProjection =
            serde_json::from_str(policy.provenance.as_deref().unwrap()).unwrap();
        assert_eq!(projection.rules.len(), 2);
        assert_eq!(projection.omissions[0].matched, 3);
        assert_eq!(projection.omissions[1].matched, 0);
        assert_eq!(projection.ignored, 3);
    }

    #[test]
    fn relevant_zero_match_environment_rule_is_retained_as_policy() {
        let workspace = TempDir::new().unwrap();
        fs::write(
            workspace.path().join(".reyignore"),
            "application: definitely-missing\n",
        )
        .unwrap();
        let snapshot = inspect_environment_with_mapping(
            workspace.path(),
            DiscoveryLimits::default(),
            None,
            rey_environment::EnvironmentMapLimits::default(),
        )
        .unwrap();
        let policy = snapshot
            .capabilities
            .iter()
            .find(|record| record.capability_kind == "ignore_policy")
            .unwrap();
        let projection: crate::ignore::ReyIgnoreProjection =
            serde_json::from_str(policy.provenance.as_deref().unwrap()).unwrap();
        assert_eq!(projection.rules.len(), 1);
        assert_eq!(projection.omissions[0].matched, 0);
        assert_eq!(projection.ignored, 0);
    }
}
