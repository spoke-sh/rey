import {
  createTerrainValidityClassification,
  TERRAIN_VALIDITY_NO_DATA,
  TERRAIN_VALIDITY_VALID,
} from "@rey/explorer";
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
import { regionalTerrainGridValueColumns } from "../terrain/regional-grid-transport";
import type {
  CountyFrame,
  CountyFootprint,
  ProjectedCountyFootprint,
} from "./county-frame";
import { nativePositionToCountyLocal } from "./county-frame";

export const REGIONAL_TERRAIN_SCENE_COMPILER_REVISION =
  "rey.explorer.regional-terrain-grid@4";

export interface RegionalTerrainLandscapeFrame {
  frame_id: string;
  member_scene_ids: readonly string[];
  native_bounds: RegionalBounds;
  elevation_minimum: number;
  elevation_maximum: number;
  coordinate_reference: "native_crs84_projected_to_shared_landscape_frame";
  vertical_reference: string;
}

const TERRAIN_CANVAS_INSET_X = 96;
const TERRAIN_CANVAS_INSET_Y = 72;
const REGIONAL_ELEVATION_HEIGHT_RATIO = 0.24;

/**
 * Pads the admitted region's shown geographic span before it's projected
 * into the landscape frame, so landing settles into a wider halo of context
 * around the region instead of the data's own tight bounds filling the
 * frame edge-to-edge. Applied symmetrically (and to both the forward and
 * inverse projection) so every overlay derived from these functions —
 * footprints, contours, the Atlas<->Landscape target frame — shifts
 * together rather than drifting out of registration with the terrain.
 */
export const REGIONAL_TERRAIN_FRAME_PADDING_RATIO = 0.3;

/**
 * Keyed by the admitted scene's own object identity (stable across renders
 * — admittedRegionalScenes hands back the same `result.scene` reference
 * from the portfolio, not a freshly built object each call), with `world`
 * as a nested key for defensive correctness even though it's always the
 * same TOPOLOGY_WORLD constant in practice. Nothing here invalidates the
 * cache explicitly; a scene that's genuinely re-admitted with new data
 * gets a new object reference from the portfolio, which is simply a miss.
 *
 * Every scene compile (buildTopologyScene, run fresh on every focusId/
 * regime change — nothing upstream of this caches either) was redoing this
 * same admitted-DEM compile (elevation summary, per-cell material, normal
 * derivation) from scratch, including for scenes whose underlying data
 * hadn't changed at all. That's real, synchronous, main-thread work,
 * landing squarely on interactions like the Atlas-to-Landscape zoom that
 * need to keep painting frames to read as a morph rather than a stall.
 */
const regionalTerrainFieldCache = new WeakMap<
  AdmittedRegionalScene,
  Map<string, TerrainFieldSet | null>
>();

export function compileRegionalTerrainField(
  scene: AdmittedRegionalScene,
  world: { width: number; height: number },
  landscapeFrame?: RegionalTerrainLandscapeFrame,
): TerrainFieldSet | null {
  const worldKey = `${world.width}x${world.height}|${landscapeFrame?.frame_id ?? "scene-local-frame"}`;
  let byWorld = regionalTerrainFieldCache.get(scene);
  if (!byWorld) {
    byWorld = new Map();
    regionalTerrainFieldCache.set(scene, byWorld);
  }
  if (byWorld.has(worldKey)) return byWorld.get(worldKey)!;
  const field = compileRegionalTerrainFieldUncached(
    scene,
    world,
    landscapeFrame,
  );
  byWorld.set(worldKey, field);
  return field;
}

function compileRegionalTerrainFieldUncached(
  scene: AdmittedRegionalScene,
  world: { width: number; height: number },
  landscapeFrame?: RegionalTerrainLandscapeFrame,
): TerrainFieldSet | null {
  const program = scene.projection.terrain;
  const dataset = program?.grid;
  if (!program || !dataset) return null;
  const datasetValues = regionalTerrainGridValueColumns(dataset);
  const bounds = projectRegionalTerrainBounds(
    landscapeFrame?.native_bounds ?? scene.native_bounds,
    dataset.native_bounds,
    world,
  );
  const grid = createFieldGrid(dataset.columns, dataset.rows, bounds);
  const cells = fieldCellCount(grid);
  if (datasetValues.validity.length !== cells)
    throw new Error("regional terrain dataset shape changed after admission");
  const validityValues = datasetValues.validity.slice();
  const elevationSummary = regionalTerrainElevationSummary(
    validityValues,
    datasetValues.elevation_micrometers,
  );
  const minimumElevation =
    landscapeFrame?.elevation_minimum ?? elevationSummary.minimum;
  const maximumElevation =
    landscapeFrame?.elevation_maximum ?? elevationSummary.maximum;
  if (
    elevationSummary.minimum < minimumElevation ||
    elevationSummary.maximum > maximumElevation
  )
    throw new Error(
      "regional terrain field exceeds its landscape elevation frame",
    );
  const elevationRange = Math.max(1, maximumElevation - minimumElevation);
  const elevationValues = new Float32Array(cells);
  for (let index = 0; index < cells; index += 1) {
    if (validityValues[index] === 1)
      elevationValues[index] =
        (datasetValues.elevation_micrometers[index]! / 1_000_000 -
          minimumElevation) /
        elevationRange;
  }
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
  for (let index = 0; index < cells; index += 1) {
    const offset = index * 3;
    const valid = validityValues[index] === 1;
    const materialIndex = datasetValues.material_indices[index]!;
    const color = terrainMaterialTint(
      valid ? datasetValues.material_palette[materialIndex]! : null,
      elevationValues[index] ?? 0,
      valid,
    );
    tint[offset] = color[0];
    tint[offset + 1] = color[1];
    tint[offset + 2] = color[2];
    occlusion[index] = valid ? 0.92 : 0;
    roughness[index] = valid ? 0.88 : 1;
  }
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
  const validityClassification = createTerrainValidityClassification(
    Uint8Array.from(validityValues, (value) =>
      value === 0 ? TERRAIN_VALIDITY_NO_DATA : TERRAIN_VALIDITY_VALID,
    ),
    `${REGIONAL_TERRAIN_SCENE_COMPILER_REVISION}:validity-classification:${dataset.dataset_id}`,
  );
  return Object.freeze({
    schema: TERRAIN_FIELD_SCHEMA,
    field_set_id: `${TERRAIN_FIELD_SCHEMA}|${program.program_id}|${dataset.dataset_id}|${REGIONAL_TERRAIN_SCENE_COMPILER_REVISION}|${landscapeFrame?.frame_id ?? "scene-local-frame"}`,
    program_id: program.program_id,
    working_set_id: `admitted-grid:${dataset.dataset_id}`,
    active_band_ids: Object.freeze(["admitted_dem"]),
    detail_authority: landscapeFrame
      ? `${dataset.authority}; projected into shared frame ${landscapeFrame.frame_id} with one qualified component-wide elevation normalization`
      : dataset.authority,
    source_revision: dataset.dataset_id,
    source_summary: Object.freeze({
      columns: dataset.columns,
      rows: dataset.rows,
      valid_vertices: elevationSummary.valid_count,
      no_data_vertices: cells - elevationSummary.valid_count,
      unsupported_vertices: 0,
      elevation_minimum: elevationSummary.minimum,
      elevation_maximum: elevationSummary.maximum,
    }),
    grid,
    elevation_scale: elevationScale,
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
    relief_metrics: regionalTerrainReliefMetrics(
      dataset.native_bounds,
      dataset.columns,
      dataset.rows,
      elevationRange,
    ),
    field_cells: cells,
    field_bytes: fields.reduce(
      (total, field) => total + fieldByteLength(field),
      validityClassification.values.byteLength,
    ),
  });
}

function regionalTerrainReliefMetrics(
  bounds: RegionalBounds,
  columns: number,
  rows: number,
  elevationRangeMeters: number,
) {
  const centerLatitudeRadians =
    (((bounds.south_microdegrees + bounds.north_microdegrees) / 2) * Math.PI) /
    180_000_000;
  const longitudeSpanDegrees =
    (bounds.east_microdegrees - bounds.west_microdegrees) / 1_000_000;
  const latitudeSpanDegrees =
    (bounds.north_microdegrees - bounds.south_microdegrees) / 1_000_000;
  const sampleSpacingX =
    (longitudeSpanDegrees *
      111_320 *
      Math.max(0.01, Math.cos(centerLatitudeRadians))) /
    (columns - 1);
  const sampleSpacingY = (latitudeSpanDegrees * 110_574) / (rows - 1);
  if (
    !Number.isFinite(sampleSpacingX) ||
    sampleSpacingX <= 0 ||
    !Number.isFinite(sampleSpacingY) ||
    sampleSpacingY <= 0
  )
    throw new Error("regional terrain metric sample spacing is invalid");
  return Object.freeze({
    schema: "rey.terrain-relief-metrics.v1" as const,
    sample_spacing_x_meters: sampleSpacingX,
    sample_spacing_y_meters: sampleSpacingY,
    elevation_range_meters: elevationRangeMeters,
    authority:
      "local metric relief scale derived from the exact CRS84 grid bounds, dimensions, center latitude, and admitted elevation range; presentation only and not a geodetic transform",
  });
}

export function compileRegionalTerrainLandscapeFrame(
  scenes: readonly AdmittedRegionalScene[],
  compositionRevision: string,
): RegionalTerrainLandscapeFrame {
  if (scenes.length === 0 || !compositionRevision)
    throw new Error("regional terrain landscape frame has no admitted members");
  const members = scenes
    .map((scene) => {
      const dataset = scene.projection.terrain?.grid;
      if (!dataset)
        throw new Error(
          "regional terrain landscape member has no terrain grid",
        );
      if (scene.native_bounds.crosses_antimeridian)
        throw new Error(
          "regional terrain landscape cannot cross the antimeridian",
        );
      const values = regionalTerrainGridValueColumns(dataset);
      return {
        scene,
        dataset,
        elevation: regionalTerrainElevationSummary(
          values.validity,
          values.elevation_micrometers,
        ),
      };
    })
    .sort((left, right) =>
      left.scene.scene_id.localeCompare(right.scene.scene_id),
    );
  const nativeBounds = Object.freeze({
    west_microdegrees: Math.min(
      ...members.map(({ scene }) => scene.native_bounds.west_microdegrees),
    ),
    south_microdegrees: Math.min(
      ...members.map(({ scene }) => scene.native_bounds.south_microdegrees),
    ),
    east_microdegrees: Math.max(
      ...members.map(({ scene }) => scene.native_bounds.east_microdegrees),
    ),
    north_microdegrees: Math.max(
      ...members.map(({ scene }) => scene.native_bounds.north_microdegrees),
    ),
    crosses_antimeridian: false,
  });
  const elevationMinimum = Math.min(
    ...members.map(({ elevation }) => elevation.minimum),
  );
  const elevationMaximum = Math.max(
    ...members.map(({ elevation }) => elevation.maximum),
  );
  const memberSceneIds = Object.freeze(
    members.map(({ scene }) => scene.scene_id),
  );
  return Object.freeze({
    frame_id: [
      "rey.regional-terrain-landscape-frame.v1",
      compositionRevision,
      ...members.flatMap(({ scene, dataset }) => [
        scene.scene_id,
        dataset.dataset_id,
      ]),
      `${nativeBounds.west_microdegrees},${nativeBounds.south_microdegrees},${nativeBounds.east_microdegrees},${nativeBounds.north_microdegrees}`,
      `${elevationMinimum},${elevationMaximum}`,
    ].join("|"),
    member_scene_ids: memberSceneIds,
    native_bounds: nativeBounds,
    elevation_minimum: elevationMinimum,
    elevation_maximum: elevationMaximum,
    coordinate_reference: "native_crs84_projected_to_shared_landscape_frame",
    vertical_reference: `qualified_shared_elevation_meters:${compositionRevision}`,
  });
}

export function regionalTerrainElevationSummary(
  validity: Uint8Array,
  elevationMicrometers: readonly number[],
): { valid_count: number; minimum: number; maximum: number } {
  if (validity.length !== elevationMicrometers.length)
    throw new Error("regional terrain elevation channels changed shape");
  let validCount = 0;
  let minimum = Number.POSITIVE_INFINITY;
  let maximum = Number.NEGATIVE_INFINITY;
  for (let index = 0; index < validity.length; index += 1) {
    if (validity[index] !== 1) continue;
    const elevation = elevationMicrometers[index]! / 1_000_000;
    validCount += 1;
    minimum = Math.min(minimum, elevation);
    maximum = Math.max(maximum, elevation);
  }
  if (validCount < 3)
    throw new Error("regional terrain dataset has no supported triangle");
  return Object.freeze({
    valid_count: validCount,
    minimum,
    maximum,
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
  const padded = paddedRegionalTerrainBounds(sceneBounds);
  const longitudeSpan = padded.east_microdegrees - padded.west_microdegrees;
  const latitudeSpan = padded.north_microdegrees - padded.south_microdegrees;
  if (longitudeSpan <= 0 || latitudeSpan <= 0)
    throw new Error("regional terrain scene bounds are invalid");
  const frame = regionalTerrainCanvasFrame(world);
  return Object.freeze({
    x:
      frame.x +
      ((position[0] - padded.west_microdegrees) / longitudeSpan) * frame.width,
    y:
      frame.y +
      ((padded.north_microdegrees - position[1]) / latitudeSpan) * frame.height,
  });
}

export function invertRegionalTerrainPosition(
  sceneBounds: RegionalBounds,
  point: { x: number; y: number },
  world: { width: number; height: number },
): readonly [number, number] {
  if (sceneBounds.crosses_antimeridian)
    throw new Error("regional terrain grids do not yet cross the antimeridian");
  const padded = paddedRegionalTerrainBounds(sceneBounds);
  const frame = regionalTerrainCanvasFrame(world);
  if (
    !Number.isFinite(point.x) ||
    !Number.isFinite(point.y) ||
    frame.width <= 0 ||
    frame.height <= 0
  )
    throw new Error("regional terrain inverse requires a finite point");
  const longitude =
    padded.west_microdegrees +
    ((point.x - frame.x) / frame.width) *
      (padded.east_microdegrees - padded.west_microdegrees);
  const latitude =
    padded.north_microdegrees -
    ((point.y - frame.y) / frame.height) *
      (padded.north_microdegrees - padded.south_microdegrees);
  return Object.freeze([longitude, latitude]);
}

function paddedRegionalTerrainBounds(bounds: RegionalBounds): RegionalBounds {
  const longitudeSpan = bounds.east_microdegrees - bounds.west_microdegrees;
  const latitudeSpan = bounds.north_microdegrees - bounds.south_microdegrees;
  const longitudePad = longitudeSpan * REGIONAL_TERRAIN_FRAME_PADDING_RATIO;
  const latitudePad = latitudeSpan * REGIONAL_TERRAIN_FRAME_PADDING_RATIO;
  return {
    ...bounds,
    west_microdegrees: bounds.west_microdegrees - longitudePad,
    east_microdegrees: bounds.east_microdegrees + longitudePad,
    south_microdegrees: bounds.south_microdegrees - latitudePad,
    north_microdegrees: bounds.north_microdegrees + latitudePad,
  };
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

/**
 * Keeps material hue and hypsometric context continuous. Quantized elevation
 * exposed the source triangulation as broad color plates and fought the
 * relief engine's illumination instead of reading as cartographic terrain.
 */
function terrainMaterialTint(
  material: string | null,
  elevation: number,
  valid: boolean,
): readonly [number, number, number] {
  if (!valid) return [0, 0, 0];
  const palettes: Record<string, readonly [number, number, number]> = {
    granite: [0.66, 0.66, 0.61],
    rock: [0.64, 0.64, 0.59],
    sand: [0.76, 0.72, 0.58],
    soil: [0.69, 0.62, 0.48],
    vegetation: [0.53, 0.65, 0.47],
  };
  const base = palettes[material ?? ""] ?? [0.61, 0.67, 0.55];
  const lift = (Math.max(0, Math.min(1, elevation)) - 0.5) * 0.055;
  return [
    Math.min(1, base[0] + lift),
    Math.min(1, base[1] + lift),
    Math.min(1, base[2] + lift),
  ];
}
