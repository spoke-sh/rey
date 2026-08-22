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
      "rey.landscape.chromatic-relief@1",
    );
    expect(color).toEqual(replay);
    expect(color[3]! / color[5]!).toBeGreaterThan(color[6]! / color[8]!);
    expect(
      [...color].every((component) => component >= 0 && component <= 1),
    ).toBe(true);
    expect(linearTerrainColorToCss([0, 0.5, 1])).toBe("rgb(0 188 255)");
  });

  it("does not color unsupported terrain", () => {
    const field = terrainFieldFixture();
    field.validity.values[3] = 0;
    const relief = deriveLandscapeReliefField(field);
    const color = composeCartographicTerrainColor(field, relief);
    expect([...color.slice(9, 12)]).toEqual([0, 0, 0]);
  });
});
