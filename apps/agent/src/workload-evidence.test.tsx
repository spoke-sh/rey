import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import {
  DeltaEvidencePage,
  ScenarioEvidencePage,
  WorkloadEvidenceIndexSection,
  type WorkloadDeltaEvidence,
  type WorkloadEvidenceIndex,
  type WorkloadScenarioEvidence,
} from "./workload-evidence";

describe("exact workload evidence routes", () => {
  it("projects retained scenario references as exact content-addressed routes", () => {
    const markup = renderToStaticMarkup(
      createElement(WorkloadEvidenceIndexSection, { evidence: index() }),
    );

    expect(markup).toContain("04 / SCENARIO EVIDENCE");
    expect(markup).toContain("PLAIN → -V → -VV");
    expect(markup).toContain("rey.example.scenario.mismatch@1");
    expect(markup).toContain("FAILED");
    expect(markup).toContain(
      'href="/workloads/rey.example/scenarios/blake3:execution"',
    );
    expect(markup).toContain("verified_retained_result_projection");
  });

  it("retains the CLI plain, -v, and -vv layers on a scenario route", () => {
    const markup = renderToStaticMarkup(
      createElement(ScenarioEvidencePage, { evidence: scenarioEvidence() }),
    );

    for (const value of [
      "PLAIN / OUTCOME",
      "FAILED · rey.example.scenario.mismatch",
      "-V / ASSERTIONS",
      "EXPECTED",
      "ACTUAL",
      "- REY",
      "+  REY ",
      "-VV / EXACT EVIDENCE",
      "SOURCE REVISION",
      "blake3:package",
      "SCENARIO LIMITS",
      "NO EXECUTION, QUALIFICATION, ADMISSION, ACTION, OR PROOF AUTHORITY",
    ]) {
      expect(markup.toUpperCase()).toContain(value.toUpperCase());
    }
    expect(markup).toContain(
      'href="/workloads/rey.example/deltas/blake3%3Adelta"',
    );
  });

  it("opens one exact directed delta without recomputing its assessment", () => {
    const markup = renderToStaticMarkup(
      createElement(DeltaEvidencePage, { evidence: deltaEvidence() }),
    );

    expect(markup).toContain("PLAIN / DIRECTED DELTA");
    expect(markup).toContain("output.text · DIFFERENT");
    expect(markup).toContain("-V / PROJECTION");
    expect(markup).toContain("rey.text-delta.v1");
    expect(markup).toContain("-VV / EXACT EVIDENCE");
    expect(markup).toContain("blake3:text-delta");
    expect(markup).toContain("max_alignment_cells=1000");
    expect(markup).toContain(
      'href="/workloads/rey.example/scenarios/blake3:execution"',
    );
  });
});

function identity(id: string, digest = `blake3:${id}`) {
  return { id, revision: 1, semantic_digest: digest };
}

function current() {
  return {
    workload: identity("rey.example", "blake3:workload"),
    graph: identity("rey.example.graph", "blake3:graph"),
    scenario_suite: identity("rey.example.scenarios", "blake3:suite"),
    evaluator: identity("rey.scenario.utf8-exact", "blake3:evaluator"),
    source: {
      origin: "workspace_package" as const,
      source: "sys/rey.example/workload.yaml",
      source_digest: "blake3:package",
      generation: {
        kind: "coding_harness" as const,
        producer: "codex",
        producer_revision: "gpt-5",
      },
      admission: {
        state: "accepted" as const,
        scenario_oracle: "frozen" as const,
      },
    },
    limits: {
      max_scenarios: 64,
      max_outputs_per_scenario: 16,
      max_owned_surfaces: 64,
      max_git_dependencies: 64,
      max_required_capabilities: 256,
      max_string_bytes: 524288,
      scenario_delta: scenarioLimits(),
    },
  };
}

function result() {
  const exact = current();
  return {
    result_id: "blake3:test",
    campaign_id: "blake3:campaign",
    workload: exact.workload,
    graph: exact.graph,
    scenario_suite: exact.scenario_suite,
    evaluator: exact.evaluator,
    status: "failed" as const,
    stop_reason: "conclusive_failure",
  };
}

function scenarioLimits() {
  return {
    max_value_bytes: 65536,
    max_lines: 4096,
    max_alignment_cells: 1000,
    max_changes: 8192,
    max_string_bytes: 262144,
  };
}

function scenarioOutputDelta() {
  const exact = current();
  const scenario = identity("rey.example.scenario.mismatch", "blake3:scenario");
  return {
    schema: "rey.scenario-output-delta.v1" as const,
    delta_id: "blake3:delta",
    inputs: {
      workload: exact.workload,
      graph: exact.graph,
      scenario,
      output_id: "text",
      comparator: exact.evaluator,
    },
    value_type: "utf8" as const,
    expected: "REY",
    observed: " REY ",
    assessment: "different" as const,
    text_delta: {
      schema: "rey.text-delta.v1" as const,
      delta_id: "blake3:text-delta",
      inputs: {
        source_artifact_id: "blake3:expected",
        target_artifact_id: "blake3:actual",
        source_label: "EXPECTED",
        target_label: "OBSERVED",
        comparator: exact.evaluator,
        encoding: "utf-8",
        segmentation: "lines-preserve-terminators",
      },
      assessment: "different" as const,
      source_line_count: 1,
      target_line_count: 1,
      source_final_newline: false,
      target_final_newline: false,
      hunks: [
        {
          source_start_line: 1,
          source_line_count: 1,
          target_start_line: 1,
          target_line_count: 1,
          lines: [
            {
              kind: "delete" as const,
              source_line: 1,
              target_line: null,
              text: "REY",
            },
            {
              kind: "insert" as const,
              source_line: null,
              target_line: 1,
              text: " REY ",
            },
          ],
        },
      ],
      limits: {
        max_input_bytes: 65536,
        max_lines: 4096,
        max_alignment_cells: 1000,
        max_changes: 8192,
        max_string_bytes: 262144,
      },
    },
    limits: scenarioLimits(),
  };
}

function index(): WorkloadEvidenceIndex {
  return {
    schema: "rey.ui-workload-evidence-index.v1",
    authority:
      "verified_retained_result_projection; read_only; no execution, qualification, admission, action, or proof authority",
    workload_id: "rey.example",
    availability: "retained",
    freshness: "fresh",
    source_binding: "exact_current",
    current: current(),
    result: result(),
    scenarios: [
      {
        scenario: identity("rey.example.scenario.mismatch", "blake3:scenario"),
        required: true,
        execution_id: "blake3:execution",
        evaluation: "failed",
        route: "/workloads/rey.example/scenarios/blake3:execution",
        deltas: [
          {
            kind: "scenario_output",
            delta_id: "blake3:delta",
            label: "output.text",
            assessment: "different",
            route: "/workloads/rey.example/deltas/blake3:delta",
          },
        ],
      },
    ],
  };
}

function scenarioEvidence(): WorkloadScenarioEvidence {
  const reference = index().scenarios[0]!;
  return {
    schema: "rey.ui-workload-scenario-evidence.v1",
    authority:
      "verified_retained_result_projection; read_only; no execution, qualification, admission, action, or proof authority",
    freshness: "fresh",
    source_binding: "exact_current",
    current: current(),
    result: result(),
    scenario: {
      scenario: reference.scenario,
      required: true,
      execution_id: reference.execution_id,
      evaluation: "failed",
      deltas: [scenarioOutputDelta()],
      mining: [],
      topography: [],
      attention: [],
    },
    deltas: reference.deltas,
  };
}

function deltaEvidence(): WorkloadDeltaEvidence {
  const scenario = scenarioEvidence();
  return {
    schema: "rey.ui-workload-delta-evidence.v1",
    authority: scenario.authority,
    freshness: "fresh",
    source_binding: "exact_current",
    current: scenario.current,
    result: scenario.result,
    scenario: scenario.scenario.scenario,
    scenario_execution_id: scenario.scenario.execution_id,
    scenario_route: "/workloads/rey.example/scenarios/blake3:execution",
    delta_id: "blake3:delta",
    evidence: { kind: "scenario_output", delta: scenarioOutputDelta() },
  };
}
