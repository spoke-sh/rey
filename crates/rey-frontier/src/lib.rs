#![forbid(unsafe_code)]

mod frontier;
mod progress;
mod scheduling;

use rey_core::{ContractIdentity, SemanticDigest};

pub use frontier::{
    FRONTIER_RELATION, FRONTIER_SCHEMA, FRONTIER_SCHEMA_VERSION, Frontier, FrontierAssessment,
    FrontierBlocker, FrontierBlockerKind, FrontierCoverage, FrontierInputs, FrontierLimits,
    FrontierRow, FrontierRowInput, Readiness, RequiredClaims,
};
pub use progress::{
    FRONTIER_PROGRESS_RELATION, FRONTIER_PROGRESS_SCHEMA, FRONTIER_PROGRESS_SCHEMA_VERSION,
    FrontierProgress, ProgressAssessment, ProgressChange, ProgressChangeKind, ProgressInputs,
    ProgressLimits, ProgressSummary, frontier_comparator,
};
pub use scheduling::{
    SCHEDULING_DECISION_RELATION, SCHEDULING_DECISION_SCHEMA, SCHEDULING_DECISION_SCHEMA_VERSION,
    ScheduleOutcome, ScheduledWork, SchedulerLimits, SchedulingDecision, SchedulingInputs,
    SchedulingPreconditions, deterministic_scheduler, schedule,
};
use thiserror::Error;

pub(crate) const MAX_TEXT_BYTES: usize = 4_096;

pub(crate) fn validate_contract(
    field: &'static str,
    contract: &ContractIdentity,
) -> Result<(), FrontierError> {
    validate_text(field, &contract.id)?;
    if contract.revision == 0 {
        return Err(FrontierError::ZeroRevision(field));
    }
    validate_digest(&contract.semantic_digest)
}

pub(crate) fn validate_text(field: &'static str, value: &str) -> Result<(), FrontierError> {
    if value.is_empty() || value.len() > MAX_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(FrontierError::InvalidText(field));
    }
    Ok(())
}

pub(crate) fn validate_digest(digest: &SemanticDigest) -> Result<(), FrontierError> {
    let value = digest.as_str();
    let Some(hex) = value.strip_prefix("blake3:") else {
        return Err(FrontierError::InvalidDigest(value.to_owned()));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(FrontierError::InvalidDigest(value.to_owned()));
    }
    Ok(())
}

pub(crate) fn add_string_bytes(total: &mut u64, value: &str) -> Result<(), FrontierError> {
    *total = total
        .checked_add(value.len() as u64)
        .ok_or(FrontierError::CountOverflow)?;
    Ok(())
}

#[derive(Debug, Error)]
pub enum FrontierError {
    #[error("unsupported {kind} schema {actual}; expected {expected}")]
    UnsupportedSchema {
        kind: &'static str,
        expected: &'static str,
        actual: String,
    },
    #[error("{kind} digest mismatch: declared {declared}, actual {actual}")]
    DigestMismatch {
        kind: &'static str,
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("{0} is not in canonical order")]
    NonCanonical(&'static str),
    #[error("invalid text field {0}")]
    InvalidText(&'static str),
    #[error("contract field {0} has revision zero")]
    ZeroRevision(&'static str),
    #[error("{0} does not match the implemented contract")]
    UnexpectedContract(&'static str),
    #[error("invalid semantic digest {0}")]
    InvalidDigest(String),
    #[error("frontier limit {0} must be greater than zero")]
    ZeroLimit(&'static str),
    #[error("frontier {kind} limit {limit} exceeded by {observed}")]
    Limit {
        kind: &'static str,
        limit: u64,
        observed: u64,
    },
    #[error("frontier contains duplicate work id {0}")]
    DuplicateWork(String),
    #[error("frontier row {0} cites neither a delta nor a claim")]
    UndirectedWork(String),
    #[error("frontier row {0} assigns one delta both transition and residual roles")]
    AmbiguousDeltaRole(String),
    #[error("frontier row {0} has zero estimated cost")]
    ZeroEstimatedCost(String),
    #[error("frontier row {0} readiness does not agree with its blockers")]
    ReadinessBlockerMismatch(String),
    #[error("a violated required claim requires at least one frontier row")]
    MissingViolatedClaimWork,
    #[error("frontier assessment does not match rows and coverage")]
    AssessmentMismatch,
    #[error("frontiers are not comparable because {0} differs")]
    IncompatibleFrontiers(&'static str),
    #[error("progress limit {0} must be greater than zero")]
    ZeroProgressLimit(&'static str),
    #[error("progress {kind} limit {limit} exceeded by {observed}")]
    ProgressLimit {
        kind: &'static str,
        limit: u64,
        observed: u64,
    },
    #[error("progress summary does not match its change relation")]
    ProgressSummaryMismatch,
    #[error("scheduler limit {0} must be greater than zero")]
    ZeroSchedulerLimit(&'static str),
    #[error("scheduler row limit {limit} exceeded by {observed}")]
    SchedulerRowLimit { limit: u64, observed: u64 },
    #[error("stale committed record: expected {expected}, actual {actual}")]
    StaleCommittedRecord {
        expected: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("stale frontier: expected {expected}, actual {actual}")]
    StaleFrontier {
        expected: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("stale capability snapshot: expected {expected}, actual {actual}")]
    StaleCapabilitySnapshot {
        expected: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("scheduling decision shape does not match its outcome or counters")]
    SchedulingShape,
    #[error("scheduling decision does not reproduce against its frontier")]
    SchedulingReplayMismatch,
    #[error("frontier count or byte arithmetic overflowed")]
    CountOverflow,
    #[error("frontier dataframe failed: {0}")]
    Frame(#[from] rey_dataframe::FrameError),
    #[error("frontier JSON projection failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("frontier dataframe failed: {0}")]
    Polars(#[from] polars::error::PolarsError),
}

#[cfg(test)]
mod tests {
    use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
    use rey_dataframe::Frame;

    use super::*;

    fn digest(value: &str) -> SemanticDigest {
        let mut hasher = SemanticHasher::new("rey.frontier-test.v1");
        hasher.add_str(value);
        hasher.finish()
    }

    fn contract(id: &str) -> ContractIdentity {
        ContractIdentity::new(id, 1, id)
    }

    fn inputs(record: &str, capability: &str) -> FrontierInputs {
        FrontierInputs {
            workload: contract("workload"),
            graph: contract("graph"),
            scenario_suite: contract("scenario-suite"),
            campaign_id: digest("campaign"),
            space: contract("space"),
            trace_id: digest("trace"),
            committed_record_id: digest(record),
            capability_snapshot_id: digest(capability),
            derivation: contract("frontier-derivation"),
            prioritization: contract("priority-inputs"),
        }
    }

    fn complete_coverage() -> FrontierCoverage {
        FrontierCoverage {
            deltas_complete: true,
            claims_complete: true,
            required_claims: RequiredClaims::Violated,
        }
    }

    fn row(work_id: &str, priority: u64, cost: u64) -> FrontierRowInput {
        FrontierRowInput {
            work_id: work_id.to_owned(),
            entity_kind: "symbol".to_owned(),
            entity_id: format!("entity-{work_id}"),
            transition_delta_ids: vec![digest(&format!("transition-{work_id}"))],
            residual_delta_ids: vec![digest(&format!("residual-{work_id}"))],
            claim_ids: vec![format!("claim-{work_id}")],
            dependent_lens_ids: vec![format!("lens-{work_id}")],
            admissible_action_ids: vec![format!("action-{work_id}")],
            readiness: Readiness::Ready,
            blockers: Vec::new(),
            priority,
            estimated_cost_units: cost,
        }
    }

    fn blocked_row(work_id: &str) -> FrontierRowInput {
        let mut row = row(work_id, 1, 1);
        row.readiness = Readiness::Blocked;
        row.blockers = vec![FrontierBlocker {
            kind: FrontierBlockerKind::Dependency,
            blocker_id: "dependency-a".to_owned(),
            reason: "dependency remains unresolved".to_owned(),
        }];
        row
    }

    fn frontier(record: &str, rows: Vec<FrontierRowInput>) -> Frontier {
        Frontier::new(
            inputs(record, "capabilities"),
            FrontierLimits::default(),
            complete_coverage(),
            rows,
        )
        .unwrap()
    }

    fn preconditions(frontier: &Frontier) -> SchedulingPreconditions {
        SchedulingPreconditions {
            expected_committed_record_id: frontier.inputs.committed_record_id.clone(),
            expected_frontier_id: frontier.frontier_id.clone(),
            expected_capability_snapshot_id: frontier.inputs.capability_snapshot_id.clone(),
        }
    }

    #[test]
    fn frontier_is_canonical_content_identified_and_arrow_projected() {
        let left = frontier("record-1", vec![row("b", 1, 2), row("a", 2, 1)]);
        let mut a = row("a", 2, 1);
        a.claim_ids.push("claim-a".to_owned());
        a.residual_delta_ids.push(digest("residual-a"));
        let right = frontier("record-1", vec![a, row("b", 1, 2)]);

        assert_eq!(left, right);
        assert_eq!(left.rows[0].work_id, "a");
        assert_eq!(left.assessment, FrontierAssessment::Open);
        left.verify().unwrap();

        let frame = left.to_frame().unwrap();
        assert_eq!(frame.metadata().relation, FRONTIER_RELATION);
        assert_eq!(frame.metadata().row_count, 2);
        assert_eq!(frame.metadata().key_columns, ["work_id"]);
        let decoded = Frame::from_arrow_stream(&frame.to_arrow_stream().unwrap()).unwrap();
        assert_eq!(decoded.metadata(), frame.metadata());
        assert!(decoded.dataframe().equals_missing(frame.dataframe()));
    }

    #[test]
    fn convergence_is_derived_only_from_complete_empty_evidence() {
        let converged = Frontier::new(
            inputs("record", "capabilities"),
            FrontierLimits::default(),
            FrontierCoverage {
                deltas_complete: true,
                claims_complete: true,
                required_claims: RequiredClaims::Satisfied,
            },
            Vec::new(),
        )
        .unwrap();
        assert_eq!(converged.assessment, FrontierAssessment::Converged);
        let frame = converged.to_frame().unwrap();
        assert_eq!(frame.dataframe().height(), 0);
        let decoded = Frame::from_arrow_stream(&frame.to_arrow_stream().unwrap()).unwrap();
        assert_eq!(decoded.metadata(), frame.metadata());
        assert!(decoded.dataframe().equals_missing(frame.dataframe()));

        let inconclusive = Frontier::new(
            inputs("record", "capabilities"),
            FrontierLimits::default(),
            FrontierCoverage {
                deltas_complete: false,
                claims_complete: true,
                required_claims: RequiredClaims::Satisfied,
            },
            Vec::new(),
        )
        .unwrap();
        assert_eq!(inconclusive.assessment, FrontierAssessment::Inconclusive);
        assert!(matches!(
            Frontier::new(
                inputs("record", "capabilities"),
                FrontierLimits::default(),
                complete_coverage(),
                Vec::new(),
            ),
            Err(FrontierError::MissingViolatedClaimWork)
        ));
    }

    #[test]
    fn row_direction_readiness_and_limits_fail_closed() {
        let mut undirected = row("undirected", 1, 1);
        undirected.transition_delta_ids.clear();
        undirected.residual_delta_ids.clear();
        undirected.claim_ids.clear();
        assert!(matches!(
            Frontier::new(
                inputs("record", "capabilities"),
                FrontierLimits::default(),
                complete_coverage(),
                vec![undirected],
            ),
            Err(FrontierError::UndirectedWork(_))
        ));

        let mut mismatched = row("mismatched", 1, 1);
        mismatched.readiness = Readiness::Blocked;
        assert!(matches!(
            Frontier::new(
                inputs("record", "capabilities"),
                FrontierLimits::default(),
                complete_coverage(),
                vec![mismatched],
            ),
            Err(FrontierError::ReadinessBlockerMismatch(_))
        ));

        assert!(matches!(
            Frontier::new(
                inputs("record", "capabilities"),
                FrontierLimits {
                    max_rows: 1,
                    ..FrontierLimits::default()
                },
                complete_coverage(),
                vec![row("a", 1, 1), row("b", 1, 1)],
            ),
            Err(FrontierError::Limit { kind: "row", .. })
        ));
    }

    #[test]
    fn duplicate_work_and_tampered_frontier_are_rejected() {
        assert!(matches!(
            Frontier::new(
                inputs("record", "capabilities"),
                FrontierLimits::default(),
                complete_coverage(),
                vec![row("same", 1, 1), row("same", 2, 1)],
            ),
            Err(FrontierError::DuplicateWork(_))
        ));
        let mut tampered = frontier("record", vec![row("a", 1, 1)]);
        tampered.rows[0].priority = 99;
        assert!(tampered.verify().is_err());
    }

    #[test]
    fn progress_relation_preserves_direction_and_updated_work() {
        let source = frontier(
            "record-1",
            vec![
                row("resolved", 2, 2),
                row("updated", 1, 1),
                row("same", 1, 1),
            ],
        );
        let target = frontier(
            "record-2",
            vec![
                row("introduced", 3, 1),
                row("updated", 9, 1),
                row("same", 1, 1),
            ],
        );
        let progress =
            FrontierProgress::compare(&source, &target, ProgressLimits::default()).unwrap();

        assert_eq!(progress.summary.resolved, 1);
        assert_eq!(progress.summary.introduced, 1);
        assert_eq!(progress.summary.updated, 1);
        assert_eq!(progress.summary.unchanged, 1);
        assert_eq!(progress.summary.assessment, ProgressAssessment::Mixed);
        assert_eq!(progress.changes[0].work_id, "introduced");
        progress.verify_against(&source, &target).unwrap();

        let frame = progress.to_frame().unwrap();
        let decoded = Frame::from_arrow_stream(&frame.to_arrow_stream().unwrap()).unwrap();
        assert_eq!(decoded.metadata(), frame.metadata());
        assert!(decoded.dataframe().equals_missing(frame.dataframe()));
    }

    #[test]
    fn progress_assessment_does_not_invent_a_scalar_score() {
        let source = frontier("record-1", vec![row("a", 1, 1), row("b", 1, 1)]);
        let target = frontier("record-2", vec![row("b", 1, 1)]);
        let progressing =
            FrontierProgress::compare(&source, &target, ProgressLimits::default()).unwrap();
        assert_eq!(
            progressing.summary.assessment,
            ProgressAssessment::Progressing
        );

        let regressing =
            FrontierProgress::compare(&target, &source, ProgressLimits::default()).unwrap();
        assert_eq!(
            regressing.summary.assessment,
            ProgressAssessment::Regressing
        );

        let converged = Frontier::new(
            inputs("record-3", "capabilities"),
            FrontierLimits::default(),
            FrontierCoverage {
                deltas_complete: true,
                claims_complete: true,
                required_claims: RequiredClaims::Satisfied,
            },
            Vec::new(),
        )
        .unwrap();
        let result =
            FrontierProgress::compare(&target, &converged, ProgressLimits::default()).unwrap();
        assert_eq!(result.summary.assessment, ProgressAssessment::Converged);

        let unchanged =
            FrontierProgress::compare(&target, &target, ProgressLimits::default()).unwrap();
        assert!(unchanged.changes.is_empty());
        let frame = unchanged.to_frame().unwrap();
        let decoded = Frame::from_arrow_stream(&frame.to_arrow_stream().unwrap()).unwrap();
        assert_eq!(decoded.metadata(), frame.metadata());
        assert!(decoded.dataframe().equals_missing(frame.dataframe()));

        let incomplete = Frontier::new(
            inputs("record-4", "capabilities"),
            FrontierLimits::default(),
            FrontierCoverage {
                deltas_complete: false,
                claims_complete: true,
                required_claims: RequiredClaims::Violated,
            },
            vec![row("b", 1, 1)],
        )
        .unwrap();
        let result =
            FrontierProgress::compare(&target, &incomplete, ProgressLimits::default()).unwrap();
        assert_eq!(result.summary.assessment, ProgressAssessment::Inconclusive);
        assert!(!result.to_frame().unwrap().metadata().complete);
    }

    #[test]
    fn incompatible_frontier_contracts_are_not_compared() {
        let source = frontier("record-1", vec![row("a", 1, 1)]);
        let mut target_inputs = inputs("record-2", "capabilities");
        target_inputs.space = contract("other-space");
        let target = Frontier::new(
            target_inputs,
            FrontierLimits::default(),
            complete_coverage(),
            vec![row("a", 1, 1)],
        )
        .unwrap();
        assert!(matches!(
            FrontierProgress::compare(&source, &target, ProgressLimits::default(),),
            Err(FrontierError::IncompatibleFrontiers("space contract"))
        ));
    }

    #[test]
    fn progress_and_scheduler_consideration_limits_fail_closed() {
        let source = frontier("record-1", vec![row("a", 1, 1), row("b", 1, 1)]);
        let target = frontier("record-2", vec![row("a", 1, 1)]);
        assert!(matches!(
            FrontierProgress::compare(
                &source,
                &target,
                ProgressLimits {
                    max_source_rows: 1,
                    ..ProgressLimits::default()
                },
            ),
            Err(FrontierError::ProgressLimit {
                kind: "source row",
                ..
            })
        ));
        assert!(matches!(
            schedule(
                &source,
                preconditions(&source),
                SchedulerLimits {
                    max_rows_considered: 1,
                    ..SchedulerLimits::default()
                },
            ),
            Err(FrontierError::SchedulerRowLimit { .. })
        ));
    }

    #[test]
    fn scheduler_uses_priority_cost_and_stable_identity_with_hard_bounds() {
        let frontier = frontier(
            "record",
            vec![
                row("expensive", 10, 8),
                row("cheap", 10, 3),
                row("lower", 5, 2),
                blocked_row("blocked"),
            ],
        );
        let decision = schedule(
            &frontier,
            preconditions(&frontier),
            SchedulerLimits {
                max_rows_considered: 4,
                max_work_units: 2,
                max_total_cost_units: 5,
                max_string_bytes: 64 * 1_024,
            },
        )
        .unwrap();

        assert_eq!(decision.outcome, ScheduleOutcome::Selected);
        assert_eq!(
            decision
                .selected
                .iter()
                .map(|work| work.work_id.as_str())
                .collect::<Vec<_>>(),
            ["cheap", "lower"]
        );
        assert_eq!(decision.selected_cost_units, 5);
        assert_eq!(decision.deferred_ready_rows, 1);
        assert_eq!(decision.skipped_over_cost_rows, 1);
        decision.verify_against(&frontier).unwrap();

        let frame = decision.to_frame().unwrap();
        let decoded = Frame::from_arrow_stream(&frame.to_arrow_stream().unwrap()).unwrap();
        assert_eq!(decoded.metadata(), frame.metadata());
        assert!(decoded.dataframe().equals_missing(frame.dataframe()));
    }

    #[test]
    fn scheduler_rejects_each_stale_precondition() {
        let frontier = frontier("record", vec![row("a", 1, 1)]);
        let mut stale = preconditions(&frontier);
        stale.expected_frontier_id = digest("old-frontier");
        assert!(matches!(
            schedule(&frontier, stale, SchedulerLimits::default(),),
            Err(FrontierError::StaleFrontier { .. })
        ));

        let mut stale = preconditions(&frontier);
        stale.expected_capability_snapshot_id = digest("old-capabilities");
        assert!(matches!(
            schedule(&frontier, stale, SchedulerLimits::default(),),
            Err(FrontierError::StaleCapabilitySnapshot { .. })
        ));

        let mut stale = preconditions(&frontier);
        stale.expected_committed_record_id = digest("old-record");
        assert!(matches!(
            schedule(&frontier, stale, SchedulerLimits::default(),),
            Err(FrontierError::StaleCommittedRecord { .. })
        ));
    }

    #[test]
    fn scheduler_reports_no_ready_budget_and_terminal_frontiers() {
        let blocked = frontier("blocked", vec![blocked_row("a")]);
        let no_ready = schedule(
            &blocked,
            preconditions(&blocked),
            SchedulerLimits::default(),
        )
        .unwrap();
        assert_eq!(no_ready.outcome, ScheduleOutcome::NoReadyWork);

        let costly = frontier("costly", vec![row("a", 1, 10)]);
        let exhausted = schedule(
            &costly,
            preconditions(&costly),
            SchedulerLimits {
                max_total_cost_units: 5,
                ..SchedulerLimits::default()
            },
        )
        .unwrap();
        assert_eq!(exhausted.outcome, ScheduleOutcome::BudgetExhausted);

        let converged = Frontier::new(
            inputs("converged", "capabilities"),
            FrontierLimits::default(),
            FrontierCoverage {
                deltas_complete: true,
                claims_complete: true,
                required_claims: RequiredClaims::Satisfied,
            },
            Vec::new(),
        )
        .unwrap();
        let converged_decision = schedule(
            &converged,
            preconditions(&converged),
            SchedulerLimits::default(),
        )
        .unwrap();
        assert_eq!(
            converged_decision.outcome,
            ScheduleOutcome::FrontierConverged
        );
        let frame = converged_decision.to_frame().unwrap();
        let decoded = Frame::from_arrow_stream(&frame.to_arrow_stream().unwrap()).unwrap();
        assert_eq!(decoded.metadata(), frame.metadata());
        assert!(decoded.dataframe().equals_missing(frame.dataframe()));

        let inconclusive = Frontier::new(
            inputs("inconclusive", "capabilities"),
            FrontierLimits::default(),
            FrontierCoverage {
                deltas_complete: false,
                claims_complete: true,
                required_claims: RequiredClaims::Unknown,
            },
            Vec::new(),
        )
        .unwrap();
        assert_eq!(
            schedule(
                &inconclusive,
                preconditions(&inconclusive),
                SchedulerLimits::default(),
            )
            .unwrap()
            .outcome,
            ScheduleOutcome::FrontierInconclusive
        );
    }

    #[test]
    fn scheduling_decision_json_replay_and_tamper_detection_are_stable() {
        let frontier = frontier("record", vec![row("a", 1, 1)]);
        let decision = schedule(
            &frontier,
            preconditions(&frontier),
            SchedulerLimits::default(),
        )
        .unwrap();
        let json = serde_json::to_vec(&decision).unwrap();
        let decoded: SchedulingDecision = serde_json::from_slice(&json).unwrap();
        assert_eq!(decoded, decision);
        decoded.verify_against(&frontier).unwrap();

        let mut tampered = decoded;
        tampered.selected[0].priority += 1;
        assert!(tampered.verify().is_err());

        let mut wrong_contract = decision;
        wrong_contract.inputs.scheduler = contract("other-scheduler");
        assert!(matches!(
            wrong_contract.verify(),
            Err(FrontierError::UnexpectedContract("scheduler"))
        ));
    }
}
