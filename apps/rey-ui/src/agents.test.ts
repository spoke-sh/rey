import { describe, expect, it } from "vitest";
import { deriveSystemRecommendations, deriveWorkInsights } from "./agents";
import type { WorkloadList, WorkloadSummary } from "./domain";

describe("agent collaboration intelligence", () => {
  it("ranks typed recommendations without duplicating request attention", () => {
    const portfolio = emptyPortfolio();
    portfolio.drafts.push({
      request: {
        request_id: "request:alpha",
        workload_id: "alpha",
        title: "Author alpha",
        intent: "Create a graph over the surveyed artifacts",
        proposer: "coding_harness",
        target_package: "workloads/alpha/workload.yaml",
      },
      source: ".rey/workload-requests/alpha.yaml",
      source_digest: "blake3:request",
    });
    portfolio.attention.rows.push(
      {
        row_id: "attention:alpha",
        action: "create",
        subject_kind: "workload",
        subject_id: "alpha",
        reason: "workload graph missing",
        readiness: "ready",
        evidence_ids: ["evidence:survey"],
        dependency_ids: [],
        priority: 8,
        estimated_cost_units: 3,
      },
      {
        row_id: "attention:surface",
        action: "block",
        subject_kind: "surface",
        subject_id: "source:unknown",
        reason: "locator missing",
        readiness: "blocked",
        evidence_ids: [],
        dependency_ids: ["locator:required"],
        priority: 12,
        estimated_cost_units: 2,
      },
    );

    expect(deriveSystemRecommendations(portfolio)).toMatchObject([
      {
        id: "request:alpha",
        operation: "AUTHOR",
        profile: "CODING HARNESS",
        source: "REQUEST + ATTENTION",
        evidence_count: 1,
        readiness: "ready",
      },
      {
        id: "attention:surface",
        operation: "RESOLVE",
        profile: "SURVEY / OPERATOR",
        dependency_count: 1,
        readiness: "blocked",
      },
    ]);
  });

  it("reports observed work from retained results rather than agent activity", () => {
    const portfolio = emptyPortfolio();
    portfolio.workloads.push(workload());

    expect(deriveWorkInsights(portfolio)).toEqual([
      expect.objectContaining({
        workload_id: "alpha",
        kind: "ADMITTED",
        observed_operation: "RUN",
        result: "PASSED",
        journey: "RUN COMPLETE",
        scenarios_passed: 3,
        scenarios_required: 4,
        artifact_summary: "2 mining · 1 deltas · 1 surfaces",
        evidence_id: "test:alpha",
      }),
    ]);
  });
});

function workload(): WorkloadSummary {
  return {
    provenance: null,
    workload: { id: "alpha", revision: 2, semantic_digest: "blake3:workload" },
    title: "Alpha workload",
    candidate_graph: {
      id: "alpha.graph",
      revision: 3,
      semantic_digest: "blake3:graph",
    },
    freshness: "fresh",
    qualification: "qualified",
    required: 4,
    passed: 3,
    failed: 0,
    inconclusive: 0,
    evaluated: 3,
    stale: 0,
    optional: 1,
    mining_operations: 2,
    mining_results: 2,
    incomplete_mining_results: 0,
    relation_deltas: 1,
    reasoning_surfaces: 1,
    attention_rows: 0,
    last_run_status: "passed",
    last_test_result_id: "test:alpha",
  };
}

function emptyPortfolio(): WorkloadList {
  return {
    schema: "rey.workload-list.v5",
    catalog: {
      schema: "rey.workload-catalog.v1",
      kind: "workspace_packages",
      root: "workloads",
      workload_count: 0,
      admitted_count: 0,
      draft_count: 0,
    },
    workloads: [],
    drafts: [],
    attention: {
      schema: "rey.workload-attention.v1",
      attention_id: "blake3:attention",
      source_snapshot_id: "blake3:source",
      rows: [],
      summary: {
        refine: 0,
        retest: 0,
        create: 0,
        blocked: 0,
        policy_excluded: 0,
        workloads: 0,
        surfaces: 0,
        owned_surfaces: 0,
        unowned_surfaces: 0,
      },
    },
  };
}
