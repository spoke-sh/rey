use std::collections::{BTreeMap, BTreeSet};

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ExplorerGrammar, ExplorerGrammarError};

pub const ADMITTED_REGIONAL_SCENE_SCHEMA: &str = "rey.admitted-regional-scene.v1";
pub const REGIONAL_PROJECTION_PACKET_SCHEMA: &str = "rey.regional-projection-packet.v1";
pub const REGIONAL_TERRAIN_PROGRAM_SCHEMA: &str = "rey.regional-terrain-program.v1";
pub const REGIONAL_TERRAIN_GRID_PROGRAM_SCHEMA: &str = "rey.regional-terrain-program.v2";
pub const REGIONAL_TERRAIN_GRID_SCHEMA: &str = "rey.regional-terrain-grid.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionalCoordinateSpace {
    NativeCrs84,
    SyntheticSemantic,
    SemanticMercator,
    CountyLocal,
    Camera,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionalCoordinateStatus {
    Bound,
    Derived,
    ViewOnly,
    Unsupported,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalCoordinateBinding {
    pub space: RegionalCoordinateSpace,
    pub status: RegionalCoordinateStatus,
    pub dimensions: Vec<String>,
    pub units: Vec<String>,
    pub authority: String,
    pub source_revision: SemanticDigest,
    pub disclosure: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalBounds {
    pub west_microdegrees: i64,
    pub south_microdegrees: i64,
    pub east_microdegrees: i64,
    pub north_microdegrees: i64,
    pub crosses_antimeridian: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalTransform {
    pub transform: ContractIdentity,
    pub source_space: RegionalCoordinateSpace,
    pub target_space: RegionalCoordinateSpace,
    pub source_origin: Vec<i64>,
    pub target_origin: Vec<i64>,
    pub parameters: Vec<String>,
    pub inverse_policy: String,
    pub distortion: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionalLayerKind {
    NativeFeature,
    Terrain,
    TerrainControl,
    Hydrology,
    Boundary,
    Poi,
    Highway,
    Road,
    District,
    Lot,
    Structure,
    Utility,
    Label,
    Beacon,
    Construction,
    Connector,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalTerrainSample {
    pub sample_id: SemanticDigest,
    pub source_object_id: String,
    pub source_artifact_id: SemanticDigest,
    pub source_object_revision: SemanticDigest,
    pub position: [i64; 3],
    pub material: String,
    pub authority: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalTerrainGridCell {
    pub cell_id: SemanticDigest,
    pub source_object_id: String,
    pub source_artifact_id: SemanticDigest,
    pub source_object_revision: SemanticDigest,
    pub grid_position: [u64; 2],
    pub native_position: [i64; 2],
    pub elevation_micrometers: Option<i64>,
    pub material: Option<String>,
    pub validity: RegionalValidityClass,
    pub authority: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalTerrainGrid {
    pub schema: String,
    pub dataset_id: SemanticDigest,
    pub source_dataset_id: String,
    pub columns: u64,
    pub rows: u64,
    pub native_bounds: RegionalBounds,
    pub cells: Vec<RegionalTerrainGridCell>,
    pub validity_semantics: String,
    pub interpolation: String,
    pub authority: String,
}

impl RegionalTerrainGrid {
    pub fn finalize(mut self) -> Result<Self, RegionalSceneError> {
        for cell in &mut self.cells {
            cell.cell_id = regional_terrain_grid_cell_digest(cell)?;
        }
        self.dataset_id = regional_terrain_grid_digest(&self)?;
        self.verify()?;
        Ok(self)
    }

    pub fn verify(&self) -> Result<(), RegionalSceneError> {
        validate_identifier(&self.source_dataset_id)?;
        validate_bounds(&self.native_bounds)?;
        let expected_cells = self
            .columns
            .checked_mul(self.rows)
            .ok_or(RegionalSceneError::TerrainAuthority)?;
        if self.schema != REGIONAL_TERRAIN_GRID_SCHEMA
            || self.columns < 2
            || self.rows < 2
            || self.native_bounds.crosses_antimeridian
            || self.native_bounds.west_microdegrees >= self.native_bounds.east_microdegrees
            || self.native_bounds.south_microdegrees >= self.native_bounds.north_microdegrees
            || self.cells.len() as u64 != expected_cells
            || self.validity_semantics
                != "row-major source vertices are explicitly valid or no_data; no_data cuts triangle support"
            || self.interpolation
                != "piecewise linear only within triangles whose three admitted source vertices are valid"
            || self.authority
                != "qualified rectilinear height/material grid; validity ends at supported source triangles"
            || self.dataset_id != regional_terrain_grid_digest(self)?
        {
            return Err(RegionalSceneError::TerrainAuthority);
        }
        let longitude_span =
            i128::from(self.native_bounds.east_microdegrees - self.native_bounds.west_microdegrees);
        let latitude_span = i128::from(
            self.native_bounds.north_microdegrees - self.native_bounds.south_microdegrees,
        );
        let column_divisor = i128::from(self.columns - 1);
        let row_divisor = i128::from(self.rows - 1);
        if longitude_span % column_divisor != 0 || latitude_span % row_divisor != 0 {
            return Err(RegionalSceneError::TerrainAuthority);
        }
        let longitude_step = longitude_span / column_divisor;
        let latitude_step = latitude_span / row_divisor;
        unique(
            self.cells.iter().map(|cell| cell.cell_id.as_str()),
            "terrain grid cell",
        )?;
        unique(
            self.cells.iter().map(|cell| cell.source_object_id.as_str()),
            "terrain grid source object",
        )?;
        let source_artifact_id = self
            .cells
            .first()
            .map(|cell| &cell.source_artifact_id)
            .ok_or(RegionalSceneError::TerrainAuthority)?;
        let mut has_valid_triangle = false;
        for (index, cell) in self.cells.iter().enumerate() {
            let expected_column = index as u64 % self.columns;
            let expected_row = index as u64 / self.columns;
            let expected_longitude = i128::from(self.native_bounds.west_microdegrees)
                + i128::from(expected_column) * longitude_step;
            let expected_latitude = i128::from(self.native_bounds.north_microdegrees)
                - i128::from(expected_row) * latitude_step;
            validate_identifier_path(&cell.source_object_id)?;
            if cell.grid_position != [expected_column, expected_row]
                || i128::from(cell.native_position[0]) != expected_longitude
                || i128::from(cell.native_position[1]) != expected_latitude
                || &cell.source_artifact_id != source_artifact_id
                || cell.cell_id != regional_terrain_grid_cell_digest(cell)?
            {
                return Err(RegionalSceneError::TerrainAuthority);
            }
            match cell.validity {
                RegionalValidityClass::Valid
                    if cell.elevation_micrometers.is_some_and(|height| {
                        (-12_000_000_000..=100_000_000_000).contains(&height)
                    }) && cell.material.as_ref().is_some_and(|material| {
                        !material.is_empty()
                            && material.chars().count() <= 64
                            && material.chars().all(|character| {
                                character.is_ascii_alphanumeric()
                                    || matches!(character, '-' | '_' | '.')
                            })
                    }) && cell.authority
                        == "exact admitted Point altitude and material at one valid grid vertex" => {
                }
                RegionalValidityClass::NoData
                    if cell.elevation_micrometers.is_none()
                        && cell.material.is_none()
                        && cell.authority
                            == "explicit source no-data vertex; geometry locates the hole but supplies no height or material" =>
                    {}
                _ => return Err(RegionalSceneError::TerrainAuthority),
            }
        }
        for row in 0..self.rows - 1 {
            for column in 0..self.columns - 1 {
                let top_left = (row * self.columns + column) as usize;
                let top_right = top_left + 1;
                let bottom_left = top_left + self.columns as usize;
                let bottom_right = bottom_left + 1;
                let valid =
                    |index: usize| self.cells[index].validity == RegionalValidityClass::Valid;
                if (valid(top_left) && valid(bottom_left) && valid(bottom_right))
                    || (valid(top_left) && valid(bottom_right) && valid(top_right))
                    || (valid(top_left) && valid(bottom_left) && valid(top_right))
                    || (valid(top_right) && valid(bottom_left) && valid(bottom_right))
                {
                    has_valid_triangle = true;
                }
            }
        }
        if !has_valid_triangle {
            return Err(RegionalSceneError::TerrainAuthority);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalTerrainProgram {
    pub schema: String,
    pub program_id: SemanticDigest,
    pub evaluator: ContractIdentity,
    pub samples: Vec<RegionalTerrainSample>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<RegionalTerrainGrid>,
    pub height_unit: String,
    pub interpolation: String,
    pub material_semantics: String,
    pub authority: String,
}

impl RegionalTerrainProgram {
    pub fn finalize(mut self) -> Result<Self, RegionalSceneError> {
        self.program_id = regional_terrain_program_digest(&self)?;
        self.verify()?;
        Ok(self)
    }

    pub fn verify(&self) -> Result<(), RegionalSceneError> {
        let point_program = self.grid.is_none()
            && !self.samples.is_empty()
            && self.evaluator
                == ContractIdentity::new(
                    "rey.regional-terrain.exact-samples",
                    1,
                    "retain exact admitted Point altitude/material samples without interpolation or coverage expansion",
                )
            && self.interpolation == "none; exact admitted samples only"
            && self.authority
                == "qualified exact height/material samples; no interpolated terrain coverage";
        let grid_program = self.samples.is_empty()
            && self.grid.is_some()
            && self.evaluator
                == ContractIdentity::new(
                    "rey.regional-terrain.rectilinear-grid",
                    1,
                    "retain one exact row-major terrain grid and authorize piecewise-linear interpolation only inside fully supported source triangles",
                )
            && self.interpolation
                == "piecewise linear only within triangles whose three admitted source vertices are valid"
            && self.authority
                == "qualified rectilinear height/material grid; validity ends at supported source triangles";
        if (!point_program && !grid_program)
            || (point_program && self.schema != REGIONAL_TERRAIN_PROGRAM_SCHEMA)
            || (grid_program && self.schema != REGIONAL_TERRAIN_GRID_PROGRAM_SCHEMA)
            || !self
                .samples
                .windows(2)
                .all(|pair| pair[0].sample_id < pair[1].sample_id)
            || self.height_unit != "micrometer"
            || self.material_semantics
                != "source-declared bounded material identifier; no inferred physical properties"
            || self.program_id != regional_terrain_program_digest(self)?
        {
            return Err(RegionalSceneError::TerrainAuthority);
        }
        if let Some(grid) = &self.grid {
            grid.verify()?;
        }
        unique(
            self.samples.iter().map(|sample| sample.sample_id.as_str()),
            "terrain sample",
        )?;
        for sample in &self.samples {
            validate_identifier_path(&sample.source_object_id)?;
            if !(-180_000_000..=180_000_000).contains(&sample.position[0])
                || !(-90_000_000..=90_000_000).contains(&sample.position[1])
                || !(-12_000_000_000..=100_000_000_000).contains(&sample.position[2])
                || sample.material.is_empty()
                || sample.material.chars().count() > 64
                || !sample.material.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
                })
                || sample.authority
                    != "exact admitted Point altitude and material property; valid only at this source coordinate"
                || sample.sample_id != regional_terrain_sample_digest(sample)?
            {
                return Err(RegionalSceneError::TerrainAuthority);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalNativeObject {
    pub object_id: String,
    pub source_id: String,
    pub source_path: String,
    pub source_artifact_id: SemanticDigest,
    pub object_revision: SemanticDigest,
    pub geometry_kind: String,
    pub native_bounds: RegionalBounds,
    pub layer: RegionalLayerKind,
    pub authority: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalFootprint {
    pub footprint_id: SemanticDigest,
    pub source_object_id: String,
    pub source_artifact_id: SemanticDigest,
    pub source_object_revision: SemanticDigest,
    pub geometry_kind: String,
    pub native_bounds: RegionalBounds,
    pub rings: Vec<Vec<[i64; 2]>>,
    pub coordinate_count: u64,
    pub authority: String,
}

impl RegionalFootprint {
    pub fn finalize(mut self) -> Result<Self, RegionalSceneError> {
        self.footprint_id = regional_footprint_digest(&self)?;
        self.verify()?;
        Ok(self)
    }

    pub fn verify(&self) -> Result<(), RegionalSceneError> {
        validate_identifier_path(&self.source_object_id)?;
        validate_bounds(&self.native_bounds)?;
        let positions = self.rings.iter().flatten().copied().collect::<Vec<_>>();
        if self.geometry_kind != "Polygon"
            || self.rings.is_empty()
            || self
                .rings
                .iter()
                .any(|ring| ring.len() < 4 || ring.first() != ring.last())
            || positions.len() as u64 != self.coordinate_count
            || regional_positions_bounds(&positions).as_ref() != Some(&self.native_bounds)
            || self.authority
                != "exact admitted native boundary polygon; footprint validity ends at its rings"
            || self.footprint_id != regional_footprint_digest(self)?
        {
            return Err(RegionalSceneError::Footprint);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalLayer {
    pub layer_id: String,
    pub kind: RegionalLayerKind,
    pub object_ids: Vec<String>,
    pub authority: String,
    pub semantics: String,
    pub source_revision: SemanticDigest,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionalValidityClass {
    Valid,
    NoData,
    Unknown,
    Unsupported,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalValidity {
    pub validity_id: SemanticDigest,
    pub class: RegionalValidityClass,
    pub scope: String,
    pub source_revision: SemanticDigest,
    pub rule: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalSceneLimits {
    pub max_sources: u64,
    pub max_native_objects: u64,
    pub max_native_coordinates: u64,
    pub max_layers: u64,
    pub max_validity_records: u64,
    pub max_transforms: u64,
    pub max_omissions: u64,
    pub max_native_bytes: u64,
}

impl Default for RegionalSceneLimits {
    fn default() -> Self {
        Self {
            max_sources: 64,
            max_native_objects: 10_000,
            max_native_coordinates: 1_000_000,
            max_layers: 16,
            max_validity_records: 256,
            max_transforms: 4,
            max_omissions: 1_024,
            max_native_bytes: 64 * 1_024 * 1_024,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalSceneOmission {
    pub kind: String,
    pub subject: String,
    pub omitted_count: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalSceneLineage {
    pub kind: String,
    pub identity: String,
    pub revision: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneAdmissionBinding {
    pub admission_id: SemanticDigest,
    pub operation: ContractIdentity,
    pub implementation: ContractIdentity,
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario_suite: ContractIdentity,
    pub evaluator: ContractIdentity,
    pub capability_snapshot_id: SemanticDigest,
    pub editor_commit_id: SemanticDigest,
    pub editor_sequence: u64,
    pub package_id: SemanticDigest,
    pub parent_package_id: Option<SemanticDigest>,
    pub package_snapshot_revision: SemanticDigest,
    pub admission_request_id: SemanticDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalArtifactBindings {
    pub source_topography_patch_id: Option<SemanticDigest>,
    pub admitted_atlas_revision: Option<SemanticDigest>,
    pub projection_packet_id: SemanticDigest,
    pub terrain_program_id: Option<SemanticDigest>,
    pub terrain_authority: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionalProjectionPacket {
    pub schema: String,
    pub packet_id: SemanticDigest,
    pub source_package_id: SemanticDigest,
    pub source_snapshot_revision: SemanticDigest,
    pub grammar_id: SemanticDigest,
    pub coordinate_bindings: Vec<RegionalCoordinateBinding>,
    pub transforms: Vec<RegionalTransform>,
    pub objects: Vec<RegionalNativeObject>,
    pub footprint: Option<RegionalFootprint>,
    pub layers: Vec<RegionalLayer>,
    pub validity: Vec<RegionalValidity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain: Option<RegionalTerrainProgram>,
    pub terrain_program_id: Option<SemanticDigest>,
    pub limits: RegionalSceneLimits,
    pub complete: bool,
    pub omissions: Vec<RegionalSceneOmission>,
    pub lineage: Vec<RegionalSceneLineage>,
}

impl RegionalProjectionPacket {
    pub fn finalize(mut self) -> Result<Self, RegionalSceneError> {
        self.packet_id = projection_packet_digest(&self)?;
        self.verify()?;
        Ok(self)
    }

    pub fn verify(&self) -> Result<(), RegionalSceneError> {
        if self.schema != REGIONAL_PROJECTION_PACKET_SCHEMA {
            return Err(RegionalSceneError::ProjectionSchema);
        }
        validate_projection_shape(self)?;
        if self.packet_id != projection_packet_digest(self)? {
            return Err(RegionalSceneError::ProjectionIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdmittedRegionalScene {
    pub schema: String,
    pub scene_id: SemanticDigest,
    pub region_id: String,
    pub admission: SceneAdmissionBinding,
    pub native_bounds: RegionalBounds,
    pub projection: RegionalProjectionPacket,
    pub artifacts: RegionalArtifactBindings,
    pub complete: bool,
    pub omissions: Vec<RegionalSceneOmission>,
    pub lineage: Vec<RegionalSceneLineage>,
}

impl AdmittedRegionalScene {
    pub fn finalize(mut self) -> Result<Self, RegionalSceneError> {
        self.projection.verify()?;
        self.artifacts.projection_packet_id = self.projection.packet_id.clone();
        self.scene_id = regional_scene_digest(&self)?;
        self.verify()?;
        Ok(self)
    }

    pub fn verify(&self) -> Result<(), RegionalSceneError> {
        if self.schema != ADMITTED_REGIONAL_SCENE_SCHEMA {
            return Err(RegionalSceneError::Schema);
        }
        validate_identifier(&self.region_id)?;
        validate_bounds(&self.native_bounds)?;
        self.projection.verify()?;
        validate_county_frame(self)?;
        if self.admission.editor_sequence == 0
            || self.admission.package_id != self.projection.source_package_id
            || self.admission.package_snapshot_revision != self.projection.source_snapshot_revision
            || self.artifacts.projection_packet_id != self.projection.packet_id
            || self.artifacts.terrain_program_id != self.projection.terrain_program_id
            || self
                .artifacts
                .admitted_atlas_revision
                .as_ref()
                .is_some_and(|revision| revision.as_str().is_empty())
            || self.omissions.len() as u64 > self.projection.limits.max_omissions
        {
            return Err(RegionalSceneError::Binding);
        }
        if self.artifacts.terrain_program_id.is_none()
            && self.artifacts.terrain_authority
                != "none; candidate-only terrain controls were not copied into observed terrain truth"
        {
            return Err(RegionalSceneError::TerrainAuthority);
        }
        if self.artifacts.terrain_program_id.is_some()
            && self.artifacts.terrain_authority
                != self
                    .projection
                    .terrain
                    .as_ref()
                    .map(|terrain| terrain.authority.as_str())
                    .unwrap_or_default()
        {
            return Err(RegionalSceneError::TerrainAuthority);
        }
        if self.complete != self.projection.complete || self.omissions != self.projection.omissions
        {
            return Err(RegionalSceneError::Completeness);
        }
        if self.scene_id != regional_scene_digest(self)? {
            return Err(RegionalSceneError::Identity);
        }
        Ok(())
    }

    pub fn with_admitted_atlas_revision(
        mut self,
        atlas_revision: &SemanticDigest,
    ) -> Result<Self, RegionalSceneError> {
        self.verify()?;
        if atlas_revision.as_str().is_empty()
            || self
                .artifacts
                .admitted_atlas_revision
                .as_ref()
                .is_some_and(|current| current != atlas_revision)
        {
            return Err(RegionalSceneError::AtlasBinding);
        }
        self.artifacts.admitted_atlas_revision = Some(atlas_revision.clone());
        self.verify()?;
        Ok(self)
    }
}

fn validate_county_frame(scene: &AdmittedRegionalScene) -> Result<(), RegionalSceneError> {
    let transforms = scene
        .projection
        .transforms
        .iter()
        .filter(|transform| {
            transform.source_space == RegionalCoordinateSpace::NativeCrs84
                && transform.target_space == RegionalCoordinateSpace::CountyLocal
        })
        .collect::<Vec<_>>();
    let binding = scene
        .projection
        .coordinate_bindings
        .iter()
        .find(|binding| binding.space == RegionalCoordinateSpace::CountyLocal);
    if transforms.len() != 1
        || transforms[0].source_origin != regional_bounds_center(&scene.native_bounds)
        || transforms[0].target_origin != [0, 0, 0]
        || transforms[0].parameters != ["east_north_up_microunits"]
        || !transforms[0]
            .inverse_policy
            .contains("bounded analytic inverse")
        || transforms[0].distortion.is_empty()
        || binding.is_none_or(|binding| {
            binding.status != RegionalCoordinateStatus::Bound
                || binding.dimensions != ["east", "north", "up"]
                || binding.units != ["local_microunit", "local_microunit", "local_microunit"]
        })
    {
        return Err(RegionalSceneError::CountyFrame);
    }
    Ok(())
}

fn regional_bounds_center(bounds: &RegionalBounds) -> Vec<i64> {
    let east = if bounds.crosses_antimeridian {
        bounds.east_microdegrees + 360_000_000
    } else {
        bounds.east_microdegrees
    };
    let mut longitude = (bounds.west_microdegrees + east) / 2;
    if longitude > 180_000_000 {
        longitude -= 360_000_000;
    }
    vec![
        longitude,
        (bounds.south_microdegrees + bounds.north_microdegrees) / 2,
    ]
}

fn validate_projection_shape(packet: &RegionalProjectionPacket) -> Result<(), RegionalSceneError> {
    let grammar = ExplorerGrammar::v1()?;
    if packet.grammar_id != grammar.grammar_id
        || packet.coordinate_bindings.len() != 5
        || packet
            .coordinate_bindings
            .iter()
            .map(|binding| binding.space)
            .collect::<Vec<_>>()
            != [
                RegionalCoordinateSpace::NativeCrs84,
                RegionalCoordinateSpace::SyntheticSemantic,
                RegionalCoordinateSpace::SemanticMercator,
                RegionalCoordinateSpace::CountyLocal,
                RegionalCoordinateSpace::Camera,
            ]
        || packet.transforms.len() as u64 > packet.limits.max_transforms
        || packet.objects.len() as u64 > packet.limits.max_native_objects
        || packet.footprint.as_ref().is_some_and(|footprint| {
            footprint.coordinate_count > packet.limits.max_native_coordinates
        })
        || packet.layers.len() as u64 > packet.limits.max_layers
        || packet.validity.len() as u64 > packet.limits.max_validity_records
        || packet.omissions.len() as u64 > packet.limits.max_omissions
    {
        return Err(RegionalSceneError::ProjectionShape);
    }
    let source_count = packet
        .objects
        .iter()
        .map(|object| object.source_id.as_str())
        .collect::<BTreeSet<_>>()
        .len() as u64;
    if source_count > packet.limits.max_sources {
        return Err(RegionalSceneError::ProjectionShape);
    }
    unique(
        packet
            .objects
            .iter()
            .map(|object| object.object_id.as_str()),
        "object",
    )?;
    unique(
        packet.layers.iter().map(|layer| layer.layer_id.as_str()),
        "layer",
    )?;
    unique(
        packet
            .validity
            .iter()
            .map(|record| record.validity_id.as_str()),
        "validity",
    )?;
    let object_ids = packet
        .objects
        .iter()
        .map(|object| object.object_id.as_str())
        .collect::<BTreeSet<_>>();
    let objects_by_id = packet
        .objects
        .iter()
        .map(|object| (object.object_id.as_str(), object))
        .collect::<BTreeMap<_, _>>();
    for object in &packet.objects {
        validate_identifier_path(&object.object_id)?;
        validate_bounds(&object.native_bounds)?;
        if object.authority
            != "exact admitted native geometry; appearance grants no relationship, activity, or action authority"
        {
            return Err(RegionalSceneError::ObjectAuthority);
        }
    }
    if let Some(footprint) = &packet.footprint {
        footprint.verify()?;
        let source = objects_by_id.get(footprint.source_object_id.as_str());
        if source.is_none_or(|object| {
            object.layer != RegionalLayerKind::Boundary
                || object.geometry_kind != "Polygon"
                || object.source_artifact_id != footprint.source_artifact_id
                || object.object_revision != footprint.source_object_revision
                || object.native_bounds != footprint.native_bounds
        }) {
            return Err(RegionalSceneError::Footprint);
        }
    }
    if packet.terrain.as_ref().map(|terrain| &terrain.program_id)
        != packet.terrain_program_id.as_ref()
    {
        return Err(RegionalSceneError::TerrainAuthority);
    }
    if let Some(terrain) = &packet.terrain {
        terrain.verify()?;
        let grid_cells = terrain
            .grid
            .as_ref()
            .map_or(0, |grid| grid.cells.len() as u64);
        if terrain.samples.len() as u64 + grid_cells > packet.limits.max_native_objects
            || terrain.samples.iter().any(|sample| {
                objects_by_id
                    .get(sample.source_object_id.as_str())
                    .is_none_or(|object| {
                        !(object.object_id == sample.source_object_id
                            && object.layer == RegionalLayerKind::Terrain
                            && object.geometry_kind == "Point"
                            && object.source_artifact_id == sample.source_artifact_id
                            && object.object_revision == sample.source_object_revision
                            && object.native_bounds.west_microdegrees == sample.position[0]
                            && object.native_bounds.east_microdegrees == sample.position[0]
                            && object.native_bounds.south_microdegrees == sample.position[1]
                            && object.native_bounds.north_microdegrees == sample.position[1])
                    })
            })
            || terrain.grid.as_ref().is_some_and(|grid| {
                grid.cells.iter().any(|cell| {
                    objects_by_id
                        .get(cell.source_object_id.as_str())
                        .is_none_or(|object| {
                            !(object.object_id == cell.source_object_id
                                && object.layer == RegionalLayerKind::Terrain
                                && object.geometry_kind == "Point"
                                && object.source_artifact_id == cell.source_artifact_id
                                && object.object_revision == cell.source_object_revision
                                && object.native_bounds.west_microdegrees
                                    == cell.native_position[0]
                                && object.native_bounds.east_microdegrees
                                    == cell.native_position[0]
                                && object.native_bounds.south_microdegrees
                                    == cell.native_position[1]
                                && object.native_bounds.north_microdegrees
                                    == cell.native_position[1])
                        })
                })
            })
        {
            return Err(RegionalSceneError::TerrainAuthority);
        }
    }
    for layer in &packet.layers {
        if !layer.object_ids.windows(2).all(|pair| pair[0] < pair[1])
            || layer
                .object_ids
                .iter()
                .any(|id| !object_ids.contains(id.as_str()))
        {
            return Err(RegionalSceneError::LayerReference);
        }
        if layer.kind == RegionalLayerKind::TerrainControl
            && layer.authority
                != "candidate control geometry only; no observed height, material, or terrain validity"
        {
            return Err(RegionalSceneError::TerrainAuthority);
        }
        if layer.kind == RegionalLayerKind::Terrain
            && layer.authority
                != packet
                    .terrain
                    .as_ref()
                    .map(|terrain| terrain.authority.as_str())
                    .unwrap_or(
                        "qualified exact height/material samples; no interpolated terrain coverage",
                    )
        {
            return Err(RegionalSceneError::TerrainAuthority);
        }
    }
    if packet.terrain_program_id.is_none()
        && !packet.validity.iter().any(|record| {
            record.class == RegionalValidityClass::Unsupported && record.scope == "terrain_height"
        })
    {
        return Err(RegionalSceneError::TerrainAuthority);
    }
    if packet.terrain_program_id.is_some()
        && packet.validity.iter().any(|record| {
            record.class == RegionalValidityClass::Unsupported && record.scope == "terrain_height"
        })
    {
        return Err(RegionalSceneError::TerrainAuthority);
    }
    Ok(())
}

fn validate_bounds(bounds: &RegionalBounds) -> Result<(), RegionalSceneError> {
    if !(-180_000_000..=180_000_000).contains(&bounds.west_microdegrees)
        || !(-180_000_000..=180_000_000).contains(&bounds.east_microdegrees)
        || !(-90_000_000..=90_000_000).contains(&bounds.south_microdegrees)
        || !(-90_000_000..=90_000_000).contains(&bounds.north_microdegrees)
        || bounds.south_microdegrees > bounds.north_microdegrees
        || (!bounds.crosses_antimeridian && bounds.west_microdegrees > bounds.east_microdegrees)
        || (bounds.crosses_antimeridian && bounds.west_microdegrees <= bounds.east_microdegrees)
    {
        return Err(RegionalSceneError::CoordinateBounds);
    }
    Ok(())
}

fn validate_identifier(value: &str) -> Result<(), RegionalSceneError> {
    if value.is_empty()
        || value.chars().count() > 96
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err(RegionalSceneError::Identifier(value.to_owned()));
    }
    Ok(())
}

fn validate_identifier_path(value: &str) -> Result<(), RegionalSceneError> {
    let mut parts = value.split('/');
    let Some(first) = parts.next() else {
        return Err(RegionalSceneError::Identifier(value.to_owned()));
    };
    validate_identifier(first)?;
    for part in parts {
        validate_identifier(part)?;
    }
    Ok(())
}

fn unique<'a>(
    values: impl Iterator<Item = &'a str>,
    kind: &'static str,
) -> Result<(), RegionalSceneError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(RegionalSceneError::Duplicate(kind));
        }
    }
    Ok(())
}

fn placeholder_digest() -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.regional-scene.placeholder.v1");
    hasher.add_str("excluded from identity");
    hasher.finish()
}

fn regional_footprint_digest(
    footprint: &RegionalFootprint,
) -> Result<SemanticDigest, RegionalSceneError> {
    let mut normalized = footprint.clone();
    normalized.footprint_id = placeholder_digest();
    let mut hasher = SemanticHasher::new("rey.regional-footprint.v1");
    hasher.add_bytes(&serde_json::to_vec(&normalized)?);
    Ok(hasher.finish())
}

pub fn finalize_regional_terrain_sample(
    mut sample: RegionalTerrainSample,
) -> Result<RegionalTerrainSample, RegionalSceneError> {
    sample.sample_id = regional_terrain_sample_digest(&sample)?;
    Ok(sample)
}

fn regional_terrain_sample_digest(
    sample: &RegionalTerrainSample,
) -> Result<SemanticDigest, RegionalSceneError> {
    let mut normalized = sample.clone();
    normalized.sample_id = placeholder_digest();
    let mut hasher = SemanticHasher::new("rey.regional-terrain-sample.v1");
    hasher.add_bytes(&serde_json::to_vec(&normalized)?);
    Ok(hasher.finish())
}

fn regional_terrain_program_digest(
    program: &RegionalTerrainProgram,
) -> Result<SemanticDigest, RegionalSceneError> {
    let mut normalized = program.clone();
    normalized.program_id = placeholder_digest();
    let identity_schema = if program.grid.is_some() {
        REGIONAL_TERRAIN_GRID_PROGRAM_SCHEMA
    } else {
        REGIONAL_TERRAIN_PROGRAM_SCHEMA
    };
    let mut hasher = SemanticHasher::new(identity_schema);
    hasher.add_bytes(&serde_json::to_vec(&normalized)?);
    Ok(hasher.finish())
}

fn regional_terrain_grid_cell_digest(
    cell: &RegionalTerrainGridCell,
) -> Result<SemanticDigest, RegionalSceneError> {
    let mut normalized = cell.clone();
    normalized.cell_id = placeholder_digest();
    let mut hasher = SemanticHasher::new("rey.regional-terrain-grid-cell.v1");
    hasher.add_bytes(&serde_json::to_vec(&normalized)?);
    Ok(hasher.finish())
}

fn regional_terrain_grid_digest(
    grid: &RegionalTerrainGrid,
) -> Result<SemanticDigest, RegionalSceneError> {
    let mut normalized = grid.clone();
    normalized.dataset_id = placeholder_digest();
    let mut hasher = SemanticHasher::new(REGIONAL_TERRAIN_GRID_SCHEMA);
    hasher.add_bytes(&serde_json::to_vec(&normalized)?);
    Ok(hasher.finish())
}

fn regional_positions_bounds(positions: &[[i64; 2]]) -> Option<RegionalBounds> {
    if positions.is_empty() {
        return None;
    }
    let south = positions.iter().map(|position| position[1]).min()?;
    let north = positions.iter().map(|position| position[1]).max()?;
    let mut longitudes = positions
        .iter()
        .map(|position| position[0])
        .collect::<Vec<_>>();
    longitudes.sort_unstable();
    longitudes.dedup();
    let (west, east, crosses_antimeridian) = if longitudes.len() == 1 {
        (longitudes[0], longitudes[0], false)
    } else {
        let (gap_index, _) = (0..longitudes.len())
            .map(|index| {
                let next = if index + 1 < longitudes.len() {
                    longitudes[index + 1]
                } else {
                    longitudes[0] + 360_000_000
                };
                (index, next - longitudes[index])
            })
            .max_by_key(|(_, gap)| *gap)?;
        let west = longitudes[(gap_index + 1) % longitudes.len()];
        let east = longitudes[gap_index];
        (west, east, west > east)
    };
    Some(RegionalBounds {
        west_microdegrees: west,
        south_microdegrees: south,
        east_microdegrees: east,
        north_microdegrees: north,
        crosses_antimeridian,
    })
}

fn projection_packet_digest(
    packet: &RegionalProjectionPacket,
) -> Result<SemanticDigest, RegionalSceneError> {
    let mut normalized = packet.clone();
    normalized.packet_id = placeholder_digest();
    let mut hasher = SemanticHasher::new(REGIONAL_PROJECTION_PACKET_SCHEMA);
    hasher.add_bytes(&serde_json::to_vec(&normalized)?);
    Ok(hasher.finish())
}

fn regional_scene_digest(
    scene: &AdmittedRegionalScene,
) -> Result<SemanticDigest, RegionalSceneError> {
    let mut normalized = scene.clone();
    normalized.scene_id = placeholder_digest();
    normalized.artifacts.admitted_atlas_revision = None;
    let mut hasher = SemanticHasher::new(ADMITTED_REGIONAL_SCENE_SCHEMA);
    hasher.add_bytes(&serde_json::to_vec(&normalized)?);
    Ok(hasher.finish())
}

#[derive(Debug, Error)]
pub enum RegionalSceneError {
    #[error("admitted regional scene schema is unsupported")]
    Schema,
    #[error("regional projection packet schema is unsupported")]
    ProjectionSchema,
    #[error("regional scene identifier is invalid: {0}")]
    Identifier(String),
    #[error("regional coordinate bounds are invalid")]
    CoordinateBounds,
    #[error("regional projection shape or limits are invalid")]
    ProjectionShape,
    #[error("regional scene binding does not match its package, projection, or artifacts")]
    Binding,
    #[error("regional terrain authority is invalid")]
    TerrainAuthority,
    #[error("regional native-object authority is invalid")]
    ObjectAuthority,
    #[error("regional County footprint is invalid")]
    Footprint,
    #[error("regional layer references an unknown or non-canonical object")]
    LayerReference,
    #[error("duplicate regional {0} identity")]
    Duplicate(&'static str),
    #[error("regional projection identity does not match its semantic content")]
    ProjectionIdentity,
    #[error("admitted regional scene identity does not match its semantic content")]
    Identity,
    #[error("admitted regional scene atlas back-reference is invalid")]
    AtlasBinding,
    #[error("admitted regional scene County-local frame is invalid")]
    CountyFrame,
    #[error("regional scene completeness does not match its projection")]
    Completeness,
    #[error(transparent)]
    Grammar(#[from] ExplorerGrammarError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(kind: &str, value: &str) -> SemanticDigest {
        let mut hasher = SemanticHasher::new(kind);
        hasher.add_str(value);
        hasher.finish()
    }

    fn contract(id: &str) -> ContractIdentity {
        ContractIdentity::new(id, 1, id)
    }

    fn fixture_scene(
        region: &str,
        west: i64,
        east: i64,
        south: i64,
        north: i64,
    ) -> AdmittedRegionalScene {
        let package = digest("fixture.package", region);
        let snapshot = digest("fixture.snapshot", region);
        let grammar = ExplorerGrammar::v1().unwrap();
        let native_bounds = RegionalBounds {
            west_microdegrees: west,
            south_microdegrees: south,
            east_microdegrees: east,
            north_microdegrees: north,
            crosses_antimeridian: west > east,
        };
        let object_id = format!("{region}/boundary");
        let object = RegionalNativeObject {
            object_id: object_id.clone(),
            source_id: region.to_owned(),
            source_path: format!("fixtures/{region}.geojson"),
            source_artifact_id: digest("fixture.artifact", region),
            object_revision: digest("fixture.object", region),
            geometry_kind: "Polygon".to_owned(),
            native_bounds: native_bounds.clone(),
            layer: RegionalLayerKind::Boundary,
            authority: "exact admitted native geometry; appearance grants no relationship, activity, or action authority".to_owned(),
        };
        let footprint = RegionalFootprint {
            footprint_id: placeholder_digest(),
            source_object_id: object.object_id.clone(),
            source_artifact_id: object.source_artifact_id.clone(),
            source_object_revision: object.object_revision.clone(),
            geometry_kind: "Polygon".to_owned(),
            native_bounds: native_bounds.clone(),
            rings: vec![vec![
                [west, south],
                [east, south],
                [east, north],
                [west, north],
                [west, south],
            ]],
            coordinate_count: 5,
            authority:
                "exact admitted native boundary polygon; footprint validity ends at its rings"
                    .to_owned(),
        }
        .finalize()
        .unwrap();
        let unsupported = RegionalValidity {
            validity_id: digest("fixture.validity", region),
            class: RegionalValidityClass::Unsupported,
            scope: "terrain_height".to_owned(),
            source_revision: snapshot.clone(),
            rule: "no qualified terrain-height evidence was supplied".to_owned(),
        };
        let bindings = [
            (
                RegionalCoordinateSpace::NativeCrs84,
                RegionalCoordinateStatus::Bound,
                "OGC CRS84 longitude/latitude",
            ),
            (
                RegionalCoordinateSpace::SyntheticSemantic,
                RegionalCoordinateStatus::Derived,
                "revision-bound synthetic atlas placement",
            ),
            (
                RegionalCoordinateSpace::SemanticMercator,
                RegionalCoordinateStatus::Derived,
                "spherical Mercator over synthetic semantic coordinates",
            ),
            (
                RegionalCoordinateSpace::CountyLocal,
                RegionalCoordinateStatus::Bound,
                "revision-bound local east/north/up tangent frame",
            ),
            (
                RegionalCoordinateSpace::Camera,
                RegionalCoordinateStatus::ViewOnly,
                "browser view envelope only; never evidence identity",
            ),
        ]
        .into_iter()
        .map(|(space, status, authority)| RegionalCoordinateBinding {
            space,
            status,
            dimensions: match space {
                RegionalCoordinateSpace::CountyLocal => {
                    vec!["east".to_owned(), "north".to_owned(), "up".to_owned()]
                }
                RegionalCoordinateSpace::Camera => vec![
                    "center".to_owned(),
                    "scale".to_owned(),
                    "viewport".to_owned(),
                ],
                _ => vec!["longitude".to_owned(), "latitude".to_owned()],
            },
            units: match space {
                RegionalCoordinateSpace::CountyLocal => vec!["local_microunit".to_owned(); 3],
                RegionalCoordinateSpace::Camera => vec!["view_state".to_owned(); 3],
                _ => vec!["microdegree".to_owned(); 2],
            },
            authority: authority.to_owned(),
            source_revision: snapshot.clone(),
            disclosure: "coordinate space is not interchangeable with another binding".to_owned(),
        })
        .collect();
        let omission = RegionalSceneOmission {
            kind: "terrain_program_absent".to_owned(),
            subject: region.to_owned(),
            omitted_count: 1,
            reason: "candidate terrain controls cannot become observed terrain without a qualified terrain adapter".to_owned(),
        };
        let projection = RegionalProjectionPacket {
            schema: REGIONAL_PROJECTION_PACKET_SCHEMA.to_owned(),
            packet_id: placeholder_digest(),
            source_package_id: package.clone(),
            source_snapshot_revision: snapshot.clone(),
            grammar_id: grammar.grammar_id,
            coordinate_bindings: bindings,
            transforms: vec![RegionalTransform {
                transform: contract("rey.scene.native-to-county"),
                source_space: RegionalCoordinateSpace::NativeCrs84,
                target_space: RegionalCoordinateSpace::CountyLocal,
                source_origin: regional_bounds_center(&native_bounds),
                target_origin: vec![0, 0, 0],
                parameters: vec!["east_north_up_microunits".to_owned()],
                inverse_policy: "bounded analytic inverse inside admitted native envelope"
                    .to_owned(),
                distortion: "declared local tangent approximation; not physical survey accuracy"
                    .to_owned(),
            }],
            objects: vec![object],
            footprint: Some(footprint),
            layers: vec![RegionalLayer {
                layer_id: format!("{region}.boundary"),
                kind: RegionalLayerKind::Boundary,
                object_ids: vec![object_id],
                authority: "exact admitted native boundary geometry".to_owned(),
                semantics: "boundary appearance grants no ownership or jurisdiction claim"
                    .to_owned(),
                source_revision: snapshot.clone(),
            }],
            validity: vec![unsupported],
            terrain: None,
            terrain_program_id: None,
            limits: RegionalSceneLimits::default(),
            complete: false,
            omissions: vec![omission.clone()],
            lineage: vec![RegionalSceneLineage {
                kind: "fixture".to_owned(),
                identity: region.to_owned(),
                revision: snapshot.to_string(),
            }],
        }
        .finalize()
        .unwrap();
        AdmittedRegionalScene {
            schema: ADMITTED_REGIONAL_SCENE_SCHEMA.to_owned(),
            scene_id: placeholder_digest(),
            region_id: region.to_owned(),
            admission: SceneAdmissionBinding {
                admission_id: digest("fixture.admission", region),
                operation: contract("rey.scene-admission.validate"),
                implementation: contract("rey.scene-admission.builtin"),
                workload: contract("scene-admission"),
                graph: contract("scene-admission.graph"),
                scenario_suite: contract("scene-admission.scenarios"),
                evaluator: contract("rey.scenario.utf8-exact"),
                capability_snapshot_id: digest("fixture.capabilities", region),
                editor_commit_id: digest("fixture.editor-commit", region),
                editor_sequence: 1,
                package_id: package,
                parent_package_id: None,
                package_snapshot_revision: snapshot,
                admission_request_id: digest("fixture.request", region),
            },
            native_bounds,
            artifacts: RegionalArtifactBindings {
                source_topography_patch_id: None,
                admitted_atlas_revision: None,
                projection_packet_id: placeholder_digest(),
                terrain_program_id: None,
                terrain_authority: "none; candidate-only terrain controls were not copied into observed terrain truth".to_owned(),
            },
            complete: false,
            omissions: vec![omission],
            lineage: Vec::new(),
            projection,
        }
        .finalize()
        .unwrap()
    }

    #[test]
    fn bounded_multi_region_fixture_preserves_overlap_poles_and_antimeridian_outcomes() {
        let west = fixture_scene(
            "west-county",
            -123_000_000,
            -122_000_000,
            37_000_000,
            38_000_000,
        );
        let overlap = fixture_scene(
            "overlap-county",
            -122_500_000,
            -121_500_000,
            37_500_000,
            38_500_000,
        );
        assert_ne!(west.scene_id, overlap.scene_id);
        assert!(west.native_bounds.east_microdegrees > overlap.native_bounds.west_microdegrees);

        let polar = fixture_scene(
            "polar-county",
            10_000_000,
            11_000_000,
            86_000_000,
            87_000_000,
        );
        assert!(polar.verify().is_ok());
        assert_eq!(
            ExplorerGrammar::v1().unwrap().mercator.polar_policy,
            "World retains polar caps; Atlas exposes clipped contents and never silently drops them"
        );

        let antimeridian = fixture_scene(
            "wrap-county",
            179_000_000,
            -179_000_000,
            -1_000_000,
            1_000_000,
        );
        assert!(antimeridian.native_bounds.crosses_antimeridian);
        assert!(antimeridian.verify().is_ok());

        let mut rejected = antimeridian;
        rejected.native_bounds.crosses_antimeridian = false;
        assert!(matches!(
            rejected.verify(),
            Err(RegionalSceneError::CoordinateBounds)
        ));
    }

    #[test]
    fn scene_rejects_candidate_terrain_controls_as_observed_truth() {
        let mut scene = fixture_scene(
            "terrain-county",
            -123_000_000,
            -122_000_000,
            37_000_000,
            38_000_000,
        );
        scene.artifacts.terrain_authority = "candidate terrain looks convincing".to_owned();
        assert!(matches!(
            scene.verify(),
            Err(RegionalSceneError::TerrainAuthority)
        ));
    }

    #[test]
    fn county_frame_must_bind_the_exact_native_envelope_center() {
        let mut scene = fixture_scene(
            "frame-county",
            -123_000_000,
            -122_000_000,
            37_000_000,
            38_000_000,
        );
        scene.projection.transforms[0].source_origin[0] += 1;
        scene.projection = scene.projection.finalize().expect("projection");
        scene.artifacts.projection_packet_id = scene.projection.packet_id.clone();
        assert!(matches!(
            scene.finalize(),
            Err(RegionalSceneError::CountyFrame)
        ));
    }

    #[test]
    fn county_footprint_must_bind_exact_closed_boundary_geometry() {
        let scene = fixture_scene(
            "footprint-county",
            -123_000_000,
            -122_000_000,
            37_000_000,
            38_000_000,
        );
        let mut footprint = scene.projection.footprint.clone().expect("footprint");
        footprint.rings[0][1][0] += 1;
        assert!(matches!(
            footprint.verify(),
            Err(RegionalSceneError::Footprint)
        ));

        let mut packet = scene.projection.clone();
        packet.footprint.as_mut().unwrap().source_object_revision =
            digest("fixture.object", "other");
        packet.packet_id = projection_packet_digest(&packet).unwrap();
        assert!(matches!(
            packet.verify(),
            Err(RegionalSceneError::Footprint)
        ));
    }

    #[test]
    fn atlas_back_reference_is_non_owning_and_cannot_be_rebound() {
        let scene = fixture_scene(
            "atlas-county",
            -123_000_000,
            -122_000_000,
            37_000_000,
            38_000_000,
        );
        let scene_id = scene.scene_id.clone();
        let atlas_revision = digest("fixture.atlas", "one");
        let bound = scene
            .with_admitted_atlas_revision(&atlas_revision)
            .expect("bound scene");
        assert_eq!(bound.scene_id, scene_id);
        assert_eq!(
            bound.artifacts.admitted_atlas_revision.as_ref(),
            Some(&atlas_revision)
        );
        assert!(bound.verify().is_ok());
        assert!(matches!(
            bound.with_admitted_atlas_revision(&digest("fixture.atlas", "two")),
            Err(RegionalSceneError::AtlasBinding)
        ));
    }
}
