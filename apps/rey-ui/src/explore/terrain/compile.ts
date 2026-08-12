import type { ProjectionPacket, ProjectionTerrainBand } from "../../domain";
import {
  TERRAIN_FIELD_SCHEMA,
  createFieldGrid,
  fieldByteLength,
  fieldCellCount,
  type FieldBounds,
  type FieldGrid,
  type MaskField2D,
  type MaterialField2D,
  type ScalarField2D,
  type VectorField2D,
} from "../engine/fields";
import { deriveAnchorElevation, type TerrainAnchorSample } from "./elevation";
import { deriveHydrology, type TerrainAtmosphereSample } from "./hydrology";
import { deriveTerrainMaterial } from "./materials";
import { deriveTerrainNormals } from "./normals";

export const COMPILED_TERRAIN_PROGRAM_SCHEMA =
  "rey.compiled-terrain-program.v1" as const;

export interface TerrainProgram {
  schema: typeof COMPILED_TERRAIN_PROGRAM_SCHEMA;
  program_id: string;
  source_id: string;
  source_revision: string;
  bounds: FieldBounds;
  anchors: readonly TerrainAnchorSample[];
  atmosphere: readonly TerrainAtmosphereSample[];
  unresolved_pressure: number;
  projection: ProjectionPacket;
}

export interface TerrainFieldSet {
  schema: typeof TERRAIN_FIELD_SCHEMA;
  field_set_id: string;
  program_id: string;
  working_set_id: string;
  active_band_ids: readonly string[];
  detail_authority: string;
  source_revision: string;
  grid: FieldGrid;
  elevation_scale: number;
  validity: MaskField2D;
  elevation: ScalarField2D;
  rainfall: ScalarField2D;
  flow_direction: VectorField2D;
  flow_accumulation: ScalarField2D;
  erosion: ScalarField2D;
  normal: VectorField2D;
  curvature: ScalarField2D;
  material: MaterialField2D;
  field_cells: number;
  field_bytes: number;
}

export interface TerrainProgramCompilation {
  source_id: string;
  source_revision: string;
  bounds: FieldBounds;
  anchors: readonly TerrainAnchorSample[];
  atmosphere: readonly TerrainAtmosphereSample[];
  unresolved_pressure: number;
  projection: ProjectionPacket;
}

export interface TerrainWorkingSetRequest {
  working_set_id: string;
  bounds: FieldBounds;
  columns: number;
  rows: number;
  detail_authority: string;
}

export interface TerrainCameraView {
  world_width: number;
  world_height: number;
  viewport_width: number;
  viewport_height: number;
  rendered_scale: number;
  pan_x: number;
  pan_y: number;
}

export function terrainWorkingSetForView(
  program: TerrainProgram,
  view: TerrainCameraView,
): TerrainWorkingSetRequest {
  const working = program.projection.terrain_program.working_set;
  const scale = Math.max(0.000_001, view.rendered_scale);
  const targetSpacing = working.target_sample_spacing_pixels / scale;
  const centerX = view.world_width / 2 - view.pan_x / scale;
  const centerY = view.world_height / 2 - view.pan_y / scale;
  const overscan = targetSpacing * working.overscan_samples;
  const requested = {
    x: centerX - view.viewport_width / scale / 2 - overscan,
    y: centerY - view.viewport_height / scale / 2 - overscan,
    width: view.viewport_width / scale + overscan * 2,
    height: view.viewport_height / scale + overscan * 2,
  };
  const left = Math.max(
    program.bounds.x,
    Math.floor(requested.x / targetSpacing) * targetSpacing,
  );
  const top = Math.max(
    program.bounds.y,
    Math.floor(requested.y / targetSpacing) * targetSpacing,
  );
  const right = Math.min(
    program.bounds.x + program.bounds.width,
    Math.ceil((requested.x + requested.width) / targetSpacing) * targetSpacing,
  );
  const bottom = Math.min(
    program.bounds.y + program.bounds.height,
    Math.ceil((requested.y + requested.height) / targetSpacing) * targetSpacing,
  );
  const bounds =
    right > left && bottom > top
      ? { x: left, y: top, width: right - left, height: bottom - top }
      : { ...program.bounds };
  let columns = Math.min(
    working.max_columns,
    Math.max(2, Math.ceil(bounds.width / targetSpacing) + 1),
  );
  let rows = Math.min(
    working.max_rows,
    Math.max(2, Math.ceil(bounds.height / targetSpacing) + 1),
  );
  if (columns * rows > working.max_cells) {
    const reduction = Math.sqrt(working.max_cells / (columns * rows));
    columns = Math.max(2, Math.floor(columns * reduction));
    rows = Math.max(2, Math.floor(rows * reduction));
  }
  return {
    working_set_id: [
      "camera",
      bounds.x.toFixed(4),
      bounds.y.toFixed(4),
      bounds.width.toFixed(4),
      bounds.height.toFixed(4),
      `${columns}x${rows}`,
    ].join(":"),
    bounds,
    columns,
    rows,
    detail_authority:
      "transient camera-relative evaluation of the admitted terrain program",
  };
}

export function compileTerrainProgram(
  input: TerrainProgramCompilation,
): TerrainProgram {
  const declared = input.projection.terrain_program;
  if (declared.schema !== "rey.terrain-program.v1")
    throw new Error("unsupported terrain program schema");
  if (input.source_revision !== input.projection.source_topography_revision)
    throw new Error("terrain program source revision is not bound");
  if (
    input.bounds.width <= 0 ||
    input.bounds.height <= 0 ||
    declared.bands.length === 0 ||
    declared.bands.length > input.projection.limits.max_terrain_bands
  )
    throw new Error("terrain program shape or limits are invalid");
  const program = Object.freeze({
    schema: COMPILED_TERRAIN_PROGRAM_SCHEMA,
    program_id: [
      COMPILED_TERRAIN_PROGRAM_SCHEMA,
      input.projection.packet_id,
      input.source_id,
      input.source_revision,
      declared.evaluator.semantic_digest,
      declared.seed,
      `${input.bounds.x},${input.bounds.y},${input.bounds.width},${input.bounds.height}`,
    ].join("|"),
    source_id: input.source_id,
    source_revision: input.source_revision,
    bounds: Object.freeze({ ...input.bounds }),
    anchors: Object.freeze(
      input.anchors.map((anchor) => Object.freeze({ ...anchor })),
    ),
    atmosphere: Object.freeze(
      input.atmosphere.map((sample) => Object.freeze({ ...sample })),
    ),
    unresolved_pressure: input.unresolved_pressure,
    projection: input.projection,
  });
  return program;
}

export function materializeTerrainWorkingSet(
  program: TerrainProgram,
  request: TerrainWorkingSetRequest,
): TerrainFieldSet {
  const declared = program.projection.terrain_program;
  const working = declared.working_set;
  if (
    !Number.isInteger(request.columns) ||
    !Number.isInteger(request.rows) ||
    request.columns < 2 ||
    request.rows < 2 ||
    request.columns > working.max_columns ||
    request.rows > working.max_rows
  )
    throw new Error("terrain working-set dimensions are invalid");
  const grid = createFieldGrid(request.columns, request.rows, request.bounds);
  const cells = fieldCellCount(grid);
  const expectedBytes = cells * working.bytes_per_cell;
  if (
    cells > working.max_cells ||
    cells > program.projection.limits.max_working_set_cells ||
    expectedBytes > working.max_bytes ||
    expectedBytes > program.projection.limits.max_working_set_bytes
  )
    throw new Error("terrain working-set allocation exceeds its packet limits");

  const elevationScaleRatio = Number(
    program.projection.projection_basis.parameters.elevation_scale_ratio,
  );
  if (!Number.isFinite(elevationScaleRatio) || elevationScaleRatio <= 0)
    throw new Error("projection packet has no valid elevation scale ratio");
  const elevationScale =
    Math.min(program.bounds.width, program.bounds.height) * elevationScaleRatio;
  const revision = (channel: string) => {
    const field = program.projection.field_channels.find(
      (candidate) => candidate.id === channel,
    );
    if (!field) throw new Error(`projection packet omits ${channel} channel`);
    return field.implementation.semantic_digest;
  };
  const spacing = Math.max(
    request.bounds.width / Math.max(1, request.columns - 1),
    request.bounds.height / Math.max(1, request.rows - 1),
  );
  const activeBands = declared.bands.filter(
    (band) =>
      band.wavelength_scene_units / spacing >=
      band.minimum_samples_per_wavelength,
  );
  const anchor = deriveAnchorElevation(
    grid,
    program.anchors,
    { validity: revision("validity"), elevation: revision("elevation") },
    {
      seed: declared.seed,
      bands: activeBands,
    },
  );
  const hydrology = deriveHydrology(
    program.source_id,
    anchor.elevation,
    anchor.validity,
    program.atmosphere,
    program.unresolved_pressure,
    {
      rainfall: revision("rainfall"),
      flow_direction: revision("flow_direction"),
      flow_accumulation: revision("flow_accumulation"),
      erosion: revision("erosion"),
      elevation: revision("elevation"),
    },
  );
  const relief = deriveTerrainNormals(
    hydrology.elevation,
    anchor.validity,
    elevationScale,
    { normal: revision("normal"), curvature: revision("curvature") },
  );
  const material = deriveTerrainMaterial(
    hydrology.elevation,
    relief.normal,
    relief.curvature,
    hydrology.flow_accumulation,
    anchor.validity,
    revision("material"),
  );
  const fields = [
    anchor.validity,
    hydrology.elevation,
    hydrology.rainfall,
    hydrology.flow_direction,
    hydrology.flow_accumulation,
    hydrology.erosion,
    relief.normal,
    relief.curvature,
    material,
  ] as const;
  const fieldBytes = fields.reduce(
    (total, field) => total + fieldByteLength(field),
    0,
  );
  if (fields.length > program.projection.limits.max_field_channels)
    throw new Error("terrain channel limit exceeded");
  if (fieldBytes !== expectedBytes)
    throw new Error(
      `terrain working-set allocation ${fieldBytes} does not match declared ${expectedBytes}`,
    );
  const result = Object.freeze({
    schema: TERRAIN_FIELD_SCHEMA,
    field_set_id: [
      TERRAIN_FIELD_SCHEMA,
      program.program_id,
      request.working_set_id,
      `${grid.columns}x${grid.rows}`,
      `${grid.bounds.x},${grid.bounds.y},${grid.bounds.width},${grid.bounds.height}`,
      ...activeBands.map((band) => band.band_id),
      ...fields.map((field) => field.implementation_revision),
    ].join("|"),
    program_id: program.program_id,
    working_set_id: request.working_set_id,
    active_band_ids: Object.freeze(activeBands.map((band) => band.band_id)),
    detail_authority: request.detail_authority,
    source_revision: program.source_revision,
    grid,
    elevation_scale: elevationScale,
    validity: anchor.validity,
    elevation: hydrology.elevation,
    rainfall: hydrology.rainfall,
    flow_direction: hydrology.flow_direction,
    flow_accumulation: hydrology.flow_accumulation,
    erosion: hydrology.erosion,
    normal: relief.normal,
    curvature: relief.curvature,
    material,
    field_cells: cells,
    field_bytes: fieldBytes,
  });
  verifyTerrainWorkingSet(result, program);
  return result;
}

export function verifyTerrainWorkingSet(
  fields: TerrainFieldSet,
  program: TerrainProgram,
): void {
  const working = program.projection.terrain_program.working_set;
  if (
    fields.schema !== TERRAIN_FIELD_SCHEMA ||
    fields.program_id !== program.program_id ||
    fields.source_revision !== program.source_revision ||
    fields.field_cells !== fieldCellCount(fields.grid) ||
    fields.field_cells > working.max_cells ||
    fields.field_bytes > working.max_bytes
  )
    throw new Error("terrain working-set identity or limits are invalid");
  const sameGrid = [
    fields.validity,
    fields.elevation,
    fields.rainfall,
    fields.flow_direction,
    fields.flow_accumulation,
    fields.erosion,
    fields.normal,
    fields.curvature,
    fields.material,
  ].every(
    (field) =>
      field.grid.columns === fields.grid.columns &&
      field.grid.rows === fields.grid.rows &&
      field.grid.bounds.x === fields.grid.bounds.x &&
      field.grid.bounds.y === fields.grid.bounds.y &&
      field.grid.bounds.width === fields.grid.bounds.width &&
      field.grid.bounds.height === fields.grid.bounds.height,
  );
  if (!sameGrid) throw new Error("terrain fields do not share one exact grid");
}

export function terrainBandsForSpacing(
  bands: readonly ProjectionTerrainBand[],
  sampleSpacing: number,
): readonly ProjectionTerrainBand[] {
  return bands.filter(
    (band) =>
      band.wavelength_scene_units / sampleSpacing >=
      band.minimum_samples_per_wavelength,
  );
}
