use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{TopographyAnchorKind, TopographyPatch};

pub const SEMANTIC_ATLAS_SCHEMA: &str = "rey.semantic-atlas.v1";
const MICRODEGREES_PER_DEGREE: f64 = 1_000_000.0;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticAtlasLimits {
    pub max_regions: u64,
    pub max_world_clusters: u64,
    pub max_members_per_cluster: u64,
    pub max_omissions: u64,
}

impl Default for SemanticAtlasLimits {
    fn default() -> Self {
        Self {
            max_regions: 128,
            max_world_clusters: 16,
            max_members_per_cluster: 128,
            max_omissions: 32,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticAtlasCoordinateSystem {
    pub kind: String,
    pub axes: Vec<String>,
    pub unit: String,
    pub longitude_range_microdegrees: [i64; 2],
    pub latitude_range_microdegrees: [i64; 2],
    pub wraps_longitude: bool,
    pub authority: String,
    pub earth_crs: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticAtlasLayoutPolicy {
    pub clustering: String,
    pub placement: String,
    pub recluster_trigger: String,
    pub zoom_rule: String,
    pub distance_claim: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticAtlasSource {
    pub region_id: SemanticDigest,
    pub workload_id: String,
    pub source_patch_id: SemanticDigest,
    pub source_topography_revision: SemanticDigest,
    pub complete: bool,
    pub workspace_anchors: u64,
    pub file_anchors: u64,
    pub document_anchors: u64,
    pub external_resource_anchors: u64,
    pub requested_seeds: u64,
    pub surveyed_seeds: u64,
    pub candidates: u64,
    pub frontier_rows: u64,
}

impl SemanticAtlasSource {
    #[must_use]
    pub fn from_topography(workload_id: &str, patch: &TopographyPatch) -> Self {
        let mut anchor_counts = BTreeMap::new();
        for anchor in &patch.anchors {
            *anchor_counts.entry(anchor.kind).or_insert(0_u64) += 1;
        }
        let mut hasher = SemanticHasher::new("rey.semantic-atlas-region.v1");
        hasher.add_str(workload_id);
        Self {
            region_id: hasher.finish(),
            workload_id: workload_id.to_owned(),
            source_patch_id: patch.patch_id.clone(),
            source_topography_revision: patch.topography_revision.clone(),
            complete: patch.complete,
            workspace_anchors: *anchor_counts
                .get(&TopographyAnchorKind::Workspace)
                .unwrap_or(&0),
            file_anchors: *anchor_counts.get(&TopographyAnchorKind::File).unwrap_or(&0),
            document_anchors: *anchor_counts
                .get(&TopographyAnchorKind::Document)
                .unwrap_or(&0),
            external_resource_anchors: *anchor_counts
                .get(&TopographyAnchorKind::ExternalResource)
                .unwrap_or(&0),
            requested_seeds: patch.coverage.requested_seeds,
            surveyed_seeds: patch.coverage.surveyed_seeds,
            candidates: patch.coverage.unique_candidates,
            frontier_rows: patch.frontier.len() as u64,
        }
    }

    fn feature_vector(&self) -> [u64; 8] {
        let anchors = self
            .workspace_anchors
            .saturating_add(self.file_anchors)
            .saturating_add(self.document_anchors)
            .saturating_add(self.external_resource_anchors)
            .max(1);
        let ratio = |value: u64, denominator: u64| {
            value
                .saturating_mul(10_000)
                .saturating_div(denominator.max(1))
        };
        [
            ratio(self.workspace_anchors, anchors),
            ratio(self.file_anchors, anchors),
            ratio(self.document_anchors, anchors),
            ratio(self.external_resource_anchors, anchors),
            ratio(self.surveyed_seeds, self.requested_seeds),
            ratio(self.candidates, self.requested_seeds).min(100_000),
            ratio(self.frontier_rows, self.candidates.max(1)).min(100_000),
            if self.complete { 10_000 } else { 0 },
        ]
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticAtlasRegion {
    pub region_id: SemanticDigest,
    pub cluster_id: SemanticDigest,
    pub workload_id: String,
    pub source_patch_id: SemanticDigest,
    pub source_topography_revision: SemanticDigest,
    pub semantic_longitude_microdegrees: i64,
    pub semantic_latitude_microdegrees: i64,
    pub angular_radius_microdegrees: u64,
    pub anchor_count: u64,
    pub frontier_rows: u64,
    pub complete: bool,
    pub dominant_feature: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticAtlasCluster {
    pub cluster_id: SemanticDigest,
    pub semantic_longitude_microdegrees: i64,
    pub semantic_latitude_microdegrees: i64,
    pub angular_radius_microdegrees: u64,
    pub member_region_ids: Vec<SemanticDigest>,
    pub dominant_feature: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticAtlasOmission {
    pub kind: String,
    pub omitted_count: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticAtlasLineage {
    pub kind: String,
    pub identity: String,
    pub revision: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticAtlas {
    pub schema: String,
    pub atlas_id: SemanticDigest,
    pub atlas_revision: SemanticDigest,
    pub compiler: ContractIdentity,
    pub coordinate_system: SemanticAtlasCoordinateSystem,
    pub layout_policy: SemanticAtlasLayoutPolicy,
    pub submitted_sources: u64,
    pub sources: Vec<SemanticAtlasSource>,
    pub clusters: Vec<SemanticAtlasCluster>,
    pub regions: Vec<SemanticAtlasRegion>,
    pub limits: SemanticAtlasLimits,
    pub complete: bool,
    pub omissions: Vec<SemanticAtlasOmission>,
    pub lineage: Vec<SemanticAtlasLineage>,
}

impl SemanticAtlas {
    pub fn from_topographies<'a>(
        topographies: impl IntoIterator<Item = (&'a str, &'a TopographyPatch)>,
    ) -> Result<Option<Self>, SemanticAtlasError> {
        let mut sources = Vec::new();
        for (workload_id, patch) in topographies {
            patch.verify()?;
            if patch.workload.id != workload_id {
                return Err(SemanticAtlasError::WorkloadBinding);
            }
            sources.push(SemanticAtlasSource::from_topography(workload_id, patch));
        }
        if sources.is_empty() {
            return Ok(None);
        }
        Self::from_sources(sources).map(Some)
    }

    pub fn from_sources(sources: Vec<SemanticAtlasSource>) -> Result<Self, SemanticAtlasError> {
        let atlas = build_atlas(sources)?;
        atlas.verify()?;
        Ok(atlas)
    }

    pub fn verify(&self) -> Result<(), SemanticAtlasError> {
        if self.schema != SEMANTIC_ATLAS_SCHEMA {
            return Err(SemanticAtlasError::Schema);
        }
        validate_limits(&self.limits)?;
        if self.coordinate_system.kind != "synthetic_semantic_sphere"
            || self.coordinate_system.earth_crs.is_some()
            || self.coordinate_system.unit != "microdegree"
            || self.coordinate_system.longitude_range_microdegrees != [-180_000_000, 180_000_000]
            || self.coordinate_system.latitude_range_microdegrees != [-90_000_000, 90_000_000]
        {
            return Err(SemanticAtlasError::CoordinateSystem);
        }
        unique(
            self.sources.iter().map(|source| source.region_id.as_str()),
            "source region",
        )?;
        for source in &self.sources {
            let mut hasher = SemanticHasher::new("rey.semantic-atlas-region.v1");
            hasher.add_str(&source.workload_id);
            if source.workload_id.is_empty()
                || source.region_id != hasher.finish()
                || source.source_patch_id.as_str().is_empty()
                || source.source_topography_revision.as_str().is_empty()
            {
                return Err(SemanticAtlasError::Shape("source"));
            }
        }
        unique(
            self.regions.iter().map(|region| region.region_id.as_str()),
            "region",
        )?;
        unique(
            self.clusters
                .iter()
                .map(|cluster| cluster.cluster_id.as_str()),
            "cluster",
        )?;
        if self.sources.len() != self.regions.len()
            || self.sources.len() as u64 > self.limits.max_regions
            || self.clusters.len() as u64 > self.limits.max_world_clusters
            || self.omissions.len() as u64 > self.limits.max_omissions
        {
            return Err(SemanticAtlasError::Shape("bounded rows"));
        }
        let source_ids = self
            .sources
            .iter()
            .map(|source| source.region_id.as_str())
            .collect::<BTreeSet<_>>();
        let mut clustered = BTreeSet::new();
        for cluster in &self.clusters {
            if cluster.member_region_ids.is_empty()
                || cluster.member_region_ids.len() as u64 > self.limits.max_members_per_cluster
                || !valid_coordinate(
                    cluster.semantic_longitude_microdegrees,
                    cluster.semantic_latitude_microdegrees,
                )
            {
                return Err(SemanticAtlasError::Shape("cluster"));
            }
            for region_id in &cluster.member_region_ids {
                if !source_ids.contains(region_id.as_str()) || !clustered.insert(region_id.as_str())
                {
                    return Err(SemanticAtlasError::Membership);
                }
            }
        }
        if clustered != source_ids {
            return Err(SemanticAtlasError::Membership);
        }
        for region in &self.regions {
            if !valid_coordinate(
                region.semantic_longitude_microdegrees,
                region.semantic_latitude_microdegrees,
            ) || !self
                .clusters
                .iter()
                .any(|cluster| cluster.cluster_id == region.cluster_id)
            {
                return Err(SemanticAtlasError::Shape("region"));
            }
            let source = self
                .sources
                .iter()
                .find(|source| source.region_id == region.region_id)
                .ok_or(SemanticAtlasError::Membership)?;
            if region.workload_id != source.workload_id
                || region.source_patch_id != source.source_patch_id
                || region.source_topography_revision != source.source_topography_revision
                || region.frontier_rows != source.frontier_rows
                || region.complete != source.complete
            {
                return Err(SemanticAtlasError::Membership);
            }
        }
        let actual_revision = atlas_revision_digest(self)?;
        if actual_revision != self.atlas_revision || self.atlas_id != self.atlas_revision {
            return Err(SemanticAtlasError::Digest);
        }
        Ok(())
    }
}

fn build_atlas(mut sources: Vec<SemanticAtlasSource>) -> Result<SemanticAtlas, SemanticAtlasError> {
    let limits = SemanticAtlasLimits::default();
    sources.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    unique(
        sources.iter().map(|source| source.region_id.as_str()),
        "source region",
    )?;
    let submitted_sources = sources.len() as u64;
    let omitted = sources.len().saturating_sub(limits.max_regions as usize);
    sources.truncate(limits.max_regions as usize);
    let assignments = cluster_sources(&sources, limits.max_world_clusters as usize);
    let mut clusters = Vec::new();
    let mut regions = Vec::new();
    for (cluster_index, member_indices) in assignments.iter().enumerate() {
        let (cluster_longitude, cluster_latitude) =
            cluster_center(cluster_index, assignments.len());
        let member_region_ids = member_indices
            .iter()
            .map(|index| sources[*index].region_id.clone())
            .collect::<Vec<_>>();
        let cluster_id = cluster_identity(&member_region_ids);
        let cluster_dominant_feature = dominant_feature(
            member_indices
                .iter()
                .map(|index| &sources[*index])
                .collect::<Vec<_>>(),
        );
        let angular_radius = if member_indices.len() <= 1 {
            7_000_000
        } else {
            (8_000_000_u64 + member_indices.len() as u64 * 1_250_000).min(22_000_000)
        };
        clusters.push(SemanticAtlasCluster {
            cluster_id: cluster_id.clone(),
            semantic_longitude_microdegrees: cluster_longitude,
            semantic_latitude_microdegrees: cluster_latitude,
            angular_radius_microdegrees: angular_radius,
            member_region_ids,
            dominant_feature: cluster_dominant_feature,
        });
        for (member_position, source_index) in member_indices.iter().enumerate() {
            let source = &sources[*source_index];
            let (longitude, latitude) = if member_indices.len() == 1 {
                (cluster_longitude, cluster_latitude)
            } else {
                polar_member_coordinate(
                    cluster_longitude,
                    cluster_latitude,
                    member_position,
                    member_indices.len(),
                )
            };
            regions.push(SemanticAtlasRegion {
                region_id: source.region_id.clone(),
                cluster_id: cluster_id.clone(),
                workload_id: source.workload_id.clone(),
                source_patch_id: source.source_patch_id.clone(),
                source_topography_revision: source.source_topography_revision.clone(),
                semantic_longitude_microdegrees: longitude,
                semantic_latitude_microdegrees: latitude,
                angular_radius_microdegrees: 5_500_000,
                anchor_count: source
                    .workspace_anchors
                    .saturating_add(source.file_anchors)
                    .saturating_add(source.document_anchors)
                    .saturating_add(source.external_resource_anchors),
                frontier_rows: source.frontier_rows,
                complete: source.complete,
                dominant_feature: dominant_feature(vec![source]),
            });
        }
    }
    clusters.sort_by(|left, right| left.cluster_id.cmp(&right.cluster_id));
    regions.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    let mut omissions = Vec::new();
    if omitted > 0 {
        omissions.push(SemanticAtlasOmission {
            kind: "region_limit".to_owned(),
            omitted_count: omitted as u64,
            reason: "admitted regions beyond the atlas source limit remain unplaced".to_owned(),
        });
    }
    let mut lineage = sources
        .iter()
        .map(|source| SemanticAtlasLineage {
            kind: "topography_patch".to_owned(),
            identity: source.source_patch_id.to_string(),
            revision: source.source_topography_revision.to_string(),
        })
        .collect::<Vec<_>>();
    lineage.push(SemanticAtlasLineage {
        kind: "layout_compiler".to_owned(),
        identity: atlas_compiler().id,
        revision: atlas_compiler().semantic_digest.to_string(),
    });
    let mut atlas = SemanticAtlas {
        schema: SEMANTIC_ATLAS_SCHEMA.to_owned(),
        atlas_id: placeholder("rey.semantic-atlas.placeholder"),
        atlas_revision: placeholder("rey.semantic-atlas-revision.placeholder"),
        compiler: atlas_compiler(),
        coordinate_system: SemanticAtlasCoordinateSystem {
            kind: "synthetic_semantic_sphere".to_owned(),
            axes: vec![
                "semantic_longitude".to_owned(),
                "semantic_latitude".to_owned(),
            ],
            unit: "microdegree".to_owned(),
            longitude_range_microdegrees: [-180_000_000, 180_000_000],
            latitude_range_microdegrees: [-90_000_000, 90_000_000],
            wraps_longitude: true,
            authority: "layout coordinates derived only from retained admitted survey structure; visual proximity is not source truth".to_owned(),
            earth_crs: None,
        },
        layout_policy: SemanticAtlasLayoutPolicy {
            clustering: "deterministic bounded k-medoids over admitted survey-structure features".to_owned(),
            placement: "equal-area world cluster centers with deterministic polar member placement".to_owned(),
            recluster_trigger: "an admitted source set or source topography revision changes".to_owned(),
            zoom_rule: "zoom selects retained level of detail and never reclusters".to_owned(),
            distance_claim: "cluster membership reflects only declared survey-structure features; angular distance is presentation, not semantic similarity evidence".to_owned(),
        },
        submitted_sources,
        sources,
        clusters,
        regions,
        limits,
        complete: omitted == 0,
        omissions,
        lineage,
    };
    atlas.atlas_revision = atlas_revision_digest(&atlas)?;
    atlas.atlas_id = atlas.atlas_revision.clone();
    Ok(atlas)
}

fn cluster_sources(sources: &[SemanticAtlasSource], max_clusters: usize) -> Vec<Vec<usize>> {
    if sources.is_empty() {
        return Vec::new();
    }
    let cluster_count = integer_sqrt_ceil(sources.len()).min(max_clusters).max(1);
    let mut ordered = (0..sources.len()).collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        sources[*left]
            .feature_vector()
            .cmp(&sources[*right].feature_vector())
            .then_with(|| sources[*left].region_id.cmp(&sources[*right].region_id))
    });
    let mut medoids = (0..cluster_count)
        .map(|index| ordered[index * ordered.len() / cluster_count])
        .collect::<Vec<_>>();
    let mut assignments = vec![0_usize; sources.len()];
    for _ in 0..16 {
        for (source_index, source) in sources.iter().enumerate() {
            assignments[source_index] = medoids
                .iter()
                .enumerate()
                .min_by_key(|(cluster_index, medoid)| {
                    (feature_distance(source, &sources[**medoid]), *cluster_index)
                })
                .map_or(0, |(cluster_index, _)| cluster_index);
        }
        let mut changed = false;
        for (cluster_index, medoid) in medoids.iter_mut().enumerate() {
            let members = assignments
                .iter()
                .enumerate()
                .filter_map(|(index, assigned)| (*assigned == cluster_index).then_some(index))
                .collect::<Vec<_>>();
            if let Some(next) = members.iter().copied().min_by_key(|candidate| {
                let total = members.iter().fold(0_u128, |sum, member| {
                    sum.saturating_add(feature_distance(&sources[*candidate], &sources[*member]))
                });
                (total, sources[*candidate].region_id.as_str())
            }) {
                changed |= next != *medoid;
                *medoid = next;
            }
        }
        if !changed {
            break;
        }
    }
    let mut clusters = vec![Vec::new(); cluster_count];
    for (source_index, cluster_index) in assignments.into_iter().enumerate() {
        clusters[cluster_index].push(source_index);
    }
    clusters.retain(|cluster| !cluster.is_empty());
    for cluster in &mut clusters {
        cluster.sort_by(|left, right| sources[*left].region_id.cmp(&sources[*right].region_id));
    }
    clusters.sort_by(|left, right| sources[left[0]].region_id.cmp(&sources[right[0]].region_id));
    clusters
}

fn feature_distance(left: &SemanticAtlasSource, right: &SemanticAtlasSource) -> u128 {
    left.feature_vector()
        .into_iter()
        .zip(right.feature_vector())
        .fold(0_u128, |sum, (left, right)| {
            let delta = left.abs_diff(right) as u128;
            sum.saturating_add(delta.saturating_mul(delta))
        })
}

fn integer_sqrt_ceil(value: usize) -> usize {
    let mut root = 1_usize;
    while root.saturating_mul(root) < value {
        root += 1;
    }
    root
}

fn cluster_center(index: usize, count: usize) -> (i64, i64) {
    let fraction = (index as f64 + 0.5) / count.max(1) as f64;
    let latitude = (1.0 - 2.0 * fraction).asin().to_degrees();
    let longitude = normalize_longitude(index as f64 * 137.507_764_050_037_85);
    (microdegrees(longitude), microdegrees(latitude))
}

fn polar_member_coordinate(
    center_longitude: i64,
    center_latitude: i64,
    index: usize,
    count: usize,
) -> (i64, i64) {
    let ring = index / 6;
    let slot = index % 6;
    let ring_members = count.saturating_sub(ring * 6).clamp(1, 6);
    let bearing = 2.0 * PI * slot as f64 / ring_members as f64 + ring as f64 * 0.31;
    let distance = (5.5 + ring as f64 * 4.25).to_radians();
    let latitude = (center_latitude as f64 / MICRODEGREES_PER_DEGREE).to_radians();
    let longitude = (center_longitude as f64 / MICRODEGREES_PER_DEGREE).to_radians();
    let target_latitude =
        (latitude.sin() * distance.cos() + latitude.cos() * distance.sin() * bearing.cos()).asin();
    let target_longitude = longitude
        + (bearing.sin() * distance.sin() * latitude.cos())
            .atan2(distance.cos() - latitude.sin() * target_latitude.sin());
    (
        microdegrees(normalize_longitude(target_longitude.to_degrees())),
        microdegrees(target_latitude.to_degrees()),
    )
}

fn dominant_feature(sources: Vec<&SemanticAtlasSource>) -> String {
    let totals = sources.iter().fold([0_u64; 4], |mut totals, source| {
        totals[0] = totals[0].saturating_add(source.workspace_anchors);
        totals[1] = totals[1].saturating_add(source.file_anchors);
        totals[2] = totals[2].saturating_add(source.document_anchors);
        totals[3] = totals[3].saturating_add(source.external_resource_anchors);
        totals
    });
    ["workspace", "file", "document", "external_resource"]
        .into_iter()
        .enumerate()
        .max_by_key(|(index, _)| (totals[*index], std::cmp::Reverse(*index)))
        .map_or("workspace", |(_, feature)| feature)
        .to_owned()
}

fn cluster_identity(region_ids: &[SemanticDigest]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.semantic-atlas-cluster.v1");
    for region_id in region_ids {
        hasher.add_str(region_id.as_str());
    }
    hasher.finish()
}

fn atlas_compiler() -> ContractIdentity {
    let id = "rey.semantic-atlas.polar-cluster";
    let revision = 1;
    let mut hasher = SemanticHasher::new("rey.contract.v1");
    hasher.add_str(id);
    hasher.add_u64(revision);
    ContractIdentity {
        id: id.to_owned(),
        revision,
        semantic_digest: hasher.finish(),
    }
}

fn atlas_revision_digest(atlas: &SemanticAtlas) -> Result<SemanticDigest, SemanticAtlasError> {
    let mut normalized = atlas.clone();
    normalized.atlas_id = placeholder("rey.semantic-atlas.placeholder");
    normalized.atlas_revision = placeholder("rey.semantic-atlas-revision.placeholder");
    let bytes = serde_json::to_vec(&normalized)?;
    let mut hasher = SemanticHasher::new(SEMANTIC_ATLAS_SCHEMA);
    hasher.add_bytes(&bytes);
    Ok(hasher.finish())
}

fn validate_limits(limits: &SemanticAtlasLimits) -> Result<(), SemanticAtlasError> {
    if [
        limits.max_regions,
        limits.max_world_clusters,
        limits.max_members_per_cluster,
        limits.max_omissions,
    ]
    .contains(&0)
    {
        return Err(SemanticAtlasError::Limit);
    }
    Ok(())
}

fn valid_coordinate(longitude: i64, latitude: i64) -> bool {
    (-180_000_000..=180_000_000).contains(&longitude)
        && (-90_000_000..=90_000_000).contains(&latitude)
}

fn normalize_longitude(longitude: f64) -> f64 {
    (longitude + 180.0).rem_euclid(360.0) - 180.0
}

fn microdegrees(value: f64) -> i64 {
    (value * MICRODEGREES_PER_DEGREE).round() as i64
}

fn unique<'a>(
    values: impl IntoIterator<Item = &'a str>,
    kind: &'static str,
) -> Result<(), SemanticAtlasError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if value.is_empty() || !seen.insert(value) {
            return Err(SemanticAtlasError::Duplicate(kind));
        }
    }
    Ok(())
}

fn placeholder(domain: &str) -> SemanticDigest {
    SemanticHasher::new(domain).finish()
}

#[derive(Debug, Error)]
pub enum SemanticAtlasError {
    #[error("semantic atlas schema is unsupported")]
    Schema,
    #[error("semantic atlas coordinate-system contract is invalid")]
    CoordinateSystem,
    #[error("semantic atlas contains a duplicate or empty {0}")]
    Duplicate(&'static str),
    #[error("semantic atlas {0} shape is invalid")]
    Shape(&'static str),
    #[error("semantic atlas cluster membership is invalid")]
    Membership,
    #[error("semantic atlas workload does not match its source topography")]
    WorkloadBinding,
    #[error("semantic atlas limit is invalid")]
    Limit,
    #[error("semantic atlas digest does not match its content")]
    Digest,
    #[error(transparent)]
    Topography(#[from] crate::TopographyError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(id: &str, file_anchors: u64, document_anchors: u64) -> SemanticAtlasSource {
        let mut region = SemanticHasher::new("rey.semantic-atlas-region.v1");
        region.add_str(id);
        let mut patch = SemanticHasher::new("test-patch");
        patch.add_str(id);
        let mut revision = SemanticHasher::new("test-revision");
        revision.add_str(id);
        SemanticAtlasSource {
            region_id: region.finish(),
            workload_id: id.to_owned(),
            source_patch_id: patch.finish(),
            source_topography_revision: revision.finish(),
            complete: true,
            workspace_anchors: 1,
            file_anchors,
            document_anchors,
            external_resource_anchors: 0,
            requested_seeds: 4,
            surveyed_seeds: 4,
            candidates: file_anchors.saturating_add(document_anchors),
            frontier_rows: 0,
        }
    }

    #[test]
    fn atlas_is_deterministic_and_bound_to_source_revisions() {
        let first = SemanticAtlas::from_sources(vec![
            source("docs", 1, 8),
            source("code", 9, 1),
            source("mixed", 4, 4),
        ])
        .expect("atlas");
        let reordered = SemanticAtlas::from_sources(vec![
            source("mixed", 4, 4),
            source("code", 9, 1),
            source("docs", 1, 8),
        ])
        .expect("atlas");
        assert_eq!(first, reordered);
        assert_eq!(first.regions.len(), 3);
        assert_eq!(first.clusters.len(), 2);
        assert!(first.verify().is_ok());

        let mut changed_sources = first.sources.clone();
        changed_sources[0].source_topography_revision = placeholder("changed");
        let changed = SemanticAtlas::from_sources(changed_sources).expect("changed atlas");
        assert_ne!(first.atlas_revision, changed.atlas_revision);
    }

    #[test]
    fn atlas_rejects_earth_crs_and_tampered_coordinates() {
        let mut atlas = SemanticAtlas::from_sources(vec![source("one", 2, 1)]).expect("atlas");
        atlas.coordinate_system.earth_crs = Some("OGC:CRS84".to_owned());
        assert!(matches!(
            atlas.verify(),
            Err(SemanticAtlasError::CoordinateSystem)
        ));

        let mut atlas = SemanticAtlas::from_sources(vec![source("one", 2, 1)]).expect("atlas");
        atlas.regions[0].semantic_latitude_microdegrees = 91_000_000;
        assert!(matches!(
            atlas.verify(),
            Err(SemanticAtlasError::Shape("region"))
        ));
    }

    #[test]
    fn zoom_is_not_an_atlas_input() {
        let atlas = SemanticAtlas::from_sources(vec![source("one", 2, 1)]).expect("atlas");
        assert_eq!(
            atlas.layout_policy.zoom_rule,
            "zoom selects retained level of detail and never reclusters"
        );
        assert!(atlas.coordinate_system.earth_crs.is_none());
    }
}
