import {
  terrainTriangleIndices,
  type TerrainAreaFeatureInput,
  type TerrainLineFeatureInput,
  type TerrainPointFeatureInput,
  type TerrainRenderPassSetInput,
} from "@rey/explorer";
import type { TopologyScene } from "../../topology";
import type { TerrainFieldSet } from "../terrain/compile";
import { fieldPoint } from "./fields";
import { featureVisibleAtLens } from "./cartography";
import {
  activeExplorerRenderPasses,
  compactPresentationRevision,
  type ExplorerRenderGraph,
  type ExplorerRenderVisibility,
  type ExplorerRenderPassId,
} from "./render-graph";

export const TERRAIN_RENDER_PASS_COMPILER_REVISION =
  "rey.explorer.terrain-render-passes@2" as const;

export function compileTerrainRenderPasses(
  scene: TopologyScene,
  graph: ExplorerRenderGraph,
  visibility: ExplorerRenderVisibility,
): TerrainRenderPassSetInput | null {
  if (scene.terrain_fields.length === 0) return null;
  const active = activeExplorerRenderPasses(graph, visibility);
  const activeIds = new Set(active.map(({ id }) => id));
  const passes = active
    .filter(
      (
        pass,
      ): pass is typeof pass & {
        id: Exclude<ExplorerRenderPassId, "evidence_accessibility">;
      } => pass.id !== "evidence_accessibility",
    )
    .map((pass) =>
      Object.freeze({
        id: pass.id,
        implementation_revision: pass.implementation_revision,
        input_revision: pass.input_revision,
        authority: pass.authority,
      }),
    );
  const lines: TerrainLineFeatureInput[] = [];
  const areas: TerrainAreaFeatureInput[] = [];
  const points: TerrainPointFeatureInput[] = [];
  const omissions: string[] = [];
  const appendPath = (
    feature: Omit<TerrainLineFeatureInput, "id" | "positions"> & {
      id: string;
      path: string;
    },
  ) => {
    const polylines = parseSvgPolylines(feature.path);
    let admitted = 0;
    const positions: number[] = [];
    polylines.forEach((polyline) => {
      const segments = drapePolyline(polyline, scene.terrain_fields, 1.4);
      segments.forEach((strip) => {
        for (let component = 3; component < strip.length; component += 3) {
          positions.push(
            strip[component - 3]!,
            strip[component - 2]!,
            strip[component - 1]!,
            strip[component]!,
            strip[component + 1]!,
            strip[component + 2]!,
          );
          admitted += 1;
        }
      });
    });
    if (admitted > 0)
      lines.push(
        Object.freeze({
          id: `${feature.id}:batch`,
          pass_id: feature.pass_id,
          kind: feature.kind,
          source_revision: feature.source_revision,
          authority: feature.authority,
          positions: Float32Array.from(positions),
          color: feature.color,
          opacity: feature.opacity,
        }),
      );
    if (admitted === 0 && polylines.length > 0)
      omissions.push(
        `${feature.id} has no fully valid terrain-supported accelerated segment; reference geometry retained`,
      );
  };

  if (activeIds.has("contours"))
    for (const contour of scene.contours)
      appendPath({
        id: contour.id,
        pass_id: "contours",
        kind: "contour",
        source_revision: `${contour.id}:${contour.threshold}`,
        authority: "derived contour over the same admitted validity field",
        path: contour.path,
        color: 0x756d54,
        opacity: 0.28,
      });

  if (activeIds.has("water_weather_boundary")) {
    for (const feature of scene.natural_features)
      if (
        (feature.kind === "weather_front" && visibility.weather) ||
        (feature.kind !== "weather_front" && visibility.water)
      )
        appendPath({
          id: feature.id,
          pass_id: "water_weather_boundary",
          kind: feature.kind,
          source_revision: `${feature.id}:${feature.path}`,
          authority: feature.detail,
          path: feature.path,
          color:
            feature.kind === "river"
              ? 0x73b6cf
              : feature.kind === "stream"
                ? 0x89c6da
                : 0xe1b66d,
          opacity: feature.kind === "weather_front" ? 0.52 : 0.78,
        });
    if (scene.county_footprint)
      appendPath({
        id: scene.county_footprint.footprint_id,
        pass_id: "water_weather_boundary",
        kind: "admitted_boundary",
        source_revision: scene.county_footprint.source_object_revision,
        authority: scene.county_footprint.authority,
        path: scene.county_footprint.path,
        color: 0x615f4d,
        opacity: 0.34,
      });
    for (const node of scene.nodes) {
      const feature = node.spatial_feature;
      if (
        !feature ||
        feature.layer !== "hydrology" ||
        !visibility.water ||
        !featureVisibleAtLens(
          feature,
          scene.regime,
          node.focus_id === scene.focus_id,
        ) ||
        feature.geometry_kind.toLowerCase() === "point"
      )
        continue;
      if (
        feature.geometry_kind.toLowerCase() === "polygon" &&
        feature.geometry_representation === "exact_native"
      ) {
        const positions = drapeTerrainArea(
          feature.geometry_path,
          scene.terrain_fields,
          1.05,
        );
        if (positions.length > 0)
          areas.push(
            Object.freeze({
              id: `${node.id}:surface`,
              pass_id: "water_weather_boundary",
              kind: "water_area",
              source_revision: `${node.id}:${feature.geometry_path}:${feature.geometry_representation}`,
              authority: `${feature.authority}; surface is conservatively quantized to fully valid terrain triangles`,
              positions,
              color: 0x4f93a0,
              opacity: 0.48,
            }),
          );
        else
          omissions.push(
            `${node.id} has no fully valid terrain-supported water surface; exact reference outline retained`,
          );
      }
      appendPath({
        id: node.id,
        pass_id: "water_weather_boundary",
        kind: feature.layer,
        source_revision: `${node.id}:${feature.layer}:${feature.geometry_path}:${feature.geometry_representation}:${node.focus_id === scene.focus_id ? "selected" : "unselected"}`,
        authority: feature.authority,
        path: feature.geometry_path,
        color: featureColor(feature.layer, node.focus_id === scene.focus_id),
        opacity: node.focus_id === scene.focus_id ? 0.72 : 0.42,
      });
    }
  }

  if (activeIds.has("features_labels_selection")) {
    for (const node of scene.nodes) {
      const feature = node.spatial_feature;
      if (
        feature &&
        featureVisibleAtLens(
          feature,
          scene.regime,
          node.focus_id === scene.focus_id,
        ) &&
        feature.layer !== "hydrology" &&
        feature.geometry_kind.toLowerCase() !== "point"
      ) {
        appendPath({
          id: node.id,
          pass_id: "features_labels_selection",
          kind: feature.layer,
          source_revision: `${node.id}:${feature.layer}:${feature.geometry_path}:${feature.geometry_representation}:${node.focus_id === scene.focus_id ? "selected" : "unselected"}`,
          authority: feature.authority,
          path: feature.geometry_path,
          color: featureColor(feature.layer, node.focus_id === scene.focus_id),
          opacity: node.focus_id === scene.focus_id ? 0.82 : 0.32,
        });
      } else if (
        !feature ||
        (feature.geometry_kind.toLowerCase() === "point" &&
          featureVisibleAtLens(
            feature,
            scene.regime,
            node.focus_id === scene.focus_id,
          ))
      ) {
        const height = terrainHeightAtPoint(
          scene.terrain_fields,
          node.x,
          node.y,
        );
        if (height !== null)
          points.push(
            pointFeature(
              node.id,
              feature?.layer ?? "semantic_object",
              `${node.id}:${node.x},${node.y}:${feature?.layer ?? "semantic_object"}:${node.focus_id === scene.focus_id ? "selected" : "unselected"}`,
              feature?.authority ?? "interface projection of retained identity",
              node.x,
              height + 1.8,
              node.y,
              featureColor(
                feature?.layer ?? "native_feature",
                node.focus_id === scene.focus_id,
              ),
              node.focus_id === scene.focus_id ? 4.5 : 2.5,
            ),
          );
      }
    }
    for (const point of scene.points) {
      if (point.kind === "frontier" && !visibility.probes) continue;
      const height = terrainHeightAtPoint(
        scene.terrain_fields,
        point.x,
        point.y,
      );
      if (height !== null)
        points.push(
          pointFeature(
            point.id,
            point.kind,
            `${point.id}:${point.kind}:${point.x},${point.y}:${point.prominence}:${point.focus_id === scene.focus_id ? "selected" : "unselected"}`,
            point.detail,
            point.x,
            height + 1.8,
            point.y,
            point.kind === "frontier" ? 0xe6aa58 : 0xf1d17a,
            Math.max(3, Math.min(8, point.prominence * 1.4)),
          ),
        );
    }
  }

  const bounds = terrainBounds(scene.terrain_fields);
  const passSetId = compactPresentationRevision(
    "rey.terrain-render-pass-set.v1",
    [
      TERRAIN_RENDER_PASS_COMPILER_REVISION,
      graph.graph_id,
      ...passes.map(
        (pass) =>
          `${pass.id}:${pass.implementation_revision}:${pass.input_revision}`,
      ),
      ...areas.map(({ id, source_revision }) => `${id}:${source_revision}`),
      ...lines.map(({ id, source_revision }) => `${id}:${source_revision}`),
      ...points.map(({ id, source_revision }) => `${id}:${source_revision}`),
      ...omissions,
    ],
  );
  return Object.freeze({
    schema: "rey.terrain-render-pass-set.v1",
    pass_set_id: passSetId,
    bounds,
    passes: Object.freeze(passes),
    areas: Object.freeze(areas),
    lines: Object.freeze(lines),
    points: Object.freeze(points),
    omissions: Object.freeze(omissions),
  });
}

export function drapeTerrainArea(
  path: string,
  fields: readonly TerrainFieldSet[],
  offset: number,
): Float32Array {
  const rings = parseSvgPolylines(path);
  if (rings.length === 0) return new Float32Array();
  const positions: number[] = [];
  for (const field of fields) {
    const indices = terrainTriangleIndices(field);
    const pointForIndex = (index: number) => {
      const column = index % field.grid.columns;
      const row = Math.floor(index / field.grid.columns);
      const point = fieldPoint(field.grid, column, row);
      return {
        x: point.x,
        y: point.y,
        height: field.elevation.values[index]! * field.elevation_scale + offset,
      };
    };
    for (let index = 0; index < indices.length; index += 3) {
      const triangle = [
        pointForIndex(indices[index]!),
        pointForIndex(indices[index + 1]!),
        pointForIndex(indices[index + 2]!),
      ] as const;
      const center = {
        x: (triangle[0].x + triangle[1].x + triangle[2].x) / 3,
        y: (triangle[0].y + triangle[1].y + triangle[2].y) / 3,
      };
      if (!pointInPolygon(center, rings)) continue;
      for (const point of triangle)
        positions.push(point.x, point.height, point.y);
    }
  }
  return Float32Array.from(positions);
}

function pointInPolygon(
  point: { x: number; y: number },
  rings: ReadonlyArray<ReadonlyArray<{ x: number; y: number }>>,
): boolean {
  let inside = false;
  for (const ring of rings) {
    for (
      let index = 0, previous = ring.length - 1;
      index < ring.length;
      previous = index++
    ) {
      const first = ring[index]!;
      const second = ring[previous]!;
      if (
        first.y > point.y !== second.y > point.y &&
        point.x <
          ((second.x - first.x) * (point.y - first.y)) / (second.y - first.y) +
            first.x
      )
        inside = !inside;
    }
  }
  return inside;
}

export function parseSvgPolylines(
  path: string,
): ReadonlyArray<ReadonlyArray<{ x: number; y: number }>> {
  const tokens =
    path.match(/[MLCZmlcz]|[-+]?(?:\d*\.\d+|\d+\.?)(?:[eE][-+]?\d+)?/g) ?? [];
  const polylines: Array<Array<{ x: number; y: number }>> = [];
  let current: Array<{ x: number; y: number }> = [];
  let cursor = { x: 0, y: 0 };
  let command = "";
  let index = 0;
  const number = () => {
    const token = tokens[index++];
    if (token === undefined || /[MLCZmlcz]/.test(token))
      throw new Error("terrain overlay path is malformed");
    const value = Number(token);
    if (!Number.isFinite(value))
      throw new Error("terrain overlay path contains a non-finite coordinate");
    return value;
  };
  const finish = () => {
    if (current.length >= 2) polylines.push(current);
    current = [];
  };
  while (index < tokens.length) {
    if (/[MLCZmlcz]/.test(tokens[index]!)) command = tokens[index++]!;
    if (command === "M" || command === "m") {
      finish();
      const relative = command === "m";
      cursor = {
        x: number() + (relative ? cursor.x : 0),
        y: number() + (relative ? cursor.y : 0),
      };
      current.push({ ...cursor });
      command = relative ? "l" : "L";
    } else if (command === "L" || command === "l") {
      const relative = command === "l";
      cursor = {
        x: number() + (relative ? cursor.x : 0),
        y: number() + (relative ? cursor.y : 0),
      };
      current.push({ ...cursor });
    } else if (command === "C" || command === "c") {
      const relative = command === "c";
      const origin = { ...cursor };
      const first = {
        x: number() + (relative ? origin.x : 0),
        y: number() + (relative ? origin.y : 0),
      };
      const second = {
        x: number() + (relative ? origin.x : 0),
        y: number() + (relative ? origin.y : 0),
      };
      const end = {
        x: number() + (relative ? origin.x : 0),
        y: number() + (relative ? origin.y : 0),
      };
      for (let sample = 1; sample <= 12; sample += 1) {
        const amount = sample / 12;
        const inverse = 1 - amount;
        current.push({
          x:
            inverse ** 3 * origin.x +
            3 * inverse ** 2 * amount * first.x +
            3 * inverse * amount ** 2 * second.x +
            amount ** 3 * end.x,
          y:
            inverse ** 3 * origin.y +
            3 * inverse ** 2 * amount * first.y +
            3 * inverse * amount ** 2 * second.y +
            amount ** 3 * end.y,
        });
      }
      cursor = end;
    } else if (command === "Z" || command === "z") {
      if (current.length > 0) current.push({ ...current[0]! });
      finish();
      command = "";
    } else throw new Error("terrain overlay path uses an unsupported command");
  }
  finish();
  return Object.freeze(
    polylines.map((polyline) =>
      Object.freeze(polyline.map((point) => Object.freeze(point))),
    ),
  );
}

function drapePolyline(
  polyline: readonly { x: number; y: number }[],
  fields: readonly TerrainFieldSet[],
  offset: number,
): Float32Array[] {
  const segments: Float32Array[] = [];
  let current: number[] = [];
  const finish = () => {
    if (current.length >= 6) segments.push(Float32Array.from(current));
    current = [];
  };
  for (const point of validitySamplingPoints(polyline, fields)) {
    const height = terrainHeightAtPoint(fields, point.x, point.y);
    if (height === null) {
      finish();
      continue;
    }
    current.push(point.x, height + offset, point.y);
  }
  finish();
  return segments;
}

function validitySamplingPoints(
  polyline: readonly { x: number; y: number }[],
  fields: readonly TerrainFieldSet[],
): readonly { x: number; y: number }[] {
  if (polyline.length < 2) return polyline;
  const sampled: Array<{ x: number; y: number }> = [];
  for (let index = 1; index < polyline.length; index += 1) {
    const start = polyline[index - 1]!;
    const end = polyline[index]!;
    const progress = new Set([0, 1]);
    for (const field of fields) {
      const { bounds, columns, rows } = field.grid;
      appendGridCrossings(
        progress,
        start.x,
        end.x,
        bounds.x,
        bounds.width,
        columns,
      );
      appendGridCrossings(
        progress,
        start.y,
        end.y,
        bounds.y,
        bounds.height,
        rows,
      );
    }
    const crossings = [...progress].sort((left, right) => left - right);
    const probes = crossings.flatMap((value, crossingIndex) => {
      const next = crossings[crossingIndex + 1];
      return next === undefined ? [value] : [value, (value + next) / 2];
    });
    for (const amount of probes) {
      if (index > 1 && amount === 0) continue;
      sampled.push({
        x: interpolate(start.x, end.x, amount),
        y: interpolate(start.y, end.y, amount),
      });
    }
  }
  return sampled;
}

function appendGridCrossings(
  progress: Set<number>,
  start: number,
  end: number,
  origin: number,
  extent: number,
  samples: number,
) {
  const delta = end - start;
  if (delta === 0) return;
  for (let sample = 1; sample < samples - 1; sample += 1) {
    const coordinate = origin + (sample / (samples - 1)) * extent;
    const amount = (coordinate - start) / delta;
    if (amount > 0 && amount < 1) progress.add(amount);
  }
}

function terrainHeightAtPoint(
  fields: readonly TerrainFieldSet[],
  x: number,
  y: number,
): number | null {
  for (const field of fields) {
    const { bounds, columns, rows } = field.grid;
    if (
      x < bounds.x ||
      x > bounds.x + bounds.width ||
      y < bounds.y ||
      y > bounds.y + bounds.height
    )
      continue;
    const columnPosition = ((x - bounds.x) / bounds.width) * (columns - 1);
    const rowPosition = ((y - bounds.y) / bounds.height) * (rows - 1);
    const left = Math.min(columns - 2, Math.max(0, Math.floor(columnPosition)));
    const top = Math.min(rows - 2, Math.max(0, Math.floor(rowPosition)));
    const right = left + 1;
    const bottom = top + 1;
    const indexes = [
      top * columns + left,
      top * columns + right,
      bottom * columns + left,
      bottom * columns + right,
    ] as const;
    if (indexes.some((index) => field.validity.values[index] === 0))
      return null;
    const amountX = columnPosition - left;
    const amountY = rowPosition - top;
    const topHeight = interpolate(
      field.elevation.values[indexes[0]]!,
      field.elevation.values[indexes[1]]!,
      amountX,
    );
    const bottomHeight = interpolate(
      field.elevation.values[indexes[2]]!,
      field.elevation.values[indexes[3]]!,
      amountX,
    );
    return (
      interpolate(topHeight, bottomHeight, amountY) * field.elevation_scale
    );
  }
  return null;
}

function terrainBounds(fields: readonly TerrainFieldSet[]) {
  const left = Math.min(...fields.map((field) => field.grid.bounds.x));
  const top = Math.min(...fields.map((field) => field.grid.bounds.y));
  const right = Math.max(
    ...fields.map((field) => field.grid.bounds.x + field.grid.bounds.width),
  );
  const bottom = Math.max(
    ...fields.map((field) => field.grid.bounds.y + field.grid.bounds.height),
  );
  return Object.freeze({
    x: left,
    y: top,
    width: right - left,
    height: bottom - top,
  });
}

function pointFeature(
  id: string,
  kind: string,
  sourceRevision: string,
  authority: string,
  x: number,
  height: number,
  y: number,
  color: number,
  radius: number,
): TerrainPointFeatureInput {
  return Object.freeze({
    id,
    pass_id: "features_labels_selection",
    kind,
    source_revision: sourceRevision,
    authority,
    position: Object.freeze([x, height, y]) as readonly [
      number,
      number,
      number,
    ],
    color,
    radius,
  });
}

function featureColor(layer: string, selected: boolean): number {
  if (selected) return 0xffd36e;
  if (layer === "hydrology") return 0x77bfd8;
  if (layer === "boundary") return 0xe4cb86;
  if (layer === "highway" || layer === "road") return 0xe5ded0;
  if (layer === "structure" || layer === "construction") return 0xb8a98f;
  if (layer === "terrain_control") return 0xa88c65;
  return 0xc9c2ad;
}

function interpolate(left: number, right: number, progress: number): number {
  return left + (right - left) * progress;
}
