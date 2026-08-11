import { describe, expect, it } from "vitest";
import type { WorkloadList } from "../../domain";
import { admittedTopographies } from "./topography-projector";

const portfolio = {
  schema: "rey.workload-list.v8",
  semantic_atlas: null,
  workloads: [
    {
      workload: { id: "survey" },
      topography_patch: {
        patch_id: "patch:1",
        topography_revision: "topography:1",
      },
      topography_projection: {
        schema: "rey.projection-packet.v2",
        source_patch_id: "patch:1",
        source_topography_revision: "topography:1",
      },
    },
  ],
} as unknown as WorkloadList;

describe("topography evidence adapter", () => {
  it("admits only an exact patch and projection-packet binding", () => {
    expect(admittedTopographies(portfolio)).toHaveLength(1);
    expect(
      admittedTopographies({
        ...portfolio,
        workloads: [
          {
            ...portfolio.workloads[0]!,
            topography_projection: {
              ...portfolio.workloads[0]!.topography_projection!,
              source_patch_id: "patch:other",
            },
          },
        ],
      }),
    ).toEqual([]);
    expect(
      admittedTopographies({
        ...portfolio,
        workloads: [
          { ...portfolio.workloads[0]!, topography_projection: null },
        ],
      }),
    ).toEqual([]);
  });
});
