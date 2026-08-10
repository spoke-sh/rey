import { describe, expect, it } from "vitest";
import type { WorkloadList, WorkloadSummary } from "./domain";
import {
  DEFAULT_LENS_ZOOM,
  MAX_LENS_ZOOM,
  MIN_LENS_ZOOM,
  NEIGHBORHOOD_LENS_ZOOM,
  OBJECT_LENS_ZOOM,
  buildTopologyScene,
  clampLensZoom,
  lensRegimeForZoom,
  stepLensZoom,
} from "./topology";

const workload = (id: string): WorkloadSummary => ({
  provenance: null,
  workload: { id, revision: 1, semantic_digest: `digest:${id}` },
  title: id,
  candidate_graph: {
    id: `${id}.graph`,
    revision: 2,
    semantic_digest: `graph:${id}`,
  },
  freshness: "fresh",
  qualification: "failing",
  required: 3,
  passed: 2,
  failed: 1,
  inconclusive: 0,
  evaluated: 3,
  stale: 0,
  optional: 0,
  mining_operations: 2,
  mining_results: 4,
  incomplete_mining_results: 0,
  relation_deltas: 1,
  reasoning_surfaces: 1,
  attention_rows: 1,
  last_run_status: "blocked",
  last_test_result_id: `test:${id}`,
});

const portfolio: WorkloadList = {
  schema: "rey.workload-list.v5",
  catalog: {
    schema: "rey.workload-catalog.v2",
    kind: "workspace_packages",
    root: "workloads",
    workload_count: 1,
    admitted_count: 1,
    draft_count: 0,
  },
  workloads: [workload("rey.example")],
  drafts: [],
  attention: {
    schema: "rey.workload-attention.v1",
    attention_id: "attention:1",
    source_snapshot_id: "snapshot:1",
    rows: [
      {
        row_id: "row:1",
        action: "refine",
        subject_kind: "workload",
        subject_id: "rey.example",
        reason: "scenario_delta",
        readiness: "ready",
        evidence_ids: ["evidence:1", "evidence:2"],
        dependency_ids: [],
        priority: 10,
        estimated_cost_units: 2,
      },
    ],
    summary: {
      refine: 1,
      retest: 0,
      create: 0,
      blocked: 0,
      policy_excluded: 0,
      workloads: 1,
      surfaces: 2,
      owned_surfaces: 1,
      unowned_surfaces: 1,
    },
  },
};

describe("context topology lens", () => {
  it("moves through every semantic regime without a control step skipping one", () => {
    expect(lensRegimeForZoom(DEFAULT_LENS_ZOOM)).toBe("landscape");
    expect(lensRegimeForZoom(NEIGHBORHOOD_LENS_ZOOM)).toBe("neighborhoods");
    expect(lensRegimeForZoom(OBJECT_LENS_ZOOM)).toBe("objects");
    expect(stepLensZoom(DEFAULT_LENS_ZOOM, 1)).toBe(NEIGHBORHOOD_LENS_ZOOM);
    expect(stepLensZoom(NEIGHBORHOOD_LENS_ZOOM, 1)).toBe(OBJECT_LENS_ZOOM);
    expect(stepLensZoom(OBJECT_LENS_ZOOM, -1)).toBe(NEIGHBORHOOD_LENS_ZOOM);
  });

  it("clamps the optical coordinate to the declared bounded range", () => {
    expect(clampLensZoom(-4)).toBe(MIN_LENS_ZOOM);
    expect(clampLensZoom(8)).toBe(MAX_LENS_ZOOM);
  });

  it("changes object families while retaining exact portfolio identities", () => {
    const landscape = buildTopologyScene(portfolio, DEFAULT_LENS_ZOOM);
    const neighborhoods = buildTopologyScene(portfolio, NEIGHBORHOOD_LENS_ZOOM);
    const objects = buildTopologyScene(
      portfolio,
      OBJECT_LENS_ZOOM,
      "workload:rey.example",
    );

    expect(landscape.regime).toBe("landscape");
    expect(landscape.nodes.map((node) => node.family)).toContain("WORKLOADS");
    expect(neighborhoods.regime).toBe("neighborhoods");
    expect(neighborhoods.nodes).toContainEqual(
      expect.objectContaining({
        focus_id: "workload:rey.example",
        label: "rey.example",
      }),
    );
    expect(objects.regime).toBe("objects");
    expect(objects.nodes).toContainEqual(
      expect.objectContaining({
        family: "COMPUTE GRAPH",
        label: "rey.example.graph",
      }),
    );
    expect(objects.nodes).toContainEqual(
      expect.objectContaining({
        family: "WORKLOAD",
        workload_id: "rey.example",
      }),
    );
  });

  it("discloses folded evidence instead of implying a complete object view", () => {
    const scene = buildTopologyScene(
      portfolio,
      OBJECT_LENS_ZOOM,
      "attention:row:1",
    );
    expect(scene.omissions).toEqual(["1 evidence references folded"]);
  });
});
