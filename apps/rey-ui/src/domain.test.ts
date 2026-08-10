import { describe, expect, it } from "vitest";
import {
  derivePortfolioMetrics,
  scenarioPercent,
  workloadJourney,
  type WorkloadList,
  type WorkloadSummary,
} from "./domain";

const summary = (
  qualification: WorkloadSummary["qualification"],
): WorkloadSummary => ({
  provenance: null,
  workload: {
    id: `rey.${qualification}`,
    revision: 1,
    semantic_digest: `digest-${qualification}`,
  },
  title: qualification,
  candidate_graph: {
    id: "graph",
    revision: 1,
    semantic_digest: "graph-digest",
  },
  freshness: qualification === "stale" ? "stale" : "fresh",
  qualification,
  required: 2,
  passed: qualification === "qualified" ? 2 : 1,
  failed: qualification === "failing" ? 1 : 0,
  inconclusive: qualification === "inconclusive" ? 1 : 0,
  evaluated: 2,
  stale: qualification === "stale" ? 2 : 0,
  optional: 0,
  mining_operations: 0,
  mining_results: 0,
  incomplete_mining_results: 0,
  relation_deltas: 0,
  reasoning_surfaces: 0,
  attention_rows: 0,
  last_run_status: qualification === "qualified" ? "passed" : null,
  last_test_result_id: null,
});

const portfolio = (workloads: WorkloadSummary[]): WorkloadList => ({
  schema: "rey.workload-list.v5",
  catalog: {
    schema: "rey.workload-catalog.v2",
    kind: "workspace_packages",
    root: "workloads",
    workload_count: workloads.length + 1,
    admitted_count: workloads.length,
    draft_count: 1,
  },
  workloads,
  drafts: [],
  attention: {
    schema: "rey.workload-attention.v1",
    attention_id: "attention",
    source_snapshot_id: "snapshot",
    rows: [],
    summary: {
      refine: 0,
      retest: 0,
      create: 0,
      blocked: 0,
      policy_excluded: 0,
      workloads: workloads.length,
      surfaces: 0,
      owned_surfaces: 0,
      unowned_surfaces: 0,
    },
  },
});

describe("portfolio projection", () => {
  it("keeps qualification, scenario, run, and draft dimensions separate", () => {
    const metrics = derivePortfolioMetrics(
      portfolio([
        summary("qualified"),
        summary("failing"),
        summary("inconclusive"),
        summary("stale"),
      ]),
    );

    expect(metrics).toMatchObject({
      total: 5,
      admitted: 4,
      drafts: 1,
      qualified: 1,
      failing: 2,
      stale: 1,
      scenariosPassed: 5,
      scenariosRequired: 8,
      runsPassed: 1,
      runsPending: 3,
    });
  });

  it("derives journeys without collapsing run state into qualification", () => {
    expect(workloadJourney(summary("untested"))).toBe("TEST");
    expect(workloadJourney(summary("failing"))).toBe("REVISE GRAPH");
    expect(workloadJourney(summary("inconclusive"))).toBe("RESTORE EVIDENCE");
    expect(workloadJourney(summary("stale"))).toBe("RETEST");
    expect(workloadJourney(summary("qualified"))).toBe("RUN COMPLETE");
  });

  it("keeps typed empty scenario coverage at zero", () => {
    expect(scenarioPercent(0, 0)).toBe(0);
    expect(scenarioPercent(2, 3)).toBe(67);
  });
});
