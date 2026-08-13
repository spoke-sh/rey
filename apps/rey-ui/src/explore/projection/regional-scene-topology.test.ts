import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { SceneAdmissionResult, WorkloadList } from "../../domain";
import { buildTopologyScene } from "../../topology";
import { ReferenceRenderer } from "../renderers/reference";

const contract = (id: string, digest = `${id}:digest`) => ({
  id,
  revision: 1,
  semantic_digest: digest,
});

const regionalAtlas = {
  atlas_revision: "atlas:1",
  compiler: contract("rey.semantic-atlas.polar-cluster", "atlas-compiler:1"),
  sources: [],
  regions: [],
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
  clusters: [
    {
      cluster_id: "cluster:1",
      semantic_longitude_microdegrees: -42_000_000,
      semantic_latitude_microdegrees: 18_000_000,
      angular_radius_microdegrees: 7_000_000,
      member_region_ids: ["atlas-region:1"],
      dominant_feature: "terrain_control",
    },
  ],
};

const regionalPortfolio = {
  schema: "rey.workload-list.v1",
  catalog: {
    schema: "rey.workload-catalog.v1",
    kind: "workspace_packages",
    root: "sys",
    workload_count: 1,
    admitted_count: 1,
    draft_count: 0,
  },
  semantic_atlas: regionalAtlas,
  semantic_atlas_history: [regionalAtlas],
  semantic_atlas_deltas: [
    {
      delta_id: "delta:1",
      target_revision: "atlas:1",
    },
  ],
  drafts: [],
  attention: {
    attention_id: "attention:1",
    source_snapshot_id: "portfolio:1",
    rows: [],
    summary: {},
  },
  workloads: [
    {
      workload: contract("scene-admission", "workload:1"),
      candidate_graph: contract("scene-admission.graph", "graph:1"),
      latest_scene_admission: {
        schema: "rey.scene-admission-result.v1",
        result_id: "result:1",
        status: "accepted",
        scenario: null,
        workload: contract("scene-admission", "workload:1"),
        graph: contract("scene-admission.graph", "graph:1"),
        capability_snapshot_id: "capability:1",
        scene: {
          schema: "rey.admitted-regional-scene.v1",
          scene_id: "scene:1",
          region_id: "regional-demo",
          complete: true,
          native_bounds: {
            west_microdegrees: -123_000_000,
            south_microdegrees: 37_000_000,
            east_microdegrees: -122_000_000,
            north_microdegrees: 38_000_000,
            crosses_antimeridian: false,
          },
          admission: {
            admission_id: "admission:1",
            editor_sequence: 1,
            implementation: contract("rey.scene-admission.builtin", "impl:1"),
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
          omissions: [
            {
              kind: "terrain_program_absent",
              reason: "no qualified terrain height",
            },
          ],
          projection: {
            schema: "rey.regional-projection-packet.v1",
            packet_id: "packet:1",
            source_package_id: "package:1",
            source_snapshot_revision: "snapshot:1",
            grammar_id: "grammar:1",
            terrain_program_id: null,
            complete: true,
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
            layers: [
              {
                layer_id: "regional-demo.terrain-control",
                object_ids: ["ridge"],
              },
            ],
            objects: [
              {
                object_id: "ridge",
                source_id: "controls",
                source_path: "terrain.geojson",
                source_artifact_id: "artifact:1",
                object_revision: "object:1",
                geometry_kind: "Polygon",
                layer: "terrain_control",
                native_bounds: {
                  west_microdegrees: -122_800_000,
                  south_microdegrees: 37_200_000,
                  east_microdegrees: -122_200_000,
                  north_microdegrees: 37_800_000,
                  crosses_antimeridian: false,
                },
              },
            ],
            validity: [
              {
                class: "valid",
                scope: "native_geometry:ridge",
                rule: "exact native geometry",
              },
              {
                class: "unsupported",
                scope: "terrain_height",
                rule: "no qualified terrain-height adapter",
              },
            ],
          },
        },
      },
    },
  ],
} as unknown as WorkloadList;

describe("regional scene topology projection", () => {
  it("moves one admitted identity from World through Atlas into County", () => {
    const world = buildTopologyScene(
      regionalPortfolio,
      0.1,
      "cluster:portfolio",
    );
    expect(world.globe?.posture).toBe("regional_scenes");
    expect(world.globe?.regions[0]).toMatchObject({
      focus_id: "regional:scene:1",
      longitude_degrees: -42,
      latitude_degrees: 18,
      angular_radius_degrees: 0,
    });

    const atlas = buildTopologyScene(
      regionalPortfolio,
      0.26,
      "regional:scene:1",
    );
    expect(atlas.label).toBe("SEMANTIC MERCATOR ATLAS");
    expect(atlas.nodes[0]?.focus_id).toBe("regional:scene:1");
    expect(atlas.regions[0]).toMatchObject({
      id: "atlas-sector:sector:1",
      label: "SECTOR 5.4",
      variant: "map-zone",
    });
    expect(atlas.omissions).toContain(
      "synthetic sector polygons express membership only; they are not surveyed coverage or native County footprints",
    );

    const county = buildTopologyScene(
      regionalPortfolio,
      0.58,
      "regional:scene:1",
    );
    expect(county.label).toBe("ADMITTED COUNTY");
    expect(county.terrain).toBe(false);
    expect(county.nodes[0]).toMatchObject({
      focus_id: "regional-object:ridge",
      workload_id: "scene-admission",
      tone: "unsupported",
    });
    expect(county.omissions).toContain("no qualified terrain height");
    expect(county.omissions).toContain(
      "unsupported: terrain_height · no qualified terrain-height adapter",
    );
    const markup = renderToStaticMarkup(
      ReferenceRenderer({
        layers: { relief: true, water: true, weather: true, probes: true },
        onFocus: () => undefined,
        scene: county,
      }),
    );
    expect(markup).toContain("regional-demo / SCENE@1");
    expect(markup).toContain("ridge");
    expect(markup).toContain("terrain.geojson");
    expect(markup).not.toContain("topology-terrain-field");
  });

  it("does not project rejected or scenario-fixture results", () => {
    const workload = regionalPortfolio.workloads[0]!;
    const accepted = workload.latest_scene_admission!;
    const variants: SceneAdmissionResult[] = [
      { ...accepted, status: "rejected", scene: null },
      { ...accepted, scenario: contract("fixture") },
    ];
    for (const latest_scene_admission of variants) {
      const scene = buildTopologyScene(
        {
          ...regionalPortfolio,
          workloads: [{ ...workload, latest_scene_admission }],
        },
        0.1,
      );
      expect(scene.globe?.posture).not.toBe("regional_scenes");
    }
  });
});
