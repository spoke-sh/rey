use std::collections::{BTreeMap, BTreeSet};

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_diff::{
    DeltaAssessment, ScenarioDeltaInputs, ScenarioDeltaLimits, ScenarioOutputDelta,
    compare_scenario_utf8,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const WORKLOAD_SCHEMA: &str = "rey.workload.v1";
pub const COMPUTE_GRAPH_SCHEMA: &str = "rey.compute-graph.v1";
pub const SCENARIO_SUITE_SCHEMA: &str = "rey.scenario-suite.v1";
pub const WORKLOAD_TEST_RESULT_SCHEMA: &str = "rey.workload-test-result.v1";
pub const WORKLOAD_QUALIFICATION_SCHEMA: &str = "rey.workload-qualification.v1";
pub const WORKLOAD_RUN_RESULT_SCHEMA: &str = "rey.workload-run-result.v1";

pub const BUILT_IN_NORMALIZE_WORKLOAD_ID: &str = "rey.fixture.text-normalize";
pub const BUILT_IN_MISMATCH_WORKLOAD_ID: &str = "rey.fixture.text-mismatch";

const NODE_OUTPUT_ID: &str = "value";
const INPUT_ID: &str = "text";
const OUTPUT_ID: &str = "text";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    Utf8,
}

impl ValueType {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Utf8 => "utf8",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum WorkloadValue {
    Utf8(String),
}

impl WorkloadValue {
    #[must_use]
    pub const fn value_type(&self) -> ValueType {
        match self {
            Self::Utf8(_) => ValueType::Utf8,
        }
    }

    fn byte_len(&self) -> u64 {
        match self {
            Self::Utf8(value) => value.len() as u64,
        }
    }

    fn as_utf8(&self) -> &str {
        match self {
            Self::Utf8(value) => value,
        }
    }

    fn add_semantics(&self, hasher: &mut SemanticHasher) {
        hasher.add_str(self.value_type().as_str());
        hasher.add_str(self.as_utf8());
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadPort {
    pub port_id: String,
    pub value_type: ValueType,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ValueSource {
    ExternalInput { input_id: String },
    NodeOutput { node_id: String, output_id: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GraphNode {
    pub node_id: String,
    pub operation: ContractIdentity,
    pub input: ValueSource,
    pub output_id: String,
    pub value_type: ValueType,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GraphOutput {
    pub output_id: String,
    pub source: ValueSource,
    pub value_type: ValueType,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GraphLimits {
    pub max_nodes: u64,
    pub max_edges: u64,
    pub max_depth: u64,
    pub max_input_bytes: u64,
    pub max_output_bytes: u64,
    pub max_string_bytes: u64,
}

impl Default for GraphLimits {
    fn default() -> Self {
        Self {
            max_nodes: 32,
            max_edges: 64,
            max_depth: 16,
            max_input_bytes: 64 * 1_024,
            max_output_bytes: 64 * 1_024,
            max_string_bytes: 256 * 1_024,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComputeGraph {
    pub schema: String,
    pub graph: ContractIdentity,
    pub nodes: Vec<GraphNode>,
    pub outputs: Vec<GraphOutput>,
    pub limits: GraphLimits,
}

impl ComputeGraph {
    fn new(
        id: &str,
        revision: u64,
        nodes: Vec<GraphNode>,
        outputs: Vec<GraphOutput>,
        limits: GraphLimits,
    ) -> Result<Self, WorkloadError> {
        let mut graph = Self {
            schema: COMPUTE_GRAPH_SCHEMA.to_owned(),
            graph: placeholder_contract(id, revision, "rey.compute-graph.placeholder"),
            nodes,
            outputs,
            limits,
        };
        graph.graph.semantic_digest = graph_digest(&graph);
        Ok(graph)
    }

    fn verify(
        &self,
        inputs: &[WorkloadPort],
        outputs: &[WorkloadPort],
    ) -> Result<Vec<String>, WorkloadError> {
        if self.schema != COMPUTE_GRAPH_SCHEMA {
            return Err(WorkloadError::UnsupportedSchema {
                expected: COMPUTE_GRAPH_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_contract("graph", &self.graph)?;
        validate_graph_limits(&self.limits)?;
        if self.nodes.is_empty() {
            return Err(WorkloadError::EmptyGraph);
        }
        enforce_count("graph nodes", self.nodes.len(), self.limits.max_nodes)?;
        enforce_count(
            "graph edges",
            self.nodes.len().saturating_add(self.outputs.len()),
            self.limits.max_edges,
        )?;

        let input_types = ports_by_id("workload input", inputs)?;
        let output_types = ports_by_id("workload output", outputs)?;
        let mut node_ids = BTreeSet::new();
        for node in &self.nodes {
            validate_text("node id", &node.node_id)?;
            validate_text("node output id", &node.output_id)?;
            if node.output_id != NODE_OUTPUT_ID {
                return Err(WorkloadError::UnknownNodeOutput(node.output_id.clone()));
            }
            if node.value_type != ValueType::Utf8 {
                return Err(WorkloadError::TypeMismatch(node.node_id.clone()));
            }
            resolve_operation(&node.operation)?;
            if !node_ids.insert(node.node_id.clone()) {
                return Err(WorkloadError::DuplicateId(node.node_id.clone()));
            }
        }

        let order = topological_order(self, &input_types)?;
        let selected_outputs = graph_outputs_by_id(&self.outputs, &node_ids, &input_types)?;
        if selected_outputs != output_types {
            return Err(WorkloadError::OutputContractMismatch);
        }
        if semantic_string_bytes_graph(self)? > self.limits.max_string_bytes {
            return Err(WorkloadError::StringByteLimit {
                limit: self.limits.max_string_bytes,
            });
        }
        let actual = graph_digest(self);
        if actual != self.graph.semantic_digest {
            return Err(WorkloadError::ContractDigest {
                role: "graph",
                declared: self.graph.semantic_digest.clone(),
                actual,
            });
        }
        Ok(order)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Scenario {
    pub scenario: ContractIdentity,
    pub required: bool,
    pub inputs: BTreeMap<String, WorkloadValue>,
    pub expected_outputs: BTreeMap<String, WorkloadValue>,
}

impl Scenario {
    fn new(
        id: &str,
        required: bool,
        inputs: BTreeMap<String, WorkloadValue>,
        expected_outputs: BTreeMap<String, WorkloadValue>,
    ) -> Self {
        let mut scenario = Self {
            scenario: placeholder_contract(id, 1, "rey.scenario.placeholder"),
            required,
            inputs,
            expected_outputs,
        };
        scenario.scenario.semantic_digest = scenario_digest(&scenario);
        scenario
    }

    fn verify(
        &self,
        inputs: &[WorkloadPort],
        outputs: &[WorkloadPort],
        limits: &WorkloadLimits,
    ) -> Result<(), WorkloadError> {
        validate_contract("scenario", &self.scenario)?;
        validate_bindings("scenario input", &self.inputs, inputs)?;
        validate_bindings("scenario output", &self.expected_outputs, outputs)?;
        enforce_count(
            "scenario outputs",
            self.expected_outputs.len(),
            limits.max_outputs_per_scenario,
        )?;
        let actual = scenario_digest(self);
        if actual != self.scenario.semantic_digest {
            return Err(WorkloadError::ContractDigest {
                role: "scenario",
                declared: self.scenario.semantic_digest.clone(),
                actual,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScenarioSuite {
    pub schema: String,
    pub suite: ContractIdentity,
    pub scenarios: Vec<Scenario>,
}

impl ScenarioSuite {
    fn new(id: &str, scenarios: Vec<Scenario>) -> Self {
        let mut suite = Self {
            schema: SCENARIO_SUITE_SCHEMA.to_owned(),
            suite: placeholder_contract(id, 1, "rey.scenario-suite.placeholder"),
            scenarios,
        };
        suite.suite.semantic_digest = suite_digest(&suite);
        suite
    }

    fn verify(
        &self,
        inputs: &[WorkloadPort],
        outputs: &[WorkloadPort],
        limits: &WorkloadLimits,
    ) -> Result<(), WorkloadError> {
        if self.schema != SCENARIO_SUITE_SCHEMA {
            return Err(WorkloadError::UnsupportedSchema {
                expected: SCENARIO_SUITE_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_contract("scenario suite", &self.suite)?;
        enforce_count("scenarios", self.scenarios.len(), limits.max_scenarios)?;
        if self.scenarios.is_empty() {
            return Err(WorkloadError::EmptyScenarioSuite);
        }
        if !self.scenarios.iter().any(|scenario| scenario.required) {
            return Err(WorkloadError::NoRequiredScenario);
        }
        let mut ids = BTreeSet::new();
        let mut previous_scenario = None;
        for scenario in &self.scenarios {
            scenario.verify(inputs, outputs, limits)?;
            if previous_scenario.is_some_and(|previous| previous >= scenario.scenario.id.as_str()) {
                return Err(WorkloadError::ResultShape(
                    "scenario suite is not in canonical order",
                ));
            }
            previous_scenario = Some(scenario.scenario.id.as_str());
            if !ids.insert(scenario.scenario.id.clone()) {
                return Err(WorkloadError::DuplicateId(scenario.scenario.id.clone()));
            }
        }
        let actual = suite_digest(self);
        if actual != self.suite.semantic_digest {
            return Err(WorkloadError::ContractDigest {
                role: "scenario suite",
                declared: self.suite.semantic_digest.clone(),
                actual,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadLimits {
    pub max_scenarios: u64,
    pub max_outputs_per_scenario: u64,
    pub max_string_bytes: u64,
    pub scenario_delta: ScenarioDeltaLimits,
}

impl Default for WorkloadLimits {
    fn default() -> Self {
        Self {
            max_scenarios: 64,
            max_outputs_per_scenario: 16,
            max_string_bytes: 512 * 1_024,
            scenario_delta: ScenarioDeltaLimits::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadDefinition {
    pub schema: String,
    pub workload: ContractIdentity,
    pub title: String,
    pub inputs: Vec<WorkloadPort>,
    pub outputs: Vec<WorkloadPort>,
    pub graph: ComputeGraph,
    pub scenario_suite: ScenarioSuite,
    pub evaluator: ContractIdentity,
    pub limits: WorkloadLimits,
}

impl WorkloadDefinition {
    fn finalize(mut self) -> Result<Self, WorkloadError> {
        self.workload.semantic_digest = workload_digest(&self);
        self.verify()?;
        Ok(self)
    }

    pub fn verify(&self) -> Result<(), WorkloadError> {
        if self.schema != WORKLOAD_SCHEMA {
            return Err(WorkloadError::UnsupportedSchema {
                expected: WORKLOAD_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_contract("workload", &self.workload)?;
        validate_contract("evaluator", &self.evaluator)?;
        validate_text("workload title", &self.title)?;
        validate_workload_limits(&self.limits)?;
        ports_by_id("workload input", &self.inputs)?;
        ports_by_id("workload output", &self.outputs)?;
        self.graph.verify(&self.inputs, &self.outputs)?;
        self.scenario_suite
            .verify(&self.inputs, &self.outputs, &self.limits)?;
        if semantic_string_bytes_workload(self)? > self.limits.max_string_bytes {
            return Err(WorkloadError::StringByteLimit {
                limit: self.limits.max_string_bytes,
            });
        }
        let actual = workload_digest(self);
        if actual != self.workload.semantic_digest {
            return Err(WorkloadError::ContractDigest {
                role: "workload",
                declared: self.workload.semantic_digest.clone(),
                actual,
            });
        }
        Ok(())
    }

    #[must_use]
    pub fn required_scenario_count(&self) -> u64 {
        self.scenario_suite
            .scenarios
            .iter()
            .filter(|scenario| scenario.required)
            .count() as u64
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GraphExecution {
    pub execution_id: SemanticDigest,
    pub graph: ContractIdentity,
    pub node_order: Vec<String>,
    pub outputs: BTreeMap<String, WorkloadValue>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioEvaluation {
    Passed,
    Failed,
    Inconclusive,
}

impl ScenarioEvaluation {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScenarioResult {
    pub scenario: ContractIdentity,
    pub required: bool,
    pub execution_id: SemanticDigest,
    pub evaluation: ScenarioEvaluation,
    pub deltas: Vec<ScenarioOutputDelta>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TestStatus {
    Passed,
    Failed,
    Inconclusive,
}

impl TestStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TestSummary {
    pub required: u64,
    pub passed: u64,
    pub failed: u64,
    pub inconclusive: u64,
    pub evaluated: u64,
    pub optional: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QualificationRecord {
    pub schema: String,
    pub qualification_id: SemanticDigest,
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario_suite: ContractIdentity,
    pub evaluator: ContractIdentity,
    pub test_result_id: SemanticDigest,
}

impl QualificationRecord {
    pub fn verify(&self) -> Result<(), WorkloadError> {
        if self.schema != WORKLOAD_QUALIFICATION_SCHEMA {
            return Err(WorkloadError::UnsupportedSchema {
                expected: WORKLOAD_QUALIFICATION_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        for (role, contract) in [
            ("workload", &self.workload),
            ("graph", &self.graph),
            ("scenario suite", &self.scenario_suite),
            ("evaluator", &self.evaluator),
        ] {
            validate_contract(role, contract)?;
        }
        validate_digest(&self.test_result_id)?;
        let actual = qualification_digest(self);
        if actual != self.qualification_id {
            return Err(WorkloadError::ArtifactDigest {
                role: "qualification",
                declared: self.qualification_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    #[must_use]
    pub fn is_fresh_for(&self, workload: &WorkloadDefinition) -> bool {
        self.workload == workload.workload
            && self.graph == workload.graph.graph
            && self.scenario_suite == workload.scenario_suite.suite
            && self.evaluator == workload.evaluator
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadTestResult {
    pub schema: String,
    pub result_id: SemanticDigest,
    pub campaign_id: SemanticDigest,
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario_suite: ContractIdentity,
    pub evaluator: ContractIdentity,
    pub status: TestStatus,
    pub stop_reason: String,
    pub summary: TestSummary,
    pub scenarios: Vec<ScenarioResult>,
    pub qualification: Option<QualificationRecord>,
}

impl WorkloadTestResult {
    pub fn verify(&self) -> Result<(), WorkloadError> {
        if self.schema != WORKLOAD_TEST_RESULT_SCHEMA {
            return Err(WorkloadError::UnsupportedSchema {
                expected: WORKLOAD_TEST_RESULT_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        for (role, contract) in [
            ("workload", &self.workload),
            ("graph", &self.graph),
            ("scenario suite", &self.scenario_suite),
            ("evaluator", &self.evaluator),
        ] {
            validate_contract(role, contract)?;
        }
        validate_digest(&self.campaign_id)?;
        let expected_campaign = campaign_digest_from_contracts(
            &self.workload,
            &self.graph,
            &self.scenario_suite,
            &self.evaluator,
        );
        if self.campaign_id != expected_campaign {
            return Err(WorkloadError::ArtifactDigest {
                role: "test campaign",
                declared: self.campaign_id.clone(),
                actual: expected_campaign,
            });
        }
        if self.scenarios.is_empty() {
            return Err(WorkloadError::ResultShape(
                "test result must contain at least one scenario",
            ));
        }
        let mut ids = BTreeSet::new();
        let mut previous_scenario = None;
        for scenario in &self.scenarios {
            validate_contract("scenario", &scenario.scenario)?;
            validate_digest(&scenario.execution_id)?;
            if previous_scenario.is_some_and(|previous| previous >= scenario.scenario.id.as_str()) {
                return Err(WorkloadError::ResultShape(
                    "scenario results are not in canonical order",
                ));
            }
            previous_scenario = Some(scenario.scenario.id.as_str());
            if !ids.insert(scenario.scenario.id.clone()) {
                return Err(WorkloadError::DuplicateId(scenario.scenario.id.clone()));
            }
            if scenario.deltas.is_empty() {
                return Err(WorkloadError::ResultShape(
                    "scenario result must contain at least one output delta",
                ));
            }
            let mut output_ids = BTreeSet::new();
            let mut previous_output = None;
            for delta in &scenario.deltas {
                delta.verify()?;
                if delta.inputs.workload != self.workload
                    || delta.inputs.graph != self.graph
                    || delta.inputs.scenario != scenario.scenario
                    || delta.inputs.comparator != self.evaluator
                {
                    return Err(WorkloadError::ResultShape(
                        "scenario delta does not bind the test result",
                    ));
                }
                if previous_output
                    .is_some_and(|previous| previous >= delta.inputs.output_id.as_str())
                {
                    return Err(WorkloadError::ResultShape(
                        "scenario output deltas are not in canonical order",
                    ));
                }
                previous_output = Some(delta.inputs.output_id.as_str());
                if !output_ids.insert(delta.inputs.output_id.clone()) {
                    return Err(WorkloadError::DuplicateId(delta.inputs.output_id.clone()));
                }
            }
            let expected = scenario_evaluation(&scenario.deltas);
            if expected != scenario.evaluation {
                return Err(WorkloadError::ResultShape(
                    "scenario evaluation does not match its deltas",
                ));
            }
        }
        let (status, summary) = summarize(&self.scenarios);
        if status != self.status || summary != self.summary {
            return Err(WorkloadError::ResultShape(
                "test status or summary does not match scenario results",
            ));
        }
        let expected_stop_reason = match self.status {
            TestStatus::Passed => "qualified",
            TestStatus::Failed => "conclusive_failure",
            TestStatus::Inconclusive => "inconclusive",
        };
        if self.stop_reason != expected_stop_reason {
            return Err(WorkloadError::ResultShape(
                "test stop reason does not match its status",
            ));
        }
        match (&self.qualification, self.status) {
            (Some(qualification), TestStatus::Passed) => {
                qualification.verify()?;
                if qualification.test_result_id != self.result_id
                    || qualification.workload != self.workload
                    || qualification.graph != self.graph
                    || qualification.scenario_suite != self.scenario_suite
                    || qualification.evaluator != self.evaluator
                {
                    return Err(WorkloadError::ResultShape(
                        "qualification does not bind the test result",
                    ));
                }
            }
            (None, TestStatus::Failed | TestStatus::Inconclusive) => {}
            _ => {
                return Err(WorkloadError::ResultShape(
                    "qualification presence does not match test status",
                ));
            }
        }
        let actual = test_result_digest(self);
        if actual != self.result_id {
            return Err(WorkloadError::ArtifactDigest {
                role: "test result",
                declared: self.result_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    #[must_use]
    pub fn is_fresh_for(&self, workload: &WorkloadDefinition) -> bool {
        self.workload == workload.workload
            && self.graph == workload.graph.graph
            && self.scenario_suite == workload.scenario_suite.suite
            && self.evaluator == workload.evaluator
    }

    pub fn verify_for(&self, workload: &WorkloadDefinition) -> Result<(), WorkloadError> {
        workload.verify()?;
        self.verify()?;
        if !self.is_fresh_for(workload) {
            return Err(WorkloadError::StaleQualification);
        }
        if self.scenarios.len() != workload.scenario_suite.scenarios.len() {
            return Err(WorkloadError::ResultShape(
                "test result does not cover the exact scenario suite",
            ));
        }
        for (result, scenario) in self
            .scenarios
            .iter()
            .zip(&workload.scenario_suite.scenarios)
        {
            if result.scenario != scenario.scenario || result.required != scenario.required {
                return Err(WorkloadError::ResultShape(
                    "scenario result does not match the exact scenario contract",
                ));
            }
            if result.deltas.len() != scenario.expected_outputs.len() {
                return Err(WorkloadError::ResultShape(
                    "scenario result does not cover every expected output",
                ));
            }
            for delta in &result.deltas {
                let expected = scenario
                    .expected_outputs
                    .get(&delta.inputs.output_id)
                    .ok_or(WorkloadError::ResultShape(
                        "scenario result contains an undeclared output",
                    ))?;
                if delta.expected != expected.as_utf8() {
                    return Err(WorkloadError::ResultShape(
                        "scenario delta does not match its declared expected output",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Passed,
    Blocked,
}

impl RunStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadRunResult {
    pub schema: String,
    pub run_id: SemanticDigest,
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub qualification_id: Option<SemanticDigest>,
    pub status: RunStatus,
    pub stop_reason: String,
    pub inputs: BTreeMap<String, WorkloadValue>,
    pub outputs: BTreeMap<String, WorkloadValue>,
    pub node_order: Vec<String>,
}

impl WorkloadRunResult {
    pub fn blocked(workload: &WorkloadDefinition, inputs: BTreeMap<String, WorkloadValue>) -> Self {
        let mut result = Self {
            schema: WORKLOAD_RUN_RESULT_SCHEMA.to_owned(),
            run_id: placeholder_digest("rey.workload-run-result.placeholder"),
            workload: workload.workload.clone(),
            graph: workload.graph.graph.clone(),
            qualification_id: None,
            status: RunStatus::Blocked,
            stop_reason: "qualification_missing_or_stale".to_owned(),
            inputs,
            outputs: BTreeMap::new(),
            node_order: Vec::new(),
        };
        result.run_id = run_result_digest(&result);
        result
    }

    pub fn verify(&self) -> Result<(), WorkloadError> {
        if self.schema != WORKLOAD_RUN_RESULT_SCHEMA {
            return Err(WorkloadError::UnsupportedSchema {
                expected: WORKLOAD_RUN_RESULT_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_contract("workload", &self.workload)?;
        validate_contract("graph", &self.graph)?;
        if let Some(qualification_id) = &self.qualification_id {
            validate_digest(qualification_id)?;
        }
        match self.status {
            RunStatus::Passed
                if self.qualification_id.is_some()
                    && !self.outputs.is_empty()
                    && !self.node_order.is_empty()
                    && self.stop_reason == "completed" => {}
            RunStatus::Blocked
                if self.qualification_id.is_none()
                    && self.outputs.is_empty()
                    && self.node_order.is_empty()
                    && self.stop_reason == "qualification_missing_or_stale" => {}
            _ => return Err(WorkloadError::ResultShape("invalid run result shape")),
        }
        let actual = run_result_digest(self);
        if actual != self.run_id {
            return Err(WorkloadError::ArtifactDigest {
                role: "run result",
                declared: self.run_id.clone(),
                actual,
            });
        }
        Ok(())
    }
}

pub fn built_in_workloads() -> Result<Vec<WorkloadDefinition>, WorkloadError> {
    Ok(vec![text_workload(true)?, text_workload(false)?])
}

pub fn built_in_workload(id: &str) -> Result<WorkloadDefinition, WorkloadError> {
    built_in_workloads()?
        .into_iter()
        .find(|workload| workload.workload.id == id)
        .ok_or_else(|| WorkloadError::UnknownWorkload(id.to_owned()))
}

pub fn execute_workload(
    workload: &WorkloadDefinition,
    inputs: BTreeMap<String, WorkloadValue>,
) -> Result<GraphExecution, WorkloadError> {
    workload.verify()?;
    validate_bindings("run input", &inputs, &workload.inputs)?;
    enforce_value_bytes(&inputs, workload.graph.limits.max_input_bytes, "input")?;
    let node_order = workload.graph.verify(&workload.inputs, &workload.outputs)?;
    let node_by_id = workload
        .graph
        .nodes
        .iter()
        .map(|node| (node.node_id.clone(), node))
        .collect::<BTreeMap<_, _>>();
    let mut node_values = BTreeMap::new();
    for node_id in &node_order {
        let node = node_by_id
            .get(node_id)
            .ok_or_else(|| WorkloadError::UnknownNode(node_id.clone()))?;
        let input = resolve_value(&node.input, &inputs, &node_values)?;
        let output = apply_operation(&node.operation, input)?;
        if output.value_type() != node.value_type {
            return Err(WorkloadError::TypeMismatch(node.node_id.clone()));
        }
        node_values.insert((node.node_id.clone(), node.output_id.clone()), output);
    }
    let mut outputs = BTreeMap::new();
    for output in &workload.graph.outputs {
        outputs.insert(
            output.output_id.clone(),
            resolve_value(&output.source, &inputs, &node_values)?.clone(),
        );
    }
    enforce_value_bytes(&outputs, workload.graph.limits.max_output_bytes, "output")?;
    let execution_id = execution_digest(&workload.graph.graph, &inputs, &node_order, &outputs);
    Ok(GraphExecution {
        execution_id,
        graph: workload.graph.graph.clone(),
        node_order,
        outputs,
    })
}

pub fn test_workload(workload: &WorkloadDefinition) -> Result<WorkloadTestResult, WorkloadError> {
    test_workload_with_observer(workload, |_| {})
}

pub fn test_workload_with_observer(
    workload: &WorkloadDefinition,
    mut observer: impl FnMut(&ScenarioResult),
) -> Result<WorkloadTestResult, WorkloadError> {
    workload.verify()?;
    let campaign_id = campaign_digest(workload);
    let mut scenarios = Vec::with_capacity(workload.scenario_suite.scenarios.len());
    for scenario in &workload.scenario_suite.scenarios {
        let execution = execute_workload(workload, scenario.inputs.clone())?;
        let mut deltas = Vec::with_capacity(scenario.expected_outputs.len());
        for (output_id, expected) in &scenario.expected_outputs {
            let observed = execution
                .outputs
                .get(output_id)
                .ok_or_else(|| WorkloadError::MissingOutput(output_id.clone()))?;
            let delta = compare_scenario_utf8(
                ScenarioDeltaInputs {
                    workload: workload.workload.clone(),
                    graph: workload.graph.graph.clone(),
                    scenario: scenario.scenario.clone(),
                    output_id: output_id.clone(),
                    comparator: workload.evaluator.clone(),
                },
                expected.as_utf8().to_owned(),
                observed.as_utf8().to_owned(),
                workload.limits.scenario_delta.clone(),
            )?;
            deltas.push(delta);
        }
        deltas.sort_by(|left, right| left.inputs.output_id.cmp(&right.inputs.output_id));
        let scenario_result = ScenarioResult {
            scenario: scenario.scenario.clone(),
            required: scenario.required,
            execution_id: execution.execution_id,
            evaluation: scenario_evaluation(&deltas),
            deltas,
        };
        observer(&scenario_result);
        scenarios.push(scenario_result);
    }
    scenarios.sort_by(|left, right| left.scenario.id.cmp(&right.scenario.id));
    let (status, summary) = summarize(&scenarios);
    let stop_reason = match status {
        TestStatus::Passed => "qualified",
        TestStatus::Failed => "conclusive_failure",
        TestStatus::Inconclusive => "inconclusive",
    }
    .to_owned();
    let mut result = WorkloadTestResult {
        schema: WORKLOAD_TEST_RESULT_SCHEMA.to_owned(),
        result_id: placeholder_digest("rey.workload-test-result.placeholder"),
        campaign_id,
        workload: workload.workload.clone(),
        graph: workload.graph.graph.clone(),
        scenario_suite: workload.scenario_suite.suite.clone(),
        evaluator: workload.evaluator.clone(),
        status,
        stop_reason,
        summary,
        scenarios,
        qualification: None,
    };
    result.result_id = test_result_digest(&result);
    if status == TestStatus::Passed {
        let mut qualification = QualificationRecord {
            schema: WORKLOAD_QUALIFICATION_SCHEMA.to_owned(),
            qualification_id: placeholder_digest("rey.workload-qualification.placeholder"),
            workload: workload.workload.clone(),
            graph: workload.graph.graph.clone(),
            scenario_suite: workload.scenario_suite.suite.clone(),
            evaluator: workload.evaluator.clone(),
            test_result_id: result.result_id.clone(),
        };
        qualification.qualification_id = qualification_digest(&qualification);
        result.qualification = Some(qualification);
    }
    result.verify()?;
    Ok(result)
}

pub fn run_workload(
    workload: &WorkloadDefinition,
    qualification: &QualificationRecord,
    inputs: BTreeMap<String, WorkloadValue>,
) -> Result<WorkloadRunResult, WorkloadError> {
    qualification.verify()?;
    if !qualification.is_fresh_for(workload) {
        return Err(WorkloadError::StaleQualification);
    }
    let execution = execute_workload(workload, inputs.clone())?;
    let mut result = WorkloadRunResult {
        schema: WORKLOAD_RUN_RESULT_SCHEMA.to_owned(),
        run_id: placeholder_digest("rey.workload-run-result.placeholder"),
        workload: workload.workload.clone(),
        graph: workload.graph.graph.clone(),
        qualification_id: Some(qualification.qualification_id.clone()),
        status: RunStatus::Passed,
        stop_reason: "completed".to_owned(),
        inputs,
        outputs: execution.outputs,
        node_order: execution.node_order,
    };
    result.run_id = run_result_digest(&result);
    result.verify()?;
    Ok(result)
}

fn text_workload(normalize: bool) -> Result<WorkloadDefinition, WorkloadError> {
    let workload_id = if normalize {
        BUILT_IN_NORMALIZE_WORKLOAD_ID
    } else {
        BUILT_IN_MISMATCH_WORKLOAD_ID
    };
    let mut nodes = Vec::new();
    if normalize {
        nodes.push(GraphNode {
            node_id: "trim".to_owned(),
            operation: trim_contract(),
            input: ValueSource::ExternalInput {
                input_id: INPUT_ID.to_owned(),
            },
            output_id: NODE_OUTPUT_ID.to_owned(),
            value_type: ValueType::Utf8,
        });
        nodes.push(GraphNode {
            node_id: "uppercase".to_owned(),
            operation: uppercase_contract(),
            input: ValueSource::NodeOutput {
                node_id: "trim".to_owned(),
                output_id: NODE_OUTPUT_ID.to_owned(),
            },
            output_id: NODE_OUTPUT_ID.to_owned(),
            value_type: ValueType::Utf8,
        });
    } else {
        nodes.push(GraphNode {
            node_id: "uppercase".to_owned(),
            operation: uppercase_contract(),
            input: ValueSource::ExternalInput {
                input_id: INPUT_ID.to_owned(),
            },
            output_id: NODE_OUTPUT_ID.to_owned(),
            value_type: ValueType::Utf8,
        });
    }
    let graph = ComputeGraph::new(
        &format!("{workload_id}.graph"),
        1,
        nodes,
        vec![GraphOutput {
            output_id: OUTPUT_ID.to_owned(),
            source: ValueSource::NodeOutput {
                node_id: "uppercase".to_owned(),
                output_id: NODE_OUTPUT_ID.to_owned(),
            },
            value_type: ValueType::Utf8,
        }],
        GraphLimits::default(),
    )?;
    let scenarios = vec![
        text_scenario(workload_id, "plain", "spoke", "SPOKE"),
        text_scenario(workload_id, "surrounded", " rey ", "REY"),
    ];
    WorkloadDefinition {
        schema: WORKLOAD_SCHEMA.to_owned(),
        workload: placeholder_contract(workload_id, 1, "rey.workload.placeholder"),
        title: if normalize {
            "Normalize fixture text".to_owned()
        } else {
            "Deliberate fixture mismatch".to_owned()
        },
        inputs: vec![WorkloadPort {
            port_id: INPUT_ID.to_owned(),
            value_type: ValueType::Utf8,
        }],
        outputs: vec![WorkloadPort {
            port_id: OUTPUT_ID.to_owned(),
            value_type: ValueType::Utf8,
        }],
        graph,
        scenario_suite: ScenarioSuite::new(&format!("{workload_id}.scenarios"), scenarios),
        evaluator: utf8_comparator(),
        limits: WorkloadLimits::default(),
    }
    .finalize()
}

fn text_scenario(workload_id: &str, id: &str, input: &str, expected: &str) -> Scenario {
    Scenario::new(
        &format!("{workload_id}.scenario.{id}"),
        true,
        BTreeMap::from([(INPUT_ID.to_owned(), WorkloadValue::Utf8(input.to_owned()))]),
        BTreeMap::from([(
            OUTPUT_ID.to_owned(),
            WorkloadValue::Utf8(expected.to_owned()),
        )]),
    )
}

fn trim_contract() -> ContractIdentity {
    ContractIdentity::new(
        "rey.builtin.utf8.trim",
        1,
        "remove Unicode whitespace from both ends of one UTF-8 value",
    )
}

fn uppercase_contract() -> ContractIdentity {
    ContractIdentity::new(
        "rey.builtin.utf8.uppercase",
        1,
        "apply Rust Unicode uppercase mapping to one UTF-8 value",
    )
}

fn utf8_comparator() -> ContractIdentity {
    ContractIdentity::new(
        "rey.scenario.utf8-exact",
        1,
        "exact UTF-8 expected-to-observed equality",
    )
}

#[derive(Clone, Copy)]
enum BuiltInOperation {
    Trim,
    Uppercase,
}

fn resolve_operation(contract: &ContractIdentity) -> Result<BuiltInOperation, WorkloadError> {
    if contract == &trim_contract() {
        Ok(BuiltInOperation::Trim)
    } else if contract == &uppercase_contract() {
        Ok(BuiltInOperation::Uppercase)
    } else {
        Err(WorkloadError::UnknownOperation(contract.id.clone()))
    }
}

fn apply_operation(
    contract: &ContractIdentity,
    value: &WorkloadValue,
) -> Result<WorkloadValue, WorkloadError> {
    let value = value.as_utf8();
    Ok(match resolve_operation(contract)? {
        BuiltInOperation::Trim => WorkloadValue::Utf8(value.trim().to_owned()),
        BuiltInOperation::Uppercase => WorkloadValue::Utf8(value.to_uppercase()),
    })
}

fn topological_order(
    graph: &ComputeGraph,
    inputs: &BTreeMap<String, ValueType>,
) -> Result<Vec<String>, WorkloadError> {
    let node_by_id = graph
        .nodes
        .iter()
        .map(|node| (node.node_id.clone(), node))
        .collect::<BTreeMap<_, _>>();
    let mut completed = BTreeSet::new();
    let mut depths = BTreeMap::new();
    let mut order = Vec::with_capacity(graph.nodes.len());
    while order.len() < graph.nodes.len() {
        let ready = node_by_id.iter().find(|(id, node)| {
            !completed.contains(*id)
                && source_ready(&node.input, inputs, &completed, &node_by_id).unwrap_or(false)
        });
        let Some((id, node)) = ready else {
            for node in &graph.nodes {
                validate_source(&node.input, inputs, &node_by_id)?;
            }
            return Err(WorkloadError::GraphCycle);
        };
        validate_source(&node.input, inputs, &node_by_id)?;
        let depth = match &node.input {
            ValueSource::ExternalInput { .. } => 1,
            ValueSource::NodeOutput { node_id, .. } => depths
                .get(node_id)
                .copied()
                .unwrap_or(0_u64)
                .checked_add(1)
                .ok_or(WorkloadError::CountOverflow)?,
        };
        if depth > graph.limits.max_depth {
            return Err(WorkloadError::DepthLimit {
                limit: graph.limits.max_depth,
            });
        }
        depths.insert(id.clone(), depth);
        completed.insert(id.clone());
        order.push(id.clone());
    }
    Ok(order)
}

fn source_ready(
    source: &ValueSource,
    inputs: &BTreeMap<String, ValueType>,
    completed: &BTreeSet<String>,
    node_by_id: &BTreeMap<String, &GraphNode>,
) -> Result<bool, WorkloadError> {
    validate_source(source, inputs, node_by_id)?;
    Ok(match source {
        ValueSource::ExternalInput { .. } => true,
        ValueSource::NodeOutput { node_id, .. } => completed.contains(node_id),
    })
}

fn validate_source(
    source: &ValueSource,
    inputs: &BTreeMap<String, ValueType>,
    node_by_id: &BTreeMap<String, &GraphNode>,
) -> Result<(), WorkloadError> {
    match source {
        ValueSource::ExternalInput { input_id } => {
            if inputs.get(input_id) != Some(&ValueType::Utf8) {
                return Err(WorkloadError::UnknownInput(input_id.clone()));
            }
        }
        ValueSource::NodeOutput { node_id, output_id } => {
            let node = node_by_id
                .get(node_id)
                .ok_or_else(|| WorkloadError::UnknownNode(node_id.clone()))?;
            if output_id != &node.output_id || node.value_type != ValueType::Utf8 {
                return Err(WorkloadError::UnknownNodeOutput(format!(
                    "{node_id}.{output_id}"
                )));
            }
        }
    }
    Ok(())
}

fn graph_outputs_by_id(
    outputs: &[GraphOutput],
    node_ids: &BTreeSet<String>,
    inputs: &BTreeMap<String, ValueType>,
) -> Result<BTreeMap<String, ValueType>, WorkloadError> {
    let mut selected = BTreeMap::new();
    for output in outputs {
        validate_text("graph output id", &output.output_id)?;
        match &output.source {
            ValueSource::ExternalInput { input_id } => {
                if inputs.get(input_id) != Some(&output.value_type) {
                    return Err(WorkloadError::UnknownInput(input_id.clone()));
                }
            }
            ValueSource::NodeOutput { node_id, output_id } => {
                if !node_ids.contains(node_id) || output_id != NODE_OUTPUT_ID {
                    return Err(WorkloadError::UnknownNodeOutput(format!(
                        "{node_id}.{output_id}"
                    )));
                }
            }
        }
        if selected
            .insert(output.output_id.clone(), output.value_type)
            .is_some()
        {
            return Err(WorkloadError::DuplicateId(output.output_id.clone()));
        }
    }
    Ok(selected)
}

fn resolve_value<'a>(
    source: &ValueSource,
    inputs: &'a BTreeMap<String, WorkloadValue>,
    node_values: &'a BTreeMap<(String, String), WorkloadValue>,
) -> Result<&'a WorkloadValue, WorkloadError> {
    match source {
        ValueSource::ExternalInput { input_id } => inputs
            .get(input_id)
            .ok_or_else(|| WorkloadError::UnknownInput(input_id.clone())),
        ValueSource::NodeOutput { node_id, output_id } => node_values
            .get(&(node_id.clone(), output_id.clone()))
            .ok_or_else(|| WorkloadError::UnknownNodeOutput(format!("{node_id}.{output_id}"))),
    }
}

fn ports_by_id(
    role: &'static str,
    ports: &[WorkloadPort],
) -> Result<BTreeMap<String, ValueType>, WorkloadError> {
    if ports.is_empty() {
        return Err(WorkloadError::EmptyPorts(role));
    }
    let mut result = BTreeMap::new();
    for port in ports {
        validate_text(role, &port.port_id)?;
        if result
            .insert(port.port_id.clone(), port.value_type)
            .is_some()
        {
            return Err(WorkloadError::DuplicateId(port.port_id.clone()));
        }
    }
    Ok(result)
}

fn validate_bindings(
    role: &'static str,
    bindings: &BTreeMap<String, WorkloadValue>,
    ports: &[WorkloadPort],
) -> Result<(), WorkloadError> {
    let expected = ports_by_id(role, ports)?;
    if bindings.len() != expected.len() {
        return Err(WorkloadError::BindingMismatch(role));
    }
    for (id, value) in bindings {
        if expected.get(id) != Some(&value.value_type()) {
            return Err(WorkloadError::BindingMismatch(role));
        }
    }
    Ok(())
}

fn enforce_value_bytes(
    values: &BTreeMap<String, WorkloadValue>,
    limit: u64,
    role: &'static str,
) -> Result<(), WorkloadError> {
    let observed = values.values().try_fold(0_u64, |total, value| {
        total
            .checked_add(value.byte_len())
            .ok_or(WorkloadError::CountOverflow)
    })?;
    if observed > limit {
        return Err(WorkloadError::ValueByteLimit {
            role,
            limit,
            observed,
        });
    }
    Ok(())
}

fn scenario_evaluation(deltas: &[ScenarioOutputDelta]) -> ScenarioEvaluation {
    if deltas
        .iter()
        .any(|delta| delta.assessment == DeltaAssessment::Different)
    {
        ScenarioEvaluation::Failed
    } else if deltas
        .iter()
        .any(|delta| delta.assessment == DeltaAssessment::Inconclusive)
    {
        ScenarioEvaluation::Inconclusive
    } else {
        ScenarioEvaluation::Passed
    }
}

fn summarize(scenarios: &[ScenarioResult]) -> (TestStatus, TestSummary) {
    let mut summary = TestSummary {
        required: 0,
        passed: 0,
        failed: 0,
        inconclusive: 0,
        evaluated: 0,
        optional: 0,
    };
    for scenario in scenarios {
        if scenario.required {
            summary.required += 1;
            match scenario.evaluation {
                ScenarioEvaluation::Passed => summary.passed += 1,
                ScenarioEvaluation::Failed => summary.failed += 1,
                ScenarioEvaluation::Inconclusive => summary.inconclusive += 1,
            }
            summary.evaluated += 1;
        } else {
            summary.optional += 1;
        }
    }
    let status = if summary.failed > 0 {
        TestStatus::Failed
    } else if summary.inconclusive > 0 || summary.passed != summary.required {
        TestStatus::Inconclusive
    } else {
        TestStatus::Passed
    };
    (status, summary)
}

fn validate_graph_limits(limits: &GraphLimits) -> Result<(), WorkloadError> {
    if limits.max_nodes == 0
        || limits.max_edges == 0
        || limits.max_depth == 0
        || limits.max_input_bytes == 0
        || limits.max_output_bytes == 0
        || limits.max_string_bytes == 0
    {
        return Err(WorkloadError::InvalidLimit);
    }
    Ok(())
}

fn validate_workload_limits(limits: &WorkloadLimits) -> Result<(), WorkloadError> {
    if limits.max_scenarios == 0
        || limits.max_outputs_per_scenario == 0
        || limits.max_string_bytes == 0
        || limits.scenario_delta.max_value_bytes == 0
        || limits.scenario_delta.max_string_bytes == 0
    {
        return Err(WorkloadError::InvalidLimit);
    }
    Ok(())
}

fn enforce_count(role: &'static str, observed: usize, limit: u64) -> Result<(), WorkloadError> {
    if observed as u64 > limit {
        return Err(WorkloadError::CountLimit {
            role,
            limit,
            observed: observed as u64,
        });
    }
    Ok(())
}

fn validate_text(role: &'static str, value: &str) -> Result<(), WorkloadError> {
    if value.is_empty() || value.contains('\0') {
        return Err(WorkloadError::InvalidText { role });
    }
    Ok(())
}

fn validate_contract(role: &'static str, contract: &ContractIdentity) -> Result<(), WorkloadError> {
    validate_text(role, &contract.id)?;
    if contract.revision == 0 {
        return Err(WorkloadError::InvalidContract { role });
    }
    validate_digest(&contract.semantic_digest)
}

fn validate_digest(digest: &SemanticDigest) -> Result<(), WorkloadError> {
    let value = digest.as_str();
    if value.len() != "blake3:".len() + 64
        || !value.starts_with("blake3:")
        || !value["blake3:".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(WorkloadError::InvalidDigest(value.to_owned()));
    }
    Ok(())
}

fn placeholder_digest(domain: &str) -> SemanticDigest {
    SemanticHasher::new(domain).finish()
}

fn placeholder_contract(id: &str, revision: u64, domain: &str) -> ContractIdentity {
    ContractIdentity {
        id: id.to_owned(),
        revision,
        semantic_digest: placeholder_digest(domain),
    }
}

fn add_contract(hasher: &mut SemanticHasher, contract: &ContractIdentity) {
    contract.add_semantics(hasher);
}

fn add_source(hasher: &mut SemanticHasher, source: &ValueSource) {
    match source {
        ValueSource::ExternalInput { input_id } => {
            hasher.add_str("external_input");
            hasher.add_str(input_id);
        }
        ValueSource::NodeOutput { node_id, output_id } => {
            hasher.add_str("node_output");
            hasher.add_str(node_id);
            hasher.add_str(output_id);
        }
    }
}

fn add_value_map(hasher: &mut SemanticHasher, values: &BTreeMap<String, WorkloadValue>) {
    hasher.add_u64(values.len() as u64);
    for (id, value) in values {
        hasher.add_str(id);
        value.add_semantics(hasher);
    }
}

fn graph_digest(graph: &ComputeGraph) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(COMPUTE_GRAPH_SCHEMA);
    hasher.add_str(&graph.graph.id);
    hasher.add_u64(graph.graph.revision);
    hasher.add_u64(graph.nodes.len() as u64);
    for node in &graph.nodes {
        hasher.add_str(&node.node_id);
        add_contract(&mut hasher, &node.operation);
        add_source(&mut hasher, &node.input);
        hasher.add_str(&node.output_id);
        hasher.add_str(node.value_type.as_str());
    }
    hasher.add_u64(graph.outputs.len() as u64);
    for output in &graph.outputs {
        hasher.add_str(&output.output_id);
        add_source(&mut hasher, &output.source);
        hasher.add_str(output.value_type.as_str());
    }
    hasher.add_u64(graph.limits.max_nodes);
    hasher.add_u64(graph.limits.max_edges);
    hasher.add_u64(graph.limits.max_depth);
    hasher.add_u64(graph.limits.max_input_bytes);
    hasher.add_u64(graph.limits.max_output_bytes);
    hasher.add_u64(graph.limits.max_string_bytes);
    hasher.finish()
}

fn scenario_digest(scenario: &Scenario) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.scenario.v1");
    hasher.add_str(&scenario.scenario.id);
    hasher.add_u64(scenario.scenario.revision);
    hasher.add_bool(scenario.required);
    add_value_map(&mut hasher, &scenario.inputs);
    add_value_map(&mut hasher, &scenario.expected_outputs);
    hasher.finish()
}

fn suite_digest(suite: &ScenarioSuite) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(SCENARIO_SUITE_SCHEMA);
    hasher.add_str(&suite.suite.id);
    hasher.add_u64(suite.suite.revision);
    hasher.add_u64(suite.scenarios.len() as u64);
    for scenario in &suite.scenarios {
        add_contract(&mut hasher, &scenario.scenario);
    }
    hasher.finish()
}

fn workload_digest(workload: &WorkloadDefinition) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(WORKLOAD_SCHEMA);
    hasher.add_str(&workload.workload.id);
    hasher.add_u64(workload.workload.revision);
    hasher.add_str(&workload.title);
    hasher.add_u64(workload.inputs.len() as u64);
    for port in &workload.inputs {
        hasher.add_str(&port.port_id);
        hasher.add_str(port.value_type.as_str());
    }
    hasher.add_u64(workload.outputs.len() as u64);
    for port in &workload.outputs {
        hasher.add_str(&port.port_id);
        hasher.add_str(port.value_type.as_str());
    }
    add_contract(&mut hasher, &workload.graph.graph);
    add_contract(&mut hasher, &workload.scenario_suite.suite);
    add_contract(&mut hasher, &workload.evaluator);
    hasher.add_u64(workload.limits.max_scenarios);
    hasher.add_u64(workload.limits.max_outputs_per_scenario);
    hasher.add_u64(workload.limits.max_string_bytes);
    hasher.add_u64(workload.limits.scenario_delta.max_value_bytes);
    hasher.add_u64(workload.limits.scenario_delta.max_string_bytes);
    hasher.finish()
}

fn campaign_digest(workload: &WorkloadDefinition) -> SemanticDigest {
    campaign_digest_from_contracts(
        &workload.workload,
        &workload.graph.graph,
        &workload.scenario_suite.suite,
        &workload.evaluator,
    )
}

fn campaign_digest_from_contracts(
    workload: &ContractIdentity,
    graph: &ContractIdentity,
    scenario_suite: &ContractIdentity,
    evaluator: &ContractIdentity,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.workload-test-campaign.v1");
    add_contract(&mut hasher, workload);
    add_contract(&mut hasher, graph);
    add_contract(&mut hasher, scenario_suite);
    add_contract(&mut hasher, evaluator);
    hasher.finish()
}

fn execution_digest(
    graph: &ContractIdentity,
    inputs: &BTreeMap<String, WorkloadValue>,
    node_order: &[String],
    outputs: &BTreeMap<String, WorkloadValue>,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.graph-execution.v1");
    add_contract(&mut hasher, graph);
    add_value_map(&mut hasher, inputs);
    hasher.add_u64(node_order.len() as u64);
    for node in node_order {
        hasher.add_str(node);
    }
    add_value_map(&mut hasher, outputs);
    hasher.finish()
}

fn test_result_digest(result: &WorkloadTestResult) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(WORKLOAD_TEST_RESULT_SCHEMA);
    hasher.add_str(result.campaign_id.as_str());
    add_contract(&mut hasher, &result.workload);
    add_contract(&mut hasher, &result.graph);
    add_contract(&mut hasher, &result.scenario_suite);
    add_contract(&mut hasher, &result.evaluator);
    hasher.add_str(result.status.as_str());
    hasher.add_str(&result.stop_reason);
    hasher.add_u64(result.summary.required);
    hasher.add_u64(result.summary.passed);
    hasher.add_u64(result.summary.failed);
    hasher.add_u64(result.summary.inconclusive);
    hasher.add_u64(result.summary.evaluated);
    hasher.add_u64(result.summary.optional);
    hasher.add_u64(result.scenarios.len() as u64);
    for scenario in &result.scenarios {
        add_contract(&mut hasher, &scenario.scenario);
        hasher.add_bool(scenario.required);
        hasher.add_str(scenario.execution_id.as_str());
        hasher.add_str(scenario.evaluation.as_str());
        hasher.add_u64(scenario.deltas.len() as u64);
        for delta in &scenario.deltas {
            hasher.add_str(delta.delta_id.as_str());
        }
    }
    hasher.finish()
}

fn qualification_digest(qualification: &QualificationRecord) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(WORKLOAD_QUALIFICATION_SCHEMA);
    add_contract(&mut hasher, &qualification.workload);
    add_contract(&mut hasher, &qualification.graph);
    add_contract(&mut hasher, &qualification.scenario_suite);
    add_contract(&mut hasher, &qualification.evaluator);
    hasher.add_str(qualification.test_result_id.as_str());
    hasher.finish()
}

fn run_result_digest(result: &WorkloadRunResult) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(WORKLOAD_RUN_RESULT_SCHEMA);
    add_contract(&mut hasher, &result.workload);
    add_contract(&mut hasher, &result.graph);
    hasher.add_optional_str(result.qualification_id.as_ref().map(SemanticDigest::as_str));
    hasher.add_str(result.status.as_str());
    hasher.add_str(&result.stop_reason);
    add_value_map(&mut hasher, &result.inputs);
    add_value_map(&mut hasher, &result.outputs);
    hasher.add_u64(result.node_order.len() as u64);
    for node in &result.node_order {
        hasher.add_str(node);
    }
    hasher.finish()
}

fn semantic_string_bytes_graph(graph: &ComputeGraph) -> Result<u64, WorkloadError> {
    let mut bytes = 0;
    add_string_bytes(&mut bytes, &graph.schema)?;
    add_contract_string_bytes(&mut bytes, &graph.graph)?;
    for node in &graph.nodes {
        add_string_bytes(&mut bytes, &node.node_id)?;
        add_contract_string_bytes(&mut bytes, &node.operation)?;
        add_source_string_bytes(&mut bytes, &node.input)?;
        add_string_bytes(&mut bytes, &node.output_id)?;
        add_string_bytes(&mut bytes, node.value_type.as_str())?;
    }
    for output in &graph.outputs {
        add_string_bytes(&mut bytes, &output.output_id)?;
        add_source_string_bytes(&mut bytes, &output.source)?;
        add_string_bytes(&mut bytes, output.value_type.as_str())?;
    }
    Ok(bytes)
}

fn semantic_string_bytes_workload(workload: &WorkloadDefinition) -> Result<u64, WorkloadError> {
    let mut bytes = 0;
    add_string_bytes(&mut bytes, &workload.schema)?;
    add_contract_string_bytes(&mut bytes, &workload.workload)?;
    add_string_bytes(&mut bytes, &workload.title)?;
    for port in workload.inputs.iter().chain(&workload.outputs) {
        add_string_bytes(&mut bytes, &port.port_id)?;
        add_string_bytes(&mut bytes, port.value_type.as_str())?;
    }
    bytes = bytes
        .checked_add(semantic_string_bytes_graph(&workload.graph)?)
        .ok_or(WorkloadError::CountOverflow)?;
    add_string_bytes(&mut bytes, &workload.scenario_suite.schema)?;
    add_contract_string_bytes(&mut bytes, &workload.scenario_suite.suite)?;
    for scenario in &workload.scenario_suite.scenarios {
        add_contract_string_bytes(&mut bytes, &scenario.scenario)?;
        for (id, value) in scenario.inputs.iter().chain(&scenario.expected_outputs) {
            add_string_bytes(&mut bytes, id)?;
            add_string_bytes(&mut bytes, value.value_type().as_str())?;
            add_string_bytes(&mut bytes, value.as_utf8())?;
        }
    }
    add_contract_string_bytes(&mut bytes, &workload.evaluator)?;
    Ok(bytes)
}

fn add_string_bytes(total: &mut u64, value: &str) -> Result<(), WorkloadError> {
    *total = total
        .checked_add(value.len() as u64)
        .ok_or(WorkloadError::CountOverflow)?;
    Ok(())
}

fn add_contract_string_bytes(
    total: &mut u64,
    contract: &ContractIdentity,
) -> Result<(), WorkloadError> {
    add_string_bytes(total, &contract.id)?;
    add_string_bytes(total, contract.semantic_digest.as_str())
}

fn add_source_string_bytes(total: &mut u64, source: &ValueSource) -> Result<(), WorkloadError> {
    match source {
        ValueSource::ExternalInput { input_id } => add_string_bytes(total, input_id),
        ValueSource::NodeOutput { node_id, output_id } => {
            add_string_bytes(total, node_id)?;
            add_string_bytes(total, output_id)
        }
    }
}

#[derive(Debug, Error)]
pub enum WorkloadError {
    #[error("workload limits must be greater than zero")]
    InvalidLimit,
    #[error("invalid {role} contract")]
    InvalidContract { role: &'static str },
    #[error("invalid {role} text")]
    InvalidText { role: &'static str },
    #[error("invalid semantic digest {0}")]
    InvalidDigest(String),
    #[error("unsupported schema {actual}; expected {expected}")]
    UnsupportedSchema {
        expected: &'static str,
        actual: String,
    },
    #[error("duplicate identity {0}")]
    DuplicateId(String),
    #[error("{0} ports must not be empty")]
    EmptyPorts(&'static str),
    #[error("compute graph must contain at least one node")]
    EmptyGraph,
    #[error("scenario suite must contain at least one scenario")]
    EmptyScenarioSuite,
    #[error("scenario suite must contain at least one required scenario")]
    NoRequiredScenario,
    #[error("compute graph contains a cycle")]
    GraphCycle,
    #[error("unknown workload {0}")]
    UnknownWorkload(String),
    #[error("unknown built-in operation {0}")]
    UnknownOperation(String),
    #[error("unknown workload input {0}")]
    UnknownInput(String),
    #[error("unknown graph node {0}")]
    UnknownNode(String),
    #[error("unknown graph node output {0}")]
    UnknownNodeOutput(String),
    #[error("missing graph output {0}")]
    MissingOutput(String),
    #[error("type mismatch at {0}")]
    TypeMismatch(String),
    #[error("graph selected outputs do not match the workload output contract")]
    OutputContractMismatch,
    #[error("{0} bindings do not exactly match the declared ports")]
    BindingMismatch(&'static str),
    #[error("{role} count limit {limit} exceeded by {observed}")]
    CountLimit {
        role: &'static str,
        limit: u64,
        observed: u64,
    },
    #[error("graph depth limit {limit} exceeded")]
    DepthLimit { limit: u64 },
    #[error("{role} byte limit {limit} exceeded by {observed}")]
    ValueByteLimit {
        role: &'static str,
        limit: u64,
        observed: u64,
    },
    #[error("workload string-byte limit {limit} exceeded")]
    StringByteLimit { limit: u64 },
    #[error("workload count overflowed")]
    CountOverflow,
    #[error("{role} contract digest mismatch: declared {declared}, actual {actual}")]
    ContractDigest {
        role: &'static str,
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("{role} digest mismatch: declared {declared}, actual {actual}")]
    ArtifactDigest {
        role: &'static str,
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("invalid retained result: {0}")]
    ResultShape(&'static str),
    #[error("qualification is stale for the selected workload")]
    StaleQualification,
    #[error(transparent)]
    ScenarioDelta(#[from] rey_diff::ScenarioDeltaError),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rey_core::{ContractIdentity, SemanticHasher};
    use rey_diff::DeltaAssessment;

    use super::{
        BUILT_IN_MISMATCH_WORKLOAD_ID, BUILT_IN_NORMALIZE_WORKLOAD_ID, GraphLimits, RunStatus,
        TestStatus, WorkloadError, WorkloadValue, built_in_workload, built_in_workloads,
        execute_workload, run_workload, test_workload, test_workload_with_observer,
    };

    #[test]
    fn built_in_catalog_is_canonical_and_verified() {
        let workloads = built_in_workloads().unwrap();
        assert_eq!(workloads.len(), 2);
        assert_eq!(workloads[0].workload.id, BUILT_IN_NORMALIZE_WORKLOAD_ID);
        assert_eq!(workloads[1].workload.id, BUILT_IN_MISMATCH_WORKLOAD_ID);
        for workload in workloads {
            workload.verify().unwrap();
        }
    }

    #[test]
    fn stable_dag_execution_normalizes_text() {
        let workload = built_in_workload(BUILT_IN_NORMALIZE_WORKLOAD_ID).unwrap();
        let execution = execute_workload(
            &workload,
            BTreeMap::from([("text".to_owned(), WorkloadValue::Utf8(" rey ".to_owned()))]),
        )
        .unwrap();
        assert_eq!(execution.node_order, ["trim", "uppercase"]);
        assert_eq!(
            execution.outputs["text"],
            WorkloadValue::Utf8("REY".to_owned())
        );
    }

    #[test]
    fn passing_and_failing_scenarios_retain_typed_deltas() {
        let passing =
            test_workload(&built_in_workload(BUILT_IN_NORMALIZE_WORKLOAD_ID).unwrap()).unwrap();
        assert_eq!(passing.status, TestStatus::Passed);
        assert_eq!(passing.summary.passed, 2);
        assert!(passing.qualification.is_some());
        passing.verify().unwrap();

        let failing =
            test_workload(&built_in_workload(BUILT_IN_MISMATCH_WORKLOAD_ID).unwrap()).unwrap();
        assert_eq!(failing.status, TestStatus::Failed);
        assert_eq!(failing.summary.passed, 1);
        assert_eq!(failing.summary.failed, 1);
        assert!(failing.qualification.is_none());
        assert!(
            failing
                .scenarios
                .iter()
                .flat_map(|scenario| &scenario.deltas)
                .any(|delta| delta.assessment == DeltaAssessment::Different)
        );
        failing.verify().unwrap();
    }

    #[test]
    fn scenario_observer_follows_declaration_order_without_changing_results() {
        let workload = built_in_workload(BUILT_IN_NORMALIZE_WORKLOAD_ID).unwrap();
        let mut observed = Vec::new();
        let result = test_workload_with_observer(&workload, |scenario| {
            observed.push(scenario.scenario.id.clone());
        })
        .unwrap();

        assert_eq!(
            observed,
            result
                .scenarios
                .iter()
                .map(|scenario| scenario.scenario.id.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(result, test_workload(&workload).unwrap());
    }

    #[test]
    fn run_requires_and_binds_fresh_qualification() {
        let workload = built_in_workload(BUILT_IN_NORMALIZE_WORKLOAD_ID).unwrap();
        let test = test_workload(&workload).unwrap();
        let run = run_workload(
            &workload,
            test.qualification.as_ref().unwrap(),
            BTreeMap::from([("text".to_owned(), WorkloadValue::Utf8(" spoke ".to_owned()))]),
        )
        .unwrap();
        assert_eq!(run.status, RunStatus::Passed);
        assert_eq!(run.outputs["text"], WorkloadValue::Utf8("SPOKE".to_owned()));
        run.verify().unwrap();
    }

    #[test]
    fn invalid_graphs_and_tampered_results_fail_closed() {
        let mut workload = built_in_workload(BUILT_IN_NORMALIZE_WORKLOAD_ID).unwrap();
        workload.graph.nodes[0].input = super::ValueSource::NodeOutput {
            node_id: "uppercase".to_owned(),
            output_id: "value".to_owned(),
        };
        workload.graph.nodes[1].input = super::ValueSource::NodeOutput {
            node_id: "trim".to_owned(),
            output_id: "value".to_owned(),
        };
        assert!(matches!(workload.verify(), Err(WorkloadError::GraphCycle)));

        let workload = built_in_workload(BUILT_IN_NORMALIZE_WORKLOAD_ID).unwrap();
        let mut result = test_workload(&workload).unwrap();
        result.result_id = SemanticHasher::new("tampered").finish();
        assert!(result.verify().is_err());
    }

    #[test]
    fn operation_and_execution_bounds_fail_closed() {
        let mut workload = built_in_workload(BUILT_IN_NORMALIZE_WORKLOAD_ID).unwrap();
        workload.graph.nodes[0].operation = ContractIdentity::new("unknown", 1, "unknown");
        assert!(matches!(
            workload.verify(),
            Err(WorkloadError::UnknownOperation(_))
        ));

        let workload = built_in_workload(BUILT_IN_NORMALIZE_WORKLOAD_ID).unwrap();
        assert!(
            execute_workload(
                &workload,
                BTreeMap::from([(
                    "text".to_owned(),
                    WorkloadValue::Utf8(
                        "x".repeat(GraphLimits::default().max_input_bytes as usize + 1)
                    ),
                )]),
            )
            .is_err()
        );
    }
}
