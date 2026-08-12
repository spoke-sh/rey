import { describe, expect, it } from "vitest";
import type { ProjectionPacket } from "../../domain";
import {
  compileTerrainProgram,
  materializeTerrainWorkingSet,
  terrainWorkingSetForView,
} from "./compile";
import { PROJECTED_SUPPORT } from "./elevation";

const channelIds = [
  "validity",
  "elevation",
  "rainfall",
  "flow_direction",
  "flow_accumulation",
  "erosion",
  "normal",
  "curvature",
  "material",
] as const;

export const proceduralProjection = {
  schema: "rey.projection-packet.v1",
  packet_id: "packet:one",
  source_patch_id: "patch:one",
  source_topography_revision: "topography:one",
  projection_basis: {
    contract: { id: "basis", revision: 1, semantic_digest: "basis:one" },
    input_dimensions: ["anchor"],
    output_dimensions: ["x", "y"],
    parameters: { elevation_scale_ratio: "0.085" },
    normalization: "fixture",
    random_seed: null,
    distance_semantics: "fixture distance",
    neighborhood_semantics: "fixture neighborhood",
    distortion: "fixture distortion",
    stable_coordinate_rule: "fixture stability",
  },
  scene_compiler: { id: "scene", revision: 1, semantic_digest: "scene:one" },
  extent: { width: 1500, height: 1000, unit: "synthetic_scene_unit" },
  terrain_program: {
    schema: "rey.terrain-program.v1",
    evaluator: {
      id: "rey.projection.procedural-terrain",
      revision: 1,
      semantic_digest: "terrain:one",
    },
    seed: 42,
    bands: [
      {
        band_id: "macro",
        wavelength_scene_units: 420,
        amplitude_microunits: 210000,
        octaves: 2,
        minimum_samples_per_wavelength: 8,
        detail_authority: "derived fixture macro relief",
      },
      {
        band_id: "meso",
        wavelength_scene_units: 105,
        amplitude_microunits: 72000,
        octaves: 3,
        minimum_samples_per_wavelength: 7,
        detail_authority: "derived fixture meso relief",
      },
      {
        band_id: "micro",
        wavelength_scene_units: 24,
        amplitude_microunits: 18000,
        octaves: 2,
        minimum_samples_per_wavelength: 6,
        detail_authority: "presentation-only fixture detail",
      },
    ],
    working_set: {
      max_columns: 255,
      max_rows: 255,
      max_cells: 65025,
      bytes_per_cell: 55,
      max_bytes: 3576375,
      target_sample_spacing_pixels: 4,
      overscan_samples: 3,
      recenter_rule: "fixture camera-relative working set",
    },
    coordinate_rule: "absolute fixture coordinates",
    validity_rule: "fixture support only",
    detail_rule: "camera selects bands without changing evidence",
  },
  objects: [],
  validity: [],
  field_channels: channelIds.map((id) => ({
    id,
    kind:
      id === "validity"
        ? ("mask" as const)
        : id === "normal" || id === "flow_direction" || id === "material"
          ? ("vector" as const)
          : ("scalar" as const),
    semantics: id,
    units: "relative",
    normalization: "fixture",
    source_revision: "topography:one",
    implementation: {
      id: `rey.projection.${id}`,
      revision: 1,
      semantic_digest: `implementation:${id}`,
    },
  })),
  layers: [],
  excluded_source_relationships: 0,
  limits: {
    max_anchor_objects: 64,
    max_frontier_objects: 6,
    max_validity_regions: 256,
    max_field_channels: 12,
    max_terrain_bands: 8,
    max_layers: 8,
    max_omissions: 1032,
    max_working_set_cells: 65025,
    max_working_set_bytes: 3576375,
    max_contours: 7,
    max_natural_features: 96,
    max_labels: 70,
  },
  complete: true,
  degradation: [],
  omissions: [],
  lineage: [],
} satisfies ProjectionPacket;

function program() {
  return compileTerrainProgram({
    source_id: "survey:one",
    source_revision: "topography:one",
    bounds: { x: 100, y: 80, width: 1300, height: 840 },
    anchors: [
      { id: "workspace", x: 750, y: 500, prominence: 4 },
      { id: "document", x: 1010, y: 420, prominence: 2 },
    ],
    atmosphere: [{ x: 1300, y: 240 }],
    unresolved_pressure: 0.4,
    projection: proceduralProjection,
  });
}

describe("procedural terrain compiler", () => {
  it("materializes deterministic bounded working sets without stored terrain levels", () => {
    const firstProgram = program();
    const first = materializeTerrainWorkingSet(firstProgram, {
      working_set_id: "camera:one",
      bounds: firstProgram.bounds,
      columns: 121,
      rows: 81,
      detail_authority: "camera fixture",
    });
    const second = materializeTerrainWorkingSet(program(), {
      working_set_id: "camera:one",
      bounds: firstProgram.bounds,
      columns: 121,
      rows: 81,
      detail_authority: "camera fixture",
    });

    expect(first.active_band_ids).toEqual(["macro", "meso"]);
    expect(first.field_cells).toBe(9801);
    expect(first.field_bytes).toBe(539055);
    expect(Array.from(first.elevation.values)).toEqual(
      Array.from(second.elevation.values),
    );
    expect(Array.from(first.validity.values)).toContain(PROJECTED_SUPPORT);
    expect(Array.from(first.validity.values)).toContain(0);
    expect(first.erosion.maximum).toBeGreaterThan(0);
    expect(first.material.tint.some((value) => value > 0)).toBe(true);

    const supported = first.validity.values.findIndex(
      (value) => value === PROJECTED_SUPPORT,
    );
    const normal = first.normal.values.slice(supported * 3, supported * 3 + 3);
    expect(Math.hypot(...normal)).toBeCloseTo(1, 5);
  });

  it("reveals finer bands as the transient sample spacing tightens", () => {
    const compiled = program();
    const overview = materializeTerrainWorkingSet(compiled, {
      working_set_id: "camera:overview",
      bounds: compiled.bounds,
      columns: 41,
      rows: 27,
      detail_authority: "overview fixture",
    });
    const close = materializeTerrainWorkingSet(compiled, {
      working_set_id: "camera:close",
      bounds: compiled.bounds,
      columns: 255,
      rows: 165,
      detail_authority: "close fixture",
    });
    expect(overview.active_band_ids).toEqual(["macro"]);
    expect(close.active_band_ids).toEqual(["macro", "meso"]);
    expect(close.field_cells).toBeGreaterThan(overview.field_cells);
  });

  it("snaps a bounded working set to the camera envelope", () => {
    const compiled = program();
    const first = terrainWorkingSetForView(compiled, {
      world_width: 1500,
      world_height: 1000,
      viewport_width: 900,
      viewport_height: 600,
      rendered_scale: 1.5,
      pan_x: 0,
      pan_y: 0,
    });
    const subSamplePan = terrainWorkingSetForView(compiled, {
      world_width: 1500,
      world_height: 1000,
      viewport_width: 900,
      viewport_height: 600,
      rendered_scale: 1.5,
      pan_x: 0.5,
      pan_y: 0.5,
    });
    expect(first.bounds).toEqual(subSamplePan.bounds);
    expect(first.columns * first.rows).toBeLessThanOrEqual(
      proceduralProjection.terrain_program.working_set.max_cells,
    );
    expect(first.bounds.width).toBeLessThan(compiled.bounds.width);
  });
});
