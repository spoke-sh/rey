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
      columns: 201,
      rows: 201,
      longitude_step_degrees: 0.0044,
      latitude_step_degrees: 0.00375,
      nominal_longitude_spacing_meters: 461.36,
      nominal_latitude_spacing_meters: 416.75,
    });
    expect(cells).toHaveLength(40_401);
    expect(at(0, 0).geometry.coordinates).toEqual([-160, -19.25]);
    expect(at(200, 200).geometry.coordinates).toEqual([-159.12, -20]);
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
    expect(at(200, 200).properties.terrain_grid_validity).toBe("no_data");
    expect(at(40, 175).properties.terrain_grid_validity).toBe("no_data");
    expect(at(40, 175).geometry.coordinates).toHaveLength(2);
    expect(terrain.terrain_derivation.summary).toMatchObject({
      valid_vertices: expect.any(Number),
      no_data_vertices: expect.any(Number),
      outside_footprint_vertices: expect.any(Number),
      unexplored_vertices: expect.any(Number),
    });
    expect(terrain.terrain_derivation.summary.valid_vertices).toBeGreaterThan(
      28_000,
    );
    expect(
      terrain.terrain_derivation.summary.outside_footprint_vertices,
    ).toBeGreaterThan(10_500);
    expect(
      terrain.terrain_derivation.summary.unexplored_vertices,
    ).toBeGreaterThan(600);
  });

  it("expresses distinct project landforms, relief, and bounded materials", () => {
    const anchorSummit = at(50, 40);
    const runtimeBasin = at(110, 125);
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
      schema: "rey.county-terrain-source.v4",
      dataset_id: "rey-county-semantic-terrain-v4",
      compiler_revision: "rey.agent-geography.rey-county@4",
      synthesis: {
        elevation: expect.stringContaining("domain-warped"),
        hydrology: expect.stringContaining("drainage constraints"),
        land_cover: expect.stringContaining("moisture"),
        cartography: expect.stringContaining("railway"),
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
        if (residual > 1) detailedCenters += 1;
        maximumResidual = Math.max(maximumResidual, residual);
      }
    }
    expect(supportedCenters).toBeGreaterThan(6_500);
    expect(detailedCenters).toBeGreaterThan(1_000);
    expect(maximumResidual).toBeGreaterThan(20);
  });

  it("retains a landscape-scale transport and label hierarchy", () => {
    const source = (name) =>
      JSON.parse(readFileSync(resolve(sceneDirectory, name), "utf8"));
    const hierarchy = {
      highways: source("highways.geojson").features,
      roads: source("roads.geojson").features,
      railways: source("railways.geojson").features,
      labels: source("labels.geojson").features,
    };
    expect(hierarchy.highways).toHaveLength(4);
    expect(hierarchy.roads).toHaveLength(12);
    expect(hierarchy.railways).toHaveLength(4);
    expect(hierarchy.labels).toHaveLength(16);
    expect(
      new Set(
        Object.values(hierarchy)
          .flat()
          .map(({ id }) => id),
      ).size,
    ).toBe(36);
    expect(
      hierarchy.labels.filter(
        ({ properties }) => properties.min_zoom <= 6,
      ).length,
    ).toBeGreaterThanOrEqual(14);
  });

  it("matches the checked-in native artifact byte for byte", () => {
    expect(
      readFileSync(resolve(sceneDirectory, "terrain.geojson"), "utf8"),
    ).toBe(serializeReyCountyTerrain(terrain));
  });
});
