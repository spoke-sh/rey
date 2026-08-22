import { describe, expect, it } from "vitest";
import type {
  RegionalGeographyComposition,
  AdmittedRegionalScene,
} from "../../domain";
import { regionalLandscapeMembers } from "../../topology";
import type { AdmittedRegionalProjection } from "./regional-scene-projector";

describe("regional landscape composition selection", () => {
  it("selects only the connected conflict-free terrain-qualified component", () => {
    const west = projection("west");
    const east = projection("east");
    const isolated = projection("isolated");
    const composition = compositionFixture();

    expect(
      regionalLandscapeMembers(
        [west, east, isolated],
        west,
        composition,
      ).members.map(({ member_id }) => member_id),
    ).toEqual(["member:east", "member:west"]);

    composition.conflicts.push({
      conflict_id: "conflict:edge",
      seam_id: "seam:west-east",
      member_ids: ["member:west", "member:east"],
      kind: "seam_elevation",
      count: 1,
      detail: "fixture conflict",
    });
    expect(
      regionalLandscapeMembers(
        [west, east, isolated],
        west,
        composition,
      ).members.map(({ member_id }) => member_id),
    ).toEqual(["member:west"]);
  });
});

function projection(id: string): AdmittedRegionalProjection {
  return {
    scene: {
      scene_id: `scene:${id}`,
      admission: {
        admission_id: `admission:${id}`,
        package_id: `package:${id}`,
        package_snapshot_revision: `snapshot:${id}`,
      },
      artifacts: { admitted_atlas_revision: "atlas:selection" },
      projection: { terrain: { grid: {} } },
    } as unknown as AdmittedRegionalScene,
  } as AdmittedRegionalProjection;
}

function compositionFixture(): RegionalGeographyComposition {
  const member = (id: string) => ({
    member_id: `member:${id}`,
    workload_id: `workload:${id}`,
    region_id: `region:${id}`,
    scene_id: `scene:${id}`,
    atlas_region_id: `atlas-region:${id}`,
    admitted_atlas_revision: "atlas:selection",
    admission_id: `admission:${id}`,
    package_id: `package:${id}`,
    package_revision: `snapshot:${id}`,
    projection_packet_id: `packet:${id}`,
    terrain_program_id: `terrain-program:${id}`,
    terrain_dataset_id: `terrain-dataset:${id}`,
    native_bounds: {
      west_microdegrees: 0,
      south_microdegrees: 0,
      east_microdegrees: 1,
      north_microdegrees: 1,
      crosses_antimeridian: false,
    },
    terrain_valid_vertices: 4,
    terrain_no_data_vertices: 0,
  });
  return {
    schema: "rey.regional-geography-composition.v1",
    composition_id: "composition:selection",
    compiler: {
      id: "rey.regional-geography-composition",
      revision: 1,
      semantic_digest: "compiler:selection",
    },
    atlas_revision: "atlas:selection",
    members: [member("west"), member("east"), member("isolated")],
    seams: [
      {
        seam_id: "seam:west-east",
        member_ids: ["member:west", "member:east"],
        relationship: "edge_adjacent",
        shared_boundary: {
          axis: "longitude",
          coordinate_microdegrees: 1,
          start_microdegrees: 0,
          end_microdegrees: 1,
        },
        longitude_gap_microdegrees: 0,
        latitude_gap_microdegrees: 0,
        terrain_status: "qualified",
        compared_vertices: 2,
        valid_vertices: 2,
        no_data_vertices: 0,
        validity_conflicts: 0,
        elevation_conflicts: 0,
        material_conflicts: 0,
        max_elevation_delta_micrometers: 0,
      },
      {
        seam_id: "seam:east-isolated",
        member_ids: ["member:east", "member:isolated"],
        relationship: "disjoint",
        shared_boundary: null,
        longitude_gap_microdegrees: 1,
        latitude_gap_microdegrees: 0,
        terrain_status: "not_applicable",
        compared_vertices: 0,
        valid_vertices: 0,
        no_data_vertices: 0,
        validity_conflicts: 0,
        elevation_conflicts: 0,
        material_conflicts: 0,
        max_elevation_delta_micrometers: null,
      },
    ],
    conflicts: [],
    stitch_status: "ready",
    limits: { max_members: 8, max_pairs: 28, max_conflicts: 32 },
    complete: true,
    authority: "fixture composition assessment",
  };
}
