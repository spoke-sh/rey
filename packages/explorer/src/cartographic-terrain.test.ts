import { describe, expect, it } from "vitest";
import { deriveLandscapeReliefField } from "./landscape-relief";
import { terrainFieldFixture } from "./test-fixtures";
import {
  composeCartographicTerrainColor,
  LANDSCAPE_CARTOGRAPHIC_COLOR_REVISION,
  linearTerrainColorToCss,
} from "./cartographic-terrain";

describe("cartographic terrain color", () => {
  it("composes deterministic warm direct and cool ambient linear color", () => {
    const field = terrainFieldFixture();
    const relief = deriveLandscapeReliefField(field);
    relief.mdow[1] = 1.16;
    relief.sky_view_factor[1] = 1;
    relief.mdow[2] = 0.64;
    relief.sky_view_factor[2] = 0.42;
    const color = composeCartographicTerrainColor(field, relief);
    const replay = composeCartographicTerrainColor(field, relief);

    expect(LANDSCAPE_CARTOGRAPHIC_COLOR_REVISION).toBe(
      "rey.landscape.chromatic-relief@2",
    );
    expect(color).toEqual(replay);
    expect(color[3]! / color[5]!).toBeGreaterThan(color[6]! / color[8]!);
    expect(
      [...color].every((component) => component >= 0 && component <= 1),
    ).toBe(true);
    expect(linearTerrainColorToCss([0, 0.5, 1])).toBe("rgb(0 188 255)");
  });

  it("retains SVF valley depth and local ridge separation in luminance", () => {
    const field = terrainFieldFixture();
    const relief = deriveLandscapeReliefField(field);
    for (const index of [1, 2]) {
      const offset = index * 3;
      field.material.tint[offset] = 0.46;
      field.material.tint[offset + 1] = 0.5;
      field.material.tint[offset + 2] = 0.38;
      field.material.occlusion[index] = 0.92;
      relief.mdow[index] = 0.88;
      relief.hillshade[index] = 0.82;
      relief.salience[index] = 0.24;
    }
    relief.sky_view_factor[1] = 0.42;
    relief.openness[1] = -0.55;
    relief.local_contrast[1] = 0.28;
    relief.sky_view_factor[2] = 0.96;
    relief.openness[2] = 0.44;
    relief.local_contrast[2] = 0.74;

    const color = composeCartographicTerrainColor(field, relief);
    const luminance = (index: number) =>
      color[index * 3]! * 0.2126 +
      color[index * 3 + 1]! * 0.7152 +
      color[index * 3 + 2]! * 0.0722;

    expect(luminance(2)).toBeGreaterThan(luminance(1) * 1.45);
    expect(luminance(1)).toBeGreaterThan(0.08);
    expect(luminance(2)).toBeLessThan(0.55);
  });

  it("does not color unsupported terrain", () => {
    const field = terrainFieldFixture();
    field.validity.values[3] = 0;
    const relief = deriveLandscapeReliefField(field);
    const color = composeCartographicTerrainColor(field, relief);
    expect([...color.slice(9, 12)]).toEqual([0, 0, 0]);
  });
});
