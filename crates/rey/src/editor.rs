#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use rey_core::{SemanticDigest, SemanticHasher};
use rey_diff::DeltaAssessment;
use rey_mining::RegionalBounds;
use rey_runtime::{
    SCENE_ADMISSION_CANDIDATE_SCHEMA, SCENE_ADMISSION_REQUESTED_OPERATION, SceneAdmissionCandidate,
    SceneAdmissionCoordinateSystem, SceneAdmissionFeature, SceneAdmissionSource,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const EDITOR_PROJECT_SCHEMA: &str = "rey.editor-project.v1";
pub const SCENE_CANDIDATE_SNAPSHOT_SCHEMA: &str = "rey.scene-candidate-snapshot.v1";
pub const SCENE_CHANGE_SET_SCHEMA: &str = "rey.scene-change-set.v1";
pub const SCENE_PACKAGE_SCHEMA: &str = "rey.scene-package.v1";
pub const SCENE_ADMISSION_REQUEST_SCHEMA: &str = "rey.scene-admission-request.v1";
pub const EDITOR_STATUS_SCHEMA: &str = "rey.editor-status.v2";
pub const EDITOR_STATE_SCHEMA: &str = "rey.editor-state.v1";
pub const EDITOR_ADD_RESULT_SCHEMA: &str = "rey.editor-add-result.v1";
pub const SCENE_COMMIT_SCHEMA: &str = "rey.scene-commit.v1";
pub const EDITOR_COMMIT_RESULT_SCHEMA: &str = "rey.editor-commit-result.v1";
pub const EDITOR_LOG_SCHEMA: &str = "rey.editor-log.v1";
pub const SCENE_GENERATION_SCHEMA: &str = "rey.scene-generation.v1";
pub const EDITOR_GENERATE_RESULT_SCHEMA: &str = "rey.editor-generate-result.v1";
pub const EDITOR_SOURCE_ADD_RESULT_SCHEMA: &str = "rey.editor-source-add-result.v1";

const PROJECT_FILE_NAME: &str = "project.json";
const STATE_FILE_NAME: &str = "state.json";
const LOCK_FILE_NAME: &str = "editor.lock";
const MAX_PROJECT_BYTES: u64 = 1_048_576;
const MAX_SOURCE_BYTES: u64 = 32 * 1_048_576;
const MAX_STATE_BYTES: u64 = 64 * 1_048_576;
const MAX_SOURCES: usize = 64;
const MAX_FEATURES: usize = 50_000;
const MAX_COORDINATES: usize = 1_000_000;
const MAX_PROPERTIES: usize = 64;
const MAX_PROPERTIES_BYTES: usize = 65_536;
const MAX_IDENTIFIER_CHARS: usize = 96;
const MAX_LABEL_CHARS: usize = 160;
const MAX_SCENE_COMMITS: usize = 256;
const MAX_COMMIT_MESSAGE_BYTES: usize = 4_096;
const MAX_GENERATED_FEATURES: u64 = 512;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneCoordinateSystem {
    pub kind: String,
    pub authority: String,
    pub code: String,
    pub axis_order: String,
}

impl SceneCoordinateSystem {
    #[must_use]
    pub fn geojson_crs84() -> Self {
        Self {
            kind: "geographic".to_owned(),
            authority: "OGC".to_owned(),
            code: "CRS84".to_owned(),
            axis_order: "longitude_latitude".to_owned(),
        }
    }

    fn verify_geojson(&self) -> Result<(), EditorError> {
        if self != &Self::geojson_crs84() {
            return Err(EditorError::CoordinateSystem);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneSourceRole {
    Features,
    Markers,
    Terrain,
    TerrainControl,
    Hydrology,
    Boundary,
    Highway,
    Road,
    Railway,
    District,
    Lot,
    Structure,
    Utility,
    Label,
    Beacon,
    Construction,
    Connector,
}

impl SceneSourceRole {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Features => "features",
            Self::Markers => "markers",
            Self::Terrain => "terrain",
            Self::TerrainControl => "terrain_control",
            Self::Hydrology => "hydrology",
            Self::Boundary => "boundary",
            Self::Highway => "highway",
            Self::Road => "road",
            Self::Railway => "railway",
            Self::District => "district",
            Self::Lot => "lot",
            Self::Structure => "structure",
            Self::Utility => "utility",
            Self::Label => "label",
            Self::Beacon => "beacon",
            Self::Construction => "construction",
            Self::Connector => "connector",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneSourceFormat {
    GeoJson,
}

impl SceneSourceFormat {
    #[must_use]
    pub const fn media_type(self) -> &'static str {
        match self {
            Self::GeoJson => "application/geo+json",
        }
    }

    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::GeoJson => "geojson",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneSourceDeclaration {
    pub source_id: String,
    pub path: String,
    pub format: SceneSourceFormat,
    pub role: SceneSourceRole,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EditorProject {
    pub schema: String,
    pub project_id: String,
    pub coordinate_system: SceneCoordinateSystem,
    pub sources: Vec<SceneSourceDeclaration>,
}

impl EditorProject {
    pub fn new(project_id: String) -> Result<Self, EditorError> {
        let project = Self {
            schema: EDITOR_PROJECT_SCHEMA.to_owned(),
            project_id,
            coordinate_system: SceneCoordinateSystem::geojson_crs84(),
            sources: Vec::new(),
        };
        project.verify()?;
        Ok(project)
    }

    pub fn canonicalize(mut self) -> Result<Self, EditorError> {
        self.sources
            .sort_by(|left, right| left.source_id.cmp(&right.source_id));
        self.verify()?;
        Ok(self)
    }

    pub fn verify(&self) -> Result<(), EditorError> {
        if self.schema != EDITOR_PROJECT_SCHEMA {
            return Err(EditorError::Schema {
                expected: EDITOR_PROJECT_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_identifier("project id", &self.project_id)?;
        self.coordinate_system.verify_geojson()?;
        if self.sources.len() > MAX_SOURCES {
            return Err(EditorError::SourceLimit(MAX_SOURCES));
        }
        let mut ids = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for source in &self.sources {
            validate_identifier("source id", &source.source_id)?;
            validate_relative_path(&source.path)?;
            if !ids.insert(source.source_id.as_str()) {
                return Err(EditorError::DuplicateSourceId(source.source_id.clone()));
            }
            if !paths.insert(source.path.as_str()) {
                return Err(EditorError::DuplicateSourcePath(source.path.clone()));
            }
        }
        if !self
            .sources
            .windows(2)
            .all(|pair| pair[0].source_id < pair[1].source_id)
        {
            return Err(EditorError::NonCanonicalProject);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneLimits {
    pub max_sources: u64,
    pub max_source_bytes: u64,
    pub max_features: u64,
    pub max_coordinates: u64,
    pub max_properties_per_feature: u64,
    pub max_properties_bytes_per_feature: u64,
}

impl Default for SceneLimits {
    fn default() -> Self {
        Self {
            max_sources: MAX_SOURCES as u64,
            max_source_bytes: MAX_SOURCE_BYTES,
            max_features: MAX_FEATURES as u64,
            max_coordinates: MAX_COORDINATES as u64,
            max_properties_per_feature: MAX_PROPERTIES as u64,
            max_properties_bytes_per_feature: MAX_PROPERTIES_BYTES as u64,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneNativeArtifact {
    pub media_type: String,
    pub content_digest: SemanticDigest,
    pub bytes: u64,
    pub object_path: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneSourceSnapshot {
    pub source_id: String,
    pub worktree_path: String,
    pub format: SceneSourceFormat,
    pub role: SceneSourceRole,
    pub artifact: SceneNativeArtifact,
    pub feature_count: u64,
    pub coordinate_count: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneBounds {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

impl SceneBounds {
    fn include(&mut self, longitude: f64, latitude: f64) {
        self.west = self.west.min(longitude);
        self.south = self.south.min(latitude);
        self.east = self.east.max(longitude);
        self.north = self.north.max(latitude);
    }

    fn merge(&mut self, other: &Self) {
        self.include(other.west, other.south);
        self.include(other.east, other.north);
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneMarkerIndex {
    pub title: String,
    pub category: Option<String>,
    pub symbol: Option<String>,
    pub min_zoom: u64,
    pub max_zoom: u64,
    pub collision_priority: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneFeatureIndex {
    pub feature_id: String,
    pub source_id: String,
    pub source_feature_id: String,
    pub role: SceneSourceRole,
    pub geometry_kind: String,
    pub bounds: SceneBounds,
    pub coordinate_count: u64,
    pub properties_digest: SemanticDigest,
    pub feature_revision: SemanticDigest,
    pub marker: Option<SceneMarkerIndex>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cartographic_label: Option<SceneMarkerIndex>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain_sample: Option<SceneTerrainSample>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneTerrainSample {
    pub longitude_microdegrees: i64,
    pub latitude_microdegrees: i64,
    pub elevation_micrometers: Option<i64>,
    pub material: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<SceneTerrainGridCell>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packed_grid: Option<SceneTerrainPackedGrid>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneTerrainGridCell {
    pub dataset_id: String,
    pub column: u64,
    pub row: u64,
    pub columns: u64,
    pub rows: u64,
    pub validity: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneTerrainPackedGrid {
    pub schema: String,
    pub dataset_id: String,
    pub compiler_revision: String,
    pub columns: u64,
    pub rows: u64,
    pub native_bounds_microdegrees: [i64; 4],
    pub validity_hex: String,
    pub elevation_centimeters_le_hex: String,
    pub material_palette: Vec<String>,
    pub material_indices_hex: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneCoverage {
    pub sources: u64,
    pub features: u64,
    pub markers: u64,
    pub coordinates: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneCandidateSnapshot {
    pub schema: String,
    pub project_id: String,
    pub snapshot_revision: SemanticDigest,
    pub coordinate_system: SceneCoordinateSystem,
    pub bounds: Option<SceneBounds>,
    pub sources: Vec<SceneSourceSnapshot>,
    pub features: Vec<SceneFeatureIndex>,
    pub coverage: SceneCoverage,
    pub limits: SceneLimits,
    pub complete: bool,
    pub omissions: Vec<String>,
}

impl SceneCandidateSnapshot {
    pub fn verify(&self) -> Result<(), EditorError> {
        if self.schema != SCENE_CANDIDATE_SNAPSHOT_SCHEMA {
            return Err(EditorError::Schema {
                expected: SCENE_CANDIDATE_SNAPSHOT_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_identifier("project id", &self.project_id)?;
        self.coordinate_system.verify_geojson()?;
        if !self
            .sources
            .windows(2)
            .all(|pair| pair[0].source_id < pair[1].source_id)
        {
            return Err(EditorError::NonCanonicalSnapshot);
        }
        if !self
            .features
            .windows(2)
            .all(|pair| pair[0].feature_id < pair[1].feature_id)
        {
            return Err(EditorError::NonCanonicalSnapshot);
        }
        let source_ids = self
            .sources
            .iter()
            .map(|source| source.source_id.as_str())
            .collect::<BTreeSet<_>>();
        if self
            .features
            .iter()
            .any(|feature| !source_ids.contains(feature.source_id.as_str()))
        {
            return Err(EditorError::SnapshotReference);
        }
        let source_feature_count = self
            .sources
            .iter()
            .map(|source| source.feature_count)
            .sum::<u64>();
        let source_coordinate_count = self
            .sources
            .iter()
            .map(|source| source.coordinate_count)
            .sum::<u64>();
        if self.sources.iter().any(|source| {
            source.artifact.media_type != source.format.media_type()
                || source.artifact.object_path
                    != format!(
                        "objects/{}.{}",
                        digest_key(&source.artifact.content_digest),
                        source.format.extension()
                    )
                || validate_identifier("source id", &source.source_id).is_err()
                || validate_relative_path(&source.worktree_path).is_err()
        }) {
            return Err(EditorError::SnapshotSource);
        }
        let source_roles = self
            .sources
            .iter()
            .map(|source| (source.source_id.as_str(), source.role))
            .collect::<BTreeMap<_, _>>();
        if self.features.iter().any(|feature| {
            source_roles.get(feature.source_id.as_str()) != Some(&feature.role)
                || feature.feature_id
                    != format!("{}/{}", feature.source_id, feature.source_feature_id)
                || feature.coordinate_count == 0
                || (feature.marker.is_some() && feature.role != SceneSourceRole::Markers)
                || (feature.cartographic_label.is_some() && feature.role != SceneSourceRole::Label)
        }) {
            return Err(EditorError::SnapshotFeature);
        }
        let expected_coverage = SceneCoverage {
            sources: self.sources.len() as u64,
            features: self.features.len() as u64,
            markers: self
                .features
                .iter()
                .filter(|feature| feature.marker.is_some())
                .count() as u64,
            coordinates: self
                .features
                .iter()
                .map(|feature| feature.coordinate_count)
                .sum(),
        };
        if self.coverage != expected_coverage {
            return Err(EditorError::SnapshotCoverage);
        }
        if source_feature_count != self.coverage.features
            || source_coordinate_count != self.coverage.coordinates
        {
            return Err(EditorError::SnapshotCoverage);
        }
        if !self.complete && self.omissions.is_empty() {
            return Err(EditorError::SnapshotCompleteness);
        }
        let expected = snapshot_identity(self)?;
        if self.snapshot_revision != expected {
            return Err(EditorError::SnapshotIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneObjectKind {
    Source,
    Feature,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneChangeKind {
    Inserted,
    Deleted,
    Modified,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneObjectChange {
    pub object_kind: SceneObjectKind,
    pub object_id: String,
    pub change_kind: SceneChangeKind,
    pub source_revision: Option<SemanticDigest>,
    pub target_revision: Option<SemanticDigest>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneChangeSet {
    pub schema: String,
    pub source_label: String,
    pub target_label: String,
    pub source_revision: Option<SemanticDigest>,
    pub target_revision: Option<SemanticDigest>,
    pub assessment: DeltaAssessment,
    pub inserted: u64,
    pub deleted: u64,
    pub modified: u64,
    pub changes: Vec<SceneObjectChange>,
}

impl SceneChangeSet {
    pub fn derive(
        source_label: &str,
        source: Option<&SceneCandidateSnapshot>,
        target_label: &str,
        target: Option<&SceneCandidateSnapshot>,
    ) -> Self {
        let mut changes = Vec::new();
        diff_objects(
            SceneObjectKind::Source,
            source.map(source_revisions).unwrap_or_default(),
            target.map(source_revisions).unwrap_or_default(),
            &mut changes,
        );
        diff_objects(
            SceneObjectKind::Feature,
            source.map(feature_revisions).unwrap_or_default(),
            target.map(feature_revisions).unwrap_or_default(),
            &mut changes,
        );
        changes.sort_by(|left, right| {
            left.object_kind
                .cmp(&right.object_kind)
                .then_with(|| left.object_id.cmp(&right.object_id))
        });
        let inserted = changes
            .iter()
            .filter(|change| change.change_kind == SceneChangeKind::Inserted)
            .count() as u64;
        let deleted = changes
            .iter()
            .filter(|change| change.change_kind == SceneChangeKind::Deleted)
            .count() as u64;
        let modified = changes
            .iter()
            .filter(|change| change.change_kind == SceneChangeKind::Modified)
            .count() as u64;
        Self {
            schema: SCENE_CHANGE_SET_SCHEMA.to_owned(),
            source_label: source_label.to_owned(),
            target_label: target_label.to_owned(),
            source_revision: source.map(|snapshot| snapshot.snapshot_revision.clone()),
            target_revision: target.map(|snapshot| snapshot.snapshot_revision.clone()),
            assessment: if changes.is_empty() {
                DeltaAssessment::Equal
            } else {
                DeltaAssessment::Different
            },
            inserted,
            deleted,
            modified,
            changes,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorWorkingState {
    Clean,
    Working,
    Staged,
    Mixed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenePackageReference {
    pub package_id: SemanticDigest,
    pub snapshot_revision: SemanticDigest,
    pub package_path: String,
    pub admission_request_path: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneCommit {
    pub schema: String,
    pub commit_id: SemanticDigest,
    pub sequence: u64,
    pub parent_commit_id: Option<SemanticDigest>,
    pub committed_at_unix: i64,
    pub message: String,
    pub package: ScenePackageReference,
}

impl SceneCommit {
    fn new(
        sequence: u64,
        parent_commit_id: Option<SemanticDigest>,
        message: String,
        package: ScenePackageReference,
    ) -> Result<Self, EditorError> {
        let message = normalize_commit_message(message)?;
        let committed_at_unix = Utc::now().timestamp();
        validate_commit_timestamp(committed_at_unix)?;
        let commit_id = scene_commit_identity(
            sequence,
            parent_commit_id.as_ref(),
            committed_at_unix,
            &message,
            &package,
        );
        let commit = Self {
            schema: SCENE_COMMIT_SCHEMA.to_owned(),
            commit_id,
            sequence,
            parent_commit_id,
            committed_at_unix,
            message,
            package,
        };
        commit.verify()?;
        Ok(commit)
    }

    fn verify(&self) -> Result<(), EditorError> {
        if self.schema != SCENE_COMMIT_SCHEMA {
            return Err(EditorError::Schema {
                expected: SCENE_COMMIT_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        if self.sequence == 0 {
            return Err(EditorError::CommitSequence {
                expected: 1,
                actual: self.sequence,
            });
        }
        validate_commit_timestamp(self.committed_at_unix)?;
        if normalize_commit_message(self.message.clone())? != self.message {
            return Err(EditorError::NonCanonicalCommitMessage);
        }
        let expected = scene_commit_identity(
            self.sequence,
            self.parent_commit_id.as_ref(),
            self.committed_at_unix,
            &self.message,
            &self.package,
        );
        if self.commit_id != expected {
            return Err(EditorError::CommitIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EditorStatus {
    pub schema: String,
    pub initialized: bool,
    pub state: EditorWorkingState,
    pub head: Option<SceneCommit>,
    pub index: Option<SceneCandidateSnapshot>,
    pub working: Option<SceneCandidateSnapshot>,
    pub staged: SceneChangeSet,
    pub unstaged: SceneChangeSet,
    pub admission_boundary: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EditorAddResult {
    pub schema: String,
    pub staged: bool,
    pub snapshot: SceneCandidateSnapshot,
    pub delta: SceneChangeSet,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenePackage {
    pub schema: String,
    pub package_id: SemanticDigest,
    pub parent_package_id: Option<SemanticDigest>,
    pub snapshot: SceneCandidateSnapshot,
    pub change_set: SceneChangeSet,
    pub admission_authority: String,
}

impl ScenePackage {
    pub fn verify(&self) -> Result<(), EditorError> {
        if self.schema != SCENE_PACKAGE_SCHEMA {
            return Err(EditorError::Schema {
                expected: SCENE_PACKAGE_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        self.snapshot.verify()?;
        if self.admission_authority != "candidate_only" {
            return Err(EditorError::PackageAuthority);
        }
        if self.package_id != package_identity(self) {
            return Err(EditorError::PackageIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneAdmissionRequest {
    pub schema: String,
    pub request_id: SemanticDigest,
    pub package_id: SemanticDigest,
    pub package_path: String,
    pub requested_operation: String,
    pub status: String,
    pub admitted: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EditorCommitResult {
    pub schema: String,
    pub commit: SceneCommit,
    pub package: ScenePackage,
    pub admission_request: SceneAdmissionRequest,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EditorLogEntry {
    pub commit: SceneCommit,
    pub package: ScenePackage,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EditorLog {
    pub schema: String,
    pub head_commit_id: Option<SemanticDigest>,
    pub total_commits: u64,
    pub selected_commits: u64,
    pub patch: bool,
    pub entries: Vec<EditorLogEntry>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneTerrainGenerationParameters {
    pub feature_count: u64,
    pub vertices: u64,
    pub scale_min: f64,
    pub scale_max: f64,
    pub uplift_ratio: f64,
    pub strength: f64,
    pub strength_jitter: f64,
    pub roughness: f64,
    pub roughness_jitter: f64,
    pub anisotropy: f64,
    pub orientation_degrees: f64,
    pub orientation_jitter_degrees: f64,
    pub edge_jitter: f64,
    pub falloff: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneGenerationRecipe {
    pub schema: String,
    pub generator: String,
    pub source_id: String,
    pub seed: u64,
    pub bounds: SceneBounds,
    pub parameters: SceneTerrainGenerationParameters,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EditorGenerateResult {
    pub schema: String,
    pub changed: bool,
    pub project_created: bool,
    pub project_path: String,
    pub output_path: String,
    pub source: SceneSourceDeclaration,
    pub recipe: SceneGenerationRecipe,
    pub feature_count: u64,
    pub coordinate_count: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EditorSourceAddResult {
    pub schema: String,
    pub changed: bool,
    pub project_created: bool,
    pub project_path: String,
    pub source: SceneSourceDeclaration,
    pub source_revision: SemanticDigest,
    pub source_bytes: u64,
    pub feature_count: u64,
    pub coordinate_count: u64,
    pub native_bounds: Option<SceneBounds>,
    pub authority: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct EditorStateDocument {
    schema: String,
    commits: Vec<SceneCommit>,
    index: Option<SceneCandidateSnapshot>,
}

impl Default for EditorStateDocument {
    fn default() -> Self {
        Self {
            schema: EDITOR_STATE_SCHEMA.to_owned(),
            commits: Vec::new(),
            index: None,
        }
    }
}

#[derive(Debug)]
struct ObservedScene {
    snapshot: SceneCandidateSnapshot,
    artifacts: BTreeMap<String, Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct LocalEditorStore {
    workspace: PathBuf,
    directory: PathBuf,
}

impl LocalEditorStore {
    #[must_use]
    pub fn new(workspace: PathBuf, directory: PathBuf) -> Self {
        Self {
            workspace,
            directory,
        }
    }

    #[must_use]
    pub fn default_for_workspace(workspace: &Path) -> Self {
        Self::new(workspace.to_owned(), workspace.join(".rey").join("editor"))
    }

    fn init_project(&self, project_id: String) -> Result<EditorProject, EditorError> {
        let project = EditorProject::new(project_id)?.canonicalize()?;
        self.prepare_directory()?;
        let path = self.directory.join(PROJECT_FILE_NAME);
        let bytes = serde_json::to_vec_pretty(&project)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| EditorError::Write {
                path: path.clone(),
                source,
            })?;
        file.write_all(&bytes)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.flush())
            .map_err(|source| EditorError::Write { path, source })?;
        Ok(project)
    }

    pub fn add_source(
        &self,
        source_path: &Path,
        scene_id: Option<String>,
        source_id: String,
        role: SceneSourceRole,
    ) -> Result<EditorSourceAddResult, EditorError> {
        validate_identifier("source id", &source_id)?;
        if let Some(scene_id) = &scene_id {
            validate_identifier("scene id", scene_id)?;
        }
        let relative = workspace_relative(&self.workspace, source_path, MAX_SOURCE_BYTES)?;
        if relative
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("geojson")
        {
            return Err(EditorError::SourceExtension(relative));
        }
        let source_path = path_string(&relative)?;
        let bytes = read_bounded_file(
            &self.workspace.join(&relative),
            MAX_SOURCE_BYTES,
            "scene source",
        )?;
        let parsed = parse_geojson(&source_id, role, &bytes)?;
        let source_revision = content_identity(&bytes);
        let source = SceneSourceDeclaration {
            source_id: source_id.clone(),
            path: source_path.clone(),
            format: SceneSourceFormat::GeoJson,
            role,
        };
        self.with_lock(|| {
            let (project_file, mut project, project_created) =
                match self.load_optional_project()? {
                    Some((path, project)) => (path, project, false),
                    None => {
                        let project = EditorProject::new(
                            scene_id.clone().unwrap_or_else(|| source_id.clone()),
                        )?;
                        (
                            self.directory.join(PROJECT_FILE_NAME),
                            project,
                            true,
                        )
                    }
                };
            if scene_id
                .as_ref()
                .is_some_and(|scene_id| scene_id != &project.project_id)
            {
                return Err(EditorError::ProjectIdentity {
                    expected: project.project_id,
                    actual: scene_id.expect("scene id was present"),
                });
            }
            if let Some(existing) = project
                .sources
                .iter()
                .find(|existing| existing.source_id == source_id)
                && existing != &source
            {
                return Err(EditorError::DuplicateSourceId(source_id));
            }
            if let Some(existing) = project
                .sources
                .iter()
                .find(|existing| existing.path == source_path)
                && existing != &source
            {
                return Err(EditorError::DuplicateSourcePath(source_path));
            }
            let changed = !project.sources.iter().any(|existing| existing == &source);
            if changed {
                project.sources.push(source.clone());
                project = project.canonicalize()?;
                self.write_project(&project_file, &project)?;
            }
            Ok(EditorSourceAddResult {
                schema: EDITOR_SOURCE_ADD_RESULT_SCHEMA.to_owned(),
                changed,
                project_created,
                project_path: self.project_storage_path()?,
                source: source.clone(),
                source_revision: source_revision.clone(),
                source_bytes: bytes.len() as u64,
                feature_count: parsed.features.len() as u64,
                coordinate_count: parsed.coordinate_count,
                native_bounds: parsed.bounds.clone(),
                authority: "verified native GeoJSON registered in editor WORKING only; the source remains mutable until `rey editor add` freezes its exact bytes and identity in INDEX".to_owned(),
            })
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn generate_terrain(
        &self,
        output_path: &Path,
        scene_id: Option<String>,
        source_id: String,
        seed: u64,
        bounds: SceneBounds,
        parameters: SceneTerrainGenerationParameters,
    ) -> Result<EditorGenerateResult, EditorError> {
        validate_identifier("source id", &source_id)?;
        if let Some(scene_id) = &scene_id {
            validate_identifier("scene id", scene_id)?;
        }
        validate_generation_bounds(&bounds)?;
        validate_generation_parameters(&parameters)?;
        validate_path_argument(output_path)?;
        if output_path
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("geojson")
        {
            return Err(EditorError::GeneratedOutputExtension(
                output_path.to_owned(),
            ));
        }

        let (project_file, mut project, project_created) = match self.load_optional_project()? {
            Some((path, project)) => (path, project, false),
            None => {
                let project =
                    self.init_project(scene_id.clone().unwrap_or_else(|| source_id.clone()))?;
                (self.directory.join(PROJECT_FILE_NAME), project, true)
            }
        };
        if scene_id
            .as_ref()
            .is_some_and(|scene_id| scene_id != &project.project_id)
        {
            return Err(EditorError::ProjectIdentity {
                expected: project.project_id,
                actual: scene_id.expect("scene id was present"),
            });
        }
        let output_file = self.resolve_new_workspace_path(output_path)?;
        let output = path_string(output_path)?;
        let source = SceneSourceDeclaration {
            source_id: source_id.clone(),
            path: output.clone(),
            format: SceneSourceFormat::GeoJson,
            role: SceneSourceRole::TerrainControl,
        };
        if let Some(existing) = project
            .sources
            .iter()
            .find(|existing| existing.source_id == source_id)
            && existing != &source
        {
            return Err(EditorError::DuplicateSourceId(source_id));
        }
        if let Some(existing) = project
            .sources
            .iter()
            .find(|existing| existing.path == output)
            && existing != &source
        {
            return Err(EditorError::DuplicateSourcePath(output));
        }

        let recipe = SceneGenerationRecipe {
            schema: SCENE_GENERATION_SCHEMA.to_owned(),
            generator: "rey.editor.terrain-controls@1".to_owned(),
            source_id: source_id.clone(),
            seed,
            bounds,
            parameters,
        };
        let bytes = generate_terrain_geojson(&recipe)?;
        let parsed = parse_geojson(&source_id, SceneSourceRole::TerrainControl, &bytes)?;
        let existing_bytes = match fs::symlink_metadata(&output_file) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(EditorError::UnsafePath(output_file));
                }
                let bytes =
                    read_bounded_file(&output_file, MAX_SOURCE_BYTES, "generated scene source")?;
                verify_generated_output(&bytes, &source_id)?;
                Some(bytes)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(source) => {
                return Err(EditorError::Read {
                    path: output_file,
                    source,
                });
            }
        };
        let source_changed = existing_bytes.as_deref().map(strip_final_newline) != Some(&bytes);
        if source_changed {
            write_atomic(&output_file, &bytes)?;
        }
        let project_changed = !project.sources.iter().any(|existing| existing == &source);
        if project_changed {
            project.sources.push(source.clone());
            project = project.canonicalize()?;
            self.write_project(&project_file, &project)?;
        }
        Ok(EditorGenerateResult {
            schema: EDITOR_GENERATE_RESULT_SCHEMA.to_owned(),
            changed: source_changed || project_changed,
            project_created,
            project_path: self.project_storage_path()?,
            output_path: output,
            source,
            recipe,
            feature_count: parsed.features.len() as u64,
            coordinate_count: parsed.coordinate_count,
        })
    }

    pub fn status(&self) -> Result<EditorStatus, EditorError> {
        let state = self.load_state()?;
        let head = state.commits.last().cloned();
        let initialized = self.project_exists()?;
        if !initialized && (head.is_some() || state.index.is_some()) {
            return Err(EditorError::MissingProject(
                self.directory.join(PROJECT_FILE_NAME),
            ));
        }
        let working = if initialized {
            Some(self.observe()?.snapshot)
        } else {
            None
        };
        let package = self.load_commit_package(head.as_ref())?;
        let head_snapshot = package.as_ref().map(|package| &package.snapshot);
        let staged = SceneChangeSet::derive(
            "HEAD",
            head_snapshot,
            "INDEX",
            state.index.as_ref().or(head_snapshot),
        );
        let unstaged = SceneChangeSet::derive(
            "INDEX",
            state.index.as_ref().or(head_snapshot),
            "WORKING",
            working.as_ref(),
        );
        let state_kind = match (
            staged.assessment == DeltaAssessment::Different,
            unstaged.assessment == DeltaAssessment::Different,
        ) {
            (false, false) => EditorWorkingState::Clean,
            (false, true) => EditorWorkingState::Working,
            (true, false) => EditorWorkingState::Staged,
            (true, true) => EditorWorkingState::Mixed,
        };
        Ok(EditorStatus {
            schema: EDITOR_STATUS_SCHEMA.to_owned(),
            initialized,
            state: state_kind,
            head,
            index: state.index,
            working,
            staged,
            unstaged,
            admission_boundary: "scene commits retain candidate only packages; no scene package is admitted until a validated rey.scene-admission workload retains an admitted projection".to_owned(),
        })
    }

    pub fn diff(&self, staged: bool) -> Result<SceneChangeSet, EditorError> {
        let status = self.status()?;
        Ok(if staged {
            status.staged
        } else {
            status.unstaged
        })
    }

    pub fn add(&self) -> Result<EditorAddResult, EditorError> {
        self.with_lock(|| {
            let observed = self.observe()?;
            let mut state = self.load_state()?;
            let package = self.load_commit_package(state.commits.last())?;
            let head_snapshot = package.as_ref().map(|package| &package.snapshot);
            let delta =
                SceneChangeSet::derive("HEAD", head_snapshot, "INDEX", Some(&observed.snapshot));
            self.write_artifacts(&observed)?;
            let staged = state.index.as_ref().or(head_snapshot) != Some(&observed.snapshot);
            state.index = if head_snapshot == Some(&observed.snapshot) {
                None
            } else {
                Some(observed.snapshot.clone())
            };
            self.save_state(&state)?;
            Ok(EditorAddResult {
                schema: EDITOR_ADD_RESULT_SCHEMA.to_owned(),
                staged,
                snapshot: observed.snapshot,
                delta,
            })
        })
    }

    pub fn commit(&self, message: String) -> Result<EditorCommitResult, EditorError> {
        let message = normalize_commit_message(message)?;
        self.with_lock(|| {
            let mut state = self.load_state()?;
            let snapshot = state.index.clone().ok_or(EditorError::EmptyIndex)?;
            snapshot.verify()?;
            self.verify_staged_artifacts(&snapshot)?;
            if state.commits.len() >= MAX_SCENE_COMMITS {
                return Err(EditorError::CommitLimit(MAX_SCENE_COMMITS));
            }
            let head = state.commits.last().cloned();
            let parent = self.load_commit_package(head.as_ref())?;
            let parent_snapshot = parent.as_ref().map(|package| &package.snapshot);
            if parent_snapshot == Some(&snapshot) {
                return Err(EditorError::NothingToCommit);
            }
            let sequence = state.commits.len() as u64 + 1;
            let change_set = SceneChangeSet::derive(
                if sequence == 1 { "EMPTY" } else { "HEAD" },
                parent_snapshot,
                &format!("SCENE@{sequence}"),
                Some(&snapshot),
            );
            let mut package = ScenePackage {
                schema: SCENE_PACKAGE_SCHEMA.to_owned(),
                package_id: digest_placeholder(),
                parent_package_id: parent.map(|package| package.package_id),
                snapshot,
                change_set,
                admission_authority: "candidate_only".to_owned(),
            };
            package.package_id = package_identity(&package);
            package.verify()?;
            let package_relative = format!("packages/{}.json", digest_key(&package.package_id));
            let package_path = self.directory.join(&package_relative);
            write_content_addressed_json(&package_path, &package)?;
            let mut request = SceneAdmissionRequest {
                schema: SCENE_ADMISSION_REQUEST_SCHEMA.to_owned(),
                request_id: digest_placeholder(),
                package_id: package.package_id.clone(),
                package_path: package_relative.clone(),
                requested_operation: SCENE_ADMISSION_REQUESTED_OPERATION.to_owned(),
                status: "requires_workload".to_owned(),
                admitted: false,
            };
            request.request_id = admission_request_identity(&request);
            let request_relative = format!("requests/{}.json", digest_key(&request.request_id));
            write_content_addressed_json(&self.directory.join(&request_relative), &request)?;
            let reference = ScenePackageReference {
                package_id: package.package_id.clone(),
                snapshot_revision: package.snapshot.snapshot_revision.clone(),
                package_path: package_relative,
                admission_request_path: request_relative,
            };
            let commit = SceneCommit::new(
                sequence,
                head.map(|head| head.commit_id),
                message,
                reference,
            )?;
            state.commits.push(commit.clone());
            state.index = None;
            self.save_state(&state)?;
            Ok(EditorCommitResult {
                schema: EDITOR_COMMIT_RESULT_SCHEMA.to_owned(),
                commit,
                package,
                admission_request: request,
            })
        })
    }

    pub fn log(&self, max_count: usize, patch: bool) -> Result<EditorLog, EditorError> {
        if max_count == 0 || max_count > MAX_SCENE_COMMITS {
            return Err(EditorError::LogLimit {
                limit: MAX_SCENE_COMMITS,
                actual: max_count,
            });
        }
        let state = self.load_state()?;
        let mut entries = Vec::with_capacity(max_count.min(state.commits.len()));
        for commit in state.commits.iter().rev().take(max_count) {
            let package = self.load_commit_package(Some(commit))?.ok_or_else(|| {
                EditorError::UnknownPackage(commit.package.package_id.to_string())
            })?;
            entries.push(EditorLogEntry {
                commit: commit.clone(),
                package,
            });
        }
        Ok(EditorLog {
            schema: EDITOR_LOG_SCHEMA.to_owned(),
            head_commit_id: state.commits.last().map(|commit| commit.commit_id.clone()),
            total_commits: state.commits.len() as u64,
            selected_commits: entries.len() as u64,
            patch,
            entries,
        })
    }

    pub fn admission_candidate(
        &self,
        sequence: u64,
    ) -> Result<SceneAdmissionCandidate, EditorError> {
        let state = self.load_state()?;
        let latest_editor_sequence = state.commits.last().map_or(0, |commit| commit.sequence);
        let commit = state
            .commits
            .iter()
            .find(|commit| commit.sequence == sequence)
            .ok_or(EditorError::UnknownSceneSequence(sequence))?;
        let package = self
            .load_commit_package(Some(commit))?
            .ok_or_else(|| EditorError::UnknownPackage(commit.package.package_id.to_string()))?;
        let request = self.load_admission_request(&commit.package.admission_request_path)?;
        let bounds = package
            .snapshot
            .bounds
            .as_ref()
            .ok_or(EditorError::EmptySceneBounds)?;
        let native_bounds = regional_bounds(bounds);
        let sources = package
            .snapshot
            .sources
            .iter()
            .map(|source| {
                let object_path = self.safe_store_path(&source.artifact.object_path)?;
                let native_bytes =
                    read_bounded_file(&object_path, MAX_SOURCE_BYTES, "committed scene object")?;
                Ok(SceneAdmissionSource {
                    source_id: source.source_id.clone(),
                    worktree_path: source.worktree_path.clone(),
                    format: match source.format {
                        SceneSourceFormat::GeoJson => "geo_json".to_owned(),
                    },
                    role: source.role.label().to_owned(),
                    media_type: source.artifact.media_type.clone(),
                    artifact_id: source.artifact.content_digest.clone(),
                    artifact_path: source.artifact.object_path.clone(),
                    declared_bytes: source.artifact.bytes,
                    native_bytes: Some(native_bytes),
                    feature_count: source.feature_count,
                    coordinate_count: source.coordinate_count,
                })
            })
            .collect::<Result<Vec<_>, EditorError>>()?;
        let features = package
            .snapshot
            .features
            .iter()
            .map(|feature| SceneAdmissionFeature {
                feature_id: feature.feature_id.clone(),
                source_id: feature.source_id.clone(),
                source_feature_id: feature.source_feature_id.clone(),
                role: feature.role.label().to_owned(),
                geometry_kind: feature.geometry_kind.clone(),
                native_bounds: regional_bounds(&feature.bounds),
                coordinate_count: feature.coordinate_count,
                properties_digest: feature.properties_digest.clone(),
                feature_revision: feature.feature_revision.clone(),
                cartographic_label: feature
                    .marker
                    .as_ref()
                    .or(feature.cartographic_label.as_ref())
                    .map(|label| rey_runtime::SceneAdmissionCartographicLabel {
                        title: label.title.clone(),
                        category: label.category.clone(),
                        symbol: label.symbol.clone(),
                        min_zoom: label.min_zoom,
                        max_zoom: label.max_zoom,
                        collision_priority: label.collision_priority,
                    }),
                terrain_sample: feature.terrain_sample.as_ref().map(|sample| {
                    rey_runtime::SceneAdmissionTerrainSample {
                        longitude_microdegrees: sample.longitude_microdegrees,
                        latitude_microdegrees: sample.latitude_microdegrees,
                        elevation_micrometers: sample.elevation_micrometers,
                        material: sample.material.clone(),
                        grid: sample.grid.as_ref().map(|grid| {
                            rey_runtime::SceneAdmissionTerrainGridCell {
                                dataset_id: grid.dataset_id.clone(),
                                column: grid.column,
                                row: grid.row,
                                columns: grid.columns,
                                rows: grid.rows,
                                validity: grid.validity.clone(),
                            }
                        }),
                        packed_grid: sample.packed_grid.as_ref().map(|grid| {
                            rey_runtime::SceneAdmissionTerrainPackedGrid {
                                schema: grid.schema.clone(),
                                dataset_id: grid.dataset_id.clone(),
                                compiler_revision: grid.compiler_revision.clone(),
                                columns: grid.columns,
                                rows: grid.rows,
                                native_bounds_microdegrees: grid.native_bounds_microdegrees,
                                validity_hex: grid.validity_hex.clone(),
                                elevation_centimeters_le_hex: grid
                                    .elevation_centimeters_le_hex
                                    .clone(),
                                material_palette: grid.material_palette.clone(),
                                material_indices_hex: grid.material_indices_hex.clone(),
                            }
                        }),
                    }
                }),
            })
            .collect();
        SceneAdmissionCandidate {
            schema: SCENE_ADMISSION_CANDIDATE_SCHEMA.to_owned(),
            candidate_id: digest_placeholder(),
            editor_commit_id: commit.commit_id.clone(),
            editor_sequence: commit.sequence,
            latest_editor_sequence,
            package_id: package.package_id.clone(),
            parent_package_id: package.parent_package_id.clone(),
            package_snapshot_revision: package.snapshot.snapshot_revision.clone(),
            package_authority: package.admission_authority.clone(),
            admission_request_id: request.request_id,
            admission_request_package_id: request.package_id,
            requested_operation: request.requested_operation,
            request_status: request.status,
            request_admitted: request.admitted,
            project_id: package.snapshot.project_id,
            coordinate_system: SceneAdmissionCoordinateSystem {
                kind: package.snapshot.coordinate_system.kind,
                authority: package.snapshot.coordinate_system.authority,
                code: package.snapshot.coordinate_system.code,
                axis_order: package.snapshot.coordinate_system.axis_order,
            },
            native_bounds,
            sources,
            features,
            complete: package.snapshot.complete,
            omissions: package.snapshot.omissions,
        }
        .finalize()
        .map_err(EditorError::from)
    }

    fn observe(&self) -> Result<ObservedScene, EditorError> {
        let (_, project) = self.load_project()?;
        let mut artifacts = BTreeMap::new();
        let mut sources = Vec::with_capacity(project.sources.len());
        let mut features = Vec::new();
        let mut coordinate_count = 0_u64;
        let mut bounds: Option<SceneBounds> = None;
        for declaration in &project.sources {
            let relative = workspace_relative(
                &self.workspace,
                Path::new(&declaration.path),
                MAX_SOURCE_BYTES,
            )?;
            let path = self.workspace.join(relative);
            let bytes = read_bounded_file(&path, MAX_SOURCE_BYTES, "scene source")?;
            let parsed = match declaration.format {
                SceneSourceFormat::GeoJson => {
                    parse_geojson(&declaration.source_id, declaration.role, &bytes)?
                }
            };
            if features.len().saturating_add(parsed.features.len()) > MAX_FEATURES {
                return Err(EditorError::FeatureLimit(MAX_FEATURES));
            }
            coordinate_count = coordinate_count.saturating_add(parsed.coordinate_count);
            if coordinate_count > MAX_COORDINATES as u64 {
                return Err(EditorError::CoordinateLimit(MAX_COORDINATES));
            }
            if let Some(parsed_bounds) = &parsed.bounds {
                match &mut bounds {
                    Some(bounds) => bounds.merge(parsed_bounds),
                    None => bounds = Some(parsed_bounds.clone()),
                }
            }
            let content_digest = content_identity(&bytes);
            let object_path = format!(
                "objects/{}.{}",
                digest_key(&content_digest),
                declaration.format.extension()
            );
            sources.push(SceneSourceSnapshot {
                source_id: declaration.source_id.clone(),
                worktree_path: declaration.path.clone(),
                format: declaration.format,
                role: declaration.role,
                artifact: SceneNativeArtifact {
                    media_type: declaration.format.media_type().to_owned(),
                    content_digest,
                    bytes: bytes.len() as u64,
                    object_path: object_path.clone(),
                },
                feature_count: parsed.features.len() as u64,
                coordinate_count: parsed.coordinate_count,
            });
            artifacts.insert(object_path, bytes);
            features.extend(parsed.features);
        }
        sources.sort_by(|left, right| left.source_id.cmp(&right.source_id));
        features.sort_by(|left, right| left.feature_id.cmp(&right.feature_id));
        if let Some(duplicate) = features
            .windows(2)
            .find(|pair| pair[0].feature_id == pair[1].feature_id)
        {
            return Err(EditorError::DuplicateFeatureId(
                duplicate[0].feature_id.clone(),
            ));
        }
        let markers = features
            .iter()
            .filter(|feature| feature.marker.is_some())
            .count() as u64;
        let mut snapshot = SceneCandidateSnapshot {
            schema: SCENE_CANDIDATE_SNAPSHOT_SCHEMA.to_owned(),
            project_id: project.project_id,
            snapshot_revision: digest_placeholder(),
            coordinate_system: project.coordinate_system,
            bounds,
            coverage: SceneCoverage {
                sources: sources.len() as u64,
                features: features.len() as u64,
                markers,
                coordinates: coordinate_count,
            },
            sources,
            features,
            limits: SceneLimits::default(),
            complete: true,
            omissions: Vec::new(),
        };
        snapshot.snapshot_revision = snapshot_identity(&snapshot)?;
        snapshot.verify()?;
        Ok(ObservedScene {
            snapshot,
            artifacts,
        })
    }

    fn load_project(&self) -> Result<(PathBuf, EditorProject), EditorError> {
        self.load_optional_project()?.ok_or_else(|| {
            EditorError::UninitializedProject(self.directory.join(PROJECT_FILE_NAME))
        })
    }

    fn load_optional_project(&self) -> Result<Option<(PathBuf, EditorProject)>, EditorError> {
        self.verify_directory_boundary()?;
        let path = self.directory.join(PROJECT_FILE_NAME);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(EditorError::Read { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(EditorError::UnsafePath(path));
        }
        if metadata.len() > MAX_PROJECT_BYTES {
            return Err(EditorError::InputLimit {
                path,
                limit: MAX_PROJECT_BYTES,
            });
        }
        let bytes = read_bounded_file(&path, MAX_PROJECT_BYTES, "editor project")?;
        let project: EditorProject = serde_json::from_slice(&bytes)?;
        let project = project.canonicalize()?;
        Ok(Some((path, project)))
    }

    fn project_exists(&self) -> Result<bool, EditorError> {
        Ok(self.load_optional_project()?.is_some())
    }

    fn project_storage_path(&self) -> Result<String, EditorError> {
        let path = self.directory.join(PROJECT_FILE_NAME);
        match path.strip_prefix(&self.workspace) {
            Ok(relative) => path_string(relative),
            Err(_) => path_string(&path),
        }
    }

    fn resolve_new_workspace_path(&self, workspace_path: &Path) -> Result<PathBuf, EditorError> {
        validate_path_argument(workspace_path)?;
        let path = self.workspace.join(workspace_path);
        let parent = path
            .parent()
            .ok_or_else(|| EditorError::Path(workspace_path.to_owned()))?;
        let canonical_parent = parent.canonicalize().map_err(|source| EditorError::Read {
            path: parent.to_owned(),
            source,
        })?;
        if !canonical_parent.starts_with(&self.workspace) {
            return Err(EditorError::PathEscape(workspace_path.to_owned()));
        }
        Ok(path)
    }

    fn write_project(&self, path: &Path, project: &EditorProject) -> Result<(), EditorError> {
        project.verify()?;
        let bytes = serde_json::to_vec_pretty(project)?;
        write_atomic(path, &bytes)
    }

    fn load_state(&self) -> Result<EditorStateDocument, EditorError> {
        self.verify_directory_boundary()?;
        let path = self.directory.join(STATE_FILE_NAME);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(EditorStateDocument::default());
            }
            Err(source) => return Err(EditorError::Read { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(EditorError::UnsafePath(path));
        }
        let bytes = read_bounded_file(&path, MAX_STATE_BYTES, "editor state")?;
        let state: EditorStateDocument = serde_json::from_slice(&bytes)?;
        if state.schema != EDITOR_STATE_SCHEMA {
            return Err(EditorError::Schema {
                expected: EDITOR_STATE_SCHEMA,
                actual: state.schema,
            });
        }
        if let Some(index) = &state.index {
            index.verify()?;
        }
        verify_commit_history(&state.commits)?;
        Ok(state)
    }

    fn save_state(&self, state: &EditorStateDocument) -> Result<(), EditorError> {
        verify_commit_history(&state.commits)?;
        if let Some(index) = &state.index {
            index.verify()?;
        }
        self.prepare_directory()?;
        let bytes = serde_json::to_vec_pretty(state)?;
        if bytes.len().saturating_add(1) as u64 > MAX_STATE_BYTES {
            return Err(EditorError::StateLimit(MAX_STATE_BYTES));
        }
        write_atomic(&self.directory.join(STATE_FILE_NAME), &bytes)
    }

    fn load_commit_package(
        &self,
        commit: Option<&SceneCommit>,
    ) -> Result<Option<ScenePackage>, EditorError> {
        let Some(commit) = commit else {
            return Ok(None);
        };
        let package = self.load_package_path(&commit.package.package_path)?;
        if package.package_id != commit.package.package_id
            || package.snapshot.snapshot_revision != commit.package.snapshot_revision
        {
            return Err(EditorError::CommitPackageIdentity);
        }
        let request = self.load_admission_request(&commit.package.admission_request_path)?;
        if request.package_id != package.package_id
            || request.package_path != commit.package.package_path
        {
            return Err(EditorError::CommitPackageIdentity);
        }
        Ok(Some(package))
    }

    fn load_package_path(&self, relative: &str) -> Result<ScenePackage, EditorError> {
        let path = self.safe_store_path(relative)?;
        let bytes = read_bounded_file(&path, MAX_STATE_BYTES, "scene package")?;
        let package: ScenePackage = serde_json::from_slice(&bytes)?;
        package.verify()?;
        Ok(package)
    }

    fn load_admission_request(&self, relative: &str) -> Result<SceneAdmissionRequest, EditorError> {
        let path = self.safe_store_path(relative)?;
        let bytes = read_bounded_file(&path, MAX_STATE_BYTES, "scene admission request")?;
        let request: SceneAdmissionRequest = serde_json::from_slice(&bytes)?;
        if request.schema != SCENE_ADMISSION_REQUEST_SCHEMA
            || request.request_id != admission_request_identity(&request)
            || request.status != "requires_workload"
            || request.admitted
        {
            return Err(EditorError::AdmissionRequestIdentity);
        }
        Ok(request)
    }

    fn write_artifacts(&self, observed: &ObservedScene) -> Result<(), EditorError> {
        self.prepare_directory()?;
        for (relative, bytes) in &observed.artifacts {
            let path = self.safe_store_path(relative)?;
            write_content_addressed_bytes(&path, bytes)?;
        }
        Ok(())
    }

    fn verify_staged_artifacts(
        &self,
        snapshot: &SceneCandidateSnapshot,
    ) -> Result<(), EditorError> {
        for source in &snapshot.sources {
            let path = self.safe_store_path(&source.artifact.object_path)?;
            let bytes = read_bounded_file(&path, MAX_SOURCE_BYTES, "staged scene object")?;
            if content_identity(&bytes) != source.artifact.content_digest {
                return Err(EditorError::ArtifactIdentity(source.source_id.clone()));
            }
        }
        Ok(())
    }

    fn with_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, EditorError>,
    ) -> Result<T, EditorError> {
        self.prepare_directory()?;
        let lock_path = self.directory.join(LOCK_FILE_NAME);
        if let Ok(metadata) = fs::symlink_metadata(&lock_path)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(EditorError::UnsafePath(lock_path));
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| EditorError::Write {
                path: lock_path.clone(),
                source,
            })?;
        File::lock(&lock).map_err(|source| EditorError::Lock {
            path: lock_path.clone(),
            source,
        })?;
        let result = operation();
        let unlock = File::unlock(&lock).map_err(|source| EditorError::Lock {
            path: lock_path,
            source,
        });
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    fn prepare_directory(&self) -> Result<(), EditorError> {
        self.verify_directory_boundary()?;
        match fs::symlink_metadata(&self.directory) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                Err(EditorError::UnsafePath(self.directory.clone()))
            }
            Ok(_) => {
                self.prepare_store_child("objects")?;
                self.prepare_store_child("packages")?;
                self.prepare_store_child("requests")?;
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.directory).map_err(|source| EditorError::Write {
                    path: self.directory.clone(),
                    source,
                })?;
                self.prepare_store_child("objects")?;
                self.prepare_store_child("packages")?;
                self.prepare_store_child("requests")?;
                Ok(())
            }
            Err(source) => Err(EditorError::Write {
                path: self.directory.clone(),
                source,
            }),
        }
    }

    fn prepare_store_child(&self, child: &str) -> Result<(), EditorError> {
        let path = self.directory.join(child);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                Err(EditorError::UnsafePath(path))
            }
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&path).map_err(|source| EditorError::Write { path, source })
            }
            Err(source) => Err(EditorError::Write { path, source }),
        }
    }

    fn safe_store_path(&self, relative: &str) -> Result<PathBuf, EditorError> {
        self.verify_directory_boundary()?;
        validate_relative_path(relative)?;
        let mut current = self.directory.clone();
        for component in Path::new(relative).components() {
            let std::path::Component::Normal(component) = component else {
                return Err(EditorError::Path(PathBuf::from(relative)));
            };
            current.push(component);
            match fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(EditorError::UnsafePath(current));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(EditorError::Read {
                        path: current,
                        source,
                    });
                }
            }
        }
        Ok(current)
    }

    fn verify_directory_boundary(&self) -> Result<(), EditorError> {
        for ancestor in self.directory.ancestors() {
            match fs::symlink_metadata(ancestor) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(EditorError::UnsafePath(ancestor.to_owned()));
                }
                Ok(metadata) if ancestor == self.directory && !metadata.is_dir() => {
                    return Err(EditorError::UnsafePath(ancestor.to_owned()));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(EditorError::Read {
                        path: ancestor.to_owned(),
                        source,
                    });
                }
            }
        }
        Ok(())
    }
}

fn validate_generation_bounds(bounds: &SceneBounds) -> Result<(), EditorError> {
    if ![bounds.west, bounds.south, bounds.east, bounds.north]
        .into_iter()
        .all(f64::is_finite)
        || !(-180.0..=180.0).contains(&bounds.west)
        || !(-180.0..=180.0).contains(&bounds.east)
        || !(-90.0..=90.0).contains(&bounds.south)
        || !(-90.0..=90.0).contains(&bounds.north)
        || bounds.west >= bounds.east
        || bounds.south >= bounds.north
    {
        return Err(EditorError::GenerationParameter(
            "bounds require finite ordered CRS84 west/south/east/north values".to_owned(),
        ));
    }
    Ok(())
}

fn validate_generation_parameters(
    parameters: &SceneTerrainGenerationParameters,
) -> Result<(), EditorError> {
    let unit = |name: &str, value: f64| {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(())
        } else {
            Err(EditorError::GenerationParameter(format!(
                "{name} must be finite and within 0..=1"
            )))
        }
    };
    if !(1..=MAX_GENERATED_FEATURES).contains(&parameters.feature_count) {
        return Err(EditorError::GenerationParameter(format!(
            "features must be within 1..={MAX_GENERATED_FEATURES}"
        )));
    }
    if !(3..=32).contains(&parameters.vertices) {
        return Err(EditorError::GenerationParameter(
            "vertices must be within 3..=32".to_owned(),
        ));
    }
    unit("scale_min", parameters.scale_min)?;
    unit("scale_max", parameters.scale_max)?;
    if parameters.scale_min <= 0.0 || parameters.scale_min > parameters.scale_max {
        return Err(EditorError::GenerationParameter(
            "scale_min must be positive and no greater than scale_max".to_owned(),
        ));
    }
    unit("uplift_ratio", parameters.uplift_ratio)?;
    unit("strength", parameters.strength)?;
    unit("strength_jitter", parameters.strength_jitter)?;
    unit("roughness", parameters.roughness)?;
    unit("roughness_jitter", parameters.roughness_jitter)?;
    unit("edge_jitter", parameters.edge_jitter)?;
    if parameters.edge_jitter > 0.5 {
        return Err(EditorError::GenerationParameter(
            "edge_jitter must be within 0..=0.5".to_owned(),
        ));
    }
    if !parameters.anisotropy.is_finite() || !(1.0..=8.0).contains(&parameters.anisotropy) {
        return Err(EditorError::GenerationParameter(
            "anisotropy must be finite and within 1..=8".to_owned(),
        ));
    }
    if parameters.scale_max * parameters.anisotropy.sqrt() * (1.0 + parameters.edge_jitter) >= 0.5 {
        return Err(EditorError::GenerationParameter(
            "scale_max, anisotropy, and edge_jitter exceed the generation bounds".to_owned(),
        ));
    }
    if !parameters.orientation_degrees.is_finite() {
        return Err(EditorError::GenerationParameter(
            "orientation_degrees must be finite".to_owned(),
        ));
    }
    if !parameters.orientation_jitter_degrees.is_finite()
        || !(0.0..=180.0).contains(&parameters.orientation_jitter_degrees)
    {
        return Err(EditorError::GenerationParameter(
            "orientation_jitter_degrees must be finite and within 0..=180".to_owned(),
        ));
    }
    if !parameters.falloff.is_finite() || !(0.1..=16.0).contains(&parameters.falloff) {
        return Err(EditorError::GenerationParameter(
            "falloff must be finite and within 0.1..=16".to_owned(),
        ));
    }
    Ok(())
}

fn generate_terrain_geojson(recipe: &SceneGenerationRecipe) -> Result<Vec<u8>, EditorError> {
    let parameters = &recipe.parameters;
    let width = recipe.bounds.east - recipe.bounds.west;
    let height = recipe.bounds.north - recipe.bounds.south;
    let axis_root = parameters.anisotropy.sqrt();
    let mut random = SplitMix64::new(recipe.seed);
    let mut features = Vec::with_capacity(parameters.feature_count as usize);
    for index in 0..parameters.feature_count {
        let scale = lerp(parameters.scale_min, parameters.scale_max, random.unit());
        let major = scale * axis_root;
        let minor = scale / axis_root;
        let margin = major * (1.0 + parameters.edge_jitter);
        let center_x = lerp(margin, 1.0 - margin, random.unit());
        let center_y = lerp(margin, 1.0 - margin, random.unit());
        let orientation = parameters.orientation_degrees
            + random.signed() * parameters.orientation_jitter_degrees;
        let orientation_radians = orientation.to_radians();
        let orientation_cos = orientation_radians.cos();
        let orientation_sin = orientation_radians.sin();
        let mut ring = Vec::with_capacity(parameters.vertices as usize + 1);
        for vertex in 0..parameters.vertices {
            let angle = std::f64::consts::TAU * vertex as f64 / parameters.vertices as f64;
            let radius = 1.0 + random.signed() * parameters.edge_jitter;
            let local_x = major * angle.cos() * radius;
            let local_y = minor * angle.sin() * radius;
            let normalized_x = center_x + local_x * orientation_cos - local_y * orientation_sin;
            let normalized_y = center_y + local_x * orientation_sin + local_y * orientation_cos;
            ring.push(vec![
                rounded_coordinate(recipe.bounds.west + normalized_x * width),
                rounded_coordinate(recipe.bounds.south + normalized_y * height),
            ]);
        }
        ring.push(ring[0].clone());

        let uplift = random.unit() < parameters.uplift_ratio;
        let strength =
            clamp_unit(parameters.strength + random.signed() * parameters.strength_jitter);
        let roughness =
            clamp_unit(parameters.roughness + random.signed() * parameters.roughness_jitter);
        let effect = if uplift { "uplift" } else { "depression" };
        let relative_elevation = if uplift {
            0.5 + strength * 0.5
        } else {
            0.5 - strength * 0.5
        };
        features.push(serde_json::json!({
            "type": "Feature",
            "id": format!("control-{:04}", index + 1),
            "properties": {
                "name": format!("Generated {} {:04}", if uplift { "uplift" } else { "basin" }, index + 1),
                "terrain_class": format!("generated_{effect}_control"),
                "terrain_effect": effect,
                "effect_strength": rounded_parameter(strength),
                "relative_elevation": rounded_parameter(relative_elevation),
                "roughness": rounded_parameter(roughness),
                "falloff": rounded_parameter(parameters.falloff),
                "anisotropy": rounded_parameter(parameters.anisotropy),
                "orientation_degrees": rounded_parameter(orientation.rem_euclid(180.0)),
                "generator": recipe.generator,
                "generation_seed": recipe.seed,
                "generation_index": index + 1,
                "authority": "generated_candidate_hint"
            },
            "geometry": {"type": "Polygon", "coordinates": [ring]}
        }));
    }
    let document = serde_json::json!({
        "type": "FeatureCollection",
        "name": format!("{} deterministic terrain controls", recipe.source_id),
        "rey_generation": recipe,
        "features": features
    });
    Ok(serde_json::to_vec_pretty(&document)?)
}

fn verify_generated_output(bytes: &[u8], source_id: &str) -> Result<(), EditorError> {
    let document: Value = serde_json::from_slice(bytes)?;
    let recipe: SceneGenerationRecipe = serde_json::from_value(
        document
            .get("rey_generation")
            .cloned()
            .ok_or_else(|| EditorError::GeneratedOutputAuthority(source_id.to_owned()))?,
    )?;
    if recipe.schema != SCENE_GENERATION_SCHEMA
        || recipe.generator != "rey.editor.terrain-controls@1"
        || recipe.source_id != source_id
    {
        return Err(EditorError::GeneratedOutputAuthority(source_id.to_owned()));
    }
    Ok(())
}

fn strip_final_newline(bytes: &[u8]) -> &[u8] {
    bytes.strip_suffix(b"\n").unwrap_or(bytes)
}

fn lerp(start: f64, end: f64, amount: f64) -> f64 {
    start + (end - start) * amount
}

fn clamp_unit(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn rounded_coordinate(value: f64) -> f64 {
    (value * 1_000_000_000.0).round() / 1_000_000_000.0
}

fn rounded_parameter(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1_u64 << 53) as f64
    }

    fn signed(&mut self) -> f64 {
        self.unit() * 2.0 - 1.0
    }
}

#[derive(Debug)]
struct ParsedGeoJson {
    features: Vec<SceneFeatureIndex>,
    coordinate_count: u64,
    bounds: Option<SceneBounds>,
}

fn parse_geojson(
    source_id: &str,
    role: SceneSourceRole,
    bytes: &[u8],
) -> Result<ParsedGeoJson, EditorError> {
    let document: Value = serde_json::from_slice(bytes)?;
    let object = document.as_object().ok_or(EditorError::GeoJsonRoot)?;
    if object.contains_key("crs") {
        return Err(EditorError::GeoJsonCrs);
    }
    let document_type = string_member(object, "type")?;
    let feature_values: Vec<&Value> = match document_type {
        "FeatureCollection" => object
            .get("features")
            .and_then(Value::as_array)
            .ok_or(EditorError::GeoJsonMember("features"))?
            .iter()
            .collect(),
        "Feature" => vec![&document],
        actual => return Err(EditorError::GeoJsonType(actual.to_owned())),
    };
    if feature_values.len() > MAX_FEATURES {
        return Err(EditorError::FeatureLimit(MAX_FEATURES));
    }
    let mut features = Vec::with_capacity(feature_values.len());
    let mut source_coordinates = 0_u64;
    let mut source_bounds: Option<SceneBounds> = None;
    let mut ids = BTreeSet::new();
    for feature in feature_values {
        let feature_object = feature.as_object().ok_or(EditorError::GeoJsonFeature)?;
        if string_member(feature_object, "type")? != "Feature" {
            return Err(EditorError::GeoJsonFeature);
        }
        let source_feature_id = geojson_feature_id(feature_object.get("id"))?;
        if !ids.insert(source_feature_id.clone()) {
            return Err(EditorError::DuplicateFeatureId(format!(
                "{source_id}/{source_feature_id}"
            )));
        }
        let geometry = feature_object
            .get("geometry")
            .and_then(Value::as_object)
            .ok_or_else(|| EditorError::MissingGeometry(source_feature_id.clone()))?;
        let mut geometry_summary = GeometrySummary::default();
        validate_geometry(geometry, &mut geometry_summary)?;
        if matches!(role, SceneSourceRole::Markers | SceneSourceRole::Label)
            && !matches!(geometry_summary.kind.as_str(), "Point" | "MultiPoint")
        {
            return Err(EditorError::MarkerGeometry(source_feature_id));
        }
        source_coordinates = source_coordinates.saturating_add(geometry_summary.coordinates);
        if source_coordinates > MAX_COORDINATES as u64 {
            return Err(EditorError::CoordinateLimit(MAX_COORDINATES));
        }
        let properties = match feature_object.get("properties") {
            Some(Value::Object(properties)) => properties,
            Some(Value::Null) | None => {
                static EMPTY: std::sync::LazyLock<serde_json::Map<String, Value>> =
                    std::sync::LazyLock::new(serde_json::Map::new);
                &EMPTY
            }
            Some(_) => return Err(EditorError::GeoJsonProperties(source_feature_id)),
        };
        if properties.len() > MAX_PROPERTIES {
            return Err(EditorError::PropertyLimit {
                feature_id: source_feature_id,
                limit: MAX_PROPERTIES,
            });
        }
        let property_bytes = serde_json::to_vec(properties)?;
        if property_bytes.len() > MAX_PROPERTIES_BYTES {
            return Err(EditorError::PropertyByteLimit {
                feature_id: source_feature_id,
                limit: MAX_PROPERTIES_BYTES,
            });
        }
        let properties_digest = properties_identity(&property_bytes);
        let marker = if role == SceneSourceRole::Markers {
            Some(marker_index(&source_feature_id, properties)?)
        } else {
            None
        };
        let cartographic_label = if role == SceneSourceRole::Label {
            Some(marker_index(&source_feature_id, properties)?)
        } else {
            None
        };
        let bounds = geometry_summary
            .bounds
            .clone()
            .ok_or_else(|| EditorError::MissingGeometry(source_feature_id.clone()))?;
        let terrain_sample = terrain_sample(
            role,
            &source_feature_id,
            feature_object,
            geometry,
            &geometry_summary.kind,
            &bounds,
            properties,
        )?;
        let feature_id = format!("{source_id}/{source_feature_id}");
        let feature_bytes = serde_json::to_vec(feature)?;
        let feature_revision = feature_identity(source_id, role, &feature_bytes);
        match &mut source_bounds {
            Some(source_bounds) => source_bounds.merge(&bounds),
            None => source_bounds = Some(bounds.clone()),
        }
        features.push(SceneFeatureIndex {
            feature_id,
            source_id: source_id.to_owned(),
            source_feature_id,
            role,
            geometry_kind: geometry_summary.kind,
            bounds,
            coordinate_count: geometry_summary.coordinates,
            properties_digest,
            feature_revision,
            marker,
            cartographic_label,
            terrain_sample,
        });
    }
    features.sort_by(|left, right| left.feature_id.cmp(&right.feature_id));
    Ok(ParsedGeoJson {
        features,
        coordinate_count: source_coordinates,
        bounds: source_bounds,
    })
}

fn terrain_sample(
    role: SceneSourceRole,
    feature_id: &str,
    feature: &serde_json::Map<String, Value>,
    geometry: &serde_json::Map<String, Value>,
    geometry_kind: &str,
    bounds: &SceneBounds,
    properties: &serde_json::Map<String, Value>,
) -> Result<Option<SceneTerrainSample>, EditorError> {
    if role != SceneSourceRole::Terrain {
        return Ok(None);
    }
    if let Some(value) = feature.get("terrain_grid") {
        if properties
            .keys()
            .any(|key| key.starts_with("terrain_grid_"))
        {
            return Err(EditorError::TerrainSample(format!(
                "{feature_id} mixes packed and point terrain grid bindings"
            )));
        }
        let packed_grid = packed_terrain_grid(feature_id, geometry_kind, bounds, value)?;
        return Ok(Some(SceneTerrainSample {
            longitude_microdegrees: packed_grid.native_bounds_microdegrees[0],
            latitude_microdegrees: packed_grid.native_bounds_microdegrees[3],
            elevation_micrometers: None,
            material: None,
            grid: None,
            packed_grid: Some(packed_grid),
        }));
    }
    if geometry_kind != "Point" {
        return Err(EditorError::TerrainSample(format!(
            "{feature_id} requires Point geometry"
        )));
    }
    let coordinates = geometry
        .get("coordinates")
        .and_then(Value::as_array)
        .ok_or_else(|| EditorError::TerrainSample(feature_id.to_owned()))?;
    let grid = terrain_grid_cell(feature_id, properties)?;
    let no_data = grid.as_ref().is_some_and(|grid| grid.validity == "no_data");
    if coordinates.len() != 3 && !(no_data && coordinates.len() == 2) {
        return Err(EditorError::TerrainSample(format!(
            "{feature_id} requires longitude, latitude, and elevation unless it is an explicit grid no-data vertex"
        )));
    }
    let longitude = finite_number(&coordinates[0])?;
    let latitude = finite_number(&coordinates[1])?;
    let elevation = coordinates.get(2).map(finite_number).transpose()?;
    if elevation.is_some_and(|elevation| !(-12_000.0..=100_000.0).contains(&elevation)) {
        return Err(EditorError::TerrainSample(format!(
            "{feature_id} elevation is outside -12000..=100000 meters"
        )));
    }
    let material = properties.get("material").and_then(Value::as_str);
    let valid_material = material.is_some_and(|material| {
        !material.is_empty()
            && material.chars().count() <= 64
            && material.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
            })
    });
    if !no_data && (elevation.is_none() || !valid_material) {
        return Err(EditorError::TerrainSample(format!(
            "{feature_id} requires a bounded elevation and material identifier"
        )));
    }
    if no_data && material.is_some() {
        return Err(EditorError::TerrainSample(format!(
            "{feature_id} no-data vertex must not declare a material"
        )));
    }
    Ok(Some(SceneTerrainSample {
        longitude_microdegrees: (longitude * 1_000_000.0).round() as i64,
        latitude_microdegrees: (latitude * 1_000_000.0).round() as i64,
        elevation_micrometers: elevation.map(|value| (value * 1_000_000.0).round() as i64),
        material: material.map(str::to_owned),
        grid,
        packed_grid: None,
    }))
}

fn packed_terrain_grid(
    feature_id: &str,
    geometry_kind: &str,
    geometry_bounds: &SceneBounds,
    value: &Value,
) -> Result<SceneTerrainPackedGrid, EditorError> {
    const SCHEMA: &str = "rey.packed-terrain-grid.v1";
    const MAX_CELLS: u64 = 1_000_000;
    let grid = serde_json::from_value::<SceneTerrainPackedGrid>(value.clone()).map_err(|_| {
        EditorError::TerrainSample(format!("{feature_id} has an invalid packed terrain grid"))
    })?;
    validate_identifier("terrain grid id", &grid.dataset_id)?;
    let compiler_valid = !grid.compiler_revision.is_empty()
        && grid.compiler_revision.chars().count() <= 128
        && grid.compiler_revision.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '@')
        });
    let cells = grid
        .columns
        .checked_mul(grid.rows)
        .filter(|cells| *cells <= MAX_CELLS)
        .ok_or_else(|| EditorError::TerrainSample(format!("{feature_id} grid is too large")))?;
    let [west, south, east, north] = grid.native_bounds_microdegrees;
    let expected_bounds = [
        (geometry_bounds.west * 1_000_000.0).round() as i64,
        (geometry_bounds.south * 1_000_000.0).round() as i64,
        (geometry_bounds.east * 1_000_000.0).round() as i64,
        (geometry_bounds.north * 1_000_000.0).round() as i64,
    ];
    let longitude_span = east.checked_sub(west);
    let latitude_span = north.checked_sub(south);
    if grid.schema != SCHEMA
        || geometry_kind != "Polygon"
        || !compiler_valid
        || grid.columns < 2
        || grid.rows < 2
        || grid.native_bounds_microdegrees != expected_bounds
        || !(-180_000_000..=180_000_000).contains(&west)
        || !(-180_000_000..=180_000_000).contains(&east)
        || !(-90_000_000..=90_000_000).contains(&south)
        || !(-90_000_000..=90_000_000).contains(&north)
        || longitude_span.is_none_or(|span| span <= 0 || span % (grid.columns as i64 - 1) != 0)
        || latitude_span.is_none_or(|span| span <= 0 || span % (grid.rows as i64 - 1) != 0)
        || grid.material_palette.is_empty()
        || grid.material_palette.len() > 255
        || !grid
            .material_palette
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        || grid
            .material_palette
            .iter()
            .any(|material| validate_identifier("terrain material", material).is_err())
    {
        return Err(EditorError::TerrainSample(format!(
            "{feature_id} packed terrain metadata is invalid"
        )));
    }
    let validity = decode_terrain_hex(feature_id, &grid.validity_hex)?;
    let elevations = decode_terrain_hex(feature_id, &grid.elevation_centimeters_le_hex)?;
    let materials = decode_terrain_hex(feature_id, &grid.material_indices_hex)?;
    let expected = usize::try_from(cells)
        .map_err(|_| EditorError::TerrainSample(format!("{feature_id} grid is too large")))?;
    if validity.len() != expected || elevations.len() != expected * 4 || materials.len() != expected
    {
        return Err(EditorError::TerrainSample(format!(
            "{feature_id} packed terrain channels do not match the declared shape"
        )));
    }
    let mut has_supported_triangle = false;
    for index in 0..expected {
        let elevation = i32::from_le_bytes(
            elevations[index * 4..index * 4 + 4]
                .try_into()
                .map_err(|_| EditorError::TerrainSample(feature_id.to_owned()))?,
        );
        match validity[index] {
            1 if (-1_200_000..=10_000_000).contains(&elevation)
                && usize::from(materials[index]) < grid.material_palette.len() => {}
            0 if elevation == 0 && materials[index] == 255 => {}
            _ => {
                return Err(EditorError::TerrainSample(format!(
                    "{feature_id} packed terrain cell {index} is invalid"
                )));
            }
        }
    }
    let columns = grid.columns as usize;
    let rows = grid.rows as usize;
    for row in 0..rows - 1 {
        for column in 0..columns - 1 {
            let top_left = row * columns + column;
            let top_right = top_left + 1;
            let bottom_left = top_left + columns;
            let bottom_right = bottom_left + 1;
            let valid = |index: usize| validity[index] == 1;
            if (valid(top_left) && valid(bottom_left) && valid(bottom_right))
                || (valid(top_left) && valid(bottom_right) && valid(top_right))
                || (valid(top_left) && valid(bottom_left) && valid(top_right))
                || (valid(top_right) && valid(bottom_left) && valid(bottom_right))
            {
                has_supported_triangle = true;
                break;
            }
        }
        if has_supported_triangle {
            break;
        }
    }
    if !has_supported_triangle {
        return Err(EditorError::TerrainSample(format!(
            "{feature_id} packed terrain has no supported triangle"
        )));
    }
    Ok(grid)
}

fn decode_terrain_hex(feature_id: &str, encoded: &str) -> Result<Vec<u8>, EditorError> {
    if !encoded.len().is_multiple_of(2) {
        return Err(EditorError::TerrainSample(format!(
            "{feature_id} packed terrain channel is not canonical hexadecimal"
        )));
    }
    encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                _ => None,
            };
            digit(pair[0])
                .zip(digit(pair[1]))
                .map(|(high, low)| high * 16 + low)
                .ok_or_else(|| {
                    EditorError::TerrainSample(format!(
                        "{feature_id} packed terrain channel is not canonical hexadecimal"
                    ))
                })
        })
        .collect()
}

fn terrain_grid_cell(
    feature_id: &str,
    properties: &serde_json::Map<String, Value>,
) -> Result<Option<SceneTerrainGridCell>, EditorError> {
    const KEYS: [&str; 6] = [
        "terrain_grid_id",
        "terrain_grid_column",
        "terrain_grid_row",
        "terrain_grid_columns",
        "terrain_grid_rows",
        "terrain_grid_validity",
    ];
    if !KEYS.iter().any(|key| properties.contains_key(*key)) {
        return Ok(None);
    }
    if KEYS.iter().any(|key| !properties.contains_key(*key)) {
        return Err(EditorError::TerrainSample(format!(
            "{feature_id} has an incomplete terrain grid binding"
        )));
    }
    let dataset_id = properties["terrain_grid_id"]
        .as_str()
        .ok_or_else(|| EditorError::TerrainSample(feature_id.to_owned()))?;
    validate_identifier("terrain grid id", dataset_id)?;
    let integer = |key: &'static str| {
        properties[key]
            .as_u64()
            .ok_or_else(|| EditorError::TerrainSample(format!("{feature_id} has invalid {key}")))
    };
    let column = integer("terrain_grid_column")?;
    let row = integer("terrain_grid_row")?;
    let columns = integer("terrain_grid_columns")?;
    let rows = integer("terrain_grid_rows")?;
    let validity = properties["terrain_grid_validity"]
        .as_str()
        .filter(|validity| matches!(*validity, "valid" | "no_data"))
        .ok_or_else(|| {
            EditorError::TerrainSample(format!(
                "{feature_id} terrain_grid_validity must be valid or no_data"
            ))
        })?;
    if columns < 2 || rows < 2 || column >= columns || row >= rows {
        return Err(EditorError::TerrainSample(format!(
            "{feature_id} has an out-of-bounds terrain grid position"
        )));
    }
    Ok(Some(SceneTerrainGridCell {
        dataset_id: dataset_id.to_owned(),
        column,
        row,
        columns,
        rows,
        validity: validity.to_owned(),
    }))
}

#[derive(Debug, Default)]
struct GeometrySummary {
    kind: String,
    coordinates: u64,
    bounds: Option<SceneBounds>,
}

fn validate_geometry(
    geometry: &serde_json::Map<String, Value>,
    summary: &mut GeometrySummary,
) -> Result<(), EditorError> {
    let kind = string_member(geometry, "type")?;
    summary.kind = kind.to_owned();
    match kind {
        "Point" => validate_coordinates_member(geometry, 0, summary),
        "MultiPoint" => validate_coordinates_member(geometry, 1, summary),
        "LineString" => {
            validate_sequence(geometry, "LineString", 2, false)?;
            validate_coordinates_member(geometry, 1, summary)
        }
        "MultiLineString" => {
            validate_nested_sequences(geometry, "MultiLineString", 2, false)?;
            validate_coordinates_member(geometry, 2, summary)
        }
        "Polygon" => {
            validate_nested_sequences(geometry, "Polygon", 4, true)?;
            validate_coordinates_member(geometry, 2, summary)
        }
        "MultiPolygon" => {
            let polygons = geometry
                .get("coordinates")
                .and_then(Value::as_array)
                .ok_or(EditorError::GeoJsonMember("coordinates"))?;
            if polygons.is_empty() {
                return Err(EditorError::EmptyGeometry);
            }
            for polygon in polygons {
                validate_sequences_value(polygon, "MultiPolygon", 4, true)?;
            }
            validate_coordinates_member(geometry, 3, summary)
        }
        "GeometryCollection" => {
            let geometries = geometry
                .get("geometries")
                .and_then(Value::as_array)
                .ok_or(EditorError::GeoJsonMember("geometries"))?;
            if geometries.is_empty() {
                return Err(EditorError::EmptyGeometry);
            }
            for nested in geometries {
                let nested = nested.as_object().ok_or(EditorError::GeoJsonFeature)?;
                let mut nested_summary = GeometrySummary::default();
                validate_geometry(nested, &mut nested_summary)?;
                summary.coordinates = summary
                    .coordinates
                    .saturating_add(nested_summary.coordinates);
                if let Some(nested_bounds) = nested_summary.bounds {
                    match &mut summary.bounds {
                        Some(bounds) => bounds.merge(&nested_bounds),
                        None => summary.bounds = Some(nested_bounds),
                    }
                }
            }
            Ok(())
        }
        actual => Err(EditorError::GeometryType(actual.to_owned())),
    }
}

fn validate_sequence(
    geometry: &serde_json::Map<String, Value>,
    kind: &'static str,
    minimum: usize,
    closed: bool,
) -> Result<(), EditorError> {
    let sequence = geometry
        .get("coordinates")
        .and_then(Value::as_array)
        .ok_or(EditorError::GeoJsonMember("coordinates"))?;
    validate_sequence_values(sequence, kind, minimum, closed)
}

fn validate_nested_sequences(
    geometry: &serde_json::Map<String, Value>,
    kind: &'static str,
    minimum: usize,
    closed: bool,
) -> Result<(), EditorError> {
    let sequences = geometry
        .get("coordinates")
        .ok_or(EditorError::GeoJsonMember("coordinates"))?;
    validate_sequences_value(sequences, kind, minimum, closed)
}

fn validate_sequences_value(
    value: &Value,
    kind: &'static str,
    minimum: usize,
    closed: bool,
) -> Result<(), EditorError> {
    let sequences = value.as_array().ok_or(EditorError::InvalidCoordinates)?;
    if sequences.is_empty() {
        return Err(EditorError::EmptyGeometry);
    }
    for sequence in sequences {
        let sequence = sequence.as_array().ok_or(EditorError::InvalidCoordinates)?;
        validate_sequence_values(sequence, kind, minimum, closed)?;
    }
    Ok(())
}

fn validate_sequence_values(
    sequence: &[Value],
    kind: &'static str,
    minimum: usize,
    closed: bool,
) -> Result<(), EditorError> {
    if sequence.len() < minimum {
        return Err(EditorError::GeometryMinimum {
            kind,
            minimum,
            actual: sequence.len(),
        });
    }
    if closed && sequence.first() != sequence.last() {
        return Err(EditorError::PolygonRing);
    }
    Ok(())
}

fn validate_coordinates_member(
    geometry: &serde_json::Map<String, Value>,
    nesting: usize,
    summary: &mut GeometrySummary,
) -> Result<(), EditorError> {
    let coordinates = geometry
        .get("coordinates")
        .ok_or(EditorError::GeoJsonMember("coordinates"))?;
    validate_coordinates(coordinates, nesting, summary)
}

fn validate_coordinates(
    value: &Value,
    nesting: usize,
    summary: &mut GeometrySummary,
) -> Result<(), EditorError> {
    let values = value.as_array().ok_or(EditorError::InvalidCoordinates)?;
    if values.is_empty() {
        return Err(EditorError::EmptyGeometry);
    }
    if nesting > 0 {
        for nested in values {
            validate_coordinates(nested, nesting - 1, summary)?;
        }
        return Ok(());
    }
    if !(2..=3).contains(&values.len()) {
        return Err(EditorError::PositionDimensions(values.len()));
    }
    let longitude = finite_number(&values[0])?;
    let latitude = finite_number(&values[1])?;
    if !(-180.0..=180.0).contains(&longitude) || !(-90.0..=90.0).contains(&latitude) {
        return Err(EditorError::CoordinateRange {
            longitude,
            latitude,
        });
    }
    if let Some(altitude) = values.get(2) {
        let _ = finite_number(altitude)?;
    }
    summary.coordinates = summary.coordinates.saturating_add(1);
    if summary.coordinates > MAX_COORDINATES as u64 {
        return Err(EditorError::CoordinateLimit(MAX_COORDINATES));
    }
    match &mut summary.bounds {
        Some(bounds) => bounds.include(longitude, latitude),
        None => {
            summary.bounds = Some(SceneBounds {
                west: longitude,
                south: latitude,
                east: longitude,
                north: latitude,
            });
        }
    }
    Ok(())
}

fn marker_index(
    feature_id: &str,
    properties: &serde_json::Map<String, Value>,
) -> Result<SceneMarkerIndex, EditorError> {
    let title = optional_string_property(properties, "title")?
        .or(optional_string_property(properties, "name")?)
        .ok_or_else(|| EditorError::MarkerTitle(feature_id.to_owned()))?;
    validate_label("marker title", &title)?;
    let category = optional_string_property(properties, "category")?;
    if let Some(category) = &category {
        validate_label("marker category", category)?;
    }
    let symbol = optional_string_property(properties, "symbol")?;
    if let Some(symbol) = &symbol {
        validate_label("marker symbol", symbol)?;
    }
    let min_zoom = optional_u64_property(properties, "min_zoom")?.unwrap_or(0);
    let max_zoom = optional_u64_property(properties, "max_zoom")?.unwrap_or(24);
    if min_zoom > max_zoom || max_zoom > 24 {
        return Err(EditorError::MarkerZoom(feature_id.to_owned()));
    }
    let collision_priority = optional_u64_property(properties, "collision_priority")?.unwrap_or(0);
    if collision_priority > 1_000 {
        return Err(EditorError::MarkerPriority(feature_id.to_owned()));
    }
    Ok(SceneMarkerIndex {
        title,
        category,
        symbol,
        min_zoom,
        max_zoom,
        collision_priority,
    })
}

fn optional_string_property(
    properties: &serde_json::Map<String, Value>,
    name: &'static str,
) -> Result<Option<String>, EditorError> {
    properties
        .get(name)
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or(EditorError::MarkerProperty(name))
        })
        .transpose()
}

fn optional_u64_property(
    properties: &serde_json::Map<String, Value>,
    name: &'static str,
) -> Result<Option<u64>, EditorError> {
    properties
        .get(name)
        .map(|value| value.as_u64().ok_or(EditorError::MarkerProperty(name)))
        .transpose()
}

fn string_member<'a>(
    object: &'a serde_json::Map<String, Value>,
    member: &'static str,
) -> Result<&'a str, EditorError> {
    object
        .get(member)
        .and_then(Value::as_str)
        .ok_or(EditorError::GeoJsonMember(member))
}

fn geojson_feature_id(value: Option<&Value>) -> Result<String, EditorError> {
    let id = match value {
        Some(Value::String(id)) => id.clone(),
        Some(Value::Number(id)) => id.to_string(),
        _ => return Err(EditorError::FeatureId),
    };
    validate_identifier("GeoJSON feature id", &id)?;
    Ok(id)
}

fn finite_number(value: &Value) -> Result<f64, EditorError> {
    value
        .as_f64()
        .filter(|number| number.is_finite())
        .ok_or(EditorError::CoordinateNumber)
}

fn source_revisions(snapshot: &SceneCandidateSnapshot) -> BTreeMap<String, SemanticDigest> {
    snapshot
        .sources
        .iter()
        .map(|source| {
            (
                source.source_id.clone(),
                source.artifact.content_digest.clone(),
            )
        })
        .collect()
}

fn feature_revisions(snapshot: &SceneCandidateSnapshot) -> BTreeMap<String, SemanticDigest> {
    snapshot
        .features
        .iter()
        .map(|feature| (feature.feature_id.clone(), feature.feature_revision.clone()))
        .collect()
}

fn diff_objects(
    object_kind: SceneObjectKind,
    source: BTreeMap<String, SemanticDigest>,
    target: BTreeMap<String, SemanticDigest>,
    changes: &mut Vec<SceneObjectChange>,
) {
    for (id, source_revision) in &source {
        match target.get(id) {
            None => changes.push(SceneObjectChange {
                object_kind,
                object_id: id.clone(),
                change_kind: SceneChangeKind::Deleted,
                source_revision: Some(source_revision.clone()),
                target_revision: None,
            }),
            Some(target_revision) if target_revision != source_revision => {
                changes.push(SceneObjectChange {
                    object_kind,
                    object_id: id.clone(),
                    change_kind: SceneChangeKind::Modified,
                    source_revision: Some(source_revision.clone()),
                    target_revision: Some(target_revision.clone()),
                });
            }
            Some(_) => {}
        }
    }
    for (id, target_revision) in target {
        if !source.contains_key(&id) {
            changes.push(SceneObjectChange {
                object_kind,
                object_id: id,
                change_kind: SceneChangeKind::Inserted,
                source_revision: None,
                target_revision: Some(target_revision),
            });
        }
    }
}

fn snapshot_identity(snapshot: &SceneCandidateSnapshot) -> Result<SemanticDigest, EditorError> {
    let mut normalized = snapshot.clone();
    normalized.snapshot_revision = digest_placeholder();
    let mut hasher = SemanticHasher::new(SCENE_CANDIDATE_SNAPSHOT_SCHEMA);
    hasher.add_bytes(&serde_json::to_vec(&normalized)?);
    Ok(hasher.finish())
}

fn verify_commit_history(commits: &[SceneCommit]) -> Result<(), EditorError> {
    if commits.len() > MAX_SCENE_COMMITS {
        return Err(EditorError::CommitLimit(MAX_SCENE_COMMITS));
    }
    let mut ids = BTreeSet::new();
    let mut parent: Option<&SceneCommit> = None;
    for (index, commit) in commits.iter().enumerate() {
        commit.verify()?;
        let expected_sequence = index as u64 + 1;
        if commit.sequence != expected_sequence {
            return Err(EditorError::CommitSequence {
                expected: expected_sequence,
                actual: commit.sequence,
            });
        }
        if commit.parent_commit_id.as_ref() != parent.map(|parent| &parent.commit_id) {
            return Err(EditorError::CommitParent(commit.sequence));
        }
        if !ids.insert(commit.commit_id.clone()) {
            return Err(EditorError::DuplicateCommit(commit.commit_id.clone()));
        }
        parent = Some(commit);
    }
    Ok(())
}

fn scene_commit_identity(
    sequence: u64,
    parent_commit_id: Option<&SemanticDigest>,
    committed_at_unix: i64,
    message: &str,
    package: &ScenePackageReference,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(SCENE_COMMIT_SCHEMA);
    hasher.add_u64(sequence);
    hasher.add_optional_str(parent_commit_id.map(SemanticDigest::as_str));
    hasher.add_str(&committed_at_unix.to_string());
    hasher.add_str(message);
    hasher.add_str(package.package_id.as_str());
    hasher.add_str(package.snapshot_revision.as_str());
    hasher.add_str(&package.package_path);
    hasher.add_str(&package.admission_request_path);
    hasher.finish()
}

fn normalize_commit_message(message: String) -> Result<String, EditorError> {
    let message = message.trim().to_owned();
    if message.is_empty() {
        return Err(EditorError::EmptyCommitMessage);
    }
    if message.len() > MAX_COMMIT_MESSAGE_BYTES {
        return Err(EditorError::CommitMessageLimit(MAX_COMMIT_MESSAGE_BYTES));
    }
    if message.contains('\0') {
        return Err(EditorError::CommitMessageNul);
    }
    Ok(message)
}

fn validate_commit_timestamp(committed_at_unix: i64) -> Result<(), EditorError> {
    DateTime::<Utc>::from_timestamp(committed_at_unix, 0)
        .ok_or(EditorError::CommitTimestamp(committed_at_unix))?;
    Ok(())
}

fn package_identity(package: &ScenePackage) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(SCENE_PACKAGE_SCHEMA);
    hasher.add_str(package.snapshot.snapshot_revision.as_str());
    hasher.add_optional_str(
        package
            .parent_package_id
            .as_ref()
            .map(SemanticDigest::as_str),
    );
    hasher.add_str(
        package
            .change_set
            .source_revision
            .as_ref()
            .map_or("", SemanticDigest::as_str),
    );
    hasher.add_str(
        package
            .change_set
            .target_revision
            .as_ref()
            .map_or("", SemanticDigest::as_str),
    );
    hasher.add_str(&package.change_set.source_label);
    hasher.add_str(&package.change_set.target_label);
    hasher.add_str(match package.change_set.assessment {
        DeltaAssessment::Equal => "equal",
        DeltaAssessment::Different => "different",
        DeltaAssessment::Inconclusive => "inconclusive",
    });
    hasher.add_u64(package.change_set.inserted);
    hasher.add_u64(package.change_set.deleted);
    hasher.add_u64(package.change_set.modified);
    for change in &package.change_set.changes {
        hasher.add_str(match change.object_kind {
            SceneObjectKind::Source => "source",
            SceneObjectKind::Feature => "feature",
        });
        hasher.add_str(&change.object_id);
        hasher.add_str(match change.change_kind {
            SceneChangeKind::Inserted => "inserted",
            SceneChangeKind::Deleted => "deleted",
            SceneChangeKind::Modified => "modified",
        });
        hasher.add_optional_str(change.source_revision.as_ref().map(SemanticDigest::as_str));
        hasher.add_optional_str(change.target_revision.as_ref().map(SemanticDigest::as_str));
    }
    hasher.add_str(&package.admission_authority);
    hasher.finish()
}

fn admission_request_identity(request: &SceneAdmissionRequest) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(SCENE_ADMISSION_REQUEST_SCHEMA);
    hasher.add_str(request.package_id.as_str());
    hasher.add_str(&request.package_path);
    hasher.add_str(&request.requested_operation);
    hasher.add_str(&request.status);
    hasher.add_bool(request.admitted);
    hasher.finish()
}

fn content_identity(bytes: &[u8]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.scene-native-artifact.v1");
    hasher.add_bytes(bytes);
    hasher.finish()
}

fn regional_bounds(bounds: &SceneBounds) -> RegionalBounds {
    RegionalBounds {
        west_microdegrees: (bounds.west * 1_000_000.0).round() as i64,
        south_microdegrees: (bounds.south * 1_000_000.0).round() as i64,
        east_microdegrees: (bounds.east * 1_000_000.0).round() as i64,
        north_microdegrees: (bounds.north * 1_000_000.0).round() as i64,
        crosses_antimeridian: false,
    }
}

fn properties_identity(bytes: &[u8]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.scene-feature-properties.v1");
    hasher.add_bytes(bytes);
    hasher.finish()
}

fn feature_identity(source_id: &str, role: SceneSourceRole, bytes: &[u8]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.scene-feature.v1");
    hasher.add_str(source_id);
    hasher.add_str(role.label());
    hasher.add_bytes(bytes);
    hasher.finish()
}

fn digest_placeholder() -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.identity.placeholder.v1");
    hasher.add_str("excluded from identity");
    hasher.finish()
}

fn digest_key(digest: &SemanticDigest) -> &str {
    digest
        .as_str()
        .strip_prefix("blake3:")
        .unwrap_or(digest.as_str())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), EditorError> {
    let count = value.chars().count();
    if count == 0
        || count > MAX_IDENTIFIER_CHARS
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err(EditorError::Identifier {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_label(field: &'static str, value: &str) -> Result<(), EditorError> {
    let count = value.chars().count();
    if count == 0 || count > MAX_LABEL_CHARS || value.chars().any(char::is_control) {
        return Err(EditorError::Label {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_path_argument(path: &Path) -> Result<(), EditorError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(EditorError::Path(path.to_owned()));
    }
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<(), EditorError> {
    validate_path_argument(Path::new(path))
}

fn workspace_relative(
    workspace: &Path,
    supplied: &Path,
    max_bytes: u64,
) -> Result<PathBuf, EditorError> {
    validate_path_argument(supplied)?;
    let joined = workspace.join(supplied);
    let metadata = fs::symlink_metadata(&joined).map_err(|source| EditorError::Read {
        path: joined.clone(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(EditorError::UnsafePath(joined));
    }
    if metadata.len() > max_bytes {
        return Err(EditorError::InputLimit {
            path: joined,
            limit: max_bytes,
        });
    }
    let canonical = joined.canonicalize().map_err(|source| EditorError::Read {
        path: joined.clone(),
        source,
    })?;
    if !canonical.starts_with(workspace) {
        return Err(EditorError::PathEscape(supplied.to_owned()));
    }
    Ok(canonical
        .strip_prefix(workspace)
        .expect("workspace containment was checked")
        .to_owned())
}

fn path_string(path: &Path) -> Result<String, EditorError> {
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| EditorError::PathEncoding(path.to_owned()))
}

fn read_bounded_file(path: &Path, limit: u64, kind: &'static str) -> Result<Vec<u8>, EditorError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| EditorError::Read {
        path: path.to_owned(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(EditorError::UnsafePath(path.to_owned()));
    }
    if metadata.len() > limit {
        return Err(EditorError::InputLimit {
            path: path.to_owned(),
            limit,
        });
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|source| EditorError::Read {
            path: path.to_owned(),
            source,
        })?
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| EditorError::Read {
            path: path.to_owned(),
            source,
        })?;
    if bytes.len() as u64 > limit {
        return Err(EditorError::InputLimit {
            path: path.to_owned(),
            limit,
        });
    }
    if bytes.is_empty() {
        return Err(EditorError::EmptyInput {
            kind,
            path: path.to_owned(),
        });
    }
    Ok(bytes)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), EditorError> {
    if let Ok(metadata) = fs::symlink_metadata(path)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err(EditorError::UnsafePath(path.to_owned()));
    }
    let parent = path
        .parent()
        .ok_or_else(|| EditorError::Path(path.to_owned()))?;
    fs::create_dir_all(parent).map_err(|source| EditorError::Write {
        path: parent.to_owned(),
        source,
    })?;
    for attempt in 0..32_u8 {
        let temporary = parent.join(format!(".rey-editor.tmp-{}-{attempt}", std::process::id()));
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => {
                return Err(EditorError::Write {
                    path: temporary,
                    source,
                });
            }
        };
        let publication = file
            .write_all(bytes)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.flush())
            .and_then(|()| {
                drop(file);
                fs::rename(&temporary, path)
            });
        if let Err(source) = publication {
            let _ = fs::remove_file(&temporary);
            return Err(EditorError::Write {
                path: path.to_owned(),
                source,
            });
        }
        return Ok(());
    }
    Err(EditorError::TemporaryLimit(parent.to_owned()))
}

fn write_content_addressed_json(path: &Path, value: &impl Serialize) -> Result<bool, EditorError> {
    let bytes = serde_json::to_vec_pretty(value)?;
    write_content_addressed_bytes(path, &bytes)
}

fn write_content_addressed_bytes(path: &Path, bytes: &[u8]) -> Result<bool, EditorError> {
    if path.exists() {
        let retained = read_bounded_file(
            path,
            MAX_STATE_BYTES.max(MAX_SOURCE_BYTES),
            "retained object",
        )?;
        if retained == bytes || retained.strip_suffix(b"\n") == Some(bytes) {
            return Ok(false);
        }
        return Err(EditorError::ContentAddressCollision(path.to_owned()));
    }
    let parent = path
        .parent()
        .ok_or_else(|| EditorError::Path(path.to_owned()))?;
    fs::create_dir_all(parent).map_err(|source| EditorError::Write {
        path: parent.to_owned(),
        source,
    })?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| EditorError::Write {
            path: path.to_owned(),
            source,
        })?;
    file.write_all(bytes)
        .and_then(|()| file.flush())
        .map_err(|source| EditorError::Write {
            path: path.to_owned(),
            source,
        })?;
    Ok(true)
}

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("expected schema {expected}, found {actual}")]
    Schema {
        expected: &'static str,
        actual: String,
    },
    #[error(
        "editor identifiers must be 1..={MAX_IDENTIFIER_CHARS} ASCII letters, digits, '.', '-', or '_': {field}={value}"
    )]
    Identifier { field: &'static str, value: String },
    #[error("{field} must be 1..={MAX_LABEL_CHARS} non-control characters: {value}")]
    Label { field: &'static str, value: String },
    #[error("editor project paths must be workspace-relative without '..': {0}")]
    Path(PathBuf),
    #[error("editor path escapes the workspace: {0}")]
    PathEscape(PathBuf),
    #[error("editor path is not valid UTF-8: {0}")]
    PathEncoding(PathBuf),
    #[error("editor input is not a regular non-symlinked file: {0}")]
    UnsafePath(PathBuf),
    #[error("{path} exceeds the {limit}-byte editor input limit")]
    InputLimit { path: PathBuf, limit: u64 },
    #[error("{kind} is empty: {path}")]
    EmptyInput { kind: &'static str, path: PathBuf },
    #[error("editor input {path} could not be read: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("editor artifact {path} could not be written: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "editor project is not initialized in {0}; run `rey editor source add --help` or `rey editor generate terrain --help`"
    )]
    UninitializedProject(PathBuf),
    #[error("editor project state is missing from {0} while retained INDEX or HEAD exists")]
    MissingProject(PathBuf),
    #[error("editor state lock {path} failed: {source}")]
    Lock {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("editor state exceeds {0} bytes")]
    StateLimit(u64),
    #[error("editor temporary-file attempts exhausted below {0}")]
    TemporaryLimit(PathBuf),
    #[error("content-addressed editor object changed in place: {0}")]
    ContentAddressCollision(PathBuf),
    #[error("editor project uses a non-canonical source order")]
    NonCanonicalProject,
    #[error("scene snapshot uses a non-canonical object order")]
    NonCanonicalSnapshot,
    #[error("editor project coordinate system must be OGC CRS84 longitude/latitude for GeoJSON")]
    CoordinateSystem,
    #[error("editor project exceeds {0} sources")]
    SourceLimit(usize),
    #[error("duplicate editor source id: {0}")]
    DuplicateSourceId(String),
    #[error("duplicate editor source path: {0}")]
    DuplicateSourcePath(String),
    #[error(
        "requested scene identity does not match the project: expected {expected}, found {actual}"
    )]
    ProjectIdentity { expected: String, actual: String },
    #[error("duplicate scene feature id: {0}")]
    DuplicateFeatureId(String),
    #[error("generated scene output must use the .geojson extension: {0}")]
    GeneratedOutputExtension(PathBuf),
    #[error("editor scene sources must use the .geojson extension: {0}")]
    SourceExtension(PathBuf),
    #[error("refusing to overwrite a source not owned by the declared generator: {0}")]
    GeneratedOutputAuthority(String),
    #[error("invalid terrain generation parameter: {0}")]
    GenerationParameter(String),
    #[error("GeoJSON root must be a Feature or FeatureCollection object")]
    GeoJsonRoot,
    #[error("GeoJSON member is missing or has the wrong type: {0}")]
    GeoJsonMember(&'static str),
    #[error("unsupported GeoJSON root type: {0}")]
    GeoJsonType(String),
    #[error("GeoJSON must not declare a custom CRS; RFC 7946 data is OGC CRS84")]
    GeoJsonCrs,
    #[error("GeoJSON Feature is malformed")]
    GeoJsonFeature,
    #[error("GeoJSON Features require stable string or number ids")]
    FeatureId,
    #[error("GeoJSON Feature properties must be an object or null: {0}")]
    GeoJsonProperties(String),
    #[error("GeoJSON Feature has no admitted geometry: {0}")]
    MissingGeometry(String),
    #[error("qualified terrain sample is invalid: {0}")]
    TerrainSample(String),
    #[error("unsupported GeoJSON geometry type: {0}")]
    GeometryType(String),
    #[error("GeoJSON {kind} requires at least {minimum} positions, found {actual}")]
    GeometryMinimum {
        kind: &'static str,
        minimum: usize,
        actual: usize,
    },
    #[error("GeoJSON Polygon linear rings must have identical first and last positions")]
    PolygonRing,
    #[error("GeoJSON geometry has no coordinates")]
    EmptyGeometry,
    #[error("GeoJSON coordinates have invalid nesting")]
    InvalidCoordinates,
    #[error("GeoJSON positions require two or three numbers, found {0}")]
    PositionDimensions(usize),
    #[error("GeoJSON coordinate is not a finite number")]
    CoordinateNumber,
    #[error(
        "GeoJSON coordinate is outside CRS84 bounds: longitude={longitude}, latitude={latitude}"
    )]
    CoordinateRange { longitude: f64, latitude: f64 },
    #[error("scene source exceeds {0} features")]
    FeatureLimit(usize),
    #[error("scene source exceeds {0} coordinate positions")]
    CoordinateLimit(usize),
    #[error("feature {feature_id} exceeds {limit} properties")]
    PropertyLimit { feature_id: String, limit: usize },
    #[error("feature {feature_id} properties exceed {limit} bytes")]
    PropertyByteLimit { feature_id: String, limit: usize },
    #[error("marker source feature must use Point or MultiPoint geometry: {0}")]
    MarkerGeometry(String),
    #[error("marker requires a title or name property: {0}")]
    MarkerTitle(String),
    #[error("marker property has the wrong type: {0}")]
    MarkerProperty(&'static str),
    #[error("marker zoom range must satisfy 0 <= min_zoom <= max_zoom <= 24: {0}")]
    MarkerZoom(String),
    #[error("marker collision_priority must be at most 1000: {0}")]
    MarkerPriority(String),
    #[error("scene snapshot identity does not match its normalized contents")]
    SnapshotIdentity,
    #[error("scene snapshot contains a feature whose source is absent")]
    SnapshotReference,
    #[error("scene snapshot contains an invalid or inconsistent source index")]
    SnapshotSource,
    #[error("scene snapshot contains an invalid or inconsistent feature index")]
    SnapshotFeature,
    #[error("scene snapshot coverage does not match its retained objects")]
    SnapshotCoverage,
    #[error("a bounded scene snapshot must disclose at least one omission")]
    SnapshotCompleteness,
    #[error("scene package identity does not match its contents")]
    PackageIdentity,
    #[error("scene package authority must remain candidate_only")]
    PackageAuthority,
    #[error("scene admission request identity or candidate-only boundary is invalid")]
    AdmissionRequestIdentity,
    #[error("staged scene artifact identity changed: {0}")]
    ArtifactIdentity(String),
    #[error("editor index is empty; run `rey editor add` before committing")]
    EmptyIndex,
    #[error("nothing staged for scene commit")]
    NothingToCommit,
    #[error("scene commit message must not be empty")]
    EmptyCommitMessage,
    #[error("scene commit message exceeds {0} bytes")]
    CommitMessageLimit(usize),
    #[error("scene commit message must not contain NUL")]
    CommitMessageNul,
    #[error("scene commit message is not canonical")]
    NonCanonicalCommitMessage,
    #[error("scene commit timestamp is invalid: {0}")]
    CommitTimestamp(i64),
    #[error("scene commit sequence mismatch: expected {expected}, found {actual}")]
    CommitSequence { expected: u64, actual: u64 },
    #[error("scene commit {0} has the wrong parent")]
    CommitParent(u64),
    #[error("duplicate scene commit identity: {0}")]
    DuplicateCommit(SemanticDigest),
    #[error("scene commit history exceeds {0} commits")]
    CommitLimit(usize),
    #[error("scene commit does not resolve to its exact package and admission request")]
    CommitPackageIdentity,
    #[error("scene log count must be 1..={limit}, found {actual}")]
    LogLimit { limit: usize, actual: usize },
    #[error("scene commit identity does not match its contents")]
    CommitIdentity,
    #[error("unknown current scene package: {0}")]
    UnknownPackage(String),
    #[error("unknown editor scene revision SCENE@{0}")]
    UnknownSceneSequence(u64),
    #[error("editor scene revision has no bounded native coordinates")]
    EmptySceneBounds,
    #[error(transparent)]
    SceneAdmission(#[from] rey_runtime::SceneAdmissionError),
    #[error("editor JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("editor filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rey_diff::DeltaAssessment;
    use tempfile::TempDir;

    use super::{LocalEditorStore, SceneSourceDeclaration, SceneSourceFormat, SceneSourceRole};

    #[test]
    fn scene_source_roles_have_exact_project_document_labels() {
        for (role, label) in [
            (SceneSourceRole::Features, "features"),
            (SceneSourceRole::Markers, "markers"),
            (SceneSourceRole::Terrain, "terrain"),
            (SceneSourceRole::TerrainControl, "terrain_control"),
            (SceneSourceRole::Hydrology, "hydrology"),
            (SceneSourceRole::Boundary, "boundary"),
            (SceneSourceRole::Highway, "highway"),
            (SceneSourceRole::Road, "road"),
            (SceneSourceRole::Railway, "railway"),
            (SceneSourceRole::District, "district"),
            (SceneSourceRole::Lot, "lot"),
            (SceneSourceRole::Structure, "structure"),
            (SceneSourceRole::Utility, "utility"),
            (SceneSourceRole::Label, "label"),
            (SceneSourceRole::Beacon, "beacon"),
            (SceneSourceRole::Construction, "construction"),
            (SceneSourceRole::Connector, "connector"),
        ] {
            assert_eq!(role.label(), label);
            assert_eq!(serde_json::to_value(role).unwrap(), label);
            assert_eq!(
                serde_json::from_value::<SceneSourceRole>(label.into()).unwrap(),
                role
            );
        }
    }

    #[test]
    fn source_add_registers_verified_native_geometry_through_index_and_commit() {
        let workspace = TempDir::new().unwrap();
        let store = LocalEditorStore::default_for_workspace(workspace.path());
        fs::write(
            workspace.path().join("boundary.geojson"),
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"county","properties":{},"geometry":{"type":"Polygon","coordinates":[[[-123.0,37.0],[-122.0,37.0],[-122.0,38.0],[-123.0,37.0]]]}}]}"#,
        )
        .unwrap();

        let added = store
            .add_source(
                std::path::Path::new("boundary.geojson"),
                Some("county-demo".to_owned()),
                "county-boundary".to_owned(),
                SceneSourceRole::Boundary,
            )
            .unwrap();
        assert!(added.changed);
        assert!(added.project_created);
        assert_eq!(added.source.path, "boundary.geojson");
        assert_eq!(added.source.role, SceneSourceRole::Boundary);
        assert_eq!(added.feature_count, 1);
        assert_eq!(added.coordinate_count, 4);
        assert!(
            !store
                .add_source(
                    std::path::Path::new("boundary.geojson"),
                    Some("county-demo".to_owned()),
                    "county-boundary".to_owned(),
                    SceneSourceRole::Boundary,
                )
                .unwrap()
                .changed
        );

        let status = store.status().unwrap();
        assert_eq!(status.state, super::EditorWorkingState::Working);
        assert_eq!(status.unstaged.inserted, 2);
        let index = store.add().unwrap();
        assert!(index.staged);
        assert_eq!(index.snapshot.sources[0].role, SceneSourceRole::Boundary);
        let committed = store.commit("Admit county boundary".to_owned()).unwrap();
        let candidate = store.admission_candidate(1).unwrap();
        assert_eq!(candidate.package_id, committed.package.package_id);
        assert_eq!(candidate.sources[0].role, "boundary");
        assert_eq!(candidate.features[0].role, "boundary");
        assert_eq!(
            candidate.features[0].feature_revision,
            index.snapshot.features[0].feature_revision
        );
    }

    #[test]
    fn source_add_rejects_symlinks_and_role_or_identity_rebinding() {
        let workspace = TempDir::new().unwrap();
        let store = LocalEditorStore::default_for_workspace(workspace.path());
        fs::write(
            workspace.path().join("roads.geojson"),
            r#"{"type":"Feature","id":"main","properties":{},"geometry":{"type":"LineString","coordinates":[[-123.0,37.0],[-122.0,38.0]]}}"#,
        )
        .unwrap();
        store
            .add_source(
                std::path::Path::new("roads.geojson"),
                Some("county-demo".to_owned()),
                "roads".to_owned(),
                SceneSourceRole::Road,
            )
            .unwrap();
        assert!(matches!(
            store.add_source(
                std::path::Path::new("roads.geojson"),
                None,
                "roads".to_owned(),
                SceneSourceRole::Highway,
            ),
            Err(super::EditorError::DuplicateSourceId(_))
        ));
        assert!(matches!(
            store.add_source(
                std::path::Path::new("roads.geojson"),
                None,
                "other".to_owned(),
                SceneSourceRole::Road,
            ),
            Err(super::EditorError::DuplicateSourcePath(_))
        ));

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(
                workspace.path().join("roads.geojson"),
                workspace.path().join("linked.geojson"),
            )
            .unwrap();
            assert!(matches!(
                store.add_source(
                    std::path::Path::new("linked.geojson"),
                    None,
                    "linked".to_owned(),
                    SceneSourceRole::Road,
                ),
                Err(super::EditorError::UnsafePath(_))
            ));
        }
    }

    #[test]
    fn qualified_terrain_requires_exact_point_altitude_and_material() {
        let workspace = TempDir::new().unwrap();
        let store = LocalEditorStore::default_for_workspace(workspace.path());
        fs::write(
            workspace.path().join("terrain.geojson"),
            r#"{"type":"Feature","id":"summit","properties":{"material":"granite"},"geometry":{"type":"Point","coordinates":[-122.5,37.5,153.25]}}"#,
        )
        .unwrap();
        let added = store
            .add_source(
                std::path::Path::new("terrain.geojson"),
                Some("county-demo".to_owned()),
                "terrain-samples".to_owned(),
                SceneSourceRole::Terrain,
            )
            .unwrap();
        assert_eq!(added.feature_count, 1);
        let snapshot = store.add().unwrap().snapshot;
        assert_eq!(
            snapshot.features[0].terrain_sample,
            Some(super::SceneTerrainSample {
                longitude_microdegrees: -122_500_000,
                latitude_microdegrees: 37_500_000,
                elevation_micrometers: Some(153_250_000),
                material: Some("granite".to_owned()),
                grid: None,
                packed_grid: None,
            })
        );

        fs::write(
            workspace.path().join("invalid.geojson"),
            r#"{"type":"Feature","id":"unknown","properties":{},"geometry":{"type":"Point","coordinates":[-122.5,37.5]}}"#,
        )
        .unwrap();
        assert!(matches!(
            store.add_source(
                std::path::Path::new("invalid.geojson"),
                None,
                "invalid-terrain".to_owned(),
                SceneSourceRole::Terrain,
            ),
            Err(super::EditorError::TerrainSample(_))
        ));
    }

    #[test]
    fn terrain_grid_vertices_require_complete_layout_and_explicit_no_data() {
        let workspace = TempDir::new().unwrap();
        let store = LocalEditorStore::default_for_workspace(workspace.path());
        fs::write(
            workspace.path().join("terrain-grid.geojson"),
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"northwest","properties":{"material":"granite","terrain_grid_id":"dem","terrain_grid_column":0,"terrain_grid_row":0,"terrain_grid_columns":2,"terrain_grid_rows":2,"terrain_grid_validity":"valid"},"geometry":{"type":"Point","coordinates":[-123.0,38.0,120.0]}},{"type":"Feature","id":"northeast","properties":{"terrain_grid_id":"dem","terrain_grid_column":1,"terrain_grid_row":0,"terrain_grid_columns":2,"terrain_grid_rows":2,"terrain_grid_validity":"no_data"},"geometry":{"type":"Point","coordinates":[-122.0,38.0]}},{"type":"Feature","id":"southwest","properties":{"material":"granite","terrain_grid_id":"dem","terrain_grid_column":0,"terrain_grid_row":1,"terrain_grid_columns":2,"terrain_grid_rows":2,"terrain_grid_validity":"valid"},"geometry":{"type":"Point","coordinates":[-123.0,37.0,80.0]}},{"type":"Feature","id":"southeast","properties":{"material":"granite","terrain_grid_id":"dem","terrain_grid_column":1,"terrain_grid_row":1,"terrain_grid_columns":2,"terrain_grid_rows":2,"terrain_grid_validity":"valid"},"geometry":{"type":"Point","coordinates":[-122.0,37.0,90.0]}}]}"#,
        )
        .unwrap();
        store
            .add_source(
                std::path::Path::new("terrain-grid.geojson"),
                Some("county-grid".to_owned()),
                "terrain-grid".to_owned(),
                SceneSourceRole::Terrain,
            )
            .unwrap();
        let snapshot = store.add().unwrap().snapshot;
        let no_data = snapshot
            .features
            .iter()
            .find(|feature| feature.source_feature_id == "northeast")
            .and_then(|feature| feature.terrain_sample.as_ref())
            .unwrap();
        assert_eq!(no_data.elevation_micrometers, None);
        assert_eq!(no_data.material, None);
        assert_eq!(no_data.grid.as_ref().unwrap().validity, "no_data");

        fs::write(
            workspace.path().join("incomplete-grid.geojson"),
            r#"{"type":"Feature","id":"broken","properties":{"material":"granite","terrain_grid_id":"dem"},"geometry":{"type":"Point","coordinates":[-123.0,38.0,120.0]}}"#,
        )
        .unwrap();
        assert!(matches!(
            store.add_source(
                std::path::Path::new("incomplete-grid.geojson"),
                None,
                "incomplete-grid".to_owned(),
                SceneSourceRole::Terrain,
            ),
            Err(super::EditorError::TerrainSample(_))
        ));
    }

    #[test]
    fn packed_terrain_grid_retains_exact_bounded_channels() {
        let workspace = TempDir::new().unwrap();
        let store = LocalEditorStore::default_for_workspace(workspace.path());
        let elevation_values = [
            10_000_i32, 10_400, 10_800, 9_800, 0, 10_600, 9_600, 10_000, 10_400,
        ];
        let elevation_hex = elevation_values
            .into_iter()
            .flat_map(i32::to_le_bytes)
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let terrain = serde_json::json!({
            "type": "Feature",
            "id": "packed-dem",
            "properties": {"title": "Packed fixture terrain"},
            "terrain_grid": {
                "schema": "rey.packed-terrain-grid.v1",
                "dataset_id": "packed-dem-v1",
                "compiler_revision": "rey.fixture.packed-terrain@1",
                "columns": 3,
                "rows": 3,
                "native_bounds_microdegrees": [-123000000, 37000000, -122000000, 38000000],
                "validity_hex": "010101010001010101",
                "elevation_centimeters_le_hex": elevation_hex,
                "material_palette": ["granite"],
                "material_indices_hex": "00000000ff00000000"
            },
            "geometry": {
                "type": "Polygon",
                "coordinates": [[
                    [-123.0, 38.0], [-122.0, 38.0], [-122.0, 37.0],
                    [-123.0, 37.0], [-123.0, 38.0]
                ]]
            }
        });
        fs::write(
            workspace.path().join("packed-terrain.geojson"),
            serde_json::to_vec(&terrain).unwrap(),
        )
        .unwrap();
        store
            .add_source(
                std::path::Path::new("packed-terrain.geojson"),
                Some("packed-county".to_owned()),
                "packed-terrain".to_owned(),
                SceneSourceRole::Terrain,
            )
            .unwrap();
        let snapshot = store.add().unwrap().snapshot;
        let packed = snapshot.features[0]
            .terrain_sample
            .as_ref()
            .and_then(|sample| sample.packed_grid.as_ref())
            .unwrap();
        assert_eq!(packed.columns, 3);
        assert_eq!(packed.rows, 3);
        assert_eq!(packed.validity_hex, "010101010001010101");
        assert_eq!(snapshot.coverage.features, 1);
        assert_eq!(snapshot.coverage.coordinates, 5);

        let mut invalid = terrain;
        invalid["terrain_grid"]["material_indices_hex"] =
            serde_json::Value::String("000000000000000000".to_owned());
        fs::write(
            workspace.path().join("invalid-packed-terrain.geojson"),
            serde_json::to_vec(&invalid).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            store.add_source(
                std::path::Path::new("invalid-packed-terrain.geojson"),
                None,
                "invalid-packed-terrain".to_owned(),
                SceneSourceRole::Terrain,
            ),
            Err(super::EditorError::TerrainSample(_))
        ));
    }

    fn declare_geojson_source(
        store: &LocalEditorStore,
        source_path: &str,
        source_id: &str,
        role: SceneSourceRole,
    ) {
        let (project_file, mut project) = store.load_project().unwrap();
        project.sources = vec![SceneSourceDeclaration {
            source_id: source_id.to_owned(),
            path: source_path.to_owned(),
            format: SceneSourceFormat::GeoJson,
            role,
        }];
        let project = project.canonicalize().unwrap();
        store.write_project(&project_file, &project).unwrap();
    }

    #[test]
    fn stages_exact_native_geojson_and_commits_only_the_index() {
        let workspace = TempDir::new().unwrap();
        let store = LocalEditorStore::default_for_workspace(workspace.path());
        store.init_project("atlas".to_owned()).unwrap();
        fs::write(
            workspace.path().join("markers.geojson"),
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"ridge","geometry":{"type":"Point","coordinates":[-122.4,37.8]},"properties":{"title":"Ridge","category":"survey","min_zoom":3,"collision_priority":10}}]}"#,
        )
        .unwrap();
        declare_geojson_source(
            &store,
            "markers.geojson",
            "survey-poi",
            SceneSourceRole::Markers,
        );

        let working = store.status().unwrap();
        assert_eq!(working.working.unwrap().coverage.markers, 1);
        assert_eq!(working.unstaged.inserted, 2);
        let added = store.add().unwrap();
        assert!(added.staged);
        let committed = store.commit("initial terrain".to_owned()).unwrap();
        assert_eq!(committed.commit.sequence, 1);
        assert_eq!(committed.commit.message, "initial terrain");
        assert!(!committed.admission_request.admitted);
        assert_eq!(committed.admission_request.status, "requires_workload");
        assert!(store.commit("duplicate".to_owned()).is_err());

        fs::write(
            workspace.path().join("markers.geojson"),
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"ridge","geometry":{"type":"Point","coordinates":[-122.5,37.8]},"properties":{"title":"Ridge"}}]}"#,
        )
        .unwrap();
        let second = store.status().unwrap();
        assert_eq!(second.staged.assessment, DeltaAssessment::Equal);
        assert_eq!(second.unstaged.modified, 2);
        store.add().unwrap();
        let second_commit = store.commit("move ridge".to_owned()).unwrap();
        assert_ne!(
            second_commit.package.package_id,
            committed.package.package_id
        );
        assert_eq!(second_commit.commit.sequence, 2);
        assert_eq!(
            second_commit.commit.parent_commit_id.as_ref(),
            Some(&committed.commit.commit_id)
        );
        let log = store.log(32, true).unwrap();
        assert_eq!(log.total_commits, 2);
        assert_eq!(
            log.entries[1].package.snapshot.features[0].bounds.west,
            -122.4
        );
    }

    #[test]
    fn indexes_exact_cartographic_label_metadata() {
        let workspace = TempDir::new().unwrap();
        let store = LocalEditorStore::default_for_workspace(workspace.path());
        store.init_project("atlas".to_owned()).unwrap();
        fs::write(
            workspace.path().join("labels.geojson"),
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"county-seat","geometry":{"type":"Point","coordinates":[-122.4,37.8]},"properties":{"title":"Fixture County","category":"county_seat","min_zoom":3,"max_zoom":18,"collision_priority":100}}]}"#,
        )
        .unwrap();
        declare_geojson_source(
            &store,
            "labels.geojson",
            "county-labels",
            SceneSourceRole::Label,
        );

        let snapshot = store.add().unwrap().snapshot;
        let label = snapshot.features[0]
            .cartographic_label
            .as_ref()
            .expect("cartographic label index");
        assert_eq!(label.title, "Fixture County");
        assert_eq!(label.min_zoom, 3);
        assert_eq!(label.max_zoom, 18);
        assert_eq!(snapshot.coverage.markers, 0);
    }

    #[test]
    fn commit_validates_frozen_index_before_advancing_head() {
        let workspace = TempDir::new().unwrap();
        let store = LocalEditorStore::default_for_workspace(workspace.path());
        store.init_project("atlas".to_owned()).unwrap();
        fs::write(
            workspace.path().join("terrain.geojson"),
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"ridge","geometry":{"type":"Point","coordinates":[-122.4,37.8]},"properties":{}}]}"#,
        )
        .unwrap();
        declare_geojson_source(
            &store,
            "terrain.geojson",
            "terrain",
            SceneSourceRole::TerrainControl,
        );
        let added = store.add().unwrap();
        let object_path = workspace
            .path()
            .join(".rey/editor")
            .join(&added.snapshot.sources[0].artifact.object_path);
        fs::write(object_path, b"tampered frozen bytes").unwrap();

        let error = store.commit("must fail closed".to_owned()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("staged scene artifact identity changed")
        );
        assert_eq!(store.log(32, false).unwrap().total_commits, 0);
    }

    #[test]
    fn rejects_geojson_without_stable_feature_identity() {
        let workspace = TempDir::new().unwrap();
        let store = LocalEditorStore::default_for_workspace(workspace.path());
        store.init_project("atlas".to_owned()).unwrap();
        fs::write(
            workspace.path().join("features.geojson"),
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","geometry":{"type":"Point","coordinates":[0,0]},"properties":{}}]}"#,
        )
        .unwrap();
        declare_geojson_source(
            &store,
            "features.geojson",
            "features",
            SceneSourceRole::Features,
        );
        let error = store.status().unwrap_err();
        assert!(error.to_string().contains("stable string or number ids"));
    }

    #[test]
    fn missing_project_with_a_retained_index_fails_closed() {
        let workspace = TempDir::new().unwrap();
        let store = LocalEditorStore::default_for_workspace(workspace.path());
        store.init_project("atlas".to_owned()).unwrap();
        fs::write(
            workspace.path().join("features.geojson"),
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"poi","geometry":{"type":"Point","coordinates":[0,0]},"properties":{}}]}"#,
        )
        .unwrap();
        declare_geojson_source(
            &store,
            "features.geojson",
            "features",
            SceneSourceRole::Features,
        );
        store.add().unwrap();
        fs::remove_file(workspace.path().join(".rey/editor/project.json")).unwrap();

        let error = store.status().unwrap_err();
        assert!(
            error
                .to_string()
                .contains("while retained INDEX or HEAD exists")
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlinked_internal_project() {
        use std::os::unix::fs::symlink;

        let workspace = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let editor = workspace.path().join(".rey/editor");
        fs::create_dir_all(&editor).unwrap();
        fs::write(
            outside.path().join("project.json"),
            serde_json::to_vec_pretty(&super::EditorProject::new("atlas".to_owned()).unwrap())
                .unwrap(),
        )
        .unwrap();
        symlink(
            outside.path().join("project.json"),
            editor.join("project.json"),
        )
        .unwrap();

        let store = LocalEditorStore::default_for_workspace(workspace.path());
        let error = store.status().unwrap_err();
        assert!(error.to_string().contains("non-symlinked"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_source_and_content_store_ancestors() {
        use std::os::unix::fs::symlink;

        let workspace = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let store = LocalEditorStore::default_for_workspace(workspace.path());
        store.init_project("atlas".to_owned()).unwrap();
        fs::write(
            outside.path().join("features.geojson"),
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"poi","geometry":{"type":"Point","coordinates":[0,0]},"properties":{}}]}"#,
        )
        .unwrap();
        symlink(
            outside.path().join("features.geojson"),
            workspace.path().join("linked.geojson"),
        )
        .unwrap();
        declare_geojson_source(
            &store,
            "linked.geojson",
            "features",
            SceneSourceRole::Features,
        );
        let source_error = store.status().unwrap_err();
        assert!(source_error.to_string().contains("non-symlinked"));

        fs::write(
            workspace.path().join("features.geojson"),
            fs::read(outside.path().join("features.geojson")).unwrap(),
        )
        .unwrap();
        declare_geojson_source(
            &store,
            "features.geojson",
            "features",
            SceneSourceRole::Features,
        );
        store.add().unwrap();
        fs::remove_dir(workspace.path().join(".rey/editor/packages")).unwrap();
        symlink(
            outside.path(),
            workspace.path().join(".rey/editor/packages"),
        )
        .unwrap();
        let package_error = store.commit("blocked package".to_owned()).unwrap_err();
        assert!(package_error.to_string().contains("non-symlinked"));
    }
}
