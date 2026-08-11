import type { ProjectionFieldLevel, ProjectionPacket } from "../../domain";
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
import type { LensRegime } from "../engine/camera";
import { deriveAnchorElevation, type TerrainAnchorSample } from "./elevation";
import { deriveHydrology, type TerrainAtmosphereSample } from "./hydrology";
import { deriveTerrainMaterial } from "./materials";
import { deriveTerrainNormals } from "./normals";

export interface TerrainFieldSet {
  schema: typeof TERRAIN_FIELD_SCHEMA;
  field_set_id: string;
  level_id: string;
  sample_stride: number;
  regimes: readonly LensRegime[];
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

export const TERRAIN_FIELD_PYRAMID_SCHEMA =
  "rey.terrain-field-pyramid.v1" as const;

export interface TerrainFieldPyramid {
  schema: typeof TERRAIN_FIELD_PYRAMID_SCHEMA;
  pyramid_id: string;
  source_revision: string;
  levels: readonly TerrainFieldSet[];
  total_cells: number;
  total_bytes: number;
  stable_coordinate_rule: string;
}

export interface TerrainFieldCompilation {
  source_id: string;
  source_revision: string;
  level: ProjectionFieldLevel;
  grid: FieldGrid;
  anchors: readonly TerrainAnchorSample[];
  atmosphere: readonly TerrainAtmosphereSample[];
  unresolved_pressure: number;
  projection: ProjectionPacket;
}

export interface TerrainFieldPyramidCompilation {
  source_id: string;
  source_revision: string;
  bounds: FieldBounds;
  anchors: readonly TerrainAnchorSample[];
  atmosphere: readonly TerrainAtmosphereSample[];
  unresolved_pressure: number;
  projection: ProjectionPacket;
}

export function compileTerrainFieldPyramid(
  input: TerrainFieldPyramidCompilation,
): TerrainFieldPyramid {
  if (input.projection.field_pyramid.schema !== TERRAIN_FIELD_PYRAMID_SCHEMA)
    throw new Error("unsupported terrain field pyramid schema");
  if (input.source_revision !== input.projection.source_topography_revision)
    throw new Error("terrain field pyramid source revision is not bound");
  const levels = input.projection.field_pyramid.levels.map((level) =>
    compileTerrainFields({
      ...input,
      level,
      grid: createFieldGrid(level.columns, level.rows, input.bounds),
    }),
  );
  const pyramid = Object.freeze({
    schema: TERRAIN_FIELD_PYRAMID_SCHEMA,
    pyramid_id: [
      TERRAIN_FIELD_PYRAMID_SCHEMA,
      input.projection.packet_id,
      input.source_id,
      input.source_revision,
      ...levels.map((level) => level.field_set_id),
    ].join("|"),
    source_revision: input.source_revision,
    levels: Object.freeze(levels),
    total_cells: levels.reduce((total, level) => total + level.field_cells, 0),
    total_bytes: levels.reduce((total, level) => total + level.field_bytes, 0),
    stable_coordinate_rule:
      input.projection.field_pyramid.stable_coordinate_rule,
  });
  verifyTerrainFieldPyramid(pyramid, input.projection);
  return pyramid;
}

export function terrainFieldForRegime(
  pyramid: TerrainFieldPyramid,
  regime: LensRegime,
): TerrainFieldSet {
  const level = pyramid.levels.find((candidate) =>
    candidate.regimes.includes(regime),
  );
  if (!level)
    throw new Error(`terrain field pyramid has no level for ${regime}`);
  return level;
}

export function compileTerrainFields(
  input: TerrainFieldCompilation,
): TerrainFieldSet {
  const cells = fieldCellCount(input.grid);
  const declaredLevel = input.projection.field_pyramid.levels.find(
    (level) => level.level_id === input.level.level_id,
  );
  if (
    !declaredLevel ||
    declaredLevel.columns !== input.level.columns ||
    declaredLevel.rows !== input.level.rows ||
    declaredLevel.cells !== input.level.cells ||
    declaredLevel.bytes_per_cell !== input.level.bytes_per_cell ||
    declaredLevel.total_bytes !== input.level.total_bytes ||
    declaredLevel.sample_stride !== input.level.sample_stride ||
    declaredLevel.regimes.join("|") !== input.level.regimes.join("|") ||
    declaredLevel.detail_authority !== input.level.detail_authority ||
    input.grid.columns !== input.level.columns ||
    input.grid.rows !== input.level.rows ||
    cells !== input.level.cells
  )
    throw new Error(
      `terrain field level ${input.level.level_id} does not match its projection packet layout`,
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
  if (fieldBytes !== input.level.total_bytes)
    throw new Error(
      `terrain field allocation ${fieldBytes} does not match level allocation ${input.level.total_bytes}`,
    );
  const fieldSet = Object.freeze({
    schema: TERRAIN_FIELD_SCHEMA,
    field_set_id: [
      TERRAIN_FIELD_SCHEMA,
      input.source_id,
      input.source_revision,
      input.level.level_id,
      `${input.grid.columns}x${input.grid.rows}`,
      `${input.grid.bounds.x},${input.grid.bounds.y},${input.grid.bounds.width},${input.grid.bounds.height}`,
      `elevation-scale:${elevationScale}`,
      ...fields.map((field) => field.implementation_revision),
    ].join("|"),
    level_id: input.level.level_id,
    sample_stride: input.level.sample_stride,
    regimes: Object.freeze([...input.level.regimes]),
    detail_authority: input.level.detail_authority,
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
  const level = projection.field_pyramid.levels.find(
    (candidate) => candidate.level_id === fields.level_id,
  );
  if (
    !level ||
    fields.field_cells !== fieldCellCount(fields.grid) ||
    fields.field_cells !== level.cells ||
    fields.grid.columns !== level.columns ||
    fields.grid.rows !== level.rows ||
    fields.field_bytes !== level.total_bytes ||
    fields.sample_stride !== level.sample_stride ||
    fields.detail_authority !== level.detail_authority ||
    fields.regimes.join("|") !== level.regimes.join("|") ||
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

export function verifyTerrainFieldPyramid(
  pyramid: TerrainFieldPyramid,
  projection: ProjectionPacket,
): void {
  if (
    pyramid.schema !== TERRAIN_FIELD_PYRAMID_SCHEMA ||
    pyramid.levels.length !== projection.field_pyramid.levels.length ||
    pyramid.levels.length > projection.limits.max_field_levels ||
    pyramid.total_cells !== projection.field_pyramid.total_cells ||
    pyramid.total_bytes !== projection.field_pyramid.total_bytes ||
    pyramid.total_cells > projection.limits.max_total_field_cells ||
    pyramid.total_bytes > projection.limits.max_total_field_bytes ||
    pyramid.stable_coordinate_rule !==
      projection.field_pyramid.stable_coordinate_rule
  )
    throw new Error("terrain field pyramid limits or identity are invalid");
  const finest = pyramid.levels.find((level) => level.sample_stride === 1);
  if (!finest) throw new Error("terrain field pyramid has no finest level");
  const levelIds = new Set<string>();
  const sampleStrides = new Set<number>();
  const regimes = new Set<LensRegime>();
  for (const level of pyramid.levels) {
    verifyTerrainFields(level, projection);
    if (levelIds.has(level.level_id))
      throw new Error("terrain field pyramid repeats a level identity");
    levelIds.add(level.level_id);
    if (sampleStrides.has(level.sample_stride))
      throw new Error("terrain field pyramid repeats a sample stride");
    sampleStrides.add(level.sample_stride);
    for (const regime of level.regimes) {
      if (regimes.has(regime))
        throw new Error("terrain field pyramid repeats a semantic regime");
      regimes.add(regime);
    }
    if (
      (finest.grid.columns - 1) / level.sample_stride + 1 !==
        level.grid.columns ||
      (finest.grid.rows - 1) / level.sample_stride + 1 !== level.grid.rows ||
      level.grid.bounds.x !== finest.grid.bounds.x ||
      level.grid.bounds.y !== finest.grid.bounds.y ||
      level.grid.bounds.width !== finest.grid.bounds.width ||
      level.grid.bounds.height !== finest.grid.bounds.height
    )
      throw new Error(
        "terrain field pyramid levels do not share nested coordinates",
      );
  }
  if (
    [...regimes].sort().join("|") !==
    [
      "atlas",
      "evidence",
      "landscape",
      "neighborhoods",
      "objects",
      "world",
    ].join("|")
  )
    throw new Error(
      "terrain field pyramid does not cover every semantic regime",
    );
}
