import { landscapeReliefFieldByteLength } from "@rey/explorer";
import { describe, expect, it } from "vitest";
import { TerrainTileResidency } from "./residency";
import { executeTerrainCompilationJob } from "./worker";
import { admittedField, terrainTileView } from "./tiles.fixture";

describe("terrain tile residency", () => {
  it("evicts the oldest unrequested tile deterministically", () => {
    const result = executeTerrainCompilationJob({
      job_id: "terrain-job:residency",
      workload_id: "landscape-residency-fixture",
      regime: "landscape",
      fields: [admittedField()],
      programs: [],
      view: terrainTileView(4),
      maximum_cpu_bytes: 8 * 1024 * 1024,
      maximum_gpu_bytes: 8 * 1024 * 1024,
    });
    const [first, second] = result.compiled_tiles;
    if (!first || !second) throw new Error("fixture needs multiple tiles");
    const cpuBytes = (tile: typeof first) =>
      tile.fields.field_bytes + landscapeReliefFieldByteLength(tile.relief);
    const cpuBudget = Math.max(cpuBytes(first), cpuBytes(second));
    const gpuBytes = (tile: typeof first) =>
      tile.mesh.positions.byteLength +
      tile.mesh.normals.byteLength +
      tile.mesh.tint.byteLength +
      tile.mesh.occlusion.byteLength +
      tile.mesh.roughness.byteLength +
      tile.mesh.curvature.byteLength +
      tile.mesh.hillshade.byteLength +
      tile.mesh.salience.byteLength +
      tile.mesh.indices.byteLength;
    const residency = new TerrainTileResidency(
      cpuBudget,
      Math.max(gpuBytes(first), gpuBytes(second)),
    );
    residency.admit([first], [first.descriptor.tile_id]);
    residency.admit([second], [second.descriptor.tile_id]);
    const retained = residency.admit([second], [second.descriptor.tile_id]);
    expect(retained[0]?.descriptor.tile_id).toBe(second.descriptor.tile_id);
    expect(residency.stats()).toMatchObject({
      entries: 1,
      hits: 1,
      misses: 2,
      evictions: 1,
    });
  });

  it("rejects an active set larger than either retained budget", () => {
    const result = executeTerrainCompilationJob({
      job_id: "terrain-job:residency-overflow",
      workload_id: "landscape-residency-overflow-fixture",
      regime: "landscape",
      fields: [admittedField()],
      programs: [],
      view: terrainTileView(4),
      maximum_cpu_bytes: 8 * 1024 * 1024,
      maximum_gpu_bytes: 8 * 1024 * 1024,
    });
    const [first, second] = result.compiled_tiles;
    if (!first || !second) throw new Error("fixture needs multiple tiles");
    expect(() =>
      new TerrainTileResidency(
        first.fields.field_bytes + landscapeReliefFieldByteLength(first.relief),
        8 * 1024 * 1024,
      ).admit(
        [first, second],
        [first.descriptor.tile_id, second.descriptor.tile_id],
      ),
    ).toThrow("active terrain tiles exceed");
  });
});
