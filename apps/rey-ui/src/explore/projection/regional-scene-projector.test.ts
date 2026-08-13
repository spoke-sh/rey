import { describe, expect, it } from "vitest";
import type { WorkloadList } from "../../domain";
import { admittedRegionalScenes } from "./regional-scene-projector";

const contract = (id: string, digest = `${id}:digest`) => ({
  id,
  revision: 1,
  semantic_digest: digest,
});

const portfolio = {
  workloads: [
    {
      workload: contract("scene-admission", "workload:1"),
      candidate_graph: contract("scene-admission.graph", "graph:1"),
      latest_scene_admission: {
        schema: "rey.scene-admission-result.v1",
        status: "accepted",
        scenario: null,
        workload: contract("scene-admission", "workload:1"),
        graph: contract("scene-admission.graph", "graph:1"),
        capability_snapshot_id: "capability:1",
        scene: {
          schema: "rey.admitted-regional-scene.v1",
          admission: {
            workload: contract("scene-admission", "workload:1"),
            graph: contract("scene-admission.graph", "graph:1"),
            capability_snapshot_id: "capability:1",
            package_id: "package:1",
            package_snapshot_revision: "snapshot:1",
          },
          artifacts: {
            projection_packet_id: "packet:1",
            terrain_program_id: null,
          },
          projection: {
            schema: "rey.regional-projection-packet.v1",
            packet_id: "packet:1",
            source_package_id: "package:1",
            source_snapshot_revision: "snapshot:1",
            terrain_program_id: null,
            coordinate_bindings: [
              { space: "native_crs84" },
              { space: "synthetic_semantic" },
              { space: "semantic_mercator" },
              { space: "county_local" },
              { space: "camera" },
            ],
            transforms: [
              {
                source_space: "native_crs84",
                target_space: "synthetic_semantic",
                target_origin: [-42_000_000, 18_000_000],
              },
            ],
          },
        },
      },
    },
  ],
} as unknown as WorkloadList;

describe("regional scene evidence adapter", () => {
  it("admits only a production accepted result with exact scene bindings", () => {
    expect(admittedRegionalScenes(portfolio)).toHaveLength(1);
    const workload = portfolio.workloads[0]!;
    const result = workload.latest_scene_admission!;
    expect(
      admittedRegionalScenes({
        ...portfolio,
        workloads: [
          {
            ...workload,
            latest_scene_admission: {
              ...result,
              scenario: contract("fixture"),
            },
          },
        ],
      }),
    ).toEqual([]);
    expect(
      admittedRegionalScenes({
        ...portfolio,
        workloads: [
          {
            ...workload,
            latest_scene_admission: {
              ...result,
              status: "rejected",
              scene: null,
            },
          },
        ],
      }),
    ).toEqual([]);
    expect(
      admittedRegionalScenes({
        ...portfolio,
        workloads: [
          {
            ...workload,
            latest_scene_admission: {
              ...result,
              scene: {
                ...result.scene!,
                artifacts: {
                  ...result.scene!.artifacts,
                  projection_packet_id: "packet:other",
                },
              },
            },
          },
        ],
      }),
    ).toEqual([]);
    expect(
      admittedRegionalScenes({
        ...portfolio,
        workloads: [
          {
            ...workload,
            latest_scene_admission: {
              ...result,
              graph: { ...result.graph, revision: result.graph.revision + 1 },
            },
          },
        ],
      }),
    ).toEqual([]);
    expect(
      admittedRegionalScenes({
        ...portfolio,
        workloads: [
          {
            ...workload,
            latest_scene_admission: {
              ...result,
              scene: {
                ...result.scene!,
                projection: {
                  ...result.scene!.projection,
                  transforms: [],
                },
              },
            },
          },
        ],
      }),
    ).toEqual([]);
  });
});
