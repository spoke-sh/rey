import type { ProjectionPacket, ProjectionTerrainBand } from "../../domain";
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
  type FieldGrid,
  type MaskField2D,
  type MaterialField2D,
  type ScalarField2D,
  type VectorField2D,
} from "../engine/fields";
import { deriveAnchorElevation, type TerrainAnchorSample } from "./elevation";
import {
  HYDROLOGY_PROPAGATION_STEPS,
  deriveHydrology,
  type TerrainAtmosphereSample,
} from "./hydrology";
import { deriveTerrainMaterial } from "./materials";
import { deriveTerrainNormals } from "./normals";

export const COMPILED_TERRAIN_PROGRAM_SCHEMA =
  "rey.compiled-terrain-program.v1" as const;
export const TERRAIN_PATCH_COMPILER_REVISION =
  "rey.terrain.absolute-patches@1" as const;
export const TERRAIN_PATCH_HALO_SAMPLES = HYDROLOGY_PROPAGATION_STEPS + 2;

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
  source_summary?: {
    columns: number;
    rows: number;
    valid_vertices: number;
    no_data_vertices: number;
    elevation_minimum: number;
    elevation_maximum: number;
  };
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
  render_window?: TerrainRenderWindow;
}

export interface TerrainRenderWindow {
  column_offset: number;
  row_offset: number;
  columns: number;
  rows: number;
  bounds: FieldBounds;
  halo_samples: number;
}

export interface TerrainCameraView {
  world_width: number;
  world_height: number;
  viewport_width: number;
  viewport_height: number;
  rendered_scale: number;
  pan_x: number;
  pan_y: number;
  pitch_degrees?: number;
  yaw_degrees?: number;
  model_transform?: {
    scale_x: number;
    scale_z: number;
    translate_x: number;
    translate_z: number;
    elevation_scale: number;
  };
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

export function terrainPatchRequestsForView(
  program: TerrainProgram,
  view: TerrainCameraView,
  maxPatchColumns = 65,
  maxPatchRows = 65,
): readonly TerrainWorkingSetRequest[] {
  if (
    !Number.isInteger(maxPatchColumns) ||
    !Number.isInteger(maxPatchRows) ||
    maxPatchColumns < 3 ||
    maxPatchRows < 3
  )
    throw new Error("terrain patch dimensions are invalid");
  let request = terrainWorkingSetForView(program, view);
  const working = program.projection.terrain_program.working_set;
  const maximumCells = Math.min(
    working.max_cells,
    program.projection.limits.max_working_set_cells,
    Math.floor(
      Math.min(
        working.max_bytes,
        program.projection.limits.max_working_set_bytes,
      ) / working.bytes_per_cell,
    ),
  );
  for (;;) {
    if (request.columns <= maxPatchColumns && request.rows <= maxPatchRows)
      return Object.freeze([request]);
    const patches = compileTerrainPatches(
      request,
      maxPatchColumns,
      maxPatchRows,
    );
    const cells = patches.reduce(
      (total, patch) => total + patch.columns * patch.rows,
      0,
    );
    if (cells <= maximumCells) return Object.freeze(patches);
    const reduction = Math.min(0.98, Math.sqrt(maximumCells / cells) * 0.98);
    const columns = Math.max(
      2,
      Math.min(
        request.columns - 1,
        Math.floor((request.columns - 1) * reduction) + 1,
      ),
    );
    const rows = Math.max(
      2,
      Math.min(
        request.rows - 1,
        Math.floor((request.rows - 1) * reduction) + 1,
      ),
    );
    request = {
      ...request,
      working_set_id: `${request.working_set_id}:budget:${columns}x${rows}`,
      columns,
      rows,
      detail_authority:
        "transient camera-relative evaluation reduced deterministically to reserve bounded terrain-patch halos",
    };
  }
}

function compileTerrainPatches(
  request: TerrainWorkingSetRequest,
  maxPatchColumns: number,
  maxPatchRows: number,
): TerrainWorkingSetRequest[] {
  const columnRanges = overlappingRanges(request.columns, maxPatchColumns);
  const rowRanges = overlappingRanges(request.rows, maxPatchRows);
  const columnSpacing = request.bounds.width / (request.columns - 1);
  const rowSpacing = request.bounds.height / (request.rows - 1);
  return rowRanges.flatMap(([rowStart, rows]) =>
    columnRanges.map(([columnStart, columns]) => {
      const computeColumnStart = Math.max(
        0,
        columnStart - TERRAIN_PATCH_HALO_SAMPLES,
      );
      const computeRowStart = Math.max(
        0,
        rowStart - TERRAIN_PATCH_HALO_SAMPLES,
      );
      const computeColumnEnd = Math.min(
        request.columns,
        columnStart + columns + TERRAIN_PATCH_HALO_SAMPLES,
      );
      const computeRowEnd = Math.min(
        request.rows,
        rowStart + rows + TERRAIN_PATCH_HALO_SAMPLES,
      );
      const computeColumns = computeColumnEnd - computeColumnStart;
      const computeRows = computeRowEnd - computeRowStart;
      const renderBounds = Object.freeze({
        x: request.bounds.x + columnStart * columnSpacing,
        y: request.bounds.y + rowStart * rowSpacing,
        width: (columns - 1) * columnSpacing,
        height: (rows - 1) * rowSpacing,
      });
      return Object.freeze({
        working_set_id: [
          TERRAIN_PATCH_COMPILER_REVISION,
          `${renderBounds.x.toFixed(4)},${renderBounds.y.toFixed(4)}`,
          `${renderBounds.width.toFixed(4)},${renderBounds.height.toFixed(4)}`,
          `${columns}x${rows}`,
          `halo:${TERRAIN_PATCH_HALO_SAMPLES}`,
        ].join(":"),
        bounds: Object.freeze({
          x: request.bounds.x + computeColumnStart * columnSpacing,
          y: request.bounds.y + computeRowStart * rowSpacing,
          width: (computeColumns - 1) * columnSpacing,
          height: (computeRows - 1) * rowSpacing,
        }),
        columns: computeColumns,
        rows: computeRows,
        detail_authority:
          "bounded absolute-coordinate terrain patch with finite hydrology and relief halos; one shared render border preserves neighboring channel identity",
        render_window: Object.freeze({
          column_offset: columnStart - computeColumnStart,
          row_offset: rowStart - computeRowStart,
          columns,
          rows,
          bounds: renderBounds,
          halo_samples: TERRAIN_PATCH_HALO_SAMPLES,
        }),
      });
    }),
  );
}

function overlappingRanges(
  sampleCount: number,
  maximumPatchSamples: number,
): Array<readonly [number, number]> {
  const ranges: Array<readonly [number, number]> = [];
  let start = 0;
  while (start < sampleCount - 1) {
    const count = Math.min(maximumPatchSamples, sampleCount - start);
    ranges.push([start, count]);
    start += count - 1;
  }
  return ranges;
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
    program.bounds,
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
    hydrology.flow_accumulation,
    anchor.validity,
    revision("material"),
  );
  const computedFields = [
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
  const computedBytes = computedFields.reduce(
    (total, field) => total + fieldByteLength(field),
    0,
  );
  if (computedFields.length > program.projection.limits.max_field_channels)
    throw new Error("terrain channel limit exceeded");
  if (computedBytes !== expectedBytes)
    throw new Error(
      `terrain working-set allocation ${computedBytes} does not match declared ${expectedBytes}`,
    );
  const cropped = request.render_window
    ? cropTerrainFields(
        request.render_window,
        anchor.validity,
        hydrology.elevation,
        hydrology.rainfall,
        hydrology.flow_direction,
        hydrology.flow_accumulation,
        hydrology.erosion,
        relief.normal,
        relief.curvature,
        material,
      )
    : {
        grid,
        validity: anchor.validity,
        elevation: hydrology.elevation,
        rainfall: hydrology.rainfall,
        flow_direction: hydrology.flow_direction,
        flow_accumulation: hydrology.flow_accumulation,
        erosion: hydrology.erosion,
        normal: relief.normal,
        curvature: relief.curvature,
        material,
      };
  const fields = [
    cropped.validity,
    cropped.elevation,
    cropped.rainfall,
    cropped.flow_direction,
    cropped.flow_accumulation,
    cropped.erosion,
    cropped.normal,
    cropped.curvature,
    cropped.material,
  ] as const;
  const fieldBytes = fields.reduce(
    (total, field) => total + fieldByteLength(field),
    0,
  );
  const result = Object.freeze({
    schema: TERRAIN_FIELD_SCHEMA,
    field_set_id: [
      TERRAIN_FIELD_SCHEMA,
      program.program_id,
      request.working_set_id,
      `${cropped.grid.columns}x${cropped.grid.rows}`,
      `${cropped.grid.bounds.x},${cropped.grid.bounds.y},${cropped.grid.bounds.width},${cropped.grid.bounds.height}`,
      ...activeBands.map((band) => band.band_id),
      ...fields.map((field) => field.implementation_revision),
    ].join("|"),
    program_id: program.program_id,
    working_set_id: request.working_set_id,
    active_band_ids: Object.freeze(activeBands.map((band) => band.band_id)),
    detail_authority: request.detail_authority,
    source_revision: program.source_revision,
    grid: cropped.grid,
    elevation_scale: elevationScale,
    validity: cropped.validity,
    elevation: cropped.elevation,
    rainfall: cropped.rainfall,
    flow_direction: cropped.flow_direction,
    flow_accumulation: cropped.flow_accumulation,
    erosion: cropped.erosion,
    normal: cropped.normal,
    curvature: cropped.curvature,
    material: cropped.material,
    field_cells: fieldCellCount(cropped.grid),
    field_bytes: fieldBytes,
  });
  verifyTerrainWorkingSet(result, program);
  return result;
}

function cropTerrainFields(
  window: TerrainRenderWindow,
  validity: MaskField2D,
  elevation: ScalarField2D,
  rainfall: ScalarField2D,
  flowDirection: VectorField2D,
  flowAccumulation: ScalarField2D,
  erosion: ScalarField2D,
  normal: VectorField2D,
  curvature: ScalarField2D,
  material: MaterialField2D,
) {
  const source = validity.grid;
  if (
    !Number.isInteger(window.column_offset) ||
    !Number.isInteger(window.row_offset) ||
    !Number.isInteger(window.columns) ||
    !Number.isInteger(window.rows) ||
    window.column_offset < 0 ||
    window.row_offset < 0 ||
    window.columns < 2 ||
    window.rows < 2 ||
    window.column_offset + window.columns > source.columns ||
    window.row_offset + window.rows > source.rows ||
    window.halo_samples < TERRAIN_PATCH_HALO_SAMPLES
  )
    throw new Error("terrain patch render window or halo is invalid");
  const spacingX = source.bounds.width / (source.columns - 1);
  const spacingY = source.bounds.height / (source.rows - 1);
  const expectedBounds = {
    x: source.bounds.x + window.column_offset * spacingX,
    y: source.bounds.y + window.row_offset * spacingY,
    width: (window.columns - 1) * spacingX,
    height: (window.rows - 1) * spacingY,
  };
  if (
    !sameNumber(expectedBounds.x, window.bounds.x) ||
    !sameNumber(expectedBounds.y, window.bounds.y) ||
    !sameNumber(expectedBounds.width, window.bounds.width) ||
    !sameNumber(expectedBounds.height, window.bounds.height)
  )
    throw new Error("terrain patch render bounds do not match its grid");
  const grid = createFieldGrid(window.columns, window.rows, window.bounds);
  const scalar = (field: ScalarField2D) =>
    scalarField(
      field.channel,
      field.implementation_revision,
      grid,
      cropComponents(field.values, source, window, 1),
    );
  const vector = (field: VectorField2D) =>
    vectorField(
      field.channel,
      field.implementation_revision,
      grid,
      field.components,
      cropComponents(field.values, source, window, field.components),
    );
  return {
    grid,
    validity: maskField(
      validity.channel,
      validity.implementation_revision,
      grid,
      cropComponents(validity.values, source, window, 1),
    ),
    elevation: scalar(elevation),
    rainfall: scalar(rainfall),
    flow_direction: vector(flowDirection),
    flow_accumulation: scalar(flowAccumulation),
    erosion: scalar(erosion),
    normal: vector(normal),
    curvature: scalar(curvature),
    material: materialField(
      material.channel,
      material.implementation_revision,
      grid,
      cropComponents(material.tint, source, window, 3),
      cropComponents(material.occlusion, source, window, 1),
      cropComponents(material.roughness, source, window, 1),
    ),
  };
}

function cropComponents<T extends Float32Array | Int8Array | Uint8Array>(
  sourceValues: T,
  sourceGrid: FieldGrid,
  window: TerrainRenderWindow,
  components: number,
): T {
  const Constructor = sourceValues.constructor as {
    new (length: number): T;
  };
  const values = new Constructor(window.columns * window.rows * components);
  for (let row = 0; row < window.rows; row += 1) {
    const sourceOffset =
      ((window.row_offset + row) * sourceGrid.columns + window.column_offset) *
      components;
    const targetOffset = row * window.columns * components;
    values.set(
      sourceValues.subarray(
        sourceOffset,
        sourceOffset + window.columns * components,
      ),
      targetOffset,
    );
  }
  return values;
}

function sameNumber(left: number, right: number): boolean {
  return (
    Math.abs(left - right) <= Number.EPSILON * Math.max(1, left, right) * 8
  );
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
