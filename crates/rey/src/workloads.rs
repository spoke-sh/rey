use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

use chrono::{DateTime, Utc};
use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_diff::DeltaAssessment;
use rey_environment::{Availability, CapabilitySnapshot, ENVIRONMENT_MAP_PROVIDER_ID};
use rey_git::{GitActivationBudget, GitActivationProposal, GitSnapshot};
use rey_mining::{
    ProjectionPacket, SemanticAtlas, SemanticAtlasDelta, TopographyCoverage, TopographyPatch,
};
use rey_runtime::{
    AttentionPolicy, BUILT_IN_MISMATCH_WORKLOAD_ID, BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID,
    CONTEXT_ANCHOR_SURVEY_OPERATION_ID, ComputeGraph, GraphLimits, GraphNode, GraphOutput,
    PortfolioError, PortfolioLimits, PortfolioQualificationState, PortfolioReasoningEvidence,
    PortfolioSnapshot, PortfolioSurfaceObservation, PortfolioWorkloadObservation,
    QualificationRecord, RENDER_ADMITTED_REGIONAL_SCENE_OPERATION_ID,
    RENDER_TOPOGRAPHY_PATCH_OPERATION_ID, RunStatus, SCENE_ADMISSION_OPERATION_ID, Scenario,
    ScenarioSuite, SceneAdmissionResult, SceneAdmissionScenario, TestStatus,
    TopographySurveyScenario, ValueSource, ValueType, WorkloadAttention, WorkloadDefinition,
    WorkloadDefinitionParts, WorkloadGitDependency, WorkloadGitDependencyKind, WorkloadLimits,
    WorkloadOwnedSurface, WorkloadPort, WorkloadRunResult, WorkloadScenarioExecutionResult,
    WorkloadTestResult, WorkloadValue, built_in_operation_contract, built_in_workloads,
    utf8_exact_comparator_contract,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ignore::{ReyIgnoreError, ReyIgnoreFile, ReyIgnoreProjection};

pub const LOCAL_WORKLOAD_STATE_SCHEMA: &str = "rey.local-workload-state.v1";
pub const WORKLOAD_LIST_SCHEMA: &str = "rey.workload-list.v1";
pub const WORKLOAD_STATUS_SCHEMA: &str = "rey.workload-status.v1";
pub const WORKLOAD_STATUS_BATCH_SCHEMA: &str = "rey.workload-status-batch.v1";
pub const WORKLOAD_TEST_BATCH_SCHEMA: &str = "rey.workload-test-batch.v1";
pub const WORKLOAD_PACKAGE_SCHEMA: &str = "rey.workload-package.v1";
pub const WORKLOAD_CREATION_REQUEST_SCHEMA: &str = "rey.workload-creation-request.v1";
pub const WORKLOAD_CREATION_ATTENTION_SCHEMA: &str = "rey.workload-creation-attention.v1";
pub const WORKLOAD_CREATE_RESULT_SCHEMA: &str = "rey.workload-create-result.v1";
pub const WORKLOAD_CATALOG_SCHEMA: &str = "rey.workload-catalog.v1";
pub const WORKLOAD_RUN_VIEW_SCHEMA: &str = "rey.workload-run-view.v1";
pub const WORKLOAD_ADMISSION_SNAPSHOT_SCHEMA: &str = "rey.workload-admission-snapshot.v1";
pub const WORKLOAD_CHANGE_SET_SCHEMA: &str = "rey.workload-change-set.v1";
pub const WORKLOAD_REVISION_STATUS_SCHEMA: &str = "rey.workload-revision-status.v1";
pub const WORKLOAD_ADD_RESULT_SCHEMA: &str = "rey.workload-add-result.v1";
pub const WORKLOAD_COMMIT_SCHEMA: &str = "rey.workload-commit.v1";
pub const WORKLOAD_COMMIT_RESULT_SCHEMA: &str = "rey.workload-commit-result.v1";
pub const WORKLOAD_LOG_SCHEMA: &str = "rey.workload-log.v1";
pub const WORKLOAD_ACTIVATION_ADMISSION_SCHEMA: &str = "rey.workload-activation-admission.v1";
pub const WORKLOAD_ACTIVATION_EXECUTION_SCHEMA: &str = "rey.workload-activation-execution.v1";
pub const WORKLOAD_ACTIVATION_RECOMPUTATION_SCHEMA: &str =
    "rey.workload-activation-recomputation.v1";

const STATE_FILE_NAME: &str = "state.json";
const LOCK_FILE_NAME: &str = "workloads.lock";
const MAX_STATE_BYTES: u64 = 4 * 1_024 * 1_024;
const MAX_STATE_RECORDS: usize = 64;
const MAX_RETAINED_SEMANTIC_ATLASES: usize = 64;
const WORKLOAD_PACKAGE_FILE_NAME: &str = "workload.yaml";
const WORKLOAD_CREATION_REQUEST_FILE_NAME: &str = "request.yaml";
const MAX_WORKLOAD_PACKAGES: usize = 128;
const MAX_WORKLOAD_PACKAGE_BYTES: u64 = 1_024 * 1_024;
const MAX_GENERATION_INPUTS: usize = 64;
const MAX_PROVENANCE_TEXT_BYTES: usize = 1_024;
const MAX_WORKLOAD_INTENT_BYTES: usize = 16 * 1_024;
const MAX_WORKLOAD_COMMITS: usize = 256;
const MAX_COMMIT_MESSAGE_BYTES: usize = 4_096;
const MAX_WORKLOAD_ACTIVATION_ADMISSIONS: usize = 256;
const MAX_WORKLOAD_ACTIVATION_EXECUTIONS: usize = 256;
const MAX_WORKLOAD_ACTIVATION_EVIDENCE_BYTES: u64 = 4 * 1_024 * 1_024;
const MAX_WORKLOAD_ACTIVATION_RECOMPUTATIONS: usize = 256;
pub const MAX_WORKLOAD_RECOMPUTATION_EVIDENCE_BYTES: u64 = 4 * 1_024 * 1_024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadCatalogKind {
    WorkspacePackages,
    BuiltInConformance,
}

impl WorkloadCatalogKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::WorkspacePackages => "WORKSPACE PACKAGES",
            Self::BuiltInConformance => "BUILT-IN CONFORMANCE",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadOrigin {
    WorkspacePackage,
    BuiltInConformance,
    BuiltInSystem,
}

impl WorkloadOrigin {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::WorkspacePackage => "WORKSPACE PACKAGE",
            Self::BuiltInConformance => "BUILT-IN CONFORMANCE",
            Self::BuiltInSystem => "BUILT-IN SYSTEM",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadProposalKind {
    CodingHarness,
    Rule,
    Human,
}

impl WorkloadProposalKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::CodingHarness => "CODING HARNESS",
            Self::Rule => "RULE",
            Self::Human => "HUMAN",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedWorkloadArtifact {
    ComputeGraph,
    ScenarioSuite,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadGeneratorProvenance {
    pub kind: WorkloadProposalKind,
    pub producer: String,
    pub producer_revision: String,
    pub generated: Vec<GeneratedWorkloadArtifact>,
    pub inputs: Vec<WorkloadGenerationInput>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadGenerationInput {
    pub source: String,
    pub revision: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadAdmissionState {
    Proposed,
    Accepted,
    Rejected,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioOraclePolicy {
    Mutable,
    Frozen,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadAdmission {
    pub state: WorkloadAdmissionState,
    pub scenario_oracle: ScenarioOraclePolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadProvenance {
    pub origin: WorkloadOrigin,
    pub source: String,
    pub source_digest: Option<SemanticDigest>,
    pub generation: Option<WorkloadGeneratorProvenance>,
    pub admission: WorkloadAdmission,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadCatalogDescriptor {
    pub schema: String,
    pub kind: WorkloadCatalogKind,
    pub root: Option<String>,
    pub workload_count: u64,
    pub admitted_count: u64,
    pub draft_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadCreationLimits {
    pub max_package_bytes: u64,
    pub max_graph_nodes: u64,
    pub max_scenarios: u64,
    pub max_string_bytes: u64,
}

impl Default for WorkloadCreationLimits {
    fn default() -> Self {
        let graph = GraphLimits::default();
        let workload = WorkloadLimits::default();
        Self {
            max_package_bytes: MAX_WORKLOAD_PACKAGE_BYTES,
            max_graph_nodes: graph.max_nodes,
            max_scenarios: workload.max_scenarios,
            max_string_bytes: workload.max_string_bytes,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadCreationRequest {
    pub schema: String,
    pub request_id: SemanticDigest,
    pub workload_id: String,
    pub title: String,
    pub intent: Option<String>,
    pub proposer: WorkloadProposalKind,
    pub catalog_root: String,
    pub target_package: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attention: Option<WorkloadCreationAttentionBinding>,
    pub requirements: Vec<String>,
    pub limits: WorkloadCreationLimits,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadCreationAttentionBinding {
    pub schema: String,
    pub portfolio_snapshot_id: SemanticDigest,
    pub environment_snapshot_id: SemanticDigest,
    pub attention_id: SemanticDigest,
    pub attention_row_id: SemanticDigest,
    pub frontier_id: SemanticDigest,
    pub frontier_row_id: SemanticDigest,
    pub scheduling_decision_id: SemanticDigest,
    pub reasoning_surface_id: SemanticDigest,
    pub action: rey_runtime::AttentionAction,
    pub reason: rey_runtime::AttentionReason,
    pub subject_id: String,
    pub current_package_revision: Option<SemanticDigest>,
    pub evidence_ids: Vec<SemanticDigest>,
    pub dependency_ids: Vec<String>,
    pub delta_ids: Vec<SemanticDigest>,
    pub admissible_action_ids: Vec<String>,
    pub surface_limits: WorkloadCreationSurfaceLimits,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadCreationSurfaceLimits {
    pub max_rows: u64,
    pub max_delta_refs: u64,
    pub max_evidence_refs: u64,
    pub max_action_refs: u64,
    pub max_omissions: u64,
    pub max_total_evidence_bytes: u64,
    pub max_string_bytes: u64,
    pub max_retrieval_iterations: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadDraft {
    pub request: WorkloadCreationRequest,
    pub source: String,
    pub source_digest: SemanticDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadCreateResult {
    pub schema: String,
    pub draft: WorkloadDraft,
    pub created_files: Vec<String>,
    pub action_required: bool,
    pub instructions: Vec<String>,
    pub next: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadPackageSnapshot {
    pub workload_id: String,
    pub workload_revision: u64,
    pub title: String,
    pub source: String,
    pub source_digest: SemanticDigest,
    pub object_path: String,
    pub bytes: u64,
    pub generation: WorkloadGeneratorProvenance,
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario_suite: ContractIdentity,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadAdmissionSnapshot {
    pub schema: String,
    pub snapshot_revision: SemanticDigest,
    pub packages: Vec<WorkloadPackageSnapshot>,
    pub ignore: Option<ReyIgnoreProjection>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadChangeKind {
    Inserted,
    Deleted,
    Modified,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadChange {
    pub workload_id: String,
    pub change_kind: WorkloadChangeKind,
    pub source_revision: Option<SemanticDigest>,
    pub target_revision: Option<SemanticDigest>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadChangeSet {
    pub schema: String,
    pub source_label: String,
    pub target_label: String,
    pub source_revision: Option<SemanticDigest>,
    pub target_revision: Option<SemanticDigest>,
    pub assessment: DeltaAssessment,
    pub inserted: u64,
    pub deleted: u64,
    pub modified: u64,
    pub changes: Vec<WorkloadChange>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadWorkingState {
    Clean,
    Working,
    Staged,
    Mixed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadCommit {
    pub schema: String,
    pub commit_id: SemanticDigest,
    pub sequence: u64,
    pub parent_commit_id: Option<SemanticDigest>,
    pub committed_at_unix: i64,
    pub message: String,
    pub snapshot: WorkloadAdmissionSnapshot,
    pub qualification_ids: Vec<SemanticDigest>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadRevisionStatus {
    pub schema: String,
    pub state: WorkloadWorkingState,
    pub head: Option<WorkloadCommit>,
    pub index: Option<WorkloadAdmissionSnapshot>,
    pub working: WorkloadAdmissionSnapshot,
    pub staged: WorkloadChangeSet,
    pub unstaged: WorkloadChangeSet,
    pub drafts: Vec<WorkloadDraft>,
    pub commit_ready: bool,
    pub qualification_omissions: Vec<String>,
    pub admission_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadAddResult {
    pub schema: String,
    pub staged: bool,
    pub snapshot: WorkloadAdmissionSnapshot,
    pub delta: WorkloadChangeSet,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadCommitResult {
    pub schema: String,
    pub commit: WorkloadCommit,
    pub delta: WorkloadChangeSet,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadLog {
    pub schema: String,
    pub head_commit_id: Option<SemanticDigest>,
    pub total_commits: u64,
    pub selected_commits: u64,
    pub patch: bool,
    pub commits: Vec<WorkloadCommit>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadActivationAdmission {
    pub schema: String,
    pub admission_id: SemanticDigest,
    pub activation: GitActivationProposal,
    pub workload_head_commit_id: SemanticDigest,
    pub workload_head_snapshot_id: SemanticDigest,
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario_suite: ContractIdentity,
    pub evaluator: ContractIdentity,
    pub declared_scenarios: Vec<ContractIdentity>,
    pub selected_scenario_ids: Vec<String>,
    pub capability_snapshot_id: SemanticDigest,
    pub effective_budget: GitActivationBudget,
    pub authority: String,
}

impl WorkloadActivationAdmission {
    pub fn new(
        activation: GitActivationProposal,
        workload_head: &WorkloadCommit,
        workload: &WorkloadDefinition,
        capability_snapshot_id: SemanticDigest,
    ) -> Result<Self, LocalWorkloadStateError> {
        activation.verify()?;
        workload_head.verify()?;
        workload.verify()?;
        if activation.workload_id != workload.workload.id
            || activation.graph != workload.graph.graph
            || !is_semantic_digest(&capability_snapshot_id)
            || !workload_head.snapshot.packages.iter().any(|package| {
                package.workload == workload.workload
                    && package.graph == workload.graph.graph
                    && package.scenario_suite == workload.scenario_suite.suite
            })
        {
            return Err(LocalWorkloadStateError::ActivationPrecondition(
                "activation does not bind the exact admitted workload HEAD".to_owned(),
            ));
        }
        let mut declared_scenarios = workload
            .scenario_suite
            .scenarios
            .iter()
            .map(|scenario| scenario.scenario.clone())
            .collect::<Vec<_>>();
        declared_scenarios.sort_by(|left, right| left.id.cmp(&right.id));
        let declared_scenario_ids = declared_scenarios
            .iter()
            .map(|scenario| scenario.id.clone())
            .collect::<BTreeSet<_>>();
        let mut selected_scenario_ids = if activation.scenario_ids.is_empty() {
            declared_scenario_ids.iter().cloned().collect::<Vec<_>>()
        } else {
            activation.scenario_ids.clone()
        };
        selected_scenario_ids.sort();
        if selected_scenario_ids.is_empty()
            || !is_canonical(&selected_scenario_ids)
            || selected_scenario_ids
                .iter()
                .any(|scenario| !declared_scenario_ids.contains(scenario))
            || selected_scenario_ids.len() as u64 > activation.budget.max_scenarios
            || selected_scenario_ids.len() as u64 > workload.limits.max_scenarios
        {
            return Err(LocalWorkloadStateError::ActivationPrecondition(
                "activation scenario selection is unknown, noncanonical, or over budget".to_owned(),
            ));
        }
        let mut admission = Self {
            schema: WORKLOAD_ACTIVATION_ADMISSION_SCHEMA.to_owned(),
            admission_id: SemanticHasher::new("rey.workload-activation-admission.pending.v1")
                .finish(),
            activation,
            workload_head_commit_id: workload_head.commit_id.clone(),
            workload_head_snapshot_id: workload_head.snapshot.snapshot_revision.clone(),
            workload: workload.workload.clone(),
            graph: workload.graph.graph.clone(),
            scenario_suite: workload.scenario_suite.suite.clone(),
            evaluator: workload.evaluator.clone(),
            declared_scenarios,
            effective_budget: GitActivationBudget {
                max_scenarios: selected_scenario_ids.len() as u64,
                max_actions: 1,
                max_evidence_bytes: MAX_WORKLOAD_ACTIVATION_EVIDENCE_BYTES,
            },
            selected_scenario_ids,
            capability_snapshot_id,
            authority: "admitted_for_runtime_scheduling; no workload or Git execution has occurred"
                .to_owned(),
        };
        admission.effective_budget.max_evidence_bytes = admission
            .effective_budget
            .max_evidence_bytes
            .min(admission.activation.budget.max_evidence_bytes);
        admission.admission_id = workload_activation_admission_digest(&admission);
        admission.verify()?;
        Ok(admission)
    }

    pub fn verify(&self) -> Result<(), LocalWorkloadStateError> {
        self.activation.verify()?;
        for contract in [
            &self.workload,
            &self.graph,
            &self.scenario_suite,
            &self.evaluator,
        ] {
            if contract.id.is_empty()
                || contract.revision == 0
                || !is_semantic_digest(&contract.semantic_digest)
            {
                return Err(LocalWorkloadStateError::InvalidActivationAdmission);
            }
        }
        if self.schema != WORKLOAD_ACTIVATION_ADMISSION_SCHEMA
            || !is_semantic_digest(&self.admission_id)
            || !is_semantic_digest(&self.workload_head_commit_id)
            || !is_semantic_digest(&self.workload_head_snapshot_id)
            || !is_semantic_digest(&self.capability_snapshot_id)
            || self.activation.workload_id != self.workload.id
            || self.activation.graph != self.graph
            || self.declared_scenarios.is_empty()
            || self.declared_scenarios.iter().any(|scenario| {
                scenario.id.is_empty()
                    || scenario.revision == 0
                    || !is_semantic_digest(&scenario.semantic_digest)
            })
            || self
                .declared_scenarios
                .windows(2)
                .any(|window| window[0].id >= window[1].id)
            || self.selected_scenario_ids.is_empty()
            || !is_canonical(&self.selected_scenario_ids)
            || self.selected_scenario_ids.iter().any(|scenario| {
                self.declared_scenarios
                    .binary_search_by(|declared| declared.id.as_str().cmp(scenario))
                    .is_err()
            })
            || if self.activation.scenario_ids.is_empty() {
                self.selected_scenario_ids
                    != self
                        .declared_scenarios
                        .iter()
                        .map(|scenario| scenario.id.clone())
                        .collect::<Vec<_>>()
            } else {
                self.selected_scenario_ids != self.activation.scenario_ids
            }
            || self.effective_budget.max_scenarios != self.selected_scenario_ids.len() as u64
            || self.effective_budget.max_scenarios > self.activation.budget.max_scenarios
            || self.effective_budget.max_actions != 1
            || self.effective_budget.max_actions > self.activation.budget.max_actions
            || self.effective_budget.max_evidence_bytes == 0
            || self.effective_budget.max_evidence_bytes > self.activation.budget.max_evidence_bytes
            || self.effective_budget.max_evidence_bytes > MAX_WORKLOAD_ACTIVATION_EVIDENCE_BYTES
            || self.authority
                != "admitted_for_runtime_scheduling; no workload or Git execution has occurred"
            || self.admission_id != workload_activation_admission_digest(self)
        {
            return Err(LocalWorkloadStateError::InvalidActivationAdmission);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadActivationExecution {
    pub schema: String,
    pub execution_id: SemanticDigest,
    pub admission_id: SemanticDigest,
    pub activation_id: SemanticDigest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_execution_id: Option<SemanticDigest>,
    pub result: WorkloadScenarioExecutionResult,
    pub evidence_bytes: u64,
    pub authority: String,
}

impl WorkloadActivationExecution {
    pub fn new(
        admission: &WorkloadActivationAdmission,
        workload: &WorkloadDefinition,
        result: WorkloadScenarioExecutionResult,
    ) -> Result<Self, LocalWorkloadStateError> {
        admission.verify()?;
        result.verify_for(workload)?;
        let evidence_bytes = serde_json::to_vec(&result)
            .map_err(|_| LocalWorkloadStateError::InvalidActivationExecution)?
            .len() as u64;
        let mut execution = Self {
            schema: WORKLOAD_ACTIVATION_EXECUTION_SCHEMA.to_owned(),
            execution_id: SemanticHasher::new("rey.workload-activation-execution.pending.v1")
                .finish(),
            admission_id: admission.admission_id.clone(),
            activation_id: admission.activation.activation_id.clone(),
            source_execution_id: None,
            result,
            evidence_bytes,
            authority: "activation_scenarios_evaluated; no Git mutation occurred".to_owned(),
        };
        execution.execution_id = workload_activation_execution_digest(&execution);
        execution.verify_for(admission)?;
        Ok(execution)
    }

    pub fn coalesce(
        admission: &WorkloadActivationAdmission,
        source_admission: &WorkloadActivationAdmission,
        source: &Self,
    ) -> Result<Option<Self>, LocalWorkloadStateError> {
        admission.verify()?;
        source.verify_for(source_admission)?;
        if source.source_execution_id.is_some()
            || admission.activation.transition_id != source_admission.activation.transition_id
            || admission.activation.source_snapshot_id
                != source_admission.activation.source_snapshot_id
            || admission.activation.target_snapshot_id
                != source_admission.activation.target_snapshot_id
            || admission.workload_head_commit_id != source_admission.workload_head_commit_id
            || admission.workload_head_snapshot_id != source_admission.workload_head_snapshot_id
            || admission.workload != source_admission.workload
            || admission.graph != source_admission.graph
            || admission.scenario_suite != source_admission.scenario_suite
            || admission.evaluator != source_admission.evaluator
            || admission.declared_scenarios != source_admission.declared_scenarios
            || admission.selected_scenario_ids != source_admission.selected_scenario_ids
            || admission.capability_snapshot_id != source_admission.capability_snapshot_id
            || source.evidence_bytes > admission.effective_budget.max_evidence_bytes
        {
            return Ok(None);
        }
        let mut execution = Self {
            schema: WORKLOAD_ACTIVATION_EXECUTION_SCHEMA.to_owned(),
            execution_id: SemanticHasher::new(
                "rey.workload-activation-coalesced-execution.pending.v1",
            )
            .finish(),
            admission_id: admission.admission_id.clone(),
            activation_id: admission.activation.activation_id.clone(),
            source_execution_id: Some(source.execution_id.clone()),
            result: source.result.clone(),
            evidence_bytes: source.evidence_bytes,
            authority: "activation_coalesced_with_retained_execution; graph was not executed again"
                .to_owned(),
        };
        execution.execution_id = workload_activation_execution_digest(&execution);
        execution.verify_for(admission)?;
        Ok(Some(execution))
    }

    pub fn verify_for(
        &self,
        admission: &WorkloadActivationAdmission,
    ) -> Result<(), LocalWorkloadStateError> {
        admission.verify()?;
        self.result.verify()?;
        let evidence_bytes = serde_json::to_vec(&self.result)
            .map_err(|_| LocalWorkloadStateError::InvalidActivationExecution)?
            .len() as u64;
        let declared_scenarios = admission
            .declared_scenarios
            .iter()
            .filter(|scenario| {
                admission
                    .selected_scenario_ids
                    .binary_search(&scenario.id)
                    .is_ok()
            })
            .collect::<Vec<_>>();
        if self.schema != WORKLOAD_ACTIVATION_EXECUTION_SCHEMA
            || !is_semantic_digest(&self.execution_id)
            || self.admission_id != admission.admission_id
            || self.activation_id != admission.activation.activation_id
            || self
                .source_execution_id
                .as_ref()
                .is_some_and(|source| !is_semantic_digest(source))
            || self.result.workload != admission.workload
            || self.result.graph != admission.graph
            || self.result.scenario_suite != admission.scenario_suite
            || self.result.evaluator != admission.evaluator
            || self.result.capability_snapshot_id != admission.capability_snapshot_id
            || self.result.selected_scenario_ids != admission.selected_scenario_ids
            || self.result.scenarios.len() != declared_scenarios.len()
            || self
                .result
                .scenarios
                .iter()
                .zip(declared_scenarios)
                .any(|(result, declared)| result.scenario != *declared)
            || self.evidence_bytes == 0
            || self.evidence_bytes != evidence_bytes
            || self.evidence_bytes > admission.effective_budget.max_evidence_bytes
            || admission.effective_budget.max_actions != 1
            || match &self.source_execution_id {
                None => {
                    self.authority != "activation_scenarios_evaluated; no Git mutation occurred"
                }
                Some(_) => {
                    self.authority
                        != "activation_coalesced_with_retained_execution; graph was not executed again"
                }
            }
            || self.execution_id != workload_activation_execution_digest(self)
        {
            return Err(LocalWorkloadStateError::InvalidActivationExecution);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadRecomputationAssessment {
    Equivalent,
    Different,
}

impl WorkloadRecomputationAssessment {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Equivalent => "equivalent",
            Self::Different => "different",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadScenarioRecomputationComparison {
    pub scenario_id: String,
    pub selected_execution_id: SemanticDigest,
    pub full_execution_id: SemanticDigest,
    pub equivalent: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadActivationRecomputation {
    pub schema: String,
    pub recomputation_id: SemanticDigest,
    pub execution_id: SemanticDigest,
    pub admission_id: SemanticDigest,
    pub selected_result_id: SemanticDigest,
    pub full_result: WorkloadScenarioExecutionResult,
    pub full_evidence_bytes: u64,
    pub max_evidence_bytes: u64,
    pub comparisons: Vec<WorkloadScenarioRecomputationComparison>,
    pub assessment: WorkloadRecomputationAssessment,
    pub authority: String,
}

impl WorkloadActivationRecomputation {
    pub fn new(
        execution: &WorkloadActivationExecution,
        admission: &WorkloadActivationAdmission,
        full_result: WorkloadScenarioExecutionResult,
        max_evidence_bytes: u64,
    ) -> Result<Self, LocalWorkloadStateError> {
        execution.verify_for(admission)?;
        full_result.verify()?;
        let full_evidence_bytes = serde_json::to_vec(&full_result)
            .map_err(|_| LocalWorkloadStateError::InvalidActivationRecomputation)?
            .len() as u64;
        if max_evidence_bytes == 0
            || max_evidence_bytes > MAX_WORKLOAD_RECOMPUTATION_EVIDENCE_BYTES
            || full_evidence_bytes > max_evidence_bytes
        {
            return Err(LocalWorkloadStateError::ActivationRecomputationBudget {
                observed: full_evidence_bytes,
                limit: max_evidence_bytes,
            });
        }
        let comparisons = recomputation_comparisons(execution, &full_result)?;
        let assessment = if comparisons.iter().all(|comparison| comparison.equivalent) {
            WorkloadRecomputationAssessment::Equivalent
        } else {
            WorkloadRecomputationAssessment::Different
        };
        let mut recomputation = Self {
            schema: WORKLOAD_ACTIVATION_RECOMPUTATION_SCHEMA.to_owned(),
            recomputation_id: SemanticHasher::new(
                "rey.workload-activation-recomputation.pending.v1",
            )
            .finish(),
            execution_id: execution.execution_id.clone(),
            admission_id: admission.admission_id.clone(),
            selected_result_id: execution.result.result_id.clone(),
            full_result,
            full_evidence_bytes,
            max_evidence_bytes,
            comparisons,
            assessment,
            authority:
                "comparison_evidence_only; recomputation does not qualify the workload or execute Git"
                    .to_owned(),
        };
        recomputation.recomputation_id = workload_activation_recomputation_digest(&recomputation);
        recomputation.verify_for(execution, admission)?;
        Ok(recomputation)
    }

    pub fn verify_for(
        &self,
        execution: &WorkloadActivationExecution,
        admission: &WorkloadActivationAdmission,
    ) -> Result<(), LocalWorkloadStateError> {
        execution.verify_for(admission)?;
        self.full_result.verify()?;
        let full_evidence_bytes = serde_json::to_vec(&self.full_result)
            .map_err(|_| LocalWorkloadStateError::InvalidActivationRecomputation)?
            .len() as u64;
        let declared_scenario_ids = admission
            .declared_scenarios
            .iter()
            .map(|scenario| scenario.id.clone())
            .collect::<Vec<_>>();
        let declared_scenarios_match = self
            .full_result
            .scenarios
            .iter()
            .zip(&admission.declared_scenarios)
            .all(|(result, declared)| result.scenario == *declared);
        let comparisons = recomputation_comparisons(execution, &self.full_result)?;
        let assessment = if comparisons.iter().all(|comparison| comparison.equivalent) {
            WorkloadRecomputationAssessment::Equivalent
        } else {
            WorkloadRecomputationAssessment::Different
        };
        if self.schema != WORKLOAD_ACTIVATION_RECOMPUTATION_SCHEMA
            || !is_semantic_digest(&self.recomputation_id)
            || self.execution_id != execution.execution_id
            || self.admission_id != admission.admission_id
            || self.selected_result_id != execution.result.result_id
            || self.full_result.workload != admission.workload
            || self.full_result.graph != admission.graph
            || self.full_result.scenario_suite != admission.scenario_suite
            || self.full_result.evaluator != admission.evaluator
            || self.full_result.capability_snapshot_id != admission.capability_snapshot_id
            || self.full_result.selected_scenario_ids != declared_scenario_ids
            || self.full_result.scenarios.len() != admission.declared_scenarios.len()
            || !declared_scenarios_match
            || self.full_evidence_bytes == 0
            || self.full_evidence_bytes != full_evidence_bytes
            || self.max_evidence_bytes == 0
            || self.max_evidence_bytes > MAX_WORKLOAD_RECOMPUTATION_EVIDENCE_BYTES
            || self.full_evidence_bytes > self.max_evidence_bytes
            || self.comparisons != comparisons
            || self.assessment != assessment
            || self.authority
                != "comparison_evidence_only; recomputation does not qualify the workload or execute Git"
            || self.recomputation_id != workload_activation_recomputation_digest(self)
        {
            return Err(LocalWorkloadStateError::InvalidActivationRecomputation);
        }
        Ok(())
    }
}

fn recomputation_comparisons(
    execution: &WorkloadActivationExecution,
    full_result: &WorkloadScenarioExecutionResult,
) -> Result<Vec<WorkloadScenarioRecomputationComparison>, LocalWorkloadStateError> {
    execution.result.verify()?;
    full_result.verify()?;
    execution
        .result
        .scenarios
        .iter()
        .map(|selected| {
            let full = full_result
                .scenarios
                .iter()
                .find(|candidate| candidate.scenario.id == selected.scenario.id)
                .ok_or(LocalWorkloadStateError::InvalidActivationRecomputation)?;
            Ok(WorkloadScenarioRecomputationComparison {
                scenario_id: selected.scenario.id.clone(),
                selected_execution_id: selected.execution_id.clone(),
                full_execution_id: full.execution_id.clone(),
                equivalent: selected == full,
            })
        })
        .collect()
}

impl WorkloadAdmissionSnapshot {
    fn new(
        mut packages: Vec<WorkloadPackageSnapshot>,
        ignore: Option<ReyIgnoreProjection>,
    ) -> Result<Self, LocalWorkloadStateError> {
        packages.sort_by(|left, right| left.workload_id.cmp(&right.workload_id));
        let mut snapshot = Self {
            schema: WORKLOAD_ADMISSION_SNAPSHOT_SCHEMA.to_owned(),
            snapshot_revision: workload_digest_placeholder(),
            packages,
            ignore,
        };
        snapshot.snapshot_revision = workload_snapshot_identity(&snapshot);
        snapshot.verify()?;
        Ok(snapshot)
    }

    fn verify(&self) -> Result<(), LocalWorkloadStateError> {
        if self.schema != WORKLOAD_ADMISSION_SNAPSHOT_SCHEMA {
            return Err(LocalWorkloadStateError::SnapshotSchema(self.schema.clone()));
        }
        if self.packages.len() > MAX_WORKLOAD_PACKAGES {
            return Err(LocalWorkloadStateError::SnapshotLimit(
                MAX_WORKLOAD_PACKAGES,
            ));
        }
        let mut previous = None;
        for package in &self.packages {
            validate_workload_id(&package.workload_id)?;
            if previous.is_some_and(|previous| previous >= package.workload_id.as_str()) {
                return Err(LocalWorkloadStateError::NonCanonicalSnapshot);
            }
            previous = Some(package.workload_id.as_str());
            if package.workload.id != package.workload_id
                || package.workload.revision != package.workload_revision
                || package.source.is_empty()
                || package.object_path.is_empty()
                || package.bytes == 0
            {
                return Err(LocalWorkloadStateError::SnapshotPackage(
                    package.workload_id.clone(),
                ));
            }
        }
        if self.snapshot_revision != workload_snapshot_identity(self) {
            return Err(LocalWorkloadStateError::SnapshotIdentity);
        }
        Ok(())
    }
}

impl WorkloadChangeSet {
    #[must_use]
    pub fn derive(
        source_label: &str,
        source: Option<&WorkloadAdmissionSnapshot>,
        target_label: &str,
        target: Option<&WorkloadAdmissionSnapshot>,
    ) -> Self {
        let source_packages = source
            .into_iter()
            .flat_map(|snapshot| &snapshot.packages)
            .map(|package| (package.workload_id.as_str(), &package.source_digest))
            .collect::<BTreeMap<_, _>>();
        let target_packages = target
            .into_iter()
            .flat_map(|snapshot| &snapshot.packages)
            .map(|package| (package.workload_id.as_str(), &package.source_digest))
            .collect::<BTreeMap<_, _>>();
        let ids = source_packages
            .keys()
            .chain(target_packages.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        let changes = ids
            .into_iter()
            .filter_map(|workload_id| {
                let source_revision = source_packages.get(workload_id).copied().cloned();
                let target_revision = target_packages.get(workload_id).copied().cloned();
                let change_kind = match (&source_revision, &target_revision) {
                    (None, Some(_)) => WorkloadChangeKind::Inserted,
                    (Some(_), None) => WorkloadChangeKind::Deleted,
                    (Some(source), Some(target)) if source != target => {
                        WorkloadChangeKind::Modified
                    }
                    _ => return None,
                };
                Some(WorkloadChange {
                    workload_id: workload_id.to_owned(),
                    change_kind,
                    source_revision,
                    target_revision,
                })
            })
            .collect::<Vec<_>>();
        let inserted = changes
            .iter()
            .filter(|change| change.change_kind == WorkloadChangeKind::Inserted)
            .count() as u64;
        let deleted = changes
            .iter()
            .filter(|change| change.change_kind == WorkloadChangeKind::Deleted)
            .count() as u64;
        let modified = changes
            .iter()
            .filter(|change| change.change_kind == WorkloadChangeKind::Modified)
            .count() as u64;
        Self {
            schema: WORKLOAD_CHANGE_SET_SCHEMA.to_owned(),
            source_label: source_label.to_owned(),
            target_label: target_label.to_owned(),
            source_revision: source.map(|snapshot| snapshot.snapshot_revision.clone()),
            target_revision: target.map(|snapshot| snapshot.snapshot_revision.clone()),
            assessment: if changes.is_empty()
                && source.map(|snapshot| &snapshot.snapshot_revision)
                    == target.map(|snapshot| &snapshot.snapshot_revision)
            {
                DeltaAssessment::Equal
            } else {
                DeltaAssessment::Different
            },
            inserted,
            deleted,
            modified,
            changes,
        }
    }
}

impl WorkloadCommit {
    fn new(
        sequence: u64,
        parent_commit_id: Option<SemanticDigest>,
        message: String,
        snapshot: WorkloadAdmissionSnapshot,
        qualification_ids: Vec<SemanticDigest>,
    ) -> Result<Self, LocalWorkloadStateError> {
        let message = normalize_workload_commit_message(message)?;
        let committed_at_unix = Utc::now().timestamp();
        let mut commit = Self {
            schema: WORKLOAD_COMMIT_SCHEMA.to_owned(),
            commit_id: workload_digest_placeholder(),
            sequence,
            parent_commit_id,
            committed_at_unix,
            message,
            snapshot,
            qualification_ids,
        };
        commit.commit_id = workload_commit_identity(&commit);
        commit.verify()?;
        Ok(commit)
    }

    fn verify(&self) -> Result<(), LocalWorkloadStateError> {
        if self.schema != WORKLOAD_COMMIT_SCHEMA {
            return Err(LocalWorkloadStateError::CommitSchema(self.schema.clone()));
        }
        if self.sequence == 0
            || DateTime::<Utc>::from_timestamp(self.committed_at_unix, 0).is_none()
            || normalize_workload_commit_message(self.message.clone())? != self.message
        {
            return Err(LocalWorkloadStateError::CommitIdentity);
        }
        self.snapshot.verify()?;
        if self.qualification_ids.len() != self.snapshot.packages.len() {
            return Err(LocalWorkloadStateError::CommitQualificationShape);
        }
        if self.commit_id != workload_commit_identity(self) {
            return Err(LocalWorkloadStateError::CommitIdentity);
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ObservedWorkloadAdmission {
    catalog: WorkloadCatalog,
    snapshot: WorkloadAdmissionSnapshot,
    artifacts: BTreeMap<String, Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedWorkload {
    pub definition: WorkloadDefinition,
    pub provenance: WorkloadProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkloadCatalog {
    pub descriptor: WorkloadCatalogDescriptor,
    pub workloads: Vec<ResolvedWorkload>,
    pub drafts: Vec<WorkloadDraft>,
}

#[derive(Debug)]
enum WorkloadCatalogDocument {
    Package {
        path: PathBuf,
        request: Option<PathBuf>,
    },
    Draft(PathBuf),
}

impl WorkloadCatalog {
    pub fn load_workspace(
        workspace: &Path,
        catalog_dir: &Path,
    ) -> Result<Self, WorkloadCatalogError> {
        validate_relative_catalog_dir(catalog_dir)?;
        let descriptor_root = catalog_dir.display().to_string();
        let root = match existing_catalog_root(workspace, catalog_dir)? {
            Some(root) => root,
            None => {
                return Ok(Self {
                    descriptor: WorkloadCatalogDescriptor {
                        schema: WORKLOAD_CATALOG_SCHEMA.to_owned(),
                        kind: WorkloadCatalogKind::WorkspacePackages,
                        root: Some(descriptor_root),
                        workload_count: 0,
                        admitted_count: 0,
                        draft_count: 0,
                    },
                    workloads: Vec::new(),
                    drafts: Vec::new(),
                });
            }
        };

        let mut documents = Vec::new();
        for entry in fs::read_dir(&root).map_err(|source| WorkloadCatalogError::Path {
            path: root.clone(),
            source,
        })? {
            let entry = entry.map_err(|source| WorkloadCatalogError::Path {
                path: root.clone(),
                source,
            })?;
            let path = entry.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|source| WorkloadCatalogError::Path {
                    path: path.clone(),
                    source,
                })?;
            if metadata.file_type().is_symlink() {
                return Err(WorkloadCatalogError::UnsafePath(path));
            }
            if metadata.is_dir() {
                let manifest = path.join(WORKLOAD_PACKAGE_FILE_NAME);
                let request = path.join(WORKLOAD_CREATION_REQUEST_FILE_NAME);
                let manifest_exists = validate_optional_regular_file(&manifest)?;
                let request_exists = validate_optional_regular_file(&request)?;
                match (manifest_exists, request_exists) {
                    (true, request_exists) => documents.push(WorkloadCatalogDocument::Package {
                        path: manifest,
                        request: request_exists.then_some(request),
                    }),
                    (false, true) => documents.push(WorkloadCatalogDocument::Draft(request)),
                    (false, false) => {
                        return Err(WorkloadCatalogError::MissingCatalogDocument(path));
                    }
                }
            }
        }
        documents
            .sort_by(|left, right| catalog_document_path(left).cmp(catalog_document_path(right)));
        if documents.len() > MAX_WORKLOAD_PACKAGES {
            return Err(WorkloadCatalogError::PackageLimit {
                limit: MAX_WORKLOAD_PACKAGES,
                actual: documents.len(),
            });
        }

        let mut ids = BTreeSet::new();
        let mut workloads = Vec::new();
        let mut drafts = Vec::new();
        for document in documents {
            match document {
                WorkloadCatalogDocument::Package { path, request } => {
                    let bytes = read_bounded_catalog_file(&path)?;
                    let supplied: SuppliedWorkloadPackage = serde_saphyr::from_slice(&bytes)?;
                    let source = relative_source(workspace, &path);
                    let resolved =
                        supplied.resolve(source, &bytes, WorkloadAdmissionState::Proposed)?;
                    let workload_id = resolved.definition.workload.id.clone();
                    if let Some(request) = request {
                        let draft = load_workload_draft(workspace, &request)?;
                        if draft.request.workload_id != workload_id {
                            return Err(WorkloadCatalogError::RequestIdentity {
                                request: draft.request.workload_id,
                                package: workload_id,
                            });
                        }
                        validate_harness_response_lineage(&resolved, &draft)?;
                    }
                    if !ids.insert(workload_id.clone()) {
                        return Err(WorkloadCatalogError::DuplicateWorkload(workload_id));
                    }
                    workloads.push(resolved);
                }
                WorkloadCatalogDocument::Draft(path) => {
                    let draft = load_workload_draft(workspace, &path)?;
                    if !ids.insert(draft.request.workload_id.clone()) {
                        return Err(WorkloadCatalogError::DuplicateWorkload(
                            draft.request.workload_id,
                        ));
                    }
                    drafts.push(draft);
                }
            }
        }
        workloads.sort_by(|left, right| {
            left.definition
                .workload
                .id
                .cmp(&right.definition.workload.id)
        });
        drafts.sort_by(|left, right| left.request.workload_id.cmp(&right.request.workload_id));
        Ok(Self {
            descriptor: WorkloadCatalogDescriptor {
                schema: WORKLOAD_CATALOG_SCHEMA.to_owned(),
                kind: WorkloadCatalogKind::WorkspacePackages,
                root: Some(descriptor_root),
                workload_count: workloads.len().saturating_add(drafts.len()) as u64,
                admitted_count: 0,
                draft_count: drafts.len() as u64,
            },
            workloads,
            drafts,
        })
    }

    pub fn built_in_conformance() -> Result<Self, WorkloadCatalogError> {
        let workloads = built_in_workloads()?
            .into_iter()
            .map(|definition| {
                let origin = if definition.workload.id == BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID {
                    WorkloadOrigin::BuiltInSystem
                } else {
                    WorkloadOrigin::BuiltInConformance
                };
                ResolvedWorkload {
                    definition,
                    provenance: WorkloadProvenance {
                        origin,
                        source: "rey-runtime compiled catalog".to_owned(),
                        source_digest: None,
                        generation: None,
                        admission: WorkloadAdmission {
                            state: WorkloadAdmissionState::Accepted,
                            scenario_oracle: ScenarioOraclePolicy::Frozen,
                        },
                    },
                }
            })
            .collect::<Vec<_>>();
        Ok(Self {
            descriptor: WorkloadCatalogDescriptor {
                schema: WORKLOAD_CATALOG_SCHEMA.to_owned(),
                kind: WorkloadCatalogKind::BuiltInConformance,
                root: None,
                workload_count: workloads.len() as u64,
                admitted_count: workloads.len() as u64,
                draft_count: 0,
            },
            workloads,
            drafts: Vec::new(),
        })
    }

    pub fn select(
        &self,
        workload_id: Option<&str>,
    ) -> Result<Vec<ResolvedWorkload>, WorkloadCatalogError> {
        match workload_id {
            Some(id) => self
                .workloads
                .iter()
                .find(|workload| workload.definition.workload.id == id)
                .cloned()
                .map(|workload| vec![workload])
                .ok_or_else(|| {
                    if self
                        .drafts
                        .iter()
                        .any(|draft| draft.request.workload_id == id)
                    {
                        WorkloadCatalogError::WorkloadAwaitingHarness(id.to_owned())
                    } else {
                        WorkloadCatalogError::UnknownWorkload {
                            id: id.to_owned(),
                            catalog: self.descriptor.kind,
                        }
                    }
                }),
            None => Ok(self.workloads.clone()),
        }
    }

    #[must_use]
    pub fn select_drafts(&self, workload_id: Option<&str>) -> Vec<WorkloadDraft> {
        match workload_id {
            Some(id) => self
                .drafts
                .iter()
                .filter(|draft| draft.request.workload_id == id)
                .cloned()
                .collect(),
            None => self.drafts.clone(),
        }
    }

    #[must_use]
    pub fn definitions(&self) -> Vec<WorkloadDefinition> {
        self.workloads
            .iter()
            .map(|workload| workload.definition.clone())
            .collect()
    }

    pub fn create_workspace_request(
        workspace: &Path,
        catalog_dir: &Path,
        workload_id: &str,
        title: Option<&str>,
        intent: Option<&str>,
        attention: Option<WorkloadCreationAttentionBinding>,
    ) -> Result<WorkloadCreateResult, WorkloadCatalogError> {
        validate_relative_catalog_dir(catalog_dir)?;
        validate_workload_id(workload_id)?;
        let title = title.unwrap_or(workload_id);
        validate_creation_text("title", title, MAX_PROVENANCE_TEXT_BYTES, false)?;
        if let Some(intent) = intent {
            validate_creation_text("intent", intent, MAX_WORKLOAD_INTENT_BYTES, false)?;
        }

        let catalog_root = catalog_dir.display().to_string();
        let package_dir = catalog_dir.join(workload_id);
        let target_package = package_dir.join(WORKLOAD_PACKAGE_FILE_NAME);
        let requirements = workload_creation_requirements(attention.is_some());
        let limits = WorkloadCreationLimits::default();
        let target_package = target_package.display().to_string();
        let request_id = workload_creation_request_digest(&WorkloadCreationDigestInput {
            workload_id,
            title,
            intent,
            catalog_root: &catalog_root,
            target_package: &target_package,
            attention: attention.as_ref(),
            requirements: &requirements,
            limits: &limits,
        });
        let request = WorkloadCreationRequest {
            schema: WORKLOAD_CREATION_REQUEST_SCHEMA.to_owned(),
            request_id,
            workload_id: workload_id.to_owned(),
            title: title.to_owned(),
            intent: intent.map(str::to_owned),
            proposer: WorkloadProposalKind::CodingHarness,
            catalog_root,
            target_package,
            attention,
            requirements,
            limits,
        };
        request.verify()?;
        let mut bytes = serde_json::to_vec_pretty(&request)?;
        bytes.push(b'\n');

        let root = prepare_catalog_root(workspace, catalog_dir)?;
        let directory = root.join(workload_id);
        match fs::symlink_metadata(&directory) {
            Ok(_) => {
                return Err(WorkloadCatalogError::WorkloadAlreadyExists(
                    workload_id.to_owned(),
                ));
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(WorkloadCatalogError::Path {
                    path: directory,
                    source,
                });
            }
        }
        fs::create_dir(&directory).map_err(|source| WorkloadCatalogError::Path {
            path: directory.clone(),
            source,
        })?;
        let request_path = directory.join(WORKLOAD_CREATION_REQUEST_FILE_NAME);
        let write_result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&request_path)
                .map_err(|source| WorkloadCatalogError::Path {
                    path: request_path.clone(),
                    source,
                })?;
            file.write_all(&bytes)
                .and_then(|()| file.flush())
                .map_err(|source| WorkloadCatalogError::Path {
                    path: request_path.clone(),
                    source,
                })
        })();
        if let Err(error) = write_result {
            let _ = fs::remove_file(&request_path);
            let _ = fs::remove_dir(&directory);
            return Err(error);
        }

        let source = relative_source(workspace, &request_path);
        let source_digest = source_digest("rey.workload-creation-request-source.v1", &bytes);
        let draft = WorkloadDraft {
            request,
            source: source.clone(),
            source_digest,
        };
        let next = format!(
            "Coding harness: hydrate {source} into {} and retain exact generation inputs",
            draft.request.target_package
        );
        Ok(WorkloadCreateResult {
            schema: WORKLOAD_CREATE_RESULT_SCHEMA.to_owned(),
            created_files: vec![source],
            action_required: true,
            instructions: draft.request.requirements.clone(),
            next,
            draft,
        })
    }
}

impl WorkloadCreationAttentionBinding {
    pub fn from_runtime(
        snapshot: &PortfolioSnapshot,
        attention: &WorkloadAttention,
        runtime: &PortfolioReasoningEvidence,
        requested_row_id: &str,
    ) -> Result<Self, WorkloadCatalogError> {
        runtime.verify_against(snapshot, attention)?;
        let attention_row = attention
            .rows
            .iter()
            .find(|row| row.row_id.as_str() == requested_row_id)
            .ok_or_else(|| {
                WorkloadCatalogError::UnknownAttentionRow(requested_row_id.to_owned())
            })?;
        if attention_row.action != rey_runtime::AttentionAction::Create
            || attention_row.subject_kind != rey_runtime::AttentionSubjectKind::Surface
            || attention_row.readiness != rey_runtime::AttentionReadiness::Ready
        {
            return Err(WorkloadCatalogError::CreationAttentionRequired(
                requested_row_id.to_owned(),
            ));
        }
        let frontier_row = runtime
            .frontier
            .rows
            .iter()
            .find(|row| {
                row.claim_ids.contains(&format!(
                    "rey.workload-attention-row:{}",
                    attention_row.row_id
                ))
            })
            .ok_or_else(|| {
                WorkloadCatalogError::UnscheduledAttentionRow(requested_row_id.to_owned())
            })?;
        if !runtime
            .scheduling
            .selected
            .iter()
            .any(|selected| selected.frontier_row_id == frontier_row.row_id)
        {
            return Err(WorkloadCatalogError::UnscheduledAttentionRow(
                requested_row_id.to_owned(),
            ));
        }
        let surface = runtime.surface.as_ref().ok_or_else(|| {
            WorkloadCatalogError::UnscheduledAttentionRow(requested_row_id.to_owned())
        })?;
        let surface_row = surface
            .rows
            .iter()
            .find(|row| row.frontier_row_id == frontier_row.row_id.as_str())
            .ok_or_else(|| {
                WorkloadCatalogError::UnscheduledAttentionRow(requested_row_id.to_owned())
            })?;
        let mut delta_ids = surface_row.transition_delta_ids.clone();
        delta_ids.extend(surface_row.residual_delta_ids.clone());
        delta_ids.sort();
        delta_ids.dedup();
        let limits = &surface.limits;
        let binding = Self {
            schema: WORKLOAD_CREATION_ATTENTION_SCHEMA.to_owned(),
            portfolio_snapshot_id: snapshot.snapshot_id.clone(),
            environment_snapshot_id: runtime.frontier.inputs.capability_snapshot_id.clone(),
            attention_id: attention.attention_id.clone(),
            attention_row_id: attention_row.row_id.clone(),
            frontier_id: runtime.frontier.frontier_id.clone(),
            frontier_row_id: frontier_row.row_id.clone(),
            scheduling_decision_id: runtime.scheduling.decision_id.clone(),
            reasoning_surface_id: surface.surface_id.clone(),
            action: attention_row.action,
            reason: attention_row.reason,
            subject_id: attention_row.subject_id.clone(),
            current_package_revision: None,
            evidence_ids: attention_row.evidence_ids.clone(),
            dependency_ids: attention_row.dependency_ids.clone(),
            delta_ids,
            admissible_action_ids: surface_row.admissible_action_ids.clone(),
            surface_limits: WorkloadCreationSurfaceLimits {
                max_rows: limits.max_rows,
                max_delta_refs: limits.max_delta_refs,
                max_evidence_refs: limits.max_evidence_refs,
                max_action_refs: limits.max_action_refs,
                max_omissions: limits.max_omissions,
                max_total_evidence_bytes: limits.max_total_evidence_bytes,
                max_string_bytes: limits.max_string_bytes,
                max_retrieval_iterations: limits.max_retrieval_iterations,
            },
        };
        binding.verify()?;
        Ok(binding)
    }

    fn verify(&self) -> Result<(), WorkloadCatalogError> {
        let digests = [
            &self.portfolio_snapshot_id,
            &self.environment_snapshot_id,
            &self.attention_id,
            &self.attention_row_id,
            &self.frontier_id,
            &self.frontier_row_id,
            &self.scheduling_decision_id,
            &self.reasoning_surface_id,
        ];
        if self.schema != WORKLOAD_CREATION_ATTENTION_SCHEMA
            || self.action != rey_runtime::AttentionAction::Create
            || self.subject_id.trim().is_empty()
            || self.current_package_revision.is_some()
            || self.admissible_action_ids != ["rey.action.create-workload"]
            || digests.iter().any(|digest| !is_semantic_digest(digest))
            || !is_canonical(&self.evidence_ids)
            || !is_canonical(&self.dependency_ids)
            || !is_canonical(&self.delta_ids)
            || !is_canonical(&self.admissible_action_ids)
            || [
                self.surface_limits.max_rows,
                self.surface_limits.max_delta_refs,
                self.surface_limits.max_evidence_refs,
                self.surface_limits.max_action_refs,
                self.surface_limits.max_omissions,
                self.surface_limits.max_total_evidence_bytes,
                self.surface_limits.max_string_bytes,
                self.surface_limits.max_retrieval_iterations,
            ]
            .contains(&0)
        {
            return Err(WorkloadCatalogError::InvalidAttentionBinding(
                self.attention_row_id.to_string(),
            ));
        }
        Ok(())
    }
}

impl WorkloadCreationRequest {
    fn verify(&self) -> Result<(), WorkloadCatalogError> {
        if self.schema != WORKLOAD_CREATION_REQUEST_SCHEMA {
            return Err(WorkloadCatalogError::UnsupportedCreationRequestSchema(
                self.schema.clone(),
            ));
        }
        validate_workload_id(&self.workload_id)?;
        validate_creation_text("title", &self.title, MAX_PROVENANCE_TEXT_BYTES, false)?;
        if let Some(intent) = &self.intent {
            validate_creation_text("intent", intent, MAX_WORKLOAD_INTENT_BYTES, false)?;
        }
        if self.proposer != WorkloadProposalKind::CodingHarness
            || self.requirements != workload_creation_requirements(self.attention.is_some())
            || self.limits != WorkloadCreationLimits::default()
        {
            return Err(WorkloadCatalogError::InvalidCreationRequest(
                self.workload_id.clone(),
            ));
        }
        if let Some(attention) = &self.attention {
            attention.verify()?;
        }
        validate_relative_catalog_dir(Path::new(&self.catalog_root))?;
        let expected_target = Path::new(&self.catalog_root)
            .join(&self.workload_id)
            .join(WORKLOAD_PACKAGE_FILE_NAME)
            .display()
            .to_string();
        if self.target_package != expected_target
            || self.request_id
                != workload_creation_request_digest(&WorkloadCreationDigestInput {
                    workload_id: &self.workload_id,
                    title: &self.title,
                    intent: self.intent.as_deref(),
                    catalog_root: &self.catalog_root,
                    target_package: &self.target_package,
                    attention: self.attention.as_ref(),
                    requirements: &self.requirements,
                    limits: &self.limits,
                })
        {
            return Err(WorkloadCatalogError::InvalidCreationRequest(
                self.workload_id.clone(),
            ));
        }
        Ok(())
    }
}

fn catalog_document_path(document: &WorkloadCatalogDocument) -> &Path {
    match document {
        WorkloadCatalogDocument::Package { path, .. } | WorkloadCatalogDocument::Draft(path) => {
            path
        }
    }
}

fn validate_optional_regular_file(path: &Path) -> Result<bool, WorkloadCatalogError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(WorkloadCatalogError::UnsafePath(path.to_owned()))
        }
        Ok(_) => Ok(true),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(WorkloadCatalogError::Path {
            path: path.to_owned(),
            source,
        }),
    }
}

fn read_bounded_catalog_file(path: &Path) -> Result<Vec<u8>, WorkloadCatalogError> {
    let bytes = fs::read(path).map_err(|source| WorkloadCatalogError::Path {
        path: path.to_owned(),
        source,
    })?;
    if bytes.len() as u64 > MAX_WORKLOAD_PACKAGE_BYTES {
        return Err(WorkloadCatalogError::ByteLimit {
            path: path.to_owned(),
            limit: MAX_WORKLOAD_PACKAGE_BYTES,
            actual: bytes.len() as u64,
        });
    }
    Ok(bytes)
}

fn relative_source(workspace: &Path, path: &Path) -> String {
    path.strip_prefix(workspace)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn source_digest(domain: &str, bytes: &[u8]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(domain);
    hasher.add_bytes(bytes);
    hasher.finish()
}

fn load_workload_draft(
    workspace: &Path,
    path: &Path,
) -> Result<WorkloadDraft, WorkloadCatalogError> {
    let bytes = read_bounded_catalog_file(path)?;
    let request: WorkloadCreationRequest = serde_saphyr::from_slice(&bytes)?;
    request.verify()?;
    let directory_id = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if request.workload_id != directory_id {
        return Err(WorkloadCatalogError::RequestDirectoryIdentity {
            request: request.workload_id,
            directory: directory_id.to_owned(),
        });
    }
    Ok(WorkloadDraft {
        request,
        source: relative_source(workspace, path),
        source_digest: source_digest("rey.workload-creation-request-source.v1", &bytes),
    })
}

fn workload_creation_requirements(attention_bound: bool) -> Vec<String> {
    let mut requirements = vec![
        "Mine exact authoritative workspace and environment sources; retain their revision references."
            .to_owned(),
        "Define a bounded typed compute graph using only admitted operation contracts."
            .to_owned(),
        "Generate required and optional scenarios from authoritative behavior; never derive expected values from candidate execution."
            .to_owned(),
        "Freeze the scenario oracle, stage the complete package, and leave admission to an exact qualified workload commit."
            .to_owned(),
        "Materialize the target workload.yaml and preserve request.yaml as creation lineage."
            .to_owned(),
    ];
    if attention_bound {
        requirements.push(
            "Treat the exact selected attention row and reasoning surface as immutable request preconditions; cite the complete request bytes in generation inputs."
                .to_owned(),
        );
    }
    requirements
}

struct WorkloadCreationDigestInput<'a> {
    workload_id: &'a str,
    title: &'a str,
    intent: Option<&'a str>,
    catalog_root: &'a str,
    target_package: &'a str,
    attention: Option<&'a WorkloadCreationAttentionBinding>,
    requirements: &'a [String],
    limits: &'a WorkloadCreationLimits,
}

fn workload_creation_request_digest(input: &WorkloadCreationDigestInput<'_>) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(WORKLOAD_CREATION_REQUEST_SCHEMA);
    hasher.add_str(input.workload_id);
    hasher.add_str(input.title);
    hasher.add_optional_str(input.intent);
    hasher.add_str("coding_harness");
    hasher.add_str(input.catalog_root);
    hasher.add_str(input.target_package);
    if let Some(attention) = input.attention {
        hasher.add_str(WORKLOAD_CREATION_ATTENTION_SCHEMA);
        for digest in [
            &attention.portfolio_snapshot_id,
            &attention.environment_snapshot_id,
            &attention.attention_id,
            &attention.attention_row_id,
            &attention.frontier_id,
            &attention.frontier_row_id,
            &attention.scheduling_decision_id,
            &attention.reasoning_surface_id,
        ] {
            hasher.add_str(digest.as_str());
        }
        hasher.add_str(attention.action.as_str());
        hasher.add_str(attention.reason.as_str());
        hasher.add_str(&attention.subject_id);
        hasher.add_optional_str(
            attention
                .current_package_revision
                .as_ref()
                .map(SemanticDigest::as_str),
        );
        add_creation_digests(&mut hasher, &attention.evidence_ids);
        add_creation_strings(&mut hasher, &attention.dependency_ids);
        add_creation_digests(&mut hasher, &attention.delta_ids);
        add_creation_strings(&mut hasher, &attention.admissible_action_ids);
        for limit in [
            attention.surface_limits.max_rows,
            attention.surface_limits.max_delta_refs,
            attention.surface_limits.max_evidence_refs,
            attention.surface_limits.max_action_refs,
            attention.surface_limits.max_omissions,
            attention.surface_limits.max_total_evidence_bytes,
            attention.surface_limits.max_string_bytes,
            attention.surface_limits.max_retrieval_iterations,
        ] {
            hasher.add_u64(limit);
        }
    }
    hasher.add_u64(input.requirements.len() as u64);
    for requirement in input.requirements {
        hasher.add_str(requirement);
    }
    hasher.add_u64(input.limits.max_package_bytes);
    hasher.add_u64(input.limits.max_graph_nodes);
    hasher.add_u64(input.limits.max_scenarios);
    hasher.add_u64(input.limits.max_string_bytes);
    hasher.finish()
}

fn add_creation_digests(hasher: &mut SemanticHasher, values: &[SemanticDigest]) {
    hasher.add_u64(values.len() as u64);
    for value in values {
        hasher.add_str(value.as_str());
    }
}

fn add_creation_strings(hasher: &mut SemanticHasher, values: &[String]) {
    hasher.add_u64(values.len() as u64);
    for value in values {
        hasher.add_str(value);
    }
}

fn is_semantic_digest(digest: &SemanticDigest) -> bool {
    let Some(value) = digest.as_str().strip_prefix("blake3:") else {
        return false;
    };
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_canonical<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|window| window[0] < window[1])
}

fn validate_workload_id(workload_id: &str) -> Result<(), WorkloadCatalogError> {
    let valid = !workload_id.is_empty()
        && workload_id.len() <= MAX_PROVENANCE_TEXT_BYTES
        && workload_id != "."
        && workload_id != ".."
        && workload_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if !valid {
        return Err(WorkloadCatalogError::InvalidWorkloadId(
            workload_id.to_owned(),
        ));
    }
    Ok(())
}

fn validate_creation_text(
    field: &'static str,
    value: &str,
    limit: usize,
    allow_empty: bool,
) -> Result<(), WorkloadCatalogError> {
    if (!allow_empty && value.trim().is_empty()) || value.len() > limit || value.contains('\0') {
        return Err(WorkloadCatalogError::InvalidCreationText { field, limit });
    }
    Ok(())
}

fn prepare_catalog_root(
    workspace: &Path,
    catalog_dir: &Path,
) -> Result<PathBuf, WorkloadCatalogError> {
    let mut current = workspace.to_owned();
    for component in catalog_dir.components() {
        let Component::Normal(component) = component else {
            return Err(WorkloadCatalogError::CatalogPathEscape(
                catalog_dir.to_owned(),
            ));
        };
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(WorkloadCatalogError::UnsafePath(current));
            }
            Ok(_) => {}
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|source| WorkloadCatalogError::Path {
                    path: current.clone(),
                    source,
                })?;
            }
            Err(source) => {
                return Err(WorkloadCatalogError::Path {
                    path: current,
                    source,
                });
            }
        }
    }
    Ok(current)
}

fn existing_catalog_root(
    workspace: &Path,
    catalog_dir: &Path,
) -> Result<Option<PathBuf>, WorkloadCatalogError> {
    let mut current = workspace.to_owned();
    for component in catalog_dir.components() {
        let Component::Normal(component) = component else {
            return Err(WorkloadCatalogError::CatalogPathEscape(
                catalog_dir.to_owned(),
            ));
        };
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(WorkloadCatalogError::UnsafePath(current));
            }
            Ok(_) => {}
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(WorkloadCatalogError::Path {
                    path: current,
                    source,
                });
            }
        }
    }
    Ok(Some(current))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuppliedWorkloadPackage {
    schema: String,
    workload: SuppliedWorkloadIdentity,
    generation: WorkloadGeneratorProvenance,
    #[serde(default)]
    ownership: SuppliedWorkloadOwnership,
    graph: SuppliedGraph,
    scenarios: SuppliedScenarioSuite,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuppliedWorkloadOwnership {
    #[serde(default)]
    surfaces: Vec<SuppliedOwnedSurface>,
    #[serde(default)]
    git_dependencies: Vec<WorkloadGitDependency>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuppliedOwnedSurface {
    surface_id: String,
    source_revision: String,
    #[serde(default)]
    required_capabilities: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuppliedWorkloadIdentity {
    id: String,
    revision: u64,
    title: String,
    inputs: Vec<WorkloadPort>,
    outputs: Vec<WorkloadPort>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuppliedContractReference {
    id: String,
    revision: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuppliedGraph {
    id: String,
    revision: u64,
    nodes: Vec<SuppliedGraphNode>,
    outputs: Vec<SuppliedGraphOutput>,
    #[serde(default)]
    limits: GraphLimits,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuppliedGraphNode {
    id: String,
    operation: SuppliedContractReference,
    input: ValueSource,
    value_type: ValueType,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuppliedGraphOutput {
    id: String,
    source: ValueSource,
    value_type: ValueType,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuppliedScenarioSuite {
    id: String,
    revision: u64,
    cases: Vec<SuppliedScenario>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuppliedScenario {
    id: String,
    revision: u64,
    required: bool,
    inputs: BTreeMap<String, String>,
    expected: BTreeMap<String, String>,
    #[serde(default)]
    survey: Option<TopographySurveyScenario>,
    #[serde(default)]
    scene: Option<SceneAdmissionScenario>,
}

impl SuppliedWorkloadPackage {
    fn resolve(
        self,
        source: String,
        bytes: &[u8],
        admission_state: WorkloadAdmissionState,
    ) -> Result<ResolvedWorkload, WorkloadCatalogError> {
        if self.schema != WORKLOAD_PACKAGE_SCHEMA {
            return Err(WorkloadCatalogError::UnsupportedPackageSchema(self.schema));
        }
        validate_generation(&self.generation)?;
        if self
            .workload
            .inputs
            .iter()
            .chain(&self.workload.outputs)
            .any(|port| port.value_type != ValueType::Utf8)
        {
            return Err(WorkloadCatalogError::UnsupportedPackageValueType);
        }
        if self
            .graph
            .outputs
            .iter()
            .any(|output| output.value_type != ValueType::Utf8)
        {
            return Err(WorkloadCatalogError::UnsupportedPackageValueType);
        }
        let nodes = self
            .graph
            .nodes
            .into_iter()
            .map(|node| {
                let supported = matches!(
                    (node.operation.id.as_str(), node.value_type),
                    (
                        "rey.builtin.utf8.trim" | "rey.builtin.utf8.uppercase",
                        ValueType::Utf8
                    ) | (
                        CONTEXT_ANCHOR_SURVEY_OPERATION_ID,
                        ValueType::TopographyPatch
                    ) | (RENDER_TOPOGRAPHY_PATCH_OPERATION_ID, ValueType::Utf8)
                        | (SCENE_ADMISSION_OPERATION_ID, ValueType::SceneAdmission)
                        | (RENDER_ADMITTED_REGIONAL_SCENE_OPERATION_ID, ValueType::Utf8)
                );
                if !supported {
                    return Err(WorkloadCatalogError::UnsupportedPackageOperation(
                        node.operation.id,
                    ));
                }
                Ok(GraphNode {
                    node_id: node.id,
                    operation: built_in_operation_contract(
                        &node.operation.id,
                        node.operation.revision,
                    )?,
                    input: node.input,
                    output_id: "value".to_owned(),
                    value_type: node.value_type,
                })
            })
            .collect::<Result<Vec<_>, WorkloadCatalogError>>()?;
        let outputs = self
            .graph
            .outputs
            .into_iter()
            .map(|output| GraphOutput {
                output_id: output.id,
                source: output.source,
                value_type: output.value_type,
            })
            .collect();
        let graph = ComputeGraph::new(
            &self.graph.id,
            self.graph.revision,
            nodes,
            outputs,
            self.graph.limits,
        )?;
        let scenarios = self
            .scenarios
            .cases
            .into_iter()
            .map(|scenario| -> Result<Scenario, WorkloadCatalogError> {
                let id = format!("{}.scenario.{}", self.workload.id, scenario.id);
                let inputs = scenario
                    .inputs
                    .into_iter()
                    .map(|(id, value)| (id, WorkloadValue::Utf8(value)))
                    .collect();
                let expected = scenario
                    .expected
                    .into_iter()
                    .map(|(id, value)| (id, WorkloadValue::Utf8(value)))
                    .collect();
                Ok(match (scenario.survey, scenario.scene) {
                    (Some(survey), None) => Scenario::new_versioned_topography(
                        &id,
                        scenario.revision,
                        scenario.required,
                        inputs,
                        expected,
                        survey,
                    ),
                    (None, Some(scene)) => Scenario::new_versioned_scene_admission(
                        &id,
                        scenario.revision,
                        scenario.required,
                        inputs,
                        expected,
                        scene,
                    ),
                    (None, None) => Scenario::new_versioned(
                        &id,
                        scenario.revision,
                        scenario.required,
                        inputs,
                        expected,
                        None,
                    ),
                    (Some(_), Some(_)) => {
                        return Err(WorkloadCatalogError::MultipleScenarioProbeFamilies);
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let scenario_suite =
            ScenarioSuite::new_versioned(&self.scenarios.id, self.scenarios.revision, scenarios);
        let mut source_hasher = SemanticHasher::new("rey.workload-package-source.v1");
        source_hasher.add_bytes(bytes);
        let source_digest = source_hasher.finish();
        let proposal = ContractIdentity::new(
            format!("{}.proposal", self.workload.id),
            self.workload.revision,
            &format!("{source}\n{source_digest}"),
        );
        let definition = WorkloadDefinition::from_parts(WorkloadDefinitionParts {
            id: self.workload.id,
            revision: self.workload.revision,
            title: self.workload.title,
            owned_surfaces: self
                .ownership
                .surfaces
                .into_iter()
                .map(|surface| WorkloadOwnedSurface {
                    surface_id: surface.surface_id,
                    source_revision: surface.source_revision,
                    required_capability_ids: surface.required_capabilities,
                })
                .collect(),
            git_dependencies: self.ownership.git_dependencies,
            proposal: Some(proposal),
            inputs: self.workload.inputs,
            outputs: self.workload.outputs,
            graph,
            scenario_suite,
            evaluator: utf8_exact_comparator_contract(),
            limits: WorkloadLimits::default(),
        })?;
        Ok(ResolvedWorkload {
            definition,
            provenance: WorkloadProvenance {
                origin: WorkloadOrigin::WorkspacePackage,
                source,
                source_digest: Some(source_digest),
                generation: Some(self.generation),
                admission: WorkloadAdmission {
                    state: admission_state,
                    scenario_oracle: ScenarioOraclePolicy::Frozen,
                },
            },
        })
    }
}

fn validate_generation(
    generation: &WorkloadGeneratorProvenance,
) -> Result<(), WorkloadCatalogError> {
    if generation.producer.trim().is_empty()
        || generation.producer_revision.trim().is_empty()
        || generation.producer.len() > MAX_PROVENANCE_TEXT_BYTES
        || generation.producer_revision.len() > MAX_PROVENANCE_TEXT_BYTES
        || generation.inputs.is_empty()
        || generation.inputs.len() > MAX_GENERATION_INPUTS
        || generation
            .inputs
            .iter()
            .any(|input| input.source.trim().is_empty() || input.revision.trim().is_empty())
        || generation.inputs.iter().any(|input| {
            input.source.len() > MAX_PROVENANCE_TEXT_BYTES
                || input.revision.len() > MAX_PROVENANCE_TEXT_BYTES
        })
    {
        return Err(WorkloadCatalogError::InvalidGenerationProvenance);
    }
    let generated = generation
        .generated
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if generated.len() != generation.generated.len()
        || !generated.contains(&GeneratedWorkloadArtifact::ComputeGraph)
        || !generated.contains(&GeneratedWorkloadArtifact::ScenarioSuite)
    {
        return Err(WorkloadCatalogError::IncompleteGenerationProvenance);
    }
    Ok(())
}

fn validate_harness_response_lineage(
    workload: &ResolvedWorkload,
    draft: &WorkloadDraft,
) -> Result<(), WorkloadCatalogError> {
    let lineage_error = || WorkloadCatalogError::HarnessResponseLineage {
        workload: workload.definition.workload.id.clone(),
        request_source: draft.source.clone(),
        revision: draft.source_digest.clone(),
    };
    let generation = workload
        .provenance
        .generation
        .as_ref()
        .ok_or_else(lineage_error)?;
    if !generation.inputs.iter().any(|input| {
        input.source == draft.source && input.revision == draft.source_digest.to_string()
    }) {
        return Err(lineage_error());
    }
    Ok(())
}

fn validate_relative_catalog_dir(path: &Path) -> Result<(), WorkloadCatalogError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(WorkloadCatalogError::CatalogPathEscape(path.to_owned()));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum WorkloadCatalogError {
    #[error("relative workload catalog path {0} escapes or does not name a workspace directory")]
    CatalogPathEscape(PathBuf),
    #[error("workload catalog path {0} is symlinked or has the wrong file type")]
    UnsafePath(PathBuf),
    #[error("workload catalog path {path} failed: {source}")]
    Path {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("workload catalog document {path} exceeds the {limit}-byte limit with {actual} bytes")]
    ByteLimit {
        path: PathBuf,
        limit: u64,
        actual: u64,
    },
    #[error("workload catalog exceeds the {limit}-package limit with {actual} packages")]
    PackageLimit { limit: usize, actual: usize },
    #[error("unsupported workload package schema {0}")]
    UnsupportedPackageSchema(String),
    #[error("unsupported workload creation request schema {0}")]
    UnsupportedCreationRequestSchema(String),
    #[error("workload directory {0} contains neither workload.yaml nor request.yaml")]
    MissingCatalogDocument(PathBuf),
    #[error("workload id {0} must use 1-1024 ASCII letters, digits, dots, underscores, or hyphens")]
    InvalidWorkloadId(String),
    #[error("workload creation {field} must be nonempty, NUL-free, and at most {limit} bytes")]
    InvalidCreationText { field: &'static str, limit: usize },
    #[error("workload creation request for {0} is invalid or has a mismatched semantic identity")]
    InvalidCreationRequest(String),
    #[error("workload creation request id {request} does not match directory {directory}")]
    RequestDirectoryIdentity { request: String, directory: String },
    #[error("workload creation request id {request} does not match admitted package {package}")]
    RequestIdentity { request: String, package: String },
    #[error("unknown portfolio-attention row {0}")]
    UnknownAttentionRow(String),
    #[error("attention row {0} is not a ready CREATE surface row")]
    CreationAttentionRequired(String),
    #[error("attention row {0} was not selected into the current reasoning surface")]
    UnscheduledAttentionRow(String),
    #[error("workload creation attention binding {0} is invalid")]
    InvalidAttentionBinding(String),
    #[error(
        "workload package {workload} does not cite the exact retained harness request bytes as {request_source}@{revision}"
    )]
    HarnessResponseLineage {
        workload: String,
        request_source: String,
        revision: SemanticDigest,
    },
    #[error("workload {0} already has a catalog directory; refusing to overwrite it")]
    WorkloadAlreadyExists(String),
    #[error("workload package {0} is not accepted with a frozen scenario oracle")]
    NotAdmitted(String),
    #[error("workload package generation provenance is incomplete or empty")]
    InvalidGenerationProvenance,
    #[error("workload package generation must cover both compute_graph and scenario_suite")]
    IncompleteGenerationProvenance,
    #[error("workload package v1 supports only UTF-8 ports and values")]
    UnsupportedPackageValueType,
    #[error("workload package v1 does not admit operation {0}")]
    UnsupportedPackageOperation(String),
    #[error("workload package scenario cannot bind more than one probe input family")]
    MultipleScenarioProbeFamilies,
    #[error("duplicate workspace workload id {0}")]
    DuplicateWorkload(String),
    #[error(
        "workload {0} is awaiting coding harness hydration and is not admitted for test or run"
    )]
    WorkloadAwaitingHarness(String),
    #[error("unknown workload {id} in {catalog:?} catalog")]
    UnknownWorkload {
        id: String,
        catalog: WorkloadCatalogKind,
    },
    #[error("workload package YAML is invalid: {0}")]
    Yaml(#[from] serde_saphyr::Error),
    #[error("workload creation request encoding failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Workload(#[from] rey_runtime::WorkloadError),
    #[error(transparent)]
    Portfolio(#[from] rey_runtime::PortfolioError),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LocalWorkloadRecord {
    pub last_test: Option<WorkloadTestResult>,
    pub last_run: Option<WorkloadRunResult>,
}

impl LocalWorkloadRecord {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            last_test: None,
            last_run: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalWorkloadState {
    pub schema: String,
    pub commits: Vec<WorkloadCommit>,
    pub index: Option<WorkloadAdmissionSnapshot>,
    #[serde(default)]
    pub qualified_index: Option<SemanticDigest>,
    pub records: BTreeMap<String, LocalWorkloadRecord>,
    #[serde(default)]
    pub semantic_atlas_history: Vec<SemanticAtlas>,
    #[serde(default)]
    pub semantic_atlas_deltas: Vec<SemanticAtlasDelta>,
    #[serde(default)]
    pub activation_admissions: Vec<WorkloadActivationAdmission>,
    #[serde(default)]
    pub activation_executions: Vec<WorkloadActivationExecution>,
    #[serde(default)]
    pub activation_recomputations: Vec<WorkloadActivationRecomputation>,
}

impl Default for LocalWorkloadState {
    fn default() -> Self {
        Self {
            schema: LOCAL_WORKLOAD_STATE_SCHEMA.to_owned(),
            commits: Vec::new(),
            index: None,
            qualified_index: None,
            records: BTreeMap::new(),
            semantic_atlas_history: Vec::new(),
            semantic_atlas_deltas: Vec::new(),
            activation_admissions: Vec::new(),
            activation_executions: Vec::new(),
            activation_recomputations: Vec::new(),
        }
    }
}

impl LocalWorkloadState {
    pub fn verify(&self) -> Result<(), LocalWorkloadStateError> {
        if self.schema != LOCAL_WORKLOAD_STATE_SCHEMA {
            return Err(LocalWorkloadStateError::UnsupportedSchema {
                actual: self.schema.clone(),
            });
        }
        if self.records.len() > MAX_STATE_RECORDS {
            return Err(LocalWorkloadStateError::RecordLimit {
                limit: MAX_STATE_RECORDS,
            });
        }
        if self.semantic_atlas_history.len() > MAX_RETAINED_SEMANTIC_ATLASES
            || self.semantic_atlas_deltas.len() != self.semantic_atlas_history.len()
        {
            return Err(LocalWorkloadStateError::SemanticAtlasHistory);
        }
        let mut atlas_revisions = BTreeSet::new();
        for (index, atlas) in self.semantic_atlas_history.iter().enumerate() {
            atlas.verify()?;
            if !atlas_revisions.insert(atlas.atlas_revision.clone()) {
                return Err(LocalWorkloadStateError::SemanticAtlasHistory);
            }
            self.semantic_atlas_deltas[index].verify_between(
                index
                    .checked_sub(1)
                    .and_then(|prior| self.semantic_atlas_history.get(prior)),
                atlas,
            )?;
        }
        if self.activation_admissions.len() > MAX_WORKLOAD_ACTIVATION_ADMISSIONS {
            return Err(LocalWorkloadStateError::ActivationAdmissionLimit(
                MAX_WORKLOAD_ACTIVATION_ADMISSIONS,
            ));
        }
        if self.activation_executions.len() > MAX_WORKLOAD_ACTIVATION_EXECUTIONS {
            return Err(LocalWorkloadStateError::ActivationExecutionLimit(
                MAX_WORKLOAD_ACTIVATION_EXECUTIONS,
            ));
        }
        if self.activation_recomputations.len() > MAX_WORKLOAD_ACTIVATION_RECOMPUTATIONS {
            return Err(LocalWorkloadStateError::ActivationRecomputationLimit(
                MAX_WORKLOAD_ACTIVATION_RECOMPUTATIONS,
            ));
        }
        verify_workload_commit_history(&self.commits)?;
        if let Some(index) = &self.index {
            index.verify()?;
        }
        if self.qualified_index.is_some()
            && self.qualified_index.as_ref()
                != self.index.as_ref().map(|index| &index.snapshot_revision)
        {
            return Err(LocalWorkloadStateError::QualifiedIndexMismatch);
        }
        for (workload_id, record) in &self.records {
            if workload_id.is_empty() {
                return Err(LocalWorkloadStateError::EmptyWorkloadId);
            }
            if record.last_test.is_none() && record.last_run.is_none() {
                return Err(LocalWorkloadStateError::EmptyRecord(workload_id.clone()));
            }
            if let Some(result) = &record.last_test {
                result.verify()?;
                if result.workload.id != *workload_id {
                    return Err(LocalWorkloadStateError::RecordIdentity {
                        key: workload_id.clone(),
                        artifact: result.workload.id.clone(),
                    });
                }
            }
            if let Some(result) = &record.last_run {
                result.verify()?;
                if result.workload.id != *workload_id {
                    return Err(LocalWorkloadStateError::RecordIdentity {
                        key: workload_id.clone(),
                        artifact: result.workload.id.clone(),
                    });
                }
                for admission in &result.scene_admissions {
                    let Some(scene) = &admission.scene else {
                        continue;
                    };
                    let atlas_revision = scene
                        .artifacts
                        .admitted_atlas_revision
                        .as_ref()
                        .ok_or_else(|| {
                            LocalWorkloadStateError::RegionalSceneAtlasBinding(workload_id.clone())
                        })?;
                    let atlas = self
                        .semantic_atlas_history
                        .iter()
                        .find(|atlas| &atlas.atlas_revision == atlas_revision)
                        .ok_or_else(|| {
                            LocalWorkloadStateError::RegionalSceneAtlasBinding(workload_id.clone())
                        })?;
                    atlas.verify_regional_scene_binding(workload_id, scene)?;
                }
            }
        }
        let mut previous_admission = None;
        let mut activation_ids = BTreeSet::new();
        for admission in &self.activation_admissions {
            admission.verify()?;
            let commit = self
                .commits
                .iter()
                .find(|commit| commit.commit_id == admission.workload_head_commit_id)
                .ok_or(LocalWorkloadStateError::InvalidActivationAdmission)?;
            let package = commit
                .snapshot
                .packages
                .iter()
                .find(|package| package.workload == admission.workload)
                .ok_or(LocalWorkloadStateError::InvalidActivationAdmission)?;
            if previous_admission
                .is_some_and(|previous| previous >= admission.admission_id.as_str())
                || !activation_ids.insert(admission.activation.activation_id.clone())
                || commit.snapshot.snapshot_revision != admission.workload_head_snapshot_id
                || package.graph != admission.graph
                || package.scenario_suite != admission.scenario_suite
            {
                return Err(LocalWorkloadStateError::InvalidActivationAdmission);
            }
            previous_admission = Some(admission.admission_id.as_str());
        }
        let mut previous_execution = None;
        let mut admission_ids = BTreeSet::new();
        for execution in &self.activation_executions {
            let admission = self
                .activation_admissions
                .iter()
                .find(|admission| admission.admission_id == execution.admission_id)
                .ok_or(LocalWorkloadStateError::InvalidActivationExecution)?;
            execution.verify_for(admission)?;
            if let Some(source_execution_id) = &execution.source_execution_id {
                let source = self
                    .activation_executions
                    .iter()
                    .find(|source| &source.execution_id == source_execution_id)
                    .ok_or(LocalWorkloadStateError::InvalidActivationExecution)?;
                let source_admission = self
                    .activation_admissions
                    .iter()
                    .find(|admission| admission.admission_id == source.admission_id)
                    .ok_or(LocalWorkloadStateError::InvalidActivationExecution)?;
                if WorkloadActivationExecution::coalesce(admission, source_admission, source)?
                    .as_ref()
                    != Some(execution)
                {
                    return Err(LocalWorkloadStateError::InvalidActivationExecution);
                }
            }
            if previous_execution
                .is_some_and(|previous| previous >= execution.execution_id.as_str())
                || !admission_ids.insert(execution.admission_id.clone())
            {
                return Err(LocalWorkloadStateError::InvalidActivationExecution);
            }
            previous_execution = Some(execution.execution_id.as_str());
        }
        let mut previous_recomputation = None;
        let mut recomputed_execution_ids = BTreeSet::new();
        for recomputation in &self.activation_recomputations {
            let execution = self
                .activation_executions
                .iter()
                .find(|execution| execution.execution_id == recomputation.execution_id)
                .ok_or(LocalWorkloadStateError::InvalidActivationRecomputation)?;
            let admission = self
                .activation_admissions
                .iter()
                .find(|admission| admission.admission_id == recomputation.admission_id)
                .ok_or(LocalWorkloadStateError::InvalidActivationRecomputation)?;
            recomputation.verify_for(execution, admission)?;
            if previous_recomputation
                .is_some_and(|previous| previous >= recomputation.recomputation_id.as_str())
                || !recomputed_execution_ids.insert(recomputation.execution_id.clone())
            {
                return Err(LocalWorkloadStateError::InvalidActivationRecomputation);
            }
            previous_recomputation = Some(recomputation.recomputation_id.as_str());
        }
        Ok(())
    }

    #[must_use]
    pub fn record(&self, workload_id: &str) -> Option<&LocalWorkloadRecord> {
        self.records.get(workload_id)
    }

    pub fn retain_test(&mut self, result: WorkloadTestResult) {
        let workload_id = result.workload.id.clone();
        self.records
            .entry(workload_id)
            .or_insert_with(LocalWorkloadRecord::empty)
            .last_test = Some(result);
    }

    pub fn retain_run(&mut self, result: WorkloadRunResult) {
        let workload_id = result.workload.id.clone();
        self.records
            .entry(workload_id)
            .or_insert_with(LocalWorkloadRecord::empty)
            .last_run = Some(result);
    }

    pub fn retain_semantic_atlas_transition(
        &mut self,
        source: Option<SemanticAtlas>,
        target: Option<SemanticAtlas>,
    ) -> Result<(), LocalWorkloadStateError> {
        let Some(target) = target else { return Ok(()) };
        if source
            .as_ref()
            .is_some_and(|source| source.atlas_revision == target.atlas_revision)
        {
            return Ok(());
        }
        if self.semantic_atlas_history.is_empty()
            && let Some(source) = source
        {
            self.push_semantic_atlas(source)?;
        }
        if self
            .semantic_atlas_history
            .last()
            .is_some_and(|atlas| atlas.atlas_revision == target.atlas_revision)
        {
            return Ok(());
        }
        self.push_semantic_atlas(target)
    }

    fn push_semantic_atlas(&mut self, atlas: SemanticAtlas) -> Result<(), LocalWorkloadStateError> {
        if self.semantic_atlas_history.len() >= MAX_RETAINED_SEMANTIC_ATLASES {
            return Err(LocalWorkloadStateError::SemanticAtlasHistory);
        }
        let delta = atlas.delta_from(self.semantic_atlas_history.last())?;
        self.semantic_atlas_history.push(atlas);
        self.semantic_atlas_deltas.push(delta);
        Ok(())
    }

    pub fn refresh_index_qualification(&mut self, definitions: &[WorkloadDefinition]) {
        self.qualified_index = self.index.as_ref().and_then(|index| {
            let exact_catalog = index.packages.len() == definitions.len()
                && index.packages.iter().all(|package| {
                    definitions
                        .iter()
                        .any(|definition| definition.workload.id == package.workload_id)
                });
            let all_qualified = exact_catalog
                && definitions.iter().all(|definition| {
                    fresh_qualification(definition, self.record(&definition.workload.id)).is_some()
                });
            all_qualified.then(|| index.snapshot_revision.clone())
        });
    }
}

#[derive(Clone, Debug)]
pub struct LocalWorkloadStore {
    directory: PathBuf,
}

impl LocalWorkloadStore {
    #[must_use]
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    #[must_use]
    pub fn default_for_workspace(workspace: &Path) -> Self {
        Self::new(workspace.join(".rey").join("workloads"))
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.directory.join(STATE_FILE_NAME)
    }

    fn observe(
        &self,
        workspace: &Path,
        catalog_dir: &Path,
    ) -> Result<ObservedWorkloadAdmission, LocalWorkloadStateError> {
        let mut catalog = WorkloadCatalog::load_workspace(workspace, catalog_dir)?;
        let ignore = ReyIgnoreFile::load(workspace)?;
        let candidate_names = catalog
            .workloads
            .iter()
            .map(|workload| workload.definition.workload.id.clone())
            .chain(
                catalog
                    .drafts
                    .iter()
                    .map(|draft| draft.request.workload_id.clone()),
            )
            .collect::<Vec<_>>();
        let candidates = candidate_names
            .iter()
            .map(|name| ("workload", name.as_str()))
            .collect::<Vec<_>>();
        let ignored = ignore.as_ref().and_then(|ignore| {
            let projection = ignore.project(&candidates, &["workload"]);
            if projection.rules.is_empty() {
                return None;
            }
            catalog
                .workloads
                .retain(|workload| !ignore.matches("workload", &workload.definition.workload.id));
            catalog
                .drafts
                .retain(|draft| !ignore.matches("workload", &draft.request.workload_id));
            catalog.descriptor.workload_count =
                catalog.workloads.len().saturating_add(catalog.drafts.len()) as u64;
            catalog.descriptor.draft_count = catalog.drafts.len() as u64;
            Some(projection)
        });
        let mut artifacts = BTreeMap::new();
        let mut packages = Vec::with_capacity(catalog.workloads.len());
        for workload in &catalog.workloads {
            let source = &workload.provenance.source;
            let source_path = Path::new(source);
            if source_path.as_os_str().is_empty()
                || source_path
                    .components()
                    .any(|component| !matches!(component, Component::Normal(_)))
            {
                return Err(LocalWorkloadStateError::ArtifactSource(source.clone()));
            }
            let bytes = read_bounded_catalog_file(&workspace.join(source_path))?;
            let source_digest = source_digest("rey.workload-package-source.v1", &bytes);
            if workload.provenance.source_digest.as_ref() != Some(&source_digest) {
                return Err(LocalWorkloadStateError::ArtifactIdentity(
                    workload.definition.workload.id.clone(),
                ));
            }
            let object_path = format!("objects/{}.yaml", workload_digest_key(&source_digest));
            packages.push(WorkloadPackageSnapshot {
                workload_id: workload.definition.workload.id.clone(),
                workload_revision: workload.definition.workload.revision,
                title: workload.definition.title.clone(),
                source: source.clone(),
                source_digest: source_digest.clone(),
                object_path: object_path.clone(),
                bytes: bytes.len() as u64,
                generation: workload.provenance.generation.clone().ok_or_else(|| {
                    LocalWorkloadStateError::ArtifactIdentity(
                        workload.definition.workload.id.clone(),
                    )
                })?,
                workload: workload.definition.workload.clone(),
                graph: workload.definition.graph.graph.clone(),
                scenario_suite: workload.definition.scenario_suite.suite.clone(),
            });
            artifacts.insert(object_path, bytes);
        }
        Ok(ObservedWorkloadAdmission {
            catalog,
            snapshot: WorkloadAdmissionSnapshot::new(packages, ignored.clone())?,
            artifacts,
        })
    }

    pub fn status(
        &self,
        workspace: &Path,
        catalog_dir: &Path,
    ) -> Result<WorkloadRevisionStatus, LocalWorkloadStateError> {
        let observed = self.observe(workspace, catalog_dir)?;
        let state = self.load()?;
        let head = state.commits.last().cloned();
        let head_snapshot = head.as_ref().map(|commit| &commit.snapshot);
        let staged = WorkloadChangeSet::derive(
            "HEAD",
            head_snapshot,
            "INDEX",
            state.index.as_ref().or(head_snapshot),
        );
        let unstaged = WorkloadChangeSet::derive(
            "INDEX",
            state.index.as_ref().or(head_snapshot),
            "WORKING",
            Some(&observed.snapshot),
        );
        let state_kind = match (
            staged.assessment == DeltaAssessment::Different,
            unstaged.assessment == DeltaAssessment::Different,
        ) {
            (false, false) => WorkloadWorkingState::Clean,
            (false, true) => WorkloadWorkingState::Working,
            (true, false) => WorkloadWorkingState::Staged,
            (true, true) => WorkloadWorkingState::Mixed,
        };
        let mut qualification_omissions = state.index.as_ref().map_or_else(Vec::new, |snapshot| {
            match self.catalog_from_snapshot(snapshot, WorkloadAdmissionState::Proposed) {
                Ok(catalog) => catalog
                    .workloads
                    .iter()
                    .filter(|workload| {
                        fresh_qualification(
                            &workload.definition,
                            state.record(&workload.definition.workload.id),
                        )
                        .is_none()
                    })
                    .map(|workload| {
                        format!(
                            "{} lacks fresh passing qualification for the exact staged package",
                            workload.definition.workload.id
                        )
                    })
                    .collect(),
                Err(error) => vec![error.to_string()],
            }
        });
        if let Some(index) = &state.index
            && state.qualified_index.as_ref() != Some(&index.snapshot_revision)
        {
            qualification_omissions.push(
                "the complete scenario suite has not qualified this exact INDEX snapshot"
                    .to_owned(),
            );
        }
        let commit_ready = state.index.is_some()
            && staged.assessment == DeltaAssessment::Different
            && qualification_omissions.is_empty();
        Ok(WorkloadRevisionStatus {
            schema: WORKLOAD_REVISION_STATUS_SCHEMA.to_owned(),
            state: state_kind,
            head,
            index: state.index,
            working: observed.snapshot,
            staged,
            unstaged,
            drafts: observed.catalog.drafts,
            commit_ready,
            qualification_omissions,
            admission_boundary: "only a human admission advances HEAD; the browser freezes and qualifies an exact reviewed WORKING file snapshot, while CLI commit reads only an already-qualified INDEX".to_owned(),
        })
    }

    pub fn diff(
        &self,
        workspace: &Path,
        catalog_dir: &Path,
        staged: bool,
    ) -> Result<WorkloadChangeSet, LocalWorkloadStateError> {
        let status = self.status(workspace, catalog_dir)?;
        Ok(if staged {
            status.staged
        } else {
            status.unstaged
        })
    }

    pub fn add(
        &self,
        workspace: &Path,
        catalog_dir: &Path,
    ) -> Result<WorkloadAddResult, LocalWorkloadStateError> {
        self.add_expected(workspace, catalog_dir, None)
    }

    pub fn add_expected(
        &self,
        workspace: &Path,
        catalog_dir: &Path,
        expected_working: Option<&str>,
    ) -> Result<WorkloadAddResult, LocalWorkloadStateError> {
        self.with_lock(|| {
            let observed = self.observe(workspace, catalog_dir)?;
            if expected_working
                .is_some_and(|expected| expected != observed.snapshot.snapshot_revision.as_str())
            {
                return Err(LocalWorkloadStateError::ApprovalPrecondition(
                    "WORKING file snapshot changed before admission".to_owned(),
                ));
            }
            let mut state = self.load()?;
            let head_snapshot = state.commits.last().map(|commit| &commit.snapshot);
            let delta =
                WorkloadChangeSet::derive("HEAD", head_snapshot, "INDEX", Some(&observed.snapshot));
            self.write_artifacts(&observed.artifacts)?;
            let staged = head_snapshot != Some(&observed.snapshot);
            let preserves_qualification = state.index.as_ref() == Some(&observed.snapshot);
            state.index = staged.then_some(observed.snapshot.clone());
            if !preserves_qualification || state.index.is_none() {
                state.qualified_index = None;
            }
            self.save(&state)?;
            Ok(WorkloadAddResult {
                schema: WORKLOAD_ADD_RESULT_SCHEMA.to_owned(),
                staged,
                snapshot: observed.snapshot,
                delta,
            })
        })
    }

    pub fn head_catalog(&self) -> Result<WorkloadCatalog, LocalWorkloadStateError> {
        let state = self.load()?;
        match state.commits.last() {
            Some(commit) => {
                self.catalog_from_snapshot(&commit.snapshot, WorkloadAdmissionState::Accepted)
            }
            None => Ok(empty_workspace_catalog("WORKLOAD HEAD")),
        }
    }

    pub fn index_catalog(&self) -> Result<WorkloadCatalog, LocalWorkloadStateError> {
        let state = self.load()?;
        let snapshot = state
            .index
            .as_ref()
            .ok_or(LocalWorkloadStateError::EmptyIndex)?;
        self.catalog_from_snapshot(snapshot, WorkloadAdmissionState::Proposed)
    }

    pub fn admit_activation(
        &self,
        admission: WorkloadActivationAdmission,
    ) -> Result<WorkloadActivationAdmission, LocalWorkloadStateError> {
        admission.verify()?;
        self.with_lock(|| {
            let mut state = self.load()?;
            if let Some(existing) = state.activation_admissions.iter().find(|existing| {
                existing.activation.activation_id == admission.activation.activation_id
            }) {
                if existing == &admission {
                    return Ok(existing.clone());
                }
                return Err(LocalWorkloadStateError::ActivationAlreadyAdmitted {
                    activation_id: admission.activation.activation_id.clone(),
                    admission_id: existing.admission_id.clone(),
                });
            }
            if state.activation_admissions.len() >= MAX_WORKLOAD_ACTIVATION_ADMISSIONS {
                return Err(LocalWorkloadStateError::ActivationAdmissionLimit(
                    MAX_WORKLOAD_ACTIVATION_ADMISSIONS,
                ));
            }
            state.activation_admissions.push(admission.clone());
            state
                .activation_admissions
                .sort_by(|left, right| left.admission_id.cmp(&right.admission_id));
            self.save(&state)?;
            Ok(admission)
        })
    }

    pub fn retain_activation_execution(
        &self,
        execution: WorkloadActivationExecution,
    ) -> Result<WorkloadActivationExecution, LocalWorkloadStateError> {
        self.with_lock(|| {
            let mut state = self.load()?;
            let admission = state
                .activation_admissions
                .iter()
                .find(|admission| admission.admission_id == execution.admission_id)
                .ok_or(LocalWorkloadStateError::UnknownActivationAdmission(
                    execution.admission_id.clone(),
                ))?;
            execution.verify_for(admission)?;
            if state.commits.last().is_none_or(|head| {
                head.commit_id != admission.workload_head_commit_id
                    || head.snapshot.snapshot_revision != admission.workload_head_snapshot_id
            }) {
                return Err(LocalWorkloadStateError::ActivationPrecondition(
                    "workload HEAD changed before activation execution retention".to_owned(),
                ));
            }
            if let Some(existing) = state
                .activation_executions
                .iter()
                .find(|existing| existing.admission_id == execution.admission_id)
            {
                if existing == &execution {
                    return Ok(existing.clone());
                }
                return Err(LocalWorkloadStateError::ActivationAlreadyExecuted {
                    admission_id: execution.admission_id.clone(),
                    execution_id: existing.execution_id.clone(),
                });
            }
            if state.activation_executions.len() >= MAX_WORKLOAD_ACTIVATION_EXECUTIONS {
                return Err(LocalWorkloadStateError::ActivationExecutionLimit(
                    MAX_WORKLOAD_ACTIVATION_EXECUTIONS,
                ));
            }
            state.activation_executions.push(execution.clone());
            state
                .activation_executions
                .sort_by(|left, right| left.execution_id.cmp(&right.execution_id));
            self.save(&state)?;
            Ok(execution)
        })
    }

    pub fn retain_activation_recomputation(
        &self,
        recomputation: WorkloadActivationRecomputation,
    ) -> Result<WorkloadActivationRecomputation, LocalWorkloadStateError> {
        self.with_lock(|| {
            let mut state = self.load()?;
            let execution = state
                .activation_executions
                .iter()
                .find(|execution| execution.execution_id == recomputation.execution_id)
                .ok_or_else(|| {
                    LocalWorkloadStateError::UnknownActivationExecution(
                        recomputation.execution_id.clone(),
                    )
                })?;
            let admission = state
                .activation_admissions
                .iter()
                .find(|admission| admission.admission_id == recomputation.admission_id)
                .ok_or_else(|| {
                    LocalWorkloadStateError::UnknownActivationAdmission(
                        recomputation.admission_id.clone(),
                    )
                })?;
            recomputation.verify_for(execution, admission)?;
            if state.commits.last().is_none_or(|head| {
                head.commit_id != admission.workload_head_commit_id
                    || head.snapshot.snapshot_revision != admission.workload_head_snapshot_id
            }) {
                return Err(LocalWorkloadStateError::ActivationPrecondition(
                    "workload HEAD changed before full recomputation retention".to_owned(),
                ));
            }
            if let Some(existing) = state
                .activation_recomputations
                .iter()
                .find(|existing| existing.execution_id == recomputation.execution_id)
            {
                if existing == &recomputation {
                    return Ok(existing.clone());
                }
                return Err(LocalWorkloadStateError::ActivationAlreadyRecomputed {
                    execution_id: recomputation.execution_id.clone(),
                    recomputation_id: existing.recomputation_id.clone(),
                });
            }
            if state.activation_recomputations.len() >= MAX_WORKLOAD_ACTIVATION_RECOMPUTATIONS {
                return Err(LocalWorkloadStateError::ActivationRecomputationLimit(
                    MAX_WORKLOAD_ACTIVATION_RECOMPUTATIONS,
                ));
            }
            state.activation_recomputations.push(recomputation.clone());
            state
                .activation_recomputations
                .sort_by(|left, right| left.recomputation_id.cmp(&right.recomputation_id));
            self.save(&state)?;
            Ok(recomputation)
        })
    }

    pub fn retain_index_tests(
        &self,
        expected_index: &SemanticDigest,
        definitions: &[WorkloadDefinition],
        results: Vec<WorkloadTestResult>,
    ) -> Result<(), LocalWorkloadStateError> {
        self.with_lock(|| {
            let mut state = self.load()?;
            if state.index.as_ref().map(|index| &index.snapshot_revision) != Some(expected_index) {
                return Err(LocalWorkloadStateError::ApprovalPrecondition(
                    "INDEX changed while the reviewed file snapshot was qualifying".to_owned(),
                ));
            }
            for result in results {
                state.retain_test(result);
            }
            state.refresh_index_qualification(definitions);
            state.verify()?;
            self.save(&state)
        })
    }

    pub fn commit(
        &self,
        message: String,
        expected_head: Option<&str>,
        expected_index: Option<&str>,
    ) -> Result<WorkloadCommitResult, LocalWorkloadStateError> {
        let message = normalize_workload_commit_message(message)?;
        self.with_lock(|| {
            let mut state = self.load()?;
            let snapshot = state
                .index
                .clone()
                .ok_or(LocalWorkloadStateError::EmptyIndex)?;
            if state.qualified_index.as_ref() != Some(&snapshot.snapshot_revision) {
                return Err(LocalWorkloadStateError::ExactIndexQualificationRequired(
                    snapshot.snapshot_revision.clone(),
                ));
            }
            snapshot.verify()?;
            let current_head = state.commits.last();
            if let Some(expected) = expected_head {
                let matches = if expected == "EMPTY" {
                    current_head.is_none()
                } else {
                    current_head.map(|commit| commit.commit_id.as_str()) == Some(expected)
                };
                if !matches {
                    return Err(LocalWorkloadStateError::ApprovalPrecondition(
                        "HEAD changed before approval".to_owned(),
                    ));
                }
            }
            if expected_index
                .is_some_and(|expected| expected != snapshot.snapshot_revision.as_str())
            {
                return Err(LocalWorkloadStateError::ApprovalPrecondition(
                    "INDEX changed before approval".to_owned(),
                ));
            }
            self.verify_staged_artifacts(&snapshot)?;
            let catalog =
                self.catalog_from_snapshot(&snapshot, WorkloadAdmissionState::Proposed)?;
            let mut qualification_ids = Vec::with_capacity(catalog.workloads.len());
            for workload in &catalog.workloads {
                let qualification = fresh_qualification(
                    &workload.definition,
                    state.record(&workload.definition.workload.id),
                )
                .ok_or_else(|| {
                    LocalWorkloadStateError::QualificationRequired(
                        workload.definition.workload.id.clone(),
                    )
                })?;
                qualification_ids.push(qualification.qualification_id.clone());
            }
            let head_snapshot = current_head.map(|commit| &commit.snapshot);
            if head_snapshot == Some(&snapshot) {
                return Err(LocalWorkloadStateError::NothingToCommit);
            }
            if state.commits.len() >= MAX_WORKLOAD_COMMITS {
                return Err(LocalWorkloadStateError::CommitLimit(MAX_WORKLOAD_COMMITS));
            }
            let sequence = state.commits.len() as u64 + 1;
            let delta = WorkloadChangeSet::derive(
                if sequence == 1 { "EMPTY" } else { "HEAD" },
                head_snapshot,
                &format!("WORKLOAD@{sequence}"),
                Some(&snapshot),
            );
            let commit = WorkloadCommit::new(
                sequence,
                current_head.map(|commit| commit.commit_id.clone()),
                message,
                snapshot,
                qualification_ids,
            )?;
            state.commits.push(commit.clone());
            state.index = None;
            state.qualified_index = None;
            self.save(&state)?;
            Ok(WorkloadCommitResult {
                schema: WORKLOAD_COMMIT_RESULT_SCHEMA.to_owned(),
                commit,
                delta,
            })
        })
    }

    pub fn log(
        &self,
        max_count: usize,
        patch: bool,
    ) -> Result<WorkloadLog, LocalWorkloadStateError> {
        if max_count == 0 || max_count > MAX_WORKLOAD_COMMITS {
            return Err(LocalWorkloadStateError::LogLimit {
                limit: MAX_WORKLOAD_COMMITS,
                actual: max_count,
            });
        }
        let state = self.load()?;
        let commits = state
            .commits
            .iter()
            .rev()
            .take(max_count)
            .cloned()
            .collect::<Vec<_>>();
        Ok(WorkloadLog {
            schema: WORKLOAD_LOG_SCHEMA.to_owned(),
            head_commit_id: state.commits.last().map(|commit| commit.commit_id.clone()),
            total_commits: state.commits.len() as u64,
            selected_commits: commits.len() as u64,
            patch,
            commits,
        })
    }

    fn catalog_from_snapshot(
        &self,
        snapshot: &WorkloadAdmissionSnapshot,
        admission_state: WorkloadAdmissionState,
    ) -> Result<WorkloadCatalog, LocalWorkloadStateError> {
        snapshot.verify()?;
        let mut workloads = Vec::with_capacity(snapshot.packages.len());
        for package in &snapshot.packages {
            let path = self.safe_object_path(&package.object_path)?;
            let bytes = read_local_workload_file(&path, MAX_WORKLOAD_PACKAGE_BYTES)?;
            if source_digest("rey.workload-package-source.v1", &bytes) != package.source_digest {
                return Err(LocalWorkloadStateError::ArtifactIdentity(
                    package.workload_id.clone(),
                ));
            }
            let supplied: SuppliedWorkloadPackage = serde_saphyr::from_slice(&bytes)?;
            let resolved = supplied.resolve(package.source.clone(), &bytes, admission_state)?;
            if resolved.definition.workload != package.workload
                || resolved.definition.graph.graph != package.graph
                || resolved.definition.scenario_suite.suite != package.scenario_suite
                || resolved.definition.title != package.title
            {
                return Err(LocalWorkloadStateError::ArtifactIdentity(
                    package.workload_id.clone(),
                ));
            }
            workloads.push(resolved);
        }
        Ok(WorkloadCatalog {
            descriptor: WorkloadCatalogDescriptor {
                schema: WORKLOAD_CATALOG_SCHEMA.to_owned(),
                kind: WorkloadCatalogKind::WorkspacePackages,
                root: Some(
                    match admission_state {
                        WorkloadAdmissionState::Accepted => "WORKLOAD HEAD",
                        WorkloadAdmissionState::Proposed => "WORKLOAD INDEX",
                        WorkloadAdmissionState::Rejected => "REJECTED WORKLOAD SNAPSHOT",
                    }
                    .to_owned(),
                ),
                workload_count: workloads.len() as u64,
                admitted_count: if admission_state == WorkloadAdmissionState::Accepted {
                    workloads.len() as u64
                } else {
                    0
                },
                draft_count: 0,
            },
            workloads,
            drafts: Vec::new(),
        })
    }

    fn write_artifacts(
        &self,
        artifacts: &BTreeMap<String, Vec<u8>>,
    ) -> Result<(), LocalWorkloadStateError> {
        self.prepare_directory()?;
        for (relative, bytes) in artifacts {
            let path = self.safe_object_path(relative)?;
            write_local_content_addressed(&path, bytes)?;
        }
        Ok(())
    }

    fn verify_staged_artifacts(
        &self,
        snapshot: &WorkloadAdmissionSnapshot,
    ) -> Result<(), LocalWorkloadStateError> {
        for package in &snapshot.packages {
            let path = self.safe_object_path(&package.object_path)?;
            let bytes = read_local_workload_file(&path, MAX_WORKLOAD_PACKAGE_BYTES)?;
            if source_digest("rey.workload-package-source.v1", &bytes) != package.source_digest {
                return Err(LocalWorkloadStateError::ArtifactIdentity(
                    package.workload_id.clone(),
                ));
            }
        }
        Ok(())
    }

    fn safe_object_path(&self, relative: &str) -> Result<PathBuf, LocalWorkloadStateError> {
        let components = Path::new(relative).components().collect::<Vec<_>>();
        if components.len() != 2
            || components.first() != Some(&Component::Normal("objects".as_ref()))
            || !matches!(components.get(1), Some(Component::Normal(_)))
        {
            return Err(LocalWorkloadStateError::ArtifactSource(relative.to_owned()));
        }
        Ok(self.directory.join(relative))
    }

    fn with_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, LocalWorkloadStateError>,
    ) -> Result<T, LocalWorkloadStateError> {
        self.prepare_directory()?;
        let lock_path = self.directory.join(LOCK_FILE_NAME);
        if let Ok(metadata) = fs::symlink_metadata(&lock_path)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(LocalWorkloadStateError::UnsafeStatePath(lock_path));
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| LocalWorkloadStateError::Write {
                path: lock_path.clone(),
                source,
            })?;
        File::lock(&lock).map_err(|source| LocalWorkloadStateError::Lock {
            path: lock_path.clone(),
            source,
        })?;
        let result = operation();
        let unlock = File::unlock(&lock).map_err(|source| LocalWorkloadStateError::Lock {
            path: lock_path,
            source,
        });
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    pub fn load(&self) -> Result<LocalWorkloadState, LocalWorkloadStateError> {
        let path = self.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(LocalWorkloadState::default());
            }
            Err(source) => return Err(LocalWorkloadStateError::Read { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(LocalWorkloadStateError::UnsafeStatePath(path));
        }
        if metadata.len() > MAX_STATE_BYTES {
            return Err(LocalWorkloadStateError::ByteLimit {
                path,
                limit: MAX_STATE_BYTES,
            });
        }
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|source| LocalWorkloadStateError::Read {
                path: path.clone(),
                source,
            })?
            .take(MAX_STATE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| LocalWorkloadStateError::Read {
                path: path.clone(),
                source,
            })?;
        if bytes.len() as u64 > MAX_STATE_BYTES {
            return Err(LocalWorkloadStateError::ByteLimit {
                path,
                limit: MAX_STATE_BYTES,
            });
        }
        let state: LocalWorkloadState =
            serde_json::from_slice(&bytes).map_err(|source| LocalWorkloadStateError::Json {
                path: path.clone(),
                source,
            })?;
        state.verify()?;
        Ok(state)
    }

    pub fn save(&self, state: &LocalWorkloadState) -> Result<(), LocalWorkloadStateError> {
        state.verify()?;
        let bytes =
            serde_json::to_vec_pretty(state).map_err(|source| LocalWorkloadStateError::Json {
                path: self.path(),
                source,
            })?;
        if bytes.len().saturating_add(1) as u64 > MAX_STATE_BYTES {
            return Err(LocalWorkloadStateError::ByteLimit {
                path: self.path(),
                limit: MAX_STATE_BYTES,
            });
        }
        self.prepare_directory()?;
        let target = self.path();
        if let Ok(metadata) = fs::symlink_metadata(&target)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(LocalWorkloadStateError::UnsafeStatePath(target));
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
            return Err(LocalWorkloadStateError::Write {
                path: target,
                source,
            });
        }
        Ok(())
    }

    fn prepare_directory(&self) -> Result<(), LocalWorkloadStateError> {
        match fs::symlink_metadata(&self.directory) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => Err(
                LocalWorkloadStateError::UnsafeStatePath(self.directory.clone()),
            ),
            Ok(_) => self.prepare_child_directory("objects"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.directory).map_err(|source| {
                    LocalWorkloadStateError::Write {
                        path: self.directory.clone(),
                        source,
                    }
                })?;
                self.prepare_child_directory("objects")
            }
            Err(source) => Err(LocalWorkloadStateError::Write {
                path: self.directory.clone(),
                source,
            }),
        }
    }

    fn prepare_child_directory(&self, child: &str) -> Result<(), LocalWorkloadStateError> {
        let path = self.directory.join(child);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                Err(LocalWorkloadStateError::UnsafeStatePath(path))
            }
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::create_dir(&path)
                .map_err(|source| LocalWorkloadStateError::Write { path, source }),
            Err(source) => Err(LocalWorkloadStateError::Write { path, source }),
        }
    }

    fn create_temporary(&self) -> Result<(PathBuf, File), LocalWorkloadStateError> {
        for attempt in 0..32_u8 {
            let path = self
                .directory
                .join(format!(".state.json.tmp-{}-{attempt}", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(LocalWorkloadStateError::Write { path, source }),
            }
        }
        Err(LocalWorkloadStateError::TemporaryLimit(
            self.directory.clone(),
        ))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadFreshness {
    Untested,
    Fresh,
    Stale,
}

impl WorkloadFreshness {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Untested => "untested",
            Self::Fresh => "fresh",
            Self::Stale => "stale",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationState {
    Untested,
    Qualified,
    Failing,
    Inconclusive,
    Stale,
}

impl QualificationState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Untested => "untested",
            Self::Qualified => "qualified",
            Self::Failing => "failing",
            Self::Inconclusive => "inconclusive",
            Self::Stale => "stale",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadSummary {
    pub provenance: Option<WorkloadProvenance>,
    pub workload: ContractIdentity,
    pub title: String,
    pub owned_surfaces: Vec<WorkloadOwnedSurface>,
    pub git_dependencies: Vec<WorkloadGitDependency>,
    pub candidate_graph: ContractIdentity,
    pub scenario_suite: ContractIdentity,
    pub evaluator: ContractIdentity,
    pub freshness: WorkloadFreshness,
    pub qualification: QualificationState,
    pub qualified_graph: Option<ContractIdentity>,
    pub required: u64,
    pub passed: u64,
    pub failed: u64,
    pub inconclusive: u64,
    pub evaluated: u64,
    pub stale: u64,
    pub optional: u64,
    pub last_test_result_id: Option<rey_core::SemanticDigest>,
    pub last_run_status: Option<RunStatus>,
    pub operations: Vec<ContractIdentity>,
    pub mining_operations: u64,
    pub mining_results: u64,
    pub complete_mining_results: u64,
    pub incomplete_mining_results: u64,
    pub relation_deltas: u64,
    pub reasoning_surfaces: u64,
    pub attention_results: u64,
    pub attention_rows: u64,
    pub topography_results: u64,
    pub topography_revision: Option<SemanticDigest>,
    pub topography_coverage: Option<TopographyCoverage>,
    pub topography_frontier_rows: u64,
    pub topography_patch: Option<TopographyPatch>,
    pub topography_projection: Option<ProjectionPacket>,
    pub scene_admission_results: u64,
    pub latest_scene_admission: Option<SceneAdmissionResult>,
}

impl WorkloadSummary {
    #[must_use]
    pub fn derive(workload: &WorkloadDefinition, record: Option<&LocalWorkloadRecord>) -> Self {
        let required = workload.required_scenario_count();
        let optional = workload.scenario_suite.scenarios.len() as u64 - required;
        let retained_test = record.and_then(|record| record.last_test.as_ref());
        let (freshness, qualification, passed, failed, inconclusive, evaluated, stale) =
            match retained_test {
                None => (
                    WorkloadFreshness::Untested,
                    QualificationState::Untested,
                    0,
                    0,
                    0,
                    0,
                    0,
                ),
                Some(result) if result.verify_for(workload).is_err() => (
                    WorkloadFreshness::Stale,
                    QualificationState::Stale,
                    0,
                    0,
                    0,
                    0,
                    required,
                ),
                Some(result) => (
                    WorkloadFreshness::Fresh,
                    match result.status {
                        TestStatus::Passed => QualificationState::Qualified,
                        TestStatus::Failed => QualificationState::Failing,
                        TestStatus::Inconclusive => QualificationState::Inconclusive,
                    },
                    result.summary.passed,
                    result.summary.failed,
                    result.summary.inconclusive,
                    result.summary.evaluated,
                    0,
                ),
            };
        let qualified_graph = retained_test
            .filter(|result| result.verify_for(workload).is_ok())
            .and_then(|result| result.qualification.as_ref())
            .map(|qualification| qualification.graph.clone());
        let mut operations = Vec::new();
        for node in &workload.graph.nodes {
            if !operations.contains(&node.operation) {
                operations.push(node.operation.clone());
            }
        }
        let mining_operations = operations
            .iter()
            .filter(|operation| {
                operation.id.starts_with("rey.source-")
                    || operation.id == CONTEXT_ANCHOR_SURVEY_OPERATION_ID
                    || operation.id == "rey.portfolio.attention.derive"
            })
            .count() as u64;
        let mining = retained_test
            .filter(|result| result.verify_for(workload).is_ok())
            .into_iter()
            .flat_map(|result| &result.scenarios)
            .flat_map(|scenario| &scenario.mining)
            .collect::<Vec<_>>();
        let source_mining_results = mining.len() as u64;
        let complete_source_mining_results = mining
            .iter()
            .filter(|evidence| {
                evidence.execution.evidence.result.completeness
                    == rey_mining::MiningCompleteness::Complete
            })
            .count() as u64;
        let reasoning_surfaces = mining
            .iter()
            .filter(|evidence| evidence.reasoning.is_some())
            .count() as u64;
        let attention = retained_test
            .filter(|result| result.verify_for(workload).is_ok())
            .into_iter()
            .flat_map(|result| &result.scenarios)
            .flat_map(|scenario| &scenario.attention)
            .collect::<Vec<_>>();
        let attention_results = attention.len() as u64;
        let attention_rows = attention
            .iter()
            .map(|attention| attention.rows.len() as u64)
            .sum();
        let test_topography = retained_test
            .filter(|result| result.verify_for(workload).is_ok())
            .into_iter()
            .flat_map(|result| &result.scenarios)
            .flat_map(|scenario| &scenario.topography)
            .collect::<Vec<_>>();
        let run_topography = record
            .and_then(|record| record.last_run.as_ref())
            .filter(|run| run.workload == workload.workload && run.graph == workload.graph.graph)
            .into_iter()
            .flat_map(|run| &run.topography)
            .collect::<Vec<_>>();
        let topography_results = test_topography.len().saturating_add(run_topography.len()) as u64;
        let complete_topography = test_topography
            .iter()
            .chain(&run_topography)
            .filter(|patch| patch.complete)
            .count() as u64;
        let last_patch = run_topography.last().copied();
        let mining_results = source_mining_results
            .saturating_add(attention_results)
            .saturating_add(topography_results);
        let complete_mining_results = complete_source_mining_results
            .saturating_add(attention_results)
            .saturating_add(complete_topography);
        let topography_projection = last_patch.map(|patch| {
            ProjectionPacket::from_topography_patch(patch)
                .expect("retained verified topography must produce a projection packet")
        });
        let production_scene_admissions = record
            .and_then(|record| record.last_run.as_ref())
            .filter(|run| run.workload == workload.workload && run.graph == workload.graph.graph)
            .map(|run| run.scene_admissions.as_slice())
            .unwrap_or_default();
        let latest_scene_admission = production_scene_admissions.last().cloned();
        Self {
            provenance: None,
            workload: workload.workload.clone(),
            title: workload.title.clone(),
            owned_surfaces: workload.owned_surfaces.clone(),
            git_dependencies: workload.git_dependencies.clone(),
            candidate_graph: workload.graph.graph.clone(),
            scenario_suite: workload.scenario_suite.suite.clone(),
            evaluator: workload.evaluator.clone(),
            freshness,
            qualification,
            qualified_graph,
            required,
            passed,
            failed,
            inconclusive,
            evaluated,
            stale,
            optional,
            last_test_result_id: retained_test.map(|result| result.result_id.clone()),
            last_run_status: record
                .and_then(|record| record.last_run.as_ref().map(|result| result.status)),
            operations,
            mining_operations,
            mining_results,
            complete_mining_results,
            incomplete_mining_results: mining_results.saturating_sub(complete_mining_results),
            relation_deltas: source_mining_results
                .saturating_add(attention_results)
                .saturating_add(topography_results),
            reasoning_surfaces,
            attention_results,
            attention_rows,
            topography_results,
            topography_revision: last_patch.map(|patch| patch.topography_revision.clone()),
            topography_coverage: last_patch.map(|patch| patch.coverage.clone()),
            topography_frontier_rows: last_patch.map_or(0, |patch| patch.frontier.len() as u64),
            topography_patch: last_patch.cloned(),
            topography_projection,
            scene_admission_results: production_scene_admissions.len() as u64,
            latest_scene_admission,
        }
    }

    #[must_use]
    pub fn derive_resolved(
        workload: &ResolvedWorkload,
        record: Option<&LocalWorkloadRecord>,
    ) -> Self {
        let mut summary = Self::derive(&workload.definition, record);
        summary.provenance = Some(workload.provenance.clone());
        summary
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadList {
    pub schema: String,
    pub catalog: WorkloadCatalogDescriptor,
    pub workloads: Vec<WorkloadSummary>,
    pub drafts: Vec<WorkloadDraft>,
    #[serde(default)]
    pub activation_admissions: Vec<WorkloadActivationAdmission>,
    #[serde(default)]
    pub activation_executions: Vec<WorkloadActivationExecution>,
    #[serde(default)]
    pub activation_recomputations: Vec<WorkloadActivationRecomputation>,
    pub attention: WorkloadAttention,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<PortfolioReasoningEvidence>,
    pub semantic_atlas: Option<SemanticAtlas>,
    pub semantic_atlas_history: Vec<SemanticAtlas>,
    pub semantic_atlas_deltas: Vec<SemanticAtlasDelta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<WorkloadRevisionStatus>,
}

impl WorkloadList {
    #[must_use]
    pub fn new(
        catalog: WorkloadCatalogDescriptor,
        workloads: Vec<WorkloadSummary>,
        drafts: Vec<WorkloadDraft>,
        activation_admissions: Vec<WorkloadActivationAdmission>,
        activation_executions: Vec<WorkloadActivationExecution>,
        attention: WorkloadAttention,
        runtime: Option<PortfolioReasoningEvidence>,
    ) -> Self {
        let semantic_atlas = semantic_atlas_from_summaries(&workloads)
            .expect("retained verified evidence must produce a semantic atlas");
        Self {
            schema: WORKLOAD_LIST_SCHEMA.to_owned(),
            catalog,
            workloads,
            drafts,
            activation_admissions,
            activation_executions,
            activation_recomputations: Vec::new(),
            attention,
            runtime,
            semantic_atlas,
            semantic_atlas_history: Vec::new(),
            semantic_atlas_deltas: Vec::new(),
            revision: None,
        }
    }

    #[must_use]
    pub fn with_revision(mut self, revision: WorkloadRevisionStatus) -> Self {
        self.revision = Some(revision);
        self
    }

    #[must_use]
    pub fn with_activation_recomputations(
        mut self,
        recomputations: Vec<WorkloadActivationRecomputation>,
    ) -> Self {
        self.activation_recomputations = recomputations;
        self
    }

    #[must_use]
    pub fn with_semantic_atlas_history(
        mut self,
        history: Vec<SemanticAtlas>,
        deltas: Vec<SemanticAtlasDelta>,
    ) -> Self {
        self.semantic_atlas_history = history;
        self.semantic_atlas_deltas = deltas;
        self
    }
}

pub fn derive_semantic_atlas(
    definitions: &[WorkloadDefinition],
    state: &LocalWorkloadState,
) -> Result<Option<SemanticAtlas>, rey_mining::SemanticAtlasError> {
    let summaries = definitions
        .iter()
        .map(|workload| WorkloadSummary::derive(workload, state.record(&workload.workload.id)))
        .collect::<Vec<_>>();
    semantic_atlas_from_summaries(&summaries)
}

fn semantic_atlas_from_summaries(
    workloads: &[WorkloadSummary],
) -> Result<Option<SemanticAtlas>, rey_mining::SemanticAtlasError> {
    SemanticAtlas::from_admitted_evidence(
        workloads.iter().filter_map(|workload| {
            workload
                .topography_patch
                .as_ref()
                .map(|patch| (workload.workload.id.as_str(), patch))
        }),
        workloads.iter().filter_map(|workload| {
            workload
                .latest_scene_admission
                .as_ref()
                .and_then(|result| result.scene.as_ref())
                .map(|scene| (workload.workload.id.as_str(), scene))
        }),
    )
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadStatusView {
    pub schema: String,
    pub summary: WorkloadSummary,
    pub definition: WorkloadDefinition,
    pub last_test: Option<WorkloadTestResult>,
    pub last_run: Option<WorkloadRunResult>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadStatusBatch {
    pub schema: String,
    pub catalog: WorkloadCatalogDescriptor,
    pub statuses: Vec<WorkloadStatusView>,
    pub drafts: Vec<WorkloadDraft>,
    pub attention: WorkloadAttention,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<PortfolioReasoningEvidence>,
}

impl WorkloadStatusBatch {
    #[must_use]
    pub fn new(
        catalog: WorkloadCatalogDescriptor,
        statuses: Vec<WorkloadStatusView>,
        drafts: Vec<WorkloadDraft>,
        attention: WorkloadAttention,
        runtime: Option<PortfolioReasoningEvidence>,
    ) -> Self {
        Self {
            schema: WORKLOAD_STATUS_BATCH_SCHEMA.to_owned(),
            catalog,
            statuses,
            drafts,
            attention,
            runtime,
        }
    }
}

pub fn derive_portfolio_snapshot(
    definitions: &[WorkloadDefinition],
    state: &LocalWorkloadState,
    environment: Option<&CapabilitySnapshot>,
    git: Option<&GitSnapshot>,
) -> Result<PortfolioSnapshot, PortfolioError> {
    if let Some(git) = git {
        git.verify()
            .map_err(|error| PortfolioError::InvalidContract(format!("Git snapshot: {error}")))?;
    }
    let mut catalog_hasher = SemanticHasher::new("rey.workload-catalog.v1");
    catalog_hasher.add_u64(definitions.len() as u64);
    let available_capability_ids = environment
        .into_iter()
        .flat_map(|snapshot| &snapshot.capabilities)
        .filter(|capability| capability.availability == Availability::Available)
        .map(|capability| capability.capability_id.as_str())
        .collect::<BTreeSet<_>>();
    let input_surfaces = environment
        .into_iter()
        .flat_map(|snapshot| &snapshot.capabilities)
        .filter(|capability| {
            capability.provider_id == ENVIRONMENT_MAP_PROVIDER_ID
                && capability.capability_kind == "input_file"
        })
        .filter_map(|capability| {
            capability
                .resolved_location
                .as_deref()
                .map(|location| (location, capability))
        })
        .collect::<BTreeMap<_, _>>();
    let mut surface_owners = BTreeMap::<&str, Vec<&str>>::new();
    for definition in definitions {
        for surface in &definition.owned_surfaces {
            surface_owners
                .entry(&surface.surface_id)
                .or_default()
                .push(&definition.workload.id);
        }
    }
    if let Some((surface_id, owners)) = surface_owners.iter().find(|(_, owners)| owners.len() > 1) {
        return Err(PortfolioError::OwnershipCollision {
            surface_id: (*surface_id).to_owned(),
            owners: owners.iter().map(|owner| (*owner).to_owned()).collect(),
        });
    }
    let mut workloads = Vec::with_capacity(definitions.len());
    for definition in definitions {
        definition.workload.add_semantics(&mut catalog_hasher);
        definition.graph.graph.add_semantics(&mut catalog_hasher);
        let summary = WorkloadSummary::derive(definition, state.record(&definition.workload.id));
        let (policy, policy_reason) = if definition.workload.id == BUILT_IN_MISMATCH_WORKLOAD_ID {
            (
                AttentionPolicy::Exclude,
                Some("deliberate failing conformance fixture".to_owned()),
            )
        } else if definition.workload.id == BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID {
            (
                AttentionPolicy::Exclude,
                Some("portfolio miner cannot schedule itself".to_owned()),
            )
        } else {
            (AttentionPolicy::Track, None)
        };
        let record = state.record(&definition.workload.id);
        let mut evidence_ids = Vec::new();
        if let Some(result) = record.and_then(|record| record.last_test.as_ref()) {
            evidence_ids.push(result.result_id.clone());
        }
        if let Some(result) = record.and_then(|record| record.last_run.as_ref()) {
            evidence_ids.push(result.run_id.clone());
        }
        if !definition.git_dependencies.is_empty()
            && let Some(git) = git
        {
            evidence_ids.push(git.snapshot_id.clone());
        }
        let mut changed_dependency_ids = Vec::new();
        let mut missing_capability_ids = Vec::new();
        for surface in &definition.owned_surfaces {
            match input_surfaces.get(surface.surface_id.as_str()) {
                Some(capability)
                    if capability.content_digest.as_deref()
                        == Some(surface.source_revision.as_str()) => {}
                Some(capability) => changed_dependency_ids.push(format!(
                    "surface:{}@{}",
                    surface.surface_id,
                    capability.content_digest.as_deref().unwrap_or("unknown")
                )),
                None => changed_dependency_ids
                    .push(format!("surface:{}@unobserved", surface.surface_id)),
            }
            missing_capability_ids.extend(
                surface
                    .required_capability_ids
                    .iter()
                    .filter(|capability_id| {
                        !available_capability_ids.contains(capability_id.as_str())
                    })
                    .cloned(),
            );
        }
        for dependency in &definition.git_dependencies {
            let actual = git_dependency_revision(dependency, git);
            if actual.as_deref() != Some(dependency.source_revision.as_str()) {
                changed_dependency_ids.push(format!(
                    "git:{}@{}",
                    dependency.dependency_id,
                    actual.as_deref().unwrap_or("unobserved")
                ));
            }
        }
        workloads.push(PortfolioWorkloadObservation {
            workload: definition.workload.clone(),
            graph: definition.graph.graph.clone(),
            qualification: match summary.qualification {
                QualificationState::Untested => PortfolioQualificationState::Untested,
                QualificationState::Qualified => PortfolioQualificationState::Qualified,
                QualificationState::Failing => PortfolioQualificationState::Failing,
                QualificationState::Inconclusive => PortfolioQualificationState::Inconclusive,
                QualificationState::Stale => PortfolioQualificationState::Stale,
            },
            policy,
            policy_reason,
            evidence_ids,
            changed_dependency_ids,
            missing_capability_ids,
        });
    }
    let surfaces = environment
        .into_iter()
        .flat_map(|snapshot| &snapshot.capabilities)
        .filter(|capability| {
            capability.provider_id == ENVIRONMENT_MAP_PROVIDER_ID
                && capability.capability_kind == "input_file"
        })
        .map(|capability| {
            let mut hasher = SemanticHasher::new("rey.portfolio-surface.v1");
            hasher.add_str(&capability.capability_id);
            hasher.add_optional_str(capability.resolved_location.as_deref());
            hasher.add_optional_str(capability.content_digest.as_deref());
            PortfolioSurfaceObservation {
                surface_id: capability
                    .resolved_location
                    .clone()
                    .unwrap_or_else(|| capability.capability_id.clone()),
                source_revision: hasher.finish(),
                owners: capability
                    .resolved_location
                    .as_deref()
                    .and_then(|location| surface_owners.get(location))
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                evidence_ids: environment
                    .map(|snapshot| vec![snapshot.semantic_digest.clone()])
                    .unwrap_or_default(),
            }
        })
        .collect();
    PortfolioSnapshot::new(
        catalog_hasher.finish(),
        environment.map(|snapshot| snapshot.semantic_digest.clone()),
        workloads,
        surfaces,
        PortfolioLimits::default(),
    )
}

pub fn derive_workload_attention(
    definitions: &[WorkloadDefinition],
    state: &LocalWorkloadState,
    environment: Option<&CapabilitySnapshot>,
    git: Option<&GitSnapshot>,
) -> Result<WorkloadAttention, PortfolioError> {
    WorkloadAttention::derive(&derive_portfolio_snapshot(
        definitions,
        state,
        environment,
        git,
    )?)
}

fn git_dependency_revision(
    dependency: &WorkloadGitDependency,
    git: Option<&GitSnapshot>,
) -> Option<String> {
    let snapshot = git?;
    if dependency.repository_id != snapshot.repository_id.as_str() {
        return Some(format!("repository:{}", snapshot.repository_id));
    }
    if dependency.worktree_id.as_deref()
        != snapshot.worktree_id.as_ref().map(SemanticDigest::as_str)
    {
        return Some(format!(
            "worktree:{}",
            snapshot
                .worktree_id
                .as_ref()
                .map_or("absent", SemanticDigest::as_str)
        ));
    }
    match dependency.kind {
        WorkloadGitDependencyKind::Head => {
            if dependency.symbolic_ref.as_deref() != snapshot.head.symbolic_ref.as_deref() {
                return Some(format!(
                    "ref:{}",
                    snapshot.head.symbolic_ref.as_deref().unwrap_or("detached")
                ));
            }
            Some(snapshot.head.commit_oid.as_ref().map_or_else(
                || "unborn".to_owned(),
                |oid| format!("{}:{oid}", snapshot.object_format),
            ))
        }
        WorkloadGitDependencyKind::SemanticIndex => Some(snapshot.index.as_ref().map_or_else(
            || "absent".to_owned(),
            |index| index.entry_digest.to_string(),
        )),
    }
}

impl WorkloadStatusView {
    #[must_use]
    pub fn new(workload: WorkloadDefinition, record: Option<&LocalWorkloadRecord>) -> Self {
        Self {
            schema: WORKLOAD_STATUS_SCHEMA.to_owned(),
            summary: WorkloadSummary::derive(&workload, record),
            definition: workload,
            last_test: record.and_then(|record| record.last_test.clone()),
            last_run: record.and_then(|record| record.last_run.clone()),
        }
    }

    #[must_use]
    pub fn new_resolved(workload: ResolvedWorkload, record: Option<&LocalWorkloadRecord>) -> Self {
        Self {
            schema: WORKLOAD_STATUS_SCHEMA.to_owned(),
            summary: WorkloadSummary::derive_resolved(&workload, record),
            definition: workload.definition,
            last_test: record.and_then(|record| record.last_test.clone()),
            last_run: record.and_then(|record| record.last_run.clone()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadTestBatch {
    pub schema: String,
    pub catalog: WorkloadCatalogDescriptor,
    pub workloads: Vec<WorkloadProvenance>,
    pub results: Vec<WorkloadTestResult>,
}

impl WorkloadTestBatch {
    #[must_use]
    pub fn new(
        catalog: WorkloadCatalogDescriptor,
        workloads: Vec<WorkloadProvenance>,
        results: Vec<WorkloadTestResult>,
    ) -> Self {
        Self {
            schema: WORKLOAD_TEST_BATCH_SCHEMA.to_owned(),
            catalog,
            workloads,
            results,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadRunView {
    pub schema: String,
    pub catalog: WorkloadCatalogDescriptor,
    pub provenance: WorkloadProvenance,
    pub result: WorkloadRunResult,
}

impl WorkloadRunView {
    #[must_use]
    pub fn new(
        catalog: WorkloadCatalogDescriptor,
        provenance: WorkloadProvenance,
        result: WorkloadRunResult,
    ) -> Self {
        Self {
            schema: WORKLOAD_RUN_VIEW_SCHEMA.to_owned(),
            catalog,
            provenance,
            result,
        }
    }
}

#[must_use]
pub fn fresh_qualification<'a>(
    workload: &WorkloadDefinition,
    record: Option<&'a LocalWorkloadRecord>,
) -> Option<&'a QualificationRecord> {
    record
        .and_then(|record| record.last_test.as_ref())
        .filter(|result| result.verify_for(workload).is_ok())
        .and_then(|result| result.qualification.as_ref())
        .filter(|qualification| qualification.is_fresh_for(workload))
}

fn empty_workspace_catalog(root: &str) -> WorkloadCatalog {
    WorkloadCatalog {
        descriptor: WorkloadCatalogDescriptor {
            schema: WORKLOAD_CATALOG_SCHEMA.to_owned(),
            kind: WorkloadCatalogKind::WorkspacePackages,
            root: Some(root.to_owned()),
            workload_count: 0,
            admitted_count: 0,
            draft_count: 0,
        },
        workloads: Vec::new(),
        drafts: Vec::new(),
    }
}

fn workload_digest_placeholder() -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.workload-identity-placeholder.v1");
    hasher.add_str("excluded from identity");
    hasher.finish()
}

fn workload_digest_key(digest: &SemanticDigest) -> &str {
    digest
        .as_str()
        .strip_prefix("blake3:")
        .unwrap_or(digest.as_str())
}

fn workload_snapshot_identity(snapshot: &WorkloadAdmissionSnapshot) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(WORKLOAD_ADMISSION_SNAPSHOT_SCHEMA);
    hasher.add_u64(snapshot.packages.len() as u64);
    for package in &snapshot.packages {
        hasher.add_str(&package.workload_id);
        hasher.add_str(&package.workload_revision.to_string());
        hasher.add_str(&package.title);
        hasher.add_str(&package.source);
        hasher.add_str(package.source_digest.as_str());
        hasher.add_str(&package.object_path);
        hasher.add_u64(package.bytes);
        package.workload.add_semantics(&mut hasher);
        package.graph.add_semantics(&mut hasher);
        package.scenario_suite.add_semantics(&mut hasher);
    }
    hasher.add_optional_str(
        snapshot
            .ignore
            .as_ref()
            .map(|ignore| ignore.source_digest.as_str()),
    );
    if let Some(ignore) = &snapshot.ignore {
        hasher.add_u64(ignore.rules.len() as u64);
        for rule in &ignore.rules {
            hasher.add_str(&rule.kind);
            hasher.add_str(&rule.pattern);
            hasher.add_u64(rule.source_line);
        }
        hasher.add_u64(ignore.omissions.len() as u64);
        for omission in &ignore.omissions {
            hasher.add_str(&omission.rule.kind);
            hasher.add_str(&omission.rule.pattern);
            hasher.add_u64(omission.rule.source_line);
            hasher.add_u64(omission.matched);
        }
        hasher.add_u64(ignore.ignored);
    }
    hasher.finish()
}

fn workload_commit_identity(commit: &WorkloadCommit) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(WORKLOAD_COMMIT_SCHEMA);
    hasher.add_u64(commit.sequence);
    hasher.add_optional_str(commit.parent_commit_id.as_ref().map(SemanticDigest::as_str));
    hasher.add_str(&commit.committed_at_unix.to_string());
    hasher.add_str(&commit.message);
    hasher.add_str(commit.snapshot.snapshot_revision.as_str());
    hasher.add_u64(commit.qualification_ids.len() as u64);
    for qualification_id in &commit.qualification_ids {
        hasher.add_str(qualification_id.as_str());
    }
    hasher.finish()
}

fn workload_activation_admission_digest(admission: &WorkloadActivationAdmission) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(WORKLOAD_ACTIVATION_ADMISSION_SCHEMA);
    hasher.add_str(admission.activation.activation_id.as_str());
    hasher.add_str(admission.workload_head_commit_id.as_str());
    hasher.add_str(admission.workload_head_snapshot_id.as_str());
    admission.workload.add_semantics(&mut hasher);
    admission.graph.add_semantics(&mut hasher);
    admission.scenario_suite.add_semantics(&mut hasher);
    admission.evaluator.add_semantics(&mut hasher);
    hasher.add_u64(admission.declared_scenarios.len() as u64);
    for scenario in &admission.declared_scenarios {
        scenario.add_semantics(&mut hasher);
    }
    hasher.add_u64(admission.selected_scenario_ids.len() as u64);
    for scenario_id in &admission.selected_scenario_ids {
        hasher.add_str(scenario_id);
    }
    hasher.add_str(admission.capability_snapshot_id.as_str());
    hasher.add_u64(admission.effective_budget.max_scenarios);
    hasher.add_u64(admission.effective_budget.max_actions);
    hasher.add_u64(admission.effective_budget.max_evidence_bytes);
    hasher.add_str(&admission.authority);
    hasher.finish()
}

fn workload_activation_execution_digest(execution: &WorkloadActivationExecution) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(WORKLOAD_ACTIVATION_EXECUTION_SCHEMA);
    hasher.add_str(execution.admission_id.as_str());
    hasher.add_str(execution.activation_id.as_str());
    hasher.add_optional_str(
        execution
            .source_execution_id
            .as_ref()
            .map(SemanticDigest::as_str),
    );
    hasher.add_str(execution.result.result_id.as_str());
    hasher.add_u64(execution.evidence_bytes);
    hasher.add_str(&execution.authority);
    hasher.finish()
}

fn workload_activation_recomputation_digest(
    recomputation: &WorkloadActivationRecomputation,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(WORKLOAD_ACTIVATION_RECOMPUTATION_SCHEMA);
    hasher.add_str(recomputation.execution_id.as_str());
    hasher.add_str(recomputation.admission_id.as_str());
    hasher.add_str(recomputation.selected_result_id.as_str());
    hasher.add_str(recomputation.full_result.result_id.as_str());
    hasher.add_u64(recomputation.full_evidence_bytes);
    hasher.add_u64(recomputation.max_evidence_bytes);
    hasher.add_u64(recomputation.comparisons.len() as u64);
    for comparison in &recomputation.comparisons {
        hasher.add_str(&comparison.scenario_id);
        hasher.add_str(comparison.selected_execution_id.as_str());
        hasher.add_str(comparison.full_execution_id.as_str());
        hasher.add_bool(comparison.equivalent);
    }
    hasher.add_str(recomputation.assessment.as_str());
    hasher.add_str(&recomputation.authority);
    hasher.finish()
}

fn normalize_workload_commit_message(message: String) -> Result<String, LocalWorkloadStateError> {
    let normalized = message.trim().to_owned();
    if normalized.is_empty()
        || normalized.len() > MAX_COMMIT_MESSAGE_BYTES
        || normalized.contains('\0')
    {
        return Err(LocalWorkloadStateError::CommitMessage);
    }
    Ok(normalized)
}

fn verify_workload_commit_history(
    commits: &[WorkloadCommit],
) -> Result<(), LocalWorkloadStateError> {
    if commits.len() > MAX_WORKLOAD_COMMITS {
        return Err(LocalWorkloadStateError::CommitLimit(MAX_WORKLOAD_COMMITS));
    }
    let mut parent: Option<&WorkloadCommit> = None;
    let mut ids = BTreeSet::new();
    for (index, commit) in commits.iter().enumerate() {
        commit.verify()?;
        let expected_sequence = index as u64 + 1;
        if commit.sequence != expected_sequence
            || commit.parent_commit_id.as_ref() != parent.map(|parent| &parent.commit_id)
            || !ids.insert(commit.commit_id.clone())
        {
            return Err(LocalWorkloadStateError::CommitHistory);
        }
        parent = Some(commit);
    }
    Ok(())
}

fn read_local_workload_file(path: &Path, limit: u64) -> Result<Vec<u8>, LocalWorkloadStateError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| LocalWorkloadStateError::Read {
        path: path.to_owned(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LocalWorkloadStateError::UnsafeStatePath(path.to_owned()));
    }
    if metadata.len() > limit {
        return Err(LocalWorkloadStateError::ByteLimit {
            path: path.to_owned(),
            limit,
        });
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|source| LocalWorkloadStateError::Read {
            path: path.to_owned(),
            source,
        })?
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| LocalWorkloadStateError::Read {
            path: path.to_owned(),
            source,
        })?;
    if bytes.len() as u64 > limit {
        return Err(LocalWorkloadStateError::ByteLimit {
            path: path.to_owned(),
            limit,
        });
    }
    Ok(bytes)
}

fn write_local_content_addressed(path: &Path, bytes: &[u8]) -> Result<(), LocalWorkloadStateError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(LocalWorkloadStateError::UnsafeStatePath(path.to_owned()));
        }
        Ok(_) => {
            if read_local_workload_file(path, MAX_WORKLOAD_PACKAGE_BYTES)? == bytes {
                return Ok(());
            }
            return Err(LocalWorkloadStateError::ContentAddressCollision(
                path.to_owned(),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(LocalWorkloadStateError::Write {
                path: path.to_owned(),
                source,
            });
        }
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| LocalWorkloadStateError::Write {
            path: path.to_owned(),
            source,
        })?;
    file.write_all(bytes)
        .and_then(|()| file.flush())
        .map_err(|source| LocalWorkloadStateError::Write {
            path: path.to_owned(),
            source,
        })
}

#[derive(Debug, Error)]
pub enum LocalWorkloadStateError {
    #[error("unsupported local workload state schema {actual}")]
    UnsupportedSchema { actual: String },
    #[error("local workload state exceeds the {limit}-record limit")]
    RecordLimit { limit: usize },
    #[error("local workload state contains an empty workload id")]
    EmptyWorkloadId,
    #[error("retained semantic atlas history is invalid, non-linear, or exceeds its bound")]
    SemanticAtlasHistory,
    #[error("retained regional scene for workload {0} does not bind an exact retained atlas")]
    RegionalSceneAtlasBinding(String),
    #[error("local workload state record {0} has no retained artifact")]
    EmptyRecord(String),
    #[error("workload activation admission is invalid or has been tampered with")]
    InvalidActivationAdmission,
    #[error("workload activation execution is invalid, over budget, or has been tampered with")]
    InvalidActivationExecution,
    #[error("workload activation full recomputation is invalid or has been tampered with")]
    InvalidActivationRecomputation,
    #[error("workload activation admission precondition failed: {0}")]
    ActivationPrecondition(String),
    #[error("local workload state exceeds {0} activation admissions")]
    ActivationAdmissionLimit(usize),
    #[error(
        "Git activation {activation_id} is already admitted as workload activation {admission_id}"
    )]
    ActivationAlreadyAdmitted {
        activation_id: SemanticDigest,
        admission_id: SemanticDigest,
    },
    #[error("unknown workload activation admission {0}")]
    UnknownActivationAdmission(SemanticDigest),
    #[error("local workload state exceeds {0} activation executions")]
    ActivationExecutionLimit(usize),
    #[error("unknown workload activation execution {0}")]
    UnknownActivationExecution(SemanticDigest),
    #[error("workload activation admission {admission_id} already executed as {execution_id}")]
    ActivationAlreadyExecuted {
        admission_id: SemanticDigest,
        execution_id: SemanticDigest,
    },
    #[error("local workload state exceeds {0} activation full recomputations")]
    ActivationRecomputationLimit(usize),
    #[error(
        "workload activation execution {execution_id} already has full recomputation proof {recomputation_id}"
    )]
    ActivationAlreadyRecomputed {
        execution_id: SemanticDigest,
        recomputation_id: SemanticDigest,
    },
    #[error(
        "workload activation full recomputation evidence uses {observed} bytes, exceeding the {limit}-byte limit"
    )]
    ActivationRecomputationBudget { observed: u64, limit: u64 },
    #[error("state record key {key} does not match artifact workload {artifact}")]
    RecordIdentity { key: String, artifact: String },
    #[error("unsupported workload admission snapshot schema {0}")]
    SnapshotSchema(String),
    #[error("workload admission snapshot exceeds {0} packages")]
    SnapshotLimit(usize),
    #[error("workload admission snapshot is not in canonical workload order")]
    NonCanonicalSnapshot,
    #[error("workload admission snapshot package is invalid: {0}")]
    SnapshotPackage(String),
    #[error("workload admission snapshot identity does not match its contents")]
    SnapshotIdentity,
    #[error("unsupported workload commit schema {0}")]
    CommitSchema(String),
    #[error("workload commit identity does not match its contents")]
    CommitIdentity,
    #[error("workload commit qualification set does not cover its exact snapshot")]
    CommitQualificationShape,
    #[error("workload commit history is not a canonical linear chain")]
    CommitHistory,
    #[error("workload commit history exceeds {0} commits")]
    CommitLimit(usize),
    #[error("workload commit message must be nonempty, NUL-free, and at most 4096 bytes")]
    CommitMessage,
    #[error("workload INDEX is empty; run `rey workloads add` before qualification or approval")]
    EmptyIndex,
    #[error("nothing is staged for workload admission")]
    NothingToCommit,
    #[error("retained workload qualification does not identify the current INDEX")]
    QualifiedIndexMismatch,
    #[error("workload INDEX {0} requires complete passing qualification of that exact snapshot")]
    ExactIndexQualificationRequired(SemanticDigest),
    #[error("workload {0} requires fresh passing qualification for its exact staged package")]
    QualificationRequired(String),
    #[error("workload approval precondition failed: {0}")]
    ApprovalPrecondition(String),
    #[error("workload log count {actual} is outside 1..={limit}")]
    LogLimit { limit: usize, actual: usize },
    #[error("workload artifact source is not a canonical retained path: {0}")]
    ArtifactSource(String),
    #[error("staged workload artifact identity changed: {0}")]
    ArtifactIdentity(String),
    #[error("content-addressed workload object changed in place: {0}")]
    ContentAddressCollision(PathBuf),
    #[error("unsafe symlink or non-regular local workload state path {0}")]
    UnsafeStatePath(PathBuf),
    #[error("local workload state {path} exceeds the {limit}-byte limit")]
    ByteLimit { path: PathBuf, limit: u64 },
    #[error("local workload state {path} could not be read: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("local workload state {path} could not be written: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("local workload state lock {path} failed: {source}")]
    Lock {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("local workload state {path} is invalid JSON: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("could not allocate a local workload state temporary file in {0}")]
    TemporaryLimit(PathBuf),
    #[error(transparent)]
    Catalog(#[from] WorkloadCatalogError),
    #[error("staged workload package YAML is invalid: {0}")]
    Yaml(#[from] serde_saphyr::Error),
    #[error(transparent)]
    Workload(#[from] rey_runtime::WorkloadError),
    #[error(transparent)]
    Git(#[from] rey_git::GitError),
    #[error(transparent)]
    SemanticAtlas(#[from] rey_mining::SemanticAtlasError),
    #[error(transparent)]
    Ignore(#[from] ReyIgnoreError),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rey_core::SemanticHasher;
    use rey_environment::{
        Availability, CapabilityRecord, CapabilitySnapshot, DiscoveryLimits, TrustClass,
    };
    use rey_runtime::{
        AttentionAction, BUILT_IN_NORMALIZE_WORKLOAD_ID, PortfolioError, WorkloadRunResult,
        built_in_workload, test_workload, test_workload_with_observer_and_snapshot,
    };
    use tempfile::TempDir;

    use super::{
        LocalWorkloadState, LocalWorkloadStore, QualificationState, WorkloadAdmissionState,
        WorkloadCatalog, WorkloadCatalogError, WorkloadCatalogKind, WorkloadFreshness,
        WorkloadOrigin, WorkloadSummary, derive_portfolio_snapshot,
    };

    const WORKSPACE_PACKAGE: &str =
        include_str!("../../../sys/context-anchor-survey/workload.yaml");

    #[test]
    fn workspace_catalog_loads_exact_proposal_and_rejects_incomplete_provenance() {
        let workspace = TempDir::new().unwrap();
        let package = workspace.path().join("workloads/package");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("workload.yaml"), WORKSPACE_PACKAGE).unwrap();

        let catalog =
            WorkloadCatalog::load_workspace(workspace.path(), std::path::Path::new("workloads"))
                .unwrap();
        assert_eq!(
            catalog.descriptor.kind,
            WorkloadCatalogKind::WorkspacePackages
        );
        assert_eq!(catalog.workloads.len(), 1);
        assert_eq!(catalog.descriptor.admitted_count, 0);
        assert_eq!(
            catalog.workloads[0].provenance.origin,
            WorkloadOrigin::WorkspacePackage
        );
        assert_eq!(
            catalog.workloads[0].provenance.admission.state,
            WorkloadAdmissionState::Proposed
        );
        assert!(catalog.workloads[0].definition.proposal.is_some());

        fs::write(
            package.join("workload.yaml"),
            WORKSPACE_PACKAGE.replace("    - scenario_suite\n", ""),
        )
        .unwrap();
        assert!(matches!(
            WorkloadCatalog::load_workspace(workspace.path(), std::path::Path::new("workloads")),
            Err(WorkloadCatalogError::IncompleteGenerationProvenance)
        ));
    }

    #[test]
    fn portfolio_binds_declared_ownership_and_live_environment_invalidation() {
        let workspace = TempDir::new().unwrap();
        let package = workspace.path().join("workloads/package");
        fs::create_dir_all(&package).unwrap();
        let source_revision = SemanticHasher::new("owned-source").finish().to_string();
        let package_document = WORKSPACE_PACKAGE.replace(
            "\ngraph:\n",
            &format!(
                "\nownership:\n  surfaces:\n    - surface_id: plans/0003-scene-to-explorer.md\n      source_revision: {source_revision}\n      required_capabilities:\n        - parser.rust\n\ngraph:\n"
            ),
        );
        fs::write(package.join("workload.yaml"), package_document).unwrap();
        let catalog =
            WorkloadCatalog::load_workspace(workspace.path(), std::path::Path::new("workloads"))
                .unwrap();
        let definition = catalog.definitions().remove(0);
        let input = fixture_capability(
            "rey.env-map",
            "env.mapping.node.plan",
            "input_file",
            Some("plans/0003-scene-to-explorer.md"),
            Some(&source_revision),
            Availability::Available,
        );
        let parser = fixture_capability(
            "fixture.parser",
            "parser.rust",
            "parser",
            None,
            None,
            Availability::Available,
        );
        let environment = CapabilitySnapshot::new(
            "fixture",
            DiscoveryLimits::default(),
            vec![input.clone(), parser],
        )
        .unwrap();
        let state = LocalWorkloadState::default();

        let snapshot = derive_portfolio_snapshot(
            std::slice::from_ref(&definition),
            &state,
            Some(&environment),
            None,
        )
        .unwrap();
        assert_eq!(
            snapshot.surfaces[0].owners.as_slice(),
            std::slice::from_ref(&definition.workload.id)
        );
        assert!(snapshot.workloads[0].changed_dependency_ids.is_empty());
        assert!(snapshot.workloads[0].missing_capability_ids.is_empty());

        let changed = CapabilitySnapshot::new(
            "fixture",
            DiscoveryLimits::default(),
            vec![fixture_capability(
                "rey.env-map",
                "env.mapping.node.plan",
                "input_file",
                Some("plans/0003-scene-to-explorer.md"),
                Some(SemanticHasher::new("changed-source").finish().as_str()),
                Availability::Available,
            )],
        )
        .unwrap();
        let changed_snapshot = derive_portfolio_snapshot(
            std::slice::from_ref(&definition),
            &state,
            Some(&changed),
            None,
        )
        .unwrap();
        assert_eq!(
            changed_snapshot.workloads[0].missing_capability_ids,
            ["parser.rust"]
        );
        assert_eq!(
            changed_snapshot.workloads[0].changed_dependency_ids.len(),
            1
        );

        let mut colliding = definition.clone();
        colliding.workload.id = "other-owner".to_owned();
        assert!(matches!(
            derive_portfolio_snapshot(&[definition, colliding], &state, Some(&environment), None,),
            Err(PortfolioError::OwnershipCollision { .. })
        ));

        let unowned_environment = CapabilitySnapshot::new(
            "fixture",
            DiscoveryLimits::default(),
            vec![fixture_capability(
                "rey.env-map",
                "env.mapping.node.other",
                "input_file",
                Some("docs/unowned.md"),
                Some(SemanticHasher::new("unowned").finish().as_str()),
                Availability::Available,
            )],
        )
        .unwrap();
        let unowned =
            derive_portfolio_snapshot(&[], &state, Some(&unowned_environment), None).unwrap();
        let attention = rey_runtime::WorkloadAttention::derive(&unowned).unwrap();
        assert_eq!(attention.rows[0].action, AttentionAction::Create);
    }

    fn fixture_capability(
        provider_id: &str,
        capability_id: &str,
        capability_kind: &str,
        resolved_location: Option<&str>,
        content_digest: Option<&str>,
        availability: Availability,
    ) -> CapabilityRecord {
        CapabilityRecord {
            provider_id: provider_id.to_owned(),
            provider_revision: 1,
            provider_kind: "fixture".to_owned(),
            capability_id: capability_id.to_owned(),
            capability_kind: capability_kind.to_owned(),
            resolved_location: resolved_location.map(str::to_owned),
            version: None,
            content_digest: content_digest.map(str::to_owned),
            provenance: Some("fixture".to_owned()),
            availability,
            trust_class: TrustClass::ExplicitLocal,
            operations: Vec::new(),
            enforced_limits: Vec::new(),
            unsupported_limits: Vec::new(),
            observed_at: None,
            error_code: None,
            error_detail: None,
        }
    }

    #[test]
    fn creation_request_is_content_identified_and_yields_to_a_working_package() {
        let workspace = TempDir::new().unwrap();
        let catalog_dir = std::path::Path::new("workloads");
        let workload_id = "context-anchor-survey";
        let created = WorkloadCatalog::create_workspace_request(
            workspace.path(),
            catalog_dir,
            workload_id,
            Some("Context anchor survey"),
            Some("Survey admitted context anchors"),
            None,
        )
        .unwrap();
        let request_path = workspace
            .path()
            .join("workloads/context-anchor-survey/request.yaml");
        let request_bytes = fs::read(&request_path).unwrap();

        let draft_catalog = WorkloadCatalog::load_workspace(workspace.path(), catalog_dir).unwrap();
        assert_eq!(draft_catalog.descriptor.workload_count, 1);
        assert_eq!(draft_catalog.descriptor.admitted_count, 0);
        assert_eq!(draft_catalog.descriptor.draft_count, 1);
        assert_eq!(draft_catalog.drafts[0], created.draft);

        let mut tampered: serde_json::Value = serde_json::from_slice(&request_bytes).unwrap();
        tampered["title"] = "silently changed".into();
        fs::write(&request_path, serde_json::to_vec_pretty(&tampered).unwrap()).unwrap();
        assert!(matches!(
            WorkloadCatalog::load_workspace(workspace.path(), catalog_dir),
            Err(WorkloadCatalogError::InvalidCreationRequest(_))
        ));

        fs::write(&request_path, request_bytes).unwrap();
        fs::write(
            request_path.with_file_name("workload.yaml"),
            WORKSPACE_PACKAGE,
        )
        .unwrap();
        assert!(matches!(
            WorkloadCatalog::load_workspace(workspace.path(), catalog_dir),
            Err(WorkloadCatalogError::HarnessResponseLineage { .. })
        ));
        let response = WORKSPACE_PACKAGE
            .replace(
                "sys/context-anchor-survey/request.yaml",
                &created.draft.source,
            )
            .replace(
                "blake3:ae6225ab5d7809dd325de1df03e228b9051fbad1d0030e530e4417d9c1048f24",
                &created.draft.source_digest.to_string(),
            );
        fs::write(request_path.with_file_name("workload.yaml"), response).unwrap();
        let working_catalog =
            WorkloadCatalog::load_workspace(workspace.path(), catalog_dir).unwrap();
        assert_eq!(working_catalog.descriptor.workload_count, 1);
        assert_eq!(working_catalog.descriptor.admitted_count, 0);
        assert_eq!(working_catalog.descriptor.draft_count, 0);
        assert!(working_catalog.drafts.is_empty());
        assert_eq!(
            working_catalog.workloads[0].definition.workload.id,
            workload_id
        );
    }

    #[test]
    fn workload_admission_commits_only_the_qualified_frozen_index() {
        let workspace = TempDir::new().unwrap();
        let catalog_dir = std::path::Path::new("workloads");
        let package_dir = workspace.path().join("workloads/context-anchor-survey");
        fs::create_dir_all(&package_dir).unwrap();
        let package_path = package_dir.join("workload.yaml");
        fs::write(&package_path, WORKSPACE_PACKAGE).unwrap();
        let store = LocalWorkloadStore::default_for_workspace(workspace.path());

        let working = store.status(workspace.path(), catalog_dir).unwrap();
        assert_eq!(working.state, super::WorkloadWorkingState::Working);
        assert_eq!(working.unstaged.inserted, 1);
        let added = store.add(workspace.path(), catalog_dir).unwrap();
        assert!(added.staged);
        let staged_revision = added.snapshot.snapshot_revision.clone();

        let staged = store.index_catalog().unwrap();
        let result = test_workload_with_observer_and_snapshot(
            &staged.workloads[0].definition,
            SemanticHasher::new("rey.fixture.topography-capability-snapshot.v1").finish(),
            |_| {},
        )
        .unwrap();
        assert!(result.qualification.is_some(), "{:?}", result.status);
        let mut state = store.load().unwrap();
        state.retain_test(result);
        state.refresh_index_qualification(&staged.definitions());
        store.save(&state).unwrap();
        let ready = store.status(workspace.path(), catalog_dir).unwrap();
        assert!(ready.commit_ready, "{:?}", ready.qualification_omissions);

        fs::write(
            &package_path,
            WORKSPACE_PACKAGE.replace("Survey project context anchors", "Agent editing continues"),
        )
        .unwrap();
        let committed = store
            .commit(
                "approve context survey".to_owned(),
                Some("EMPTY"),
                Some(staged_revision.as_str()),
            )
            .unwrap();
        assert_eq!(committed.commit.sequence, 1);
        assert_eq!(committed.commit.qualification_ids.len(), 1);
        assert_eq!(store.head_catalog().unwrap().workloads.len(), 1);
        let status = store.status(workspace.path(), catalog_dir).unwrap();
        assert_eq!(status.state, super::WorkloadWorkingState::Working);
        assert_eq!(status.unstaged.modified, 1);
    }

    #[test]
    fn workload_add_rejects_a_changed_expected_working_file_snapshot() {
        let workspace = TempDir::new().unwrap();
        let catalog_dir = std::path::Path::new("sys");
        let package_dir = workspace.path().join("sys/context-anchor-survey");
        fs::create_dir_all(&package_dir).unwrap();
        let package_path = package_dir.join("workload.yaml");
        fs::write(&package_path, WORKSPACE_PACKAGE).unwrap();
        let store = LocalWorkloadStore::default_for_workspace(workspace.path());
        let reviewed = store
            .status(workspace.path(), catalog_dir)
            .unwrap()
            .working
            .snapshot_revision;

        fs::write(
            package_path,
            WORKSPACE_PACKAGE.replace("Survey project context anchors", "Changed after review"),
        )
        .unwrap();
        let error = store
            .add_expected(workspace.path(), catalog_dir, Some(reviewed.as_str()))
            .unwrap_err();
        assert!(matches!(
            error,
            super::LocalWorkloadStateError::ApprovalPrecondition(_)
        ));
        assert!(store.load().unwrap().index.is_none());
    }

    #[test]
    fn state_round_trips_verified_results_and_derives_progress() {
        let directory = TempDir::new().unwrap();
        let store = LocalWorkloadStore::new(directory.path().join("state"));
        let workload = built_in_workload(BUILT_IN_NORMALIZE_WORKLOAD_ID).unwrap();
        let result = test_workload(&workload).unwrap();
        let mut state = LocalWorkloadState::default();
        state.retain_test(result);

        store.save(&state).unwrap();
        let loaded = store.load().unwrap();
        let summary =
            WorkloadSummary::derive(&workload, loaded.record(BUILT_IN_NORMALIZE_WORKLOAD_ID));

        assert_eq!(summary.freshness, WorkloadFreshness::Fresh);
        assert_eq!(summary.qualification, QualificationState::Qualified);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.evaluated, 2);
        assert!(summary.qualified_graph.is_some());
    }

    #[test]
    fn missing_state_is_empty_and_tampering_fails_closed() {
        let directory = TempDir::new().unwrap();
        let store = LocalWorkloadStore::new(directory.path().join("state"));
        assert!(store.load().unwrap().records.is_empty());

        let workload = built_in_workload(BUILT_IN_NORMALIZE_WORKLOAD_ID).unwrap();
        let mut state = LocalWorkloadState::default();
        state.retain_run(WorkloadRunResult::blocked(&workload, Default::default()));
        store.save(&state).unwrap();
        let path = store.path();
        let mut document: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        document["records"][BUILT_IN_NORMALIZE_WORKLOAD_ID]["last_run"]["stop_reason"] =
            "tampered".into();
        fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();

        assert!(store.load().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn state_file_symlinks_are_rejected() {
        use std::os::unix::fs::symlink;

        let directory = TempDir::new().unwrap();
        let state_directory = directory.path().join("state");
        fs::create_dir(&state_directory).unwrap();
        let target = directory.path().join("target");
        fs::write(&target, b"{}").unwrap();
        symlink(&target, state_directory.join("state.json")).unwrap();

        assert!(LocalWorkloadStore::new(state_directory).load().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn workspace_catalog_rejects_symlinked_packages() {
        use std::os::unix::fs::symlink;

        let workspace = TempDir::new().unwrap();
        let catalog = workspace.path().join("workloads");
        let outside = workspace.path().join("outside");
        fs::create_dir_all(&catalog).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("workload.yaml"), WORKSPACE_PACKAGE).unwrap();
        symlink(&outside, catalog.join("linked")).unwrap();

        assert!(matches!(
            WorkloadCatalog::load_workspace(workspace.path(), std::path::Path::new("workloads")),
            Err(WorkloadCatalogError::UnsafePath(_))
        ));
    }
}
