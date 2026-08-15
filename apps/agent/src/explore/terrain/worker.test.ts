import { describe, expect, it } from "vitest";
import { TerrainCompilationWorkerClient } from "./worker-client";
import { executeTerrainCompilationJob } from "./worker";
import { admittedField, terrainTileView } from "./tiles.fixture";

describe("bounded terrain compilation worker", () => {
  it("projects, resamples, and prepares a named tile workload", () => {
    const source = admittedField();
    source.validity.values[16 * source.grid.columns + 32] = 0;
    const result = executeTerrainCompilationJob({
      job_id: "terrain-job:one",
      workload_id: "landscape-seam-fixture",
      fields: [source],
      programs: [],
      view: terrainTileView(4),
      maximum_cpu_bytes: 8 * 1024 * 1024,
      maximum_gpu_bytes: 8 * 1024 * 1024,
    });
    expect(result.execution).toBe("main_thread_fallback");
    expect(result.active_tile_ids.length).toBeGreaterThan(1);
    expect(result.compiled_tiles).toHaveLength(result.active_tile_ids.length);
    expect(result.metrics).toMatchObject({
      workload_id: "landscape-seam-fixture",
      decode_ms: 0,
      maximum_screen_error_pixels: expect.any(Number),
      tile_seam_mismatches: 0,
      no_data_leak_triangles: 0,
      gpu_timing_ms: null,
      gpu_timing_authority: "unavailable_without_capable_gpu_timer",
    });
    expect(result.compiled.statistics.gpu_bytes).toBeLessThanOrEqual(
      8 * 1024 * 1024,
    );
    for (const tile of result.compiled_tiles) {
      expect(tile.fields.normal.implementation_revision).toContain(
        "rey.terrain.worker-relief@1",
      );
      for (const index of tile.mesh.indices)
        expect(tile.fields.validity.values[index]).toBe(1);
    }
  });

  it("rejects CPU overflow and cancels before fallback evaluation", async () => {
    expect(() =>
      executeTerrainCompilationJob({
        job_id: "terrain-job:overflow",
        workload_id: "landscape-budget-fixture",
        fields: [admittedField()],
        programs: [],
        view: terrainTileView(4),
        maximum_cpu_bytes: 1,
        maximum_gpu_bytes: 8 * 1024 * 1024,
      }),
    ).toThrow("exceeds CPU budget");

    const abort = new AbortController();
    abort.abort();
    await expect(
      new TerrainCompilationWorkerClient().compile(
        {
          job_id: "terrain-job:cancelled",
          workload_id: "landscape-cancellation-fixture",
          fields: [admittedField()],
          programs: [],
          view: terrainTileView(4),
          maximum_cpu_bytes: 8 * 1024 * 1024,
          maximum_gpu_bytes: 8 * 1024 * 1024,
        },
        abort.signal,
      ),
    ).rejects.toMatchObject({ name: "AbortError" });
  });
});
