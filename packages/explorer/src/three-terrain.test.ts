import { describe, expect, it } from "vitest";
import {
  buildTerrainMeshData,
  compileContinuousRelief,
  createContinuousReliefMaterial,
  terrainCameraProjection,
  terrainMeshByteLength,
  terrainNoDataLeakTriangleCount,
  verifyTerrainMeshParity,
} from "./three-terrain";
import { terrainFieldFixture, terrainRenderPassFixture } from "./test-fixtures";

describe("accelerated continuous terrain compiler", () => {
  it("builds triangles only from valid procedural working-set support", () => {
    const fieldSet = terrainFieldFixture();
    const mesh = buildTerrainMeshData(fieldSet);
    expect(mesh.positions).toHaveLength(fieldSet.field_cells * 3);
    expect(mesh.indices.length).toBeGreaterThan(0);
    expect(terrainMeshByteLength(mesh)).toBeGreaterThan(fieldSet.field_bytes);
    expect(verifyTerrainMeshParity(fieldSet, mesh)).toBe(fieldSet.field_cells);
    expect(terrainNoDataLeakTriangleCount(fieldSet, mesh)).toBe(0);
    for (const index of mesh.indices)
      expect(fieldSet.validity.values[index]).not.toBe(0);
  });

  it("constructs one TSL material graph and a bounded compiled scene", () => {
    const fieldSet = terrainFieldFixture();
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
      target: [700, 0, 520],
    });

    const orbit = terrainCameraProjection(
      { width: 1500, height: 1000 },
      {
        world_width: 1500,
        world_height: 1000,
        viewport_width: 900,
        viewport_height: 600,
        rendered_scale: 2,
        pan_x: 100,
        pan_y: -40,
        pitch_degrees: 35.26439,
        yaw_degrees: 45,
      },
    );
    expect(orbit.position[1]).toBeGreaterThan(0);
    expect(orbit.position[0]).not.toBe(orbit.center_x);
    expect(orbit.position[2]).not.toBe(orbit.center_y);
    expect(orbit.target).toEqual([orbit.center_x, 0, orbit.center_y]);
  });

  it("binds executable material stages to the compiled pass-set revision", () => {
    const fieldSet = terrainFieldFixture();
    const passes = terrainRenderPassFixture();
    const material = createContinuousReliefMaterial(passes);
    expect(material.name).toBe(
      "rey.terrain.tsl-continuous-relief@1:terrain-passes:fixture",
    );
    expect(material.colorNode).not.toBeNull();
    material.dispose();

    const compiled = compileContinuousRelief(
      [fieldSet],
      64 * 1024 * 1024,
      passes,
    );
    expect(compiled.render_passes).toBe(passes);
    expect(compiled.material_revision).toContain(passes.pass_set_id);
  });

  it("chooses the supported cell diagonal around explicit no-data", () => {
    const fieldSet = terrainFieldFixture();
    fieldSet.validity.values[6] = 0;
    const mesh = buildTerrainMeshData(fieldSet);
    expect(mesh.indices.length).toBeGreaterThan(0);
    expect([...mesh.indices]).not.toContain(6);
    expect(mesh.indices.length % 3).toBe(0);
  });

  it("fails closed when an accelerated input diverges from its CPU field", () => {
    const fieldSet = terrainFieldFixture();
    const mesh = buildTerrainMeshData(fieldSet);
    mesh.tint[6] = Math.fround(mesh.tint[6]! + 0.01);
    expect(() => verifyTerrainMeshParity(fieldSet, mesh)).toThrow(
      "diverges from CPU fields at sample 2",
    );
  });

  it("counts any triangle that leaks across explicit no-data", () => {
    const fieldSet = terrainFieldFixture();
    const mesh = buildTerrainMeshData(fieldSet);
    const leakedVertex = mesh.indices[0]!;
    fieldSet.validity.values[leakedVertex] = 0;
    expect(terrainNoDataLeakTriangleCount(fieldSet, mesh)).toBeGreaterThan(0);
    expect(() => verifyTerrainMeshParity(fieldSet, mesh)).toThrow(
      "indexes invalid CPU support",
    );
  });

  it("rejects mesh allocation beyond the explicit GPU budget", () => {
    expect(() => compileContinuousRelief([terrainFieldFixture()], 1)).toThrow(
      "exceeds GPU budget",
    );
  });
});
