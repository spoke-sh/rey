use std::collections::{BTreeMap, BTreeSet};

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_locator::ResolutionStatus;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{TopographyAnchorKind, TopographyPatch, TopographyRegionState};

pub const PROJECTION_PACKET_SCHEMA: &str = "rey.projection-packet.v1";

const TERRAIN_WIDTH: u64 = 1_500;
const TERRAIN_HEIGHT: u64 = 1_000;
const TERRAIN_GRID_COLUMNS: u64 = 60;
const TERRAIN_GRID_ROWS: u64 = 40;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionLimits {
    pub max_anchor_objects: u64,
    pub max_frontier_objects: u64,
    pub max_validity_regions: u64,
    pub max_field_channels: u64,
    pub max_layers: u64,
    pub max_omissions: u64,
    pub max_field_cells: u64,
    pub max_field_bytes: u64,
    pub max_contours: u64,
    pub max_natural_features: u64,
    pub max_labels: u64,
}

impl Default for ProjectionLimits {
    fn default() -> Self {
        let max_field_cells = (TERRAIN_GRID_COLUMNS + 1) * (TERRAIN_GRID_ROWS + 1);
        Self {
            max_anchor_objects: 64,
            max_frontier_objects: 6,
            max_validity_regions: 256,
            max_field_channels: 8,
            max_layers: 8,
            max_omissions: 1_032,
            max_field_cells,
            max_field_bytes: max_field_cells * 8 * 8,
            max_contours: 7,
            max_natural_features: 96,
            max_labels: 70,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionExtent {
    pub width: u64,
    pub height: u64,
    pub unit: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionBasis {
    pub contract: ContractIdentity,
    pub input_dimensions: Vec<String>,
    pub output_dimensions: Vec<String>,
    pub parameters: BTreeMap<String, String>,
    pub normalization: String,
    pub random_seed: Option<u64>,
    pub distance_semantics: String,
    pub neighborhood_semantics: String,
    pub distortion: String,
    pub stable_coordinate_rule: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionObjectKind {
    Anchor,
    Frontier,
}

impl ProjectionObjectKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Anchor => "anchor",
            Self::Frontier => "frontier",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionObject {
    pub object_id: String,
    pub source_id: SemanticDigest,
    pub kind: ProjectionObjectKind,
    pub anchor_kind: Option<TopographyAnchorKind>,
    pub frontier_status: Option<ResolutionStatus>,
    pub coordinate: Option<String>,
    pub label: String,
    pub detail: String,
    pub source_revision: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionValidityRegion {
    pub region_id: SemanticDigest,
    pub coordinate: String,
    pub state: TopographyRegionState,
    pub detail: String,
    pub source_revision: SemanticDigest,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionFieldKind {
    Scalar,
    Vector,
    Mask,
}

impl ProjectionFieldKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::Vector => "vector",
            Self::Mask => "mask",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionFieldChannel {
    pub id: String,
    pub kind: ProjectionFieldKind,
    pub semantics: String,
    pub units: String,
    pub normalization: String,
    pub source_revision: SemanticDigest,
    pub implementation: ContractIdentity,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionLayerAuthority {
    Evidence,
    Derived,
    Presentation,
}

impl ProjectionLayerAuthority {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Evidence => "evidence",
            Self::Derived => "derived",
            Self::Presentation => "presentation",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionLayer {
    pub id: String,
    pub authority: ProjectionLayerAuthority,
    pub semantics: String,
    pub source_revision: SemanticDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionOmission {
    pub kind: String,
    pub subject: String,
    pub omitted_count: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionDegradation {
    pub kind: String,
    pub omitted_count: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionLineage {
    pub kind: String,
    pub identity: String,
    pub revision: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionPacket {
    pub schema: String,
    pub packet_id: SemanticDigest,
    pub source_patch_id: SemanticDigest,
    pub source_topography_revision: SemanticDigest,
    pub projection_basis: ProjectionBasis,
    pub scene_compiler: ContractIdentity,
    pub extent: ProjectionExtent,
    pub objects: Vec<ProjectionObject>,
    pub validity: Vec<ProjectionValidityRegion>,
    pub field_channels: Vec<ProjectionFieldChannel>,
    pub layers: Vec<ProjectionLayer>,
    pub excluded_source_relationships: u64,
    pub limits: ProjectionLimits,
    pub complete: bool,
    pub degradation: Vec<ProjectionDegradation>,
    pub omissions: Vec<ProjectionOmission>,
    pub lineage: Vec<ProjectionLineage>,
}

impl ProjectionPacket {
    pub fn from_topography_patch(patch: &TopographyPatch) -> Result<Self, ProjectionError> {
        patch.verify()?;
        let limits = ProjectionLimits::default();
        let projection_basis = anchor_orientation_basis();
        let scene_compiler = topography_scene_compiler();

        let mut anchors = patch.anchors.iter().collect::<Vec<_>>();
        anchors.sort_by(|left, right| {
            let left_rank = u8::from(left.kind != TopographyAnchorKind::Workspace);
            let right_rank = u8::from(right.kind != TopographyAnchorKind::Workspace);
            (left_rank, left.coordinate.coordinate.as_str())
                .cmp(&(right_rank, right.coordinate.coordinate.as_str()))
        });
        let anchor_count = anchors.len();
        let mut objects = anchors
            .into_iter()
            .take(limits.max_anchor_objects as usize)
            .map(|anchor| ProjectionObject {
                object_id: format!("anchor:{}", anchor.anchor_id),
                source_id: anchor.anchor_id.clone(),
                kind: ProjectionObjectKind::Anchor,
                anchor_kind: Some(anchor.kind),
                frontier_status: None,
                coordinate: Some(anchor.coordinate.coordinate.clone()),
                label: anchor.label.clone(),
                detail: "admitted topography anchor".to_owned(),
                source_revision: anchor.source_revision.clone(),
            })
            .collect::<Vec<_>>();

        let mut frontier = patch.frontier.iter().collect::<Vec<_>>();
        frontier.sort_by(|left, right| left.row_id.cmp(&right.row_id));
        let frontier_count = frontier.len();
        objects.extend(
            frontier
                .into_iter()
                .take(limits.max_frontier_objects as usize)
                .map(|row| ProjectionObject {
                    object_id: format!("frontier:{}", row.row_id),
                    source_id: row.row_id.clone(),
                    kind: ProjectionObjectKind::Frontier,
                    anchor_kind: None,
                    frontier_status: Some(row.status),
                    coordinate: None,
                    label: row.locator.clone(),
                    detail: row.reason.clone(),
                    source_revision: patch.topography_revision.to_string(),
                }),
        );

        let validity = patch
            .regions
            .iter()
            .take(limits.max_validity_regions as usize)
            .map(|region| ProjectionValidityRegion {
                region_id: region.region_id.clone(),
                coordinate: region.coordinate.clone(),
                state: region.state,
                detail: region.detail.clone(),
                source_revision: patch.topography_revision.clone(),
            })
            .collect::<Vec<_>>();
        let field_channels = topography_field_channels(&patch.topography_revision);
        let layers = topography_layers(&patch.topography_revision);

        let folded_anchors = anchor_count.saturating_sub(limits.max_anchor_objects as usize) as u64;
        let folded_frontier =
            frontier_count.saturating_sub(limits.max_frontier_objects as usize) as u64;
        let folded_validity = patch
            .regions
            .len()
            .saturating_sub(limits.max_validity_regions as usize)
            as u64;
        let mut degradation = Vec::new();
        for (kind, omitted_count, reason) in [
            (
                "anchor_limit",
                folded_anchors,
                "anchor scene objects exceed the declared projection limit",
            ),
            (
                "frontier_limit",
                folded_frontier,
                "frontier scene objects exceed the declared projection limit",
            ),
            (
                "validity_limit",
                folded_validity,
                "validity regions exceed the declared projection limit",
            ),
        ] {
            if omitted_count > 0 {
                degradation.push(ProjectionDegradation {
                    kind: kind.to_owned(),
                    omitted_count,
                    reason: reason.to_owned(),
                });
            }
        }

        let mut omissions = patch
            .omissions
            .iter()
            .map(|omission| ProjectionOmission {
                kind: omission.kind.clone(),
                subject: omission.subject.clone(),
                omitted_count: omission.omitted_count,
                reason: omission.reason.clone(),
            })
            .collect::<Vec<_>>();
        omissions.extend([
            ProjectionOmission {
                kind: "semantic_boundary".to_owned(),
                subject: "relief".to_owned(),
                omitted_count: 0,
                reason: "relief height is admitted anchor-sample influence, not inferred semantic similarity".to_owned(),
            },
            ProjectionOmission {
                kind: "semantic_boundary".to_owned(),
                subject: "natural_features".to_owned(),
                omitted_count: 0,
                reason: "streams, rivers, weather fronts, and erosion are deterministic survey-field projections, not retained paths or source relationships".to_owned(),
            },
        ]);
        omissions.extend(degradation.iter().map(|item| ProjectionOmission {
            kind: item.kind.clone(),
            subject: "projection".to_owned(),
            omitted_count: item.omitted_count,
            reason: item.reason.clone(),
        }));
        omissions.sort_by(|left, right| {
            (&left.kind, &left.subject, &left.reason).cmp(&(
                &right.kind,
                &right.subject,
                &right.reason,
            ))
        });

        let mut lineage = patch
            .lineage
            .iter()
            .map(|item| ProjectionLineage {
                kind: format!("source_{}", item.kind),
                identity: item.identity.clone(),
                revision: item.revision.clone(),
            })
            .collect::<Vec<_>>();
        lineage.extend([
            ProjectionLineage {
                kind: "source_patch".to_owned(),
                identity: patch.patch_id.to_string(),
                revision: patch.topography_revision.to_string(),
            },
            contract_lineage("projection_basis", &projection_basis.contract),
            contract_lineage("scene_compiler", &scene_compiler),
        ]);
        lineage.extend(
            field_channels
                .iter()
                .map(|channel| contract_lineage("field_channel", &channel.implementation)),
        );
        lineage.sort_by(|left, right| {
            (&left.kind, &left.identity, &left.revision).cmp(&(
                &right.kind,
                &right.identity,
                &right.revision,
            ))
        });
        lineage.dedup_by(|left, right| left == right);

        let complete = patch.complete && degradation.is_empty();
        let mut packet = Self {
            schema: PROJECTION_PACKET_SCHEMA.to_owned(),
            packet_id: placeholder("rey.projection-packet.placeholder"),
            source_patch_id: patch.patch_id.clone(),
            source_topography_revision: patch.topography_revision.clone(),
            projection_basis,
            scene_compiler,
            extent: ProjectionExtent {
                width: TERRAIN_WIDTH,
                height: TERRAIN_HEIGHT,
                unit: "synthetic_scene_unit".to_owned(),
            },
            objects,
            validity,
            field_channels,
            layers,
            excluded_source_relationships: patch.edges.len() as u64,
            limits,
            complete,
            degradation,
            omissions,
            lineage,
        };
        packet.packet_id = packet_digest(&packet)?;
        packet.verify()?;
        Ok(packet)
    }

    pub fn verify(&self) -> Result<(), ProjectionError> {
        if self.schema != PROJECTION_PACKET_SCHEMA {
            return Err(ProjectionError::Schema);
        }
        validate_contract(&self.projection_basis.contract)?;
        validate_contract(&self.scene_compiler)?;
        validate_limits(&self.limits)?;
        if self.extent.width == 0 || self.extent.height == 0 || self.extent.unit.is_empty() {
            return Err(ProjectionError::Shape("extent"));
        }
        if self.projection_basis.input_dimensions.is_empty()
            || self.projection_basis.output_dimensions.is_empty()
            || self.projection_basis.normalization.is_empty()
            || self.projection_basis.distance_semantics.is_empty()
            || self.projection_basis.neighborhood_semantics.is_empty()
            || self.projection_basis.distortion.is_empty()
            || self.projection_basis.stable_coordinate_rule.is_empty()
        {
            return Err(ProjectionError::Shape("projection basis"));
        }
        unique(
            self.projection_basis.input_dimensions.iter(),
            "input dimension",
        )?;
        unique(
            self.projection_basis.output_dimensions.iter(),
            "output dimension",
        )?;
        enforce(
            "objects",
            self.objects.len(),
            self.limits.max_anchor_objects + self.limits.max_frontier_objects,
        )?;
        enforce(
            "validity regions",
            self.validity.len(),
            self.limits.max_validity_regions,
        )?;
        enforce(
            "field channels",
            self.field_channels.len(),
            self.limits.max_field_channels,
        )?;
        enforce("layers", self.layers.len(), self.limits.max_layers)?;
        enforce("omissions", self.omissions.len(), self.limits.max_omissions)?;
        unique(self.objects.iter().map(|item| &item.object_id), "object")?;
        unique(
            self.validity.iter().map(|item| &item.region_id),
            "validity region",
        )?;
        unique(
            self.field_channels.iter().map(|item| &item.id),
            "field channel",
        )?;
        unique(self.layers.iter().map(|item| &item.id), "layer")?;
        unique(
            self.lineage
                .iter()
                .map(|item| (&item.kind, &item.identity, &item.revision)),
            "lineage",
        )?;
        for object in &self.objects {
            if object.object_id.is_empty()
                || object.label.is_empty()
                || object.detail.is_empty()
                || object.source_revision.is_empty()
            {
                return Err(ProjectionError::Shape("object"));
            }
            match object.kind {
                ProjectionObjectKind::Anchor
                    if object.anchor_kind.is_none()
                        || object.frontier_status.is_some()
                        || object.coordinate.is_none() =>
                {
                    return Err(ProjectionError::Shape("anchor object"));
                }
                ProjectionObjectKind::Frontier
                    if object.anchor_kind.is_some()
                        || object.frontier_status.is_none()
                        || object.coordinate.is_some() =>
                {
                    return Err(ProjectionError::Shape("frontier object"));
                }
                _ => {}
            }
        }
        for region in &self.validity {
            if region.coordinate.is_empty() || region.detail.is_empty() {
                return Err(ProjectionError::Shape("validity region"));
            }
        }
        for channel in &self.field_channels {
            validate_contract(&channel.implementation)?;
            if channel.id.is_empty()
                || channel.semantics.is_empty()
                || channel.units.is_empty()
                || channel.normalization.is_empty()
            {
                return Err(ProjectionError::Shape("field channel"));
            }
        }
        for layer in &self.layers {
            if layer.id.is_empty() || layer.semantics.is_empty() {
                return Err(ProjectionError::Shape("layer"));
            }
        }
        for omission in &self.omissions {
            if omission.kind.is_empty() || omission.subject.is_empty() || omission.reason.is_empty()
            {
                return Err(ProjectionError::Shape("omission"));
            }
        }
        for item in &self.degradation {
            if item.kind.is_empty() || item.omitted_count == 0 || item.reason.is_empty() {
                return Err(ProjectionError::Shape("degradation"));
            }
        }
        if self.complete && !self.degradation.is_empty() {
            return Err(ProjectionError::Completeness);
        }
        if packet_digest(self)? != self.packet_id {
            return Err(ProjectionError::Digest);
        }
        Ok(())
    }

    pub fn verify_for(&self, patch: &TopographyPatch) -> Result<(), ProjectionError> {
        patch.verify()?;
        self.verify()?;
        if self != &Self::from_topography_patch(patch)? {
            return Err(ProjectionError::SourceBinding);
        }
        Ok(())
    }
}

fn anchor_orientation_basis() -> ProjectionBasis {
    let mut parameters = BTreeMap::new();
    parameters.insert("terrain_width".to_owned(), TERRAIN_WIDTH.to_string());
    parameters.insert("terrain_height".to_owned(), TERRAIN_HEIGHT.to_string());
    parameters.insert("grid_columns".to_owned(), TERRAIN_GRID_COLUMNS.to_string());
    parameters.insert("grid_rows".to_owned(), TERRAIN_GRID_ROWS.to_string());
    parameters.insert(
        "anchor_layout".to_owned(),
        "workspace_centered_rings".to_owned(),
    );
    parameters.insert("anchor_jitter".to_owned(), "fnv1a_coordinate".to_owned());
    ProjectionBasis {
        contract: ContractIdentity::new(
            "rey.projection.anchor-orientation",
            1,
            "synthetic workspace-centered anchor rings with deterministic coordinate jitter",
        ),
        input_dimensions: vec![
            "anchor.coordinate".to_owned(),
            "anchor.kind".to_owned(),
            "anchor.sample_count".to_owned(),
            "frontier.status".to_owned(),
            "region.validity".to_owned(),
        ],
        output_dimensions: vec![
            "scene_x".to_owned(),
            "scene_y".to_owned(),
            "relative_elevation".to_owned(),
        ],
        parameters,
        normalization: "per-chart relative anchor prominence".to_owned(),
        random_seed: None,
        distance_semantics: "synthetic orientation distance; not language or semantic distance"
            .to_owned(),
        neighborhood_semantics: "ordered anchor rings; no source relationship implied".to_owned(),
        distortion: "ring placement and bounded folding distort source-space distance".to_owned(),
        stable_coordinate_rule: "semantic coordinates remain stable; scene placement changes only with admitted anchor order or basis revision".to_owned(),
    }
}

fn topography_scene_compiler() -> ContractIdentity {
    ContractIdentity::new(
        "rey.projection.topography-scene",
        1,
        "compile admitted anchor objects, validity, relative relief, weather, runoff, erosion, and overlays without source-edge geography",
    )
}

fn topography_field_channels(revision: &SemanticDigest) -> Vec<ProjectionFieldChannel> {
    [
        (
            "validity",
            ProjectionFieldKind::Mask,
            "per-region admitted survey validity",
            "topography_region_state",
            "categorical",
            "rey.projection.survey-validity",
            "project exact topography region states without interpolation",
        ),
        (
            "elevation",
            ProjectionFieldKind::Scalar,
            "anchor-only relative prominence with runoff erosion",
            "relative_anchor_prominence",
            "per-chart maximum",
            "rey.projection.anchor-relief-field",
            "gaussian anchor influence with bounded deterministic erosion",
        ),
        (
            "rainfall",
            ProjectionFieldKind::Scalar,
            "survey-atmosphere pressure over admitted anchor relief",
            "relative_precipitation",
            "per-chart maximum",
            "rey.projection.survey-atmosphere",
            "deterministic rainfall from sampled and unresolved survey conditions",
        ),
        (
            "flow_direction",
            ProjectionFieldKind::Vector,
            "eight-neighbor downslope direction",
            "grid_offset",
            "unit grid direction",
            "rey.projection.anchor-hydrology",
            "deterministic downslope selection over hydraulic height",
        ),
        (
            "flow_accumulation",
            ProjectionFieldKind::Scalar,
            "accumulated projected runoff",
            "relative_runoff",
            "per-chart maximum",
            "rey.projection.anchor-hydrology",
            "deterministic accumulation over the flow-direction field",
        ),
        (
            "erosion",
            ProjectionFieldKind::Scalar,
            "displayed relief removed by accumulated runoff",
            "relative_elevation_delta",
            "per-chart maximum",
            "rey.projection.anchor-hydrology",
            "bounded deterministic erosion; no source assessment change",
        ),
    ]
    .into_iter()
    .map(
        |(id, kind, semantics, units, normalization, implementation, definition)| {
            ProjectionFieldChannel {
                id: id.to_owned(),
                kind,
                semantics: semantics.to_owned(),
                units: units.to_owned(),
                normalization: normalization.to_owned(),
                source_revision: revision.clone(),
                implementation: ContractIdentity::new(implementation, 1, definition),
            }
        },
    )
    .collect()
}

fn topography_layers(revision: &SemanticDigest) -> Vec<ProjectionLayer> {
    [
        (
            "validity",
            ProjectionLayerAuthority::Evidence,
            "surveyed, unexplored, omitted, stale, unsupported, and frontier boundaries",
        ),
        (
            "relief",
            ProjectionLayerAuthority::Derived,
            "anchor-only relative terrain and contours",
        ),
        (
            "water",
            ProjectionLayerAuthority::Derived,
            "projected runoff streams, rivers, and erosion",
        ),
        (
            "weather",
            ProjectionLayerAuthority::Derived,
            "unresolved survey conditions as atmospheric fronts",
        ),
        (
            "anchors",
            ProjectionLayerAuthority::Evidence,
            "stable admitted anchor points of interest",
        ),
        (
            "probes",
            ProjectionLayerAuthority::Evidence,
            "read-only prerequisites at the admitted survey frontier",
        ),
        (
            "labels",
            ProjectionLayerAuthority::Presentation,
            "redundant accessible labels and exact evidence links",
        ),
    ]
    .into_iter()
    .map(|(id, authority, semantics)| ProjectionLayer {
        id: id.to_owned(),
        authority,
        semantics: semantics.to_owned(),
        source_revision: revision.clone(),
    })
    .collect()
}

fn contract_lineage(kind: &str, contract: &ContractIdentity) -> ProjectionLineage {
    ProjectionLineage {
        kind: kind.to_owned(),
        identity: format!("{}@{}", contract.id, contract.revision),
        revision: contract.semantic_digest.to_string(),
    }
}

fn packet_digest(packet: &ProjectionPacket) -> Result<SemanticDigest, ProjectionError> {
    let mut normalized = packet.clone();
    normalized.packet_id = placeholder("rey.projection-packet.placeholder");
    let bytes = serde_json::to_vec(&normalized)?;
    let mut hasher = SemanticHasher::new(PROJECTION_PACKET_SCHEMA);
    hasher.add_bytes(&bytes);
    Ok(hasher.finish())
}

fn placeholder(domain: &str) -> SemanticDigest {
    SemanticHasher::new(domain).finish()
}

fn validate_contract(contract: &ContractIdentity) -> Result<(), ProjectionError> {
    if contract.id.is_empty()
        || contract.revision == 0
        || contract.semantic_digest.as_str().is_empty()
    {
        return Err(ProjectionError::Contract);
    }
    Ok(())
}

fn validate_limits(limits: &ProjectionLimits) -> Result<(), ProjectionError> {
    if [
        limits.max_anchor_objects,
        limits.max_frontier_objects,
        limits.max_validity_regions,
        limits.max_field_channels,
        limits.max_layers,
        limits.max_omissions,
        limits.max_field_cells,
        limits.max_field_bytes,
        limits.max_contours,
        limits.max_natural_features,
        limits.max_labels,
    ]
    .contains(&0)
    {
        return Err(ProjectionError::Limit);
    }
    Ok(())
}

fn enforce(role: &'static str, actual: usize, limit: u64) -> Result<(), ProjectionError> {
    if actual as u64 > limit {
        return Err(ProjectionError::Count {
            role,
            limit,
            actual: actual as u64,
        });
    }
    Ok(())
}

fn unique<T: Ord>(
    values: impl IntoIterator<Item = T>,
    role: &'static str,
) -> Result<(), ProjectionError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(ProjectionError::Duplicate(role));
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum ProjectionError {
    #[error("unsupported projection packet schema")]
    Schema,
    #[error("invalid projection contract")]
    Contract,
    #[error("invalid projection {0} shape")]
    Shape(&'static str),
    #[error("invalid projection limit")]
    Limit,
    #[error("projection {role} count limit {limit} exceeded by {actual}")]
    Count {
        role: &'static str,
        limit: u64,
        actual: u64,
    },
    #[error("duplicate projection {0}")]
    Duplicate(&'static str),
    #[error("projection completeness conflicts with degradation")]
    Completeness,
    #[error("projection packet source binding mismatch")]
    SourceBinding,
    #[error("projection packet digest mismatch")]
    Digest,
    #[error(transparent)]
    Topography(#[from] crate::TopographyError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        SurveySeed, SurveySeedState, TopographyCoverage, TopographyLimits, TopographyLineage,
        TopographyPatchParts, TopographyRegion, anchor_identity, region_identity,
    };
    use rey_locator::{CoordinateBinding, CoordinateIdentityClass, LocalCoordinate};

    fn contract(id: &str) -> ContractIdentity {
        ContractIdentity::new(id, 1, id)
    }

    fn fixture_patch(anchor_count: usize) -> TopographyPatch {
        let provider = contract("rey.provider.local-worktree");
        let mut anchors = Vec::new();
        for index in 0..anchor_count {
            let revision = format!("blake3:source-{index}");
            let coordinate = CoordinateBinding::local(
                provider.clone(),
                LocalCoordinate::new(
                    if index == 0 { "workspace" } else { "file" },
                    format!("fixture-{index}"),
                    &revision,
                    BTreeMap::new(),
                )
                .unwrap(),
                CoordinateIdentityClass::RevisionBound,
                &revision,
            )
            .unwrap();
            let kind = if index == 0 {
                TopographyAnchorKind::Workspace
            } else {
                TopographyAnchorKind::File
            };
            anchors.push(crate::TopographyAnchor {
                anchor_id: anchor_identity(&coordinate, kind),
                coordinate,
                kind,
                label: format!("fixture {index}"),
                source_revision: revision,
            });
        }
        let workspace = anchors[0].coordinate.clone();
        let region_coordinate = workspace.coordinate.clone();
        TopographyPatch::from_parts(
            TopographyPatchParts {
                workload: contract("workload"),
                graph: contract("graph"),
                scenario: None,
                campaign_id: placeholder("campaign"),
                execution_id: placeholder("execution"),
                operation: contract("operation"),
                implementation: contract("implementation"),
                provider,
                capability_snapshot_id: placeholder("capability"),
                limits: TopographyLimits::default(),
                complete: true,
                seeds: vec![SurveySeed {
                    path: "AGENTS.md".to_owned(),
                    state: SurveySeedState::SurveyedEmpty,
                    source_revision: Some("blake3:source-0".to_owned()),
                    logical_bytes: 10,
                    coordinate: Some(workspace),
                    candidate_count: 0,
                    detail: "surveyed exact seed".to_owned(),
                }],
                candidates: Vec::new(),
                resolutions: Vec::new(),
                anchors,
                edges: Vec::new(),
                regions: vec![TopographyRegion {
                    region_id: region_identity(
                        &region_coordinate,
                        TopographyRegionState::Unexplored,
                    ),
                    coordinate: region_coordinate,
                    state: TopographyRegionState::Unexplored,
                    surveyed_seeds: 0,
                    candidate_count: 0,
                    detail: "outside admitted survey".to_owned(),
                }],
                coverage: TopographyCoverage {
                    requested_seeds: 1,
                    surveyed_seeds: 1,
                    surveyed_empty_seeds: 1,
                    missing_seeds: 0,
                    omitted_seeds: 0,
                    candidates: 0,
                    unique_candidates: 0,
                    resolved_candidates: 0,
                    unresolved_candidates: 0,
                },
                frontier: Vec::new(),
                omissions: Vec::new(),
                lineage: vec![TopographyLineage {
                    kind: "provider".to_owned(),
                    identity: "local".to_owned(),
                    revision: "1".to_owned(),
                }],
            },
            None,
        )
        .unwrap()
    }

    #[test]
    fn packet_is_content_identified_and_excludes_source_relationship_geometry() {
        let patch = fixture_patch(3);
        let packet = ProjectionPacket::from_topography_patch(&patch).unwrap();
        packet.verify_for(&patch).unwrap();
        assert_eq!(packet.schema, PROJECTION_PACKET_SCHEMA);
        assert_eq!(packet.source_patch_id, patch.patch_id);
        assert_eq!(packet.objects.len(), 3);
        assert_eq!(packet.validity[0].state, TopographyRegionState::Unexplored);
        assert_eq!(packet.excluded_source_relationships, 0);
        assert!(
            packet
                .layers
                .iter()
                .all(|layer| layer.id != "relationships")
        );
        assert!(
            packet
                .projection_basis
                .distance_semantics
                .contains("not language or semantic distance")
        );

        let encoded = serde_json::to_vec(&packet).unwrap();
        let decoded: ProjectionPacket = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, packet);
    }

    #[test]
    fn packet_folds_objects_visibly_and_rejects_tampering() {
        let patch = fixture_patch(70);
        let mut packet = ProjectionPacket::from_topography_patch(&patch).unwrap();
        assert_eq!(
            packet.objects.len() as u64,
            packet.limits.max_anchor_objects
        );
        assert!(!packet.complete);
        assert_eq!(packet.degradation[0].kind, "anchor_limit");
        assert_eq!(packet.degradation[0].omitted_count, 6);

        packet.extent.width += 1;
        assert!(matches!(
            packet.verify_for(&patch),
            Err(ProjectionError::Digest)
        ));

        let mut self_consistent = ProjectionPacket::from_topography_patch(&patch).unwrap();
        self_consistent.objects.pop();
        self_consistent.packet_id = packet_digest(&self_consistent).unwrap();
        assert!(matches!(
            self_consistent.verify_for(&patch),
            Err(ProjectionError::SourceBinding)
        ));

        let mut relabeled = ProjectionPacket::from_topography_patch(&patch).unwrap();
        relabeled.objects[0].label = "invented label".to_owned();
        relabeled.packet_id = packet_digest(&relabeled).unwrap();
        assert!(matches!(
            relabeled.verify_for(&patch),
            Err(ProjectionError::SourceBinding)
        ));
    }
}
