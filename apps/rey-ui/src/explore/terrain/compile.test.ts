import { describe, expect, it } from "vitest";
import type { ProjectionPacket } from "../../domain";
import { fieldPoint } from "../engine/fields";
import { compileTerrainFieldPyramid, terrainFieldForRegime } from "./compile";
import { PROJECTED_SUPPORT } from "./elevation";

const channels = [
  "validity",
  "elevation",
  "rainfall",
  "flow_direction",
  "flow_accumulation",
  "erosion",
  "normal",
  "curvature",
  "material",
].map((id) => ({
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
  source_revision: "patch:one",
  implementation: {
    id: `rey.projection.${id}`,
    revision: 1,
    semantic_digest: `implementation:${id}`,
  },
}));

const projection = {
  schema: "rey.projection-packet.v2",
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
  field_pyramid: {
    schema: "rey.terrain-field-pyramid.v1",
    levels: [
      {
        level_id: "overview",
        columns: 6,
        rows: 5,
        cells: 30,
        bytes_per_cell: 55,
        total_bytes: 1650,
        sample_stride: 4,
        regimes: ["world"],
        detail_authority: "coarse fixture resampling",
      },
      {
        level_id: "regional",
        columns: 11,
        rows: 9,
        cells: 99,
        bytes_per_cell: 55,
        total_bytes: 5445,
        sample_stride: 2,
        regimes: ["atlas", "landscape"],
        detail_authority: "regional fixture resampling",
      },
      {
        level_id: "local",
        columns: 21,
        rows: 17,
        cells: 357,
        bytes_per_cell: 55,
        total_bytes: 19635,
        sample_stride: 1,
        regimes: ["neighborhoods", "objects", "evidence"],
        detail_authority: "local fixture resampling",
      },
    ],
    total_cells: 486,
    total_bytes: 26730,
    stable_coordinate_rule: "nested fixture coordinates",
  },
  objects: [],
  validity: [],
  field_channels: channels,
  layers: [],
  excluded_source_relationships: 0,
  limits: {
    max_anchor_objects: 64,
    max_frontier_objects: 6,
    max_validity_regions: 256,
    max_field_channels: 12,
    max_field_levels: 3,
    max_layers: 8,
    max_omissions: 1032,
    max_field_cells: 357,
    max_field_bytes: 19635,
    max_total_field_cells: 486,
    max_total_field_bytes: 26730,
    max_contours: 7,
    max_natural_features: 96,
    max_labels: 70,
  },
  complete: true,
  degradation: [],
  omissions: [],
  lineage: [],
} satisfies ProjectionPacket;

describe("terrain field compiler", () => {
  it("deterministically derives bounded validity, hydrology, relief, and material channels", () => {
    const input = {
      source_id: "survey:one",
      source_revision: "topography:one",
      bounds: {
        x: 100,
        y: 80,
        width: 1300,
        height: 840,
      },
      anchors: [
        { id: "workspace", x: 750, y: 500, prominence: 4 },
        { id: "document", x: 1010, y: 420, prominence: 2 },
      ],
      atmosphere: [{ x: 1300, y: 240 }],
      unresolved_pressure: 0.4,
      projection,
    } as const;
    const first = compileTerrainFieldPyramid(input);
    const second = compileTerrainFieldPyramid(input);
    const local = terrainFieldForRegime(first, "neighborhoods");
    const regional = terrainFieldForRegime(first, "atlas");
    const overview = terrainFieldForRegime(first, "world");

    expect(first.levels.map((level) => level.level_id)).toEqual([
      "overview",
      "regional",
      "local",
    ]);
    expect(first.total_cells).toBe(projection.field_pyramid.total_cells);
    expect(first.total_bytes).toBe(projection.field_pyramid.total_bytes);
    expect(Array.from(local.elevation.values)).toEqual(
      Array.from(second.levels[2]!.elevation.values),
    );
    expect(Array.from(local.validity.values)).toContain(PROJECTED_SUPPORT);
    expect(Array.from(local.validity.values)).toContain(0);
    expect(local.erosion.maximum).toBeGreaterThan(0);
    expect(local.flow_accumulation.maximum).toBeGreaterThan(
      local.rainfall.maximum,
    );
    expect(local.material.tint.some((value) => value > 0)).toBe(true);

    expect(fieldPoint(overview.grid, 1, 1)).toEqual(
      fieldPoint(local.grid, 4, 4),
    );
    expect(fieldPoint(regional.grid, 3, 2)).toEqual(
      fieldPoint(local.grid, 6, 4),
    );

    const supported = local.validity.values.findIndex(
      (value) => value === PROJECTED_SUPPORT,
    );
    const normal = local.normal.values.slice(supported * 3, supported * 3 + 3);
    expect(Math.hypot(...normal)).toBeCloseTo(1, 5);
  });
});
