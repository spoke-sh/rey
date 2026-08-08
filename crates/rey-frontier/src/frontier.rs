use std::collections::{BTreeMap, BTreeSet};

use polars::df;
use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_dataframe::{Frame, FrameMetadata};
use serde::{Deserialize, Serialize};

use crate::{FrontierError, add_string_bytes, validate_contract, validate_digest, validate_text};

pub const FRONTIER_SCHEMA: &str = "rey.frontier.v2";
pub const FRONTIER_RELATION: &str = "rey.frontier-rows";
pub const FRONTIER_SCHEMA_VERSION: &str = "2";
const FRONTIER_ROW_SCHEMA: &str = "rey.frontier-row.v2";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FrontierInputs {
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario_suite: ContractIdentity,
    pub campaign_id: SemanticDigest,
    pub space: ContractIdentity,
    pub trace_id: SemanticDigest,
    pub committed_record_id: SemanticDigest,
    pub capability_snapshot_id: SemanticDigest,
    pub derivation: ContractIdentity,
    pub prioritization: ContractIdentity,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FrontierLimits {
    pub max_rows: u64,
    pub max_delta_refs: u64,
    pub max_claim_refs: u64,
    pub max_lens_refs: u64,
    pub max_action_refs: u64,
    pub max_blockers: u64,
    pub max_string_bytes: u64,
}

impl Default for FrontierLimits {
    fn default() -> Self {
        Self {
            max_rows: 1_024,
            max_delta_refs: 4_096,
            max_claim_refs: 4_096,
            max_lens_refs: 4_096,
            max_action_refs: 1_024,
            max_blockers: 4_096,
            max_string_bytes: 2 * 1_024 * 1_024,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredClaims {
    Satisfied,
    Violated,
    Unknown,
}

impl RequiredClaims {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Satisfied => "satisfied",
            Self::Violated => "violated",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FrontierCoverage {
    pub deltas_complete: bool,
    pub claims_complete: bool,
    pub required_claims: RequiredClaims,
}

impl FrontierCoverage {
    #[must_use]
    pub const fn is_complete(self) -> bool {
        self.deltas_complete
            && self.claims_complete
            && !matches!(self.required_claims, RequiredClaims::Unknown)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontierAssessment {
    Open,
    Converged,
    Inconclusive,
}

impl FrontierAssessment {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Converged => "converged",
            Self::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Readiness {
    Ready,
    Blocked,
    Inconclusive,
}

impl Readiness {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Blocked => "blocked",
            Self::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontierBlockerKind {
    Dependency,
    Capability,
    Evidence,
    Budget,
    Unsupported,
    Incomplete,
    Other,
}

impl FrontierBlockerKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Dependency => "dependency",
            Self::Capability => "capability",
            Self::Evidence => "evidence",
            Self::Budget => "budget",
            Self::Unsupported => "unsupported",
            Self::Incomplete => "incomplete",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct FrontierBlocker {
    pub kind: FrontierBlockerKind,
    pub blocker_id: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FrontierRowInput {
    pub work_id: String,
    pub entity_kind: String,
    pub entity_id: String,
    pub transition_delta_ids: Vec<SemanticDigest>,
    pub residual_delta_ids: Vec<SemanticDigest>,
    pub claim_ids: Vec<String>,
    pub dependent_lens_ids: Vec<String>,
    pub admissible_action_ids: Vec<String>,
    pub readiness: Readiness,
    pub blockers: Vec<FrontierBlocker>,
    pub priority: u64,
    pub estimated_cost_units: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FrontierRow {
    pub row_id: SemanticDigest,
    pub work_id: String,
    pub entity_kind: String,
    pub entity_id: String,
    pub transition_delta_ids: Vec<SemanticDigest>,
    pub residual_delta_ids: Vec<SemanticDigest>,
    pub claim_ids: Vec<String>,
    pub dependent_lens_ids: Vec<String>,
    pub admissible_action_ids: Vec<String>,
    pub readiness: Readiness,
    pub blockers: Vec<FrontierBlocker>,
    pub priority: u64,
    pub estimated_cost_units: u64,
}

impl FrontierRow {
    fn from_input(input: FrontierRowInput) -> Result<Self, FrontierError> {
        let mut row = Self {
            row_id: placeholder_digest("rey.frontier-row.placeholder"),
            work_id: input.work_id,
            entity_kind: input.entity_kind,
            entity_id: input.entity_id,
            transition_delta_ids: input.transition_delta_ids,
            residual_delta_ids: input.residual_delta_ids,
            claim_ids: input.claim_ids,
            dependent_lens_ids: input.dependent_lens_ids,
            admissible_action_ids: input.admissible_action_ids,
            readiness: input.readiness,
            blockers: input.blockers,
            priority: input.priority,
            estimated_cost_units: input.estimated_cost_units,
        };
        row.normalize();
        row.validate()?;
        row.row_id = row_digest(&row);
        Ok(row)
    }

    fn as_input(&self) -> FrontierRowInput {
        FrontierRowInput {
            work_id: self.work_id.clone(),
            entity_kind: self.entity_kind.clone(),
            entity_id: self.entity_id.clone(),
            transition_delta_ids: self.transition_delta_ids.clone(),
            residual_delta_ids: self.residual_delta_ids.clone(),
            claim_ids: self.claim_ids.clone(),
            dependent_lens_ids: self.dependent_lens_ids.clone(),
            admissible_action_ids: self.admissible_action_ids.clone(),
            readiness: self.readiness,
            blockers: self.blockers.clone(),
            priority: self.priority,
            estimated_cost_units: self.estimated_cost_units,
        }
    }

    fn normalize(&mut self) {
        self.transition_delta_ids.sort();
        self.transition_delta_ids.dedup();
        self.residual_delta_ids.sort();
        self.residual_delta_ids.dedup();
        self.claim_ids.sort();
        self.claim_ids.dedup();
        self.dependent_lens_ids.sort();
        self.dependent_lens_ids.dedup();
        self.admissible_action_ids.sort();
        self.admissible_action_ids.dedup();
        self.blockers.sort();
        self.blockers.dedup();
    }

    fn validate(&self) -> Result<(), FrontierError> {
        validate_text("work id", &self.work_id)?;
        validate_text("entity kind", &self.entity_kind)?;
        validate_text("entity id", &self.entity_id)?;
        for digest in self
            .transition_delta_ids
            .iter()
            .chain(&self.residual_delta_ids)
        {
            validate_digest(digest)?;
        }
        for value in self
            .claim_ids
            .iter()
            .chain(&self.dependent_lens_ids)
            .chain(&self.admissible_action_ids)
        {
            validate_text("frontier reference", value)?;
        }
        for blocker in &self.blockers {
            validate_text("blocker id", &blocker.blocker_id)?;
            validate_text("blocker reason", &blocker.reason)?;
        }
        if self.transition_delta_ids.is_empty()
            && self.residual_delta_ids.is_empty()
            && self.claim_ids.is_empty()
        {
            return Err(FrontierError::UndirectedWork(self.work_id.clone()));
        }
        let transition = self.transition_delta_ids.iter().collect::<BTreeSet<_>>();
        if self
            .residual_delta_ids
            .iter()
            .any(|digest| transition.contains(digest))
        {
            return Err(FrontierError::AmbiguousDeltaRole(self.work_id.clone()));
        }
        if self.estimated_cost_units == 0 {
            return Err(FrontierError::ZeroEstimatedCost(self.work_id.clone()));
        }
        if (self.readiness == Readiness::Ready) != self.blockers.is_empty() {
            return Err(FrontierError::ReadinessBlockerMismatch(
                self.work_id.clone(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Frontier {
    pub schema: String,
    pub frontier_id: SemanticDigest,
    pub inputs: FrontierInputs,
    pub limits: FrontierLimits,
    pub coverage: FrontierCoverage,
    pub assessment: FrontierAssessment,
    pub rows: Vec<FrontierRow>,
}

impl Frontier {
    pub fn new(
        inputs: FrontierInputs,
        limits: FrontierLimits,
        coverage: FrontierCoverage,
        rows: Vec<FrontierRowInput>,
    ) -> Result<Self, FrontierError> {
        validate_inputs(&inputs)?;
        validate_limits(&limits)?;
        let mut rows = rows
            .into_iter()
            .map(FrontierRow::from_input)
            .collect::<Result<Vec<_>, _>>()?;
        rows.sort_by(|left, right| left.work_id.cmp(&right.work_id));
        validate_rows(&rows, &limits)?;
        if coverage.required_claims == RequiredClaims::Violated && rows.is_empty() {
            return Err(FrontierError::MissingViolatedClaimWork);
        }
        let assessment = derive_assessment(coverage, &rows);
        validate_string_bytes(&inputs, &rows, &limits)?;

        let mut frontier = Self {
            schema: FRONTIER_SCHEMA.to_owned(),
            frontier_id: placeholder_digest("rey.frontier.placeholder"),
            inputs,
            limits,
            coverage,
            assessment,
            rows,
        };
        frontier.frontier_id = frontier_digest(&frontier);
        Ok(frontier)
    }

    pub fn verify(&self) -> Result<(), FrontierError> {
        if self.schema != FRONTIER_SCHEMA {
            return Err(FrontierError::UnsupportedSchema {
                kind: "frontier",
                expected: FRONTIER_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        let canonical = Self::new(
            self.inputs.clone(),
            self.limits.clone(),
            self.coverage,
            self.rows.iter().map(FrontierRow::as_input).collect(),
        )?;
        if self.frontier_id != canonical.frontier_id {
            return Err(FrontierError::DigestMismatch {
                kind: "frontier",
                declared: self.frontier_id.clone(),
                actual: canonical.frontier_id,
            });
        }
        if self.assessment != canonical.assessment {
            return Err(FrontierError::AssessmentMismatch);
        }
        if self != &canonical {
            return Err(FrontierError::NonCanonical("frontier"));
        }
        Ok(())
    }

    pub fn to_frame(&self) -> Result<Frame, FrontierError> {
        self.verify()?;
        let rows = &self.rows;
        let transition_delta_ids = digest_arrays(rows, |row| &row.transition_delta_ids)?;
        let residual_delta_ids = digest_arrays(rows, |row| &row.residual_delta_ids)?;
        let claim_ids = string_arrays(rows, |row| &row.claim_ids)?;
        let dependent_lens_ids = string_arrays(rows, |row| &row.dependent_lens_ids)?;
        let admissible_action_ids = string_arrays(rows, |row| &row.admissible_action_ids)?;
        let blockers = rows
            .iter()
            .map(|row| serde_json::to_string(&row.blockers))
            .collect::<Result<Vec<_>, _>>()?;
        let dataframe = df!(
            "work_id" => rows.iter().map(|row| row.work_id.as_str()).collect::<Vec<_>>(),
            "row_id" => rows.iter().map(|row| row.row_id.as_str()).collect::<Vec<_>>(),
            "entity_kind" => rows.iter().map(|row| row.entity_kind.as_str()).collect::<Vec<_>>(),
            "entity_id" => rows.iter().map(|row| row.entity_id.as_str()).collect::<Vec<_>>(),
            "transition_delta_ids" => transition_delta_ids,
            "residual_delta_ids" => residual_delta_ids,
            "claim_ids" => claim_ids,
            "dependent_lens_ids" => dependent_lens_ids,
            "admissible_action_ids" => admissible_action_ids,
            "readiness" => rows.iter().map(|row| row.readiness.as_str()).collect::<Vec<_>>(),
            "blockers" => blockers,
            "priority" => rows.iter().map(|row| row.priority).collect::<Vec<_>>(),
            "estimated_cost_units" => rows.iter().map(|row| row.estimated_cost_units).collect::<Vec<_>>(),
        )?;
        let attributes = BTreeMap::from([
            ("rey.frontier-schema".to_owned(), self.schema.clone()),
            ("rey.frontier-id".to_owned(), self.frontier_id.to_string()),
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
                "rey.committed-record-id".to_owned(),
                self.inputs.committed_record_id.to_string(),
            ),
            (
                "rey.capability-snapshot-id".to_owned(),
                self.inputs.capability_snapshot_id.to_string(),
            ),
            (
                "rey.derivation-id".to_owned(),
                self.inputs.derivation.id.clone(),
            ),
            (
                "rey.derivation-revision".to_owned(),
                self.inputs.derivation.revision.to_string(),
            ),
            (
                "rey.derivation-digest".to_owned(),
                self.inputs.derivation.semantic_digest.to_string(),
            ),
            (
                "rey.prioritization-id".to_owned(),
                self.inputs.prioritization.id.clone(),
            ),
            (
                "rey.prioritization-revision".to_owned(),
                self.inputs.prioritization.revision.to_string(),
            ),
            (
                "rey.prioritization-digest".to_owned(),
                self.inputs.prioritization.semantic_digest.to_string(),
            ),
            (
                "rey.assessment".to_owned(),
                self.assessment.as_str().to_owned(),
            ),
            (
                "rey.required-claims".to_owned(),
                self.coverage.required_claims.as_str().to_owned(),
            ),
            (
                "rey.deltas-complete".to_owned(),
                self.coverage.deltas_complete.to_string(),
            ),
            (
                "rey.claims-complete".to_owned(),
                self.coverage.claims_complete.to_string(),
            ),
            ("rey.max-rows".to_owned(), self.limits.max_rows.to_string()),
            (
                "rey.max-delta-refs".to_owned(),
                self.limits.max_delta_refs.to_string(),
            ),
            (
                "rey.max-claim-refs".to_owned(),
                self.limits.max_claim_refs.to_string(),
            ),
            (
                "rey.max-lens-refs".to_owned(),
                self.limits.max_lens_refs.to_string(),
            ),
            (
                "rey.max-action-refs".to_owned(),
                self.limits.max_action_refs.to_string(),
            ),
            (
                "rey.max-blockers".to_owned(),
                self.limits.max_blockers.to_string(),
            ),
            (
                "rey.max-string-bytes".to_owned(),
                self.limits.max_string_bytes.to_string(),
            ),
        ]);
        Ok(Frame::new(
            dataframe,
            FrameMetadata {
                relation: FRONTIER_RELATION.to_owned(),
                schema_version: FRONTIER_SCHEMA_VERSION.to_owned(),
                semantic_digest: self.frontier_id.to_string(),
                row_count: rows.len() as u64,
                complete: self.coverage.is_complete(),
                key_columns: vec!["work_id".to_owned()],
                attributes,
            },
        )?)
    }
}

fn validate_inputs(inputs: &FrontierInputs) -> Result<(), FrontierError> {
    validate_contract("workload", &inputs.workload)?;
    validate_contract("graph", &inputs.graph)?;
    validate_contract("scenario suite", &inputs.scenario_suite)?;
    validate_contract("space", &inputs.space)?;
    validate_contract("derivation", &inputs.derivation)?;
    validate_contract("prioritization", &inputs.prioritization)?;
    validate_digest(&inputs.trace_id)?;
    validate_digest(&inputs.campaign_id)?;
    validate_digest(&inputs.committed_record_id)?;
    validate_digest(&inputs.capability_snapshot_id)
}

fn validate_limits(limits: &FrontierLimits) -> Result<(), FrontierError> {
    let limits = [
        ("max_rows", limits.max_rows),
        ("max_delta_refs", limits.max_delta_refs),
        ("max_claim_refs", limits.max_claim_refs),
        ("max_lens_refs", limits.max_lens_refs),
        ("max_action_refs", limits.max_action_refs),
        ("max_blockers", limits.max_blockers),
        ("max_string_bytes", limits.max_string_bytes),
    ];
    if let Some((name, _)) = limits.into_iter().find(|(_, value)| *value == 0) {
        return Err(FrontierError::ZeroLimit(name));
    }
    Ok(())
}

fn validate_rows(rows: &[FrontierRow], limits: &FrontierLimits) -> Result<(), FrontierError> {
    enforce_limit("row", rows.len() as u64, limits.max_rows)?;
    let mut delta_refs = 0_u64;
    let mut claim_refs = 0_u64;
    let mut lens_refs = 0_u64;
    let mut action_refs = 0_u64;
    let mut blockers = 0_u64;
    for row in rows {
        validate_digest(&row.row_id)?;
        row.validate()?;
        delta_refs = checked_add(delta_refs, row.transition_delta_ids.len())?;
        delta_refs = checked_add(delta_refs, row.residual_delta_ids.len())?;
        claim_refs = checked_add(claim_refs, row.claim_ids.len())?;
        lens_refs = checked_add(lens_refs, row.dependent_lens_ids.len())?;
        action_refs = checked_add(action_refs, row.admissible_action_ids.len())?;
        blockers = checked_add(blockers, row.blockers.len())?;
    }
    enforce_limit("delta reference", delta_refs, limits.max_delta_refs)?;
    enforce_limit("claim reference", claim_refs, limits.max_claim_refs)?;
    enforce_limit("lens reference", lens_refs, limits.max_lens_refs)?;
    enforce_limit("action reference", action_refs, limits.max_action_refs)?;
    enforce_limit("blocker", blockers, limits.max_blockers)?;
    if let Some(window) = rows
        .windows(2)
        .find(|window| window[0].work_id >= window[1].work_id)
    {
        if window[0].work_id == window[1].work_id {
            return Err(FrontierError::DuplicateWork(window[0].work_id.clone()));
        }
        return Err(FrontierError::NonCanonical("frontier rows"));
    }
    Ok(())
}

fn checked_add(total: u64, count: usize) -> Result<u64, FrontierError> {
    total
        .checked_add(count as u64)
        .ok_or(FrontierError::CountOverflow)
}

fn enforce_limit(kind: &'static str, observed: u64, limit: u64) -> Result<(), FrontierError> {
    if observed > limit {
        return Err(FrontierError::Limit {
            kind,
            limit,
            observed,
        });
    }
    Ok(())
}

fn derive_assessment(coverage: FrontierCoverage, rows: &[FrontierRow]) -> FrontierAssessment {
    if !rows.is_empty() {
        FrontierAssessment::Open
    } else if coverage.deltas_complete
        && coverage.claims_complete
        && coverage.required_claims == RequiredClaims::Satisfied
    {
        FrontierAssessment::Converged
    } else {
        FrontierAssessment::Inconclusive
    }
}

fn validate_string_bytes(
    inputs: &FrontierInputs,
    rows: &[FrontierRow],
    limits: &FrontierLimits,
) -> Result<(), FrontierError> {
    let mut total = 0_u64;
    for contract in [
        &inputs.workload,
        &inputs.graph,
        &inputs.scenario_suite,
        &inputs.space,
        &inputs.derivation,
        &inputs.prioritization,
    ] {
        add_string_bytes(&mut total, &contract.id)?;
        add_string_bytes(&mut total, contract.semantic_digest.as_str())?;
    }
    for digest in [
        &inputs.campaign_id,
        &inputs.trace_id,
        &inputs.committed_record_id,
        &inputs.capability_snapshot_id,
    ] {
        add_string_bytes(&mut total, digest.as_str())?;
    }
    for row in rows {
        for value in [&row.work_id, &row.entity_kind, &row.entity_id] {
            add_string_bytes(&mut total, value)?;
        }
        add_string_bytes(&mut total, row.row_id.as_str())?;
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
            .chain(&row.dependent_lens_ids)
            .chain(&row.admissible_action_ids)
        {
            add_string_bytes(&mut total, value)?;
        }
        for blocker in &row.blockers {
            add_string_bytes(&mut total, &blocker.blocker_id)?;
            add_string_bytes(&mut total, &blocker.reason)?;
        }
    }
    enforce_limit("string byte", total, limits.max_string_bytes)
}

fn frontier_digest(frontier: &Frontier) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(FRONTIER_SCHEMA);
    add_inputs(&mut hasher, &frontier.inputs);
    add_limits(&mut hasher, &frontier.limits);
    hasher.add_bool(frontier.coverage.deltas_complete);
    hasher.add_bool(frontier.coverage.claims_complete);
    hasher.add_str(frontier.coverage.required_claims.as_str());
    hasher.add_str(frontier.assessment.as_str());
    hasher.add_u64(frontier.rows.len() as u64);
    for row in &frontier.rows {
        hasher.add_str(row.row_id.as_str());
    }
    hasher.finish()
}

fn row_digest(row: &FrontierRow) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(FRONTIER_ROW_SCHEMA);
    hasher.add_str(&row.work_id);
    hasher.add_str(&row.entity_kind);
    hasher.add_str(&row.entity_id);
    add_digests(&mut hasher, &row.transition_delta_ids);
    add_digests(&mut hasher, &row.residual_delta_ids);
    add_strings(&mut hasher, &row.claim_ids);
    add_strings(&mut hasher, &row.dependent_lens_ids);
    add_strings(&mut hasher, &row.admissible_action_ids);
    hasher.add_str(row.readiness.as_str());
    hasher.add_u64(row.blockers.len() as u64);
    for blocker in &row.blockers {
        hasher.add_str(blocker.kind.as_str());
        hasher.add_str(&blocker.blocker_id);
        hasher.add_str(&blocker.reason);
    }
    hasher.add_u64(row.priority);
    hasher.add_u64(row.estimated_cost_units);
    hasher.finish()
}

pub(crate) fn add_inputs(hasher: &mut SemanticHasher, inputs: &FrontierInputs) {
    inputs.workload.add_semantics(hasher);
    inputs.graph.add_semantics(hasher);
    inputs.scenario_suite.add_semantics(hasher);
    hasher.add_str(inputs.campaign_id.as_str());
    inputs.space.add_semantics(hasher);
    hasher.add_str(inputs.trace_id.as_str());
    hasher.add_str(inputs.committed_record_id.as_str());
    hasher.add_str(inputs.capability_snapshot_id.as_str());
    inputs.derivation.add_semantics(hasher);
    inputs.prioritization.add_semantics(hasher);
}

fn add_limits(hasher: &mut SemanticHasher, limits: &FrontierLimits) {
    hasher.add_u64(limits.max_rows);
    hasher.add_u64(limits.max_delta_refs);
    hasher.add_u64(limits.max_claim_refs);
    hasher.add_u64(limits.max_lens_refs);
    hasher.add_u64(limits.max_action_refs);
    hasher.add_u64(limits.max_blockers);
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

fn digest_arrays(
    rows: &[FrontierRow],
    select: impl Fn(&FrontierRow) -> &[SemanticDigest],
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
    rows: &[FrontierRow],
    select: impl Fn(&FrontierRow) -> &[String],
) -> Result<Vec<String>, serde_json::Error> {
    rows.iter()
        .map(|row| serde_json::to_string(select(row)))
        .collect()
}
