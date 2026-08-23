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
      schema: "rey.authored-regional-seam.v3",
      source_dataset_id: "rey-county-semantic-terrain-v15",
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

  it("keeps independently authored interior support bounded by its polygon", () => {
    const summary = uplands.document.terrain_derivation.summary;
    expect(summary.valid_vertices).toBeGreaterThan(20_000);
    expect(summary.no_data_vertices).toBeGreaterThan(1_000);
    expect(summary.maximum_elevation_meters).toBeGreaterThan(
      summary.minimum_elevation_meters + 300,
    );
    expect(uplands.cells.at(-1).valid).toBe(false);
  });

  it("matches the checked-in native artifact byte for byte", () => {
    expect(
      readFileSync(resolve(uplandsDirectory, "terrain.geojson"), "utf8"),
    ).toBe(serializeReyEasternUplandsTerrain(uplands.document));
  });
});
