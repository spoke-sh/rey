use std::collections::{BTreeMap, BTreeSet};

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_mining::{
    ADMITTED_REGIONAL_SCENE_SCHEMA, AdmittedRegionalScene, ExplorerGrammar,
    REGIONAL_PROJECTION_PACKET_SCHEMA, RegionalArtifactBindings, RegionalBounds,
    RegionalCoordinateBinding, RegionalCoordinateSpace, RegionalCoordinateStatus,
    RegionalFootprint, RegionalLayer, RegionalLayerKind, RegionalNativeObject,
    RegionalProjectionPacket, RegionalSceneLimits, RegionalSceneLineage, RegionalSceneOmission,
    RegionalTransform, RegionalValidity, RegionalValidityClass, SceneAdmissionBinding,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const SCENE_ADMISSION_RESULT_SCHEMA: &str = "rey.scene-admission-result.v1";
pub const SCENE_ADMISSION_CANDIDATE_SCHEMA: &str = "rey.scene-admission-candidate.v1";
pub const SCENE_ADMISSION_WORKLOAD_ID: &str = "scene-admission";
pub const SCENE_ADMISSION_OPERATION_ID: &str = "rey.scene-admission.validate";
pub const RENDER_ADMITTED_REGIONAL_SCENE_OPERATION_ID: &str =
    "rey.admitted-regional-scene.render-lines";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneAdmissionCoordinateSystem {
    pub kind: String,
    pub authority: String,
    pub code: String,
    pub axis_order: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneAdmissionSource {
    pub source_id: String,
    pub worktree_path: String,
    pub format: String,
    pub role: String,
    pub media_type: String,
    pub artifact_id: SemanticDigest,
    pub artifact_path: String,
    pub declared_bytes: u64,
    pub native_bytes: Option<Vec<u8>>,
    pub feature_count: u64,
    pub coordinate_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneAdmissionFeature {
    pub feature_id: String,
    pub source_id: String,
    pub source_feature_id: String,
    pub role: String,
    pub geometry_kind: String,
    pub native_bounds: RegionalBounds,
    pub coordinate_count: u64,
    pub properties_digest: SemanticDigest,
    pub feature_revision: SemanticDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneAdmissionCandidate {
    pub schema: String,
    pub candidate_id: SemanticDigest,
    pub editor_commit_id: SemanticDigest,
    pub editor_sequence: u64,
    pub latest_editor_sequence: u64,
    pub package_id: SemanticDigest,
    pub parent_package_id: Option<SemanticDigest>,
    pub package_snapshot_revision: SemanticDigest,
    pub package_authority: String,
    pub admission_request_id: SemanticDigest,
    pub admission_request_package_id: SemanticDigest,
    pub requested_operation: String,
    pub request_status: String,
    pub request_admitted: bool,
    pub project_id: String,
    pub coordinate_system: SceneAdmissionCoordinateSystem,
    pub native_bounds: RegionalBounds,
    pub sources: Vec<SceneAdmissionSource>,
    pub features: Vec<SceneAdmissionFeature>,
    pub complete: bool,
    pub omissions: Vec<String>,
}

impl SceneAdmissionCandidate {
    pub fn finalize(mut self) -> Result<Self, SceneAdmissionError> {
        self.candidate_id = candidate_digest(&self)?;
        self.verify()?;
        Ok(self)
    }

    pub fn verify(&self) -> Result<(), SceneAdmissionError> {
        if self.schema != SCENE_ADMISSION_CANDIDATE_SCHEMA {
            return Err(SceneAdmissionError::Schema);
        }
        validate_identifier(&self.project_id)?;
        validate_bounds(&self.native_bounds)?;
        if self.editor_sequence == 0
            || self.latest_editor_sequence == 0
            || self.editor_sequence > self.latest_editor_sequence
        {
            return Err(SceneAdmissionError::EditorSequence);
        }
        if self.editor_sequence == 1 && self.parent_package_id.is_some()
            || self.editor_sequence > 1 && self.parent_package_id.is_none()
        {
            return Err(SceneAdmissionError::ParentBinding);
        }
        if self.package_authority != "candidate_only"
            || self.admission_request_package_id != self.package_id
            || self.requested_operation != "rey.scene-admission.validate@1"
            || self.request_status != "requires_workload"
            || self.request_admitted
        {
            return Err(SceneAdmissionError::PackageBinding);
        }
        if self.coordinate_system
            != (SceneAdmissionCoordinateSystem {
                kind: "geographic".to_owned(),
                authority: "OGC".to_owned(),
                code: "CRS84".to_owned(),
                axis_order: "longitude_latitude".to_owned(),
            })
        {
            return Err(SceneAdmissionError::CoordinateSystem);
        }
        if self.sources.is_empty()
            || self.features.is_empty()
            || !self
                .sources
                .windows(2)
                .all(|pair| pair[0].source_id < pair[1].source_id)
            || !self
                .features
                .windows(2)
                .all(|pair| pair[0].feature_id < pair[1].feature_id)
        {
            return Err(SceneAdmissionError::CanonicalOrder);
        }
        unique(self.sources.iter().map(|source| source.source_id.as_str()))?;
        unique(
            self.features
                .iter()
                .map(|feature| feature.feature_id.as_str()),
        )?;
        if !self.complete || !self.omissions.is_empty() {
            return Err(SceneAdmissionError::CandidateIncomplete);
        }
        if self.candidate_id != candidate_digest(self)? {
            return Err(SceneAdmissionError::CandidateIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneAdmissionLimits {
    pub max_sources: u64,
    pub max_features: u64,
    pub max_coordinates: u64,
    pub max_source_bytes: u64,
    pub max_total_bytes: u64,
    pub max_omissions: u64,
}

impl Default for SceneAdmissionLimits {
    fn default() -> Self {
        Self {
            max_sources: 64,
            max_features: 10_000,
            max_coordinates: 1_000_000,
            max_source_bytes: 16 * 1_024 * 1_024,
            max_total_bytes: 64 * 1_024 * 1_024,
            max_omissions: 1_024,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneAdmissionFixture {
    Accepted,
    PackageTampering,
    ObjectTampering,
    StaleParent,
    UnsupportedFormat,
    CoordinateMismatch,
    DuplicateIdentity,
    MissingObject,
    BoundsExceeded,
    Polar,
    Antimeridian,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneAdmissionScenario {
    pub fixture: SceneAdmissionFixture,
    #[serde(default)]
    pub limits: SceneAdmissionLimits,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SceneAdmissionInput {
    pub candidate: SceneAdmissionCandidate,
    pub limits: SceneAdmissionLimits,
    pub capability_snapshot_id: SemanticDigest,
}

#[derive(Clone, Debug)]
pub struct SceneAdmissionExecutionContext<'a> {
    pub workload: &'a ContractIdentity,
    pub graph: &'a ContractIdentity,
    pub scenario_suite: &'a ContractIdentity,
    pub evaluator: &'a ContractIdentity,
    pub scenario: Option<&'a ContractIdentity>,
    pub campaign_id: &'a SemanticDigest,
    pub graph_node_id: &'a str,
    pub declared_scene: &'a str,
    pub input: &'a SceneAdmissionInput,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneAdmissionStatus {
    Accepted,
    Rejected,
}

impl SceneAdmissionStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneAdmissionResult {
    pub schema: String,
    pub result_id: SemanticDigest,
    pub candidate_id: SemanticDigest,
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario: Option<ContractIdentity>,
    pub campaign_id: SemanticDigest,
    pub capability_snapshot_id: SemanticDigest,
    pub status: SceneAdmissionStatus,
    pub code: String,
    pub detail: String,
    pub scene: Option<AdmittedRegionalScene>,
    pub limits: SceneAdmissionLimits,
    pub authority: String,
}

impl SceneAdmissionResult {
    pub fn verify(&self) -> Result<(), SceneAdmissionError> {
        if self.schema != SCENE_ADMISSION_RESULT_SCHEMA {
            return Err(SceneAdmissionError::ResultSchema);
        }
        match (self.status, self.scene.as_ref()) {
            (SceneAdmissionStatus::Accepted, Some(scene)) => {
                scene.verify()?;
                if scene.admission.workload != self.workload
                    || scene.admission.graph != self.graph
                    || scene.admission.capability_snapshot_id != self.capability_snapshot_id
                {
                    return Err(SceneAdmissionError::ResultShape);
                }
            }
            (SceneAdmissionStatus::Rejected, None) => {}
            _ => return Err(SceneAdmissionError::ResultShape),
        }
        if self.code.is_empty()
            || self.detail.is_empty()
            || self.authority
                != "qualified workload result only; no editor mutation, browser admission, terrain inference, action, or proof authority"
        {
            return Err(SceneAdmissionError::ResultShape);
        }
        if self.result_id != result_digest(self)? {
            return Err(SceneAdmissionError::ResultIdentity);
        }
        Ok(())
    }

    pub fn with_atlas_bound_scene(
        mut self,
        scene: AdmittedRegionalScene,
    ) -> Result<Self, SceneAdmissionError> {
        self.verify()?;
        scene.verify()?;
        let current = self
            .scene
            .as_ref()
            .ok_or(SceneAdmissionError::ResultShape)?;
        let mut without_back_reference = scene.clone();
        without_back_reference.artifacts.admitted_atlas_revision =
            current.artifacts.admitted_atlas_revision.clone();
        if scene.artifacts.admitted_atlas_revision.is_none() || without_back_reference != *current {
            return Err(SceneAdmissionError::ResultShape);
        }
        self.scene = Some(scene);
        self.result_id = result_digest(&self)?;
        self.verify()?;
        Ok(self)
    }
}

pub fn scene_admission_operation_contract() -> ContractIdentity {
    ContractIdentity::new(
        SCENE_ADMISSION_OPERATION_ID,
        1,
        "validate one exact current editor candidate transfer envelope and emit an accepted regional scene or typed rejection without mutating editor or Explorer state",
    )
}

pub fn render_admitted_regional_scene_contract() -> ContractIdentity {
    ContractIdentity::new(
        RENDER_ADMITTED_REGIONAL_SCENE_OPERATION_ID,
        1,
        "render one typed scene-admission outcome as stable coordinate-explicit UTF-8 evidence",
    )
}

pub fn execute_scene_admission(
    context: SceneAdmissionExecutionContext<'_>,
) -> Result<SceneAdmissionResult, SceneAdmissionError> {
    validate_limits(&context.input.limits)?;
    let candidate = &context.input.candidate;
    let declared = format!("SCENE@{}", candidate.editor_sequence);
    let outcome = if context.declared_scene != declared {
        Err(SceneAdmissionRejection::new(
            "scene_label_mismatch",
            format!(
                "declared scene {} does not match exact editor revision {declared}",
                context.declared_scene
            ),
        ))
    } else {
        validate_candidate(candidate, &context.input.limits)
    };
    let (status, code, detail, scene) = match outcome {
        Ok(()) => {
            let scene = build_scene(&context)?;
            (
                SceneAdmissionStatus::Accepted,
                "accepted".to_owned(),
                "exact current editor package and every bounded native object passed the declared admission contract".to_owned(),
                Some(scene),
            )
        }
        Err(rejection) => (
            SceneAdmissionStatus::Rejected,
            rejection.code,
            rejection.detail,
            None,
        ),
    };
    let mut result = SceneAdmissionResult {
        schema: SCENE_ADMISSION_RESULT_SCHEMA.to_owned(),
        result_id: placeholder_digest(),
        candidate_id: candidate.candidate_id.clone(),
        workload: context.workload.clone(),
        graph: context.graph.clone(),
        scenario: context.scenario.cloned(),
        campaign_id: context.campaign_id.clone(),
        capability_snapshot_id: context.input.capability_snapshot_id.clone(),
        status,
        code,
        detail,
        scene,
        limits: context.input.limits.clone(),
        authority: "qualified workload result only; no editor mutation, browser admission, terrain inference, action, or proof authority".to_owned(),
    };
    result.result_id = result_digest(&result)?;
    result.verify()?;
    Ok(result)
}

pub fn render_scene_admission_result(result: &SceneAdmissionResult) -> String {
    let mut lines = vec![
        "SCENE ADMISSION".to_owned(),
        format!(
            "OUTCOME {} · {}",
            result.status.as_str().to_ascii_uppercase(),
            result.code
        ),
        format!("DETAIL {}", result.detail),
    ];
    if let Some(scene) = &result.scene {
        lines.extend([
            format!(
                "SCENE {} · SCENE@{} · {} native objects · {} layers · {} validity records",
                scene.region_id,
                scene.admission.editor_sequence,
                scene.projection.objects.len(),
                scene.projection.layers.len(),
                scene.projection.validity.len()
            ),
            format!(
                "NATIVE OGC:CRS84 longitude/latitude · {} → {} · exact geographic source evidence",
                format_microdegrees(
                    scene.native_bounds.west_microdegrees,
                    scene.native_bounds.south_microdegrees
                ),
                format_microdegrees(
                    scene.native_bounds.east_microdegrees,
                    scene.native_bounds.north_microdegrees
                )
            ),
            "SYNTHETIC semantic longitude/latitude · revision-bound projection placement · no Earth CRS or distance claim".to_owned(),
            "MERCATOR spherical chart of synthetic semantic coordinates · 360000000µ° wrap · ±85051129µ° cutoff with polar disclosure · analytic inverse · not EPSG:3857".to_owned(),
            "COUNTY local east/north/up · revision-bound tangent frame · bounded inverse inside admitted native envelope · envelope is not footprint geometry".to_owned(),
            scene.projection.footprint.as_ref().map_or_else(
                || "FOOTPRINT none · no unique admitted boundary Polygon matches the exact scene envelope".to_owned(),
                |footprint| format!(
                    "FOOTPRINT admitted · source {} · {} · {} rings / {} coordinates · exact identity and validity boundary retained",
                    footprint.source_object_id,
                    footprint.geometry_kind,
                    footprint.rings.len(),
                    footprint.coordinate_count,
                ),
            ),
            "CAMERA view only · center/scale/viewport/selection are not evidence identity".to_owned(),
            format!(
                "ARTIFACTS topography={} · atlas={} · projection={} · terrain={}",
                optional_artifact(scene.artifacts.source_topography_patch_id.as_ref()),
                optional_artifact(scene.artifacts.admitted_atlas_revision.as_ref()),
                "retained",
                optional_artifact(scene.artifacts.terrain_program_id.as_ref())
            ),
            format!(
                "COVERAGE {} · {} omissions · native objects complete; terrain height explicitly unsupported",
                if scene.complete { "complete" } else { "partial" },
                scene.omissions.len()
            ),
        ]);
    }
    lines.push(format!("AUTHORITY {}", result.authority));
    lines.join("\n")
}

pub fn scene_admission_fixture(
    fixture: SceneAdmissionFixture,
) -> Result<SceneAdmissionCandidate, SceneAdmissionError> {
    let (west, south, east, north, coordinates) = match fixture {
        SceneAdmissionFixture::Polar => (
            10_000_000,
            86_000_000,
            11_000_000,
            87_000_000,
            vec![[10.0, 86.0], [11.0, 86.0], [11.0, 87.0], [10.0, 86.0]],
        ),
        SceneAdmissionFixture::Antimeridian => (
            179_000_000,
            -1_000_000,
            -179_000_000,
            1_000_000,
            vec![[179.0, -1.0], [-179.0, -1.0], [-179.0, 1.0], [179.0, -1.0]],
        ),
        _ => (
            -123_000_000,
            37_000_000,
            -122_000_000,
            38_000_000,
            vec![
                [-123.0, 37.0],
                [-122.0, 37.0],
                [-122.0, 38.0],
                [-123.0, 37.0],
            ],
        ),
    };
    let native = serde_json::to_vec(&serde_json::json!({
        "type": "FeatureCollection",
        "features": [{
            "type": "Feature",
            "id": "county-boundary",
            "properties": {"name": "Fixture County"},
            "geometry": {"type": "Polygon", "coordinates": [coordinates]},
        }]
    }))?;
    let artifact_id = native_artifact_digest(&native);
    let feature_value: Value = serde_json::from_slice::<Value>(&native)?["features"][0].clone();
    let properties = serde_json::to_vec(
        feature_value["properties"]
            .as_object()
            .ok_or(SceneAdmissionError::GeoJson)?,
    )?;
    let bounds = RegionalBounds {
        west_microdegrees: west,
        south_microdegrees: south,
        east_microdegrees: east,
        north_microdegrees: north,
        crosses_antimeridian: west > east,
    };
    let source = SceneAdmissionSource {
        source_id: "fixture-county".to_owned(),
        worktree_path: "fixtures/county.geojson".to_owned(),
        format: "geo_json".to_owned(),
        role: "boundary".to_owned(),
        media_type: "application/geo+json".to_owned(),
        artifact_id,
        artifact_path: "objects/fixture.geojson".to_owned(),
        declared_bytes: native.len() as u64,
        native_bytes: Some(native),
        feature_count: 1,
        coordinate_count: 4,
    };
    let feature = SceneAdmissionFeature {
        feature_id: "fixture-county/county-boundary".to_owned(),
        source_id: "fixture-county".to_owned(),
        source_feature_id: "county-boundary".to_owned(),
        role: "boundary".to_owned(),
        geometry_kind: "Polygon".to_owned(),
        native_bounds: bounds.clone(),
        coordinate_count: 4,
        properties_digest: properties_digest(&properties),
        feature_revision: feature_digest("fixture-county", "boundary", &feature_value)?,
    };
    let package_id = semantic_digest("rey.fixture.scene-package.v1", "fixture-current");
    let mut candidate = SceneAdmissionCandidate {
        schema: SCENE_ADMISSION_CANDIDATE_SCHEMA.to_owned(),
        candidate_id: placeholder_digest(),
        editor_commit_id: semantic_digest("rey.fixture.scene-commit.v1", "SCENE@2"),
        editor_sequence: 2,
        latest_editor_sequence: 2,
        package_id: package_id.clone(),
        parent_package_id: Some(semantic_digest(
            "rey.fixture.scene-package.v1",
            "fixture-parent",
        )),
        package_snapshot_revision: semantic_digest(
            "rey.fixture.scene-snapshot.v1",
            "fixture-current",
        ),
        package_authority: "candidate_only".to_owned(),
        admission_request_id: semantic_digest("rey.fixture.scene-request.v1", "fixture-current"),
        admission_request_package_id: package_id,
        requested_operation: "rey.scene-admission.validate@1".to_owned(),
        request_status: "requires_workload".to_owned(),
        request_admitted: false,
        project_id: "fixture-county".to_owned(),
        coordinate_system: SceneAdmissionCoordinateSystem {
            kind: "geographic".to_owned(),
            authority: "OGC".to_owned(),
            code: "CRS84".to_owned(),
            axis_order: "longitude_latitude".to_owned(),
        },
        native_bounds: bounds,
        sources: vec![source],
        features: vec![feature],
        complete: true,
        omissions: Vec::new(),
    };
    match fixture {
        SceneAdmissionFixture::Accepted
        | SceneAdmissionFixture::Polar
        | SceneAdmissionFixture::Antimeridian
        | SceneAdmissionFixture::BoundsExceeded => {}
        SceneAdmissionFixture::PackageTampering => {
            candidate.package_id = semantic_digest("rey.fixture.scene-package.v1", "tampered");
        }
        SceneAdmissionFixture::ObjectTampering => {
            candidate.sources[0]
                .native_bytes
                .as_mut()
                .expect("fixture bytes")
                .push(b' ');
        }
        SceneAdmissionFixture::StaleParent => {
            candidate.latest_editor_sequence = 3;
        }
        SceneAdmissionFixture::UnsupportedFormat => {
            candidate.sources[0].format = "geo_package".to_owned();
        }
        SceneAdmissionFixture::CoordinateMismatch => {
            candidate.coordinate_system.code = "EPSG:3857".to_owned();
        }
        SceneAdmissionFixture::DuplicateIdentity => {
            candidate.features.push(candidate.features[0].clone());
        }
        SceneAdmissionFixture::MissingObject => {
            candidate.sources[0].native_bytes = None;
        }
    }
    candidate.candidate_id = candidate_digest(&candidate)?;
    Ok(candidate)
}

fn validate_candidate(
    candidate: &SceneAdmissionCandidate,
    limits: &SceneAdmissionLimits,
) -> Result<(), SceneAdmissionRejection> {
    candidate.verify().map_err(|error| {
        let code = match error {
            SceneAdmissionError::CandidateIdentity | SceneAdmissionError::PackageBinding => {
                "package_tampering"
            }
            SceneAdmissionError::CoordinateSystem | SceneAdmissionError::CoordinateBounds => {
                "coordinate_mismatch"
            }
            SceneAdmissionError::CanonicalOrder => "duplicate_identity",
            _ => "candidate_invalid",
        };
        SceneAdmissionRejection::new(code, error.to_string())
    })?;
    if candidate.editor_sequence != candidate.latest_editor_sequence {
        return Err(SceneAdmissionRejection::new(
            "stale_parent",
            format!(
                "SCENE@{} is not current editor HEAD SCENE@{}",
                candidate.editor_sequence, candidate.latest_editor_sequence
            ),
        ));
    }
    if candidate.sources.len() as u64 > limits.max_sources
        || candidate.features.len() as u64 > limits.max_features
        || candidate.omissions.len() as u64 > limits.max_omissions
    {
        return Err(SceneAdmissionRejection::new(
            "bounds_exceeded",
            "candidate count exceeds the effective scene-admission limit".to_owned(),
        ));
    }
    let mut total_bytes = 0_u64;
    let mut total_coordinates = 0_u64;
    let features_by_source = candidate.features.iter().fold(
        BTreeMap::<&str, Vec<SceneAdmissionFeature>>::new(),
        |mut grouped, feature| {
            grouped
                .entry(feature.source_id.as_str())
                .or_default()
                .push(feature.clone());
            grouped
        },
    );
    let mut observed_bounds: Option<RegionalBounds> = None;
    for source in &candidate.sources {
        if source.format != "geo_json" || source.media_type != "application/geo+json" {
            return Err(SceneAdmissionRejection::new(
                "unsupported_format",
                format!(
                    "source {} uses unsupported format {}",
                    source.source_id, source.format
                ),
            ));
        }
        let Some(bytes) = source.native_bytes.as_ref() else {
            return Err(SceneAdmissionRejection::new(
                "missing_object",
                format!("source {} has no frozen native bytes", source.source_id),
            ));
        };
        if bytes.len() as u64 != source.declared_bytes {
            return Err(SceneAdmissionRejection::new(
                "object_tampering",
                format!(
                    "source {} bytes do not match the frozen byte binding",
                    source.source_id
                ),
            ));
        }
        if bytes.len() as u64 > limits.max_source_bytes {
            return Err(SceneAdmissionRejection::new(
                "bounds_exceeded",
                format!("source {} exceeds its byte limit", source.source_id),
            ));
        }
        if native_artifact_digest(bytes) != source.artifact_id {
            return Err(SceneAdmissionRejection::new(
                "object_tampering",
                format!(
                    "source {} bytes do not match the frozen artifact identity",
                    source.source_id
                ),
            ));
        }
        total_bytes = total_bytes.saturating_add(bytes.len() as u64);
        let observed = inspect_geojson_source(source, bytes).map_err(|error| {
            SceneAdmissionRejection::new("coordinate_mismatch", error.to_string())
        })?;
        let declared = features_by_source
            .get(source.source_id.as_str())
            .cloned()
            .unwrap_or_default();
        if observed.features != declared {
            return Err(SceneAdmissionRejection::new(
                "package_tampering",
                format!(
                    "source {} feature index does not match exact native bytes",
                    source.source_id
                ),
            ));
        }
        total_coordinates = total_coordinates.saturating_add(observed.coordinate_count);
        observed_bounds = Some(match observed_bounds {
            Some(bounds) => merge_bounds(&bounds, &observed.bounds).ok_or_else(|| {
                SceneAdmissionRejection::new(
                    "coordinate_mismatch",
                    "candidate mixes incompatible antimeridian envelopes".to_owned(),
                )
            })?,
            None => observed.bounds,
        });
    }
    if total_bytes > limits.max_total_bytes || total_coordinates > limits.max_coordinates {
        return Err(SceneAdmissionRejection::new(
            "bounds_exceeded",
            "candidate exceeds total byte or coordinate limits".to_owned(),
        ));
    }
    if observed_bounds.as_ref() != Some(&candidate.native_bounds) {
        return Err(SceneAdmissionRejection::new(
            "coordinate_mismatch",
            "candidate native bounds do not match its exact object coordinates".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct InspectedSource {
    features: Vec<SceneAdmissionFeature>,
    coordinate_count: u64,
    bounds: RegionalBounds,
}

fn inspect_geojson_source(
    source: &SceneAdmissionSource,
    bytes: &[u8],
) -> Result<InspectedSource, SceneAdmissionError> {
    let document: Value = serde_json::from_slice(bytes)?;
    let object = document.as_object().ok_or(SceneAdmissionError::GeoJson)?;
    if object.contains_key("crs") {
        return Err(SceneAdmissionError::CoordinateSystem);
    }
    let feature_values = match object.get("type").and_then(Value::as_str) {
        Some("FeatureCollection") => object
            .get("features")
            .and_then(Value::as_array)
            .ok_or(SceneAdmissionError::GeoJson)?
            .iter()
            .collect::<Vec<_>>(),
        Some("Feature") => vec![&document],
        _ => return Err(SceneAdmissionError::GeoJson),
    };
    if feature_values.len() as u64 != source.feature_count {
        return Err(SceneAdmissionError::FeatureIndex);
    }
    let mut inspected = Vec::with_capacity(feature_values.len());
    let mut all_positions = Vec::new();
    for feature in feature_values {
        let object = feature.as_object().ok_or(SceneAdmissionError::GeoJson)?;
        if object.get("type").and_then(Value::as_str) != Some("Feature") {
            return Err(SceneAdmissionError::GeoJson);
        }
        let source_feature_id = feature_id(object.get("id"))?;
        let geometry = object
            .get("geometry")
            .and_then(Value::as_object)
            .ok_or(SceneAdmissionError::GeoJson)?;
        let geometry_kind = geometry
            .get("type")
            .and_then(Value::as_str)
            .ok_or(SceneAdmissionError::GeoJson)?;
        let mut positions = Vec::new();
        collect_geometry_positions(geometry, &mut positions)?;
        all_positions.extend(positions.iter().copied());
        let bounds = bounds_from_positions(&positions)?;
        let properties = match object.get("properties") {
            Some(Value::Object(properties)) => properties,
            Some(Value::Null) | None => {
                static EMPTY: std::sync::LazyLock<serde_json::Map<String, Value>> =
                    std::sync::LazyLock::new(serde_json::Map::new);
                &EMPTY
            }
            _ => return Err(SceneAdmissionError::GeoJson),
        };
        let properties_bytes = serde_json::to_vec(properties)?;
        let feature_revision = feature_digest(source.source_id.as_str(), &source.role, feature)?;
        inspected.push(SceneAdmissionFeature {
            feature_id: format!("{}/{}", source.source_id, source_feature_id),
            source_id: source.source_id.clone(),
            source_feature_id,
            role: source.role.clone(),
            geometry_kind: geometry_kind.to_owned(),
            native_bounds: bounds,
            coordinate_count: positions.len() as u64,
            properties_digest: properties_digest(&properties_bytes),
            feature_revision,
        });
    }
    inspected.sort_by(|left, right| left.feature_id.cmp(&right.feature_id));
    Ok(InspectedSource {
        features: inspected,
        coordinate_count: all_positions.len() as u64,
        bounds: bounds_from_positions(&all_positions)?,
    })
}

fn build_scene(
    context: &SceneAdmissionExecutionContext<'_>,
) -> Result<AdmittedRegionalScene, SceneAdmissionError> {
    let candidate = &context.input.candidate;
    let grammar = ExplorerGrammar::v1()?;
    let limits = RegionalSceneLimits {
        max_sources: context.input.limits.max_sources,
        max_native_objects: context.input.limits.max_features,
        max_native_coordinates: context.input.limits.max_coordinates,
        max_layers: 16,
        max_validity_records: context.input.limits.max_features.saturating_add(2),
        max_transforms: 4,
        max_omissions: context.input.limits.max_omissions,
        max_native_bytes: context.input.limits.max_total_bytes,
    };
    let admission_id = admission_digest(context);
    let mut objects = candidate
        .features
        .iter()
        .map(|feature| {
            let source = candidate
                .sources
                .iter()
                .find(|source| source.source_id == feature.source_id)
                .ok_or(SceneAdmissionError::FeatureIndex)?;
            Ok(RegionalNativeObject {
                object_id: feature.feature_id.clone(),
                source_id: feature.source_id.clone(),
                source_path: source.worktree_path.clone(),
                source_artifact_id: source.artifact_id.clone(),
                object_revision: feature.feature_revision.clone(),
                geometry_kind: feature.geometry_kind.clone(),
                native_bounds: feature.native_bounds.clone(),
                layer: layer_kind(&feature.role)?,
                authority: "exact admitted native geometry; appearance grants no relationship, activity, or action authority".to_owned(),
            })
        })
        .collect::<Result<Vec<_>, SceneAdmissionError>>()?;
    objects.sort_by(|left, right| left.object_id.cmp(&right.object_id));
    let footprint = build_regional_footprint(candidate, &objects)?;
    let mut grouped = BTreeMap::<RegionalLayerKind, Vec<String>>::new();
    for object in &objects {
        grouped
            .entry(object.layer)
            .or_default()
            .push(object.object_id.clone());
    }
    let mut layers = grouped
        .into_iter()
        .map(|(kind, object_ids)| {
            let (authority, semantics) = if kind == RegionalLayerKind::TerrainControl {
                (
                    "candidate control geometry only; no observed height, material, or terrain validity",
                    "native terrain-control geometry retained for exact inspection; generated effects and hints are excluded",
                )
            } else {
                (
                    "exact admitted native geometry",
                    "typed native geometry; familiar appearance grants no inferred relationship or authority",
                )
            };
            RegionalLayer {
                layer_id: format!("{}.{}", candidate.project_id, layer_label(kind)),
                kind,
                object_ids,
                authority: authority.to_owned(),
                semantics: semantics.to_owned(),
                source_revision: candidate.package_snapshot_revision.clone(),
            }
        })
        .collect::<Vec<_>>();
    layers.sort_by(|left, right| left.layer_id.cmp(&right.layer_id));
    let mut validity = objects
        .iter()
        .map(|object| RegionalValidity {
            validity_id: semantic_digest(
                "rey.regional-validity.native-object.v1",
                &object.object_id,
            ),
            class: RegionalValidityClass::Valid,
            scope: format!("native_geometry:{}", object.object_id),
            source_revision: object.object_revision.clone(),
            rule: "valid only for the exact admitted native geometry; bounds alone do not fill its interior".to_owned(),
        })
        .collect::<Vec<_>>();
    validity.push(RegionalValidity {
        validity_id: semantic_digest("rey.regional-validity.no-data.v1", &candidate.project_id),
        class: RegionalValidityClass::NoData,
        scope: "outside_admitted_native_geometries".to_owned(),
        source_revision: candidate.package_snapshot_revision.clone(),
        rule: "the regional envelope does not interpolate source geometry into unsupplied space"
            .to_owned(),
    });
    validity.push(RegionalValidity {
        validity_id: semantic_digest(
            "rey.regional-validity.terrain-unsupported.v1",
            &candidate.project_id,
        ),
        class: RegionalValidityClass::Unsupported,
        scope: "terrain_height".to_owned(),
        source_revision: candidate.package_snapshot_revision.clone(),
        rule: "no qualified terrain-height adapter or observed terrain program was supplied"
            .to_owned(),
    });
    validity.sort_by(|left, right| left.validity_id.cmp(&right.validity_id));
    let semantic_center = synthetic_center(&candidate.package_id)?;
    let native_center = bounds_center(&candidate.native_bounds);
    let transforms = vec![
        RegionalTransform {
            transform: ContractIdentity::new(
                "rey.scene.native-to-semantic-region",
                1,
                "affine placement of one bounded native envelope into a deterministic synthetic atlas footprint without changing native coordinate authority",
            ),
            source_space: RegionalCoordinateSpace::NativeCrs84,
            target_space: RegionalCoordinateSpace::SyntheticSemantic,
            source_origin: native_center.clone(),
            target_origin: semantic_center.clone(),
            parameters: vec![
                "bounded_envelope_affine".to_owned(),
                "maximum_angular_radius_microdegrees=5000000".to_owned(),
            ],
            inverse_policy: "analytic inside the exact admitted footprint; no extrapolation outside validity".to_owned(),
            distortion: "synthetic placement only; no semantic similarity, physical distance, or geographic-area claim".to_owned(),
        },
        RegionalTransform {
            transform: ContractIdentity::new(
                "rey.scene.native-to-county-local",
                1,
                "revision-bound local east/north/up tangent approximation around the native envelope center",
            ),
            source_space: RegionalCoordinateSpace::NativeCrs84,
            target_space: RegionalCoordinateSpace::CountyLocal,
            source_origin: native_center,
            target_origin: vec![0, 0, 0],
            parameters: vec!["east_north_up_microunits".to_owned()],
            inverse_policy: "bounded analytic inverse inside the admitted native envelope".to_owned(),
            distortion: "local tangent presentation; not a geodetic accuracy or physical-distance proof".to_owned(),
        },
        RegionalTransform {
            transform: ContractIdentity::new(
                "rey.explore.semantic-mercator",
                1,
                "spherical Mercator chart over synthetic semantic longitude/latitude with wrap and polar disclosure",
            ),
            source_space: RegionalCoordinateSpace::SyntheticSemantic,
            target_space: RegionalCoordinateSpace::SemanticMercator,
            source_origin: semantic_center,
            target_origin: vec![0, 0],
            parameters: vec![
                format!(
                    "latitude_cutoff_microdegrees={}",
                    grammar.mercator.latitude_cutoff_microdegrees
                ),
                "wrap_microdegrees=360000000".to_owned(),
            ],
            inverse_policy: grammar.picking.atlas_inverse.clone(),
            distortion: grammar.mercator.distance_claim.clone(),
        },
    ];
    let coordinate_bindings = coordinate_bindings(candidate, &grammar);
    let terrain_omission = RegionalSceneOmission {
        kind: "terrain_program_absent".to_owned(),
        subject: candidate.project_id.clone(),
        omitted_count: 1,
        reason: "candidate terrain controls and generated effects cannot become observed height without a separately qualified terrain adapter".to_owned(),
    };
    let mut omissions = vec![terrain_omission];
    if footprint.is_none() {
        omissions.push(RegionalSceneOmission {
            kind: "county_footprint_absent".to_owned(),
            subject: candidate.project_id.clone(),
            omitted_count: 1,
            reason: "no unique admitted boundary Polygon matches the exact scene envelope; County footprint fabric remains unavailable".to_owned(),
        });
    }
    let lineage = vec![
        RegionalSceneLineage {
            kind: "editor_commit".to_owned(),
            identity: format!("SCENE@{}", candidate.editor_sequence),
            revision: candidate.editor_commit_id.to_string(),
        },
        RegionalSceneLineage {
            kind: "scene_package".to_owned(),
            identity: candidate.package_id.to_string(),
            revision: candidate.package_snapshot_revision.to_string(),
        },
        RegionalSceneLineage {
            kind: "workload_admission".to_owned(),
            identity: admission_id.to_string(),
            revision: scene_admission_operation_contract()
                .semantic_digest
                .to_string(),
        },
    ];
    let projection = RegionalProjectionPacket {
        schema: REGIONAL_PROJECTION_PACKET_SCHEMA.to_owned(),
        packet_id: placeholder_digest(),
        source_package_id: candidate.package_id.clone(),
        source_snapshot_revision: candidate.package_snapshot_revision.clone(),
        grammar_id: grammar.grammar_id,
        coordinate_bindings,
        transforms,
        objects,
        footprint,
        layers,
        validity,
        terrain_program_id: None,
        limits,
        complete: true,
        omissions: omissions.clone(),
        lineage: lineage.clone(),
    }
    .finalize()?;
    AdmittedRegionalScene {
        schema: ADMITTED_REGIONAL_SCENE_SCHEMA.to_owned(),
        scene_id: placeholder_digest(),
        region_id: candidate.project_id.clone(),
        admission: SceneAdmissionBinding {
            admission_id,
            operation: scene_admission_operation_contract(),
            implementation: ContractIdentity::new(
                "rey.scene-admission.builtin",
                1,
                "deterministic bounded validation of the exact editor transfer envelope and native GeoJSON objects",
            ),
            workload: context.workload.clone(),
            graph: context.graph.clone(),
            scenario_suite: context.scenario_suite.clone(),
            evaluator: context.evaluator.clone(),
            capability_snapshot_id: context.input.capability_snapshot_id.clone(),
            editor_commit_id: candidate.editor_commit_id.clone(),
            editor_sequence: candidate.editor_sequence,
            package_id: candidate.package_id.clone(),
            parent_package_id: candidate.parent_package_id.clone(),
            package_snapshot_revision: candidate.package_snapshot_revision.clone(),
            admission_request_id: candidate.admission_request_id.clone(),
        },
        native_bounds: candidate.native_bounds.clone(),
        artifacts: RegionalArtifactBindings {
            source_topography_patch_id: None,
            admitted_atlas_revision: None,
            projection_packet_id: projection.packet_id.clone(),
            terrain_program_id: None,
            terrain_authority: "none; candidate-only terrain controls were not copied into observed terrain truth".to_owned(),
        },
        complete: true,
        omissions,
        lineage,
        projection,
    }
    .finalize()
    .map_err(SceneAdmissionError::from)
}

fn coordinate_bindings(
    candidate: &SceneAdmissionCandidate,
    grammar: &ExplorerGrammar,
) -> Vec<RegionalCoordinateBinding> {
    [
        (
            RegionalCoordinateSpace::NativeCrs84,
            RegionalCoordinateStatus::Bound,
            vec!["longitude".to_owned(), "latitude".to_owned()],
            vec!["microdegree".to_owned(), "microdegree".to_owned()],
            "OGC CRS84 exact native geographic evidence",
            "native coordinates retain longitude/latitude axis order and are never relabeled as semantic or Mercator coordinates",
        ),
        (
            RegionalCoordinateSpace::SyntheticSemantic,
            RegionalCoordinateStatus::Derived,
            vec![
                "semantic_longitude".to_owned(),
                "semantic_latitude".to_owned(),
            ],
            vec!["microdegree".to_owned(), "microdegree".to_owned()],
            "revision-bound deterministic atlas placement",
            "synthetic placement has no Earth CRS, physical-distance, area, or general semantic-similarity claim",
        ),
        (
            RegionalCoordinateSpace::SemanticMercator,
            RegionalCoordinateStatus::Derived,
            vec!["chart_x".to_owned(), "chart_y".to_owned()],
            vec!["chart_microunit".to_owned(), "chart_microunit".to_owned()],
            "spherical Mercator over synthetic semantic coordinates",
            grammar.mercator.native_crs_claim.as_str(),
        ),
        (
            RegionalCoordinateSpace::CountyLocal,
            RegionalCoordinateStatus::Bound,
            vec!["east".to_owned(), "north".to_owned(), "up".to_owned()],
            vec!["local_microunit".to_owned(); 3],
            "revision-bound local tangent frame",
            "County-local coordinates are bounded presentation geometry and do not replace native CRS84 evidence",
        ),
        (
            RegionalCoordinateSpace::Camera,
            RegionalCoordinateStatus::ViewOnly,
            vec![
                "center".to_owned(),
                "scale".to_owned(),
                "viewport".to_owned(),
            ],
            vec!["view_state".to_owned(); 3],
            "browser view envelope only",
            grammar.camera.camera_identity_rule.as_str(),
        ),
    ]
    .into_iter()
    .map(
        |(space, status, dimensions, units, authority, disclosure)| {
            RegionalCoordinateBinding {
                space,
                status,
                dimensions,
                units,
                authority: authority.to_owned(),
                source_revision: candidate.package_snapshot_revision.clone(),
                disclosure: disclosure.to_owned(),
            }
        },
    )
    .collect()
}

fn build_regional_footprint(
    candidate: &SceneAdmissionCandidate,
    objects: &[RegionalNativeObject],
) -> Result<Option<RegionalFootprint>, SceneAdmissionError> {
    let eligible = objects
        .iter()
        .filter(|object| {
            object.layer == RegionalLayerKind::Boundary
                && object.geometry_kind == "Polygon"
                && object.native_bounds == candidate.native_bounds
        })
        .collect::<Vec<_>>();
    let [object] = eligible.as_slice() else {
        return Ok(None);
    };
    let declared = candidate
        .features
        .iter()
        .find(|feature| feature.feature_id == object.object_id)
        .ok_or(SceneAdmissionError::FeatureIndex)?;
    let source = candidate
        .sources
        .iter()
        .find(|source| source.source_id == object.source_id)
        .ok_or(SceneAdmissionError::FeatureIndex)?;
    let bytes = source
        .native_bytes
        .as_ref()
        .ok_or(SceneAdmissionError::FeatureIndex)?;
    let document: Value = serde_json::from_slice(bytes)?;
    let root = document.as_object().ok_or(SceneAdmissionError::GeoJson)?;
    let features = match root.get("type").and_then(Value::as_str) {
        Some("FeatureCollection") => root
            .get("features")
            .and_then(Value::as_array)
            .ok_or(SceneAdmissionError::GeoJson)?
            .iter()
            .collect::<Vec<_>>(),
        Some("Feature") => vec![&document],
        _ => return Err(SceneAdmissionError::GeoJson),
    };
    let feature = features
        .into_iter()
        .find(|feature| {
            feature
                .as_object()
                .and_then(|feature| feature_id(feature.get("id")).ok())
                .as_deref()
                == Some(declared.source_feature_id.as_str())
        })
        .ok_or(SceneAdmissionError::FeatureIndex)?;
    let geometry = feature
        .as_object()
        .and_then(|feature| feature.get("geometry"))
        .and_then(Value::as_object)
        .ok_or(SceneAdmissionError::GeoJson)?;
    let rings = regional_polygon_rings(geometry)?;
    let coordinate_count = rings.iter().map(Vec::len).sum::<usize>() as u64;
    RegionalFootprint {
        footprint_id: placeholder_digest(),
        source_object_id: object.object_id.clone(),
        source_artifact_id: object.source_artifact_id.clone(),
        source_object_revision: object.object_revision.clone(),
        geometry_kind: object.geometry_kind.clone(),
        native_bounds: object.native_bounds.clone(),
        rings,
        coordinate_count,
        authority: "exact admitted native boundary polygon; footprint validity ends at its rings"
            .to_owned(),
    }
    .finalize()
    .map(Some)
    .map_err(SceneAdmissionError::from)
}

fn regional_polygon_rings(
    geometry: &serde_json::Map<String, Value>,
) -> Result<Vec<Vec<[i64; 2]>>, SceneAdmissionError> {
    if geometry.get("type").and_then(Value::as_str) != Some("Polygon") {
        return Err(SceneAdmissionError::GeoJson);
    }
    geometry
        .get("coordinates")
        .and_then(Value::as_array)
        .ok_or(SceneAdmissionError::GeoJson)?
        .iter()
        .map(|ring| {
            ring.as_array()
                .ok_or(SceneAdmissionError::GeoJson)?
                .iter()
                .map(|position| {
                    let coordinates = position.as_array().ok_or(SceneAdmissionError::GeoJson)?;
                    let longitude = coordinates
                        .first()
                        .and_then(Value::as_f64)
                        .filter(|value| value.is_finite())
                        .ok_or(SceneAdmissionError::GeoJson)?;
                    let latitude = coordinates
                        .get(1)
                        .and_then(Value::as_f64)
                        .filter(|value| value.is_finite())
                        .ok_or(SceneAdmissionError::GeoJson)?;
                    if !(-180.0..=180.0).contains(&longitude) || !(-90.0..=90.0).contains(&latitude)
                    {
                        return Err(SceneAdmissionError::CoordinateBounds);
                    }
                    Ok([to_microdegrees(longitude), to_microdegrees(latitude)])
                })
                .collect()
        })
        .collect()
}

fn layer_kind(role: &str) -> Result<RegionalLayerKind, SceneAdmissionError> {
    match role {
        "features" => Ok(RegionalLayerKind::NativeFeature),
        "terrain_control" => Ok(RegionalLayerKind::TerrainControl),
        "hydrology" => Ok(RegionalLayerKind::Hydrology),
        "boundary" => Ok(RegionalLayerKind::Boundary),
        "markers" => Ok(RegionalLayerKind::Poi),
        _ => Err(SceneAdmissionError::UnsupportedRole(role.to_owned())),
    }
}

const fn layer_label(kind: RegionalLayerKind) -> &'static str {
    match kind {
        RegionalLayerKind::NativeFeature => "native-feature",
        RegionalLayerKind::TerrainControl => "terrain-control",
        RegionalLayerKind::Hydrology => "hydrology",
        RegionalLayerKind::Boundary => "boundary",
        RegionalLayerKind::Poi => "poi",
    }
}

fn validate_limits(limits: &SceneAdmissionLimits) -> Result<(), SceneAdmissionError> {
    if limits.max_sources == 0
        || limits.max_features == 0
        || limits.max_coordinates == 0
        || limits.max_source_bytes == 0
        || limits.max_total_bytes == 0
        || limits.max_omissions == 0
        || limits.max_source_bytes > limits.max_total_bytes
    {
        return Err(SceneAdmissionError::InvalidLimits);
    }
    Ok(())
}

fn collect_geometry_positions(
    geometry: &serde_json::Map<String, Value>,
    positions: &mut Vec<(i64, i64)>,
) -> Result<(), SceneAdmissionError> {
    if geometry.get("type").and_then(Value::as_str) == Some("GeometryCollection") {
        for nested in geometry
            .get("geometries")
            .and_then(Value::as_array)
            .ok_or(SceneAdmissionError::GeoJson)?
        {
            collect_geometry_positions(
                nested.as_object().ok_or(SceneAdmissionError::GeoJson)?,
                positions,
            )?;
        }
        return Ok(());
    }
    collect_positions(
        geometry
            .get("coordinates")
            .ok_or(SceneAdmissionError::GeoJson)?,
        positions,
    )
}

fn collect_positions(
    value: &Value,
    positions: &mut Vec<(i64, i64)>,
) -> Result<(), SceneAdmissionError> {
    let values = value.as_array().ok_or(SceneAdmissionError::GeoJson)?;
    if values.len() >= 2 && values[0].is_number() && values[1].is_number() {
        let longitude = values[0]
            .as_f64()
            .filter(|value| value.is_finite())
            .ok_or(SceneAdmissionError::GeoJson)?;
        let latitude = values[1]
            .as_f64()
            .filter(|value| value.is_finite())
            .ok_or(SceneAdmissionError::GeoJson)?;
        if !(-180.0..=180.0).contains(&longitude) || !(-90.0..=90.0).contains(&latitude) {
            return Err(SceneAdmissionError::CoordinateBounds);
        }
        positions.push((to_microdegrees(longitude), to_microdegrees(latitude)));
        return Ok(());
    }
    for nested in values {
        collect_positions(nested, positions)?;
    }
    Ok(())
}

fn bounds_from_positions(positions: &[(i64, i64)]) -> Result<RegionalBounds, SceneAdmissionError> {
    if positions.is_empty() {
        return Err(SceneAdmissionError::GeoJson);
    }
    let south = positions
        .iter()
        .map(|(_, latitude)| *latitude)
        .min()
        .ok_or(SceneAdmissionError::GeoJson)?;
    let north = positions
        .iter()
        .map(|(_, latitude)| *latitude)
        .max()
        .ok_or(SceneAdmissionError::GeoJson)?;
    let mut longitudes = positions
        .iter()
        .map(|(longitude, _)| *longitude)
        .collect::<Vec<_>>();
    longitudes.sort_unstable();
    longitudes.dedup();
    let (west, east, crosses) = if longitudes.len() == 1 {
        (longitudes[0], longitudes[0], false)
    } else {
        let mut largest_gap = i64::MIN;
        let mut gap_index = 0;
        for index in 0..longitudes.len() {
            let current = longitudes[index];
            let next = if index + 1 < longitudes.len() {
                longitudes[index + 1]
            } else {
                longitudes[0] + 360_000_000
            };
            let gap = next - current;
            if gap > largest_gap {
                largest_gap = gap;
                gap_index = index;
            }
        }
        let west = longitudes[(gap_index + 1) % longitudes.len()];
        let east = longitudes[gap_index];
        (west, east, west > east)
    };
    let bounds = RegionalBounds {
        west_microdegrees: west,
        south_microdegrees: south,
        east_microdegrees: east,
        north_microdegrees: north,
        crosses_antimeridian: crosses,
    };
    validate_bounds(&bounds)?;
    Ok(bounds)
}

fn merge_bounds(left: &RegionalBounds, right: &RegionalBounds) -> Option<RegionalBounds> {
    if left.crosses_antimeridian || right.crosses_antimeridian {
        return (left == right).then(|| left.clone());
    }
    Some(RegionalBounds {
        west_microdegrees: left.west_microdegrees.min(right.west_microdegrees),
        south_microdegrees: left.south_microdegrees.min(right.south_microdegrees),
        east_microdegrees: left.east_microdegrees.max(right.east_microdegrees),
        north_microdegrees: left.north_microdegrees.max(right.north_microdegrees),
        crosses_antimeridian: false,
    })
}

fn validate_bounds(bounds: &RegionalBounds) -> Result<(), SceneAdmissionError> {
    if !(-180_000_000..=180_000_000).contains(&bounds.west_microdegrees)
        || !(-180_000_000..=180_000_000).contains(&bounds.east_microdegrees)
        || !(-90_000_000..=90_000_000).contains(&bounds.south_microdegrees)
        || !(-90_000_000..=90_000_000).contains(&bounds.north_microdegrees)
        || bounds.south_microdegrees > bounds.north_microdegrees
        || (!bounds.crosses_antimeridian && bounds.west_microdegrees > bounds.east_microdegrees)
        || (bounds.crosses_antimeridian && bounds.west_microdegrees <= bounds.east_microdegrees)
    {
        return Err(SceneAdmissionError::CoordinateBounds);
    }
    Ok(())
}

fn bounds_center(bounds: &RegionalBounds) -> Vec<i64> {
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

fn synthetic_center(digest: &SemanticDigest) -> Result<Vec<i64>, SceneAdmissionError> {
    let hex = digest
        .as_str()
        .strip_prefix("blake3:")
        .ok_or(SceneAdmissionError::CandidateIdentity)?;
    let longitude_lane =
        u64::from_str_radix(&hex[0..12], 16).map_err(|_| SceneAdmissionError::CandidateIdentity)?;
    let latitude_lane = u64::from_str_radix(&hex[12..24], 16)
        .map_err(|_| SceneAdmissionError::CandidateIdentity)?;
    Ok(vec![
        i64::try_from(longitude_lane % 320_000_001).unwrap_or(0) - 160_000_000,
        i64::try_from(latitude_lane % 140_000_001).unwrap_or(0) - 70_000_000,
    ])
}

fn feature_id(value: Option<&Value>) -> Result<String, SceneAdmissionError> {
    let value = match value {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        _ => return Err(SceneAdmissionError::FeatureIndex),
    };
    validate_identifier(&value)?;
    Ok(value)
}

fn validate_identifier(value: &str) -> Result<(), SceneAdmissionError> {
    if value.is_empty()
        || value.chars().count() > 96
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err(SceneAdmissionError::Identifier(value.to_owned()));
    }
    Ok(())
}

fn unique<'a>(values: impl Iterator<Item = &'a str>) -> Result<(), SceneAdmissionError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(SceneAdmissionError::CanonicalOrder);
        }
    }
    Ok(())
}

fn to_microdegrees(value: f64) -> i64 {
    (value * 1_000_000.0).round() as i64
}

fn format_microdegrees(longitude: i64, latitude: i64) -> String {
    format!("{longitude}µ°, {latitude}µ°")
}

fn optional_artifact(value: Option<&SemanticDigest>) -> &'static str {
    value.map_or("none", |_| "retained")
}

fn native_artifact_digest(bytes: &[u8]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.scene-native-artifact.v1");
    hasher.add_bytes(bytes);
    hasher.finish()
}

fn properties_digest(bytes: &[u8]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.scene-feature-properties.v1");
    hasher.add_bytes(bytes);
    hasher.finish()
}

fn feature_digest(
    source_id: &str,
    role: &str,
    feature: &Value,
) -> Result<SemanticDigest, SceneAdmissionError> {
    let mut hasher = SemanticHasher::new("rey.scene-feature.v1");
    hasher.add_str(source_id);
    hasher.add_str(role);
    hasher.add_bytes(&serde_json::to_vec(feature)?);
    Ok(hasher.finish())
}

fn semantic_digest(domain: &str, value: &str) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(domain);
    hasher.add_str(value);
    hasher.finish()
}

fn placeholder_digest() -> SemanticDigest {
    semantic_digest(
        "rey.scene-admission.placeholder.v1",
        "excluded from identity",
    )
}

fn candidate_digest(
    candidate: &SceneAdmissionCandidate,
) -> Result<SemanticDigest, SceneAdmissionError> {
    let mut normalized = candidate.clone();
    normalized.candidate_id = placeholder_digest();
    let mut hasher = SemanticHasher::new(SCENE_ADMISSION_CANDIDATE_SCHEMA);
    hasher.add_bytes(&serde_json::to_vec(&normalized)?);
    Ok(hasher.finish())
}

fn admission_digest(context: &SceneAdmissionExecutionContext<'_>) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.scene-admission.v1");
    context.workload.add_semantics(&mut hasher);
    context.graph.add_semantics(&mut hasher);
    context.scenario_suite.add_semantics(&mut hasher);
    context.evaluator.add_semantics(&mut hasher);
    hasher.add_optional_str(
        context
            .scenario
            .map(|scenario| scenario.semantic_digest.as_str()),
    );
    hasher.add_str(context.campaign_id.as_str());
    hasher.add_str(context.graph_node_id);
    hasher.add_str(context.input.candidate.candidate_id.as_str());
    hasher.add_str(context.input.capability_snapshot_id.as_str());
    add_limits(&mut hasher, &context.input.limits);
    hasher.finish()
}

fn result_digest(result: &SceneAdmissionResult) -> Result<SemanticDigest, SceneAdmissionError> {
    let mut normalized = result.clone();
    normalized.result_id = placeholder_digest();
    let mut hasher = SemanticHasher::new(SCENE_ADMISSION_RESULT_SCHEMA);
    hasher.add_bytes(&serde_json::to_vec(&normalized)?);
    Ok(hasher.finish())
}

pub fn add_scene_admission_scenario_semantics(
    hasher: &mut SemanticHasher,
    scenario: &SceneAdmissionScenario,
) {
    hasher.add_str(match scenario.fixture {
        SceneAdmissionFixture::Accepted => "accepted",
        SceneAdmissionFixture::PackageTampering => "package_tampering",
        SceneAdmissionFixture::ObjectTampering => "object_tampering",
        SceneAdmissionFixture::StaleParent => "stale_parent",
        SceneAdmissionFixture::UnsupportedFormat => "unsupported_format",
        SceneAdmissionFixture::CoordinateMismatch => "coordinate_mismatch",
        SceneAdmissionFixture::DuplicateIdentity => "duplicate_identity",
        SceneAdmissionFixture::MissingObject => "missing_object",
        SceneAdmissionFixture::BoundsExceeded => "bounds_exceeded",
        SceneAdmissionFixture::Polar => "polar",
        SceneAdmissionFixture::Antimeridian => "antimeridian",
    });
    add_limits(hasher, &scenario.limits);
}

fn add_limits(hasher: &mut SemanticHasher, limits: &SceneAdmissionLimits) {
    hasher.add_u64(limits.max_sources);
    hasher.add_u64(limits.max_features);
    hasher.add_u64(limits.max_coordinates);
    hasher.add_u64(limits.max_source_bytes);
    hasher.add_u64(limits.max_total_bytes);
    hasher.add_u64(limits.max_omissions);
}

#[derive(Debug)]
struct SceneAdmissionRejection {
    code: String,
    detail: String,
}

impl SceneAdmissionRejection {
    fn new(code: &str, detail: String) -> Self {
        Self {
            code: code.to_owned(),
            detail,
        }
    }
}

#[derive(Debug, Error)]
pub enum SceneAdmissionError {
    #[error("scene-admission candidate schema is unsupported")]
    Schema,
    #[error("scene-admission result schema is unsupported")]
    ResultSchema,
    #[error("scene-admission identifier is invalid: {0}")]
    Identifier(String),
    #[error("scene-admission editor sequence is invalid")]
    EditorSequence,
    #[error("scene-admission package parent binding is invalid")]
    ParentBinding,
    #[error("scene-admission package/request binding or candidate-only authority is invalid")]
    PackageBinding,
    #[error("scene-admission coordinate system must be geographic OGC CRS84 longitude/latitude")]
    CoordinateSystem,
    #[error("scene-admission coordinates or bounds are invalid")]
    CoordinateBounds,
    #[error("scene-admission candidate identities are duplicated or noncanonical")]
    CanonicalOrder,
    #[error("scene-admission candidate must be complete and omission-free")]
    CandidateIncomplete,
    #[error("scene-admission candidate identity does not match its exact transfer content")]
    CandidateIdentity,
    #[error("scene-admission effective limits are invalid")]
    InvalidLimits,
    #[error("scene-admission native GeoJSON is malformed or unsupported")]
    GeoJson,
    #[error("scene-admission feature index does not match native objects")]
    FeatureIndex,
    #[error("scene-admission source role is unsupported: {0}")]
    UnsupportedRole(String),
    #[error("scene-admission result has an invalid status/evidence shape")]
    ResultShape,
    #[error("scene-admission result identity does not match its semantic content")]
    ResultIdentity,
    #[error(transparent)]
    Regional(#[from] rey_mining::RegionalSceneError),
    #[error(transparent)]
    Grammar(#[from] rey_mining::ExplorerGrammarError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context<'a>(
        input: &'a SceneAdmissionInput,
        contracts: &'a [ContractIdentity; 4],
        campaign_id: &'a SemanticDigest,
    ) -> SceneAdmissionExecutionContext<'a> {
        SceneAdmissionExecutionContext {
            workload: &contracts[0],
            graph: &contracts[1],
            scenario_suite: &contracts[2],
            evaluator: &contracts[3],
            scenario: None,
            campaign_id,
            graph_node_id: "admit",
            declared_scene: "SCENE@2",
            input,
        }
    }

    fn contracts() -> [ContractIdentity; 4] {
        [
            ContractIdentity::new("scene-admission", 1, "fixture"),
            ContractIdentity::new("scene-admission.graph", 1, "fixture"),
            ContractIdentity::new("scene-admission.scenarios", 1, "fixture"),
            ContractIdentity::new("rey.scenario.utf8-exact", 1, "fixture"),
        ]
    }

    #[test]
    fn accepted_scene_keeps_all_coordinate_planes_and_excludes_terrain_hints() {
        let candidate = scene_admission_fixture(SceneAdmissionFixture::Accepted).unwrap();
        let input = SceneAdmissionInput {
            candidate,
            limits: SceneAdmissionLimits::default(),
            capability_snapshot_id: semantic_digest("fixture.capabilities", "one"),
        };
        let contracts = contracts();
        let campaign = semantic_digest("fixture.campaign", "one");
        let result = execute_scene_admission(context(&input, &contracts, &campaign)).unwrap();
        assert_eq!(result.status, SceneAdmissionStatus::Accepted);
        let scene = result.scene.unwrap();
        assert_eq!(scene.projection.coordinate_bindings.len(), 5);
        let footprint = scene.projection.footprint.as_ref().expect("footprint");
        assert_eq!(footprint.source_object_id, "fixture-county/county-boundary");
        assert_eq!(footprint.rings.len(), 1);
        assert_eq!(footprint.coordinate_count, 4);
        assert!(scene.artifacts.terrain_program_id.is_none());
        assert!(scene.projection.validity.iter().any(|record| {
            record.class == RegionalValidityClass::Unsupported && record.scope == "terrain_height"
        }));
    }

    #[test]
    fn deterministic_rejections_cover_candidate_failure_families() {
        let contracts = contracts();
        let campaign = semantic_digest("fixture.campaign", "rejections");
        for (fixture, code) in [
            (SceneAdmissionFixture::PackageTampering, "package_tampering"),
            (SceneAdmissionFixture::ObjectTampering, "object_tampering"),
            (SceneAdmissionFixture::StaleParent, "stale_parent"),
            (
                SceneAdmissionFixture::UnsupportedFormat,
                "unsupported_format",
            ),
            (
                SceneAdmissionFixture::CoordinateMismatch,
                "coordinate_mismatch",
            ),
            (
                SceneAdmissionFixture::DuplicateIdentity,
                "duplicate_identity",
            ),
            (SceneAdmissionFixture::MissingObject, "missing_object"),
        ] {
            let candidate = scene_admission_fixture(fixture).unwrap();
            let input = SceneAdmissionInput {
                candidate,
                limits: SceneAdmissionLimits::default(),
                capability_snapshot_id: semantic_digest("fixture.capabilities", code),
            };
            let result = execute_scene_admission(context(&input, &contracts, &campaign)).unwrap();
            assert_eq!(result.status, SceneAdmissionStatus::Rejected, "{code}");
            assert_eq!(result.code, code);
        }
    }

    #[test]
    fn polar_antimeridian_bounds_and_replay_are_exact() {
        let contracts = contracts();
        let campaign = semantic_digest("fixture.campaign", "coordinates");
        for fixture in [
            SceneAdmissionFixture::Polar,
            SceneAdmissionFixture::Antimeridian,
        ] {
            let candidate = scene_admission_fixture(fixture).unwrap();
            let input = SceneAdmissionInput {
                candidate,
                limits: SceneAdmissionLimits::default(),
                capability_snapshot_id: semantic_digest("fixture.capabilities", "coordinates"),
            };
            let first = execute_scene_admission(context(&input, &contracts, &campaign)).unwrap();
            let replay = execute_scene_admission(context(&input, &contracts, &campaign)).unwrap();
            assert_eq!(first, replay);
            assert_eq!(first.status, SceneAdmissionStatus::Accepted);
            assert!(
                first
                    .scene
                    .as_ref()
                    .and_then(|scene| scene.projection.footprint.as_ref())
                    .is_some()
            );
        }
    }
}
