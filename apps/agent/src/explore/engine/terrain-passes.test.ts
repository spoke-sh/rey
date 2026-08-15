import { describe, expect, it } from "vitest";
import type { TopologyScene } from "../../topology";
import {
  createFieldGrid,
  fieldByteLength,
  maskField,
  materialField,
  scalarField,
  vectorField,
} from "./fields";
import { compileExplorerRenderGraph } from "./render-graph";
import {
  compileTerrainRenderPasses,
  parseSvgPolylines,
} from "./terrain-passes";
import type { TerrainFieldSet } from "../terrain/compile";

describe("executable terrain render passes", () => {
  it("parses line and cubic native paths into deterministic polylines", () => {
    const paths = parseSvgPolylines("M0,0 L5,0 10,0 M10,5 C12,5 14,7 16,8 Z");
    expect(paths).toHaveLength(2);
    expect(paths[0]).toEqual([
      { x: 0, y: 0 },
      { x: 5, y: 0 },
      { x: 10, y: 0 },
    ]);
    expect(paths[1]).toHaveLength(14);
    expect(paths[1]?.at(-1)).toEqual({ x: 10, y: 5 });
    expect(Object.isFrozen(paths)).toBe(true);
  });

  it("binds revisioned geographic passes and cuts draped lines at no-data", () => {
    const scene = sceneFixture();
    const graph = compileExplorerRenderGraph(scene);
    const compiled = compileTerrainRenderPasses(scene, graph, visible());
    expect(compiled?.passes.map(({ id }) => id)).toEqual([
      "validity_background",
      "base_terrain",
      "height_normals_hillshade",
      "ambient_valley_occlusion",
      "contours",
      "water_weather_boundary",
      "features_labels_selection",
    ]);
    expect(
      compiled?.passes.find(({ id }) => id === "base_terrain"),
    ).toMatchObject({
      implementation_revision: "rey.render-pass.base-terrain@1",
      input_revision: "terrain:fixture:source:fixture:material:fixture",
      authority: "derived",
    });
    const contours = compiled?.lines.filter(({ kind }) => kind === "contour");
    expect(contours).toHaveLength(2);
    for (const contour of contours ?? []) {
      const xCoordinates = Array.from(contour.positions).filter(
        (_, component) => component % 3 === 0,
      );
      expect(xCoordinates).not.toContain(10);
      expect(
        Math.min(...xCoordinates) < 10 && Math.max(...xCoordinates) > 10,
      ).toBe(false);
    }
    expect(
      compiled?.lines.find(({ id }) => id.startsWith("stream:"))?.authority,
    ).toBe("derived hydrology fixture");
    expect(
      compiled?.lines.find(({ id }) => id.startsWith("county:fixture:"))
        ?.source_revision,
    ).toBe("county-source:one");
    expect(compiled?.pass_set_id).toContain(graph.graph_id);
    expect(Object.isFrozen(compiled?.lines)).toBe(true);
  });

  it("projects transient layer visibility without changing graph identity", () => {
    const scene = sceneFixture();
    const graph = compileExplorerRenderGraph(scene);
    const all = compileTerrainRenderPasses(scene, graph, visible())!;
    const quiet = compileTerrainRenderPasses(scene, graph, {
      contours: false,
      water: false,
      weather: false,
      probes: false,
    })!;
    expect(graph.graph_id).toBe(compileExplorerRenderGraph(scene).graph_id);
    expect(quiet.pass_set_id).not.toBe(all.pass_set_id);
    expect(quiet.lines.some(({ kind }) => kind === "contour")).toBe(false);
    expect(quiet.lines.some(({ kind }) => kind === "stream")).toBe(false);
    expect(quiet.lines.some(({ kind }) => kind === "weather_front")).toBe(
      false,
    );
    expect(quiet.lines.some(({ kind }) => kind === "road")).toBe(true);
    expect(quiet.lines.some(({ kind }) => kind === "admitted_boundary")).toBe(
      true,
    );
    expect(quiet.points.some(({ kind }) => kind === "frontier")).toBe(false);
  });

  it("does not create accelerated passes without a terrain field", () => {
    const scene = { ...sceneFixture(), terrain_fields: [] };
    expect(
      compileTerrainRenderPasses(
        scene,
        compileExplorerRenderGraph(scene),
        visible(),
      ),
    ).toBeNull();
  });
});

function visible() {
  return { contours: true, water: true, weather: true, probes: true };
}

function sceneFixture(): TopologyScene {
  const terrain = terrainFixture();
  return {
    regime: "landscape",
    label: "FIXTURE LANDSCAPE",
    detail: "bounded fixture",
    focus_id: "node:selected",
    regions: [],
    landforms: [],
    contours: [
      {
        id: "contour",
        path: "M0,5 L20,5",
        level: 2,
        threshold: 0.5,
        anchor_count: 2,
      },
    ],
    natural_features: [
      {
        id: "stream",
        path: "M15,0 L20,5",
        kind: "stream",
        label: "STREAM",
        detail: "derived hydrology fixture",
        intensity: 1,
        workload_id: "fixture",
      },
      {
        id: "weather",
        path: "M15,5 L20,10",
        kind: "weather_front",
        label: "FRONT",
        detail: "derived weather fixture",
        intensity: 0.5,
        workload_id: "fixture",
      },
    ],
    points: [
      {
        id: "anchor",
        focus_id: "anchor",
        kind: "anchor",
        family: "fixture",
        label: "ANCHOR",
        detail: "exact fixture anchor",
        x: 18,
        y: 8,
        prominence: 2,
        signal: "retained",
        action: "inspect",
        tone: "healthy",
        workload_id: "fixture",
      },
      {
        id: "frontier",
        focus_id: "frontier",
        kind: "frontier",
        family: "fixture",
        label: "FRONTIER",
        detail: "bounded fixture frontier",
        x: 17,
        y: 7,
        prominence: 2,
        signal: "unresolved",
        action: "inspect",
        tone: "attention",
        workload_id: "fixture",
      },
    ],
    nodes: [
      {
        id: "node:selected",
        focus_id: "node:selected",
        family: "road",
        label: "ROAD",
        detail: "admitted fixture road",
        x: 18,
        y: 8,
        width: 8,
        tone: "accent",
        evidence_uri: "rey://fixture/road",
        spatial_feature: {
          geometry_kind: "LineString",
          layer: "road",
          envelope_path: "M15,8 L20,8",
          authority: "exact admitted road envelope",
        },
      },
      {
        id: "node:hydrology",
        focus_id: "node:hydrology",
        family: "hydrology",
        label: "WATER",
        detail: "admitted fixture hydrology",
        x: 18,
        y: 7,
        width: 8,
        tone: "neutral",
        spatial_feature: {
          geometry_kind: "LineString",
          layer: "hydrology",
          envelope_path: "M15,7 L20,7",
          authority: "exact admitted hydrology envelope",
        },
      },
    ],
    edges: [],
    omissions: [],
    bearing: {
      status: "charted",
      label: "FIXTURE",
      detail: "bounded fixture",
      sampled_conditions: 2,
      unresolved_boundaries: 1,
    },
    world: { width: 20, height: 10 },
    fit_world: { width: 20, height: 10 },
    terrain: true,
    terrain_fields: [terrain],
    terrain_programs: [],
    globe: null,
    world_atlas_transition: null,
    atlas_landscape_transition: null,
    county_frame: null,
    county_footprint: {
      footprint_id: "county:fixture",
      scene_id: "scene:fixture",
      source_object_id: "county",
      source_artifact_id: "artifact:county",
      source_object_revision: "county-source:one",
      native_bounds: {
        west_microdegrees: -1,
        south_microdegrees: -1,
        east_microdegrees: 1,
        north_microdegrees: 1,
        crosses_antimeridian: false,
      },
      rings: [],
      coordinate_count: 4,
      authority: "exact admitted County boundary",
      path: "M15,1 L20,1 20,9 15,9 Z",
      screen_rings: [],
    },
  };
}

function terrainFixture(): TerrainFieldSet {
  const grid = createFieldGrid(5, 3, { x: 0, y: 0, width: 20, height: 10 });
  const cells = grid.columns * grid.rows;
  const values = Float32Array.from({ length: cells }, (_, index) => index / 10);
  const zero = new Float32Array(cells);
  const vectors = new Float32Array(cells * 3);
  const flow = new Float32Array(cells * 2);
  const tint = new Float32Array(cells * 3).fill(0.45);
  const occlusion = new Float32Array(cells).fill(0.9);
  const roughness = new Float32Array(cells).fill(0.8);
  for (let index = 0; index < cells; index += 1) vectors[index * 3 + 1] = 1;
  const validityValues = new Uint8Array(cells).fill(1);
  validityValues[7] = 0;
  const fields = {
    validity: maskField("validity", "validity:fixture", grid, validityValues),
    elevation: scalarField("elevation", "elevation:fixture", grid, values),
    rainfall: scalarField("rainfall", "rainfall:fixture", grid, zero.slice()),
    flow_direction: vectorField(
      "flow_direction",
      "flow:fixture",
      grid,
      2,
      flow,
    ),
    flow_accumulation: scalarField(
      "flow_accumulation",
      "accumulation:fixture",
      grid,
      zero.slice(),
    ),
    erosion: scalarField("erosion", "erosion:fixture", grid, zero.slice()),
    normal: vectorField("normal", "normal:fixture", grid, 3, vectors),
    curvature: scalarField(
      "curvature",
      "curvature:fixture",
      grid,
      zero.slice(),
    ),
    material: materialField(
      "material",
      "material:fixture",
      grid,
      tint,
      occlusion,
      roughness,
    ),
  };
  return {
    schema: "rey.terrain-fields.v1",
    field_set_id: "terrain:fixture",
    program_id: "program:fixture",
    working_set_id: "working:fixture",
    active_band_ids: [],
    detail_authority: "exact admitted fixture terrain",
    source_revision: "source:fixture",
    grid,
    elevation_scale: 10,
    ...fields,
    field_cells: cells,
    field_bytes: Object.values(fields).reduce(
      (total, field) => total + fieldByteLength(field),
      0,
    ),
  };
}
