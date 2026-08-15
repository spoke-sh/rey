import type { AdmittedRegionalScene, RegionalBounds } from "../../domain";
import {
  TERRAIN_FIELD_SCHEMA,
  createFieldGrid,
  fieldByteLength,
  fieldCellCount,
  maskField,
  materialField,
  scalarField,
  vectorField,
  type FieldBounds,
} from "../engine/fields";
import type { TerrainFieldSet } from "../terrain/compile";
import { deriveTerrainNormals } from "../terrain/normals";
import type {
  CountyFrame,
  CountyFootprint,
  ProjectedCountyFootprint,
} from "./county-frame";
import { nativePositionToCountyLocal } from "./county-frame";

export const REGIONAL_TERRAIN_SCENE_COMPILER_REVISION =
  "rey.explorer.regional-terrain-grid@1";

const TERRAIN_CANVAS_INSET_X = 96;
const TERRAIN_CANVAS_INSET_Y = 72;
const REGIONAL_ELEVATION_HEIGHT_RATIO = 0.24;

export function compileRegionalTerrainField(
  scene: AdmittedRegionalScene,
  world: { width: number; height: number },
): TerrainFieldSet | null {
  const program = scene.projection.terrain;
  const dataset = program?.grid;
  if (!program || !dataset) return null;
  const bounds = projectRegionalTerrainBounds(
    scene.native_bounds,
    dataset.native_bounds,
    world,
  );
  const grid = createFieldGrid(dataset.columns, dataset.rows, bounds);
  const cells = fieldCellCount(grid);
  if (dataset.cells.length !== cells)
    throw new Error("regional terrain dataset shape changed after admission");
  const validityValues = Uint8Array.from(
    dataset.cells.map((cell) => (cell.validity === "valid" ? 1 : 0)),
  );
  const validElevations = dataset.cells.flatMap((cell) =>
    cell.validity === "valid" && cell.elevation_micrometers !== null
      ? [cell.elevation_micrometers / 1_000_000]
      : [],
  );
  if (validElevations.length < 3)
    throw new Error("regional terrain dataset has no supported triangle");
  const minimumElevation = Math.min(...validElevations);
  const maximumElevation = Math.max(...validElevations);
  const elevationRange = Math.max(1, maximumElevation - minimumElevation);
  const elevationValues = Float32Array.from(
    dataset.cells.map((cell) =>
      cell.validity === "valid" && cell.elevation_micrometers !== null
        ? (cell.elevation_micrometers / 1_000_000 - minimumElevation) /
          elevationRange
        : 0,
    ),
  );
  const validity = maskField(
    "validity",
    `${REGIONAL_TERRAIN_SCENE_COMPILER_REVISION}:validity:${dataset.dataset_id}`,
    grid,
    validityValues,
  );
  const elevation = scalarField(
    "elevation",
    `${REGIONAL_TERRAIN_SCENE_COMPILER_REVISION}:elevation:${dataset.dataset_id}`,
    grid,
    elevationValues,
  );
  const elevationScale =
    Math.min(world.width, world.height) * REGIONAL_ELEVATION_HEIGHT_RATIO;
  const relief = deriveTerrainNormals(elevation, validity, elevationScale, {
    normal: `${REGIONAL_TERRAIN_SCENE_COMPILER_REVISION}:normal:${dataset.dataset_id}`,
    curvature: `${REGIONAL_TERRAIN_SCENE_COMPILER_REVISION}:curvature:${dataset.dataset_id}`,
  });
  const zeroValues = new Float32Array(cells);
  const rainfall = scalarField(
    "rainfall",
    `${REGIONAL_TERRAIN_SCENE_COMPILER_REVISION}:not-observed:rainfall`,
    grid,
    zeroValues.slice(),
  );
  const flowDirection = vectorField(
    "flow_direction",
    `${REGIONAL_TERRAIN_SCENE_COMPILER_REVISION}:not-observed:flow-direction`,
    grid,
    2,
    new Float32Array(cells * 2),
  );
  const flowAccumulation = scalarField(
    "flow_accumulation",
    `${REGIONAL_TERRAIN_SCENE_COMPILER_REVISION}:not-observed:flow-accumulation`,
    grid,
    zeroValues.slice(),
  );
  const erosion = scalarField(
    "erosion",
    `${REGIONAL_TERRAIN_SCENE_COMPILER_REVISION}:not-observed:erosion`,
    grid,
    zeroValues.slice(),
  );
  const tint = new Float32Array(cells * 3);
  const occlusion = new Float32Array(cells);
  const roughness = new Float32Array(cells);
  dataset.cells.forEach((cell, index) => {
    const offset = index * 3;
    const color = terrainMaterialTint(
      cell.material,
      elevationValues[index] ?? 0,
      cell.validity === "valid",
    );
    tint[offset] = color[0];
    tint[offset + 1] = color[1];
    tint[offset + 2] = color[2];
    occlusion[index] = cell.validity === "valid" ? 0.92 : 0;
    roughness[index] = cell.validity === "valid" ? 0.88 : 1;
  });
  const material = materialField(
    "material",
    `${REGIONAL_TERRAIN_SCENE_COMPILER_REVISION}:material:${dataset.dataset_id}`,
    grid,
    tint,
    occlusion,
    roughness,
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
    field_set_id: `${TERRAIN_FIELD_SCHEMA}|${program.program_id}|${dataset.dataset_id}|${REGIONAL_TERRAIN_SCENE_COMPILER_REVISION}`,
    program_id: program.program_id,
    working_set_id: `admitted-grid:${dataset.dataset_id}`,
    active_band_ids: Object.freeze(["admitted_dem"]),
    detail_authority: dataset.authority,
    source_revision: dataset.dataset_id,
    grid,
    elevation_scale: elevationScale,
    validity,
    elevation,
    rainfall,
    flow_direction: flowDirection,
    flow_accumulation: flowAccumulation,
    erosion,
    normal: relief.normal,
    curvature: relief.curvature,
    material,
    field_cells: cells,
    field_bytes: fields.reduce(
      (total, field) => total + fieldByteLength(field),
      0,
    ),
  });
}

export function projectRegionalTerrainBounds(
  sceneBounds: RegionalBounds,
  sourceBounds: RegionalBounds,
  world: { width: number; height: number },
): FieldBounds {
  const frame = regionalTerrainCanvasFrame(world);
  const northwest = projectRegionalTerrainPosition(
    sceneBounds,
    [sourceBounds.west_microdegrees, sourceBounds.north_microdegrees],
    world,
  );
  const southeast = projectRegionalTerrainPosition(
    sceneBounds,
    [sourceBounds.east_microdegrees, sourceBounds.south_microdegrees],
    world,
  );
  const width = southeast.x - northwest.x;
  const height = southeast.y - northwest.y;
  if (width <= 0 || height <= 0)
    throw new Error("regional terrain bounds do not fit the admitted scene");
  return Object.freeze({
    x: Math.max(frame.x, northwest.x),
    y: Math.max(frame.y, northwest.y),
    width: Math.min(frame.width, width),
    height: Math.min(frame.height, height),
  });
}

export function projectRegionalTerrainPosition(
  sceneBounds: RegionalBounds,
  position: readonly [number, number],
  world: { width: number; height: number },
) {
  if (sceneBounds.crosses_antimeridian)
    throw new Error("regional terrain grids do not yet cross the antimeridian");
  const longitudeSpan =
    sceneBounds.east_microdegrees - sceneBounds.west_microdegrees;
  const latitudeSpan =
    sceneBounds.north_microdegrees - sceneBounds.south_microdegrees;
  if (longitudeSpan <= 0 || latitudeSpan <= 0)
    throw new Error("regional terrain scene bounds are invalid");
  const frame = regionalTerrainCanvasFrame(world);
  return Object.freeze({
    x:
      frame.x +
      ((position[0] - sceneBounds.west_microdegrees) / longitudeSpan) *
        frame.width,
    y:
      frame.y +
      ((sceneBounds.north_microdegrees - position[1]) / latitudeSpan) *
        frame.height,
  });
}

export function invertRegionalTerrainPosition(
  sceneBounds: RegionalBounds,
  point: { x: number; y: number },
  world: { width: number; height: number },
): readonly [number, number] {
  if (sceneBounds.crosses_antimeridian)
    throw new Error("regional terrain grids do not yet cross the antimeridian");
  const frame = regionalTerrainCanvasFrame(world);
  if (
    !Number.isFinite(point.x) ||
    !Number.isFinite(point.y) ||
    frame.width <= 0 ||
    frame.height <= 0
  )
    throw new Error("regional terrain inverse requires a finite point");
  const longitude =
    sceneBounds.west_microdegrees +
    ((point.x - frame.x) / frame.width) *
      (sceneBounds.east_microdegrees - sceneBounds.west_microdegrees);
  const latitude =
    sceneBounds.north_microdegrees -
    ((point.y - frame.y) / frame.height) *
      (sceneBounds.north_microdegrees - sceneBounds.south_microdegrees);
  return Object.freeze([longitude, latitude]);
}

export function projectRegionalTerrainFootprint(
  frame: CountyFrame,
  sceneBounds: RegionalBounds,
  footprint: CountyFootprint,
  world: { width: number; height: number },
): ProjectedCountyFootprint {
  const screenRings = footprint.rings.map((ring) =>
    ring.map((position) => {
      const screen = projectRegionalTerrainPosition(
        sceneBounds,
        position,
        world,
      );
      const local = nativePositionToCountyLocal(frame, position);
      return Object.freeze({
        ...local,
        x: screen.x,
        y: screen.y,
      });
    }),
  );
  return Object.freeze({
    ...footprint,
    path: screenRings
      .map(
        (ring) =>
          ring
            .map(
              ({ x, y }, index) =>
                `${index === 0 ? "M" : "L"}${x.toFixed(2)} ${y.toFixed(2)}`,
            )
            .join(" ") + " Z",
      )
      .join(" "),
    screen_rings: Object.freeze(screenRings.map((ring) => Object.freeze(ring))),
  });
}

export function regionalTerrainCanvasFrame(world: {
  width: number;
  height: number;
}): FieldBounds {
  return Object.freeze({
    x: TERRAIN_CANVAS_INSET_X,
    y: TERRAIN_CANVAS_INSET_Y,
    width: world.width - TERRAIN_CANVAS_INSET_X * 2,
    height: world.height - TERRAIN_CANVAS_INSET_Y * 2,
  });
}

function terrainMaterialTint(
  material: string | null,
  elevation: number,
  valid: boolean,
): readonly [number, number, number] {
  if (!valid) return [0, 0, 0];
  const palettes: Record<string, readonly [number, number, number]> = {
    granite: [0.48, 0.5, 0.46],
    rock: [0.44, 0.46, 0.43],
    sand: [0.58, 0.54, 0.42],
    soil: [0.4, 0.43, 0.35],
    vegetation: [0.31, 0.42, 0.32],
  };
  const base = palettes[material ?? ""] ?? [0.41, 0.46, 0.39];
  const lift = elevation * 0.16;
  return [
    Math.min(1, base[0] + lift),
    Math.min(1, base[1] + lift),
    Math.min(1, base[2] + lift),
  ];
}
