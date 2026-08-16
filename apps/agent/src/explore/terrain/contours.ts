import type { TopologyContour } from "../../topology";
import type { LensRegime } from "../engine/camera";
import {
  fieldPoint,
  type MaskField2D,
  type ScalarField2D,
} from "../engine/fields";
import type { TerrainFieldSet } from "./compile";

export const TERRAIN_CONTOUR_COMPILER_REVISION =
  "rey.terrain.contours@1" as const;

const REGIONAL_CONTOUR_COUNTS: Record<LensRegime, number> = {
  world: 0,
  atlas: 0,
  landscape: 7,
  neighborhoods: 10,
  objects: 13,
  evidence: 13,
};

export function deriveRegionalTerrainContours(
  field: TerrainFieldSet,
  regime: LensRegime,
): TopologyContour[] {
  const count = REGIONAL_CONTOUR_COUNTS[regime];
  if (count === 0) return [];
  const span = field.elevation.maximum - field.elevation.minimum;
  if (!(span > 0)) return [];
  const anchorCount = field.validity.values.reduce(
    (total, value) => total + (value === 0 ? 0 : 1),
    0,
  );
  return Array.from({ length: count }, (_, index) => {
    const ratio = (index + 1) / (count + 1);
    const threshold = field.elevation.minimum + span * ratio;
    const path = terrainContourPath(field.elevation, field.validity, threshold);
    if (!path) return null;
    return {
      id: `${TERRAIN_CONTOUR_COMPILER_REVISION}:${field.field_set_id}:${regime}:${index}`,
      path,
      level: Math.max(1, Math.min(7, Math.round(ratio * 7))),
      threshold,
      anchor_count: anchorCount,
    };
  }).filter((contour): contour is TopologyContour => contour !== null);
}

export function terrainContourPath(
  elevation: ScalarField2D,
  validity: MaskField2D,
  threshold: number,
): string {
  const segments: string[] = [];
  const { grid } = elevation;
  const point = (column: number, row: number) => fieldPoint(grid, column, row);
  const value = (column: number, row: number) =>
    elevation.values[row * grid.columns + column]!;
  const crossing = (
    first: { x: number; y: number; value: number },
    second: { x: number; y: number; value: number },
  ) => {
    const denominator = second.value - first.value;
    const amount =
      denominator === 0 ? 0.5 : (threshold - first.value) / denominator;
    return {
      x: first.x + (second.x - first.x) * amount,
      y: first.y + (second.y - first.y) * amount,
    };
  };
  const line = (
    first: { x: number; y: number },
    second: { x: number; y: number },
  ) =>
    `M${first.x.toFixed(1)},${first.y.toFixed(1)}L${second.x.toFixed(1)},${second.y.toFixed(1)}`;

  for (let row = 0; row < grid.rows - 1; row += 1) {
    for (let column = 0; column < grid.columns - 1; column += 1) {
      const cornerIndices = [
        row * grid.columns + column,
        row * grid.columns + column + 1,
        (row + 1) * grid.columns + column + 1,
        (row + 1) * grid.columns + column,
      ];
      if (cornerIndices.some((index) => validity.values[index] === 0)) continue;
      const topLeft = { ...point(column, row), value: value(column, row) };
      const topRight = {
        ...point(column + 1, row),
        value: value(column + 1, row),
      };
      const bottomRight = {
        ...point(column + 1, row + 1),
        value: value(column + 1, row + 1),
      };
      const bottomLeft = {
        ...point(column, row + 1),
        value: value(column, row + 1),
      };
      const crossings: Array<{
        edge: "top" | "right" | "bottom" | "left";
        point: { x: number; y: number };
      }> = [];
      const addCrossing = (
        edgeName: "top" | "right" | "bottom" | "left",
        first: typeof topLeft,
        second: typeof topLeft,
      ) => {
        if (first.value >= threshold !== second.value >= threshold)
          crossings.push({ edge: edgeName, point: crossing(first, second) });
      };
      addCrossing("top", topLeft, topRight);
      addCrossing("right", topRight, bottomRight);
      addCrossing("bottom", bottomRight, bottomLeft);
      addCrossing("left", bottomLeft, topLeft);
      if (crossings.length === 2)
        segments.push(line(crossings[0]!.point, crossings[1]!.point));
      else if (crossings.length === 4) {
        const byEdge = new Map(
          crossings.map((candidate) => [candidate.edge, candidate.point]),
        );
        const center =
          (topLeft.value +
            topRight.value +
            bottomRight.value +
            bottomLeft.value) /
          4;
        const pairs: Array<
          [
            "top" | "right" | "bottom" | "left",
            "top" | "right" | "bottom" | "left",
          ]
        > =
          center >= threshold
            ? [
                ["top", "left"],
                ["right", "bottom"],
              ]
            : [
                ["top", "right"],
                ["bottom", "left"],
              ];
        for (const [first, second] of pairs) {
          const firstPoint = byEdge.get(first);
          const secondPoint = byEdge.get(second);
          if (firstPoint && secondPoint)
            segments.push(line(firstPoint, secondPoint));
        }
      }
    }
  }
  return segments.join("");
}
