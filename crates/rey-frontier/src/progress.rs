use std::collections::{BTreeMap, BTreeSet};

use polars::df;
use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_dataframe::{Frame, FrameMetadata};
use serde::{Deserialize, Serialize};

use crate::{
    Frontier, FrontierAssessment, FrontierError, add_string_bytes, validate_contract,
    validate_digest,
};

pub const FRONTIER_PROGRESS_SCHEMA: &str = "rey.frontier-progress.v1";
pub const FRONTIER_PROGRESS_RELATION: &str = "rey.frontier-progress-changes";
pub const FRONTIER_PROGRESS_SCHEMA_VERSION: &str = "1";
const FRONTIER_COMPARATOR_ID: &str = "rey.frontier-work-exact";
const FRONTIER_COMPARATOR_REVISION: u64 = 1;
const FRONTIER_COMPARATOR_DEFINITION: &str = "align compatible rey.frontier.v1 rows by stable work_id across exact source/target graph revisions in one workload campaign; source-only resolved, target-only introduced, changed row_id updated, equal row_id unchanged; no scalar score";

#[must_use]
pub fn frontier_comparator() -> ContractIdentity {
    ContractIdentity::new(
        FRONTIER_COMPARATOR_ID,
        FRONTIER_COMPARATOR_REVISION,
        FRONTIER_COMPARATOR_DEFINITION,
    )
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProgressInputs {
    pub workload: ContractIdentity,
    pub source_graph: ContractIdentity,
    pub target_graph: ContractIdentity,
    pub scenario_suite: ContractIdentity,
    pub campaign_id: SemanticDigest,
    pub space: ContractIdentity,
    pub trace_id: SemanticDigest,
    pub source_frontier_id: SemanticDigest,
    pub target_frontier_id: SemanticDigest,
    pub source_record_id: SemanticDigest,
    pub target_record_id: SemanticDigest,
    pub source_assessment: FrontierAssessment,
    pub target_assessment: FrontierAssessment,
    pub source_complete: bool,
    pub target_complete: bool,
    pub comparator: ContractIdentity,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProgressLimits {
    pub max_source_rows: u64,
    pub max_target_rows: u64,
    pub max_changes: u64,
    pub max_string_bytes: u64,
}

impl Default for ProgressLimits {
    fn default() -> Self {
        Self {
            max_source_rows: 1_024,
            max_target_rows: 1_024,
            max_changes: 2_048,
            max_string_bytes: 512 * 1_024,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressChangeKind {
    Resolved,
    Introduced,
    Updated,
}

impl ProgressChangeKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Resolved => "resolved",
            Self::Introduced => "introduced",
            Self::Updated => "updated",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProgressChange {
    pub work_id: String,
    pub kind: ProgressChangeKind,
    pub source_row_id: Option<SemanticDigest>,
    pub target_row_id: Option<SemanticDigest>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressAssessment {
    Progressing,
    Regressing,
    Mixed,
    Unchanged,
    Converged,
    Inconclusive,
}

impl ProgressAssessment {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Progressing => "progressing",
            Self::Regressing => "regressing",
            Self::Mixed => "mixed",
            Self::Unchanged => "unchanged",
            Self::Converged => "converged",
            Self::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProgressSummary {
    pub source_rows: u64,
    pub target_rows: u64,
    pub resolved: u64,
    pub introduced: u64,
    pub updated: u64,
    pub unchanged: u64,
    pub assessment: ProgressAssessment,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FrontierProgress {
    pub schema: String,
    pub progress_id: SemanticDigest,
    pub inputs: ProgressInputs,
    pub limits: ProgressLimits,
    pub summary: ProgressSummary,
    pub changes: Vec<ProgressChange>,
}

impl FrontierProgress {
    pub fn compare(
        source: &Frontier,
        target: &Frontier,
        limits: ProgressLimits,
    ) -> Result<Self, FrontierError> {
        source.verify()?;
        target.verify()?;
        let comparator = frontier_comparator();
        validate_contract("progress comparator", &comparator)?;
        validate_limits(&limits)?;
        validate_compatibility(source, target)?;
        enforce_limit(
            "source row",
            source.rows.len() as u64,
            limits.max_source_rows,
        )?;
        enforce_limit(
            "target row",
            target.rows.len() as u64,
            limits.max_target_rows,
        )?;

        let source_rows = source
            .rows
            .iter()
            .map(|row| (row.work_id.as_str(), row))
            .collect::<BTreeMap<_, _>>();
        let target_rows = target
            .rows
            .iter()
            .map(|row| (row.work_id.as_str(), row))
            .collect::<BTreeMap<_, _>>();
        let work_ids = source_rows
            .keys()
            .chain(target_rows.keys())
            .copied()
            .collect::<BTreeSet<_>>();

        let mut changes = Vec::new();
        let mut unchanged = 0_u64;
        for work_id in work_ids {
            match (source_rows.get(work_id), target_rows.get(work_id)) {
                (Some(source_row), Some(target_row)) if source_row.row_id == target_row.row_id => {
                    unchanged = unchanged
                        .checked_add(1)
                        .ok_or(FrontierError::CountOverflow)?;
                }
                (Some(source_row), Some(target_row)) => changes.push(ProgressChange {
                    work_id: work_id.to_owned(),
                    kind: ProgressChangeKind::Updated,
                    source_row_id: Some(source_row.row_id.clone()),
                    target_row_id: Some(target_row.row_id.clone()),
                }),
                (Some(source_row), None) => changes.push(ProgressChange {
                    work_id: work_id.to_owned(),
                    kind: ProgressChangeKind::Resolved,
                    source_row_id: Some(source_row.row_id.clone()),
                    target_row_id: None,
                }),
                (None, Some(target_row)) => changes.push(ProgressChange {
                    work_id: work_id.to_owned(),
                    kind: ProgressChangeKind::Introduced,
                    source_row_id: None,
                    target_row_id: Some(target_row.row_id.clone()),
                }),
                (None, None) => unreachable!("work id originates from source or target"),
            }
        }
        enforce_limit("change", changes.len() as u64, limits.max_changes)?;

        let resolved = count_kind(&changes, ProgressChangeKind::Resolved);
        let introduced = count_kind(&changes, ProgressChangeKind::Introduced);
        let updated = count_kind(&changes, ProgressChangeKind::Updated);
        let assessment = derive_assessment(
            source.assessment,
            target.assessment,
            source.coverage.is_complete(),
            target.coverage.is_complete(),
            resolved,
            introduced,
            updated,
        );
        let summary = ProgressSummary {
            source_rows: source.rows.len() as u64,
            target_rows: target.rows.len() as u64,
            resolved,
            introduced,
            updated,
            unchanged,
            assessment,
        };
        let inputs = ProgressInputs {
            workload: source.inputs.workload.clone(),
            source_graph: source.inputs.graph.clone(),
            target_graph: target.inputs.graph.clone(),
            scenario_suite: source.inputs.scenario_suite.clone(),
            campaign_id: source.inputs.campaign_id.clone(),
            space: source.inputs.space.clone(),
            trace_id: source.inputs.trace_id.clone(),
            source_frontier_id: source.frontier_id.clone(),
            target_frontier_id: target.frontier_id.clone(),
            source_record_id: source.inputs.committed_record_id.clone(),
            target_record_id: target.inputs.committed_record_id.clone(),
            source_assessment: source.assessment,
            target_assessment: target.assessment,
            source_complete: source.coverage.is_complete(),
            target_complete: target.coverage.is_complete(),
            comparator,
        };
        validate_string_bytes(&inputs, &changes, &limits)?;
        let mut progress = Self {
            schema: FRONTIER_PROGRESS_SCHEMA.to_owned(),
            progress_id: placeholder_digest(),
            inputs,
            limits,
            summary,
            changes,
        };
        progress.progress_id = progress_digest(&progress);
        Ok(progress)
    }

    pub fn verify(&self) -> Result<(), FrontierError> {
        if self.schema != FRONTIER_PROGRESS_SCHEMA {
            return Err(FrontierError::UnsupportedSchema {
                kind: "frontier progress",
                expected: FRONTIER_PROGRESS_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_contract("workload", &self.inputs.workload)?;
        validate_contract("source graph", &self.inputs.source_graph)?;
        validate_contract("target graph", &self.inputs.target_graph)?;
        validate_contract("scenario suite", &self.inputs.scenario_suite)?;
        validate_contract("space", &self.inputs.space)?;
        validate_contract("progress comparator", &self.inputs.comparator)?;
        if self.inputs.comparator != frontier_comparator() {
            return Err(FrontierError::UnexpectedContract("progress comparator"));
        }
        for digest in [
            &self.inputs.campaign_id,
            &self.inputs.trace_id,
            &self.inputs.source_frontier_id,
            &self.inputs.target_frontier_id,
            &self.inputs.source_record_id,
            &self.inputs.target_record_id,
        ] {
            validate_digest(digest)?;
        }
        validate_limits(&self.limits)?;
        enforce_limit(
            "source row",
            self.summary.source_rows,
            self.limits.max_source_rows,
        )?;
        enforce_limit(
            "target row",
            self.summary.target_rows,
            self.limits.max_target_rows,
        )?;
        enforce_limit("change", self.changes.len() as u64, self.limits.max_changes)?;
        validate_changes(&self.changes)?;
        let expected_resolved = count_kind(&self.changes, ProgressChangeKind::Resolved);
        let expected_introduced = count_kind(&self.changes, ProgressChangeKind::Introduced);
        let expected_updated = count_kind(&self.changes, ProgressChangeKind::Updated);
        let total = expected_resolved
            .checked_add(expected_updated)
            .and_then(|value| value.checked_add(self.summary.unchanged))
            .ok_or(FrontierError::CountOverflow)?;
        let target_total = expected_introduced
            .checked_add(expected_updated)
            .and_then(|value| value.checked_add(self.summary.unchanged))
            .ok_or(FrontierError::CountOverflow)?;
        if self.summary.resolved != expected_resolved
            || self.summary.introduced != expected_introduced
            || self.summary.updated != expected_updated
            || self.summary.source_rows != total
            || self.summary.target_rows != target_total
        {
            return Err(FrontierError::ProgressSummaryMismatch);
        }
        let expected_assessment = derive_assessment(
            self.inputs.source_assessment,
            self.inputs.target_assessment,
            self.inputs.source_complete,
            self.inputs.target_complete,
            expected_resolved,
            expected_introduced,
            expected_updated,
        );
        if self.summary.assessment != expected_assessment {
            return Err(FrontierError::ProgressSummaryMismatch);
        }
        validate_string_bytes(&self.inputs, &self.changes, &self.limits)?;
        let actual = progress_digest(self);
        if self.progress_id != actual {
            return Err(FrontierError::DigestMismatch {
                kind: "frontier progress",
                declared: self.progress_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    pub fn verify_against(
        &self,
        source: &Frontier,
        target: &Frontier,
    ) -> Result<(), FrontierError> {
        self.verify()?;
        let expected = Self::compare(source, target, self.limits.clone())?;
        if self != &expected {
            return Err(FrontierError::NonCanonical("frontier progress"));
        }
        Ok(())
    }

    pub fn to_frame(&self) -> Result<Frame, FrontierError> {
        self.verify()?;
        let dataframe = df!(
            "work_id" => self.changes.iter().map(|change| change.work_id.as_str()).collect::<Vec<_>>(),
            "change_kind" => self.changes.iter().map(|change| change.kind.as_str()).collect::<Vec<_>>(),
            "source_row_id" => self.changes.iter().map(|change| change.source_row_id.as_ref().map(SemanticDigest::as_str)).collect::<Vec<_>>(),
            "target_row_id" => self.changes.iter().map(|change| change.target_row_id.as_ref().map(SemanticDigest::as_str)).collect::<Vec<_>>(),
        )?;
        let attributes = BTreeMap::from([
            ("rey.progress-schema".to_owned(), self.schema.clone()),
            ("rey.progress-id".to_owned(), self.progress_id.to_string()),
            (
                "rey.workload-id".to_owned(),
                self.inputs.workload.id.clone(),
            ),
            (
                "rey.workload-revision".to_owned(),
                self.inputs.workload.revision.to_string(),
            ),
            (
                "rey.workload-digest".to_owned(),
                self.inputs.workload.semantic_digest.to_string(),
            ),
            (
                "rey.source-graph-id".to_owned(),
                self.inputs.source_graph.id.clone(),
            ),
            (
                "rey.source-graph-revision".to_owned(),
                self.inputs.source_graph.revision.to_string(),
            ),
            (
                "rey.source-graph-digest".to_owned(),
                self.inputs.source_graph.semantic_digest.to_string(),
            ),
            (
                "rey.target-graph-id".to_owned(),
                self.inputs.target_graph.id.clone(),
            ),
            (
                "rey.target-graph-revision".to_owned(),
                self.inputs.target_graph.revision.to_string(),
            ),
            (
                "rey.target-graph-digest".to_owned(),
                self.inputs.target_graph.semantic_digest.to_string(),
            ),
            (
                "rey.scenario-suite-id".to_owned(),
                self.inputs.scenario_suite.id.clone(),
            ),
            (
                "rey.scenario-suite-revision".to_owned(),
                self.inputs.scenario_suite.revision.to_string(),
            ),
            (
                "rey.scenario-suite-digest".to_owned(),
                self.inputs.scenario_suite.semantic_digest.to_string(),
            ),
            (
                "rey.campaign-id".to_owned(),
                self.inputs.campaign_id.to_string(),
            ),
            ("rey.space-id".to_owned(), self.inputs.space.id.clone()),
            (
                "rey.space-revision".to_owned(),
                self.inputs.space.revision.to_string(),
            ),
            (
                "rey.space-digest".to_owned(),
                self.inputs.space.semantic_digest.to_string(),
            ),
            (
                "rey.source-frontier-id".to_owned(),
                self.inputs.source_frontier_id.to_string(),
            ),
            (
                "rey.target-frontier-id".to_owned(),
                self.inputs.target_frontier_id.to_string(),
            ),
            ("rey.trace-id".to_owned(), self.inputs.trace_id.to_string()),
            (
                "rey.source-record-id".to_owned(),
                self.inputs.source_record_id.to_string(),
            ),
            (
                "rey.target-record-id".to_owned(),
                self.inputs.target_record_id.to_string(),
            ),
            (
                "rey.source-assessment".to_owned(),
                self.inputs.source_assessment.as_str().to_owned(),
            ),
            (
                "rey.target-assessment".to_owned(),
                self.inputs.target_assessment.as_str().to_owned(),
            ),
            (
                "rey.source-complete".to_owned(),
                self.inputs.source_complete.to_string(),
            ),
            (
                "rey.target-complete".to_owned(),
                self.inputs.target_complete.to_string(),
            ),
            (
                "rey.comparator-id".to_owned(),
                self.inputs.comparator.id.clone(),
            ),
            (
                "rey.comparator-revision".to_owned(),
                self.inputs.comparator.revision.to_string(),
            ),
            (
                "rey.comparator-digest".to_owned(),
                self.inputs.comparator.semantic_digest.to_string(),
            ),
            (
                "rey.assessment".to_owned(),
                self.summary.assessment.as_str().to_owned(),
            ),
            ("rey.resolved".to_owned(), self.summary.resolved.to_string()),
            (
                "rey.introduced".to_owned(),
                self.summary.introduced.to_string(),
            ),
            ("rey.updated".to_owned(), self.summary.updated.to_string()),
            (
                "rey.unchanged".to_owned(),
                self.summary.unchanged.to_string(),
            ),
            (
                "rey.max-source-rows".to_owned(),
                self.limits.max_source_rows.to_string(),
            ),
            (
                "rey.max-target-rows".to_owned(),
                self.limits.max_target_rows.to_string(),
            ),
            (
                "rey.max-changes".to_owned(),
                self.limits.max_changes.to_string(),
            ),
            (
                "rey.max-string-bytes".to_owned(),
                self.limits.max_string_bytes.to_string(),
            ),
        ]);
        Ok(Frame::new(
            dataframe,
            FrameMetadata {
                relation: FRONTIER_PROGRESS_RELATION.to_owned(),
                schema_version: FRONTIER_PROGRESS_SCHEMA_VERSION.to_owned(),
                semantic_digest: self.progress_id.to_string(),
                row_count: self.changes.len() as u64,
                complete: self.inputs.source_complete && self.inputs.target_complete,
                key_columns: vec!["work_id".to_owned()],
                attributes,
            },
        )?)
    }
}

fn validate_compatibility(source: &Frontier, target: &Frontier) -> Result<(), FrontierError> {
    let pairs = [
        (
            "workload contract",
            source.inputs.workload == target.inputs.workload,
        ),
        (
            "scenario suite contract",
            source.inputs.scenario_suite == target.inputs.scenario_suite,
        ),
        (
            "campaign identity",
            source.inputs.campaign_id == target.inputs.campaign_id,
        ),
        ("space contract", source.inputs.space == target.inputs.space),
        (
            "trace identity",
            source.inputs.trace_id == target.inputs.trace_id,
        ),
        (
            "derivation contract",
            source.inputs.derivation == target.inputs.derivation,
        ),
        (
            "prioritization contract",
            source.inputs.prioritization == target.inputs.prioritization,
        ),
    ];
    if let Some((field, _)) = pairs.into_iter().find(|(_, equal)| !equal) {
        return Err(FrontierError::IncompatibleFrontiers(field));
    }
    Ok(())
}

fn validate_limits(limits: &ProgressLimits) -> Result<(), FrontierError> {
    let values = [
        ("max_source_rows", limits.max_source_rows),
        ("max_target_rows", limits.max_target_rows),
        ("max_changes", limits.max_changes),
        ("max_string_bytes", limits.max_string_bytes),
    ];
    if let Some((name, _)) = values.into_iter().find(|(_, value)| *value == 0) {
        return Err(FrontierError::ZeroProgressLimit(name));
    }
    Ok(())
}

fn enforce_limit(kind: &'static str, observed: u64, limit: u64) -> Result<(), FrontierError> {
    if observed > limit {
        return Err(FrontierError::ProgressLimit {
            kind,
            limit,
            observed,
        });
    }
    Ok(())
}

fn validate_changes(changes: &[ProgressChange]) -> Result<(), FrontierError> {
    let mut previous: Option<&str> = None;
    for change in changes {
        crate::validate_text("progress work id", &change.work_id)?;
        if previous.is_some_and(|value| value >= change.work_id.as_str()) {
            return Err(FrontierError::NonCanonical("frontier progress changes"));
        }
        previous = Some(&change.work_id);
        if let Some(digest) = &change.source_row_id {
            validate_digest(digest)?;
        }
        if let Some(digest) = &change.target_row_id {
            validate_digest(digest)?;
        }
        let valid = match change.kind {
            ProgressChangeKind::Resolved => {
                change.source_row_id.is_some() && change.target_row_id.is_none()
            }
            ProgressChangeKind::Introduced => {
                change.source_row_id.is_none() && change.target_row_id.is_some()
            }
            ProgressChangeKind::Updated => {
                change.source_row_id.is_some()
                    && change.target_row_id.is_some()
                    && change.source_row_id != change.target_row_id
            }
        };
        if !valid {
            return Err(FrontierError::ProgressSummaryMismatch);
        }
    }
    Ok(())
}

fn count_kind(changes: &[ProgressChange], kind: ProgressChangeKind) -> u64 {
    changes.iter().filter(|change| change.kind == kind).count() as u64
}

fn derive_assessment(
    source: FrontierAssessment,
    target: FrontierAssessment,
    source_complete: bool,
    target_complete: bool,
    resolved: u64,
    introduced: u64,
    updated: u64,
) -> ProgressAssessment {
    if !source_complete
        || !target_complete
        || source == FrontierAssessment::Inconclusive
        || target == FrontierAssessment::Inconclusive
    {
        ProgressAssessment::Inconclusive
    } else if target == FrontierAssessment::Converged {
        ProgressAssessment::Converged
    } else if updated > 0 || (resolved > 0 && introduced > 0) {
        ProgressAssessment::Mixed
    } else if resolved > 0 {
        ProgressAssessment::Progressing
    } else if introduced > 0 {
        ProgressAssessment::Regressing
    } else {
        ProgressAssessment::Unchanged
    }
}

fn validate_string_bytes(
    inputs: &ProgressInputs,
    changes: &[ProgressChange],
    limits: &ProgressLimits,
) -> Result<(), FrontierError> {
    let mut total = 0_u64;
    for contract in [
        &inputs.workload,
        &inputs.source_graph,
        &inputs.target_graph,
        &inputs.scenario_suite,
        &inputs.space,
        &inputs.comparator,
    ] {
        add_string_bytes(&mut total, &contract.id)?;
        add_string_bytes(&mut total, contract.semantic_digest.as_str())?;
    }
    for digest in [
        &inputs.campaign_id,
        &inputs.trace_id,
        &inputs.source_frontier_id,
        &inputs.target_frontier_id,
        &inputs.source_record_id,
        &inputs.target_record_id,
    ] {
        add_string_bytes(&mut total, digest.as_str())?;
    }
    for change in changes {
        add_string_bytes(&mut total, &change.work_id)?;
        if let Some(digest) = &change.source_row_id {
            add_string_bytes(&mut total, digest.as_str())?;
        }
        if let Some(digest) = &change.target_row_id {
            add_string_bytes(&mut total, digest.as_str())?;
        }
    }
    enforce_limit("string byte", total, limits.max_string_bytes)
}

fn progress_digest(progress: &FrontierProgress) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(FRONTIER_PROGRESS_SCHEMA);
    progress.inputs.workload.add_semantics(&mut hasher);
    progress.inputs.source_graph.add_semantics(&mut hasher);
    progress.inputs.target_graph.add_semantics(&mut hasher);
    progress.inputs.scenario_suite.add_semantics(&mut hasher);
    hasher.add_str(progress.inputs.campaign_id.as_str());
    progress.inputs.space.add_semantics(&mut hasher);
    hasher.add_str(progress.inputs.trace_id.as_str());
    hasher.add_str(progress.inputs.source_frontier_id.as_str());
    hasher.add_str(progress.inputs.target_frontier_id.as_str());
    hasher.add_str(progress.inputs.source_record_id.as_str());
    hasher.add_str(progress.inputs.target_record_id.as_str());
    hasher.add_str(progress.inputs.source_assessment.as_str());
    hasher.add_str(progress.inputs.target_assessment.as_str());
    hasher.add_bool(progress.inputs.source_complete);
    hasher.add_bool(progress.inputs.target_complete);
    progress.inputs.comparator.add_semantics(&mut hasher);
    hasher.add_u64(progress.limits.max_source_rows);
    hasher.add_u64(progress.limits.max_target_rows);
    hasher.add_u64(progress.limits.max_changes);
    hasher.add_u64(progress.limits.max_string_bytes);
    hasher.add_u64(progress.summary.source_rows);
    hasher.add_u64(progress.summary.target_rows);
    hasher.add_u64(progress.summary.resolved);
    hasher.add_u64(progress.summary.introduced);
    hasher.add_u64(progress.summary.updated);
    hasher.add_u64(progress.summary.unchanged);
    hasher.add_str(progress.summary.assessment.as_str());
    hasher.add_u64(progress.changes.len() as u64);
    for change in &progress.changes {
        hasher.add_str(&change.work_id);
        hasher.add_str(change.kind.as_str());
        hasher.add_optional_str(change.source_row_id.as_ref().map(SemanticDigest::as_str));
        hasher.add_optional_str(change.target_row_id.as_ref().map(SemanticDigest::as_str));
    }
    hasher.finish()
}

fn placeholder_digest() -> SemanticDigest {
    SemanticHasher::new("rey.frontier-progress.placeholder").finish()
}
