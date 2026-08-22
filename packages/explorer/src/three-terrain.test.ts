import { describe, expect, it } from "vitest";
import {
  buildTerrainMeshData,
  compileContinuousRelief,
  continuousReliefMaterialRevision,
  createContinuousReliefMaterial,
  projectTerrainCoordinate,
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
    expect(material.isMeshBasicNodeMaterial).toBe(true);
    expect(material.colorNode).not.toBeNull();
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

  it("projects terrain points through the same orbit camera terrainCameraProjection poses", () => {
    const world = { width: 1500, height: 1000 };

    // Pure top-down (pitch 90) is an identity mapping onto the ground plane.
    expect(
      projectTerrainCoordinate(
        { x: 1200, z: 300 },
        { pitch_degrees: 90, yaw_degrees: 0 },
        world,
      ),
    ).toEqual({ x: 1200, y: 300 });

    expect(
      projectTerrainCoordinate(
        { x: 1200, z: 300 },
        { pitch_degrees: 45, yaw_degrees: 45 },
        world,
      ),
    ).toEqual({ x: 1209.6194077712557, y: 625 });

    // Elevation lifts the point up-screen, independent of the ground-plane
    // yaw rotation already exercised above.
    expect(
      projectTerrainCoordinate(
        { x: 1200, z: 300, elevation: 50 },
        { pitch_degrees: 28, yaw_degrees: -90 },
        world,
      ),
    ).toEqual({ x: 550, y: 244.59041710340279 });

    // The orbit target always projects to the pivot itself, for any pose —
    // the same invariant terrainCameraProjection's own `target` encodes.
    for (const view of [
      { pitch_degrees: 90, yaw_degrees: 0 },
      { pitch_degrees: 60, yaw_degrees: 120 },
      { pitch_degrees: 22, yaw_degrees: -180 },
    ]) {
      expect(
        projectTerrainCoordinate(
          { x: world.width / 2, z: world.height / 2 },
          view,
          world,
        ),
      ).toEqual({ x: world.width / 2, y: world.height / 2 });
    }
  });

  it("separates the base material graph from independently changing overlays", () => {
    const fieldSet = terrainFieldFixture();
    const passes = terrainRenderPassFixture();
    const material = createContinuousReliefMaterial(passes);
    expect(material.name).toBe(
      "rey.terrain.tsl-cartographic-relief@4:rey.landscape-relief-engine@1",
    );
    expect(material.colorNode).not.toBeNull();
    material.dispose();

    const changedOverlays = {
      ...passes,
      pass_set_id: "terrain-passes:changed-overlays",
      lines: [],
      points: [],
    };
    expect(continuousReliefMaterialRevision(changedOverlays)).toBe(
      continuousReliefMaterialRevision(passes),
    );
    const withoutHillshade = {
      ...passes,
      passes: passes.passes.filter(
        ({ id }) => id !== "height_normals_hillshade",
      ),
    };
    expect(continuousReliefMaterialRevision(withoutHillshade)).toContain(
      "without=height_normals_hillshade",
    );

    const compiled = compileContinuousRelief(
      [fieldSet],
      64 * 1024 * 1024,
      passes,
    );
    expect(compiled.render_passes).toBe(passes);
    expect(compiled.material_revision).toBe(material.name);
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
