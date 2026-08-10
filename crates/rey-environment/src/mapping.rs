use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
};

use rey_core::{SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Availability, CapabilityRecord, TrustClass};

pub const ENVIRONMENT_MAP_SCHEMA: &str = "rey.env-map.v3";
pub const ENVIRONMENT_MAP_OBSERVATION_SCHEMA: &str = "rey.env-map-observation.v3";
pub const ENVIRONMENT_MAP_PROVIDER_ID: &str = "rey.env-map";
pub const ENVIRONMENT_MAP_PROVIDER_REVISION: u64 = 3;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentMapLimits {
    pub max_document_bytes: u64,
    pub max_nodes: u64,
    pub max_edges: u64,
    pub max_projection_rows: u64,
    pub max_string_bytes: u64,
    pub max_variable_value_bytes: u64,
    pub max_input_file_bytes: u64,
    pub max_total_input_bytes: u64,
    pub max_executable_bytes: u64,
}

impl Default for EnvironmentMapLimits {
    fn default() -> Self {
        Self {
            max_document_bytes: 1_048_576,
            max_nodes: 32,
            max_edges: 64,
            max_projection_rows: 48,
            max_string_bytes: 512,
            max_variable_value_bytes: 16_384,
            max_input_file_bytes: 16_777_216,
            max_total_input_bytes: 67_108_864,
            max_executable_bytes: 67_108_864,
        }
    }
}

impl EnvironmentMapLimits {
    fn verify(&self) -> Result<(), EnvironmentMapError> {
        if self.max_document_bytes == 0
            || self.max_nodes == 0
            || self.max_edges == 0
            || self.max_projection_rows == 0
            || self.max_string_bytes == 0
            || self.max_variable_value_bytes == 0
            || self.max_input_file_bytes == 0
            || self.max_total_input_bytes == 0
            || self.max_executable_bytes == 0
        {
            return Err(EnvironmentMapError::ZeroLimit);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableCapture {
    #[default]
    Presence,
    Digest,
    Value,
}

impl VariableCapture {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Presence => "presence",
            Self::Digest => "digest",
            Self::Value => "value",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EnvironmentMapNode {
    Variable {
        id: String,
        name: String,
        #[serde(default)]
        sensitive: bool,
        #[serde(default)]
        capture: VariableCapture,
    },
    File {
        id: String,
        path: PathBuf,
        #[serde(default)]
        required: bool,
    },
    Executable {
        id: String,
        name: String,
        #[serde(default)]
        purpose: Option<String>,
        #[serde(default)]
        required: bool,
        #[serde(default)]
        potential_capabilities: Vec<String>,
    },
}

impl EnvironmentMapNode {
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Variable { id, .. } | Self::File { id, .. } | Self::Executable { id, .. } => id,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Variable { .. } => "variable",
            Self::File { .. } => "file",
            Self::Executable { .. } => "executable",
        }
    }

    fn normalize(&mut self) {
        if let Self::Executable {
            potential_capabilities,
            ..
        } = self
        {
            potential_capabilities.sort();
            potential_capabilities.dedup();
        }
    }

    fn add_semantics(&self, hasher: &mut SemanticHasher) {
        hasher.add_str(self.id());
        hasher.add_str(self.kind());
        match self {
            Self::Variable {
                name,
                sensitive,
                capture,
                ..
            } => {
                hasher.add_str(name);
                hasher.add_bool(*sensitive);
                hasher.add_str(capture.as_str());
            }
            Self::File { path, required, .. } => {
                hasher.add_str(&path.to_string_lossy());
                hasher.add_bool(*required);
            }
            Self::Executable {
                name,
                purpose,
                required,
                potential_capabilities,
                ..
            } => {
                hasher.add_str(name);
                hasher.add_optional_str(purpose.as_deref());
                hasher.add_bool(*required);
                add_strings(hasher, potential_capabilities);
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentMapEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

impl EnvironmentMapEdge {
    fn add_semantics(&self, hasher: &mut SemanticHasher) {
        hasher.add_str(&self.from);
        hasher.add_str(&self.to);
        hasher.add_str(&self.relation);
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SuppliedEnvironmentMap {
    schema: String,
    #[serde(default)]
    nodes: Vec<EnvironmentMapNode>,
    #[serde(default)]
    edges: Vec<EnvironmentMapEdge>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentMap {
    pub schema: String,
    pub graph_id: SemanticDigest,
    pub limits: EnvironmentMapLimits,
    pub nodes: Vec<EnvironmentMapNode>,
    pub edges: Vec<EnvironmentMapEdge>,
}

impl EnvironmentMap {
    pub fn from_yaml_slice(
        bytes: &[u8],
        limits: EnvironmentMapLimits,
    ) -> Result<Self, EnvironmentMapError> {
        limits.verify()?;
        if bytes.len() as u64 > limits.max_document_bytes {
            return Err(EnvironmentMapError::DocumentLimit {
                limit: limits.max_document_bytes,
                actual: bytes.len() as u64,
            });
        }
        let supplied: SuppliedEnvironmentMap = serde_saphyr::from_slice(bytes)?;
        Self::from_supplied(supplied, limits)
    }

    fn from_supplied(
        supplied: SuppliedEnvironmentMap,
        limits: EnvironmentMapLimits,
    ) -> Result<Self, EnvironmentMapError> {
        if supplied.schema != ENVIRONMENT_MAP_SCHEMA {
            return Err(EnvironmentMapError::UnsupportedSchema(supplied.schema));
        }
        if supplied.nodes.len() as u64 > limits.max_nodes {
            return Err(EnvironmentMapError::NodeLimit {
                limit: limits.max_nodes,
                actual: supplied.nodes.len() as u64,
            });
        }
        if supplied.edges.len() as u64 > limits.max_edges {
            return Err(EnvironmentMapError::EdgeLimit {
                limit: limits.max_edges,
                actual: supplied.edges.len() as u64,
            });
        }
        let projected = supplied
            .nodes
            .len()
            .saturating_add(supplied.edges.len())
            .saturating_add(1) as u64;
        if projected > limits.max_projection_rows {
            return Err(EnvironmentMapError::ProjectionLimit {
                limit: limits.max_projection_rows,
                actual: projected,
            });
        }
        let mut nodes = supplied.nodes;
        for node in &mut nodes {
            node.normalize();
            validate_node(node, &limits)?;
        }
        nodes.sort_by(|left, right| left.id().cmp(right.id()));
        if let Some(duplicate) = nodes.windows(2).find(|pair| pair[0].id() == pair[1].id()) {
            return Err(EnvironmentMapError::DuplicateNode(
                duplicate[0].id().to_owned(),
            ));
        }
        let node_ids = nodes
            .iter()
            .map(|node| node.id().to_owned())
            .collect::<BTreeSet<_>>();
        let mut edges = supplied.edges;
        for edge in &edges {
            validate_identifier("edge from", &edge.from, &limits)?;
            validate_identifier("edge to", &edge.to, &limits)?;
            validate_identifier("edge relation", &edge.relation, &limits)?;
            if edge.from == edge.to {
                return Err(EnvironmentMapError::SelfEdge(edge.from.clone()));
            }
            for endpoint in [&edge.from, &edge.to] {
                if !node_ids.contains(endpoint) {
                    return Err(EnvironmentMapError::MissingEndpoint(endpoint.clone()));
                }
            }
        }
        edges.sort();
        if let Some(duplicate) = edges.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(EnvironmentMapError::DuplicateEdge {
                from: duplicate[0].from.clone(),
                to: duplicate[0].to.clone(),
                relation: duplicate[0].relation.clone(),
            });
        }
        let graph_id = graph_digest(&limits, &nodes, &edges);
        Ok(Self {
            schema: ENVIRONMENT_MAP_SCHEMA.to_owned(),
            graph_id,
            limits,
            nodes,
            edges,
        })
    }

    pub fn verify(&self) -> Result<(), EnvironmentMapError> {
        let supplied = SuppliedEnvironmentMap {
            schema: self.schema.clone(),
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
        };
        let recomputed = Self::from_supplied(supplied, self.limits.clone())?;
        if self != &recomputed {
            return Err(EnvironmentMapError::NonCanonicalGraph);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EnvironmentMapInputs {
    pub variables: BTreeMap<OsString, OsString>,
    pub search_paths: Vec<PathBuf>,
}

impl EnvironmentMapInputs {
    #[must_use]
    pub fn from_environment() -> Self {
        let variables = std::env::vars_os().collect::<BTreeMap<_, _>>();
        let search_paths = variables
            .get(OsStr::new("PATH"))
            .map(std::env::split_paths)
            .map(Iterator::collect)
            .unwrap_or_default();
        Self {
            variables,
            search_paths,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnvironmentMapObservation {
    pub schema: String,
    pub source_id: SemanticDigest,
    pub source_path: PathBuf,
    pub graph: EnvironmentMap,
    pub capabilities: Vec<CapabilityRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentMapNodeProvenance {
    pub declaration: EnvironmentMapNode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_length: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captured_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_path_count: Option<u64>,
}

impl EnvironmentMapObservation {
    pub fn load(
        workspace: &Path,
        selected_path: Option<&Path>,
        inputs: &EnvironmentMapInputs,
        limits: EnvironmentMapLimits,
    ) -> Result<Option<Self>, EnvironmentMapError> {
        limits.verify()?;
        let Some(selected_path) = selected_path else {
            return Ok(None);
        };
        let workspace = workspace
            .canonicalize()
            .map_err(|source| EnvironmentMapError::Path {
                path: workspace.to_owned(),
                source,
            })?;
        let relative = selected_path.to_owned();
        validate_relative_path(&relative)?;
        let path = workspace.join(&relative);
        validate_no_symlinks(&workspace, &path)?;
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(source) => return Err(EnvironmentMapError::Path { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(EnvironmentMapError::UnsafeMapPath(path));
        }
        let bytes = read_bounded(&path, limits.max_document_bytes)?;
        let graph = EnvironmentMap::from_yaml_slice(&bytes, limits)?;
        let source_id = source_digest(&relative, &bytes);
        let mut total_input_bytes = 0_u64;
        let mut capabilities = Vec::with_capacity(
            graph
                .nodes
                .len()
                .saturating_add(graph.edges.len())
                .saturating_add(1),
        );
        capabilities.push(graph_capability(&relative, &source_id, &graph)?);
        for node in &graph.nodes {
            capabilities.push(observe_node(
                &workspace,
                node,
                inputs,
                &graph.limits,
                &mut total_input_bytes,
            )?);
        }
        for edge in &graph.edges {
            capabilities.push(edge_capability(edge)?);
        }
        Ok(Some(Self {
            schema: ENVIRONMENT_MAP_OBSERVATION_SCHEMA.to_owned(),
            source_id,
            source_path: relative,
            graph,
            capabilities,
        }))
    }
}

fn observe_node(
    workspace: &Path,
    node: &EnvironmentMapNode,
    inputs: &EnvironmentMapInputs,
    limits: &EnvironmentMapLimits,
    total_input_bytes: &mut u64,
) -> Result<CapabilityRecord, EnvironmentMapError> {
    match node {
        EnvironmentMapNode::Variable {
            id,
            name,
            sensitive,
            capture,
        } => {
            let value = inputs.variables.get(OsStr::new(name));
            let content_digest = match (value, capture) {
                (Some(value), VariableCapture::Digest | VariableCapture::Value) => {
                    Some(os_value_digest(name, value).to_string())
                }
                _ => None,
            };
            let captured_value = match (value, capture) {
                (Some(value), VariableCapture::Value) => {
                    let value = value
                        .to_str()
                        .ok_or_else(|| EnvironmentMapError::VariableValueEncoding(name.clone()))?;
                    if value.len() as u64 > limits.max_variable_value_bytes {
                        return Err(EnvironmentMapError::VariableValueLimit {
                            name: name.clone(),
                            limit: limits.max_variable_value_bytes,
                            actual: value.len() as u64,
                        });
                    }
                    Some(value.to_owned())
                }
                _ => None,
            };
            Ok(CapabilityRecord {
                provider_id: ENVIRONMENT_MAP_PROVIDER_ID.to_owned(),
                provider_revision: ENVIRONMENT_MAP_PROVIDER_REVISION,
                provider_kind: "environment_mapping".to_owned(),
                capability_id: format!("env.mapping.node.{id}"),
                capability_kind: "environment_variable".to_owned(),
                resolved_location: Some(format!("env://{name}")),
                version: None,
                content_digest,
                provenance: Some(node_provenance(node, None, captured_value, None)?),
                availability: if value.is_some() {
                    Availability::Available
                } else {
                    Availability::Unavailable
                },
                trust_class: TrustClass::ExplicitLocal,
                operations: vec![match capture {
                    VariableCapture::Presence => "observe_presence".to_owned(),
                    VariableCapture::Digest => "observe_digest".to_owned(),
                    VariableCapture::Value => "observe_value".to_owned(),
                }],
                enforced_limits: vec![if *sensitive {
                    "sensitive_presence_only".to_owned()
                } else {
                    format!(
                        "capture={}{}",
                        capture.as_str(),
                        if *capture == VariableCapture::Value {
                            format!(";max_bytes={}", limits.max_variable_value_bytes)
                        } else {
                            String::new()
                        }
                    )
                }],
                unsupported_limits: Vec::new(),
                observed_at: None,
                error_code: None,
                error_detail: None,
            })
        }
        EnvironmentMapNode::File { id, path, .. } => {
            let full_path = workspace.join(path);
            let (availability, content_digest, byte_length, error_code) =
                observe_file(workspace, &full_path, limits, total_input_bytes);
            Ok(CapabilityRecord {
                provider_id: ENVIRONMENT_MAP_PROVIDER_ID.to_owned(),
                provider_revision: ENVIRONMENT_MAP_PROVIDER_REVISION,
                provider_kind: "environment_mapping".to_owned(),
                capability_id: format!("env.mapping.node.{id}"),
                capability_kind: "input_file".to_owned(),
                resolved_location: Some(path.to_string_lossy().into_owned()),
                version: None,
                content_digest,
                provenance: Some(node_provenance(node, byte_length, None, None)?),
                availability,
                trust_class: TrustClass::ExplicitLocal,
                operations: vec!["observe_identity".to_owned()],
                enforced_limits: vec![
                    "regular_file".to_owned(),
                    "workspace_bounded".to_owned(),
                    format!("max_bytes={}", limits.max_input_file_bytes),
                ],
                unsupported_limits: vec!["symlink_inputs".to_owned()],
                observed_at: None,
                error_code,
                error_detail: None,
            })
        }
        EnvironmentMapNode::Executable {
            id,
            name,
            potential_capabilities,
            ..
        } => {
            let resolved = resolve_executable(workspace, name, &inputs.search_paths);
            let (availability, location, digest, byte_length, error_code) = match resolved {
                None => (Availability::Unavailable, None, None, None, None),
                Some(path) => match read_bounded(&path, limits.max_executable_bytes) {
                    Ok(bytes) => (
                        Availability::Available,
                        Some(path.to_string_lossy().into_owned()),
                        Some(byte_digest("rey.env-map.executable.v1", &bytes).to_string()),
                        Some(bytes.len().to_string()),
                        None,
                    ),
                    Err(EnvironmentMapError::ByteLimit { .. }) => (
                        Availability::Error,
                        Some(path.to_string_lossy().into_owned()),
                        None,
                        None,
                        Some("executable_byte_limit".to_owned()),
                    ),
                    Err(_) => (
                        Availability::Error,
                        Some(path.to_string_lossy().into_owned()),
                        None,
                        None,
                        Some("executable_read_failed".to_owned()),
                    ),
                },
            };
            Ok(CapabilityRecord {
                provider_id: ENVIRONMENT_MAP_PROVIDER_ID.to_owned(),
                provider_revision: ENVIRONMENT_MAP_PROVIDER_REVISION,
                provider_kind: "environment_mapping".to_owned(),
                capability_id: format!("env.mapping.node.{id}"),
                capability_kind: "potential_executable".to_owned(),
                resolved_location: location,
                version: None,
                content_digest: digest,
                provenance: Some(node_provenance(
                    node,
                    byte_length,
                    None,
                    Some(inputs.search_paths.len() as u64),
                )?),
                availability,
                trust_class: TrustClass::DiscoveredLocal,
                operations: vec!["resolve_identity".to_owned()],
                enforced_limits: vec![
                    "no_execution".to_owned(),
                    format!("max_bytes={}", limits.max_executable_bytes),
                ],
                unsupported_limits: potential_capabilities
                    .iter()
                    .map(|capability| format!("unadmitted:{capability}"))
                    .collect(),
                observed_at: None,
                error_code,
                error_detail: None,
            })
        }
    }
}

fn observe_file(
    workspace: &Path,
    path: &Path,
    limits: &EnvironmentMapLimits,
    total_input_bytes: &mut u64,
) -> (Availability, Option<String>, Option<String>, Option<String>) {
    if validate_no_symlinks(workspace, path).is_err() {
        return (
            Availability::Error,
            None,
            None,
            Some("unsafe_file_path".to_owned()),
        );
    }
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return (Availability::Unavailable, None, None, None);
        }
        Err(_) => {
            return (
                Availability::Error,
                None,
                None,
                Some("file_metadata_failed".to_owned()),
            );
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return (
            Availability::Error,
            None,
            None,
            Some("not_regular_file".to_owned()),
        );
    }
    if metadata.len() > limits.max_input_file_bytes {
        return (
            Availability::Error,
            None,
            Some(metadata.len().to_string()),
            Some("file_byte_limit".to_owned()),
        );
    }
    let next_total = total_input_bytes.saturating_add(metadata.len());
    if next_total > limits.max_total_input_bytes {
        return (
            Availability::Error,
            None,
            Some(metadata.len().to_string()),
            Some("total_file_byte_limit".to_owned()),
        );
    }
    match read_bounded(path, limits.max_input_file_bytes) {
        Ok(bytes) => {
            *total_input_bytes = next_total;
            (
                Availability::Available,
                Some(byte_digest("rey.env-map.input-file.v1", &bytes).to_string()),
                Some(bytes.len().to_string()),
                None,
            )
        }
        Err(_) => (
            Availability::Error,
            None,
            Some(metadata.len().to_string()),
            Some("file_read_failed".to_owned()),
        ),
    }
}

fn graph_capability(
    relative_path: &Path,
    source_id: &SemanticDigest,
    graph: &EnvironmentMap,
) -> Result<CapabilityRecord, EnvironmentMapError> {
    #[derive(Serialize)]
    struct GraphProvenance<'a> {
        source_id: &'a SemanticDigest,
        graph: &'a EnvironmentMap,
    }
    Ok(CapabilityRecord {
        provider_id: ENVIRONMENT_MAP_PROVIDER_ID.to_owned(),
        provider_revision: ENVIRONMENT_MAP_PROVIDER_REVISION,
        provider_kind: "environment_mapping".to_owned(),
        capability_id: "env.mapping.graph".to_owned(),
        capability_kind: "environment_map".to_owned(),
        resolved_location: Some(relative_path.to_string_lossy().into_owned()),
        version: Some(graph.schema.clone()),
        content_digest: Some(graph.graph_id.to_string()),
        provenance: Some(serde_json::to_string(&GraphProvenance {
            source_id,
            graph,
        })?),
        availability: Availability::Available,
        trust_class: TrustClass::ExplicitLocal,
        operations: vec!["observe_mapping".to_owned()],
        enforced_limits: vec![
            format!("max_document_bytes={}", graph.limits.max_document_bytes),
            format!("max_edges={}", graph.limits.max_edges),
            format!("max_nodes={}", graph.limits.max_nodes),
            format!("max_projection_rows={}", graph.limits.max_projection_rows),
        ],
        unsupported_limits: Vec::new(),
        observed_at: None,
        error_code: None,
        error_detail: None,
    })
}

fn edge_capability(edge: &EnvironmentMapEdge) -> Result<CapabilityRecord, EnvironmentMapError> {
    let mut hasher = SemanticHasher::new("rey.env-map.edge.v1");
    edge.add_semantics(&mut hasher);
    Ok(CapabilityRecord {
        provider_id: ENVIRONMENT_MAP_PROVIDER_ID.to_owned(),
        provider_revision: ENVIRONMENT_MAP_PROVIDER_REVISION,
        provider_kind: "environment_mapping".to_owned(),
        capability_id: format!(
            "env.mapping.edge.{}.{}.{}",
            edge.from, edge.relation, edge.to
        ),
        capability_kind: "environment_edge".to_owned(),
        resolved_location: Some(format!(
            "env-map://{}/{}/{}",
            edge.from, edge.relation, edge.to
        )),
        version: None,
        content_digest: Some(hasher.finish().to_string()),
        provenance: Some(serde_json::to_string(edge)?),
        availability: Availability::Available,
        trust_class: TrustClass::ExplicitLocal,
        operations: vec!["declare_relevance".to_owned()],
        enforced_limits: vec!["declaration_only".to_owned()],
        unsupported_limits: vec!["parser_verified_reference".to_owned()],
        observed_at: None,
        error_code: None,
        error_detail: None,
    })
}

fn validate_node(
    node: &EnvironmentMapNode,
    limits: &EnvironmentMapLimits,
) -> Result<(), EnvironmentMapError> {
    validate_identifier("node id", node.id(), limits)?;
    match node {
        EnvironmentMapNode::Variable {
            name,
            sensitive,
            capture,
            ..
        } => {
            validate_string("variable name", name, limits)?;
            if !valid_variable_name(name) {
                return Err(EnvironmentMapError::InvalidVariableName(name.clone()));
            }
            if *sensitive && *capture == VariableCapture::Digest {
                return Err(EnvironmentMapError::SensitiveDigest(name.clone()));
            }
            if *sensitive && *capture == VariableCapture::Value {
                return Err(EnvironmentMapError::SensitiveValue(name.clone()));
            }
        }
        EnvironmentMapNode::File { path, .. } => {
            validate_string("file path", &path.to_string_lossy(), limits)?;
            validate_relative_path(path)?;
        }
        EnvironmentMapNode::Executable {
            name,
            purpose,
            potential_capabilities,
            ..
        } => {
            validate_identifier("executable name", name, limits)?;
            let purpose = purpose
                .as_deref()
                .ok_or_else(|| EnvironmentMapError::MissingExecutablePurpose(name.clone()))?;
            validate_string("executable purpose", purpose, limits)?;
            for capability in potential_capabilities {
                validate_identifier("potential capability", capability, limits)?;
            }
        }
    }
    Ok(())
}

fn node_provenance(
    node: &EnvironmentMapNode,
    byte_length: Option<String>,
    captured_value: Option<String>,
    search_path_count: Option<u64>,
) -> Result<String, EnvironmentMapError> {
    Ok(serde_json::to_string(&EnvironmentMapNodeProvenance {
        declaration: node.clone(),
        byte_length,
        captured_value,
        search_path_count,
    })?)
}

fn validate_identifier(
    field: &'static str,
    value: &str,
    limits: &EnvironmentMapLimits,
) -> Result<(), EnvironmentMapError> {
    validate_string(field, value, limits)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(EnvironmentMapError::InvalidIdentifier {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_string(
    field: &'static str,
    value: &str,
    limits: &EnvironmentMapLimits,
) -> Result<(), EnvironmentMapError> {
    if value.is_empty() || value.contains('\0') || value.len() as u64 > limits.max_string_bytes {
        return Err(EnvironmentMapError::InvalidString {
            field,
            limit: limits.max_string_bytes,
        });
    }
    Ok(())
}

fn valid_variable_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn validate_relative_path(path: &Path) -> Result<(), EnvironmentMapError> {
    if path.as_os_str().is_empty()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(EnvironmentMapError::PathEscape(path.to_owned()));
    }
    Ok(())
}

fn validate_no_symlinks(workspace: &Path, path: &Path) -> Result<(), EnvironmentMapError> {
    let relative = path
        .strip_prefix(workspace)
        .map_err(|_| EnvironmentMapError::PathEscape(path.to_owned()))?;
    let mut current = workspace.to_owned();
    for component in relative.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(EnvironmentMapError::UnsafeMapPath(current));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(source) => {
                return Err(EnvironmentMapError::Path {
                    path: current,
                    source,
                });
            }
        }
    }
    Ok(())
}

fn resolve_executable(workspace: &Path, name: &str, search_paths: &[PathBuf]) -> Option<PathBuf> {
    search_paths.iter().find_map(|directory| {
        let directory = if directory.is_absolute() {
            directory.clone()
        } else {
            workspace.join(directory)
        };
        let candidate = directory.join(name);
        let metadata = fs::metadata(&candidate).ok()?;
        if !metadata.is_file() || !is_executable(&metadata) {
            return None;
        }
        candidate.canonicalize().ok()
    })
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &fs::Metadata) -> bool {
    true
}

fn read_bounded(path: &Path, max_bytes: u64) -> Result<Vec<u8>, EnvironmentMapError> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|source| EnvironmentMapError::Path {
            path: path.to_owned(),
            source,
        })?
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| EnvironmentMapError::Path {
            path: path.to_owned(),
            source,
        })?;
    if bytes.len() as u64 > max_bytes {
        return Err(EnvironmentMapError::ByteLimit {
            path: path.to_owned(),
            limit: max_bytes,
        });
    }
    Ok(bytes)
}

fn graph_digest(
    limits: &EnvironmentMapLimits,
    nodes: &[EnvironmentMapNode],
    edges: &[EnvironmentMapEdge],
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(ENVIRONMENT_MAP_SCHEMA);
    add_limits(&mut hasher, limits);
    hasher.add_u64(nodes.len() as u64);
    for node in nodes {
        node.add_semantics(&mut hasher);
    }
    hasher.add_u64(edges.len() as u64);
    for edge in edges {
        edge.add_semantics(&mut hasher);
    }
    hasher.finish()
}

fn source_digest(relative_path: &Path, bytes: &[u8]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.env-map.source.v1");
    hasher.add_str(&relative_path.to_string_lossy());
    hasher.add_bytes(bytes);
    hasher.finish()
}

fn os_value_digest(name: &str, value: &OsStr) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.env-map.variable-value.v1");
    hasher.add_str(name);
    hasher.add_bytes(os_bytes(value));
    hasher.finish()
}

fn byte_digest(domain: &str, bytes: &[u8]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(domain);
    hasher.add_bytes(bytes);
    hasher.finish()
}

#[cfg(unix)]
fn os_bytes(value: &OsStr) -> &[u8] {
    use std::os::unix::ffi::OsStrExt;

    value.as_bytes()
}

#[cfg(not(unix))]
fn os_bytes(value: &OsStr) -> &[u8] {
    value.to_str().unwrap_or_default().as_bytes()
}

fn add_limits(hasher: &mut SemanticHasher, limits: &EnvironmentMapLimits) {
    hasher.add_u64(limits.max_document_bytes);
    hasher.add_u64(limits.max_nodes);
    hasher.add_u64(limits.max_edges);
    hasher.add_u64(limits.max_projection_rows);
    hasher.add_u64(limits.max_string_bytes);
    hasher.add_u64(limits.max_variable_value_bytes);
    hasher.add_u64(limits.max_input_file_bytes);
    hasher.add_u64(limits.max_total_input_bytes);
    hasher.add_u64(limits.max_executable_bytes);
}

fn add_strings(hasher: &mut SemanticHasher, values: &[String]) {
    hasher.add_u64(values.len() as u64);
    for value in values {
        hasher.add_str(value);
    }
}

#[derive(Debug, Error)]
pub enum EnvironmentMapError {
    #[error("environment mapping limits must be greater than zero")]
    ZeroLimit,
    #[error("unsupported environment mapping schema {0}")]
    UnsupportedSchema(String),
    #[error("environment mapping document exceeds the {limit}-byte limit ({actual})")]
    DocumentLimit { limit: u64, actual: u64 },
    #[error("environment mapping exceeds the {limit}-node limit ({actual})")]
    NodeLimit { limit: u64, actual: u64 },
    #[error("environment mapping exceeds the {limit}-edge limit ({actual})")]
    EdgeLimit { limit: u64, actual: u64 },
    #[error("environment mapping exceeds the {limit}-row projection limit ({actual})")]
    ProjectionLimit { limit: u64, actual: u64 },
    #[error("duplicate environment mapping node {0}")]
    DuplicateNode(String),
    #[error("duplicate environment mapping edge {from} -[{relation}]-> {to}")]
    DuplicateEdge {
        from: String,
        to: String,
        relation: String,
    },
    #[error("environment mapping edge references missing node {0}")]
    MissingEndpoint(String),
    #[error("environment mapping node {0} cannot reference itself")]
    SelfEdge(String),
    #[error("invalid {field} identifier {value}")]
    InvalidIdentifier { field: &'static str, value: String },
    #[error("invalid {field}; value must be non-empty, NUL-free, and at most {limit} bytes")]
    InvalidString { field: &'static str, limit: u64 },
    #[error("invalid environment variable name {0}")]
    InvalidVariableName(String),
    #[error("desired executable {0} is missing its purpose")]
    MissingExecutablePurpose(String),
    #[error("sensitive environment variable {0} cannot retain a value digest")]
    SensitiveDigest(String),
    #[error("sensitive environment variable {0} cannot retain a raw value")]
    SensitiveValue(String),
    #[error("environment variable {0} is not valid UTF-8 for value capture")]
    VariableValueEncoding(String),
    #[error("environment variable {name} exceeds the {limit}-byte value capture limit ({actual})")]
    VariableValueLimit {
        name: String,
        limit: u64,
        actual: u64,
    },
    #[error("environment mapping path {0} escapes the workspace")]
    PathEscape(PathBuf),
    #[error("environment mapping path {0} is symlinked or not a safe regular file")]
    UnsafeMapPath(PathBuf),
    #[error("environment mapping path {path} failed: {source}")]
    Path {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("environment mapping input {path} exceeds the {limit}-byte limit")]
    ByteLimit { path: PathBuf, limit: u64 },
    #[error("environment mapping graph is not canonical")]
    NonCanonicalGraph,
    #[error("environment mapping YAML is invalid: {0}")]
    Yaml(#[from] serde_saphyr::Error),
    #[error("environment mapping JSON projection failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, ffi::OsString, fs, path::Path};

    use tempfile::TempDir;

    use super::{
        Availability, EnvironmentMap, EnvironmentMapError, EnvironmentMapInputs,
        EnvironmentMapLimits, EnvironmentMapNodeProvenance, EnvironmentMapObservation,
    };

    const VALID: &str = r#"
schema: rey.env-map.v3
nodes:
  - id: config
    kind: variable
    name: REY_CONFIG
    sensitive: false
    capture: digest
  - id: input
    kind: file
    path: input.txt
    required: true
  - id: tool
    kind: executable
    name: rey-tool
    purpose: Search the bounded workspace source corpus
    required: true
    potential_capabilities: [source.search]
edges:
  - from: config
    to: input
    relation: locates
  - from: input
    to: tool
    relation: input_to
"#;

    #[test]
    fn graph_is_canonical_bounded_and_rejects_invalid_edges_and_secrets() {
        let graph =
            EnvironmentMap::from_yaml_slice(VALID.as_bytes(), EnvironmentMapLimits::default())
                .unwrap();
        graph.verify().unwrap();
        assert_eq!(graph.nodes[0].id(), "config");
        assert_eq!(graph.edges[0].from, "config");

        let duplicate = VALID.replace(
            "  - from: input\n    to: tool\n    relation: input_to",
            "  - from: config\n    to: input\n    relation: locates",
        );
        assert!(matches!(
            EnvironmentMap::from_yaml_slice(duplicate.as_bytes(), EnvironmentMapLimits::default())
                .unwrap_err(),
            EnvironmentMapError::DuplicateEdge { .. }
        ));

        let duplicate_node = VALID.replace(
            "  - id: input\n    kind: file",
            "  - id: config\n    kind: file",
        );
        assert!(matches!(
            EnvironmentMap::from_yaml_slice(
                duplicate_node.as_bytes(),
                EnvironmentMapLimits::default()
            ),
            Err(EnvironmentMapError::DuplicateNode(_))
        ));

        let missing_endpoint = VALID.replace("    to: tool", "    to: absent");
        assert!(matches!(
            EnvironmentMap::from_yaml_slice(
                missing_endpoint.as_bytes(),
                EnvironmentMapLimits::default()
            ),
            Err(EnvironmentMapError::MissingEndpoint(_))
        ));

        let sensitive = VALID.replace("sensitive: false", "sensitive: true");
        assert!(matches!(
            EnvironmentMap::from_yaml_slice(sensitive.as_bytes(), EnvironmentMapLimits::default()),
            Err(EnvironmentMapError::SensitiveDigest(_))
        ));
        let sensitive_value = sensitive.replace("capture: digest", "capture: value");
        assert!(matches!(
            EnvironmentMap::from_yaml_slice(
                sensitive_value.as_bytes(),
                EnvironmentMapLimits::default()
            ),
            Err(EnvironmentMapError::SensitiveValue(_))
        ));

        let unknown = VALID.replace("capture: digest", "capture: digest\n    surprise: true");
        assert!(
            EnvironmentMap::from_yaml_slice(unknown.as_bytes(), EnvironmentMapLimits::default())
                .is_err()
        );

        let missing_purpose = VALID.replace(
            "    purpose: Search the bounded workspace source corpus\n",
            "",
        );
        assert!(matches!(
            EnvironmentMap::from_yaml_slice(
                missing_purpose.as_bytes(),
                EnvironmentMapLimits::default()
            ),
            Err(EnvironmentMapError::MissingExecutablePurpose(name)) if name == "rey-tool"
        ));
    }

    #[test]
    fn observation_retains_graph_facts_without_raw_variable_values_or_execution() {
        let workspace = TempDir::new().unwrap();
        fs::write(workspace.path().join("rey.env.yaml"), VALID).unwrap();
        fs::write(workspace.path().join("input.txt"), "mapped input\n").unwrap();
        let bin = workspace.path().join("bin");
        fs::create_dir(&bin).unwrap();
        let executable = bin.join("rey-tool");
        fs::write(&executable, "fixture executable\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&executable).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&executable, permissions).unwrap();
        }
        let inputs = EnvironmentMapInputs {
            variables: BTreeMap::from([(
                OsString::from("REY_CONFIG"),
                OsString::from("do-not-retain"),
            )]),
            search_paths: vec![bin],
        };
        assert!(
            EnvironmentMapObservation::load(
                workspace.path(),
                None,
                &inputs,
                EnvironmentMapLimits::default(),
            )
            .unwrap()
            .is_none()
        );
        let observation = EnvironmentMapObservation::load(
            workspace.path(),
            Some(Path::new("rey.env.yaml")),
            &inputs,
            EnvironmentMapLimits::default(),
        )
        .unwrap()
        .unwrap();

        assert_eq!(observation.graph.nodes.len(), 3);
        assert_eq!(observation.graph.edges.len(), 2);
        assert_eq!(observation.capabilities.len(), 6);
        let serialized = serde_json::to_string(&observation).unwrap();
        assert!(!serialized.contains("do-not-retain"));
        assert!(serialized.contains("Search the bounded workspace source corpus"));
        assert!(serialized.contains("unadmitted:source.search"));
        assert!(observation.capabilities.iter().any(|row| {
            row.capability_kind == "potential_executable" && row.content_digest.is_some()
        }));

        fs::remove_file(workspace.path().join("input.txt")).unwrap();
        let unavailable = EnvironmentMapObservation::load(
            workspace.path(),
            Some(Path::new("rey.env.yaml")),
            &EnvironmentMapInputs::default(),
            EnvironmentMapLimits::default(),
        )
        .unwrap()
        .unwrap();
        for capability_id in [
            "env.mapping.node.config",
            "env.mapping.node.input",
            "env.mapping.node.tool",
        ] {
            assert!(unavailable.capabilities.iter().any(|row| {
                row.capability_id == capability_id && row.availability == Availability::Unavailable
            }));
        }
    }

    #[test]
    fn explicit_value_capture_is_bounded_and_retained_in_typed_provenance() {
        let workspace = TempDir::new().unwrap();
        fs::write(
            workspace.path().join("rey.env.yaml"),
            "schema: rey.env-map.v3\nnodes:\n  - id: mode\n    kind: variable\n    name: REY_MODE\n    capture: value\n",
        )
        .unwrap();
        let inputs = EnvironmentMapInputs {
            variables: BTreeMap::from([(
                OsString::from("REY_MODE"),
                OsString::from("development"),
            )]),
            search_paths: Vec::new(),
        };
        let observation = EnvironmentMapObservation::load(
            workspace.path(),
            Some(Path::new("rey.env.yaml")),
            &inputs,
            EnvironmentMapLimits::default(),
        )
        .unwrap()
        .unwrap();
        let variable = observation
            .capabilities
            .iter()
            .find(|row| row.capability_id == "env.mapping.node.mode")
            .unwrap();
        let provenance: EnvironmentMapNodeProvenance =
            serde_json::from_str(variable.provenance.as_deref().unwrap()).unwrap();
        assert_eq!(provenance.captured_value.as_deref(), Some("development"));
        assert!(variable.content_digest.is_some());

        let limits = EnvironmentMapLimits {
            max_variable_value_bytes: 4,
            ..EnvironmentMapLimits::default()
        };
        assert!(matches!(
            EnvironmentMapObservation::load(
                workspace.path(),
                Some(Path::new("rey.env.yaml")),
                &inputs,
                limits,
            ),
            Err(EnvironmentMapError::VariableValueLimit { .. })
        ));
    }

    #[test]
    fn path_escape_symlink_and_document_bounds_fail_closed() {
        let workspace = TempDir::new().unwrap();
        fs::write(workspace.path().join("rey.env.yaml"), VALID).unwrap();
        let escaped = VALID.replace("path: input.txt", "path: ../input.txt");
        assert!(matches!(
            EnvironmentMap::from_yaml_slice(escaped.as_bytes(), EnvironmentMapLimits::default()),
            Err(EnvironmentMapError::PathEscape(_))
        ));

        let limits = EnvironmentMapLimits {
            max_document_bytes: 8,
            ..EnvironmentMapLimits::default()
        };
        assert!(matches!(
            EnvironmentMap::from_yaml_slice(VALID.as_bytes(), limits),
            Err(EnvironmentMapError::DocumentLimit { .. })
        ));

        assert!(matches!(
            EnvironmentMapObservation::load(
                workspace.path(),
                Some(std::path::Path::new("../outside.yaml")),
                &EnvironmentMapInputs::default(),
                EnvironmentMapLimits::default(),
            ),
            Err(EnvironmentMapError::PathEscape(_))
        ));

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let target = workspace.path().join("actual.yaml");
            fs::rename(workspace.path().join("rey.env.yaml"), &target).unwrap();
            symlink(&target, workspace.path().join("rey.env.yaml")).unwrap();
            assert!(matches!(
                EnvironmentMapObservation::load(
                    workspace.path(),
                    Some(Path::new("rey.env.yaml")),
                    &EnvironmentMapInputs::default(),
                    EnvironmentMapLimits::default(),
                ),
                Err(EnvironmentMapError::UnsafeMapPath(_))
            ));
        }
    }
}
