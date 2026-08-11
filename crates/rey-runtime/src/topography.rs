use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_locator::{
    CoordinateBinding, CoordinateIdentityClass, LocalCoordinate, Locator, LocatorKind,
    LocatorResolution, ResolutionLimits, ResolutionStatus,
};
use rey_mining::{
    SurveySeed, SurveySeedState, TopographyAnchor, TopographyAnchorKind, TopographyCoverage,
    TopographyEdge, TopographyFrontierRow, TopographyLimits, TopographyLineage,
    TopographyLocatorCandidate, TopographyOmission, TopographyPatch, TopographyPatchParts,
    TopographyRegion, TopographyRegionState, TopographyRelationshipKind, anchor_identity,
    candidate_identity, edge_identity, frontier_identity, region_identity,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CONTEXT_ANCHOR_SURVEY_WORKLOAD_ID: &str = "context-anchor-survey";
pub const CONTEXT_ANCHOR_SURVEY_OPERATION_ID: &str = "rey.context-anchor-survey.locate";
pub const RENDER_TOPOGRAPHY_PATCH_OPERATION_ID: &str = "rey.topography-patch.render-lines";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopographySurveyScenario {
    pub fixture_project: String,
    pub seed_paths: Vec<String>,
    #[serde(default)]
    pub limits: TopographyLimits,
    #[serde(default)]
    pub resolution_limits: ResolutionLimits,
}

#[derive(Clone, Debug)]
pub struct TopographySurveyInput {
    pub root: PathBuf,
    pub relative_paths: Vec<PathBuf>,
    pub capability_snapshot_id: SemanticDigest,
    pub limits: TopographyLimits,
    pub resolution_limits: ResolutionLimits,
    pub prior: Option<TopographyPatch>,
}

pub struct TopographyExecutionContext<'a> {
    pub workload: &'a ContractIdentity,
    pub graph: &'a ContractIdentity,
    pub scenario: Option<&'a ContractIdentity>,
    pub campaign_id: &'a SemanticDigest,
    pub graph_node_id: &'a str,
    pub declared_seeds: &'a str,
    pub input: &'a TopographySurveyInput,
}

#[must_use]
pub fn context_anchor_survey_operation_contract() -> ContractIdentity {
    ContractIdentity::new(
        CONTEXT_ANCHOR_SURVEY_OPERATION_ID,
        1,
        "read only explicitly selected bounded markdown seeds, locate URI/reference candidates, resolve only local workspace references under frozen authority, and emit rey.topography-patch.v1",
    )
}

#[must_use]
pub fn context_anchor_survey_implementation_contract() -> ContractIdentity {
    ContractIdentity::new(
        "rey.context-anchor-survey.builtin",
        1,
        "deterministic markdown target scanner and workspace-confined local resolver",
    )
}

#[must_use]
pub fn local_topography_provider_contract() -> ContractIdentity {
    ContractIdentity::new(
        "rey.provider.local-worktree-topography",
        1,
        "bounded read-only regular-file survey below one canonical workspace root; rejects symlinks and path escape",
    )
}

#[must_use]
pub fn render_topography_patch_contract() -> ContractIdentity {
    ContractIdentity::new(
        RENDER_TOPOGRAPHY_PATCH_OPERATION_ID,
        1,
        "render one verified topography patch as deterministic ordered UTF-8 evidence lines",
    )
}

pub fn topography_fixture_root(project: &str) -> Result<PathBuf, TopographySurveyError> {
    if project.is_empty()
        || !project
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(TopographySurveyError::Fixture(project.to_owned()));
    }
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/topography-projects")
        .join(project))
}

pub fn execute_context_anchor_survey(
    context: TopographyExecutionContext<'_>,
) -> Result<TopographyPatch, TopographySurveyError> {
    validate_input(&context)?;
    let provider = local_topography_provider_contract();
    let implementation = context_anchor_survey_implementation_contract();
    let operation = context_anchor_survey_operation_contract();
    let paths = context
        .input
        .relative_paths
        .iter()
        .map(|path| normalize_declared_path(path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut observations = Vec::new();
    let mut total_bytes = 0_u64;
    let mut omissions = Vec::new();
    let mut complete = true;
    for (index, path) in paths.iter().enumerate() {
        if index as u64 >= context.input.limits.max_seeds {
            complete = false;
            omissions.push(TopographyOmission {
                kind: "seed_limit".to_owned(),
                subject: path.clone(),
                omitted_count: (paths.len() - index) as u64,
                reason: format!("seed limit {} reached", context.input.limits.max_seeds),
            });
            break;
        }
        let observed = observe_seed(
            &context.input.root,
            path,
            total_bytes,
            &context.input.limits,
            &provider,
        )?;
        total_bytes = total_bytes.saturating_add(observed.bytes.len() as u64);
        if observed.state == SurveySeedState::Omitted {
            complete = false;
            omissions.push(TopographyOmission {
                kind: "byte_limit".to_owned(),
                subject: path.clone(),
                omitted_count: 1,
                reason: observed.detail.clone(),
            });
        }
        observations.push(observed);
    }

    let workspace_revision = workspace_revision(&observations);
    let workspace_coordinate = local_binding(&provider, "workspace", ".", &workspace_revision)?;
    let mut anchors = vec![TopographyAnchor {
        anchor_id: anchor_identity(&workspace_coordinate, TopographyAnchorKind::Workspace),
        coordinate: workspace_coordinate.clone(),
        kind: TopographyAnchorKind::Workspace,
        label: "workspace survey boundary".to_owned(),
        source_revision: workspace_revision.clone(),
    }];
    let mut edges = Vec::new();
    let mut candidates = Vec::new();
    let mut resolutions = Vec::new();
    let mut frontier = Vec::new();
    let mut seen_candidates = BTreeSet::new();
    let mut seen_anchor_coordinates = BTreeSet::from([workspace_coordinate.coordinate.clone()]);
    let mut seen_edges = BTreeSet::new();
    let mut seen_resolutions = BTreeSet::new();
    let mut seeds = Vec::new();

    for observed in &observations {
        let Some(seed_coordinate) = &observed.coordinate else {
            seeds.push(observed.to_seed(0));
            continue;
        };
        if seen_anchor_coordinates.insert(seed_coordinate.coordinate.clone()) {
            anchors.push(TopographyAnchor {
                anchor_id: anchor_identity(seed_coordinate, TopographyAnchorKind::File),
                coordinate: seed_coordinate.clone(),
                kind: TopographyAnchorKind::File,
                label: observed.path.clone(),
                source_revision: observed.revision.clone().unwrap_or_default(),
            });
        }
        let contains_id = edge_identity(
            &workspace_coordinate.coordinate,
            &seed_coordinate.coordinate,
            TopographyRelationshipKind::Contains,
            &observed.path,
            &workspace_revision,
        );
        if seen_edges.insert(contains_id.clone()) {
            edges.push(TopographyEdge {
                edge_id: contains_id,
                source_coordinate: workspace_coordinate.coordinate.clone(),
                target_coordinate: seed_coordinate.coordinate.clone(),
                kind: TopographyRelationshipKind::Contains,
                locator: observed.path.clone(),
                evidence_revision: workspace_revision.clone(),
            });
        }

        let extracted = extract_markdown_targets(&observed.bytes);
        let mut admitted_count = 0_u64;
        for extracted in extracted {
            if candidates.len() as u64 >= context.input.limits.max_candidates {
                complete = false;
                omissions.push(TopographyOmission {
                    kind: "candidate_limit".to_owned(),
                    subject: observed.path.clone(),
                    omitted_count: 1,
                    reason: format!(
                        "candidate limit {} reached; remaining seed content was not interpreted",
                        context.input.limits.max_candidates
                    ),
                });
                break;
            }
            admitted_count += 1;
            let duplicate = !seen_candidates.insert(extracted.raw.clone());
            let candidate_id = candidate_identity(
                &seed_coordinate.coordinate,
                &extracted.raw,
                extracted.start_byte,
                extracted.end_byte,
            );
            candidates.push(TopographyLocatorCandidate {
                candidate_id,
                seed_coordinate: seed_coordinate.coordinate.clone(),
                seed_revision: observed.revision.clone().unwrap_or_default(),
                raw: extracted.raw.clone(),
                start_byte: extracted.start_byte,
                end_byte: extracted.end_byte,
                relationship: "references".to_owned(),
                duplicate,
            });
            let resolution = resolve_candidate(
                &context.input.root,
                &observed.path,
                observed.revision.as_deref().unwrap_or_default(),
                &extracted.raw,
                &provider,
                context.input.capability_snapshot_id.clone(),
                context.input.resolution_limits.clone(),
            )?;
            let resolution_is_new = seen_resolutions.insert(resolution.resolution_id.clone());
            if let Some(binding) = &resolution.coordinate {
                if seen_anchor_coordinates.insert(binding.coordinate.clone()) {
                    anchors.push(TopographyAnchor {
                        anchor_id: anchor_identity(binding, TopographyAnchorKind::File),
                        coordinate: binding.clone(),
                        kind: TopographyAnchorKind::File,
                        label: resolution.locator.as_ref().map_or_else(
                            || extracted.raw.clone(),
                            |locator| locator.payload.clone(),
                        ),
                        source_revision: binding.source_revision.clone(),
                    });
                }
                let edge_id = edge_identity(
                    &seed_coordinate.coordinate,
                    &binding.coordinate,
                    TopographyRelationshipKind::References,
                    &extracted.raw,
                    observed.revision.as_deref().unwrap_or_default(),
                );
                if seen_edges.insert(edge_id.clone()) {
                    edges.push(TopographyEdge {
                        edge_id,
                        source_coordinate: seed_coordinate.coordinate.clone(),
                        target_coordinate: binding.coordinate.clone(),
                        kind: TopographyRelationshipKind::References,
                        locator: extracted.raw.clone(),
                        evidence_revision: observed.revision.clone().unwrap_or_default(),
                    });
                }
            } else if resolution_is_new {
                frontier.push(TopographyFrontierRow {
                    row_id: frontier_identity(
                        &seed_coordinate.coordinate,
                        &extracted.raw,
                        resolution.status,
                    ),
                    source_coordinate: seed_coordinate.coordinate.clone(),
                    locator: extracted.raw.clone(),
                    status: resolution.status,
                    reason: resolution.detail.clone(),
                });
            }
            if resolution_is_new {
                resolutions.push(resolution);
            }
        }
        seeds.push(observed.to_seed(admitted_count));
    }

    let surveyed_count = seeds
        .iter()
        .filter(|seed| {
            matches!(
                seed.state,
                SurveySeedState::Surveyed | SurveySeedState::SurveyedEmpty
            )
        })
        .count() as u64;
    let state = if candidates.is_empty() {
        TopographyRegionState::SurveyedEmpty
    } else {
        TopographyRegionState::Surveyed
    };
    let mut regions = vec![region(
        &workspace_coordinate.coordinate,
        state,
        surveyed_count,
        candidates.len() as u64,
        "declared seed boundary",
    )];
    regions.push(derived_region(
        &workspace_revision,
        "unexplored",
        TopographyRegionState::Unexplored,
        "outside the admitted seed boundary; no survey claim",
    )?);
    if !frontier.is_empty() {
        regions.push(derived_region(
            &workspace_revision,
            "frontier",
            TopographyRegionState::Frontier,
            "unresolved locator boundary",
        )?);
    }
    if resolutions
        .iter()
        .any(|resolution| resolution.status == ResolutionStatus::Unsupported)
    {
        regions.push(derived_region(
            &workspace_revision,
            "unsupported",
            TopographyRegionState::Unsupported,
            "located coordinates whose resolver is not admitted",
        )?);
    }
    if !omissions.is_empty() {
        regions.push(derived_region(
            &workspace_revision,
            "omitted",
            TopographyRegionState::Omitted,
            "survey work stopped at an explicit bound",
        )?);
    }

    let coverage = TopographyCoverage {
        requested_seeds: paths.len() as u64,
        surveyed_seeds: seeds
            .iter()
            .filter(|seed| {
                matches!(
                    seed.state,
                    SurveySeedState::Surveyed | SurveySeedState::SurveyedEmpty
                )
            })
            .count() as u64,
        surveyed_empty_seeds: seeds
            .iter()
            .filter(|seed| seed.state == SurveySeedState::SurveyedEmpty)
            .count() as u64,
        missing_seeds: seeds
            .iter()
            .filter(|seed| seed.state == SurveySeedState::Missing)
            .count() as u64,
        omitted_seeds: paths.len().saturating_sub(seeds.len()) as u64
            + seeds
                .iter()
                .filter(|seed| seed.state == SurveySeedState::Omitted)
                .count() as u64,
        candidates: candidates.len() as u64,
        unique_candidates: candidates
            .iter()
            .map(|candidate| candidate.raw.as_str())
            .collect::<BTreeSet<_>>()
            .len() as u64,
        resolved_candidates: resolutions
            .iter()
            .filter(|resolution| resolution.status == ResolutionStatus::Resolved)
            .count() as u64,
        unresolved_candidates: resolutions
            .iter()
            .filter(|resolution| resolution.status != ResolutionStatus::Resolved)
            .count() as u64,
    };
    let execution_id = survey_execution_id(&context, &workspace_revision, &candidates);
    TopographyPatch::from_parts(
        TopographyPatchParts {
            workload: context.workload.clone(),
            graph: context.graph.clone(),
            scenario: context.scenario.cloned(),
            campaign_id: context.campaign_id.clone(),
            execution_id,
            operation: operation.clone(),
            implementation: implementation.clone(),
            provider: provider.clone(),
            capability_snapshot_id: context.input.capability_snapshot_id.clone(),
            limits: context.input.limits.clone(),
            complete,
            seeds,
            candidates,
            resolutions,
            anchors,
            edges,
            regions,
            coverage,
            frontier,
            omissions,
            lineage: vec![
                TopographyLineage {
                    kind: "implementation".to_owned(),
                    identity: implementation.id.clone(),
                    revision: implementation.semantic_digest.to_string(),
                },
                TopographyLineage {
                    kind: "operation".to_owned(),
                    identity: operation.id.clone(),
                    revision: operation.semantic_digest.to_string(),
                },
                TopographyLineage {
                    kind: "provider".to_owned(),
                    identity: provider.id.clone(),
                    revision: provider.semantic_digest.to_string(),
                },
            ],
        },
        context.input.prior.as_ref(),
    )
    .map_err(TopographySurveyError::Topography)
}

#[must_use]
pub fn render_topography_patch(patch: &TopographyPatch) -> String {
    let mut lines = vec![
        "TOPOGRAPHY PATCH".to_owned(),
        format!(
            "SEEDS {}/{} · EMPTY {} · MISSING {} · OMITTED {}",
            patch.coverage.surveyed_seeds,
            patch.coverage.requested_seeds,
            patch.coverage.surveyed_empty_seeds,
            patch.coverage.missing_seeds,
            patch.coverage.omitted_seeds,
        ),
        format!(
            "CANDIDATES {} · UNIQUE {} · RESOLVED {} · FRONTIER {}",
            patch.coverage.candidates,
            patch.coverage.unique_candidates,
            patch.coverage.resolved_candidates,
            patch.coverage.unresolved_candidates,
        ),
        format!(
            "MAP {} anchors · {} edges · {} regions · {} omissions · {}",
            patch.anchors.len(),
            patch.edges.len(),
            patch.regions.len(),
            patch.omissions.len(),
            if patch.complete {
                "complete"
            } else {
                "bounded"
            },
        ),
        format!(
            "DELTA SOURCE → TARGET · +{} -{} ~{}",
            patch.delta.inserted, patch.delta.deleted, patch.delta.modified,
        ),
    ];
    for seed in &patch.seeds {
        lines.push(format!(
            "SEED {} · {} · {} candidates",
            seed.path,
            seed.state.as_str(),
            seed.candidate_count
        ));
    }
    for resolution in &patch.resolutions {
        lines.push(format!(
            "LOCATOR {} · {} · {}",
            resolution.candidate,
            resolution.status.as_str(),
            resolution.coordinate.as_ref().map_or_else(
                || resolution.detail.clone(),
                |binding| binding.coordinate.clone()
            )
        ));
    }
    for omission in &patch.omissions {
        lines.push(format!(
            "OMISSION {} · {} · {}",
            omission.kind, omission.subject, omission.reason
        ));
    }
    lines.join("\n")
}

fn validate_input(context: &TopographyExecutionContext<'_>) -> Result<(), TopographySurveyError> {
    if context.input.relative_paths.is_empty() {
        return Err(TopographySurveyError::MissingSeeds);
    }
    let declared = context
        .declared_seeds
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let actual = context
        .input
        .relative_paths
        .iter()
        .map(|path| path.to_string_lossy())
        .collect::<Vec<_>>();
    if declared.len() != actual.len()
        || declared
            .iter()
            .zip(&actual)
            .any(|(declared, actual)| *declared != actual.as_ref())
    {
        return Err(TopographySurveyError::SeedBinding);
    }
    Ok(())
}

#[derive(Clone)]
struct ObservedSeed {
    path: String,
    state: SurveySeedState,
    revision: Option<String>,
    bytes: Vec<u8>,
    coordinate: Option<CoordinateBinding>,
    detail: String,
}

impl ObservedSeed {
    fn to_seed(&self, candidate_count: u64) -> SurveySeed {
        SurveySeed {
            path: self.path.clone(),
            state: if self.state == SurveySeedState::Surveyed && candidate_count == 0 {
                SurveySeedState::SurveyedEmpty
            } else {
                self.state
            },
            source_revision: self.revision.clone(),
            logical_bytes: self.bytes.len() as u64,
            coordinate: self.coordinate.clone(),
            candidate_count,
            detail: self.detail.clone(),
        }
    }
}

fn observe_seed(
    root: &Path,
    relative: &str,
    total_bytes: u64,
    limits: &TopographyLimits,
    provider: &ContractIdentity,
) -> Result<ObservedSeed, TopographySurveyError> {
    let path = safe_path(root, relative)?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Ok(ObservedSeed {
                path: relative.to_owned(),
                state: SurveySeedState::Unsupported,
                revision: None,
                bytes: Vec::new(),
                coordinate: None,
                detail: "symlinked seed is outside the admitted regular-file surface".to_owned(),
            });
        }
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => {
            return Ok(ObservedSeed {
                path: relative.to_owned(),
                state: SurveySeedState::Unsupported,
                revision: None,
                bytes: Vec::new(),
                coordinate: None,
                detail: "seed is not a regular file".to_owned(),
            });
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ObservedSeed {
                path: relative.to_owned(),
                state: SurveySeedState::Missing,
                revision: None,
                bytes: Vec::new(),
                coordinate: None,
                detail: "declared seed is missing".to_owned(),
            });
        }
        Err(source) => return Err(TopographySurveyError::Read { path, source }),
    };
    if metadata.len() > limits.max_seed_bytes
        || total_bytes.saturating_add(metadata.len()) > limits.max_total_bytes
    {
        return Ok(ObservedSeed {
            path: relative.to_owned(),
            state: SurveySeedState::Omitted,
            revision: None,
            bytes: Vec::new(),
            coordinate: None,
            detail: "seed byte bound reached before content admission".to_owned(),
        });
    }
    let bytes = fs::read(&path).map_err(|source| TopographySurveyError::Read {
        path: path.clone(),
        source,
    })?;
    if std::str::from_utf8(&bytes).is_err() {
        return Ok(ObservedSeed {
            path: relative.to_owned(),
            state: SurveySeedState::Unsupported,
            revision: None,
            bytes: Vec::new(),
            coordinate: None,
            detail: "seed is not valid UTF-8".to_owned(),
        });
    }
    let revision = content_revision("rey.topography-file.v1", &bytes).to_string();
    let coordinate = local_binding(provider, "file", relative, &revision)?;
    Ok(ObservedSeed {
        path: relative.to_owned(),
        state: SurveySeedState::Surveyed,
        revision: Some(revision),
        bytes,
        coordinate: Some(coordinate),
        detail: "surveyed exact regular UTF-8 seed".to_owned(),
    })
}

#[derive(Clone)]
struct ExtractedTarget {
    raw: String,
    start_byte: u64,
    end_byte: u64,
}

fn extract_markdown_targets(bytes: &[u8]) -> Vec<ExtractedTarget> {
    let text = std::str::from_utf8(bytes).unwrap_or_default();
    let raw = text.as_bytes();
    let mut targets = Vec::new();
    let mut index = 0;
    while index + 2 < raw.len() {
        if raw[index] == b']' && raw[index + 1] == b'(' {
            let start = index + 2;
            if let Some(relative_end) = raw[start..].iter().position(|byte| *byte == b')') {
                let end = start + relative_end;
                let candidate = text[start..end].trim();
                if !candidate.is_empty() {
                    let trim_start = text[start..end].find(candidate).unwrap_or(0);
                    targets.push(ExtractedTarget {
                        raw: candidate.to_owned(),
                        start_byte: (start + trim_start) as u64,
                        end_byte: (start + trim_start + candidate.len()) as u64,
                    });
                }
                index = end + 1;
                continue;
            }
        }
        if raw[index] == b'<' {
            let start = index + 1;
            if let Some(relative_end) = raw[start..].iter().position(|byte| *byte == b'>') {
                let end = start + relative_end;
                let candidate = &text[start..end];
                if candidate.starts_with("http://") || candidate.starts_with("https://") {
                    targets.push(ExtractedTarget {
                        raw: candidate.to_owned(),
                        start_byte: start as u64,
                        end_byte: end as u64,
                    });
                }
                index = end + 1;
                continue;
            }
        }
        index += 1;
    }
    targets
}

#[allow(clippy::too_many_arguments)]
fn resolve_candidate(
    root: &Path,
    seed_path: &str,
    seed_revision: &str,
    raw: &str,
    provider: &ContractIdentity,
    capability_snapshot_id: SemanticDigest,
    limits: ResolutionLimits,
) -> Result<LocatorResolution, TopographySurveyError> {
    let locator = match Locator::parse(raw) {
        Ok(locator) => locator,
        Err(_) => {
            return LocatorResolution::new(
                raw,
                None,
                ResolutionStatus::Malformed,
                None,
                provider.clone(),
                seed_revision,
                capability_snapshot_id,
                limits,
                true,
                "candidate is not a canonical supported locator",
            )
            .map_err(TopographySurveyError::Locator);
        }
    };
    if matches!(locator.kind, LocatorKind::HttpUri | LocatorKind::HttpsUri) {
        return LocatorResolution::new(
            raw,
            Some(locator),
            ResolutionStatus::Unsupported,
            None,
            provider.clone(),
            seed_revision,
            capability_snapshot_id,
            limits,
            true,
            "no network URI resolver is admitted for this survey",
        )
        .map_err(TopographySurveyError::Locator);
    }
    let relative = match resolve_reference_path(seed_path, &locator.payload) {
        Ok(relative) => relative,
        Err(TopographySurveyError::PathEscape(_)) => {
            return LocatorResolution::new(
                raw,
                Some(locator),
                ResolutionStatus::Unauthorized,
                None,
                provider.clone(),
                seed_revision,
                capability_snapshot_id,
                limits,
                true,
                "reference escapes the admitted workspace root",
            )
            .map_err(TopographySurveyError::Locator);
        }
        Err(error) => return Err(error),
    };
    let path = match safe_path(root, &relative) {
        Ok(path) => path,
        Err(TopographySurveyError::Symlink(_)) => {
            return LocatorResolution::new(
                raw,
                Some(locator),
                ResolutionStatus::Unauthorized,
                None,
                provider.clone(),
                seed_revision,
                capability_snapshot_id,
                limits,
                true,
                "symlinked reference is outside the admitted resolver surface",
            )
            .map_err(TopographySurveyError::Locator);
        }
        Err(error) => return Err(error),
    };
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => {
            return LocatorResolution::new(
                raw,
                Some(locator),
                ResolutionStatus::Unsupported,
                None,
                provider.clone(),
                seed_revision,
                capability_snapshot_id,
                limits,
                true,
                "reference target is not a regular file",
            )
            .map_err(TopographySurveyError::Locator);
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return LocatorResolution::new(
                raw,
                Some(locator),
                ResolutionStatus::Missing,
                None,
                provider.clone(),
                seed_revision,
                capability_snapshot_id,
                limits,
                true,
                "reference target is missing",
            )
            .map_err(TopographySurveyError::Locator);
        }
        Err(source) => return Err(TopographySurveyError::Read { path, source }),
    };
    if metadata.len() > limits.max_source_bytes {
        return LocatorResolution::new(
            raw,
            Some(locator),
            ResolutionStatus::Truncated,
            None,
            provider.clone(),
            seed_revision,
            capability_snapshot_id,
            limits,
            false,
            "target exceeds the resolver byte bound",
        )
        .map_err(TopographySurveyError::Locator);
    }
    let bytes = fs::read(&path).map_err(|source| TopographySurveyError::Read {
        path: path.clone(),
        source,
    })?;
    let revision = content_revision("rey.topography-file.v1", &bytes).to_string();
    let coordinate = local_binding(provider, "file", &relative, &revision)?;
    LocatorResolution::new(
        raw,
        Some(locator),
        ResolutionStatus::Resolved,
        Some(coordinate),
        provider.clone(),
        seed_revision,
        capability_snapshot_id,
        limits,
        true,
        "resolved exact regular file below the admitted workspace root",
    )
    .map_err(TopographySurveyError::Locator)
}

fn resolve_reference_path(
    seed_path: &str,
    candidate: &str,
) -> Result<String, TopographySurveyError> {
    let candidate_path = Path::new(candidate);
    if candidate_path.is_absolute() {
        return Err(TopographySurveyError::PathEscape(candidate.to_owned()));
    }
    let mut components = Path::new(seed_path)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_os_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for component in candidate_path.components() {
        match component {
            Component::Normal(value) => components.push(value.to_os_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                if components.pop().is_none() {
                    return Err(TopographySurveyError::PathEscape(candidate.to_owned()));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(TopographySurveyError::PathEscape(candidate.to_owned()));
            }
        }
    }
    if components.is_empty() {
        return Err(TopographySurveyError::PathEscape(candidate.to_owned()));
    }
    Ok(components
        .into_iter()
        .collect::<PathBuf>()
        .to_string_lossy()
        .into_owned())
}

fn normalize_declared_path(path: &Path) -> Result<String, TopographySurveyError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(TopographySurveyError::PathEscape(
            path.display().to_string(),
        ));
    }
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| TopographySurveyError::PathEncoding(path.to_owned()))
}

fn safe_path(root: &Path, relative: &str) -> Result<PathBuf, TopographySurveyError> {
    let mut current = root.to_owned();
    for component in Path::new(relative).components() {
        let Component::Normal(value) = component else {
            return Err(TopographySurveyError::PathEscape(relative.to_owned()));
        };
        current.push(value);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(TopographySurveyError::Symlink(current));
            }
            Ok(_) => {}
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => break,
            Err(source) => {
                return Err(TopographySurveyError::Read {
                    path: current,
                    source,
                });
            }
        }
    }
    Ok(current)
}

fn local_binding(
    provider: &ContractIdentity,
    kind: &str,
    identity: &str,
    revision: &str,
) -> Result<CoordinateBinding, TopographySurveyError> {
    let coordinate = LocalCoordinate::new(kind, identity, revision, BTreeMap::new())?;
    CoordinateBinding::local(
        provider.clone(),
        coordinate,
        CoordinateIdentityClass::RevisionBound,
        revision,
    )
    .map_err(TopographySurveyError::Locator)
}

fn derived_region(
    revision: &str,
    identity: &str,
    state: TopographyRegionState,
    detail: &str,
) -> Result<TopographyRegion, TopographySurveyError> {
    let coordinate = LocalCoordinate::new("region", identity, revision, BTreeMap::new())?.to_uri();
    Ok(region(&coordinate, state, 0, 0, detail))
}

fn region(
    coordinate: &str,
    state: TopographyRegionState,
    surveyed_seeds: u64,
    candidate_count: u64,
    detail: &str,
) -> TopographyRegion {
    TopographyRegion {
        region_id: region_identity(coordinate, state),
        coordinate: coordinate.to_owned(),
        state,
        surveyed_seeds,
        candidate_count,
        detail: detail.to_owned(),
    }
}

fn workspace_revision(observations: &[ObservedSeed]) -> String {
    let mut hasher = SemanticHasher::new("rey.topography-workspace-selection.v1");
    hasher.add_u64(observations.len() as u64);
    for observation in observations {
        hasher.add_str(&observation.path);
        hasher.add_str(observation.state.as_str());
        hasher.add_optional_str(observation.revision.as_deref());
    }
    hasher.finish().to_string()
}

fn survey_execution_id(
    context: &TopographyExecutionContext<'_>,
    workspace_revision: &str,
    candidates: &[TopographyLocatorCandidate],
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.context-anchor-survey-execution.v1");
    context.workload.add_semantics(&mut hasher);
    context.graph.add_semantics(&mut hasher);
    hasher.add_optional_str(
        context
            .scenario
            .map(|scenario| scenario.semantic_digest.as_str()),
    );
    hasher.add_str(context.campaign_id.as_str());
    hasher.add_str(context.graph_node_id);
    hasher.add_str(workspace_revision);
    hasher.add_u64(candidates.len() as u64);
    for candidate in candidates {
        hasher.add_str(candidate.candidate_id.as_str());
    }
    hasher.finish()
}

fn content_revision(domain: &str, bytes: &[u8]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(domain);
    hasher.add_bytes(bytes);
    hasher.finish()
}

#[derive(Debug, Error)]
pub enum TopographySurveyError {
    #[error("topography survey requires at least one explicit seed")]
    MissingSeeds,
    #[error("declared seed input does not match the explicit source bindings")]
    SeedBinding,
    #[error("invalid topography fixture {0}")]
    Fixture(String),
    #[error("topography path escapes the workspace: {0}")]
    PathEscape(String),
    #[error("topography path has unsupported platform encoding: {0}")]
    PathEncoding(PathBuf),
    #[error("topography path traverses a symlink: {0}")]
    Symlink(PathBuf),
    #[error("failed to read topography source {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    Locator(#[from] rey_locator::LocatorError),
    #[error(transparent)]
    Topography(#[from] rey_mining::TopographyError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context<'a>(
        workload: &'a ContractIdentity,
        graph: &'a ContractIdentity,
        campaign: &'a SemanticDigest,
        input: &'a TopographySurveyInput,
        seeds: &'a str,
    ) -> TopographyExecutionContext<'a> {
        TopographyExecutionContext {
            workload,
            graph,
            scenario: None,
            campaign_id: campaign,
            graph_node_id: "survey",
            declared_seeds: seeds,
            input,
        }
    }

    #[test]
    fn fixture_voyage_retains_resolution_classes_coverage_and_frontier() {
        let input = TopographySurveyInput {
            root: topography_fixture_root("basic").unwrap(),
            relative_paths: vec!["AGENTS.md".into(), "README.md".into()],
            capability_snapshot_id: SemanticHasher::new("capability").finish(),
            limits: TopographyLimits::default(),
            resolution_limits: ResolutionLimits::default(),
            prior: None,
        };
        let workload = ContractIdentity::new("context-anchor-survey", 1, "fixture");
        let graph = ContractIdentity::new("context-anchor-survey.graph", 1, "fixture");
        let campaign = SemanticHasher::new("campaign").finish();
        let patch = execute_context_anchor_survey(context(
            &workload,
            &graph,
            &campaign,
            &input,
            "AGENTS.md\nREADME.md",
        ))
        .unwrap();
        assert_eq!(patch.coverage.requested_seeds, 2);
        assert_eq!(patch.coverage.surveyed_seeds, 2);
        assert!(patch.coverage.candidates >= 6);
        assert!(patch.coverage.resolved_candidates >= 2);
        assert!(
            patch
                .resolutions
                .iter()
                .any(|resolution| resolution.status == ResolutionStatus::Malformed)
        );
        assert!(
            patch
                .resolutions
                .iter()
                .any(|resolution| resolution.status == ResolutionStatus::Missing)
        );
        assert!(
            patch
                .resolutions
                .iter()
                .any(|resolution| resolution.status == ResolutionStatus::Unsupported)
        );
        assert!(
            patch
                .resolutions
                .iter()
                .any(|resolution| resolution.status == ResolutionStatus::Unauthorized)
        );
        assert!(patch.candidates.iter().any(|candidate| candidate.duplicate));
        let readme_seed = patch
            .seeds
            .iter()
            .find(|seed| seed.path == "README.md")
            .and_then(|seed| seed.coordinate.as_ref())
            .unwrap();
        let readme_resolution = patch
            .resolutions
            .iter()
            .find(|resolution| resolution.candidate == "README.md")
            .and_then(|resolution| resolution.coordinate.as_ref())
            .unwrap();
        assert_eq!(readme_seed, readme_resolution);
        assert_eq!(
            patch
                .anchors
                .iter()
                .filter(|anchor| anchor.label == "README.md")
                .count(),
            1
        );
        assert!(!patch.frontier.is_empty());
        assert!(
            patch
                .regions
                .iter()
                .any(|region| region.state == TopographyRegionState::Unexplored)
        );
        patch.verify().unwrap();

        let replay_input = TopographySurveyInput {
            prior: Some(patch.clone()),
            ..input.clone()
        };
        let next_campaign = SemanticHasher::new("next-campaign").finish();
        let replay = execute_context_anchor_survey(context(
            &workload,
            &graph,
            &next_campaign,
            &replay_input,
            "AGENTS.md\nREADME.md",
        ))
        .unwrap();
        assert_eq!(replay.topography_revision, patch.topography_revision);
        assert_eq!(replay.prior_topography_revision, patch.topography_revision);
        assert_eq!(replay.delta.inserted, 0);
        assert_eq!(replay.delta.deleted, 0);
        assert_eq!(replay.delta.modified, 0);
    }

    #[test]
    fn missing_and_bounded_seeds_are_explicit_and_deterministic() {
        let limits = TopographyLimits {
            max_candidates: 1,
            ..TopographyLimits::default()
        };
        let input = TopographySurveyInput {
            root: topography_fixture_root("basic").unwrap(),
            relative_paths: vec!["AGENTS.md".into(), "MISSING.md".into()],
            capability_snapshot_id: SemanticHasher::new("capability").finish(),
            limits,
            resolution_limits: ResolutionLimits::default(),
            prior: None,
        };
        let workload = ContractIdentity::new("context-anchor-survey", 1, "fixture");
        let graph = ContractIdentity::new("context-anchor-survey.graph", 1, "fixture");
        let campaign = SemanticHasher::new("campaign").finish();
        let first = execute_context_anchor_survey(context(
            &workload,
            &graph,
            &campaign,
            &input,
            "AGENTS.md\nMISSING.md",
        ))
        .unwrap();
        let second = execute_context_anchor_survey(context(
            &workload,
            &graph,
            &campaign,
            &input,
            "AGENTS.md\nMISSING.md",
        ))
        .unwrap();
        assert_eq!(first, second);
        assert!(!first.complete);
        assert_eq!(first.coverage.missing_seeds, 1);
        assert!(
            first
                .omissions
                .iter()
                .any(|omission| omission.kind == "candidate_limit")
        );

        let seed_limited = TopographySurveyInput {
            limits: TopographyLimits {
                max_seeds: 1,
                ..TopographyLimits::default()
            },
            ..input
        };
        let bounded = execute_context_anchor_survey(context(
            &workload,
            &graph,
            &campaign,
            &seed_limited,
            "AGENTS.md\nMISSING.md",
        ))
        .unwrap();
        assert_eq!(bounded.coverage.requested_seeds, 2);
        assert_eq!(bounded.coverage.omitted_seeds, 1);
        assert!(
            bounded
                .omissions
                .iter()
                .any(|omission| omission.kind == "seed_limit")
        );
        bounded.verify().unwrap();
    }
}
