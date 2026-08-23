#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const SCENE_DIRECTORY = dirname(fileURLToPath(import.meta.url));
const OUTPUT_PATH = resolve(SCENE_DIRECTORY, "terrain.geojson");
// The rectangular lattice uses exact integer-microdegree intervals while
// remaining below the packed-source and retained hierarchy budgets. Its
// near-square metric cells increase native geomorphic support without asking
// the renderer to synthesize detail. A tiled raster adapter remains the path
// beyond this bounded in-memory grid.
const COLUMNS = 705;
const ROWS = 626;
const DATASET_ID = "rey-county-semantic-terrain-v11";
const GEOGRAPHY_COMPILER_REVISION = "rey.agent-geography.rey-county@11";
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

export function buildReyCountyTerrainSource(sceneDirectory = SCENE_DIRECTORY) {
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
  const longitudeSpacingMeters =
    longitudeStep *
    111_320 *
    Math.cos((((bounds.north + bounds.south) / 2) * Math.PI) / 180);
  const latitudeSpacingMeters = latitudeStep * 111_132;
  const cells = [];
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
      cells.push({
        row,
        column,
        longitude,
        latitude,
        insideFootprint,
        unexplored,
        valid,
        sample: valid
          ? terrainSample(
              longitude,
              latitude,
              bounds,
              controls,
              normalizedWaterways,
              meadow,
              wetland,
            )
          : null,
      });
    }
  }

  const drainage = applyDrainageIncision(
    cells,
    longitudeSpacingMeters,
    latitudeSpacingMeters,
  );
  for (const cell of cells) {
    if (cell.valid) {
      const sample = cell.sample;
      sample.elevation = roundElevation(Math.max(32, sample.elevation));
      sample.material = classifyTerrainMaterial(
        sample.elevation,
        sample.material_context,
      );
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
      if (!cell.insideFootprint) summary.outside_footprint_vertices += 1;
      if (cell.unexplored) summary.unexplored_vertices += 1;
    }
  }

  summary.minimum_elevation_meters = roundElevation(
    summary.minimum_elevation_meters,
  );
  summary.maximum_elevation_meters = roundElevation(
    summary.maximum_elevation_meters,
  );
  const geomorphology = summarizeLocalRelief(
    cells,
    4,
    longitudeSpacingMeters,
    latitudeSpacingMeters,
  );

  const materialPalette = Object.keys(summary.materials).sort();
  const materialIndices = new Map(
    materialPalette.map((material, index) => [material, index]),
  );
  const validity = Buffer.alloc(cells.length);
  const elevations = Buffer.alloc(cells.length * 4);
  const materials = Buffer.alloc(cells.length, 255);
  for (let index = 0; index < cells.length; index += 1) {
    const cell = cells[index];
    if (!cell.valid) continue;
    validity[index] = 1;
    elevations.writeInt32LE(Math.round(cell.sample.elevation * 100), index * 4);
    materials[index] = materialIndices.get(cell.sample.material);
  }

  const document = {
    type: "FeatureCollection",
    name: "Rey County authored semantic terrain",
    terrain_derivation: {
      schema: "rey.county-terrain-source.v11",
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
          longitudeSpacingMeters,
        ),
        nominal_latitude_spacing_meters: roundElevation(latitudeSpacingMeters),
      },
      synthesis: {
        topology:
          "named terrain controls, exact County footprint, districts, hydrology, meadow, wetland, transport hierarchy, labels, and explicit unexplored polygon",
        elevation:
          "anisotropic named landforms plus deterministic domain-warped irregular mountain mass, locally bounded cross-oriented hybrid ridges, branching sharp crests, incised ravines, and macro-to-fine relief followed by slope-conditioned dendritic stream-power and valley-width drainage below the source-grid Nyquist limit",
        hydrology:
          "exact river and wetland areas accompany a tributary hierarchy; authored constraints and deterministic depression-safe source drainage carve the final height field without crossing no-data",
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
      drainage,
      geomorphology,
      source_inputs: INPUT_FILES.map((name) => ({
        path: `scenes/rey-county/${name}`,
        sha256: inputs[name].sha256,
      })),
      summary,
    },
    features: [
      {
        type: "Feature",
        id: "rey-county-packed-terrain-v11",
        properties: {
          title: "Rey County admitted landscape terrain",
          source_kind: "packed_rectilinear_terrain",
        },
        terrain_grid: {
          schema: "rey.packed-terrain-grid.v1",
          dataset_id: DATASET_ID,
          compiler_revision: GEOGRAPHY_COMPILER_REVISION,
          columns: COLUMNS,
          rows: ROWS,
          native_bounds_microdegrees: [
            Math.round(bounds.west * 1_000_000),
            Math.round(bounds.south * 1_000_000),
            Math.round(bounds.east * 1_000_000),
            Math.round(bounds.north * 1_000_000),
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
              [bounds.west, bounds.north],
              [bounds.east, bounds.north],
              [bounds.east, bounds.south],
              [bounds.west, bounds.south],
              [bounds.west, bounds.north],
            ],
          ],
        },
      },
    ],
  };
  return { document, cells };
}

function summarizeLocalRelief(
  cells,
  radiusCells,
  longitudeSpacingMeters,
  latitudeSpacingMeters,
) {
  const ranges = [];
  const indexAt = (column, row) => row * COLUMNS + column;
  for (let row = radiusCells; row < ROWS - radiusCells; row += radiusCells) {
    for (
      let column = radiusCells;
      column < COLUMNS - radiusCells;
      column += radiusCells
    ) {
      const neighborhood = [
        cells[indexAt(column, row)],
        cells[indexAt(column - radiusCells, row)],
        cells[indexAt(column + radiusCells, row)],
        cells[indexAt(column, row - radiusCells)],
        cells[indexAt(column, row + radiusCells)],
      ];
      if (neighborhood.some(({ valid }) => !valid)) continue;
      const elevations = neighborhood.map(({ sample }) => sample.elevation);
      ranges.push(Math.max(...elevations) - Math.min(...elevations));
    }
  }
  ranges.sort((left, right) => left - right);
  const percentile = (proportion) =>
    roundElevation(ranges[Math.floor((ranges.length - 1) * proportion)]);
  return {
    schema: "rey.county-source-geomorphology.v1",
    authority:
      "deterministic authored-source local-relief summary; not an Earth DEM observation or perceptual fidelity result",
    sample_radius_cells: radiusCells,
    nominal_sample_radius_meters: roundElevation(
      (radiusCells * (longitudeSpacingMeters + latitudeSpacingMeters)) / 2,
    ),
    supported_samples: ranges.length,
    local_relief_p50_meters: percentile(0.5),
    local_relief_p75_meters: percentile(0.75),
    local_relief_p90_meters: percentile(0.9),
    local_relief_p99_meters: percentile(0.99),
  };
}

export function buildReyCountyTerrain(sceneDirectory = SCENE_DIRECTORY) {
  return buildReyCountyTerrainSource(sceneDirectory).document;
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
  const mesoTexture = fractalNoise(warpedX, warpedY, 211, [3.5, 7, 14]) * 82;
  const ridgeTexture =
    ridgedFractalNoise(warpedX, warpedY, 307, [2.2, 4.4, 8.8, 17.6]) * 24;
  const ruggedMass =
    fractalNoise(
      warpedX + warpY * 0.37,
      warpedY - warpX * 0.29,
      337,
      [4.7, 9.9, 20.3, 41.1, 83.7],
    ) * 165;
  const mountainMass =
    hybridRidgedFractalNoise(
      warpedX + warpY * 1.3,
      warpedY - warpX * 0.8,
      521,
      [1.35, 2.7, 5.4, 10.8, 21.6],
    ) *
      42 +
    hybridRidgedFractalNoise(
      warpedX * 0.72 - warpedY * 0.44,
      warpedY * 0.86 + warpedX * 0.31,
      557,
      [2.1, 4.2, 8.4, 16.8, 33.6],
    ) *
      24;
  const fineTexture =
    fractalNoise(warpedX, warpedY, 401, [17, 37, 79, 157]) * 52;
  const fineRidges =
    ridgedFractalNoise(warpedX, warpedY, 457, [19, 41, 83, 167, 223]) * 14;
  elevation +=
    (macroTexture +
      mesoTexture +
      ridgeTexture +
      ruggedMass +
      mountainMass +
      fineTexture +
      fineRidges) *
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
    const width = main ? 0.0035 : 0.0022;
    const depth = main ? 32 : 18;
    elevation -= depth * Math.exp(-((distance / width) ** 2));
  }

  const terraceInfluence = influenceById["terrain-explorer-terraces"] ?? 0;
  const terracedElevation = Math.round(elevation / 12) * 12;
  elevation =
    elevation * (1 - terraceInfluence * 0.12) +
    terracedElevation * terraceInfluence * 0.12;

  const insideWetland = pointInRing([longitude, latitude], wetland);
  if (insideWetland) elevation -= 42;

  const insideMeadow = pointInRing([longitude, latitude], meadow);
  const moisture = fractalNoise(warpedX, warpedY, 503, [3, 7, 17]);
  const exposure = fractalNoise(warpedX, warpedY, 601, [4, 11, 29]);

  return {
    elevation,
    material: null,
    material_context: {
      insideWetland,
      insideMeadow,
      waterDistance: nearestWaterway.distance,
      roughness,
      exposure,
      moisture,
    },
    landform: strongest.id.replace(/^terrain-/, ""),
  };
}

function classifyTerrainMaterial(elevation, context) {
  if (context.insideWetland || context.waterDistance < 0.009) return "sand";
  if (context.insideMeadow) return "vegetation";
  if (elevation >= 1_060 && context.exposure > -0.48) return "granite";
  if (
    elevation >= 735 ||
    (elevation >= 560 && context.roughness >= 0.57 && context.exposure > -0.12)
  )
    return "rock";
  if (elevation <= 350 && context.moisture < 0.18) return "soil";
  return "vegetation";
}

function applyDrainageIncision(
  cells,
  longitudeSpacingMeters,
  latitudeSpacingMeters,
) {
  const count = COLUMNS * ROWS;
  const hydraulicHeight = new Float64Array(count);
  const receiver = new Int32Array(count);
  receiver.fill(-1);
  const floodParent = new Int32Array(count);
  floodParent.fill(-1);
  const visited = new Uint8Array(count);
  const queue = new MinimumHeightQueue();
  const indexAt = (column, row) => row * COLUMNS + column;
  const offsets = [
    [-1, -1],
    [0, -1],
    [1, -1],
    [-1, 0],
    [1, 0],
    [-1, 1],
    [0, 1],
    [1, 1],
  ];
  const validityBoundary = (column, row) =>
    column === 0 ||
    row === 0 ||
    column === COLUMNS - 1 ||
    row === ROWS - 1 ||
    offsets.some(([dx, dy]) => {
      const nextColumn = column + dx;
      const nextRow = row + dy;
      return (
        nextColumn < 0 ||
        nextColumn >= COLUMNS ||
        nextRow < 0 ||
        nextRow >= ROWS ||
        !cells[indexAt(nextColumn, nextRow)].valid
      );
    });
  for (const cell of cells) {
    if (!cell.valid || !validityBoundary(cell.column, cell.row)) continue;
    const index = indexAt(cell.column, cell.row);
    visited[index] = 1;
    hydraulicHeight[index] = cell.sample.elevation;
    queue.push(index, hydraulicHeight[index]);
  }
  while (queue.size > 0) {
    const current = queue.pop();
    const column = current.index % COLUMNS;
    const row = Math.floor(current.index / COLUMNS);
    for (const [dx, dy] of offsets) {
      const nextColumn = column + dx;
      const nextRow = row + dy;
      if (
        nextColumn < 0 ||
        nextColumn >= COLUMNS ||
        nextRow < 0 ||
        nextRow >= ROWS
      )
        continue;
      const next = indexAt(nextColumn, nextRow);
      if (!cells[next].valid || visited[next] !== 0) continue;
      visited[next] = 1;
      floodParent[next] = current.index;
      hydraulicHeight[next] = Math.max(
        cells[next].sample.elevation,
        current.height + 0.001,
      );
      queue.push(next, hydraulicHeight[next]);
    }
  }

  // Priority-flood parents guarantee an outlet but form a traversal tree whose
  // grid posture becomes visible when used directly. Recover D8 steepest
  // descent over the depression-safe surface, retaining the flood parent only
  // as a deterministic escape from a numerically flat cell.
  for (const cell of cells) {
    if (!cell.valid || validityBoundary(cell.column, cell.row)) continue;
    const index = indexAt(cell.column, cell.row);
    let steepestReceiver = -1;
    let steepestSlope = 0;
    for (const [dx, dy] of offsets) {
      const nextColumn = cell.column + dx;
      const nextRow = cell.row + dy;
      if (
        nextColumn < 0 ||
        nextColumn >= COLUMNS ||
        nextRow < 0 ||
        nextRow >= ROWS
      )
        continue;
      const next = indexAt(nextColumn, nextRow);
      if (!cells[next].valid) continue;
      const drop = hydraulicHeight[index] - hydraulicHeight[next];
      if (drop <= 0) continue;
      const slope = drop / Math.hypot(dx, dy);
      if (
        slope > steepestSlope ||
        (slope === steepestSlope && next < steepestReceiver)
      ) {
        steepestSlope = slope;
        steepestReceiver = next;
      }
    }
    receiver[index] =
      steepestReceiver >= 0 ? steepestReceiver : floodParent[index];
  }

  const ordered = cells
    .map((cell, index) => ({ cell, index }))
    .filter(({ cell }) => cell.valid)
    .sort(
      (left, right) =>
        hydraulicHeight[right.index] - hydraulicHeight[left.index] ||
        right.index - left.index,
    );
  const accumulation = new Float64Array(count);
  for (const { index } of ordered) accumulation[index] = 1;
  for (const { index } of ordered) {
    const target = receiver[index];
    if (target >= 0) accumulation[target] += accumulation[index];
  }
  const maximumAccumulation = ordered.reduce(
    (maximum, { index }) => Math.max(maximum, accumulation[index]),
    1,
  );
  const denominator = Math.log1p(maximumAccumulation);
  const strength = new Float64Array(count);
  for (const { index } of ordered) {
    const normalized = Math.log1p(accumulation[index]) / denominator;
    strength[index] = smootherstep((normalized - 0.36) / 0.52);
  }
  const innerValley = smoothWithinValidity(strength, cells, 1);
  const outerValley = smoothWithinValidity(strength, cells, 4);
  const channel = new Uint8Array(count);
  const incomingChannels = new Uint16Array(count);
  for (const { index } of ordered) {
    if (strength[index] < 0.02) continue;
    channel[index] = 1;
  }
  for (const { index } of ordered) {
    const target = receiver[index];
    if (channel[index] !== 0 && target >= 0 && channel[target] !== 0)
      incomingChannels[target] += 1;
  }
  const maximumIncomingOrder = new Uint8Array(count);
  const maximumIncomingOrderCount = new Uint8Array(count);
  const strahlerOrder = new Uint8Array(count);
  for (const { index } of ordered) {
    if (channel[index] === 0) continue;
    const incomingOrder = maximumIncomingOrder[index];
    strahlerOrder[index] =
      incomingOrder === 0
        ? 1
        : incomingOrder + (maximumIncomingOrderCount[index] >= 2 ? 1 : 0);
    const target = receiver[index];
    if (target < 0 || channel[target] === 0) continue;
    if (strahlerOrder[index] > maximumIncomingOrder[target]) {
      maximumIncomingOrder[target] = strahlerOrder[index];
      maximumIncomingOrderCount[target] = 1;
    } else if (strahlerOrder[index] === maximumIncomingOrder[target]) {
      maximumIncomingOrderCount[target] += 1;
    }
  }
  let maximumIncision = 0;
  let maximumStreamPower = 0;
  let derivedChannelVertices = 0;
  let channelHeadVertices = 0;
  let branchJunctionVertices = 0;
  let maximumStrahlerOrder = 0;
  for (const { cell, index } of ordered) {
    const target = receiver[index];
    const receiverCell = target < 0 ? null : cells[target];
    const distanceMeters = receiverCell
      ? Math.hypot(
          (receiverCell.column - cell.column) * longitudeSpacingMeters,
          (receiverCell.row - cell.row) * latitudeSpacingMeters,
        )
      : 1;
    const localSlope = receiverCell
      ? Math.max(
          0,
          (cell.sample.elevation - receiverCell.sample.elevation) /
            distanceMeters,
        )
      : 0;
    const normalizedAccumulation =
      Math.log1p(accumulation[index]) / denominator;
    const streamPower =
      normalizedAccumulation ** 0.46 *
      Math.min(1, Math.max(0.08, localSlope / 0.12)) ** 0.7;
    maximumStreamPower = Math.max(maximumStreamPower, streamPower);
    const slopeResponse = smootherstep((localSlope - 0.0025) / 0.035);
    const incision =
      (strength[index] * (8 + streamPower * 22) +
        innerValley[index] * 16 +
        outerValley[index] * 4) *
      (0.02 + slopeResponse * 0.98);
    cell.sample.elevation -= incision;
    maximumIncision = Math.max(maximumIncision, incision);
    if (channel[index] === 0) continue;
    derivedChannelVertices += 1;
    if (incomingChannels[index] === 0) channelHeadVertices += 1;
    if (incomingChannels[index] >= 2) branchJunctionVertices += 1;
    maximumStrahlerOrder = Math.max(maximumStrahlerOrder, strahlerOrder[index]);
  }
  return {
    schema: "rey.county-source-drainage.v2",
    authority:
      "deterministic authored-source derivation inside exact validity; not observed hydrology",
    depression_handling:
      "priority flood seeded only from exact validity boundaries followed by steepest descent and slope-conditioned stream-power incision on the unfilled local terrain slope; flat escape topology cannot become visible height and variable valley widths never cross no-data",
    maximum_accumulation_vertices: maximumAccumulation,
    derived_channel_vertices: derivedChannelVertices,
    channel_head_vertices: channelHeadVertices,
    branch_junction_vertices: branchJunctionVertices,
    maximum_strahler_order: maximumStrahlerOrder,
    maximum_incision_meters: roundElevation(maximumIncision),
    maximum_stream_power: Number(maximumStreamPower.toFixed(6)),
    maximum_valley_half_width_cells: 6,
  };
}

function smoothWithinValidity(values, cells, passes) {
  let current = values.slice();
  const weights = [1, 2, 1];
  for (let pass = 0; pass < passes; pass += 1) {
    const next = new Float64Array(current.length);
    for (const cell of cells) {
      if (!cell.valid) continue;
      let total = 0;
      let totalWeight = 0;
      for (let dy = -1; dy <= 1; dy += 1) {
        for (let dx = -1; dx <= 1; dx += 1) {
          const column = cell.column + dx;
          const row = cell.row + dy;
          if (column < 0 || column >= COLUMNS || row < 0 || row >= ROWS)
            continue;
          const index = row * COLUMNS + column;
          if (!cells[index].valid) continue;
          const weight = weights[dx + 1] * weights[dy + 1];
          total += current[index] * weight;
          totalWeight += weight;
        }
      }
      next[cell.row * COLUMNS + cell.column] =
        totalWeight === 0
          ? current[cell.row * COLUMNS + cell.column]
          : total / totalWeight;
    }
    current = next;
  }
  return current;
}

function smootherstep(value) {
  const bounded = Math.max(0, Math.min(1, value));
  return bounded * bounded * bounded * (bounded * (bounded * 6 - 15) + 10);
}

class MinimumHeightQueue {
  entries = [];

  get size() {
    return this.entries.length;
  }

  push(index, height) {
    const entry = { index, height };
    this.entries.push(entry);
    let cursor = this.entries.length - 1;
    while (cursor > 0) {
      const parent = Math.floor((cursor - 1) / 2);
      if (this.entries[parent].height <= height) break;
      this.entries[cursor] = this.entries[parent];
      cursor = parent;
    }
    this.entries[cursor] = entry;
  }

  pop() {
    const result = this.entries[0];
    const tail = this.entries.pop();
    if (!result || !tail || this.entries.length === 0) return result;
    let cursor = 0;
    while (true) {
      const left = cursor * 2 + 1;
      const right = left + 1;
      if (left >= this.entries.length) break;
      const child =
        right < this.entries.length &&
        this.entries[right].height < this.entries[left].height
          ? right
          : left;
      if (this.entries[child].height >= tail.height) break;
      this.entries[cursor] = this.entries[child];
      cursor = child;
    }
    this.entries[cursor] = tail;
    return result;
  }
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

function hybridRidgedFractalNoise(x, y, seed, frequencies) {
  let value = 0;
  let totalWeight = 0;
  let ridgeWeight = 1;
  frequencies.forEach((frequency, octave) => {
    const amplitude = 1 / 2 ** octave;
    const noise = valueNoise(x * frequency, y * frequency, seed + octave * 191);
    const ridge = (1 - Math.abs(noise)) ** 2;
    value += (ridge * 2.35 - 0.98) * amplitude * ridgeWeight;
    totalWeight += amplitude;
    ridgeWeight = Math.max(0.22, Math.min(1, ridge * 1.85));
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
    fractalNoise(rx * 0.18 + 0.31, ry * 0.18 - 0.17, seed, [1, 2, 4]) * 0.38;
  const along = ry * 0.72 + warp;
  const across = rx * 0.96 + warp * 0.7;
  const massWarpX =
    fractalNoise(across * 0.34, along * 0.34, seed + 17, [1, 2, 4]) * 0.68;
  const massWarpY =
    fractalNoise(across * 0.31, along * 0.31, seed + 23, [1, 2, 4]) * 0.58;
  const massX = across + massWarpX;
  const massY = along + massWarpY;
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
  const sharpCrests = sharpRidgedFractalNoise(
    across * 0.9 - along * 0.24,
    along * 1.12,
    seed + 83,
    [2.1, 4.2, 8.4, 16.8, 33.6],
  );
  const mountainMass = hybridRidgedFractalNoise(
    massX,
    massY,
    seed + 89,
    [0.85, 1.7, 3.4, 6.8, 13.6, 27.2],
  );
  const crossMass = hybridRidgedFractalNoise(
    massX * 0.66 - massY * 0.48,
    massY * 0.74 + massX * 0.37,
    seed + 97,
    [1.3, 2.6, 5.2, 10.4, 20.8],
  );
  const ruggedMass = fractalNoise(
    massX + massY * 0.21,
    massY - massX * 0.17,
    seed + 93,
    [0.95, 1.9, 3.9, 8.1, 16.5, 33.3],
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
  const gain = 180 + control.roughness * 250;
  return (
    envelope *
    gain *
    (backbone * 0.08 +
      branches * 0.06 +
      sharpCrests * 0.08 +
      mountainMass * 0.16 +
      crossMass * 0.1 +
      ruggedMass * 0.76 -
      ravines * 0.12)
  );
}

function sharpRidgedFractalNoise(x, y, seed, frequencies) {
  let value = 0;
  let totalWeight = 0;
  frequencies.forEach((frequency, octave) => {
    const weight = 1 / 2 ** octave;
    const noise = valueNoise(x * frequency, y * frequency, seed + octave * 173);
    const ridge = 1 - Math.abs(noise);
    value += (ridge ** 2.4 * 2.7 - 0.72) * weight;
    totalWeight += weight;
  });
  return value / totalWeight;
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
