import {
  fieldCellCount,
  fieldPoint,
  scalarField,
  vectorField,
  type MaskField2D,
  type ScalarField2D,
  type VectorField2D,
  type FieldBounds,
} from "../engine/fields";
import { PROJECTED_SUPPORT } from "./elevation";

export const HYDROLOGY_PROPAGATION_STEPS = 4;
export const HYDROLOGY_ACCUMULATION_NORMALIZATION = 6;

export interface TerrainAtmosphereSample {
  x: number;
  y: number;
}

export interface HydrologyResult {
  rainfall: ScalarField2D;
  flow_direction: VectorField2D;
  flow_accumulation: ScalarField2D;
  erosion: ScalarField2D;
  elevation: ScalarField2D;
}

export function deriveHydrology(
  sourceId: string,
  elevation: ScalarField2D,
  validity: MaskField2D,
  atmosphere: readonly TerrainAtmosphereSample[],
  unresolvedPressure: number,
  absoluteBounds: FieldBounds,
  revisions: {
    rainfall: string;
    flow_direction: string;
    flow_accumulation: string;
    erosion: string;
    elevation: string;
  },
): HydrologyResult {
  const { grid } = elevation;
  const count = fieldCellCount(grid);
  const rainfallValues = new Float32Array(count);
  const hydraulicHeight = new Float32Array(count);
  let accumulationValues = new Float32Array(count);
  const flowValues = new Int8Array(count * 2);
  const downstream = new Int32Array(count);
  downstream.fill(-1);
  const drainageHash = stableHash(sourceId);
  const xDirection = drainageHash % 2 === 0 ? 1 : -1;
  const yDirection = (drainageHash >>> 1) % 2 === 0 ? 1 : -1;

  for (let row = 0; row < grid.rows; row += 1) {
    for (let column = 0; column < grid.columns; column += 1) {
      const index = row * grid.columns + column;
      if (validity.values[index] !== PROJECTED_SUPPORT) continue;
      const point = fieldPoint(grid, column, row);
      let atmosphericInput = 0;
      for (const sample of atmosphere) {
        const distanceSquared =
          (point.x - sample.x) ** 2 + (point.y - sample.y) ** 2;
        atmosphericInput += Math.exp(-distanceSquared / (2 * 210 * 210));
      }
      const rain =
        0.18 +
        elevation.values[index]! * 0.82 +
        atmosphericInput * 0.28 +
        unresolvedPressure * 0.12;
      const tilt =
        0.035 *
        (((point.x - absoluteBounds.x) / absoluteBounds.width) * xDirection +
          ((point.y - absoluteBounds.y) / absoluteBounds.height) * yDirection);
      rainfallValues[index] = rain;
      accumulationValues[index] = rain;
      hydraulicHeight[index] = elevation.values[index]! + tilt;
    }
  }

  for (let row = 0; row < grid.rows; row += 1) {
    for (let column = 0; column < grid.columns; column += 1) {
      const index = row * grid.columns + column;
      if (validity.values[index] !== PROJECTED_SUPPORT) continue;
      let selected = -1;
      let selectedHeight = hydraulicHeight[index]!;
      for (let rowOffset = -1; rowOffset <= 1; rowOffset += 1) {
        for (let columnOffset = -1; columnOffset <= 1; columnOffset += 1) {
          if (rowOffset === 0 && columnOffset === 0) continue;
          const nextColumn = column + columnOffset;
          const nextRow = row + rowOffset;
          if (
            nextColumn < 0 ||
            nextColumn >= grid.columns ||
            nextRow < 0 ||
            nextRow >= grid.rows
          )
            continue;
          const candidate = nextRow * grid.columns + nextColumn;
          if (
            validity.values[candidate] === PROJECTED_SUPPORT &&
            hydraulicHeight[candidate]! < selectedHeight
          ) {
            selected = candidate;
            selectedHeight = hydraulicHeight[candidate]!;
            flowValues[index * 2] = columnOffset;
            flowValues[index * 2 + 1] = rowOffset;
          }
        }
      }
      downstream[index] = selected;
    }
  }

  for (let step = 0; step < HYDROLOGY_PROPAGATION_STEPS; step += 1) {
    const propagated = rainfallValues.slice();
    for (let index = 0; index < count; index += 1) {
      if (validity.values[index] !== PROJECTED_SUPPORT) continue;
      const target = downstream[index]!;
      if (target >= 0)
        propagated[target] = propagated[target]! + accumulationValues[index]!;
    }
    accumulationValues = propagated;
  }

  const erosionValues = new Float32Array(count);
  const erodedValues = new Float32Array(count);
  for (let index = 0; index < count; index += 1) {
    if (validity.values[index] !== PROJECTED_SUPPORT) continue;
    const erosion =
      0.18 *
      Math.pow(
        Math.min(
          1,
          accumulationValues[index]! / HYDROLOGY_ACCUMULATION_NORMALIZATION,
        ),
        0.72,
      );
    erosionValues[index] = erosion;
    erodedValues[index] = Math.max(0, elevation.values[index]! - erosion);
  }

  return {
    rainfall: scalarField("rainfall", revisions.rainfall, grid, rainfallValues),
    flow_direction: vectorField(
      "flow_direction",
      revisions.flow_direction,
      grid,
      2,
      flowValues,
    ),
    flow_accumulation: scalarField(
      "flow_accumulation",
      revisions.flow_accumulation,
      grid,
      accumulationValues,
    ),
    erosion: scalarField("erosion", revisions.erosion, grid, erosionValues),
    elevation: scalarField(
      "elevation",
      revisions.elevation,
      grid,
      erodedValues,
    ),
  };
}

function stableHash(value: string): number {
  let hash = 2_166_136_261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16_777_619);
  }
  return hash >>> 0;
}
