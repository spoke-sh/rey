import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { buildReyCountyTerrainSource } from "../../../scenes/rey-county/generate-terrain.mjs";
import {
  buildReyEasternUplandsTerrainSource,
  serializeReyEasternUplandsTerrain,
} from "../../../scenes/rey-eastern-uplands/generate-terrain.mjs";

const repositoryRoot = resolve(
  fileURLToPath(new URL("../../..", import.meta.url)),
);
const countyDirectory = resolve(repositoryRoot, "scenes/rey-county");
const uplandsDirectory = resolve(repositoryRoot, "scenes/rey-eastern-uplands");

describe("Rey regional source seam", () => {
  const county = buildReyCountyTerrainSource(countyDirectory);
  const uplands = buildReyEasternUplandsTerrainSource(
    uplandsDirectory,
    resolve(countyDirectory, "terrain.geojson"),
  );
  const countyColumns = county.document.terrain_derivation.grid.columns;
  const uplandsColumns = uplands.document.terrain_derivation.grid.columns;

  it("retains an exact validity, elevation, and material boundary", () => {
    for (let row = 0; row < 168; row += 1) {
      const countyCell = county.cells[(225 + row) * countyColumns + 704];
      const uplandsCell = uplands.cells[row * uplandsColumns];
      expect(countyCell.valid).toBe(true);
      expect(uplandsCell).toMatchObject({
        longitude: countyCell.longitude,
        latitude: countyCell.latitude,
        valid: countyCell.valid,
        sample: {
          elevation: countyCell.sample.elevation,
          material: countyCell.sample.material,
        },
      });
    }
    expect(uplands.document.terrain_derivation.seam).toMatchObject({
      schema: "rey.authored-regional-seam.v4",
      source_dataset_id: "rey-county-semantic-terrain-v16",
      source_interior_context_columns: 1,
      low_pass_trend_radius_rows: 8,
      transition_columns: 24,
      compared_vertices: 168,
      validity_conflicts: 0,
      elevation_conflicts: 0,
      material_conflicts: 0,
    });
  });

  it("continues the exact County edge slope into the first interior column", () => {
    for (let row = 0; row < 168; row += 1) {
      const countyInterior = county.cells[(225 + row) * countyColumns + 703];
      const countySeam = county.cells[(225 + row) * countyColumns + 704];
      const uplandsSeam = uplands.cells[row * uplandsColumns];
      const uplandsInterior = uplands.cells[row * uplandsColumns + 1];
      expect(countyInterior.valid).toBe(true);
      expect(uplandsInterior.valid).toBe(true);
      expect(
        countySeam.sample.elevation - countyInterior.sample.elevation,
      ).toBeCloseTo(
        uplandsInterior.sample.elevation - uplandsSeam.sample.elevation,
        10,
      );
      expect(uplandsInterior.sample.material).toBe(uplandsSeam.sample.material);
    }
  });

  it("damps row-scale edge noise before independent relief takes over", () => {
    const meanRowSecondDifference = (column) => {
      const differences = [];
      for (let row = 1; row < 167; row += 1) {
        const north = uplands.cells[(row - 1) * uplandsColumns + column];
        const center = uplands.cells[row * uplandsColumns + column];
        const south = uplands.cells[(row + 1) * uplandsColumns + column];
        if (!north.valid || !center.valid || !south.valid) continue;
        differences.push(
          Math.abs(
            north.sample.elevation -
              2 * center.sample.elevation +
              south.sample.elevation,
          ),
        );
      }
      return (
        differences.reduce((total, value) => total + value, 0) /
        differences.length
      );
    };

    expect(meanRowSecondDifference(8)).toBeLessThan(meanRowSecondDifference(1));
    expect(meanRowSecondDifference(24)).toBeLessThan(0.25);
  });

  it("authors continuous source-scale ridges and valleys beyond the seam corridor", () => {
    expect(
      uplands.document.terrain_derivation.synthesis.independent_relief,
    ).toMatchObject({
      schema: "rey.authored-domain-warped-relief.v1",
      transition_columns: 36,
      domain_warp_octaves: 3,
      ridge_octaves: 5,
      valley_octaves: 4,
      source_scale_octaves: 4,
    });

    const radius = 4;
    const localRelief = [];
    for (let row = radius; row < 168 - radius; row += 4) {
      for (let column = 60; column < uplandsColumns - radius; column += 4) {
        let minimum = Number.POSITIVE_INFINITY;
        let maximum = Number.NEGATIVE_INFINITY;
        let supported = true;
        for (
          let sampleRow = row - radius;
          sampleRow <= row + radius;
          sampleRow += 1
        ) {
          for (
            let sampleColumn = column - radius;
            sampleColumn <= column + radius;
            sampleColumn += 1
          ) {
            const sample =
              uplands.cells[sampleRow * uplandsColumns + sampleColumn];
            if (!sample.valid) {
              supported = false;
              break;
            }
            minimum = Math.min(minimum, sample.sample.elevation);
            maximum = Math.max(maximum, sample.sample.elevation);
          }
          if (!supported) break;
        }
        if (supported) localRelief.push(maximum - minimum);
      }
    }
    localRelief.sort((left, right) => left - right);
    const percentile = (value) =>
      localRelief[Math.floor((localRelief.length - 1) * value)];
    expect(localRelief.length).toBeGreaterThan(900);
    expect(percentile(0.5)).toBeGreaterThan(50);
    expect(percentile(0.9)).toBeGreaterThan(90);
    expect(percentile(0.9)).toBeLessThan(130);

    const gradientDirections = Array.from({ length: 12 }, () => 0);
    for (let row = 1; row < 167; row += 1) {
      for (let column = 60; column < uplandsColumns - 1; column += 1) {
        const west = uplands.cells[row * uplandsColumns + column - 1];
        const east = uplands.cells[row * uplandsColumns + column + 1];
        const north = uplands.cells[(row - 1) * uplandsColumns + column];
        const south = uplands.cells[(row + 1) * uplandsColumns + column];
        if (![west, east, north, south].every((sample) => sample.valid))
          continue;
        const eastward = east.sample.elevation - west.sample.elevation;
        const southward = south.sample.elevation - north.sample.elevation;
        if (Math.hypot(eastward, southward) < 1) continue;
        const angle =
          (Math.atan2(southward, eastward) + Math.PI * 2) % (Math.PI * 2);
        const bucket = Math.min(
          gradientDirections.length - 1,
          Math.floor((angle / (Math.PI * 2)) * gradientDirections.length),
        );
        gradientDirections[bucket] += 1;
      }
    }
    expect(Math.min(...gradientDirections)).toBeGreaterThan(800);
    expect(
      Math.max(...gradientDirections) / Math.min(...gradientDirections),
    ).toBeLessThan(2.5);
  });

  it("keeps independently authored interior support bounded by its polygon", () => {
    const summary = uplands.document.terrain_derivation.summary;
    expect(summary.valid_vertices).toBeGreaterThan(20_000);
    expect(summary.no_data_vertices).toBeGreaterThan(1_000);
    expect(summary.maximum_elevation_meters).toBeGreaterThan(
      summary.minimum_elevation_meters + 400,
    );
    expect(uplands.cells.at(-1).valid).toBe(false);
  });

  it("matches the checked-in native artifact byte for byte", () => {
    expect(
      readFileSync(resolve(uplandsDirectory, "terrain.geojson"), "utf8"),
    ).toBe(serializeReyEasternUplandsTerrain(uplands.document));
  });
});
