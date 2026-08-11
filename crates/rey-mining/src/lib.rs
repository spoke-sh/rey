#![forbid(unsafe_code)]

mod atlas;
mod projection;
mod topography;

pub use atlas::*;
pub use projection::*;
pub use topography::*;

use std::collections::{BTreeMap, BTreeSet};

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MINING_OPERATION_SCHEMA: &str = "rey.mining-operation.v1";
pub const MINING_REQUEST_SCHEMA: &str = "rey.mining-request.v1";
pub const MINING_RESULT_SCHEMA: &str = "rey.mining-result.v1";

trait SemanticName {
    fn semantic_name(&self) -> &'static str;
}

macro_rules! impl_semantic_names {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        impl SemanticName for $name {
            fn semantic_name(&self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }
        }
    };
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MiningFamily {
    Relational,
    Source,
}

impl_semantic_names!(MiningFamily {
    Relational => "relational",
    Source => "source",
});

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MiningOperationKind {
    Retrieve,
    Search,
    Select,
    Filter,
    Join,
    Group,
    Aggregate,
    Transform,
    Align,
    Order,
    Traverse,
    Segment,
    Tokenize,
    Parse,
    Index,
    Measure,
    Compare,
    Summarize,
    Visualize,
}

impl_semantic_names!(MiningOperationKind {
    Retrieve => "retrieve",
    Search => "search",
    Select => "select",
    Filter => "filter",
    Join => "join",
    Group => "group",
    Aggregate => "aggregate",
    Transform => "transform",
    Align => "align",
    Order => "order",
    Traverse => "traverse",
    Segment => "segment",
    Tokenize => "tokenize",
    Parse => "parse",
    Index => "index",
    Measure => "measure",
    Compare => "compare",
    Summarize => "summarize",
    Visualize => "visualize",
});

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MiningExecutionClass {
    ExactRead,
    PureProjection,
    Probe,
}

impl_semantic_names!(MiningExecutionClass {
    ExactRead => "exact_read",
    PureProjection => "pure_projection",
    Probe => "probe",
});

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MiningDeterminism {
    FrozenDeterministic,
    ProviderObserved,
}

impl_semantic_names!(MiningDeterminism {
    FrozenDeterministic => "frozen_deterministic",
    ProviderObserved => "provider_observed",
});

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MiningArtifactKind {
    Native,
    Relation,
    Tree,
    Graph,
    Metric,
    Delta,
    Visualization,
}

impl_semantic_names!(MiningArtifactKind {
    Native => "native",
    Relation => "relation",
    Tree => "tree",
    Graph => "graph",
    Metric => "metric",
    Delta => "delta",
    Visualization => "visualization",
});

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MiningParameterType {
    Bool,
    I64,
    U64,
    Utf8,
    Utf8List,
}

impl_semantic_names!(MiningParameterType {
    Bool => "bool",
    I64 => "i64",
    U64 => "u64",
    Utf8 => "utf8",
    Utf8List => "utf8_list",
});

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum MiningParameterValue {
    Bool(bool),
    I64(i64),
    U64(u64),
    Utf8(String),
    Utf8List(Vec<String>),
}

impl MiningParameterValue {
    #[must_use]
    pub const fn value_type(&self) -> MiningParameterType {
        match self {
            Self::Bool(_) => MiningParameterType::Bool,
            Self::I64(_) => MiningParameterType::I64,
            Self::U64(_) => MiningParameterType::U64,
            Self::Utf8(_) => MiningParameterType::Utf8,
            Self::Utf8List(_) => MiningParameterType::Utf8List,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MiningInvalidation {
    CapabilitySnapshot,
    ProviderRevision,
    ImplementationRevision,
    InputArtifactRevision,
    ParameterChange,
    EffectiveLimitChange,
}

impl_semantic_names!(MiningInvalidation {
    CapabilitySnapshot => "capability_snapshot",
    ProviderRevision => "provider_revision",
    ImplementationRevision => "implementation_revision",
    InputArtifactRevision => "input_artifact_revision",
    ParameterChange => "parameter_change",
    EffectiveLimitChange => "effective_limit_change",
});

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MiningLimits {
    pub max_input_artifacts: u64,
    pub max_output_artifacts: u64,
    pub max_parameters: u64,
    pub max_required_capabilities: u64,
    pub max_rationale_refs: u64,
    pub max_lineage_entries: u64,
    pub max_dependencies: u64,
    pub max_omissions: u64,
    pub max_files: u64,
    pub max_rows: u64,
    pub max_matches: u64,
    pub max_nodes: u64,
    pub max_edges: u64,
    pub max_depth: u64,
    pub max_bytes: u64,
    pub max_string_bytes: u64,
    pub max_time_ms: u64,
}

impl Default for MiningLimits {
    fn default() -> Self {
        Self {
            max_input_artifacts: 32,
            max_output_artifacts: 32,
            max_parameters: 128,
            max_required_capabilities: 128,
            max_rationale_refs: 1_024,
            max_lineage_entries: 128,
            max_dependencies: 4_096,
            max_omissions: 1_024,
            max_files: 1_024,
            max_rows: 100_000,
            max_matches: 100_000,
            max_nodes: 100_000,
            max_edges: 200_000,
            max_depth: 256,
            max_bytes: 64 * 1_024 * 1_024,
            max_string_bytes: 4 * 1_024 * 1_024,
            max_time_ms: 30_000,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MiningArtifactContract {
    pub port_id: String,
    pub kind: MiningArtifactKind,
    pub schema: Option<ContractIdentity>,
    pub media_type: Option<String>,
    pub required: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MiningParameterContract {
    pub name: String,
    pub value_type: MiningParameterType,
    pub required: bool,
    pub default: Option<MiningParameterValue>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MiningOperation {
    pub schema: String,
    pub operation: ContractIdentity,
    pub implementation: ContractIdentity,
    pub family: MiningFamily,
    pub kind: MiningOperationKind,
    pub execution_class: MiningExecutionClass,
    pub determinism: MiningDeterminism,
    pub inputs: Vec<MiningArtifactContract>,
    pub outputs: Vec<MiningArtifactContract>,
    pub parameters: Vec<MiningParameterContract>,
    pub required_capabilities: Vec<ContractIdentity>,
    pub invalidation: Vec<MiningInvalidation>,
    pub limits: MiningLimits,
}

impl MiningOperation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        revision: u64,
        implementation: ContractIdentity,
        family: MiningFamily,
        kind: MiningOperationKind,
        execution_class: MiningExecutionClass,
        determinism: MiningDeterminism,
        mut inputs: Vec<MiningArtifactContract>,
        mut outputs: Vec<MiningArtifactContract>,
        mut parameters: Vec<MiningParameterContract>,
        mut required_capabilities: Vec<ContractIdentity>,
        mut invalidation: Vec<MiningInvalidation>,
        limits: MiningLimits,
    ) -> Result<Self, MiningError> {
        inputs.sort_by(|left, right| left.port_id.cmp(&right.port_id));
        outputs.sort_by(|left, right| left.port_id.cmp(&right.port_id));
        parameters.sort_by(|left, right| left.name.cmp(&right.name));
        required_capabilities.sort_by(contract_order);
        invalidation.sort();
        let mut operation = Self {
            schema: MINING_OPERATION_SCHEMA.to_owned(),
            operation: placeholder_contract(
                id.into(),
                revision,
                "rey.mining-operation.placeholder",
            ),
            implementation,
            family,
            kind,
            execution_class,
            determinism,
            inputs,
            outputs,
            parameters,
            required_capabilities,
            invalidation,
            limits,
        };
        validate_operation(&operation)?;
        operation.operation.semantic_digest = operation_digest(&operation);
        Ok(operation)
    }

    pub fn verify(&self) -> Result<(), MiningError> {
        if self.schema != MINING_OPERATION_SCHEMA {
            return Err(MiningError::UnsupportedSchema {
                kind: "mining operation",
                expected: MINING_OPERATION_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_operation(self)?;
        let actual = operation_digest(self);
        if actual != self.operation.semantic_digest {
            return Err(MiningError::DigestMismatch {
                kind: "mining operation",
                declared: self.operation.semantic_digest.clone(),
                actual,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MiningArtifactRef {
    pub port_id: String,
    pub artifact_id: SemanticDigest,
    pub kind: MiningArtifactKind,
    pub schema: Option<ContractIdentity>,
    pub media_type: String,
    pub provider: ContractIdentity,
    pub source_id: String,
    pub source_revision: String,
    pub logical_bytes: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MiningRationaleKind {
    WorkloadGraph,
    Frontier,
}

impl_semantic_names!(MiningRationaleKind {
    WorkloadGraph => "workload_graph",
    Frontier => "frontier",
});

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MiningRequestContext {
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario: Option<ContractIdentity>,
    pub campaign_id: Option<SemanticDigest>,
    pub space: ContractIdentity,
    pub active_transition_id: Option<SemanticDigest>,
    pub graph_node_id: String,
    pub rationale: MiningRationaleKind,
    pub frontier_row_ids: Vec<SemanticDigest>,
    pub delta_ids: Vec<SemanticDigest>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MiningRequest {
    pub schema: String,
    pub request_id: SemanticDigest,
    pub context: MiningRequestContext,
    pub operation: ContractIdentity,
    pub provider: ContractIdentity,
    pub capability_snapshot_id: SemanticDigest,
    pub inputs: Vec<MiningArtifactRef>,
    pub parameters: BTreeMap<String, MiningParameterValue>,
    pub requested_limits: MiningLimits,
    pub effective_limits: MiningLimits,
}

impl MiningRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mut context: MiningRequestContext,
        operation: &MiningOperation,
        provider: ContractIdentity,
        capability_snapshot_id: SemanticDigest,
        mut inputs: Vec<MiningArtifactRef>,
        mut parameters: BTreeMap<String, MiningParameterValue>,
        requested_limits: MiningLimits,
        effective_limits: MiningLimits,
    ) -> Result<Self, MiningError> {
        operation.verify()?;
        context.frontier_row_ids.sort();
        context.delta_ids.sort();
        inputs.sort_by(|left, right| left.port_id.cmp(&right.port_id));
        resolve_parameters(operation, &mut parameters)?;
        let mut request = Self {
            schema: MINING_REQUEST_SCHEMA.to_owned(),
            request_id: placeholder_digest("rey.mining-request.placeholder"),
            context,
            operation: operation.operation.clone(),
            provider,
            capability_snapshot_id,
            inputs,
            parameters,
            requested_limits,
            effective_limits,
        };
        request.request_id = request_digest(&request);
        request.verify_against(operation)?;
        Ok(request)
    }

    pub fn verify(&self) -> Result<(), MiningError> {
        if self.schema != MINING_REQUEST_SCHEMA {
            return Err(MiningError::UnsupportedSchema {
                kind: "mining request",
                expected: MINING_REQUEST_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_request(self)?;
        let actual = request_digest(self);
        if actual != self.request_id {
            return Err(MiningError::DigestMismatch {
                kind: "mining request",
                declared: self.request_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    pub fn verify_against(&self, operation: &MiningOperation) -> Result<(), MiningError> {
        operation.verify()?;
        self.verify()?;
        if self.operation != operation.operation {
            return Err(MiningError::BindingMismatch("operation"));
        }
        if !limits_fit(&self.effective_limits, &operation.limits) {
            return Err(MiningError::EffectiveLimitsExceed("operation limits"));
        }
        validate_artifact_shape(&self.inputs, &operation.inputs, true)?;
        validate_parameter_shape(&self.parameters, &operation.parameters)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MiningCompleteness {
    Complete,
    Partial,
    Truncated,
    Unsupported,
    Unavailable,
    Failed,
}

impl_semantic_names!(MiningCompleteness {
    Complete => "complete",
    Partial => "partial",
    Truncated => "truncated",
    Unsupported => "unsupported",
    Unavailable => "unavailable",
    Failed => "failed",
});

impl MiningCompleteness {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Truncated => "truncated",
            Self::Unsupported => "unsupported",
            Self::Unavailable => "unavailable",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MiningOmissionKind {
    FileLimit,
    RowLimit,
    MatchLimit,
    NodeLimit,
    EdgeLimit,
    DepthLimit,
    ByteLimit,
    TimeLimit,
    ProviderUnavailable,
    Unsupported,
    ExecutionFailed,
    SourceDrift,
    MalformedInput,
}

impl_semantic_names!(MiningOmissionKind {
    FileLimit => "file_limit",
    RowLimit => "row_limit",
    MatchLimit => "match_limit",
    NodeLimit => "node_limit",
    EdgeLimit => "edge_limit",
    DepthLimit => "depth_limit",
    ByteLimit => "byte_limit",
    TimeLimit => "time_limit",
    ProviderUnavailable => "provider_unavailable",
    Unsupported => "unsupported",
    ExecutionFailed => "execution_failed",
    SourceDrift => "source_drift",
    MalformedInput => "malformed_input",
});

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct MiningOmission {
    pub kind: MiningOmissionKind,
    pub subject_id: Option<String>,
    pub omitted_count: u64,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MiningLineageKind {
    Implementation,
    Provider,
    Tool,
    Query,
    Parser,
    Run,
    Capture,
}

impl_semantic_names!(MiningLineageKind {
    Implementation => "implementation",
    Provider => "provider",
    Tool => "tool",
    Query => "query",
    Parser => "parser",
    Run => "run",
    Capture => "capture",
});

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MiningLineage {
    pub kind: MiningLineageKind,
    pub identity: ContractIdentity,
    pub execution_id: Option<SemanticDigest>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MiningDependencyKind {
    Request,
    CapabilitySnapshot,
    InputArtifact,
    SourceRevision,
    ProviderRevision,
    ImplementationRevision,
    ParameterSet,
    EffectiveLimits,
}

impl_semantic_names!(MiningDependencyKind {
    Request => "request",
    CapabilitySnapshot => "capability_snapshot",
    InputArtifact => "input_artifact",
    SourceRevision => "source_revision",
    ProviderRevision => "provider_revision",
    ImplementationRevision => "implementation_revision",
    ParameterSet => "parameter_set",
    EffectiveLimits => "effective_limits",
});

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct MiningDependencyEdge {
    pub artifact_id: SemanticDigest,
    pub kind: MiningDependencyKind,
    pub dependency_id: SemanticDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MiningConsumption {
    pub files: u64,
    pub rows: u64,
    pub matches: u64,
    pub nodes: u64,
    pub edges: u64,
    pub depth: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub observed_time_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MiningResult {
    pub schema: String,
    pub result_id: SemanticDigest,
    pub request_id: SemanticDigest,
    pub operation: ContractIdentity,
    pub provider: ContractIdentity,
    pub capability_snapshot_id: SemanticDigest,
    pub completeness: MiningCompleteness,
    pub outputs: Vec<MiningArtifactRef>,
    pub lineage: Vec<MiningLineage>,
    pub dependencies: Vec<MiningDependencyEdge>,
    pub omissions: Vec<MiningOmission>,
    pub consumption: MiningConsumption,
}

impl MiningResult {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request: &MiningRequest,
        operation: &MiningOperation,
        completeness: MiningCompleteness,
        mut outputs: Vec<MiningArtifactRef>,
        mut lineage: Vec<MiningLineage>,
        mut dependencies: Vec<MiningDependencyEdge>,
        mut omissions: Vec<MiningOmission>,
        consumption: MiningConsumption,
    ) -> Result<Self, MiningError> {
        request.verify_against(operation)?;
        outputs.sort_by(|left, right| left.port_id.cmp(&right.port_id));
        lineage.sort_by(lineage_order);
        for output in &outputs {
            dependencies.extend([
                MiningDependencyEdge {
                    artifact_id: output.artifact_id.clone(),
                    kind: MiningDependencyKind::Request,
                    dependency_id: request.request_id.clone(),
                },
                MiningDependencyEdge {
                    artifact_id: output.artifact_id.clone(),
                    kind: MiningDependencyKind::CapabilitySnapshot,
                    dependency_id: request.capability_snapshot_id.clone(),
                },
                MiningDependencyEdge {
                    artifact_id: output.artifact_id.clone(),
                    kind: MiningDependencyKind::ProviderRevision,
                    dependency_id: request.provider.semantic_digest.clone(),
                },
                MiningDependencyEdge {
                    artifact_id: output.artifact_id.clone(),
                    kind: MiningDependencyKind::ProviderRevision,
                    dependency_id: output.provider.semantic_digest.clone(),
                },
                MiningDependencyEdge {
                    artifact_id: output.artifact_id.clone(),
                    kind: MiningDependencyKind::ImplementationRevision,
                    dependency_id: operation.implementation.semantic_digest.clone(),
                },
            ]);
        }
        dependencies.sort();
        dependencies.dedup();
        omissions.sort();
        let mut result = Self {
            schema: MINING_RESULT_SCHEMA.to_owned(),
            result_id: placeholder_digest("rey.mining-result.placeholder"),
            request_id: request.request_id.clone(),
            operation: request.operation.clone(),
            provider: request.provider.clone(),
            capability_snapshot_id: request.capability_snapshot_id.clone(),
            completeness,
            outputs,
            lineage,
            dependencies,
            omissions,
            consumption,
        };
        result.result_id = result_digest(&result);
        result.verify_against(request, operation)?;
        Ok(result)
    }

    pub fn verify(&self) -> Result<(), MiningError> {
        if self.schema != MINING_RESULT_SCHEMA {
            return Err(MiningError::UnsupportedSchema {
                kind: "mining result",
                expected: MINING_RESULT_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_result(self)?;
        let actual = result_digest(self);
        if actual != self.result_id {
            return Err(MiningError::DigestMismatch {
                kind: "mining result",
                declared: self.result_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    pub fn verify_against(
        &self,
        request: &MiningRequest,
        operation: &MiningOperation,
    ) -> Result<(), MiningError> {
        request.verify_against(operation)?;
        self.verify()?;
        if self.request_id != request.request_id {
            return Err(MiningError::BindingMismatch("request"));
        }
        if self.operation != request.operation {
            return Err(MiningError::BindingMismatch("operation"));
        }
        if self.provider != request.provider {
            return Err(MiningError::BindingMismatch("provider"));
        }
        if self.capability_snapshot_id != request.capability_snapshot_id {
            return Err(MiningError::BindingMismatch("capability snapshot"));
        }
        validate_artifact_shape(
            &self.outputs,
            &operation.outputs,
            self.completeness == MiningCompleteness::Complete,
        )?;
        validate_artifact_refs(
            &self.outputs,
            request.effective_limits.max_output_artifacts,
            request.effective_limits.max_bytes,
            "result outputs",
        )?;
        enforce_count(
            "result lineage",
            self.lineage.len(),
            request.effective_limits.max_lineage_entries,
        )?;
        enforce_count(
            "result dependencies",
            self.dependencies.len(),
            request.effective_limits.max_dependencies,
        )?;
        enforce_count(
            "result omissions",
            self.omissions.len(),
            request.effective_limits.max_omissions,
        )?;
        validate_string_budget(
            "result string byte",
            result_string_bytes(self)?,
            request.effective_limits.max_string_bytes,
        )?;
        validate_required_dependencies(self, request, operation)?;
        validate_consumption(&self.consumption, &request.effective_limits)?;
        let implementation_required = !matches!(
            self.completeness,
            MiningCompleteness::Unsupported | MiningCompleteness::Unavailable
        );
        if implementation_required
            && !self.lineage.iter().any(|entry| {
                entry.kind == MiningLineageKind::Implementation
                    && entry.identity == operation.implementation
            })
        {
            return Err(MiningError::MissingImplementationLineage);
        }
        Ok(())
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum MiningError {
    #[error("unsupported {kind} schema: expected {expected}, got {actual}")]
    UnsupportedSchema {
        kind: &'static str,
        expected: &'static str,
        actual: String,
    },
    #[error("invalid text in {0}")]
    InvalidText(&'static str),
    #[error("{0} revision must be non-zero")]
    ZeroRevision(&'static str),
    #[error("invalid semantic digest {0}")]
    InvalidDigest(String),
    #[error("mining limit {0} must be non-zero")]
    ZeroLimit(&'static str),
    #[error("{kind} limit exceeded: limit {limit}, observed {observed}")]
    Limit {
        kind: &'static str,
        limit: u64,
        observed: u64,
    },
    #[error("duplicate {0}")]
    Duplicate(&'static str),
    #[error("non-canonical {0}")]
    NonCanonical(&'static str),
    #[error("a mining operation must declare at least one output")]
    EmptyOutputs,
    #[error("a mining operation must declare invalidation inputs")]
    EmptyInvalidation,
    #[error("mining operation is missing the {0:?} invalidation input")]
    MissingInvalidation(MiningInvalidation),
    #[error("parameter {0} has a default with the wrong type")]
    DefaultType(String),
    #[error("required parameter {0} cannot also have a default")]
    RequiredDefault(String),
    #[error("request must bind a scenario or campaign")]
    MissingScenarioOrCampaign,
    #[error("frontier-directed rationale needs a frontier row or delta")]
    EmptyFrontierRationale,
    #[error("effective limits exceed {0}")]
    EffectiveLimitsExceed(&'static str),
    #[error("unknown artifact port {0}")]
    UnknownArtifactPort(String),
    #[error("missing required artifact port {0}")]
    MissingArtifactPort(String),
    #[error("artifact does not satisfy port contract {0}")]
    ArtifactContract(String),
    #[error("structured artifact port {0} must declare an exact schema")]
    MissingArtifactSchema(String),
    #[error("unknown mining parameter {0}")]
    UnknownParameter(String),
    #[error("missing required mining parameter {0}")]
    MissingParameter(String),
    #[error("mining parameter {0} has the wrong type")]
    ParameterType(String),
    #[error("invalid completeness, artifacts, and omissions shape")]
    CompletenessShape,
    #[error("invalid mining omission")]
    InvalidOmission,
    #[error("missing exact mining implementation lineage")]
    MissingImplementationLineage,
    #[error("dependency edge names an artifact not produced by this result")]
    UnknownDependencyArtifact,
    #[error("resource counter overflow")]
    CountOverflow,
    #[error("{0} binding does not match")]
    BindingMismatch(&'static str),
    #[error("{kind} digest mismatch: declared {declared}, actual {actual}")]
    DigestMismatch {
        kind: &'static str,
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
}

fn validate_operation(operation: &MiningOperation) -> Result<(), MiningError> {
    validate_contract("operation", &operation.operation)?;
    validate_contract("implementation", &operation.implementation)?;
    validate_limits(&operation.limits)?;
    if operation.outputs.is_empty() {
        return Err(MiningError::EmptyOutputs);
    }
    if operation.invalidation.is_empty() {
        return Err(MiningError::EmptyInvalidation);
    }
    enforce_count(
        "operation input artifact",
        operation.inputs.len(),
        operation.limits.max_input_artifacts,
    )?;
    enforce_count(
        "operation output artifact",
        operation.outputs.len(),
        operation.limits.max_output_artifacts,
    )?;
    enforce_count(
        "operation parameter",
        operation.parameters.len(),
        operation.limits.max_parameters,
    )?;
    enforce_count(
        "operation required capability",
        operation.required_capabilities.len(),
        operation.limits.max_required_capabilities,
    )?;
    validate_artifact_contracts(&operation.inputs, "input artifact port")?;
    validate_artifact_contracts(&operation.outputs, "output artifact port")?;
    validate_parameter_contracts(&operation.parameters)?;
    for capability in &operation.required_capabilities {
        validate_contract("required capability", capability)?;
    }
    if operation
        .required_capabilities
        .windows(2)
        .any(|window| contract_order(&window[0], &window[1]).is_ge())
    {
        return Err(MiningError::NonCanonical("required capabilities"));
    }
    if operation
        .invalidation
        .windows(2)
        .any(|window| window[0] >= window[1])
    {
        return Err(MiningError::NonCanonical("invalidation inputs"));
    }
    for required in [
        MiningInvalidation::CapabilitySnapshot,
        MiningInvalidation::ProviderRevision,
        MiningInvalidation::ImplementationRevision,
        MiningInvalidation::InputArtifactRevision,
        MiningInvalidation::ParameterChange,
        MiningInvalidation::EffectiveLimitChange,
    ] {
        if operation.invalidation.binary_search(&required).is_err() {
            return Err(MiningError::MissingInvalidation(required));
        }
    }
    validate_string_budget(
        "operation string byte",
        operation_string_bytes(operation)?,
        operation.limits.max_string_bytes,
    )?;
    Ok(())
}

fn validate_request(request: &MiningRequest) -> Result<(), MiningError> {
    for contract in [
        &request.context.workload,
        &request.context.graph,
        &request.context.space,
        &request.operation,
        &request.provider,
    ] {
        validate_contract("request contract", contract)?;
    }
    if let Some(scenario) = &request.context.scenario {
        validate_contract("scenario", scenario)?;
    }
    if request.context.scenario.is_none() && request.context.campaign_id.is_none() {
        return Err(MiningError::MissingScenarioOrCampaign);
    }
    validate_optional_digest(request.context.campaign_id.as_ref())?;
    validate_optional_digest(request.context.active_transition_id.as_ref())?;
    validate_text("graph node id", &request.context.graph_node_id)?;
    validate_digest(&request.capability_snapshot_id)?;
    validate_limits(&request.requested_limits)?;
    validate_limits(&request.effective_limits)?;
    if !limits_fit(&request.effective_limits, &request.requested_limits) {
        return Err(MiningError::EffectiveLimitsExceed("requested limits"));
    }
    validate_digest_list(&request.context.frontier_row_ids, "frontier rows")?;
    validate_digest_list(&request.context.delta_ids, "delta ids")?;
    let rationale_refs = request
        .context
        .frontier_row_ids
        .len()
        .checked_add(request.context.delta_ids.len())
        .ok_or(MiningError::CountOverflow)?;
    enforce_count(
        "request rationale reference",
        rationale_refs,
        request.effective_limits.max_rationale_refs,
    )?;
    if request.context.rationale == MiningRationaleKind::Frontier
        && request.context.frontier_row_ids.is_empty()
        && request.context.delta_ids.is_empty()
    {
        return Err(MiningError::EmptyFrontierRationale);
    }
    validate_artifact_refs(
        &request.inputs,
        request.effective_limits.max_input_artifacts,
        request.effective_limits.max_bytes,
        "request input artifacts",
    )?;
    validate_parameter_values(
        &request.parameters,
        request.effective_limits.max_string_bytes,
    )?;
    enforce_count(
        "request parameter",
        request.parameters.len(),
        request.effective_limits.max_parameters,
    )?;
    validate_string_budget(
        "request string byte",
        request_string_bytes(request)?,
        request.effective_limits.max_string_bytes,
    )?;
    Ok(())
}

fn validate_result(result: &MiningResult) -> Result<(), MiningError> {
    for contract in [&result.operation, &result.provider] {
        validate_contract("result contract", contract)?;
    }
    for digest in [
        &result.request_id,
        &result.capability_snapshot_id,
        &result.result_id,
    ] {
        validate_digest(digest)?;
    }
    validate_artifact_refs(&result.outputs, u64::MAX, u64::MAX, "result outputs")?;
    validate_lineage(&result.lineage)?;
    validate_dependencies(&result.dependencies, &result.outputs)?;
    validate_omissions(&result.omissions)?;
    let complete = result.completeness == MiningCompleteness::Complete;
    let productive = matches!(
        result.completeness,
        MiningCompleteness::Partial | MiningCompleteness::Truncated
    );
    let terminal = matches!(
        result.completeness,
        MiningCompleteness::Unsupported
            | MiningCompleteness::Unavailable
            | MiningCompleteness::Failed
    );
    if (complete && (!result.omissions.is_empty() || result.outputs.is_empty()))
        || (productive && (result.omissions.is_empty() || result.outputs.is_empty()))
        || (terminal && (result.omissions.is_empty() || !result.outputs.is_empty()))
    {
        return Err(MiningError::CompletenessShape);
    }
    let has_omission = |kind| {
        result
            .omissions
            .iter()
            .any(|omission| omission.kind == kind)
    };
    let state_reason_is_legal = match result.completeness {
        MiningCompleteness::Complete | MiningCompleteness::Partial => true,
        MiningCompleteness::Truncated => result.omissions.iter().any(|omission| {
            matches!(
                omission.kind,
                MiningOmissionKind::FileLimit
                    | MiningOmissionKind::RowLimit
                    | MiningOmissionKind::MatchLimit
                    | MiningOmissionKind::NodeLimit
                    | MiningOmissionKind::EdgeLimit
                    | MiningOmissionKind::DepthLimit
                    | MiningOmissionKind::ByteLimit
                    | MiningOmissionKind::TimeLimit
            )
        }),
        MiningCompleteness::Unsupported => has_omission(MiningOmissionKind::Unsupported),
        MiningCompleteness::Unavailable => has_omission(MiningOmissionKind::ProviderUnavailable),
        MiningCompleteness::Failed => result.omissions.iter().any(|omission| {
            matches!(
                omission.kind,
                MiningOmissionKind::ExecutionFailed
                    | MiningOmissionKind::SourceDrift
                    | MiningOmissionKind::MalformedInput
            )
        }),
    };
    if !state_reason_is_legal {
        return Err(MiningError::CompletenessShape);
    }
    Ok(())
}

fn validate_artifact_contracts(
    contracts: &[MiningArtifactContract],
    field: &'static str,
) -> Result<(), MiningError> {
    for contract in contracts {
        validate_text(field, &contract.port_id)?;
        if contract.kind != MiningArtifactKind::Native && contract.schema.is_none() {
            return Err(MiningError::MissingArtifactSchema(contract.port_id.clone()));
        }
        if let Some(schema) = &contract.schema {
            validate_contract("artifact schema", schema)?;
        }
        if let Some(media_type) = &contract.media_type {
            validate_text("artifact media type", media_type)?;
        }
    }
    if contracts
        .windows(2)
        .any(|window| window[0].port_id >= window[1].port_id)
    {
        return Err(MiningError::NonCanonical(field));
    }
    Ok(())
}

fn validate_parameter_contracts(contracts: &[MiningParameterContract]) -> Result<(), MiningError> {
    for contract in contracts {
        validate_text("parameter name", &contract.name)?;
        if contract.required && contract.default.is_some() {
            return Err(MiningError::RequiredDefault(contract.name.clone()));
        }
        if contract
            .default
            .as_ref()
            .is_some_and(|value| value.value_type() != contract.value_type)
        {
            return Err(MiningError::DefaultType(contract.name.clone()));
        }
        if let Some(default) = &contract.default {
            validate_parameter_text(default)?;
        }
    }
    if contracts
        .windows(2)
        .any(|window| window[0].name >= window[1].name)
    {
        return Err(MiningError::NonCanonical("parameter contracts"));
    }
    Ok(())
}

fn resolve_parameters(
    operation: &MiningOperation,
    values: &mut BTreeMap<String, MiningParameterValue>,
) -> Result<(), MiningError> {
    for contract in &operation.parameters {
        if !values.contains_key(&contract.name) {
            if let Some(default) = &contract.default {
                values.insert(contract.name.clone(), default.clone());
            } else if contract.required {
                return Err(MiningError::MissingParameter(contract.name.clone()));
            }
        }
    }
    validate_parameter_shape(values, &operation.parameters)
}

fn validate_parameter_shape(
    values: &BTreeMap<String, MiningParameterValue>,
    contracts: &[MiningParameterContract],
) -> Result<(), MiningError> {
    let by_name = contracts
        .iter()
        .map(|contract| (contract.name.as_str(), contract))
        .collect::<BTreeMap<_, _>>();
    for (name, value) in values {
        let Some(contract) = by_name.get(name.as_str()) else {
            return Err(MiningError::UnknownParameter(name.clone()));
        };
        if value.value_type() != contract.value_type {
            return Err(MiningError::ParameterType(name.clone()));
        }
    }
    if let Some(missing) = contracts
        .iter()
        .find(|contract| contract.required && !values.contains_key(&contract.name))
    {
        return Err(MiningError::MissingParameter(missing.name.clone()));
    }
    Ok(())
}

fn validate_parameter_values(
    values: &BTreeMap<String, MiningParameterValue>,
    max_string_bytes: u64,
) -> Result<(), MiningError> {
    let mut bytes = 0_u64;
    for (name, value) in values {
        validate_text("parameter name", name)?;
        add_bytes(&mut bytes, name)?;
        validate_parameter_text(value)?;
        add_parameter_string_bytes(&mut bytes, value)?;
    }
    if bytes > max_string_bytes {
        return Err(MiningError::Limit {
            kind: "parameter string byte",
            limit: max_string_bytes,
            observed: bytes,
        });
    }
    Ok(())
}

fn validate_parameter_text(value: &MiningParameterValue) -> Result<(), MiningError> {
    match value {
        MiningParameterValue::Utf8(_) | MiningParameterValue::Utf8List(_) => Ok(()),
        MiningParameterValue::Bool(_)
        | MiningParameterValue::I64(_)
        | MiningParameterValue::U64(_) => Ok(()),
    }
}

fn validate_artifact_shape(
    artifacts: &[MiningArtifactRef],
    contracts: &[MiningArtifactContract],
    enforce_required: bool,
) -> Result<(), MiningError> {
    let by_port = contracts
        .iter()
        .map(|contract| (contract.port_id.as_str(), contract))
        .collect::<BTreeMap<_, _>>();
    for artifact in artifacts {
        let Some(contract) = by_port.get(artifact.port_id.as_str()) else {
            return Err(MiningError::UnknownArtifactPort(artifact.port_id.clone()));
        };
        if artifact.kind != contract.kind
            || contract
                .schema
                .as_ref()
                .is_some_and(|schema| artifact.schema.as_ref() != Some(schema))
            || contract
                .media_type
                .as_ref()
                .is_some_and(|media_type| artifact.media_type != *media_type)
        {
            return Err(MiningError::ArtifactContract(artifact.port_id.clone()));
        }
    }
    if let Some(missing) = contracts.iter().find(|contract| {
        enforce_required
            && contract.required
            && !artifacts
                .iter()
                .any(|artifact| artifact.port_id == contract.port_id)
    }) {
        return Err(MiningError::MissingArtifactPort(missing.port_id.clone()));
    }
    Ok(())
}

fn validate_artifact_refs(
    artifacts: &[MiningArtifactRef],
    max_count: u64,
    max_bytes: u64,
    field: &'static str,
) -> Result<(), MiningError> {
    enforce_count(field, artifacts.len(), max_count)?;
    let mut bytes = 0_u64;
    for artifact in artifacts {
        validate_text("artifact port", &artifact.port_id)?;
        validate_digest(&artifact.artifact_id)?;
        validate_contract("artifact provider", &artifact.provider)?;
        if artifact.kind != MiningArtifactKind::Native && artifact.schema.is_none() {
            return Err(MiningError::MissingArtifactSchema(artifact.port_id.clone()));
        }
        if let Some(schema) = &artifact.schema {
            validate_contract("artifact schema", schema)?;
        }
        validate_text("artifact media type", &artifact.media_type)?;
        validate_text("artifact source", &artifact.source_id)?;
        validate_text("artifact source revision", &artifact.source_revision)?;
        bytes = bytes
            .checked_add(artifact.logical_bytes)
            .ok_or(MiningError::CountOverflow)?;
    }
    if artifacts
        .windows(2)
        .any(|window| window[0].port_id >= window[1].port_id)
    {
        return Err(MiningError::NonCanonical(field));
    }
    if bytes > max_bytes {
        return Err(MiningError::Limit {
            kind: "artifact byte",
            limit: max_bytes,
            observed: bytes,
        });
    }
    Ok(())
}

fn validate_lineage(lineage: &[MiningLineage]) -> Result<(), MiningError> {
    for entry in lineage {
        validate_contract("lineage identity", &entry.identity)?;
        validate_optional_digest(entry.execution_id.as_ref())?;
    }
    if lineage
        .windows(2)
        .any(|window| lineage_order(&window[0], &window[1]).is_ge())
    {
        return Err(MiningError::NonCanonical("lineage"));
    }
    Ok(())
}

fn validate_dependencies(
    dependencies: &[MiningDependencyEdge],
    outputs: &[MiningArtifactRef],
) -> Result<(), MiningError> {
    let output_ids = outputs
        .iter()
        .map(|artifact| &artifact.artifact_id)
        .collect::<BTreeSet<_>>();
    for edge in dependencies {
        validate_digest(&edge.artifact_id)?;
        validate_digest(&edge.dependency_id)?;
        if !output_ids.contains(&edge.artifact_id) {
            return Err(MiningError::UnknownDependencyArtifact);
        }
    }
    if dependencies.windows(2).any(|window| window[0] >= window[1]) {
        return Err(MiningError::NonCanonical("dependency edges"));
    }
    Ok(())
}

fn validate_required_dependencies(
    result: &MiningResult,
    request: &MiningRequest,
    operation: &MiningOperation,
) -> Result<(), MiningError> {
    for output in &result.outputs {
        let required = [
            (MiningDependencyKind::Request, request.request_id.clone()),
            (
                MiningDependencyKind::CapabilitySnapshot,
                request.capability_snapshot_id.clone(),
            ),
            (
                MiningDependencyKind::ProviderRevision,
                request.provider.semantic_digest.clone(),
            ),
            (
                MiningDependencyKind::ProviderRevision,
                output.provider.semantic_digest.clone(),
            ),
            (
                MiningDependencyKind::ImplementationRevision,
                operation.implementation.semantic_digest.clone(),
            ),
        ];
        for (kind, dependency_id) in required {
            if !result.dependencies.iter().any(|edge| {
                edge.artifact_id == output.artifact_id
                    && edge.kind == kind
                    && edge.dependency_id == dependency_id
            }) {
                return Err(MiningError::BindingMismatch("artifact dependency"));
            }
        }
    }
    Ok(())
}

fn validate_omissions(omissions: &[MiningOmission]) -> Result<(), MiningError> {
    for omission in omissions {
        if omission.omitted_count == 0 {
            return Err(MiningError::InvalidOmission);
        }
        if let Some(subject) = &omission.subject_id {
            validate_text("omission subject", subject)?;
        }
        validate_text("omission reason", &omission.reason)?;
    }
    if omissions.windows(2).any(|window| window[0] >= window[1]) {
        return Err(MiningError::NonCanonical("omissions"));
    }
    Ok(())
}

fn validate_consumption(
    consumption: &MiningConsumption,
    limits: &MiningLimits,
) -> Result<(), MiningError> {
    for (kind, observed, limit) in [
        ("file", consumption.files, limits.max_files),
        ("row", consumption.rows, limits.max_rows),
        ("match", consumption.matches, limits.max_matches),
        ("node", consumption.nodes, limits.max_nodes),
        ("edge", consumption.edges, limits.max_edges),
        ("depth", consumption.depth, limits.max_depth),
    ] {
        if observed > limit {
            return Err(MiningError::Limit {
                kind,
                limit,
                observed,
            });
        }
    }
    if let Some(observed) = consumption.observed_time_ms
        && observed > limits.max_time_ms
    {
        return Err(MiningError::Limit {
            kind: "observed time",
            limit: limits.max_time_ms,
            observed,
        });
    }
    let bytes = consumption
        .bytes_read
        .checked_add(consumption.bytes_written)
        .ok_or(MiningError::CountOverflow)?;
    if bytes > limits.max_bytes {
        return Err(MiningError::Limit {
            kind: "consumed byte",
            limit: limits.max_bytes,
            observed: bytes,
        });
    }
    Ok(())
}

fn validate_limits(limits: &MiningLimits) -> Result<(), MiningError> {
    let values = [
        ("max_input_artifacts", limits.max_input_artifacts),
        ("max_output_artifacts", limits.max_output_artifacts),
        ("max_parameters", limits.max_parameters),
        (
            "max_required_capabilities",
            limits.max_required_capabilities,
        ),
        ("max_rationale_refs", limits.max_rationale_refs),
        ("max_lineage_entries", limits.max_lineage_entries),
        ("max_dependencies", limits.max_dependencies),
        ("max_omissions", limits.max_omissions),
        ("max_files", limits.max_files),
        ("max_rows", limits.max_rows),
        ("max_matches", limits.max_matches),
        ("max_nodes", limits.max_nodes),
        ("max_edges", limits.max_edges),
        ("max_depth", limits.max_depth),
        ("max_bytes", limits.max_bytes),
        ("max_string_bytes", limits.max_string_bytes),
        ("max_time_ms", limits.max_time_ms),
    ];
    if let Some((name, _)) = values.into_iter().find(|(_, value)| *value == 0) {
        return Err(MiningError::ZeroLimit(name));
    }
    Ok(())
}

fn limits_fit(inner: &MiningLimits, outer: &MiningLimits) -> bool {
    inner.max_input_artifacts <= outer.max_input_artifacts
        && inner.max_output_artifacts <= outer.max_output_artifacts
        && inner.max_parameters <= outer.max_parameters
        && inner.max_required_capabilities <= outer.max_required_capabilities
        && inner.max_rationale_refs <= outer.max_rationale_refs
        && inner.max_lineage_entries <= outer.max_lineage_entries
        && inner.max_dependencies <= outer.max_dependencies
        && inner.max_omissions <= outer.max_omissions
        && inner.max_files <= outer.max_files
        && inner.max_rows <= outer.max_rows
        && inner.max_matches <= outer.max_matches
        && inner.max_nodes <= outer.max_nodes
        && inner.max_edges <= outer.max_edges
        && inner.max_depth <= outer.max_depth
        && inner.max_bytes <= outer.max_bytes
        && inner.max_string_bytes <= outer.max_string_bytes
        && inner.max_time_ms <= outer.max_time_ms
}

fn validate_contract(field: &'static str, contract: &ContractIdentity) -> Result<(), MiningError> {
    validate_text(field, &contract.id)?;
    if contract.revision == 0 {
        return Err(MiningError::ZeroRevision(field));
    }
    validate_digest(&contract.semantic_digest)
}

fn validate_text(field: &'static str, value: &str) -> Result<(), MiningError> {
    if value.is_empty() || value.len() > 4_096 || value.chars().any(char::is_control) {
        return Err(MiningError::InvalidText(field));
    }
    Ok(())
}

fn validate_digest(digest: &SemanticDigest) -> Result<(), MiningError> {
    let value = digest.as_str();
    let Some(hex) = value.strip_prefix("blake3:") else {
        return Err(MiningError::InvalidDigest(value.to_owned()));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(MiningError::InvalidDigest(value.to_owned()));
    }
    Ok(())
}

fn validate_optional_digest(digest: Option<&SemanticDigest>) -> Result<(), MiningError> {
    if let Some(digest) = digest {
        validate_digest(digest)?;
    }
    Ok(())
}

fn validate_digest_list(
    digests: &[SemanticDigest],
    field: &'static str,
) -> Result<(), MiningError> {
    for digest in digests {
        validate_digest(digest)?;
    }
    if digests.windows(2).any(|window| window[0] >= window[1]) {
        return Err(MiningError::NonCanonical(field));
    }
    Ok(())
}

fn enforce_count(kind: &'static str, observed: usize, limit: u64) -> Result<(), MiningError> {
    if observed as u64 > limit {
        return Err(MiningError::Limit {
            kind,
            limit,
            observed: observed as u64,
        });
    }
    Ok(())
}

fn add_bytes(total: &mut u64, value: &str) -> Result<(), MiningError> {
    *total = total
        .checked_add(value.len() as u64)
        .ok_or(MiningError::CountOverflow)?;
    Ok(())
}

fn validate_string_budget(
    kind: &'static str,
    observed: u64,
    limit: u64,
) -> Result<(), MiningError> {
    if observed > limit {
        Err(MiningError::Limit {
            kind,
            limit,
            observed,
        })
    } else {
        Ok(())
    }
}

fn operation_string_bytes(operation: &MiningOperation) -> Result<u64, MiningError> {
    let mut total = 0_u64;
    add_contract_string_bytes(&mut total, &operation.operation)?;
    add_contract_string_bytes(&mut total, &operation.implementation)?;
    add_artifact_contract_string_bytes(&mut total, &operation.inputs)?;
    add_artifact_contract_string_bytes(&mut total, &operation.outputs)?;
    for parameter in &operation.parameters {
        add_bytes(&mut total, &parameter.name)?;
        if let Some(default) = &parameter.default {
            add_parameter_string_bytes(&mut total, default)?;
        }
    }
    for capability in &operation.required_capabilities {
        add_contract_string_bytes(&mut total, capability)?;
    }
    Ok(total)
}

fn request_string_bytes(request: &MiningRequest) -> Result<u64, MiningError> {
    let mut total = 0_u64;
    for contract in [
        &request.context.workload,
        &request.context.graph,
        &request.context.space,
        &request.operation,
        &request.provider,
    ] {
        add_contract_string_bytes(&mut total, contract)?;
    }
    if let Some(scenario) = &request.context.scenario {
        add_contract_string_bytes(&mut total, scenario)?;
    }
    for digest in request
        .context
        .campaign_id
        .iter()
        .chain(request.context.active_transition_id.iter())
        .chain(request.context.frontier_row_ids.iter())
        .chain(request.context.delta_ids.iter())
    {
        add_bytes(&mut total, digest.as_str())?;
    }
    add_bytes(&mut total, &request.context.graph_node_id)?;
    add_bytes(&mut total, request.capability_snapshot_id.as_str())?;
    add_artifact_ref_string_bytes(&mut total, &request.inputs)?;
    for (name, value) in &request.parameters {
        add_bytes(&mut total, name)?;
        add_parameter_string_bytes(&mut total, value)?;
    }
    Ok(total)
}

fn result_string_bytes(result: &MiningResult) -> Result<u64, MiningError> {
    let mut total = 0_u64;
    for digest in [
        &result.result_id,
        &result.request_id,
        &result.capability_snapshot_id,
    ] {
        add_bytes(&mut total, digest.as_str())?;
    }
    add_contract_string_bytes(&mut total, &result.operation)?;
    add_contract_string_bytes(&mut total, &result.provider)?;
    add_artifact_ref_string_bytes(&mut total, &result.outputs)?;
    for entry in &result.lineage {
        add_contract_string_bytes(&mut total, &entry.identity)?;
        if let Some(execution_id) = &entry.execution_id {
            add_bytes(&mut total, execution_id.as_str())?;
        }
    }
    for edge in &result.dependencies {
        add_bytes(&mut total, edge.artifact_id.as_str())?;
        add_bytes(&mut total, edge.dependency_id.as_str())?;
    }
    for omission in &result.omissions {
        if let Some(subject) = &omission.subject_id {
            add_bytes(&mut total, subject)?;
        }
        add_bytes(&mut total, &omission.reason)?;
    }
    Ok(total)
}

fn add_contract_string_bytes(
    total: &mut u64,
    contract: &ContractIdentity,
) -> Result<(), MiningError> {
    add_bytes(total, &contract.id)?;
    add_bytes(total, contract.semantic_digest.as_str())
}

fn add_artifact_contract_string_bytes(
    total: &mut u64,
    contracts: &[MiningArtifactContract],
) -> Result<(), MiningError> {
    for contract in contracts {
        add_bytes(total, &contract.port_id)?;
        if let Some(schema) = &contract.schema {
            add_contract_string_bytes(total, schema)?;
        }
        if let Some(media_type) = &contract.media_type {
            add_bytes(total, media_type)?;
        }
    }
    Ok(())
}

fn add_artifact_ref_string_bytes(
    total: &mut u64,
    artifacts: &[MiningArtifactRef],
) -> Result<(), MiningError> {
    for artifact in artifacts {
        add_bytes(total, &artifact.port_id)?;
        add_bytes(total, artifact.artifact_id.as_str())?;
        if let Some(schema) = &artifact.schema {
            add_contract_string_bytes(total, schema)?;
        }
        add_bytes(total, &artifact.media_type)?;
        add_contract_string_bytes(total, &artifact.provider)?;
        add_bytes(total, &artifact.source_id)?;
        add_bytes(total, &artifact.source_revision)?;
    }
    Ok(())
}

fn add_parameter_string_bytes(
    total: &mut u64,
    value: &MiningParameterValue,
) -> Result<(), MiningError> {
    match value {
        MiningParameterValue::Utf8(value) => add_bytes(total, value),
        MiningParameterValue::Utf8List(values) => {
            for value in values {
                add_bytes(total, value)?;
            }
            Ok(())
        }
        MiningParameterValue::Bool(_)
        | MiningParameterValue::I64(_)
        | MiningParameterValue::U64(_) => Ok(()),
    }
}

fn contract_order(left: &ContractIdentity, right: &ContractIdentity) -> std::cmp::Ordering {
    (&left.id, left.revision, &left.semantic_digest).cmp(&(
        &right.id,
        right.revision,
        &right.semantic_digest,
    ))
}

fn lineage_order(left: &MiningLineage, right: &MiningLineage) -> std::cmp::Ordering {
    left.kind
        .cmp(&right.kind)
        .then_with(|| contract_order(&left.identity, &right.identity))
        .then_with(|| left.execution_id.cmp(&right.execution_id))
}

fn placeholder_contract(id: String, revision: u64, domain: &str) -> ContractIdentity {
    ContractIdentity {
        id,
        revision,
        semantic_digest: placeholder_digest(domain),
    }
}

fn placeholder_digest(domain: &str) -> SemanticDigest {
    SemanticHasher::new(domain).finish()
}

fn operation_digest(operation: &MiningOperation) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(MINING_OPERATION_SCHEMA);
    hasher.add_str(&operation.operation.id);
    hasher.add_u64(operation.operation.revision);
    operation.implementation.add_semantics(&mut hasher);
    hasher.add_str(enum_json(&operation.family));
    hasher.add_str(enum_json(&operation.kind));
    hasher.add_str(enum_json(&operation.execution_class));
    hasher.add_str(enum_json(&operation.determinism));
    add_artifact_contracts(&mut hasher, &operation.inputs);
    add_artifact_contracts(&mut hasher, &operation.outputs);
    hasher.add_u64(operation.parameters.len() as u64);
    for parameter in &operation.parameters {
        hasher.add_str(&parameter.name);
        hasher.add_str(enum_json(&parameter.value_type));
        hasher.add_bool(parameter.required);
        add_optional_parameter(&mut hasher, parameter.default.as_ref());
    }
    hasher.add_u64(operation.required_capabilities.len() as u64);
    for capability in &operation.required_capabilities {
        capability.add_semantics(&mut hasher);
    }
    hasher.add_u64(operation.invalidation.len() as u64);
    for invalidation in &operation.invalidation {
        hasher.add_str(enum_json(invalidation));
    }
    add_limits(&mut hasher, &operation.limits);
    hasher.finish()
}

fn request_digest(request: &MiningRequest) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(MINING_REQUEST_SCHEMA);
    request.context.workload.add_semantics(&mut hasher);
    request.context.graph.add_semantics(&mut hasher);
    add_optional_contract(&mut hasher, request.context.scenario.as_ref());
    add_optional_digest(&mut hasher, request.context.campaign_id.as_ref());
    request.context.space.add_semantics(&mut hasher);
    add_optional_digest(&mut hasher, request.context.active_transition_id.as_ref());
    hasher.add_str(&request.context.graph_node_id);
    hasher.add_str(enum_json(&request.context.rationale));
    add_digests(&mut hasher, &request.context.frontier_row_ids);
    add_digests(&mut hasher, &request.context.delta_ids);
    request.operation.add_semantics(&mut hasher);
    request.provider.add_semantics(&mut hasher);
    hasher.add_str(request.capability_snapshot_id.as_str());
    add_artifact_refs(&mut hasher, &request.inputs);
    add_parameters(&mut hasher, &request.parameters);
    add_limits(&mut hasher, &request.requested_limits);
    add_limits(&mut hasher, &request.effective_limits);
    hasher.finish()
}

fn result_digest(result: &MiningResult) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(MINING_RESULT_SCHEMA);
    hasher.add_str(result.request_id.as_str());
    result.operation.add_semantics(&mut hasher);
    result.provider.add_semantics(&mut hasher);
    hasher.add_str(result.capability_snapshot_id.as_str());
    hasher.add_str(enum_json(&result.completeness));
    add_artifact_refs(&mut hasher, &result.outputs);
    hasher.add_u64(result.lineage.len() as u64);
    for entry in &result.lineage {
        hasher.add_str(enum_json(&entry.kind));
        entry.identity.add_semantics(&mut hasher);
        add_optional_digest(&mut hasher, entry.execution_id.as_ref());
    }
    hasher.add_u64(result.dependencies.len() as u64);
    for edge in &result.dependencies {
        hasher.add_str(edge.artifact_id.as_str());
        hasher.add_str(enum_json(&edge.kind));
        hasher.add_str(edge.dependency_id.as_str());
    }
    hasher.add_u64(result.omissions.len() as u64);
    for omission in &result.omissions {
        hasher.add_str(enum_json(&omission.kind));
        hasher.add_optional_str(omission.subject_id.as_deref());
        hasher.add_u64(omission.omitted_count);
        hasher.add_str(&omission.reason);
    }
    add_consumption(&mut hasher, &result.consumption);
    hasher.finish()
}

fn add_artifact_contracts(hasher: &mut SemanticHasher, values: &[MiningArtifactContract]) {
    hasher.add_u64(values.len() as u64);
    for value in values {
        hasher.add_str(&value.port_id);
        hasher.add_str(enum_json(&value.kind));
        add_optional_contract(hasher, value.schema.as_ref());
        hasher.add_optional_str(value.media_type.as_deref());
        hasher.add_bool(value.required);
    }
}

fn add_artifact_refs(hasher: &mut SemanticHasher, values: &[MiningArtifactRef]) {
    hasher.add_u64(values.len() as u64);
    for value in values {
        hasher.add_str(&value.port_id);
        hasher.add_str(value.artifact_id.as_str());
        hasher.add_str(enum_json(&value.kind));
        add_optional_contract(hasher, value.schema.as_ref());
        hasher.add_str(&value.media_type);
        value.provider.add_semantics(hasher);
        hasher.add_str(&value.source_id);
        hasher.add_str(&value.source_revision);
        hasher.add_u64(value.logical_bytes);
    }
}

fn add_parameters(hasher: &mut SemanticHasher, values: &BTreeMap<String, MiningParameterValue>) {
    hasher.add_u64(values.len() as u64);
    for (name, value) in values {
        hasher.add_str(name);
        add_parameter(hasher, value);
    }
}

fn add_optional_parameter(hasher: &mut SemanticHasher, value: Option<&MiningParameterValue>) {
    hasher.add_bool(value.is_some());
    if let Some(value) = value {
        add_parameter(hasher, value);
    }
}

fn add_parameter(hasher: &mut SemanticHasher, value: &MiningParameterValue) {
    hasher.add_str(enum_json(&value.value_type()));
    match value {
        MiningParameterValue::Bool(value) => hasher.add_bool(*value),
        MiningParameterValue::I64(value) => hasher.add_bytes(&value.to_le_bytes()),
        MiningParameterValue::U64(value) => hasher.add_u64(*value),
        MiningParameterValue::Utf8(value) => hasher.add_str(value),
        MiningParameterValue::Utf8List(values) => {
            hasher.add_u64(values.len() as u64);
            for value in values {
                hasher.add_str(value);
            }
        }
    }
}

fn add_limits(hasher: &mut SemanticHasher, limits: &MiningLimits) {
    hasher.add_u64(limits.max_input_artifacts);
    hasher.add_u64(limits.max_output_artifacts);
    hasher.add_u64(limits.max_parameters);
    hasher.add_u64(limits.max_required_capabilities);
    hasher.add_u64(limits.max_rationale_refs);
    hasher.add_u64(limits.max_lineage_entries);
    hasher.add_u64(limits.max_dependencies);
    hasher.add_u64(limits.max_omissions);
    hasher.add_u64(limits.max_files);
    hasher.add_u64(limits.max_rows);
    hasher.add_u64(limits.max_matches);
    hasher.add_u64(limits.max_nodes);
    hasher.add_u64(limits.max_edges);
    hasher.add_u64(limits.max_depth);
    hasher.add_u64(limits.max_bytes);
    hasher.add_u64(limits.max_string_bytes);
    hasher.add_u64(limits.max_time_ms);
}

fn add_consumption(hasher: &mut SemanticHasher, consumption: &MiningConsumption) {
    hasher.add_u64(consumption.files);
    hasher.add_u64(consumption.rows);
    hasher.add_u64(consumption.matches);
    hasher.add_u64(consumption.nodes);
    hasher.add_u64(consumption.edges);
    hasher.add_u64(consumption.depth);
    hasher.add_u64(consumption.bytes_read);
    hasher.add_u64(consumption.bytes_written);
    hasher.add_bool(consumption.observed_time_ms.is_some());
    if let Some(observed_time_ms) = consumption.observed_time_ms {
        hasher.add_u64(observed_time_ms);
    }
}

fn add_optional_contract(hasher: &mut SemanticHasher, value: Option<&ContractIdentity>) {
    hasher.add_bool(value.is_some());
    if let Some(value) = value {
        value.add_semantics(hasher);
    }
}

fn add_optional_digest(hasher: &mut SemanticHasher, value: Option<&SemanticDigest>) {
    hasher.add_bool(value.is_some());
    if let Some(value) = value {
        hasher.add_str(value.as_str());
    }
}

fn add_digests(hasher: &mut SemanticHasher, values: &[SemanticDigest]) {
    hasher.add_u64(values.len() as u64);
    for value in values {
        hasher.add_str(value.as_str());
    }
}

fn enum_json<T: SemanticName>(value: &T) -> &'static str {
    value.semantic_name()
}

#[cfg(test)]
mod tests;
