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
import {
  executeTerrainCompilationJob,
  MAX_MATERIALIZED_LANDSCAPE_CACHE_BYTES,
  MAX_TERRAIN_COMPILATION_OUTPUT_BYTES,
  TERRAIN_CARTOGRAPHY_MAXIMUM_HIERARCHY_LEVEL,
  terrainCompilationResultForWorkerTransfer,
  terrainCompilationResultWithTransferMetrics,
  terrainCompilationTransferableBuffers,
  TERRAIN_COMPILATION_FABRIC_SAMPLE_LIMIT,
  TERRAIN_COMPILATION_WORKER_REVISION,
} from "./worker";
import { admittedField, terrainTileView } from "./tiles.fixture";

describe("bounded terrain compilation worker", () => {
  it("retains the bounded high-density hierarchy output budget", () => {
    expect(TERRAIN_COMPILATION_WORKER_REVISION).toBe(
      "rey.terrain.compilation-worker@23",
    );
    expect(MAX_TERRAIN_COMPILATION_OUTPUT_BYTES).toBe(160 * 1024 * 1024);
    expect(MAX_MATERIALIZED_LANDSCAPE_CACHE_BYTES).toBe(112 * 1024 * 1024);
    expect(TERRAIN_COMPILATION_FABRIC_SAMPLE_LIMIT).toBe(2_600);
    expect(TERRAIN_CARTOGRAPHY_MAXIMUM_HIERARCHY_LEVEL).toBe(6);
  });

  it("projects, resamples, and prepares a named tile workload", () => {
    const source = admittedField();
    const noDataIndex = 16 * source.grid.columns + 32;
    source.validity.values[noDataIndex] = 0;
    source.validity_classification!.values[noDataIndex] =
      TERRAIN_VALIDITY_NO_DATA;
    const result = executeTerrainCompilationJob({
      job_id: "terrain-job:one",
      source_key: "terrain-source:worker-fixture",
      workload_id: "landscape-seam-fixture",
      regime: "landscape",
      fields: [source],
      programs: [],
      view: terrainTileView(4),
      maximum_cpu_bytes: 24 * 1024 * 1024,
      maximum_gpu_bytes: 8 * 1024 * 1024,
    });
    expect(result.execution).toBe("main_thread_fallback");
    expect(result.active_tile_ids.length).toBeGreaterThan(1);
    expect(result.compiled_tiles).toHaveLength(result.active_tile_ids.length);
    expect(result.cartography_fields.length).toBeGreaterThan(0);
    expect(result.cartography_tile_ids.length).toBe(
      result.cartography_fields.length,
    );
    const cartographyTileIds = new Set(result.cartography_tile_ids);
    expect(
      result.pyramids
        .flatMap(({ tiles }) => tiles)
        .filter(({ tile_id }) => cartographyTileIds.has(tile_id))
        .every(
          ({ level }) => level <= TERRAIN_CARTOGRAPHY_MAXIMUM_HIERARCHY_LEVEL,
        ),
    ).toBe(true);
    expect(
      result.cartography_fields.reduce(
        (total, field) => total + field.field_cells,
        0,
      ),
    ).toBe(result.metrics.cartography_field_cells);
    expect(result.landscape_pyramids).toHaveLength(1);
    expect(result.height_hierarchies).toHaveLength(1);
    expect(result.height_hierarchies[0]).toMatchObject({
      complete: true,
      levels: expect.arrayContaining([
        expect.objectContaining({
          height_id: expect.stringMatching(/^blake3:/),
          source_contribution_id: expect.stringMatching(/^blake3:/),
        }),
      ]),
    });
    expect(result.compiled.pyramid_envelopes).toEqual(
      result.landscape_pyramids,
    );
    const finestTextureField =
      result.materialized_landscape_pyramids[0]!.relief_levels.at(-1)!.field;
    expect(result.compiled.cartographic_textures).toEqual([
      expect.objectContaining({
        columns: finestTextureField.grid.columns,
        rows: finestTextureField.grid.rows,
        rgba: expect.any(Uint8Array),
      }),
    ]);
    expect(result.compiled.cartographic_textures[0]!.rgba).toHaveLength(
      finestTextureField.field_cells * 4,
    );
    expect(result.terrain_fabrics).toEqual([]);
    expect(result.landscape_pyramids[0]).toMatchObject({
      height_pyramid: { complete: true },
      relief_pyramid: { complete: true },
    });
    expect(result.materialized_landscape_pyramids[0]).toMatchObject({
      border_mismatches: 0,
      partition_mismatches: 0,
    });
    expect(result.metrics).toMatchObject({
      workload_id: "landscape-seam-fixture",
      decode_ms: 0,
      maximum_screen_error_pixels: expect.any(Number),
      tile_seam_mismatches: 0,
      relief_seam_mismatches: 0,
      relief_partition_mismatches: 0,
      no_data_leak_triangles: 0,
      height_hierarchy_levels: result.height_hierarchies[0]!.levels.length,
      height_hierarchy_bytes: result.height_hierarchies[0]!.byte_length,
      relief_hierarchy_levels:
        result.materialized_landscape_pyramids[0]!.relief_levels.length,
      relief_hierarchy_bytes: expect.any(Number),
      relief_derived_bytes: expect.any(Number),
      relief_halo_source_cells: expect.any(Number),
      selected_tile_cpu_bytes: expect.any(Number),
      selected_tile_gpu_bytes: expect.any(Number),
      materialized_pyramid_cache_hits: 0,
      materialized_pyramid_cache_misses: 1,
      relief_border_digest_mismatches: 0,
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
    const moving = executeTerrainCompilationJob({
      job_id: "terrain-job:moving",
      source_key: "terrain-source:worker-fixture",
      workload_id: "landscape-moving-fixture",
      regime: "landscape",
      fields: [source],
      programs: [],
      view: terrainTileView(4),
      maximum_hierarchy_level: 3,
      presentation_mode: "moving",
      maximum_cpu_bytes: 24 * 1024 * 1024,
      maximum_gpu_bytes: 8 * 1024 * 1024,
    });
    expect(moving.derived_lines).toEqual([]);
    expect(moving.terrain_fabrics).toHaveLength(1);
    expect(moving.terrain_fabrics[0]).toMatchObject({
      source_field_set_id: source.field_set_id,
      hierarchy_id: result.materialized_landscape_pyramids[0]!.hierarchy_id,
      relief_field_id: expect.stringContaining(
        "rey.terrain.relief-hierarchy@3",
      ),
      samples: expect.arrayContaining([
        expect.objectContaining({
          source_sample_id: expect.any(String),
          reveal_priority: expect.any(Number),
        }),
      ]),
    });
    expect(moving.selections[0]!.level).toBeLessThanOrEqual(3);
    expect(moving.selections[0]!.level).toBeLessThan(
      result.selections[0]!.level,
    );
    const completeField = deriveRegionalTerrainGeography(
      refineRegionalTerrainField(source),
    );
    const completeRelief =
      result.materialized_landscape_pyramids[0]!.relief_levels.at(-1)!.relief;
    let independentlyDerivedTileDiffers = false;
    for (const [tileSequence, tile] of result.compiled_tiles.entries()) {
      expect(tile.fields.normal.implementation_revision).toContain(
        REGIONAL_TERRAIN_GEOGRAPHY_REVISION,
      );
      expect(tile.relief).toMatchObject({
        source_field_set_id: completeField.field_set_id,
        source_relief_field_id: completeRelief.relief_field_id,
        derivation_scope: "sampled_from_complete_field",
      });
      const independent =
        tileSequence === 0 ? deriveLandscapeReliefField(tile.fields) : null;
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
            independent !== null &&
            (independent.hillshade[tileIndex] !==
              completeRelief.hillshade[sourceIndex] ||
              independent.salience[tileIndex] !==
                completeRelief.salience[sourceIndex]);
          tileIndex += 1;
        }
      }
      for (const index of tile.mesh.indices)
        expect(tile.fields.validity.values[index]).toBe(1);
    }
    expect(independentlyDerivedTileDiffers).toBe(true);
    const retained = executeTerrainCompilationJob({
      job_id: "terrain-job:two",
      source_key: "terrain-source:worker-fixture",
      workload_id: "landscape-seam-fixture",
      regime: "objects",
      fields: [source],
      programs: [],
      view: terrainTileView(5),
      maximum_cpu_bytes: 24 * 1024 * 1024,
      maximum_gpu_bytes: 8 * 1024 * 1024,
    });
    expect(retained.metrics).toMatchObject({
      materialized_pyramid_cache_hits: 1,
      materialized_pyramid_cache_misses: 0,
    });
    expect(retained.height_hierarchies[0]!.hierarchy_id).toBe(
      result.height_hierarchies[0]!.hierarchy_id,
    );
    const workerResult = terrainCompilationResultForWorkerTransfer(
      retained,
      "registered_source",
    );
    expect(workerResult.pyramids).toEqual([]);
    expect(workerResult.materialized_landscape_pyramids).toEqual([]);
    expect(workerResult.height_hierarchies).toEqual([]);
    expect(workerResult.cartography_fields).toHaveLength(
      retained.cartography_fields.length,
    );
    expect(workerResult.height_hierarchy_summaries).toEqual([
      expect.objectContaining({
        hierarchy_id: retained.height_hierarchies[0]!.hierarchy_id,
        complete: true,
      }),
    ]);
    expect(workerResult.relief_hierarchy_summaries[0]!.levels).toHaveLength(
      retained.materialized_landscape_pyramids[0]!.relief_levels.length,
    );
    expect(workerResult.transport).toMatchObject({
      source_payload: "registered_source",
      result_payload: "active_working_set",
      worker_retained_hierarchy_bytes:
        retained.metrics.height_hierarchy_bytes +
        retained.metrics.relief_hierarchy_bytes,
    });
    const transfer = terrainCompilationTransferableBuffers(workerResult);
    expect(transfer.length).toBeGreaterThan(0);
    const metered = terrainCompilationResultWithTransferMetrics(
      workerResult,
      transfer,
    );
    expect(metered.transport.transferred_array_buffers).toBe(transfer.length);
    expect(metered.transport.transferred_bytes).toBe(
      transfer.reduce((total, buffer) => total + buffer.byteLength, 0),
    );
    expect(metered.transport.transferred_bytes).toBeGreaterThan(0);
  }, 15_000);

  it("rejects CPU overflow and cancels before fallback evaluation", async () => {
    expect(() =>
      executeTerrainCompilationJob({
        job_id: "terrain-job:overflow",
        source_key: "terrain-source:overflow-fixture",
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
          source_key: "terrain-source:cancellation-fixture",
          workload_id: "landscape-cancellation-fixture",
          regime: "landscape",
          fields: [admittedField()],
          programs: [],
          view: terrainTileView(4),
          maximum_cpu_bytes: 24 * 1024 * 1024,
          maximum_gpu_bytes: 8 * 1024 * 1024,
        },
        abort.signal,
      ),
    ).rejects.toMatchObject({ name: "AbortError" });
  });
});
