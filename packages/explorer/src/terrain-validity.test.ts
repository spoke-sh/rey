import { describe, expect, it } from "vitest";
import { terrainFieldFixture } from "./test-fixtures";
import {
  createTerrainValidityClassification,
  summarizeTerrainFieldValidity,
  TERRAIN_VALIDITY_NO_DATA,
  TERRAIN_VALIDITY_UNSUPPORTED,
  TERRAIN_VALIDITY_VALID,
  verifyTerrainFieldValidityClassification,
} from "./terrain-validity";

describe("terrain validity classification", () => {
  it("content-identifies valid, source no-data, and unsupported vertices", () => {
    const field = terrainFieldFixture();
    field.validity.values[1] = 0;
    field.validity.values[2] = 0;
    field.validity_classification = createTerrainValidityClassification(
      Uint8Array.from({ length: field.field_cells }, (_, index) =>
        index === 1
          ? TERRAIN_VALIDITY_NO_DATA
          : index === 2
            ? TERRAIN_VALIDITY_UNSUPPORTED
            : TERRAIN_VALIDITY_VALID,
      ),
      "fixture:validity-classification@1",
    );

    const summary = summarizeTerrainFieldValidity(field);
    expect(summary).toMatchObject({
      valid_vertices: field.field_cells - 2,
      no_data_vertices: 1,
      unsupported_vertices: 1,
    });
    expect(summary.validity_id).toMatch(/^blake3:[0-9a-f]{64}$/);
    expect(summarizeTerrainFieldValidity(field)).toEqual(summary);
  });

  it("rejects classification that contradicts the geometry mask", () => {
    const field = terrainFieldFixture();
    field.validity_classification = createTerrainValidityClassification(
      new Uint8Array(field.field_cells).fill(TERRAIN_VALIDITY_VALID),
      "fixture:validity-classification@1",
    );
    field.validity.values[0] = 0;

    expect(() => verifyTerrainFieldValidityClassification(field)).toThrow(
      "contradicts geometry",
    );
  });
});
