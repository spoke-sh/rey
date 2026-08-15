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
      columns: 41,
      rows: 41,
      longitude_step_degrees: 0.022,
      latitude_step_degrees: 0.01875,
    });
    expect(cells).toHaveLength(1_681);
    expect(at(0, 0).geometry.coordinates).toEqual([-160, -19.25]);
    expect(at(40, 40).geometry.coordinates).toEqual([-159.12, -20]);
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
    expect(at(40, 40).properties.terrain_grid_validity).toBe("no_data");
    expect(at(8, 35).properties.terrain_grid_validity).toBe("no_data");
    expect(at(8, 35).geometry.coordinates).toHaveLength(2);
    expect(terrain.terrain_derivation.summary).toMatchObject({
      valid_vertices: expect.any(Number),
      no_data_vertices: expect.any(Number),
      outside_footprint_vertices: expect.any(Number),
      unexplored_vertices: expect.any(Number),
    });
    expect(terrain.terrain_derivation.summary.valid_vertices).toBeGreaterThan(
      1_000,
    );
    expect(
      terrain.terrain_derivation.summary.outside_footprint_vertices,
    ).toBeGreaterThan(400);
    expect(
      terrain.terrain_derivation.summary.unexplored_vertices,
    ).toBeGreaterThan(20);
  });

  it("expresses distinct project landforms, relief, and bounded materials", () => {
    const anchorSummit = at(10, 8);
    const runtimeBasin = at(22, 25);
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

  it("matches the checked-in native artifact byte for byte", () => {
    expect(
      readFileSync(resolve(sceneDirectory, "terrain.geojson"), "utf8"),
    ).toBe(serializeReyCountyTerrain(terrain));
  });
});
