import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  buildReyCountyTerrain,
  serializeReyCountyTerrain,
} from "../../../scenes/rey-county/generate-terrain.mjs";

const repositoryRoot = resolve(
  fileURLToPath(new URL("../../..", import.meta.url)),
);
const sceneDirectory = resolve(repositoryRoot, "scenes/rey-county");

describe("Rey County terrain source", () => {
  const terrain = buildReyCountyTerrain(sceneDirectory);
  const cells = terrain.features;
  const { columns, rows } = terrain.terrain_derivation.grid;
  const at = (column, row) => cells[row * columns + column];

  it("retains one exact row-major grid over the County bounds", () => {
    expect(terrain.terrain_derivation.grid).toMatchObject({
      columns: 81,
      rows: 81,
      longitude_step_degrees: 0.011,
      latitude_step_degrees: 0.009375,
      nominal_longitude_spacing_meters: 1153.39,
      nominal_latitude_spacing_meters: 1041.86,
    });
    expect(cells).toHaveLength(6_561);
    expect(at(0, 0).geometry.coordinates).toEqual([-160, -19.25]);
    expect(at(80, 80).geometry.coordinates).toEqual([-159.12, -20]);
    expect(
      cells.map(({ properties }) => [
        properties.terrain_grid_column,
        properties.terrain_grid_row,
      ]),
    ).toEqual(
      Array.from({ length: rows }, (_, row) =>
        Array.from({ length: columns }, (_, column) => [column, row]),
      ).flat(),
    );
  });

  it("keeps the footprint exterior and Unexplored Scrub as explicit no-data", () => {
    expect(at(0, 0).properties.terrain_grid_validity).toBe("no_data");
    expect(at(80, 80).properties.terrain_grid_validity).toBe("no_data");
    expect(at(16, 70).properties.terrain_grid_validity).toBe("no_data");
    expect(at(16, 70).geometry.coordinates).toHaveLength(2);
    expect(terrain.terrain_derivation.summary).toMatchObject({
      valid_vertices: expect.any(Number),
      no_data_vertices: expect.any(Number),
      outside_footprint_vertices: expect.any(Number),
      unexplored_vertices: expect.any(Number),
    });
    expect(terrain.terrain_derivation.summary.valid_vertices).toBeGreaterThan(
      4_500,
    );
    expect(
      terrain.terrain_derivation.summary.outside_footprint_vertices,
    ).toBeGreaterThan(1_800);
    expect(
      terrain.terrain_derivation.summary.unexplored_vertices,
    ).toBeGreaterThan(100);
  });

  it("expresses distinct project landforms, relief, and bounded materials", () => {
    const anchorSummit = at(20, 16);
    const runtimeBasin = at(44, 50);
    expect(anchorSummit.properties.landform).toBe("anchor-summit");
    expect(anchorSummit.geometry.coordinates[2]).toBeGreaterThan(
      runtimeBasin.geometry.coordinates[2] + 350,
    );
    expect(
      terrain.terrain_derivation.summary.maximum_elevation_meters -
        terrain.terrain_derivation.summary.minimum_elevation_meters,
    ).toBeGreaterThan(700);
    expect(
      Object.keys(terrain.terrain_derivation.summary.materials).sort(),
    ).toEqual(["granite", "rock", "sand", "soil", "vegetation"]);
  });

  it("binds explicit multi-scale synthesis without claiming package seams", () => {
    expect(terrain.terrain_derivation).toMatchObject({
      schema: "rey.county-terrain-source.v2",
      dataset_id: "rey-county-semantic-terrain-v2",
      compiler_revision: "rey.agent-geography.rey-county@2",
      synthesis: {
        elevation: expect.stringContaining("domain-warped"),
        hydrology: expect.stringContaining("authored river and stream"),
        land_cover: expect.stringContaining("moisture"),
        stitching: {
          strategy: "single bounded County authoring domain",
          seam_count: 0,
          conflict_count: 0,
          omissions: [expect.stringContaining("not implemented")],
        },
      },
    });

    let supportedCenters = 0;
    let detailedCenters = 0;
    let maximumResidual = 0;
    for (let row = 1; row < rows - 1; row += 2) {
      for (let column = 1; column < columns - 1; column += 2) {
        const neighborhood = [
          at(column, row),
          at(column - 1, row),
          at(column + 1, row),
          at(column, row - 1),
          at(column, row + 1),
        ];
        if (
          neighborhood.some(
            ({ properties }) => properties.terrain_grid_validity !== "valid",
          )
        )
          continue;
        supportedCenters += 1;
        const predicted =
          neighborhood
            .slice(1)
            .reduce(
              (sum, feature) => sum + feature.geometry.coordinates[2],
              0,
            ) / 4;
        const residual = Math.abs(
          neighborhood[0].geometry.coordinates[2] - predicted,
        );
        if (residual > 2) detailedCenters += 1;
        maximumResidual = Math.max(maximumResidual, residual);
      }
    }
    expect(supportedCenters).toBeGreaterThan(1_000);
    expect(detailedCenters).toBeGreaterThan(250);
    expect(maximumResidual).toBeGreaterThan(25);
  });

  it("matches the checked-in native artifact byte for byte", () => {
    expect(
      readFileSync(resolve(sceneDirectory, "terrain.geojson"), "utf8"),
    ).toBe(serializeReyCountyTerrain(terrain));
  });
});
