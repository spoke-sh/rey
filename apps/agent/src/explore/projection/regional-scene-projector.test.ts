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
      terrain_objects: 0,
      terrain_control_objects: 1,
      hydrology_objects: 0,
      boundary_objects: 0,
      poi_objects: 0,
      highway_objects: 0,
      road_objects: 0,
      railway_objects: 0,
      district_objects: 0,
      lot_objects: 0,
      structure_objects: 0,
      utility_objects: 0,
      label_objects: 0,
      beacon_objects: 0,
      construction_objects: 0,
      connector_objects: 0,
      validity_boundaries: 0,
      omissions: 0,
    },
  ],
  regional_regions: [
    {
      region_id: "atlas-region:1",
      cluster_id: "cluster:1",
      sector_id: "sector:1",
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
  sectors: [
    {
      sector_id: "sector:1",
      longitude_band: 4,
      latitude_band: 3,
      west_microdegrees: -60_000_000,
      south_microdegrees: 0,
      east_microdegrees: -30_000_000,
      north_microdegrees: 30_000_000,
      member_region_ids: ["atlas-region:1"],
      authority: "synthetic fixture partition",
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
        landscape: null,
        scene: {
          schema: "rey.admitted-regional-scene.v1",
          scene_id: "scene:1",
          region_id: "regional-demo",
          native_bounds: {
            west_microdegrees: -123_000_000,
            south_microdegrees: 37_000_000,
            east_microdegrees: -122_000_000,
            north_microdegrees: 38_000_000,
            crosses_antimeridian: false,
          },
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
              {
                space: "county_local",
                status: "bound",
                dimensions: ["east", "north", "up"],
                units: [
                  "local_microunit",
                  "local_microunit",
                  "local_microunit",
                ],
              },
              { space: "camera" },
            ],
            transforms: [
              {
                source_space: "native_crs84",
                target_space: "synthetic_semantic",
                target_origin: [-42_000_000, 18_000_000],
              },
              {
                transform: contract(
                  "rey.scene.native-to-county-local",
                  "county-transform:1",
                ),
                source_space: "native_crs84",
                target_space: "county_local",
                source_origin: [-122_500_000, 37_500_000],
                target_origin: [0, 0, 0],
                parameters: ["east_north_up_microunits"],
                inverse_policy:
                  "bounded analytic inverse inside admitted envelope",
                distortion: "presentation only",
              },
            ],
            footprint: null,
          },
        },
      },
    },
  ],
} as unknown as WorkloadList;

describe("regional scene evidence adapter", () => {
  it("projects every active package through its exact historical atlas binding", () => {
    const workload = portfolio.workloads[0]!;
    const first = workload.latest_scene_admission!;
    const firstScene = first.scene!;
    const secondScene = {
      ...firstScene,
      scene_id: "scene:2",
      region_id: "regional-east",
      native_bounds: {
        west_microdegrees: -121_900_000,
        south_microdegrees: 37_000_000,
        east_microdegrees: -120_900_000,
        north_microdegrees: 38_000_000,
        crosses_antimeridian: false,
      },
      admission: {
        ...firstScene.admission,
        admission_id: "admission:2",
        package_id: "package:2",
        package_snapshot_revision: "snapshot:2",
      },
      artifacts: {
        ...firstScene.artifacts,
        admitted_atlas_revision: "atlas:2",
        projection_packet_id: "packet:2",
      },
      projection: {
        ...firstScene.projection,
        packet_id: "packet:2",
        source_package_id: "package:2",
        source_snapshot_revision: "snapshot:2",
        transforms: firstScene.projection.transforms.map((transform) =>
          transform.target_space === "synthetic_semantic"
            ? { ...transform, target_origin: [-39_000_000, 18_000_000] }
            : transform.target_space === "county_local"
              ? { ...transform, source_origin: [-121_400_000, 37_500_000] }
              : transform,
        ),
      },
    };
    const second = { ...first, scene: secondScene };
    const secondSource = {
      ...regionalAtlas.regional_sources[0]!,
      region_id: "atlas-region:2",
      scene_region_id: "regional-east",
      source_scene_id: "scene:2",
      source_admission_id: "admission:2",
      source_package_id: "package:2",
      source_package_revision: "snapshot:2",
      projection_packet_id: "packet:2",
      semantic_longitude_microdegrees: -39_000_000,
    };
    const secondRegion = {
      ...regionalAtlas.regional_regions[0]!,
      region_id: "atlas-region:2",
      scene_region_id: "regional-east",
      source_scene_id: "scene:2",
      source_admission_id: "admission:2",
      source_package_id: "package:2",
      source_package_revision: "snapshot:2",
      projection_packet_id: "packet:2",
      semantic_longitude_microdegrees: -39_000_000,
    };
    const currentAtlas = {
      ...regionalAtlas,
      atlas_revision: "atlas:2",
      regional_sources: [regionalAtlas.regional_sources[0]!, secondSource],
      regional_regions: [regionalAtlas.regional_regions[0]!, secondRegion],
      sectors: [
        {
          ...regionalAtlas.sectors[0]!,
          member_region_ids: ["atlas-region:1", "atlas-region:2"],
        },
      ],
    };

    const projected = admittedRegionalScenes({
      ...portfolio,
      semantic_atlas: currentAtlas,
      semantic_atlas_history: [regionalAtlas, currentAtlas],
      semantic_atlas_deltas: [
        { delta_id: "delta:1", target_revision: "atlas:1" },
        { delta_id: "delta:2", target_revision: "atlas:2" },
      ],
      workloads: [
        {
          ...workload,
          scene_admissions: [first, second],
          latest_scene_admission: second,
        },
      ],
    } as unknown as WorkloadList);

    expect(projected.map(({ scene }) => scene.region_id)).toEqual([
      "regional-demo",
      "regional-east",
    ]);
  });

  it("admits only a production accepted result with exact scene bindings", () => {
    expect(admittedRegionalScenes(portfolio)).toMatchObject([
      { county_footprint: null },
    ]);
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
                projection: {
                  ...result.scene!.projection,
                  transforms: result.scene!.projection.transforms.map(
                    (transform) =>
                      transform.target_space === "county_local"
                        ? {
                            ...transform,
                            source_origin: [-122_499_999, 37_500_000],
                          }
                        : transform,
                  ),
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
