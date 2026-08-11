import {
  fieldCellCount,
  scalarField,
  vectorField,
  type MaskField2D,
  type ScalarField2D,
  type VectorField2D,
} from "../engine/fields";
import { PROJECTED_SUPPORT } from "./elevation";

export interface TerrainNormalResult {
  normal: VectorField2D;
  curvature: ScalarField2D;
}

export function deriveTerrainNormals(
  elevation: ScalarField2D,
  validity: MaskField2D,
  elevationScale: number,
  revisions: { normal: string; curvature: string },
): TerrainNormalResult {
  const { grid } = elevation;
  if (!Number.isFinite(elevationScale) || elevationScale <= 0)
    throw new Error(
      "terrain normals require a finite positive elevation scale",
    );
  const spacingX = grid.bounds.width / (grid.columns - 1);
  const spacingY = grid.bounds.height / (grid.rows - 1);
  const normals = new Float32Array(fieldCellCount(grid) * 3);
  const curvatureValues = new Float32Array(fieldCellCount(grid));
  const sample = (column: number, row: number, fallback: number) => {
    if (column < 0 || column >= grid.columns || row < 0 || row >= grid.rows)
      return fallback;
    const index = row * grid.columns + column;
    return validity.values[index] === PROJECTED_SUPPORT
      ? elevation.values[index]!
      : fallback;
  };

  for (let row = 0; row < grid.rows; row += 1) {
    for (let column = 0; column < grid.columns; column += 1) {
      const index = row * grid.columns + column;
      const normalOffset = index * 3;
      normals[normalOffset + 2] = 1;
      if (validity.values[index] !== PROJECTED_SUPPORT) continue;
      const center = elevation.values[index]!;
      const left = sample(column - 1, row, center);
      const right = sample(column + 1, row, center);
      const top = sample(column, row - 1, center);
      const bottom = sample(column, row + 1, center);
      const derivativeX = ((right - left) * elevationScale) / (2 * spacingX);
      const derivativeY = ((bottom - top) * elevationScale) / (2 * spacingY);
      const length = Math.hypot(derivativeX, derivativeY, 1);
      normals[normalOffset] = -derivativeX / length;
      normals[normalOffset + 1] = -derivativeY / length;
      normals[normalOffset + 2] = 1 / length;
      curvatureValues[index] = left + right + top + bottom - center * 4;
    }
  }

  return {
    normal: vectorField("normal", revisions.normal, grid, 3, normals),
    curvature: scalarField(
      "curvature",
      revisions.curvature,
      grid,
      curvatureValues,
    ),
  };
}
