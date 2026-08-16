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
  "rey.terrain.regional-geography@2" as const;
export const REGIONAL_TERRAIN_LINEWORK_REVISION =
  "rey.terrain.regional-linework@2" as const;

const DRAINAGE_EPSILON = 1e-7;
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
  hydraulic_height: Float32Array;
  receiver: Int32Array;
}

/**
 * Builds presentation geography only inside an already-admitted validity
 * field. The source DEM remains the authority; all added channels are derived
 * render inputs and never expand support.
 */
export function deriveRegionalTerrainGeography(
  source: TerrainFieldSet,
): TerrainFieldSet {
  if (!source.active_band_ids.includes("admitted_dem")) return source;
  const drainage = priorityFloodDrainage(source);
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
        drainage.hydraulic_height[right]! - drainage.hydraulic_height[left]! ||
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
    `${revision}:priority-flood-flow`,
    source.grid,
    2,
    directionValues,
  );
  const flowAccumulation = scalarField(
    "flow_accumulation",
    `${revision}:full-basin-accumulation`,
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
    relief.curvature,
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
      ]),
    ]),
    detail_authority: `${source.detail_authority}; depression-safe drainage, non-displacing erosion potential, and validity-bounded land cover are deterministic presentation derivations within admitted support; admitted elevation remains unchanged and the result is not observed hydrology or new geographic evidence`,
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
      0,
    ),
  });
}

export function deriveRegionalTerrainPresentationLines(
  field: TerrainFieldSet,
  regime: LensRegime,
): readonly TerrainLineFeatureInput[] {
  if (!field.active_band_ids.includes("derived_drainage"))
    return Object.freeze([]);
  const revision = `${REGIONAL_TERRAIN_LINEWORK_REVISION}:${field.field_set_id}:${regime}`;
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
      }),
    );
  }
  if (regime === "landscape") return Object.freeze(lines);
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
      }),
    );
  return Object.freeze(lines);
}

function priorityFloodDrainage(field: TerrainFieldSet): DrainageTopology {
  const cells = fieldCellCount(field.grid);
  const hydraulicHeight = new Float32Array(cells);
  const receiver = new Int32Array(cells);
  receiver.fill(-1);
  const visited = new Uint8Array(cells);
  const queue = new MinimumHeap();
  for (let row = 0; row < field.grid.rows; row += 1) {
    for (let column = 0; column < field.grid.columns; column += 1) {
      const index = row * field.grid.columns + column;
      if (
        field.validity.values[index] === 0 ||
        !isValidityBoundary(field, column, row)
      )
        continue;
      visited[index] = 1;
      hydraulicHeight[index] = field.elevation.values[index]!;
      queue.push(index, hydraulicHeight[index]!);
    }
  }
  while (queue.size > 0) {
    const current = queue.pop()!;
    const column = current.index % field.grid.columns;
    const row = Math.floor(current.index / field.grid.columns);
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
      if (field.validity.values[next] === 0 || visited[next] !== 0) continue;
      visited[next] = 1;
      receiver[next] = current.index;
      hydraulicHeight[next] = Math.max(
        field.elevation.values[next]!,
        current.height + DRAINAGE_EPSILON,
      );
      queue.push(next, hydraulicHeight[next]!);
    }
  }
  return { hydraulic_height: hydraulicHeight, receiver };
}

function isValidityBoundary(
  field: TerrainFieldSet,
  column: number,
  row: number,
): boolean {
  if (
    column === 0 ||
    row === 0 ||
    column === field.grid.columns - 1 ||
    row === field.grid.rows - 1
  )
    return true;
  return NEIGHBORS.some(([columnOffset, rowOffset]) => {
    const index =
      (row + rowOffset) * field.grid.columns + column + columnOffset;
    return field.validity.values[index] === 0;
  });
}

function deriveRegionalLandCover(
  source: TerrainFieldSet,
  elevation: TerrainFieldSet["elevation"],
  rainfall: TerrainFieldSet["rainfall"],
  accumulation: TerrainFieldSet["flow_accumulation"],
  normal: TerrainFieldSet["normal"],
  curvature: TerrainFieldSet["curvature"],
  seed: number,
  revision: string,
) {
  const cells = fieldCellCount(source.grid);
  const tint = new Float32Array(cells * 3);
  const occlusion = new Float32Array(cells);
  const roughness = new Float32Array(cells);
  const palette = {
    dry: [0.39, 0.37, 0.24] as const,
    grass: [0.3, 0.39, 0.23] as const,
    forest: [0.17, 0.29, 0.19] as const,
    alpine: [0.43, 0.43, 0.35] as const,
    rock: [0.39, 0.39, 0.35] as const,
  };
  for (let row = 0; row < source.grid.rows; row += 1) {
    for (let column = 0; column < source.grid.columns; column += 1) {
      const index = row * source.grid.columns + column;
      if (source.validity.values[index] === 0) continue;
      const height = elevation.values[index]!;
      const normalUp = normal.values[index * 3 + 2]!;
      const slope = 1 - normalUp;
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
        smootherstep((slope - 0.18) / 0.34) + alpine * 0.42,
        0,
        1,
      );
      const lowCover = mixColor(palette.dry, palette.grass, wetness);
      const vegetated = mixColor(lowCover, palette.forest, forest);
      const upland = mixColor(vegetated, palette.alpine, alpine);
      const color = mixColor(upland, palette.rock, exposedRock);
      for (let component = 0; component < 3; component += 1) {
        const sourceColor = source.material.tint[index * 3 + component]!;
        tint[index * 3 + component] =
          color[component]! * 0.88 + sourceColor * 0.12;
      }
      const valley = Math.max(0, curvature.values[index]!);
      occlusion[index] = clamp(
        0.9 - valley * 5.2 - accumulation.values[index]! * 0.08 - slope * 0.08,
        0.5,
        0.96,
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

class MinimumHeap {
  readonly entries: Array<{ index: number; height: number }> = [];

  get size() {
    return this.entries.length;
  }

  push(index: number, height: number) {
    const entry = { index, height };
    this.entries.push(entry);
    let position = this.entries.length - 1;
    while (position > 0) {
      const parent = Math.floor((position - 1) / 2);
      if (this.entries[parent]!.height <= height) break;
      this.entries[position] = this.entries[parent]!;
      position = parent;
    }
    this.entries[position] = entry;
  }

  pop() {
    const first = this.entries[0];
    const tail = this.entries.pop();
    if (!first || !tail || this.entries.length === 0) return first;
    let position = 0;
    while (true) {
      const left = position * 2 + 1;
      const right = left + 1;
      if (left >= this.entries.length) break;
      const child =
        right < this.entries.length &&
        this.entries[right]!.height < this.entries[left]!.height
          ? right
          : left;
      if (this.entries[child]!.height >= tail.height) break;
      this.entries[position] = this.entries[child]!;
      position = child;
    }
    this.entries[position] = tail;
    return first;
  }
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
