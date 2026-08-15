#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const SCENE_DIRECTORY = dirname(fileURLToPath(import.meta.url));
const OUTPUT_PATH = resolve(SCENE_DIRECTORY, "terrain.geojson");
// Forty intervals on each axis are the smallest exact County grid that crosses
// the renderer's 32-interval tile boundary in both directions. The resulting
// 2×2 leaf set exercises residency without overflowing the bounded local
// workload result that must retain every admitted source binding.
const COLUMNS = 41;
const ROWS = 41;
const DATASET_ID = "rey-county-semantic-terrain-v1";
const INPUT_FILES = [
  "boundary.geojson",
  "features.geojson",
  "hydrology.geojson",
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
          waterways,
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
      schema: "rey.county-terrain-source.v1",
      dataset_id: DATASET_ID,
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
  waterways,
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

  const normalizedWaterways = waterways.map((feature) => ({
    id: feature.id,
    points: feature.geometry.coordinates.map((point) =>
      normalizePoint(point, bounds),
    ),
  }));
  let nearestWaterway = { id: null, distance: Number.POSITIVE_INFINITY };
  for (const waterway of normalizedWaterways) {
    const distance = distanceToPolyline([x, y], waterway.points);
    if (distance < nearestWaterway.distance)
      nearestWaterway = { id: waterway.id, distance };
    const main = waterway.id === "hydrology-evidence-river";
    const width = main ? 0.031 : 0.019;
    const depth = main ? 112 : 58;
    elevation -= depth * Math.exp(-((distance / width) ** 2));
  }

  const macroTexture =
    Math.sin((x * 6.4 + y * 2.7) * Math.PI * 2) * 16 +
    Math.cos((x * 2.8 - y * 7.1) * Math.PI * 2) * 11;
  const mesoTexture =
    Math.sin((x * 14.3 + y * 11.7) * Math.PI * 2) * 5.5 +
    Math.cos((x * 21.1 - y * 8.6) * Math.PI * 2) * 3.5;
  elevation += (macroTexture + mesoTexture) * Math.min(1, roughness);

  const terraceInfluence = influenceById["terrain-explorer-terraces"] ?? 0;
  const terracedElevation = Math.round(elevation / 24) * 24;
  elevation =
    elevation * (1 - terraceInfluence * 0.34) +
    terracedElevation * terraceInfluence * 0.34;

  const insideWetland = pointInRing([longitude, latitude], wetland);
  if (insideWetland) elevation -= 42;
  elevation = roundElevation(Math.max(32, elevation));

  const insideMeadow = pointInRing([longitude, latitude], meadow);
  let material;
  if (insideWetland || nearestWaterway.distance < 0.012) material = "sand";
  else if (insideMeadow) material = "vegetation";
  else if (elevation >= 860) material = "granite";
  else if (elevation >= 610 || roughness >= 0.58) material = "rock";
  else if (elevation <= 335) material = "soil";
  else material = "vegetation";

  return {
    elevation,
    material,
    landform: strongest.id.replace(/^terrain-/, ""),
  };
}

function anisotropicGaussian(x, y, control) {
  const dx = x - control.center[0];
  const dy = y - control.center[1];
  const cosine = Math.cos(control.rotation);
  const sine = Math.sin(control.rotation);
  const rx = dx * cosine - dy * sine;
  const ry = dx * sine + dy * cosine;
  const sigmaX = Math.max(0.035, control.width * 0.22);
  const sigmaY = Math.max(0.035, control.height * 0.22);
  return Math.exp(-0.5 * ((rx / sigmaX) ** 2 + (ry / sigmaY) ** 2));
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
