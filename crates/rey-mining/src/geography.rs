use std::collections::{BTreeMap, BTreeSet};

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    AdmittedRegionalScene, RegionalBounds, RegionalTerrainGrid, RegionalTerrainGridCell,
    RegionalValidityClass, SemanticAtlas, SemanticAtlasError, SemanticAtlasRegionalSource,
};

pub const REGIONAL_GEOGRAPHY_COMPOSITION_SCHEMA: &str = "rey.regional-geography-composition.v2";
const COMPOSITION_AUTHORITY: &str = "deterministic exact-boundary assessment only; no merge, interpolation, synthesis, rendered coverage, or geographic truth authority";
const MAX_COMPOSITION_MEMBERS: usize = 128;
const MAX_COMPOSITION_PAIRS: usize = 8_128;
const MAX_COMPOSITION_CONFLICTS: usize = 16_256;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionalGeographyRelationship {
    EdgeAdjacent,
    CornerTouch,
    Overlap,
    Disjoint,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionalGeographyBoundaryAxis {
    Longitude,
    Latitude,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionalGeographyTerrainStatus {
    Qualified,
    Conflict,
    Unsupported,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionalGeographyConflictKind {
    NativeOverlap,
    AntimeridianUnsupported,
    SeamSampleAlignment,
    SeamValidity,
    SeamElevation,
    SeamMaterial,
    SeamTerrainUnsupported,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionalGeographyStitchStatus {
    SinglePackage,
    Ready,
    Blocked,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalGeographyCompositionLimits {
    pub max_members: u64,
    pub max_pairs: u64,
    pub max_conflicts: u64,
}

impl Default for RegionalGeographyCompositionLimits {
    fn default() -> Self {
        Self {
            max_members: MAX_COMPOSITION_MEMBERS as u64,
            max_pairs: MAX_COMPOSITION_PAIRS as u64,
            max_conflicts: MAX_COMPOSITION_CONFLICTS as u64,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalGeographyMember {
    pub member_id: SemanticDigest,
    pub workload_id: String,
    pub region_id: String,
    pub scene_id: SemanticDigest,
    pub atlas_region_id: SemanticDigest,
    pub admitted_atlas_revision: SemanticDigest,
    pub admission_id: SemanticDigest,
    pub package_id: SemanticDigest,
    pub package_revision: SemanticDigest,
    pub projection_packet_id: SemanticDigest,
    pub terrain_program_id: Option<SemanticDigest>,
    pub terrain_dataset_id: Option<SemanticDigest>,
    pub native_bounds: RegionalBounds,
    pub terrain_valid_vertices: u64,
    pub terrain_no_data_vertices: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalGeographySharedBoundary {
    pub axis: RegionalGeographyBoundaryAxis,
    pub coordinate_microdegrees: i64,
    pub start_microdegrees: i64,
    pub end_microdegrees: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalGeographySeam {
    pub seam_id: SemanticDigest,
    pub member_ids: [SemanticDigest; 2],
    pub relationship: RegionalGeographyRelationship,
    pub shared_boundary: Option<RegionalGeographySharedBoundary>,
    pub longitude_gap_microdegrees: u64,
    pub latitude_gap_microdegrees: u64,
    pub terrain_status: RegionalGeographyTerrainStatus,
    pub compared_vertices: u64,
    pub valid_vertices: u64,
    pub no_data_vertices: u64,
    pub validity_conflicts: u64,
    pub elevation_conflicts: u64,
    pub material_conflicts: u64,
    pub max_elevation_delta_micrometers: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalGeographyConflict {
    pub conflict_id: SemanticDigest,
    pub seam_id: SemanticDigest,
    pub member_ids: [SemanticDigest; 2],
    pub kind: RegionalGeographyConflictKind,
    pub count: u64,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalGeographyTerrainComponent {
    pub component_id: SemanticDigest,
    pub member_ids: Vec<SemanticDigest>,
    pub qualified_seam_ids: Vec<SemanticDigest>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalGeographyComposition {
    pub schema: String,
    pub composition_id: SemanticDigest,
    pub compiler: ContractIdentity,
    pub atlas_revision: SemanticDigest,
    pub members: Vec<RegionalGeographyMember>,
    pub seams: Vec<RegionalGeographySeam>,
    pub conflicts: Vec<RegionalGeographyConflict>,
    pub terrain_components: Vec<RegionalGeographyTerrainComponent>,
    pub stitch_status: RegionalGeographyStitchStatus,
    pub limits: RegionalGeographyCompositionLimits,
    pub complete: bool,
    pub authority: String,
}

impl RegionalGeographyComposition {
    pub fn from_admitted_scenes<'a>(
        atlas: &SemanticAtlas,
        scenes: impl IntoIterator<Item = (&'a str, &'a AdmittedRegionalScene)>,
    ) -> Result<Option<Self>, RegionalGeographyCompositionError> {
        atlas.verify()?;
        let mut sources = scenes
            .into_iter()
            .map(|(workload_id, scene)| {
                scene.verify()?;
                let expected = SemanticAtlasRegionalSource::from_scene(workload_id, scene)?;
                if atlas
                    .regional_sources
                    .iter()
                    .find(|source| source.region_id == expected.region_id)
                    != Some(&expected)
                {
                    return Err(RegionalGeographyCompositionError::AtlasBinding);
                }
                let member = member(workload_id, scene, expected.region_id)?;
                Ok((member, scene))
            })
            .collect::<Result<Vec<_>, RegionalGeographyCompositionError>>()?;
        if sources.is_empty() {
            return Ok(None);
        }
        if sources.len() > MAX_COMPOSITION_MEMBERS {
            return Err(RegionalGeographyCompositionError::Limit);
        }
        sources.sort_by(|left, right| left.0.member_id.cmp(&right.0.member_id));
        if sources
            .windows(2)
            .any(|pair| pair[0].0.member_id == pair[1].0.member_id)
        {
            return Err(RegionalGeographyCompositionError::DuplicateMember);
        }

        let pair_count = sources
            .len()
            .saturating_mul(sources.len().saturating_sub(1))
            / 2;
        if pair_count > MAX_COMPOSITION_PAIRS {
            return Err(RegionalGeographyCompositionError::Limit);
        }
        let mut seams = Vec::with_capacity(pair_count);
        let mut conflicts = Vec::new();
        for left in 0..sources.len() {
            for right in (left + 1)..sources.len() {
                let (seam, mut pair_conflicts) = assess_pair(
                    &sources[left].0,
                    sources[left].1,
                    &sources[right].0,
                    sources[right].1,
                )?;
                seams.push(seam);
                conflicts.append(&mut pair_conflicts);
            }
        }
        if conflicts.len() > MAX_COMPOSITION_CONFLICTS {
            return Err(RegionalGeographyCompositionError::Limit);
        }
        seams.sort_by(|left, right| left.seam_id.cmp(&right.seam_id));
        conflicts.sort_by(|left, right| left.conflict_id.cmp(&right.conflict_id));
        let members = sources
            .into_iter()
            .map(|(member, _)| member)
            .collect::<Vec<_>>();
        let terrain_components =
            terrain_components(&atlas.atlas_revision, &members, &seams, &conflicts);
        let stitch_status = stitch_status(&members, &seams, &conflicts);
        let mut composition = Self {
            schema: REGIONAL_GEOGRAPHY_COMPOSITION_SCHEMA.to_owned(),
            composition_id: placeholder("regional-geography-composition"),
            compiler: composition_compiler(),
            atlas_revision: atlas.atlas_revision.clone(),
            members,
            seams,
            conflicts,
            terrain_components,
            stitch_status,
            limits: RegionalGeographyCompositionLimits::default(),
            complete: true,
            authority: COMPOSITION_AUTHORITY.to_owned(),
        };
        composition.composition_id = composition_digest(&composition)?;
        composition.verify()?;
        Ok(Some(composition))
    }

    pub fn verify(&self) -> Result<(), RegionalGeographyCompositionError> {
        let pair_count = self
            .members
            .len()
            .saturating_mul(self.members.len().saturating_sub(1))
            / 2;
        if self.schema != REGIONAL_GEOGRAPHY_COMPOSITION_SCHEMA
            || self.compiler != composition_compiler()
            || self.atlas_revision.as_str().is_empty()
            || self.members.is_empty()
            || self.members.len() > self.limits.max_members as usize
            || pair_count != self.seams.len()
            || self.seams.len() > self.limits.max_pairs as usize
            || self.conflicts.len() > self.limits.max_conflicts as usize
            || self.limits != RegionalGeographyCompositionLimits::default()
            || !self.complete
            || self.authority != COMPOSITION_AUTHORITY
        {
            return Err(RegionalGeographyCompositionError::Shape);
        }
        canonical_unique(self.members.iter().map(|member| &member.member_id))?;
        canonical_unique(self.seams.iter().map(|seam| &seam.seam_id))?;
        canonical_unique(self.conflicts.iter().map(|conflict| &conflict.conflict_id))?;
        canonical_unique(
            self.terrain_components
                .iter()
                .map(|component| &component.component_id),
        )?;
        let member_ids = self
            .members
            .iter()
            .map(|member| member.member_id.clone())
            .collect::<BTreeSet<_>>();
        if self.members.iter().any(|member| {
            member.workload_id.is_empty()
                || member.region_id.is_empty()
                || member.admitted_atlas_revision.as_str().is_empty()
                || (!member.native_bounds.crosses_antimeridian
                    && member.native_bounds.west_microdegrees
                        >= member.native_bounds.east_microdegrees)
                || (member.native_bounds.crosses_antimeridian
                    && member.native_bounds.west_microdegrees
                        <= member.native_bounds.east_microdegrees)
                || member.native_bounds.south_microdegrees
                    >= member.native_bounds.north_microdegrees
                || member.member_id != member_digest(member)
        }) {
            return Err(RegionalGeographyCompositionError::Shape);
        }
        for seam in &self.seams {
            if seam.member_ids[0] >= seam.member_ids[1]
                || !seam
                    .member_ids
                    .iter()
                    .all(|member| member_ids.contains(member))
                || seam.seam_id != seam_digest(&seam.member_ids)
                || !valid_seam_shape(seam)
            {
                return Err(RegionalGeographyCompositionError::Shape);
            }
        }
        for conflict in &self.conflicts {
            let Some(seam) = self
                .seams
                .iter()
                .find(|seam| seam.seam_id == conflict.seam_id)
            else {
                return Err(RegionalGeographyCompositionError::Shape);
            };
            if conflict.member_ids != seam.member_ids
                || conflict.count == 0
                || conflict.detail.is_empty()
                || conflict.conflict_id != conflict_digest(conflict)
            {
                return Err(RegionalGeographyCompositionError::Shape);
            }
        }
        if self.terrain_components
            != terrain_components(
                &self.atlas_revision,
                &self.members,
                &self.seams,
                &self.conflicts,
            )
        {
            return Err(RegionalGeographyCompositionError::Shape);
        }
        if self.stitch_status != stitch_status(&self.members, &self.seams, &self.conflicts)
            || self.composition_id != composition_digest(self)?
        {
            return Err(RegionalGeographyCompositionError::Digest);
        }
        Ok(())
    }
}

fn terrain_components(
    atlas_revision: &SemanticDigest,
    members: &[RegionalGeographyMember],
    seams: &[RegionalGeographySeam],
    conflicts: &[RegionalGeographyConflict],
) -> Vec<RegionalGeographyTerrainComponent> {
    let terrain_member_ids = members
        .iter()
        .filter(|member| member.terrain_dataset_id.is_some())
        .map(|member| member.member_id.clone())
        .collect::<BTreeSet<_>>();
    let qualified_seams = seams
        .iter()
        .filter(|seam| {
            seam.relationship == RegionalGeographyRelationship::EdgeAdjacent
                && seam.terrain_status == RegionalGeographyTerrainStatus::Qualified
                && seam
                    .member_ids
                    .iter()
                    .all(|member_id| terrain_member_ids.contains(member_id))
                && !conflicts
                    .iter()
                    .any(|conflict| conflict.seam_id == seam.seam_id)
        })
        .collect::<Vec<_>>();
    let mut remaining = terrain_member_ids;
    let mut components = Vec::new();
    while let Some(seed) = remaining.iter().next().cloned() {
        let mut connected = BTreeSet::from([seed]);
        loop {
            let before = connected.len();
            for seam in &qualified_seams {
                if connected.contains(&seam.member_ids[0]) {
                    connected.insert(seam.member_ids[1].clone());
                }
                if connected.contains(&seam.member_ids[1]) {
                    connected.insert(seam.member_ids[0].clone());
                }
            }
            if connected.len() == before {
                break;
            }
        }
        for member_id in &connected {
            remaining.remove(member_id);
        }
        if conflicts.iter().any(|conflict| {
            conflict
                .member_ids
                .iter()
                .all(|member_id| connected.contains(member_id))
        }) {
            for member_id in connected {
                let member_ids = vec![member_id];
                let component_id =
                    terrain_component_digest(atlas_revision, member_ids.iter(), std::iter::empty());
                components.push(RegionalGeographyTerrainComponent {
                    component_id,
                    member_ids,
                    qualified_seam_ids: Vec::new(),
                });
            }
            continue;
        }
        let member_ids = connected.into_iter().collect::<Vec<_>>();
        let qualified_seam_ids = qualified_seams
            .iter()
            .filter(|seam| {
                seam.member_ids
                    .iter()
                    .all(|member_id| member_ids.binary_search(member_id).is_ok())
            })
            .map(|seam| seam.seam_id.clone())
            .collect::<Vec<_>>();
        let component_id =
            terrain_component_digest(atlas_revision, member_ids.iter(), qualified_seam_ids.iter());
        components.push(RegionalGeographyTerrainComponent {
            component_id,
            member_ids,
            qualified_seam_ids,
        });
    }
    components.sort_by(|left, right| left.component_id.cmp(&right.component_id));
    components
}

fn terrain_component_digest<'a>(
    atlas_revision: &SemanticDigest,
    member_ids: impl Iterator<Item = &'a SemanticDigest>,
    seam_ids: impl Iterator<Item = &'a SemanticDigest>,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.regional-geography-terrain-component.v1");
    hasher.add_str(atlas_revision.as_str());
    for member_id in member_ids {
        hasher.add_str(member_id.as_str());
    }
    for seam_id in seam_ids {
        hasher.add_str(seam_id.as_str());
    }
    hasher.finish()
}

fn member(
    workload_id: &str,
    scene: &AdmittedRegionalScene,
    atlas_region_id: SemanticDigest,
) -> Result<RegionalGeographyMember, RegionalGeographyCompositionError> {
    let admitted_atlas_revision = scene
        .artifacts
        .admitted_atlas_revision
        .clone()
        .ok_or(RegionalGeographyCompositionError::AtlasBinding)?;
    let grid = terrain_grid(scene);
    let (terrain_valid_vertices, terrain_no_data_vertices) = grid
        .map(RegionalTerrainGrid::validity_counts)
        .transpose()?
        .unwrap_or_default();
    let mut member = RegionalGeographyMember {
        member_id: placeholder("regional-geography-member"),
        workload_id: workload_id.to_owned(),
        region_id: scene.region_id.clone(),
        scene_id: scene.scene_id.clone(),
        atlas_region_id,
        admitted_atlas_revision,
        admission_id: scene.admission.admission_id.clone(),
        package_id: scene.admission.package_id.clone(),
        package_revision: scene.admission.package_snapshot_revision.clone(),
        projection_packet_id: scene.projection.packet_id.clone(),
        terrain_program_id: scene.projection.terrain_program_id.clone(),
        terrain_dataset_id: grid.map(|grid| grid.dataset_id.clone()),
        native_bounds: scene.native_bounds.clone(),
        terrain_valid_vertices,
        terrain_no_data_vertices,
    };
    member.member_id = member_digest(&member);
    Ok(member)
}

fn assess_pair(
    left: &RegionalGeographyMember,
    left_scene: &AdmittedRegionalScene,
    right: &RegionalGeographyMember,
    right_scene: &AdmittedRegionalScene,
) -> Result<
    (RegionalGeographySeam, Vec<RegionalGeographyConflict>),
    RegionalGeographyCompositionError,
> {
    let member_ids = [left.member_id.clone(), right.member_id.clone()];
    let seam_id = seam_digest(&member_ids);
    let mut seam = RegionalGeographySeam {
        seam_id: seam_id.clone(),
        member_ids: member_ids.clone(),
        relationship: RegionalGeographyRelationship::Disjoint,
        shared_boundary: None,
        longitude_gap_microdegrees: axis_gap(
            left.native_bounds.west_microdegrees,
            left.native_bounds.east_microdegrees,
            right.native_bounds.west_microdegrees,
            right.native_bounds.east_microdegrees,
        ),
        latitude_gap_microdegrees: axis_gap(
            left.native_bounds.south_microdegrees,
            left.native_bounds.north_microdegrees,
            right.native_bounds.south_microdegrees,
            right.native_bounds.north_microdegrees,
        ),
        terrain_status: RegionalGeographyTerrainStatus::NotApplicable,
        compared_vertices: 0,
        valid_vertices: 0,
        no_data_vertices: 0,
        validity_conflicts: 0,
        elevation_conflicts: 0,
        material_conflicts: 0,
        max_elevation_delta_micrometers: None,
    };
    let mut conflicts = Vec::new();
    if left.native_bounds.crosses_antimeridian || right.native_bounds.crosses_antimeridian {
        seam.relationship = RegionalGeographyRelationship::Unsupported;
        seam.terrain_status = RegionalGeographyTerrainStatus::Unsupported;
        conflicts.push(conflict(
            &seam,
            RegionalGeographyConflictKind::AntimeridianUnsupported,
            1,
            "cross-antimeridian package relationships require a separately qualified wrap policy",
        ));
        return Ok((seam, conflicts));
    }

    let longitude_overlap = overlap(
        left.native_bounds.west_microdegrees,
        left.native_bounds.east_microdegrees,
        right.native_bounds.west_microdegrees,
        right.native_bounds.east_microdegrees,
    );
    let latitude_overlap = overlap(
        left.native_bounds.south_microdegrees,
        left.native_bounds.north_microdegrees,
        right.native_bounds.south_microdegrees,
        right.native_bounds.north_microdegrees,
    );
    if longitude_overlap > 0 && latitude_overlap > 0 {
        seam.relationship = RegionalGeographyRelationship::Overlap;
        conflicts.push(conflict(
            &seam,
            RegionalGeographyConflictKind::NativeOverlap,
            1,
            "native package bounds overlap; no precedence or merge policy is admitted",
        ));
        return Ok((seam, conflicts));
    }

    seam.shared_boundary = shared_boundary(&left.native_bounds, &right.native_bounds);
    if seam.shared_boundary.is_some() {
        seam.relationship = RegionalGeographyRelationship::EdgeAdjacent;
        assess_terrain_boundary(&mut seam, left_scene, right_scene, &mut conflicts)?;
    } else if seam.longitude_gap_microdegrees == 0 && seam.latitude_gap_microdegrees == 0 {
        seam.relationship = RegionalGeographyRelationship::CornerTouch;
    }
    Ok((seam, conflicts))
}

fn assess_terrain_boundary(
    seam: &mut RegionalGeographySeam,
    left_scene: &AdmittedRegionalScene,
    right_scene: &AdmittedRegionalScene,
    conflicts: &mut Vec<RegionalGeographyConflict>,
) -> Result<(), RegionalGeographyCompositionError> {
    let boundary = seam
        .shared_boundary
        .as_ref()
        .ok_or(RegionalGeographyCompositionError::Shape)?;
    let Some(left) = terrain_grid(left_scene) else {
        seam.terrain_status = RegionalGeographyTerrainStatus::Unsupported;
        conflicts.push(conflict(
            seam,
            RegionalGeographyConflictKind::SeamTerrainUnsupported,
            1,
            "left package has no qualified terrain grid at the shared boundary",
        ));
        return Ok(());
    };
    let Some(right) = terrain_grid(right_scene) else {
        seam.terrain_status = RegionalGeographyTerrainStatus::Unsupported;
        conflicts.push(conflict(
            seam,
            RegionalGeographyConflictKind::SeamTerrainUnsupported,
            1,
            "right package has no qualified terrain grid at the shared boundary",
        ));
        return Ok(());
    };
    let left = boundary_cells(left, boundary)?;
    let right = boundary_cells(right, boundary)?;
    if left.is_empty() || left.keys().ne(right.keys()) {
        seam.terrain_status = RegionalGeographyTerrainStatus::Unsupported;
        conflicts.push(conflict(
            seam,
            RegionalGeographyConflictKind::SeamSampleAlignment,
            1,
            "terrain boundary sample coordinates do not align exactly; resampling is not implicit",
        ));
        return Ok(());
    }
    seam.compared_vertices = left.len() as u64;
    let mut max_delta = 0_u64;
    for (coordinate, left) in left {
        let right = right
            .get(&coordinate)
            .ok_or(RegionalGeographyCompositionError::Shape)?;
        match (left.validity, right.validity) {
            (RegionalValidityClass::Valid, RegionalValidityClass::Valid) => {
                seam.valid_vertices += 1;
                let left_height = left
                    .elevation_micrometers
                    .ok_or(RegionalGeographyCompositionError::Shape)?;
                let right_height = right
                    .elevation_micrometers
                    .ok_or(RegionalGeographyCompositionError::Shape)?;
                let delta = left_height.abs_diff(right_height);
                max_delta = max_delta.max(delta);
                if delta != 0 {
                    seam.elevation_conflicts += 1;
                }
                if left.material != right.material {
                    seam.material_conflicts += 1;
                }
            }
            (RegionalValidityClass::NoData, RegionalValidityClass::NoData) => {
                seam.no_data_vertices += 1;
            }
            _ => seam.validity_conflicts += 1,
        }
    }
    seam.max_elevation_delta_micrometers = (seam.valid_vertices > 0).then_some(max_delta);
    if seam.validity_conflicts > 0 {
        conflicts.push(conflict(
            seam,
            RegionalGeographyConflictKind::SeamValidity,
            seam.validity_conflicts,
            "shared terrain samples disagree on explicit validity",
        ));
    }
    if seam.elevation_conflicts > 0 {
        conflicts.push(conflict(
            seam,
            RegionalGeographyConflictKind::SeamElevation,
            seam.elevation_conflicts,
            "shared valid terrain samples disagree on exact admitted elevation",
        ));
    }
    if seam.material_conflicts > 0 {
        conflicts.push(conflict(
            seam,
            RegionalGeographyConflictKind::SeamMaterial,
            seam.material_conflicts,
            "shared valid terrain samples disagree on exact admitted material",
        ));
    }
    if seam.valid_vertices == 0 && conflicts.is_empty() {
        conflicts.push(conflict(
            seam,
            RegionalGeographyConflictKind::SeamTerrainUnsupported,
            seam.no_data_vertices.max(1),
            "shared boundary has no valid terrain support",
        ));
    }
    seam.terrain_status = if conflicts.is_empty() {
        RegionalGeographyTerrainStatus::Qualified
    } else if conflicts.iter().any(|conflict| {
        matches!(
            conflict.kind,
            RegionalGeographyConflictKind::SeamSampleAlignment
                | RegionalGeographyConflictKind::SeamTerrainUnsupported
        )
    }) {
        RegionalGeographyTerrainStatus::Unsupported
    } else {
        RegionalGeographyTerrainStatus::Conflict
    };
    Ok(())
}

fn terrain_grid(scene: &AdmittedRegionalScene) -> Option<&RegionalTerrainGrid> {
    scene
        .projection
        .terrain
        .as_ref()
        .and_then(|terrain| terrain.grid.as_ref())
}

fn boundary_cells(
    grid: &RegionalTerrainGrid,
    boundary: &RegionalGeographySharedBoundary,
) -> Result<BTreeMap<i64, RegionalTerrainGridCell>, RegionalGeographyCompositionError> {
    Ok(grid
        .expanded_cells()?
        .iter()
        .filter_map(|cell| {
            let (fixed, varying) = match boundary.axis {
                RegionalGeographyBoundaryAxis::Longitude => {
                    (cell.native_position[0], cell.native_position[1])
                }
                RegionalGeographyBoundaryAxis::Latitude => {
                    (cell.native_position[1], cell.native_position[0])
                }
            };
            (fixed == boundary.coordinate_microdegrees
                && varying >= boundary.start_microdegrees
                && varying <= boundary.end_microdegrees)
                .then_some((varying, cell.clone()))
        })
        .collect())
}

fn shared_boundary(
    left: &RegionalBounds,
    right: &RegionalBounds,
) -> Option<RegionalGeographySharedBoundary> {
    let latitude_start = left.south_microdegrees.max(right.south_microdegrees);
    let latitude_end = left.north_microdegrees.min(right.north_microdegrees);
    if latitude_start < latitude_end
        && (left.east_microdegrees == right.west_microdegrees
            || right.east_microdegrees == left.west_microdegrees)
    {
        return Some(RegionalGeographySharedBoundary {
            axis: RegionalGeographyBoundaryAxis::Longitude,
            coordinate_microdegrees: if left.east_microdegrees == right.west_microdegrees {
                left.east_microdegrees
            } else {
                right.east_microdegrees
            },
            start_microdegrees: latitude_start,
            end_microdegrees: latitude_end,
        });
    }
    let longitude_start = left.west_microdegrees.max(right.west_microdegrees);
    let longitude_end = left.east_microdegrees.min(right.east_microdegrees);
    if longitude_start < longitude_end
        && (left.north_microdegrees == right.south_microdegrees
            || right.north_microdegrees == left.south_microdegrees)
    {
        return Some(RegionalGeographySharedBoundary {
            axis: RegionalGeographyBoundaryAxis::Latitude,
            coordinate_microdegrees: if left.north_microdegrees == right.south_microdegrees {
                left.north_microdegrees
            } else {
                right.north_microdegrees
            },
            start_microdegrees: longitude_start,
            end_microdegrees: longitude_end,
        });
    }
    None
}

fn stitch_status(
    members: &[RegionalGeographyMember],
    seams: &[RegionalGeographySeam],
    conflicts: &[RegionalGeographyConflict],
) -> RegionalGeographyStitchStatus {
    if members.len() == 1 {
        return RegionalGeographyStitchStatus::SinglePackage;
    }
    if !conflicts.is_empty() {
        return RegionalGeographyStitchStatus::Blocked;
    }
    let mut connected = BTreeSet::from([members[0].member_id.clone()]);
    loop {
        let before = connected.len();
        for seam in seams.iter().filter(|seam| {
            seam.relationship == RegionalGeographyRelationship::EdgeAdjacent
                && seam.terrain_status == RegionalGeographyTerrainStatus::Qualified
        }) {
            if connected.contains(&seam.member_ids[0]) {
                connected.insert(seam.member_ids[1].clone());
            }
            if connected.contains(&seam.member_ids[1]) {
                connected.insert(seam.member_ids[0].clone());
            }
        }
        if connected.len() == before {
            break;
        }
    }
    if connected.len() == members.len() {
        RegionalGeographyStitchStatus::Ready
    } else {
        RegionalGeographyStitchStatus::Blocked
    }
}

fn valid_seam_shape(seam: &RegionalGeographySeam) -> bool {
    if seam.compared_vertices
        != seam
            .valid_vertices
            .saturating_add(seam.no_data_vertices)
            .saturating_add(seam.validity_conflicts)
        || (seam.valid_vertices > 0) != seam.max_elevation_delta_micrometers.is_some()
    {
        return false;
    }
    match seam.relationship {
        RegionalGeographyRelationship::EdgeAdjacent => {
            seam.shared_boundary.is_some()
                && seam.terrain_status != RegionalGeographyTerrainStatus::NotApplicable
        }
        RegionalGeographyRelationship::Unsupported => {
            seam.shared_boundary.is_none()
                && seam.terrain_status == RegionalGeographyTerrainStatus::Unsupported
        }
        _ => {
            seam.shared_boundary.is_none()
                && seam.terrain_status == RegionalGeographyTerrainStatus::NotApplicable
        }
    }
}

fn member_digest(member: &RegionalGeographyMember) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.regional-geography-member.v1");
    hasher.add_str(&member.workload_id);
    hasher.add_str(&member.region_id);
    hasher.add_str(member.scene_id.as_str());
    hasher.add_str(member.package_revision.as_str());
    hasher.finish()
}

fn seam_digest(member_ids: &[SemanticDigest; 2]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.regional-geography-seam.v1");
    hasher.add_str(member_ids[0].as_str());
    hasher.add_str(member_ids[1].as_str());
    hasher.finish()
}

fn conflict(
    seam: &RegionalGeographySeam,
    kind: RegionalGeographyConflictKind,
    count: u64,
    detail: &str,
) -> RegionalGeographyConflict {
    let mut conflict = RegionalGeographyConflict {
        conflict_id: placeholder("regional-geography-conflict"),
        seam_id: seam.seam_id.clone(),
        member_ids: seam.member_ids.clone(),
        kind,
        count,
        detail: detail.to_owned(),
    };
    conflict.conflict_id = conflict_digest(&conflict);
    conflict
}

fn conflict_digest(conflict: &RegionalGeographyConflict) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.regional-geography-conflict.v1");
    hasher.add_str(conflict.seam_id.as_str());
    hasher.add_str(conflict_kind_name(conflict.kind));
    hasher.add_u64(conflict.count);
    hasher.add_str(&conflict.detail);
    hasher.finish()
}

fn conflict_kind_name(kind: RegionalGeographyConflictKind) -> &'static str {
    match kind {
        RegionalGeographyConflictKind::NativeOverlap => "native_overlap",
        RegionalGeographyConflictKind::AntimeridianUnsupported => "antimeridian_unsupported",
        RegionalGeographyConflictKind::SeamSampleAlignment => "seam_sample_alignment",
        RegionalGeographyConflictKind::SeamValidity => "seam_validity",
        RegionalGeographyConflictKind::SeamElevation => "seam_elevation",
        RegionalGeographyConflictKind::SeamMaterial => "seam_material",
        RegionalGeographyConflictKind::SeamTerrainUnsupported => "seam_terrain_unsupported",
    }
}

fn composition_digest(
    composition: &RegionalGeographyComposition,
) -> Result<SemanticDigest, RegionalGeographyCompositionError> {
    let mut normalized = composition.clone();
    normalized.composition_id = placeholder("regional-geography-composition");
    let mut hasher = SemanticHasher::new(REGIONAL_GEOGRAPHY_COMPOSITION_SCHEMA);
    hasher.add_bytes(&serde_json::to_vec(&normalized)?);
    Ok(hasher.finish())
}

fn composition_compiler() -> ContractIdentity {
    ContractIdentity::new(
        "rey.regional-geography.composer",
        2,
        "exact native-boundary relationship, terrain-seam assessment, and qualified connected-component identity; no implicit merge or synthesis",
    )
}

fn placeholder(domain: &str) -> SemanticDigest {
    SemanticHasher::new(domain).finish()
}

fn overlap(left_start: i64, left_end: i64, right_start: i64, right_end: i64) -> i64 {
    left_end
        .min(right_end)
        .saturating_sub(left_start.max(right_start))
}

fn axis_gap(left_start: i64, left_end: i64, right_start: i64, right_end: i64) -> u64 {
    if left_end < right_start {
        left_end.abs_diff(right_start)
    } else if right_end < left_start {
        right_end.abs_diff(left_start)
    } else {
        0
    }
}

fn canonical_unique<'a>(
    values: impl Iterator<Item = &'a SemanticDigest>,
) -> Result<(), RegionalGeographyCompositionError> {
    let mut prior = None;
    for value in values {
        if prior.is_some_and(|prior| prior >= value) {
            return Err(RegionalGeographyCompositionError::Shape);
        }
        prior = Some(value);
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum RegionalGeographyCompositionError {
    #[error("regional geography composition exceeded a deterministic bound")]
    Limit,
    #[error("regional geography composition contains a duplicate package member")]
    DuplicateMember,
    #[error("regional geography package does not bind its exact current atlas member")]
    AtlasBinding,
    #[error("regional geography composition shape is invalid")]
    Shape,
    #[error("regional geography composition identity does not match its content")]
    Digest,
    #[error(transparent)]
    Atlas(#[from] SemanticAtlasError),
    #[error(transparent)]
    Scene(#[from] crate::RegionalSceneError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(west: i64, south: i64, east: i64, north: i64) -> RegionalBounds {
        RegionalBounds {
            west_microdegrees: west,
            south_microdegrees: south,
            east_microdegrees: east,
            north_microdegrees: north,
            crosses_antimeridian: false,
        }
    }

    #[test]
    fn native_relationships_distinguish_edges_corners_overlaps_and_gaps() {
        let origin = bounds(0, 0, 10, 10);
        assert_eq!(
            shared_boundary(&origin, &bounds(10, 0, 20, 10)),
            Some(RegionalGeographySharedBoundary {
                axis: RegionalGeographyBoundaryAxis::Longitude,
                coordinate_microdegrees: 10,
                start_microdegrees: 0,
                end_microdegrees: 10,
            })
        );
        assert_eq!(shared_boundary(&origin, &bounds(10, 10, 20, 20)), None);
        assert_eq!(overlap(0, 10, 5, 15), 5);
        assert_eq!(axis_gap(0, 10, 12, 20), 2);
    }

    #[test]
    fn stitch_readiness_requires_a_conflict_free_connected_seam_graph() {
        let member = |id: &str| RegionalGeographyMember {
            member_id: placeholder(id),
            workload_id: id.to_owned(),
            region_id: id.to_owned(),
            scene_id: placeholder(&format!("{id}.scene")),
            atlas_region_id: placeholder(&format!("{id}.atlas")),
            admitted_atlas_revision: placeholder(&format!("{id}.admitted")),
            admission_id: placeholder(&format!("{id}.admission")),
            package_id: placeholder(&format!("{id}.package")),
            package_revision: placeholder(&format!("{id}.revision")),
            projection_packet_id: placeholder(&format!("{id}.packet")),
            terrain_program_id: None,
            terrain_dataset_id: Some(placeholder(&format!("{id}.dataset"))),
            native_bounds: bounds(0, 0, 10, 10),
            terrain_valid_vertices: 0,
            terrain_no_data_vertices: 0,
        };
        let mut members = vec![member("a"), member("b")];
        members.sort_by(|left, right| left.member_id.cmp(&right.member_id));
        let ids = [members[0].member_id.clone(), members[1].member_id.clone()];
        let seam = RegionalGeographySeam {
            seam_id: seam_digest(&ids),
            member_ids: ids.clone(),
            relationship: RegionalGeographyRelationship::EdgeAdjacent,
            shared_boundary: Some(RegionalGeographySharedBoundary {
                axis: RegionalGeographyBoundaryAxis::Longitude,
                coordinate_microdegrees: 10,
                start_microdegrees: 0,
                end_microdegrees: 10,
            }),
            longitude_gap_microdegrees: 0,
            latitude_gap_microdegrees: 0,
            terrain_status: RegionalGeographyTerrainStatus::Qualified,
            compared_vertices: 2,
            valid_vertices: 2,
            no_data_vertices: 0,
            validity_conflicts: 0,
            elevation_conflicts: 0,
            material_conflicts: 0,
            max_elevation_delta_micrometers: Some(0),
        };
        assert_eq!(
            stitch_status(&members, std::slice::from_ref(&seam), &[]),
            RegionalGeographyStitchStatus::Ready
        );
        let components = terrain_components(
            &placeholder("atlas"),
            &members,
            std::slice::from_ref(&seam),
            &[],
        );
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].member_ids, ids);
        assert_eq!(components[0].qualified_seam_ids, [seam.seam_id.clone()]);
        let internal_conflict = RegionalGeographyConflict {
            conflict_id: placeholder("internal-conflict"),
            seam_id: placeholder("other-pair"),
            member_ids: seam.member_ids.clone(),
            kind: RegionalGeographyConflictKind::NativeOverlap,
            count: 1,
            detail: "a retained conflict within the qualified graph".to_owned(),
        };
        let components = terrain_components(
            &placeholder("atlas"),
            &members,
            std::slice::from_ref(&seam),
            std::slice::from_ref(&internal_conflict),
        );
        assert_eq!(components.len(), 2);
        assert!(components.iter().all(|component| {
            component.member_ids.len() == 1 && component.qualified_seam_ids.is_empty()
        }));
        let conflict = conflict(
            &seam,
            RegionalGeographyConflictKind::SeamElevation,
            1,
            "test conflict",
        );
        assert_eq!(
            stitch_status(
                &members,
                std::slice::from_ref(&seam),
                std::slice::from_ref(&conflict),
            ),
            RegionalGeographyStitchStatus::Blocked
        );
        let components = terrain_components(
            &placeholder("atlas"),
            &members,
            std::slice::from_ref(&seam),
            std::slice::from_ref(&conflict),
        );
        assert_eq!(components.len(), 2);
        assert!(components.iter().all(|component| {
            component.member_ids.len() == 1 && component.qualified_seam_ids.is_empty()
        }));
    }
}
