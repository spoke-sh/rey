import {
  TERRAIN_VALIDITY_NO_DATA,
  TERRAIN_VALIDITY_UNSUPPORTED,
  verifyLandscapePyramidEnvelope,
} from "@rey/explorer";
import { describe, expect, it } from "vitest";
import { admittedField } from "./tiles.fixture";
import { compileMaterializedLandscapePyramid } from "./relief-pyramid";

describe("materialized landscape relief pyramid", () => {
  it("derives every level from haloed tiles with exact borders and partitions", () => {
    const field = admittedField(126, 94);
    const noData = 37 * field.grid.columns + 51;
    const unsupported = 19 * field.grid.columns + 88;
    field.validity.values[noData] = 0;
    field.validity_classification!.values[noData] = TERRAIN_VALIDITY_NO_DATA;
    field.validity.values[unsupported] = 0;
    field.validity_classification!.values[unsupported] =
      TERRAIN_VALIDITY_UNSUPPORTED;

    const pyramid = compileMaterializedLandscapePyramid(field, 16);
    const replay = compileMaterializedLandscapePyramid(field, 16);
    const fine = pyramid.relief_levels.at(-1)!;

    expect(pyramid).toMatchObject({
      schema: "rey.materialized-landscape-pyramid.v1",
      border_mismatches: 0,
      partition_mismatches: 0,
      envelope: {
        height_pyramid: { complete: true },
        relief_pyramid: { complete: true },
      },
    });
    expect(pyramid.hierarchy_id).toBe(replay.hierarchy_id);
    expect(pyramid.height_hierarchy.levels[0]).toMatchObject({
      columns: 2,
      rows: 2,
    });
    expect(pyramid.relief_levels).toHaveLength(
      pyramid.height_hierarchy.levels.length,
    );
    expect(fine.tiles.length).toBeGreaterThan(1);
    expect(fine.maximum_gutter_radius_cells).toBeGreaterThan(0);
    expect(fine.border_digest_id).toMatch(/^blake3:[0-9a-f]{64}$/);
    expect(
      fine.tiles.every(
        ({ interior, source_window, gutter_radius_cells }) =>
          gutter_radius_cells === fine.maximum_gutter_radius_cells &&
          source_window.column_start <= interior.column_start &&
          source_window.column_end >= interior.column_end &&
          source_window.row_start <= interior.row_start &&
          source_window.row_end >= interior.row_end,
      ),
    ).toBe(true);
    expect(() =>
      verifyLandscapePyramidEnvelope(pyramid.envelope, field, fine.relief),
    ).not.toThrow();
  });
});
