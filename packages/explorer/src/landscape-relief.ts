import {
  planarPresentationSamples,
  type PlanarPresentationSample,
} from "./globe-samples";
import type { TerrainFieldSetInput } from "./types";

export const LANDSCAPE_RELIEF_ENGINE_REVISION =
  "rey.landscape-relief-engine@1" as const;
export const LANDSCAPE_TERRAIN_FABRIC_REVISION =
  "rey.landscape-terrain-fabric@1" as const;
export const LANDSCAPE_PATCH_SET_REVISION =
  "rey.landscape-patch-set@1" as const;

export interface LandscapeReliefField {
  schema: "rey.landscape-relief-field.v1";
  implementation_revision: typeof LANDSCAPE_RELIEF_ENGINE_REVISION;
  field_set_id: string;
  hillshade: Float32Array;
  salience: Float32Array;
  tangent: Float32Array;
}

export interface LandscapeTerrainFabricSample extends PlanarPresentationSample {
  relief: number;
  tangent_u: number;
  tangent_v: number;
  length: number;
  reveal_priority: number;
}

export interface LandscapePatchSet {
  schema: "rey.landscape-patch-set.v1";
  implementation_revision: typeof LANDSCAPE_PATCH_SET_REVISION;
  patch_set_id: string;
  patch_ids: readonly string[];
  overlap_pairs: readonly (readonly [string, string])[];
  bounds: { x: number; y: number; width: number; height: number } | null;
  overlap_policy: "later_patch_wins_with_deterministic_depth_bias";
  gap_policy: "unsupported_remains_transparent";
}

const RELIEF_RADII = Object.freeze([0, 2, 8] as const);
const RELIEF_WEIGHTS = Object.freeze([0.56, 0.29, 0.15] as const);
const KEY_LIGHT = normalize3(-0.48, -0.44, 0.76);
const FILL_LIGHT = normalize3(0.36, 0.28, 0.89);

/**
 * Binds one ordered landscape from zero or more admitted terrain patches.
 * Patch order is semantic input: later patches win coincident depth without
 * changing either patch's validity. Unsupported space between patch masks is
 * deliberately left transparent.
 */
export function compileLandscapePatchSet(
  fields: readonly TerrainFieldSetInput[],
): LandscapePatchSet {
  const patchIds = fields.map(({ field_set_id }) => field_set_id);
  if (new Set(patchIds).size !== patchIds.length)
    throw new Error("landscape patch set contains duplicate field identity");
  const overlapPairs: Array<readonly [string, string]> = [];
  for (let left = 0; left < fields.length; left += 1)
    for (let right = left + 1; right < fields.length; right += 1)
      if (boundsOverlap(fields[left]!.grid.bounds, fields[right]!.grid.bounds))
        overlapPairs.push(
          Object.freeze([
            fields[left]!.field_set_id,
            fields[right]!.field_set_id,
          ] as const),
        );
  const bounds = fields.reduce<LandscapePatchSet["bounds"]>((union, field) => {
    const next = field.grid.bounds;
    if (!union) return Object.freeze({ ...next });
    const left = Math.min(union.x, next.x);
    const top = Math.min(union.y, next.y);
    const right = Math.max(union.x + union.width, next.x + next.width);
    const bottom = Math.max(union.y + union.height, next.y + next.height);
    return Object.freeze({
      x: left,
      y: top,
      width: right - left,
      height: bottom - top,
    });
  }, null);
  return Object.freeze({
    schema: "rey.landscape-patch-set.v1",
    implementation_revision: LANDSCAPE_PATCH_SET_REVISION,
    patch_set_id: `${LANDSCAPE_PATCH_SET_REVISION}:${patchIds.join("|") || "empty"}`,
    patch_ids: Object.freeze(patchIds),
    overlap_pairs: Object.freeze(overlapPairs),
    bounds,
    overlap_policy: "later_patch_wins_with_deterministic_depth_bias",
    gap_policy: "unsupported_remains_transparent",
  });
}

/**
 * Compiles a renderer-neutral relief field from admitted terrain channels.
 * Every non-local scale requires a completely valid support window, so a
 * no-data edge cannot cast invented light, shadow, or stipple structure into
 * the admitted surface.
 */
export function deriveLandscapeReliefField(
  field: TerrainFieldSetInput,
): LandscapeReliefField {
  verifyTerrainFieldShape(field);
  const { columns, rows } = field.grid;
  const cells = field.field_cells;
  const normalX = new Float32Array(cells);
  const normalY = new Float32Array(cells);
  const normalUp = new Float32Array(cells);
  for (let index = 0; index < cells; index += 1) {
    if (field.validity.values[index] === 0) continue;
    const offset = index * 3;
    const normalized = normalize3(
      field.normal.values[offset]!,
      field.normal.values[offset + 1]!,
      field.normal.values[offset + 2]!,
    );
    normalX[index] = normalized[0];
    normalY[index] = normalized[1];
    normalUp[index] = normalized[2];
  }

  const supportPrefix = prefixSum(columns, rows, (index) =>
    field.validity.values[index] === 0 ? 0 : 1,
  );
  const elevationPrefix = prefixSum(columns, rows, (index) =>
    field.validity.values[index] === 0 ? 0 : field.elevation.values[index]!,
  );
  const normalXPrefix = prefixSum(columns, rows, (index) => normalX[index]!);
  const normalYPrefix = prefixSum(columns, rows, (index) => normalY[index]!);
  const normalUpPrefix = prefixSum(columns, rows, (index) => normalUp[index]!);
  const elevationSpan = Math.max(
    0.000_001,
    maximumSupported(field.elevation.values, field.validity.values) -
      minimumSupported(field.elevation.values, field.validity.values),
  );
  const hillshade = new Float32Array(cells);
  const salience = new Float32Array(cells);
  const tangent = new Float32Array(cells * 2);

  for (let row = 0; row < rows; row += 1) {
    for (let column = 0; column < columns; column += 1) {
      const index = row * columns + column;
      if (field.validity.values[index] === 0) continue;
      let illumination = 0;
      let reliefStrength = 0;
      let position = 0;
      let weightTotal = 0;
      let directionX = normalX[index]!;
      let directionY = normalY[index]!;

      for (let scale = 0; scale < RELIEF_RADII.length; scale += 1) {
        const radius = RELIEF_RADII[scale]!;
        const weight = RELIEF_WEIGHTS[scale]!;
        const window = completeWindow(
          supportPrefix,
          columns,
          rows,
          column,
          row,
          radius,
        );
        if (!window) continue;
        const inverseArea = 1 / window.area;
        const nx = rectangleSum(normalXPrefix, columns, window) * inverseArea;
        const ny = rectangleSum(normalYPrefix, columns, window) * inverseArea;
        const up = rectangleSum(normalUpPrefix, columns, window) * inverseArea;
        const normal = normalize3(nx, ny, up);
        const key = Math.max(0, dot3(normal, KEY_LIGHT));
        const fill = Math.max(0, dot3(normal, FILL_LIGHT));
        illumination += (0.4 + key * 0.52 + fill * 0.09) * weight;
        reliefStrength += Math.hypot(normal[0], normal[1]) * weight;
        const meanElevation =
          rectangleSum(elevationPrefix, columns, window) * inverseArea;
        const scaleRange = elevationSpan * (0.012 + radius * 0.0035);
        position +=
          clamp(
            (field.elevation.values[index]! - meanElevation) / scaleRange,
            -1,
            1,
          ) * weight;
        directionX += normal[0] * weight;
        directionY += normal[1] * weight;
        weightTotal += weight;
      }

      const inverseWeight = weightTotal > 0 ? 1 / weightTotal : 1;
      illumination *= inverseWeight;
      reliefStrength *= inverseWeight;
      position *= inverseWeight;
      const curvature = clamp(
        Math.abs(field.curvature.values[index]!) /
          Math.max(elevationSpan * 0.004, 0.000_01),
        0,
        1,
      );
      hillshade[index] = Math.fround(
        clamp(illumination + position * 0.075, 0.48, 1.08),
      );
      salience[index] = Math.fround(
        clamp(
          reliefStrength * 1.55 + Math.abs(position) * 0.24 + curvature * 0.13,
          0,
          1,
        ),
      );
      const tangentLength = Math.hypot(directionX, directionY);
      tangent[index * 2] = Math.fround(
        tangentLength > 0.000_01 ? -directionY / tangentLength : 1,
      );
      tangent[index * 2 + 1] = Math.fround(
        tangentLength > 0.000_01 ? directionX / tangentLength : 0,
      );
    }
  }

  return Object.freeze({
    schema: "rey.landscape-relief-field.v1",
    implementation_revision: LANDSCAPE_RELIEF_ENGINE_REVISION,
    field_set_id: field.field_set_id,
    hillshade,
    salience,
    tangent,
  });
}

/**
 * Projects the same relief field into the Atlas/planar dot vocabulary. The
 * candidate coordinates remain revision-stable; terrain content controls
 * their brightness, contour tangent, mark length, and reveal ordering.
 */
export function landscapeTerrainFabricSamples(
  field: TerrainFieldSetInput & { source_revision?: string },
  candidateCount = 6_000,
): readonly LandscapeTerrainFabricSample[] {
  const relief = deriveLandscapeReliefField(field);
  const revision = field.source_revision ?? field.field_set_id;
  const samples = planarPresentationSamples(revision, candidateCount)
    .flatMap((sample, sequence) => {
      const column = Math.min(
        field.grid.columns - 1,
        Math.max(0, Math.round(sample.u * (field.grid.columns - 1))),
      );
      const row = Math.min(
        field.grid.rows - 1,
        Math.max(0, Math.round(sample.v * (field.grid.rows - 1))),
      );
      const index = row * field.grid.columns + column;
      if (field.validity.values[index] === 0) return [];
      const terrainSalience = relief.salience[index]!;
      const illumination = relief.hillshade[index]!;
      return [
        {
          u: sample.u,
          v: sample.v,
          brightness: clamp(
            illumination * 0.7 + sample.brightness * 0.3,
            0.28,
            1,
          ),
          relief: terrainSalience,
          tangent_u: relief.tangent[index * 2]!,
          tangent_v: relief.tangent[index * 2 + 1]!,
          length: 0.52 + terrainSalience * 1.72,
          reveal_priority:
            terrainSalience * 0.72 +
            sample.brightness * 0.18 +
            stableSequenceNoise(sequence, revision) * 0.1,
        },
      ];
    })
    .sort(
      (left, right) =>
        right.reveal_priority - left.reveal_priority ||
        left.v - right.v ||
        left.u - right.u,
    );
  return Object.freeze(samples.map((sample) => Object.freeze(sample)));
}

interface PrefixWindow {
  left: number;
  top: number;
  right: number;
  bottom: number;
  area: number;
}

function prefixSum(
  columns: number,
  rows: number,
  value: (index: number) => number,
): Float64Array {
  const stride = columns + 1;
  const result = new Float64Array(stride * (rows + 1));
  for (let row = 0; row < rows; row += 1) {
    let rowSum = 0;
    for (let column = 0; column < columns; column += 1) {
      rowSum += value(row * columns + column);
      result[(row + 1) * stride + column + 1] =
        result[row * stride + column + 1]! + rowSum;
    }
  }
  return result;
}

function completeWindow(
  support: Float64Array,
  columns: number,
  rows: number,
  column: number,
  row: number,
  radius: number,
): PrefixWindow | null {
  const left = column - radius;
  const top = row - radius;
  const right = column + radius + 1;
  const bottom = row + radius + 1;
  if (left < 0 || top < 0 || right > columns || bottom > rows) return null;
  const window = { left, top, right, bottom, area: (radius * 2 + 1) ** 2 };
  return rectangleSum(support, columns, window) === window.area ? window : null;
}

function rectangleSum(
  prefix: Float64Array,
  columns: number,
  window: PrefixWindow,
): number {
  const stride = columns + 1;
  return (
    prefix[window.bottom * stride + window.right]! -
    prefix[window.top * stride + window.right]! -
    prefix[window.bottom * stride + window.left]! +
    prefix[window.top * stride + window.left]!
  );
}

function verifyTerrainFieldShape(field: TerrainFieldSetInput): void {
  const cells = field.grid.columns * field.grid.rows;
  if (
    field.field_cells !== cells ||
    field.validity.values.length !== cells ||
    field.elevation.values.length !== cells ||
    field.normal.values.length !== cells * 3 ||
    field.curvature.values.length !== cells
  )
    throw new Error("landscape relief input shape is invalid");
}

function minimumSupported(values: Float32Array, validity: Uint8Array): number {
  let result = Number.POSITIVE_INFINITY;
  for (let index = 0; index < values.length; index += 1)
    if (validity[index] !== 0) result = Math.min(result, values[index]!);
  return Number.isFinite(result) ? result : 0;
}

function maximumSupported(values: Float32Array, validity: Uint8Array): number {
  let result = Number.NEGATIVE_INFINITY;
  for (let index = 0; index < values.length; index += 1)
    if (validity[index] !== 0) result = Math.max(result, values[index]!);
  return Number.isFinite(result) ? result : 0;
}

function normalize3(
  x: number,
  y: number,
  z: number,
): readonly [number, number, number] {
  const length = Math.hypot(x, y, z);
  return length > 0 ? [x / length, y / length, z / length] : [0, 0, 1];
}

function dot3(
  left: readonly [number, number, number],
  right: readonly [number, number, number],
): number {
  return left[0] * right[0] + left[1] * right[1] + left[2] * right[2];
}

function stableSequenceNoise(sequence: number, revision: string): number {
  let value = sequence + 1;
  for (let index = 0; index < revision.length; index += 1)
    value = Math.imul(value ^ revision.charCodeAt(index), 16_777_619);
  value = Math.imul(value ^ (value >>> 16), 0x7feb352d);
  value = Math.imul(value ^ (value >>> 15), 0x846ca68b);
  return ((value ^ (value >>> 16)) >>> 0) / 4_294_967_295;
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(maximum, value));
}

function boundsOverlap(
  left: TerrainFieldSetInput["grid"]["bounds"],
  right: TerrainFieldSetInput["grid"]["bounds"],
): boolean {
  return (
    left.x < right.x + right.width &&
    left.x + left.width > right.x &&
    left.y < right.y + right.height &&
    left.y + left.height > right.y
  );
}
