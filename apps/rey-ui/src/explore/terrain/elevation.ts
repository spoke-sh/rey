import {
  fieldCellCount,
  fieldPoint,
  maskField,
  scalarField,
  type FieldGrid,
  type MaskField2D,
  type ScalarField2D,
} from "../engine/fields";

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
      const supported = values[index]! >= supportThreshold;
      validityValues[index] = supported ? PROJECTED_SUPPORT : 0;
      values[index] = supported ? values[index]! / maximum : 0;
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
