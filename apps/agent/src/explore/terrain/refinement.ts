import {
  createTerrainValidityClassification,
  TERRAIN_VALIDITY_NO_DATA,
  TERRAIN_VALIDITY_UNSUPPORTED,
  TERRAIN_VALIDITY_VALID,
  verifyTerrainFieldValidityClassification,
} from "@rey/explorer";
import {
  TERRAIN_FIELD_SCHEMA,
  createFieldGrid,
  fieldByteLength,
  fieldCellCount,
  maskField,
  materialField,
  scalarField,
  vectorField,
} from "../engine/fields";
import type { TerrainFieldSet } from "./compile";
import { deriveTerrainNormals } from "./normals";

export const REGIONAL_TERRAIN_REFINEMENT_REVISION =
  "rey.terrain.validity-safe-refinement@3" as const;
export const REGIONAL_TERRAIN_PRESENTATION_INTERVALS = 320;
const MAXIMUM_REFINEMENT_FACTOR = 4;
const MICRO_RELIEF_AMPLITUDE = 0.007;

interface SampledTerrain {
  elevation: number;
  rainfall: number;
  flow_direction: readonly [number, number];
  flow_accumulation: number;
  erosion: number;
  tint: readonly [number, number, number];
  occlusion: number;
  roughness: number;
}

export function regionalTerrainRefinementFactor(
  field: TerrainFieldSet,
): number {
  const intervals = Math.max(field.grid.columns - 1, field.grid.rows - 1);
  if (intervals < 32) return 1;
  return Math.max(
    1,
    Math.min(
      MAXIMUM_REFINEMENT_FACTOR,
      Math.ceil(REGIONAL_TERRAIN_PRESENTATION_INTERVALS / intervals),
    ),
  );
}

export function refineRegionalTerrainField(
  source: TerrainFieldSet,
  factor = regionalTerrainRefinementFactor(source),
): TerrainFieldSet {
  if (!Number.isInteger(factor) || factor < 1 || factor > 8)
    throw new Error("regional terrain refinement factor is invalid");
  if (factor === 1) return source;
  const grid = createFieldGrid(
    (source.grid.columns - 1) * factor + 1,
    (source.grid.rows - 1) * factor + 1,
    source.grid.bounds,
  );
  const cells = fieldCellCount(grid);
  const validityValues = new Uint8Array(cells);
  const validityClassificationValues = new Uint8Array(cells);
  const sourceValidityClassification =
    verifyTerrainFieldValidityClassification(source);
  const elevationValues = new Float32Array(cells);
  const rainfallValues = new Float32Array(cells);
  const flowDirectionValues = new Float32Array(cells * 2);
  const flowAccumulationValues = new Float32Array(cells);
  const erosionValues = new Float32Array(cells);
  const tintValues = new Float32Array(cells * 3);
  const occlusionValues = new Float32Array(cells);
  const roughnessValues = new Float32Array(cells);
  const seed = revisionSeed(source.source_revision);

  for (let row = 0; row < grid.rows; row += 1) {
    const sourceY = row / factor;
    for (let column = 0; column < grid.columns; column += 1) {
      const sourceX = column / factor;
      const index = row * grid.columns + column;
      const sample = sampleSupportedTerrain(source, sourceX, sourceY);
      if (!sample) {
        validityClassificationValues[index] = invalidRefinedValidityClass(
          source,
          sourceValidityClassification.values,
          sourceX,
          sourceY,
        );
        continue;
      }
      validityValues[index] = 1;
      validityClassificationValues[index] = TERRAIN_VALIDITY_VALID;
      elevationValues[index] = clamp01(
        sample.elevation +
          constrainedMicroRelief(sourceX, sourceY, seed) *
            MICRO_RELIEF_AMPLITUDE,
      );
      rainfallValues[index] = sample.rainfall;
      flowDirectionValues[index * 2] = sample.flow_direction[0];
      flowDirectionValues[index * 2 + 1] = sample.flow_direction[1];
      flowAccumulationValues[index] = sample.flow_accumulation;
      erosionValues[index] = sample.erosion;
      tintValues[index * 3] = sample.tint[0];
      tintValues[index * 3 + 1] = sample.tint[1];
      tintValues[index * 3 + 2] = sample.tint[2];
      occlusionValues[index] = sample.occlusion;
      roughnessValues[index] = sample.roughness;
    }
  }

  const revision = `${REGIONAL_TERRAIN_REFINEMENT_REVISION}:${source.source_revision}:${factor}`;
  const validity = maskField(
    "validity",
    `${revision}:validity`,
    grid,
    validityValues,
  );
  const validityClassification = createTerrainValidityClassification(
    validityClassificationValues,
    `${revision}:validity-classification`,
  );
  const elevation = scalarField(
    "elevation",
    `${revision}:elevation`,
    grid,
    elevationValues,
  );
  const relief = deriveTerrainNormals(
    elevation,
    validity,
    source.elevation_scale,
    {
      normal: `${revision}:normal`,
      curvature: `${revision}:curvature`,
    },
  );
  const rainfall = scalarField(
    source.rainfall.channel,
    `${revision}:rainfall`,
    grid,
    rainfallValues,
  );
  const flowDirection = vectorField(
    source.flow_direction.channel,
    `${revision}:flow-direction`,
    grid,
    2,
    flowDirectionValues,
  );
  const flowAccumulation = scalarField(
    source.flow_accumulation.channel,
    `${revision}:flow-accumulation`,
    grid,
    flowAccumulationValues,
  );
  const erosion = scalarField(
    source.erosion.channel,
    `${revision}:erosion`,
    grid,
    erosionValues,
  );
  const material = materialField(
    source.material.channel,
    `${revision}:material`,
    grid,
    tintValues,
    occlusionValues,
    roughnessValues,
  );
  const fields = [
    validity,
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
    schema: TERRAIN_FIELD_SCHEMA,
    field_set_id: `${source.field_set_id}|${revision}|${grid.columns}x${grid.rows}`,
    program_id: source.program_id,
    working_set_id: `refined:${source.working_set_id}:${factor}`,
    active_band_ids: Object.freeze([
      ...source.active_band_ids,
      "presentation_microrelief",
    ]),
    detail_authority: `${source.detail_authority}; validity-safe bilinear refinement inside fully supported cells and triangular refinement along explicit support boundaries, with deterministic band-limited presentation-only microrelief constrained to zero at every admitted source vertex; refined displacement is not observed or authored elevation`,
    source_revision: source.source_revision,
    source_summary: source.source_summary,
    grid,
    elevation_scale: source.elevation_scale,
    validity,
    validity_classification: validityClassification,
    elevation,
    rainfall,
    flow_direction: flowDirection,
    flow_accumulation: flowAccumulation,
    erosion,
    normal: relief.normal,
    curvature: relief.curvature,
    material,
    relief_metrics: source.relief_metrics
      ? Object.freeze({
          ...source.relief_metrics,
          sample_spacing_x_meters:
            source.relief_metrics.sample_spacing_x_meters / factor,
          sample_spacing_y_meters:
            source.relief_metrics.sample_spacing_y_meters / factor,
          authority: `${source.relief_metrics.authority}; refinement spacing includes disclosed presentation-only interpolation and microrelief`,
        })
      : undefined,
    field_cells: cells,
    field_bytes: fields.reduce(
      (total, field) => total + fieldByteLength(field),
      validityClassification.values.byteLength,
    ),
    landscape_mosaic: source.landscape_mosaic,
  });
}

function invalidRefinedValidityClass(
  field: TerrainFieldSet,
  classifications: Uint8Array,
  x: number,
  y: number,
): typeof TERRAIN_VALIDITY_NO_DATA | typeof TERRAIN_VALIDITY_UNSUPPORTED {
  const roundedColumn = Math.round(x);
  const roundedRow = Math.round(y);
  if (Math.abs(x - roundedColumn) < 1e-9 && Math.abs(y - roundedRow) < 1e-9) {
    const exact =
      classifications[roundedRow * field.grid.columns + roundedColumn];
    return exact === TERRAIN_VALIDITY_UNSUPPORTED
      ? TERRAIN_VALIDITY_UNSUPPORTED
      : TERRAIN_VALIDITY_NO_DATA;
  }
  const left = Math.max(0, Math.min(field.grid.columns - 1, Math.floor(x)));
  const right = Math.max(0, Math.min(field.grid.columns - 1, Math.ceil(x)));
  const top = Math.max(0, Math.min(field.grid.rows - 1, Math.floor(y)));
  const bottom = Math.max(0, Math.min(field.grid.rows - 1, Math.ceil(y)));
  const neighborhood = [
    top * field.grid.columns + left,
    top * field.grid.columns + right,
    bottom * field.grid.columns + left,
    bottom * field.grid.columns + right,
  ];
  return neighborhood.some(
    (index) => classifications[index] === TERRAIN_VALIDITY_UNSUPPORTED,
  )
    ? TERRAIN_VALIDITY_UNSUPPORTED
    : TERRAIN_VALIDITY_NO_DATA;
}

function sampleSupportedTerrain(
  field: TerrainFieldSet,
  x: number,
  y: number,
): SampledTerrain | null {
  const maximumX = field.grid.columns - 1;
  const maximumY = field.grid.rows - 1;
  const column = Math.min(maximumX - 1, Math.max(0, Math.floor(x)));
  const row = Math.min(maximumY - 1, Math.max(0, Math.floor(y)));
  const localX = Math.max(0, Math.min(1, x - column));
  const localY = Math.max(0, Math.min(1, y - row));
  const topLeft = row * field.grid.columns + column;
  const topRight = topLeft + 1;
  const bottomLeft = topLeft + field.grid.columns;
  const bottomRight = bottomLeft + 1;
  const supportedCorners = [
    topLeft,
    topRight,
    bottomLeft,
    bottomRight,
  ] as const;
  if (supportedCorners.every((index) => field.validity.values[index] !== 0)) {
    return interpolateTerrainSample(field, supportedCorners, [
      (1 - localX) * (1 - localY),
      localX * (1 - localY),
      (1 - localX) * localY,
      localX * localY,
    ]);
  }
  const descending = [
    [topLeft, bottomLeft, bottomRight],
    [topLeft, bottomRight, topRight],
  ] as const;
  const ascending = [
    [topLeft, bottomLeft, topRight],
    [topRight, bottomLeft, bottomRight],
  ] as const;
  const score = (triangles: typeof descending) =>
    triangles.filter((triangle) =>
      triangle.every((index) => field.validity.values[index] !== 0),
    ).length;
  const descendingScore = score(descending);
  const ascendingScore = score(ascending);
  const triangles =
    descendingScore === ascendingScore
      ? (row + column) % 2 === 0
        ? descending
        : ascending
      : descendingScore > ascendingScore
        ? descending
        : ascending;
  const coordinates = new Map<number, readonly [number, number]>([
    [topLeft, [0, 0]],
    [topRight, [1, 0]],
    [bottomLeft, [0, 1]],
    [bottomRight, [1, 1]],
  ]);
  for (const triangle of triangles) {
    if (triangle.some((index) => field.validity.values[index] === 0)) continue;
    const weights = barycentricWeights(
      [localX, localY],
      coordinates.get(triangle[0])!,
      coordinates.get(triangle[1])!,
      coordinates.get(triangle[2])!,
    );
    if (!weights || weights.some((weight) => weight < -1e-7)) continue;
    return interpolateTerrainSample(field, triangle, weights);
  }
  return null;
}

function interpolateTerrainSample(
  field: TerrainFieldSet,
  indices: readonly number[],
  weights: readonly number[],
): SampledTerrain {
  const scalar = (values: Float32Array) =>
    indices.reduce(
      (total, index, vertex) => total + values[index]! * weights[vertex]!,
      0,
    );
  const component = (values: Float32Array, components: number, part: number) =>
    indices.reduce(
      (total, index, vertex) =>
        total + values[index * components + part]! * weights[vertex]!,
      0,
    );
  return {
    elevation: scalar(field.elevation.values),
    rainfall: scalar(field.rainfall.values),
    flow_direction: [
      component(field.flow_direction.values as Float32Array, 2, 0),
      component(field.flow_direction.values as Float32Array, 2, 1),
    ],
    flow_accumulation: scalar(field.flow_accumulation.values),
    erosion: scalar(field.erosion.values),
    tint: [
      component(field.material.tint, 3, 0),
      component(field.material.tint, 3, 1),
      component(field.material.tint, 3, 2),
    ],
    occlusion: scalar(field.material.occlusion),
    roughness: scalar(field.material.roughness),
  };
}

function barycentricWeights(
  point: readonly [number, number],
  a: readonly [number, number],
  b: readonly [number, number],
  c: readonly [number, number],
): readonly [number, number, number] | null {
  const denominator =
    (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
  if (Math.abs(denominator) < 1e-12) return null;
  const first =
    ((b[1] - c[1]) * (point[0] - c[0]) + (c[0] - b[0]) * (point[1] - c[1])) /
    denominator;
  const second =
    ((c[1] - a[1]) * (point[0] - c[0]) + (a[0] - c[0]) * (point[1] - c[1])) /
    denominator;
  return [first, second, 1 - first - second];
}

function constrainedMicroRelief(x: number, y: number, seed: number): number {
  const column = Math.floor(x);
  const row = Math.floor(y);
  const localX = x - column;
  const localY = y - row;
  const sample = microRelief(x, y, seed);
  const north = interpolate(
    microRelief(column, row, seed),
    microRelief(column + 1, row, seed),
    localX,
  );
  const south = interpolate(
    microRelief(column, row + 1, seed),
    microRelief(column + 1, row + 1, seed),
    localX,
  );
  return sample - interpolate(north, south, localY);
}

function microRelief(x: number, y: number, seed: number): number {
  return (
    valueNoise(x * 0.42, y * 0.42, seed + 101) * 0.55 +
    valueNoise(x * 0.84, y * 0.84, seed + 211) * 0.3 +
    valueNoise(x * 1.35, y * 1.35, seed + 307) * 0.15
  );
}

function valueNoise(x: number, y: number, seed: number): number {
  const column = Math.floor(x);
  const row = Math.floor(y);
  const amountX = smoothstep(x - column);
  const amountY = smoothstep(y - row);
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

function signedHash(x: number, y: number, seed: number): number {
  let value = Math.imul(x, 374_761_393);
  value = Math.imul(value ^ Math.imul(y, 668_265_263), 1_274_126_177);
  value = Math.imul(value ^ seed, 2_246_822_519);
  value ^= value >>> 13;
  return (value >>> 0) / 2_147_483_647.5 - 1;
}

function revisionSeed(revision: string): number {
  let seed = 2_166_136_261;
  for (let index = 0; index < revision.length; index += 1) {
    seed ^= revision.charCodeAt(index);
    seed = Math.imul(seed, 16_777_619);
  }
  return seed | 0;
}

function smoothstep(value: number): number {
  return value * value * (3 - 2 * value);
}

function interpolate(left: number, right: number, progress: number): number {
  return left + (right - left) * progress;
}

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, value));
}
