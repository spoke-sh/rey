use std::{collections::BTreeMap, path::PathBuf};

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_diff::{
    DeltaAssessment, ExpectedSourceMatch, ScenarioOutputDelta, SourceMatchDelta,
    SourceMatchDeltaInputs, SourceMatchDeltaLimits, compare_source_matches,
    source_match_comparator,
};
use rey_environment::{
    LocalSourceCorpus, SourceBindingLimits, SourceCorpusBinding, SourceSearchEvidence,
    builtin_source_search_operation, local_source_provider,
};
use rey_frontier::{
    Frontier, FrontierCoverage, FrontierInputs, FrontierLimits, FrontierRowInput, Readiness,
    RequiredClaims, SchedulerLimits, SchedulingDecision, SchedulingPreconditions, schedule,
};
use rey_mining::{
    MiningLimits, MiningParameterValue, MiningRationaleKind, MiningRequest, MiningRequestContext,
};
use rey_policy::{
    EvidenceReference, ReasoningSurface, ReasoningSurfaceInputs, ReasoningSurfaceLimits,
    ReasoningSurfaceRow, SurfaceCompleteness,
};
use serde::{Deserialize, Serialize};

use crate::WorkloadError;

pub const BUILT_IN_SOURCE_SEARCH_WORKLOAD_ID: &str = "rey.fixture.source-search";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceSearchScenario {
    pub fixture_paths: Vec<String>,
    pub context_before: u64,
    pub context_after: u64,
    pub binding_limits: SourceBindingLimits,
    pub mining_limits: MiningLimits,
    pub expected_matches: Vec<ExpectedSourceMatch>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceMiningExecution {
    pub corpus: SourceCorpusBinding,
    pub request: MiningRequest,
    pub evidence: SourceSearchEvidence,
}

impl SourceMiningExecution {
    pub fn verify(&self) -> Result<(), WorkloadError> {
        self.corpus.verify()?;
        self.request
            .verify_against(&builtin_source_search_operation())?;
        self.evidence.verify_detached(&self.corpus, &self.request)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MiningReasoningEvidence {
    pub frontier: Frontier,
    pub scheduling: SchedulingDecision,
    pub surface: ReasoningSurface,
}

impl MiningReasoningEvidence {
    pub fn verify(&self) -> Result<(), WorkloadError> {
        self.frontier.verify()?;
        self.scheduling.verify_against(&self.frontier)?;
        self.surface.verify()?;
        if self.scheduling.inputs.frontier_id != self.frontier.frontier_id
            || self.surface.inputs.scheduling_decision_id != self.scheduling.decision_id
            || self.surface.inputs.frontier_frame_id != self.frontier.frontier_id
            || self.surface.inputs.workload != self.frontier.inputs.workload
            || self.surface.inputs.graph != self.frontier.inputs.graph
            || self.surface.inputs.scenario_suite != self.frontier.inputs.scenario_suite
            || self.surface.inputs.campaign_id != self.frontier.inputs.campaign_id
            || self.surface.inputs.space != self.frontier.inputs.space
            || self.surface.inputs.trace_id != self.frontier.inputs.trace_id
            || self.surface.inputs.committed_transition_id
                != self.frontier.inputs.committed_record_id
            || self.surface.inputs.capability_snapshot_id
                != self.frontier.inputs.capability_snapshot_id
        {
            return Err(WorkloadError::ResultShape(
                "mining reasoning evidence is not internally bound",
            ));
        }
        if self.surface.rows.len() != self.scheduling.selected.len()
            || self.surface.rows.iter().any(|surface_row| {
                let Some(selected) = self.scheduling.selected.iter().find(|selected| {
                    selected.frontier_row_id.as_str() == surface_row.frontier_row_id
                }) else {
                    return true;
                };
                let Some(frontier_row) = self
                    .frontier
                    .rows
                    .iter()
                    .find(|row| row.row_id == selected.frontier_row_id)
                else {
                    return true;
                };
                surface_row.entity_kind != frontier_row.entity_kind
                    || surface_row.entity_id != frontier_row.entity_id
                    || surface_row.transition_delta_ids != frontier_row.transition_delta_ids
                    || surface_row.residual_delta_ids != frontier_row.residual_delta_ids
                    || surface_row.claim_ids != frontier_row.claim_ids
                    || surface_row.admissible_action_ids != frontier_row.admissible_action_ids
            })
        {
            return Err(WorkloadError::ResultShape(
                "mining reasoning surface does not reproduce scheduled frontier rows",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MiningScenarioEvidence {
    pub execution: SourceMiningExecution,
    pub relation_delta: SourceMatchDelta,
    pub reasoning: Option<MiningReasoningEvidence>,
}

impl MiningScenarioEvidence {
    pub fn verify(&self) -> Result<(), WorkloadError> {
        self.execution.verify()?;
        self.relation_delta.verify(&self.execution.evidence)?;
        if let Some(reasoning) = &self.reasoning {
            reasoning.verify()?;
            if !reasoning.frontier.rows.iter().any(|row| {
                row.residual_delta_ids
                    .contains(&self.relation_delta.delta_id)
            }) {
                return Err(WorkloadError::ResultShape(
                    "reasoning surface does not cite the mining delta",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct SourceRunInput {
    pub root: PathBuf,
    pub relative_paths: Vec<PathBuf>,
    pub context_before: u64,
    pub context_after: u64,
    pub binding_limits: SourceBindingLimits,
    pub mining_limits: MiningLimits,
    pub capability_snapshot_id: SemanticDigest,
}

pub(crate) struct SourceExecutionContext<'a> {
    pub workload: &'a ContractIdentity,
    pub graph: &'a ContractIdentity,
    pub scenario: Option<&'a ContractIdentity>,
    pub campaign_id: Option<&'a SemanticDigest>,
    pub graph_node_id: &'a str,
    pub pattern: &'a str,
    pub input: &'a SourceRunInput,
}

pub(crate) fn execute_source_search(
    context: SourceExecutionContext<'_>,
) -> Result<SourceMiningExecution, WorkloadError> {
    let corpus = LocalSourceCorpus::bind(
        &context.input.root,
        context.input.relative_paths.clone(),
        context.input.binding_limits.clone(),
    )?;
    let operation = builtin_source_search_operation();
    let request = MiningRequest::new(
        MiningRequestContext {
            workload: context.workload.clone(),
            graph: context.graph.clone(),
            scenario: context.scenario.cloned(),
            campaign_id: context.campaign_id.cloned(),
            space: local_source_space(),
            active_transition_id: None,
            graph_node_id: context.graph_node_id.to_owned(),
            rationale: MiningRationaleKind::WorkloadGraph,
            frontier_row_ids: Vec::new(),
            delta_ids: Vec::new(),
        },
        &operation,
        local_source_provider(),
        context.input.capability_snapshot_id.clone(),
        vec![corpus.binding().artifact_ref()],
        BTreeMap::from([
            (
                "pattern".to_owned(),
                MiningParameterValue::Utf8(context.pattern.to_owned()),
            ),
            (
                "context_before".to_owned(),
                MiningParameterValue::U64(context.input.context_before),
            ),
            (
                "context_after".to_owned(),
                MiningParameterValue::U64(context.input.context_after),
            ),
        ]),
        MiningLimits::default(),
        context.input.mining_limits.clone(),
    )?;
    let evidence = corpus.search(&request)?;
    let execution = SourceMiningExecution {
        corpus: corpus.binding().clone(),
        request,
        evidence,
    };
    execution.verify()?;
    Ok(execution)
}

pub(crate) fn compare_execution_matches(
    workload: &ContractIdentity,
    graph: &ContractIdentity,
    scenario: &ContractIdentity,
    expected: Vec<ExpectedSourceMatch>,
    execution: &SourceMiningExecution,
) -> Result<SourceMatchDelta, WorkloadError> {
    Ok(compare_source_matches(
        SourceMatchDeltaInputs {
            workload: workload.clone(),
            graph: graph.clone(),
            scenario: scenario.clone(),
            comparator: source_match_comparator(),
            binding_id: execution.corpus.binding_id.clone(),
            mining_request_id: execution.request.request_id.clone(),
            mining_result_id: execution.evidence.result.result_id.clone(),
        },
        expected,
        &execution.evidence,
        SourceMatchDeltaLimits::default(),
    )?)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_reasoning_evidence(
    workload: &ContractIdentity,
    graph: &ContractIdentity,
    scenario_suite: &ContractIdentity,
    campaign_id: &SemanticDigest,
    scenario: &ContractIdentity,
    execution_id: &SemanticDigest,
    text_deltas: &[ScenarioOutputDelta],
    mining: &MiningScenarioEvidence,
) -> Result<Option<MiningReasoningEvidence>, WorkloadError> {
    if mining.relation_delta.assessment != DeltaAssessment::Different {
        return Ok(None);
    }
    let trace_id = derived_digest(
        "rey.mining-failure-trace.v1",
        &[campaign_id, execution_id, &mining.relation_delta.delta_id],
    );
    let derivation = ContractIdentity::new(
        "rey.workload.mining-failure-frontier",
        1,
        "derive one ready graph-revision work row from complete failing source relation and ordered text deltas",
    );
    let prioritization = ContractIdentity::new(
        "rey.workload.mining-failure-priority",
        1,
        "rank the single reviewed mining conformance gap at priority 100 and cost 1",
    );
    let action = ContractIdentity::new(
        "rey.action.propose-graph-revision",
        1,
        "propose a new immutable compute graph revision against cited failing evidence",
    );
    let mut residual_delta_ids = vec![mining.relation_delta.delta_id.clone()];
    residual_delta_ids.extend(text_deltas.iter().map(|delta| delta.delta_id.clone()));
    let frontier = Frontier::new(
        FrontierInputs {
            workload: workload.clone(),
            graph: graph.clone(),
            scenario_suite: scenario_suite.clone(),
            campaign_id: campaign_id.clone(),
            space: local_source_space(),
            trace_id: trace_id.clone(),
            committed_record_id: execution_id.clone(),
            capability_snapshot_id: mining.execution.request.capability_snapshot_id.clone(),
            derivation,
            prioritization,
        },
        FrontierLimits::default(),
        FrontierCoverage {
            deltas_complete: true,
            claims_complete: true,
            required_claims: RequiredClaims::Violated,
        },
        vec![FrontierRowInput {
            work_id: format!("{}.revise-graph", scenario.id),
            entity_kind: "workload_scenario".to_owned(),
            entity_id: scenario.id.clone(),
            transition_delta_ids: Vec::new(),
            residual_delta_ids,
            claim_ids: vec![format!("{}.expected-output", scenario.id)],
            dependent_lens_ids: vec!["rey.source-matches.v1".to_owned()],
            admissible_action_ids: vec![action.id.clone()],
            readiness: Readiness::Ready,
            blockers: Vec::new(),
            priority: 100,
            estimated_cost_units: 1,
        }],
    )?;
    let scheduling = schedule(
        &frontier,
        SchedulingPreconditions {
            expected_committed_record_id: execution_id.clone(),
            expected_frontier_id: frontier.frontier_id.clone(),
            expected_capability_snapshot_id: mining
                .execution
                .request
                .capability_snapshot_id
                .clone(),
        },
        SchedulerLimits {
            max_work_units: 1,
            max_total_cost_units: 1,
            ..SchedulerLimits::default()
        },
    )?;
    let frontier_row = frontier.rows.first().ok_or(WorkloadError::ResultShape(
        "failing mining frontier has no work row",
    ))?;
    let mut evidence = Vec::new();
    let mining_result = &mining.execution.evidence.result;
    evidence.push(EvidenceReference {
        evidence_id: mining_result.result_id.to_string(),
        provider: mining_result.provider.clone(),
        source_id: format!("rey-mining://{}", mining_result.result_id),
        source_revision: mining_result.result_id.to_string(),
        semantic_digest: mining_result.result_id.clone(),
        media_type: "application/vnd.rey.mining-result+json".to_owned(),
        byte_length: serde_json::to_vec(mining_result)?.len() as u64,
    });
    if let Some(artifact) = &mining.execution.evidence.match_artifact {
        evidence.push(EvidenceReference {
            evidence_id: artifact.artifact_id.to_string(),
            provider: artifact.provider.clone(),
            source_id: artifact.source_id.clone(),
            source_revision: artifact.source_revision.clone(),
            semantic_digest: artifact.artifact_id.clone(),
            media_type: artifact.media_type.clone(),
            byte_length: artifact.logical_bytes,
        });
    }
    for context in &mining.execution.evidence.contexts {
        evidence.push(EvidenceReference {
            evidence_id: context.artifact_id.to_string(),
            provider: mining.execution.request.provider.clone(),
            source_id: format!(
                "rey-local-source://{}#bytes={}-{}",
                context.source_artifact_id, context.start_byte, context.end_byte
            ),
            source_revision: context.source_artifact_id.to_string(),
            semantic_digest: context.artifact_id.clone(),
            media_type: "text/plain; charset=utf-8".to_owned(),
            byte_length: context.text.len() as u64,
        });
    }
    let evidence_ids = evidence
        .iter()
        .map(|reference| reference.evidence_id.clone())
        .collect::<Vec<_>>();
    let transition_id = derived_digest(
        "rey.mining-failure-transition.v1",
        &[execution_id, &frontier.frontier_id, &scheduling.decision_id],
    );
    let surface = ReasoningSurface::new(
        ReasoningSurfaceInputs {
            workload: workload.clone(),
            graph: graph.clone(),
            scenario_suite: scenario_suite.clone(),
            campaign_id: campaign_id.clone(),
            space: local_source_space(),
            trace_id,
            committed_transition_id: execution_id.clone(),
            transition_id,
            scheduling_decision_id: scheduling.decision_id.clone(),
            frontier_frame_id: frontier.frontier_id.clone(),
            capability_snapshot_id: mining.execution.request.capability_snapshot_id.clone(),
            projection: ContractIdentity::new(
                "rey.reasoning-surface.mining-gap",
                1,
                "project one selected mining failure with exact match, context, delta, provider, and limit evidence",
            ),
        },
        ReasoningSurfaceLimits::default(),
        1,
        SurfaceCompleteness::Complete,
        vec![ReasoningSurfaceRow {
            frontier_row_id: frontier_row.row_id.to_string(),
            entity_kind: frontier_row.entity_kind.clone(),
            entity_id: frontier_row.entity_id.clone(),
            transition_delta_ids: frontier_row.transition_delta_ids.clone(),
            residual_delta_ids: frontier_row.residual_delta_ids.clone(),
            claim_ids: frontier_row.claim_ids.clone(),
            evidence_ids,
            admissible_action_ids: frontier_row.admissible_action_ids.clone(),
        }],
        evidence,
        vec![action],
        Vec::new(),
    )?;
    let reasoning = MiningReasoningEvidence {
        frontier,
        scheduling,
        surface,
    };
    reasoning.verify()?;
    Ok(Some(reasoning))
}

fn derived_digest(domain: &str, inputs: &[&SemanticDigest]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(domain);
    for input in inputs {
        hasher.add_str(input.as_str());
    }
    hasher.finish()
}

pub(crate) fn render_source_matches(execution: &SourceMiningExecution) -> String {
    let mut rendered = String::new();
    for row in &execution.evidence.matches {
        rendered.push_str(&format!(
            "{}:{}:{}-{}:{}\n",
            row.path.display,
            row.start_line,
            row.start_byte_in_line,
            row.end_byte_in_line,
            row.matched_text
        ));
    }
    rendered
}

pub(crate) fn render_expected_matches(expected: &[ExpectedSourceMatch]) -> String {
    let mut rows = expected.to_vec();
    rows.sort_by_key(ExpectedSourceMatch::key);
    let mut rendered = String::new();
    for row in rows {
        rendered.push_str(&format!(
            "{}:{}:{}-{}:{}\n",
            row.path.display,
            row.start_line,
            row.start_byte_in_line,
            row.end_byte_in_line,
            row.matched_text
        ));
    }
    rendered
}

#[must_use]
pub fn source_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../rey-environment/tests/fixtures/source-corpus")
}

#[must_use]
pub fn source_fixture_paths() -> Vec<PathBuf> {
    vec![PathBuf::from("alpha.txt"), PathBuf::from("nested/beta.rs")]
}

pub(crate) fn fixture_capability_snapshot_id() -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.fixture.source-capability-snapshot.v1");
    local_source_provider().add_semantics(&mut hasher);
    builtin_source_search_operation()
        .operation
        .add_semantics(&mut hasher);
    hasher.finish()
}

pub(crate) fn local_source_space() -> ContractIdentity {
    ContractIdentity::new(
        "rey.space.local-source",
        1,
        "one explicitly bound canonical local source corpus",
    )
}
