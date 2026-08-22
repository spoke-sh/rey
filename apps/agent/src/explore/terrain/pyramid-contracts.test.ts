import {
  deriveLandscapeReliefField,
  landscapePyramidContentId,
  TERRAIN_VALIDITY_NO_DATA,
  verifyLandscapePyramidEnvelope,
} from "@rey/explorer";
import { describe, expect, it } from "vitest";
import { compileCurrentLandscapePyramidEnvelope } from "./pyramid-contracts";
import { admittedField } from "./tiles.fixture";

describe("current landscape pyramid envelope", () => {
  it("binds exact complete-field height, validity, and relief content", () => {
    const field = admittedField();
    const noDataIndex = 16 * field.grid.columns + 32;
    field.validity.values[noDataIndex] = 0;
    field.validity_classification!.values[noDataIndex] =
      TERRAIN_VALIDITY_NO_DATA;
    const relief = deriveLandscapeReliefField(field);
    const envelope = compileCurrentLandscapePyramidEnvelope(field, relief);

    expect(envelope.height_pyramid).toMatchObject({
      complete: false,
      mosaic_id: field.landscape_reference?.reference_id,
    });
    expect(envelope.height_pyramid.levels).toHaveLength(1);
    expect(envelope.height_pyramid.levels[0]).toMatchObject({
      height_id: landscapePyramidContentId("height", [field.elevation.values]),
      columns: field.grid.columns,
      rows: field.grid.rows,
      validity: {
        valid_vertices: field.field_cells - 1,
        no_data_vertices: 1,
        unsupported_vertices: 0,
      },
    });
    expect(envelope.relief_pyramid.complete).toBe(false);
    expect(
      envelope.relief_pyramid.levels[0]!.operator_support.every(
        ({ gutter_radius_cells, supported }) =>
          gutter_radius_cells === 0 && !supported,
      ),
    ).toBe(true);
    expect(envelope.relief_pyramid.omissions).toContain(
      "relief source gutters and adjacent-tile border digests are not retained",
    );
    expect(() =>
      verifyLandscapePyramidEnvelope(envelope, field, relief),
    ).not.toThrow();
  });

  it("rejects field content drift after envelope compilation", () => {
    const field = admittedField();
    const relief = deriveLandscapeReliefField(field);
    const envelope = compileCurrentLandscapePyramidEnvelope(field, relief);
    field.elevation.values[0] = Math.fround(field.elevation.values[0]! + 0.1);

    expect(() =>
      verifyLandscapePyramidEnvelope(envelope, field, relief),
    ).toThrow("diverges from its field");
  });
});
