use std::collections::BTreeSet;

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const EXPLORER_GRAMMAR_SCHEMA: &str = "rey.explore-grammar.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplorerSemanticLevel {
    World,
    Atlas,
    Landscape,
    Neighborhood,
    Object,
    Evidence,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplorerProjectionPosture {
    WorldGlobe,
    SemanticMercator,
    CountyIsometric,
    CountyEvidenceOverlay,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplorerLevelBand {
    pub level: ExplorerSemanticLevel,
    pub posture: ExplorerProjectionPosture,
    pub minimum_scale_microunits: u64,
    pub nominal_scale_microunits: u64,
    pub enter_scale_microunits: u64,
    pub leave_scale_microunits: u64,
    pub max_scene_objects: u64,
    pub max_labels: u64,
    pub max_pick_candidates: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplorerPostureMorph {
    pub source: ExplorerProjectionPosture,
    pub target: ExplorerProjectionPosture,
    pub start_scale_microunits: u64,
    pub end_scale_microunits: u64,
    pub geometry_policy: String,
    pub identity_policy: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplorerCameraConstraints {
    pub minimum_scale_microunits: u64,
    pub maximum_scale_microunits: u64,
    pub world_pitch_microdegrees: [i64; 2],
    pub atlas_pitch_microdegrees: i64,
    pub county_pitch_microdegrees: i64,
    pub county_yaw_microdegrees: i64,
    pub pointer_focus_rule: String,
    pub control_focus_rule: String,
    pub camera_identity_rule: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplorerPickingPolicy {
    pub world_inverse: String,
    pub atlas_inverse: String,
    pub county_inverse: String,
    pub fragment_identity_rule: String,
    pub selection_authority: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplorerMercatorPolicy {
    pub input_coordinate_space: String,
    pub longitude_wrap_microdegrees: i64,
    pub latitude_cutoff_microdegrees: i64,
    pub polar_policy: String,
    pub antimeridian_policy: String,
    pub distance_claim: String,
    pub native_crs_claim: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplorerGrammarLimits {
    pub max_levels: u64,
    pub max_posture_morphs: u64,
    pub max_scene_objects: u64,
    pub max_labels: u64,
    pub max_pick_candidates: u64,
    pub max_morph_fragments_per_identity: u64,
}

impl Default for ExplorerGrammarLimits {
    fn default() -> Self {
        Self {
            max_levels: 6,
            max_posture_morphs: 3,
            max_scene_objects: 4_096,
            max_labels: 256,
            max_pick_candidates: 64,
            max_morph_fragments_per_identity: 4,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplorerGrammar {
    pub schema: String,
    pub grammar_id: SemanticDigest,
    pub contract: ContractIdentity,
    pub scale_unit: String,
    pub levels: Vec<ExplorerLevelBand>,
    pub posture_morphs: Vec<ExplorerPostureMorph>,
    pub camera: ExplorerCameraConstraints,
    pub picking: ExplorerPickingPolicy,
    pub mercator: ExplorerMercatorPolicy,
    pub semantic_lod_rule: String,
    pub geometric_lod_rule: String,
    pub validity_rule: String,
    pub limits: ExplorerGrammarLimits,
}

impl ExplorerGrammar {
    pub fn v1() -> Result<Self, ExplorerGrammarError> {
        let levels = vec![
            level(
                ExplorerSemanticLevel::World,
                ExplorerProjectionPosture::WorldGlobe,
                50_000,
                100_000,
                50_000,
                240_000,
                512,
                70,
                32,
            ),
            level(
                ExplorerSemanticLevel::Atlas,
                ExplorerProjectionPosture::SemanticMercator,
                140_000,
                260_000,
                240_000,
                470_000,
                1_024,
                96,
                48,
            ),
            level(
                ExplorerSemanticLevel::Landscape,
                ExplorerProjectionPosture::CountyIsometric,
                370_000,
                580_000,
                470_000,
                870_000,
                2_048,
                128,
                64,
            ),
            level(
                ExplorerSemanticLevel::Neighborhood,
                ExplorerProjectionPosture::CountyIsometric,
                770_000,
                1_080_000,
                870_000,
                1_570_000,
                3_072,
                160,
                64,
            ),
            level(
                ExplorerSemanticLevel::Object,
                ExplorerProjectionPosture::CountyIsometric,
                1_470_000,
                2_050_000,
                1_570_000,
                2_850_000,
                4_096,
                192,
                64,
            ),
            level(
                ExplorerSemanticLevel::Evidence,
                ExplorerProjectionPosture::CountyEvidenceOverlay,
                2_750_000,
                3_550_000,
                2_850_000,
                5_400_000,
                4_096,
                256,
                64,
            ),
        ];
        let posture_morphs = vec![
            ExplorerPostureMorph {
                source: ExplorerProjectionPosture::WorldGlobe,
                target: ExplorerProjectionPosture::SemanticMercator,
                start_scale_microunits: 140_000,
                end_scale_microunits: 240_000,
                geometry_policy: "unwrap identical synthetic semantic vertices; split antimeridian draw fragments without splitting identity".to_owned(),
                identity_policy: "retain atlas revision, sector, region, selection, and focus".to_owned(),
            },
            ExplorerPostureMorph {
                source: ExplorerProjectionPosture::SemanticMercator,
                target: ExplorerProjectionPosture::CountyIsometric,
                start_scale_microunits: 370_000,
                end_scale_microunits: 470_000,
                geometry_policy: "expand only the selected admitted county footprint through its exact local tangent transform".to_owned(),
                identity_policy: "retain admitted regional scene, selected object, and exact source binding".to_owned(),
            },
            ExplorerPostureMorph {
                source: ExplorerProjectionPosture::CountyIsometric,
                target: ExplorerProjectionPosture::CountyEvidenceOverlay,
                start_scale_microunits: 2_750_000,
                end_scale_microunits: 2_850_000,
                geometry_policy: "retain county geometry and add exact evidence overlays without resampling source identity".to_owned(),
                identity_policy: "retain county, selected object, validity, limits, omissions, and lineage".to_owned(),
            },
        ];
        let mut grammar = Self {
            schema: EXPLORER_GRAMMAR_SCHEMA.to_owned(),
            grammar_id: placeholder_digest(),
            contract: ContractIdentity::new(
                "rey.explore-grammar",
                1,
                "renderer-independent World globe, semantic Mercator, County isometric, semantic/geometric LOD, camera, morph, and inverse-picking contract",
            ),
            scale_unit: "microunit (1.0 lens scale = 1000000)".to_owned(),
            levels,
            posture_morphs,
            camera: ExplorerCameraConstraints {
                minimum_scale_microunits: 50_000,
                maximum_scale_microunits: 5_400_000,
                world_pitch_microdegrees: [-62_000_000, 62_000_000],
                atlas_pitch_microdegrees: 0,
                county_pitch_microdegrees: 35_264_390,
                county_yaw_microdegrees: 45_000_000,
                pointer_focus_rule: "wheel zoom preserves the exact semantic coordinate beneath the pointer".to_owned(),
                control_focus_rule: "discrete controls preserve the selected semantic identity and cannot skip a semantic level".to_owned(),
                camera_identity_rule: "center, scale, viewport, pitch, yaw, and transient selection are view state and never resource identity".to_owned(),
            },
            picking: ExplorerPickingPolicy {
                world_inverse: "ray-to-synthetic-sphere, then exact atlas region/sector identity".to_owned(),
                atlas_inverse: "wrapped chart point to one synthetic longitude/latitude; draw fragments resolve to their shared identity".to_owned(),
                county_inverse: "screen ray to admitted county-local east/north/up frame, then exact scene object".to_owned(),
                fragment_identity_rule: "one semantic object may have bounded draw fragments; picking returns the unsplit object identity".to_owned(),
                selection_authority: "picking selects retained evidence only; it grants no locate, survey, admission, action, or proof authority".to_owned(),
            },
            mercator: ExplorerMercatorPolicy {
                input_coordinate_space: "synthetic_semantic_longitude_latitude".to_owned(),
                longitude_wrap_microdegrees: 360_000_000,
                latitude_cutoff_microdegrees: 85_051_129,
                polar_policy: "World retains polar caps; Atlas exposes clipped contents and never silently drops them".to_owned(),
                antimeridian_policy: "split drawing at the wrap while retaining one sector/region/object identity and inverse pick".to_owned(),
                distance_claim: "chart distance and area are presentation distortion, not semantic or physical distance".to_owned(),
                native_crs_claim: "semantic Mercator is not EPSG:3857 and never relabels OGC CRS84 native evidence".to_owned(),
            },
            semantic_lod_rule: "the six evidence levels have independent object and label budgets; level changes never change source truth".to_owned(),
            geometric_lod_rule: "renderer geometry may refine inside admitted validity only and may not mint coverage or source objects".to_owned(),
            validity_rule: "validity/no-data survives projection, morph, LOD, simulation, material, rendering, and picking".to_owned(),
            limits: ExplorerGrammarLimits::default(),
        };
        grammar.grammar_id = grammar_digest(&grammar)?;
        grammar.verify()?;
        Ok(grammar)
    }

    pub fn verify(&self) -> Result<(), ExplorerGrammarError> {
        if self.schema != EXPLORER_GRAMMAR_SCHEMA {
            return Err(ExplorerGrammarError::Schema);
        }
        if self.levels.len() as u64 != self.limits.max_levels
            || self.posture_morphs.len() as u64 > self.limits.max_posture_morphs
            || self.camera.minimum_scale_microunits >= self.camera.maximum_scale_microunits
            || self.camera.minimum_scale_microunits != 50_000
            || self.camera.maximum_scale_microunits != 5_400_000
        {
            return Err(ExplorerGrammarError::Bounds);
        }
        let expected_levels = [
            ExplorerSemanticLevel::World,
            ExplorerSemanticLevel::Atlas,
            ExplorerSemanticLevel::Landscape,
            ExplorerSemanticLevel::Neighborhood,
            ExplorerSemanticLevel::Object,
            ExplorerSemanticLevel::Evidence,
        ];
        if self
            .levels
            .iter()
            .map(|level| level.level)
            .collect::<Vec<_>>()
            != expected_levels
        {
            return Err(ExplorerGrammarError::LevelOrder);
        }
        let mut postures = BTreeSet::new();
        for level in &self.levels {
            if level.minimum_scale_microunits > level.nominal_scale_microunits
                || level.enter_scale_microunits > level.leave_scale_microunits
                || level.max_scene_objects > self.limits.max_scene_objects
                || level.max_labels > self.limits.max_labels
                || level.max_pick_candidates > self.limits.max_pick_candidates
            {
                return Err(ExplorerGrammarError::Bounds);
            }
            postures.insert(level.posture);
        }
        for morph in &self.posture_morphs {
            if morph.start_scale_microunits >= morph.end_scale_microunits
                || !postures.contains(&morph.source)
                || !postures.contains(&morph.target)
            {
                return Err(ExplorerGrammarError::Morph);
            }
        }
        if self.mercator.input_coordinate_space != "synthetic_semantic_longitude_latitude"
            || self.mercator.longitude_wrap_microdegrees != 360_000_000
            || self.mercator.latitude_cutoff_microdegrees != 85_051_129
        {
            return Err(ExplorerGrammarError::Mercator);
        }
        if self.grammar_id != grammar_digest(self)? {
            return Err(ExplorerGrammarError::Identity);
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
const fn level(
    level: ExplorerSemanticLevel,
    posture: ExplorerProjectionPosture,
    minimum_scale_microunits: u64,
    nominal_scale_microunits: u64,
    enter_scale_microunits: u64,
    leave_scale_microunits: u64,
    max_scene_objects: u64,
    max_labels: u64,
    max_pick_candidates: u64,
) -> ExplorerLevelBand {
    ExplorerLevelBand {
        level,
        posture,
        minimum_scale_microunits,
        nominal_scale_microunits,
        enter_scale_microunits,
        leave_scale_microunits,
        max_scene_objects,
        max_labels,
        max_pick_candidates,
    }
}

fn placeholder_digest() -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.explore-grammar.placeholder.v1");
    hasher.add_str("excluded from identity");
    hasher.finish()
}

fn grammar_digest(grammar: &ExplorerGrammar) -> Result<SemanticDigest, ExplorerGrammarError> {
    let mut normalized = grammar.clone();
    normalized.grammar_id = placeholder_digest();
    let mut hasher = SemanticHasher::new(EXPLORER_GRAMMAR_SCHEMA);
    hasher.add_str(normalized.contract.id.as_str());
    hasher.add_str(&normalized.contract.revision.to_string());
    hasher.add_str(normalized.contract.semantic_digest.as_str());
    hasher.add_bytes(&serde_json::to_vec(&normalized)?);
    Ok(hasher.finish())
}

#[derive(Debug, Error)]
pub enum ExplorerGrammarError {
    #[error("Explorer grammar schema is unsupported")]
    Schema,
    #[error("Explorer grammar level order is not canonical")]
    LevelOrder,
    #[error("Explorer grammar scale or LOD bounds are invalid")]
    Bounds,
    #[error("Explorer posture morph is invalid")]
    Morph,
    #[error("Explorer semantic Mercator policy is invalid")]
    Mercator,
    #[error("Explorer grammar identity does not match its semantic content")]
    Identity,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammar_is_deterministic_and_keeps_lod_separate_from_posture() {
        let first = ExplorerGrammar::v1().unwrap();
        let second = ExplorerGrammar::v1().unwrap();
        assert_eq!(first, second);
        assert_eq!(first.levels.len(), 6);
        assert_eq!(first.levels[2].posture, first.levels[4].posture);
        assert_ne!(first.levels[2].level, first.levels[4].level);
        assert_eq!(first.mercator.latitude_cutoff_microdegrees, 85_051_129);
    }

    #[test]
    fn grammar_rejects_identity_and_coordinate_policy_tampering() {
        let grammar = ExplorerGrammar::v1().unwrap();
        let mut changed = grammar.clone();
        changed.mercator.input_coordinate_space = "OGC:CRS84".to_owned();
        assert!(matches!(
            changed.verify(),
            Err(ExplorerGrammarError::Mercator)
        ));

        let mut changed = grammar;
        changed.levels[0].max_labels += 1;
        assert!(matches!(
            changed.verify(),
            Err(ExplorerGrammarError::Identity)
        ));
    }
}
