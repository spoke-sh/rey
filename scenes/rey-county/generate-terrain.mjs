#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const SCENE_DIRECTORY = dirname(fileURLToPath(import.meta.url));
const OUTPUT_PATH = resolve(SCENE_DIRECTORY, "terrain.geojson");
// Two hundred intervals preserve exact integer-microdegree coordinates and
// give the worker a six-level source pyramid before bounded presentation
// refinement. GeoJSON remains an interchange slice; raster-native pyramids are
// still required for substantially finer fields.
const COLUMNS = 201;
const ROWS = 201;
const DATASET_ID = "rey-county-semantic-terrain-v5";
const GEOGRAPHY_COMPILER_REVISION = "rey.agent-geography.rey-county@5";
const INPUT_FILES = [
  "boundary.geojson",
  "districts.geojson",
  "features.geojson",
  "highways.geojson",
  "hydrology.geojson",
  "labels.geojson",
  "railways.geojson",
  "roads.geojson",
  "terrain-controls.geojson",
];

const CONTROL_POSTURE = Object.freeze({
  "terrain-anchor-range": { rotation: -0.42, width: 0.92, height: 0.72 },
  "terrain-architecture-highlands": {
    rotation: -0.16,
    width: 1.08,
    height: 0.58,
  },
  "terrain-explorer-terraces": {
    rotation: 0.18,
    width: 0.92,
    height: 0.76,
  },
  "terrain-runtime-basin": { rotation: 0.08, width: 1.12, height: 0.9 },
  "terrain-mining-ridge": { rotation: 0.3, width: 0.66, height: 1.08 },
  "terrain-proof-escarpment": {
    rotation: -0.24,
    width: 1.08,
    height: 0.56,
  },
  "terrain-anchor-summit": { rotation: 0, width: 0.17, height: 0.17 },
  "terrain-frontier-saddle": { rotation: 0, width: 0.2, height: 0.2 },
});

export function buildReyCountyTerrain(sceneDirectory = SCENE_DIRECTORY) {
  const inputs = Object.fromEntries(
    INPUT_FILES.map((name) => {
      const bytes = readFileSync(resolve(sceneDirectory, name));
      return [
        name,
        {
          bytes,
          document: JSON.parse(bytes.toString("utf8")),
          sha256: createHash("sha256").update(bytes).digest("hex"),
        },
      ];
    }),
  );
  const boundary =
    inputs["boundary.geojson"].document.features[0].geometry.coordinates[0];
  const bounds = ringBounds(boundary);
  const omission = inputs["features.geojson"].document.features.find(
    ({ id }) => id === "feature-omission-scrub",
  ).geometry.coordinates[0];
  const meadow = inputs["features.geojson"].document.features.find(
    ({ id }) => id === "feature-nine-channel-meadow",
  ).geometry.coordinates[0];
  const wetland = inputs["hydrology.geojson"].document.features.find(
    ({ id }) => id === "hydrology-frontier-wetland",
  ).geometry.coordinates[0];
  const waterways = inputs["hydrology.geojson"].document.features.filter(
    ({ geometry }) => geometry.type === "LineString",
  );
  const normalizedWaterways = waterways.map((feature) => ({
    id: feature.id,
    points: feature.geometry.coordinates.map((point) =>
      normalizePoint(point, bounds),
    ),
  }));
  const controls = inputs["terrain-controls.geojson"].document.features.map(
    (feature) => compileControl(feature, bounds),
  );
  const longitudeStep = (bounds.east - bounds.west) / (COLUMNS - 1);
  const latitudeStep = (bounds.north - bounds.south) / (ROWS - 1);
  const features = [];
  const summary = {
    valid_vertices: 0,
    no_data_vertices: 0,
    outside_footprint_vertices: 0,
    unexplored_vertices: 0,
    minimum_elevation_meters: Number.POSITIVE_INFINITY,
    maximum_elevation_meters: Number.NEGATIVE_INFINITY,
    materials: {},
  };

  for (let row = 0; row < ROWS; row += 1) {
    const latitude = roundCoordinate(bounds.north - row * latitudeStep);
    for (let column = 0; column < COLUMNS; column += 1) {
      const longitude = roundCoordinate(bounds.west + column * longitudeStep);
      const insideFootprint = pointInRing([longitude, latitude], boundary);
      const unexplored = pointInRing([longitude, latitude], omission);
      const valid = insideFootprint && !unexplored;
      const properties = {
        terrain_grid_id: DATASET_ID,
        terrain_grid_column: column,
        terrain_grid_row: row,
        terrain_grid_columns: COLUMNS,
        terrain_grid_rows: ROWS,
        terrain_grid_validity: valid ? "valid" : "no_data",
      };
      const coordinates = [longitude, latitude];

      if (valid) {
        const sample = terrainSample(
          longitude,
          latitude,
          bounds,
          controls,
          normalizedWaterways,
          meadow,
          wetland,
        );
        coordinates.push(sample.elevation);
        properties.material = sample.material;
        properties.landform = sample.landform;
        summary.valid_vertices += 1;
        summary.minimum_elevation_meters = Math.min(
          summary.minimum_elevation_meters,
          sample.elevation,
        );
        summary.maximum_elevation_meters = Math.max(
          summary.maximum_elevation_meters,
          sample.elevation,
        );
        summary.materials[sample.material] =
          (summary.materials[sample.material] ?? 0) + 1;
      } else {
        summary.no_data_vertices += 1;
        if (!insideFootprint) summary.outside_footprint_vertices += 1;
        if (unexplored) summary.unexplored_vertices += 1;
      }

      features.push({
        type: "Feature",
        id: `terrain-r${String(row).padStart(2, "0")}-c${String(column).padStart(2, "0")}`,
        properties,
        geometry: { type: "Point", coordinates },
      });
    }
  }

  summary.minimum_elevation_meters = roundElevation(
    summary.minimum_elevation_meters,
  );
  summary.maximum_elevation_meters = roundElevation(
    summary.maximum_elevation_meters,
  );

  return {
    type: "FeatureCollection",
    name: "Rey County authored semantic terrain",
    terrain_derivation: {
      schema: "rey.county-terrain-source.v5",
      dataset_id: DATASET_ID,
      compiler_revision: GEOGRAPHY_COMPILER_REVISION,
      authority:
        "authored semantic terrain candidate; not Earth elevation, an environment observation, or inferred survey coverage",
      generator: "scenes/rey-county/generate-terrain.mjs",
      grid: {
        columns: COLUMNS,
        rows: ROWS,
        row_zero: "north",
        column_zero: "west",
        longitude_step_degrees: roundCoordinate(longitudeStep),
        latitude_step_degrees: roundCoordinate(latitudeStep),
        nominal_longitude_spacing_meters: roundElevation(
          longitudeStep *
            111_320 *
            Math.cos((((bounds.north + bounds.south) / 2) * Math.PI) / 180),
        ),
        nominal_latitude_spacing_meters: roundElevation(latitudeStep * 111_132),
      },
      synthesis: {
        topology:
          "named terrain controls, exact County footprint, districts, hydrology, meadow, wetland, transport hierarchy, labels, and explicit unexplored polygon",
        elevation:
          "anisotropic named landforms plus deterministic domain-warped orographic backbones, branching ridges, incised ravines, and macro-to-fine relief kept below the source-grid Nyquist limit",
        hydrology:
          "exact river and wetland areas accompany a tributary hierarchy; smooth authored drainage constraints carve the final height field before the renderer derives bounded flow accumulation inside exact validity",
        land_cover:
          "deterministic elevation, moisture, exposure, meadow, wetland, and water-distance classification",
        cartography:
          "separately admitted district, highway, road, railway, marker, and label sources form a scale-aware hierarchy without changing terrain validity",
        stitching: {
          strategy: "single bounded County authoring domain",
          seam_count: 0,
          conflict_count: 0,
          omissions: [
            "cross-package seam and conflict resolution is not implemented by this compiler revision",
          ],
        },
      },
      first_principles: [
        "one admitted dataset retains identity across Atlas and Landscape postures",
        "explicit validity follows the County footprint and preserves Unexplored Scrub as no-data",
        "terrain controls influence authoring but do not themselves become admitted height",
        "hydrology carves the authored field without being relabeled as a route or observation",
        "material, elevation, validity, and presentation remain separate channels",
      ],
      project_bearings: {
        foundations: "Anchor Range",
        architecture: "Architecture Highlands",
        explorer: "Explorer Terraces",
        runtime: "Runtime Basin",
        mining: "Mining Ridge",
        proof: "Proof Escarpment",
        unknown: "Unexplored Scrub",
      },
      source_inputs: INPUT_FILES.map((name) => ({
        path: `scenes/rey-county/${name}`,
        sha256: inputs[name].sha256,
      })),
      summary,
    },
    features,
  };
}

export function serializeReyCountyTerrain(document) {
  return `${JSON.stringify(document, null, 2)}\n`;
}

function compileControl(feature, bounds) {
  const points = geometryPoints(feature.geometry);
  const featureBounds = ringBounds(points);
  const center = points.reduce(
    (sum, point) => [sum[0] + point[0], sum[1] + point[1]],
    [0, 0],
  );
  center[0] /= points.length;
  center[1] /= points.length;
  const posture = CONTROL_POSTURE[feature.id];
  const width = Math.max(
    posture?.width ?? 0.7,
    ((featureBounds.east - featureBounds.west) / (bounds.east - bounds.west)) *
      1.35,
  );
  const height = Math.max(
    posture?.height ?? 0.7,
    ((featureBounds.north - featureBounds.south) /
      (bounds.north - bounds.south)) *
      1.35,
  );
  return {
    id: feature.id,
    center: normalizePoint(center, bounds),
    width,
    height,
    rotation: posture?.rotation ?? 0,
    relativeElevation: feature.properties.relative_elevation,
    roughness: feature.properties.roughness ?? 0.45,
  };
}

function terrainSample(
  longitude,
  latitude,
  bounds,
  controls,
  normalizedWaterways,
  meadow,
  wetland,
) {
  const [x, y] = normalizePoint([longitude, latitude], bounds);
  let elevation = 360 + y * 115 + (1 - x) * 34;
  let roughness = 0.16;
  let strongest = { id: "county-foundation", magnitude: 0 };
  const influenceById = {};

  for (const control of controls) {
    const influence = anisotropicGaussian(x, y, control);
    influenceById[control.id] = influence;
    const amplitude = (control.relativeElevation - 0.47) * 1_180;
    const contribution = amplitude * influence;
    elevation += contribution;
    roughness += control.roughness * influence * 0.48;
    if (Math.abs(contribution) > strongest.magnitude) {
      strongest = { id: control.id, magnitude: Math.abs(contribution) };
    }
  }

  const warpX = fractalNoise(x, y, 17, [1.1, 2.2, 4.4]) * 0.026;
  const warpY = fractalNoise(x, y, 53, [1.0, 2.0, 4.0]) * 0.024;
  const warpedX = x + warpX;
  const warpedY = y + warpY;
  const reliefWeight = 0.24 + Math.min(1, roughness) * 0.76;
  const macroTexture =
    fractalNoise(warpedX, warpedY, 101, [1.2, 2.4, 4.8]) * 92;
  const mesoTexture = fractalNoise(warpedX, warpedY, 211, [3.5, 7, 14]) * 38;
  const ridgeTexture =
    ridgedFractalNoise(warpedX, warpedY, 307, [2.2, 4.4, 8.8, 17.6]) * 72;
  const fineTexture = fractalNoise(warpedX, warpedY, 401, [13, 27, 51]) * 24;
  const fineRidges =
    ridgedFractalNoise(warpedX, warpedY, 457, [11, 23, 43, 71]) * 34;
  elevation +=
    (macroTexture + mesoTexture + ridgeTexture + fineTexture + fineRidges) *
    reliefWeight;

  for (let index = 0; index < controls.length; index += 1) {
    const control = controls[index];
    if (control.roughness < 0.42) continue;
    elevation += orographicRelief(x, y, control, 701 + index * 149);
  }

  let nearestWaterway = { id: null, distance: Number.POSITIVE_INFINITY };
  for (const waterway of normalizedWaterways) {
    const distance = distanceToPolyline([x, y], waterway.points);
    if (distance < nearestWaterway.distance)
      nearestWaterway = { id: waterway.id, distance };
    const main = waterway.id === "hydrology-evidence-river";
    const width = main ? 0.021 : 0.011;
    const depth = main ? 148 : 76;
    elevation -= depth * Math.exp(-((distance / width) ** 2));
  }

  const terraceInfluence = influenceById["terrain-explorer-terraces"] ?? 0;
  const terracedElevation = Math.round(elevation / 12) * 12;
  elevation =
    elevation * (1 - terraceInfluence * 0.12) +
    terracedElevation * terraceInfluence * 0.12;

  const insideWetland = pointInRing([longitude, latitude], wetland);
  if (insideWetland) elevation -= 42;
  elevation = roundElevation(Math.max(32, elevation));

  const insideMeadow = pointInRing([longitude, latitude], meadow);
  const moisture = fractalNoise(warpedX, warpedY, 503, [3, 7, 17]);
  const exposure = fractalNoise(warpedX, warpedY, 601, [4, 11, 29]);
  let material;
  if (insideWetland || nearestWaterway.distance < 0.009) material = "sand";
  else if (insideMeadow) material = "vegetation";
  else if (elevation >= 1_060 && exposure > -0.48) material = "granite";
  else if (
    elevation >= 735 ||
    (elevation >= 560 && roughness >= 0.57 && exposure > -0.12)
  )
    material = "rock";
  else if (elevation <= 350 && moisture < 0.18) material = "soil";
  else material = "vegetation";

  return {
    elevation,
    material,
    landform: strongest.id.replace(/^terrain-/, ""),
  };
}

function fractalNoise(x, y, seed, frequencies) {
  let value = 0;
  let totalWeight = 0;
  frequencies.forEach((frequency, octave) => {
    const weight = 1 / 2 ** octave;
    value +=
      valueNoise(x * frequency, y * frequency, seed + octave * 101) * weight;
    totalWeight += weight;
  });
  return value / totalWeight;
}

function ridgedFractalNoise(x, y, seed, frequencies) {
  let value = 0;
  let totalWeight = 0;
  frequencies.forEach((frequency, octave) => {
    const weight = 1 / 2 ** octave;
    const noise = valueNoise(x * frequency, y * frequency, seed + octave * 131);
    value += ((1 - Math.abs(noise)) * 2 - 1) * weight;
    totalWeight += weight;
  });
  return value / totalWeight;
}

function valueNoise(x, y, seed) {
  const column = Math.floor(x);
  const row = Math.floor(y);
  const amountX = smoothFraction(x - column);
  const amountY = smoothFraction(y - row);
  const north = interpolate(
    signedHash(column, row, seed),
    signedHash(column + 1, row, seed),
    amountX,
  );
  const south = interpolate(
    signedHash(column, row + 1, seed),
    signedHash(column + 1, row + 1, seed),
    amountX,
  );
  return interpolate(north, south, amountY);
}

function signedHash(x, y, seed) {
  let value = Math.imul(x, 374_761_393);
  value = Math.imul(value ^ Math.imul(y, 668_265_263), 1_274_126_177);
  value = Math.imul(value ^ seed, 2_246_822_519);
  value ^= value >>> 13;
  return (value >>> 0) / 2_147_483_647.5 - 1;
}

function smoothFraction(value) {
  return value * value * (3 - 2 * value);
}

function interpolate(left, right, progress) {
  return left + (right - left) * progress;
}

function anisotropicGaussian(x, y, control) {
  const { rx, ry } = anisotropicCoordinates(x, y, control);
  return Math.exp(-0.5 * (rx ** 2 + ry ** 2));
}

function anisotropicCoordinates(x, y, control) {
  const dx = x - control.center[0];
  const dy = y - control.center[1];
  const cosine = Math.cos(control.rotation);
  const sine = Math.sin(control.rotation);
  const rx = dx * cosine - dy * sine;
  const ry = dx * sine + dy * cosine;
  const sigmaX = Math.max(0.035, control.width * 0.22);
  const sigmaY = Math.max(0.035, control.height * 0.22);
  return { rx: rx / sigmaX, ry: ry / sigmaY };
}

function orographicRelief(x, y, control, seed) {
  const { rx, ry } = anisotropicCoordinates(x, y, control);
  const distanceSquared = rx ** 2 + ry ** 2;
  const envelope = Math.exp(-0.42 * distanceSquared);
  if (envelope < 0.002) return 0;
  const warp =
    fractalNoise(rx * 0.18 + 0.31, ry * 0.18 - 0.17, seed, [1, 2, 4]) *
    0.38;
  const along = ry * 0.72 + warp;
  const across = rx * 0.96 + warp * 0.7;
  const backbone = ridgedFractalNoise(
    across,
    along,
    seed + 31,
    [0.72, 1.45, 2.9, 5.8],
  );
  const branches = ridgedFractalNoise(
    across * 0.78 + along * 0.36,
    along * 1.18,
    seed + 67,
    [1.5, 3, 6, 12],
  );
  const ravines = Math.max(
    0,
    ridgedFractalNoise(
      across * 0.62 - along * 0.48,
      along * 1.42,
      seed + 101,
      [2.4, 4.8, 9.6, 19.2],
    ),
  );
  const gain = 54 + control.roughness * 74;
  return envelope * gain * (backbone * 0.62 + branches * 0.38 - ravines * 0.3);
}

function distanceToPolyline(point, line) {
  let minimum = Number.POSITIVE_INFINITY;
  for (let index = 1; index < line.length; index += 1) {
    minimum = Math.min(
      minimum,
      distanceToSegment(point, line[index - 1], line[index]),
    );
  }
  return minimum;
}

function distanceToSegment(point, start, end) {
  const dx = end[0] - start[0];
  const dy = end[1] - start[1];
  const lengthSquared = dx * dx + dy * dy;
  if (lengthSquared === 0)
    return Math.hypot(point[0] - start[0], point[1] - start[1]);
  const projection = Math.max(
    0,
    Math.min(
      1,
      ((point[0] - start[0]) * dx + (point[1] - start[1]) * dy) / lengthSquared,
    ),
  );
  return Math.hypot(
    point[0] - (start[0] + projection * dx),
    point[1] - (start[1] + projection * dy),
  );
}

function geometryPoints(geometry) {
  if (geometry.type === "Point") return [geometry.coordinates];
  if (geometry.type === "Polygon") return geometry.coordinates[0].slice(0, -1);
  throw new Error(`unsupported terrain control geometry ${geometry.type}`);
}

function normalizePoint(point, bounds) {
  return [
    (point[0] - bounds.west) / (bounds.east - bounds.west),
    (point[1] - bounds.south) / (bounds.north - bounds.south),
  ];
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

function ringBounds(ring) {
  return ring.reduce(
    (bounds, point) => ({
      west: Math.min(bounds.west, point[0]),
      south: Math.min(bounds.south, point[1]),
      east: Math.max(bounds.east, point[0]),
      north: Math.max(bounds.north, point[1]),
    }),
    {
      west: Number.POSITIVE_INFINITY,
      south: Number.POSITIVE_INFINITY,
      east: Number.NEGATIVE_INFINITY,
      north: Number.NEGATIVE_INFINITY,
    },
  );
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
  const terrain = buildReyCountyTerrain();
  const serialized = serializeReyCountyTerrain(terrain);
  if (process.argv.includes("--check")) {
    const current = readFileSync(OUTPUT_PATH, "utf8");
    if (current !== serialized) {
      console.error("Rey County terrain is stale; regenerate terrain.geojson");
      process.exitCode = 1;
    } else {
      console.log(JSON.stringify(terrain.terrain_derivation.summary));
    }
  } else {
    writeFileSync(OUTPUT_PATH, serialized);
    console.log(JSON.stringify(terrain.terrain_derivation.summary));
  }
}
