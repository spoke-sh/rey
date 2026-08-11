import { describe, expect, it } from "vitest";
import type { ProjectionPacket } from "../../domain";
import { createFieldGrid } from "../engine/fields";
import { compileTerrainFields } from "./compile";
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
  field_layout: {
    columns: 21,
    rows: 15,
    cells: 315,
    bytes_per_cell: 55,
    total_bytes: 17325,
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
    max_layers: 8,
    max_omissions: 1032,
    max_field_cells: 2501,
    max_field_bytes: 160064,
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
      grid: createFieldGrid(21, 15, {
        x: 100,
        y: 80,
        width: 1300,
        height: 840,
      }),
      anchors: [
        { id: "workspace", x: 750, y: 500, prominence: 4 },
        { id: "document", x: 1010, y: 420, prominence: 2 },
      ],
      atmosphere: [{ x: 1300, y: 240 }],
      unresolved_pressure: 0.4,
      projection,
    } as const;
    const first = compileTerrainFields(input);
    const second = compileTerrainFields(input);

    expect(first.field_cells).toBe(315);
    expect(first.field_bytes).toBe(projection.field_layout.total_bytes);
    expect(first.field_bytes).toBeLessThanOrEqual(
      projection.limits.max_field_bytes,
    );
    expect(Array.from(first.elevation.values)).toEqual(
      Array.from(second.elevation.values),
    );
    expect(Array.from(first.validity.values)).toContain(PROJECTED_SUPPORT);
    expect(Array.from(first.validity.values)).toContain(0);
    expect(first.erosion.maximum).toBeGreaterThan(0);
    expect(first.flow_accumulation.maximum).toBeGreaterThan(
      first.rainfall.maximum,
    );
    expect(first.material.tint.some((value) => value > 0)).toBe(true);

    const supported = first.validity.values.findIndex(
      (value) => value === PROJECTED_SUPPORT,
    );
    const normal = first.normal.values.slice(supported * 3, supported * 3 + 3);
    expect(Math.hypot(...normal)).toBeCloseTo(1, 5);
  });
});
