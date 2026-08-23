import { deriveLandscapeReliefField } from "@rey/explorer";
import { describe, expect, it } from "vitest";
import { admittedField } from "../terrain/tiles.fixture";
import {
  rasterizeReferenceTerrain,
  REFERENCE_TERRAIN_RASTER_REVISION,
} from "./reference-terrain-raster";

describe("reference terrain raster", () => {
  it("paints only support owned by a valid source triangle", () => {
    const field = admittedField(3, 3);
    field.validity.values.fill(0);
    field.validity.values[0] = 1;
    field.validity.values[3] = 1;
    field.validity.values[4] = 1;
    const pixels = rasterizeReferenceTerrain(
      [{ field, relief: deriveLandscapeReliefField(field) }],
      6,
      6,
      { width: 1_500, height: 1_000 },
    );
    const alpha = Array.from(
      { length: 36 },
      (_, index) => pixels[index * 4 + 3],
    );

    expect(REFERENCE_TERRAIN_RASTER_REVISION).toBe(
      "rey.reference-regional-terrain@5",
    );
    expect(alpha.some((value) => value === 255)).toBe(true);
    expect(alpha.some((value) => value === 0)).toBe(true);
    expect(alpha.filter((value) => value === 255).length).toBeLessThan(18);
  });
});
