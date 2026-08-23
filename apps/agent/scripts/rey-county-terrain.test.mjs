import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  buildReyCountyTerrainSource,
  serializeReyCountyTerrain,
} from "../../../scenes/rey-county/generate-terrain.mjs";

const repositoryRoot = resolve(
  fileURLToPath(new URL("../../..", import.meta.url)),
);
const sceneDirectory = resolve(repositoryRoot, "scenes/rey-county");

describe("Rey County terrain source", () => {
  const source = buildReyCountyTerrainSource(sceneDirectory);
  const terrain = source.document;
  const cells = source.cells;
  const { columns, rows } = terrain.terrain_derivation.grid;
  const at = (column, row) => cells[row * columns + column];

  it("retains one exact row-major grid over the County bounds", () => {
    expect(terrain.terrain_derivation.grid).toMatchObject({
      columns: 705,
      rows: 626,
      longitude_step_degrees: 0.00125,
      latitude_step_degrees: 0.0012,
      nominal_longitude_spacing_meters: 131.07,
      nominal_latitude_spacing_meters: 133.36,
    });
    expect(cells).toHaveLength(441_330);
    expect([at(0, 0).longitude, at(0, 0).latitude]).toEqual([-160, -19.25]);
    expect([at(704, 625).longitude, at(704, 625).latitude]).toEqual([
      -159.12, -20,
    ]);
    expect(cells.map(({ column, row }) => [column, row])).toEqual(
      Array.from({ length: rows }, (_, row) =>
        Array.from({ length: columns }, (_, column) => [column, row]),
      ).flat(),
    );
  });

  it("keeps the footprint exterior and Unexplored Scrub as explicit no-data", () => {
    expect(at(0, 0).valid).toBe(false);
    expect(at(704, 625).valid).toBe(false);
    expect(at(141, 548).valid).toBe(false);
    expect(at(141, 548).sample).toBeNull();
    expect(terrain.terrain_derivation.summary).toMatchObject({
      valid_vertices: expect.any(Number),
      no_data_vertices: expect.any(Number),
      outside_footprint_vertices: expect.any(Number),
      unexplored_vertices: expect.any(Number),
    });
    expect(terrain.terrain_derivation.summary.valid_vertices).toBeGreaterThan(
      316_000,
    );
    expect(
      terrain.terrain_derivation.summary.outside_footprint_vertices,
    ).toBeGreaterThan(116_000);
    expect(
      terrain.terrain_derivation.summary.unexplored_vertices,
    ).toBeGreaterThan(9_000);
  });

  it("expresses distinct project landforms, relief, and bounded materials", () => {
    const anchorSummit = at(176, 125);
    const runtimeBasin = at(387, 391);
    expect(anchorSummit.sample.landform).toBe("anchor-summit");
    expect(anchorSummit.sample.elevation).toBeGreaterThan(
      runtimeBasin.sample.elevation + 350,
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
      schema: "rey.county-terrain-source.v8",
      dataset_id: "rey-county-semantic-terrain-v8",
      compiler_revision: "rey.agent-geography.rey-county@8",
      synthesis: {
        elevation: expect.stringContaining("orographic backbones"),
        hydrology: expect.stringContaining("river and wetland areas"),
        land_cover: expect.stringContaining("moisture"),
        cartography: expect.stringContaining("railway"),
        stitching: {
          strategy: "single bounded County authoring domain",
          seam_count: 0,
          conflict_count: 0,
          omissions: [expect.stringContaining("not implemented")],
        },
      },
      drainage: {
        schema: "rey.county-source-drainage.v2",
        authority: expect.stringContaining("not observed hydrology"),
        depression_handling: expect.stringContaining("stream-power"),
        maximum_accumulation_vertices: expect.any(Number),
        derived_channel_vertices: expect.any(Number),
        maximum_incision_meters: expect.any(Number),
        maximum_stream_power: expect.any(Number),
        maximum_valley_half_width_cells: 6,
      },
    });
    expect(
      terrain.terrain_derivation.drainage.maximum_accumulation_vertices,
    ).toBeGreaterThan(10_000);
    expect(
      terrain.terrain_derivation.drainage.derived_channel_vertices,
    ).toBeGreaterThan(1_000);
    expect(
      terrain.terrain_derivation.drainage.maximum_incision_meters,
    ).toBeGreaterThan(25);
    expect(
      terrain.terrain_derivation.drainage.maximum_incision_meters,
    ).toBeLessThan(65);
    expect(
      terrain.terrain_derivation.drainage.maximum_stream_power,
    ).toBeGreaterThan(0.15);

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
        if (neighborhood.some(({ valid }) => !valid)) continue;
        supportedCenters += 1;
        const predicted =
          neighborhood
            .slice(1)
            .reduce((sum, cell) => sum + cell.sample.elevation, 0) / 4;
        const residual = Math.abs(neighborhood[0].sample.elevation - predicted);
        if (residual > 1) detailedCenters += 1;
        maximumResidual = Math.max(maximumResidual, residual);
      }
    }
    expect(supportedCenters).toBeGreaterThan(6_500);
    expect(detailedCenters).toBeGreaterThan(5_000);
    expect(maximumResidual).toBeGreaterThan(20);
    expect(detailedCenters / supportedCenters).toBeGreaterThan(0.15);
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
      hierarchy.labels.filter(({ properties }) => properties.min_zoom <= 6)
        .length,
    ).toBeGreaterThanOrEqual(14);
  });

  it("matches the checked-in native artifact byte for byte", () => {
    expect(
      readFileSync(resolve(sceneDirectory, "terrain.geojson"), "utf8"),
    ).toBe(serializeReyCountyTerrain(terrain));
  });

  it("packs the complete source grid into one bounded GeoJSON feature", () => {
    expect(terrain.features).toHaveLength(1);
    expect(terrain.features[0]).toMatchObject({
      id: "rey-county-packed-terrain-v8",
      geometry: { type: "Polygon" },
      terrain_grid: {
        schema: "rey.packed-terrain-grid.v1",
        dataset_id: "rey-county-semantic-terrain-v8",
        compiler_revision: "rey.agent-geography.rey-county@8",
        columns: 705,
        rows: 626,
        native_bounds_microdegrees: [
          -160000000, -20000000, -159120000, -19250000,
        ],
      },
    });
    expect(terrain.features[0].terrain_grid.validity_hex).toHaveLength(
      441_330 * 2,
    );
    expect(
      terrain.features[0].terrain_grid.elevation_centimeters_le_hex,
    ).toHaveLength(441_330 * 8);
    expect(terrain.features[0].terrain_grid.material_indices_hex).toHaveLength(
      441_330 * 2,
    );
  });
});
