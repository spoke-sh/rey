import type { TerrainFieldSetInput } from "@rey/explorer";
import {
  TERRAIN_FIELD_SCHEMA,
  createFieldGrid,
  fieldByteLength,
  fieldCellCount,
  maskField,
  materialField,
  scalarField,
  vectorField,
} from "../engine/fields";
import type { TerrainFieldSet } from "./compile";
import { deriveTerrainNormals } from "./normals";

export const REGIONAL_TERRAIN_MOSAIC_SCHEMA =
  "rey.landscape-mosaic.v1" as const;
export const REGIONAL_TERRAIN_MOSAIC_REVISION =
  "rey.terrain.regional-mosaic@1" as const;
export const MAXIMUM_REGIONAL_TERRAIN_MOSAIC_CELLS = 2_000_000;

export interface RegionalTerrainMosaicPatch {
  member_id: string;
  scene_id: string;
  role: "detail" | "overview";
  field: TerrainFieldSet;
}

export interface RegionalTerrainMosaicManifest {
  schema: typeof REGIONAL_TERRAIN_MOSAIC_SCHEMA;
  implementation_revision: typeof REGIONAL_TERRAIN_MOSAIC_REVISION;
  mosaic_id: string;
  composition_revision: string;
  primary_patch_id: string;
  patch_ids: readonly string[];
  member_ids: readonly string[];
  coordinate_reference: string;
  vertical_reference: string;
  bounds: { x: number; y: number; width: number; height: number };
  columns: number;
  rows: number;
  valid_vertices: number;
  no_data_vertices: number;
  unsupported_vertices: number;
  shared_vertices: number;
  overlap_pairs: readonly (readonly [string, string])[];
  overlap_policy: "qualified_shared_samples_must_match_before_derivation";
  gap_policy: "unsupported_remains_transparent";
  limits: { maximum_cells: number };
  omissions: readonly string[];
  patches: readonly {
    member_id: string;
    scene_id: string;
    field_set_id: string;
    source_revision: string;
    authority: string;
    role: "detail" | "overview";
    sample_spacing_x: number;
    sample_spacing_y: number;
    column_offset: number;
    row_offset: number;
    columns: number;
    rows: number;
  }[];
}

export interface CompiledRegionalTerrainMosaic {
  manifest: RegionalTerrainMosaicManifest;
  field: TerrainFieldSet;
}

export function compileRegionalTerrainMosaic(
  patches: readonly RegionalTerrainMosaicPatch[],
  primaryPatchId: string,
  compositionRevision: string,
  coordinateReference: string,
  verticalReference: string,
  maximumCells = MAXIMUM_REGIONAL_TERRAIN_MOSAIC_CELLS,
): CompiledRegionalTerrainMosaic {
  if (
    patches.length === 0 ||
    !compositionRevision ||
    !coordinateReference ||
    !verticalReference ||
    !Number.isSafeInteger(maximumCells) ||
    maximumCells < 4
  )
    throw new Error("regional terrain mosaic contract is incomplete");
  const ordered = [...patches].sort(
    (left, right) =>
      left.member_id.localeCompare(right.member_id) ||
      left.field.field_set_id.localeCompare(right.field.field_set_id),
  );
  if (
    new Set(ordered.map(({ member_id }) => member_id)).size !==
      ordered.length ||
    new Set(ordered.map(({ field }) => field.field_set_id)).size !==
      ordered.length ||
    !ordered.some(({ field }) => field.field_set_id === primaryPatchId)
  )
    throw new Error("regional terrain mosaic patch identity is invalid");
  for (let left = 0; left < ordered.length; left += 1)
    for (let right = left + 1; right < ordered.length; right += 1)
      if (
        positiveAreaOverlap(
          ordered[left]!.field.grid.bounds,
          ordered[right]!.field.grid.bounds,
        )
      )
        throw new Error(
          "regional terrain mosaic does not admit overlapping patch areas",
        );

  const bounds = unionBounds(ordered.map(({ field }) => field.grid.bounds));
  const spacingX = sampleSpacing(ordered[0]!.field, "x");
  const spacingY = sampleSpacing(ordered[0]!.field, "y");
  const elevationScale = ordered[0]!.field.elevation_scale;
  const placements = ordered.map((patch) => {
    const { field } = patch;
    if (
      !sameNumber(sampleSpacing(field, "x"), spacingX) ||
      !sameNumber(sampleSpacing(field, "y"), spacingY) ||
      !sameNumber(field.elevation_scale, elevationScale)
    )
      throw new Error("regional terrain mosaic patch scale is incompatible");
    const columnOffset = alignedOffset(field.grid.bounds.x, bounds.x, spacingX);
    const rowOffset = alignedOffset(field.grid.bounds.y, bounds.y, spacingY);
    return Object.freeze({
      patch,
      column_offset: columnOffset,
      row_offset: rowOffset,
    });
  });
  const columns = alignedExtent(bounds.width, spacingX) + 1;
  const rows = alignedExtent(bounds.height, spacingY) + 1;
  const cells = columns * rows;
  if (!Number.isSafeInteger(cells) || cells > maximumCells)
    throw new Error("regional terrain mosaic exceeds its cell budget");
  const grid = createFieldGrid(columns, rows, bounds);
  const occupancy = new Int32Array(cells).fill(-1);
  const validityValues = new Uint8Array(cells);
  const elevationValues = new Float32Array(cells);
  const rainfallValues = new Float32Array(cells);
  const flowDirectionValues = new Float32Array(cells * 2);
  const flowAccumulationValues = new Float32Array(cells);
  const erosionValues = new Float32Array(cells);
  const tintValues = new Float32Array(cells * 3);
  const occlusionValues = new Float32Array(cells);
  const roughnessValues = new Float32Array(cells);
  const overlapPairs = new Set<string>();
  let sharedVertices = 0;

  for (const [patchIndex, placement] of placements.entries()) {
    const source = placement.patch.field;
    for (let row = 0; row < source.grid.rows; row += 1) {
      for (let column = 0; column < source.grid.columns; column += 1) {
        const sourceIndex = row * source.grid.columns + column;
        const targetIndex =
          (placement.row_offset + row) * columns +
          placement.column_offset +
          column;
        const owner = occupancy[targetIndex]!;
        if (owner >= 0) {
          const ownerPatch = placements[owner]!.patch;
          const pair = [
            ownerPatch.field.field_set_id,
            source.field_set_id,
          ].sort((left, right) => left.localeCompare(right));
          overlapPairs.add(`${pair[0]}\u0000${pair[1]}`);
          sharedVertices += 1;
          verifySharedSample(
            validityValues,
            elevationValues,
            tintValues,
            occlusionValues,
            roughnessValues,
            targetIndex,
            source,
            sourceIndex,
          );
          continue;
        }
        occupancy[targetIndex] = patchIndex;
        copySample(
          validityValues,
          elevationValues,
          rainfallValues,
          flowDirectionValues,
          flowAccumulationValues,
          erosionValues,
          tintValues,
          occlusionValues,
          roughnessValues,
          targetIndex,
          source,
          sourceIndex,
        );
      }
    }
  }

  const revision = `${REGIONAL_TERRAIN_MOSAIC_REVISION}:${compositionRevision}`;
  const validity = maskField(
    "validity",
    `${revision}:validity`,
    grid,
    validityValues,
  );
  const elevation = scalarField(
    "elevation",
    `${revision}:elevation`,
    grid,
    elevationValues,
  );
  const rainfall = scalarField(
    "rainfall",
    `${revision}:rainfall`,
    grid,
    rainfallValues,
  );
  const flowDirection = vectorField(
    "flow_direction",
    `${revision}:flow-direction`,
    grid,
    2,
    flowDirectionValues,
  );
  const flowAccumulation = scalarField(
    "flow_accumulation",
    `${revision}:flow-accumulation`,
    grid,
    flowAccumulationValues,
  );
  const erosion = scalarField(
    "erosion",
    `${revision}:erosion`,
    grid,
    erosionValues,
  );
  const relief = deriveTerrainNormals(elevation, validity, elevationScale, {
    normal: `${revision}:normal`,
    curvature: `${revision}:curvature`,
  });
  const material = materialField(
    "material",
    `${revision}:material`,
    grid,
    tintValues,
    occlusionValues,
    roughnessValues,
  );
  const patchIds = Object.freeze(
    ordered.map(({ field }) => field.field_set_id),
  );
  const memberIds = Object.freeze(ordered.map(({ member_id }) => member_id));
  const mosaicId = [
    REGIONAL_TERRAIN_MOSAIC_SCHEMA,
    REGIONAL_TERRAIN_MOSAIC_REVISION,
    compositionRevision,
    primaryPatchId,
    ...ordered.flatMap(({ member_id, field }) => [
      member_id,
      field.field_set_id,
      field.source_revision,
    ]),
    `${columns}x${rows}`,
  ].join("|");
  const validVertices = validityValues.reduce(
    (total, value) => total + (value === 0 ? 0 : 1),
    0,
  );
  const unsupportedVertices = occupancy.reduce(
    (total, owner) => total + (owner < 0 ? 1 : 0),
    0,
  );
  const noDataVertices = occupancy.reduce(
    (total, owner, index) =>
      total + (owner >= 0 && validityValues[index] === 0 ? 1 : 0),
    0,
  );
  const pairs = Object.freeze(
    [...overlapPairs]
      .sort((left, right) => left.localeCompare(right))
      .map((pair) => Object.freeze(pair.split("\u0000") as [string, string])),
  );
  const binding = Object.freeze({
    schema: "rey.landscape-mosaic-binding.v1" as const,
    mosaic_id: mosaicId,
    composition_revision: compositionRevision,
    primary_patch_id: primaryPatchId,
    patch_ids: patchIds,
    overlap_pairs: pairs,
    bounds,
    coordinate_reference: coordinateReference,
    vertical_reference: verticalReference,
    overlap_policy:
      "qualified_shared_samples_must_match_before_derivation" as const,
    gap_policy: "unsupported_remains_transparent" as const,
  }) satisfies NonNullable<TerrainFieldSetInput["landscape_mosaic"]>;
  const manifest = Object.freeze({
    schema: REGIONAL_TERRAIN_MOSAIC_SCHEMA,
    implementation_revision: REGIONAL_TERRAIN_MOSAIC_REVISION,
    mosaic_id: mosaicId,
    composition_revision: compositionRevision,
    primary_patch_id: primaryPatchId,
    patch_ids: patchIds,
    member_ids: memberIds,
    coordinate_reference: coordinateReference,
    vertical_reference: verticalReference,
    bounds,
    columns,
    rows,
    valid_vertices: validVertices,
    no_data_vertices: noDataVertices,
    unsupported_vertices: unsupportedVertices,
    shared_vertices: sharedVertices,
    overlap_pairs: pairs,
    overlap_policy: binding.overlap_policy,
    gap_policy: binding.gap_policy,
    limits: Object.freeze({ maximum_cells: maximumCells }),
    omissions: Object.freeze(
      unsupportedVertices > 0
        ? [
            `${unsupportedVertices} mosaic vertices have no admitted terrain source and remain unsupported`,
          ]
        : [],
    ),
    patches: Object.freeze(
      placements.map(({ patch, column_offset, row_offset }) =>
        Object.freeze({
          member_id: patch.member_id,
          scene_id: patch.scene_id,
          field_set_id: patch.field.field_set_id,
          source_revision: patch.field.source_revision,
          authority: patch.field.detail_authority,
          role: patch.role,
          sample_spacing_x: sampleSpacing(patch.field, "x"),
          sample_spacing_y: sampleSpacing(patch.field, "y"),
          column_offset,
          row_offset,
          columns: patch.field.grid.columns,
          rows: patch.field.grid.rows,
        }),
      ),
    ),
  }) satisfies RegionalTerrainMosaicManifest;
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
  const sourceMinimum = Math.min(
    ...ordered.map(({ field }) => field.source_summary?.elevation_minimum ?? 0),
  );
  const sourceMaximum = Math.max(
    ...ordered.map(({ field }) => field.source_summary?.elevation_maximum ?? 0),
  );
  const field = Object.freeze({
    schema: TERRAIN_FIELD_SCHEMA,
    field_set_id: `${TERRAIN_FIELD_SCHEMA}|${mosaicId}`,
    program_id: `regional-mosaic:${compositionRevision}`,
    working_set_id: `regional-mosaic:${mosaicId}`,
    active_band_ids: Object.freeze([
      "admitted_dem",
      "admitted_multi_region_mosaic",
    ]),
    detail_authority:
      "shared-frame mosaic of a connected terrain-qualified regional set; exact shared samples must agree, source no-data and unsupported gaps retain zero validity, and no overlap or gap is resolved by draw order",
    source_revision: mosaicId,
    source_summary: Object.freeze({
      columns,
      rows,
      valid_vertices: validVertices,
      no_data_vertices: noDataVertices + unsupportedVertices,
      elevation_minimum: sourceMinimum,
      elevation_maximum: sourceMaximum,
    }),
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
    field_cells: fieldCellCount(grid),
    field_bytes: fields.reduce(
      (total, channel) => total + fieldByteLength(channel),
      0,
    ),
    landscape_mosaic: binding,
  }) satisfies TerrainFieldSet;
  return Object.freeze({ manifest, field });
}

function sampleSpacing(field: TerrainFieldSet, axis: "x" | "y"): number {
  return axis === "x"
    ? field.grid.bounds.width / (field.grid.columns - 1)
    : field.grid.bounds.height / (field.grid.rows - 1);
}

function alignedOffset(value: number, origin: number, spacing: number): number {
  const offset = (value - origin) / spacing;
  const rounded = Math.round(offset);
  if (!sameNumber(offset, rounded))
    throw new Error("regional terrain mosaic patch origin is not aligned");
  return rounded;
}

function alignedExtent(value: number, spacing: number): number {
  const extent = value / spacing;
  const rounded = Math.round(extent);
  if (!sameNumber(extent, rounded))
    throw new Error("regional terrain mosaic bounds are not aligned");
  return rounded;
}

function unionBounds(
  values: readonly TerrainFieldSet["grid"]["bounds"][],
): TerrainFieldSet["grid"]["bounds"] {
  const left = Math.min(...values.map(({ x }) => x));
  const top = Math.min(...values.map(({ y }) => y));
  const right = Math.max(...values.map(({ x, width }) => x + width));
  const bottom = Math.max(...values.map(({ y, height }) => y + height));
  return Object.freeze({
    x: left,
    y: top,
    width: right - left,
    height: bottom - top,
  });
}

function positiveAreaOverlap(
  left: TerrainFieldSet["grid"]["bounds"],
  right: TerrainFieldSet["grid"]["bounds"],
): boolean {
  return (
    Math.min(left.x + left.width, right.x + right.width) -
      Math.max(left.x, right.x) >
      0.000_001 &&
    Math.min(left.y + left.height, right.y + right.height) -
      Math.max(left.y, right.y) >
      0.000_001
  );
}

function verifySharedSample(
  validity: Uint8Array,
  elevation: Float32Array,
  tint: Float32Array,
  occlusion: Float32Array,
  roughness: Float32Array,
  targetIndex: number,
  source: TerrainFieldSet,
  sourceIndex: number,
): void {
  if (validity[targetIndex] !== source.validity.values[sourceIndex])
    throw new Error("regional terrain mosaic shared validity conflicts");
  if (validity[targetIndex] === 0) return;
  if (
    elevation[targetIndex] !== source.elevation.values[sourceIndex] ||
    occlusion[targetIndex] !== source.material.occlusion[sourceIndex] ||
    roughness[targetIndex] !== source.material.roughness[sourceIndex]
  )
    throw new Error("regional terrain mosaic shared elevation conflicts");
  const targetOffset = targetIndex * 3;
  const sourceOffset = sourceIndex * 3;
  for (let component = 0; component < 3; component += 1)
    if (
      tint[targetOffset + component] !==
      source.material.tint[sourceOffset + component]
    )
      throw new Error("regional terrain mosaic shared material conflicts");
}

function copySample(
  validity: Uint8Array,
  elevation: Float32Array,
  rainfall: Float32Array,
  flowDirection: Float32Array,
  flowAccumulation: Float32Array,
  erosion: Float32Array,
  tint: Float32Array,
  occlusion: Float32Array,
  roughness: Float32Array,
  targetIndex: number,
  source: TerrainFieldSet,
  sourceIndex: number,
): void {
  validity[targetIndex] = source.validity.values[sourceIndex]!;
  elevation[targetIndex] = source.elevation.values[sourceIndex]!;
  rainfall[targetIndex] = source.rainfall.values[sourceIndex]!;
  flowAccumulation[targetIndex] = source.flow_accumulation.values[sourceIndex]!;
  erosion[targetIndex] = source.erosion.values[sourceIndex]!;
  occlusion[targetIndex] = source.material.occlusion[sourceIndex]!;
  roughness[targetIndex] = source.material.roughness[sourceIndex]!;
  const targetVectorOffset = targetIndex * 2;
  const sourceVectorOffset = sourceIndex * 2;
  flowDirection[targetVectorOffset] =
    source.flow_direction.values[sourceVectorOffset]!;
  flowDirection[targetVectorOffset + 1] =
    source.flow_direction.values[sourceVectorOffset + 1]!;
  const targetColorOffset = targetIndex * 3;
  const sourceColorOffset = sourceIndex * 3;
  for (let component = 0; component < 3; component += 1)
    tint[targetColorOffset + component] =
      source.material.tint[sourceColorOffset + component]!;
}

function sameNumber(left: number, right: number): boolean {
  return (
    Math.abs(left - right) <=
    Math.max(1, Math.abs(left), Math.abs(right)) * 1e-6
  );
}
