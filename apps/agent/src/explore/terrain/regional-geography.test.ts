import { describe, expect, it } from "vitest";
import { refineRegionalTerrainField } from "./refinement";
import {
  REGIONAL_TERRAIN_GEOGRAPHY_REVISION,
  deriveRegionalTerrainGeography,
  deriveRegionalTerrainPresentationLines,
} from "./regional-geography";
import { admittedField } from "./tiles.fixture";

describe("regional terrain geography", () => {
  it("derives a deterministic drainage hierarchy without expanding validity or moving source vertices", () => {
    const source = admittedField();
    const center =
      Math.floor(source.grid.rows / 2) * source.grid.columns +
      Math.floor(source.grid.columns / 2);
    source.validity.values[center] = 0;
    const admitted = {
      ...source,
      source_summary: {
        columns: source.grid.columns,
        rows: source.grid.rows,
        valid_vertices: source.field_cells - 1,
        no_data_vertices: 1,
        elevation_minimum: 0,
        elevation_maximum: 1_000,
      },
    };
    const refined = refineRegionalTerrainField(admitted, 2);
    const geography = deriveRegionalTerrainGeography(refined);
    const replay = deriveRegionalTerrainGeography(refined);

    expect(geography.field_set_id).toContain(
      REGIONAL_TERRAIN_GEOGRAPHY_REVISION,
    );
    expect(geography.active_band_ids).toEqual(
      expect.arrayContaining(["derived_drainage", "derived_land_cover"]),
    );
    expect(geography.detail_authority).toContain("not observed hydrology");
    expect(geography.validity.values).toEqual(refined.validity.values);
    expect(geography.elevation.values).toEqual(replay.elevation.values);
    expect(geography.flow_accumulation.values).toEqual(
      replay.flow_accumulation.values,
    );
    expect(Math.max(...geography.flow_accumulation.values)).toBeCloseTo(1, 6);
    expect(geography.erosion.values.some((value) => value > 0)).toBe(true);

    for (let row = 0; row < source.grid.rows; row += 1) {
      for (let column = 0; column < source.grid.columns; column += 1) {
        const refinedIndex = row * 2 * refined.grid.columns + column * 2;
        if (refined.validity.values[refinedIndex] === 0) continue;
        expect(geography.elevation.values[refinedIndex]).toBe(
          refined.elevation.values[refinedIndex],
        );
      }
    }
    for (let index = 0; index < geography.field_cells; index += 1) {
      if (geography.validity.values[index] !== 0) continue;
      expect(geography.flow_accumulation.values[index]).toBe(0);
      expect(geography.material.occlusion[index]).toBe(0);
    }
  });

  it("emits metric contours and a terrain-following synthetic water hierarchy", () => {
    const source = admittedField();
    const refined = refineRegionalTerrainField(
      {
        ...source,
        source_summary: {
          columns: source.grid.columns,
          rows: source.grid.rows,
          valid_vertices: source.field_cells,
          no_data_vertices: 0,
          elevation_minimum: 0,
          elevation_maximum: 1_000,
        },
      },
      2,
    );
    const geography = deriveRegionalTerrainGeography(refined);
    const lines = deriveRegionalTerrainPresentationLines(
      geography,
      "landscape",
    );

    expect(lines.filter(({ kind }) => kind === "derived_contour").length).toBe(
      9,
    );
    expect(lines.some(({ kind }) => kind === "derived_stream")).toBe(true);
    expect(lines.some(({ kind }) => kind === "derived_river")).toBe(true);
    expect(lines.every(({ positions }) => positions.length % 6 === 0)).toBe(
      true,
    );
    expect(
      lines
        .filter(({ kind }) => kind.startsWith("derived_"))
        .every(({ authority }) => authority.includes("derived")),
    ).toBe(true);
  });
});
