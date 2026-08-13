import { describe, expect, it } from "vitest";
import type { WorkloadList } from "../../domain";
import { admittedRegionalScenes } from "./regional-scene-projector";

const contract = (id: string, digest = `${id}:digest`) => ({
  id,
  revision: 1,
  semantic_digest: digest,
});

const regionalAtlas = {
  atlas_revision: "atlas:1",
  regional_sources: [
    {
      region_id: "atlas-region:1",
      workload_id: "scene-admission",
      scene_region_id: "regional-demo",
      source_scene_id: "scene:1",
      source_admission_id: "admission:1",
      source_package_id: "package:1",
      source_package_revision: "snapshot:1",
      projection_packet_id: "packet:1",
      semantic_longitude_microdegrees: -42_000_000,
      semantic_latitude_microdegrees: 18_000_000,
      complete: true,
      native_objects: 1,
      native_feature_objects: 0,
      terrain_control_objects: 1,
      hydrology_objects: 0,
      boundary_objects: 0,
      poi_objects: 0,
      validity_boundaries: 0,
      omissions: 0,
    },
  ],
  regional_regions: [
    {
      region_id: "atlas-region:1",
      cluster_id: "cluster:1",
      workload_id: "scene-admission",
      scene_region_id: "regional-demo",
      source_scene_id: "scene:1",
      source_admission_id: "admission:1",
      source_package_id: "package:1",
      source_package_revision: "snapshot:1",
      projection_packet_id: "packet:1",
      semantic_longitude_microdegrees: -42_000_000,
      semantic_latitude_microdegrees: 18_000_000,
      angular_radius_microdegrees: 0,
      native_objects: 1,
      validity_boundaries: 0,
      omissions: 0,
      complete: true,
      dominant_feature: "terrain_control",
    },
  ],
};

const portfolio = {
  semantic_atlas: regionalAtlas,
  semantic_atlas_history: [regionalAtlas],
  semantic_atlas_deltas: [{ delta_id: "delta:1", target_revision: "atlas:1" }],
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
          scene_id: "scene:1",
          region_id: "regional-demo",
          admission: {
            admission_id: "admission:1",
            workload: contract("scene-admission", "workload:1"),
            graph: contract("scene-admission.graph", "graph:1"),
            capability_snapshot_id: "capability:1",
            package_id: "package:1",
            package_snapshot_revision: "snapshot:1",
          },
          artifacts: {
            admitted_atlas_revision: "atlas:1",
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
              scene: {
                ...result.scene!,
                artifacts: {
                  ...result.scene!.artifacts,
                  admitted_atlas_revision: "atlas:other",
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
    expect(
      admittedRegionalScenes({
        ...portfolio,
        semantic_atlas: {
          ...regionalAtlas,
          regional_regions: [
            {
              ...regionalAtlas.regional_regions[0]!,
              semantic_longitude_microdegrees: -41_000_000,
            },
          ],
        } as unknown as WorkloadList["semantic_atlas"],
      }),
    ).toEqual([]);
    expect(
      admittedRegionalScenes({
        ...portfolio,
        semantic_atlas_history: [],
        semantic_atlas_deltas: [],
      }),
    ).toEqual([]);
  });
});
