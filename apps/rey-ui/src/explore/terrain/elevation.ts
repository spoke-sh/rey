import {
  fieldCellCount,
  fieldPoint,
  maskField,
  scalarField,
  type FieldGrid,
  type MaskField2D,
  type ScalarField2D,
} from "../engine/fields";
import type { ProjectionTerrainBand } from "../../domain";

export const PROJECTED_SUPPORT = 1;

export interface TerrainAnchorSample {
  id: string;
  x: number;
  y: number;
  prominence: number;
}

export interface ElevationResult {
  validity: MaskField2D;
  elevation: ScalarField2D;
}

export function deriveAnchorElevation(
  grid: FieldGrid,
  anchors: readonly TerrainAnchorSample[],
  revisions: { validity: string; elevation: string },
  procedural?: { seed: number; bands: readonly ProjectionTerrainBand[] },
): ElevationResult {
  const values = new Float32Array(fieldCellCount(grid));
  let maximum = 0;
  for (let row = 0; row < grid.rows; row += 1) {
    for (let column = 0; column < grid.columns; column += 1) {
      const index = row * grid.columns + column;
      const point = fieldPoint(grid, column, row);
      let height = 0;
      for (const anchor of anchors) {
        const sigma = 88 + anchor.prominence * 22;
        const distanceSquared =
          (point.x - anchor.x) ** 2 + (point.y - anchor.y) ** 2;
        height +=
          anchor.prominence * Math.exp(-distanceSquared / (2 * sigma * sigma));
      }
      values[index] = height;
      maximum = Math.max(maximum, height);
    }
  }

  const validityValues = new Uint8Array(values.length);
  const supportThreshold = maximum * 0.006;
  if (maximum > 0) {
    for (let index = 0; index < values.length; index += 1) {
      const base = values[index]!;
      const supported = base >= supportThreshold;
      validityValues[index] = supported ? PROJECTED_SUPPORT : 0;
      if (!supported) {
        values[index] = 0;
        continue;
      }
      const point = fieldPoint(
        grid,
        index % grid.columns,
        Math.floor(index / grid.columns),
      );
      const normalized = base / maximum;
      const edgeFade = smoothstep(supportThreshold, supportThreshold * 7, base);
      const detail = procedural
        ? procedural.bands.reduce(
            (sum, band, bandIndex) =>
              sum +
              terrainBand(point.x, point.y, procedural.seed, band, bandIndex),
            0,
          )
        : 0;
      values[index] = Math.max(0, normalized + detail * edgeFade);
    }
  }

  return {
    validity: maskField("validity", revisions.validity, grid, validityValues),
    elevation: scalarField(
      "anchor_elevation",
      revisions.elevation,
      grid,
      values,
    ),
  };
}

function terrainBand(
  x: number,
  y: number,
  seed: number,
  band: ProjectionTerrainBand,
  bandIndex: number,
): number {
  let frequency = 1 / band.wavelength_scene_units;
  let amplitude = band.amplitude_microunits / 1_000_000;
  let sum = 0;
  let weight = 0;
  for (let octave = 0; octave < band.octaves; octave += 1) {
    const warp = valueNoise(
      x * frequency * 0.47,
      y * frequency * 0.47,
      seed + bandIndex * 1_009 + octave * 97,
    );
    const sample = valueNoise(
      x * frequency + warp * 1.8,
      y * frequency - warp * 1.4,
      seed + bandIndex * 7_919 + octave * 503,
    );
    const ridged = 1 - Math.abs(sample * 2 - 1);
    sum += (ridged * 2 - 1) * amplitude;
    weight += amplitude;
    frequency *= 2.03;
    amplitude *= 0.52;
  }
  return weight > 0 ? sum : 0;
}

function valueNoise(x: number, y: number, seed: number): number {
  const x0 = Math.floor(x);
  const y0 = Math.floor(y);
  const tx = smoothstep(0, 1, x - x0);
  const ty = smoothstep(0, 1, y - y0);
  const a = hash2(x0, y0, seed);
  const b = hash2(x0 + 1, y0, seed);
  const c = hash2(x0, y0 + 1, seed);
  const d = hash2(x0 + 1, y0 + 1, seed);
  return mix(mix(a, b, tx), mix(c, d, tx), ty);
}

function hash2(x: number, y: number, seed: number): number {
  let value = Math.imul(x, 374_761_393) ^ Math.imul(y, 668_265_263) ^ seed;
  value = Math.imul(value ^ (value >>> 13), 1_274_126_177);
  return ((value ^ (value >>> 16)) >>> 0) / 4_294_967_295;
}

function smoothstep(edge0: number, edge1: number, value: number): number {
  const t = Math.min(1, Math.max(0, (value - edge0) / (edge1 - edge0)));
  return t * t * (3 - 2 * t);
}

function mix(left: number, right: number, amount: number): number {
  return left + (right - left) * amount;
}
