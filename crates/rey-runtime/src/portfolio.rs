use std::collections::{BTreeMap, BTreeSet};

use polars::df;
use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_dataframe::{Frame, FrameError, FrameMetadata};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID: &str = "rey.portfolio.attention";
pub const PORTFOLIO_SNAPSHOT_SCHEMA: &str = "rey.portfolio-snapshot.v1";
pub const WORKLOAD_ATTENTION_RELATION: &str = "rey.workload-attention";
pub const WORKLOAD_ATTENTION_SCHEMA_VERSION: &str = "1";
pub const WORKLOAD_ATTENTION_SCHEMA: &str = "rey.workload-attention.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PortfolioLimits {
    pub max_workloads: u64,
    pub max_surfaces: u64,
    pub max_evidence_refs: u64,
    pub max_dependency_refs: u64,
    pub max_attention_rows: u64,
    pub max_string_bytes: u64,
}

impl Default for PortfolioLimits {
    fn default() -> Self {
        Self {
            max_workloads: 64,
            max_surfaces: 256,
            max_evidence_refs: 1_024,
            max_dependency_refs: 1_024,
            max_attention_rows: 512,
            max_string_bytes: 512 * 1_024,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioQualificationState {
    Untested,
    Qualified,
    Failing,
    Inconclusive,
    Stale,
}

impl PortfolioQualificationState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Untested => "untested",
            Self::Qualified => "qualified",
            Self::Failing => "failing",
            Self::Inconclusive => "inconclusive",
            Self::Stale => "stale",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionPolicy {
    Track,
    Exclude,
}

impl AttentionPolicy {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Track => "track",
            Self::Exclude => "exclude",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PortfolioWorkloadObservation {
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub qualification: PortfolioQualificationState,
    pub policy: AttentionPolicy,
    pub policy_reason: Option<String>,
    pub evidence_ids: Vec<SemanticDigest>,
    pub changed_dependency_ids: Vec<String>,
    pub missing_capability_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PortfolioSurfaceObservation {
    pub surface_id: String,
    pub source_revision: SemanticDigest,
    pub owners: Vec<String>,
    pub evidence_ids: Vec<SemanticDigest>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PortfolioSnapshot {
    pub schema: String,
    pub snapshot_id: SemanticDigest,
    pub catalog_id: SemanticDigest,
    pub environment_snapshot_id: Option<SemanticDigest>,
    pub workloads: Vec<PortfolioWorkloadObservation>,
    pub surfaces: Vec<PortfolioSurfaceObservation>,
    pub limits: PortfolioLimits,
}

impl PortfolioSnapshot {
    pub fn new(
        catalog_id: SemanticDigest,
        environment_snapshot_id: Option<SemanticDigest>,
        mut workloads: Vec<PortfolioWorkloadObservation>,
        mut surfaces: Vec<PortfolioSurfaceObservation>,
        limits: PortfolioLimits,
    ) -> Result<Self, PortfolioError> {
        validate_limits(&limits)?;
        canonicalize_workloads(&mut workloads);
        canonicalize_surfaces(&mut surfaces);
        enforce_count("workloads", workloads.len(), limits.max_workloads)?;
        enforce_count("surfaces", surfaces.len(), limits.max_surfaces)?;
        let mut snapshot = Self {
            schema: PORTFOLIO_SNAPSHOT_SCHEMA.to_owned(),
            snapshot_id: placeholder_digest("rey.portfolio-snapshot.placeholder"),
            catalog_id,
            environment_snapshot_id,
            workloads,
            surfaces,
            limits,
        };
        validate_snapshot_shape(&snapshot)?;
        snapshot.snapshot_id = snapshot_digest(&snapshot);
        Ok(snapshot)
    }

    pub fn verify(&self) -> Result<(), PortfolioError> {
        if self.schema != PORTFOLIO_SNAPSHOT_SCHEMA {
            return Err(PortfolioError::UnsupportedSchema {
                expected: PORTFOLIO_SNAPSHOT_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_limits(&self.limits)?;
        validate_snapshot_shape(self)?;
        let mut workloads = self.workloads.clone();
        let mut surfaces = self.surfaces.clone();
        canonicalize_workloads(&mut workloads);
        canonicalize_surfaces(&mut surfaces);
        if workloads != self.workloads || surfaces != self.surfaces {
            return Err(PortfolioError::NonCanonical);
        }
        let actual = snapshot_digest(self);
        if actual != self.snapshot_id {
            return Err(PortfolioError::Digest {
                role: "portfolio snapshot",
                declared: self.snapshot_id.clone(),
                actual,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionAction {
    Refine,
    Retest,
    Create,
    Block,
    PolicyExcluded,
}

impl AttentionAction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Refine => "refine",
            Self::Retest => "retest",
            Self::Create => "create",
            Self::Block => "block",
            Self::PolicyExcluded => "policy_excluded",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionReason {
    RequiredScenarioFailing,
    StaleEvidence,
    Untested,
    DependencyChanged,
    RequiredCapabilityUnavailable,
    InconclusiveEvidence,
    UnownedSurface,
    PolicyExcluded,
}

impl AttentionReason {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RequiredScenarioFailing => "required_scenario_failing",
            Self::StaleEvidence => "stale_evidence",
            Self::Untested => "untested",
            Self::DependencyChanged => "dependency_changed",
            Self::RequiredCapabilityUnavailable => "required_capability_unavailable",
            Self::InconclusiveEvidence => "inconclusive_evidence",
            Self::UnownedSurface => "unowned_surface",
            Self::PolicyExcluded => "policy_excluded",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionSubjectKind {
    Workload,
    Surface,
}

impl AttentionSubjectKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Workload => "workload",
            Self::Surface => "surface",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionReadiness {
    Ready,
    Blocked,
    Excluded,
}

impl AttentionReadiness {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Blocked => "blocked",
            Self::Excluded => "excluded",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadAttentionRow {
    pub row_id: SemanticDigest,
    pub action: AttentionAction,
    pub subject_kind: AttentionSubjectKind,
    pub subject_id: String,
    pub workload: Option<ContractIdentity>,
    pub graph: Option<ContractIdentity>,
    pub reason: AttentionReason,
    pub readiness: AttentionReadiness,
    pub evidence_ids: Vec<SemanticDigest>,
    pub dependency_ids: Vec<String>,
    pub priority: u64,
    pub estimated_cost_units: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadAttentionSummary {
    pub refine: u64,
    pub retest: u64,
    pub create: u64,
    pub blocked: u64,
    pub policy_excluded: u64,
    pub workloads: u64,
    pub surfaces: u64,
    pub owned_surfaces: u64,
    pub unowned_surfaces: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadAttention {
    pub schema: String,
    pub attention_id: SemanticDigest,
    pub source_snapshot_id: SemanticDigest,
    pub derivation: ContractIdentity,
    pub limits: PortfolioLimits,
    pub rows: Vec<WorkloadAttentionRow>,
    pub summary: WorkloadAttentionSummary,
}

impl WorkloadAttention {
    pub fn derive(snapshot: &PortfolioSnapshot) -> Result<Self, PortfolioError> {
        snapshot.verify()?;
        let derivation = portfolio_attention_operation();
        let mut rows = Vec::new();
        for workload in &snapshot.workloads {
            if let Some(row) = workload_attention_row(workload) {
                rows.push(row);
            }
        }
        for surface in &snapshot.surfaces {
            if surface.owners.is_empty() {
                rows.push(attention_row(
                    AttentionAction::Create,
                    AttentionSubjectKind::Surface,
                    surface.surface_id.clone(),
                    None,
                    None,
                    AttentionReason::UnownedSurface,
                    AttentionReadiness::Ready,
                    surface.evidence_ids.clone(),
                    Vec::new(),
                ));
            }
        }
        rows.sort_by(|left, right| {
            (left.subject_kind, &left.subject_id, left.action).cmp(&(
                right.subject_kind,
                &right.subject_id,
                right.action,
            ))
        });
        enforce_count(
            "attention rows",
            rows.len(),
            snapshot.limits.max_attention_rows,
        )?;
        let summary = summarize(snapshot, &rows);
        let mut attention = Self {
            schema: WORKLOAD_ATTENTION_SCHEMA.to_owned(),
            attention_id: placeholder_digest("rey.workload-attention.placeholder"),
            source_snapshot_id: snapshot.snapshot_id.clone(),
            derivation,
            limits: snapshot.limits.clone(),
            rows,
            summary,
        };
        validate_attention_shape(&attention)?;
        attention.attention_id = attention_digest(&attention);
        Ok(attention)
    }

    pub fn verify(&self) -> Result<(), PortfolioError> {
        if self.schema != WORKLOAD_ATTENTION_SCHEMA {
            return Err(PortfolioError::UnsupportedSchema {
                expected: WORKLOAD_ATTENTION_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_limits(&self.limits)?;
        if self.derivation != portfolio_attention_operation() {
            return Err(PortfolioError::InvalidDerivation);
        }
        validate_attention_shape(self)?;
        let actual = attention_digest(self);
        if actual != self.attention_id {
            return Err(PortfolioError::Digest {
                role: "workload attention",
                declared: self.attention_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    pub fn verify_against(&self, snapshot: &PortfolioSnapshot) -> Result<(), PortfolioError> {
        snapshot.verify()?;
        if self.source_snapshot_id != snapshot.snapshot_id {
            return Err(PortfolioError::SourceSnapshotMismatch);
        }
        let expected = Self::derive(snapshot)?;
        if self != &expected {
            return Err(PortfolioError::DerivationMismatch);
        }
        Ok(())
    }

    pub fn to_frame(&self) -> Result<Frame, PortfolioError> {
        self.verify()?;
        let action = self
            .rows
            .iter()
            .map(|row| row.action.as_str())
            .collect::<Vec<_>>();
        let subject_kind = self
            .rows
            .iter()
            .map(|row| row.subject_kind.as_str())
            .collect::<Vec<_>>();
        let subject_id = self
            .rows
            .iter()
            .map(|row| row.subject_id.as_str())
            .collect::<Vec<_>>();
        let reason = self
            .rows
            .iter()
            .map(|row| row.reason.as_str())
            .collect::<Vec<_>>();
        let readiness = self
            .rows
            .iter()
            .map(|row| row.readiness.as_str())
            .collect::<Vec<_>>();
        let priority = self.rows.iter().map(|row| row.priority).collect::<Vec<_>>();
        let estimated_cost_units = self
            .rows
            .iter()
            .map(|row| row.estimated_cost_units)
            .collect::<Vec<_>>();
        let row_id = self
            .rows
            .iter()
            .map(|row| row.row_id.as_str())
            .collect::<Vec<_>>();
        let dataframe = df!(
            "row_id" => row_id,
            "action" => action,
            "subject_kind" => subject_kind,
            "subject_id" => subject_id,
            "reason" => reason,
            "readiness" => readiness,
            "priority" => priority,
            "estimated_cost_units" => estimated_cost_units,
        )?;
        Ok(Frame::new(
            dataframe,
            FrameMetadata {
                relation: WORKLOAD_ATTENTION_RELATION.to_owned(),
                schema_version: WORKLOAD_ATTENTION_SCHEMA_VERSION.to_owned(),
                semantic_digest: self.attention_id.to_string(),
                row_count: self.rows.len() as u64,
                complete: true,
                key_columns: vec!["row_id".to_owned()],
                attributes: BTreeMap::from([(
                    "source_snapshot_id".to_owned(),
                    self.source_snapshot_id.to_string(),
                )]),
            },
        )?)
    }
}

#[must_use]
pub fn portfolio_attention_operation() -> ContractIdentity {
    ContractIdentity::new(
        "rey.portfolio.attention.derive",
        1,
        "derive bounded workload and surface attention rows from one canonical portfolio snapshot; emit explicit blocked and policy-excluded rows",
    )
}

#[must_use]
pub fn render_workload_attention_operation() -> ContractIdentity {
    ContractIdentity::new(
        "rey.portfolio.attention.render-lines",
        1,
        "render canonical workload-attention rows as action subject-kind subject-id reason readiness UTF-8 lines without changing assessment",
    )
}

#[must_use]
pub fn render_workload_attention(attention: &WorkloadAttention) -> String {
    let mut rendered = String::new();
    for row in &attention.rows {
        rendered.push_str(row.action.as_str());
        rendered.push(' ');
        rendered.push_str(row.subject_kind.as_str());
        rendered.push(' ');
        rendered.push_str(&row.subject_id);
        rendered.push(' ');
        rendered.push_str(row.reason.as_str());
        rendered.push(' ');
        rendered.push_str(row.readiness.as_str());
        rendered.push('\n');
    }
    rendered
}

fn workload_attention_row(workload: &PortfolioWorkloadObservation) -> Option<WorkloadAttentionRow> {
    let candidate = if !workload.missing_capability_ids.is_empty() {
        Some((
            AttentionAction::Block,
            AttentionReason::RequiredCapabilityUnavailable,
            AttentionReadiness::Blocked,
            workload.missing_capability_ids.clone(),
        ))
    } else {
        match workload.qualification {
            PortfolioQualificationState::Failing => Some((
                AttentionAction::Refine,
                AttentionReason::RequiredScenarioFailing,
                AttentionReadiness::Ready,
                Vec::new(),
            )),
            PortfolioQualificationState::Inconclusive => Some((
                AttentionAction::Block,
                AttentionReason::InconclusiveEvidence,
                AttentionReadiness::Blocked,
                Vec::new(),
            )),
            PortfolioQualificationState::Stale => Some((
                AttentionAction::Retest,
                AttentionReason::StaleEvidence,
                AttentionReadiness::Ready,
                workload.changed_dependency_ids.clone(),
            )),
            PortfolioQualificationState::Untested => Some((
                AttentionAction::Retest,
                AttentionReason::Untested,
                AttentionReadiness::Ready,
                Vec::new(),
            )),
            PortfolioQualificationState::Qualified
                if !workload.changed_dependency_ids.is_empty() =>
            {
                Some((
                    AttentionAction::Retest,
                    AttentionReason::DependencyChanged,
                    AttentionReadiness::Ready,
                    workload.changed_dependency_ids.clone(),
                ))
            }
            PortfolioQualificationState::Qualified => None,
        }
    };
    let (action, reason, readiness, dependencies) = candidate?;
    if workload.policy == AttentionPolicy::Exclude {
        return Some(attention_row(
            AttentionAction::PolicyExcluded,
            AttentionSubjectKind::Workload,
            workload.workload.id.clone(),
            Some(workload.workload.clone()),
            Some(workload.graph.clone()),
            AttentionReason::PolicyExcluded,
            AttentionReadiness::Excluded,
            workload.evidence_ids.clone(),
            workload
                .policy_reason
                .clone()
                .into_iter()
                .chain(dependencies)
                .collect(),
        ));
    }
    Some(attention_row(
        action,
        AttentionSubjectKind::Workload,
        workload.workload.id.clone(),
        Some(workload.workload.clone()),
        Some(workload.graph.clone()),
        reason,
        readiness,
        workload.evidence_ids.clone(),
        dependencies,
    ))
}

#[allow(clippy::too_many_arguments)]
fn attention_row(
    action: AttentionAction,
    subject_kind: AttentionSubjectKind,
    subject_id: String,
    workload: Option<ContractIdentity>,
    graph: Option<ContractIdentity>,
    reason: AttentionReason,
    readiness: AttentionReadiness,
    mut evidence_ids: Vec<SemanticDigest>,
    mut dependency_ids: Vec<String>,
) -> WorkloadAttentionRow {
    evidence_ids.sort();
    evidence_ids.dedup();
    dependency_ids.sort();
    dependency_ids.dedup();
    let (priority, estimated_cost_units) = match action {
        AttentionAction::Refine => (100, 3),
        AttentionAction::Retest => (90, 1),
        AttentionAction::Create => (80, 5),
        AttentionAction::Block | AttentionAction::PolicyExcluded => (0, 0),
    };
    let mut row = WorkloadAttentionRow {
        row_id: placeholder_digest("rey.workload-attention-row.placeholder"),
        action,
        subject_kind,
        subject_id,
        workload,
        graph,
        reason,
        readiness,
        evidence_ids,
        dependency_ids,
        priority,
        estimated_cost_units,
    };
    row.row_id = row_digest(&row);
    row
}

fn summarize(
    snapshot: &PortfolioSnapshot,
    rows: &[WorkloadAttentionRow],
) -> WorkloadAttentionSummary {
    let mut summary = WorkloadAttentionSummary {
        workloads: snapshot.workloads.len() as u64,
        surfaces: snapshot.surfaces.len() as u64,
        owned_surfaces: snapshot
            .surfaces
            .iter()
            .filter(|surface| !surface.owners.is_empty())
            .count() as u64,
        unowned_surfaces: snapshot
            .surfaces
            .iter()
            .filter(|surface| surface.owners.is_empty())
            .count() as u64,
        ..WorkloadAttentionSummary::default()
    };
    for row in rows {
        match row.action {
            AttentionAction::Refine => summary.refine += 1,
            AttentionAction::Retest => summary.retest += 1,
            AttentionAction::Create => summary.create += 1,
            AttentionAction::Block => summary.blocked += 1,
            AttentionAction::PolicyExcluded => summary.policy_excluded += 1,
        }
    }
    summary
}

fn canonicalize_workloads(workloads: &mut [PortfolioWorkloadObservation]) {
    for workload in workloads.iter_mut() {
        workload.evidence_ids.sort();
        workload.evidence_ids.dedup();
        workload.changed_dependency_ids.sort();
        workload.changed_dependency_ids.dedup();
        workload.missing_capability_ids.sort();
        workload.missing_capability_ids.dedup();
    }
    workloads.sort_by(|left, right| left.workload.id.cmp(&right.workload.id));
}

fn canonicalize_surfaces(surfaces: &mut [PortfolioSurfaceObservation]) {
    for surface in surfaces.iter_mut() {
        surface.owners.sort();
        surface.owners.dedup();
        surface.evidence_ids.sort();
        surface.evidence_ids.dedup();
    }
    surfaces.sort_by(|left, right| left.surface_id.cmp(&right.surface_id));
}

fn validate_snapshot_shape(snapshot: &PortfolioSnapshot) -> Result<(), PortfolioError> {
    validate_digest(&snapshot.catalog_id)?;
    if let Some(environment) = &snapshot.environment_snapshot_id {
        validate_digest(environment)?;
    }
    enforce_count(
        "workloads",
        snapshot.workloads.len(),
        snapshot.limits.max_workloads,
    )?;
    enforce_count(
        "surfaces",
        snapshot.surfaces.len(),
        snapshot.limits.max_surfaces,
    )?;
    let mut strings = snapshot.schema.len() as u64;
    let mut evidence = 0_u64;
    let mut dependencies = 0_u64;
    let mut workload_ids = BTreeSet::new();
    for workload in &snapshot.workloads {
        if !workload_ids.insert(workload.workload.id.as_str()) {
            return Err(PortfolioError::Duplicate(workload.workload.id.clone()));
        }
        validate_contract(&workload.workload)?;
        validate_contract(&workload.graph)?;
        validate_policy(workload)?;
        validate_text(&workload.workload.id)?;
        strings = strings
            .checked_add(workload.workload.id.len() as u64)
            .and_then(|value| value.checked_add(workload.graph.id.len() as u64))
            .ok_or(PortfolioError::Overflow)?;
        for value in workload
            .changed_dependency_ids
            .iter()
            .chain(&workload.missing_capability_ids)
        {
            validate_text(value)?;
            strings = strings
                .checked_add(value.len() as u64)
                .ok_or(PortfolioError::Overflow)?;
        }
        for evidence_id in &workload.evidence_ids {
            validate_digest(evidence_id)?;
        }
        evidence = evidence
            .checked_add(workload.evidence_ids.len() as u64)
            .ok_or(PortfolioError::Overflow)?;
        dependencies = dependencies
            .checked_add(workload.changed_dependency_ids.len() as u64)
            .and_then(|value| value.checked_add(workload.missing_capability_ids.len() as u64))
            .ok_or(PortfolioError::Overflow)?;
    }
    let mut surface_ids = BTreeSet::new();
    for surface in &snapshot.surfaces {
        validate_text(&surface.surface_id)?;
        validate_digest(&surface.source_revision)?;
        if !surface_ids.insert(surface.surface_id.as_str()) {
            return Err(PortfolioError::Duplicate(surface.surface_id.clone()));
        }
        strings = strings
            .checked_add(surface.surface_id.len() as u64)
            .ok_or(PortfolioError::Overflow)?;
        for owner in &surface.owners {
            validate_text(owner)?;
            strings = strings
                .checked_add(owner.len() as u64)
                .ok_or(PortfolioError::Overflow)?;
        }
        for evidence_id in &surface.evidence_ids {
            validate_digest(evidence_id)?;
        }
        evidence = evidence
            .checked_add(surface.evidence_ids.len() as u64)
            .ok_or(PortfolioError::Overflow)?;
    }
    if evidence > snapshot.limits.max_evidence_refs {
        return Err(PortfolioError::CountLimit {
            role: "evidence references",
            limit: snapshot.limits.max_evidence_refs,
            observed: evidence,
        });
    }
    if dependencies > snapshot.limits.max_dependency_refs {
        return Err(PortfolioError::CountLimit {
            role: "dependency references",
            limit: snapshot.limits.max_dependency_refs,
            observed: dependencies,
        });
    }
    if strings > snapshot.limits.max_string_bytes {
        return Err(PortfolioError::StringLimit {
            limit: snapshot.limits.max_string_bytes,
            observed: strings,
        });
    }
    Ok(())
}

fn validate_attention_shape(attention: &WorkloadAttention) -> Result<(), PortfolioError> {
    validate_digest(&attention.source_snapshot_id)?;
    enforce_count(
        "attention rows",
        attention.rows.len(),
        attention.limits.max_attention_rows,
    )?;
    if attention.rows.windows(2).any(|window| {
        (
            &window[0].subject_kind,
            &window[0].subject_id,
            &window[0].action,
        ) >= (
            &window[1].subject_kind,
            &window[1].subject_id,
            &window[1].action,
        )
    }) {
        return Err(PortfolioError::NonCanonical);
    }
    let mut row_ids = BTreeSet::new();
    let mut action_summary = WorkloadAttentionSummary::default();
    for row in &attention.rows {
        validate_text(&row.subject_id)?;
        validate_digest(&row.row_id)?;
        if !row_ids.insert(row.row_id.as_str()) {
            return Err(PortfolioError::Duplicate(row.row_id.to_string()));
        }
        if row.row_id != row_digest(row) {
            return Err(PortfolioError::InvalidRowDigest(row.subject_id.clone()));
        }
        if row.readiness == AttentionReadiness::Ready && row.priority == 0
            || row.readiness != AttentionReadiness::Ready && row.priority != 0
        {
            return Err(PortfolioError::InvalidRow(row.subject_id.clone()));
        }
        let expected_execution = match row.action {
            AttentionAction::Refine => (AttentionReadiness::Ready, 100, 3),
            AttentionAction::Retest => (AttentionReadiness::Ready, 90, 1),
            AttentionAction::Create => (AttentionReadiness::Ready, 80, 5),
            AttentionAction::Block => (AttentionReadiness::Blocked, 0, 0),
            AttentionAction::PolicyExcluded => (AttentionReadiness::Excluded, 0, 0),
        };
        if (row.readiness, row.priority, row.estimated_cost_units) != expected_execution {
            return Err(PortfolioError::InvalidRow(row.subject_id.clone()));
        }
        if row.subject_kind == AttentionSubjectKind::Workload
            && (row.workload.is_none() || row.graph.is_none())
            || row.subject_kind == AttentionSubjectKind::Surface
                && (row.workload.is_some() || row.graph.is_some())
        {
            return Err(PortfolioError::InvalidRow(row.subject_id.clone()));
        }
        if let Some(workload) = &row.workload {
            validate_contract(workload)?;
        }
        if let Some(graph) = &row.graph {
            validate_contract(graph)?;
        }
        if !is_canonical(&row.evidence_ids) || !is_canonical(&row.dependency_ids) {
            return Err(PortfolioError::NonCanonical);
        }
        for evidence_id in &row.evidence_ids {
            validate_digest(evidence_id)?;
        }
        for dependency_id in &row.dependency_ids {
            validate_text(dependency_id)?;
        }
        match row.action {
            AttentionAction::Refine => action_summary.refine += 1,
            AttentionAction::Retest => action_summary.retest += 1,
            AttentionAction::Create => action_summary.create += 1,
            AttentionAction::Block => action_summary.blocked += 1,
            AttentionAction::PolicyExcluded => action_summary.policy_excluded += 1,
        }
    }
    if action_summary.refine != attention.summary.refine
        || action_summary.retest != attention.summary.retest
        || action_summary.create != attention.summary.create
        || action_summary.blocked != attention.summary.blocked
        || action_summary.policy_excluded != attention.summary.policy_excluded
        || attention
            .summary
            .owned_surfaces
            .saturating_add(attention.summary.unowned_surfaces)
            != attention.summary.surfaces
    {
        return Err(PortfolioError::InvalidSummary);
    }
    Ok(())
}

fn is_canonical<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|window| window[0] < window[1])
}

fn validate_policy(workload: &PortfolioWorkloadObservation) -> Result<(), PortfolioError> {
    match (workload.policy, workload.policy_reason.as_deref()) {
        (AttentionPolicy::Track, None) | (AttentionPolicy::Exclude, Some(_)) => Ok(()),
        _ => Err(PortfolioError::InvalidPolicy(workload.workload.id.clone())),
    }
}

fn validate_limits(limits: &PortfolioLimits) -> Result<(), PortfolioError> {
    if limits.max_workloads == 0
        || limits.max_surfaces == 0
        || limits.max_evidence_refs == 0
        || limits.max_dependency_refs == 0
        || limits.max_attention_rows == 0
        || limits.max_string_bytes == 0
    {
        return Err(PortfolioError::InvalidLimit);
    }
    Ok(())
}

fn enforce_count(role: &'static str, observed: usize, limit: u64) -> Result<(), PortfolioError> {
    if observed as u64 > limit {
        return Err(PortfolioError::CountLimit {
            role,
            limit,
            observed: observed as u64,
        });
    }
    Ok(())
}

fn validate_text(value: &str) -> Result<(), PortfolioError> {
    if value.is_empty() || value.contains('\0') {
        return Err(PortfolioError::InvalidText);
    }
    Ok(())
}

fn validate_contract(contract: &ContractIdentity) -> Result<(), PortfolioError> {
    validate_text(&contract.id)?;
    if contract.revision == 0 {
        return Err(PortfolioError::InvalidContract(contract.id.clone()));
    }
    validate_digest(&contract.semantic_digest)?;
    Ok(())
}

fn validate_digest(digest: &SemanticDigest) -> Result<(), PortfolioError> {
    let value = digest.as_str();
    if value.len() != "blake3:".len() + 64
        || !value.starts_with("blake3:")
        || !value["blake3:".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(PortfolioError::InvalidDigest(value.to_owned()));
    }
    Ok(())
}

fn snapshot_digest(snapshot: &PortfolioSnapshot) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(PORTFOLIO_SNAPSHOT_SCHEMA);
    hasher.add_str(snapshot.catalog_id.as_str());
    hasher.add_optional_str(
        snapshot
            .environment_snapshot_id
            .as_ref()
            .map(SemanticDigest::as_str),
    );
    add_limits(&mut hasher, &snapshot.limits);
    hasher.add_u64(snapshot.workloads.len() as u64);
    for workload in &snapshot.workloads {
        workload.workload.add_semantics(&mut hasher);
        workload.graph.add_semantics(&mut hasher);
        hasher.add_str(workload.qualification.as_str());
        hasher.add_str(workload.policy.as_str());
        hasher.add_optional_str(workload.policy_reason.as_deref());
        add_digests(&mut hasher, &workload.evidence_ids);
        add_strings(&mut hasher, &workload.changed_dependency_ids);
        add_strings(&mut hasher, &workload.missing_capability_ids);
    }
    hasher.add_u64(snapshot.surfaces.len() as u64);
    for surface in &snapshot.surfaces {
        hasher.add_str(&surface.surface_id);
        hasher.add_str(surface.source_revision.as_str());
        add_strings(&mut hasher, &surface.owners);
        add_digests(&mut hasher, &surface.evidence_ids);
    }
    hasher.finish()
}

fn row_digest(row: &WorkloadAttentionRow) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.workload-attention-row.v1");
    hasher.add_str(row.action.as_str());
    hasher.add_str(row.subject_kind.as_str());
    hasher.add_str(&row.subject_id);
    hasher.add_bool(row.workload.is_some());
    if let Some(workload) = &row.workload {
        workload.add_semantics(&mut hasher);
    }
    hasher.add_bool(row.graph.is_some());
    if let Some(graph) = &row.graph {
        graph.add_semantics(&mut hasher);
    }
    hasher.add_str(row.reason.as_str());
    hasher.add_str(row.readiness.as_str());
    add_digests(&mut hasher, &row.evidence_ids);
    add_strings(&mut hasher, &row.dependency_ids);
    hasher.add_u64(row.priority);
    hasher.add_u64(row.estimated_cost_units);
    hasher.finish()
}

fn attention_digest(attention: &WorkloadAttention) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(WORKLOAD_ATTENTION_SCHEMA);
    hasher.add_str(attention.source_snapshot_id.as_str());
    attention.derivation.add_semantics(&mut hasher);
    add_limits(&mut hasher, &attention.limits);
    hasher.add_u64(attention.rows.len() as u64);
    for row in &attention.rows {
        hasher.add_str(row.row_id.as_str());
    }
    hasher.add_u64(attention.summary.refine);
    hasher.add_u64(attention.summary.retest);
    hasher.add_u64(attention.summary.create);
    hasher.add_u64(attention.summary.blocked);
    hasher.add_u64(attention.summary.policy_excluded);
    hasher.add_u64(attention.summary.workloads);
    hasher.add_u64(attention.summary.surfaces);
    hasher.add_u64(attention.summary.owned_surfaces);
    hasher.add_u64(attention.summary.unowned_surfaces);
    hasher.finish()
}

fn add_limits(hasher: &mut SemanticHasher, limits: &PortfolioLimits) {
    hasher.add_u64(limits.max_workloads);
    hasher.add_u64(limits.max_surfaces);
    hasher.add_u64(limits.max_evidence_refs);
    hasher.add_u64(limits.max_dependency_refs);
    hasher.add_u64(limits.max_attention_rows);
    hasher.add_u64(limits.max_string_bytes);
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

fn placeholder_digest(domain: &str) -> SemanticDigest {
    SemanticHasher::new(domain).finish()
}

#[derive(Debug, Error)]
pub enum PortfolioError {
    #[error("portfolio limits must be greater than zero")]
    InvalidLimit,
    #[error("unsupported portfolio schema {actual}; expected {expected}")]
    UnsupportedSchema {
        expected: &'static str,
        actual: String,
    },
    #[error("portfolio text must be non-empty and contain no NUL")]
    InvalidText,
    #[error("invalid portfolio contract {0}")]
    InvalidContract(String),
    #[error("invalid portfolio semantic digest {0}")]
    InvalidDigest(String),
    #[error("invalid attention policy for workload {0}")]
    InvalidPolicy(String),
    #[error("invalid portfolio-attention derivation contract")]
    InvalidDerivation,
    #[error("portfolio attention source snapshot does not match")]
    SourceSnapshotMismatch,
    #[error("portfolio attention does not match deterministic derivation")]
    DerivationMismatch,
    #[error("portfolio relation is not in canonical order")]
    NonCanonical,
    #[error("duplicate portfolio identity {0}")]
    Duplicate(String),
    #[error("{role} count limit {limit} exceeded by {observed}")]
    CountLimit {
        role: &'static str,
        limit: u64,
        observed: u64,
    },
    #[error("portfolio string-byte limit {limit} exceeded by {observed}")]
    StringLimit { limit: u64, observed: u64 },
    #[error("portfolio count overflowed")]
    Overflow,
    #[error("{role} digest mismatch: declared {declared}, actual {actual}")]
    Digest {
        role: &'static str,
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("attention row for {0} has an invalid digest")]
    InvalidRowDigest(String),
    #[error("attention row for {0} has an invalid shape")]
    InvalidRow(String),
    #[error("workload-attention summary does not match its rows or coverage")]
    InvalidSummary,
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error(transparent)]
    Polars(#[from] polars::error::PolarsError),
}

#[cfg(test)]
mod tests {
    use rey_core::{ContractIdentity, SemanticHasher};

    use super::{
        AttentionAction, AttentionPolicy, AttentionReadiness, PortfolioLimits,
        PortfolioQualificationState, PortfolioSnapshot, PortfolioSurfaceObservation,
        PortfolioWorkloadObservation, WorkloadAttention,
    };

    fn contract(id: &str) -> ContractIdentity {
        ContractIdentity::new(id, 1, id)
    }

    fn observation(
        id: &str,
        qualification: PortfolioQualificationState,
        policy: AttentionPolicy,
    ) -> PortfolioWorkloadObservation {
        PortfolioWorkloadObservation {
            workload: contract(id),
            graph: contract(&format!("{id}.graph")),
            qualification,
            policy,
            policy_reason: (policy == AttentionPolicy::Exclude).then(|| "fixture".to_owned()),
            evidence_ids: Vec::new(),
            changed_dependency_ids: Vec::new(),
            missing_capability_ids: Vec::new(),
        }
    }

    #[test]
    fn derivation_keeps_actionable_blocked_excluded_and_covered_states_distinct() {
        let mut blocked = observation(
            "blocked",
            PortfolioQualificationState::Qualified,
            AttentionPolicy::Track,
        );
        blocked.missing_capability_ids = vec!["tool.parser".to_owned()];
        let mut changed = observation(
            "changed",
            PortfolioQualificationState::Qualified,
            AttentionPolicy::Track,
        );
        changed.changed_dependency_ids = vec!["ENV@2".to_owned()];
        let snapshot = PortfolioSnapshot::new(
            SemanticHasher::new("catalog").finish(),
            None,
            vec![
                blocked,
                changed,
                observation(
                    "clean",
                    PortfolioQualificationState::Qualified,
                    AttentionPolicy::Track,
                ),
                observation(
                    "excluded",
                    PortfolioQualificationState::Failing,
                    AttentionPolicy::Exclude,
                ),
                observation(
                    "failing",
                    PortfolioQualificationState::Failing,
                    AttentionPolicy::Track,
                ),
                observation(
                    "inconclusive",
                    PortfolioQualificationState::Inconclusive,
                    AttentionPolicy::Track,
                ),
                observation(
                    "stale",
                    PortfolioQualificationState::Stale,
                    AttentionPolicy::Track,
                ),
                observation(
                    "untested",
                    PortfolioQualificationState::Untested,
                    AttentionPolicy::Track,
                ),
            ],
            vec![PortfolioSurfaceObservation {
                surface_id: "src/unowned.rs".to_owned(),
                source_revision: SemanticHasher::new("surface").finish(),
                owners: Vec::new(),
                evidence_ids: Vec::new(),
            }],
            PortfolioLimits::default(),
        )
        .unwrap();
        let attention = WorkloadAttention::derive(&snapshot).unwrap();

        assert_eq!(attention.summary.refine, 1);
        assert_eq!(attention.summary.retest, 3);
        assert_eq!(attention.summary.create, 1);
        assert_eq!(attention.summary.blocked, 2);
        assert_eq!(attention.summary.policy_excluded, 1);
        assert_eq!(attention.summary.unowned_surfaces, 1);
        assert!(attention.rows.iter().any(|row| {
            row.action == AttentionAction::Block && row.readiness == AttentionReadiness::Blocked
        }));
        assert_eq!(attention.to_frame().unwrap().dataframe().height(), 8);
        attention.verify().unwrap();
    }

    #[test]
    fn qualified_covered_portfolio_has_no_attention_rows() {
        let snapshot = PortfolioSnapshot::new(
            SemanticHasher::new("catalog").finish(),
            None,
            vec![observation(
                "clean",
                PortfolioQualificationState::Qualified,
                AttentionPolicy::Track,
            )],
            vec![PortfolioSurfaceObservation {
                surface_id: "src/owned.rs".to_owned(),
                source_revision: SemanticHasher::new("surface").finish(),
                owners: vec!["clean".to_owned()],
                evidence_ids: Vec::new(),
            }],
            PortfolioLimits::default(),
        )
        .unwrap();
        let attention = WorkloadAttention::derive(&snapshot).unwrap();
        assert!(attention.rows.is_empty());
        assert_eq!(attention.summary.owned_surfaces, 1);
        let frame = attention.to_frame().unwrap();
        assert_eq!(frame.dataframe().height(), 0);
        assert_eq!(frame.dataframe().width(), 8);
        attention.verify().unwrap();
    }

    #[test]
    fn tampering_and_wrong_source_bindings_fail_closed() {
        let snapshot = PortfolioSnapshot::new(
            SemanticHasher::new("catalog-a").finish(),
            None,
            vec![observation(
                "failing",
                PortfolioQualificationState::Failing,
                AttentionPolicy::Track,
            )],
            Vec::new(),
            PortfolioLimits::default(),
        )
        .unwrap();
        let attention = WorkloadAttention::derive(&snapshot).unwrap();
        attention.verify_against(&snapshot).unwrap();

        let mut summary_tamper = attention.clone();
        summary_tamper.summary.refine = 0;
        assert!(summary_tamper.verify().is_err());

        let mut row_tamper = attention.clone();
        row_tamper.rows[0].priority = 99;
        assert!(row_tamper.verify().is_err());

        let other = PortfolioSnapshot::new(
            SemanticHasher::new("catalog-b").finish(),
            None,
            Vec::new(),
            Vec::new(),
            PortfolioLimits::default(),
        )
        .unwrap();
        assert!(attention.verify_against(&other).is_err());
    }
}
