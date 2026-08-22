import { TERRAIN_VALIDITY_NO_DATA } from "@rey/explorer";
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
    source.validity_classification!.values[center] = TERRAIN_VALIDITY_NO_DATA;
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
      expect.arrayContaining([
        "derived_drainage",
        "derived_land_cover",
        "derived_multiscale_relief",
      ]),
    );
    expect(geography.detail_authority).toContain("not observed hydrology");
    expect(geography.detail_authority).toContain(
      "support-conservative multiscale topographic tone",
    );
    expect(geography.validity.values).toEqual(refined.validity.values);
    expect(geography.validity_classification?.values).toEqual(
      refined.validity_classification?.values,
    );
    expect(geography.elevation.values).toEqual(replay.elevation.values);
    expect(geography.elevation.values).toEqual(refined.elevation.values);
    expect(geography.flow_accumulation.values).toEqual(
      replay.flow_accumulation.values,
    );
    expect(Math.max(...geography.flow_accumulation.values)).toBeCloseTo(1, 6);
    expect(geography.erosion.values.some((value) => value > 0)).toBe(true);
    const luminance = Array.from(
      { length: geography.field_cells },
      (_, index) =>
        geography.material.tint[index * 3]! * 0.2126 +
        geography.material.tint[index * 3 + 1]! * 0.7152 +
        geography.material.tint[index * 3 + 2]! * 0.0722,
    ).filter((_, index) => geography.validity.values[index] !== 0);
    expect(Math.max(...luminance) - Math.min(...luminance)).toBeGreaterThan(
      0.12,
    );
    const supportedOcclusion = [...geography.material.occlusion].filter(
      (_, index) => geography.validity.values[index] !== 0,
    );
    expect(Math.min(...supportedOcclusion)).toBeLessThan(0.8);
    expect(Math.max(...supportedOcclusion)).toBeGreaterThan(0.9);
    expect(
      Math.max(...supportedOcclusion) - Math.min(...supportedOcclusion),
    ).toBeGreaterThan(0.18);

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
    const landscapeLines = deriveRegionalTerrainPresentationLines(
      geography,
      "landscape",
    );
    const lines = deriveRegionalTerrainPresentationLines(
      geography,
      "neighborhoods",
    );

    expect(
      landscapeLines.filter(({ kind }) => kind === "derived_contour").length,
    ).toBe(9);
    expect(landscapeLines.some(({ kind }) => kind.startsWith("derived_"))).toBe(
      true,
    );
    expect(landscapeLines.some(({ kind }) => kind === "derived_stream")).toBe(
      false,
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
