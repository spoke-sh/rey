use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_environment::{CapabilitySnapshot, ENVIRONMENT_MAP_PROVIDER_ID};
use rey_mining::{ProjectionPacket, SemanticAtlas, TopographyCoverage, TopographyPatch};
use rey_runtime::{
    AttentionPolicy, BUILT_IN_MISMATCH_WORKLOAD_ID, BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID,
    CONTEXT_ANCHOR_SURVEY_OPERATION_ID, ComputeGraph, GraphLimits, GraphNode, GraphOutput,
    PortfolioError, PortfolioLimits, PortfolioQualificationState, PortfolioSnapshot,
    PortfolioSurfaceObservation, PortfolioWorkloadObservation, QualificationRecord,
    RENDER_TOPOGRAPHY_PATCH_OPERATION_ID, RunStatus, Scenario, ScenarioSuite, TestStatus,
    TopographySurveyScenario, ValueSource, ValueType, WorkloadAttention, WorkloadDefinition,
    WorkloadDefinitionParts, WorkloadLimits, WorkloadPort, WorkloadRunResult, WorkloadTestResult,
    WorkloadValue, built_in_operation_contract, built_in_workloads, utf8_exact_comparator_contract,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const LOCAL_WORKLOAD_STATE_SCHEMA: &str = "rey.local-workload-state.v2";
pub const WORKLOAD_LIST_SCHEMA: &str = "rey.workload-list.v8";
pub const WORKLOAD_STATUS_SCHEMA: &str = "rey.workload-status.v7";
pub const WORKLOAD_STATUS_BATCH_SCHEMA: &str = "rey.workload-status-batch.v7";
pub const WORKLOAD_TEST_BATCH_SCHEMA: &str = "rey.workload-test-batch.v5";
pub const WORKLOAD_PACKAGE_SCHEMA: &str = "rey.workload-package.v1";
pub const WORKLOAD_CREATION_REQUEST_SCHEMA: &str = "rey.workload-creation-request.v1";
pub const WORKLOAD_CREATE_RESULT_SCHEMA: &str = "rey.workload-create-result.v1";
pub const WORKLOAD_CATALOG_SCHEMA: &str = "rey.workload-catalog.v2";
pub const WORKLOAD_RUN_VIEW_SCHEMA: &str = "rey.workload-run-view.v3";

const STATE_FILE_NAME: &str = "state.json";
const MAX_STATE_BYTES: u64 = 4 * 1_024 * 1_024;
const MAX_STATE_RECORDS: usize = 64;
const WORKLOAD_PACKAGE_FILE_NAME: &str = "workload.yaml";
const WORKLOAD_CREATION_REQUEST_FILE_NAME: &str = "request.yaml";
const MAX_WORKLOAD_PACKAGES: usize = 128;
const MAX_WORKLOAD_PACKAGE_BYTES: u64 = 1_024 * 1_024;
const MAX_GENERATION_INPUTS: usize = 64;
const MAX_PROVENANCE_TEXT_BYTES: usize = 1_024;
const MAX_WORKLOAD_INTENT_BYTES: usize = 16 * 1_024;

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
    pub requirements: Vec<String>,
    pub limits: WorkloadCreationLimits,
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
                    let resolved = supplied.resolve(source, &bytes)?;
                    let workload_id = resolved.definition.workload.id.clone();
                    if let Some(request) = request {
                        let draft = load_workload_draft(workspace, &request)?;
                        if draft.request.workload_id != workload_id {
                            return Err(WorkloadCatalogError::RequestIdentity {
                                request: draft.request.workload_id,
                                package: workload_id,
                            });
                        }
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
                admitted_count: workloads.len() as u64,
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
        let requirements = workload_creation_requirements();
        let limits = WorkloadCreationLimits::default();
        let request_id = workload_creation_request_digest(
            workload_id,
            title,
            intent,
            &catalog_root,
            &target_package.display().to_string(),
            &requirements,
            &limits,
        );
        let request = WorkloadCreationRequest {
            schema: WORKLOAD_CREATION_REQUEST_SCHEMA.to_owned(),
            request_id,
            workload_id: workload_id.to_owned(),
            title: title.to_owned(),
            intent: intent.map(str::to_owned),
            proposer: WorkloadProposalKind::CodingHarness,
            catalog_root,
            target_package: target_package.display().to_string(),
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
            || self.requirements != workload_creation_requirements()
            || self.limits != WorkloadCreationLimits::default()
        {
            return Err(WorkloadCatalogError::InvalidCreationRequest(
                self.workload_id.clone(),
            ));
        }
        validate_relative_catalog_dir(Path::new(&self.catalog_root))?;
        let expected_target = Path::new(&self.catalog_root)
            .join(&self.workload_id)
            .join(WORKLOAD_PACKAGE_FILE_NAME)
            .display()
            .to_string();
        if self.target_package != expected_target
            || self.request_id
                != workload_creation_request_digest(
                    &self.workload_id,
                    &self.title,
                    self.intent.as_deref(),
                    &self.catalog_root,
                    &self.target_package,
                    &self.requirements,
                    &self.limits,
                )
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

fn workload_creation_requirements() -> Vec<String> {
    vec![
        "Mine exact authoritative workspace and environment sources; retain their revision references."
            .to_owned(),
        "Define a bounded typed compute graph using only admitted operation contracts."
            .to_owned(),
        "Generate required and optional scenarios from authoritative behavior; never derive expected values from candidate execution."
            .to_owned(),
        "Freeze the scenario oracle and mark admission accepted only after graph and suite review."
            .to_owned(),
        "Materialize the target workload.yaml and preserve request.yaml as creation lineage."
            .to_owned(),
    ]
}

fn workload_creation_request_digest(
    workload_id: &str,
    title: &str,
    intent: Option<&str>,
    catalog_root: &str,
    target_package: &str,
    requirements: &[String],
    limits: &WorkloadCreationLimits,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(WORKLOAD_CREATION_REQUEST_SCHEMA);
    hasher.add_str(workload_id);
    hasher.add_str(title);
    hasher.add_optional_str(intent);
    hasher.add_str("coding_harness");
    hasher.add_str(catalog_root);
    hasher.add_str(target_package);
    hasher.add_u64(requirements.len() as u64);
    for requirement in requirements {
        hasher.add_str(requirement);
    }
    hasher.add_u64(limits.max_package_bytes);
    hasher.add_u64(limits.max_graph_nodes);
    hasher.add_u64(limits.max_scenarios);
    hasher.add_u64(limits.max_string_bytes);
    hasher.finish()
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
    admission: WorkloadAdmission,
    graph: SuppliedGraph,
    scenarios: SuppliedScenarioSuite,
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
}

impl SuppliedWorkloadPackage {
    fn resolve(
        self,
        source: String,
        bytes: &[u8],
    ) -> Result<ResolvedWorkload, WorkloadCatalogError> {
        if self.schema != WORKLOAD_PACKAGE_SCHEMA {
            return Err(WorkloadCatalogError::UnsupportedPackageSchema(self.schema));
        }
        validate_generation(&self.generation)?;
        if self.admission.state != WorkloadAdmissionState::Accepted
            || self.admission.scenario_oracle != ScenarioOraclePolicy::Frozen
        {
            return Err(WorkloadCatalogError::NotAdmitted(self.workload.id));
        }
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
            .map(|scenario| {
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
                match scenario.survey {
                    Some(survey) => Scenario::new_versioned_topography(
                        &id,
                        scenario.revision,
                        scenario.required,
                        inputs,
                        expected,
                        survey,
                    ),
                    None => Scenario::new_versioned(
                        &id,
                        scenario.revision,
                        scenario.required,
                        inputs,
                        expected,
                        None,
                    ),
                }
            })
            .collect();
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
                admission: self.admission,
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
pub struct LocalWorkloadState {
    pub schema: String,
    pub records: BTreeMap<String, LocalWorkloadRecord>,
}

impl Default for LocalWorkloadState {
    fn default() -> Self {
        Self {
            schema: LOCAL_WORKLOAD_STATE_SCHEMA.to_owned(),
            records: BTreeMap::new(),
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
            }
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
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.directory).map_err(|source| {
                    LocalWorkloadStateError::Write {
                        path: self.directory.clone(),
                        source,
                    }
                })
            }
            Err(source) => Err(LocalWorkloadStateError::Write {
                path: self.directory.clone(),
                source,
            }),
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
        let last_patch = run_topography
            .last()
            .copied()
            .or_else(|| test_topography.last().copied());
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
        Self {
            provenance: None,
            workload: workload.workload.clone(),
            title: workload.title.clone(),
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
    pub attention: WorkloadAttention,
    pub semantic_atlas: Option<SemanticAtlas>,
}

impl WorkloadList {
    #[must_use]
    pub fn new(
        catalog: WorkloadCatalogDescriptor,
        workloads: Vec<WorkloadSummary>,
        drafts: Vec<WorkloadDraft>,
        attention: WorkloadAttention,
    ) -> Self {
        let semantic_atlas =
            SemanticAtlas::from_topographies(workloads.iter().filter_map(|workload| {
                workload
                    .topography_patch
                    .as_ref()
                    .map(|patch| (workload.workload.id.as_str(), patch))
            }))
            .expect("retained verified topographies must produce a semantic atlas");
        Self {
            schema: WORKLOAD_LIST_SCHEMA.to_owned(),
            catalog,
            workloads,
            drafts,
            attention,
            semantic_atlas,
        }
    }
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
}

impl WorkloadStatusBatch {
    #[must_use]
    pub fn new(
        catalog: WorkloadCatalogDescriptor,
        statuses: Vec<WorkloadStatusView>,
        drafts: Vec<WorkloadDraft>,
        attention: WorkloadAttention,
    ) -> Self {
        Self {
            schema: WORKLOAD_STATUS_BATCH_SCHEMA.to_owned(),
            catalog,
            statuses,
            drafts,
            attention,
        }
    }
}

pub fn derive_portfolio_snapshot(
    definitions: &[WorkloadDefinition],
    state: &LocalWorkloadState,
    environment: Option<&CapabilitySnapshot>,
) -> Result<PortfolioSnapshot, PortfolioError> {
    let mut catalog_hasher = SemanticHasher::new("rey.workload-catalog.v1");
    catalog_hasher.add_u64(definitions.len() as u64);
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
            changed_dependency_ids: Vec::new(),
            missing_capability_ids: Vec::new(),
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
                owners: Vec::new(),
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
) -> Result<WorkloadAttention, PortfolioError> {
    WorkloadAttention::derive(&derive_portfolio_snapshot(definitions, state, environment)?)
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

#[derive(Debug, Error)]
pub enum LocalWorkloadStateError {
    #[error("unsupported local workload state schema {actual}")]
    UnsupportedSchema { actual: String },
    #[error("local workload state exceeds the {limit}-record limit")]
    RecordLimit { limit: usize },
    #[error("local workload state contains an empty workload id")]
    EmptyWorkloadId,
    #[error("local workload state record {0} has no retained artifact")]
    EmptyRecord(String),
    #[error("state record key {key} does not match artifact workload {artifact}")]
    RecordIdentity { key: String, artifact: String },
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
    #[error("local workload state {path} is invalid JSON: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("could not allocate a local workload state temporary file in {0}")]
    TemporaryLimit(PathBuf),
    #[error(transparent)]
    Workload(#[from] rey_runtime::WorkloadError),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rey_runtime::{
        BUILT_IN_NORMALIZE_WORKLOAD_ID, WorkloadRunResult, built_in_workload, test_workload,
    };
    use tempfile::TempDir;

    use super::{
        LocalWorkloadState, LocalWorkloadStore, QualificationState, WorkloadCatalog,
        WorkloadCatalogError, WorkloadCatalogKind, WorkloadFreshness, WorkloadOrigin,
        WorkloadSummary,
    };

    const WORKSPACE_PACKAGE: &str =
        include_str!("../../../workloads/portfolio-label-normalization/workload.yaml");

    #[test]
    fn workspace_catalog_loads_exact_admitted_package_and_rejects_incomplete_provenance() {
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
        assert_eq!(
            catalog.workloads[0].provenance.origin,
            WorkloadOrigin::WorkspacePackage
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
    fn creation_request_is_content_identified_and_yields_to_an_admitted_package() {
        let workspace = TempDir::new().unwrap();
        let catalog_dir = std::path::Path::new("workloads");
        let workload_id = "rey.portfolio.label-normalization";
        let created = WorkloadCatalog::create_workspace_request(
            workspace.path(),
            catalog_dir,
            workload_id,
            Some("Portfolio label normalization"),
            Some("Mine and normalize portfolio attention labels"),
        )
        .unwrap();
        let request_path = workspace
            .path()
            .join("workloads/rey.portfolio.label-normalization/request.yaml");
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
        let admitted_catalog =
            WorkloadCatalog::load_workspace(workspace.path(), catalog_dir).unwrap();
        assert_eq!(admitted_catalog.descriptor.workload_count, 1);
        assert_eq!(admitted_catalog.descriptor.admitted_count, 1);
        assert_eq!(admitted_catalog.descriptor.draft_count, 0);
        assert!(admitted_catalog.drafts.is_empty());
        assert_eq!(
            admitted_catalog.workloads[0].definition.workload.id,
            workload_id
        );
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
