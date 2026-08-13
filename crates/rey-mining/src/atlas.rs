use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    AdmittedRegionalScene, RegionalCoordinateSpace, RegionalLayerKind, RegionalValidityClass,
    TopographyAnchorKind, TopographyPatch,
};

pub const SEMANTIC_ATLAS_SCHEMA: &str = "rey.semantic-atlas.v1";
pub const SEMANTIC_ATLAS_DELTA_SCHEMA: &str = "rey.semantic-atlas-delta.v1";
const MICRODEGREES_PER_DEGREE: f64 = 1_000_000.0;
const MAX_ATLAS_DELTA_CHANGES: usize = 512;
const SECTOR_LONGITUDE_BANDS: u16 = 12;
const SECTOR_LATITUDE_BANDS: u16 = 6;
const SECTOR_SIZE_MICRODEGREES: i64 = 30_000_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticAtlasLimits {
    pub max_regions: u64,
    pub max_world_clusters: u64,
    pub max_members_per_cluster: u64,
    pub max_sectors: u64,
    pub max_omissions: u64,
}

impl Default for SemanticAtlasLimits {
    fn default() -> Self {
        Self {
            max_regions: 128,
            max_world_clusters: 16,
            max_members_per_cluster: 128,
            max_sectors: 72,
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
pub struct SemanticAtlasRegionalSource {
    pub region_id: SemanticDigest,
    pub workload_id: String,
    pub scene_region_id: String,
    pub source_scene_id: SemanticDigest,
    pub source_admission_id: SemanticDigest,
    pub source_package_id: SemanticDigest,
    pub source_package_revision: SemanticDigest,
    pub projection_packet_id: SemanticDigest,
    pub semantic_longitude_microdegrees: i64,
    pub semantic_latitude_microdegrees: i64,
    pub complete: bool,
    pub native_objects: u64,
    pub native_feature_objects: u64,
    pub terrain_control_objects: u64,
    pub hydrology_objects: u64,
    pub boundary_objects: u64,
    pub poi_objects: u64,
    pub highway_objects: u64,
    pub road_objects: u64,
    pub district_objects: u64,
    pub lot_objects: u64,
    pub structure_objects: u64,
    pub utility_objects: u64,
    pub label_objects: u64,
    pub beacon_objects: u64,
    pub construction_objects: u64,
    pub connector_objects: u64,
    pub validity_boundaries: u64,
    pub omissions: u64,
}

impl SemanticAtlasRegionalSource {
    pub fn from_scene(
        workload_id: &str,
        scene: &AdmittedRegionalScene,
    ) -> Result<Self, SemanticAtlasError> {
        scene.verify()?;
        if scene.admission.workload.id != workload_id {
            return Err(SemanticAtlasError::WorkloadBinding);
        }
        let placement = scene
            .projection
            .transforms
            .iter()
            .find(|transform| {
                transform.source_space == RegionalCoordinateSpace::NativeCrs84
                    && transform.target_space == RegionalCoordinateSpace::SyntheticSemantic
                    && transform.target_origin.len() == 2
            })
            .ok_or(SemanticAtlasError::RegionalPlacement)?;
        let mut hasher = SemanticHasher::new("rey.semantic-atlas-regional-region.v1");
        hasher.add_str(workload_id);
        hasher.add_str(&scene.region_id);
        let count = |kind| {
            scene
                .projection
                .objects
                .iter()
                .filter(|object| object.layer == kind)
                .count() as u64
        };
        Ok(Self {
            region_id: hasher.finish(),
            workload_id: workload_id.to_owned(),
            scene_region_id: scene.region_id.clone(),
            source_scene_id: scene.scene_id.clone(),
            source_admission_id: scene.admission.admission_id.clone(),
            source_package_id: scene.admission.package_id.clone(),
            source_package_revision: scene.admission.package_snapshot_revision.clone(),
            projection_packet_id: scene.projection.packet_id.clone(),
            semantic_longitude_microdegrees: placement.target_origin[0],
            semantic_latitude_microdegrees: placement.target_origin[1],
            complete: scene.complete,
            native_objects: scene.projection.objects.len() as u64,
            native_feature_objects: count(RegionalLayerKind::NativeFeature),
            terrain_control_objects: count(RegionalLayerKind::TerrainControl),
            hydrology_objects: count(RegionalLayerKind::Hydrology),
            boundary_objects: count(RegionalLayerKind::Boundary),
            poi_objects: count(RegionalLayerKind::Poi),
            highway_objects: count(RegionalLayerKind::Highway),
            road_objects: count(RegionalLayerKind::Road),
            district_objects: count(RegionalLayerKind::District),
            lot_objects: count(RegionalLayerKind::Lot),
            structure_objects: count(RegionalLayerKind::Structure),
            utility_objects: count(RegionalLayerKind::Utility),
            label_objects: count(RegionalLayerKind::Label),
            beacon_objects: count(RegionalLayerKind::Beacon),
            construction_objects: count(RegionalLayerKind::Construction),
            connector_objects: count(RegionalLayerKind::Connector),
            validity_boundaries: scene
                .projection
                .validity
                .iter()
                .filter(|validity| validity.class != RegionalValidityClass::Valid)
                .count() as u64,
            omissions: scene.omissions.len() as u64,
        })
    }

    fn feature_vector(&self) -> [u64; 8] {
        let objects = self.native_objects.max(1);
        let ratio = |value: u64| value.saturating_mul(10_000).saturating_div(objects);
        [
            10_000,
            ratio(self.native_feature_objects),
            ratio(self.terrain_control_objects),
            ratio(self.hydrology_objects),
            ratio(self.boundary_objects),
            ratio(self.poi_objects),
            self.validity_boundaries
                .saturating_mul(10_000)
                .saturating_div(objects),
            if self.complete { 10_000 } else { 0 },
        ]
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticAtlasRegion {
    pub region_id: SemanticDigest,
    pub cluster_id: SemanticDigest,
    pub sector_id: SemanticDigest,
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
pub struct SemanticAtlasRegionalRegion {
    pub region_id: SemanticDigest,
    pub cluster_id: SemanticDigest,
    pub sector_id: SemanticDigest,
    pub workload_id: String,
    pub scene_region_id: String,
    pub source_scene_id: SemanticDigest,
    pub source_admission_id: SemanticDigest,
    pub source_package_id: SemanticDigest,
    pub source_package_revision: SemanticDigest,
    pub projection_packet_id: SemanticDigest,
    pub semantic_longitude_microdegrees: i64,
    pub semantic_latitude_microdegrees: i64,
    pub angular_radius_microdegrees: u64,
    pub native_objects: u64,
    pub validity_boundaries: u64,
    pub omissions: u64,
    pub complete: bool,
    pub dominant_feature: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "source_kind", content = "region")]
pub enum SemanticAtlasRegionEvidence {
    SurveyTopography(SemanticAtlasRegion),
    AdmittedRegionalScene(SemanticAtlasRegionalRegion),
}

impl SemanticAtlasRegionEvidence {
    fn region_id(&self) -> &SemanticDigest {
        match self {
            Self::SurveyTopography(region) => &region.region_id,
            Self::AdmittedRegionalScene(region) => &region.region_id,
        }
    }
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
pub struct SemanticAtlasSector {
    pub sector_id: SemanticDigest,
    pub longitude_band: u16,
    pub latitude_band: u16,
    pub west_microdegrees: i64,
    pub south_microdegrees: i64,
    pub east_microdegrees: i64,
    pub north_microdegrees: i64,
    pub member_region_ids: Vec<SemanticDigest>,
    pub authority: String,
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
    pub sector_grid: ContractIdentity,
    pub submitted_sources: u64,
    pub sources: Vec<SemanticAtlasSource>,
    #[serde(default)]
    pub regional_sources: Vec<SemanticAtlasRegionalSource>,
    pub clusters: Vec<SemanticAtlasCluster>,
    #[serde(default)]
    pub sectors: Vec<SemanticAtlasSector>,
    pub regions: Vec<SemanticAtlasRegion>,
    #[serde(default)]
    pub regional_regions: Vec<SemanticAtlasRegionalRegion>,
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
        Self::from_evidence_sources(sources, Vec::new())
    }

    pub fn from_admitted_evidence<'a>(
        topographies: impl IntoIterator<Item = (&'a str, &'a TopographyPatch)>,
        regional_scenes: impl IntoIterator<Item = (&'a str, &'a AdmittedRegionalScene)>,
    ) -> Result<Option<Self>, SemanticAtlasError> {
        let mut survey_sources = Vec::new();
        for (workload_id, patch) in topographies {
            patch.verify()?;
            if patch.workload.id != workload_id {
                return Err(SemanticAtlasError::WorkloadBinding);
            }
            survey_sources.push(SemanticAtlasSource::from_topography(workload_id, patch));
        }
        let regional_sources = regional_scenes
            .into_iter()
            .map(|(workload_id, scene)| SemanticAtlasRegionalSource::from_scene(workload_id, scene))
            .collect::<Result<Vec<_>, _>>()?;
        if survey_sources.is_empty() && regional_sources.is_empty() {
            return Ok(None);
        }
        Self::from_evidence_sources(survey_sources, regional_sources).map(Some)
    }

    pub fn from_evidence_sources(
        sources: Vec<SemanticAtlasSource>,
        regional_sources: Vec<SemanticAtlasRegionalSource>,
    ) -> Result<Self, SemanticAtlasError> {
        let atlas = build_atlas(sources, regional_sources)?;
        atlas.verify()?;
        Ok(atlas)
    }

    pub fn bind_regional_scene(
        &self,
        workload_id: &str,
        scene: &AdmittedRegionalScene,
    ) -> Result<AdmittedRegionalScene, SemanticAtlasError> {
        self.verify()?;
        let expected = SemanticAtlasRegionalSource::from_scene(workload_id, scene)?;
        if self
            .regional_sources
            .iter()
            .find(|source| source.region_id == expected.region_id)
            != Some(&expected)
            || !self
                .regional_regions
                .iter()
                .any(|region| region.region_id == expected.region_id)
        {
            return Err(SemanticAtlasError::RegionalAtlasBinding);
        }
        let bound = scene
            .clone()
            .with_admitted_atlas_revision(&self.atlas_revision)?;
        self.verify_regional_scene_binding(workload_id, &bound)?;
        Ok(bound)
    }

    pub fn verify_regional_scene_binding(
        &self,
        workload_id: &str,
        scene: &AdmittedRegionalScene,
    ) -> Result<(), SemanticAtlasError> {
        self.verify()?;
        scene.verify()?;
        if scene.artifacts.admitted_atlas_revision.as_ref() != Some(&self.atlas_revision) {
            return Err(SemanticAtlasError::RegionalAtlasBinding);
        }
        let expected = SemanticAtlasRegionalSource::from_scene(workload_id, scene)?;
        if self
            .regional_sources
            .iter()
            .find(|source| source.region_id == expected.region_id)
            != Some(&expected)
            || !self
                .regional_regions
                .iter()
                .any(|region| region.region_id == expected.region_id)
        {
            return Err(SemanticAtlasError::RegionalAtlasBinding);
        }
        Ok(())
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
        if self.sector_grid != atlas_sector_grid() {
            return Err(SemanticAtlasError::Sector);
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
            self.regional_sources
                .iter()
                .map(|source| source.region_id.as_str()),
            "regional source region",
        )?;
        for source in &self.regional_sources {
            let mut hasher = SemanticHasher::new("rey.semantic-atlas-regional-region.v1");
            hasher.add_str(&source.workload_id);
            hasher.add_str(&source.scene_region_id);
            if source.workload_id.is_empty()
                || source.scene_region_id.is_empty()
                || source.region_id != hasher.finish()
                || !valid_coordinate(
                    source.semantic_longitude_microdegrees,
                    source.semantic_latitude_microdegrees,
                )
                || source.source_scene_id.as_str().is_empty()
                || source.source_admission_id.as_str().is_empty()
                || source.source_package_id.as_str().is_empty()
                || source.source_package_revision.as_str().is_empty()
                || source.projection_packet_id.as_str().is_empty()
                || source
                    .native_feature_objects
                    .saturating_add(source.terrain_control_objects)
                    .saturating_add(source.hydrology_objects)
                    .saturating_add(source.boundary_objects)
                    .saturating_add(source.poi_objects)
                    .saturating_add(source.highway_objects)
                    .saturating_add(source.road_objects)
                    .saturating_add(source.district_objects)
                    .saturating_add(source.lot_objects)
                    .saturating_add(source.structure_objects)
                    .saturating_add(source.utility_objects)
                    .saturating_add(source.label_objects)
                    .saturating_add(source.beacon_objects)
                    .saturating_add(source.construction_objects)
                    .saturating_add(source.connector_objects)
                    != source.native_objects
            {
                return Err(SemanticAtlasError::Shape("regional source"));
            }
        }
        unique(
            self.regions.iter().map(|region| region.region_id.as_str()),
            "region",
        )?;
        unique(
            self.regional_regions
                .iter()
                .map(|region| region.region_id.as_str()),
            "regional region",
        )?;
        unique(
            self.clusters
                .iter()
                .map(|cluster| cluster.cluster_id.as_str()),
            "cluster",
        )?;
        unique(
            self.sectors.iter().map(|sector| sector.sector_id.as_str()),
            "sector",
        )?;
        let source_count = self
            .sources
            .len()
            .saturating_add(self.regional_sources.len());
        if self.sources.len() != self.regions.len()
            || self.regional_sources.len() != self.regional_regions.len()
            || self.submitted_sources
                < self
                    .sources
                    .len()
                    .saturating_add(self.regional_sources.len()) as u64
            || source_count as u64 > self.limits.max_regions
            || self.clusters.len() as u64 > self.limits.max_world_clusters
            || self.sectors.len() as u64 > self.limits.max_sectors
            || self.omissions.len() as u64 > self.limits.max_omissions
        {
            return Err(SemanticAtlasError::Shape("bounded rows"));
        }
        let source_ids = self
            .sources
            .iter()
            .map(|source| source.region_id.as_str())
            .chain(
                self.regional_sources
                    .iter()
                    .map(|source| source.region_id.as_str()),
            )
            .collect::<BTreeSet<_>>();
        if source_ids.len() != source_count {
            return Err(SemanticAtlasError::Duplicate("evidence region"));
        }
        if self.regions.iter().any(|region| {
            !valid_coordinate(
                region.semantic_longitude_microdegrees,
                region.semantic_latitude_microdegrees,
            )
        }) {
            return Err(SemanticAtlasError::Shape("region"));
        }
        if self.regional_regions.iter().any(|region| {
            !valid_coordinate(
                region.semantic_longitude_microdegrees,
                region.semantic_latitude_microdegrees,
            )
        }) {
            return Err(SemanticAtlasError::Shape("regional region"));
        }
        let region_sector_by_id = self
            .regions
            .iter()
            .map(|region| {
                (
                    region.region_id.clone(),
                    sector_id_for_coordinate(
                        region.semantic_longitude_microdegrees,
                        region.semantic_latitude_microdegrees,
                    ),
                )
            })
            .chain(self.regional_regions.iter().map(|region| {
                (
                    region.region_id.clone(),
                    sector_id_for_coordinate(
                        region.semantic_longitude_microdegrees,
                        region.semantic_latitude_microdegrees,
                    ),
                )
            }))
            .collect::<BTreeMap<_, _>>();
        let mut sectored = BTreeSet::new();
        for sector in &self.sectors {
            let (west, south, east, north) =
                sector_bounds(sector.longitude_band, sector.latitude_band);
            if sector.longitude_band >= SECTOR_LONGITUDE_BANDS
                || sector.latitude_band >= SECTOR_LATITUDE_BANDS
                || sector.sector_id != sector_identity(sector.longitude_band, sector.latitude_band)
                || (sector.west_microdegrees, sector.south_microdegrees) != (west, south)
                || (sector.east_microdegrees, sector.north_microdegrees) != (east, north)
                || !canonical_nonempty_digests(&sector.member_region_ids)
                || sector.member_region_ids.len() as u64 > self.limits.max_members_per_cluster
                || sector.authority
                    != "fixed synthetic partition cell used only for atlas membership; not surveyed coverage, native County footprint, source topology, or physical area"
            {
                return Err(SemanticAtlasError::Sector);
            }
            for region_id in &sector.member_region_ids {
                if region_sector_by_id.get(region_id) != Some(&sector.sector_id)
                    || !sectored.insert(region_id.as_str())
                {
                    return Err(SemanticAtlasError::Sector);
                }
            }
        }
        if sectored != source_ids {
            return Err(SemanticAtlasError::Sector);
        }
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
            ) || region.sector_id
                != sector_id_for_coordinate(
                    region.semantic_longitude_microdegrees,
                    region.semantic_latitude_microdegrees,
                )
                || !self.clusters.iter().any(|cluster| {
                    cluster.cluster_id == region.cluster_id
                        && cluster.member_region_ids.contains(&region.region_id)
                })
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
        for region in &self.regional_regions {
            if !valid_coordinate(
                region.semantic_longitude_microdegrees,
                region.semantic_latitude_microdegrees,
            ) || region.sector_id
                != sector_id_for_coordinate(
                    region.semantic_longitude_microdegrees,
                    region.semantic_latitude_microdegrees,
                )
                || !self.clusters.iter().any(|cluster| {
                    cluster.cluster_id == region.cluster_id
                        && cluster.member_region_ids.contains(&region.region_id)
                })
            {
                return Err(SemanticAtlasError::Shape("regional region"));
            }
            let source = self
                .regional_sources
                .iter()
                .find(|source| source.region_id == region.region_id)
                .ok_or(SemanticAtlasError::Membership)?;
            if region.workload_id != source.workload_id
                || region.scene_region_id != source.scene_region_id
                || region.source_scene_id != source.source_scene_id
                || region.source_admission_id != source.source_admission_id
                || region.source_package_id != source.source_package_id
                || region.source_package_revision != source.source_package_revision
                || region.projection_packet_id != source.projection_packet_id
                || region.semantic_longitude_microdegrees != source.semantic_longitude_microdegrees
                || region.semantic_latitude_microdegrees != source.semantic_latitude_microdegrees
                || region.native_objects != source.native_objects
                || region.validity_boundaries != source.validity_boundaries
                || region.omissions != source.omissions
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

    pub fn delta_from(
        &self,
        source: Option<&Self>,
    ) -> Result<SemanticAtlasDelta, SemanticAtlasError> {
        SemanticAtlasDelta::between(source, self)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticAtlasRegionChangeKind {
    Inserted,
    Removed,
    Moved,
    InterestChanged,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticAtlasRegionChange {
    pub region_id: SemanticDigest,
    pub kind: SemanticAtlasRegionChangeKind,
    pub before: Option<SemanticAtlasRegionEvidence>,
    pub after: Option<SemanticAtlasRegionEvidence>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticAtlasClusterChangeKind {
    Merged,
    Split,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticAtlasClusterChange {
    pub kind: SemanticAtlasClusterChangeKind,
    pub region_ids: Vec<SemanticDigest>,
    pub source_cluster_ids: Vec<SemanticDigest>,
    pub target_cluster_ids: Vec<SemanticDigest>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticAtlasDelta {
    pub schema: String,
    pub delta_id: SemanticDigest,
    pub source_revision: SemanticDigest,
    pub target_revision: SemanticDigest,
    pub inserted: u64,
    pub removed: u64,
    pub moved: u64,
    pub interest_changed: u64,
    pub merged: u64,
    pub split: u64,
    pub region_changes: Vec<SemanticAtlasRegionChange>,
    pub cluster_changes: Vec<SemanticAtlasClusterChange>,
}

impl SemanticAtlasDelta {
    pub fn between(
        source: Option<&SemanticAtlas>,
        target: &SemanticAtlas,
    ) -> Result<Self, SemanticAtlasError> {
        if let Some(source) = source {
            source.verify()?;
        }
        target.verify()?;
        let source_regions = source.map_or_else(BTreeMap::new, atlas_region_evidence);
        let target_regions = atlas_region_evidence(target);
        let mut region_changes = Vec::new();
        for region_id in source_regions
            .keys()
            .chain(target_regions.keys())
            .collect::<BTreeSet<_>>()
        {
            match (source_regions.get(region_id), target_regions.get(region_id)) {
                (None, Some(after)) => region_changes.push(SemanticAtlasRegionChange {
                    region_id: (*region_id).clone(),
                    kind: SemanticAtlasRegionChangeKind::Inserted,
                    before: None,
                    after: Some(after.clone()),
                }),
                (Some(before), None) => region_changes.push(SemanticAtlasRegionChange {
                    region_id: (*region_id).clone(),
                    kind: SemanticAtlasRegionChangeKind::Removed,
                    before: Some(before.clone()),
                    after: None,
                }),
                (Some(before), Some(after)) => {
                    if atlas_region_moved(before, after) {
                        region_changes.push(SemanticAtlasRegionChange {
                            region_id: (*region_id).clone(),
                            kind: SemanticAtlasRegionChangeKind::Moved,
                            before: Some(before.clone()),
                            after: Some(after.clone()),
                        });
                    }
                    if atlas_region_interest_changed(before, after) {
                        region_changes.push(SemanticAtlasRegionChange {
                            region_id: (*region_id).clone(),
                            kind: SemanticAtlasRegionChangeKind::InterestChanged,
                            before: Some(before.clone()),
                            after: Some(after.clone()),
                        });
                    }
                }
                (None, None) => unreachable!("region identity came from one atlas"),
            }
        }
        region_changes.sort_by(|left, right| {
            (&left.region_id, left.kind).cmp(&(&right.region_id, right.kind))
        });
        let mut cluster_changes = source.map_or_else(Vec::new, |source| {
            semantic_atlas_cluster_changes(source, target)
        });
        cluster_changes.sort_by(|left, right| {
            (
                left.kind,
                &left.source_cluster_ids,
                &left.target_cluster_ids,
                &left.region_ids,
            )
                .cmp(&(
                    right.kind,
                    &right.source_cluster_ids,
                    &right.target_cluster_ids,
                    &right.region_ids,
                ))
        });
        let count_region = |kind| {
            region_changes
                .iter()
                .filter(|change| change.kind == kind)
                .count() as u64
        };
        let count_cluster = |kind| {
            cluster_changes
                .iter()
                .filter(|change| change.kind == kind)
                .count() as u64
        };
        let mut delta = Self {
            schema: SEMANTIC_ATLAS_DELTA_SCHEMA.to_owned(),
            delta_id: placeholder("rey.semantic-atlas-delta.placeholder"),
            source_revision: source
                .map_or_else(empty_atlas_revision, |atlas| atlas.atlas_revision.clone()),
            target_revision: target.atlas_revision.clone(),
            inserted: count_region(SemanticAtlasRegionChangeKind::Inserted),
            removed: count_region(SemanticAtlasRegionChangeKind::Removed),
            moved: count_region(SemanticAtlasRegionChangeKind::Moved),
            interest_changed: count_region(SemanticAtlasRegionChangeKind::InterestChanged),
            merged: count_cluster(SemanticAtlasClusterChangeKind::Merged),
            split: count_cluster(SemanticAtlasClusterChangeKind::Split),
            region_changes,
            cluster_changes,
        };
        delta.delta_id = atlas_delta_digest(&delta)?;
        delta.verify()?;
        Ok(delta)
    }

    pub fn verify(&self) -> Result<(), SemanticAtlasError> {
        if self.schema != SEMANTIC_ATLAS_DELTA_SCHEMA
            || self.source_revision.as_str().is_empty()
            || self.target_revision.as_str().is_empty()
            || self
                .region_changes
                .len()
                .saturating_add(self.cluster_changes.len())
                > MAX_ATLAS_DELTA_CHANGES
        {
            return Err(SemanticAtlasError::DeltaShape);
        }
        let mut previous_region = None;
        let mut region_keys = BTreeSet::new();
        for change in &self.region_changes {
            let key = (&change.region_id, change.kind);
            if previous_region.is_some_and(|previous| previous >= key)
                || !region_keys.insert((change.region_id.clone(), change.kind))
                || !valid_region_change(change)
            {
                return Err(SemanticAtlasError::DeltaShape);
            }
            previous_region = Some(key);
        }
        let mut previous_cluster = None;
        let mut cluster_keys = BTreeSet::new();
        for change in &self.cluster_changes {
            let key = (
                change.kind,
                &change.source_cluster_ids,
                &change.target_cluster_ids,
                &change.region_ids,
            );
            if previous_cluster.is_some_and(|previous| previous >= key)
                || !cluster_keys.insert((
                    change.kind,
                    change.source_cluster_ids.clone(),
                    change.target_cluster_ids.clone(),
                    change.region_ids.clone(),
                ))
                || !valid_cluster_change(change)
            {
                return Err(SemanticAtlasError::DeltaShape);
            }
            previous_cluster = Some(key);
        }
        let count_region = |kind| {
            self.region_changes
                .iter()
                .filter(|change| change.kind == kind)
                .count() as u64
        };
        let count_cluster = |kind| {
            self.cluster_changes
                .iter()
                .filter(|change| change.kind == kind)
                .count() as u64
        };
        if self.inserted != count_region(SemanticAtlasRegionChangeKind::Inserted)
            || self.removed != count_region(SemanticAtlasRegionChangeKind::Removed)
            || self.moved != count_region(SemanticAtlasRegionChangeKind::Moved)
            || self.interest_changed != count_region(SemanticAtlasRegionChangeKind::InterestChanged)
            || self.merged != count_cluster(SemanticAtlasClusterChangeKind::Merged)
            || self.split != count_cluster(SemanticAtlasClusterChangeKind::Split)
            || self.delta_id != atlas_delta_digest(self)?
        {
            return Err(SemanticAtlasError::DeltaDigest);
        }
        Ok(())
    }

    pub fn verify_between(
        &self,
        source: Option<&SemanticAtlas>,
        target: &SemanticAtlas,
    ) -> Result<(), SemanticAtlasError> {
        let expected = Self::between(source, target)?;
        if *self != expected {
            return Err(SemanticAtlasError::DeltaBinding);
        }
        Ok(())
    }
}

fn atlas_region_evidence(
    atlas: &SemanticAtlas,
) -> BTreeMap<SemanticDigest, SemanticAtlasRegionEvidence> {
    atlas
        .regions
        .iter()
        .cloned()
        .map(|region| {
            (
                region.region_id.clone(),
                SemanticAtlasRegionEvidence::SurveyTopography(region),
            )
        })
        .chain(atlas.regional_regions.iter().cloned().map(|region| {
            (
                region.region_id.clone(),
                SemanticAtlasRegionEvidence::AdmittedRegionalScene(region),
            )
        }))
        .collect()
}

fn atlas_region_moved(
    left: &SemanticAtlasRegionEvidence,
    right: &SemanticAtlasRegionEvidence,
) -> bool {
    match (left, right) {
        (
            SemanticAtlasRegionEvidence::SurveyTopography(left),
            SemanticAtlasRegionEvidence::SurveyTopography(right),
        ) => {
            left.cluster_id != right.cluster_id
                || left.sector_id != right.sector_id
                || left.semantic_longitude_microdegrees != right.semantic_longitude_microdegrees
                || left.semantic_latitude_microdegrees != right.semantic_latitude_microdegrees
                || left.angular_radius_microdegrees != right.angular_radius_microdegrees
        }
        (
            SemanticAtlasRegionEvidence::AdmittedRegionalScene(left),
            SemanticAtlasRegionEvidence::AdmittedRegionalScene(right),
        ) => {
            left.cluster_id != right.cluster_id
                || left.sector_id != right.sector_id
                || left.semantic_longitude_microdegrees != right.semantic_longitude_microdegrees
                || left.semantic_latitude_microdegrees != right.semantic_latitude_microdegrees
                || left.angular_radius_microdegrees != right.angular_radius_microdegrees
        }
        _ => true,
    }
}

fn atlas_region_interest_changed(
    left: &SemanticAtlasRegionEvidence,
    right: &SemanticAtlasRegionEvidence,
) -> bool {
    match (left, right) {
        (
            SemanticAtlasRegionEvidence::SurveyTopography(left),
            SemanticAtlasRegionEvidence::SurveyTopography(right),
        ) => {
            left.workload_id != right.workload_id
                || left.source_patch_id != right.source_patch_id
                || left.source_topography_revision != right.source_topography_revision
                || left.anchor_count != right.anchor_count
                || left.frontier_rows != right.frontier_rows
                || left.complete != right.complete
                || left.dominant_feature != right.dominant_feature
        }
        (
            SemanticAtlasRegionEvidence::AdmittedRegionalScene(left),
            SemanticAtlasRegionEvidence::AdmittedRegionalScene(right),
        ) => {
            left.workload_id != right.workload_id
                || left.scene_region_id != right.scene_region_id
                || left.source_scene_id != right.source_scene_id
                || left.source_admission_id != right.source_admission_id
                || left.source_package_id != right.source_package_id
                || left.source_package_revision != right.source_package_revision
                || left.projection_packet_id != right.projection_packet_id
                || left.native_objects != right.native_objects
                || left.validity_boundaries != right.validity_boundaries
                || left.omissions != right.omissions
                || left.complete != right.complete
                || left.dominant_feature != right.dominant_feature
        }
        _ => true,
    }
}

fn semantic_atlas_cluster_changes(
    source: &SemanticAtlas,
    target: &SemanticAtlas,
) -> Vec<SemanticAtlasClusterChange> {
    let source_cluster_by_region = source
        .regions
        .iter()
        .map(|region| (region.region_id.clone(), region.cluster_id.clone()))
        .chain(
            source
                .regional_regions
                .iter()
                .map(|region| (region.region_id.clone(), region.cluster_id.clone())),
        )
        .collect::<BTreeMap<_, _>>();
    let target_cluster_by_region = target
        .regions
        .iter()
        .map(|region| (region.region_id.clone(), region.cluster_id.clone()))
        .chain(
            target
                .regional_regions
                .iter()
                .map(|region| (region.region_id.clone(), region.cluster_id.clone())),
        )
        .collect::<BTreeMap<_, _>>();
    let persistent_regions = source_cluster_by_region
        .keys()
        .filter(|region_id| target_cluster_by_region.contains_key(*region_id))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut changes = Vec::new();
    for target_cluster in &target.clusters {
        let region_ids = target_cluster
            .member_region_ids
            .iter()
            .filter(|region_id| persistent_regions.contains(*region_id))
            .cloned()
            .collect::<Vec<_>>();
        let source_cluster_ids = region_ids
            .iter()
            .filter_map(|region_id| source_cluster_by_region.get(region_id))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if source_cluster_ids.len() > 1 {
            changes.push(SemanticAtlasClusterChange {
                kind: SemanticAtlasClusterChangeKind::Merged,
                region_ids,
                source_cluster_ids,
                target_cluster_ids: vec![target_cluster.cluster_id.clone()],
            });
        }
    }
    for source_cluster in &source.clusters {
        let region_ids = source_cluster
            .member_region_ids
            .iter()
            .filter(|region_id| persistent_regions.contains(*region_id))
            .cloned()
            .collect::<Vec<_>>();
        let target_cluster_ids = region_ids
            .iter()
            .filter_map(|region_id| target_cluster_by_region.get(region_id))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if target_cluster_ids.len() > 1 {
            changes.push(SemanticAtlasClusterChange {
                kind: SemanticAtlasClusterChangeKind::Split,
                region_ids,
                source_cluster_ids: vec![source_cluster.cluster_id.clone()],
                target_cluster_ids,
            });
        }
    }
    changes
}

fn valid_region_change(change: &SemanticAtlasRegionChange) -> bool {
    let bound = |region: &SemanticAtlasRegionEvidence| region.region_id() == &change.region_id;
    match (&change.kind, &change.before, &change.after) {
        (SemanticAtlasRegionChangeKind::Inserted, None, Some(after)) => bound(after),
        (SemanticAtlasRegionChangeKind::Removed, Some(before), None) => bound(before),
        (SemanticAtlasRegionChangeKind::Moved, Some(before), Some(after)) => {
            bound(before) && bound(after) && atlas_region_moved(before, after)
        }
        (SemanticAtlasRegionChangeKind::InterestChanged, Some(before), Some(after)) => {
            bound(before) && bound(after) && atlas_region_interest_changed(before, after)
        }
        _ => false,
    }
}

fn valid_cluster_change(change: &SemanticAtlasClusterChange) -> bool {
    if !canonical_nonempty_digests(&change.region_ids)
        || !canonical_nonempty_digests(&change.source_cluster_ids)
        || !canonical_nonempty_digests(&change.target_cluster_ids)
    {
        return false;
    }
    match change.kind {
        SemanticAtlasClusterChangeKind::Merged => {
            change.source_cluster_ids.len() > 1 && change.target_cluster_ids.len() == 1
        }
        SemanticAtlasClusterChangeKind::Split => {
            change.source_cluster_ids.len() == 1 && change.target_cluster_ids.len() > 1
        }
    }
}

fn canonical_nonempty_digests(values: &[SemanticDigest]) -> bool {
    !values.is_empty()
        && values.iter().all(|value| !value.as_str().is_empty())
        && values.windows(2).all(|pair| pair[0] < pair[1])
}

#[derive(Clone, Copy)]
enum AtlasLayoutSourceKind {
    Survey(usize),
    Regional(usize),
}

struct AtlasLayoutSource {
    kind: AtlasLayoutSourceKind,
    region_id: SemanticDigest,
    feature_vector: [u64; 8],
    bound_coordinate: Option<(i64, i64)>,
    dominant_feature: String,
}

fn build_atlas(
    mut sources: Vec<SemanticAtlasSource>,
    mut regional_sources: Vec<SemanticAtlasRegionalSource>,
) -> Result<SemanticAtlas, SemanticAtlasError> {
    let limits = SemanticAtlasLimits::default();
    sources.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    regional_sources.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    unique(
        sources.iter().map(|source| source.region_id.as_str()),
        "source region",
    )?;
    unique(
        regional_sources
            .iter()
            .map(|source| source.region_id.as_str()),
        "regional source region",
    )?;
    let submitted_sources = sources.len().saturating_add(regional_sources.len()) as u64;
    let mut submitted_ids = sources
        .iter()
        .map(|source| source.region_id.clone())
        .chain(
            regional_sources
                .iter()
                .map(|source| source.region_id.clone()),
        )
        .collect::<Vec<_>>();
    submitted_ids.sort();
    unique(
        submitted_ids.iter().map(SemanticDigest::as_str),
        "evidence region",
    )?;
    let omitted = submitted_ids
        .len()
        .saturating_sub(limits.max_regions as usize);
    submitted_ids.truncate(limits.max_regions as usize);
    let admitted_ids = submitted_ids.into_iter().collect::<BTreeSet<_>>();
    sources.retain(|source| admitted_ids.contains(&source.region_id));
    regional_sources.retain(|source| admitted_ids.contains(&source.region_id));
    let layout_sources = atlas_layout_sources(&sources, &regional_sources);
    let assignments = cluster_sources(&layout_sources, limits.max_world_clusters as usize);
    let mut clusters = Vec::new();
    let mut regions = Vec::new();
    let mut regional_regions = Vec::new();
    for (cluster_index, member_indices) in assignments.iter().enumerate() {
        let (cluster_longitude, cluster_latitude) = bound_cluster_center(
            member_indices
                .iter()
                .filter_map(|index| layout_sources[*index].bound_coordinate),
        )
        .unwrap_or_else(|| cluster_center(cluster_index, assignments.len()));
        let member_region_ids = member_indices
            .iter()
            .map(|index| layout_sources[*index].region_id.clone())
            .collect::<Vec<_>>();
        let cluster_id = cluster_identity(&member_region_ids);
        let cluster_dominant_feature = dominant_layout_feature(
            member_indices
                .iter()
                .map(|index| layout_sources[*index].dominant_feature.as_str())
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
        for (member_position, layout_index) in member_indices.iter().enumerate() {
            let layout = &layout_sources[*layout_index];
            match layout.kind {
                AtlasLayoutSourceKind::Survey(source_index) => {
                    let source = &sources[source_index];
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
                        sector_id: sector_id_for_coordinate(longitude, latitude),
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
                AtlasLayoutSourceKind::Regional(source_index) => {
                    let source = &regional_sources[source_index];
                    regional_regions.push(SemanticAtlasRegionalRegion {
                        region_id: source.region_id.clone(),
                        cluster_id: cluster_id.clone(),
                        sector_id: sector_id_for_coordinate(
                            source.semantic_longitude_microdegrees,
                            source.semantic_latitude_microdegrees,
                        ),
                        workload_id: source.workload_id.clone(),
                        scene_region_id: source.scene_region_id.clone(),
                        source_scene_id: source.source_scene_id.clone(),
                        source_admission_id: source.source_admission_id.clone(),
                        source_package_id: source.source_package_id.clone(),
                        source_package_revision: source.source_package_revision.clone(),
                        projection_packet_id: source.projection_packet_id.clone(),
                        semantic_longitude_microdegrees: source.semantic_longitude_microdegrees,
                        semantic_latitude_microdegrees: source.semantic_latitude_microdegrees,
                        angular_radius_microdegrees: 0,
                        native_objects: source.native_objects,
                        validity_boundaries: source.validity_boundaries,
                        omissions: source.omissions,
                        complete: source.complete,
                        dominant_feature: regional_dominant_feature(source),
                    });
                }
            }
        }
    }
    clusters.sort_by(|left, right| left.cluster_id.cmp(&right.cluster_id));
    regions.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    regional_regions.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    let sectors = build_atlas_sectors(&regions, &regional_regions);
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
    lineage.extend(regional_sources.iter().map(|source| SemanticAtlasLineage {
        kind: "admitted_regional_scene".to_owned(),
        identity: source.source_scene_id.to_string(),
        revision: source.projection_packet_id.to_string(),
    }));
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
            authority: "survey placements derive from retained admitted structure; admitted regional scenes retain their exact synthetic transform; visual proximity is not source truth".to_owned(),
            earth_crs: None,
        },
        layout_policy: SemanticAtlasLayoutPolicy {
            clustering: "deterministic bounded k-medoids over separately typed survey and regional structure features".to_owned(),
            placement: "bound regional synthetic coordinates remain exact; survey members use deterministic polar placement around the resulting cluster center".to_owned(),
            recluster_trigger: "an admitted survey or regional source set or exact source revision changes".to_owned(),
            zoom_rule: "zoom selects retained level of detail and never reclusters".to_owned(),
            distance_claim: "cluster membership reflects only separately typed admitted structure features; angular distance is presentation, not semantic similarity evidence".to_owned(),
        },
        sector_grid: atlas_sector_grid(),
        submitted_sources,
        sources,
        regional_sources,
        clusters,
        sectors,
        regions,
        regional_regions,
        limits,
        complete: omitted == 0,
        omissions,
        lineage,
    };
    atlas.atlas_revision = atlas_revision_digest(&atlas)?;
    atlas.atlas_id = atlas.atlas_revision.clone();
    Ok(atlas)
}

fn atlas_layout_sources(
    sources: &[SemanticAtlasSource],
    regional_sources: &[SemanticAtlasRegionalSource],
) -> Vec<AtlasLayoutSource> {
    let mut layout = sources
        .iter()
        .enumerate()
        .map(|(index, source)| AtlasLayoutSource {
            kind: AtlasLayoutSourceKind::Survey(index),
            region_id: source.region_id.clone(),
            feature_vector: source.feature_vector(),
            bound_coordinate: None,
            dominant_feature: dominant_feature(vec![source]),
        })
        .chain(
            regional_sources
                .iter()
                .enumerate()
                .map(|(index, source)| AtlasLayoutSource {
                    kind: AtlasLayoutSourceKind::Regional(index),
                    region_id: source.region_id.clone(),
                    feature_vector: source.feature_vector(),
                    bound_coordinate: Some((
                        source.semantic_longitude_microdegrees,
                        source.semantic_latitude_microdegrees,
                    )),
                    dominant_feature: regional_dominant_feature(source),
                }),
        )
        .collect::<Vec<_>>();
    layout.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    layout
}

fn cluster_sources(sources: &[AtlasLayoutSource], max_clusters: usize) -> Vec<Vec<usize>> {
    if sources.is_empty() {
        return Vec::new();
    }
    let cluster_count = integer_sqrt_ceil(sources.len()).min(max_clusters).max(1);
    let mut ordered = (0..sources.len()).collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        sources[*left]
            .feature_vector
            .cmp(&sources[*right].feature_vector)
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

fn feature_distance(left: &AtlasLayoutSource, right: &AtlasLayoutSource) -> u128 {
    left.feature_vector
        .into_iter()
        .zip(right.feature_vector)
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

fn bound_cluster_center(coordinates: impl IntoIterator<Item = (i64, i64)>) -> Option<(i64, i64)> {
    let coordinates = coordinates.into_iter().collect::<Vec<_>>();
    if coordinates.is_empty() {
        return None;
    }
    let (x, y, z) = coordinates.iter().fold(
        (0.0_f64, 0.0_f64, 0.0_f64),
        |(x, y, z), (longitude, latitude)| {
            let longitude = (*longitude as f64 / MICRODEGREES_PER_DEGREE).to_radians();
            let latitude = (*latitude as f64 / MICRODEGREES_PER_DEGREE).to_radians();
            (
                x + latitude.cos() * longitude.cos(),
                y + latitude.cos() * longitude.sin(),
                z + latitude.sin(),
            )
        },
    );
    let magnitude = x.hypot(y).hypot(z);
    if magnitude <= f64::EPSILON {
        return coordinates.into_iter().min();
    }
    Some((
        microdegrees(normalize_longitude(y.atan2(x).to_degrees())),
        microdegrees(z.atan2(x.hypot(y)).to_degrees()),
    ))
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

fn dominant_layout_feature(features: Vec<&str>) -> String {
    let mut counts = BTreeMap::new();
    for feature in features {
        *counts.entry(feature).or_insert(0_u64) += 1;
    }
    counts
        .into_iter()
        .max_by(|(left_feature, left_count), (right_feature, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| right_feature.cmp(left_feature))
        })
        .map_or_else(|| "unknown".to_owned(), |(feature, _)| feature.to_owned())
}

fn regional_dominant_feature(source: &SemanticAtlasRegionalSource) -> String {
    [
        ("native_feature", source.native_feature_objects),
        ("terrain_control", source.terrain_control_objects),
        ("hydrology", source.hydrology_objects),
        ("boundary", source.boundary_objects),
        ("poi", source.poi_objects),
        ("highway", source.highway_objects),
        ("road", source.road_objects),
        ("district", source.district_objects),
        ("lot", source.lot_objects),
        ("structure", source.structure_objects),
        ("utility", source.utility_objects),
        ("label", source.label_objects),
        ("beacon", source.beacon_objects),
        ("construction", source.construction_objects),
        ("connector", source.connector_objects),
    ]
    .into_iter()
    .enumerate()
    .max_by_key(|(index, (_, count))| (*count, std::cmp::Reverse(*index)))
    .map_or("native_feature", |(_, (feature, _))| feature)
    .to_owned()
}

fn sector_indices(longitude: i64, latitude: i64) -> (u16, u16) {
    let longitude_band = ((longitude + 180_000_000) / SECTOR_SIZE_MICRODEGREES)
        .clamp(0, i64::from(SECTOR_LONGITUDE_BANDS - 1)) as u16;
    let latitude_band = ((latitude + 90_000_000) / SECTOR_SIZE_MICRODEGREES)
        .clamp(0, i64::from(SECTOR_LATITUDE_BANDS - 1)) as u16;
    (longitude_band, latitude_band)
}

fn sector_identity(longitude_band: u16, latitude_band: u16) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.semantic-atlas-sector.v1");
    hasher.add_u64(u64::from(longitude_band));
    hasher.add_u64(u64::from(latitude_band));
    hasher.finish()
}

fn sector_id_for_coordinate(longitude: i64, latitude: i64) -> SemanticDigest {
    let (longitude_band, latitude_band) = sector_indices(longitude, latitude);
    sector_identity(longitude_band, latitude_band)
}

fn sector_bounds(longitude_band: u16, latitude_band: u16) -> (i64, i64, i64, i64) {
    let west = -180_000_000 + i64::from(longitude_band) * SECTOR_SIZE_MICRODEGREES;
    let south = -90_000_000 + i64::from(latitude_band) * SECTOR_SIZE_MICRODEGREES;
    (
        west,
        south,
        west + SECTOR_SIZE_MICRODEGREES,
        south + SECTOR_SIZE_MICRODEGREES,
    )
}

fn build_atlas_sectors(
    regions: &[SemanticAtlasRegion],
    regional_regions: &[SemanticAtlasRegionalRegion],
) -> Vec<SemanticAtlasSector> {
    let mut members = BTreeMap::<(u16, u16), Vec<SemanticDigest>>::new();
    for (region_id, longitude, latitude) in regions
        .iter()
        .map(|region| {
            (
                &region.region_id,
                region.semantic_longitude_microdegrees,
                region.semantic_latitude_microdegrees,
            )
        })
        .chain(regional_regions.iter().map(|region| {
            (
                &region.region_id,
                region.semantic_longitude_microdegrees,
                region.semantic_latitude_microdegrees,
            )
        }))
    {
        members
            .entry(sector_indices(longitude, latitude))
            .or_default()
            .push(region_id.clone());
    }
    let authority = "fixed synthetic partition cell used only for atlas membership; not surveyed coverage, native County footprint, source topology, or physical area";
    let mut sectors = members
        .into_iter()
        .map(|((longitude_band, latitude_band), mut member_region_ids)| {
            member_region_ids.sort();
            let (west, south, east, north) = sector_bounds(longitude_band, latitude_band);
            SemanticAtlasSector {
                sector_id: sector_identity(longitude_band, latitude_band),
                longitude_band,
                latitude_band,
                west_microdegrees: west,
                south_microdegrees: south,
                east_microdegrees: east,
                north_microdegrees: north,
                member_region_ids,
                authority: authority.to_owned(),
            }
        })
        .collect::<Vec<_>>();
    sectors.sort_by(|left, right| left.sector_id.cmp(&right.sector_id));
    sectors
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
    let revision = 3;
    let mut hasher = SemanticHasher::new("rey.contract.v1");
    hasher.add_str(id);
    hasher.add_u64(revision);
    ContractIdentity {
        id: id.to_owned(),
        revision,
        semantic_digest: hasher.finish(),
    }
}

fn atlas_sector_grid() -> ContractIdentity {
    ContractIdentity::new(
        "rey.semantic-atlas.fixed-30-degree-sectors",
        1,
        "stable occupied synthetic longitude/latitude cells; membership is not surveyed coverage, native footprint, source topology, or physical area",
    )
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

fn atlas_delta_digest(delta: &SemanticAtlasDelta) -> Result<SemanticDigest, SemanticAtlasError> {
    let mut normalized = delta.clone();
    normalized.delta_id = placeholder("rey.semantic-atlas-delta.placeholder");
    let bytes = serde_json::to_vec(&normalized)?;
    let mut hasher = SemanticHasher::new(SEMANTIC_ATLAS_DELTA_SCHEMA);
    hasher.add_bytes(&bytes);
    Ok(hasher.finish())
}

fn empty_atlas_revision() -> SemanticDigest {
    SemanticHasher::new("rey.semantic-atlas.empty.v1").finish()
}

fn validate_limits(limits: &SemanticAtlasLimits) -> Result<(), SemanticAtlasError> {
    if [
        limits.max_regions,
        limits.max_world_clusters,
        limits.max_members_per_cluster,
        limits.max_sectors,
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
    #[error("semantic atlas sector grid or membership is invalid")]
    Sector,
    #[error("semantic atlas workload does not match its source topography")]
    WorkloadBinding,
    #[error("admitted regional scene has no exact native-to-semantic atlas placement")]
    RegionalPlacement,
    #[error("admitted regional scene does not bind its exact retained atlas member")]
    RegionalAtlasBinding,
    #[error("semantic atlas limit is invalid")]
    Limit,
    #[error("semantic atlas digest does not match its content")]
    Digest,
    #[error("semantic atlas delta shape is invalid")]
    DeltaShape,
    #[error("semantic atlas delta digest does not match its content")]
    DeltaDigest,
    #[error("semantic atlas delta does not match its source and target revisions")]
    DeltaBinding,
    #[error(transparent)]
    Topography(#[from] crate::TopographyError),
    #[error(transparent)]
    RegionalScene(#[from] crate::RegionalSceneError),
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

    fn regional_source(id: &str, longitude: i64, latitude: i64) -> SemanticAtlasRegionalSource {
        let mut region = SemanticHasher::new("rey.semantic-atlas-regional-region.v1");
        region.add_str(id);
        region.add_str("county-demo");
        SemanticAtlasRegionalSource {
            region_id: region.finish(),
            workload_id: id.to_owned(),
            scene_region_id: "county-demo".to_owned(),
            source_scene_id: placeholder("regional-scene"),
            source_admission_id: placeholder("regional-admission"),
            source_package_id: placeholder("regional-package"),
            source_package_revision: placeholder("regional-package-revision"),
            projection_packet_id: placeholder("regional-packet"),
            semantic_longitude_microdegrees: longitude,
            semantic_latitude_microdegrees: latitude,
            complete: false,
            native_objects: 3,
            native_feature_objects: 1,
            terrain_control_objects: 1,
            hydrology_objects: 0,
            boundary_objects: 0,
            poi_objects: 1,
            highway_objects: 0,
            road_objects: 0,
            district_objects: 0,
            lot_objects: 0,
            structure_objects: 0,
            utility_objects: 0,
            label_objects: 0,
            beacon_objects: 0,
            construction_objects: 0,
            connector_objects: 0,
            validity_boundaries: 1,
            omissions: 2,
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
    fn regional_scene_is_a_separately_typed_exact_atlas_member() {
        let regional = regional_source("scene-admission", -42_000_000, 18_000_000);
        let atlas = SemanticAtlas::from_evidence_sources(
            vec![source("survey", 3, 1)],
            vec![regional.clone()],
        )
        .expect("combined atlas");
        let placed = &atlas.regional_regions[0];
        assert_eq!(atlas.regional_sources, vec![regional]);
        assert_eq!(placed.semantic_longitude_microdegrees, -42_000_000);
        assert_eq!(placed.semantic_latitude_microdegrees, 18_000_000);
        assert_eq!(placed.angular_radius_microdegrees, 0);
        let sector = atlas
            .sectors
            .iter()
            .find(|sector| sector.sector_id == placed.sector_id)
            .expect("regional sector");
        assert_eq!(sector.longitude_band, 4);
        assert_eq!(sector.latitude_band, 3);
        assert_eq!(sector.west_microdegrees, -60_000_000);
        assert_eq!(sector.east_microdegrees, -30_000_000);
        assert!(sector.member_region_ids.contains(&placed.region_id));
        assert!(
            atlas
                .clusters
                .iter()
                .any(|cluster| cluster.member_region_ids.contains(&placed.region_id))
        );
        assert_eq!(atlas.delta_from(None).expect("initial delta").inserted, 2);

        let mut changed_source = atlas.regional_sources[0].clone();
        changed_source.source_scene_id = placeholder("changed-regional-scene");
        changed_source.projection_packet_id = placeholder("changed-regional-packet");
        changed_source.semantic_longitude_microdegrees += 1_000_000;
        let changed =
            SemanticAtlas::from_evidence_sources(atlas.sources.clone(), vec![changed_source])
                .expect("changed combined atlas");
        let delta = changed.delta_from(Some(&atlas)).expect("regional delta");
        assert_eq!(delta.inserted, 0);
        assert_eq!(delta.removed, 0);
        assert_eq!(delta.moved, 1);
        assert_eq!(delta.interest_changed, 1);
        delta
            .verify_between(Some(&atlas), &changed)
            .expect("bound regional delta");

        let mut tampered = atlas.clone();
        tampered.regional_regions[0].semantic_longitude_microdegrees += 1;
        assert!(matches!(
            tampered.verify(),
            Err(SemanticAtlasError::Membership)
        ));

        let mut tampered = atlas;
        tampered.sectors[0].authority = "native county footprint".to_owned();
        assert!(matches!(tampered.verify(), Err(SemanticAtlasError::Sector)));
    }

    #[test]
    fn atlas_delta_is_directed_typed_and_content_identified() {
        let source = SemanticAtlas::from_sources(vec![
            source("docs", 1, 8),
            source("code", 9, 1),
            source("mixed", 4, 4),
        ])
        .expect("source atlas");
        let initial = source.delta_from(None).expect("initial delta");
        assert_eq!(initial.inserted, 3);
        assert_eq!(initial.removed, 0);
        initial.verify_between(None, &source).expect("bound delta");

        let mut target = source.clone();
        let changed_region_id = target.sources[0].region_id.clone();
        target.sources[0].source_topography_revision = placeholder("changed-source");
        let changed_region = target
            .regions
            .iter_mut()
            .find(|region| region.region_id == changed_region_id)
            .expect("changed region");
        changed_region.source_topography_revision =
            target.sources[0].source_topography_revision.clone();
        changed_region.semantic_longitude_microdegrees += 1;
        target.atlas_revision = atlas_revision_digest(&target).expect("target revision");
        target.atlas_id = target.atlas_revision.clone();
        target.verify().expect("target atlas");

        let delta = target.delta_from(Some(&source)).expect("directed delta");
        assert_eq!(delta.inserted, 0);
        assert_eq!(delta.removed, 0);
        assert_eq!(delta.moved, 1);
        assert_eq!(delta.interest_changed, 1);
        assert_eq!(delta.source_revision, source.atlas_revision);
        assert_eq!(delta.target_revision, target.atlas_revision);
        delta
            .verify_between(Some(&source), &target)
            .expect("bound delta");

        let reverse = source.delta_from(Some(&target)).expect("reverse delta");
        assert_ne!(delta.delta_id, reverse.delta_id);
        assert_eq!(reverse.source_revision, target.atlas_revision);
        assert_eq!(reverse.target_revision, source.atlas_revision);

        let mut tampered = delta.clone();
        tampered.moved = 0;
        assert!(matches!(
            tampered.verify(),
            Err(SemanticAtlasError::DeltaDigest)
        ));
    }

    #[test]
    fn atlas_delta_keeps_merge_split_insert_and_remove_distinct() {
        let split = SemanticAtlas::from_sources(vec![
            source("one", 9, 1),
            source("two", 8, 2),
            source("three", 1, 9),
            source("four", 2, 8),
        ])
        .expect("split atlas");
        assert!(split.clusters.len() > 1);
        let all_region_ids = split
            .regions
            .iter()
            .map(|region| region.region_id.clone())
            .collect::<Vec<_>>();
        let merged = regroup(split.clone(), vec![all_region_ids]);

        let merge_delta = merged.delta_from(Some(&split)).expect("merge delta");
        assert_eq!(merge_delta.merged, 1);
        assert_eq!(merge_delta.split, 0);
        assert_eq!(merge_delta.inserted, 0);
        assert_eq!(merge_delta.removed, 0);

        let split_delta = split.delta_from(Some(&merged)).expect("split delta");
        assert_eq!(split_delta.merged, 0);
        assert_eq!(split_delta.split, 1);

        let reduced =
            SemanticAtlas::from_sources(split.sources[..3].to_vec()).expect("reduced atlas");
        let remove_delta = reduced.delta_from(Some(&split)).expect("remove delta");
        assert_eq!(remove_delta.removed, 1);
        let insert_delta = split.delta_from(Some(&reduced)).expect("insert delta");
        assert_eq!(insert_delta.inserted, 1);
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

    fn regroup(mut atlas: SemanticAtlas, mut groups: Vec<Vec<SemanticDigest>>) -> SemanticAtlas {
        for group in &mut groups {
            group.sort();
        }
        groups.sort();
        atlas.clusters = groups
            .iter()
            .enumerate()
            .map(|(index, region_ids)| {
                let cluster_id = cluster_identity(region_ids);
                for region in &mut atlas.regions {
                    if region_ids.contains(&region.region_id) {
                        region.cluster_id = cluster_id.clone();
                    }
                }
                let sources = region_ids
                    .iter()
                    .filter_map(|region_id| {
                        atlas
                            .sources
                            .iter()
                            .find(|source| &source.region_id == region_id)
                    })
                    .collect::<Vec<_>>();
                let (longitude, latitude) = cluster_center(index, groups.len());
                SemanticAtlasCluster {
                    cluster_id,
                    semantic_longitude_microdegrees: longitude,
                    semantic_latitude_microdegrees: latitude,
                    angular_radius_microdegrees: 22_000_000,
                    member_region_ids: region_ids.clone(),
                    dominant_feature: dominant_feature(sources),
                }
            })
            .collect();
        atlas
            .clusters
            .sort_by(|left, right| left.cluster_id.cmp(&right.cluster_id));
        atlas.atlas_revision = atlas_revision_digest(&atlas).expect("regrouped revision");
        atlas.atlas_id = atlas.atlas_revision.clone();
        atlas.verify().expect("regrouped atlas");
        atlas
    }
}
