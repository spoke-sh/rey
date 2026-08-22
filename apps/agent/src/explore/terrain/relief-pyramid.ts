import {
  deriveLandscapeReliefField,
  finalizeLandscapeHeightPyramid,
  finalizeLandscapePyramidEnvelope,
  finalizeLandscapeReliefPyramid,
  landscapePyramidContentId,
  landscapeReliefFieldByteLength,
  summarizeTerrainFieldValidity,
  summarizeTerrainValidityClassification,
  TERRAIN_VALIDITY_VALID,
  verifyLandscapePyramidEnvelope,
  type LandscapeHeightPyramid,
  type LandscapePyramidEnvelope,
  type LandscapePyramidLineage,
  type LandscapeReliefField,
  type LandscapeReliefPyramid,
  type TerrainFieldSetInput,
} from "@rey/explorer";
import { createFieldGrid, maskField, scalarField } from "../engine/fields";
import type { TerrainFieldSet } from "./compile";
import {
  compileLandscapeHeightHierarchy,
  type MaterializedLandscapeHeightHierarchy,
  type MaterializedLandscapeHeightLevel,
} from "./height-pyramid";
import { deriveTerrainNormals } from "./normals";

export const LANDSCAPE_RELIEF_HIERARCHY_REVISION =
  "rey.terrain.relief-hierarchy@1" as const;
export const LANDSCAPE_RELIEF_DERIVATION_TILE_INTERVALS = 32;
export const LANDSCAPE_RELIEF_PARTITION_TOLERANCE = 1e-6;

type ReliefBorder = "north" | "east" | "south" | "west";

export interface MaterializedLandscapeReliefTile {
  tile_id: string;
  level: number;
  column: number;
  row: number;
  interior: Readonly<{
    column_start: number;
    column_end: number;
    row_start: number;
    row_end: number;
  }>;
  source_window: Readonly<{
    column_start: number;
    column_end: number;
    row_start: number;
    row_end: number;
  }>;
  gutter_radius_cells: number;
  border_digests: Readonly<Record<ReliefBorder, string>>;
}

export interface MaterializedLandscapeReliefLevel {
  level: number;
  source_height_level_id: string;
  relief: LandscapeReliefField;
  tiles: readonly MaterializedLandscapeReliefTile[];
  maximum_gutter_radius_cells: number;
  border_digest_id: string;
  border_mismatches: number;
  partition_mismatches: number;
  byte_length: number;
}

export interface MaterializedLandscapePyramid {
  schema: "rey.materialized-landscape-pyramid.v1";
  implementation_revision: typeof LANDSCAPE_RELIEF_HIERARCHY_REVISION;
  hierarchy_id: string;
  height_hierarchy: MaterializedLandscapeHeightHierarchy;
  relief_levels: readonly MaterializedLandscapeReliefLevel[];
  envelope: LandscapePyramidEnvelope;
  byte_length: number;
  border_mismatches: number;
  partition_mismatches: number;
}

export function compileMaterializedLandscapePyramid(
  field: TerrainFieldSet,
  tileIntervals = LANDSCAPE_RELIEF_DERIVATION_TILE_INTERVALS,
): MaterializedLandscapePyramid {
  if (!Number.isSafeInteger(tileIntervals) || tileIntervals < 2)
    throw new Error("landscape relief derivation tile bound is invalid");
  const heightHierarchy = compileLandscapeHeightHierarchy(field);
  const heightPyramid = sharedHeightPyramid(field, heightHierarchy);
  const reliefLevels = Object.freeze(
    heightHierarchy.levels.map((heightLevel) =>
      compileReliefLevel(field, heightHierarchy, heightLevel, tileIntervals),
    ),
  );
  const reliefPyramid = sharedReliefPyramid(
    field,
    heightHierarchy,
    heightPyramid,
    reliefLevels,
  );
  const envelope = finalizeLandscapePyramidEnvelope(
    field.field_set_id,
    field.source_revision,
    heightPyramid,
    reliefPyramid,
  );
  verifyLandscapePyramidEnvelope(envelope, field, reliefLevels.at(-1)!.relief);
  const borderMismatches = reliefLevels.reduce(
    (total, level) => total + level.border_mismatches,
    0,
  );
  const partitionMismatches = reliefLevels.reduce(
    (total, level) => total + level.partition_mismatches,
    0,
  );
  if (borderMismatches !== 0 || partitionMismatches !== 0)
    throw new Error(
      `haloed landscape relief changed across a tile partition (${borderMismatches} border, ${partitionMismatches} interior mismatches; ${reliefLevels
        .filter(
          ({ border_mismatches, partition_mismatches }) =>
            border_mismatches !== 0 || partition_mismatches !== 0,
        )
        .map(
          ({ level, border_mismatches, partition_mismatches }) =>
            `level ${level}: ${border_mismatches}/${partition_mismatches}`,
        )
        .join(", ")})`,
    );
  return Object.freeze({
    schema: "rey.materialized-landscape-pyramid.v1" as const,
    implementation_revision: LANDSCAPE_RELIEF_HIERARCHY_REVISION,
    hierarchy_id: landscapePyramidContentId(
      `${LANDSCAPE_RELIEF_HIERARCHY_REVISION}:${heightHierarchy.hierarchy_id}:${envelope.envelope_id}`,
      reliefLevels.flatMap(({ relief }) => [
        relief.hillshade,
        relief.salience,
        relief.tangent,
      ]),
    ),
    height_hierarchy: heightHierarchy,
    relief_levels: reliefLevels,
    envelope,
    byte_length:
      heightHierarchy.byte_length +
      reliefLevels.reduce((total, level) => total + level.byte_length, 0),
    border_mismatches: borderMismatches,
    partition_mismatches: partitionMismatches,
  });
}

function sharedHeightPyramid(
  field: TerrainFieldSet,
  hierarchy: MaterializedLandscapeHeightHierarchy,
): LandscapeHeightPyramid {
  const reference = field.landscape_reference!;
  const lineage = landscapeLineage(field, hierarchy);
  return finalizeLandscapeHeightPyramid({
    implementation_revision: hierarchy.implementation_revision,
    mosaic_id: hierarchy.mosaic_id,
    coordinate_reference: reference.coordinate_reference,
    vertical_reference: reference.vertical_reference,
    complete: hierarchy.complete,
    omissions: hierarchy.omissions,
    levels: hierarchy.levels.map((level) => {
      const range = supportedElevationRange(level);
      const validity =
        level.level === hierarchy.levels.length - 1
          ? summarizeTerrainFieldValidity(field)
          : summarizeTerrainValidityClassification({
              schema: "rey.terrain-validity-classification.v1",
              implementation_revision: `${hierarchy.implementation_revision}:${level.level_id}`,
              values: level.validity_classification,
            });
      return {
        level: level.level,
        height_id: level.height_id,
        implementation_revision: hierarchy.implementation_revision,
        sample_spacing_x_meters: level.sample_spacing_x_meters,
        sample_spacing_y_meters: level.sample_spacing_y_meters,
        columns: level.columns,
        rows: level.rows,
        bounds: level.bounds,
        validity: {
          ...validity,
          policy: "conservative_support_only" as const,
        },
        elevation_minimum_meters: range.minimum,
        elevation_maximum_meters: range.maximum,
        height_bytes: level.elevation.byteLength,
        validity_bytes: level.validity_classification.byteLength,
        source_lineage: [
          ...lineage,
          {
            kind: "height-source-contribution",
            identity: level.source_contribution_id,
            revision: hierarchy.implementation_revision,
          },
        ],
      };
    }),
  });
}

function sharedReliefPyramid(
  field: TerrainFieldSet,
  hierarchy: MaterializedLandscapeHeightHierarchy,
  heightPyramid: LandscapeHeightPyramid,
  levels: readonly MaterializedLandscapeReliefLevel[],
): LandscapeReliefPyramid {
  const reference = field.landscape_reference!;
  const lineage = landscapeLineage(field, hierarchy);
  return finalizeLandscapeReliefPyramid(
    {
      implementation_revision: LANDSCAPE_RELIEF_HIERARCHY_REVISION,
      mosaic_id: hierarchy.mosaic_id,
      source_height_pyramid_id: heightPyramid.pyramid_id,
      coordinate_reference: reference.coordinate_reference,
      vertical_reference: reference.vertical_reference,
      complete: hierarchy.complete,
      omissions: hierarchy.omissions,
      levels: levels.map((level, index) => {
        const heightLevel = heightPyramid.levels[index]!;
        const relief = level.relief;
        return {
          level: level.level,
          implementation_revision: LANDSCAPE_RELIEF_HIERARCHY_REVISION,
          source_height_level_id: heightLevel.level_id,
          sample_spacing_x_meters: heightLevel.sample_spacing_x_meters,
          sample_spacing_y_meters: heightLevel.sample_spacing_y_meters,
          columns: heightLevel.columns,
          rows: heightLevel.rows,
          bounds: heightLevel.bounds,
          validity: heightLevel.validity,
          channel_ids: reliefChannelIds(relief),
          operator_support: relief.scales.map((scale) => ({
            operator_id: `multiscale-hillshade:${scale.id}`,
            implementation_revision: relief.implementation_revision,
            target_radius_meters: scale.target_radius_meters!,
            support_radius_cells: scale.support_radius_cells,
            support_radius_meters: scale.support_radius_meters!,
            gutter_radius_cells: level.maximum_gutter_radius_cells,
            supported: scale.supported,
            validity_policy: "complete_valid_window" as const,
          })),
          derivation_tile_count: level.tiles.length,
          maximum_gutter_radius_cells: level.maximum_gutter_radius_cells,
          border_digest_id: level.border_digest_id,
          relief_bytes: level.byte_length,
          source_lineage: [
            ...lineage,
            {
              kind: "height-level",
              identity: hierarchy.levels[index]!.level_id,
              revision: hierarchy.implementation_revision,
            },
            {
              kind: "relief-border-set",
              identity: level.border_digest_id,
              revision: LANDSCAPE_RELIEF_HIERARCHY_REVISION,
            },
          ],
        };
      }),
    },
    heightPyramid,
  );
}

function compileReliefLevel(
  source: TerrainFieldSet,
  hierarchy: MaterializedLandscapeHeightHierarchy,
  heightLevel: MaterializedLandscapeHeightLevel,
  tileIntervals: number,
): MaterializedLandscapeReliefLevel {
  const levelField = heightLevelTerrainInput(source, hierarchy, heightLevel);
  const reference = deriveLandscapeReliefField(levelField);
  const gutter = reference.maximum_support_radius_cells;
  const hillshade = new Float32Array(levelField.field_cells);
  const salience = new Float32Array(levelField.field_cells);
  const tangent = new Float32Array(levelField.field_cells * 2);
  const written = new Uint8Array(levelField.field_cells);
  let borderMismatches = 0;
  const tiles: MaterializedLandscapeReliefTile[] = [];
  const tileColumns = Math.ceil((levelField.grid.columns - 1) / tileIntervals);
  const tileRows = Math.ceil((levelField.grid.rows - 1) / tileIntervals);
  for (let row = 0; row < tileRows; row += 1) {
    for (let column = 0; column < tileColumns; column += 1) {
      const columnStart = column * tileIntervals;
      const columnEnd = Math.min(
        levelField.grid.columns - 1,
        columnStart + tileIntervals,
      );
      const rowStart = row * tileIntervals;
      const rowEnd = Math.min(
        levelField.grid.rows - 1,
        rowStart + tileIntervals,
      );
      const sourceWindow = Object.freeze({
        column_start: Math.max(0, columnStart - gutter),
        column_end: Math.min(levelField.grid.columns - 1, columnEnd + gutter),
        row_start: Math.max(0, rowStart - gutter),
        row_end: Math.min(levelField.grid.rows - 1, rowEnd + gutter),
      });
      const tileId = `${LANDSCAPE_RELIEF_HIERARCHY_REVISION}:${hierarchy.hierarchy_id}:z${heightLevel.level}:x${column}:y${row}`;
      const halo = cropTerrainInput(levelField, sourceWindow, `${tileId}:halo`);
      const haloRelief = deriveLandscapeReliefField(halo);
      const interior = Object.freeze({
        column_start: columnStart,
        column_end: columnEnd,
        row_start: rowStart,
        row_end: rowEnd,
      });
      const tileRelief = cropReliefInterior(
        haloRelief,
        interior,
        sourceWindow,
        tileId,
      );
      borderMismatches += mergeReliefInterior(
        tileRelief,
        interior,
        levelField.grid.columns,
        hillshade,
        salience,
        tangent,
        written,
      );
      tiles.push(
        Object.freeze({
          tile_id: tileId,
          level: heightLevel.level,
          column,
          row,
          interior,
          source_window: sourceWindow,
          gutter_radius_cells: gutter,
          border_digests: reliefBorderDigests(
            tileRelief,
            cropValidity(levelField, interior),
          ),
        }),
      );
    }
  }
  if (written.some((value) => value === 0))
    throw new Error("landscape relief tile partition omitted an interior");
  borderMismatches += adjacentBorderMismatchCount(tiles);
  const partitionMismatches = reliefMismatchCount(reference, {
    hillshade,
    salience,
    tangent,
  });
  const borderDigestId = landscapePyramidContentId(
    `relief-border-set:${heightLevel.level}`,
    [
      new TextEncoder().encode(
        JSON.stringify(
          tiles.map(({ tile_id, border_digests }) => ({
            tile_id,
            border_digests,
          })),
        ),
      ),
    ],
  );
  const relief = Object.freeze({
    ...reference,
    relief_field_id: `${LANDSCAPE_RELIEF_HIERARCHY_REVISION}:${heightLevel.level_id}:${borderDigestId}`,
    hillshade,
    salience,
    tangent,
  });
  return Object.freeze({
    level: heightLevel.level,
    source_height_level_id: heightLevel.level_id,
    relief,
    tiles: Object.freeze(tiles),
    maximum_gutter_radius_cells: gutter,
    border_digest_id: borderDigestId,
    border_mismatches: borderMismatches,
    partition_mismatches: partitionMismatches,
    byte_length: landscapeReliefFieldByteLength(relief),
  });
}

function heightLevelTerrainInput(
  source: TerrainFieldSet,
  hierarchy: MaterializedLandscapeHeightHierarchy,
  level: MaterializedLandscapeHeightLevel,
): TerrainFieldSetInput {
  const grid = createFieldGrid(level.columns, level.rows, level.bounds);
  const validityValues = Uint8Array.from(
    level.validity_classification,
    (value) => (value === TERRAIN_VALIDITY_VALID ? 1 : 0),
  );
  const validity = maskField(
    "validity",
    `${hierarchy.implementation_revision}:${level.level_id}:validity`,
    grid,
    validityValues,
  );
  const elevation = scalarField(
    "elevation",
    `${hierarchy.implementation_revision}:${level.level_id}:height`,
    grid,
    level.elevation,
  );
  const derived = deriveTerrainNormals(
    elevation,
    validity,
    source.elevation_scale,
    {
      normal: `${LANDSCAPE_RELIEF_HIERARCHY_REVISION}:${level.level_id}:normal`,
      curvature: `${LANDSCAPE_RELIEF_HIERARCHY_REVISION}:${level.level_id}:curvature`,
    },
  );
  const cells = level.columns * level.rows;
  const elevationRange = supportedElevationRange(level);
  const tint = new Float32Array(cells * 3);
  const occlusion = new Float32Array(cells).fill(1);
  const roughness = new Float32Array(cells).fill(1);
  return Object.freeze({
    field_set_id:
      level.level === hierarchy.levels.length - 1
        ? source.field_set_id
        : level.level_id,
    source_revision: source.source_revision,
    field_cells: cells,
    field_bytes:
      level.elevation.byteLength +
      validityValues.byteLength +
      derived.normal.values.byteLength +
      derived.curvature.values.byteLength +
      tint.byteLength +
      occlusion.byteLength +
      roughness.byteLength,
    elevation_scale: source.elevation_scale,
    grid,
    validity: { values: validityValues },
    validity_classification: {
      schema: "rey.terrain-validity-classification.v1" as const,
      implementation_revision: hierarchy.implementation_revision,
      values: level.validity_classification,
    },
    elevation: { values: level.elevation },
    normal: { values: derived.normal.values },
    curvature: { values: derived.curvature.values },
    material: { tint, occlusion, roughness },
    relief_metrics: {
      schema: "rey.terrain-relief-metrics.v1" as const,
      sample_spacing_x_meters: level.sample_spacing_x_meters,
      sample_spacing_y_meters: level.sample_spacing_y_meters,
      elevation_range_meters: source.relief_metrics!.elevation_range_meters,
      elevation_value_minimum: elevationRange.minimum,
      elevation_value_maximum: elevationRange.maximum,
      authority: `${source.relief_metrics!.authority}; conservative height hierarchy level ${level.level}`,
    },
    landscape_reference: source.landscape_reference,
    landscape_mosaic: source.landscape_mosaic,
  });
}

function cropTerrainInput(
  source: TerrainFieldSetInput,
  window: MaterializedLandscapeReliefTile["source_window"],
  fieldSetId: string,
): TerrainFieldSetInput {
  const columns = window.column_end - window.column_start + 1;
  const rows = window.row_end - window.row_start + 1;
  const cells = columns * rows;
  const grid = {
    columns,
    rows,
    bounds: windowBounds(source, window),
  };
  const validity = sampleWindow(source.validity.values, source, window, 1);
  const classification = source.validity_classification
    ? sampleWindow(source.validity_classification.values, source, window, 1)
    : undefined;
  const elevation = sampleWindow(source.elevation.values, source, window, 1);
  const normal = sampleWindow(source.normal.values, source, window, 3);
  const curvature = sampleWindow(source.curvature.values, source, window, 1);
  const tint = sampleWindow(source.material.tint, source, window, 3);
  const occlusion = sampleWindow(source.material.occlusion, source, window, 1);
  const roughness = sampleWindow(source.material.roughness, source, window, 1);
  return {
    ...source,
    field_set_id: fieldSetId,
    field_cells: cells,
    field_bytes:
      validity.byteLength +
      (classification?.byteLength ?? 0) +
      elevation.byteLength +
      normal.byteLength +
      curvature.byteLength +
      tint.byteLength +
      occlusion.byteLength +
      roughness.byteLength,
    grid,
    validity: { values: validity },
    validity_classification: classification
      ? { ...source.validity_classification!, values: classification }
      : undefined,
    elevation: { values: elevation },
    normal: { values: normal },
    curvature: { values: curvature },
    material: { tint, occlusion, roughness },
  };
}

function cropReliefInterior(
  source: LandscapeReliefField,
  interior: MaterializedLandscapeReliefTile["interior"],
  sourceWindow: MaterializedLandscapeReliefTile["source_window"],
  tileId: string,
): LandscapeReliefField {
  const columnStart = interior.column_start - sourceWindow.column_start;
  const columnEnd = interior.column_end - sourceWindow.column_start;
  const rowStart = interior.row_start - sourceWindow.row_start;
  const rowEnd = interior.row_end - sourceWindow.row_start;
  const window = {
    column_start: columnStart,
    column_end: columnEnd,
    row_start: rowStart,
    row_end: rowEnd,
  };
  const columns = columnEnd - columnStart + 1;
  const rows = rowEnd - rowStart + 1;
  return Object.freeze({
    ...source,
    relief_field_id: `${source.relief_field_id}|interior:${tileId}`,
    field_set_id: tileId,
    source_field_set_id: source.source_field_set_id,
    source_relief_field_id: source.relief_field_id,
    derivation_scope: "sampled_from_complete_field" as const,
    columns,
    rows,
    hillshade: sampleReliefWindow(source.hillshade, source.columns, window, 1),
    salience: sampleReliefWindow(source.salience, source.columns, window, 1),
    tangent: sampleReliefWindow(source.tangent, source.columns, window, 2),
  });
}

function mergeReliefInterior(
  source: LandscapeReliefField,
  interior: MaterializedLandscapeReliefTile["interior"],
  targetColumns: number,
  hillshade: Float32Array,
  salience: Float32Array,
  tangent: Float32Array,
  written: Uint8Array,
): number {
  let sourceIndex = 0;
  let mismatches = 0;
  for (let row = interior.row_start; row <= interior.row_end; row += 1) {
    for (
      let column = interior.column_start;
      column <= interior.column_end;
      column += 1
    ) {
      const targetIndex = row * targetColumns + column;
      if (
        written[targetIndex] !== 0 &&
        (!reliefValueMatches(
          hillshade[targetIndex]!,
          source.hillshade[sourceIndex]!,
        ) ||
          !reliefValueMatches(
            salience[targetIndex]!,
            source.salience[sourceIndex]!,
          ) ||
          !reliefValueMatches(
            tangent[targetIndex * 2]!,
            source.tangent[sourceIndex * 2]!,
          ) ||
          !reliefValueMatches(
            tangent[targetIndex * 2 + 1]!,
            source.tangent[sourceIndex * 2 + 1]!,
          ))
      )
        mismatches += 1;
      hillshade[targetIndex] = source.hillshade[sourceIndex]!;
      salience[targetIndex] = source.salience[sourceIndex]!;
      tangent[targetIndex * 2] = source.tangent[sourceIndex * 2]!;
      tangent[targetIndex * 2 + 1] = source.tangent[sourceIndex * 2 + 1]!;
      written[targetIndex] = 1;
      sourceIndex += 1;
    }
  }
  return mismatches;
}

function reliefMismatchCount(
  expected: LandscapeReliefField,
  actual: Pick<LandscapeReliefField, "hillshade" | "salience" | "tangent">,
): number {
  let mismatches = 0;
  for (let index = 0; index < expected.hillshade.length; index += 1)
    if (
      !reliefValueMatches(
        expected.hillshade[index]!,
        actual.hillshade[index]!,
      ) ||
      !reliefValueMatches(expected.salience[index]!, actual.salience[index]!) ||
      !reliefValueMatches(
        expected.tangent[index * 2]!,
        actual.tangent[index * 2]!,
      ) ||
      !reliefValueMatches(
        expected.tangent[index * 2 + 1]!,
        actual.tangent[index * 2 + 1]!,
      )
    )
      mismatches += 1;
  return mismatches;
}

function reliefBorderDigests(
  relief: LandscapeReliefField,
  validity: Uint8Array,
): Readonly<Record<ReliefBorder, string>> {
  const digest = (edge: ReliefBorder) => {
    const indices = borderIndices(relief.columns, relief.rows, edge);
    const hillshade = Float32Array.from(indices, (index) =>
      canonicalReliefValue(relief.hillshade[index]!),
    );
    const salience = Float32Array.from(indices, (index) =>
      canonicalReliefValue(relief.salience[index]!),
    );
    const tangent = Float32Array.from(
      indices.flatMap((index) => [
        canonicalReliefValue(relief.tangent[index * 2]!),
        canonicalReliefValue(relief.tangent[index * 2 + 1]!),
      ]),
    );
    return landscapePyramidContentId("relief-border", [
      Uint8Array.from(indices, (index) => validity[index]!),
      hillshade,
      salience,
      tangent,
    ]);
  };
  return Object.freeze({
    north: digest("north"),
    east: digest("east"),
    south: digest("south"),
    west: digest("west"),
  });
}

function adjacentBorderMismatchCount(
  tiles: readonly MaterializedLandscapeReliefTile[],
): number {
  const indexed = new Map(
    tiles.map((tile) => [`${tile.column}:${tile.row}`, tile]),
  );
  let mismatches = 0;
  for (const tile of tiles) {
    const east = indexed.get(`${tile.column + 1}:${tile.row}`);
    if (east && tile.border_digests.east !== east.border_digests.west)
      mismatches += 1;
    const south = indexed.get(`${tile.column}:${tile.row + 1}`);
    if (south && tile.border_digests.south !== south.border_digests.north)
      mismatches += 1;
  }
  return mismatches;
}

function borderIndices(
  columns: number,
  rows: number,
  edge: ReliefBorder,
): number[] {
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

function cropValidity(
  source: TerrainFieldSetInput,
  interior: MaterializedLandscapeReliefTile["interior"],
): Uint8Array {
  return sampleReliefWindow(
    source.validity.values,
    source.grid.columns,
    interior,
    1,
  );
}

function sampleWindow<T extends Float32Array | Int8Array | Uint8Array>(
  values: T,
  source: TerrainFieldSetInput,
  window: MaterializedLandscapeReliefTile["source_window"],
  components: number,
): T {
  return sampleReliefWindow(values, source.grid.columns, window, components);
}

function sampleReliefWindow<T extends Float32Array | Int8Array | Uint8Array>(
  values: T,
  sourceColumns: number,
  window: MaterializedLandscapeReliefTile["source_window"],
  components: number,
): T {
  const columns = window.column_end - window.column_start + 1;
  const rows = window.row_end - window.row_start + 1;
  const result = (
    values instanceof Float32Array
      ? new Float32Array(columns * rows * components)
      : values instanceof Int8Array
        ? new Int8Array(columns * rows * components)
        : new Uint8Array(columns * rows * components)
  ) as T;
  let output = 0;
  for (let row = window.row_start; row <= window.row_end; row += 1)
    for (
      let column = window.column_start;
      column <= window.column_end;
      column += 1
    ) {
      const sourceOffset = (row * sourceColumns + column) * components;
      for (let component = 0; component < components; component += 1)
        result[output++] = values[sourceOffset + component]!;
    }
  return result;
}

function windowBounds(
  source: TerrainFieldSetInput,
  window: MaterializedLandscapeReliefTile["source_window"],
) {
  const spacingX = source.grid.bounds.width / (source.grid.columns - 1);
  const spacingY = source.grid.bounds.height / (source.grid.rows - 1);
  return {
    x: source.grid.bounds.x + window.column_start * spacingX,
    y: source.grid.bounds.y + window.row_start * spacingY,
    width: (window.column_end - window.column_start) * spacingX,
    height: (window.row_end - window.row_start) * spacingY,
  };
}

function reliefChannelIds(relief: LandscapeReliefField): readonly string[] {
  return [
    `hillshade:${landscapePyramidContentId("hillshade", [relief.hillshade])}`,
    `salience:${landscapePyramidContentId("salience", [relief.salience])}`,
    `tangent:${landscapePyramidContentId("tangent", [relief.tangent])}`,
  ];
}

function landscapeLineage(
  field: TerrainFieldSet,
  hierarchy: MaterializedLandscapeHeightHierarchy,
): readonly LandscapePyramidLineage[] {
  return [
    {
      kind: "field",
      identity: field.field_set_id,
      revision: field.source_revision,
    },
    {
      kind: "height-hierarchy",
      identity: hierarchy.hierarchy_id,
      revision: hierarchy.implementation_revision,
    },
    ...(field.landscape_mosaic
      ? [
          {
            kind: "mosaic",
            identity: field.landscape_mosaic.mosaic_id,
            revision: field.landscape_mosaic.composition_revision,
          },
        ]
      : []),
  ];
}

function supportedElevationRange(
  level: MaterializedLandscapeHeightLevel,
): Readonly<{ minimum: number; maximum: number }> {
  let minimum = Number.POSITIVE_INFINITY;
  let maximum = Number.NEGATIVE_INFINITY;
  for (let index = 0; index < level.elevation.length; index += 1) {
    if (level.validity_classification[index] !== TERRAIN_VALIDITY_VALID)
      continue;
    minimum = Math.min(minimum, level.elevation[index]!);
    maximum = Math.max(maximum, level.elevation[index]!);
  }
  return Number.isFinite(minimum)
    ? Object.freeze({ minimum, maximum })
    : Object.freeze({ minimum: 0, maximum: 0 });
}

function reliefValueMatches(left: number, right: number): boolean {
  return Math.abs(left - right) <= LANDSCAPE_RELIEF_PARTITION_TOLERANCE;
}

function canonicalReliefValue(value: number): number {
  return Math.fround(
    Math.round(value / LANDSCAPE_RELIEF_PARTITION_TOLERANCE) *
      LANDSCAPE_RELIEF_PARTITION_TOLERANCE,
  );
}
