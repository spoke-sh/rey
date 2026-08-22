import { describe, expect, it } from "vitest";
import { terrainFieldFixture } from "./test-fixtures";
import {
  compileLandscapePatchSet,
  deriveLandscapeReliefField,
  landscapeTerrainFabricSamples,
} from "./landscape-relief";

describe("landscape relief engine", () => {
  it("binds ordered intersecting patches while leaving unsupported gaps open", () => {
    const first = terrainFieldFixture();
    const second = terrainFieldFixture();
    second.field_set_id = "terrain:second";
    second.grid.bounds = { x: 900, y: 500, width: 800, height: 600 };
    const patches = compileLandscapePatchSet([first, second]);
    expect(patches.patch_ids).toEqual(["terrain:fixture", "terrain:second"]);
    expect(patches.overlap_pairs).toEqual([
      ["terrain:fixture", "terrain:second"],
    ]);
    expect(patches.bounds).toEqual({
      x: 100,
      y: 80,
      width: 1600,
      height: 1020,
    });
    expect(patches.overlap_policy).toBe(
      "later_patch_wins_with_deterministic_depth_bias",
    );
    expect(patches.gap_policy).toBe("unsupported_remains_transparent");
  });

  it("derives deterministic multiscale illumination and relief ordering", () => {
    const field = terrainFieldFixture();
    for (let row = 0; row < field.grid.rows; row += 1) {
      for (let column = 0; column < field.grid.columns; column += 1) {
        const index = row * field.grid.columns + column;
        const offset = index * 3;
        const gradientX = (column - 2) * 0.18;
        const gradientY = (row - 1.5) * 0.12;
        const length = Math.hypot(gradientX, gradientY, 1);
        field.normal.values[offset] = -gradientX / length;
        field.normal.values[offset + 1] = -gradientY / length;
        field.normal.values[offset + 2] = 1 / length;
        field.elevation.values[index] = Math.fround(
          0.18 + column * 0.11 + row * 0.05,
        );
      }
    }

    const first = deriveLandscapeReliefField(field);
    const replay = deriveLandscapeReliefField(field);
    expect(first.hillshade).toEqual(replay.hillshade);
    expect(first.salience).toEqual(replay.salience);
    expect(new Set(first.hillshade).size).toBeGreaterThan(2);
    expect(Math.max(...first.salience)).toBeGreaterThan(0.1);

    const samples = landscapeTerrainFabricSamples(field, 120);
    expect(samples.length).toBeGreaterThan(20);
    expect(samples.map(({ reveal_priority }) => reveal_priority)).toEqual(
      [...samples]
        .map(({ reveal_priority }) => reveal_priority)
        .sort((left, right) => right - left),
    );
    expect(samples.some(({ tangent_v }) => Math.abs(tangent_v) > 0.01)).toBe(
      true,
    );
  });

  it("does not derive fabric samples or multiscale spill inside no-data", () => {
    const field = terrainFieldFixture();
    const hole = 7;
    field.validity.values[hole] = 0;
    const relief = deriveLandscapeReliefField(field);
    expect(relief.hillshade[hole]).toBe(0);
    expect(relief.salience[hole]).toBe(0);
    for (const sample of landscapeTerrainFabricSamples(field, 500)) {
      const column = Math.round(sample.u * (field.grid.columns - 1));
      const row = Math.round(sample.v * (field.grid.rows - 1));
      expect(field.validity.values[row * field.grid.columns + column]).toBe(1);
    }
  });
});
