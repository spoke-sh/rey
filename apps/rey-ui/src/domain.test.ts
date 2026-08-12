import { describe, expect, it } from "vitest";
import {
  deriveAgentIndex,
  derivePortfolioMetrics,
  operatorMailboxRows,
  scenarioPercent,
  shortDigest,
  sourceCommitUrl,
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
  topography_results: 0,
  topography_revision: null,
  topography_coverage: null,
  topography_frontier_rows: 0,
  topography_patch: null,
  topography_projection: null,
  last_run_status: qualification === "qualified" ? "passed" : null,
  last_test_result_id: null,
});

const portfolio = (workloads: WorkloadSummary[]): WorkloadList => ({
  schema: "rey.workload-list.v1",
  semantic_atlas: null,
  scene_admissions: [],
  catalog: {
    schema: "rey.workload-catalog.v1",
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

  it("keeps policy-excluded rows out of the live operator mailbox", () => {
    const document = portfolio([]);
    document.attention.rows = [
      {
        row_id: "ready",
        action: "create",
        subject_kind: "surface",
        subject_id: "src/lib.rs",
        reason: "unowned surface",
        readiness: "ready",
        evidence_ids: ["surface:1"],
        dependency_ids: [],
        priority: 10,
        estimated_cost_units: 1,
      },
      {
        row_id: "excluded",
        action: "policy_excluded",
        subject_kind: "workload",
        subject_id: "fixture",
        reason: "diagnostic catalog",
        readiness: "excluded",
        evidence_ids: [],
        dependency_ids: [],
        priority: 0,
        estimated_cost_units: 0,
      },
    ];

    expect(operatorMailboxRows(document).map((row) => row.row_id)).toEqual([
      "ready",
    ]);
  });

  it("keeps typed empty scenario coverage at zero", () => {
    expect(scenarioPercent(0, 0)).toBe(0);
    expect(scenarioPercent(2, 3)).toBe(67);
  });

  it("links the short footer label through the complete source commit", () => {
    const revision = "02ad6ed24744dbeabb0b8bef5a64d547f424d9a3";
    expect(shortDigest(revision)).toBe("02ad6ed2…24d9a3");
    expect(sourceCommitUrl("https://github.com/example/rey", revision)).toBe(
      `https://github.com/example/rey/commit/${revision}`,
    );
    expect(sourceCommitUrl("https://github.com/example/rey", "unknown")).toBe(
      null,
    );
  });

  it("indexes exact generator identities without merging producer revisions", () => {
    const first = summary("qualified");
    first.provenance = {
      origin: "workspace_package",
      source: "workloads/first/workload.yaml",
      source_digest: "package:first",
      generation: {
        kind: "coding_harness",
        producer: "codex",
        producer_revision: "gpt-5",
      },
      admission: { state: "accepted", scenario_oracle: "frozen" },
    };
    first.attention_rows = 2;
    const second = summary("failing");
    second.provenance = {
      ...first.provenance,
      source: "workloads/second/workload.yaml",
      source_digest: "package:second",
    };
    const previous = summary("stale");
    previous.provenance = {
      ...first.provenance,
      source: "workloads/previous/workload.yaml",
      source_digest: "package:previous",
      generation: {
        kind: "coding_harness",
        producer: "codex",
        producer_revision: "gpt-4.9",
      },
    };

    const agents = deriveAgentIndex(portfolio([first, second, previous]));

    expect(agents).toHaveLength(2);
    expect(agents[1]).toMatchObject({
      id: "coding_harness:codex@gpt-5",
      workload_ids: ["rey.failing", "rey.qualified"],
      scenarios_passed: 3,
      scenarios_required: 4,
      attention_rows: 2,
    });
  });
});
