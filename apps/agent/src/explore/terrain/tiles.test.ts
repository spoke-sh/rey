import { terrainTriangleIndices } from "@rey/explorer";
import { describe, expect, it } from "vitest";
import { admittedField, terrainTileView } from "./tiles.fixture";
import {
  materializeTerrainTile,
  projectTerrainTilePyramid,
  selectTerrainTilesForView,
  terrainTileSeamMismatchCount,
} from "./tiles";

describe("admitted terrain tile projection", () => {
  it("retains stable parentage and identical shared validity borders", () => {
    const source = admittedField();
    const seamIndex = 16 * source.grid.columns + 32;
    source.validity.values[seamIndex] = 0;
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

  it("can only remove support at coarse levels and refines by screen error", () => {
    const source = admittedField();
    source.validity.values[16 * source.grid.columns + 16] = 0;
    const pyramid = projectTerrainTilePyramid(source);
    const root = tileAt(pyramid, 0, 0, 0);
    expect(root.no_data_vertices).toBeGreaterThan(0);
    expect(root.geometric_error).toBeGreaterThan(0);

    const overview = selectTerrainTilesForView(pyramid, terrainTileView(0.001));
    const close = selectTerrainTilesForView(pyramid, terrainTileView(4));
    expect(overview.level).toBe(0);
    expect(close.level).toBeGreaterThan(overview.level);
    expect(close.screen_error_pixels).toBeLessThanOrEqual(
      root.geometric_error * 4,
    );
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
