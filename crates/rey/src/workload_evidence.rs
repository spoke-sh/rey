use rey_core::{ContractIdentity, SemanticDigest};
use rey_diff::{DeltaAssessment, ScenarioOutputDelta};
use rey_mining::TopographyPatch;
use rey_runtime::{
    MiningScenarioEvidence, ScenarioEvaluation, ScenarioResult, TestStatus, WorkloadLimits,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::workloads::{
    LocalWorkloadState, ResolvedWorkload, WorkloadCatalog, WorkloadCatalogDescriptor,
    WorkloadProvenance,
};

pub const WORKLOAD_EVIDENCE_CATALOG_SCHEMA: &str = "rey.ui-workload-evidence-catalog.v1";
pub const WORKLOAD_EVIDENCE_INDEX_SCHEMA: &str = "rey.ui-workload-evidence-index.v1";
pub const WORKLOAD_SCENARIO_EVIDENCE_SCHEMA: &str = "rey.ui-workload-scenario-evidence.v1";
pub const WORKLOAD_DELTA_EVIDENCE_SCHEMA: &str = "rey.ui-workload-delta-evidence.v1";

const EVIDENCE_AUTHORITY: &str = "verified_retained_result_projection; read_only; no execution, qualification, admission, action, or proof authority";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadEvidenceFreshness {
    Fresh,
    Stale,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadEvidenceAvailability {
    Retained,
    Absent,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadEvidenceSourceBinding {
    ExactCurrent,
    CurrentSourceNotBoundToRetainedResult,
    NoRetainedResult,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadEvidenceCurrent {
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario_suite: ContractIdentity,
    pub evaluator: ContractIdentity,
    pub source: WorkloadProvenance,
    pub limits: WorkloadLimits,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadEvidenceResultReference {
    pub result_id: SemanticDigest,
    pub campaign_id: SemanticDigest,
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario_suite: ContractIdentity,
    pub evaluator: ContractIdentity,
    pub status: TestStatus,
    pub stop_reason: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadDirectedDeltaKind {
    ScenarioOutput,
    SourceMatches,
    TopographyPatch,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadDeltaReference {
    pub kind: WorkloadDirectedDeltaKind,
    pub delta_id: SemanticDigest,
    pub label: String,
    pub assessment: Option<DeltaAssessment>,
    pub route: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadScenarioReference {
    pub scenario: ContractIdentity,
    pub required: bool,
    pub execution_id: SemanticDigest,
    pub evaluation: ScenarioEvaluation,
    pub route: String,
    pub deltas: Vec<WorkloadDeltaReference>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadEvidenceIndex {
    pub schema: String,
    pub authority: String,
    pub workload_id: String,
    pub availability: WorkloadEvidenceAvailability,
    pub freshness: Option<WorkloadEvidenceFreshness>,
    pub source_binding: WorkloadEvidenceSourceBinding,
    pub current: WorkloadEvidenceCurrent,
    pub result: Option<WorkloadEvidenceResultReference>,
    pub scenarios: Vec<WorkloadScenarioReference>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadEvidenceCatalog {
    pub schema: String,
    pub authority: String,
    pub catalog: WorkloadCatalogDescriptor,
    pub workloads: Vec<WorkloadEvidenceIndex>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadScenarioEvidence {
    pub schema: String,
    pub authority: String,
    pub freshness: WorkloadEvidenceFreshness,
    pub source_binding: WorkloadEvidenceSourceBinding,
    pub current: WorkloadEvidenceCurrent,
    pub result: WorkloadEvidenceResultReference,
    pub scenario: ScenarioResult,
    pub deltas: Vec<WorkloadDeltaReference>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkloadDirectedDeltaEvidence {
    ScenarioOutput {
        delta: Box<ScenarioOutputDelta>,
    },
    SourceMatches {
        evidence: Box<MiningScenarioEvidence>,
    },
    TopographyPatch {
        patch: Box<TopographyPatch>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadDeltaEvidence {
    pub schema: String,
    pub authority: String,
    pub freshness: WorkloadEvidenceFreshness,
    pub source_binding: WorkloadEvidenceSourceBinding,
    pub current: WorkloadEvidenceCurrent,
    pub result: WorkloadEvidenceResultReference,
    pub scenario: ContractIdentity,
    pub scenario_execution_id: SemanticDigest,
    pub scenario_route: String,
    pub delta_id: SemanticDigest,
    pub evidence: WorkloadDirectedDeltaEvidence,
}

pub fn workload_evidence_catalog(
    catalog: &WorkloadCatalog,
    state: &LocalWorkloadState,
) -> Result<WorkloadEvidenceCatalog, WorkloadEvidenceError> {
    let workloads = catalog
        .workloads
        .iter()
        .map(|workload| workload_evidence_index_for(workload, state))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(WorkloadEvidenceCatalog {
        schema: WORKLOAD_EVIDENCE_CATALOG_SCHEMA.to_owned(),
        authority: EVIDENCE_AUTHORITY.to_owned(),
        catalog: catalog.descriptor.clone(),
        workloads,
    })
}

pub fn workload_scenario_evidence(
    catalog: &WorkloadCatalog,
    state: &LocalWorkloadState,
    workload_id: &str,
    execution_id: &str,
) -> Result<WorkloadScenarioEvidence, WorkloadEvidenceError> {
    let workload = resolved_workload(catalog, workload_id)?;
    let result = retained_result(workload, state)?;
    let scenario = result
        .scenarios
        .iter()
        .find(|scenario| scenario.execution_id.as_str() == execution_id)
        .ok_or_else(|| WorkloadEvidenceError::UnknownScenario {
            workload_id: workload_id.to_owned(),
            execution_id: execution_id.to_owned(),
        })?;
    let (freshness, source_binding) = freshness(workload, result);
    Ok(WorkloadScenarioEvidence {
        schema: WORKLOAD_SCENARIO_EVIDENCE_SCHEMA.to_owned(),
        authority: EVIDENCE_AUTHORITY.to_owned(),
        freshness,
        source_binding,
        current: current(workload),
        result: result_reference(result),
        scenario: scenario.clone(),
        deltas: delta_references(workload_id, scenario),
    })
}

pub fn workload_delta_evidence(
    catalog: &WorkloadCatalog,
    state: &LocalWorkloadState,
    workload_id: &str,
    delta_id: &str,
) -> Result<WorkloadDeltaEvidence, WorkloadEvidenceError> {
    let workload = resolved_workload(catalog, workload_id)?;
    let result = retained_result(workload, state)?;
    let (scenario, evidence) = result
        .scenarios
        .iter()
        .find_map(|scenario| find_delta(scenario, delta_id).map(|evidence| (scenario, evidence)))
        .ok_or_else(|| WorkloadEvidenceError::UnknownDelta {
            workload_id: workload_id.to_owned(),
            delta_id: delta_id.to_owned(),
        })?;
    let exact_delta_id = match &evidence {
        WorkloadDirectedDeltaEvidence::ScenarioOutput { delta } => delta.delta_id.clone(),
        WorkloadDirectedDeltaEvidence::SourceMatches { evidence } => {
            evidence.relation_delta.delta_id.clone()
        }
        WorkloadDirectedDeltaEvidence::TopographyPatch { patch } => patch.delta.delta_id.clone(),
    };
    let (freshness, source_binding) = freshness(workload, result);
    Ok(WorkloadDeltaEvidence {
        schema: WORKLOAD_DELTA_EVIDENCE_SCHEMA.to_owned(),
        authority: EVIDENCE_AUTHORITY.to_owned(),
        freshness,
        source_binding,
        current: current(workload),
        result: result_reference(result),
        scenario: scenario.scenario.clone(),
        scenario_execution_id: scenario.execution_id.clone(),
        scenario_route: scenario_route(workload_id, &scenario.execution_id),
        delta_id: exact_delta_id,
        evidence,
    })
}

fn workload_evidence_index_for(
    workload: &ResolvedWorkload,
    state: &LocalWorkloadState,
) -> Result<WorkloadEvidenceIndex, WorkloadEvidenceError> {
    let workload_id = workload.definition.workload.id.clone();
    let Some(result) = state
        .record(&workload_id)
        .and_then(|record| record.last_test.as_ref())
    else {
        return Ok(WorkloadEvidenceIndex {
            schema: WORKLOAD_EVIDENCE_INDEX_SCHEMA.to_owned(),
            authority: EVIDENCE_AUTHORITY.to_owned(),
            workload_id,
            availability: WorkloadEvidenceAvailability::Absent,
            freshness: None,
            source_binding: WorkloadEvidenceSourceBinding::NoRetainedResult,
            current: current(workload),
            result: None,
            scenarios: Vec::new(),
        });
    };
    result
        .verify()
        .map_err(|error| WorkloadEvidenceError::InvalidRetainedResult(error.to_string()))?;
    let (freshness, source_binding) = freshness(workload, result);
    Ok(WorkloadEvidenceIndex {
        schema: WORKLOAD_EVIDENCE_INDEX_SCHEMA.to_owned(),
        authority: EVIDENCE_AUTHORITY.to_owned(),
        workload_id,
        availability: WorkloadEvidenceAvailability::Retained,
        freshness: Some(freshness),
        source_binding,
        current: current(workload),
        result: Some(result_reference(result)),
        scenarios: result
            .scenarios
            .iter()
            .map(|scenario| WorkloadScenarioReference {
                scenario: scenario.scenario.clone(),
                required: scenario.required,
                execution_id: scenario.execution_id.clone(),
                evaluation: scenario.evaluation,
                route: scenario_route(&workload.definition.workload.id, &scenario.execution_id),
                deltas: delta_references(&workload.definition.workload.id, scenario),
            })
            .collect(),
    })
}

fn resolved_workload<'a>(
    catalog: &'a WorkloadCatalog,
    workload_id: &str,
) -> Result<&'a ResolvedWorkload, WorkloadEvidenceError> {
    catalog
        .workloads
        .iter()
        .find(|workload| workload.definition.workload.id == workload_id)
        .ok_or_else(|| WorkloadEvidenceError::UnknownWorkload(workload_id.to_owned()))
}

fn retained_result<'a>(
    workload: &ResolvedWorkload,
    state: &'a LocalWorkloadState,
) -> Result<&'a rey_runtime::WorkloadTestResult, WorkloadEvidenceError> {
    let workload_id = &workload.definition.workload.id;
    let result = state
        .record(workload_id)
        .and_then(|record| record.last_test.as_ref())
        .ok_or_else(|| WorkloadEvidenceError::EvidenceUnavailable(workload_id.clone()))?;
    result
        .verify()
        .map_err(|error| WorkloadEvidenceError::InvalidRetainedResult(error.to_string()))?;
    Ok(result)
}

fn current(workload: &ResolvedWorkload) -> WorkloadEvidenceCurrent {
    WorkloadEvidenceCurrent {
        workload: workload.definition.workload.clone(),
        graph: workload.definition.graph.graph.clone(),
        scenario_suite: workload.definition.scenario_suite.suite.clone(),
        evaluator: workload.definition.evaluator.clone(),
        source: workload.provenance.clone(),
        limits: workload.definition.limits.clone(),
    }
}

fn freshness(
    workload: &ResolvedWorkload,
    result: &rey_runtime::WorkloadTestResult,
) -> (WorkloadEvidenceFreshness, WorkloadEvidenceSourceBinding) {
    if result.verify_for(&workload.definition).is_ok() {
        (
            WorkloadEvidenceFreshness::Fresh,
            WorkloadEvidenceSourceBinding::ExactCurrent,
        )
    } else {
        (
            WorkloadEvidenceFreshness::Stale,
            WorkloadEvidenceSourceBinding::CurrentSourceNotBoundToRetainedResult,
        )
    }
}

fn result_reference(result: &rey_runtime::WorkloadTestResult) -> WorkloadEvidenceResultReference {
    WorkloadEvidenceResultReference {
        result_id: result.result_id.clone(),
        campaign_id: result.campaign_id.clone(),
        workload: result.workload.clone(),
        graph: result.graph.clone(),
        scenario_suite: result.scenario_suite.clone(),
        evaluator: result.evaluator.clone(),
        status: result.status,
        stop_reason: result.stop_reason.clone(),
    }
}

fn delta_references(workload_id: &str, scenario: &ScenarioResult) -> Vec<WorkloadDeltaReference> {
    let outputs = scenario.deltas.iter().map(|delta| WorkloadDeltaReference {
        kind: WorkloadDirectedDeltaKind::ScenarioOutput,
        delta_id: delta.delta_id.clone(),
        label: format!("output.{}", delta.inputs.output_id),
        assessment: Some(delta.assessment),
        route: delta_route(workload_id, &delta.delta_id),
    });
    let mining = scenario
        .mining
        .iter()
        .map(|evidence| WorkloadDeltaReference {
            kind: WorkloadDirectedDeltaKind::SourceMatches,
            delta_id: evidence.relation_delta.delta_id.clone(),
            label: "source.matches".to_owned(),
            assessment: Some(evidence.relation_delta.assessment),
            route: delta_route(workload_id, &evidence.relation_delta.delta_id),
        });
    let topography = scenario
        .topography
        .iter()
        .map(|patch| WorkloadDeltaReference {
            kind: WorkloadDirectedDeltaKind::TopographyPatch,
            delta_id: patch.delta.delta_id.clone(),
            label: "topography.patch".to_owned(),
            assessment: None,
            route: delta_route(workload_id, &patch.delta.delta_id),
        });
    outputs.chain(mining).chain(topography).collect()
}

fn find_delta(scenario: &ScenarioResult, delta_id: &str) -> Option<WorkloadDirectedDeltaEvidence> {
    if let Some(delta) = scenario
        .deltas
        .iter()
        .find(|delta| delta.delta_id.as_str() == delta_id)
    {
        return Some(WorkloadDirectedDeltaEvidence::ScenarioOutput {
            delta: Box::new(delta.clone()),
        });
    }
    if let Some(evidence) = scenario
        .mining
        .iter()
        .find(|evidence| evidence.relation_delta.delta_id.as_str() == delta_id)
    {
        return Some(WorkloadDirectedDeltaEvidence::SourceMatches {
            evidence: Box::new(evidence.clone()),
        });
    }
    scenario
        .topography
        .iter()
        .find(|patch| patch.delta.delta_id.as_str() == delta_id)
        .map(|patch| WorkloadDirectedDeltaEvidence::TopographyPatch {
            patch: Box::new(patch.clone()),
        })
}

fn scenario_route(workload_id: &str, execution_id: &SemanticDigest) -> String {
    format!("/workloads/{workload_id}/scenarios/{execution_id}")
}

fn delta_route(workload_id: &str, delta_id: &SemanticDigest) -> String {
    format!("/workloads/{workload_id}/deltas/{delta_id}")
}

#[derive(Debug, Error)]
pub enum WorkloadEvidenceError {
    #[error("unknown admitted workload {0}")]
    UnknownWorkload(String),
    #[error("workload {0} has no retained test evidence")]
    EvidenceUnavailable(String),
    #[error("workload {workload_id} has no retained scenario execution {execution_id}")]
    UnknownScenario {
        workload_id: String,
        execution_id: String,
    },
    #[error("workload {workload_id} has no retained directed delta {delta_id}")]
    UnknownDelta {
        workload_id: String,
        delta_id: String,
    },
    #[error("retained workload result failed verification: {0}")]
    InvalidRetainedResult(String),
}

#[cfg(test)]
mod tests {
    use rey_runtime::test_workload;

    use super::*;
    use crate::workloads::{LocalWorkloadRecord, LocalWorkloadState, WorkloadCatalog};

    fn retained_fixture() -> (WorkloadCatalog, LocalWorkloadState) {
        let catalog = WorkloadCatalog::built_in_conformance().unwrap();
        let workload = catalog
            .workloads
            .iter()
            .find(|workload| workload.definition.workload.id == "rey.fixture.text-mismatch")
            .unwrap();
        let result = test_workload(&workload.definition).unwrap();
        let mut state = LocalWorkloadState::default();
        state.records.insert(
            workload.definition.workload.id.clone(),
            LocalWorkloadRecord {
                last_test: Some(result),
                last_run: None,
                prior_scene_admissions: Vec::new(),
            },
        );
        (catalog, state)
    }

    #[test]
    fn resolves_exact_scenario_and_delta_routes_from_verified_retained_evidence() {
        let (catalog, state) = retained_fixture();
        let evidence = workload_evidence_catalog(&catalog, &state).unwrap();
        let index = evidence
            .workloads
            .iter()
            .find(|workload| workload.workload_id == "rey.fixture.text-mismatch")
            .unwrap();
        assert_eq!(index.availability, WorkloadEvidenceAvailability::Retained);
        assert_eq!(index.freshness, Some(WorkloadEvidenceFreshness::Fresh));
        let scenario_ref = &index.scenarios[1];
        let delta_ref = &scenario_ref.deltas[0];

        let scenario = workload_scenario_evidence(
            &catalog,
            &state,
            &index.workload_id,
            scenario_ref.execution_id.as_str(),
        )
        .unwrap();
        assert_eq!(scenario.scenario.execution_id, scenario_ref.execution_id);
        assert_eq!(scenario.deltas, scenario_ref.deltas);

        let delta = workload_delta_evidence(
            &catalog,
            &state,
            &index.workload_id,
            delta_ref.delta_id.as_str(),
        )
        .unwrap();
        assert_eq!(delta.delta_id, delta_ref.delta_id);
        assert!(matches!(
            delta.evidence,
            WorkloadDirectedDeltaEvidence::ScenarioOutput { .. }
        ));
    }

    #[test]
    fn stale_results_remain_exact_but_do_not_claim_current_source_binding() {
        let (catalog, mut state) = retained_fixture();
        let workload = catalog
            .workloads
            .iter()
            .find(|workload| workload.definition.workload.id == "rey.fixture.text-normalize")
            .unwrap();
        let stale_result = state
            .records
            .get("rey.fixture.text-mismatch")
            .unwrap()
            .last_test
            .clone();
        state.records.insert(
            workload.definition.workload.id.clone(),
            LocalWorkloadRecord {
                last_test: stale_result,
                last_run: None,
                prior_scene_admissions: Vec::new(),
            },
        );

        let evidence = workload_evidence_catalog(&catalog, &state).unwrap();
        let index = evidence
            .workloads
            .iter()
            .find(|workload| workload.workload_id == "rey.fixture.text-normalize")
            .unwrap();
        assert_eq!(index.freshness, Some(WorkloadEvidenceFreshness::Stale));
        assert_eq!(
            index.source_binding,
            WorkloadEvidenceSourceBinding::CurrentSourceNotBoundToRetainedResult
        );
    }

    #[test]
    fn rejects_unknown_exact_identifiers_without_falling_back_to_latest() {
        let (catalog, state) = retained_fixture();
        let error = workload_scenario_evidence(
            &catalog,
            &state,
            "rey.fixture.text-mismatch",
            "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap_err();
        assert!(matches!(
            error,
            WorkloadEvidenceError::UnknownScenario { .. }
        ));
    }

    #[test]
    fn resolves_source_match_deltas_with_native_context_evidence() {
        let catalog = WorkloadCatalog::built_in_conformance().unwrap();
        let workload = catalog
            .workloads
            .iter()
            .find(|workload| workload.definition.workload.id == "rey.fixture.source-search")
            .unwrap();
        let result = test_workload(&workload.definition).unwrap();
        let mut state = LocalWorkloadState::default();
        state.records.insert(
            workload.definition.workload.id.clone(),
            LocalWorkloadRecord {
                last_test: Some(result),
                last_run: None,
                prior_scene_admissions: Vec::new(),
            },
        );
        let index = workload_evidence_catalog(&catalog, &state)
            .unwrap()
            .workloads
            .into_iter()
            .find(|index| index.workload_id == workload.definition.workload.id)
            .unwrap();
        let reference = index
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.deltas)
            .find(|delta| delta.kind == WorkloadDirectedDeltaKind::SourceMatches)
            .unwrap();

        let resolved = workload_delta_evidence(
            &catalog,
            &state,
            &index.workload_id,
            reference.delta_id.as_str(),
        )
        .unwrap();
        let WorkloadDirectedDeltaEvidence::SourceMatches { evidence } = resolved.evidence else {
            panic!("expected source-match evidence");
        };
        assert_eq!(evidence.relation_delta.delta_id, reference.delta_id);
        assert!(
            evidence
                .relation_delta
                .observed
                .iter()
                .all(|row| row.context_ref.starts_with("rey-local-source://"))
        );
    }
}
