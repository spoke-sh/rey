import {
  createTerrainValidityClassification,
  landscapeReliefFieldByteLength,
  sampleLandscapeReliefField,
  summarizeTerrainValidityClassification,
  TERRAIN_VALIDITY_NO_DATA,
  TERRAIN_VALIDITY_UNSUPPORTED,
  TERRAIN_VALIDITY_VALID,
  terrainTriangleIndices,
  verifyTerrainFieldValidityClassification,
  type LandscapeReliefField,
  type TerrainMeshData,
} from "@rey/explorer";
import {
  createFieldGrid,
  fieldByteLength,
  fieldCellCount,
  maskField,
  materialField,
  scalarField,
  vectorField,
  type FieldBounds,
} from "../engine/fields";
import type { TerrainCameraView, TerrainFieldSet } from "./compile";
import type { MaterializedLandscapePyramid } from "./relief-pyramid";

export const TERRAIN_TILE_PYRAMID_SCHEMA =
  "rey.terrain-tile-pyramid.v1" as const;
export const TERRAIN_TILE_PROJECTION_REVISION =
  "rey.terrain.dataset-tiles@2" as const;
export const DEFAULT_TERRAIN_TILE_INTERVALS = 32;
export const DEFAULT_TERRAIN_SCREEN_ERROR_PIXELS = 1.5;

export interface TerrainTileDescriptor {
  tile_id: string;
  cache_key: string;
  field_set_id: string;
  source_revision: string;
  mosaic_id: string;
  height_level_id: string;
  relief_field_id: string;
  relief_operator_revision: string;
  validity_support_id: string;
  border_digest_id: string;
  level: number;
  column: number;
  row: number;
  parent_id: string | null;
  child_ids: readonly string[];
  column_indices: readonly number[];
  row_indices: readonly number[];
  validity_values: Uint8Array;
  validity_classification_values: Uint8Array;
  bounds: FieldBounds;
  geometric_error: number;
  validity_border: {
    north: string;
    east: string;
    south: string;
    west: string;
  };
  valid_vertices: number;
  no_data_vertices: number;
  unsupported_vertices: number;
  field_cells: number;
  cpu_bytes: number;
  gpu_bytes: number;
}

export interface TerrainTilePyramid {
  schema: typeof TERRAIN_TILE_PYRAMID_SCHEMA;
  pyramid_id: string;
  field_set_id: string;
  source_revision: string;
  compiler_revision: typeof TERRAIN_TILE_PROJECTION_REVISION;
  maximum_level: number;
  tile_intervals: number;
  tiles: readonly TerrainTileDescriptor[];
}

export interface TerrainTileSelection {
  pyramid_id: string;
  level: number;
  screen_error_pixels: number;
  cpu_bytes: number;
  gpu_bytes: number;
  budget_limited: boolean;
  tile_ids: readonly string[];
  tiles: readonly TerrainTileDescriptor[];
}

export interface TerrainTileSelectionBudget {
  maximum_cpu_bytes: number;
  maximum_gpu_bytes: number;
}

export function projectTerrainTilePyramid(
  field: TerrainFieldSet,
  tileIntervals = DEFAULT_TERRAIN_TILE_INTERVALS,
): TerrainTilePyramid {
  if (!Number.isInteger(tileIntervals) || tileIntervals < 2)
    throw new Error("terrain tile interval bound is invalid");
  const sourceColumns = field.grid.columns;
  const sourceRows = field.grid.rows;
  const leafColumns = Math.ceil((sourceColumns - 1) / tileIntervals);
  const leafRows = Math.ceil((sourceRows - 1) / tileIntervals);
  const maximumLevel = Math.ceil(Math.log2(Math.max(1, leafColumns, leafRows)));
  const sourceValidityClassification =
    verifyTerrainFieldValidityClassification(field);
  const invalidPrefix = invalidPrefixSum(field);
  const tileId = (level: number, column: number, row: number) =>
    [
      TERRAIN_TILE_PROJECTION_REVISION,
      field.field_set_id,
      field.source_revision,
      `z${level}`,
      `x${column}`,
      `y${row}`,
    ].join("|");
  const levelDimensions = Array.from(
    { length: maximumLevel + 1 },
    (_, level) => {
      const stride = 2 ** (maximumLevel - level);
      const span = tileIntervals * stride;
      return {
        stride,
        span,
        columns: Math.ceil((sourceColumns - 1) / span),
        rows: Math.ceil((sourceRows - 1) / span),
      };
    },
  );
  const tiles: TerrainTileDescriptor[] = [];
  for (let level = 0; level <= maximumLevel; level += 1) {
    const dimensions = levelDimensions[level]!;
    for (let row = 0; row < dimensions.rows; row += 1) {
      for (let column = 0; column < dimensions.columns; column += 1) {
        const columnStart = column * dimensions.span;
        const rowStart = row * dimensions.span;
        const columnEnd = Math.min(
          sourceColumns - 1,
          columnStart + dimensions.span,
        );
        const rowEnd = Math.min(sourceRows - 1, rowStart + dimensions.span);
        const columnIndices = sampleIndices(
          columnStart,
          columnEnd,
          dimensions.stride,
        );
        const rowIndices = sampleIndices(rowStart, rowEnd, dimensions.stride);
        const validity = tileValidity(
          field,
          columnIndices,
          rowIndices,
          dimensions.stride,
          invalidPrefix,
        );
        const validityClassification = tileValidityClassification(
          field,
          sourceValidityClassification.values,
          columnIndices,
          rowIndices,
          dimensions.stride,
          validity,
        );
        const bounds = tileBounds(
          field.grid.bounds,
          sourceColumns,
          sourceRows,
          columnStart,
          columnEnd,
          rowStart,
          rowEnd,
        );
        const fieldCells = columnIndices.length * rowIndices.length;
        const triangles = terrainTriangleIndices({
          grid: {
            columns: columnIndices.length,
            rows: rowIndices.length,
            bounds,
          },
          validity: { values: validity },
        });
        const validitySummary = summarizeTerrainValidityClassification(
          createTerrainValidityClassification(
            validityClassification,
            `${TERRAIN_TILE_PROJECTION_REVISION}:${field.source_revision}:z${level}:x${column}:y${row}`,
          ),
        );
        const nextDimensions = levelDimensions[level + 1];
        const childIds = nextDimensions
          ? (
              [
                [column * 2, row * 2],
                [column * 2 + 1, row * 2],
                [column * 2, row * 2 + 1],
                [column * 2 + 1, row * 2 + 1],
              ] as const
            )
              .filter(
                ([childColumn, childRow]) =>
                  childColumn < nextDimensions.columns &&
                  childRow < nextDimensions.rows,
              )
              .map(([childColumn, childRow]) =>
                tileId(level + 1, childColumn, childRow),
              )
          : [];
        tiles.push(
          Object.freeze({
            tile_id: tileId(level, column, row),
            cache_key: tileId(level, column, row),
            field_set_id: field.field_set_id,
            source_revision: field.source_revision,
            mosaic_id:
              field.landscape_mosaic?.mosaic_id ??
              field.landscape_reference?.reference_id ??
              field.field_set_id,
            height_level_id: field.field_set_id,
            relief_field_id: `${field.field_set_id}:unbound-relief`,
            relief_operator_revision: "unbound",
            validity_support_id: `${field.field_set_id}:validity`,
            border_digest_id: "unbound",
            level,
            column,
            row,
            parent_id:
              level === 0
                ? null
                : tileId(
                    level - 1,
                    Math.floor(column / 2),
                    Math.floor(row / 2),
                  ),
            child_ids: Object.freeze(childIds),
            column_indices: Object.freeze(columnIndices),
            row_indices: Object.freeze(rowIndices),
            validity_values: validity,
            validity_classification_values: validityClassification,
            bounds,
            geometric_error: tileGeometricError(
              field,
              columnIndices,
              rowIndices,
              validity,
              columnStart,
              columnEnd,
              rowStart,
              rowEnd,
            ),
            validity_border: Object.freeze(
              validityBorder(validity, columnIndices.length, rowIndices.length),
            ),
            valid_vertices: validitySummary.valid_vertices,
            no_data_vertices: validitySummary.no_data_vertices,
            unsupported_vertices: validitySummary.unsupported_vertices,
            field_cells: fieldCells,
            cpu_bytes: Math.ceil(
              (field.field_bytes / field.field_cells) * fieldCells,
            ),
            gpu_bytes: fieldCells * 68 + triangles.byteLength,
          }),
        );
      }
    }
  }
  const pyramidId = [
    TERRAIN_TILE_PYRAMID_SCHEMA,
    TERRAIN_TILE_PROJECTION_REVISION,
    field.field_set_id,
    field.source_revision,
    `${sourceColumns}x${sourceRows}`,
    `intervals:${tileIntervals}`,
    `levels:${maximumLevel + 1}`,
  ].join("|");
  return Object.freeze({
    schema: TERRAIN_TILE_PYRAMID_SCHEMA,
    pyramid_id: pyramidId,
    field_set_id: field.field_set_id,
    source_revision: field.source_revision,
    compiler_revision: TERRAIN_TILE_PROJECTION_REVISION,
    maximum_level: maximumLevel,
    tile_intervals: tileIntervals,
    tiles: Object.freeze(tiles),
  });
}

/**
 * Projects render tiles from the already-derived height/relief hierarchy.
 * Unlike the legacy field projector, this never strides over the finest field:
 * every selected descriptor names one exact conservative hierarchy level.
 */
export function projectMaterializedLandscapeTilePyramid(
  materialized: MaterializedLandscapePyramid,
): TerrainTilePyramid {
  const levels = materialized.relief_levels;
  if (levels.length === 0)
    throw new Error("materialized landscape pyramid has no relief levels");
  const levelErrors = cumulativeHierarchyErrors(materialized);
  const mutable = levels.flatMap((level) => {
    const heightLevel = materialized.height_hierarchy.levels[level.level];
    if (!heightLevel || heightLevel.level_id !== level.source_height_level_id)
      throw new Error("landscape render level changed height identity");
    const field = level.field;
    const classification = verifyTerrainFieldValidityClassification(field);
    return level.tiles.map((sourceTile) => {
      const columnIndices = sampleIndices(
        sourceTile.interior.column_start,
        sourceTile.interior.column_end,
        1,
      );
      const rowIndices = sampleIndices(
        sourceTile.interior.row_start,
        sourceTile.interior.row_end,
        1,
      );
      const validity = sampleIndexedComponents(
        field,
        field.validity.values,
        columnIndices,
        rowIndices,
        1,
      ) as Uint8Array;
      const validityClassification = sampleIndexedComponents(
        field,
        classification.values,
        columnIndices,
        rowIndices,
        1,
      ) as Uint8Array;
      const validitySummary = summarizeTerrainValidityClassification(
        createTerrainValidityClassification(
          validityClassification,
          `${TERRAIN_TILE_PROJECTION_REVISION}:${sourceTile.tile_id}:validity-classification`,
        ),
      );
      const bounds = tileBounds(
        field.grid.bounds,
        field.grid.columns,
        field.grid.rows,
        sourceTile.interior.column_start,
        sourceTile.interior.column_end,
        sourceTile.interior.row_start,
        sourceTile.interior.row_end,
      );
      const cells = columnIndices.length * rowIndices.length;
      const triangles = terrainTriangleIndices({
        grid: {
          columns: columnIndices.length,
          rows: rowIndices.length,
          bounds,
        },
        validity: { values: validity },
      });
      const cacheKey = [
        TERRAIN_TILE_PROJECTION_REVISION,
        materialized.height_hierarchy.mosaic_id,
        heightLevel.level_id,
        sourceTile.tile_id,
        level.relief.relief_field_id,
        level.relief.implementation_revision,
        heightLevel.validity_id,
        level.border_digest_id,
      ].join("|");
      return {
        tile_id: cacheKey,
        cache_key: cacheKey,
        field_set_id: field.field_set_id,
        source_revision: field.source_revision,
        mosaic_id: materialized.height_hierarchy.mosaic_id,
        height_level_id: heightLevel.level_id,
        relief_field_id: level.relief.relief_field_id,
        relief_operator_revision: level.relief.implementation_revision,
        validity_support_id: heightLevel.validity_id,
        border_digest_id: level.border_digest_id,
        level: level.level,
        column: sourceTile.column,
        row: sourceTile.row,
        parent_id: null as string | null,
        child_ids: [] as string[],
        column_indices: Object.freeze(columnIndices),
        row_indices: Object.freeze(rowIndices),
        validity_values: validity,
        validity_classification_values: validityClassification,
        bounds,
        geometric_error: levelErrors[level.level]!,
        validity_border: Object.freeze(
          validityBorder(validity, columnIndices.length, rowIndices.length),
        ),
        valid_vertices: validitySummary.valid_vertices,
        no_data_vertices: validitySummary.no_data_vertices,
        unsupported_vertices: validitySummary.unsupported_vertices,
        field_cells: cells,
        cpu_bytes: Math.ceil(
          ((field.field_bytes + landscapeReliefFieldByteLength(level.relief)) /
            field.field_cells) *
            cells,
        ),
        gpu_bytes: cells * 68 + triangles.byteLength,
      };
    });
  });
  for (const tile of mutable) {
    if (tile.level === 0) continue;
    const parent = hierarchyParentTile(mutable, tile);
    if (!parent) throw new Error("landscape render tile lost its parent");
    tile.parent_id = parent.tile_id;
    parent.child_ids.push(tile.tile_id);
  }
  const tiles = Object.freeze(
    mutable.map((tile) =>
      Object.freeze({
        ...tile,
        child_ids: Object.freeze(
          [...tile.child_ids].sort((left, right) => left.localeCompare(right)),
        ),
      }),
    ),
  );
  const maximumLevel = levels.length - 1;
  const tileIntervals = levels.reduce(
    (maximum, level) =>
      Math.max(
        maximum,
        ...level.tiles.map((tile) =>
          Math.max(
            tile.interior.column_end - tile.interior.column_start,
            tile.interior.row_end - tile.interior.row_start,
          ),
        ),
      ),
    0,
  );
  return Object.freeze({
    schema: TERRAIN_TILE_PYRAMID_SCHEMA,
    pyramid_id: [
      TERRAIN_TILE_PYRAMID_SCHEMA,
      TERRAIN_TILE_PROJECTION_REVISION,
      materialized.envelope.envelope_id,
      materialized.hierarchy_id,
    ].join("|"),
    field_set_id: materialized.height_hierarchy.field_set_id,
    source_revision: materialized.height_hierarchy.source_revision,
    compiler_revision: TERRAIN_TILE_PROJECTION_REVISION,
    maximum_level: maximumLevel,
    tile_intervals: tileIntervals,
    tiles,
  });
}

export function selectTerrainTilesForView(
  pyramid: TerrainTilePyramid,
  view: TerrainCameraView,
  maximumScreenError = DEFAULT_TERRAIN_SCREEN_ERROR_PIXELS,
  budget?: TerrainTileSelectionBudget,
  maximumHierarchyLevel = pyramid.maximum_level,
): TerrainTileSelection {
  if (!Number.isFinite(maximumScreenError) || maximumScreenError <= 0)
    throw new Error("terrain screen-space error bound is invalid");
  if (
    budget &&
    (!Number.isSafeInteger(budget.maximum_cpu_bytes) ||
      !Number.isSafeInteger(budget.maximum_gpu_bytes) ||
      budget.maximum_cpu_bytes < 1 ||
      budget.maximum_gpu_bytes < 1)
  )
    throw new Error("terrain tile selection budgets are invalid");
  if (
    !Number.isInteger(maximumHierarchyLevel) ||
    maximumHierarchyLevel < 0 ||
    maximumHierarchyLevel > pyramid.maximum_level
  )
    throw new Error("terrain maximum hierarchy level is invalid");
  const visible = visibleTerrainBounds(view);
  let selected: TerrainTileDescriptor[] = [];
  let selectedLevel = pyramid.maximum_level;
  let selectedError = 0;
  let selectedCpuBytes = 0;
  let selectedGpuBytes = 0;
  for (let level = 0; level <= maximumHierarchyLevel; level += 1) {
    const candidates = pyramid.tiles.filter(
      (tile) => tile.level === level && boundsIntersect(tile.bounds, visible),
    );
    if (candidates.length === 0) continue;
    const screenError = candidates.reduce(
      (maximum, tile) =>
        Math.max(maximum, tile.geometric_error * view.rendered_scale),
      0,
    );
    const cpuBytes = candidates.reduce(
      (total, tile) => total + tile.cpu_bytes,
      0,
    );
    const gpuBytes = candidates.reduce(
      (total, tile) => total + tile.gpu_bytes,
      0,
    );
    if (
      budget &&
      (cpuBytes > budget.maximum_cpu_bytes ||
        gpuBytes > budget.maximum_gpu_bytes)
    )
      continue;
    selected = candidates;
    selectedLevel = level;
    selectedError = screenError;
    selectedCpuBytes = cpuBytes;
    selectedGpuBytes = gpuBytes;
    if (screenError <= maximumScreenError) break;
  }
  if (selected.length === 0)
    throw new Error("no visible terrain hierarchy level fits the tile budgets");
  selected.sort(
    (left, right) =>
      left.row - right.row ||
      left.column - right.column ||
      left.tile_id.localeCompare(right.tile_id),
  );
  return Object.freeze({
    pyramid_id: pyramid.pyramid_id,
    level: selectedLevel,
    screen_error_pixels: selectedError,
    cpu_bytes: selectedCpuBytes,
    gpu_bytes: selectedGpuBytes,
    budget_limited: selectedError > maximumScreenError,
    tile_ids: Object.freeze(selected.map((tile) => tile.tile_id)),
    tiles: Object.freeze(selected),
  });
}

export function terrainTileSeamMismatchCount(
  tiles: readonly TerrainTileDescriptor[],
): number {
  const indexed = new Map(
    tiles.map((tile) => [`${tile.level}:${tile.column}:${tile.row}`, tile]),
  );
  let mismatches = 0;
  for (const tile of tiles) {
    const east = indexed.get(`${tile.level}:${tile.column + 1}:${tile.row}`);
    if (east && tile.validity_border.east !== east.validity_border.west)
      mismatches += 1;
    const south = indexed.get(`${tile.level}:${tile.column}:${tile.row + 1}`);
    if (south && tile.validity_border.south !== south.validity_border.north)
      mismatches += 1;
  }
  return mismatches;
}

export function terrainTileReliefSeamMismatchCount(
  tiles: readonly {
    descriptor: TerrainTileDescriptor;
    mesh: Pick<TerrainMeshData, "hillshade" | "salience">;
  }[],
): number {
  const indexed = new Map(
    tiles.map((tile) => [
      `${tile.descriptor.level}:${tile.descriptor.column}:${tile.descriptor.row}`,
      tile,
    ]),
  );
  let mismatches = 0;
  for (const tile of tiles) {
    const { descriptor } = tile;
    const east = indexed.get(
      `${descriptor.level}:${descriptor.column + 1}:${descriptor.row}`,
    );
    if (east && !reliefBordersMatch(tile, "east", east, "west"))
      mismatches += 1;
    const south = indexed.get(
      `${descriptor.level}:${descriptor.column}:${descriptor.row + 1}`,
    );
    if (south && !reliefBordersMatch(tile, "south", south, "north"))
      mismatches += 1;
  }
  return mismatches;
}

export function terrainTileReliefPartitionMismatchCount(
  source: LandscapeReliefField,
  tiles: readonly {
    descriptor: TerrainTileDescriptor;
    mesh: Pick<TerrainMeshData, "hillshade" | "salience">;
  }[],
): number {
  if (source.derivation_scope !== "complete_field")
    throw new Error("terrain relief partition requires complete-field support");
  let mismatches = 0;
  for (const tile of tiles) {
    if (tile.descriptor.field_set_id !== source.field_set_id)
      throw new Error("terrain relief partition changed source identity");
    let tileIndex = 0;
    for (const row of tile.descriptor.row_indices) {
      for (const column of tile.descriptor.column_indices) {
        const sourceIndex = row * source.columns + column;
        if (
          tile.mesh.hillshade[tileIndex] !== source.hillshade[sourceIndex] ||
          tile.mesh.salience[tileIndex] !== source.salience[sourceIndex]
        )
          mismatches += 1;
        tileIndex += 1;
      }
    }
  }
  return mismatches;
}

export function materializeTerrainTile(
  source: TerrainFieldSet,
  tile: TerrainTileDescriptor,
): TerrainFieldSet {
  if (
    tile.field_set_id !== source.field_set_id ||
    tile.source_revision !== source.source_revision
  )
    throw new Error("terrain tile is not bound to its source field");
  const grid = createFieldGrid(
    tile.column_indices.length,
    tile.row_indices.length,
    tile.bounds,
  );
  const scalar = (field: TerrainFieldSet["elevation"]) =>
    scalarField(
      field.channel,
      field.implementation_revision,
      grid,
      sampleComponents(source, field.values, tile, 1),
    );
  const vector = (field: TerrainFieldSet["normal"]) =>
    vectorField(
      field.channel,
      field.implementation_revision,
      grid,
      field.components,
      sampleComponents(source, field.values, tile, field.components),
    );
  const validity = maskField(
    source.validity.channel,
    source.validity.implementation_revision,
    grid,
    tile.validity_values.slice(),
  );
  const validityClassification = createTerrainValidityClassification(
    tile.validity_classification_values.slice(),
    `${TERRAIN_TILE_PROJECTION_REVISION}:${tile.tile_id}:validity-classification`,
  );
  const elevation = scalar(source.elevation);
  const rainfall = scalar(source.rainfall);
  const flowDirection = vector(source.flow_direction);
  const flowAccumulation = scalar(source.flow_accumulation);
  const erosion = scalar(source.erosion);
  const normal = vector(source.normal);
  const curvature = scalar(source.curvature);
  const material = materialField(
    source.material.channel,
    source.material.implementation_revision,
    grid,
    sampleComponents(source, source.material.tint, tile, 3),
    sampleComponents(source, source.material.occlusion, tile, 1),
    sampleComponents(source, source.material.roughness, tile, 1),
  );
  const fields = [
    validity,
    elevation,
    rainfall,
    flowDirection,
    flowAccumulation,
    erosion,
    normal,
    curvature,
    material,
  ] as const;
  const result: TerrainFieldSet = Object.freeze({
    schema: source.schema,
    field_set_id: tile.tile_id,
    program_id: source.program_id,
    working_set_id: tile.tile_id,
    active_band_ids: source.active_band_ids,
    detail_authority: `${source.detail_authority}; conservative tile ${tile.level}/${tile.column}/${tile.row}`,
    source_revision: source.source_revision,
    source_summary: source.source_summary,
    grid,
    elevation_scale: source.elevation_scale,
    validity,
    validity_classification: validityClassification,
    elevation,
    rainfall,
    flow_direction: flowDirection,
    flow_accumulation: flowAccumulation,
    erosion,
    normal,
    curvature,
    material,
    relief_metrics: source.relief_metrics,
    landscape_reference: source.landscape_reference,
    field_cells: fieldCellCount(grid),
    field_bytes: fields.reduce(
      (total, field) => total + fieldByteLength(field),
      validityClassification.values.byteLength,
    ),
    landscape_mosaic: source.landscape_mosaic,
  });
  if (result.field_cells !== tile.field_cells)
    throw new Error("materialized terrain tile shape changed");
  return result;
}

export function materializeTerrainTileRelief(
  source: TerrainFieldSet,
  relief: LandscapeReliefField,
  tile: TerrainTileDescriptor,
): LandscapeReliefField {
  if (
    tile.field_set_id !== source.field_set_id ||
    tile.source_revision !== source.source_revision
  )
    throw new Error("terrain tile relief is not bound to its source field");
  return sampleLandscapeReliefField(
    relief,
    source,
    tile.tile_id,
    tile.column_indices,
    tile.row_indices,
  );
}

function cumulativeHierarchyErrors(
  materialized: MaterializedLandscapePyramid,
): readonly number[] {
  const levels = materialized.relief_levels;
  const errors = new Array<number>(levels.length).fill(0);
  for (let level = levels.length - 2; level >= 0; level -= 1)
    errors[level] =
      errors[level + 1]! +
      adjacentHierarchyError(levels[level]!.field, levels[level + 1]!.field);
  return Object.freeze(errors);
}

function adjacentHierarchyError(
  coarse: TerrainFieldSet,
  fine: TerrainFieldSet,
): number {
  let maximum = 0;
  for (let row = 0; row < fine.grid.rows; row += 1) {
    const coarseRow =
      (row / Math.max(1, fine.grid.rows - 1)) * (coarse.grid.rows - 1);
    const top = Math.floor(coarseRow);
    const bottom = Math.ceil(coarseRow);
    const rowProgress = coarseRow - top;
    for (let column = 0; column < fine.grid.columns; column += 1) {
      const fineIndex = row * fine.grid.columns + column;
      if (fine.validity.values[fineIndex] === 0) continue;
      const coarseColumn =
        (column / Math.max(1, fine.grid.columns - 1)) *
        (coarse.grid.columns - 1);
      const left = Math.floor(coarseColumn);
      const right = Math.ceil(coarseColumn);
      const columnProgress = coarseColumn - left;
      const cornerIndices = [
        top * coarse.grid.columns + left,
        top * coarse.grid.columns + right,
        bottom * coarse.grid.columns + left,
        bottom * coarse.grid.columns + right,
      ] as const;
      if (cornerIndices.some((index) => coarse.validity.values[index] === 0)) {
        maximum = Math.max(maximum, fine.elevation_scale);
        continue;
      }
      const upper = interpolate(
        coarse.elevation.values[cornerIndices[0]]!,
        coarse.elevation.values[cornerIndices[1]]!,
        columnProgress,
      );
      const lower = interpolate(
        coarse.elevation.values[cornerIndices[2]]!,
        coarse.elevation.values[cornerIndices[3]]!,
        columnProgress,
      );
      maximum = Math.max(
        maximum,
        Math.abs(
          fine.elevation.values[fineIndex]! -
            interpolate(upper, lower, rowProgress),
        ) * fine.elevation_scale,
      );
    }
  }
  return maximum;
}

function hierarchyParentTile<T extends TerrainTileDescriptor>(
  tiles: readonly T[],
  child: Pick<TerrainTileDescriptor, "level" | "bounds">,
): T | undefined {
  const centerX = child.bounds.x + child.bounds.width / 2;
  const centerY = child.bounds.y + child.bounds.height / 2;
  return tiles
    .filter(
      (candidate) =>
        candidate.level === child.level - 1 &&
        centerX >= candidate.bounds.x - Number.EPSILON &&
        centerX <=
          candidate.bounds.x + candidate.bounds.width + Number.EPSILON &&
        centerY >= candidate.bounds.y - Number.EPSILON &&
        centerY <=
          candidate.bounds.y + candidate.bounds.height + Number.EPSILON,
    )
    .sort(
      (left, right) =>
        left.row - right.row ||
        left.column - right.column ||
        left.tile_id.localeCompare(right.tile_id),
    )[0];
}

function sampleIndexedComponents(
  source: TerrainFieldSet,
  values: Uint8Array,
  columnIndices: readonly number[],
  rowIndices: readonly number[],
  components: 1,
): Uint8Array {
  const result = new Uint8Array(
    columnIndices.length * rowIndices.length * components,
  );
  let output = 0;
  for (const row of rowIndices)
    for (const column of columnIndices)
      result[output++] = values[row * source.grid.columns + column]!;
  return result;
}

function sampleIndices(start: number, end: number, stride: number): number[] {
  const indices: number[] = [];
  for (let index = start; index <= end; index += stride) indices.push(index);
  if (indices.at(-1) !== end) indices.push(end);
  return indices;
}

function tileBounds(
  bounds: FieldBounds,
  columns: number,
  rows: number,
  columnStart: number,
  columnEnd: number,
  rowStart: number,
  rowEnd: number,
): FieldBounds {
  const spacingX = bounds.width / (columns - 1);
  const spacingY = bounds.height / (rows - 1);
  return Object.freeze({
    x: bounds.x + columnStart * spacingX,
    y: bounds.y + rowStart * spacingY,
    width: (columnEnd - columnStart) * spacingX,
    height: (rowEnd - rowStart) * spacingY,
  });
}

function invalidPrefixSum(field: TerrainFieldSet): Uint32Array {
  const columns = field.grid.columns + 1;
  const prefix = new Uint32Array(columns * (field.grid.rows + 1));
  for (let row = 0; row < field.grid.rows; row += 1) {
    let rowInvalid = 0;
    for (let column = 0; column < field.grid.columns; column += 1) {
      rowInvalid +=
        field.validity.values[row * field.grid.columns + column] === 0 ? 1 : 0;
      prefix[(row + 1) * columns + column + 1] =
        prefix[row * columns + column + 1]! + rowInvalid;
    }
  }
  return prefix;
}

function tileValidity(
  field: TerrainFieldSet,
  columnIndices: readonly number[],
  rowIndices: readonly number[],
  stride: number,
  invalidPrefix: Uint32Array,
): Uint8Array {
  if (stride === 1)
    return Uint8Array.from(
      rowIndices.flatMap((row) =>
        columnIndices.map(
          (column) => field.validity.values[row * field.grid.columns + column]!,
        ),
      ),
    );
  const radiusBefore = Math.floor(stride / 2);
  const radiusAfter = Math.ceil(stride / 2) - 1;
  return Uint8Array.from(
    rowIndices.flatMap((row) =>
      columnIndices.map((column) => {
        const left = Math.max(0, column - radiusBefore);
        const right = Math.min(field.grid.columns - 1, column + radiusAfter);
        const top = Math.max(0, row - radiusBefore);
        const bottom = Math.min(field.grid.rows - 1, row + radiusAfter);
        return invalidCount(
          invalidPrefix,
          field.grid.columns + 1,
          left,
          top,
          right,
          bottom,
        ) === 0
          ? 1
          : 0;
      }),
    ),
  );
}

function tileValidityClassification(
  field: TerrainFieldSet,
  sourceClassification: Uint8Array,
  columnIndices: readonly number[],
  rowIndices: readonly number[],
  stride: number,
  tileValidityValues: Uint8Array,
): Uint8Array {
  const result = new Uint8Array(tileValidityValues.length);
  const radiusBefore = Math.floor(stride / 2);
  const radiusAfter = Math.ceil(stride / 2) - 1;
  let output = 0;
  for (const row of rowIndices) {
    for (const column of columnIndices) {
      if (tileValidityValues[output] !== 0) {
        result[output] = TERRAIN_VALIDITY_VALID;
        output += 1;
        continue;
      }
      const left = Math.max(0, column - radiusBefore);
      const right = Math.min(field.grid.columns - 1, column + radiusAfter);
      const top = Math.max(0, row - radiusBefore);
      const bottom = Math.min(field.grid.rows - 1, row + radiusAfter);
      let unsupported = false;
      for (
        let sourceRow = top;
        sourceRow <= bottom && !unsupported;
        sourceRow += 1
      )
        for (let sourceColumn = left; sourceColumn <= right; sourceColumn += 1)
          if (
            sourceClassification[
              sourceRow * field.grid.columns + sourceColumn
            ] === TERRAIN_VALIDITY_UNSUPPORTED
          ) {
            unsupported = true;
            break;
          }
      result[output] = unsupported
        ? TERRAIN_VALIDITY_UNSUPPORTED
        : TERRAIN_VALIDITY_NO_DATA;
      output += 1;
    }
  }
  return result;
}

function invalidCount(
  prefix: Uint32Array,
  width: number,
  left: number,
  top: number,
  right: number,
  bottom: number,
): number {
  const x1 = left;
  const y1 = top;
  const x2 = right + 1;
  const y2 = bottom + 1;
  return (
    prefix[y2 * width + x2]! -
    prefix[y1 * width + x2]! -
    prefix[y2 * width + x1]! +
    prefix[y1 * width + x1]!
  );
}

function tileGeometricError(
  field: TerrainFieldSet,
  columnIndices: readonly number[],
  rowIndices: readonly number[],
  tileValidityValues: Uint8Array,
  columnStart: number,
  columnEnd: number,
  rowStart: number,
  rowEnd: number,
): number {
  if (
    columnIndices.length === columnEnd - columnStart + 1 &&
    rowIndices.length === rowEnd - rowStart + 1
  )
    return 0;
  let maximum = 0;
  for (let row = rowStart; row <= rowEnd; row += 1) {
    const rowBracket = bracket(rowIndices, row);
    for (let column = columnStart; column <= columnEnd; column += 1) {
      const sourceIndex = row * field.grid.columns + column;
      if (field.validity.values[sourceIndex] === 0) continue;
      const columnBracket = bracket(columnIndices, column);
      const corners = [
        rowBracket.lower * columnIndices.length + columnBracket.lower,
        rowBracket.lower * columnIndices.length + columnBracket.upper,
        rowBracket.upper * columnIndices.length + columnBracket.lower,
        rowBracket.upper * columnIndices.length + columnBracket.upper,
      ] as const;
      if (corners.some((index) => tileValidityValues[index] === 0)) {
        maximum = Math.max(maximum, field.elevation_scale);
        continue;
      }
      const top = interpolate(
        sampledElevation(
          field,
          columnIndices[columnBracket.lower]!,
          rowIndices[rowBracket.lower]!,
        ),
        sampledElevation(
          field,
          columnIndices[columnBracket.upper]!,
          rowIndices[rowBracket.lower]!,
        ),
        columnBracket.progress,
      );
      const bottom = interpolate(
        sampledElevation(
          field,
          columnIndices[columnBracket.lower]!,
          rowIndices[rowBracket.upper]!,
        ),
        sampledElevation(
          field,
          columnIndices[columnBracket.upper]!,
          rowIndices[rowBracket.upper]!,
        ),
        columnBracket.progress,
      );
      const approximation = interpolate(top, bottom, rowBracket.progress);
      maximum = Math.max(
        maximum,
        Math.abs(field.elevation.values[sourceIndex]! - approximation) *
          field.elevation_scale,
      );
    }
  }
  return maximum;
}

function bracket(samples: readonly number[], value: number) {
  let upper = samples.findIndex((sample) => sample >= value);
  if (upper < 0) upper = samples.length - 1;
  const lower = Math.max(0, upper - (samples[upper] === value ? 0 : 1));
  const span = samples[upper]! - samples[lower]!;
  return {
    lower,
    upper,
    progress: span === 0 ? 0 : (value - samples[lower]!) / span,
  };
}

function sampledElevation(
  field: TerrainFieldSet,
  column: number,
  row: number,
): number {
  return field.elevation.values[row * field.grid.columns + column]!;
}

function interpolate(left: number, right: number, progress: number): number {
  return left + (right - left) * progress;
}

function validityBorder(
  validity: Uint8Array,
  columns: number,
  rows: number,
): TerrainTileDescriptor["validity_border"] {
  const bit = (column: number, row: number) =>
    validity[row * columns + column] === 0 ? "0" : "1";
  return {
    north: Array.from({ length: columns }, (_, column) => bit(column, 0)).join(
      "",
    ),
    east: Array.from({ length: rows }, (_, row) => bit(columns - 1, row)).join(
      "",
    ),
    south: Array.from({ length: columns }, (_, column) =>
      bit(column, rows - 1),
    ).join(""),
    west: Array.from({ length: rows }, (_, row) => bit(0, row)).join(""),
  };
}

function visibleTerrainBounds(view: TerrainCameraView): FieldBounds {
  const scale = Math.max(0.000_001, view.rendered_scale);
  const pitch =
    (Math.max(22, Math.min(90, view.pitch_degrees ?? 90)) * Math.PI) / 180;
  const yaw =
    (Math.max(-180, Math.min(180, view.yaw_degrees ?? 0)) * Math.PI) / 180;
  const panX = view.pan_x / scale;
  const panY = view.pan_y / (scale * Math.max(0.2, Math.sin(pitch)));
  const centerX =
    view.world_width / 2 - panX * Math.cos(yaw) - panY * Math.sin(yaw);
  const centerY =
    view.world_height / 2 + panX * Math.sin(yaw) - panY * Math.cos(yaw);
  const halfScreenWidth = view.viewport_width / scale / 2;
  const halfScreenHeight =
    view.viewport_height / scale / Math.max(0.2, Math.sin(pitch)) / 2;
  const halfWidth =
    Math.abs(Math.cos(yaw)) * halfScreenWidth +
    Math.abs(Math.sin(yaw)) * halfScreenHeight;
  const halfHeight =
    Math.abs(Math.sin(yaw)) * halfScreenWidth +
    Math.abs(Math.cos(yaw)) * halfScreenHeight;
  const width = halfWidth * 2;
  const height = halfHeight * 2;
  const overscanX = width * 0.125;
  const overscanY = height * 0.125;
  return {
    x: centerX - width / 2 - overscanX,
    y: centerY - height / 2 - overscanY,
    width: width + overscanX * 2,
    height: height + overscanY * 2,
  };
}

function boundsIntersect(left: FieldBounds, right: FieldBounds): boolean {
  return !(
    left.x + left.width < right.x ||
    right.x + right.width < left.x ||
    left.y + left.height < right.y ||
    right.y + right.height < left.y
  );
}

function sampleComponents<T extends Float32Array | Int8Array>(
  source: TerrainFieldSet,
  values: T,
  tile: TerrainTileDescriptor,
  components: number,
): T {
  const result = (
    values instanceof Int8Array
      ? new Int8Array(tile.field_cells * components)
      : new Float32Array(tile.field_cells * components)
  ) as T;
  let output = 0;
  for (const row of tile.row_indices) {
    for (const column of tile.column_indices) {
      const sourceOffset = (row * source.grid.columns + column) * components;
      for (let component = 0; component < components; component += 1)
        result[output++] = values[sourceOffset + component]!;
    }
  }
  return result;
}

function reliefBordersMatch(
  left: {
    descriptor: TerrainTileDescriptor;
    mesh: Pick<TerrainMeshData, "hillshade" | "salience">;
  },
  leftEdge: "north" | "east" | "south" | "west",
  right: {
    descriptor: TerrainTileDescriptor;
    mesh: Pick<TerrainMeshData, "hillshade" | "salience">;
  },
  rightEdge: "north" | "east" | "south" | "west",
): boolean {
  const leftIndices = reliefBorderIndices(left.descriptor, leftEdge);
  const rightIndices = reliefBorderIndices(right.descriptor, rightEdge);
  if (leftIndices.length !== rightIndices.length) return false;
  return leftIndices.every(
    (leftIndex, sequence) =>
      left.mesh.hillshade[leftIndex] ===
        right.mesh.hillshade[rightIndices[sequence]!] &&
      left.mesh.salience[leftIndex] ===
        right.mesh.salience[rightIndices[sequence]!],
  );
}

function reliefBorderIndices(
  tile: TerrainTileDescriptor,
  edge: "north" | "east" | "south" | "west",
): readonly number[] {
  const columns = tile.column_indices.length;
  const rows = tile.row_indices.length;
  if (edge === "north")
    return Array.from({ length: columns }, (_, column) => column);
  if (edge === "south")
    return Array.from(
      { length: columns },
      (_, column) => (rows - 1) * columns + column,
    );
  if (edge === "west")
    return Array.from({ length: rows }, (_, row) => row * columns);
  return Array.from({ length: rows }, (_, row) => row * columns + columns - 1);
}
