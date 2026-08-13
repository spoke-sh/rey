import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { SceneAdmissionResult, WorkloadList } from "../../domain";
import { buildTopologyScene } from "../../topology";
import { compileSceneSnapshot } from "../engine/scene";
import { SEMANTIC_LABEL_LAYOUT_REVISION } from "../engine/labels";
import { ReferenceRenderer } from "../renderers/reference";
import {
  COUNTY_FOOTPRINT_PROJECTION_REVISION,
  COUNTY_FRAME_PROJECTION_REVISION,
} from "./county-frame";
import { SEMANTIC_MERCATOR_PROJECTION_REVISION } from "./semantic-mercator";

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
            footprint: {
              footprint_id: "footprint:1",
              source_object_id: "county-boundary",
              source_artifact_id: "artifact:boundary",
              source_object_revision: "object:boundary",
              geometry_kind: "Polygon",
              native_bounds: {
                west_microdegrees: -123_000_000,
                south_microdegrees: 37_000_000,
                east_microdegrees: -122_000_000,
                north_microdegrees: 38_000_000,
                crosses_antimeridian: false,
              },
              rings: [
                [
                  [-123_000_000, 37_000_000],
                  [-122_000_000, 37_000_000],
                  [-122_200_000, 38_000_000],
                  [-122_800_000, 37_800_000],
                  [-123_000_000, 37_000_000],
                ],
              ],
              coordinate_count: 5,
              authority:
                "exact admitted native boundary polygon; footprint validity ends at its rings",
            },
            layers: [
              {
                layer_id: "regional-demo.boundary",
                kind: "boundary",
                object_ids: ["county-boundary"],
              },
              {
                layer_id: "regional-demo.terrain-control",
                kind: "terrain_control",
                object_ids: ["ridge"],
              },
            ],
            objects: [
              {
                object_id: "county-boundary",
                source_id: "boundary",
                source_path: "boundary.geojson",
                source_artifact_id: "artifact:boundary",
                object_revision: "object:boundary",
                geometry_kind: "Polygon",
                layer: "boundary",
                native_bounds: {
                  west_microdegrees: -123_000_000,
                  south_microdegrees: 37_000_000,
                  east_microdegrees: -122_000_000,
                  north_microdegrees: 38_000_000,
                  crosses_antimeridian: false,
                },
              },
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
            limits: { max_native_coordinates: 100 },
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
    expect(world.world_atlas_transition).toMatchObject({
      schema: "rey.world-atlas-transition.v1",
      atlas_revision: "atlas:1",
      projection_revision: "rey.semantic-mercator-projection@1",
      points: [
        {
          identity: "atlas-region:1",
          focus_id: "regional:scene:1",
          longitude_microdegrees: -42_000_000,
          latitude_microdegrees: 18_000_000,
        },
      ],
      sectors: [{ identity: "sector:1" }],
    });

    const atlas = buildTopologyScene(
      regionalPortfolio,
      0.26,
      "regional:scene:1",
    );
    expect(atlas.label).toBe("SEMANTIC MERCATOR ATLAS");
    expect(atlas.nodes[0]?.focus_id).toBe("regional:scene:1");
    expect(atlas.nodes[0]).toMatchObject({
      semantic_identity: "atlas-region:1",
      semantic_coordinate: {
        longitude_microdegrees: -42_000_000,
        latitude_microdegrees: 18_000_000,
      },
    });
    expect(atlas.world_atlas_transition).toEqual(world.world_atlas_transition);
    expect(atlas.regions[0]).toMatchObject({
      id: "atlas-sector:sector:1",
      label: "SECTOR 5.4",
      variant: "map-zone",
    });
    expect(atlas.omissions).toContain(
      "synthetic sector polygons express membership only; they are not surveyed coverage or native County footprints",
    );
    expect(atlas.omissions).toContain(
      "semantic Mercator clips at ±85051129µ°; retained polar-cap membership is disclosed rather than silently dropped",
    );
    const compiledAtlas = compileSceneSnapshot(
      regionalPortfolio,
      0.26,
      "regional:scene:1",
    );
    expect(compiledAtlas.compiler_revisions).toContain(
      SEMANTIC_MERCATOR_PROJECTION_REVISION,
    );
    expect(compiledAtlas.compiler_revisions).toContain(
      SEMANTIC_LABEL_LAYOUT_REVISION,
    );
    expect(compiledAtlas.compiler_revisions).toContain(
      COUNTY_FRAME_PROJECTION_REVISION,
    );
    expect(compiledAtlas.compiler_revisions).toContain(
      COUNTY_FOOTPRINT_PROJECTION_REVISION,
    );
    expect(Object.isFrozen(compiledAtlas.scene.world_atlas_transition)).toBe(
      true,
    );
    expect(
      Object.isFrozen(compiledAtlas.scene.world_atlas_transition?.points),
    ).toBe(true);
    for (const transitionScene of [world, atlas]) {
      const transitionMarkup = renderToStaticMarkup(
        ReferenceRenderer({
          globeView: { yaw_degrees: 24, pitch_degrees: -8 },
          layers: { relief: true, water: true, weather: true, probes: true },
          onFocus: () => undefined,
          projectionMorphProgress: 0.5,
          scene: transitionScene,
        }),
      );
      expect(transitionMarkup).toContain(
        'data-projection-morph="rey.semantic-mercator-projection@1"',
      );
      expect(transitionMarkup).toContain(
        'data-projection-morph-progress="0.500"',
      );
      expect(transitionMarkup).toContain(
        'data-semantic-identity="atlas-region:1"',
      );
      expect(transitionMarkup).toContain('data-focus-id="regional:scene:1"');
    }
    const wrappedMarkup = renderToStaticMarkup(
      ReferenceRenderer({
        layers: { relief: true, water: true, weather: true, probes: true },
        onFocus: () => undefined,
        projectionMorphProgress: 1,
        scene: atlas,
      }),
    );
    for (const wrapIndex of [-1, 0, 1])
      expect(wrappedMarkup).toContain(`data-chart-wrap-index="${wrapIndex}"`);
    expect(
      wrappedMarkup.match(/data-semantic-identity="atlas-region:1"/g),
    ).toHaveLength(3);
    expect(wrappedMarkup).toContain('aria-hidden="true"');
    expect(wrappedMarkup).toContain('tabindex="-1"');
    expect(wrappedMarkup).toContain(
      'data-label-layout="rey.semantic-label-layout@1"',
    );
    expect(wrappedMarkup).toContain('data-label-disposition="selected"');

    const unselectedCloserView = buildTopologyScene(
      regionalPortfolio,
      0.58,
      "cluster:portfolio",
    );
    expect(unselectedCloserView.regime).toBe("atlas");
    expect(unselectedCloserView.county_frame).toBeNull();
    expect(unselectedCloserView.county_footprint).toBeNull();
    const unknownSelection = buildTopologyScene(
      regionalPortfolio,
      0.58,
      "regional:unknown",
    );
    expect(unknownSelection.regime).toBe("atlas");
    expect(unknownSelection.county_frame).toBeNull();
    expect(unknownSelection.county_footprint).toBeNull();
    const withoutFootprint = structuredClone(regionalPortfolio);
    withoutFootprint.workloads[0]!.latest_scene_admission!.scene!.projection.footprint =
      null;
    const footprintRequired = buildTopologyScene(
      withoutFootprint,
      0.58,
      "regional:scene:1",
    );
    expect(footprintRequired.regime).toBe("atlas");
    expect(footprintRequired.county_footprint).toBeNull();

    const county = buildTopologyScene(
      regionalPortfolio,
      0.58,
      "regional:scene:1",
    );
    expect(county.label).toBe("ADMITTED COUNTY");
    expect(county.terrain).toBe(false);
    expect(county.county_frame).toMatchObject({
      schema: "rey.county-frame.v1",
      scene_id: "scene:1",
      source_origin: [-122_500_000, 37_500_000],
      target_origin: [0, 0, 0],
      transform_digest: "county-transform:1",
      pitch_degrees: 35.26439,
      yaw_degrees: 45,
    });
    expect(county.regions).toEqual([]);
    expect(county.county_footprint).toMatchObject({
      footprint_id: "footprint:1",
      source_object_id: "county-boundary",
      coordinate_count: 5,
    });
    expect(county.bearing.label).toBe("EXACT COUNTY FOOTPRINT");
    expect(
      county.nodes.find(({ focus_id }) => focus_id === "regional-object:ridge"),
    ).toMatchObject({
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
    expect(markup).toContain('data-county-footprint="footprint:1"');
    expect(markup).toContain('data-source-object="county-boundary"');
    expect(markup).toContain('fill-rule="evenodd"');
    expect(markup).toContain("ridge");
    expect(markup).toContain("terrain.geojson");
    expect(markup).not.toContain("topology-terrain-field");
    const compiledCounty = compileSceneSnapshot(
      regionalPortfolio,
      0.58,
      "regional:scene:1",
    );
    expect(Object.isFrozen(compiledCounty.scene.county_frame)).toBe(true);
    expect(Object.isFrozen(compiledCounty.scene.county_footprint)).toBe(true);
    expect(Object.isFrozen(compiledCounty.scene.county_footprint?.rings)).toBe(
      true,
    );
    expect(
      Object.isFrozen(compiledCounty.scene.county_frame?.source_origin),
    ).toBe(true);
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
