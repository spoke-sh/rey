import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { WorkloadList, WorkloadSummary } from "./domain";
import {
  AdmittedWorkloadDetail,
  DraftWorkloadDetail,
  WorkloadsPage,
} from "./workloads";

describe("workload portfolio tables", () => {
  it("aligns admitted and requested workloads as dense evidence relations", () => {
    const markup = renderToStaticMarkup(
      createElement(WorkloadsPage, { portfolio: portfolio() }),
    );

    expect(markup.match(/data-kinetic-dense-table=""/g)).toHaveLength(3);
    expect(markup).toContain('<table aria-label="Incoming workload revisions"');
    expect(markup).toContain('<table aria-label="Admitted workload revisions"');
    expect(markup).toContain('<table aria-label="Workload creation requests"');
    for (const evidence of [
      "WORKLOAD / REVISION",
      "JOURNEY / STATE",
      "LOCAL CONFORMANCE",
      "GRAPH / EVIDENCE",
      "MINING / ATTENTION",
      "REVISE GRAPH",
      "FAILING · FRESH",
      "1/2 passing",
      "1 failed · 0 inconclusive · 0 stale",
      "example.graph@3",
      "2/3 RESULTS",
      "2 deltas · 4 surfaces",
      "AWAITING HARNESS",
      "ORACLE / NOT ADMITTED",
      "workloads/requested/workload.yaml",
    ]) {
      expect(markup).toContain(evidence);
    }
    expect(markup).toContain('href="/workloads/rey.example"');
    expect(markup).toContain('href="/workloads/rey.requested"');
    expect(markup).not.toContain("OPEN MECHANISM");
  });

  it("declares both bounded empty relations", () => {
    const document = portfolio();
    document.workloads = [];
    document.drafts = [];
    document.catalog.admitted_count = 0;
    document.catalog.draft_count = 0;
    const markup = renderToStaticMarkup(
      createElement(WorkloadsPage, { portfolio: document }),
    );

    expect(markup).toContain("NO WORKLOAD REVISION IS WAITING FOR ADMISSION");
    expect(markup).toContain("NO WORKLOAD PACKAGES HAVE BEEN ADMITTED");
    expect(markup).toContain("NO WORKLOADS AWAITING CODING HARNESS");
  });

  it("projects an admitted workload as posture, binding, and mining relations", () => {
    const markup = renderToStaticMarkup(
      createElement(AdmittedWorkloadDetail, { workload: workload() }),
    );

    expect(markup.match(/data-kinetic-dense-table=""/g)).toHaveLength(3);
    for (const evidence of [
      '<table aria-label="rey.example runtime posture"',
      '<table aria-label="rey.example exact bindings"',
      '<table aria-label="rey.example mining evidence"',
      "01 / RUNTIME POSTURE",
      "SCENARIO OUTCOMES",
      "RUN / ATTENTION",
      "02 / EXACT BINDINGS",
      "CANDIDATE GRAPH",
      "CONTENT IDENTITY",
      "blake3:graph",
      "workloads/example/workload.yaml",
      "blake3:test",
      "03 / MINING / EVIDENCE",
      "Artifact output",
    ]) {
      expect(markup).toContain(evidence);
    }
    expect(markup).toContain('href="/workloads"');
  });

  it("projects a creation request as posture and exact handoff bindings", () => {
    const draft = portfolio().drafts[0]!;
    const markup = renderToStaticMarkup(
      createElement(DraftWorkloadDetail, { draft }),
    );

    expect(markup.match(/data-kinetic-dense-table=""/g)).toHaveLength(2);
    for (const evidence of [
      '<table aria-label="rey.requested request posture"',
      '<table aria-label="rey.requested request bindings"',
      "01 / REQUEST POSTURE",
      "AWAITING CODING HARNESS",
      "MISSING",
      "NOT ADMITTED",
      "02 / REQUEST BINDINGS",
      "SOURCE IDENTITY",
      ".rey/workload-requests/rey.requested.yaml",
      "blake3:request-source",
      "workloads/requested/workload.yaml",
    ]) {
      expect(markup).toContain(evidence);
    }
    expect(markup).toContain('href="/workloads"');
  });
});

function portfolio(): WorkloadList {
  return {
    schema: "rey.workload-list.v1",
    semantic_atlas: null,
    scene_admissions: [],
    catalog: {
      schema: "rey.workload-catalog.v1",
      kind: "workspace_packages",
      root: "workloads",
      workload_count: 2,
      admitted_count: 1,
      draft_count: 1,
    },
    workloads: [workload()],
    drafts: [
      {
        request: {
          request_id: "blake3:request",
          workload_id: "rey.requested",
          title: "Map the requested surface",
          intent: "Generate the exact graph and frozen scenarios.",
          proposer: "coding_harness",
          target_package: "workloads/requested/workload.yaml",
        },
        source: ".rey/workload-requests/rey.requested.yaml",
        source_digest: "blake3:request-source",
      },
    ],
    attention: {
      schema: "rey.workload-attention.v1",
      attention_id: "blake3:attention",
      source_snapshot_id: "blake3:source",
      rows: [],
      summary: {
        refine: 1,
        retest: 0,
        create: 1,
        blocked: 0,
        policy_excluded: 0,
        workloads: 2,
        surfaces: 0,
        owned_surfaces: 0,
        unowned_surfaces: 0,
      },
    },
  };
}

function workload(): WorkloadSummary {
  return {
    provenance: {
      origin: "workspace_package",
      source: "workloads/example/workload.yaml",
      source_digest: "blake3:package",
      generation: null,
      admission: { state: "accepted", scenario_oracle: "frozen" },
    },
    workload: {
      id: "rey.example",
      revision: 2,
      semantic_digest: "blake3:workload",
    },
    title: "Example workload",
    candidate_graph: {
      id: "example.graph",
      revision: 3,
      semantic_digest: "blake3:graph",
    },
    freshness: "fresh",
    qualification: "failing",
    required: 2,
    passed: 1,
    failed: 1,
    inconclusive: 0,
    evaluated: 2,
    stale: 0,
    optional: 0,
    mining_operations: 3,
    mining_results: 2,
    incomplete_mining_results: 1,
    relation_deltas: 2,
    reasoning_surfaces: 4,
    attention_rows: 1,
    topography_results: 0,
    topography_revision: null,
    topography_coverage: null,
    topography_frontier_rows: 0,
    topography_patch: null,
    topography_projection: null,
    last_run_status: "blocked",
    last_test_result_id: "blake3:test",
  };
}
