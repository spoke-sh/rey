import { describe, expect, it } from "vitest";
import {
  compileTerrainProgram,
  materializeTerrainWorkingSet,
} from "../terrain/compile";
import { proceduralProjection } from "../terrain/compile.test";
import {
  buildTerrainMeshData,
  createContinuousReliefBundle,
  createContinuousReliefMaterial,
  terrainMeshByteLength,
} from "./three-terrain";

function fields() {
  const program = compileTerrainProgram({
    source_id: "survey:one",
    source_revision: "topography:one",
    bounds: { x: 100, y: 80, width: 1300, height: 840 },
    anchors: [{ id: "workspace", x: 750, y: 500, prominence: 4 }],
    atmosphere: [],
    unresolved_pressure: 0,
    projection: proceduralProjection,
  });
  return materializeTerrainWorkingSet(program, {
    working_set_id: "renderer:fixture",
    bounds: program.bounds,
    columns: 61,
    rows: 41,
    detail_authority: "renderer fixture",
  });
}

describe("Three.js continuous terrain", () => {
  it("builds triangles only from valid procedural working-set support", () => {
    const fieldSet = fields();
    const mesh = buildTerrainMeshData(fieldSet);
    expect(mesh.positions).toHaveLength(fieldSet.field_cells * 3);
    expect(mesh.indices.length).toBeGreaterThan(0);
    expect(terrainMeshByteLength(mesh)).toBeGreaterThan(fieldSet.field_bytes);
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
      gpu_budget_bytes: 64 * 1024 * 1024,
    });
    expect(bundle.statistics.triangles).toBeGreaterThan(0);
    expect(bundle.statistics.gpu_bytes).toBeGreaterThan(0);
    bundle.dispose();
  });

  it("rejects mesh allocation beyond the explicit GPU budget", () => {
    expect(() =>
      createContinuousReliefBundle(
        [fields()],
        { width: 1500, height: 1000 },
        undefined,
        1,
      ),
    ).toThrow("exceeds GPU budget");
  });
});
