import {
  landscapePyramidContentId,
  TERRAIN_VALIDITY_NO_DATA,
  TERRAIN_VALIDITY_UNSUPPORTED,
  TERRAIN_VALIDITY_VALID,
  verifyTerrainFieldValidityClassification,
} from "@rey/explorer";
import type { TerrainFieldSet } from "./compile";

export const LANDSCAPE_HEIGHT_HIERARCHY_REVISION =
  "rey.terrain.height-hierarchy@1" as const;
export const MAXIMUM_LANDSCAPE_HEIGHT_HIERARCHY_LEVELS = 12;

export interface MaterializedLandscapeHeightLevel {
  level: number;
  level_id: string;
  columns: number;
  rows: number;
  bounds: TerrainFieldSet["grid"]["bounds"];
  sample_spacing_x_meters: number;
  sample_spacing_y_meters: number;
  height_id: string;
  validity_id: string;
  source_contribution_id: string;
  elevation: Float32Array;
  validity_classification: Uint8Array;
  source_offsets: Uint32Array;
  source_indices: Uint32Array;
  valid_vertices: number;
  no_data_vertices: number;
  unsupported_vertices: number;
  byte_length: number;
}

export interface MaterializedLandscapeHeightHierarchy {
  schema: "rey.landscape-height-hierarchy.v1";
  implementation_revision: typeof LANDSCAPE_HEIGHT_HIERARCHY_REVISION;
  hierarchy_id: string;
  field_set_id: string;
  source_revision: string;
  mosaic_id: string;
  source_patch_ids: readonly string[];
  levels: readonly MaterializedLandscapeHeightLevel[];
  byte_length: number;
  validity_policy: "complete_child_window";
  source_set_encoding: "rey.landscape-source-sets.csr.v1";
  complete: boolean;
  omissions: readonly string[];
}

interface MutableHeightLevel {
  columns: number;
  rows: number;
  elevation: Float32Array;
  validity_classification: Uint8Array;
  source_sets: readonly (readonly number[])[];
}

export function compileLandscapeHeightHierarchy(
  field: TerrainFieldSet,
): MaterializedLandscapeHeightHierarchy {
  const metrics = field.relief_metrics;
  const reference = field.landscape_reference;
  if (!metrics || !reference)
    throw new Error("landscape height hierarchy source metadata is incomplete");
  const classification = verifyTerrainFieldValidityClassification(field);
  const sourcePatchIds = field.landscape_height_sources?.patch_ids ?? [
    field.field_set_id,
  ];
  if (
    sourcePatchIds.length === 0 ||
    new Set(sourcePatchIds).size !== sourcePatchIds.length
  )
    throw new Error("landscape height hierarchy source identities are invalid");

  const fineSourceSets = sourceSetsForField(field, classification.values);
  const descending: MutableHeightLevel[] = [
    {
      columns: field.grid.columns,
      rows: field.grid.rows,
      elevation: field.elevation.values.slice(),
      validity_classification: classification.values.slice(),
      source_sets: fineSourceSets,
    },
  ];
  while (
    descending.length < MAXIMUM_LANDSCAPE_HEIGHT_HIERARCHY_LEVELS &&
    canDownsample(descending.at(-1)!)
  )
    descending.push(downsampleHeightLevel(descending.at(-1)!));

  const complete =
    Math.min(descending.at(-1)!.columns, descending.at(-1)!.rows) <= 2;
  const ordered = descending.reverse();
  const levels = Object.freeze(
    ordered.map((source, level) =>
      finalizeHeightLevel(
        source,
        level,
        field.grid.bounds,
        metrics.sample_spacing_x_meters,
        metrics.sample_spacing_y_meters,
        field.grid.columns,
        field.grid.rows,
      ),
    ),
  );
  const mosaicId = field.landscape_mosaic?.mosaic_id ?? reference.reference_id;
  const hierarchyId = landscapePyramidContentId(
    `${LANDSCAPE_HEIGHT_HIERARCHY_REVISION}:${field.field_set_id}:${mosaicId}:${levels
      .map(({ level_id }) => level_id)
      .join("|")}`,
    [levels[0]!.validity_classification],
  );
  return Object.freeze({
    schema: "rey.landscape-height-hierarchy.v1" as const,
    implementation_revision: LANDSCAPE_HEIGHT_HIERARCHY_REVISION,
    hierarchy_id: hierarchyId,
    field_set_id: field.field_set_id,
    source_revision: field.source_revision,
    mosaic_id: mosaicId,
    source_patch_ids: Object.freeze([...sourcePatchIds]),
    levels,
    byte_length: levels.reduce((total, level) => total + level.byte_length, 0),
    validity_policy: "complete_child_window" as const,
    source_set_encoding: "rey.landscape-source-sets.csr.v1" as const,
    complete,
    omissions: Object.freeze(
      complete
        ? []
        : [
            `height hierarchy stopped at ${levels[0]!.columns}x${levels[0]!.rows} after ${MAXIMUM_LANDSCAPE_HEIGHT_HIERARCHY_LEVELS} bounded levels or a non-dyadic source extent`,
          ],
    ),
  });
}

function sourceSetsForField(
  field: TerrainFieldSet,
  classification: Uint8Array,
): readonly (readonly number[])[] {
  const sources = field.landscape_height_sources;
  if (!sources)
    return Object.freeze(
      [...classification].map((value) =>
        Object.freeze(value === TERRAIN_VALIDITY_UNSUPPORTED ? [] : [0]),
      ),
    );
  if (
    sources.primary_owner_indices.length !== classification.length ||
    sources.secondary_owner_indices.length !== classification.length
  )
    throw new Error("landscape height source attribution shape changed");
  return Object.freeze(
    [...classification].map((value, index) => {
      const primary = sources.primary_owner_indices[index]!;
      const secondary = sources.secondary_owner_indices[index]!;
      const contributors = [primary, secondary]
        .filter((owner) => owner !== sources.unsupported_index)
        .sort((left, right) => left - right)
        .filter(
          (owner, ownerIndex, values) => values[ownerIndex - 1] !== owner,
        );
      if (
        contributors.some((owner) => owner >= sources.patch_ids.length) ||
        (value === TERRAIN_VALIDITY_UNSUPPORTED && contributors.length > 0) ||
        (value !== TERRAIN_VALIDITY_UNSUPPORTED && contributors.length === 0)
      )
        throw new Error("landscape height source attribution is invalid");
      return Object.freeze(contributors);
    }),
  );
}

function canDownsample(level: MutableHeightLevel): boolean {
  return (
    level.columns > 2 &&
    level.rows > 2 &&
    level.columns % 2 === 1 &&
    level.rows % 2 === 1
  );
}

function downsampleHeightLevel(child: MutableHeightLevel): MutableHeightLevel {
  const columns = (child.columns + 1) / 2;
  const rows = (child.rows + 1) / 2;
  const cells = columns * rows;
  const elevation = new Float32Array(cells);
  const classification = new Uint8Array(cells);
  const sourceSets: Array<readonly number[]> = new Array(cells);
  for (let row = 0; row < rows; row += 1) {
    for (let column = 0; column < columns; column += 1) {
      const targetIndex = row * columns + column;
      const centerRow = row * 2;
      const centerColumn = column * 2;
      let elevationTotal = 0;
      let samples = 0;
      let allValid = true;
      let unsupported = false;
      const sources = new Set<number>();
      for (
        let childRow = Math.max(0, centerRow - 1);
        childRow <= Math.min(child.rows - 1, centerRow + 1);
        childRow += 1
      ) {
        for (
          let childColumn = Math.max(0, centerColumn - 1);
          childColumn <= Math.min(child.columns - 1, centerColumn + 1);
          childColumn += 1
        ) {
          const childIndex = childRow * child.columns + childColumn;
          const childClass = child.validity_classification[childIndex]!;
          allValid &&= childClass === TERRAIN_VALIDITY_VALID;
          unsupported ||= childClass === TERRAIN_VALIDITY_UNSUPPORTED;
          if (childClass === TERRAIN_VALIDITY_VALID) {
            elevationTotal += child.elevation[childIndex]!;
            samples += 1;
          }
          for (const source of child.source_sets[childIndex]!)
            sources.add(source);
        }
      }
      classification[targetIndex] = allValid
        ? TERRAIN_VALIDITY_VALID
        : unsupported
          ? TERRAIN_VALIDITY_UNSUPPORTED
          : TERRAIN_VALIDITY_NO_DATA;
      if (allValid)
        elevation[targetIndex] = Math.fround(elevationTotal / samples);
      sourceSets[targetIndex] = Object.freeze(
        [...sources].sort((left, right) => left - right),
      );
    }
  }
  return {
    columns,
    rows,
    elevation,
    validity_classification: classification,
    source_sets: Object.freeze(sourceSets),
  };
}

function finalizeHeightLevel(
  source: MutableHeightLevel,
  level: number,
  bounds: TerrainFieldSet["grid"]["bounds"],
  fineSpacingX: number,
  fineSpacingY: number,
  fineColumns: number,
  fineRows: number,
): MaterializedLandscapeHeightLevel {
  const sourceOffsets = new Uint32Array(source.source_sets.length + 1);
  const flattenedSources: number[] = [];
  for (const [index, sources] of source.source_sets.entries()) {
    flattenedSources.push(...sources);
    sourceOffsets[index + 1] = flattenedSources.length;
  }
  const sourceIndices = Uint32Array.from(flattenedSources);
  const heightId = landscapePyramidContentId("height", [source.elevation]);
  const validityId = landscapePyramidContentId("validity-classification", [
    source.validity_classification,
  ]);
  const sourceContributionId = landscapePyramidContentId(
    "height-source-contribution",
    [sourceOffsets, sourceIndices],
  );
  let validVertices = 0;
  let noDataVertices = 0;
  let unsupportedVertices = 0;
  for (const value of source.validity_classification) {
    if (value === TERRAIN_VALIDITY_VALID) validVertices += 1;
    else if (value === TERRAIN_VALIDITY_NO_DATA) noDataVertices += 1;
    else unsupportedVertices += 1;
  }
  const levelId = landscapePyramidContentId(
    `${LANDSCAPE_HEIGHT_HIERARCHY_REVISION}:level:${level}:${source.columns}x${source.rows}:${heightId}:${validityId}:${sourceContributionId}`,
    [source.validity_classification],
  );
  return Object.freeze({
    level,
    level_id: levelId,
    columns: source.columns,
    rows: source.rows,
    bounds,
    sample_spacing_x_meters:
      fineSpacingX * ((fineColumns - 1) / (source.columns - 1)),
    sample_spacing_y_meters:
      fineSpacingY * ((fineRows - 1) / (source.rows - 1)),
    height_id: heightId,
    validity_id: validityId,
    source_contribution_id: sourceContributionId,
    elevation: source.elevation,
    validity_classification: source.validity_classification,
    source_offsets: sourceOffsets,
    source_indices: sourceIndices,
    valid_vertices: validVertices,
    no_data_vertices: noDataVertices,
    unsupported_vertices: unsupportedVertices,
    byte_length:
      source.elevation.byteLength +
      source.validity_classification.byteLength +
      sourceOffsets.byteLength +
      sourceIndices.byteLength,
  });
}
