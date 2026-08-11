import type { ProjectionPacket } from "../../domain";
import {
  TERRAIN_FIELD_SCHEMA,
  fieldByteLength,
  fieldCellCount,
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

export interface TerrainFieldSet {
  schema: typeof TERRAIN_FIELD_SCHEMA;
  field_set_id: string;
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

export interface TerrainFieldCompilation {
  source_id: string;
  source_revision: string;
  grid: FieldGrid;
  anchors: readonly TerrainAnchorSample[];
  atmosphere: readonly TerrainAtmosphereSample[];
  unresolved_pressure: number;
  projection: ProjectionPacket;
}

export function compileTerrainFields(
  input: TerrainFieldCompilation,
): TerrainFieldSet {
  const cells = fieldCellCount(input.grid);
  if (
    input.grid.columns !== input.projection.field_layout.columns ||
    input.grid.rows !== input.projection.field_layout.rows ||
    cells !== input.projection.field_layout.cells
  )
    throw new Error(
      `terrain field layout ${input.projection.field_layout.columns}×${input.projection.field_layout.rows} does not match ${input.grid.columns}×${input.grid.rows}`,
    );
  const elevationScaleRatio = Number(
    input.projection.projection_basis.parameters.elevation_scale_ratio,
  );
  if (!Number.isFinite(elevationScaleRatio) || elevationScaleRatio <= 0)
    throw new Error("projection packet has no valid elevation scale ratio");
  const elevationScale =
    Math.min(input.grid.bounds.width, input.grid.bounds.height) *
    elevationScaleRatio;
  if (cells > input.projection.limits.max_field_cells)
    throw new Error(
      `terrain field cell limit ${input.projection.limits.max_field_cells} exceeded by ${cells}`,
    );
  const revision = (channel: string) => {
    const field = input.projection.field_channels.find(
      (candidate) => candidate.id === channel,
    );
    if (!field) throw new Error(`projection packet omits ${channel} channel`);
    return field.implementation.semantic_digest;
  };
  const anchor = deriveAnchorElevation(input.grid, input.anchors, {
    validity: revision("validity"),
    elevation: revision("elevation"),
  });
  const hydrology = deriveHydrology(
    input.source_id,
    anchor.elevation,
    anchor.validity,
    input.atmosphere,
    input.unresolved_pressure,
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
    {
      normal: revision("normal"),
      curvature: revision("curvature"),
    },
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
  if (fields.length > input.projection.limits.max_field_channels)
    throw new Error(
      `terrain channel limit ${input.projection.limits.max_field_channels} exceeded by ${fields.length}`,
    );
  if (fieldBytes > input.projection.limits.max_field_bytes)
    throw new Error(
      `terrain byte limit ${input.projection.limits.max_field_bytes} exceeded by ${fieldBytes}`,
    );
  if (fieldBytes !== input.projection.field_layout.total_bytes)
    throw new Error(
      `terrain field allocation ${fieldBytes} does not match packet allocation ${input.projection.field_layout.total_bytes}`,
    );
  const fieldSet = Object.freeze({
    schema: TERRAIN_FIELD_SCHEMA,
    field_set_id: [
      TERRAIN_FIELD_SCHEMA,
      input.source_id,
      input.source_revision,
      `${input.grid.columns}x${input.grid.rows}`,
      `${input.grid.bounds.x},${input.grid.bounds.y},${input.grid.bounds.width},${input.grid.bounds.height}`,
      `elevation-scale:${elevationScale}`,
      ...fields.map((field) => field.implementation_revision),
    ].join("|"),
    source_revision: input.source_revision,
    grid: input.grid,
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
  verifyTerrainFields(fieldSet, input.projection);
  return fieldSet;
}

export function verifyTerrainFields(
  fields: TerrainFieldSet,
  projection: ProjectionPacket,
): void {
  if (fields.schema !== TERRAIN_FIELD_SCHEMA)
    throw new Error("unsupported terrain field schema");
  if (
    fields.field_cells !== fieldCellCount(fields.grid) ||
    fields.field_cells !== projection.field_layout.cells ||
    fields.grid.columns !== projection.field_layout.columns ||
    fields.grid.rows !== projection.field_layout.rows ||
    fields.field_bytes !== projection.field_layout.total_bytes ||
    fields.field_cells > projection.limits.max_field_cells ||
    fields.field_bytes > projection.limits.max_field_bytes
  )
    throw new Error("terrain field limits or shape are invalid");
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
