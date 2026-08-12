#![forbid(unsafe_code)]

mod portfolio;
mod topography;
mod workload;
mod workload_mining;

pub use topography::*;

pub use portfolio::{
    AttentionAction, AttentionPolicy, AttentionReadiness, AttentionReason, AttentionSubjectKind,
    BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID, PORTFOLIO_SNAPSHOT_SCHEMA, PortfolioError,
    PortfolioLimits, PortfolioQualificationState, PortfolioReasoningEvidence, PortfolioSnapshot,
    PortfolioSurfaceObservation, PortfolioWorkloadObservation, WORKLOAD_ATTENTION_RELATION,
    WORKLOAD_ATTENTION_SCHEMA, WORKLOAD_ATTENTION_SCHEMA_VERSION, WorkloadAttention,
    WorkloadAttentionRow, WorkloadAttentionSummary, derive_portfolio_frontier,
    orient_portfolio_attention, portfolio_attention_operation, render_workload_attention,
    render_workload_attention_operation, verify_portfolio_frontier,
};

pub use workload::{
    BUILT_IN_MISMATCH_WORKLOAD_ID, BUILT_IN_NORMALIZE_WORKLOAD_ID, COMPUTE_GRAPH_SCHEMA,
    ComputeGraph, GraphExecution, GraphLimits, GraphNode, GraphOutput, QualificationRecord,
    RunStatus, SCENARIO_SUITE_SCHEMA, Scenario, ScenarioEvaluation, ScenarioResult, ScenarioSuite,
    TestStatus, TestSummary, ValueSource, ValueType, WORKLOAD_QUALIFICATION_SCHEMA,
    WORKLOAD_RUN_RESULT_SCHEMA, WORKLOAD_SCHEMA, WORKLOAD_TEST_RESULT_SCHEMA, WorkloadDefinition,
    WorkloadDefinitionParts, WorkloadError, WorkloadLimits, WorkloadOwnedSurface, WorkloadPort,
    WorkloadRunResult, WorkloadTestResult, WorkloadValue, built_in_operation_contract,
    built_in_workload, built_in_workloads, execute_workload, execute_workload_with_source,
    execute_workload_with_topography, run_workload, run_workload_with_source,
    run_workload_with_topography, test_workload, test_workload_with_observer,
    test_workload_with_observer_and_snapshot, utf8_exact_comparator_contract,
};
pub use workload_mining::{
    BUILT_IN_SOURCE_SEARCH_WORKLOAD_ID, MiningReasoningEvidence, MiningScenarioEvidence,
    SourceMiningExecution, SourceRunInput, SourceSearchScenario, source_fixture_paths,
    source_fixture_root,
};

use rey_core::{SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const RUNTIME_STATE_SCHEMA: &str = "rey.runtime-state.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeLimits {
    pub max_events: u64,
    pub max_observation_refs: u64,
    pub max_transition_delta_refs: u64,
    pub max_residual_delta_refs: u64,
}

impl Default for RuntimeLimits {
    fn default() -> Self {
        Self {
            max_events: 1_024,
            max_observation_refs: 256,
            max_transition_delta_refs: 256,
            max_residual_delta_refs: 256,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePhase {
    Bootstrapping,
    Ready,
    Scheduling,
    Orienting,
    AwaitingProposal,
    Admitting,
    Executing,
    Observing,
    Evaluating,
    Committing,
    Stopped,
}

impl RuntimePhase {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Bootstrapping => "bootstrapping",
            Self::Ready => "ready",
            Self::Scheduling => "scheduling",
            Self::Orienting => "orienting",
            Self::AwaitingProposal => "awaiting_proposal",
            Self::Admitting => "admitting",
            Self::Executing => "executing",
            Self::Observing => "observing",
            Self::Evaluating => "evaluating",
            Self::Committing => "committing",
            Self::Stopped => "stopped",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionProfile {
    Local,
}

impl RetentionProfile {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    Pending,
    Retained,
    Verified,
    Missing,
    Stale,
}

impl EvidenceState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Retained => "retained",
            Self::Verified => "verified",
            Self::Missing => "missing",
            Self::Stale => "stale",
        }
    }

    const fn permits_continuation(self) -> bool {
        matches!(self, Self::Retained | Self::Verified)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionOutcome {
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Lost,
}

impl ExecutionOutcome {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::Lost => "lost",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationOutcome {
    Complete,
    Partial,
    Unavailable,
    Failed,
}

impl ObservationOutcome {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Unavailable => "unavailable",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticOutcome {
    Unresolved,
    Progressing,
    Unchanged,
    Regressing,
    Converged,
    Inconclusive,
}

impl SemanticOutcome {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Unresolved => "unresolved",
            Self::Progressing => "progressing",
            Self::Unchanged => "unchanged",
            Self::Regressing => "regressing",
            Self::Converged => "converged",
            Self::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    Converged,
    BudgetExhausted,
    Cancelled,
    TimedOut,
    EvidenceMissing,
    NoEligibleEvidence,
    CapabilityUnavailable,
    Inconclusive,
    Failed,
}

impl StopReason {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Converged => "converged",
            Self::BudgetExhausted => "budget_exhausted",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::EvidenceMissing => "evidence_missing",
            Self::NoEligibleEvidence => "no_eligible_evidence",
            Self::CapabilityUnavailable => "capability_unavailable",
            Self::Inconclusive => "inconclusive",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommitDisposition {
    Continue,
    Stop { reason: StopReason },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeEvent {
    BootstrapCommitted {
        baseline_id: SemanticDigest,
        frontier_id: Option<SemanticDigest>,
        semantic_outcome: SemanticOutcome,
        evidence_state: EvidenceState,
        disposition: CommitDisposition,
    },
    BeginScheduling {
        transition_id: SemanticDigest,
    },
    SchedulingCompleted {
        transition_id: SemanticDigest,
        decision_id: SemanticDigest,
    },
    SchedulingStopped {
        transition_id: SemanticDigest,
        decision_id: SemanticDigest,
        semantic_outcome: SemanticOutcome,
        reason: StopReason,
    },
    ReasoningSurfaceReady {
        transition_id: SemanticDigest,
        surface_id: SemanticDigest,
    },
    OrientationFailed {
        transition_id: SemanticDigest,
        reason: StopReason,
    },
    ProposalReceived {
        transition_id: SemanticDigest,
        proposal_id: SemanticDigest,
    },
    ProposalAdmitted {
        transition_id: SemanticDigest,
    },
    ProposalRejected {
        transition_id: SemanticDigest,
    },
    CancellationRequested {
        transition_id: SemanticDigest,
    },
    ExecutionFinished {
        transition_id: SemanticDigest,
        outcome: ExecutionOutcome,
    },
    ObservationCompleted {
        transition_id: SemanticDigest,
        outcome: ObservationOutcome,
        observation_ids: Vec<SemanticDigest>,
    },
    EvaluationCompleted {
        transition_id: SemanticDigest,
        transition_delta_ids: Vec<SemanticDigest>,
        residual_delta_ids: Vec<SemanticDigest>,
        semantic_outcome: SemanticOutcome,
        next_frontier_id: Option<SemanticDigest>,
    },
    TransitionCommitted {
        transition_id: SemanticDigest,
        record_id: SemanticDigest,
        evidence_state: EvidenceState,
        disposition: CommitDisposition,
    },
}

impl RuntimeEvent {
    const fn name(&self) -> &'static str {
        match self {
            Self::BootstrapCommitted { .. } => "bootstrap_committed",
            Self::BeginScheduling { .. } => "begin_scheduling",
            Self::SchedulingCompleted { .. } => "scheduling_completed",
            Self::SchedulingStopped { .. } => "scheduling_stopped",
            Self::ReasoningSurfaceReady { .. } => "reasoning_surface_ready",
            Self::OrientationFailed { .. } => "orientation_failed",
            Self::ProposalReceived { .. } => "proposal_received",
            Self::ProposalAdmitted { .. } => "proposal_admitted",
            Self::ProposalRejected { .. } => "proposal_rejected",
            Self::CancellationRequested { .. } => "cancellation_requested",
            Self::ExecutionFinished { .. } => "execution_finished",
            Self::ObservationCompleted { .. } => "observation_completed",
            Self::EvaluationCompleted { .. } => "evaluation_completed",
            Self::TransitionCommitted { .. } => "transition_committed",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeState {
    pub schema: String,
    pub state_id: SemanticDigest,
    pub trace_id: SemanticDigest,
    pub event_sequence: u64,
    pub limits: RuntimeLimits,
    pub phase: RuntimePhase,
    pub retention_profile: RetentionProfile,
    pub evidence_state: EvidenceState,
    pub baseline_id: Option<SemanticDigest>,
    pub committed_transition_id: Option<SemanticDigest>,
    pub committed_record_id: Option<SemanticDigest>,
    pub frontier_id: Option<SemanticDigest>,
    pub active_transition_id: Option<SemanticDigest>,
    pub scheduling_decision_id: Option<SemanticDigest>,
    pub reasoning_surface_id: Option<SemanticDigest>,
    pub proposal_id: Option<SemanticDigest>,
    pub cancellation_requested: bool,
    pub execution_outcome: Option<ExecutionOutcome>,
    pub observation_outcome: Option<ObservationOutcome>,
    pub observation_ids: Vec<SemanticDigest>,
    pub transition_delta_ids: Vec<SemanticDigest>,
    pub residual_delta_ids: Vec<SemanticDigest>,
    pub semantic_outcome: Option<SemanticOutcome>,
    pub pending_stop_reason: Option<StopReason>,
    pub stop_reason: Option<StopReason>,
}

impl RuntimeState {
    pub fn bootstrap(
        trace_id: SemanticDigest,
        retention_profile: RetentionProfile,
        limits: RuntimeLimits,
    ) -> Result<Self, RuntimeError> {
        validate_limits(&limits)?;
        let mut state = Self {
            schema: RUNTIME_STATE_SCHEMA.to_owned(),
            state_id: placeholder_digest(),
            trace_id,
            event_sequence: 0,
            limits,
            phase: RuntimePhase::Bootstrapping,
            retention_profile,
            evidence_state: EvidenceState::Pending,
            baseline_id: None,
            committed_transition_id: None,
            committed_record_id: None,
            frontier_id: None,
            active_transition_id: None,
            scheduling_decision_id: None,
            reasoning_surface_id: None,
            proposal_id: None,
            cancellation_requested: false,
            execution_outcome: None,
            observation_outcome: None,
            observation_ids: Vec::new(),
            transition_delta_ids: Vec::new(),
            residual_delta_ids: Vec::new(),
            semantic_outcome: None,
            pending_stop_reason: None,
            stop_reason: None,
        };
        state.state_id = state_digest(&state);
        Ok(state)
    }

    pub fn apply(&self, event: RuntimeEvent) -> Result<Self, RuntimeError> {
        self.verify()?;
        let event_name = event.name();
        let mut next = self.clone();
        match event {
            RuntimeEvent::BootstrapCommitted {
                baseline_id,
                frontier_id,
                semantic_outcome,
                evidence_state,
                disposition,
            } if self.phase == RuntimePhase::Bootstrapping => {
                if !matches!(
                    semantic_outcome,
                    SemanticOutcome::Unresolved
                        | SemanticOutcome::Converged
                        | SemanticOutcome::Inconclusive
                ) {
                    return Err(RuntimeError::InvalidEvent(
                        "bootstrap cannot compare progress without a prior residual state",
                    ));
                }
                validate_commit(
                    semantic_outcome,
                    frontier_id.as_ref(),
                    evidence_state,
                    disposition,
                    None,
                )?;
                next.baseline_id = Some(baseline_id.clone());
                next.committed_record_id = Some(baseline_id);
                next.frontier_id = frontier_id;
                next.semantic_outcome = Some(semantic_outcome);
                next.evidence_state = evidence_state;
                apply_disposition(&mut next, disposition);
            }
            RuntimeEvent::BeginScheduling { transition_id }
                if self.phase == RuntimePhase::Ready =>
            {
                if self.frontier_id.is_none() {
                    return Err(RuntimeError::InvalidState(
                        "ready state requires a committed frontier",
                    ));
                }
                next.phase = RuntimePhase::Scheduling;
                next.evidence_state = EvidenceState::Pending;
                next.active_transition_id = Some(transition_id);
                next.scheduling_decision_id = None;
                next.reasoning_surface_id = None;
                next.proposal_id = None;
                next.cancellation_requested = false;
                next.execution_outcome = None;
                next.observation_outcome = None;
                next.observation_ids.clear();
                next.transition_delta_ids.clear();
                next.residual_delta_ids.clear();
                next.semantic_outcome = None;
                next.pending_stop_reason = None;
                next.stop_reason = None;
            }
            RuntimeEvent::SchedulingCompleted {
                transition_id,
                decision_id,
            } if self.phase == RuntimePhase::Scheduling => {
                require_transition(self, &transition_id)?;
                next.scheduling_decision_id = Some(decision_id);
                next.phase = RuntimePhase::Orienting;
            }
            RuntimeEvent::SchedulingStopped {
                transition_id,
                decision_id,
                semantic_outcome,
                reason,
            } if self.phase == RuntimePhase::Scheduling => {
                require_transition(self, &transition_id)?;
                if reason == StopReason::Converged {
                    return Err(RuntimeError::InvalidEvent(
                        "scheduling cannot establish convergence from a ready frontier",
                    ));
                }
                if !matches!(
                    semantic_outcome,
                    SemanticOutcome::Unresolved | SemanticOutcome::Inconclusive
                ) {
                    return Err(RuntimeError::InvalidEvent(
                        "scheduling stop can report only unresolved or inconclusive work",
                    ));
                }
                next.scheduling_decision_id = Some(decision_id);
                next.semantic_outcome = Some(semantic_outcome);
                next.pending_stop_reason = Some(reason);
                next.phase = RuntimePhase::Committing;
            }
            RuntimeEvent::ReasoningSurfaceReady {
                transition_id,
                surface_id,
            } if self.phase == RuntimePhase::Orienting => {
                require_transition(self, &transition_id)?;
                next.reasoning_surface_id = Some(surface_id);
                next.phase = RuntimePhase::AwaitingProposal;
            }
            RuntimeEvent::OrientationFailed {
                transition_id,
                reason,
            } if self.phase == RuntimePhase::Orienting => {
                require_transition(self, &transition_id)?;
                if reason == StopReason::Converged {
                    return Err(RuntimeError::InvalidEvent(
                        "orientation failure cannot establish convergence",
                    ));
                }
                next.semantic_outcome = Some(SemanticOutcome::Inconclusive);
                next.pending_stop_reason = Some(reason);
                next.phase = RuntimePhase::Committing;
            }
            RuntimeEvent::ProposalReceived {
                transition_id,
                proposal_id,
            } if self.phase == RuntimePhase::AwaitingProposal => {
                require_transition(self, &transition_id)?;
                next.proposal_id = Some(proposal_id);
                next.phase = RuntimePhase::Admitting;
            }
            RuntimeEvent::ProposalAdmitted { transition_id }
                if self.phase == RuntimePhase::Admitting =>
            {
                require_transition(self, &transition_id)?;
                next.phase = RuntimePhase::Executing;
            }
            RuntimeEvent::ProposalRejected { transition_id }
                if self.phase == RuntimePhase::Admitting =>
            {
                require_transition(self, &transition_id)?;
                next.semantic_outcome = Some(SemanticOutcome::Unresolved);
                next.phase = RuntimePhase::Committing;
            }
            RuntimeEvent::CancellationRequested { transition_id }
                if self.phase == RuntimePhase::Executing =>
            {
                require_transition(self, &transition_id)?;
                if self.cancellation_requested {
                    return Err(RuntimeError::InvalidEvent(
                        "cancellation was already requested",
                    ));
                }
                next.cancellation_requested = true;
            }
            RuntimeEvent::ExecutionFinished {
                transition_id,
                outcome,
            } if self.phase == RuntimePhase::Executing => {
                require_transition(self, &transition_id)?;
                next.execution_outcome = Some(outcome);
                next.phase = RuntimePhase::Observing;
            }
            RuntimeEvent::ObservationCompleted {
                transition_id,
                outcome,
                mut observation_ids,
            } if self.phase == RuntimePhase::Observing => {
                require_transition(self, &transition_id)?;
                normalize_digests(&mut observation_ids);
                match outcome {
                    ObservationOutcome::Complete | ObservationOutcome::Partial
                        if observation_ids.is_empty() =>
                    {
                        return Err(RuntimeError::InvalidEvent(
                            "complete or partial observation requires evidence",
                        ));
                    }
                    ObservationOutcome::Unavailable | ObservationOutcome::Failed
                        if !observation_ids.is_empty() =>
                    {
                        return Err(RuntimeError::InvalidEvent(
                            "unavailable or failed observation cannot cite completed frames",
                        ));
                    }
                    _ => {}
                }
                next.observation_outcome = Some(outcome);
                next.observation_ids = observation_ids;
                next.phase = RuntimePhase::Evaluating;
            }
            RuntimeEvent::EvaluationCompleted {
                transition_id,
                mut transition_delta_ids,
                mut residual_delta_ids,
                semantic_outcome,
                next_frontier_id,
            } if self.phase == RuntimePhase::Evaluating => {
                require_transition(self, &transition_id)?;
                normalize_digests(&mut transition_delta_ids);
                normalize_digests(&mut residual_delta_ids);
                if semantic_outcome == SemanticOutcome::Converged
                    && self.observation_outcome != Some(ObservationOutcome::Complete)
                {
                    return Err(RuntimeError::InvalidEvent(
                        "convergence requires a complete post-action observation",
                    ));
                }
                if matches!(
                    self.observation_outcome,
                    Some(ObservationOutcome::Unavailable | ObservationOutcome::Failed)
                ) && semantic_outcome != SemanticOutcome::Inconclusive
                {
                    return Err(RuntimeError::InvalidEvent(
                        "unavailable or failed observation requires an inconclusive outcome",
                    ));
                }
                validate_semantic_frontier(semantic_outcome, next_frontier_id.as_ref())?;
                next.transition_delta_ids = transition_delta_ids;
                next.residual_delta_ids = residual_delta_ids;
                next.semantic_outcome = Some(semantic_outcome);
                next.frontier_id = next_frontier_id;
                next.phase = RuntimePhase::Committing;
            }
            RuntimeEvent::TransitionCommitted {
                transition_id,
                record_id,
                evidence_state,
                disposition,
            } if self.phase == RuntimePhase::Committing => {
                require_transition(self, &transition_id)?;
                let semantic_outcome = self.semantic_outcome.ok_or(RuntimeError::InvalidState(
                    "committing state requires a semantic outcome",
                ))?;
                validate_commit(
                    semantic_outcome,
                    self.frontier_id.as_ref(),
                    evidence_state,
                    disposition,
                    self.pending_stop_reason,
                )?;
                next.committed_transition_id = Some(transition_id);
                next.committed_record_id = Some(record_id);
                next.evidence_state = evidence_state;
                next.active_transition_id = None;
                next.scheduling_decision_id = None;
                next.reasoning_surface_id = None;
                next.proposal_id = None;
                next.pending_stop_reason = None;
                apply_disposition(&mut next, disposition);
            }
            _ => {
                return Err(RuntimeError::IllegalEvent {
                    phase: self.phase,
                    event: event_name,
                });
            }
        }
        next.event_sequence = self
            .event_sequence
            .checked_add(1)
            .ok_or(RuntimeError::SequenceOverflow)?;
        if next.event_sequence > self.limits.max_events {
            return Err(RuntimeError::EventLimit {
                limit: self.limits.max_events,
                observed: next.event_sequence,
            });
        }
        next.state_id = state_digest(&next);
        next.validate_shape()?;
        Ok(next)
    }

    pub fn verify(&self) -> Result<(), RuntimeError> {
        if self.schema != RUNTIME_STATE_SCHEMA {
            return Err(RuntimeError::UnsupportedSchema {
                expected: RUNTIME_STATE_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        self.validate_shape()?;
        let actual = state_digest(self);
        if actual != self.state_id {
            return Err(RuntimeError::StateDigest {
                declared: self.state_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    fn validate_shape(&self) -> Result<(), RuntimeError> {
        validate_limits(&self.limits)?;
        for digest in [
            Some(&self.state_id),
            Some(&self.trace_id),
            self.baseline_id.as_ref(),
            self.committed_transition_id.as_ref(),
            self.committed_record_id.as_ref(),
            self.frontier_id.as_ref(),
            self.active_transition_id.as_ref(),
            self.scheduling_decision_id.as_ref(),
            self.reasoning_surface_id.as_ref(),
            self.proposal_id.as_ref(),
        ]
        .into_iter()
        .flatten()
        .chain(&self.observation_ids)
        .chain(&self.transition_delta_ids)
        .chain(&self.residual_delta_ids)
        {
            validate_digest(digest)?;
        }
        if self.event_sequence > self.limits.max_events {
            return Err(RuntimeError::EventLimit {
                limit: self.limits.max_events,
                observed: self.event_sequence,
            });
        }
        validate_reference_limit(
            "observation",
            self.observation_ids.len(),
            self.limits.max_observation_refs,
        )?;
        validate_reference_limit(
            "transition delta",
            self.transition_delta_ids.len(),
            self.limits.max_transition_delta_refs,
        )?;
        validate_reference_limit(
            "residual delta",
            self.residual_delta_ids.len(),
            self.limits.max_residual_delta_refs,
        )?;
        if !is_canonical(&self.observation_ids)
            || !is_canonical(&self.transition_delta_ids)
            || !is_canonical(&self.residual_delta_ids)
        {
            return Err(RuntimeError::InvalidState(
                "runtime evidence identities must be sorted and unique",
            ));
        }
        match self.phase {
            RuntimePhase::Bootstrapping => {
                if self.event_sequence != 0
                    || self.baseline_id.is_some()
                    || self.committed_transition_id.is_some()
                    || self.committed_record_id.is_some()
                    || self.frontier_id.is_some()
                    || self.active_transition_id.is_some()
                    || self.scheduling_decision_id.is_some()
                    || self.reasoning_surface_id.is_some()
                    || self.proposal_id.is_some()
                    || self.cancellation_requested
                    || self.execution_outcome.is_some()
                    || self.observation_outcome.is_some()
                    || !self.observation_ids.is_empty()
                    || !self.transition_delta_ids.is_empty()
                    || !self.residual_delta_ids.is_empty()
                    || self.semantic_outcome.is_some()
                    || self.pending_stop_reason.is_some()
                    || self.evidence_state != EvidenceState::Pending
                    || self.stop_reason.is_some()
                {
                    return Err(RuntimeError::InvalidState(
                        "bootstrapping state contains committed or active transition data",
                    ));
                }
            }
            RuntimePhase::Ready => {
                if self.baseline_id.is_none()
                    || self.committed_record_id.is_none()
                    || self.frontier_id.is_none()
                    || self.active_transition_id.is_some()
                    || self.scheduling_decision_id.is_some()
                    || self.reasoning_surface_id.is_some()
                    || self.proposal_id.is_some()
                    || !matches!(
                        self.semantic_outcome,
                        Some(
                            SemanticOutcome::Unresolved
                                | SemanticOutcome::Progressing
                                | SemanticOutcome::Unchanged
                                | SemanticOutcome::Regressing
                        )
                    )
                    || self.pending_stop_reason.is_some()
                    || !self.evidence_state.permits_continuation()
                    || self.stop_reason.is_some()
                {
                    return Err(RuntimeError::InvalidState(
                        "ready state requires retained baseline/frontier and no active transition",
                    ));
                }
            }
            RuntimePhase::Scheduling => {
                require_active_pending(self)?;
                if self.frontier_id.is_none()
                    || self.scheduling_decision_id.is_some()
                    || self.reasoning_surface_id.is_some()
                    || self.proposal_id.is_some()
                {
                    return Err(RuntimeError::InvalidState(
                        "scheduling cannot already contain a decision, surface, or proposal",
                    ));
                }
            }
            RuntimePhase::Orienting => {
                require_scheduled_frontier(self)?;
                if self.reasoning_surface_id.is_some() || self.proposal_id.is_some() {
                    return Err(RuntimeError::InvalidState(
                        "orientation cannot already contain a surface or proposal",
                    ));
                }
            }
            RuntimePhase::AwaitingProposal => {
                require_scheduled_frontier(self)?;
                if self.reasoning_surface_id.is_none() || self.proposal_id.is_some() {
                    return Err(RuntimeError::InvalidState(
                        "awaiting-proposal state requires a surface and no proposal",
                    ));
                }
            }
            RuntimePhase::Admitting | RuntimePhase::Executing => {
                require_scheduled_frontier(self)?;
                if self.reasoning_surface_id.is_none() || self.proposal_id.is_none() {
                    return Err(RuntimeError::InvalidState(
                        "admission and execution require a surface and proposal",
                    ));
                }
            }
            RuntimePhase::Observing => {
                require_scheduled_frontier(self)?;
                if self.execution_outcome.is_none() {
                    return Err(RuntimeError::InvalidState(
                        "observing state requires a terminal provider execution outcome",
                    ));
                }
            }
            RuntimePhase::Evaluating => {
                require_scheduled_frontier(self)?;
                if self.execution_outcome.is_none() || self.observation_outcome.is_none() {
                    return Err(RuntimeError::InvalidState(
                        "evaluating state requires execution and observation outcomes",
                    ));
                }
            }
            RuntimePhase::Committing => {
                require_scheduled(self)?;
                if self.semantic_outcome.is_none() {
                    return Err(RuntimeError::InvalidState(
                        "committing state requires a semantic outcome",
                    ));
                }
            }
            RuntimePhase::Stopped => {
                if self.baseline_id.is_none()
                    || self.active_transition_id.is_some()
                    || self.scheduling_decision_id.is_some()
                    || self.stop_reason.is_none()
                    || self.evidence_state == EvidenceState::Pending
                {
                    return Err(RuntimeError::InvalidState(
                        "stopped state requires committed evidence and a stop reason",
                    ));
                }
            }
        }
        Ok(())
    }
}

fn require_active_pending(state: &RuntimeState) -> Result<(), RuntimeError> {
    if state.active_transition_id.is_none() || state.evidence_state != EvidenceState::Pending {
        return Err(RuntimeError::InvalidState(
            "active phase requires a transition with pending evidence",
        ));
    }
    Ok(())
}

fn require_scheduled(state: &RuntimeState) -> Result<(), RuntimeError> {
    require_active_pending(state)?;
    if state.scheduling_decision_id.is_none() {
        return Err(RuntimeError::InvalidState(
            "post-scheduling phase requires a scheduling decision",
        ));
    }
    Ok(())
}

fn require_scheduled_frontier(state: &RuntimeState) -> Result<(), RuntimeError> {
    require_scheduled(state)?;
    if state.frontier_id.is_none() {
        return Err(RuntimeError::InvalidState(
            "pre-evaluation phase requires the committed frontier",
        ));
    }
    Ok(())
}

fn validate_limits(limits: &RuntimeLimits) -> Result<(), RuntimeError> {
    let values = [
        ("max_events", limits.max_events),
        ("max_observation_refs", limits.max_observation_refs),
        (
            "max_transition_delta_refs",
            limits.max_transition_delta_refs,
        ),
        ("max_residual_delta_refs", limits.max_residual_delta_refs),
    ];
    if let Some((name, _)) = values.into_iter().find(|(_, value)| *value == 0) {
        return Err(RuntimeError::ZeroLimit(name));
    }
    Ok(())
}

fn validate_reference_limit(
    kind: &'static str,
    observed: usize,
    limit: u64,
) -> Result<(), RuntimeError> {
    if observed as u64 > limit {
        return Err(RuntimeError::ReferenceLimit {
            kind,
            limit,
            observed: observed as u64,
        });
    }
    Ok(())
}

fn require_transition(
    state: &RuntimeState,
    transition_id: &SemanticDigest,
) -> Result<(), RuntimeError> {
    let expected = state
        .active_transition_id
        .as_ref()
        .ok_or(RuntimeError::InvalidState("active transition is missing"))?;
    if expected != transition_id {
        return Err(RuntimeError::TransitionIdentity {
            expected: expected.clone(),
            actual: transition_id.clone(),
        });
    }
    Ok(())
}

fn validate_semantic_frontier(
    outcome: SemanticOutcome,
    frontier: Option<&SemanticDigest>,
) -> Result<(), RuntimeError> {
    match outcome {
        SemanticOutcome::Converged if frontier.is_some() => Err(RuntimeError::InvalidEvent(
            "converged evaluation cannot retain a next frontier",
        )),
        SemanticOutcome::Unresolved
        | SemanticOutcome::Progressing
        | SemanticOutcome::Unchanged
        | SemanticOutcome::Regressing
            if frontier.is_none() =>
        {
            Err(RuntimeError::InvalidEvent(
                "nonterminal semantic outcome requires a next frontier",
            ))
        }
        _ => Ok(()),
    }
}

fn validate_commit(
    semantic_outcome: SemanticOutcome,
    frontier: Option<&SemanticDigest>,
    evidence_state: EvidenceState,
    disposition: CommitDisposition,
    pending_stop_reason: Option<StopReason>,
) -> Result<(), RuntimeError> {
    validate_semantic_frontier(semantic_outcome, frontier)?;
    match disposition {
        CommitDisposition::Continue => {
            if pending_stop_reason.is_some() {
                return Err(RuntimeError::InvalidEvent(
                    "a pending stop condition cannot continue",
                ));
            }
            if matches!(
                semantic_outcome,
                SemanticOutcome::Converged | SemanticOutcome::Inconclusive
            ) {
                return Err(RuntimeError::InvalidEvent(
                    "converged or inconclusive outcome must stop explicitly",
                ));
            }
            if !evidence_state.permits_continuation() {
                return Err(RuntimeError::InvalidEvent(
                    "the next action requires retained transition evidence",
                ));
            }
        }
        CommitDisposition::Stop { reason } => {
            if let Some(expected) = pending_stop_reason
                && expected != reason
            {
                return Err(RuntimeError::InvalidEvent(
                    "committed stop reason does not match the pending stop condition",
                ));
            }
            if (semantic_outcome == SemanticOutcome::Converged) != (reason == StopReason::Converged)
            {
                return Err(RuntimeError::InvalidEvent(
                    "convergence outcome and stop reason must agree",
                ));
            }
            if reason == StopReason::EvidenceMissing && evidence_state != EvidenceState::Missing {
                return Err(RuntimeError::InvalidEvent(
                    "evidence-missing stop requires missing evidence state",
                ));
            }
            if reason == StopReason::Converged && !evidence_state.permits_continuation() {
                return Err(RuntimeError::InvalidEvent(
                    "convergence requires retained or verified evidence",
                ));
            }
            if evidence_state == EvidenceState::Pending {
                return Err(RuntimeError::InvalidEvent(
                    "a stopped transition cannot retain pending evidence state",
                ));
            }
        }
    }
    Ok(())
}

fn apply_disposition(state: &mut RuntimeState, disposition: CommitDisposition) {
    match disposition {
        CommitDisposition::Continue => {
            state.phase = RuntimePhase::Ready;
            state.stop_reason = None;
        }
        CommitDisposition::Stop { reason } => {
            state.phase = RuntimePhase::Stopped;
            state.stop_reason = Some(reason);
        }
    }
}

fn normalize_digests(values: &mut Vec<SemanticDigest>) {
    values.sort();
    values.dedup();
}

fn is_canonical(values: &[SemanticDigest]) -> bool {
    values.windows(2).all(|window| window[0] < window[1])
}

fn validate_digest(digest: &SemanticDigest) -> Result<(), RuntimeError> {
    let value = digest.as_str();
    let Some(hex) = value.strip_prefix("blake3:") else {
        return Err(RuntimeError::InvalidDigest(value.to_owned()));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(RuntimeError::InvalidDigest(value.to_owned()));
    }
    Ok(())
}

fn placeholder_digest() -> SemanticDigest {
    SemanticHasher::new("rey.runtime-state.placeholder").finish()
}

fn state_digest(state: &RuntimeState) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(RUNTIME_STATE_SCHEMA);
    hasher.add_str(state.trace_id.as_str());
    hasher.add_u64(state.event_sequence);
    hasher.add_u64(state.limits.max_events);
    hasher.add_u64(state.limits.max_observation_refs);
    hasher.add_u64(state.limits.max_transition_delta_refs);
    hasher.add_u64(state.limits.max_residual_delta_refs);
    hasher.add_str(state.phase.as_str());
    hasher.add_str(state.retention_profile.as_str());
    hasher.add_str(state.evidence_state.as_str());
    add_optional_digest(&mut hasher, state.baseline_id.as_ref());
    add_optional_digest(&mut hasher, state.committed_transition_id.as_ref());
    add_optional_digest(&mut hasher, state.committed_record_id.as_ref());
    add_optional_digest(&mut hasher, state.frontier_id.as_ref());
    add_optional_digest(&mut hasher, state.active_transition_id.as_ref());
    add_optional_digest(&mut hasher, state.scheduling_decision_id.as_ref());
    add_optional_digest(&mut hasher, state.reasoning_surface_id.as_ref());
    add_optional_digest(&mut hasher, state.proposal_id.as_ref());
    hasher.add_bool(state.cancellation_requested);
    hasher.add_optional_str(state.execution_outcome.map(ExecutionOutcome::as_str));
    hasher.add_optional_str(state.observation_outcome.map(ObservationOutcome::as_str));
    add_digests(&mut hasher, &state.observation_ids);
    add_digests(&mut hasher, &state.transition_delta_ids);
    add_digests(&mut hasher, &state.residual_delta_ids);
    hasher.add_optional_str(state.semantic_outcome.map(SemanticOutcome::as_str));
    hasher.add_optional_str(state.pending_stop_reason.map(StopReason::as_str));
    hasher.add_optional_str(state.stop_reason.map(StopReason::as_str));
    hasher.finish()
}

fn add_optional_digest(hasher: &mut SemanticHasher, value: Option<&SemanticDigest>) {
    hasher.add_optional_str(value.map(SemanticDigest::as_str));
}

fn add_digests(hasher: &mut SemanticHasher, values: &[SemanticDigest]) {
    hasher.add_u64(values.len() as u64);
    for value in values {
        hasher.add_str(value.as_str());
    }
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("unsupported runtime state schema {actual}; expected {expected}")]
    UnsupportedSchema {
        expected: &'static str,
        actual: String,
    },
    #[error("runtime state digest mismatch: declared {declared}, actual {actual}")]
    StateDigest {
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("event {event} is not legal from runtime phase {phase:?}")]
    IllegalEvent {
        phase: RuntimePhase,
        event: &'static str,
    },
    #[error("transition identity mismatch: expected {expected}, actual {actual}")]
    TransitionIdentity {
        expected: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("invalid runtime state: {0}")]
    InvalidState(&'static str),
    #[error("invalid runtime event: {0}")]
    InvalidEvent(&'static str),
    #[error("runtime event sequence overflowed")]
    SequenceOverflow,
    #[error("invalid semantic digest {0}")]
    InvalidDigest(String),
    #[error("runtime limit {0} must be greater than zero")]
    ZeroLimit(&'static str),
    #[error("runtime event limit {limit} exceeded by {observed}")]
    EventLimit { limit: u64, observed: u64 },
    #[error("runtime {kind} reference limit {limit} exceeded by {observed}")]
    ReferenceLimit {
        kind: &'static str,
        limit: u64,
        observed: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: &str) -> SemanticDigest {
        let mut hasher = SemanticHasher::new("rey.runtime-test.v1");
        hasher.add_str(value);
        hasher.finish()
    }

    fn ready_state() -> RuntimeState {
        RuntimeState::bootstrap(
            digest("trace"),
            RetentionProfile::Local,
            RuntimeLimits::default(),
        )
        .unwrap()
        .apply(RuntimeEvent::BootstrapCommitted {
            baseline_id: digest("baseline"),
            frontier_id: Some(digest("frontier-0")),
            semantic_outcome: SemanticOutcome::Unresolved,
            evidence_state: EvidenceState::Retained,
            disposition: CommitDisposition::Continue,
        })
        .unwrap()
    }

    fn scheduled_state(state: RuntimeState, transition_id: &SemanticDigest) -> RuntimeState {
        state
            .apply(RuntimeEvent::BeginScheduling {
                transition_id: transition_id.clone(),
            })
            .unwrap()
            .apply(RuntimeEvent::SchedulingCompleted {
                transition_id: transition_id.clone(),
                decision_id: digest("scheduling-decision"),
            })
            .unwrap()
    }

    #[test]
    fn complete_transition_reaches_the_next_committed_frontier() {
        let transition_id = digest("transition-1");
        let state = scheduled_state(ready_state(), &transition_id)
            .apply(RuntimeEvent::ReasoningSurfaceReady {
                transition_id: transition_id.clone(),
                surface_id: digest("surface"),
            })
            .unwrap()
            .apply(RuntimeEvent::ProposalReceived {
                transition_id: transition_id.clone(),
                proposal_id: digest("proposal"),
            })
            .unwrap()
            .apply(RuntimeEvent::ProposalAdmitted {
                transition_id: transition_id.clone(),
            })
            .unwrap()
            .apply(RuntimeEvent::ExecutionFinished {
                transition_id: transition_id.clone(),
                outcome: ExecutionOutcome::Succeeded,
            })
            .unwrap()
            .apply(RuntimeEvent::ObservationCompleted {
                transition_id: transition_id.clone(),
                outcome: ObservationOutcome::Complete,
                observation_ids: vec![digest("observation")],
            })
            .unwrap()
            .apply(RuntimeEvent::EvaluationCompleted {
                transition_id: transition_id.clone(),
                transition_delta_ids: vec![digest("transition-delta")],
                residual_delta_ids: vec![digest("residual-delta")],
                semantic_outcome: SemanticOutcome::Progressing,
                next_frontier_id: Some(digest("frontier-1")),
            })
            .unwrap()
            .apply(RuntimeEvent::TransitionCommitted {
                transition_id: transition_id.clone(),
                record_id: digest("record"),
                evidence_state: EvidenceState::Retained,
                disposition: CommitDisposition::Continue,
            })
            .unwrap();

        assert_eq!(state.phase, RuntimePhase::Ready);
        assert_eq!(state.committed_transition_id, Some(transition_id));
        assert_eq!(state.frontier_id, Some(digest("frontier-1")));
        assert_eq!(state.semantic_outcome, Some(SemanticOutcome::Progressing));
        state.verify().unwrap();
    }

    #[test]
    fn process_success_does_not_skip_observation_and_evaluation() {
        let transition_id = digest("transition");
        let state = scheduled_state(ready_state(), &transition_id)
            .apply(RuntimeEvent::ReasoningSurfaceReady {
                transition_id: transition_id.clone(),
                surface_id: digest("surface"),
            })
            .unwrap()
            .apply(RuntimeEvent::ProposalReceived {
                transition_id: transition_id.clone(),
                proposal_id: digest("proposal"),
            })
            .unwrap()
            .apply(RuntimeEvent::ProposalAdmitted {
                transition_id: transition_id.clone(),
            })
            .unwrap()
            .apply(RuntimeEvent::ExecutionFinished {
                transition_id: transition_id.clone(),
                outcome: ExecutionOutcome::Succeeded,
            })
            .unwrap();

        assert_eq!(state.phase, RuntimePhase::Observing);
        assert!(matches!(
            state.apply(RuntimeEvent::TransitionCommitted {
                transition_id,
                record_id: digest("record"),
                evidence_state: EvidenceState::Retained,
                disposition: CommitDisposition::Continue,
            }),
            Err(RuntimeError::IllegalEvent { .. })
        ));
    }

    #[test]
    fn cancellation_still_requires_a_terminal_execution_and_observation() {
        let transition_id = digest("transition");
        let state = scheduled_state(ready_state(), &transition_id)
            .apply(RuntimeEvent::ReasoningSurfaceReady {
                transition_id: transition_id.clone(),
                surface_id: digest("surface"),
            })
            .unwrap()
            .apply(RuntimeEvent::ProposalReceived {
                transition_id: transition_id.clone(),
                proposal_id: digest("proposal"),
            })
            .unwrap()
            .apply(RuntimeEvent::ProposalAdmitted {
                transition_id: transition_id.clone(),
            })
            .unwrap()
            .apply(RuntimeEvent::CancellationRequested {
                transition_id: transition_id.clone(),
            })
            .unwrap();

        assert_eq!(state.phase, RuntimePhase::Executing);
        assert!(state.cancellation_requested);
        let state = state
            .apply(RuntimeEvent::ExecutionFinished {
                transition_id: transition_id.clone(),
                outcome: ExecutionOutcome::Cancelled,
            })
            .unwrap()
            .apply(RuntimeEvent::ObservationCompleted {
                transition_id,
                outcome: ObservationOutcome::Unavailable,
                observation_ids: Vec::new(),
            })
            .unwrap();
        assert_eq!(state.phase, RuntimePhase::Evaluating);
        assert!(matches!(
            state.apply(RuntimeEvent::EvaluationCompleted {
                transition_id: digest("transition"),
                transition_delta_ids: Vec::new(),
                residual_delta_ids: Vec::new(),
                semantic_outcome: SemanticOutcome::Progressing,
                next_frontier_id: Some(digest("unexpected-frontier")),
            }),
            Err(RuntimeError::InvalidEvent(_))
        ));
    }

    #[test]
    fn converged_evaluation_stops_only_after_commit() {
        let transition_id = digest("transition");
        let state = scheduled_state(ready_state(), &transition_id)
            .apply(RuntimeEvent::ReasoningSurfaceReady {
                transition_id: transition_id.clone(),
                surface_id: digest("surface"),
            })
            .unwrap()
            .apply(RuntimeEvent::ProposalReceived {
                transition_id: transition_id.clone(),
                proposal_id: digest("proposal"),
            })
            .unwrap()
            .apply(RuntimeEvent::ProposalAdmitted {
                transition_id: transition_id.clone(),
            })
            .unwrap()
            .apply(RuntimeEvent::ExecutionFinished {
                transition_id: transition_id.clone(),
                outcome: ExecutionOutcome::Succeeded,
            })
            .unwrap()
            .apply(RuntimeEvent::ObservationCompleted {
                transition_id: transition_id.clone(),
                outcome: ObservationOutcome::Complete,
                observation_ids: vec![digest("observation")],
            })
            .unwrap()
            .apply(RuntimeEvent::EvaluationCompleted {
                transition_id: transition_id.clone(),
                transition_delta_ids: vec![digest("transition-delta")],
                residual_delta_ids: Vec::new(),
                semantic_outcome: SemanticOutcome::Converged,
                next_frontier_id: None,
            })
            .unwrap();
        assert_eq!(state.phase, RuntimePhase::Committing);

        assert!(matches!(
            state.apply(RuntimeEvent::TransitionCommitted {
                transition_id: transition_id.clone(),
                record_id: digest("missing-record"),
                evidence_state: EvidenceState::Missing,
                disposition: CommitDisposition::Stop {
                    reason: StopReason::Converged,
                },
            }),
            Err(RuntimeError::InvalidEvent(_))
        ));

        let state = state
            .apply(RuntimeEvent::TransitionCommitted {
                transition_id,
                record_id: digest("record"),
                evidence_state: EvidenceState::Retained,
                disposition: CommitDisposition::Stop {
                    reason: StopReason::Converged,
                },
            })
            .unwrap();
        assert_eq!(state.phase, RuntimePhase::Stopped);
        assert_eq!(state.stop_reason, Some(StopReason::Converged));
    }

    #[test]
    fn missing_evidence_cannot_continue() {
        let transition_id = digest("transition");
        let state = scheduled_state(ready_state(), &transition_id)
            .apply(RuntimeEvent::OrientationFailed {
                transition_id: transition_id.clone(),
                reason: StopReason::EvidenceMissing,
            })
            .unwrap();

        assert!(matches!(
            state.apply(RuntimeEvent::TransitionCommitted {
                transition_id,
                record_id: digest("record"),
                evidence_state: EvidenceState::Missing,
                disposition: CommitDisposition::Continue,
            }),
            Err(RuntimeError::InvalidEvent(_))
        ));
    }

    #[test]
    fn mismatched_transition_identity_is_rejected() {
        let expected = digest("expected");
        let state = ready_state()
            .apply(RuntimeEvent::BeginScheduling {
                transition_id: expected,
            })
            .unwrap();
        assert!(matches!(
            state.apply(RuntimeEvent::SchedulingCompleted {
                transition_id: digest("other"),
                decision_id: digest("decision"),
            }),
            Err(RuntimeError::TransitionIdentity { .. })
        ));
    }

    #[test]
    fn orientation_requires_a_recorded_scheduling_decision() {
        let transition_id = digest("transition");
        let state = ready_state()
            .apply(RuntimeEvent::BeginScheduling {
                transition_id: transition_id.clone(),
            })
            .unwrap();
        assert_eq!(state.phase, RuntimePhase::Scheduling);
        assert!(matches!(
            state.apply(RuntimeEvent::ReasoningSurfaceReady {
                transition_id: transition_id.clone(),
                surface_id: digest("surface"),
            }),
            Err(RuntimeError::IllegalEvent { .. })
        ));

        let decision_id = digest("decision");
        let state = state
            .apply(RuntimeEvent::SchedulingCompleted {
                transition_id,
                decision_id: decision_id.clone(),
            })
            .unwrap();
        assert_eq!(state.phase, RuntimePhase::Orienting);
        assert_eq!(state.scheduling_decision_id, Some(decision_id));
    }

    #[test]
    fn scheduling_stop_preserves_unresolved_work_and_commits_explicitly() {
        let transition_id = digest("transition");
        let state = ready_state()
            .apply(RuntimeEvent::BeginScheduling {
                transition_id: transition_id.clone(),
            })
            .unwrap();
        assert!(matches!(
            state.apply(RuntimeEvent::SchedulingStopped {
                transition_id: transition_id.clone(),
                decision_id: digest("bad-decision"),
                semantic_outcome: SemanticOutcome::Converged,
                reason: StopReason::Converged,
            }),
            Err(RuntimeError::InvalidEvent(_))
        ));

        let state = state
            .apply(RuntimeEvent::SchedulingStopped {
                transition_id: transition_id.clone(),
                decision_id: digest("budget-decision"),
                semantic_outcome: SemanticOutcome::Unresolved,
                reason: StopReason::BudgetExhausted,
            })
            .unwrap();
        assert_eq!(state.phase, RuntimePhase::Committing);
        assert_eq!(state.frontier_id, Some(digest("frontier-0")));

        let state = state
            .apply(RuntimeEvent::TransitionCommitted {
                transition_id,
                record_id: digest("budget-record"),
                evidence_state: EvidenceState::Retained,
                disposition: CommitDisposition::Stop {
                    reason: StopReason::BudgetExhausted,
                },
            })
            .unwrap();
        assert_eq!(state.phase, RuntimePhase::Stopped);
        assert_eq!(state.semantic_outcome, Some(SemanticOutcome::Unresolved));
        assert_eq!(state.stop_reason, Some(StopReason::BudgetExhausted));
        assert!(state.scheduling_decision_id.is_none());
    }

    #[test]
    fn state_digest_detects_tampering() {
        let mut state = ready_state();
        state.frontier_id = Some(digest("tampered"));
        assert!(matches!(
            state.verify(),
            Err(RuntimeError::StateDigest { .. })
        ));
    }

    #[test]
    fn bootstrap_cannot_fabricate_progress() {
        let state = RuntimeState::bootstrap(
            digest("trace"),
            RetentionProfile::Local,
            RuntimeLimits::default(),
        )
        .unwrap();
        assert!(matches!(
            state.apply(RuntimeEvent::BootstrapCommitted {
                baseline_id: digest("baseline"),
                frontier_id: Some(digest("frontier")),
                semantic_outcome: SemanticOutcome::Progressing,
                evidence_state: EvidenceState::Retained,
                disposition: CommitDisposition::Continue,
            }),
            Err(RuntimeError::InvalidEvent(_))
        ));
    }

    #[test]
    fn malformed_reference_digest_is_rejected() {
        let mut state = ready_state();
        state.frontier_id =
            Some(serde_json::from_str::<SemanticDigest>("\"not-a-semantic-digest\"").unwrap());
        state.state_id = state_digest(&state);
        assert!(matches!(
            state.verify(),
            Err(RuntimeError::InvalidDigest(_))
        ));
    }

    #[test]
    fn state_json_round_trip_preserves_replay_identity() {
        let state = ready_state();
        let encoded = serde_json::to_vec(&state).unwrap();
        let decoded: RuntimeState = serde_json::from_slice(&encoded).unwrap();
        decoded.verify().unwrap();
        assert_eq!(decoded, state);
        assert_eq!(ready_state().state_id, state.state_id);
    }

    #[test]
    fn event_and_reference_bounds_fail_closed() {
        let limits = RuntimeLimits {
            max_events: 1,
            ..RuntimeLimits::default()
        };
        let state = RuntimeState::bootstrap(digest("trace"), RetentionProfile::Local, limits)
            .unwrap()
            .apply(RuntimeEvent::BootstrapCommitted {
                baseline_id: digest("baseline"),
                frontier_id: Some(digest("frontier")),
                semantic_outcome: SemanticOutcome::Unresolved,
                evidence_state: EvidenceState::Retained,
                disposition: CommitDisposition::Continue,
            })
            .unwrap();
        assert!(matches!(
            state.apply(RuntimeEvent::BeginScheduling {
                transition_id: digest("transition"),
            }),
            Err(RuntimeError::EventLimit { .. })
        ));

        let limits = RuntimeLimits {
            max_observation_refs: 1,
            ..RuntimeLimits::default()
        };
        let transition_id = digest("bounded-transition");
        let state =
            RuntimeState::bootstrap(digest("bounded-trace"), RetentionProfile::Local, limits)
                .unwrap()
                .apply(RuntimeEvent::BootstrapCommitted {
                    baseline_id: digest("bounded-baseline"),
                    frontier_id: Some(digest("bounded-frontier")),
                    semantic_outcome: SemanticOutcome::Unresolved,
                    evidence_state: EvidenceState::Retained,
                    disposition: CommitDisposition::Continue,
                })
                .unwrap();
        let state = scheduled_state(state, &transition_id)
            .apply(RuntimeEvent::ReasoningSurfaceReady {
                transition_id: transition_id.clone(),
                surface_id: digest("bounded-surface"),
            })
            .unwrap()
            .apply(RuntimeEvent::ProposalReceived {
                transition_id: transition_id.clone(),
                proposal_id: digest("bounded-proposal"),
            })
            .unwrap()
            .apply(RuntimeEvent::ProposalAdmitted {
                transition_id: transition_id.clone(),
            })
            .unwrap()
            .apply(RuntimeEvent::ExecutionFinished {
                transition_id: transition_id.clone(),
                outcome: ExecutionOutcome::Succeeded,
            })
            .unwrap();
        assert!(matches!(
            state.apply(RuntimeEvent::ObservationCompleted {
                transition_id,
                outcome: ObservationOutcome::Complete,
                observation_ids: vec![digest("observation-1"), digest("observation-2")],
            }),
            Err(RuntimeError::ReferenceLimit { .. })
        ));
    }
}
