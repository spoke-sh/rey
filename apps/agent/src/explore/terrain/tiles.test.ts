import {
  buildTerrainMeshData,
  deriveLandscapeReliefField,
  TERRAIN_VALIDITY_NO_DATA,
  terrainTriangleIndices,
} from "@rey/explorer";
import { describe, expect, it } from "vitest";
import { admittedField, terrainTileView } from "./tiles.fixture";
import { compileMaterializedLandscapePyramid } from "./relief-pyramid";
import {
  materializeTerrainTile,
  materializeTerrainTileRelief,
  projectTerrainTilePyramid,
  projectMaterializedLandscapeTilePyramid,
  selectTerrainTilesForView,
  terrainTileReliefPartitionMismatchCount,
  terrainTileReliefSeamMismatchCount,
  terrainTileSeamMismatchCount,
} from "./tiles";

describe("admitted terrain tile projection", () => {
  it("retains stable parentage and identical shared validity borders", () => {
    const source = admittedField();
    const seamIndex = 16 * source.grid.columns + 32;
    source.validity.values[seamIndex] = 0;
    source.validity_classification!.values[seamIndex] =
      TERRAIN_VALIDITY_NO_DATA;
    const pyramid = projectTerrainTilePyramid(source);
    expect(pyramid.maximum_level).toBe(2);
    const left = tileAt(pyramid, 2, 0, 0);
    const right = tileAt(pyramid, 2, 1, 0);
    expect(left.child_ids).toEqual([]);
    expect(left.parent_id).toBe(tileAt(pyramid, 1, 0, 0).tile_id);
    expect(left.column_indices.at(-1)).toBe(right.column_indices[0]);
    expect(left.validity_border.east).toBe(right.validity_border.west);
    expect(left.validity_border.east[16]).toBe("0");
    expect(terrainTileSeamMismatchCount([left, right])).toBe(0);

    for (const descriptor of [left, right]) {
      const tile = materializeTerrainTile(source, descriptor);
      for (const index of terrainTriangleIndices(tile))
        expect(tile.validity.values[index]).toBe(1);
    }
  });

  it("detects a mismatched retained validity seam", () => {
    const pyramid = projectTerrainTilePyramid(admittedField());
    const left = tileAt(pyramid, 2, 0, 0);
    const right = tileAt(pyramid, 2, 1, 0);
    const mismatched = {
      ...right,
      validity_border: { ...right.validity_border, west: "0" },
    };
    expect(terrainTileSeamMismatchCount([left, mismatched])).toBe(1);
  });

  it("samples relief from complete-field support across render-tile seams", () => {
    const source = {
      ...admittedField(),
      relief_metrics: {
        schema: "rey.terrain-relief-metrics.v1" as const,
        sample_spacing_x_meters: 300,
        sample_spacing_y_meters: 300,
        elevation_range_meters: 1_800,
        authority: "fixture metric terrain grid",
      },
    };
    const pyramid = projectTerrainTilePyramid(source);
    const descriptors = [tileAt(pyramid, 2, 0, 0), tileAt(pyramid, 2, 1, 0)];
    const completeRelief = deriveLandscapeReliefField(source);
    const sampled = descriptors.map((descriptor) => {
      const fields = materializeTerrainTile(source, descriptor);
      const relief = materializeTerrainTileRelief(
        source,
        completeRelief,
        descriptor,
      );
      return {
        descriptor,
        mesh: buildTerrainMeshData(fields, relief),
      };
    });
    const independentlyDerived = descriptors.map((descriptor) => {
      const fields = materializeTerrainTile(source, descriptor);
      return { descriptor, mesh: buildTerrainMeshData(fields) };
    });

    expect(terrainTileReliefSeamMismatchCount(sampled)).toBe(0);
    expect(
      terrainTileReliefPartitionMismatchCount(completeRelief, sampled),
    ).toBe(0);
    expect(
      terrainTileReliefPartitionMismatchCount(
        completeRelief,
        independentlyDerived,
      ),
    ).toBeGreaterThan(0);
  });

  it("can only remove support at coarse levels and refines by screen error", () => {
    const source = admittedField();
    const noDataIndex = 16 * source.grid.columns + 16;
    source.validity.values[noDataIndex] = 0;
    source.validity_classification!.values[noDataIndex] =
      TERRAIN_VALIDITY_NO_DATA;
    const pyramid = projectTerrainTilePyramid(source);
    const root = tileAt(pyramid, 0, 0, 0);
    expect(root.no_data_vertices).toBeGreaterThan(0);
    expect(root.unsupported_vertices).toBe(0);
    expect(root.geometric_error).toBeGreaterThan(0);

    const overview = selectTerrainTilesForView(pyramid, terrainTileView(0.001));
    const close = selectTerrainTilesForView(pyramid, terrainTileView(4));
    expect(overview.level).toBe(0);
    expect(close.level).toBeGreaterThan(overview.level);
    expect(close.screen_error_pixels).toBeLessThanOrEqual(
      root.geometric_error * 4,
    );
  });

  it("selects exact conservative hierarchy levels with revision-bound tiles", () => {
    const materialized = compileMaterializedLandscapePyramid(admittedField());
    const pyramid = projectMaterializedLandscapeTilePyramid(materialized);
    expect(pyramid.maximum_level).toBe(materialized.relief_levels.length - 1);
    expect([...new Set(pyramid.tiles.map(({ level }) => level))]).toHaveLength(
      materialized.relief_levels.length,
    );
    for (const tile of pyramid.tiles) {
      const reliefLevel = materialized.relief_levels[tile.level]!;
      const heightLevel = materialized.height_hierarchy.levels[tile.level]!;
      expect(tile).toMatchObject({
        cache_key: tile.tile_id,
        field_set_id: reliefLevel.field.field_set_id,
        mosaic_id: materialized.height_hierarchy.mosaic_id,
        height_level_id: heightLevel.level_id,
        relief_field_id: reliefLevel.relief.relief_field_id,
        relief_operator_revision: reliefLevel.relief.implementation_revision,
        validity_support_id: heightLevel.validity_id,
        border_digest_id: reliefLevel.border_digest_id,
      });
      expect(tile.cache_key).toContain(tile.mosaic_id);
      expect(tile.cache_key).toContain(tile.height_level_id);
      expect(tile.cache_key).toContain(tile.relief_operator_revision);
      expect(tile.cache_key).toContain(tile.validity_support_id);
      if (tile.parent_id)
        expect(
          pyramid.tiles.find(({ tile_id }) => tile_id === tile.parent_id)
            ?.child_ids,
        ).toContain(tile.tile_id);
    }
    const errors = materialized.relief_levels.map(
      ({ level }) =>
        pyramid.tiles.find((tile) => tile.level === level)!.geometric_error,
    );
    expect(errors.at(-1)).toBe(0);
    for (let level = 1; level < errors.length; level += 1)
      expect(errors[level]).toBeLessThanOrEqual(errors[level - 1]!);

    const overview = selectTerrainTilesForView(pyramid, terrainTileView(0.001));
    const close = selectTerrainTilesForView(pyramid, terrainTileView(4));
    expect(overview.level).toBeLessThanOrEqual(close.level);
    expect(new Set(close.tiles.map(({ level }) => level))).toEqual(
      new Set([close.level]),
    );
    expect(terrainTileSeamMismatchCount(close.tiles)).toBe(0);
  });

  it("falls back to the finest visible hierarchy level inside residency budgets", () => {
    const materialized = compileMaterializedLandscapePyramid(admittedField());
    const pyramid = projectMaterializedLandscapeTilePyramid(materialized);
    const view = terrainTileView(4);
    const unrestricted = selectTerrainTilesForView(pyramid, view);
    const root = pyramid.tiles.filter(({ level }) => level === 0);
    const maximumCpuBytes = root.reduce(
      (total, tile) => total + tile.cpu_bytes,
      0,
    );
    const maximumGpuBytes = root.reduce(
      (total, tile) => total + tile.gpu_bytes,
      0,
    );
    const bounded = selectTerrainTilesForView(pyramid, view, 1e-9, {
      maximum_cpu_bytes: maximumCpuBytes,
      maximum_gpu_bytes: maximumGpuBytes,
    });

    expect(bounded.level).toBe(0);
    expect(bounded.level).toBeLessThan(unrestricted.level);
    expect(bounded.cpu_bytes).toBeLessThanOrEqual(maximumCpuBytes);
    expect(bounded.gpu_bytes).toBeLessThanOrEqual(maximumGpuBytes);
    expect(bounded.budget_limited).toBe(true);
  });
});

function tileAt(
  pyramid: ReturnType<typeof projectTerrainTilePyramid>,
  level: number,
  column: number,
  row: number,
) {
  const tile = pyramid.tiles.find(
    (candidate) =>
      candidate.level === level &&
      candidate.column === column &&
      candidate.row === row,
  );
  if (!tile) throw new Error("fixture tile is missing");
  return tile;
}
