import type { ProjectionPacket, TopographyPatch } from "../../domain";
import type {
  TopologyContour,
  TopologyNaturalFeature,
  TopologyPointOfInterest,
} from "../../topology";
import type { LensRegime } from "../engine/camera";
import {
  fieldPoint,
  type MaskField2D,
  type ScalarField2D,
} from "../engine/fields";
import {
  compileTerrainProgram,
  materializeTerrainWorkingSet,
  type TerrainFieldSet,
  type TerrainProgram,
} from "../terrain/compile";

export const SURVEY_TERRAIN_SCENE_COMPILER_REVISION =
  "rey.explorer.survey-terrain-scene@1";

const TERRAIN_LEVELS = [0.12, 0.23, 0.35, 0.48, 0.61, 0.74, 0.86] as const;

export interface SurveyTerrainFieldResult {
  contours: TopologyContour[];
  fields: TerrainFieldSet;
  program: TerrainProgram;
  natural_features: TopologyNaturalFeature[];
}

export function compileSurveyTerrainField(
  id: string,
  points: TopologyPointOfInterest[],
  frontier: TopologyPointOfInterest[],
  patch: TopographyPatch,
  projection: ProjectionPacket,
  regime: LensRegime,
  bounds: { x: number; y: number; width: number; height: number },
): SurveyTerrainFieldResult {
  const unresolvedPressure = Math.min(
    1.8,
    patch.frontier.length * 0.08 +
      patch.coverage.unresolved_candidates * 0.04 +
      patch.omissions.reduce(
        (total, omission) => total + omission.omitted_count,
        0,
      ) *
        0.03,
  );
  const program = compileTerrainProgram({
    source_id: id,
    source_revision: patch.topography_revision,
    bounds,
    anchors: points.map((point) => ({
      id: point.id,
      x: point.x,
      y: point.y,
      prominence: point.prominence,
    })),
    atmosphere: frontier.map((point) => ({ x: point.x, y: point.y })),
    unresolved_pressure: unresolvedPressure,
    projection,
  });
  const fields = materializeTerrainWorkingSet(program, {
    working_set_id: `reference:${regime}`,
    bounds,
    columns: 61,
    rows: 41,
    detail_authority:
      "bounded deterministic reference projection; accelerated detail is camera-relative",
  });
  const grid = fields.grid;
  const contours = TERRAIN_LEVELS.flatMap((ratio, index) => {
    const threshold = fields.elevation.maximum * ratio;
    const path = marchingSquaresPath(
      fields.elevation,
      fields.validity,
      threshold,
    );
    return path
      ? [
          {
            id: `relief:${id}:${index}`,
            path,
            level: index + 1,
            threshold,
            anchor_count: points.length,
          },
        ]
      : [];
  });

  const maximumAccumulation = fields.flow_accumulation.maximum;
  const streamThreshold = Math.max(3.2, maximumAccumulation * 0.055);
  const riverThreshold = Math.max(
    streamThreshold * 2.4,
    maximumAccumulation * 0.2,
  );
  const segmentPath = (minimum: number, maximum?: number) => {
    const segments: string[] = [];
    for (let row = 0; row < grid.rows; row += 1) {
      for (let column = 0; column < grid.columns; column += 1) {
        const index = row * grid.columns + column;
        const accumulation = fields.flow_accumulation.values[index]!;
        const columnOffset = fields.flow_direction.values[index * 2]!;
        const rowOffset = fields.flow_direction.values[index * 2 + 1]!;
        if (
          (columnOffset === 0 && rowOffset === 0) ||
          accumulation < minimum ||
          (maximum !== undefined && accumulation >= maximum)
        )
          continue;
        const from = fieldPoint(grid, column, row);
        const to = fieldPoint(grid, column + columnOffset, row + rowOffset);
        segments.push(
          `M${from.x.toFixed(1)},${from.y.toFixed(1)}L${to.x.toFixed(1)},${to.y.toFixed(1)}`,
        );
      }
    }
    return segments.join("");
  };
  const streamPath = segmentPath(streamThreshold, riverThreshold);
  const riverPath = segmentPath(riverThreshold);
  const naturalFeatures: TopologyNaturalFeature[] = [];
  if (streamPath)
    naturalFeatures.push({
      id: `stream-system:${id}`,
      path: streamPath,
      kind: "stream",
      label: "PROJECTED HEADWATERS",
      detail:
        "downslope accumulation over anchor-only relief under admitted survey conditions; not a source relationship or discovered path",
      intensity: Math.min(4, Math.max(1, maximumAccumulation / 18)),
      workload_id: id,
    });
  if (riverPath)
    naturalFeatures.push({
      id: `river-system:${id}`,
      path: riverPath,
      kind: "river",
      label: "ACCUMULATED FLOW",
      detail:
        "high-accumulation runoff projected from the same field that erodes the displayed relief; not retained hydrology",
      intensity: Math.min(4, Math.max(2, maximumAccumulation / 28)),
      workload_id: id,
    });
  frontier.forEach((point, index) => {
    const radius = 58 + point.prominence * 14;
    const skew = index % 2 === 0 ? 1 : -1;
    naturalFeatures.push({
      id: `weather-front:${id}:${point.id}`,
      path: `M${(point.x - radius).toFixed(1)},${point.y.toFixed(1)} C${(point.x - radius * 0.45).toFixed(1)},${(point.y - radius * 0.7 * skew).toFixed(1)} ${(point.x + radius * 0.35).toFixed(1)},${(point.y + radius * 0.72 * skew).toFixed(1)} ${(point.x + radius).toFixed(1)},${point.y.toFixed(1)}`,
      kind: "weather_front",
      label: "UNRESOLVED SURVEY FRONT",
      detail: `${point.detail} · ${point.signal}`,
      intensity: Math.min(4, 1 + unresolvedPressure),
      workload_id: id,
    });
  });
  return { contours, fields, program, natural_features: naturalFeatures };
}

function marchingSquaresPath(
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
