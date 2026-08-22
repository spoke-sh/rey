import {
  createTerrainValidityClassification,
  summarizeTerrainValidityClassification,
  TERRAIN_VALIDITY_UNSUPPORTED,
  TERRAIN_VALIDITY_VALID,
  verifyTerrainFieldValidityClassification,
  type TerrainFieldSetInput,
} from "@rey/explorer";
import { blake3 } from "@noble/hashes/blake3.js";
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
  "rey.terrain.regional-mosaic@4" as const;
export const MAXIMUM_REGIONAL_TERRAIN_MOSAIC_CELLS = 2_000_000;

export interface RegionalTerrainMosaicPatch {
  member_id: string;
  scene_id: string;
  role: "detail" | "overview";
  authority: {
    identity: string;
    revision: string;
    priority: number;
  };
  field: TerrainFieldSet;
}

interface RegionalTerrainMosaicPlacement {
  patch: RegionalTerrainMosaicPatch;
  validity_classification: NonNullable<
    TerrainFieldSetInput["validity_classification"]
  >;
  column_offset: number;
  row_offset: number;
}

export type RegionalTerrainOverlapDecisionReason =
  | "identical_shared_sample"
  | "detail_source_boundary"
  | "higher_validity"
  | "higher_declared_authority"
  | "finer_nominal_spacing"
  | "stable_source_identity";

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
  relief_metrics: TerrainFieldSetInput["relief_metrics"] | null;
  bounds: { x: number; y: number; width: number; height: number };
  columns: number;
  rows: number;
  valid_vertices: number;
  no_data_vertices: number;
  unsupported_vertices: number;
  shared_vertices: number;
  overlap_vertices: number;
  conflict_vertices: number;
  overview_covered_vertices: number;
  overlap_pairs: readonly (readonly [string, string])[];
  overlap_policy: "validity_authority_resolution_then_stable_identity";
  gap_policy: "unsupported_remains_transparent";
  source_contribution: {
    schema: "rey.landscape-source-contribution.v1";
    content_id: string;
    unsupported_index: number;
    patch_ids: readonly string[];
    owner_indices: Uint32Array;
  };
  conflicts: {
    schema: "rey.landscape-overlap-conflicts.v1";
    content_id: string;
    values: Uint8Array;
  };
  feather: {
    schema: "rey.landscape-overlap-feather.v1";
    content_id: string;
    unsupported_index: number;
    policy: "mutually_valid_equal_authority_equal_spacing_edge_distance";
    secondary_owner_indices: Uint32Array;
    primary_weights: Float32Array;
    feathered_vertices: number;
  };
  overview_coverage: {
    schema: "rey.landscape-overview-coverage.v1";
    content_id: string;
    policy: "separately_admitted_compatible_overview_only";
    patch_ids: readonly string[];
    values: Uint8Array;
    covered_vertices: number;
  };
  overlap_decisions: readonly {
    reason: RegionalTerrainOverlapDecisionReason;
    samples: number;
  }[];
  limits: { maximum_cells: number };
  omissions: readonly string[];
  patches: readonly {
    member_id: string;
    scene_id: string;
    field_set_id: string;
    source_revision: string;
    authority: RegionalTerrainMosaicPatch["authority"];
    role: "detail" | "overview";
    sample_spacing_x: number;
    sample_spacing_y: number;
    nominal_sample_spacing_x_meters: number;
    nominal_sample_spacing_y_meters: number;
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
    !ordered.some(
      ({ field, role }) =>
        field.field_set_id === primaryPatchId && role === "detail",
    ) ||
    ordered.some(
      ({ authority }) =>
        !authority.identity ||
        !authority.revision ||
        !Number.isSafeInteger(authority.priority) ||
        authority.priority < 0,
    )
  )
    throw new Error("regional terrain mosaic patch identity is invalid");

  const bounds = unionBounds(ordered.map(({ field }) => field.grid.bounds));
  const spacingX = sampleSpacing(ordered[0]!.field, "x");
  const spacingY = sampleSpacing(ordered[0]!.field, "y");
  const elevationScale = ordered[0]!.field.elevation_scale;
  const placements = ordered.map((patch) => {
    const { field } = patch;
    const validityClassification =
      verifyTerrainFieldValidityClassification(field);
    if (
      !field.relief_metrics ||
      !sameNumber(sampleSpacing(field, "x"), spacingX) ||
      !sameNumber(sampleSpacing(field, "y"), spacingY) ||
      !sameNumber(field.elevation_scale, elevationScale)
    )
      throw new Error("regional terrain mosaic patch scale is incompatible");
    const columnOffset = alignedOffset(field.grid.bounds.x, bounds.x, spacingX);
    const rowOffset = alignedOffset(field.grid.bounds.y, bounds.y, spacingY);
    return Object.freeze({
      patch,
      validity_classification: validityClassification,
      column_offset: columnOffset,
      row_offset: rowOffset,
    }) satisfies RegionalTerrainMosaicPlacement;
  });
  const columns = alignedExtent(bounds.width, spacingX) + 1;
  const rows = alignedExtent(bounds.height, spacingY) + 1;
  const cells = columns * rows;
  if (!Number.isSafeInteger(cells) || cells > maximumCells)
    throw new Error("regional terrain mosaic exceeds its cell budget");
  const grid = createFieldGrid(columns, rows, bounds);
  const unsupportedOwner = 0xffff_ffff;
  const occupancy = new Uint32Array(cells).fill(unsupportedOwner);
  const coverageCounts = new Uint8Array(cells);
  const overlapTouched = new Uint8Array(cells);
  const conflictValues = new Uint8Array(cells);
  const featherSecondaryOwners = new Uint32Array(cells).fill(unsupportedOwner);
  const featherPrimaryWeights = new Float32Array(cells).fill(1);
  const validityValues = new Uint8Array(cells);
  const validityClassificationValues = new Uint8Array(cells).fill(
    TERRAIN_VALIDITY_UNSUPPORTED,
  );
  const elevationValues = new Float32Array(cells);
  const rainfallValues = new Float32Array(cells);
  const flowDirectionValues = new Float32Array(cells * 2);
  const flowAccumulationValues = new Float32Array(cells);
  const erosionValues = new Float32Array(cells);
  const tintValues = new Float32Array(cells * 3);
  const occlusionValues = new Float32Array(cells);
  const roughnessValues = new Float32Array(cells);
  const overlapPairs = new Set<string>();
  const overlapDecisionCounts = new Map<
    RegionalTerrainOverlapDecisionReason,
    number
  >();
  let sharedVertices = 0;
  let overlapVertices = 0;

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
        if (owner !== unsupportedOwner) {
          const previousCoverage = coverageCounts[targetIndex]!;
          coverageCounts[targetIndex] = Math.min(255, previousCoverage + 1);
          const ownerPlacement = placements[owner]!;
          const ownerPatch = ownerPlacement.patch;
          const pair = [
            ownerPatch.field.field_set_id,
            source.field_set_id,
          ].sort((left, right) => left.localeCompare(right));
          overlapPairs.add(`${pair[0]}\u0000${pair[1]}`);
          sharedVertices += 1;
          if (overlapTouched[targetIndex] === 0) {
            overlapTouched[targetIndex] = 1;
            overlapVertices += 1;
          }
          const ownerRow =
            placement.row_offset + row - ownerPlacement.row_offset;
          const ownerColumn =
            placement.column_offset + column - ownerPlacement.column_offset;
          const ownerSourceIndex =
            ownerRow * ownerPatch.field.grid.columns + ownerColumn;
          if (previousCoverage > 1) {
            elevationValues[targetIndex] =
              ownerPatch.field.elevation.values[ownerSourceIndex]!;
            featherSecondaryOwners[targetIndex] = unsupportedOwner;
            featherPrimaryWeights[targetIndex] = 1;
          }
          const identical = samplesEqual(
            ownerPatch.field,
            ownerPlacement.validity_classification.values,
            ownerSourceIndex,
            source,
            placement.validity_classification.values,
            sourceIndex,
          );
          const decision =
            identical && ownerPatch.role === placement.patch.role
              ? {
                  selected_patch_index: owner,
                  reason: "identical_shared_sample" as const,
                }
              : resolveOverlap(
                  ownerPlacement,
                  ownerSourceIndex,
                  placement,
                  sourceIndex,
                  owner,
                  patchIndex,
                );
          overlapDecisionCounts.set(
            decision.reason,
            (overlapDecisionCounts.get(decision.reason) ?? 0) + 1,
          );
          if (!identical) conflictValues[targetIndex] = 1;
          if (decision.selected_patch_index === patchIndex) {
            occupancy[targetIndex] = patchIndex;
            copySample(
              validityValues,
              validityClassificationValues,
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
              placement.validity_classification.values,
              sourceIndex,
            );
          }
          if (
            !identical &&
            previousCoverage === 1 &&
            canFeatherHeight(
              ownerPlacement,
              ownerSourceIndex,
              placement,
              sourceIndex,
            )
          ) {
            const selectedIsCandidate =
              decision.selected_patch_index === patchIndex;
            const primaryPlacement = selectedIsCandidate
              ? placement
              : ownerPlacement;
            const secondaryPlacement = selectedIsCandidate
              ? ownerPlacement
              : placement;
            const primarySourceIndex = selectedIsCandidate
              ? sourceIndex
              : ownerSourceIndex;
            const secondarySourceIndex = selectedIsCandidate
              ? ownerSourceIndex
              : sourceIndex;
            const primaryWeight = overlapPrimaryWeight(
              primaryPlacement.patch.field,
              primarySourceIndex,
              secondaryPlacement.patch.field,
              secondarySourceIndex,
            );
            if (primaryWeight > 0 && primaryWeight < 1) {
              const primaryHeight =
                primaryPlacement.patch.field.elevation.values[
                  primarySourceIndex
                ]!;
              const secondaryHeight =
                secondaryPlacement.patch.field.elevation.values[
                  secondarySourceIndex
                ]!;
              elevationValues[targetIndex] = Math.fround(
                primaryHeight * primaryWeight +
                  secondaryHeight * (1 - primaryWeight),
              );
              featherSecondaryOwners[targetIndex] = selectedIsCandidate
                ? owner
                : patchIndex;
              featherPrimaryWeights[targetIndex] = primaryWeight;
            }
          }
          continue;
        }
        occupancy[targetIndex] = patchIndex;
        coverageCounts[targetIndex] = 1;
        copySample(
          validityValues,
          validityClassificationValues,
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
          placement.validity_classification.values,
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
  const validityClassification = createTerrainValidityClassification(
    validityClassificationValues,
    `${revision}:validity-classification`,
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
  const overviewPatchIds = Object.freeze(
    ordered.flatMap(({ field, role }) =>
      role === "overview" ? [field.field_set_id] : [],
    ),
  );
  const memberIds = Object.freeze(ordered.map(({ member_id }) => member_id));
  const validitySummary = summarizeTerrainValidityClassification(
    validityClassification,
  );
  const sourceContributionId = mosaicContentId("source-contribution", [
    occupancy,
  ]);
  const conflictId = mosaicContentId("overlap-conflicts", [conflictValues]);
  const featherId = mosaicContentId("overlap-feather", [
    featherSecondaryOwners,
    featherPrimaryWeights,
  ]);
  const overviewCoverageValues = new Uint8Array(cells);
  for (let index = 0; index < cells; index += 1) {
    const owner = occupancy[index]!;
    if (
      owner !== unsupportedOwner &&
      placements[owner]!.patch.role === "overview" &&
      validityClassificationValues[index] === TERRAIN_VALIDITY_VALID
    )
      overviewCoverageValues[index] = 1;
  }
  const overviewCoverageId = mosaicContentId("overview-coverage", [
    overviewCoverageValues,
  ]);
  const heightId = mosaicContentId("height", [elevationValues]);
  const mosaicId = [
    REGIONAL_TERRAIN_MOSAIC_SCHEMA,
    REGIONAL_TERRAIN_MOSAIC_REVISION,
    compositionRevision,
    primaryPatchId,
    ...ordered.flatMap(({ member_id, role, authority, field }) => [
      member_id,
      role,
      authority.identity,
      authority.revision,
      `${authority.priority}`,
      field.field_set_id,
      field.source_revision,
      `${field.relief_metrics!.sample_spacing_x_meters}`,
      `${field.relief_metrics!.sample_spacing_y_meters}`,
    ]),
    heightId,
    validitySummary.validity_id,
    sourceContributionId,
    conflictId,
    featherId,
    overviewCoverageId,
    `${columns}x${rows}`,
  ].join("|");
  const {
    valid_vertices: validVertices,
    no_data_vertices: noDataVertices,
    unsupported_vertices: unsupportedVertices,
  } = validitySummary;
  const conflictVertices = conflictValues.reduce(
    (total, value) => total + (value === 0 ? 0 : 1),
    0,
  );
  const featheredVertices = featherSecondaryOwners.reduce(
    (total, owner) => total + (owner === unsupportedOwner ? 0 : 1),
    0,
  );
  const overviewCoveredVertices = overviewCoverageValues.reduce(
    (total, value) => total + (value === 0 ? 0 : 1),
    0,
  );
  const overlapDecisions = Object.freeze(
    [...overlapDecisionCounts]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([reason, samples]) => Object.freeze({ reason, samples })),
  );
  const pairs = Object.freeze(
    [...overlapPairs]
      .sort((left, right) => left.localeCompare(right))
      .map((pair) => Object.freeze(pair.split("\u0000") as [string, string])),
  );
  const reliefMetrics = regionalMosaicReliefMetrics(ordered);
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
      "validity_authority_resolution_then_stable_identity" as const,
    gap_policy: "unsupported_remains_transparent" as const,
    source_contribution_id: sourceContributionId,
    conflict_id: conflictId,
    conflict_vertices: conflictVertices,
    feather_id: featherId,
    feathered_vertices: featheredVertices,
    overview_coverage_id: overviewCoverageId,
    overview_covered_vertices: overviewCoveredVertices,
    overview_policy: "separately_admitted_compatible_overview_only" as const,
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
    relief_metrics: reliefMetrics,
    bounds,
    columns,
    rows,
    valid_vertices: validVertices,
    no_data_vertices: noDataVertices,
    unsupported_vertices: unsupportedVertices,
    shared_vertices: sharedVertices,
    overlap_vertices: overlapVertices,
    conflict_vertices: conflictVertices,
    overview_covered_vertices: overviewCoveredVertices,
    overlap_pairs: pairs,
    overlap_policy: binding.overlap_policy,
    gap_policy: binding.gap_policy,
    source_contribution: Object.freeze({
      schema: "rey.landscape-source-contribution.v1" as const,
      content_id: sourceContributionId,
      unsupported_index: unsupportedOwner,
      patch_ids: patchIds,
      owner_indices: occupancy,
    }),
    conflicts: Object.freeze({
      schema: "rey.landscape-overlap-conflicts.v1" as const,
      content_id: conflictId,
      values: conflictValues,
    }),
    feather: Object.freeze({
      schema: "rey.landscape-overlap-feather.v1" as const,
      content_id: featherId,
      unsupported_index: unsupportedOwner,
      policy:
        "mutually_valid_equal_authority_equal_spacing_edge_distance" as const,
      secondary_owner_indices: featherSecondaryOwners,
      primary_weights: featherPrimaryWeights,
      feathered_vertices: featheredVertices,
    }),
    overview_coverage: Object.freeze({
      schema: "rey.landscape-overview-coverage.v1" as const,
      content_id: overviewCoverageId,
      policy: "separately_admitted_compatible_overview_only" as const,
      patch_ids: overviewPatchIds,
      values: overviewCoverageValues,
      covered_vertices: overviewCoveredVertices,
    }),
    overlap_decisions: overlapDecisions,
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
          authority: patch.authority,
          role: patch.role,
          sample_spacing_x: sampleSpacing(patch.field, "x"),
          sample_spacing_y: sampleSpacing(patch.field, "y"),
          nominal_sample_spacing_x_meters:
            patch.field.relief_metrics!.sample_spacing_x_meters,
          nominal_sample_spacing_y_meters:
            patch.field.relief_metrics!.sample_spacing_y_meters,
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
      "shared-frame mosaic of a connected terrain-qualified regional set; detail support and explicit detail no-data boundaries take precedence over supplemental overview DEMs, which may cover only otherwise unsupported space; overlap selection is then validity-first, declared-authority, nominal-spacing, and stable-source identity; height feathering is limited to two-source valid/valid overlap at equal authority and nominal spacing, with exact primary/secondary weights retained; every source conflict and final contribution is retained, while source no-data and unsupported gaps retain zero geometry validity",
    source_revision: mosaicId,
    source_summary: Object.freeze({
      columns,
      rows,
      valid_vertices: validVertices,
      no_data_vertices: noDataVertices,
      unsupported_vertices: unsupportedVertices,
      elevation_minimum: sourceMinimum,
      elevation_maximum: sourceMaximum,
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
    relief_metrics: reliefMetrics ?? undefined,
    landscape_reference: Object.freeze({
      schema: "rey.landscape-spatial-reference.v1" as const,
      reference_id: mosaicId,
      coordinate_reference: coordinateReference,
      vertical_reference: verticalReference,
    }),
    field_cells: fieldCellCount(grid),
    field_bytes: fields.reduce(
      (total, channel) => total + fieldByteLength(channel),
      validityClassification.values.byteLength +
        occupancy.byteLength +
        conflictValues.byteLength +
        featherSecondaryOwners.byteLength +
        featherPrimaryWeights.byteLength +
        overviewCoverageValues.byteLength,
    ),
    landscape_mosaic: binding,
  }) satisfies TerrainFieldSet;
  return Object.freeze({ manifest, field });
}

function regionalMosaicReliefMetrics(
  patches: readonly RegionalTerrainMosaicPatch[],
): TerrainFieldSetInput["relief_metrics"] | null {
  const metrics = patches.flatMap(({ field }) =>
    field.relief_metrics ? [field.relief_metrics] : [],
  );
  if (metrics.length !== patches.length) return null;
  const elevationRange = metrics[0]!.elevation_range_meters;
  if (
    metrics.some(
      (metric) => !sameNumber(metric.elevation_range_meters, elevationRange),
    )
  )
    throw new Error("regional terrain mosaic relief elevation scale conflicts");
  return Object.freeze({
    schema: "rey.terrain-relief-metrics.v1" as const,
    sample_spacing_x_meters:
      metrics.reduce(
        (total, metric) => total + metric.sample_spacing_x_meters,
        0,
      ) / metrics.length,
    sample_spacing_y_meters:
      metrics.reduce(
        (total, metric) => total + metric.sample_spacing_y_meters,
        0,
      ) / metrics.length,
    elevation_range_meters: elevationRange,
    authority:
      "component-average local metric relief scale from every exact admitted CRS84 member grid; presentation only and not a geodetic transform",
  });
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

function resolveOverlap(
  owner: RegionalTerrainMosaicPlacement,
  ownerSourceIndex: number,
  candidate: RegionalTerrainMosaicPlacement,
  candidateSourceIndex: number,
  ownerPatchIndex: number,
  candidatePatchIndex: number,
): {
  selected_patch_index: number;
  reason: Exclude<
    RegionalTerrainOverlapDecisionReason,
    "identical_shared_sample"
  >;
} {
  if (owner.patch.role !== candidate.patch.role) {
    const detailIsCandidate = candidate.patch.role === "detail";
    const detail = detailIsCandidate ? candidate : owner;
    const detailSourceIndex = detailIsCandidate
      ? candidateSourceIndex
      : ownerSourceIndex;
    if (
      detail.validity_classification.values[detailSourceIndex] !==
      TERRAIN_VALIDITY_UNSUPPORTED
    )
      return {
        selected_patch_index: detailIsCandidate
          ? candidatePatchIndex
          : ownerPatchIndex,
        reason: "detail_source_boundary",
      };
  }
  const ownerValidity = validityRank(
    owner.validity_classification.values[ownerSourceIndex]!,
  );
  const candidateValidity = validityRank(
    candidate.validity_classification.values[candidateSourceIndex]!,
  );
  if (ownerValidity !== candidateValidity)
    return {
      selected_patch_index:
        candidateValidity > ownerValidity
          ? candidatePatchIndex
          : ownerPatchIndex,
      reason: "higher_validity",
    };
  if (owner.patch.authority.priority !== candidate.patch.authority.priority)
    return {
      selected_patch_index:
        candidate.patch.authority.priority > owner.patch.authority.priority
          ? candidatePatchIndex
          : ownerPatchIndex,
      reason: "higher_declared_authority",
    };
  const ownerSpacing = nominalMetricSampleArea(owner.patch.field);
  const candidateSpacing = nominalMetricSampleArea(candidate.patch.field);
  if (!sameNumber(ownerSpacing, candidateSpacing))
    return {
      selected_patch_index:
        candidateSpacing < ownerSpacing ? candidatePatchIndex : ownerPatchIndex,
      reason: "finer_nominal_spacing",
    };
  return {
    selected_patch_index:
      stablePatchIdentity(candidate.patch).localeCompare(
        stablePatchIdentity(owner.patch),
      ) < 0
        ? candidatePatchIndex
        : ownerPatchIndex,
    reason: "stable_source_identity",
  };
}

function samplesEqual(
  left: TerrainFieldSet,
  leftClassification: Uint8Array,
  leftIndex: number,
  right: TerrainFieldSet,
  rightClassification: Uint8Array,
  rightIndex: number,
): boolean {
  const scalarChannels = [
    [left.validity.values, right.validity.values],
    [leftClassification, rightClassification],
    [left.elevation.values, right.elevation.values],
    [left.material.occlusion, right.material.occlusion],
    [left.material.roughness, right.material.roughness],
  ] as const;
  if (
    scalarChannels.some(
      ([leftValues, rightValues]) =>
        leftValues[leftIndex] !== rightValues[rightIndex],
    )
  )
    return false;
  return componentsEqual(
    left.material.tint,
    leftIndex,
    right.material.tint,
    rightIndex,
    3,
  );
}

function componentsEqual(
  left: Float32Array | Int8Array,
  leftIndex: number,
  right: Float32Array | Int8Array,
  rightIndex: number,
  components: number,
): boolean {
  for (let component = 0; component < components; component += 1)
    if (
      left[leftIndex * components + component] !==
      right[rightIndex * components + component]
    )
      return false;
  return true;
}

function validityRank(value: number): number {
  return value === 1 ? 2 : value === 2 ? 1 : 0;
}

function nominalMetricSampleArea(field: TerrainFieldSet): number {
  const metrics = field.relief_metrics;
  if (!metrics)
    throw new Error("regional terrain mosaic patch metric spacing is unbound");
  return metrics.sample_spacing_x_meters * metrics.sample_spacing_y_meters;
}

function canFeatherHeight(
  left: RegionalTerrainMosaicPlacement,
  leftSourceIndex: number,
  right: RegionalTerrainMosaicPlacement,
  rightSourceIndex: number,
): boolean {
  return (
    left.patch.role === right.patch.role &&
    left.validity_classification.values[leftSourceIndex] ===
      TERRAIN_VALIDITY_VALID &&
    right.validity_classification.values[rightSourceIndex] ===
      TERRAIN_VALIDITY_VALID &&
    left.patch.authority.priority === right.patch.authority.priority &&
    sameNumber(
      nominalMetricSampleArea(left.patch.field),
      nominalMetricSampleArea(right.patch.field),
    ) &&
    left.patch.field.elevation.values[leftSourceIndex] !==
      right.patch.field.elevation.values[rightSourceIndex]
  );
}

function overlapPrimaryWeight(
  primary: TerrainFieldSet,
  primarySourceIndex: number,
  secondary: TerrainFieldSet,
  secondarySourceIndex: number,
): number {
  const primaryDistance = sampleBoundaryDistance(primary, primarySourceIndex);
  const secondaryDistance = sampleBoundaryDistance(
    secondary,
    secondarySourceIndex,
  );
  const totalDistance = primaryDistance + secondaryDistance;
  return totalDistance === 0 ? 1 : primaryDistance / totalDistance;
}

function sampleBoundaryDistance(
  field: TerrainFieldSet,
  sourceIndex: number,
): number {
  const row = Math.floor(sourceIndex / field.grid.columns);
  const column = sourceIndex % field.grid.columns;
  return Math.min(
    row,
    column,
    field.grid.rows - 1 - row,
    field.grid.columns - 1 - column,
  );
}

function stablePatchIdentity(patch: RegionalTerrainMosaicPatch): string {
  return [
    patch.authority.identity,
    patch.authority.revision,
    patch.field.field_set_id,
    patch.field.source_revision,
  ].join("\u0000");
}

function copySample(
  validity: Uint8Array,
  validityClassification: Uint8Array,
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
  sourceValidityClassification: Uint8Array,
  sourceIndex: number,
): void {
  validity[targetIndex] = source.validity.values[sourceIndex]!;
  validityClassification[targetIndex] =
    sourceValidityClassification[sourceIndex]!;
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

function mosaicContentId(
  channel: string,
  arrays: readonly (Float32Array | Uint32Array | Uint8Array)[],
): string {
  const header = new TextEncoder().encode(
    JSON.stringify({
      channel,
      byte_lengths: arrays.map(({ byteLength }) => byteLength),
    }),
  );
  const bytes = new Uint8Array(
    header.length +
      arrays.reduce((total, array) => total + array.byteLength, 0),
  );
  bytes.set(header);
  let offset = header.length;
  for (const array of arrays) {
    const content = new Uint8Array(
      array.buffer,
      array.byteOffset,
      array.byteLength,
    );
    bytes.set(content, offset);
    offset += content.length;
  }
  return `blake3:${[...blake3(bytes)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("")}`;
}
