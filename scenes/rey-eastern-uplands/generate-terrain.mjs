#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const SCENE_DIRECTORY = dirname(fileURLToPath(import.meta.url));
const REPOSITORY_ROOT = resolve(SCENE_DIRECTORY, "../..");
const OUTPUT_PATH = resolve(SCENE_DIRECTORY, "terrain.geojson");
const REY_TERRAIN_PATH = resolve(
  REPOSITORY_ROOT,
  "scenes/rey-county/terrain.geojson",
);
const COLUMNS = 193;
const ROWS = 168;
const WEST = -159.12;
const EAST = -158.88;
const NORTH = -19.52;
const SOUTH = -19.7204;
const LONGITUDE_STEP = 0.00125;
const LATITUDE_STEP = 0.0012;
const REY_SEAM_COLUMN = 704;
const REY_SEAM_ROW_START = 225;
const DATASET_ID = "rey-eastern-uplands-semantic-terrain-v1";
const GEOGRAPHY_COMPILER_REVISION = "rey.agent-geography.rey-eastern-uplands@1";

export function buildReyEasternUplandsTerrainSource(
  sceneDirectory = SCENE_DIRECTORY,
  reyTerrainPath = REY_TERRAIN_PATH,
) {
  const boundaryBytes = readFileSync(
    resolve(sceneDirectory, "boundary.geojson"),
  );
  const reyTerrainBytes = readFileSync(reyTerrainPath);
  const boundary = JSON.parse(boundaryBytes.toString("utf8"));
  const reyTerrain = JSON.parse(reyTerrainBytes.toString("utf8"));
  const ring = boundary.features[0].geometry.coordinates[0];
  const sourceGrid = reyTerrain.features[0].terrain_grid;
  verifySourceGrid(sourceGrid);
  const sourceValidity = Buffer.from(sourceGrid.validity_hex, "hex");
  const sourceElevations = Buffer.from(
    sourceGrid.elevation_centimeters_le_hex,
    "hex",
  );
  const sourceMaterials = Buffer.from(sourceGrid.material_indices_hex, "hex");
  const cells = [];
  const materialPalette = [...sourceGrid.material_palette];
  const materialIndices = new Map(
    materialPalette.map((material, index) => [material, index]),
  );
  const validity = Buffer.alloc(COLUMNS * ROWS);
  const elevations = Buffer.alloc(COLUMNS * ROWS * 4);
  const materials = Buffer.alloc(COLUMNS * ROWS, 255);
  let validVertices = 0;
  let noDataVertices = 0;
  let minimumElevation = Number.POSITIVE_INFINITY;
  let maximumElevation = Number.NEGATIVE_INFINITY;
  const materialCounts = {};

  for (let row = 0; row < ROWS; row += 1) {
    const latitude = roundCoordinate(NORTH - row * LATITUDE_STEP);
    const sourceIndex =
      (REY_SEAM_ROW_START + row) * sourceGrid.columns + REY_SEAM_COLUMN;
    const seamValid = sourceValidity[sourceIndex] === 1;
    const seamElevation = sourceElevations.readInt32LE(sourceIndex * 4) / 100;
    const seamMaterial = seamValid
      ? sourceGrid.material_palette[sourceMaterials[sourceIndex]]
      : null;
    for (let column = 0; column < COLUMNS; column += 1) {
      const index = row * COLUMNS + column;
      const longitude = roundCoordinate(WEST + column * LONGITUDE_STEP);
      const inside = pointInRing([longitude, latitude], ring);
      const valid = column === 0 ? seamValid : inside;
      const sample = valid
        ? terrainSample(column, row, seamElevation, seamMaterial)
        : null;
      if (valid) {
        validity[index] = 1;
        elevations.writeInt32LE(Math.round(sample.elevation * 100), index * 4);
        materials[index] = materialIndices.get(sample.material);
        validVertices += 1;
        minimumElevation = Math.min(minimumElevation, sample.elevation);
        maximumElevation = Math.max(maximumElevation, sample.elevation);
        materialCounts[sample.material] =
          (materialCounts[sample.material] ?? 0) + 1;
      } else {
        noDataVertices += 1;
      }
      cells.push({
        column,
        row,
        longitude,
        latitude,
        valid,
        sample,
      });
    }
  }

  const document = {
    type: "FeatureCollection",
    name: "Rey Eastern Uplands authored semantic terrain",
    terrain_derivation: {
      schema: "rey.regional-terrain-source.v1",
      dataset_id: DATASET_ID,
      compiler_revision: GEOGRAPHY_COMPILER_REVISION,
      authority:
        "authored semantic terrain candidate derived from one exact admitted-candidate boundary and the byte-exact Rey County eastern seam; not Earth elevation, an environment observation, or inferred survey coverage",
      generator: "scenes/rey-eastern-uplands/generate-terrain.mjs",
      grid: {
        columns: COLUMNS,
        rows: ROWS,
        row_zero: "north",
        column_zero: "west",
        longitude_step_degrees: LONGITUDE_STEP,
        latitude_step_degrees: LATITUDE_STEP,
      },
      source_inputs: [
        {
          path: "scenes/rey-eastern-uplands/boundary.geojson",
          sha256: sha256(boundaryBytes),
        },
        {
          path: "scenes/rey-county/terrain.geojson",
          sha256: sha256(reyTerrainBytes),
          role: "exact western seam samples only",
        },
      ],
      seam: {
        schema: "rey.authored-regional-seam.v1",
        axis: "longitude",
        coordinate_microdegrees: -159120000,
        start_microdegrees: -19720400,
        end_microdegrees: -19520000,
        source_dataset_id: sourceGrid.dataset_id,
        source_column: REY_SEAM_COLUMN,
        source_row_start: REY_SEAM_ROW_START,
        compared_vertices: ROWS,
        validity_conflicts: 0,
        elevation_conflicts: 0,
        material_conflicts: 0,
        authority:
          "byte-exact copied source samples establish an assessable candidate boundary; only scene admission and regional composition may qualify it",
      },
      synthesis: {
        elevation:
          "smooth bounded upland folds and valleys whose displacement is exactly zero on the shared western seam",
        validity:
          "explicit polygon-contained support; no-data outside the authored boundary remains unsupported",
        stitching:
          "no merge or gap fill is claimed; this package exposes one exact candidate seam for independent admission and composition assessment",
      },
      summary: {
        valid_vertices: validVertices,
        no_data_vertices: noDataVertices,
        minimum_elevation_meters: roundElevation(minimumElevation),
        maximum_elevation_meters: roundElevation(maximumElevation),
        materials: materialCounts,
      },
    },
    features: [
      {
        type: "Feature",
        id: "rey-eastern-uplands-packed-terrain-v1",
        properties: {
          title: "Rey Eastern Uplands admitted landscape terrain",
          source_kind: "packed_rectilinear_terrain",
        },
        terrain_grid: {
          schema: "rey.packed-terrain-grid.v1",
          dataset_id: DATASET_ID,
          compiler_revision: GEOGRAPHY_COMPILER_REVISION,
          columns: COLUMNS,
          rows: ROWS,
          native_bounds_microdegrees: [
            Math.round(WEST * 1_000_000),
            Math.round(SOUTH * 1_000_000),
            Math.round(EAST * 1_000_000),
            Math.round(NORTH * 1_000_000),
          ],
          validity_hex: validity.toString("hex"),
          elevation_centimeters_le_hex: elevations.toString("hex"),
          material_palette: materialPalette,
          material_indices_hex: materials.toString("hex"),
        },
        geometry: {
          type: "Polygon",
          coordinates: [
            [
              [WEST, NORTH],
              [EAST, NORTH],
              [EAST, SOUTH],
              [WEST, SOUTH],
              [WEST, NORTH],
            ],
          ],
        },
      },
    ],
  };
  return { document, cells };
}

export function serializeReyEasternUplandsTerrain(document) {
  return `${JSON.stringify(document, null, 2)}\n`;
}

function verifySourceGrid(grid) {
  if (
    grid.dataset_id !== "rey-county-semantic-terrain-v15" ||
    grid.compiler_revision !== "rey.agent-geography.rey-county@15" ||
    grid.columns !== 705 ||
    grid.rows !== 626 ||
    grid.native_bounds_microdegrees.join(",") !==
      "-160000000,-20000000,-159120000,-19250000"
  )
    throw new Error(
      "Rey Eastern Uplands requires the exact Rey County v15 grid",
    );
}

function terrainSample(column, row, seamElevation, seamMaterial) {
  if (column === 0) return { elevation: seamElevation, material: seamMaterial };
  const x = column / (COLUMNS - 1);
  const y = row / (ROWS - 1);
  const edgeEnvelope = Math.sin(Math.PI * Math.min(1, x));
  const folds =
    138 * Math.sin(Math.PI * x) * Math.cos((y * 2.2 + x * 0.35) * Math.PI) +
    74 * edgeEnvelope * Math.sin((x * 4.1 - y * 3.4) * Math.PI) +
    36 * edgeEnvelope * Math.cos((x * 9.3 + y * 5.7) * Math.PI);
  const ridge =
    245 *
    edgeEnvelope ** 0.82 *
    Math.exp(-Math.pow((y - (0.42 + 0.11 * Math.sin(x * Math.PI))) / 0.19, 2));
  const valley =
    118 * edgeEnvelope * Math.exp(-Math.pow((y - (0.7 - 0.18 * x)) / 0.085, 2));
  const elevation = roundElevation(
    Math.max(32, seamElevation + x * 95 + folds + ridge - valley),
  );
  return {
    elevation,
    material: classifyMaterial(elevation, y),
  };
}

function classifyMaterial(elevation, moisture) {
  if (elevation > 1500) return "granite";
  if (elevation > 1120) return "rock";
  if (moisture > 0.73) return "soil";
  if (elevation < 480) return "sand";
  return "vegetation";
}

function pointInRing(point, ring) {
  let inside = false;
  for (
    let current = 0, previous = ring.length - 1;
    current < ring.length;
    previous = current++
  ) {
    const start = ring[previous];
    const end = ring[current];
    if (pointOnSegment(point, start, end)) return true;
    const crosses =
      end[1] > point[1] !== start[1] > point[1] &&
      point[0] <
        ((start[0] - end[0]) * (point[1] - end[1])) / (start[1] - end[1]) +
          end[0];
    if (crosses) inside = !inside;
  }
  return inside;
}

function pointOnSegment(point, start, end) {
  const cross =
    (point[0] - start[0]) * (end[1] - start[1]) -
    (point[1] - start[1]) * (end[0] - start[0]);
  if (Math.abs(cross) > 1e-10) return false;
  return (
    point[0] >= Math.min(start[0], end[0]) - 1e-10 &&
    point[0] <= Math.max(start[0], end[0]) + 1e-10 &&
    point[1] >= Math.min(start[1], end[1]) - 1e-10 &&
    point[1] <= Math.max(start[1], end[1]) + 1e-10
  );
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function roundCoordinate(value) {
  return Number(value.toFixed(6));
}

function roundElevation(value) {
  return Number(value.toFixed(2));
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  const { document } = buildReyEasternUplandsTerrainSource();
  const serialized = serializeReyEasternUplandsTerrain(document);
  if (process.argv.includes("--check")) {
    if (readFileSync(OUTPUT_PATH, "utf8") !== serialized) {
      console.error(
        "Rey Eastern Uplands terrain is stale; regenerate terrain.geojson",
      );
      process.exitCode = 1;
    }
  } else {
    writeFileSync(OUTPUT_PATH, serialized);
  }
}
