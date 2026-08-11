use std::collections::{BTreeMap, BTreeSet};

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_locator::{CoordinateBinding, LocatorResolution, ResolutionStatus};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const TOPOGRAPHY_PATCH_SCHEMA: &str = "rey.topography-patch.v1";
pub const TOPOGRAPHY_PATCH_DELTA_SCHEMA: &str = "rey.topography-patch-delta.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopographyLimits {
    pub max_seeds: u64,
    pub max_seed_bytes: u64,
    pub max_total_bytes: u64,
    pub max_candidates: u64,
    pub max_anchors: u64,
    pub max_edges: u64,
    pub max_regions: u64,
    pub max_frontier: u64,
    pub max_omissions: u64,
}

impl Default for TopographyLimits {
    fn default() -> Self {
        Self {
            max_seeds: 32,
            max_seed_bytes: 1_048_576,
            max_total_bytes: 8 * 1_048_576,
            max_candidates: 4_096,
            max_anchors: 4_096,
            max_edges: 8_192,
            max_regions: 256,
            max_frontier: 4_096,
            max_omissions: 1_024,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurveySeedState {
    Surveyed,
    SurveyedEmpty,
    Missing,
    Omitted,
    Unsupported,
}

impl SurveySeedState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Surveyed => "surveyed",
            Self::SurveyedEmpty => "surveyed_empty",
            Self::Missing => "missing",
            Self::Omitted => "omitted",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurveySeed {
    pub path: String,
    pub state: SurveySeedState,
    pub source_revision: Option<String>,
    pub logical_bytes: u64,
    pub coordinate: Option<CoordinateBinding>,
    pub candidate_count: u64,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopographyLocatorCandidate {
    pub candidate_id: SemanticDigest,
    pub seed_coordinate: String,
    pub seed_revision: String,
    pub raw: String,
    pub start_byte: u64,
    pub end_byte: u64,
    pub relationship: String,
    pub duplicate: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TopographyAnchorKind {
    Workspace,
    File,
    Document,
    ExternalResource,
}

impl TopographyAnchorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::File => "file",
            Self::Document => "document",
            Self::ExternalResource => "external_resource",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopographyAnchor {
    pub anchor_id: SemanticDigest,
    pub coordinate: CoordinateBinding,
    pub kind: TopographyAnchorKind,
    pub label: String,
    pub source_revision: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TopographyRelationshipKind {
    Contains,
    References,
}

impl TopographyRelationshipKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::References => "references",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopographyEdge {
    pub edge_id: SemanticDigest,
    pub source_coordinate: String,
    pub target_coordinate: String,
    pub kind: TopographyRelationshipKind,
    pub locator: String,
    pub evidence_revision: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TopographyRegionState {
    Surveyed,
    SurveyedEmpty,
    Unexplored,
    Omitted,
    Stale,
    Unsupported,
    Frontier,
}

impl TopographyRegionState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Surveyed => "surveyed",
            Self::SurveyedEmpty => "surveyed_empty",
            Self::Unexplored => "unexplored",
            Self::Omitted => "omitted",
            Self::Stale => "stale",
            Self::Unsupported => "unsupported",
            Self::Frontier => "frontier",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopographyRegion {
    pub region_id: SemanticDigest,
    pub coordinate: String,
    pub state: TopographyRegionState,
    pub surveyed_seeds: u64,
    pub candidate_count: u64,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopographyCoverage {
    pub requested_seeds: u64,
    pub surveyed_seeds: u64,
    pub surveyed_empty_seeds: u64,
    pub missing_seeds: u64,
    pub omitted_seeds: u64,
    pub candidates: u64,
    pub unique_candidates: u64,
    pub resolved_candidates: u64,
    pub unresolved_candidates: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopographyFrontierRow {
    pub row_id: SemanticDigest,
    pub source_coordinate: String,
    pub locator: String,
    pub status: ResolutionStatus,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopographyOmission {
    pub kind: String,
    pub subject: String,
    pub omitted_count: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopographyLineage {
    pub kind: String,
    pub identity: String,
    pub revision: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TopographyChangeKind {
    Inserted,
    Deleted,
    Modified,
}

impl TopographyChangeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inserted => "inserted",
            Self::Deleted => "deleted",
            Self::Modified => "modified",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopographyChange {
    pub object_kind: String,
    pub object_id: String,
    pub kind: TopographyChangeKind,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopographyPatchDelta {
    pub schema: String,
    pub delta_id: SemanticDigest,
    pub source_revision: SemanticDigest,
    pub target_revision: SemanticDigest,
    pub inserted: u64,
    pub deleted: u64,
    pub modified: u64,
    pub changes: Vec<TopographyChange>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopographyPatch {
    pub schema: String,
    pub patch_id: SemanticDigest,
    pub topography_revision: SemanticDigest,
    pub prior_topography_revision: SemanticDigest,
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario: Option<ContractIdentity>,
    pub campaign_id: SemanticDigest,
    pub execution_id: SemanticDigest,
    pub operation: ContractIdentity,
    pub implementation: ContractIdentity,
    pub provider: ContractIdentity,
    pub capability_snapshot_id: SemanticDigest,
    pub limits: TopographyLimits,
    pub complete: bool,
    pub seeds: Vec<SurveySeed>,
    pub candidates: Vec<TopographyLocatorCandidate>,
    pub resolutions: Vec<LocatorResolution>,
    pub anchors: Vec<TopographyAnchor>,
    pub edges: Vec<TopographyEdge>,
    pub regions: Vec<TopographyRegion>,
    pub coverage: TopographyCoverage,
    pub frontier: Vec<TopographyFrontierRow>,
    pub omissions: Vec<TopographyOmission>,
    pub lineage: Vec<TopographyLineage>,
    pub delta: TopographyPatchDelta,
}

#[derive(Clone, Debug)]
pub struct TopographyPatchParts {
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario: Option<ContractIdentity>,
    pub campaign_id: SemanticDigest,
    pub execution_id: SemanticDigest,
    pub operation: ContractIdentity,
    pub implementation: ContractIdentity,
    pub provider: ContractIdentity,
    pub capability_snapshot_id: SemanticDigest,
    pub limits: TopographyLimits,
    pub complete: bool,
    pub seeds: Vec<SurveySeed>,
    pub candidates: Vec<TopographyLocatorCandidate>,
    pub resolutions: Vec<LocatorResolution>,
    pub anchors: Vec<TopographyAnchor>,
    pub edges: Vec<TopographyEdge>,
    pub regions: Vec<TopographyRegion>,
    pub coverage: TopographyCoverage,
    pub frontier: Vec<TopographyFrontierRow>,
    pub omissions: Vec<TopographyOmission>,
    pub lineage: Vec<TopographyLineage>,
}

impl TopographyPatch {
    pub fn from_parts(
        mut parts: TopographyPatchParts,
        prior: Option<&Self>,
    ) -> Result<Self, TopographyError> {
        parts
            .seeds
            .sort_by(|left, right| left.path.cmp(&right.path));
        parts
            .candidates
            .sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
        parts
            .resolutions
            .sort_by(|left, right| left.resolution_id.cmp(&right.resolution_id));
        parts
            .anchors
            .sort_by(|left, right| left.coordinate.coordinate.cmp(&right.coordinate.coordinate));
        parts
            .edges
            .sort_by(|left, right| left.edge_id.cmp(&right.edge_id));
        parts
            .regions
            .sort_by(|left, right| left.region_id.cmp(&right.region_id));
        parts
            .frontier
            .sort_by(|left, right| left.row_id.cmp(&right.row_id));
        parts
            .omissions
            .sort_by(|left, right| (&left.kind, &left.subject).cmp(&(&right.kind, &right.subject)));
        parts.lineage.sort_by(|left, right| {
            (&left.kind, &left.identity, &left.revision).cmp(&(
                &right.kind,
                &right.identity,
                &right.revision,
            ))
        });
        let prior_revision = prior.map_or_else(empty_topography_revision, |patch| {
            patch.topography_revision.clone()
        });
        let mut patch = Self {
            schema: TOPOGRAPHY_PATCH_SCHEMA.to_owned(),
            patch_id: placeholder("rey.topography-patch.placeholder"),
            topography_revision: placeholder("rey.topography-revision.placeholder"),
            prior_topography_revision: prior_revision.clone(),
            workload: parts.workload,
            graph: parts.graph,
            scenario: parts.scenario,
            campaign_id: parts.campaign_id,
            execution_id: parts.execution_id,
            operation: parts.operation,
            implementation: parts.implementation,
            provider: parts.provider,
            capability_snapshot_id: parts.capability_snapshot_id,
            limits: parts.limits,
            complete: parts.complete,
            seeds: parts.seeds,
            candidates: parts.candidates,
            resolutions: parts.resolutions,
            anchors: parts.anchors,
            edges: parts.edges,
            regions: parts.regions,
            coverage: parts.coverage,
            frontier: parts.frontier,
            omissions: parts.omissions,
            lineage: parts.lineage,
            delta: TopographyPatchDelta {
                schema: TOPOGRAPHY_PATCH_DELTA_SCHEMA.to_owned(),
                delta_id: placeholder("rey.topography-patch-delta.placeholder"),
                source_revision: prior_revision,
                target_revision: placeholder("rey.topography-revision.placeholder"),
                inserted: 0,
                deleted: 0,
                modified: 0,
                changes: Vec::new(),
            },
        };
        patch.topography_revision = topography_revision_digest(&patch)?;
        patch.delta = compare_topography(prior, &patch)?;
        patch.patch_id = patch_digest(&patch)?;
        patch.verify()?;
        Ok(patch)
    }

    pub fn verify(&self) -> Result<(), TopographyError> {
        if self.schema != TOPOGRAPHY_PATCH_SCHEMA
            || self.delta.schema != TOPOGRAPHY_PATCH_DELTA_SCHEMA
        {
            return Err(TopographyError::Schema);
        }
        for contract in [
            &self.workload,
            &self.graph,
            &self.operation,
            &self.implementation,
            &self.provider,
        ] {
            validate_contract(contract)?;
        }
        if let Some(scenario) = &self.scenario {
            validate_contract(scenario)?;
        }
        validate_limits(&self.limits)?;
        enforce("seeds", self.seeds.len(), self.limits.max_seeds)?;
        enforce(
            "candidates",
            self.candidates.len(),
            self.limits.max_candidates,
        )?;
        enforce("anchors", self.anchors.len(), self.limits.max_anchors)?;
        enforce("edges", self.edges.len(), self.limits.max_edges)?;
        enforce("regions", self.regions.len(), self.limits.max_regions)?;
        enforce("frontier", self.frontier.len(), self.limits.max_frontier)?;
        enforce("omissions", self.omissions.len(), self.limits.max_omissions)?;
        unique(self.seeds.iter().map(|seed| seed.path.as_str()), "seed")?;
        unique(
            self.candidates
                .iter()
                .map(|candidate| candidate.candidate_id.as_str()),
            "candidate",
        )?;
        unique(
            self.resolutions
                .iter()
                .map(|resolution| resolution.resolution_id.as_str()),
            "resolution",
        )?;
        unique(
            self.anchors
                .iter()
                .map(|anchor| anchor.coordinate.coordinate.as_str()),
            "anchor",
        )?;
        unique(self.edges.iter().map(|edge| edge.edge_id.as_str()), "edge")?;
        unique(
            self.regions.iter().map(|region| region.region_id.as_str()),
            "region",
        )?;
        unique(
            self.frontier.iter().map(|row| row.row_id.as_str()),
            "frontier",
        )?;
        for seed in &self.seeds {
            if seed.path.is_empty() || seed.detail.is_empty() {
                return Err(TopographyError::Shape("seed"));
            }
            if let Some(coordinate) = &seed.coordinate {
                coordinate.verify()?;
            }
        }
        for resolution in &self.resolutions {
            resolution.verify()?;
        }
        for anchor in &self.anchors {
            anchor.coordinate.verify()?;
            if anchor.label.is_empty() || anchor.source_revision.is_empty() {
                return Err(TopographyError::Shape("anchor"));
            }
        }
        let anchor_coordinates = self
            .anchors
            .iter()
            .map(|anchor| anchor.coordinate.coordinate.as_str())
            .collect::<BTreeSet<_>>();
        for edge in &self.edges {
            if !anchor_coordinates.contains(edge.source_coordinate.as_str())
                || !anchor_coordinates.contains(edge.target_coordinate.as_str())
                || edge.locator.is_empty()
                || edge.evidence_revision.is_empty()
            {
                return Err(TopographyError::Shape("edge"));
            }
        }
        let expected_coverage = coverage_from(self);
        if expected_coverage != self.coverage {
            return Err(TopographyError::Coverage);
        }
        if self.complete
            && (!self.omissions.is_empty()
                || self
                    .seeds
                    .iter()
                    .any(|seed| seed.state == SurveySeedState::Omitted))
        {
            return Err(TopographyError::Completeness);
        }
        let actual_revision = topography_revision_digest(self)?;
        if actual_revision != self.topography_revision {
            return Err(TopographyError::Digest("topography revision"));
        }
        self.delta.verify()?;
        if self.delta.source_revision != self.prior_topography_revision
            || self.delta.target_revision != self.topography_revision
        {
            return Err(TopographyError::DeltaBinding);
        }
        let actual_patch = patch_digest(self)?;
        if actual_patch != self.patch_id {
            return Err(TopographyError::Digest("patch"));
        }
        Ok(())
    }
}

impl TopographyPatchDelta {
    pub fn verify(&self) -> Result<(), TopographyError> {
        if self.schema != TOPOGRAPHY_PATCH_DELTA_SCHEMA {
            return Err(TopographyError::Schema);
        }
        let inserted = self
            .changes
            .iter()
            .filter(|change| change.kind == TopographyChangeKind::Inserted)
            .count() as u64;
        let deleted = self
            .changes
            .iter()
            .filter(|change| change.kind == TopographyChangeKind::Deleted)
            .count() as u64;
        let modified = self
            .changes
            .iter()
            .filter(|change| change.kind == TopographyChangeKind::Modified)
            .count() as u64;
        if (inserted, deleted, modified) != (self.inserted, self.deleted, self.modified) {
            return Err(TopographyError::DeltaShape);
        }
        unique(
            self.changes
                .iter()
                .map(|change| format!("{}:{}", change.object_kind, change.object_id)),
            "change",
        )?;
        if delta_digest(self)? != self.delta_id {
            return Err(TopographyError::Digest("patch delta"));
        }
        Ok(())
    }
}

pub fn anchor_identity(
    coordinate: &CoordinateBinding,
    kind: TopographyAnchorKind,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.topography-anchor.v1");
    hasher.add_str(coordinate.binding_id.as_str());
    hasher.add_str(kind.as_str());
    hasher.finish()
}

pub fn edge_identity(
    source: &str,
    target: &str,
    kind: TopographyRelationshipKind,
    locator: &str,
    evidence_revision: &str,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.topography-edge.v1");
    hasher.add_str(source);
    hasher.add_str(target);
    hasher.add_str(kind.as_str());
    hasher.add_str(locator);
    hasher.add_str(evidence_revision);
    hasher.finish()
}

pub fn candidate_identity(
    seed_coordinate: &str,
    raw: &str,
    start_byte: u64,
    end_byte: u64,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.topography-locator-candidate.v1");
    hasher.add_str(seed_coordinate);
    hasher.add_str(raw);
    hasher.add_u64(start_byte);
    hasher.add_u64(end_byte);
    hasher.finish()
}

pub fn region_identity(coordinate: &str, state: TopographyRegionState) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.topography-region.v1");
    hasher.add_str(coordinate);
    hasher.add_str(state.as_str());
    hasher.finish()
}

pub fn frontier_identity(
    source_coordinate: &str,
    locator: &str,
    status: ResolutionStatus,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.topography-frontier-row.v1");
    hasher.add_str(source_coordinate);
    hasher.add_str(locator);
    hasher.add_str(status.as_str());
    hasher.finish()
}

#[must_use]
pub fn empty_topography_revision() -> SemanticDigest {
    SemanticHasher::new("rey.topography.empty.v1").finish()
}

fn coverage_from(patch: &TopographyPatch) -> TopographyCoverage {
    let seed_limit_omissions = patch
        .omissions
        .iter()
        .filter(|omission| omission.kind == "seed_limit")
        .map(|omission| omission.omitted_count)
        .sum::<u64>();
    let requested_seeds = patch.seeds.len() as u64 + seed_limit_omissions;
    let surveyed_seeds = patch
        .seeds
        .iter()
        .filter(|seed| {
            matches!(
                seed.state,
                SurveySeedState::Surveyed | SurveySeedState::SurveyedEmpty
            )
        })
        .count() as u64;
    let surveyed_empty_seeds = patch
        .seeds
        .iter()
        .filter(|seed| seed.state == SurveySeedState::SurveyedEmpty)
        .count() as u64;
    let missing_seeds = patch
        .seeds
        .iter()
        .filter(|seed| seed.state == SurveySeedState::Missing)
        .count() as u64;
    let omitted_seeds = patch
        .seeds
        .iter()
        .filter(|seed| seed.state == SurveySeedState::Omitted)
        .count() as u64
        + seed_limit_omissions;
    let unique_candidates = patch
        .candidates
        .iter()
        .map(|candidate| candidate.raw.as_str())
        .collect::<BTreeSet<_>>()
        .len() as u64;
    let resolved_candidates = patch
        .resolutions
        .iter()
        .filter(|resolution| resolution.status == ResolutionStatus::Resolved)
        .count() as u64;
    TopographyCoverage {
        requested_seeds,
        surveyed_seeds,
        surveyed_empty_seeds,
        missing_seeds,
        omitted_seeds,
        candidates: patch.candidates.len() as u64,
        unique_candidates,
        resolved_candidates,
        unresolved_candidates: patch.resolutions.len() as u64 - resolved_candidates,
    }
}

fn compare_topography(
    prior: Option<&TopographyPatch>,
    target: &TopographyPatch,
) -> Result<TopographyPatchDelta, TopographyError> {
    let before = object_projection(prior);
    let after = object_projection(Some(target));
    let mut changes = Vec::new();
    for key in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
        match (before.get(key), after.get(key)) {
            (None, Some(value)) => changes.push(change(
                key,
                TopographyChangeKind::Inserted,
                None,
                Some(value.clone()),
            )),
            (Some(value), None) => changes.push(change(
                key,
                TopographyChangeKind::Deleted,
                Some(value.clone()),
                None,
            )),
            (Some(left), Some(right)) if left != right => changes.push(change(
                key,
                TopographyChangeKind::Modified,
                Some(left.clone()),
                Some(right.clone()),
            )),
            _ => {}
        }
    }
    changes.sort_by(|left, right| {
        (&left.object_kind, &left.object_id).cmp(&(&right.object_kind, &right.object_id))
    });
    let mut delta = TopographyPatchDelta {
        schema: TOPOGRAPHY_PATCH_DELTA_SCHEMA.to_owned(),
        delta_id: placeholder("rey.topography-patch-delta.placeholder"),
        source_revision: prior.map_or_else(empty_topography_revision, |patch| {
            patch.topography_revision.clone()
        }),
        target_revision: target.topography_revision.clone(),
        inserted: changes
            .iter()
            .filter(|change| change.kind == TopographyChangeKind::Inserted)
            .count() as u64,
        deleted: changes
            .iter()
            .filter(|change| change.kind == TopographyChangeKind::Deleted)
            .count() as u64,
        modified: changes
            .iter()
            .filter(|change| change.kind == TopographyChangeKind::Modified)
            .count() as u64,
        changes,
    };
    delta.delta_id = delta_digest(&delta)?;
    delta.verify()?;
    Ok(delta)
}

fn object_projection(patch: Option<&TopographyPatch>) -> BTreeMap<String, String> {
    let mut objects = BTreeMap::new();
    let Some(patch) = patch else { return objects };
    for anchor in &patch.anchors {
        objects.insert(
            format!("anchor:{}", anchor.coordinate.coordinate),
            format!(
                "{}:{}:{}",
                anchor.kind.as_str(),
                anchor.label,
                anchor.source_revision
            ),
        );
    }
    for edge in &patch.edges {
        objects.insert(
            format!("edge:{}", edge.edge_id),
            format!(
                "{}:{}:{}:{}",
                edge.source_coordinate,
                edge.target_coordinate,
                edge.kind.as_str(),
                edge.locator
            ),
        );
    }
    for region in &patch.regions {
        objects.insert(
            format!("region:{}", region.region_id),
            format!(
                "{}:{}:{}:{}",
                region.coordinate,
                region.state.as_str(),
                region.surveyed_seeds,
                region.candidate_count
            ),
        );
    }
    for row in &patch.frontier {
        objects.insert(
            format!("frontier:{}", row.row_id),
            format!(
                "{}:{}:{}",
                row.source_coordinate,
                row.locator,
                row.status.as_str()
            ),
        );
    }
    objects
}

fn change(
    key: &str,
    kind: TopographyChangeKind,
    before: Option<String>,
    after: Option<String>,
) -> TopographyChange {
    let (object_kind, object_id) = key.split_once(':').unwrap_or(("object", key));
    TopographyChange {
        object_kind: object_kind.to_owned(),
        object_id: object_id.to_owned(),
        kind,
        before,
        after,
    }
}

fn topography_revision_digest(patch: &TopographyPatch) -> Result<SemanticDigest, TopographyError> {
    let mut normalized = patch.clone();
    normalized.patch_id = placeholder("rey.topography-patch.placeholder");
    normalized.topography_revision = placeholder("rey.topography-revision.placeholder");
    normalized.prior_topography_revision = empty_topography_revision();
    normalized.workload = ContractIdentity::new(
        "rey.topography-revision.workload-lineage",
        1,
        "excluded from map identity",
    );
    normalized.graph = ContractIdentity::new(
        "rey.topography-revision.graph-lineage",
        1,
        "excluded from map identity",
    );
    normalized.scenario = None;
    normalized.campaign_id = placeholder("rey.topography-revision.campaign-lineage");
    normalized.execution_id = placeholder("rey.topography-revision.execution-lineage");
    normalized.delta = TopographyPatchDelta {
        schema: TOPOGRAPHY_PATCH_DELTA_SCHEMA.to_owned(),
        delta_id: placeholder("rey.topography-patch-delta.placeholder"),
        source_revision: empty_topography_revision(),
        target_revision: placeholder("rey.topography-revision.placeholder"),
        inserted: 0,
        deleted: 0,
        modified: 0,
        changes: Vec::new(),
    };
    semantic_json("rey.topography-revision.v1", &normalized)
}

fn patch_digest(patch: &TopographyPatch) -> Result<SemanticDigest, TopographyError> {
    let mut normalized = patch.clone();
    normalized.patch_id = placeholder("rey.topography-patch.placeholder");
    semantic_json(TOPOGRAPHY_PATCH_SCHEMA, &normalized)
}

fn delta_digest(delta: &TopographyPatchDelta) -> Result<SemanticDigest, TopographyError> {
    let mut normalized = delta.clone();
    normalized.delta_id = placeholder("rey.topography-patch-delta.placeholder");
    semantic_json(TOPOGRAPHY_PATCH_DELTA_SCHEMA, &normalized)
}

fn semantic_json(domain: &str, value: &impl Serialize) -> Result<SemanticDigest, TopographyError> {
    let bytes = serde_json::to_vec(value)?;
    let mut hasher = SemanticHasher::new(domain);
    hasher.add_bytes(&bytes);
    Ok(hasher.finish())
}

fn placeholder(domain: &str) -> SemanticDigest {
    SemanticHasher::new(domain).finish()
}

fn validate_contract(contract: &ContractIdentity) -> Result<(), TopographyError> {
    if contract.id.is_empty()
        || contract.revision == 0
        || contract.semantic_digest.as_str().is_empty()
    {
        return Err(TopographyError::Contract);
    }
    Ok(())
}

fn validate_limits(limits: &TopographyLimits) -> Result<(), TopographyError> {
    if [
        limits.max_seeds,
        limits.max_seed_bytes,
        limits.max_total_bytes,
        limits.max_candidates,
        limits.max_anchors,
        limits.max_edges,
        limits.max_regions,
        limits.max_frontier,
        limits.max_omissions,
    ]
    .contains(&0)
    {
        return Err(TopographyError::Limit("zero"));
    }
    Ok(())
}

fn enforce(role: &'static str, actual: usize, limit: u64) -> Result<(), TopographyError> {
    if actual as u64 > limit {
        return Err(TopographyError::Count {
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
) -> Result<(), TopographyError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(TopographyError::Duplicate(role));
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum TopographyError {
    #[error("unsupported topography schema")]
    Schema,
    #[error("invalid topography contract")]
    Contract,
    #[error("invalid {0} topography shape")]
    Shape(&'static str),
    #[error("invalid topography {0} limit")]
    Limit(&'static str),
    #[error("topography {role} count limit {limit} exceeded by {actual}")]
    Count {
        role: &'static str,
        limit: u64,
        actual: u64,
    },
    #[error("duplicate topography {0}")]
    Duplicate(&'static str),
    #[error("topography coverage does not match retained evidence")]
    Coverage,
    #[error("topography completeness conflicts with omissions")]
    Completeness,
    #[error("topography delta binding mismatch")]
    DeltaBinding,
    #[error("topography delta summary mismatch")]
    DeltaShape,
    #[error("topography {0} digest mismatch")]
    Digest(&'static str),
    #[error(transparent)]
    Locator(#[from] rey_locator::LocatorError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rey_locator::{CoordinateIdentityClass, LocalCoordinate};

    fn contract(id: &str) -> ContractIdentity {
        ContractIdentity::new(id, 1, id)
    }

    fn patch(prior: Option<&TopographyPatch>, label: &str) -> TopographyPatch {
        let provider = contract("rey.provider.local-worktree");
        let revision = format!("blake3:{label}");
        let coordinate = CoordinateBinding::local(
            provider.clone(),
            LocalCoordinate::new("file", "README.md", &revision, BTreeMap::new()).unwrap(),
            CoordinateIdentityClass::RevisionBound,
            &revision,
        )
        .unwrap();
        let anchor = TopographyAnchor {
            anchor_id: anchor_identity(&coordinate, TopographyAnchorKind::File),
            coordinate,
            kind: TopographyAnchorKind::File,
            label: label.to_owned(),
            source_revision: revision.clone(),
        };
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
                    path: "README.md".to_owned(),
                    state: SurveySeedState::SurveyedEmpty,
                    source_revision: Some(revision),
                    logical_bytes: 10,
                    coordinate: Some(anchor.coordinate.clone()),
                    candidate_count: 0,
                    detail: "surveyed exact seed".to_owned(),
                }],
                candidates: Vec::new(),
                resolutions: Vec::new(),
                anchors: vec![anchor],
                edges: Vec::new(),
                regions: Vec::new(),
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
            prior,
        )
        .unwrap()
    }

    #[test]
    fn patch_is_content_identified_and_delta_is_directed() {
        let first = patch(None, "first");
        assert_eq!(first.delta.source_revision, empty_topography_revision());
        assert_eq!(first.delta.target_revision, first.topography_revision);
        assert_eq!(first.delta.inserted, 1);
        first.verify().unwrap();

        let second = patch(Some(&first), "second");
        assert_eq!(second.prior_topography_revision, first.topography_revision);
        assert!(second.delta.inserted > 0);
        assert!(second.delta.deleted > 0);
        second.verify().unwrap();
    }

    #[test]
    fn tampering_and_false_completeness_fail_closed() {
        let mut tampered = patch(None, "clean");
        tampered.coverage.requested_seeds += 1;
        assert!(tampered.verify().is_err());

        let mut incomplete = patch(None, "clean");
        incomplete.omissions.push(TopographyOmission {
            kind: "bound".to_owned(),
            subject: "README.md".to_owned(),
            omitted_count: 1,
            reason: "fixture".to_owned(),
        });
        assert!(incomplete.verify().is_err());
    }
}
