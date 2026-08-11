import { describe, expect, it } from "vitest";
import type { WorkloadList } from "./domain";
import {
  agentExplorerView,
  explorerCoordinateUri,
  explorerViewPath,
  parseExplorerCoordinate,
  parseExplorerView,
  resolveExplorerView,
} from "./explorer-coordinate";

const portfolio: WorkloadList = {
  schema: "rey.workload-list.v1",
  semantic_atlas: null,
  catalog: {
    schema: "rey.workload-catalog.v1",
    kind: "workspace_packages",
    root: "workloads",
    workload_count: 1,
    admitted_count: 1,
    draft_count: 0,
  },
  workloads: [
    {
      provenance: {
        origin: "workspace_package",
        source: "workloads/example/workload.yaml",
        source_digest: "package:1",
        generation: {
          kind: "coding_harness",
          producer: "codex",
          producer_revision: "gpt-5",
        },
        admission: { state: "accepted", scenario_oracle: "frozen" },
      },
      workload: {
        id: "rey.example",
        revision: 1,
        semantic_digest: "workload:1",
      },
      title: "Example",
      candidate_graph: {
        id: "rey.example.graph",
        revision: 1,
        semantic_digest: "graph:1",
      },
      freshness: "fresh",
      qualification: "qualified",
      required: 2,
      passed: 2,
      failed: 0,
      inconclusive: 0,
      evaluated: 2,
      stale: 0,
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
      last_run_status: "passed",
      last_test_result_id: "test:1",
    },
  ],
  drafts: [],
  attention: {
    schema: "rey.workload-attention.v1",
    attention_id: "attention:1",
    source_snapshot_id: "snapshot:1",
    rows: [],
    summary: {
      refine: 0,
      retest: 0,
      create: 0,
      blocked: 0,
      policy_excluded: 0,
      workloads: 1,
      surfaces: 0,
      owned_surfaces: 0,
      unowned_surfaces: 0,
    },
  },
};

describe("Explorer coordinate views", () => {
  it("separates the semantic coordinate from continuous view scale", () => {
    const view = agentExplorerView({
      id: "coding_harness:codex@gpt-5",
      kind: "coding_harness",
      producer: "codex",
      producer_revision: "gpt-5",
      workload_ids: ["rey.example"],
      package_sources: ["workloads/example/workload.yaml"],
      scenarios_passed: 2,
      scenarios_required: 2,
      attention_rows: 0,
    });
    expect(explorerCoordinateUri(view.coordinate)).toBe(
      "rey+local://agent/codex?revision=gpt-5&role=coding_harness",
    );
    expect(explorerViewPath(view)).toBe(
      "/explore?coordinate=rey%2Blocal%3A%2F%2Fagent%2Fcodex%3Frevision%3Dgpt-5%26role%3Dcoding_harness&scale=2.05",
    );
    expect(
      parseExplorerView(explorerCoordinateUri(view.coordinate), "2.05"),
    ).toEqual(view);
    expect(
      parseExplorerView(explorerCoordinateUri(view.coordinate), "0.05"),
    ).toMatchObject({ scale: 0.05 });
    expect(
      parseExplorerView(explorerCoordinateUri(view.coordinate), "0.04"),
    ).toBeNull();
  });

  it("rejects matrix routes and non-canonical or ambiguous coordinates", () => {
    expect(
      parseExplorerCoordinate(
        "/explore/agent/codex;at=gpt-5;lens=objects;role=coding_harness",
      ),
    ).toBeNull();
    expect(
      parseExplorerCoordinate(
        "rey+local://agent/codex?revision=gpt-5&role=coding_harness&role=human",
      ),
    ).toBeNull();
    expect(
      parseExplorerCoordinate(
        "rey+local://workload/rey.example?revision=workload%3A1&zoom=2",
      ),
    ).toBeNull();
    expect(
      parseExplorerCoordinate("rey+local://workload/rey.example?role=human"),
    ).toBeNull();
    expect(
      parseExplorerCoordinate(
        "rey+local://agent/codex?role=coding_harness&revision=gpt-5",
      ),
    ).toBeNull();
    expect(
      parseExplorerView(
        "rey+local://workload/rey.example?revision=workload%3A1",
        "1.460",
      ),
    ).toBeNull();
  });

  it("resolves exact current bindings and exposes stale coordinates", () => {
    const current = parseExplorerView(
      "rey+local://workload/rey.example?revision=workload%3A1",
      "1.46",
    );
    const stale = parseExplorerView(
      "rey+local://workload/rey.example?revision=workload%3A0",
      "1.46",
    );
    expect(current && resolveExplorerView(portfolio, current)).toMatchObject({
      focus_id: "workload:rey.example",
      status: "current",
    });
    expect(stale && resolveExplorerView(portfolio, stale)).toMatchObject({
      focus_id: "workload:rey.example",
      status: "stale",
      actual_revision: "workload:1",
    });
  });

  it("deep-links patch anchors without executing a locator and reports stale revisions", () => {
    const coordinate =
      "rey+local://file/README.md?revision=blake3%3Acurrent-source";
    const withPatch: WorkloadList = {
      ...portfolio,
      workloads: [
        {
          ...portfolio.workloads[0]!,
          topography_results: 1,
          topography_revision: "blake3:topography",
          topography_patch: {
            topography_revision: "blake3:topography",
            anchors: [
              {
                anchor_id: "blake3:anchor",
                coordinate: {
                  coordinate,
                  source_revision: "blake3:current-source",
                },
                source_revision: "blake3:current-source",
              },
            ],
          } as unknown as NonNullable<
            WorkloadList["workloads"][number]["topography_patch"]
          >,
          topography_projection: null,
        },
      ],
    };
    const current = parseExplorerView(coordinate, "1.62");
    const stale = parseExplorerView(
      "rey+local://file/README.md?revision=blake3%3Aold-source",
      "1.62",
    );

    expect(current && resolveExplorerView(withPatch, current)).toMatchObject({
      focus_id: "anchor:rey.example:blake3:anchor",
      status: "current",
    });
    expect(stale && resolveExplorerView(withPatch, stale)).toMatchObject({
      focus_id: "anchor:rey.example:blake3:anchor",
      status: "stale",
      actual_revision: "blake3:current-source",
    });
  });
});
