import { describe, expect, it } from "vitest";
import {
  compileTerrainProgram,
  materializeTerrainWorkingSet,
} from "../terrain/compile";
import { proceduralProjection } from "../terrain/compile.test-fixture";
import {
  buildTerrainMeshData,
  compileContinuousRelief,
  createContinuousReliefMaterial,
  terrainCameraProjection,
  terrainMeshByteLength,
  verifyTerrainMeshParity,
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

describe("accelerated continuous terrain compiler", () => {
  it("builds triangles only from valid procedural working-set support", () => {
    const fieldSet = fields();
    const mesh = buildTerrainMeshData(fieldSet);
    expect(mesh.positions).toHaveLength(fieldSet.field_cells * 3);
    expect(mesh.indices.length).toBeGreaterThan(0);
    expect(terrainMeshByteLength(mesh)).toBeGreaterThan(fieldSet.field_bytes);
    expect(verifyTerrainMeshParity(fieldSet, mesh)).toBe(fieldSet.field_cells);
    for (const index of mesh.indices)
      expect(fieldSet.validity.values[index]).not.toBe(0);
  });

  it("constructs one TSL material graph and a bounded compiled scene", () => {
    const fieldSet = fields();
    const material = createContinuousReliefMaterial();
    expect(material.isMeshStandardNodeMaterial).toBe(true);
    expect(material.colorNode).not.toBeNull();
    expect(material.roughnessNode).not.toBeNull();
    material.dispose();

    const compiled = compileContinuousRelief([fieldSet]);
    expect(compiled.statistics).toMatchObject({
      field_sets: 1,
      vertices: fieldSet.field_cells,
      field_bytes: fieldSet.field_bytes,
      gpu_budget_bytes: 64 * 1024 * 1024,
      parity_revision: "rey.terrain.cpu-mesh-upload-parity@1",
      parity_samples: fieldSet.field_cells,
    });
    expect(compiled.statistics.triangles).toBeGreaterThan(0);
    expect(compiled.statistics.gpu_bytes).toBeGreaterThan(0);
    expect(compiled.statistics.geometry_compilation_ms).toBeGreaterThanOrEqual(
      0,
    );
    expect(compiled.meshes[0]?.field_set_id).toBe(fieldSet.field_set_id);

    expect(
      terrainCameraProjection(
        { width: 1500, height: 1000 },
        {
          world_width: 1500,
          world_height: 1000,
          viewport_width: 900,
          viewport_height: 600,
          rendered_scale: 2,
          pan_x: 100,
          pan_y: -40,
        },
      ),
    ).toMatchObject({
      center_x: 700,
      center_y: 520,
      left: -225,
      right: 225,
      top: 150,
      bottom: -150,
    });
  });

  it("fails closed when an accelerated input diverges from its CPU field", () => {
    const fieldSet = fields();
    const mesh = buildTerrainMeshData(fieldSet);
    mesh.tint[6] = Math.fround(mesh.tint[6]! + 0.01);
    expect(() => verifyTerrainMeshParity(fieldSet, mesh)).toThrow(
      "diverges from CPU fields at sample 2",
    );
  });

  it("rejects mesh allocation beyond the explicit GPU budget", () => {
    expect(() => compileContinuousRelief([fields()], 1)).toThrow(
      "exceeds GPU budget",
    );
  });
});
