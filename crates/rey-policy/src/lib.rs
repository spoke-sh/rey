#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use polars::df;
use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_dataframe::{Frame, FrameError, FrameMetadata};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const REASONING_SURFACE_SCHEMA: &str = "rey.reasoning-surface.v3";
pub const REASONING_SURFACE_RELATION: &str = "rey.reasoning-surface-rows";
pub const REASONING_SURFACE_SCHEMA_VERSION: &str = "3";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReasoningSurfaceInputs {
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario_suite: ContractIdentity,
    pub campaign_id: SemanticDigest,
    pub space: ContractIdentity,
    pub trace_id: SemanticDigest,
    pub committed_transition_id: SemanticDigest,
    pub transition_id: SemanticDigest,
    pub scheduling_decision_id: SemanticDigest,
    pub frontier_frame_id: SemanticDigest,
    pub capability_snapshot_id: SemanticDigest,
    pub projection: ContractIdentity,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReasoningSurfaceLimits {
    pub max_rows: u64,
    pub max_delta_refs: u64,
    pub max_evidence_refs: u64,
    pub max_action_refs: u64,
    pub max_omissions: u64,
    pub max_total_evidence_bytes: u64,
    pub max_string_bytes: u64,
    pub max_retrieval_iterations: u64,
}

impl Default for ReasoningSurfaceLimits {
    fn default() -> Self {
        Self {
            max_rows: 256,
            max_delta_refs: 1_024,
            max_evidence_refs: 1_024,
            max_action_refs: 128,
            max_omissions: 256,
            max_total_evidence_bytes: 4 * 1_024 * 1_024,
            max_string_bytes: 1_024 * 1_024,
            max_retrieval_iterations: 32,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceCompleteness {
    Complete,
    Partial,
    Truncated,
}

impl SurfaceCompleteness {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Truncated => "truncated",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OmissionKind {
    RowLimit,
    DeltaLimit,
    EvidenceLimit,
    ActionLimit,
    ByteLimit,
    ProviderUnavailable,
    Unsupported,
    RetrievalFailed,
}

impl OmissionKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::RowLimit => "row_limit",
            Self::DeltaLimit => "delta_limit",
            Self::EvidenceLimit => "evidence_limit",
            Self::ActionLimit => "action_limit",
            Self::ByteLimit => "byte_limit",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::Unsupported => "unsupported",
            Self::RetrievalFailed => "retrieval_failed",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SurfaceOmission {
    pub kind: OmissionKind,
    pub subject_id: Option<String>,
    pub omitted_count: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceReference {
    pub evidence_id: String,
    pub provider: ContractIdentity,
    pub source_id: String,
    pub source_revision: String,
    pub semantic_digest: SemanticDigest,
    pub media_type: String,
    pub byte_length: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReasoningSurfaceRow {
    pub frontier_row_id: String,
    pub entity_kind: String,
    pub entity_id: String,
    pub transition_delta_ids: Vec<SemanticDigest>,
    pub residual_delta_ids: Vec<SemanticDigest>,
    pub claim_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub admissible_action_ids: Vec<String>,
}

impl ReasoningSurfaceRow {
    fn normalize(&mut self) {
        self.transition_delta_ids.sort();
        self.transition_delta_ids.dedup();
        self.residual_delta_ids.sort();
        self.residual_delta_ids.dedup();
        self.claim_ids.sort();
        self.claim_ids.dedup();
        self.evidence_ids.sort();
        self.evidence_ids.dedup();
        self.admissible_action_ids.sort();
        self.admissible_action_ids.dedup();
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReasoningSurface {
    pub schema: String,
    pub surface_id: SemanticDigest,
    pub inputs: ReasoningSurfaceInputs,
    pub limits: ReasoningSurfaceLimits,
    pub retrieval_iterations: u64,
    pub completeness: SurfaceCompleteness,
    pub rows: Vec<ReasoningSurfaceRow>,
    pub evidence: Vec<EvidenceReference>,
    pub admissible_actions: Vec<ContractIdentity>,
    pub omissions: Vec<SurfaceOmission>,
}

impl ReasoningSurface {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        inputs: ReasoningSurfaceInputs,
        limits: ReasoningSurfaceLimits,
        retrieval_iterations: u64,
        completeness: SurfaceCompleteness,
        mut rows: Vec<ReasoningSurfaceRow>,
        mut evidence: Vec<EvidenceReference>,
        mut admissible_actions: Vec<ContractIdentity>,
        mut omissions: Vec<SurfaceOmission>,
    ) -> Result<Self, ReasoningSurfaceError> {
        validate_limits(&limits)?;
        if retrieval_iterations > limits.max_retrieval_iterations {
            return Err(ReasoningSurfaceError::RetrievalIterationLimit {
                limit: limits.max_retrieval_iterations,
                observed: retrieval_iterations,
            });
        }
        for row in &mut rows {
            row.normalize();
        }
        rows.sort_by(|left, right| left.frontier_row_id.cmp(&right.frontier_row_id));
        evidence.sort_by(|left, right| left.evidence_id.cmp(&right.evidence_id));
        admissible_actions.sort_by(|left, right| {
            (&left.id, left.revision, &left.semantic_digest).cmp(&(
                &right.id,
                right.revision,
                &right.semantic_digest,
            ))
        });
        omissions.sort();

        validate_contract("workload", &inputs.workload)?;
        validate_contract("graph", &inputs.graph)?;
        validate_contract("scenario suite", &inputs.scenario_suite)?;
        validate_contract("space", &inputs.space)?;
        validate_contract("projection", &inputs.projection)?;
        for digest in [
            &inputs.campaign_id,
            &inputs.trace_id,
            &inputs.committed_transition_id,
            &inputs.transition_id,
            &inputs.scheduling_decision_id,
            &inputs.frontier_frame_id,
            &inputs.capability_snapshot_id,
        ] {
            validate_digest(digest)?;
        }
        validate_rows(&rows, &limits)?;
        validate_evidence(&evidence, &limits)?;
        validate_actions(&admissible_actions, &limits)?;
        validate_omissions(&omissions, completeness, &limits)?;
        validate_references(&rows, &evidence, &admissible_actions)?;

        let string_bytes =
            semantic_string_bytes(&inputs, &rows, &evidence, &admissible_actions, &omissions)?;
        if string_bytes > limits.max_string_bytes {
            return Err(ReasoningSurfaceError::StringByteLimit {
                limit: limits.max_string_bytes,
                observed: string_bytes,
            });
        }

        let mut surface = Self {
            schema: REASONING_SURFACE_SCHEMA.to_owned(),
            surface_id: placeholder_digest(),
            inputs,
            limits,
            retrieval_iterations,
            completeness,
            rows,
            evidence,
            admissible_actions,
            omissions,
        };
        surface.surface_id = surface_digest(&surface);
        Ok(surface)
    }

    pub fn verify(&self) -> Result<(), ReasoningSurfaceError> {
        if self.schema != REASONING_SURFACE_SCHEMA {
            return Err(ReasoningSurfaceError::UnsupportedSchema {
                expected: REASONING_SURFACE_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        let recomputed = Self::new(
            self.inputs.clone(),
            self.limits.clone(),
            self.retrieval_iterations,
            self.completeness,
            self.rows.clone(),
            self.evidence.clone(),
            self.admissible_actions.clone(),
            self.omissions.clone(),
        )?;
        if self.surface_id != recomputed.surface_id {
            return Err(ReasoningSurfaceError::SurfaceDigest {
                declared: self.surface_id.clone(),
                actual: recomputed.surface_id,
            });
        }
        if self != &recomputed {
            return Err(ReasoningSurfaceError::NonCanonical);
        }
        Ok(())
    }

    pub fn to_frame(&self) -> Result<Frame, ReasoningSurfaceError> {
        self.verify()?;
        let rows = &self.rows;
        let transition_delta_ids = digest_arrays(rows, |row| &row.transition_delta_ids)?;
        let residual_delta_ids = digest_arrays(rows, |row| &row.residual_delta_ids)?;
        let claim_ids = string_arrays(rows, |row| &row.claim_ids)?;
        let evidence_ids = string_arrays(rows, |row| &row.evidence_ids)?;
        let admissible_action_ids = string_arrays(rows, |row| &row.admissible_action_ids)?;
        let dataframe = df!(
            "frontier_row_id" => rows.iter().map(|row| row.frontier_row_id.as_str()).collect::<Vec<_>>(),
            "entity_kind" => rows.iter().map(|row| row.entity_kind.as_str()).collect::<Vec<_>>(),
            "entity_id" => rows.iter().map(|row| row.entity_id.as_str()).collect::<Vec<_>>(),
            "transition_delta_ids" => transition_delta_ids,
            "residual_delta_ids" => residual_delta_ids,
            "claim_ids" => claim_ids,
            "evidence_ids" => evidence_ids,
            "admissible_action_ids" => admissible_action_ids,
        )?;
        let attributes = BTreeMap::from([
            ("rey.surface-schema".to_owned(), self.schema.clone()),
            ("rey.surface-id".to_owned(), self.surface_id.to_string()),
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
            ("rey.graph-id".to_owned(), self.inputs.graph.id.clone()),
            (
                "rey.graph-revision".to_owned(),
                self.inputs.graph.revision.to_string(),
            ),
            (
                "rey.graph-digest".to_owned(),
                self.inputs.graph.semantic_digest.to_string(),
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
            ("rey.trace-id".to_owned(), self.inputs.trace_id.to_string()),
            (
                "rey.committed-transition-id".to_owned(),
                self.inputs.committed_transition_id.to_string(),
            ),
            (
                "rey.transition-id".to_owned(),
                self.inputs.transition_id.to_string(),
            ),
            (
                "rey.scheduling-decision-id".to_owned(),
                self.inputs.scheduling_decision_id.to_string(),
            ),
            (
                "rey.frontier-frame-id".to_owned(),
                self.inputs.frontier_frame_id.to_string(),
            ),
            (
                "rey.capability-snapshot-id".to_owned(),
                self.inputs.capability_snapshot_id.to_string(),
            ),
            (
                "rey.projection-id".to_owned(),
                self.inputs.projection.id.clone(),
            ),
            (
                "rey.projection-revision".to_owned(),
                self.inputs.projection.revision.to_string(),
            ),
            (
                "rey.projection-digest".to_owned(),
                self.inputs.projection.semantic_digest.to_string(),
            ),
            (
                "rey.completeness".to_owned(),
                self.completeness.as_str().to_owned(),
            ),
            (
                "rey.retrieval-iterations".to_owned(),
                self.retrieval_iterations.to_string(),
            ),
            ("rey.max-rows".to_owned(), self.limits.max_rows.to_string()),
            (
                "rey.max-delta-refs".to_owned(),
                self.limits.max_delta_refs.to_string(),
            ),
            (
                "rey.max-evidence-refs".to_owned(),
                self.limits.max_evidence_refs.to_string(),
            ),
            (
                "rey.max-action-refs".to_owned(),
                self.limits.max_action_refs.to_string(),
            ),
            (
                "rey.max-omissions".to_owned(),
                self.limits.max_omissions.to_string(),
            ),
            (
                "rey.max-total-evidence-bytes".to_owned(),
                self.limits.max_total_evidence_bytes.to_string(),
            ),
            (
                "rey.max-string-bytes".to_owned(),
                self.limits.max_string_bytes.to_string(),
            ),
            (
                "rey.max-retrieval-iterations".to_owned(),
                self.limits.max_retrieval_iterations.to_string(),
            ),
            (
                "rey.evidence-count".to_owned(),
                self.evidence.len().to_string(),
            ),
            (
                "rey.action-count".to_owned(),
                self.admissible_actions.len().to_string(),
            ),
            (
                "rey.omission-count".to_owned(),
                self.omissions.len().to_string(),
            ),
        ]);
        Ok(Frame::new(
            dataframe,
            FrameMetadata {
                relation: REASONING_SURFACE_RELATION.to_owned(),
                schema_version: REASONING_SURFACE_SCHEMA_VERSION.to_owned(),
                semantic_digest: self.surface_id.to_string(),
                row_count: rows.len() as u64,
                complete: self.completeness == SurfaceCompleteness::Complete,
                key_columns: vec!["frontier_row_id".to_owned()],
                attributes,
            },
        )?)
    }
}

fn validate_limits(limits: &ReasoningSurfaceLimits) -> Result<(), ReasoningSurfaceError> {
    let values = [
        ("max_rows", limits.max_rows),
        ("max_delta_refs", limits.max_delta_refs),
        ("max_evidence_refs", limits.max_evidence_refs),
        ("max_action_refs", limits.max_action_refs),
        ("max_omissions", limits.max_omissions),
        ("max_total_evidence_bytes", limits.max_total_evidence_bytes),
        ("max_string_bytes", limits.max_string_bytes),
        ("max_retrieval_iterations", limits.max_retrieval_iterations),
    ];
    if let Some((name, _)) = values.into_iter().find(|(_, value)| *value == 0) {
        return Err(ReasoningSurfaceError::ZeroLimit(name));
    }
    Ok(())
}

fn validate_rows(
    rows: &[ReasoningSurfaceRow],
    limits: &ReasoningSurfaceLimits,
) -> Result<(), ReasoningSurfaceError> {
    if rows.is_empty() {
        return Err(ReasoningSurfaceError::EmptySurface);
    }
    if rows.len() as u64 > limits.max_rows {
        return Err(ReasoningSurfaceError::RowLimit {
            limit: limits.max_rows,
            observed: rows.len() as u64,
        });
    }
    let mut delta_refs = 0_u64;
    for row in rows {
        for (field, value) in [
            ("frontier_row_id", row.frontier_row_id.as_str()),
            ("entity_kind", row.entity_kind.as_str()),
            ("entity_id", row.entity_id.as_str()),
        ] {
            validate_text(field, value)?;
        }
        for value in row
            .claim_ids
            .iter()
            .chain(&row.evidence_ids)
            .chain(&row.admissible_action_ids)
        {
            validate_text("row reference", value)?;
        }
        if row.transition_delta_ids.is_empty()
            && row.residual_delta_ids.is_empty()
            && row.claim_ids.is_empty()
        {
            return Err(ReasoningSurfaceError::UndirectedRow(
                row.frontier_row_id.clone(),
            ));
        }
        if row
            .transition_delta_ids
            .iter()
            .any(|id| row.residual_delta_ids.binary_search(id).is_ok())
        {
            return Err(ReasoningSurfaceError::AmbiguousDeltaRole(
                row.frontier_row_id.clone(),
            ));
        }
        for digest in row
            .transition_delta_ids
            .iter()
            .chain(&row.residual_delta_ids)
        {
            validate_digest(digest)?;
        }
        delta_refs = delta_refs
            .checked_add(row.transition_delta_ids.len() as u64)
            .and_then(|count| count.checked_add(row.residual_delta_ids.len() as u64))
            .ok_or(ReasoningSurfaceError::CountOverflow)?;
    }
    if rows
        .windows(2)
        .any(|window| window[0].frontier_row_id >= window[1].frontier_row_id)
    {
        return Err(ReasoningSurfaceError::DuplicateFrontierRow);
    }
    if delta_refs > limits.max_delta_refs {
        return Err(ReasoningSurfaceError::DeltaReferenceLimit {
            limit: limits.max_delta_refs,
            observed: delta_refs,
        });
    }
    Ok(())
}

fn validate_evidence(
    evidence: &[EvidenceReference],
    limits: &ReasoningSurfaceLimits,
) -> Result<(), ReasoningSurfaceError> {
    if evidence.len() as u64 > limits.max_evidence_refs {
        return Err(ReasoningSurfaceError::EvidenceReferenceLimit {
            limit: limits.max_evidence_refs,
            observed: evidence.len() as u64,
        });
    }
    let mut bytes = 0_u64;
    for reference in evidence {
        validate_text("evidence_id", &reference.evidence_id)?;
        validate_contract("evidence provider", &reference.provider)?;
        validate_text("source_id", &reference.source_id)?;
        validate_text("source_revision", &reference.source_revision)?;
        validate_text("media_type", &reference.media_type)?;
        validate_digest(&reference.semantic_digest)?;
        bytes = bytes
            .checked_add(reference.byte_length)
            .ok_or(ReasoningSurfaceError::CountOverflow)?;
    }
    if evidence
        .windows(2)
        .any(|window| window[0].evidence_id >= window[1].evidence_id)
    {
        return Err(ReasoningSurfaceError::DuplicateEvidence);
    }
    if bytes > limits.max_total_evidence_bytes {
        return Err(ReasoningSurfaceError::EvidenceByteLimit {
            limit: limits.max_total_evidence_bytes,
            observed: bytes,
        });
    }
    Ok(())
}

fn validate_actions(
    actions: &[ContractIdentity],
    limits: &ReasoningSurfaceLimits,
) -> Result<(), ReasoningSurfaceError> {
    if actions.len() as u64 > limits.max_action_refs {
        return Err(ReasoningSurfaceError::ActionReferenceLimit {
            limit: limits.max_action_refs,
            observed: actions.len() as u64,
        });
    }
    for action in actions {
        validate_contract("action", action)?;
    }
    if actions
        .windows(2)
        .any(|window| window[0].id == window[1].id)
    {
        return Err(ReasoningSurfaceError::DuplicateAction);
    }
    Ok(())
}

fn validate_omissions(
    omissions: &[SurfaceOmission],
    completeness: SurfaceCompleteness,
    limits: &ReasoningSurfaceLimits,
) -> Result<(), ReasoningSurfaceError> {
    if omissions.len() as u64 > limits.max_omissions {
        return Err(ReasoningSurfaceError::OmissionLimit {
            limit: limits.max_omissions,
            observed: omissions.len() as u64,
        });
    }
    if (completeness == SurfaceCompleteness::Complete) != omissions.is_empty() {
        return Err(ReasoningSurfaceError::CompletenessMismatch);
    }
    for omission in omissions {
        if omission.omitted_count == 0 {
            return Err(ReasoningSurfaceError::ZeroOmission);
        }
        if let Some(subject) = &omission.subject_id {
            validate_text("omission subject", subject)?;
        }
        validate_text("omission reason", &omission.reason)?;
    }
    if omissions.windows(2).any(|window| window[0] >= window[1]) {
        return Err(ReasoningSurfaceError::DuplicateOmission);
    }
    Ok(())
}

fn validate_references(
    rows: &[ReasoningSurfaceRow],
    evidence: &[EvidenceReference],
    actions: &[ContractIdentity],
) -> Result<(), ReasoningSurfaceError> {
    let evidence_ids = evidence
        .iter()
        .map(|reference| reference.evidence_id.as_str())
        .collect::<BTreeSet<_>>();
    let action_ids = actions
        .iter()
        .map(|action| action.id.as_str())
        .collect::<BTreeSet<_>>();
    for row in rows {
        if let Some(missing) = row
            .evidence_ids
            .iter()
            .find(|id| !evidence_ids.contains(id.as_str()))
        {
            return Err(ReasoningSurfaceError::MissingEvidenceReference {
                row: row.frontier_row_id.clone(),
                evidence: missing.clone(),
            });
        }
        if let Some(missing) = row
            .admissible_action_ids
            .iter()
            .find(|id| !action_ids.contains(id.as_str()))
        {
            return Err(ReasoningSurfaceError::MissingActionReference {
                row: row.frontier_row_id.clone(),
                action: missing.clone(),
            });
        }
    }
    Ok(())
}

fn validate_contract(
    field: &'static str,
    contract: &ContractIdentity,
) -> Result<(), ReasoningSurfaceError> {
    validate_text(field, &contract.id)?;
    if contract.revision == 0 {
        return Err(ReasoningSurfaceError::ZeroRevision(field));
    }
    validate_digest(&contract.semantic_digest)
}

fn validate_text(field: &'static str, value: &str) -> Result<(), ReasoningSurfaceError> {
    if value.is_empty() || value.len() > 4_096 || value.chars().any(char::is_control) {
        return Err(ReasoningSurfaceError::InvalidText(field));
    }
    Ok(())
}

fn validate_digest(digest: &SemanticDigest) -> Result<(), ReasoningSurfaceError> {
    let value = digest.as_str();
    let Some(hex) = value.strip_prefix("blake3:") else {
        return Err(ReasoningSurfaceError::InvalidDigest(value.to_owned()));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ReasoningSurfaceError::InvalidDigest(value.to_owned()));
    }
    Ok(())
}

fn semantic_string_bytes(
    inputs: &ReasoningSurfaceInputs,
    rows: &[ReasoningSurfaceRow],
    evidence: &[EvidenceReference],
    actions: &[ContractIdentity],
    omissions: &[SurfaceOmission],
) -> Result<u64, ReasoningSurfaceError> {
    let mut total = 0_u64;
    for contract in [
        &inputs.workload,
        &inputs.graph,
        &inputs.scenario_suite,
        &inputs.space,
        &inputs.projection,
    ] {
        add_string_bytes(&mut total, &contract.id)?;
        add_string_bytes(&mut total, contract.semantic_digest.as_str())?;
    }
    for digest in [
        &inputs.campaign_id,
        &inputs.trace_id,
        &inputs.committed_transition_id,
        &inputs.transition_id,
        &inputs.scheduling_decision_id,
        &inputs.frontier_frame_id,
        &inputs.capability_snapshot_id,
    ] {
        add_string_bytes(&mut total, digest.as_str())?;
    }
    for row in rows {
        add_string_bytes(&mut total, &row.frontier_row_id)?;
        add_string_bytes(&mut total, &row.entity_kind)?;
        add_string_bytes(&mut total, &row.entity_id)?;
        for digest in row
            .transition_delta_ids
            .iter()
            .chain(&row.residual_delta_ids)
        {
            add_string_bytes(&mut total, digest.as_str())?;
        }
        for value in row
            .claim_ids
            .iter()
            .chain(&row.evidence_ids)
            .chain(&row.admissible_action_ids)
        {
            add_string_bytes(&mut total, value)?;
        }
    }
    for reference in evidence {
        add_string_bytes(&mut total, &reference.evidence_id)?;
        add_string_bytes(&mut total, &reference.provider.id)?;
        add_string_bytes(&mut total, reference.provider.semantic_digest.as_str())?;
        add_string_bytes(&mut total, &reference.source_id)?;
        add_string_bytes(&mut total, &reference.source_revision)?;
        add_string_bytes(&mut total, reference.semantic_digest.as_str())?;
        add_string_bytes(&mut total, &reference.media_type)?;
    }
    for action in actions {
        add_string_bytes(&mut total, &action.id)?;
        add_string_bytes(&mut total, action.semantic_digest.as_str())?;
    }
    for omission in omissions {
        if let Some(subject) = &omission.subject_id {
            add_string_bytes(&mut total, subject)?;
        }
        add_string_bytes(&mut total, &omission.reason)?;
    }
    Ok(total)
}

fn add_string_bytes(total: &mut u64, value: &str) -> Result<(), ReasoningSurfaceError> {
    *total = total
        .checked_add(value.len() as u64)
        .ok_or(ReasoningSurfaceError::CountOverflow)?;
    Ok(())
}

fn placeholder_digest() -> SemanticDigest {
    SemanticHasher::new("rey.reasoning-surface.placeholder").finish()
}

fn surface_digest(surface: &ReasoningSurface) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(REASONING_SURFACE_SCHEMA);
    add_contract(&mut hasher, &surface.inputs.workload);
    add_contract(&mut hasher, &surface.inputs.graph);
    add_contract(&mut hasher, &surface.inputs.scenario_suite);
    hasher.add_str(surface.inputs.campaign_id.as_str());
    add_contract(&mut hasher, &surface.inputs.space);
    hasher.add_str(surface.inputs.trace_id.as_str());
    hasher.add_str(surface.inputs.committed_transition_id.as_str());
    hasher.add_str(surface.inputs.transition_id.as_str());
    hasher.add_str(surface.inputs.scheduling_decision_id.as_str());
    hasher.add_str(surface.inputs.frontier_frame_id.as_str());
    hasher.add_str(surface.inputs.capability_snapshot_id.as_str());
    add_contract(&mut hasher, &surface.inputs.projection);
    add_limits(&mut hasher, &surface.limits);
    hasher.add_u64(surface.retrieval_iterations);
    hasher.add_str(surface.completeness.as_str());
    hasher.add_u64(surface.rows.len() as u64);
    for row in &surface.rows {
        hasher.add_str(&row.frontier_row_id);
        hasher.add_str(&row.entity_kind);
        hasher.add_str(&row.entity_id);
        add_digests(&mut hasher, &row.transition_delta_ids);
        add_digests(&mut hasher, &row.residual_delta_ids);
        add_strings(&mut hasher, &row.claim_ids);
        add_strings(&mut hasher, &row.evidence_ids);
        add_strings(&mut hasher, &row.admissible_action_ids);
    }
    hasher.add_u64(surface.evidence.len() as u64);
    for reference in &surface.evidence {
        hasher.add_str(&reference.evidence_id);
        add_contract(&mut hasher, &reference.provider);
        hasher.add_str(&reference.source_id);
        hasher.add_str(&reference.source_revision);
        hasher.add_str(reference.semantic_digest.as_str());
        hasher.add_str(&reference.media_type);
        hasher.add_u64(reference.byte_length);
    }
    hasher.add_u64(surface.admissible_actions.len() as u64);
    for action in &surface.admissible_actions {
        add_contract(&mut hasher, action);
    }
    hasher.add_u64(surface.omissions.len() as u64);
    for omission in &surface.omissions {
        hasher.add_str(omission.kind.as_str());
        hasher.add_optional_str(omission.subject_id.as_deref());
        hasher.add_u64(omission.omitted_count);
        hasher.add_str(&omission.reason);
    }
    hasher.finish()
}

fn add_contract(hasher: &mut SemanticHasher, contract: &ContractIdentity) {
    contract.add_semantics(hasher);
}

fn add_limits(hasher: &mut SemanticHasher, limits: &ReasoningSurfaceLimits) {
    hasher.add_u64(limits.max_rows);
    hasher.add_u64(limits.max_delta_refs);
    hasher.add_u64(limits.max_evidence_refs);
    hasher.add_u64(limits.max_action_refs);
    hasher.add_u64(limits.max_omissions);
    hasher.add_u64(limits.max_total_evidence_bytes);
    hasher.add_u64(limits.max_string_bytes);
    hasher.add_u64(limits.max_retrieval_iterations);
}

fn add_digests(hasher: &mut SemanticHasher, values: &[SemanticDigest]) {
    hasher.add_u64(values.len() as u64);
    for value in values {
        hasher.add_str(value.as_str());
    }
}

fn add_strings(hasher: &mut SemanticHasher, values: &[String]) {
    hasher.add_u64(values.len() as u64);
    for value in values {
        hasher.add_str(value);
    }
}

fn digest_arrays(
    rows: &[ReasoningSurfaceRow],
    select: impl Fn(&ReasoningSurfaceRow) -> &[SemanticDigest],
) -> Result<Vec<String>, serde_json::Error> {
    rows.iter()
        .map(|row| {
            serde_json::to_string(
                &select(row)
                    .iter()
                    .map(SemanticDigest::as_str)
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

fn string_arrays(
    rows: &[ReasoningSurfaceRow],
    select: impl Fn(&ReasoningSurfaceRow) -> &[String],
) -> Result<Vec<String>, serde_json::Error> {
    rows.iter()
        .map(|row| serde_json::to_string(select(row)))
        .collect()
}

#[derive(Debug, Error)]
pub enum ReasoningSurfaceError {
    #[error("unsupported reasoning surface schema {actual}; expected {expected}")]
    UnsupportedSchema {
        expected: &'static str,
        actual: String,
    },
    #[error("reasoning surface digest mismatch: declared {declared}, actual {actual}")]
    SurfaceDigest {
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("reasoning surface is not in canonical order")]
    NonCanonical,
    #[error("reasoning surface limit {0} must be greater than zero")]
    ZeroLimit(&'static str),
    #[error("reasoning surface must contain at least one projected frontier row")]
    EmptySurface,
    #[error("reasoning surface row limit {limit} exceeded by {observed}")]
    RowLimit { limit: u64, observed: u64 },
    #[error("reasoning surface delta-reference limit {limit} exceeded by {observed}")]
    DeltaReferenceLimit { limit: u64, observed: u64 },
    #[error("reasoning surface evidence-reference limit {limit} exceeded by {observed}")]
    EvidenceReferenceLimit { limit: u64, observed: u64 },
    #[error("reasoning surface action-reference limit {limit} exceeded by {observed}")]
    ActionReferenceLimit { limit: u64, observed: u64 },
    #[error("reasoning surface omission limit {limit} exceeded by {observed}")]
    OmissionLimit { limit: u64, observed: u64 },
    #[error("reasoning surface evidence-byte limit {limit} exceeded by {observed}")]
    EvidenceByteLimit { limit: u64, observed: u64 },
    #[error("reasoning surface string-byte limit {limit} exceeded by {observed}")]
    StringByteLimit { limit: u64, observed: u64 },
    #[error("reasoning surface retrieval-iteration limit {limit} exceeded by {observed}")]
    RetrievalIterationLimit { limit: u64, observed: u64 },
    #[error("invalid text field {0}")]
    InvalidText(&'static str),
    #[error("contract field {0} has revision zero")]
    ZeroRevision(&'static str),
    #[error("invalid semantic digest {0}")]
    InvalidDigest(String),
    #[error("duplicate frontier row id")]
    DuplicateFrontierRow,
    #[error("duplicate evidence id")]
    DuplicateEvidence,
    #[error("duplicate action id")]
    DuplicateAction,
    #[error("duplicate omission")]
    DuplicateOmission,
    #[error("surface completeness does not agree with omission evidence")]
    CompletenessMismatch,
    #[error("omission count must be greater than zero")]
    ZeroOmission,
    #[error("frontier row {0} cites neither a delta nor a claim")]
    UndirectedRow(String),
    #[error("frontier row {0} assigns one delta both transition and residual roles")]
    AmbiguousDeltaRole(String),
    #[error("frontier row {row} cites unknown evidence {evidence}")]
    MissingEvidenceReference { row: String, evidence: String },
    #[error("frontier row {row} cites unknown action {action}")]
    MissingActionReference { row: String, action: String },
    #[error("reasoning surface count overflowed")]
    CountOverflow,
    #[error("reasoning surface frame failed: {0}")]
    Frame(#[from] FrameError),
    #[error("reasoning surface JSON projection failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("reasoning surface dataframe failed: {0}")]
    Polars(#[from] polars::error::PolarsError),
}

#[cfg(test)]
mod tests {
    use rey_dataframe::Frame;

    use super::*;

    fn digest(value: &str) -> SemanticDigest {
        let mut hasher = SemanticHasher::new("rey.policy-test.v1");
        hasher.add_str(value);
        hasher.finish()
    }

    fn contract(id: &str) -> ContractIdentity {
        ContractIdentity::new(id, 1, id)
    }

    fn inputs() -> ReasoningSurfaceInputs {
        ReasoningSurfaceInputs {
            workload: contract("workload"),
            graph: contract("graph"),
            scenario_suite: contract("scenario-suite"),
            campaign_id: digest("campaign"),
            space: contract("space"),
            trace_id: digest("trace"),
            committed_transition_id: digest("committed-transition"),
            transition_id: digest("transition"),
            scheduling_decision_id: digest("scheduling-decision"),
            frontier_frame_id: digest("frontier"),
            capability_snapshot_id: digest("capabilities"),
            projection: contract("projection"),
        }
    }

    fn evidence(id: &str, bytes: u64) -> EvidenceReference {
        EvidenceReference {
            evidence_id: id.to_owned(),
            provider: contract("provider"),
            source_id: format!("source-{id}"),
            source_revision: "revision-1".to_owned(),
            semantic_digest: digest(id),
            media_type: "application/json".to_owned(),
            byte_length: bytes,
        }
    }

    fn row(id: &str, delta: &str, evidence_id: &str, action_id: &str) -> ReasoningSurfaceRow {
        ReasoningSurfaceRow {
            frontier_row_id: id.to_owned(),
            entity_kind: "symbol".to_owned(),
            entity_id: format!("entity-{id}"),
            transition_delta_ids: vec![digest(delta)],
            residual_delta_ids: Vec::new(),
            claim_ids: vec!["claim".to_owned()],
            evidence_ids: vec![evidence_id.to_owned()],
            admissible_action_ids: vec![action_id.to_owned()],
        }
    }

    fn surface() -> ReasoningSurface {
        ReasoningSurface::new(
            inputs(),
            ReasoningSurfaceLimits::default(),
            2,
            SurfaceCompleteness::Complete,
            vec![
                row("row-b", "delta-b", "evidence-b", "action-b"),
                row("row-a", "delta-a", "evidence-a", "action-a"),
            ],
            vec![evidence("evidence-b", 20), evidence("evidence-a", 10)],
            vec![contract("action-b"), contract("action-a")],
            Vec::new(),
        )
        .unwrap()
    }

    #[test]
    fn construction_is_canonical_and_deterministic() {
        let left = surface();
        let right = ReasoningSurface::new(
            inputs(),
            ReasoningSurfaceLimits::default(),
            2,
            SurfaceCompleteness::Complete,
            vec![
                row("row-a", "delta-a", "evidence-a", "action-a"),
                row("row-b", "delta-b", "evidence-b", "action-b"),
            ],
            vec![evidence("evidence-a", 10), evidence("evidence-b", 20)],
            vec![contract("action-a"), contract("action-b")],
            Vec::new(),
        )
        .unwrap();

        assert_eq!(left, right);
        assert_eq!(left.rows[0].frontier_row_id, "row-a");
        left.verify().unwrap();
    }

    #[test]
    fn frame_and_arrow_round_trip_preserve_surface_projection() {
        let surface = surface();
        let frame = surface.to_frame().unwrap();
        assert_eq!(frame.metadata().relation, REASONING_SURFACE_RELATION);
        assert_eq!(
            frame.metadata().semantic_digest,
            surface.surface_id.to_string()
        );
        assert_eq!(frame.metadata().row_count, 2);
        assert_eq!(
            frame.metadata().attributes["rey.scheduling-decision-id"],
            surface.inputs.scheduling_decision_id.to_string()
        );
        let decoded = Frame::from_arrow_stream(&frame.to_arrow_stream().unwrap()).unwrap();
        assert_eq!(decoded.metadata(), frame.metadata());
        assert!(decoded.dataframe().equals_missing(frame.dataframe()));
    }

    #[test]
    fn row_and_evidence_bounds_are_enforced() {
        let limits = ReasoningSurfaceLimits {
            max_rows: 1,
            ..ReasoningSurfaceLimits::default()
        };
        assert!(matches!(
            ReasoningSurface::new(
                inputs(),
                limits,
                1,
                SurfaceCompleteness::Complete,
                vec![
                    row("a", "a", "evidence", "action"),
                    row("b", "b", "evidence", "action")
                ],
                vec![evidence("evidence", 1)],
                vec![contract("action")],
                Vec::new(),
            ),
            Err(ReasoningSurfaceError::RowLimit { .. })
        ));

        let limits = ReasoningSurfaceLimits {
            max_total_evidence_bytes: 5,
            ..ReasoningSurfaceLimits::default()
        };
        assert!(matches!(
            ReasoningSurface::new(
                inputs(),
                limits,
                1,
                SurfaceCompleteness::Complete,
                vec![row("a", "a", "evidence", "action")],
                vec![evidence("evidence", 6)],
                vec![contract("action")],
                Vec::new(),
            ),
            Err(ReasoningSurfaceError::EvidenceByteLimit { .. })
        ));
    }

    #[test]
    fn every_row_reference_must_resolve() {
        assert!(matches!(
            ReasoningSurface::new(
                inputs(),
                ReasoningSurfaceLimits::default(),
                1,
                SurfaceCompleteness::Complete,
                vec![row("a", "delta", "missing", "action")],
                Vec::new(),
                vec![contract("action")],
                Vec::new(),
            ),
            Err(ReasoningSurfaceError::MissingEvidenceReference { .. })
        ));
    }

    #[test]
    fn completeness_requires_explicit_omissions() {
        let omission = SurfaceOmission {
            kind: OmissionKind::ByteLimit,
            subject_id: Some("provider".to_owned()),
            omitted_count: 1,
            reason: "retrieval byte bound reached".to_owned(),
        };
        assert!(matches!(
            ReasoningSurface::new(
                inputs(),
                ReasoningSurfaceLimits::default(),
                1,
                SurfaceCompleteness::Complete,
                vec![row("a", "delta", "evidence", "action")],
                vec![evidence("evidence", 1)],
                vec![contract("action")],
                vec![omission],
            ),
            Err(ReasoningSurfaceError::CompletenessMismatch)
        ));
    }

    #[test]
    fn a_surface_row_must_be_delta_or_claim_directed() {
        let mut undirected = row("a", "delta", "evidence", "action");
        undirected.transition_delta_ids.clear();
        undirected.claim_ids.clear();
        assert!(matches!(
            ReasoningSurface::new(
                inputs(),
                ReasoningSurfaceLimits::default(),
                1,
                SurfaceCompleteness::Complete,
                vec![undirected],
                vec![evidence("evidence", 1)],
                vec![contract("action")],
                Vec::new(),
            ),
            Err(ReasoningSurfaceError::UndirectedRow(_))
        ));
    }

    #[test]
    fn semantic_tampering_is_detected() {
        let mut surface = surface();
        surface.rows[0].entity_id = "changed".to_owned();
        assert!(matches!(
            surface.verify(),
            Err(ReasoningSurfaceError::SurfaceDigest { .. })
        ));
    }

    #[test]
    fn scheduling_decision_participates_in_surface_identity() {
        let left = surface();
        let mut changed_inputs = inputs();
        changed_inputs.scheduling_decision_id = digest("other-scheduling-decision");
        let right = ReasoningSurface::new(
            changed_inputs,
            ReasoningSurfaceLimits::default(),
            2,
            SurfaceCompleteness::Complete,
            vec![
                row("row-a", "delta-a", "evidence-a", "action-a"),
                row("row-b", "delta-b", "evidence-b", "action-b"),
            ],
            vec![evidence("evidence-a", 10), evidence("evidence-b", 20)],
            vec![contract("action-a"), contract("action-b")],
            Vec::new(),
        )
        .unwrap();

        assert_ne!(left.surface_id, right.surface_id);
    }

    #[test]
    fn surface_json_round_trip_preserves_identity() {
        let surface = surface();
        let encoded = serde_json::to_vec(&surface).unwrap();
        let decoded: ReasoningSurface = serde_json::from_slice(&encoded).unwrap();
        decoded.verify().unwrap();
        assert_eq!(decoded, surface);
    }
}
