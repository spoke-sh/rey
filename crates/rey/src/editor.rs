#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use rey_core::{SemanticDigest, SemanticHasher};
use rey_diff::DeltaAssessment;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const EDITOR_PROJECT_SCHEMA: &str = "rey.editor-project.v1";
pub const SCENE_CANDIDATE_SNAPSHOT_SCHEMA: &str = "rey.scene-candidate-snapshot.v1";
pub const SCENE_CHANGE_SET_SCHEMA: &str = "rey.scene-change-set.v1";
pub const SCENE_PACKAGE_SCHEMA: &str = "rey.scene-package.v1";
pub const SCENE_ADMISSION_REQUEST_SCHEMA: &str = "rey.scene-admission-request.v1";
pub const EDITOR_STATUS_SCHEMA: &str = "rey.editor-status.v1";
pub const EDITOR_STATE_SCHEMA: &str = "rey.editor-state.v1";
pub const EDITOR_ADD_RESULT_SCHEMA: &str = "rey.editor-add-result.v1";
pub const EDITOR_PACKAGE_RESULT_SCHEMA: &str = "rey.editor-package-result.v1";
pub const EDITOR_IMPORT_RESULT_SCHEMA: &str = "rey.editor-import-result.v1";

const STATE_FILE_NAME: &str = "state.json";
const LOCK_FILE_NAME: &str = "editor.lock";
const MAX_PROJECT_BYTES: u64 = 1_048_576;
const MAX_SOURCE_BYTES: u64 = 16 * 1_048_576;
const MAX_STATE_BYTES: u64 = 32 * 1_048_576;
const MAX_SOURCES: usize = 64;
const MAX_FEATURES: usize = 10_000;
const MAX_COORDINATES: usize = 1_000_000;
const MAX_PROPERTIES: usize = 64;
const MAX_PROPERTIES_BYTES: usize = 65_536;
const MAX_IDENTIFIER_CHARS: usize = 96;
const MAX_LABEL_CHARS: usize = 160;

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
    TerrainControl,
    Hydrology,
    Boundary,
}

impl SceneSourceRole {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Features => "features",
            Self::Markers => "markers",
            Self::TerrainControl => "terrain_control",
            Self::Hydrology => "hydrology",
            Self::Boundary => "boundary",
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EditorStatus {
    pub schema: String,
    pub state: EditorWorkingState,
    pub package: Option<ScenePackageReference>,
    pub index: Option<SceneCandidateSnapshot>,
    pub working: SceneCandidateSnapshot,
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
pub struct EditorPackageResult {
    pub schema: String,
    pub created: bool,
    pub package: ScenePackage,
    pub admission_request: SceneAdmissionRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EditorImportResult {
    pub schema: String,
    pub imported: bool,
    pub project_path: String,
    pub source: SceneSourceDeclaration,
    pub feature_count: u64,
    pub coordinate_count: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct EditorStateDocument {
    schema: String,
    package: Option<ScenePackageReference>,
    index: Option<SceneCandidateSnapshot>,
}

impl Default for EditorStateDocument {
    fn default() -> Self {
        Self {
            schema: EDITOR_STATE_SCHEMA.to_owned(),
            package: None,
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

    pub fn init_project(
        &self,
        project_path: &Path,
        project_id: String,
    ) -> Result<EditorProject, EditorError> {
        let project = EditorProject::new(project_id)?.canonicalize()?;
        let path = self.resolve_new_project_path(project_path)?;
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

    pub fn import_geojson(
        &self,
        project_path: &Path,
        source_path: &Path,
        source_id: String,
        role: SceneSourceRole,
    ) -> Result<EditorImportResult, EditorError> {
        validate_identifier("source id", &source_id)?;
        let (project_file, mut project) = self.load_project(project_path)?;
        let relative_source = workspace_relative(&self.workspace, source_path, MAX_SOURCE_BYTES)?;
        let source_bytes = read_bounded_file(
            &self.workspace.join(&relative_source),
            MAX_SOURCE_BYTES,
            "scene source",
        )?;
        let parsed = parse_geojson(&source_id, role, &source_bytes)?;
        let source = SceneSourceDeclaration {
            source_id,
            path: path_string(&relative_source)?,
            format: SceneSourceFormat::GeoJson,
            role,
        };
        if let Some(existing) = project
            .sources
            .iter()
            .find(|existing| existing.source_id == source.source_id)
        {
            if existing == &source {
                return Ok(EditorImportResult {
                    schema: EDITOR_IMPORT_RESULT_SCHEMA.to_owned(),
                    imported: false,
                    project_path: path_string(project_path)?,
                    source,
                    feature_count: parsed.features.len() as u64,
                    coordinate_count: parsed.coordinate_count,
                });
            }
            return Err(EditorError::DuplicateSourceId(source.source_id));
        }
        if project
            .sources
            .iter()
            .any(|existing| existing.path == source.path)
        {
            return Err(EditorError::DuplicateSourcePath(source.path));
        }
        project.sources.push(source.clone());
        project = project.canonicalize()?;
        self.write_project(&project_file, &project)?;
        Ok(EditorImportResult {
            schema: EDITOR_IMPORT_RESULT_SCHEMA.to_owned(),
            imported: true,
            project_path: path_string(project_path)?,
            source,
            feature_count: parsed.features.len() as u64,
            coordinate_count: parsed.coordinate_count,
        })
    }

    pub fn validate(&self, project_path: &Path) -> Result<SceneCandidateSnapshot, EditorError> {
        Ok(self.observe(project_path)?.snapshot)
    }

    pub fn status(&self, project_path: &Path) -> Result<EditorStatus, EditorError> {
        let working = self.observe(project_path)?.snapshot;
        let state = self.load_state()?;
        let package = self.load_package_reference(state.package.as_ref())?;
        let package_snapshot = package.as_ref().map(|package| &package.snapshot);
        let staged = SceneChangeSet::derive(
            "PACKAGE",
            package_snapshot,
            "INDEX",
            state.index.as_ref().or(package_snapshot),
        );
        let unstaged = SceneChangeSet::derive(
            "INDEX",
            state.index.as_ref().or(package_snapshot),
            "WORKING",
            Some(&working),
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
            state: state_kind,
            package: state.package,
            index: state.index,
            working,
            staged,
            unstaged,
            admission_boundary: "candidate only; no scene package is admitted until a validated rey.scene-admission workload retains an admitted projection".to_owned(),
        })
    }

    pub fn diff(&self, project_path: &Path, staged: bool) -> Result<SceneChangeSet, EditorError> {
        let status = self.status(project_path)?;
        Ok(if staged {
            status.staged
        } else {
            status.unstaged
        })
    }

    pub fn add(&self, project_path: &Path) -> Result<EditorAddResult, EditorError> {
        self.with_lock(|| {
            let observed = self.observe(project_path)?;
            let mut state = self.load_state()?;
            let package = self.load_package_reference(state.package.as_ref())?;
            let package_snapshot = package.as_ref().map(|package| &package.snapshot);
            let delta = SceneChangeSet::derive(
                "PACKAGE",
                package_snapshot,
                "INDEX",
                Some(&observed.snapshot),
            );
            self.write_artifacts(&observed)?;
            let staged = state.index.as_ref() != Some(&observed.snapshot);
            state.index = Some(observed.snapshot.clone());
            self.save_state(&state)?;
            Ok(EditorAddResult {
                schema: EDITOR_ADD_RESULT_SCHEMA.to_owned(),
                staged,
                snapshot: observed.snapshot,
                delta,
            })
        })
    }

    pub fn package(&self) -> Result<EditorPackageResult, EditorError> {
        self.with_lock(|| {
            let mut state = self.load_state()?;
            let snapshot = state.index.clone().ok_or(EditorError::EmptyIndex)?;
            snapshot.verify()?;
            self.verify_staged_artifacts(&snapshot)?;
            let parent = self.load_package_reference(state.package.as_ref())?;
            if let (Some(reference), Some(parent)) = (state.package.as_ref(), parent.as_ref())
                && parent.snapshot.snapshot_revision == snapshot.snapshot_revision
            {
                let request = self.load_admission_request(&reference.admission_request_path)?;
                return Ok(EditorPackageResult {
                    schema: EDITOR_PACKAGE_RESULT_SCHEMA.to_owned(),
                    created: false,
                    package: parent.clone(),
                    admission_request: request,
                });
            }
            let parent_snapshot = parent.as_ref().map(|package| &package.snapshot);
            let change_set =
                SceneChangeSet::derive("PACKAGE", parent_snapshot, "CANDIDATE", Some(&snapshot));
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
            let created = write_content_addressed_json(&package_path, &package)?;
            let mut request = SceneAdmissionRequest {
                schema: SCENE_ADMISSION_REQUEST_SCHEMA.to_owned(),
                request_id: digest_placeholder(),
                package_id: package.package_id.clone(),
                package_path: package_relative.clone(),
                requested_operation: "rey.scene-admission.validate@1".to_owned(),
                status: "requires_workload".to_owned(),
                admitted: false,
            };
            request.request_id = admission_request_identity(&request);
            let request_relative = format!("requests/{}.json", digest_key(&request.request_id));
            write_content_addressed_json(&self.directory.join(&request_relative), &request)?;
            state.package = Some(ScenePackageReference {
                package_id: package.package_id.clone(),
                snapshot_revision: package.snapshot.snapshot_revision.clone(),
                package_path: package_relative,
                admission_request_path: request_relative,
            });
            self.save_state(&state)?;
            Ok(EditorPackageResult {
                schema: EDITOR_PACKAGE_RESULT_SCHEMA.to_owned(),
                created,
                package,
                admission_request: request,
            })
        })
    }

    pub fn inspect(&self, package_id: &str) -> Result<ScenePackage, EditorError> {
        let key = validate_digest_argument(package_id)?;
        let package = self.load_package_path(&format!("packages/{key}.json"))?;
        if package.package_id.as_str() != package_id {
            return Err(EditorError::UnknownPackage(package_id.to_owned()));
        }
        Ok(package)
    }

    fn observe(&self, project_path: &Path) -> Result<ObservedScene, EditorError> {
        let (_, project) = self.load_project(project_path)?;
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

    fn load_project(&self, project_path: &Path) -> Result<(PathBuf, EditorProject), EditorError> {
        let relative = workspace_relative(&self.workspace, project_path, MAX_PROJECT_BYTES)?;
        let path = self.workspace.join(relative);
        let bytes = read_bounded_file(&path, MAX_PROJECT_BYTES, "editor project")?;
        let project: EditorProject = serde_json::from_slice(&bytes)?;
        let project = project.canonicalize()?;
        Ok((path, project))
    }

    fn resolve_new_project_path(&self, project_path: &Path) -> Result<PathBuf, EditorError> {
        validate_path_argument(project_path)?;
        let path = self.workspace.join(project_path);
        let parent = path
            .parent()
            .ok_or_else(|| EditorError::Path(project_path.to_owned()))?;
        let canonical_parent = parent.canonicalize().map_err(|source| EditorError::Read {
            path: parent.to_owned(),
            source,
        })?;
        if !canonical_parent.starts_with(&self.workspace) {
            return Err(EditorError::PathEscape(project_path.to_owned()));
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
        Ok(state)
    }

    fn save_state(&self, state: &EditorStateDocument) -> Result<(), EditorError> {
        self.prepare_directory()?;
        let bytes = serde_json::to_vec_pretty(state)?;
        if bytes.len().saturating_add(1) as u64 > MAX_STATE_BYTES {
            return Err(EditorError::StateLimit(MAX_STATE_BYTES));
        }
        write_atomic(&self.directory.join(STATE_FILE_NAME), &bytes)
    }

    fn load_package_reference(
        &self,
        reference: Option<&ScenePackageReference>,
    ) -> Result<Option<ScenePackage>, EditorError> {
        reference
            .map(|reference| self.load_package_path(&reference.package_path))
            .transpose()
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
        if role == SceneSourceRole::Markers
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
        let feature_id = format!("{source_id}/{source_feature_id}");
        let feature_bytes = serde_json::to_vec(feature)?;
        let feature_revision = feature_identity(source_id, role, &feature_bytes);
        let bounds = geometry_summary
            .bounds
            .ok_or_else(|| EditorError::MissingGeometry(source_feature_id.clone()))?;
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
        });
    }
    features.sort_by(|left, right| left.feature_id.cmp(&right.feature_id));
    Ok(ParsedGeoJson {
        features,
        coordinate_count: source_coordinates,
        bounds: source_bounds,
    })
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

fn validate_digest_argument(value: &str) -> Result<&str, EditorError> {
    let key = value
        .strip_prefix("blake3:")
        .ok_or_else(|| EditorError::UnknownPackage(value.to_owned()))?;
    if key.len() != 64 || !key.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(EditorError::UnknownPackage(value.to_owned()));
    }
    Ok(key)
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
    #[error("duplicate scene feature id: {0}")]
    DuplicateFeatureId(String),
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
    #[error("editor index is empty; run `rey editor add` before packaging")]
    EmptyIndex,
    #[error("unknown current scene package: {0}")]
    UnknownPackage(String),
    #[error("editor JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("editor filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use rey_diff::DeltaAssessment;
    use tempfile::TempDir;

    use super::{LocalEditorStore, SceneSourceRole};

    #[test]
    fn stages_exact_native_geojson_and_packages_only_the_index() {
        let workspace = TempDir::new().unwrap();
        let store = LocalEditorStore::default_for_workspace(workspace.path());
        store
            .init_project(Path::new("scene.json"), "atlas".to_owned())
            .unwrap();
        fs::write(
            workspace.path().join("markers.geojson"),
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"ridge","geometry":{"type":"Point","coordinates":[-122.4,37.8]},"properties":{"title":"Ridge","category":"survey","min_zoom":3,"collision_priority":10}}]}"#,
        )
        .unwrap();
        store
            .import_geojson(
                Path::new("scene.json"),
                Path::new("markers.geojson"),
                "survey-poi".to_owned(),
                SceneSourceRole::Markers,
            )
            .unwrap();

        let working = store.status(Path::new("scene.json")).unwrap();
        assert_eq!(working.working.coverage.markers, 1);
        assert_eq!(working.unstaged.inserted, 2);
        let added = store.add(Path::new("scene.json")).unwrap();
        assert!(added.staged);
        let packaged = store.package().unwrap();
        assert!(!packaged.admission_request.admitted);
        assert_eq!(packaged.admission_request.status, "requires_workload");
        let reused = store.package().unwrap();
        assert!(!reused.created);
        assert_eq!(reused.package.package_id, packaged.package.package_id);

        fs::write(
            workspace.path().join("markers.geojson"),
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"ridge","geometry":{"type":"Point","coordinates":[-122.5,37.8]},"properties":{"title":"Ridge"}}]}"#,
        )
        .unwrap();
        let second = store.status(Path::new("scene.json")).unwrap();
        assert_eq!(second.staged.assessment, DeltaAssessment::Equal);
        assert_eq!(second.unstaged.modified, 2);
        store.add(Path::new("scene.json")).unwrap();
        let second_package = store.package().unwrap();
        assert_ne!(
            second_package.package.package_id,
            packaged.package.package_id
        );
        let retained = store.inspect(packaged.package.package_id.as_str()).unwrap();
        assert_eq!(retained.snapshot.features[0].bounds.west, -122.4);
    }

    #[test]
    fn rejects_geojson_without_stable_feature_identity() {
        let workspace = TempDir::new().unwrap();
        let store = LocalEditorStore::default_for_workspace(workspace.path());
        store
            .init_project(Path::new("scene.json"), "atlas".to_owned())
            .unwrap();
        fs::write(
            workspace.path().join("features.geojson"),
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","geometry":{"type":"Point","coordinates":[0,0]},"properties":{}}]}"#,
        )
        .unwrap();
        let error = store
            .import_geojson(
                Path::new("scene.json"),
                Path::new("features.geojson"),
                "features".to_owned(),
                SceneSourceRole::Features,
            )
            .unwrap_err();
        assert!(error.to_string().contains("stable string or number ids"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_source_and_content_store_ancestors() {
        use std::os::unix::fs::symlink;

        let workspace = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let store = LocalEditorStore::default_for_workspace(workspace.path());
        store
            .init_project(Path::new("scene.json"), "atlas".to_owned())
            .unwrap();
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
        let source_error = store
            .import_geojson(
                Path::new("scene.json"),
                Path::new("linked.geojson"),
                "features".to_owned(),
                SceneSourceRole::Features,
            )
            .unwrap_err();
        assert!(source_error.to_string().contains("non-symlinked"));

        fs::write(
            workspace.path().join("features.geojson"),
            fs::read(outside.path().join("features.geojson")).unwrap(),
        )
        .unwrap();
        store
            .import_geojson(
                Path::new("scene.json"),
                Path::new("features.geojson"),
                "features".to_owned(),
                SceneSourceRole::Features,
            )
            .unwrap();
        store.add(Path::new("scene.json")).unwrap();
        fs::remove_dir(workspace.path().join(".rey/editor/packages")).unwrap();
        symlink(
            outside.path(),
            workspace.path().join(".rey/editor/packages"),
        )
        .unwrap();
        let package_error = store.package().unwrap_err();
        assert!(package_error.to_string().contains("non-symlinked"));
    }
}
