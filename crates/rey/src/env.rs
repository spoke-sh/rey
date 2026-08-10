use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use rey_core::{SemanticDigest, SemanticHasher};
use rey_diff::{
    CapabilityDelta, CapabilityKey, DeltaAssessment, DeltaLimits, DeltaOptions,
    compare_capabilities,
};
use rey_environment::{
    Availability, CapabilityRecord, CapabilitySnapshot, DiscoveryError, EnvironmentMapEdge,
    EnvironmentMapNode, EnvironmentMapNodeProvenance, VariableCapture,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const ENVIRONMENT_COMMIT_SCHEMA: &str = "rey.environment-commit.v1";
pub const ENVIRONMENT_COMMIT_RESULT_SCHEMA: &str = "rey.environment-commit-result.v1";
pub const LOCAL_ENVIRONMENT_HISTORY_SCHEMA: &str = "rey.local-environment-history.v1";
pub const ENVIRONMENT_STATUS_SCHEMA: &str = "rey.environment-status.v3";
pub const ENVIRONMENT_DIFF_SCHEMA: &str = "rey.environment-diff.v2";
pub const ENVIRONMENT_OPERATOR_PROJECTION_SCHEMA: &str = "rey.environment-operator-projection.v1";
pub const ENVIRONMENT_ADMISSION_INDEX_SCHEMA: &str = "rey.environment-admission-index.v1";
pub const ENVIRONMENT_ADD_RESULT_SCHEMA: &str = "rey.environment-add-result.v1";
pub const ENVIRONMENT_LOG_SCHEMA: &str = "rey.environment-log.v1";
pub const MAX_ENVIRONMENT_COMMITS: usize = 256;
pub const MAX_ENVIRONMENT_STATE_BYTES: u64 = 16 * 1_024 * 1_024;
pub const MAX_ENVIRONMENT_MESSAGE_BYTES: usize = 4_096;

const STATE_FILE_NAME: &str = "state.json";
const INDEX_FILE_NAME: &str = "index.json";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentCommit {
    pub schema: String,
    pub commit_id: SemanticDigest,
    pub sequence: u64,
    pub parent_commit_id: Option<SemanticDigest>,
    pub message: String,
    pub snapshot: CapabilitySnapshot,
}

impl EnvironmentCommit {
    pub fn new(
        sequence: u64,
        parent_commit_id: Option<SemanticDigest>,
        message: impl Into<String>,
        snapshot: CapabilitySnapshot,
    ) -> Result<Self, LocalEnvironmentHistoryError> {
        let message = normalize_message(message.into())?;
        if sequence == 0 {
            return Err(LocalEnvironmentHistoryError::InvalidSequence {
                expected: 1,
                actual: sequence,
            });
        }
        snapshot.verify()?;
        let commit_id = commit_digest(
            sequence,
            parent_commit_id.as_ref(),
            &message,
            &snapshot.semantic_digest,
        );
        Ok(Self {
            schema: ENVIRONMENT_COMMIT_SCHEMA.to_owned(),
            commit_id,
            sequence,
            parent_commit_id,
            message,
            snapshot,
        })
    }

    pub fn verify(&self) -> Result<(), LocalEnvironmentHistoryError> {
        if self.schema != ENVIRONMENT_COMMIT_SCHEMA {
            return Err(LocalEnvironmentHistoryError::UnsupportedCommitSchema {
                actual: self.schema.clone(),
            });
        }
        if self.sequence == 0 {
            return Err(LocalEnvironmentHistoryError::InvalidSequence {
                expected: 1,
                actual: self.sequence,
            });
        }
        let message = normalize_message(self.message.clone())?;
        if message != self.message {
            return Err(LocalEnvironmentHistoryError::NonCanonicalMessage);
        }
        self.snapshot.verify()?;
        let actual = commit_digest(
            self.sequence,
            self.parent_commit_id.as_ref(),
            &self.message,
            &self.snapshot.semantic_digest,
        );
        if self.commit_id != actual {
            return Err(LocalEnvironmentHistoryError::CommitDigest {
                declared: self.commit_id.clone(),
                actual,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LocalEnvironmentHistory {
    pub schema: String,
    pub commits: Vec<EnvironmentCommit>,
}

impl Default for LocalEnvironmentHistory {
    fn default() -> Self {
        Self {
            schema: LOCAL_ENVIRONMENT_HISTORY_SCHEMA.to_owned(),
            commits: Vec::new(),
        }
    }
}

impl LocalEnvironmentHistory {
    pub fn verify(&self) -> Result<(), LocalEnvironmentHistoryError> {
        if self.schema != LOCAL_ENVIRONMENT_HISTORY_SCHEMA {
            return Err(LocalEnvironmentHistoryError::UnsupportedHistorySchema {
                actual: self.schema.clone(),
            });
        }
        if self.commits.len() > MAX_ENVIRONMENT_COMMITS {
            return Err(LocalEnvironmentHistoryError::CommitLimit {
                limit: MAX_ENVIRONMENT_COMMITS,
            });
        }
        let mut ids = BTreeSet::new();
        let mut parent: Option<&EnvironmentCommit> = None;
        for (index, commit) in self.commits.iter().enumerate() {
            commit.verify()?;
            let expected_sequence = index as u64 + 1;
            if commit.sequence != expected_sequence {
                return Err(LocalEnvironmentHistoryError::InvalidSequence {
                    expected: expected_sequence,
                    actual: commit.sequence,
                });
            }
            let expected_parent = parent.map(|parent| &parent.commit_id);
            if commit.parent_commit_id.as_ref() != expected_parent {
                return Err(LocalEnvironmentHistoryError::ParentMismatch {
                    sequence: commit.sequence,
                });
            }
            if parent.is_some_and(|parent| {
                parent.snapshot.semantic_digest == commit.snapshot.semantic_digest
            }) {
                return Err(LocalEnvironmentHistoryError::UnchangedCommit {
                    sequence: commit.sequence,
                });
            }
            if !ids.insert(commit.commit_id.clone()) {
                return Err(LocalEnvironmentHistoryError::DuplicateCommit(
                    commit.commit_id.clone(),
                ));
            }
            parent = Some(commit);
        }
        Ok(())
    }

    #[must_use]
    pub fn head(&self) -> Option<&EnvironmentCommit> {
        self.commits.last()
    }

    pub fn commit(
        &mut self,
        message: impl Into<String>,
        snapshot: CapabilitySnapshot,
    ) -> Result<EnvironmentCommit, LocalEnvironmentHistoryError> {
        self.verify()?;
        let message = normalize_message(message.into())?;
        snapshot.verify()?;
        if self
            .head()
            .is_some_and(|head| head.snapshot.semantic_digest == snapshot.semantic_digest)
        {
            return Err(LocalEnvironmentHistoryError::NothingToCommit(
                snapshot.semantic_digest,
            ));
        }
        if self.commits.len() >= MAX_ENVIRONMENT_COMMITS {
            return Err(LocalEnvironmentHistoryError::CommitLimit {
                limit: MAX_ENVIRONMENT_COMMITS,
            });
        }
        let commit = EnvironmentCommit::new(
            self.commits.len() as u64 + 1,
            self.head().map(|head| head.commit_id.clone()),
            message,
            snapshot,
        )?;
        self.commits.push(commit.clone());
        self.verify()?;
        Ok(commit)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentAdmissionIndex {
    pub schema: String,
    pub index_id: SemanticDigest,
    pub base_commit_id: Option<SemanticDigest>,
    pub snapshot: CapabilitySnapshot,
}

impl EnvironmentAdmissionIndex {
    pub fn new(
        history: &LocalEnvironmentHistory,
        snapshot: CapabilitySnapshot,
    ) -> Result<Self, LocalEnvironmentHistoryError> {
        history.verify()?;
        snapshot.verify()?;
        let base_commit_id = history.head().map(|head| head.commit_id.clone());
        let index_id = admission_index_digest(base_commit_id.as_ref(), &snapshot.semantic_digest);
        Ok(Self {
            schema: ENVIRONMENT_ADMISSION_INDEX_SCHEMA.to_owned(),
            index_id,
            base_commit_id,
            snapshot,
        })
    }

    pub fn verify_against(
        &self,
        history: &LocalEnvironmentHistory,
    ) -> Result<(), LocalEnvironmentHistoryError> {
        history.verify()?;
        if self.schema != ENVIRONMENT_ADMISSION_INDEX_SCHEMA {
            return Err(LocalEnvironmentHistoryError::UnsupportedIndexSchema {
                actual: self.schema.clone(),
            });
        }
        self.snapshot.verify()?;
        let expected_base = history.head().map(|head| &head.commit_id);
        if self.base_commit_id.as_ref() != expected_base {
            return Err(LocalEnvironmentHistoryError::StaleIndex {
                expected: expected_base.cloned(),
                actual: self.base_commit_id.clone(),
            });
        }
        let actual =
            admission_index_digest(self.base_commit_id.as_ref(), &self.snapshot.semantic_digest);
        if self.index_id != actual {
            return Err(LocalEnvironmentHistoryError::IndexDigest {
                declared: self.index_id.clone(),
                actual,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentWorkingState {
    Unborn,
    Clean,
    Changed,
    Staged,
    Mixed,
    Inconclusive,
}

impl EnvironmentWorkingState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unborn => "unborn",
            Self::Clean => "clean",
            Self::Changed => "changed",
            Self::Staged => "staged",
            Self::Mixed => "mixed",
            Self::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentObjectChange {
    Unchanged,
    Inserted,
    Deleted,
    Modified,
}

impl EnvironmentObjectChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Inserted => "inserted",
            Self::Deleted => "deleted",
            Self::Modified => "modified",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentPlaneChanges {
    pub head_to_index: EnvironmentObjectChange,
    pub index_to_working: EnvironmentObjectChange,
    pub head_to_working: EnvironmentObjectChange,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentObjectStatus<T> {
    pub object_id: String,
    pub head: Option<T>,
    pub index: Option<T>,
    pub working: Option<T>,
    pub changes: EnvironmentPlaneChanges,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentVariableObservation {
    pub name: String,
    pub sensitive: bool,
    pub capture: VariableCapture,
    pub availability: Availability,
    pub value: Option<String>,
    pub value_digest: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentApplicationObservation {
    pub name: String,
    pub required: bool,
    pub availability: Availability,
    pub resolved_path: Option<String>,
    pub content_digest: Option<String>,
    pub potential_capabilities: Vec<String>,
    pub searched_path_count: u64,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentInputObservation {
    pub path: String,
    pub required: bool,
    pub availability: Availability,
    pub content_digest: Option<String>,
    pub byte_length: Option<u64>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentReferenceObservation {
    pub from: String,
    pub to: String,
    pub relation: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentOperatorSummary {
    pub variables: u64,
    pub changed_variables: u64,
    pub applications_searched: u64,
    pub applications_found: u64,
    pub applications_not_found: u64,
    pub application_errors: u64,
    pub changed_applications: u64,
    pub inputs: u64,
    pub changed_inputs: u64,
    pub references: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentMappingCoordinate {
    pub source_path: String,
    pub schema: String,
    pub graph_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentOperatorProjection {
    pub schema: String,
    pub source_label: String,
    pub target_label: String,
    pub complete: bool,
    pub mapping: Option<EnvironmentMappingCoordinate>,
    pub summary: EnvironmentOperatorSummary,
    pub variables: Vec<EnvironmentObjectStatus<EnvironmentVariableObservation>>,
    pub applications: Vec<EnvironmentObjectStatus<EnvironmentApplicationObservation>>,
    pub inputs: Vec<EnvironmentObjectStatus<EnvironmentInputObservation>>,
    pub references: Vec<EnvironmentObjectStatus<EnvironmentReferenceObservation>>,
}

impl EnvironmentOperatorProjection {
    fn derive(
        head: &CapabilitySnapshot,
        index: &CapabilitySnapshot,
        working: &CapabilitySnapshot,
        source_label: String,
    ) -> Result<Self, LocalEnvironmentHistoryError> {
        let head = MappedEnvironmentPlane::derive(head)?;
        let index = MappedEnvironmentPlane::derive(index)?;
        let working = MappedEnvironmentPlane::derive(working)?;
        let variables = merge_objects(&head.variables, &index.variables, &working.variables);
        let applications = merge_objects(
            &head.applications,
            &index.applications,
            &working.applications,
        );
        let inputs = merge_objects(&head.inputs, &index.inputs, &working.inputs);
        let references = merge_objects(&head.references, &index.references, &working.references);
        let working_applications = applications
            .iter()
            .filter_map(|application| application.working.as_ref())
            .collect::<Vec<_>>();
        let summary = EnvironmentOperatorSummary {
            variables: variables.len() as u64,
            changed_variables: changed_count(&variables),
            applications_searched: working_applications.len() as u64,
            applications_found: working_applications
                .iter()
                .filter(|application| application.availability == Availability::Available)
                .count() as u64,
            applications_not_found: working_applications
                .iter()
                .filter(|application| application.availability == Availability::Unavailable)
                .count() as u64,
            application_errors: working_applications
                .iter()
                .filter(|application| application.availability == Availability::Error)
                .count() as u64,
            changed_applications: changed_count(&applications),
            inputs: inputs.len() as u64,
            changed_inputs: changed_count(&inputs),
            references: references.len() as u64,
        };
        Ok(Self {
            schema: ENVIRONMENT_OPERATOR_PROJECTION_SCHEMA.to_owned(),
            source_label,
            target_label: "WORKING".to_owned(),
            complete: working.complete,
            mapping: working.mapping.or(index.mapping).or(head.mapping),
            summary,
            variables,
            applications,
            inputs,
            references,
        })
    }
}

#[derive(Default)]
struct MappedEnvironmentPlane {
    complete: bool,
    mapping: Option<EnvironmentMappingCoordinate>,
    variables: BTreeMap<String, EnvironmentVariableObservation>,
    applications: BTreeMap<String, EnvironmentApplicationObservation>,
    inputs: BTreeMap<String, EnvironmentInputObservation>,
    references: BTreeMap<String, EnvironmentReferenceObservation>,
}

impl MappedEnvironmentPlane {
    fn derive(snapshot: &CapabilitySnapshot) -> Result<Self, LocalEnvironmentHistoryError> {
        let mut plane = Self {
            complete: snapshot.complete,
            ..Self::default()
        };
        let mut revisions = BTreeMap::<String, u64>::new();
        for record in snapshot
            .capabilities
            .iter()
            .filter(|record| record.provider_id == "rey.env-map")
        {
            if record.capability_kind == "environment_map" {
                if plane.mapping.is_none()
                    || revisions
                        .get("env.mapping.graph")
                        .is_none_or(|revision| *revision <= record.provider_revision)
                {
                    plane.mapping = Some(EnvironmentMappingCoordinate {
                        source_path: record.resolved_location.clone().unwrap_or_default(),
                        schema: record
                            .version
                            .clone()
                            .unwrap_or_else(|| "unknown".to_owned()),
                        graph_id: record
                            .content_digest
                            .clone()
                            .unwrap_or_else(|| "unknown".to_owned()),
                    });
                    revisions.insert("env.mapping.graph".to_owned(), record.provider_revision);
                }
                continue;
            }
            if revisions
                .get(&record.capability_id)
                .is_some_and(|revision| *revision > record.provider_revision)
            {
                continue;
            }
            match record.capability_kind.as_str() {
                "environment_variable" => {
                    let provenance = node_provenance(record)?;
                    let EnvironmentMapNode::Variable {
                        id,
                        name,
                        sensitive,
                        capture,
                    } = provenance.declaration
                    else {
                        return Err(LocalEnvironmentHistoryError::EnvironmentProjection(
                            format!("{} has non-variable provenance", record.capability_id),
                        ));
                    };
                    plane.variables.insert(
                        id.clone(),
                        EnvironmentVariableObservation {
                            name,
                            sensitive,
                            capture,
                            availability: record.availability,
                            value: provenance.captured_value,
                            value_digest: record.content_digest.clone(),
                            error_code: record.error_code.clone(),
                        },
                    );
                    revisions.insert(record.capability_id.clone(), record.provider_revision);
                }
                "potential_executable" => {
                    let provenance = node_provenance(record)?;
                    let EnvironmentMapNode::Executable {
                        id,
                        name,
                        required,
                        potential_capabilities,
                    } = provenance.declaration
                    else {
                        return Err(LocalEnvironmentHistoryError::EnvironmentProjection(
                            format!("{} has non-executable provenance", record.capability_id),
                        ));
                    };
                    plane.applications.insert(
                        id,
                        EnvironmentApplicationObservation {
                            name,
                            required,
                            availability: record.availability,
                            resolved_path: record.resolved_location.clone(),
                            content_digest: record.content_digest.clone(),
                            potential_capabilities,
                            searched_path_count: provenance.search_path_count.unwrap_or(0),
                            error_code: record.error_code.clone(),
                        },
                    );
                    revisions.insert(record.capability_id.clone(), record.provider_revision);
                }
                "input_file" => {
                    let provenance = node_provenance(record)?;
                    let EnvironmentMapNode::File { id, path, required } = provenance.declaration
                    else {
                        return Err(LocalEnvironmentHistoryError::EnvironmentProjection(
                            format!("{} has non-file provenance", record.capability_id),
                        ));
                    };
                    plane.inputs.insert(
                        id,
                        EnvironmentInputObservation {
                            path: path.to_string_lossy().into_owned(),
                            required,
                            availability: record.availability,
                            content_digest: record.content_digest.clone(),
                            byte_length: provenance
                                .byte_length
                                .as_deref()
                                .and_then(|length| length.parse().ok()),
                            error_code: record.error_code.clone(),
                        },
                    );
                    revisions.insert(record.capability_id.clone(), record.provider_revision);
                }
                "environment_edge" => {
                    let edge: EnvironmentMapEdge =
                        serde_json::from_str(record.provenance.as_deref().ok_or_else(|| {
                            LocalEnvironmentHistoryError::EnvironmentProjection(format!(
                                "{} is missing edge provenance",
                                record.capability_id
                            ))
                        })?)
                        .map_err(|error| {
                            LocalEnvironmentHistoryError::EnvironmentProjection(format!(
                                "{} edge provenance is invalid: {error}",
                                record.capability_id
                            ))
                        })?;
                    plane.references.insert(
                        record.capability_id.clone(),
                        EnvironmentReferenceObservation {
                            from: edge.from,
                            to: edge.to,
                            relation: edge.relation,
                        },
                    );
                    revisions.insert(record.capability_id.clone(), record.provider_revision);
                }
                _ => {}
            }
        }
        Ok(plane)
    }
}

fn node_provenance(
    record: &CapabilityRecord,
) -> Result<EnvironmentMapNodeProvenance, LocalEnvironmentHistoryError> {
    let value = record.provenance.as_deref().ok_or_else(|| {
        LocalEnvironmentHistoryError::EnvironmentProjection(format!(
            "{} is missing node provenance",
            record.capability_id
        ))
    })?;
    if let Ok(provenance) = serde_json::from_str(value) {
        return Ok(provenance);
    }
    let declaration: EnvironmentMapNode = serde_json::from_str(value).map_err(|error| {
        LocalEnvironmentHistoryError::EnvironmentProjection(format!(
            "{} node provenance is invalid: {error}",
            record.capability_id
        ))
    })?;
    Ok(EnvironmentMapNodeProvenance {
        declaration,
        byte_length: None,
        captured_value: None,
        search_path_count: None,
    })
}

fn merge_objects<T: Clone + Eq>(
    head: &BTreeMap<String, T>,
    index: &BTreeMap<String, T>,
    working: &BTreeMap<String, T>,
) -> Vec<EnvironmentObjectStatus<T>> {
    let keys = head
        .keys()
        .chain(index.keys())
        .chain(working.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    keys.into_iter()
        .map(|object_id| {
            let head = head.get(&object_id).cloned();
            let index = index.get(&object_id).cloned();
            let working = working.get(&object_id).cloned();
            let changes = EnvironmentPlaneChanges {
                head_to_index: classify_object_change(head.as_ref(), index.as_ref()),
                index_to_working: classify_object_change(index.as_ref(), working.as_ref()),
                head_to_working: classify_object_change(head.as_ref(), working.as_ref()),
            };
            EnvironmentObjectStatus {
                object_id,
                head,
                index,
                working,
                changes,
            }
        })
        .collect()
}

fn classify_object_change<T: Eq>(
    source: Option<&T>,
    target: Option<&T>,
) -> EnvironmentObjectChange {
    match (source, target) {
        (None, None) => EnvironmentObjectChange::Unchanged,
        (None, Some(_)) => EnvironmentObjectChange::Inserted,
        (Some(_), None) => EnvironmentObjectChange::Deleted,
        (Some(source), Some(target)) if source == target => EnvironmentObjectChange::Unchanged,
        (Some(_), Some(_)) => EnvironmentObjectChange::Modified,
    }
}

fn changed_count<T>(objects: &[EnvironmentObjectStatus<T>]) -> u64 {
    objects
        .iter()
        .filter(|object| object.changes.head_to_working != EnvironmentObjectChange::Unchanged)
        .count() as u64
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentStatus {
    pub schema: String,
    pub head_commit_id: Option<SemanticDigest>,
    pub head_sequence: Option<u64>,
    pub head_snapshot_id: Option<SemanticDigest>,
    pub admission_index: Option<EnvironmentAdmissionIndex>,
    pub working_snapshot: CapabilitySnapshot,
    pub state: EnvironmentWorkingState,
    pub operator: EnvironmentOperatorProjection,
    pub staged_delta: CapabilityDelta,
    pub unstaged_delta: CapabilityDelta,
}

impl EnvironmentStatus {
    pub fn derive(
        history: &LocalEnvironmentHistory,
        admission_index: Option<EnvironmentAdmissionIndex>,
        working_snapshot: CapabilitySnapshot,
        max_changes: u64,
    ) -> Result<Self, LocalEnvironmentHistoryError> {
        history.verify()?;
        working_snapshot.verify()?;
        if let Some(index) = &admission_index {
            index.verify_against(history)?;
        }
        if max_changes == 0 {
            return Err(LocalEnvironmentHistoryError::ZeroChangeLimit);
        }
        let empty = empty_snapshot_like(&working_snapshot)?;
        let (head_snapshot, head_label) = history.head().map_or_else(
            || (&empty, "EMPTY".to_owned()),
            |head| (&head.snapshot, format!("ENV@{}", head.sequence)),
        );
        let index_snapshot = admission_index
            .as_ref()
            .map_or(head_snapshot, |index| &index.snapshot);
        let staged_delta = compare_capabilities(
            head_snapshot,
            index_snapshot,
            DeltaOptions {
                source_label: head_label,
                target_label: "INDEX".to_owned(),
                limits: DeltaLimits { max_changes },
                ..DeltaOptions::default()
            },
        )?;
        let unstaged_delta = compare_capabilities(
            index_snapshot,
            &working_snapshot,
            DeltaOptions {
                source_label: "INDEX".to_owned(),
                target_label: "WORKING".to_owned(),
                limits: DeltaLimits { max_changes },
                ..DeltaOptions::default()
            },
        )?;
        let state = match (
            staged_delta.summary.assessment,
            unstaged_delta.summary.assessment,
        ) {
            (DeltaAssessment::Inconclusive, _) | (_, DeltaAssessment::Inconclusive) => {
                EnvironmentWorkingState::Inconclusive
            }
            (DeltaAssessment::Different, DeltaAssessment::Different) => {
                EnvironmentWorkingState::Mixed
            }
            (DeltaAssessment::Different, DeltaAssessment::Equal) => EnvironmentWorkingState::Staged,
            (DeltaAssessment::Equal, DeltaAssessment::Different) if history.head().is_none() => {
                EnvironmentWorkingState::Unborn
            }
            (DeltaAssessment::Equal, DeltaAssessment::Different) => {
                EnvironmentWorkingState::Changed
            }
            (DeltaAssessment::Equal, DeltaAssessment::Equal) => EnvironmentWorkingState::Clean,
        };
        let operator = EnvironmentOperatorProjection::derive(
            head_snapshot,
            index_snapshot,
            &working_snapshot,
            staged_delta.source_label.clone(),
        )?;
        Ok(Self {
            schema: ENVIRONMENT_STATUS_SCHEMA.to_owned(),
            head_commit_id: history.head().map(|head| head.commit_id.clone()),
            head_sequence: history.head().map(|head| head.sequence),
            head_snapshot_id: history
                .head()
                .map(|head| head.snapshot.semantic_digest.clone()),
            admission_index,
            working_snapshot,
            state,
            operator,
            staged_delta,
            unstaged_delta,
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentDiffMode {
    Unstaged,
    Staged,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentDiff {
    pub schema: String,
    pub mode: EnvironmentDiffMode,
    pub head_commit_id: Option<SemanticDigest>,
    pub head_sequence: Option<u64>,
    pub head_snapshot_id: Option<SemanticDigest>,
    pub admission_index: Option<EnvironmentAdmissionIndex>,
    pub working_snapshot: CapabilitySnapshot,
    pub delta: CapabilityDelta,
}

impl EnvironmentDiff {
    pub fn derive(
        history: &LocalEnvironmentHistory,
        admission_index: Option<EnvironmentAdmissionIndex>,
        working_snapshot: CapabilitySnapshot,
        max_changes: u64,
        mode: EnvironmentDiffMode,
    ) -> Result<Self, LocalEnvironmentHistoryError> {
        let status =
            EnvironmentStatus::derive(history, admission_index, working_snapshot, max_changes)?;
        let delta = match mode {
            EnvironmentDiffMode::Unstaged => status.unstaged_delta,
            EnvironmentDiffMode::Staged => status.staged_delta,
        };
        Ok(Self {
            schema: ENVIRONMENT_DIFF_SCHEMA.to_owned(),
            mode,
            head_commit_id: status.head_commit_id,
            head_sequence: status.head_sequence,
            head_snapshot_id: status.head_snapshot_id,
            admission_index: status.admission_index,
            working_snapshot: status.working_snapshot,
            delta,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentAddResult {
    pub schema: String,
    pub index: Option<EnvironmentAdmissionIndex>,
    pub staged_changes: u64,
    pub remaining_changes: u64,
    pub staged_delta: CapabilityDelta,
    pub unstaged_delta: CapabilityDelta,
}

impl EnvironmentAddResult {
    #[must_use]
    pub fn new(
        index: Option<EnvironmentAdmissionIndex>,
        status: EnvironmentStatus,
        staged_changes: u64,
    ) -> Self {
        Self {
            schema: ENVIRONMENT_ADD_RESULT_SCHEMA.to_owned(),
            index,
            staged_changes,
            remaining_changes: status.unstaged_delta.changes.len() as u64,
            staged_delta: status.staged_delta,
            unstaged_delta: status.unstaged_delta,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentLogEntry {
    pub commit: EnvironmentCommit,
    pub delta: CapabilityDelta,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentCommitResult {
    pub schema: String,
    pub commit: EnvironmentCommit,
    pub delta: CapabilityDelta,
}

impl EnvironmentCommitResult {
    #[must_use]
    pub fn new(commit: EnvironmentCommit, delta: CapabilityDelta) -> Self {
        Self {
            schema: ENVIRONMENT_COMMIT_RESULT_SCHEMA.to_owned(),
            commit,
            delta,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentLog {
    pub schema: String,
    pub head_commit_id: Option<SemanticDigest>,
    pub total_commits: u64,
    pub selected_commits: u64,
    pub patch: bool,
    pub entries: Vec<EnvironmentLogEntry>,
}

impl EnvironmentLog {
    pub fn derive(
        history: &LocalEnvironmentHistory,
        max_count: usize,
        max_changes: u64,
        patch: bool,
    ) -> Result<Self, LocalEnvironmentHistoryError> {
        history.verify()?;
        if max_count == 0 || max_count > MAX_ENVIRONMENT_COMMITS {
            return Err(LocalEnvironmentHistoryError::LogLimit {
                limit: MAX_ENVIRONMENT_COMMITS,
                actual: max_count,
            });
        }
        if max_changes == 0 {
            return Err(LocalEnvironmentHistoryError::ZeroChangeLimit);
        }
        let mut entries = Vec::with_capacity(max_count.min(history.commits.len()));
        for index in (0..history.commits.len()).rev().take(max_count) {
            let commit = &history.commits[index];
            let empty;
            let (source, source_label) = if index == 0 {
                empty = CapabilitySnapshot::new(
                    commit.snapshot.profile.clone(),
                    commit.snapshot.limits.clone(),
                    Vec::new(),
                )?;
                (&empty, "EMPTY".to_owned())
            } else {
                (
                    &history.commits[index - 1].snapshot,
                    format!("ENV@{}", commit.sequence - 1),
                )
            };
            let delta = compare_capabilities(
                source,
                &commit.snapshot,
                DeltaOptions {
                    source_label,
                    target_label: format!("ENV@{}", commit.sequence),
                    limits: DeltaLimits { max_changes },
                    ..DeltaOptions::default()
                },
            )?;
            entries.push(EnvironmentLogEntry {
                commit: commit.clone(),
                delta,
            });
        }
        Ok(Self {
            schema: ENVIRONMENT_LOG_SCHEMA.to_owned(),
            head_commit_id: history.head().map(|head| head.commit_id.clone()),
            total_commits: history.commits.len() as u64,
            selected_commits: entries.len() as u64,
            patch,
            entries,
        })
    }
}

#[derive(Clone, Debug)]
pub struct LocalEnvironmentStore {
    directory: PathBuf,
}

impl LocalEnvironmentStore {
    #[must_use]
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    #[must_use]
    pub fn default_for_workspace(workspace: &Path) -> Self {
        Self::new(workspace.join(".rey").join("env"))
    }

    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.directory.join(STATE_FILE_NAME)
    }

    #[must_use]
    pub fn index_path(&self) -> PathBuf {
        self.directory.join(INDEX_FILE_NAME)
    }

    pub fn load(&self) -> Result<LocalEnvironmentHistory, LocalEnvironmentHistoryError> {
        self.verify_directory_boundary()?;
        let path = self.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(LocalEnvironmentHistory::default());
            }
            Err(source) => return Err(LocalEnvironmentHistoryError::Read { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(LocalEnvironmentHistoryError::UnsafeStatePath(path));
        }
        if metadata.len() > MAX_ENVIRONMENT_STATE_BYTES {
            return Err(LocalEnvironmentHistoryError::ByteLimit {
                path,
                limit: MAX_ENVIRONMENT_STATE_BYTES,
            });
        }
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|source| LocalEnvironmentHistoryError::Read {
                path: path.clone(),
                source,
            })?
            .take(MAX_ENVIRONMENT_STATE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| LocalEnvironmentHistoryError::Read {
                path: path.clone(),
                source,
            })?;
        if bytes.len() as u64 > MAX_ENVIRONMENT_STATE_BYTES {
            return Err(LocalEnvironmentHistoryError::ByteLimit {
                path,
                limit: MAX_ENVIRONMENT_STATE_BYTES,
            });
        }
        let history: LocalEnvironmentHistory =
            serde_json::from_slice(&bytes).map_err(|source| {
                LocalEnvironmentHistoryError::Json {
                    path: path.clone(),
                    source,
                }
            })?;
        history.verify()?;
        Ok(history)
    }

    pub fn save(
        &self,
        history: &LocalEnvironmentHistory,
    ) -> Result<(), LocalEnvironmentHistoryError> {
        history.verify()?;
        let bytes = serde_json::to_vec_pretty(history).map_err(|source| {
            LocalEnvironmentHistoryError::Json {
                path: self.path(),
                source,
            }
        })?;
        if bytes.len().saturating_add(1) as u64 > MAX_ENVIRONMENT_STATE_BYTES {
            return Err(LocalEnvironmentHistoryError::ByteLimit {
                path: self.path(),
                limit: MAX_ENVIRONMENT_STATE_BYTES,
            });
        }
        self.prepare_directory()?;
        let target = self.path();
        if let Ok(metadata) = fs::symlink_metadata(&target)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(LocalEnvironmentHistoryError::UnsafeStatePath(target));
        }
        let (temporary, mut file) = self.create_temporary(STATE_FILE_NAME)?;
        let publication = (|| {
            file.write_all(&bytes)
                .and_then(|()| file.write_all(b"\n"))
                .and_then(|()| file.flush())?;
            drop(file);
            fs::rename(&temporary, &target)
        })();
        if let Err(source) = publication {
            let _ = fs::remove_file(&temporary);
            return Err(LocalEnvironmentHistoryError::Write {
                path: target,
                source,
            });
        }
        Ok(())
    }

    pub fn load_index(
        &self,
        history: &LocalEnvironmentHistory,
    ) -> Result<Option<EnvironmentAdmissionIndex>, LocalEnvironmentHistoryError> {
        self.verify_directory_boundary()?;
        let path = self.index_path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(LocalEnvironmentHistoryError::Read { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(LocalEnvironmentHistoryError::UnsafeStatePath(path));
        }
        if metadata.len() > MAX_ENVIRONMENT_STATE_BYTES {
            return Err(LocalEnvironmentHistoryError::ByteLimit {
                path,
                limit: MAX_ENVIRONMENT_STATE_BYTES,
            });
        }
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|source| LocalEnvironmentHistoryError::Read {
                path: path.clone(),
                source,
            })?
            .take(MAX_ENVIRONMENT_STATE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| LocalEnvironmentHistoryError::Read {
                path: path.clone(),
                source,
            })?;
        if bytes.len() as u64 > MAX_ENVIRONMENT_STATE_BYTES {
            return Err(LocalEnvironmentHistoryError::ByteLimit {
                path,
                limit: MAX_ENVIRONMENT_STATE_BYTES,
            });
        }
        let index: EnvironmentAdmissionIndex =
            serde_json::from_slice(&bytes).map_err(|source| {
                LocalEnvironmentHistoryError::Json {
                    path: path.clone(),
                    source,
                }
            })?;
        index.verify_against(history)?;
        Ok(Some(index))
    }

    pub fn save_index(
        &self,
        history: &LocalEnvironmentHistory,
        index: &EnvironmentAdmissionIndex,
    ) -> Result<(), LocalEnvironmentHistoryError> {
        index.verify_against(history)?;
        let target = self.index_path();
        let bytes = serde_json::to_vec_pretty(index).map_err(|source| {
            LocalEnvironmentHistoryError::Json {
                path: target.clone(),
                source,
            }
        })?;
        if bytes.len().saturating_add(1) as u64 > MAX_ENVIRONMENT_STATE_BYTES {
            return Err(LocalEnvironmentHistoryError::ByteLimit {
                path: target,
                limit: MAX_ENVIRONMENT_STATE_BYTES,
            });
        }
        self.prepare_directory()?;
        if let Ok(metadata) = fs::symlink_metadata(&target)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(LocalEnvironmentHistoryError::UnsafeStatePath(target));
        }
        let (temporary, mut file) = self.create_temporary(INDEX_FILE_NAME)?;
        let publication = (|| {
            file.write_all(&bytes)
                .and_then(|()| file.write_all(b"\n"))
                .and_then(|()| file.flush())?;
            drop(file);
            fs::rename(&temporary, &target)
        })();
        if let Err(source) = publication {
            let _ = fs::remove_file(&temporary);
            return Err(LocalEnvironmentHistoryError::Write {
                path: target,
                source,
            });
        }
        Ok(())
    }

    pub fn clear_index(&self) -> Result<(), LocalEnvironmentHistoryError> {
        self.verify_directory_boundary()?;
        let path = self.index_path();
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                Err(LocalEnvironmentHistoryError::UnsafeStatePath(path))
            }
            Ok(_) => fs::remove_file(&path)
                .map_err(|source| LocalEnvironmentHistoryError::Write { path, source }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(LocalEnvironmentHistoryError::Read { path, source }),
        }
    }

    fn prepare_directory(&self) -> Result<(), LocalEnvironmentHistoryError> {
        self.verify_directory_boundary()?;
        match fs::symlink_metadata(&self.directory) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => Err(
                LocalEnvironmentHistoryError::UnsafeStatePath(self.directory.clone()),
            ),
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.directory).map_err(|source| {
                    LocalEnvironmentHistoryError::Write {
                        path: self.directory.clone(),
                        source,
                    }
                })
            }
            Err(source) => Err(LocalEnvironmentHistoryError::Write {
                path: self.directory.clone(),
                source,
            }),
        }
    }

    fn verify_directory_boundary(&self) -> Result<(), LocalEnvironmentHistoryError> {
        for ancestor in self.directory.ancestors() {
            match fs::symlink_metadata(ancestor) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(LocalEnvironmentHistoryError::UnsafeStatePath(
                        ancestor.to_owned(),
                    ));
                }
                Ok(metadata) if ancestor == self.directory && !metadata.is_dir() => {
                    return Err(LocalEnvironmentHistoryError::UnsafeStatePath(
                        ancestor.to_owned(),
                    ));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(LocalEnvironmentHistoryError::Read {
                        path: ancestor.to_owned(),
                        source,
                    });
                }
            }
        }
        Ok(())
    }

    fn create_temporary(
        &self,
        file_name: &str,
    ) -> Result<(PathBuf, File), LocalEnvironmentHistoryError> {
        for attempt in 0..32_u8 {
            let path = self
                .directory
                .join(format!(".{file_name}.tmp-{}-{attempt}", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(LocalEnvironmentHistoryError::Write { path, source }),
            }
        }
        Err(LocalEnvironmentHistoryError::TemporaryLimit(
            self.directory.clone(),
        ))
    }
}

pub fn effective_index_snapshot(
    history: &LocalEnvironmentHistory,
    admission_index: Option<&EnvironmentAdmissionIndex>,
    working_snapshot: &CapabilitySnapshot,
) -> Result<CapabilitySnapshot, LocalEnvironmentHistoryError> {
    history.verify()?;
    if let Some(index) = admission_index {
        index.verify_against(history)?;
        Ok(index.snapshot.clone())
    } else if let Some(head) = history.head() {
        Ok(head.snapshot.clone())
    } else {
        empty_snapshot_like(working_snapshot)
    }
}

pub fn stage_selected_capabilities(
    index_snapshot: &CapabilitySnapshot,
    working_snapshot: &CapabilitySnapshot,
    selected: &BTreeSet<CapabilityKey>,
) -> Result<CapabilitySnapshot, LocalEnvironmentHistoryError> {
    index_snapshot.verify()?;
    working_snapshot.verify()?;
    if selected.is_empty() {
        return Err(LocalEnvironmentHistoryError::EmptyPatchSelection);
    }
    let max_changes = index_snapshot
        .capabilities
        .len()
        .saturating_add(working_snapshot.capabilities.len())
        .max(1) as u64;
    let delta = compare_capabilities(
        index_snapshot,
        working_snapshot,
        DeltaOptions {
            source_label: "INDEX".to_owned(),
            target_label: "WORKING".to_owned(),
            limits: DeltaLimits { max_changes },
            ..DeltaOptions::default()
        },
    )?;
    let changed_keys = delta
        .changes
        .iter()
        .map(|change| change.key.clone())
        .collect::<BTreeSet<_>>();
    if let Some(key) = selected.difference(&changed_keys).next() {
        return Err(LocalEnvironmentHistoryError::UnknownPatchSelection {
            provider_id: key.provider_id.clone(),
            capability_id: key.capability_id.clone(),
        });
    }
    let working = working_snapshot
        .capabilities
        .iter()
        .map(|record| (CapabilityKey::from(record), record.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut staged = index_snapshot
        .capabilities
        .iter()
        .map(|record| (CapabilityKey::from(record), record.clone()))
        .collect::<BTreeMap<CapabilityKey, CapabilityRecord>>();
    for key in selected {
        if let Some(record) = working.get(key) {
            staged.insert(key.clone(), record.clone());
        } else {
            staged.remove(key);
        }
    }
    CapabilitySnapshot::new(
        working_snapshot.profile.clone(),
        working_snapshot.limits.clone(),
        staged.into_values().collect(),
    )
    .map_err(Into::into)
}

fn empty_snapshot_like(
    snapshot: &CapabilitySnapshot,
) -> Result<CapabilitySnapshot, LocalEnvironmentHistoryError> {
    Ok(CapabilitySnapshot::new(
        snapshot.profile.clone(),
        snapshot.limits.clone(),
        Vec::new(),
    )?)
}

fn normalize_message(message: String) -> Result<String, LocalEnvironmentHistoryError> {
    let message = message.trim().to_owned();
    if message.is_empty() {
        return Err(LocalEnvironmentHistoryError::EmptyMessage);
    }
    if message.len() > MAX_ENVIRONMENT_MESSAGE_BYTES {
        return Err(LocalEnvironmentHistoryError::MessageLimit {
            limit: MAX_ENVIRONMENT_MESSAGE_BYTES,
        });
    }
    if message.contains('\0') {
        return Err(LocalEnvironmentHistoryError::MessageNul);
    }
    Ok(message)
}

fn commit_digest(
    sequence: u64,
    parent_commit_id: Option<&SemanticDigest>,
    message: &str,
    snapshot_id: &SemanticDigest,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(ENVIRONMENT_COMMIT_SCHEMA);
    hasher.add_u64(sequence);
    hasher.add_optional_str(parent_commit_id.map(SemanticDigest::as_str));
    hasher.add_str(message);
    hasher.add_str(snapshot_id.as_str());
    hasher.finish()
}

fn admission_index_digest(
    base_commit_id: Option<&SemanticDigest>,
    snapshot_id: &SemanticDigest,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(ENVIRONMENT_ADMISSION_INDEX_SCHEMA);
    hasher.add_optional_str(base_commit_id.map(SemanticDigest::as_str));
    hasher.add_str(snapshot_id.as_str());
    hasher.finish()
}

#[derive(Debug, Error)]
pub enum LocalEnvironmentHistoryError {
    #[error("unsupported local environment history schema {actual}")]
    UnsupportedHistorySchema { actual: String },
    #[error("unsupported environment commit schema {actual}")]
    UnsupportedCommitSchema { actual: String },
    #[error("unsupported environment admission index schema {actual}")]
    UnsupportedIndexSchema { actual: String },
    #[error("environment commit sequence must be {expected}, got {actual}")]
    InvalidSequence { expected: u64, actual: u64 },
    #[error("environment commit {sequence} does not name its exact predecessor")]
    ParentMismatch { sequence: u64 },
    #[error("environment commit {sequence} repeats its parent snapshot")]
    UnchangedCommit { sequence: u64 },
    #[error("duplicate environment commit {0}")]
    DuplicateCommit(SemanticDigest),
    #[error("environment commit digest {declared} does not match recomputed {actual}")]
    CommitDigest {
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("environment admission index digest {declared} does not match recomputed {actual}")]
    IndexDigest {
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error(
        "environment admission index is based on {actual:?}, expected current HEAD {expected:?}"
    )]
    StaleIndex {
        expected: Option<SemanticDigest>,
        actual: Option<SemanticDigest>,
    },
    #[error("environment commit message must not be empty")]
    EmptyMessage,
    #[error("environment commit message exceeds the {limit}-byte limit")]
    MessageLimit { limit: usize },
    #[error("environment commit message must not contain NUL")]
    MessageNul,
    #[error("environment commit message is not canonical")]
    NonCanonicalMessage,
    #[error("nothing to commit; working environment matches snapshot {0}")]
    NothingToCommit(SemanticDigest),
    #[error("nothing staged in the environment admission index")]
    NothingStaged,
    #[error("working environment has no changes to add")]
    NothingToAdd,
    #[error("no environment capability changes were selected")]
    EmptyPatchSelection,
    #[error("selected capability {provider_id}/{capability_id} is not an unstaged change")]
    UnknownPatchSelection {
        provider_id: String,
        capability_id: String,
    },
    #[error("environment history exceeds the {limit}-commit limit")]
    CommitLimit { limit: usize },
    #[error("log count must be between 1 and {limit}, got {actual}")]
    LogLimit { limit: usize, actual: usize },
    #[error("maximum changes must be greater than zero")]
    ZeroChangeLimit,
    #[error("environment operator projection could not be derived: {0}")]
    EnvironmentProjection(String),
    #[error("environment state {path} could not be read: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("environment state {path} could not be written: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("environment state {0} is not a safe regular file or directory")]
    UnsafeStatePath(PathBuf),
    #[error("environment state {path} exceeds the {limit}-byte limit")]
    ByteLimit { path: PathBuf, limit: u64 },
    #[error("environment state {path} is invalid JSON: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("could not allocate a temporary environment state file in {0}")]
    TemporaryLimit(PathBuf),
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
    #[error(transparent)]
    Delta(#[from] rey_diff::DeltaError),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rey_environment::{Availability, CapabilityRecord, DiscoveryLimits, TrustClass};
    use tempfile::TempDir;

    use super::{
        EnvironmentAdmissionIndex, EnvironmentLog, EnvironmentStatus, EnvironmentWorkingState,
        LocalEnvironmentHistory, LocalEnvironmentHistoryError, LocalEnvironmentStore,
    };

    fn snapshot(version: &str) -> rey_environment::CapabilitySnapshot {
        rey_environment::CapabilitySnapshot::new(
            "standalone",
            DiscoveryLimits::default(),
            vec![CapabilityRecord {
                provider_id: "fixture".to_owned(),
                provider_revision: 1,
                provider_kind: "fixture".to_owned(),
                capability_id: "fixture.capability".to_owned(),
                capability_kind: "compute".to_owned(),
                resolved_location: None,
                version: Some(version.to_owned()),
                content_digest: None,
                provenance: None,
                availability: Availability::Available,
                trust_class: TrustClass::BuiltIn,
                operations: vec!["inspect".to_owned()],
                enforced_limits: Vec::new(),
                unsupported_limits: Vec::new(),
                observed_at: None,
                error_code: None,
                error_detail: None,
            }],
        )
        .unwrap()
    }

    #[test]
    fn linear_history_status_and_log_recompute_exact_deltas() {
        let mut history = LocalEnvironmentHistory::default();
        assert!(matches!(
            history.commit("  ", snapshot("1")),
            Err(LocalEnvironmentHistoryError::EmptyMessage)
        ));
        let first = history.commit("baseline", snapshot("1")).unwrap();
        assert!(matches!(
            history.commit("empty", snapshot("1")),
            Err(LocalEnvironmentHistoryError::NothingToCommit(_))
        ));
        let second = history.commit("upgrade", snapshot("2")).unwrap();
        assert_eq!(second.parent_commit_id.as_ref(), Some(&first.commit_id));
        history.verify().unwrap();

        let clean = EnvironmentStatus::derive(&history, None, snapshot("2"), 64).unwrap();
        assert_eq!(clean.state, EnvironmentWorkingState::Clean);
        let changed = EnvironmentStatus::derive(&history, None, snapshot("3"), 64).unwrap();
        assert_eq!(changed.state, EnvironmentWorkingState::Changed);
        assert_eq!(changed.unstaged_delta.summary.modified, 1);

        let index = EnvironmentAdmissionIndex::new(&history, snapshot("3")).unwrap();
        let staged =
            EnvironmentStatus::derive(&history, Some(index.clone()), snapshot("3"), 64).unwrap();
        assert_eq!(staged.state, EnvironmentWorkingState::Staged);
        assert_eq!(staged.staged_delta.summary.modified, 1);
        assert_eq!(staged.unstaged_delta.summary.unchanged, 1);
        let mixed = EnvironmentStatus::derive(&history, Some(index), snapshot("4"), 64).unwrap();
        assert_eq!(mixed.state, EnvironmentWorkingState::Mixed);

        let log = EnvironmentLog::derive(&history, 1, 64, true).unwrap();
        assert_eq!(log.total_commits, 2);
        assert_eq!(log.selected_commits, 1);
        assert_eq!(log.entries[0].commit.commit_id, second.commit_id);
        assert_eq!(log.entries[0].delta.summary.modified, 1);
    }

    #[test]
    fn tampered_chain_and_commit_identity_fail_closed() {
        let mut history = LocalEnvironmentHistory::default();
        history.commit("baseline", snapshot("1")).unwrap();
        history.commit("upgrade", snapshot("2")).unwrap();

        let mut parent_tampered = history.clone();
        parent_tampered.commits[1].parent_commit_id = None;
        assert!(parent_tampered.verify().is_err());

        let mut message_tampered = history;
        message_tampered.commits[0].message = "changed".to_owned();
        assert!(matches!(
            message_tampered.verify(),
            Err(LocalEnvironmentHistoryError::CommitDigest { .. })
        ));
    }

    #[test]
    fn local_store_round_trips_verified_history() {
        let directory = TempDir::new().unwrap();
        let store = LocalEnvironmentStore::new(directory.path().join("env"));
        assert!(store.load().unwrap().commits.is_empty());
        let mut history = LocalEnvironmentHistory::default();
        history.commit("baseline", snapshot("1")).unwrap();
        store.save(&history).unwrap();
        assert_eq!(store.load().unwrap(), history);

        let index = EnvironmentAdmissionIndex::new(&history, snapshot("2")).unwrap();
        store.save_index(&history, &index).unwrap();
        assert_eq!(store.load_index(&history).unwrap(), Some(index.clone()));

        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(store.index_path()).unwrap()).unwrap();
        value["snapshot"]["capabilities"][0]["version"] = "tampered".into();
        fs::write(store.index_path(), serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(store.load_index(&history).is_err());
        store.save_index(&history, &index).unwrap();

        let mut advanced = history.clone();
        advanced.commit("upgrade", snapshot("2")).unwrap();
        assert!(matches!(
            store.load_index(&advanced),
            Err(LocalEnvironmentHistoryError::StaleIndex { .. })
        ));
        store.clear_index().unwrap();
        assert!(store.load_index(&history).unwrap().is_none());

        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(store.path()).unwrap()).unwrap();
        value["commits"][0]["message"] = "tampered".into();
        fs::write(store.path(), serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(store.load().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_state_file_is_rejected() {
        use std::os::unix::fs::symlink;

        let directory = TempDir::new().unwrap();
        let store = LocalEnvironmentStore::new(directory.path().join("env"));
        fs::create_dir_all(store.directory()).unwrap();
        let target = directory.path().join("target.json");
        fs::write(&target, b"{}\n").unwrap();
        symlink(target, store.path()).unwrap();
        assert!(matches!(
            store.load(),
            Err(LocalEnvironmentHistoryError::UnsafeStatePath(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_admission_index_is_rejected() {
        use std::os::unix::fs::symlink;

        let directory = TempDir::new().unwrap();
        let store = LocalEnvironmentStore::new(directory.path().join("env"));
        fs::create_dir_all(store.directory()).unwrap();
        let target = directory.path().join("target-index.json");
        fs::write(&target, b"{}\n").unwrap();
        symlink(target, store.index_path()).unwrap();
        assert!(matches!(
            store.load_index(&LocalEnvironmentHistory::default()),
            Err(LocalEnvironmentHistoryError::UnsafeStatePath(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_state_directory_ancestor_is_rejected() {
        use std::os::unix::fs::symlink;

        let directory = TempDir::new().unwrap();
        let target = directory.path().join("target");
        fs::create_dir(&target).unwrap();
        let linked = directory.path().join("linked");
        symlink(&target, &linked).unwrap();
        let store = LocalEnvironmentStore::new(linked.join("env"));
        assert!(matches!(
            store.load(),
            Err(LocalEnvironmentHistoryError::UnsafeStatePath(path)) if path == linked
        ));
    }
}
