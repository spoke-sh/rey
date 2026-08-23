import {
  composeCartographicTerrainColor,
  linearTerrainChannelToSrgbByte,
  type LandscapeReliefField,
} from "@rey/explorer";
import type { TerrainFieldSet } from "../terrain/compile";

export const REFERENCE_TERRAIN_RASTER_REVISION =
  "rey.reference-regional-terrain@5" as const;

export interface ReferenceTerrainRasterField {
  field: TerrainFieldSet;
  relief: LandscapeReliefField;
}

/**
 * Rasterizes the same validity-safe triangle topology and linear vertex color
 * consumed by the accelerated paths. Transparent pixels are unsupported; no
 * bounds fill or bilinear sampling across an invalid triangle is permitted.
 */
export function rasterizeReferenceTerrain(
  fields: readonly ReferenceTerrainRasterField[],
  width: number,
  height: number,
  world: { width: number; height: number },
): Uint8ClampedArray {
  if (
    !Number.isSafeInteger(width) ||
    !Number.isSafeInteger(height) ||
    width < 1 ||
    height < 1 ||
    !(world.width > 0) ||
    !(world.height > 0)
  )
    throw new Error("reference terrain raster bounds are invalid");
  const pixels = new Uint8ClampedArray(width * height * 4);
  const screenScaleX = width / world.width;
  const screenScaleY = height / world.height;
  for (const { field, relief } of fields) {
    const color = composeCartographicTerrainColor(field, relief);
    const { bounds, columns, rows } = field.grid;
    const startX = Math.max(0, Math.floor(bounds.x * screenScaleX));
    const endX = Math.min(
      width,
      Math.ceil((bounds.x + bounds.width) * screenScaleX),
    );
    const startY = Math.max(0, Math.floor(bounds.y * screenScaleY));
    const endY = Math.min(
      height,
      Math.ceil((bounds.y + bounds.height) * screenScaleY),
    );
    for (let screenY = startY; screenY < endY; screenY += 1) {
      const worldY = (screenY + 0.5) / screenScaleY;
      const gridY =
        ((worldY - bounds.y) / bounds.height) * Math.max(1, rows - 1);
      const row = Math.max(0, Math.min(rows - 2, Math.floor(gridY)));
      const v = Math.max(0, Math.min(1, gridY - row));
      for (let screenX = startX; screenX < endX; screenX += 1) {
        const worldX = (screenX + 0.5) / screenScaleX;
        const gridX =
          ((worldX - bounds.x) / bounds.width) * Math.max(1, columns - 1);
        const column = Math.max(0, Math.min(columns - 2, Math.floor(gridX)));
        const u = Math.max(0, Math.min(1, gridX - column));
        const topLeft = row * columns + column;
        const topRight = topLeft + 1;
        const bottomLeft = topLeft + columns;
        const bottomRight = bottomLeft + 1;
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
        const useDescending =
          descendingScore === ascendingScore
            ? (row + column) % 2 === 0
            : descendingScore > ascendingScore;
        const sample = referenceTriangleSample(
          useDescending,
          u,
          v,
          topLeft,
          topRight,
          bottomLeft,
          bottomRight,
        );
        if (sample.indices.some((index) => field.validity.values[index] === 0))
          continue;
        const pixel = (screenY * width + screenX) * 4;
        for (let component = 0; component < 3; component += 1) {
          const linear = sample.indices.reduce(
            (total, index, sequence) =>
              total + color[index * 3 + component]! * sample.weights[sequence]!,
            0,
          );
          pixels[pixel + component] = linearTerrainChannelToSrgbByte(linear);
        }
        pixels[pixel + 3] = 255;
      }
    }
  }
  return pixels;
}

function referenceTriangleSample(
  descending: boolean,
  u: number,
  v: number,
  topLeft: number,
  topRight: number,
  bottomLeft: number,
  bottomRight: number,
): {
  indices: readonly [number, number, number];
  weights: readonly [number, number, number];
} {
  if (descending)
    return v >= u
      ? {
          indices: [topLeft, bottomLeft, bottomRight],
          weights: [1 - v, v - u, u],
        }
      : {
          indices: [topLeft, bottomRight, topRight],
          weights: [1 - u, v, u - v],
        };
  return u + v <= 1
    ? {
        indices: [topLeft, bottomLeft, topRight],
        weights: [1 - u - v, v, u],
      }
    : {
        indices: [topRight, bottomLeft, bottomRight],
        weights: [1 - v, 1 - u, u + v - 1],
      };
}
