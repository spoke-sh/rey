import {
  deriveLandscapeReliefField,
  TERRAIN_VALIDITY_NO_DATA,
} from "@rey/explorer";
import { describe, expect, it } from "vitest";
import {
  deriveRegionalTerrainGeography,
  REGIONAL_TERRAIN_GEOGRAPHY_REVISION,
} from "./regional-geography";
import { refineRegionalTerrainField } from "./refinement";
import { TerrainCompilationWorkerClient } from "./worker-client";
import { executeTerrainCompilationJob } from "./worker";
import { admittedField, terrainTileView } from "./tiles.fixture";

describe("bounded terrain compilation worker", () => {
  it("projects, resamples, and prepares a named tile workload", () => {
    const source = admittedField();
    const noDataIndex = 16 * source.grid.columns + 32;
    source.validity.values[noDataIndex] = 0;
    source.validity_classification!.values[noDataIndex] =
      TERRAIN_VALIDITY_NO_DATA;
    const result = executeTerrainCompilationJob({
      job_id: "terrain-job:one",
      workload_id: "landscape-seam-fixture",
      regime: "landscape",
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
      relief_seam_mismatches: 0,
      relief_partition_mismatches: 0,
      no_data_leak_triangles: 0,
      gpu_timing_ms: null,
      gpu_timing_authority: "unavailable_without_capable_gpu_timer",
    });
    expect(result.compiled.statistics.gpu_bytes).toBeLessThanOrEqual(
      8 * 1024 * 1024,
    );
    expect(
      result.derived_lines.some(
        ({ kind }) => kind === "derived_stream" || kind === "derived_river",
      ),
    ).toBe(false);
    expect(
      result.derived_lines.some(({ kind }) => kind === "derived_contour"),
    ).toBe(true);
    const completeField = deriveRegionalTerrainGeography(
      refineRegionalTerrainField(source),
    );
    const completeRelief = deriveLandscapeReliefField(completeField);
    let independentlyDerivedTileDiffers = false;
    for (const tile of result.compiled_tiles) {
      expect(tile.fields.normal.implementation_revision).toContain(
        REGIONAL_TERRAIN_GEOGRAPHY_REVISION,
      );
      expect(tile.relief).toMatchObject({
        source_field_set_id: completeField.field_set_id,
        source_relief_field_id: completeRelief.relief_field_id,
        derivation_scope: "sampled_from_complete_field",
      });
      const independent = deriveLandscapeReliefField(tile.fields);
      let tileIndex = 0;
      for (const row of tile.descriptor.row_indices) {
        for (const column of tile.descriptor.column_indices) {
          const sourceIndex = row * completeField.grid.columns + column;
          expect(tile.relief.hillshade[tileIndex]).toBe(
            completeRelief.hillshade[sourceIndex],
          );
          expect(tile.relief.salience[tileIndex]).toBe(
            completeRelief.salience[sourceIndex],
          );
          expect(tile.mesh.hillshade[tileIndex]).toBe(
            completeRelief.hillshade[sourceIndex],
          );
          expect(tile.mesh.salience[tileIndex]).toBe(
            completeRelief.salience[sourceIndex],
          );
          independentlyDerivedTileDiffers ||=
            independent.hillshade[tileIndex] !==
              completeRelief.hillshade[sourceIndex] ||
            independent.salience[tileIndex] !==
              completeRelief.salience[sourceIndex];
          tileIndex += 1;
        }
      }
      for (const index of tile.mesh.indices)
        expect(tile.fields.validity.values[index]).toBe(1);
    }
    expect(independentlyDerivedTileDiffers).toBe(true);
  });

  it("rejects CPU overflow and cancels before fallback evaluation", async () => {
    expect(() =>
      executeTerrainCompilationJob({
        job_id: "terrain-job:overflow",
        workload_id: "landscape-budget-fixture",
        regime: "landscape",
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
          regime: "landscape",
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
