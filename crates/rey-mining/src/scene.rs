use std::collections::BTreeSet;

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ExplorerGrammar, ExplorerGrammarError};

pub const ADMITTED_REGIONAL_SCENE_SCHEMA: &str = "rey.admitted-regional-scene.v1";
pub const REGIONAL_PROJECTION_PACKET_SCHEMA: &str = "rey.regional-projection-packet.v1";

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
    TerrainControl,
    Hydrology,
    Boundary,
    Poi,
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
    pub layers: Vec<RegionalLayer>,
    pub validity: Vec<RegionalValidity>,
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
        if self.admission.editor_sequence == 0
            || self.admission.package_id != self.projection.source_package_id
            || self.admission.package_snapshot_revision != self.projection.source_snapshot_revision
            || self.artifacts.projection_packet_id != self.projection.packet_id
            || self.artifacts.terrain_program_id != self.projection.terrain_program_id
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
        if self.complete != self.projection.complete || self.omissions != self.projection.omissions
        {
            return Err(RegionalSceneError::Completeness);
        }
        if self.scene_id != regional_scene_digest(self)? {
            return Err(RegionalSceneError::Identity);
        }
        Ok(())
    }
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
    for object in &packet.objects {
        validate_identifier_path(&object.object_id)?;
        validate_bounds(&object.native_bounds)?;
        if object.authority
            != "exact admitted native geometry; appearance grants no relationship, activity, or action authority"
        {
            return Err(RegionalSceneError::ObjectAuthority);
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
    }
    if packet.terrain_program_id.is_none()
        && !packet.validity.iter().any(|record| {
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
    #[error("regional layer references an unknown or non-canonical object")]
    LayerReference,
    #[error("duplicate regional {0} identity")]
    Duplicate(&'static str),
    #[error("regional projection identity does not match its semantic content")]
    ProjectionIdentity,
    #[error("admitted regional scene identity does not match its semantic content")]
    Identity,
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
        let object_id = format!("{region}/boundary");
        let object = RegionalNativeObject {
            object_id: object_id.clone(),
            source_id: region.to_owned(),
            source_path: format!("fixtures/{region}.geojson"),
            source_artifact_id: digest("fixture.artifact", region),
            object_revision: digest("fixture.object", region),
            geometry_kind: "Polygon".to_owned(),
            native_bounds: RegionalBounds {
                west_microdegrees: west,
                south_microdegrees: south,
                east_microdegrees: east,
                north_microdegrees: north,
                crosses_antimeridian: west > east,
            },
            layer: RegionalLayerKind::Boundary,
            authority: "exact admitted native geometry; appearance grants no relationship, activity, or action authority".to_owned(),
        };
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
            units: vec!["declared_by_space".to_owned()],
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
                source_origin: vec![(west + east) / 2, (south + north) / 2],
                target_origin: vec![0, 0, 0],
                parameters: vec!["local_tangent_east_north_up".to_owned()],
                inverse_policy: "bounded analytic inverse inside admitted footprint".to_owned(),
                distortion: "declared local tangent approximation; not physical survey accuracy"
                    .to_owned(),
            }],
            objects: vec![object],
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
            native_bounds: RegionalBounds {
                west_microdegrees: west,
                south_microdegrees: south,
                east_microdegrees: east,
                north_microdegrees: north,
                crosses_antimeridian: west > east,
            },
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
}
