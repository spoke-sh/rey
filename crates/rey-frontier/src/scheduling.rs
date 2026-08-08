use std::collections::{BTreeMap, BTreeSet};

use polars::df;
use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_dataframe::{Frame, FrameMetadata};
use serde::{Deserialize, Serialize};

use crate::{
    Frontier, FrontierAssessment, FrontierError, Readiness, add_string_bytes, validate_contract,
    validate_digest,
};

pub const SCHEDULING_DECISION_SCHEMA: &str = "rey.scheduling-decision.v1";
pub const SCHEDULING_DECISION_RELATION: &str = "rey.scheduled-work";
pub const SCHEDULING_DECISION_SCHEMA_VERSION: &str = "1";
const SCHEDULER_ID: &str = "rey.priority-cost-work-id";
const SCHEDULER_REVISION: u64 = 1;
const SCHEDULER_DEFINITION: &str = "select ready rey.frontier.v1 work by priority descending, estimated cost ascending, work_id ascending; greedy bounded units and total cost; no fairness claim";

#[must_use]
pub fn deterministic_scheduler() -> ContractIdentity {
    ContractIdentity::new(SCHEDULER_ID, SCHEDULER_REVISION, SCHEDULER_DEFINITION)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchedulingPreconditions {
    pub expected_committed_record_id: SemanticDigest,
    pub expected_frontier_id: SemanticDigest,
    pub expected_capability_snapshot_id: SemanticDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchedulerLimits {
    pub max_rows_considered: u64,
    pub max_work_units: u64,
    pub max_total_cost_units: u64,
    pub max_string_bytes: u64,
}

impl Default for SchedulerLimits {
    fn default() -> Self {
        Self {
            max_rows_considered: 1_024,
            max_work_units: 16,
            max_total_cost_units: 1_024,
            max_string_bytes: 256 * 1_024,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchedulingInputs {
    pub application: ContractIdentity,
    pub component: ContractIdentity,
    pub space: ContractIdentity,
    pub trace_id: SemanticDigest,
    pub committed_record_id: SemanticDigest,
    pub frontier_id: SemanticDigest,
    pub capability_snapshot_id: SemanticDigest,
    pub frontier_assessment: FrontierAssessment,
    pub scheduler: ContractIdentity,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleOutcome {
    Selected,
    NoReadyWork,
    BudgetExhausted,
    FrontierConverged,
    FrontierInconclusive,
}

impl ScheduleOutcome {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::NoReadyWork => "no_ready_work",
            Self::BudgetExhausted => "budget_exhausted",
            Self::FrontierConverged => "frontier_converged",
            Self::FrontierInconclusive => "frontier_inconclusive",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScheduledWork {
    pub selection_rank: u64,
    pub work_id: String,
    pub frontier_row_id: SemanticDigest,
    pub priority: u64,
    pub estimated_cost_units: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchedulingDecision {
    pub schema: String,
    pub decision_id: SemanticDigest,
    pub inputs: SchedulingInputs,
    pub limits: SchedulerLimits,
    pub outcome: ScheduleOutcome,
    pub considered_rows: u64,
    pub ready_rows: u64,
    pub blocked_rows: u64,
    pub inconclusive_rows: u64,
    pub selected_cost_units: u64,
    pub deferred_ready_rows: u64,
    pub skipped_over_cost_rows: u64,
    pub selected: Vec<ScheduledWork>,
}

pub fn schedule(
    frontier: &Frontier,
    preconditions: SchedulingPreconditions,
    limits: SchedulerLimits,
) -> Result<SchedulingDecision, FrontierError> {
    frontier.verify()?;
    let scheduler = deterministic_scheduler();
    validate_contract("scheduler", &scheduler)?;
    validate_limits(&limits)?;
    for digest in [
        &preconditions.expected_committed_record_id,
        &preconditions.expected_frontier_id,
        &preconditions.expected_capability_snapshot_id,
    ] {
        validate_digest(digest)?;
    }
    if preconditions.expected_committed_record_id != frontier.inputs.committed_record_id {
        return Err(FrontierError::StaleCommittedRecord {
            expected: preconditions.expected_committed_record_id,
            actual: frontier.inputs.committed_record_id.clone(),
        });
    }
    if preconditions.expected_frontier_id != frontier.frontier_id {
        return Err(FrontierError::StaleFrontier {
            expected: preconditions.expected_frontier_id,
            actual: frontier.frontier_id.clone(),
        });
    }
    if preconditions.expected_capability_snapshot_id != frontier.inputs.capability_snapshot_id {
        return Err(FrontierError::StaleCapabilitySnapshot {
            expected: preconditions.expected_capability_snapshot_id,
            actual: frontier.inputs.capability_snapshot_id.clone(),
        });
    }
    if frontier.rows.len() as u64 > limits.max_rows_considered {
        return Err(FrontierError::SchedulerRowLimit {
            limit: limits.max_rows_considered,
            observed: frontier.rows.len() as u64,
        });
    }

    let considered_rows = frontier.rows.len() as u64;
    let ready_rows = count_readiness(frontier, Readiness::Ready);
    let blocked_rows = count_readiness(frontier, Readiness::Blocked);
    let inconclusive_rows = count_readiness(frontier, Readiness::Inconclusive);
    let mut candidates = frontier
        .rows
        .iter()
        .filter(|row| row.readiness == Readiness::Ready)
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.estimated_cost_units.cmp(&right.estimated_cost_units))
            .then_with(|| left.work_id.cmp(&right.work_id))
    });

    let mut selected = Vec::new();
    let mut selected_cost_units = 0_u64;
    let mut skipped_over_cost_rows = 0_u64;
    for row in candidates {
        if selected.len() as u64 >= limits.max_work_units {
            continue;
        }
        let Some(next_cost) = selected_cost_units.checked_add(row.estimated_cost_units) else {
            return Err(FrontierError::CountOverflow);
        };
        if next_cost > limits.max_total_cost_units {
            skipped_over_cost_rows = skipped_over_cost_rows
                .checked_add(1)
                .ok_or(FrontierError::CountOverflow)?;
            continue;
        }
        selected_cost_units = next_cost;
        selected.push(ScheduledWork {
            selection_rank: selected.len() as u64 + 1,
            work_id: row.work_id.clone(),
            frontier_row_id: row.row_id.clone(),
            priority: row.priority,
            estimated_cost_units: row.estimated_cost_units,
        });
    }
    let deferred_ready_rows = ready_rows
        .checked_sub(selected.len() as u64)
        .ok_or(FrontierError::CountOverflow)?;
    let outcome = match frontier.assessment {
        FrontierAssessment::Converged => ScheduleOutcome::FrontierConverged,
        FrontierAssessment::Inconclusive => ScheduleOutcome::FrontierInconclusive,
        FrontierAssessment::Open if !selected.is_empty() => ScheduleOutcome::Selected,
        FrontierAssessment::Open if ready_rows == 0 => ScheduleOutcome::NoReadyWork,
        FrontierAssessment::Open => ScheduleOutcome::BudgetExhausted,
    };
    let inputs = SchedulingInputs {
        application: frontier.inputs.application.clone(),
        component: frontier.inputs.component.clone(),
        space: frontier.inputs.space.clone(),
        trace_id: frontier.inputs.trace_id.clone(),
        committed_record_id: frontier.inputs.committed_record_id.clone(),
        frontier_id: frontier.frontier_id.clone(),
        capability_snapshot_id: frontier.inputs.capability_snapshot_id.clone(),
        frontier_assessment: frontier.assessment,
        scheduler,
    };
    validate_string_bytes(&inputs, &selected, &limits)?;
    let mut decision = SchedulingDecision {
        schema: SCHEDULING_DECISION_SCHEMA.to_owned(),
        decision_id: placeholder_digest(),
        inputs,
        limits,
        outcome,
        considered_rows,
        ready_rows,
        blocked_rows,
        inconclusive_rows,
        selected_cost_units,
        deferred_ready_rows,
        skipped_over_cost_rows,
        selected,
    };
    decision.decision_id = decision_digest(&decision);
    decision.verify()?;
    Ok(decision)
}

impl SchedulingDecision {
    pub fn verify(&self) -> Result<(), FrontierError> {
        if self.schema != SCHEDULING_DECISION_SCHEMA {
            return Err(FrontierError::UnsupportedSchema {
                kind: "scheduling decision",
                expected: SCHEDULING_DECISION_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_contract("application", &self.inputs.application)?;
        validate_contract("component", &self.inputs.component)?;
        validate_contract("space", &self.inputs.space)?;
        validate_contract("scheduler", &self.inputs.scheduler)?;
        if self.inputs.scheduler != deterministic_scheduler() {
            return Err(FrontierError::UnexpectedContract("scheduler"));
        }
        for digest in [
            &self.inputs.trace_id,
            &self.inputs.committed_record_id,
            &self.inputs.frontier_id,
            &self.inputs.capability_snapshot_id,
        ] {
            validate_digest(digest)?;
        }
        validate_limits(&self.limits)?;
        if self.considered_rows > self.limits.max_rows_considered
            || self.selected.len() as u64 > self.limits.max_work_units
            || self.selected_cost_units > self.limits.max_total_cost_units
        {
            return Err(FrontierError::SchedulingShape);
        }
        let classified = self
            .ready_rows
            .checked_add(self.blocked_rows)
            .and_then(|value| value.checked_add(self.inconclusive_rows))
            .ok_or(FrontierError::CountOverflow)?;
        let deferred = self
            .ready_rows
            .checked_sub(self.selected.len() as u64)
            .ok_or(FrontierError::SchedulingShape)?;
        let selected_cost = self.selected.iter().try_fold(0_u64, |total, work| {
            total
                .checked_add(work.estimated_cost_units)
                .ok_or(FrontierError::CountOverflow)
        })?;
        if classified != self.considered_rows
            || deferred != self.deferred_ready_rows
            || selected_cost != self.selected_cost_units
            || self.skipped_over_cost_rows > self.deferred_ready_rows
        {
            return Err(FrontierError::SchedulingShape);
        }
        validate_selection(&self.selected)?;
        validate_outcome(self)?;
        validate_string_bytes(&self.inputs, &self.selected, &self.limits)?;
        let actual = decision_digest(self);
        if actual != self.decision_id {
            return Err(FrontierError::DigestMismatch {
                kind: "scheduling decision",
                declared: self.decision_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    pub fn verify_against(&self, frontier: &Frontier) -> Result<(), FrontierError> {
        self.verify()?;
        let expected = schedule(
            frontier,
            SchedulingPreconditions {
                expected_committed_record_id: self.inputs.committed_record_id.clone(),
                expected_frontier_id: self.inputs.frontier_id.clone(),
                expected_capability_snapshot_id: self.inputs.capability_snapshot_id.clone(),
            },
            self.limits.clone(),
        )?;
        if self != &expected {
            return Err(FrontierError::SchedulingReplayMismatch);
        }
        Ok(())
    }

    pub fn to_frame(&self) -> Result<Frame, FrontierError> {
        self.verify()?;
        let dataframe = df!(
            "selection_rank" => self.selected.iter().map(|work| work.selection_rank).collect::<Vec<_>>(),
            "work_id" => self.selected.iter().map(|work| work.work_id.as_str()).collect::<Vec<_>>(),
            "frontier_row_id" => self.selected.iter().map(|work| work.frontier_row_id.as_str()).collect::<Vec<_>>(),
            "priority" => self.selected.iter().map(|work| work.priority).collect::<Vec<_>>(),
            "estimated_cost_units" => self.selected.iter().map(|work| work.estimated_cost_units).collect::<Vec<_>>(),
        )?;
        let attributes = BTreeMap::from([
            ("rey.decision-schema".to_owned(), self.schema.clone()),
            ("rey.decision-id".to_owned(), self.decision_id.to_string()),
            (
                "rey.application-id".to_owned(),
                self.inputs.application.id.clone(),
            ),
            (
                "rey.application-revision".to_owned(),
                self.inputs.application.revision.to_string(),
            ),
            (
                "rey.application-digest".to_owned(),
                self.inputs.application.semantic_digest.to_string(),
            ),
            (
                "rey.component-id".to_owned(),
                self.inputs.component.id.clone(),
            ),
            (
                "rey.component-revision".to_owned(),
                self.inputs.component.revision.to_string(),
            ),
            (
                "rey.component-digest".to_owned(),
                self.inputs.component.semantic_digest.to_string(),
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
                "rey.frontier-id".to_owned(),
                self.inputs.frontier_id.to_string(),
            ),
            (
                "rey.capability-snapshot-id".to_owned(),
                self.inputs.capability_snapshot_id.to_string(),
            ),
            (
                "rey.scheduler-id".to_owned(),
                self.inputs.scheduler.id.clone(),
            ),
            (
                "rey.scheduler-revision".to_owned(),
                self.inputs.scheduler.revision.to_string(),
            ),
            (
                "rey.scheduler-digest".to_owned(),
                self.inputs.scheduler.semantic_digest.to_string(),
            ),
            (
                "rey.frontier-assessment".to_owned(),
                self.inputs.frontier_assessment.as_str().to_owned(),
            ),
            ("rey.outcome".to_owned(), self.outcome.as_str().to_owned()),
            (
                "rey.considered-rows".to_owned(),
                self.considered_rows.to_string(),
            ),
            (
                "rey.deferred-ready-rows".to_owned(),
                self.deferred_ready_rows.to_string(),
            ),
            (
                "rey.selected-cost-units".to_owned(),
                self.selected_cost_units.to_string(),
            ),
            ("rey.ready-rows".to_owned(), self.ready_rows.to_string()),
            ("rey.blocked-rows".to_owned(), self.blocked_rows.to_string()),
            (
                "rey.inconclusive-rows".to_owned(),
                self.inconclusive_rows.to_string(),
            ),
            (
                "rey.skipped-over-cost-rows".to_owned(),
                self.skipped_over_cost_rows.to_string(),
            ),
            (
                "rey.max-rows-considered".to_owned(),
                self.limits.max_rows_considered.to_string(),
            ),
            (
                "rey.max-work-units".to_owned(),
                self.limits.max_work_units.to_string(),
            ),
            (
                "rey.max-total-cost-units".to_owned(),
                self.limits.max_total_cost_units.to_string(),
            ),
            (
                "rey.max-string-bytes".to_owned(),
                self.limits.max_string_bytes.to_string(),
            ),
        ]);
        Ok(Frame::new(
            dataframe,
            FrameMetadata {
                relation: SCHEDULING_DECISION_RELATION.to_owned(),
                schema_version: SCHEDULING_DECISION_SCHEMA_VERSION.to_owned(),
                semantic_digest: self.decision_id.to_string(),
                row_count: self.selected.len() as u64,
                complete: true,
                key_columns: vec!["selection_rank".to_owned()],
                attributes,
            },
        )?)
    }
}

fn count_readiness(frontier: &Frontier, readiness: Readiness) -> u64 {
    frontier
        .rows
        .iter()
        .filter(|row| row.readiness == readiness)
        .count() as u64
}

fn validate_limits(limits: &SchedulerLimits) -> Result<(), FrontierError> {
    let values = [
        ("max_rows_considered", limits.max_rows_considered),
        ("max_work_units", limits.max_work_units),
        ("max_total_cost_units", limits.max_total_cost_units),
        ("max_string_bytes", limits.max_string_bytes),
    ];
    if let Some((name, _)) = values.into_iter().find(|(_, value)| *value == 0) {
        return Err(FrontierError::ZeroSchedulerLimit(name));
    }
    Ok(())
}

fn validate_selection(selected: &[ScheduledWork]) -> Result<(), FrontierError> {
    let mut work_ids = BTreeSet::new();
    let mut previous: Option<&ScheduledWork> = None;
    for (index, work) in selected.iter().enumerate() {
        crate::validate_text("scheduled work id", &work.work_id)?;
        validate_digest(&work.frontier_row_id)?;
        if work.selection_rank != index as u64 + 1
            || work.estimated_cost_units == 0
            || !work_ids.insert(work.work_id.as_str())
        {
            return Err(FrontierError::SchedulingShape);
        }
        if previous.is_some_and(|prior| {
            prior.priority < work.priority
                || (prior.priority == work.priority
                    && prior.estimated_cost_units > work.estimated_cost_units)
                || (prior.priority == work.priority
                    && prior.estimated_cost_units == work.estimated_cost_units
                    && prior.work_id >= work.work_id)
        }) {
            return Err(FrontierError::NonCanonical("scheduled work"));
        }
        previous = Some(work);
    }
    Ok(())
}

fn validate_outcome(decision: &SchedulingDecision) -> Result<(), FrontierError> {
    let valid = match decision.outcome {
        ScheduleOutcome::Selected => {
            decision.inputs.frontier_assessment == FrontierAssessment::Open
                && !decision.selected.is_empty()
        }
        ScheduleOutcome::NoReadyWork => {
            decision.inputs.frontier_assessment == FrontierAssessment::Open
                && decision.ready_rows == 0
                && decision.selected.is_empty()
        }
        ScheduleOutcome::BudgetExhausted => {
            decision.inputs.frontier_assessment == FrontierAssessment::Open
                && decision.ready_rows > 0
                && decision.selected.is_empty()
        }
        ScheduleOutcome::FrontierConverged => {
            decision.inputs.frontier_assessment == FrontierAssessment::Converged
                && decision.considered_rows == 0
                && decision.selected.is_empty()
        }
        ScheduleOutcome::FrontierInconclusive => {
            decision.inputs.frontier_assessment == FrontierAssessment::Inconclusive
                && decision.considered_rows == 0
                && decision.selected.is_empty()
        }
    };
    if valid {
        Ok(())
    } else {
        Err(FrontierError::SchedulingShape)
    }
}

fn validate_string_bytes(
    inputs: &SchedulingInputs,
    selected: &[ScheduledWork],
    limits: &SchedulerLimits,
) -> Result<(), FrontierError> {
    let mut total = 0_u64;
    for contract in [
        &inputs.application,
        &inputs.component,
        &inputs.space,
        &inputs.scheduler,
    ] {
        add_string_bytes(&mut total, &contract.id)?;
        add_string_bytes(&mut total, contract.semantic_digest.as_str())?;
    }
    for digest in [
        &inputs.trace_id,
        &inputs.committed_record_id,
        &inputs.frontier_id,
        &inputs.capability_snapshot_id,
    ] {
        add_string_bytes(&mut total, digest.as_str())?;
    }
    for work in selected {
        add_string_bytes(&mut total, &work.work_id)?;
        add_string_bytes(&mut total, work.frontier_row_id.as_str())?;
    }
    if total > limits.max_string_bytes {
        return Err(FrontierError::Limit {
            kind: "scheduler string byte",
            limit: limits.max_string_bytes,
            observed: total,
        });
    }
    Ok(())
}

fn decision_digest(decision: &SchedulingDecision) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(SCHEDULING_DECISION_SCHEMA);
    decision.inputs.application.add_semantics(&mut hasher);
    decision.inputs.component.add_semantics(&mut hasher);
    decision.inputs.space.add_semantics(&mut hasher);
    hasher.add_str(decision.inputs.trace_id.as_str());
    hasher.add_str(decision.inputs.committed_record_id.as_str());
    hasher.add_str(decision.inputs.frontier_id.as_str());
    hasher.add_str(decision.inputs.capability_snapshot_id.as_str());
    hasher.add_str(decision.inputs.frontier_assessment.as_str());
    decision.inputs.scheduler.add_semantics(&mut hasher);
    hasher.add_u64(decision.limits.max_rows_considered);
    hasher.add_u64(decision.limits.max_work_units);
    hasher.add_u64(decision.limits.max_total_cost_units);
    hasher.add_u64(decision.limits.max_string_bytes);
    hasher.add_str(decision.outcome.as_str());
    hasher.add_u64(decision.considered_rows);
    hasher.add_u64(decision.ready_rows);
    hasher.add_u64(decision.blocked_rows);
    hasher.add_u64(decision.inconclusive_rows);
    hasher.add_u64(decision.selected_cost_units);
    hasher.add_u64(decision.deferred_ready_rows);
    hasher.add_u64(decision.skipped_over_cost_rows);
    hasher.add_u64(decision.selected.len() as u64);
    for work in &decision.selected {
        hasher.add_u64(work.selection_rank);
        hasher.add_str(&work.work_id);
        hasher.add_str(work.frontier_row_id.as_str());
        hasher.add_u64(work.priority);
        hasher.add_u64(work.estimated_cost_units);
    }
    hasher.finish()
}

fn placeholder_digest() -> SemanticDigest {
    SemanticHasher::new("rey.scheduling-decision.placeholder").finish()
}
