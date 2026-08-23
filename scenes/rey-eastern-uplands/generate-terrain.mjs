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
const SEAM_TREND_RADIUS_ROWS = 8;
const SEAM_TRANSITION_COLUMNS = 24;
const INDEPENDENT_RELIEF_TRANSITION_COLUMNS = 36;
const DATASET_ID = "rey-eastern-uplands-semantic-terrain-v5";
const GEOGRAPHY_COMPILER_REVISION = "rey.agent-geography.rey-eastern-uplands@5";

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
  const seamContexts = Array.from({ length: ROWS }, (_, row) => {
    const sourceIndex =
      (REY_SEAM_ROW_START + row) * sourceGrid.columns + REY_SEAM_COLUMN;
    const sourceInteriorIndex = sourceIndex - 1;
    const valid = sourceValidity[sourceIndex] === 1;
    const elevation = sourceElevations.readInt32LE(sourceIndex * 4) / 100;
    return {
      valid,
      elevation,
      slope:
        elevation - sourceElevations.readInt32LE(sourceInteriorIndex * 4) / 100,
      material: valid
        ? sourceGrid.material_palette[sourceMaterials[sourceIndex]]
        : null,
    };
  });
  const seamTrend = seamContexts.map((_, row) =>
    smoothedSeamValue(seamContexts, row, "elevation"),
  );
  const seamSlopeTrend = seamContexts.map((_, row) =>
    smoothedSeamValue(seamContexts, row, "slope"),
  );

  for (let row = 0; row < ROWS; row += 1) {
    const latitude = roundCoordinate(NORTH - row * LATITUDE_STEP);
    const seam = seamContexts[row];
    for (let column = 0; column < COLUMNS; column += 1) {
      const index = row * COLUMNS + column;
      const longitude = roundCoordinate(WEST + column * LONGITUDE_STEP);
      const inside = pointInRing([longitude, latitude], ring);
      const valid = column === 0 ? seam.valid : inside;
      const sample = valid
        ? terrainSample(
            column,
            row,
            seam.elevation,
            seam.material,
            seam.slope,
            seamTrend[row],
            seamSlopeTrend[row],
          )
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
        schema: "rey.authored-regional-seam.v4",
        axis: "longitude",
        coordinate_microdegrees: -159120000,
        start_microdegrees: -19720400,
        end_microdegrees: -19520000,
        source_dataset_id: sourceGrid.dataset_id,
        source_column: REY_SEAM_COLUMN,
        source_row_start: REY_SEAM_ROW_START,
        source_interior_context_columns: 1,
        low_pass_trend_radius_rows: SEAM_TREND_RADIUS_ROWS,
        transition_columns: SEAM_TRANSITION_COLUMNS,
        compared_vertices: ROWS,
        validity_conflicts: 0,
        elevation_conflicts: 0,
        material_conflicts: 0,
        authority:
          "byte-exact copied source samples establish an assessable candidate boundary; only scene admission and regional composition may qualify it",
      },
      synthesis: {
        elevation:
          "bounded multi-scale ridge, spur, and branching valley geography whose displacement and first derivative are exactly zero on the shared western seam; the exact County edge slope continues through the first interior column, then a bounded corridor transitions into a low-pass boundary trend before independent landforms enter",
        independent_relief: {
          schema: "rey.authored-ridge-valley-network.v1",
          transition_columns: INDEPENDENT_RELIEF_TRANSITION_COLUMNS,
          ridge_segments: 8,
          valley_segments: 9,
          source_scale_octaves: 4,
          authority:
            "deterministically authored source elevation inside admitted-candidate validity; never renderer noise or inferred survey evidence",
        },
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
        id: "rey-eastern-uplands-packed-terrain-v3",
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
    grid.dataset_id !== "rey-county-semantic-terrain-v16" ||
    grid.compiler_revision !== "rey.agent-geography.rey-county@16" ||
    grid.columns !== 705 ||
    grid.rows !== 626 ||
    grid.native_bounds_microdegrees.join(",") !==
      "-160000000,-20000000,-159120000,-19250000"
  )
    throw new Error(
      "Rey Eastern Uplands requires the exact Rey County v16 grid",
    );
}

function terrainSample(
  column,
  row,
  seamElevation,
  seamMaterial,
  seamSlope,
  seamTrend,
  seamSlopeTrend,
) {
  if (column === 0) return { elevation: seamElevation, material: seamMaterial };
  if (column === 1)
    return {
      elevation: roundElevation(seamElevation + seamSlope),
      material: seamMaterial,
    };
  const x = (column - 1) / (COLUMNS - 2);
  const y = row / (ROWS - 1);
  const edgeEnvelope = Math.sin(Math.PI * Math.min(1, x)) ** 2;
  const transition = smootherstep((column - 1) / (SEAM_TRANSITION_COLUMNS - 1));
  const independentReliefEnvelope = smootherstep(
    (column - SEAM_TRANSITION_COLUMNS) / INDEPENDENT_RELIEF_TRANSITION_COLUMNS,
  );
  const boundaryTrend =
    seamElevation + seamSlope + (column - 1) * seamSlopeTrend;
  const interiorTrend = seamTrend + smootherstep(x) * 155;
  const reliefEnvelope = edgeEnvelope * independentReliefEnvelope;
  const regionalMass =
    92 * Math.sin((x * 1.35 + y * 0.42) * Math.PI) +
    58 * Math.cos((x * 0.55 - y * 1.65) * Math.PI);
  const ridge = ridgeNetwork(x, y);
  const valley = valleyNetwork(x, y);
  const sourceScaleTexture =
    54 * (fractalNoise(x * 7.5 + 19.2, y * 7.5 - 8.4) - 0.5) +
    24 * (ridgedNoise(x * 13.5 - 3.1, y * 13.5 + 11.7) - 0.5);
  const elevation = roundElevation(
    Math.max(
      32,
      boundaryTrend * (1 - transition) +
        interiorTrend * transition +
        reliefEnvelope * (regionalMass + ridge - valley + sourceScaleTexture),
    ),
  );
  return {
    elevation,
    material: classifyMaterial(elevation, y),
  };
}

function ridgeNetwork(x, y) {
  return (
    ridgeSegment(x, y, 0.2, 0.08, 0.82, 0.78, 0.055, 300) +
    ridgeSegment(x, y, 0.35, 0.92, 0.92, 0.4, 0.048, 235) +
    ridgeSegment(x, y, 0.38, 0.31, 0.24, 0.62, 0.027, 125) +
    ridgeSegment(x, y, 0.48, 0.42, 0.72, 0.18, 0.023, 118) +
    ridgeSegment(x, y, 0.55, 0.53, 0.75, 0.73, 0.022, 105) +
    ridgeSegment(x, y, 0.66, 0.61, 0.91, 0.84, 0.018, 82) +
    ridgeSegment(x, y, 0.68, 0.63, 0.9, 0.53, 0.017, 76) +
    ridgeSegment(x, y, 0.76, 0.54, 0.94, 0.24, 0.019, 88)
  );
}

function valleyNetwork(x, y) {
  return (
    valleySegment(x, y, 0.18, 0.82, 0.42, 0.69, 0.038, 152) +
    valleySegment(x, y, 0.42, 0.69, 0.64, 0.61, 0.034, 170) +
    valleySegment(x, y, 0.64, 0.61, 0.94, 0.3, 0.032, 188) +
    valleySegment(x, y, 0.31, 0.29, 0.48, 0.64, 0.021, 84) +
    valleySegment(x, y, 0.38, 0.91, 0.48, 0.7, 0.019, 76) +
    valleySegment(x, y, 0.54, 0.25, 0.62, 0.61, 0.018, 78) +
    valleySegment(x, y, 0.69, 0.82, 0.68, 0.57, 0.016, 66) +
    valleySegment(x, y, 0.84, 0.66, 0.78, 0.47, 0.015, 62) +
    valleySegment(x, y, 0.9, 0.13, 0.84, 0.4, 0.016, 70)
  );
}

function ridgeSegment(x, y, startX, startY, endX, endY, width, height) {
  const distance = distanceToSegment(x, y, startX, startY, endX, endY);
  return height * Math.exp(-Math.pow(distance / width, 2));
}

function valleySegment(x, y, startX, startY, endX, endY, width, depth) {
  const distance = distanceToSegment(x, y, startX, startY, endX, endY);
  return depth * Math.exp(-Math.pow(distance / width, 2));
}

function distanceToSegment(x, y, startX, startY, endX, endY) {
  const dx = endX - startX;
  const dy = endY - startY;
  const lengthSquared = dx * dx + dy * dy;
  const position = Math.max(
    0,
    Math.min(1, ((x - startX) * dx + (y - startY) * dy) / lengthSquared),
  );
  return Math.hypot(x - (startX + position * dx), y - (startY + position * dy));
}

function fractalNoise(x, y) {
  let amplitude = 0.5;
  let frequency = 1;
  let total = 0;
  let amplitudeTotal = 0;
  for (let octave = 0; octave < 4; octave += 1) {
    total += amplitude * valueNoise(x * frequency, y * frequency);
    amplitudeTotal += amplitude;
    amplitude *= 0.5;
    frequency *= 2;
  }
  return total / amplitudeTotal;
}

function ridgedNoise(x, y) {
  return 1 - Math.abs(2 * fractalNoise(x, y) - 1);
}

function valueNoise(x, y) {
  const column = Math.floor(x);
  const row = Math.floor(y);
  const localX = smootherstep(x - column);
  const localY = smootherstep(y - row);
  const north = interpolate(
    hashNoise(column, row),
    hashNoise(column + 1, row),
    localX,
  );
  const south = interpolate(
    hashNoise(column, row + 1),
    hashNoise(column + 1, row + 1),
    localX,
  );
  return interpolate(north, south, localY);
}

function hashNoise(column, row) {
  let value = Math.imul(column, 374761393) + Math.imul(row, 668265263);
  value = Math.imul(value ^ (value >>> 13), 1274126177);
  return ((value ^ (value >>> 16)) >>> 0) / 4294967295;
}

function interpolate(start, end, amount) {
  return start + (end - start) * amount;
}

function smoothedSeamValue(seamContexts, row, key) {
  let total = 0;
  let weightTotal = 0;
  for (
    let offset = -SEAM_TREND_RADIUS_ROWS;
    offset <= SEAM_TREND_RADIUS_ROWS;
    offset += 1
  ) {
    const context = seamContexts[row + offset];
    if (!context?.valid) continue;
    const weight = SEAM_TREND_RADIUS_ROWS + 1 - Math.abs(offset);
    total += context[key] * weight;
    weightTotal += weight;
  }
  if (weightTotal === 0)
    throw new Error(`Rey Eastern Uplands seam ${key} has no valid support`);
  return total / weightTotal;
}

function smootherstep(value) {
  const bounded = Math.max(0, Math.min(1, value));
  return bounded ** 3 * (bounded * (bounded * 6 - 15) + 10);
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
