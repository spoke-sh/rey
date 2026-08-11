import { describe, expect, it } from "vitest";
import type { ProjectionPacket } from "../../domain";
import { createFieldGrid } from "../engine/fields";
import { compileTerrainFields } from "../terrain/compile";
import {
  buildTerrainMeshData,
  createContinuousReliefBundle,
  createContinuousReliefMaterial,
} from "./three-terrain";

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

const projection = {
  projection_basis: {
    parameters: { elevation_scale_ratio: "0.085" },
  },
  field_layout: {
    columns: 13,
    rows: 9,
    cells: 117,
    bytes_per_cell: 55,
    total_bytes: 6435,
  },
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
  limits: {
    max_field_channels: 12,
    max_field_cells: 2501,
    max_field_bytes: 160064,
  },
} as unknown as ProjectionPacket;

function fields() {
  return compileTerrainFields({
    source_id: "survey:one",
    source_revision: "topography:one",
    grid: createFieldGrid(13, 9, {
      x: 100,
      y: 80,
      width: 1300,
      height: 840,
    }),
    anchors: [{ id: "workspace", x: 750, y: 500, prominence: 4 }],
    atmosphere: [],
    unresolved_pressure: 0,
    projection,
  });
}

describe("Three.js continuous terrain", () => {
  it("builds triangles only from valid field support", () => {
    const fieldSet = fields();
    const mesh = buildTerrainMeshData(fieldSet);
    expect(mesh.positions).toHaveLength(fieldSet.field_cells * 3);
    expect(mesh.indices.length).toBeGreaterThan(0);
    for (const index of mesh.indices)
      expect(fieldSet.validity.values[index]).not.toBe(0);
  });

  it("constructs one TSL material graph and disposable scene bundle", () => {
    const fieldSet = fields();
    const material = createContinuousReliefMaterial();
    expect(material.isMeshStandardNodeMaterial).toBe(true);
    expect(material.colorNode).not.toBeNull();
    expect(material.roughnessNode).not.toBeNull();
    material.dispose();

    const bundle = createContinuousReliefBundle([fieldSet], {
      width: 1500,
      height: 1000,
    });
    expect(bundle.statistics).toMatchObject({
      field_sets: 1,
      vertices: fieldSet.field_cells,
      field_bytes: fieldSet.field_bytes,
    });
    expect(bundle.statistics.triangles).toBeGreaterThan(0);
    bundle.dispose();
  });
});
