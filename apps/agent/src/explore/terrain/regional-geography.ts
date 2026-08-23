import type { TerrainLineFeatureInput } from "@rey/explorer";
import type { LensRegime } from "../engine/camera";
import {
  fieldByteLength,
  fieldCellCount,
  fieldPoint,
  materialField,
  scalarField,
  vectorField,
} from "../engine/fields";
import type { TerrainFieldSet } from "./compile";
import { regionalTerrainContourThresholds } from "./contours";
import { deriveTerrainNormals } from "./normals";

export const REGIONAL_TERRAIN_GEOGRAPHY_REVISION =
  "rey.terrain.regional-geography@8" as const;
export const REGIONAL_TERRAIN_LINEWORK_REVISION =
  "rey.terrain.regional-linework@5" as const;

const MINIMUM_DOWNHILL_DROP = 1e-6;
const MAXIMUM_CHANNEL_INCISION = 0.0045;
const STREAM_THRESHOLD = 0.58;
const RIVER_THRESHOLD = 0.78;

const NEIGHBORS = Object.freeze([
  [-1, -1],
  [0, -1],
  [1, -1],
  [-1, 0],
  [1, 0],
  [-1, 1],
  [0, 1],
  [1, 1],
] as const);

interface DrainageTopology {
  ordering_height: Float32Array;
  receiver: Int32Array;
}

const REGIONAL_TERRAIN_LINEWORK_PROFILES: Record<LensRegime, string> = {
  world: "none",
  atlas: "none",
  landscape: "contours-100m",
  neighborhoods: "contours-50m-drainage",
  objects: "contours-25m-drainage",
  evidence: "contours-25m-drainage",
};
const regionalTerrainLineworkCache = new WeakMap<
  TerrainFieldSet,
  Map<string, readonly TerrainLineFeatureInput[]>
>();

/**
 * Builds presentation geography only inside an already-admitted validity
 * field. The source DEM remains the authority; all added channels are derived
 * render inputs and never expand support.
 */
export function deriveRegionalTerrainGeography(
  source: TerrainFieldSet,
): TerrainFieldSet {
  if (!source.active_band_ids.includes("admitted_dem")) return source;
  const drainage = slopeSupportedDrainage(source);
  const cells = fieldCellCount(source.grid);
  const rainfallValues = new Float32Array(cells);
  const accumulationValues = new Float32Array(cells);
  const directionValues = new Float32Array(cells * 2);
  const seed = stableHash(source.source_revision);

  for (let row = 0; row < source.grid.rows; row += 1) {
    for (let column = 0; column < source.grid.columns; column += 1) {
      const index = row * source.grid.columns + column;
      if (source.validity.values[index] === 0) continue;
      const altitude = source.elevation.values[index]!;
      const continentality = column / Math.max(1, source.grid.columns - 1);
      const latitude = row / Math.max(1, source.grid.rows - 1);
      const climate = valueNoise(
        column / Math.max(12, source.grid.columns / 5),
        row / Math.max(12, source.grid.rows / 5),
        seed + 907,
      );
      const rainfall = clamp(
        0.36 +
          altitude * 0.34 +
          (1 - continentality) * 0.1 +
          Math.sin(latitude * Math.PI) * 0.08 +
          climate * 0.1,
        0.12,
        1,
      );
      rainfallValues[index] = rainfall;
      accumulationValues[index] = rainfall;
      const receiver = drainage.receiver[index]!;
      if (receiver >= 0) {
        const receiverColumn = receiver % source.grid.columns;
        const receiverRow = Math.floor(receiver / source.grid.columns);
        const length = Math.hypot(receiverColumn - column, receiverRow - row);
        directionValues[index * 2] = (receiverColumn - column) / length;
        directionValues[index * 2 + 1] = (receiverRow - row) / length;
      }
    }
  }

  const descending = Array.from({ length: cells }, (_, index) => index)
    .filter((index) => source.validity.values[index] !== 0)
    .sort(
      (left, right) =>
        drainage.ordering_height[right]! - drainage.ordering_height[left]! ||
        right - left,
    );
  for (const index of descending) {
    const receiver = drainage.receiver[index]!;
    if (receiver >= 0)
      accumulationValues[receiver] =
        accumulationValues[receiver]! + accumulationValues[index]!;
  }
  const maximumAccumulation = descending.reduce(
    (maximum, index) => Math.max(maximum, accumulationValues[index]!),
    1,
  );
  const accumulationDenominator = Math.log1p(maximumAccumulation);
  for (const index of descending)
    accumulationValues[index] =
      Math.log1p(accumulationValues[index]!) / accumulationDenominator;

  const erosionValues = new Float32Array(cells);
  const landCoverAccumulationValues = smoothScalarWithinValidity(
    accumulationValues,
    source,
    2,
  );
  for (const index of descending) {
    const drainageStrength = smootherstep(
      (landCoverAccumulationValues[index]! - 0.55) / 0.35,
    );
    erosionValues[index] = drainageStrength * MAXIMUM_CHANNEL_INCISION;
  }

  const revision = `${REGIONAL_TERRAIN_GEOGRAPHY_REVISION}:${source.source_revision}:${source.grid.columns}x${source.grid.rows}`;
  const elevation = scalarField(
    "elevation",
    `${revision}:admitted-elevation-preserved`,
    source.grid,
    source.elevation.values.slice(),
  );
  const relief = deriveTerrainNormals(
    elevation,
    source.validity,
    source.elevation_scale,
    {
      normal: `${revision}:normal`,
      curvature: `${revision}:curvature`,
    },
  );
  const rainfall = scalarField(
    "rainfall",
    `${revision}:derived-rainfall`,
    source.grid,
    rainfallValues,
  );
  const flowDirection = vectorField(
    "flow_direction",
    `${revision}:genuine-downhill-flow`,
    source.grid,
    2,
    directionValues,
  );
  const flowAccumulation = scalarField(
    "flow_accumulation",
    `${revision}:slope-supported-accumulation`,
    source.grid,
    accumulationValues,
  );
  const erosion = scalarField(
    "erosion",
    `${revision}:non-displacing-erosion-potential`,
    source.grid,
    erosionValues,
  );
  const landCoverAccumulation = scalarField(
    "flow_accumulation",
    `${revision}:validity-bounded-land-cover-accumulation`,
    source.grid,
    landCoverAccumulationValues,
  );
  const material = deriveRegionalLandCover(
    source,
    elevation,
    rainfall,
    landCoverAccumulation,
    relief.normal,
    seed,
    `${revision}:land-cover`,
  );
  const fields = [
    source.validity,
    elevation,
    rainfall,
    flowDirection,
    flowAccumulation,
    erosion,
    relief.normal,
    relief.curvature,
    material,
  ] as const;
  return Object.freeze({
    ...source,
    field_set_id: `${source.field_set_id}|${revision}`,
    working_set_id: `geography:${source.working_set_id}`,
    active_band_ids: Object.freeze([
      ...new Set([
        ...source.active_band_ids,
        "derived_drainage",
        "derived_land_cover",
        "derived_continuous_hypsometry",
        "derived_multiscale_relief",
      ]),
    ]),
    detail_authority: `${source.detail_authority}; genuine-downhill drainage with retained sinks, non-displacing erosion potential, validity-bounded continuous hypsometry, and slope-triggered rock exposure are deterministic presentation derivations within admitted support; admitted elevation remains unchanged and the result is not observed hydrology, land cover, or new geographic evidence`,
    elevation,
    rainfall,
    flow_direction: flowDirection,
    flow_accumulation: flowAccumulation,
    erosion,
    normal: relief.normal,
    curvature: relief.curvature,
    material,
    field_bytes: fields.reduce(
      (total, field) => total + fieldByteLength(field),
      source.validity_classification?.values.byteLength ?? 0,
    ),
  });
}

export function deriveRegionalTerrainPresentationLines(
  field: TerrainFieldSet,
  regime: LensRegime,
): readonly TerrainLineFeatureInput[] {
  if (!field.active_band_ids.includes("derived_drainage"))
    return Object.freeze([]);
  const profile = REGIONAL_TERRAIN_LINEWORK_PROFILES[regime];
  let byProfile = regionalTerrainLineworkCache.get(field);
  if (!byProfile) {
    byProfile = new Map();
    regionalTerrainLineworkCache.set(field, byProfile);
  }
  const retained = byProfile.get(profile);
  if (retained) return retained;
  const revision = `${REGIONAL_TERRAIN_LINEWORK_REVISION}:${field.field_set_id}:${profile}`;
  const lines: TerrainLineFeatureInput[] = [];
  for (const [index, threshold] of regionalTerrainContourThresholds(
    field,
    regime,
  ).entries()) {
    const positions = contourSegments(field, threshold, 1.55);
    if (positions.length === 0) continue;
    lines.push(
      Object.freeze({
        id: `${revision}:contour:${index}`,
        pass_id: "contours",
        kind: "derived_contour",
        source_revision: `${revision}:${threshold}`,
        authority:
          "metric contour derived from the drainage-conditioned presentation elevation inside admitted validity",
        positions,
        color: 0x595848,
        opacity: index % 5 === 0 ? 0.32 : 0.2,
        width: index % 5 === 0 ? 1.25 : 0.65,
      }),
    );
  }
  if (regime === "landscape") {
    const result = Object.freeze(lines);
    byProfile.set(profile, result);
    return result;
  }
  const streams = drainageSegments(
    field,
    STREAM_THRESHOLD,
    RIVER_THRESHOLD,
    1.9,
  );
  if (streams.length > 0)
    lines.push(
      Object.freeze({
        id: `${revision}:streams`,
        pass_id: "water_weather_boundary",
        kind: "derived_stream",
        source_revision: `${revision}:flow-accumulation:${STREAM_THRESHOLD}:${RIVER_THRESHOLD}`,
        authority:
          "synthetic stream hierarchy derived from presentation elevation within admitted validity; not admitted water evidence",
        positions: streams,
        color: 0x577d76,
        opacity: 0.2,
        width: 0.9,
      }),
    );
  const rivers = drainageSegments(field, RIVER_THRESHOLD, 1, 2.15);
  if (rivers.length > 0)
    lines.push(
      Object.freeze({
        id: `${revision}:rivers`,
        pass_id: "water_weather_boundary",
        kind: "derived_river",
        source_revision: `${revision}:flow-accumulation:${RIVER_THRESHOLD}`,
        authority:
          "synthetic main-stem drainage derived from presentation elevation within admitted validity; not admitted water evidence",
        positions: rivers,
        color: 0x416d74,
        opacity: 0.5,
        width: 1.45,
      }),
    );
  const result = Object.freeze(lines);
  byProfile.set(profile, result);
  return result;
}

function slopeSupportedDrainage(field: TerrainFieldSet): DrainageTopology {
  const cells = fieldCellCount(field.grid);
  const orderingHeight = new Float32Array(cells);
  const receiver = new Int32Array(cells);
  receiver.fill(-1);
  for (let row = 0; row < field.grid.rows; row += 1) {
    for (let column = 0; column < field.grid.columns; column += 1) {
      const index = row * field.grid.columns + column;
      if (field.validity.values[index] === 0) continue;
      const height = field.elevation.values[index]!;
      orderingHeight[index] = height;
      let steepestReceiver = -1;
      let steepestSlope = 0;
      for (const [columnOffset, rowOffset] of NEIGHBORS) {
        const nextColumn = column + columnOffset;
        const nextRow = row + rowOffset;
        if (
          nextColumn < 0 ||
          nextColumn >= field.grid.columns ||
          nextRow < 0 ||
          nextRow >= field.grid.rows
        )
          continue;
        const next = nextRow * field.grid.columns + nextColumn;
        if (field.validity.values[next] === 0) continue;
        const drop = height - field.elevation.values[next]!;
        if (drop <= MINIMUM_DOWNHILL_DROP) continue;
        const slope = drop / Math.hypot(columnOffset, rowOffset);
        if (
          slope > steepestSlope ||
          (slope === steepestSlope && next < steepestReceiver)
        ) {
          steepestSlope = slope;
          steepestReceiver = next;
        }
      }
      receiver[index] = steepestReceiver;
    }
  }
  return { ordering_height: orderingHeight, receiver };
}

function deriveRegionalLandCover(
  source: TerrainFieldSet,
  elevation: TerrainFieldSet["elevation"],
  rainfall: TerrainFieldSet["rainfall"],
  accumulation: TerrainFieldSet["flow_accumulation"],
  normal: TerrainFieldSet["normal"],
  seed: number,
  revision: string,
) {
  const cells = fieldCellCount(source.grid);
  const tint = new Float32Array(cells * 3);
  const occlusion = new Float32Array(cells);
  const roughness = new Float32Array(cells);
  const palette = {
    valley: [0.24, 0.44, 0.2] as const,
    montane: [0.52, 0.47, 0.24] as const,
    subalpine: [0.38, 0.46, 0.28] as const,
    alpine: [0.38, 0.41, 0.46] as const,
    forest: [0.16, 0.35, 0.14] as const,
    rock: [0.38, 0.36, 0.34] as const,
  };
  for (let row = 0; row < source.grid.rows; row += 1) {
    for (let column = 0; column < source.grid.columns; column += 1) {
      const index = row * source.grid.columns + column;
      if (source.validity.values[index] === 0) continue;
      const height = elevation.values[index]!;
      const normalUp = normal.values[index * 3 + 2]!;
      const slopeRadians = Math.acos(clamp(normalUp, -1, 1));
      const slope = clamp(slopeRadians / (Math.PI / 2), 0, 1);
      const wetness = clamp(
        rainfall.values[index]! * 0.62 + accumulation.values[index]! * 0.5,
        0,
        1,
      );
      const coverNoise = valueNoise(
        column / Math.max(8, source.grid.columns / 18),
        row / Math.max(8, source.grid.rows / 18),
        seed + 1301,
      );
      const treeLine = smootherstep((0.76 - height) / 0.2);
      const forest = clamp(
        treeLine *
          smootherstep((wetness - 0.34) / 0.34) *
          (0.78 + coverNoise * 0.22),
        0,
        1,
      );
      const alpine = smootherstep((height - 0.66) / 0.22);
      const exposedRock = clamp(
        smootherstep((slope - 0.38) / 0.32) + alpine * 0.28,
        0,
        1,
      );
      const valleyToMontane = smootherstep((height - 0.14) / 0.4);
      const montaneToSubalpine = smootherstep((height - 0.48) / 0.2);
      const subalpineToAlpine = smootherstep((height - 0.7) / 0.22);
      const valley = mixColor(
        palette.montane,
        palette.valley,
        clamp(wetness * 0.8 + 0.2, 0, 1),
      );
      const lower = mixColor(valley, palette.montane, valleyToMontane);
      const middle = mixColor(lower, palette.subalpine, montaneToSubalpine);
      const hypsometric = mixColor(middle, palette.alpine, subalpineToAlpine);
      const vegetated = mixColor(hypsometric, palette.forest, forest * 0.8);
      const upland = mixColor(vegetated, palette.alpine, alpine * 0.54);
      const color = mixColor(upland, palette.rock, exposedRock);
      for (let component = 0; component < 3; component += 1) {
        const sourceColor = source.material.tint[index * 3 + component]!;
        tint[index * 3 + component] =
          color[component]! * 0.94 + sourceColor * 0.06;
      }
      occlusion[index] = clamp(
        0.9 + forest * 0.06 - wetness * 0.05 - exposedRock * 0.02,
        0.82,
        0.98,
      );
      roughness[index] = clamp(
        0.78 + forest * 0.14 + exposedRock * 0.08 + wetness * 0.04,
        0.72,
        1,
      );
    }
  }
  return materialField(
    "material",
    revision,
    source.grid,
    tint,
    occlusion,
    roughness,
  );
}

function contourSegments(
  field: TerrainFieldSet,
  threshold: number,
  offset: number,
): Float32Array {
  const positions: number[] = [];
  const crossing = (first: number, second: number) => {
    const firstValue = field.elevation.values[first]!;
    const secondValue = field.elevation.values[second]!;
    const amount =
      secondValue === firstValue
        ? 0.5
        : (threshold - firstValue) / (secondValue - firstValue);
    const firstPoint = fieldPoint(
      field.grid,
      first % field.grid.columns,
      Math.floor(first / field.grid.columns),
    );
    const secondPoint = fieldPoint(
      field.grid,
      second % field.grid.columns,
      Math.floor(second / field.grid.columns),
    );
    return {
      x: mix(firstPoint.x, secondPoint.x, amount),
      y: mix(firstPoint.y, secondPoint.y, amount),
    };
  };
  const append = (
    first: { x: number; y: number },
    second: { x: number; y: number },
  ) => {
    const height = threshold * field.elevation_scale + offset;
    positions.push(first.x, height, first.y, second.x, height, second.y);
  };
  for (let row = 0; row < field.grid.rows - 1; row += 1) {
    for (let column = 0; column < field.grid.columns - 1; column += 1) {
      const topLeft = row * field.grid.columns + column;
      const topRight = topLeft + 1;
      const bottomLeft = topLeft + field.grid.columns;
      const bottomRight = bottomLeft + 1;
      const corners = [topLeft, topRight, bottomRight, bottomLeft] as const;
      if (corners.some((index) => field.validity.values[index] === 0)) continue;
      const edges = [
        [topLeft, topRight],
        [topRight, bottomRight],
        [bottomRight, bottomLeft],
        [bottomLeft, topLeft],
      ] as const;
      const crossings = edges.flatMap(([first, second], edge) =>
        field.elevation.values[first]! >= threshold !==
        field.elevation.values[second]! >= threshold
          ? [{ edge, point: crossing(first, second) }]
          : [],
      );
      if (crossings.length === 2)
        append(crossings[0]!.point, crossings[1]!.point);
      else if (crossings.length === 4) {
        const center =
          corners.reduce(
            (total, index) => total + field.elevation.values[index]!,
            0,
          ) / 4;
        const pairs: ReadonlyArray<readonly [number, number]> =
          center >= threshold
            ? [
                [0, 3],
                [1, 2],
              ]
            : [
                [0, 1],
                [2, 3],
              ];
        for (const [first, second] of pairs)
          append(crossings[first]!.point, crossings[second]!.point);
      }
    }
  }
  return Float32Array.from(positions);
}

function drainageSegments(
  field: TerrainFieldSet,
  minimum: number,
  maximum: number,
  offset: number,
): Float32Array {
  const receiver = new Int32Array(field.field_cells);
  receiver.fill(-1);
  const admitted = new Uint8Array(field.field_cells);
  for (let index = 0; index < field.field_cells; index += 1) {
    const strength = field.flow_accumulation.values[index]!;
    if (
      field.validity.values[index] === 0 ||
      strength < minimum ||
      strength >= maximum
    )
      continue;
    admitted[index] = 1;
    const column = index % field.grid.columns;
    const row = Math.floor(index / field.grid.columns);
    const columnOffset = Math.sign(field.flow_direction.values[index * 2]!);
    const rowOffset = Math.sign(field.flow_direction.values[index * 2 + 1]!);
    if (columnOffset === 0 && rowOffset === 0) continue;
    const nextColumn = column + columnOffset;
    const nextRow = row + rowOffset;
    if (
      nextColumn < 0 ||
      nextColumn >= field.grid.columns ||
      nextRow < 0 ||
      nextRow >= field.grid.rows
    )
      continue;
    const next = nextRow * field.grid.columns + nextColumn;
    if (field.validity.values[next] === 0) continue;
    receiver[index] = next;
  }
  const indegree = new Uint16Array(field.field_cells);
  for (let index = 0; index < field.field_cells; index += 1) {
    const next = receiver[index]!;
    if (admitted[index] !== 0 && next >= 0 && admitted[next] !== 0)
      indegree[next] = indegree[next]! + 1;
  }
  const visited = new Uint8Array(field.field_cells);
  const positions: number[] = [];
  const point = (index: number) => {
    const gridPoint = fieldPoint(
      field.grid,
      index % field.grid.columns,
      Math.floor(index / field.grid.columns),
    );
    return {
      x: gridPoint.x,
      y: field.elevation.values[index]! * field.elevation_scale + offset,
      z: gridPoint.y,
    };
  };
  const appendTrace = (start: number) => {
    const trace = [start];
    let current = start;
    while (receiver[current]! >= 0 && visited[current] === 0) {
      visited[current] = 1;
      const next = receiver[current]!;
      trace.push(next);
      if (admitted[next] === 0 || indegree[next] !== 1) break;
      current = next;
    }
    const smoothed = smoothDrainageTrace(trace.map(point));
    for (let index = 1; index < smoothed.length; index += 1) {
      const first = smoothed[index - 1]!;
      const second = smoothed[index]!;
      positions.push(first.x, first.y, first.z, second.x, second.y, second.z);
    }
  };
  for (let index = 0; index < field.field_cells; index += 1)
    if (admitted[index] !== 0 && indegree[index] !== 1) appendTrace(index);
  for (let index = 0; index < field.field_cells; index += 1)
    if (admitted[index] !== 0 && visited[index] === 0) appendTrace(index);
  return Float32Array.from(positions);
}

function smoothDrainageTrace(
  source: ReadonlyArray<{ x: number; y: number; z: number }>,
) {
  let points = source;
  for (let pass = 0; pass < 2 && points.length > 2; pass += 1) {
    const next = [points[0]!];
    for (let index = 1; index < points.length; index += 1) {
      const first = points[index - 1]!;
      const second = points[index]!;
      next.push(
        {
          x: mix(first.x, second.x, 0.25),
          y: mix(first.y, second.y, 0.25),
          z: mix(first.z, second.z, 0.25),
        },
        {
          x: mix(first.x, second.x, 0.75),
          y: mix(first.y, second.y, 0.75),
          z: mix(first.z, second.z, 0.75),
        },
      );
    }
    next[0] = points[0]!;
    next[next.length - 1] = points[points.length - 1]!;
    points = next;
  }
  return points;
}

function smoothScalarWithinValidity(
  source: Float32Array,
  field: TerrainFieldSet,
  passes: number,
): Float32Array {
  let values = source.slice();
  const kernel = [1, 2, 1] as const;
  for (let pass = 0; pass < passes; pass += 1) {
    const next = new Float32Array(values.length);
    for (let row = 0; row < field.grid.rows; row += 1) {
      for (let column = 0; column < field.grid.columns; column += 1) {
        const index = row * field.grid.columns + column;
        if (field.validity.values[index] === 0) continue;
        let weighted = 0;
        let weight = 0;
        for (let rowOffset = -1; rowOffset <= 1; rowOffset += 1) {
          for (let columnOffset = -1; columnOffset <= 1; columnOffset += 1) {
            const neighborColumn = column + columnOffset;
            const neighborRow = row + rowOffset;
            if (
              neighborColumn < 0 ||
              neighborColumn >= field.grid.columns ||
              neighborRow < 0 ||
              neighborRow >= field.grid.rows
            )
              continue;
            const neighbor = neighborRow * field.grid.columns + neighborColumn;
            if (field.validity.values[neighbor] === 0) continue;
            const neighborWeight =
              kernel[columnOffset + 1]! * kernel[rowOffset + 1]!;
            weighted += values[neighbor]! * neighborWeight;
            weight += neighborWeight;
          }
        }
        next[index] = weight === 0 ? values[index]! : weighted / weight;
      }
    }
    values = next;
  }
  return values;
}

function valueNoise(x: number, y: number, seed: number): number {
  const column = Math.floor(x);
  const row = Math.floor(y);
  const amountX = smootherstep(x - column);
  const amountY = smootherstep(y - row);
  return mix(
    mix(
      signedHash(column, row, seed),
      signedHash(column + 1, row, seed),
      amountX,
    ),
    mix(
      signedHash(column, row + 1, seed),
      signedHash(column + 1, row + 1, seed),
      amountX,
    ),
    amountY,
  );
}

function signedHash(x: number, y: number, seed: number): number {
  let value = Math.imul(x, 374_761_393);
  value = Math.imul(value ^ Math.imul(y, 668_265_263), 1_274_126_177);
  value = Math.imul(value ^ seed, 2_246_822_519);
  value ^= value >>> 13;
  return (value >>> 0) / 2_147_483_647.5 - 1;
}

function stableHash(value: string): number {
  let hash = 2_166_136_261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16_777_619);
  }
  return hash | 0;
}

function mixColor(
  left: readonly [number, number, number],
  right: readonly [number, number, number],
  amount: number,
): readonly [number, number, number] {
  const bounded = clamp(amount, 0, 1);
  return [
    mix(left[0], right[0], bounded),
    mix(left[1], right[1], bounded),
    mix(left[2], right[2], bounded),
  ];
}

function smootherstep(value: number): number {
  const bounded = clamp(value, 0, 1);
  return bounded * bounded * bounded * (bounded * (bounded * 6 - 15) + 10);
}

function mix(left: number, right: number, amount: number) {
  return left + (right - left) * amount;
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.max(minimum, Math.min(maximum, value));
}
