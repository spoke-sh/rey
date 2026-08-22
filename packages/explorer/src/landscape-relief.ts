import {
  planarPresentationSamples,
  type PlanarPresentationSample,
} from "./globe-samples";
import type { TerrainFieldSetInput } from "./types";

export const LANDSCAPE_RELIEF_ENGINE_REVISION =
  "rey.landscape-relief-engine@4" as const;
export const LANDSCAPE_METRIC_GRADIENT_REVISION =
  "rey.landscape.metric-gradient@1" as const;
export const LANDSCAPE_MDOW_REVISION = "rey.landscape.mdow@1" as const;
export const LANDSCAPE_OPENNESS_REVISION = "rey.landscape.openness@1" as const;
export const LANDSCAPE_RIDGE_SALIENCE_REVISION =
  "rey.landscape.ridge-salience@1" as const;
export const LANDSCAPE_TONE_MAPPING_REVISION =
  "rey.landscape.linear-tone-map@1" as const;
export const LANDSCAPE_TERRAIN_FABRIC_REVISION =
  "rey.landscape-terrain-fabric@2" as const;
export const LANDSCAPE_PATCH_SET_REVISION =
  "rey.landscape-patch-set@1" as const;

export interface LandscapeReliefField {
  schema: "rey.landscape-relief-field.v4";
  implementation_revision: typeof LANDSCAPE_RELIEF_ENGINE_REVISION;
  relief_field_id: string;
  field_set_id: string;
  source_field_set_id: string;
  source_relief_field_id: string | null;
  derivation_scope: "complete_field" | "sampled_from_complete_field";
  maximum_support_radius_cells: number;
  scale_basis: "metric_source_spacing" | "presentation_grid_spacing";
  scales: readonly {
    id: "local" | "midslope" | "regional";
    target_radius_meters: number | null;
    support_radius_cells: number;
    support_radius_meters: number | null;
    weight: number;
    supported: boolean;
    channel_id: string;
    gradient_revision: typeof LANDSCAPE_METRIC_GRADIENT_REVISION;
    illumination_revision: typeof LANDSCAPE_MDOW_REVISION;
    openness_revision: typeof LANDSCAPE_OPENNESS_REVISION;
  }[];
  operators: Readonly<{
    metric_gradient: typeof LANDSCAPE_METRIC_GRADIENT_REVISION;
    mdow: typeof LANDSCAPE_MDOW_REVISION;
    openness: typeof LANDSCAPE_OPENNESS_REVISION;
    ridge_salience: typeof LANDSCAPE_RIDGE_SALIENCE_REVISION;
    tone_mapping: typeof LANDSCAPE_TONE_MAPPING_REVISION;
    lighting_owner: "renderer_neutral_relief_field";
  }>;
  columns: number;
  rows: number;
  slope: Float32Array;
  aspect: Float32Array;
  mdow: Float32Array;
  sky_view_factor: Float32Array;
  openness: Float32Array;
  profile_curvature: Float32Array;
  plan_curvature: Float32Array;
  local_contrast: Float32Array;
  hillshade: Float32Array;
  salience: Float32Array;
  tangent: Float32Array;
}

export interface LandscapeTerrainFabricSample extends PlanarPresentationSample {
  sample_id: string;
  source_sample_id: string;
  source_field_set_id: string;
  source_relief_field_id: string;
  source_column: number;
  source_row: number;
  relief: number;
  tangent_u: number;
  tangent_v: number;
  length: number;
  reveal_priority: number;
}

export interface LandscapePatchSet {
  schema: "rey.landscape-patch-set.v1";
  implementation_revision: typeof LANDSCAPE_PATCH_SET_REVISION;
  patch_set_id: string;
  patch_ids: readonly string[];
  overlap_pairs: readonly (readonly [string, string])[];
  bounds: { x: number; y: number; width: number; height: number } | null;
  overlap_policy:
    | "later_patch_wins_with_deterministic_depth_bias"
    | "qualified_shared_samples_must_match_before_derivation"
    | "validity_authority_resolution_then_stable_identity";
  gap_policy: "unsupported_remains_transparent";
}

export function landscapeReliefFieldByteLength(
  relief: LandscapeReliefField,
): number {
  return (
    relief.slope.byteLength +
    relief.aspect.byteLength +
    relief.mdow.byteLength +
    relief.sky_view_factor.byteLength +
    relief.openness.byteLength +
    relief.profile_curvature.byteLength +
    relief.plan_curvature.byteLength +
    relief.local_contrast.byteLength +
    relief.hillshade.byteLength +
    relief.salience.byteLength +
    relief.tangent.byteLength
  );
}

const FALLBACK_RELIEF_RADII = Object.freeze([0, 2, 8] as const);
const RELIEF_SCALE_TARGETS = Object.freeze([
  { id: "local", radius_meters: 350, weight: 0.68 },
  { id: "midslope", radius_meters: 1_400, weight: 0.22 },
  { id: "regional", radius_meters: 5_600, weight: 0.1 },
] as const);
export const LANDSCAPE_RELIEF_MAXIMUM_SUPPORT_RADIUS_CELLS = 64;
const KEY_LIGHT = cartographicLight(315, 42);
const FILL_LIGHT = cartographicLight(225, 28);
const BACK_LIGHT = cartographicLight(45, 24);
const CARTOGRAPHIC_RELIEF_VERTICAL_EXAGGERATION = 5;
const OPENNESS_DIRECTIONS = Object.freeze([
  [-1, 0],
  [-1, -1],
  [0, -1],
  [1, -1],
  [1, 0],
  [1, 1],
  [0, 1],
  [-1, 1],
] as const);

/**
 * Binds one ordered landscape from zero or more admitted terrain patches.
 * Patch order is semantic input: later patches win coincident depth without
 * changing either patch's validity. Unsupported space between patch masks is
 * deliberately left transparent.
 */
export function compileLandscapePatchSet(
  fields: readonly TerrainFieldSetInput[],
): LandscapePatchSet {
  const mosaic = landscapeMosaicBinding(fields);
  if (mosaic)
    return Object.freeze({
      schema: "rey.landscape-patch-set.v1",
      implementation_revision: LANDSCAPE_PATCH_SET_REVISION,
      patch_set_id: `${LANDSCAPE_PATCH_SET_REVISION}:${mosaic.mosaic_id}`,
      patch_ids: mosaic.patch_ids,
      overlap_pairs: mosaic.overlap_pairs,
      bounds: mosaic.bounds,
      overlap_policy: mosaic.overlap_policy,
      gap_policy: mosaic.gap_policy,
    });
  const patchIds = fields.map(({ field_set_id }) => field_set_id);
  if (new Set(patchIds).size !== patchIds.length)
    throw new Error("landscape patch set contains duplicate field identity");
  const overlapPairs: Array<readonly [string, string]> = [];
  for (let left = 0; left < fields.length; left += 1)
    for (let right = left + 1; right < fields.length; right += 1)
      if (boundsOverlap(fields[left]!.grid.bounds, fields[right]!.grid.bounds))
        overlapPairs.push(
          Object.freeze([
            fields[left]!.field_set_id,
            fields[right]!.field_set_id,
          ] as const),
        );
  const bounds = fields.reduce<LandscapePatchSet["bounds"]>((union, field) => {
    const next = field.grid.bounds;
    if (!union) return Object.freeze({ ...next });
    const left = Math.min(union.x, next.x);
    const top = Math.min(union.y, next.y);
    const right = Math.max(union.x + union.width, next.x + next.width);
    const bottom = Math.max(union.y + union.height, next.y + next.height);
    return Object.freeze({
      x: left,
      y: top,
      width: right - left,
      height: bottom - top,
    });
  }, null);
  return Object.freeze({
    schema: "rey.landscape-patch-set.v1",
    implementation_revision: LANDSCAPE_PATCH_SET_REVISION,
    patch_set_id: `${LANDSCAPE_PATCH_SET_REVISION}:${patchIds.join("|") || "empty"}`,
    patch_ids: Object.freeze(patchIds),
    overlap_pairs: Object.freeze(overlapPairs),
    bounds,
    overlap_policy: "later_patch_wins_with_deterministic_depth_bias",
    gap_policy: "unsupported_remains_transparent",
  });
}

function landscapeMosaicBinding(
  fields: readonly TerrainFieldSetInput[],
): NonNullable<TerrainFieldSetInput["landscape_mosaic"]> | null {
  const bound = fields.filter((field) => field.landscape_mosaic !== undefined);
  if (bound.length === 0) return null;
  if (bound.length !== fields.length)
    throw new Error("landscape patch set mixes mosaic and unbound fields");
  const first = bound[0]!.landscape_mosaic!;
  const identity = JSON.stringify(first);
  if (
    !first.mosaic_id ||
    new Set(first.patch_ids).size !== first.patch_ids.length ||
    fields.some((field) => JSON.stringify(field.landscape_mosaic) !== identity)
  )
    throw new Error("landscape patch set mosaic binding is invalid");
  return first;
}

/**
 * Compiles a renderer-neutral relief field from admitted terrain channels.
 * Every non-local scale requires a completely valid support window, so a
 * no-data edge cannot cast invented light, shadow, or stipple structure into
 * the admitted surface.
 */
export function deriveLandscapeReliefField(
  field: TerrainFieldSetInput,
): LandscapeReliefField {
  verifyTerrainFieldShape(field);
  const { columns, rows } = field.grid;
  const cells = field.field_cells;
  const supportPrefix = prefixSum(columns, rows, (index) =>
    field.validity.values[index] === 0 ? 0 : 1,
  );
  const elevationPrefix = prefixSum(columns, rows, (index) =>
    field.validity.values[index] === 0 ? 0 : field.elevation.values[index]!,
  );
  const elevationSquarePrefix = prefixSum(columns, rows, (index) =>
    field.validity.values[index] === 0
      ? 0
      : field.elevation.values[index]! ** 2,
  );
  const elevationSpan = Math.max(
    0.000_001,
    field.relief_metrics?.elevation_value_minimum !== undefined &&
      field.relief_metrics.elevation_value_maximum !== undefined
      ? field.relief_metrics.elevation_value_maximum -
          field.relief_metrics.elevation_value_minimum
      : maximumSupported(field.elevation.values, field.validity.values) -
          minimumSupported(field.elevation.values, field.validity.values),
  );
  const scaleContract = landscapeReliefScales(field);
  const metricSupportRadius = maximumSupportedScaleRadius(scaleContract);
  const contrastRadius =
    metricSupportRadius === 0
      ? 0
      : Math.max(1, Math.min(4, metricSupportRadius));
  const maximumSupportRadius = metricSupportRadius + contrastRadius;
  const spacingX =
    field.relief_metrics?.sample_spacing_x_meters ??
    field.grid.bounds.width / (columns - 1);
  const spacingY =
    field.relief_metrics?.sample_spacing_y_meters ??
    field.grid.bounds.height / (rows - 1);
  const verticalScale =
    field.relief_metrics?.elevation_range_meters ?? field.elevation_scale;
  const slope = new Float32Array(cells);
  const aspect = new Float32Array(cells);
  const mdow = new Float32Array(cells);
  const skyViewFactor = new Float32Array(cells);
  const openness = new Float32Array(cells);
  const profileCurvature = new Float32Array(cells);
  const planCurvature = new Float32Array(cells);
  const localContrast = new Float32Array(cells);
  const hillshade = new Float32Array(cells);
  const salience = new Float32Array(cells);
  const tangent = new Float32Array(cells * 2);
  const activeSupport = new Uint8Array(cells);

  for (let row = 0; row < rows; row += 1) {
    for (let column = 0; column < columns; column += 1) {
      const index = row * columns + column;
      if (field.validity.values[index] === 0) continue;
      let illumination = 0;
      let slopeTotal = 0;
      let aspectX = 0;
      let aspectY = 0;
      let skyView = 0;
      let opennessTotal = 0;
      let profileTotal = 0;
      let planTotal = 0;
      let position = 0;
      let weightTotal = 0;

      for (const scale of scaleContract) {
        if (!scale.supported) continue;
        const radius = scale.support_radius_cells;
        const weight = scale.weight;
        const window = completeWindow(
          supportPrefix,
          columns,
          rows,
          column,
          row,
          radius,
        );
        if (!window) continue;
        const inverseArea = 1 / window.area;
        const normal =
          radius === 0
            ? normalize3(
                field.normal.values[index * 3]!,
                field.normal.values[index * 3 + 1]!,
                field.normal.values[index * 3 + 2]!,
              )
            : elevationNormal(
                field.elevation.values,
                columns,
                column,
                row,
                radius,
                spacingX,
                spacingY,
                verticalScale,
              );
        const scaleSlope = Math.atan2(
          Math.hypot(normal[0], normal[1]),
          normal[2],
        );
        const scaleAspect = Math.atan2(-normal[1], -normal[0]);
        const scaleIllumination = mdowIllumination(normal, scaleSlope);
        const scaleOpenness = directionalOpenness(
          field.elevation.values,
          columns,
          column,
          row,
          radius,
          spacingX,
          spacingY,
          verticalScale,
        );
        illumination += scaleIllumination * weight;
        slopeTotal += scaleSlope * weight;
        aspectX += Math.cos(scaleAspect) * weight;
        aspectY += Math.sin(scaleAspect) * weight;
        skyView += scaleOpenness.sky_view_factor * weight;
        opennessTotal += scaleOpenness.openness * weight;
        const meanElevation =
          rectangleSum(elevationPrefix, columns, window) * inverseArea;
        const meanElevationSquare =
          rectangleSum(elevationSquarePrefix, columns, window) * inverseArea;
        const localDeviation = Math.sqrt(
          Math.max(0, meanElevationSquare - meanElevation ** 2),
        );
        const scaleRange = Math.max(
          elevationSpan * 0.003,
          localDeviation * 2.2,
        );
        position +=
          clamp(
            (field.elevation.values[index]! - meanElevation) / scaleRange,
            -1,
            1,
          ) * weight;
        const curvature = metricCurvature(
          field.elevation.values,
          columns,
          column,
          row,
          radius,
          elevationSpan,
        );
        profileTotal += curvature.profile * weight;
        planTotal += curvature.plan * weight;
        weightTotal += weight;
      }

      if (weightTotal === 0) {
        mdow[index] = 1;
        skyViewFactor[index] = 1;
        localContrast[index] = 0.5;
        hillshade[index] = 1;
        tangent[index * 2] = 1;
        continue;
      }
      const inverseWeight = 1 / weightTotal;
      activeSupport[index] = 1;
      slope[index] = Math.fround(slopeTotal * inverseWeight);
      aspect[index] = Math.fround(Math.atan2(aspectY, aspectX));
      mdow[index] = Math.fround(illumination * inverseWeight);
      skyViewFactor[index] = Math.fround(skyView * inverseWeight);
      openness[index] = Math.fround(opennessTotal * inverseWeight);
      profileCurvature[index] = Math.fround(profileTotal * inverseWeight);
      planCurvature[index] = Math.fround(planTotal * inverseWeight);
      const highFrequency =
        Math.max(0, profileCurvature[index]!) * 0.52 +
        Math.abs(planCurvature[index]!) * 0.28;
      salience[index] = Math.fround(
        clamp(
          highFrequency +
            Math.sin(Math.min(Math.PI / 2, slope[index]!)) * 0.28 +
            Math.abs(position * inverseWeight) * 0.12,
          0,
          1,
        ),
      );
      tangent[index * 2] = Math.fround(-Math.sin(aspect[index]!));
      tangent[index * 2 + 1] = Math.fround(Math.cos(aspect[index]!));
    }
  }

  const mdowPrefix = prefixSum(columns, rows, (index) => mdow[index]!);
  const mdowSquarePrefix = prefixSum(
    columns,
    rows,
    (index) => mdow[index]! ** 2,
  );
  for (let row = 0; row < rows; row += 1) {
    for (let column = 0; column < columns; column += 1) {
      const index = row * columns + column;
      if (field.validity.values[index] === 0 || activeSupport[index] === 0)
        continue;
      const window = completeWindow(
        supportPrefix,
        columns,
        rows,
        column,
        row,
        contrastRadius,
      );
      let contrast = 0.5;
      if (window) {
        const inverseArea = 1 / window.area;
        const mean = rectangleSum(mdowPrefix, columns, window) * inverseArea;
        const variance = Math.max(
          0,
          rectangleSum(mdowSquarePrefix, columns, window) * inverseArea -
            mean ** 2,
        );
        contrast = clamp(
          0.5 + (mdow[index]! - mean) / Math.max(0.16, Math.sqrt(variance) * 4),
          0,
          1,
        );
      }
      localContrast[index] = Math.fround(contrast);
      const contrastIllumination = 0.62 + contrast * 0.76;
      const ambientSky = 0.58 + skyViewFactor[index]! * 0.42;
      const opennessFill = 0.94 + openness[index]! * 0.06;
      const linear =
        (mdow[index]! * 0.7 + contrastIllumination * 0.3) *
          ambientSky *
          opennessFill +
        salience[index]! * 0.04;
      const reinhard = linear / (linear + 0.35);
      hillshade[index] = Math.fround(clamp(reinhard / (1 / 1.35), 0.28, 1.2));
    }
  }

  return Object.freeze({
    schema: "rey.landscape-relief-field.v4",
    implementation_revision: LANDSCAPE_RELIEF_ENGINE_REVISION,
    relief_field_id: [
      LANDSCAPE_RELIEF_ENGINE_REVISION,
      field.field_set_id,
      field.source_revision ?? "source-revision:unbound",
      `${columns}x${rows}`,
    ].join("|"),
    field_set_id: field.field_set_id,
    source_field_set_id: field.field_set_id,
    source_relief_field_id: null,
    derivation_scope: "complete_field",
    maximum_support_radius_cells: maximumSupportRadius,
    scale_basis: field.relief_metrics
      ? "metric_source_spacing"
      : "presentation_grid_spacing",
    scales: scaleContract,
    operators: reliefOperatorContract(),
    columns,
    rows,
    slope,
    aspect,
    mdow,
    sky_view_factor: skyViewFactor,
    openness,
    profile_curvature: profileCurvature,
    plan_curvature: planCurvature,
    local_contrast: localContrast,
    hillshade,
    salience,
    tangent,
  });
}

/**
 * Samples render-tile relief from one field-wide derivation. This is
 * deliberately not another relief evaluation: internal tile borders retain
 * the exact neighborhood support and values of the complete source field.
 */
export function sampleLandscapeReliefField(
  relief: LandscapeReliefField,
  source: TerrainFieldSetInput,
  targetFieldSetId: string,
  columnIndices: readonly number[],
  rowIndices: readonly number[],
): LandscapeReliefField {
  if (relief.derivation_scope !== "complete_field")
    throw new Error("landscape relief tiles require a complete source field");
  verifyLandscapeReliefField(source, relief);
  if (!targetFieldSetId)
    throw new Error("landscape relief tile identity is required");
  verifySampleIndices(columnIndices, source.grid.columns, "column");
  verifySampleIndices(rowIndices, source.grid.rows, "row");
  const cells = columnIndices.length * rowIndices.length;
  const result = Object.freeze({
    schema: "rey.landscape-relief-field.v4" as const,
    implementation_revision: LANDSCAPE_RELIEF_ENGINE_REVISION,
    relief_field_id: `${relief.relief_field_id}|sample:${targetFieldSetId}`,
    field_set_id: targetFieldSetId,
    source_field_set_id: relief.source_field_set_id,
    source_relief_field_id: relief.relief_field_id,
    derivation_scope: "sampled_from_complete_field" as const,
    maximum_support_radius_cells: relief.maximum_support_radius_cells,
    scale_basis: relief.scale_basis,
    scales: relief.scales,
    operators: relief.operators,
    columns: columnIndices.length,
    rows: rowIndices.length,
    slope: sampleReliefComponents(
      relief.slope,
      source.grid.columns,
      columnIndices,
      rowIndices,
      1,
    ),
    aspect: sampleReliefComponents(
      relief.aspect,
      source.grid.columns,
      columnIndices,
      rowIndices,
      1,
    ),
    mdow: sampleReliefComponents(
      relief.mdow,
      source.grid.columns,
      columnIndices,
      rowIndices,
      1,
    ),
    sky_view_factor: sampleReliefComponents(
      relief.sky_view_factor,
      source.grid.columns,
      columnIndices,
      rowIndices,
      1,
    ),
    openness: sampleReliefComponents(
      relief.openness,
      source.grid.columns,
      columnIndices,
      rowIndices,
      1,
    ),
    profile_curvature: sampleReliefComponents(
      relief.profile_curvature,
      source.grid.columns,
      columnIndices,
      rowIndices,
      1,
    ),
    plan_curvature: sampleReliefComponents(
      relief.plan_curvature,
      source.grid.columns,
      columnIndices,
      rowIndices,
      1,
    ),
    local_contrast: sampleReliefComponents(
      relief.local_contrast,
      source.grid.columns,
      columnIndices,
      rowIndices,
      1,
    ),
    hillshade: sampleReliefComponents(
      relief.hillshade,
      source.grid.columns,
      columnIndices,
      rowIndices,
      1,
    ),
    salience: sampleReliefComponents(
      relief.salience,
      source.grid.columns,
      columnIndices,
      rowIndices,
      1,
    ),
    tangent: sampleReliefComponents(
      relief.tangent,
      source.grid.columns,
      columnIndices,
      rowIndices,
      2,
    ),
  });
  if (
    result.slope.length !== cells ||
    result.aspect.length !== cells ||
    result.mdow.length !== cells ||
    result.sky_view_factor.length !== cells ||
    result.openness.length !== cells ||
    result.profile_curvature.length !== cells ||
    result.plan_curvature.length !== cells ||
    result.local_contrast.length !== cells ||
    result.hillshade.length !== cells ||
    result.salience.length !== cells ||
    result.tangent.length !== cells * 2
  )
    throw new Error("sampled landscape relief shape changed");
  return result;
}

export function verifyLandscapeReliefField(
  field: TerrainFieldSetInput,
  relief: LandscapeReliefField,
): void {
  verifyTerrainFieldShape(field);
  const expectedScales = landscapeReliefScales(field);
  const scaleContractMatches =
    relief.derivation_scope === "complete_field"
      ? JSON.stringify(relief.scales) === JSON.stringify(expectedScales)
      : relief.derivation_scope === "sampled_from_complete_field" &&
        JSON.stringify(relief.scales.map(scaleGeometry)) ===
          JSON.stringify(expectedScales.map(scaleGeometry));
  const expectedMaximumSupportRadius =
    relief.derivation_scope === "complete_field"
      ? effectiveReliefSupportRadius(expectedScales)
      : effectiveReliefSupportRadius(relief.scales);
  if (
    relief.schema !== "rey.landscape-relief-field.v4" ||
    relief.implementation_revision !== LANDSCAPE_RELIEF_ENGINE_REVISION ||
    relief.field_set_id !== field.field_set_id ||
    relief.maximum_support_radius_cells !== expectedMaximumSupportRadius ||
    relief.scale_basis !==
      (field.relief_metrics
        ? "metric_source_spacing"
        : "presentation_grid_spacing") ||
    !scaleContractMatches ||
    JSON.stringify(relief.operators) !==
      JSON.stringify(reliefOperatorContract()) ||
    relief.columns !== field.grid.columns ||
    relief.rows !== field.grid.rows ||
    relief.slope.length !== field.field_cells ||
    relief.aspect.length !== field.field_cells ||
    relief.mdow.length !== field.field_cells ||
    relief.sky_view_factor.length !== field.field_cells ||
    relief.openness.length !== field.field_cells ||
    relief.profile_curvature.length !== field.field_cells ||
    relief.plan_curvature.length !== field.field_cells ||
    relief.local_contrast.length !== field.field_cells ||
    relief.hillshade.length !== field.field_cells ||
    relief.salience.length !== field.field_cells ||
    relief.tangent.length !== field.field_cells * 2
  )
    throw new Error("landscape relief field does not match terrain input");
}

function scaleGeometry(
  scale: LandscapeReliefField["scales"][number],
): Omit<LandscapeReliefField["scales"][number], "supported"> {
  const { supported: _supported, ...geometry } = scale;
  return geometry;
}

function maximumSupportedScaleRadius(
  scales: LandscapeReliefField["scales"],
): number {
  return Math.max(
    0,
    ...scales
      .filter(({ supported }) => supported)
      .map(({ support_radius_cells }) => support_radius_cells),
  );
}

function effectiveReliefSupportRadius(
  scales: LandscapeReliefField["scales"],
): number {
  const metricRadius = maximumSupportedScaleRadius(scales);
  return (
    metricRadius +
    (metricRadius === 0 ? 0 : Math.max(1, Math.min(4, metricRadius)))
  );
}

/**
 * Projects exact valid samples from the supplied Landscape relief level into
 * the Atlas/planar dot vocabulary. The planar sequence only chooses a bounded
 * deterministic subset; every emitted position retains its exact source row,
 * column, field, and relief identity. Terrain content controls density,
 * brightness, contour tangent, mark length, and reveal ordering.
 */
export function landscapeTerrainFabricSamples(
  field: TerrainFieldSetInput & { source_revision?: string },
  relief: LandscapeReliefField,
  candidateCount = 6_000,
): readonly LandscapeTerrainFabricSample[] {
  verifyLandscapeReliefField(field, relief);
  if (!Number.isSafeInteger(candidateCount) || candidateCount < 1)
    throw new Error("landscape terrain fabric sample bound is invalid");
  const revision = `${LANDSCAPE_TERRAIN_FABRIC_REVISION}:${relief.relief_field_id}`;
  const samplesBySource = new Map<number, LandscapeTerrainFabricSample>();
  for (const [sequence, sample] of planarPresentationSamples(
    revision,
    candidateCount * 4,
  ).entries()) {
    const column = Math.min(
      field.grid.columns - 1,
      Math.max(0, Math.round(sample.u * (field.grid.columns - 1))),
    );
    const row = Math.min(
      field.grid.rows - 1,
      Math.max(0, Math.round(sample.v * (field.grid.rows - 1))),
    );
    const index = row * field.grid.columns + column;
    if (field.validity.values[index] === 0 || samplesBySource.has(index))
      continue;
    const terrainSalience = relief.salience[index]!;
    const illumination = relief.hillshade[index]!;
    const sourceSampleId = `${field.field_set_id}:r${row}:c${column}`;
    samplesBySource.set(index, {
      sample_id: `${LANDSCAPE_TERRAIN_FABRIC_REVISION}:${sourceSampleId}`,
      source_sample_id: sourceSampleId,
      source_field_set_id: field.field_set_id,
      source_relief_field_id: relief.relief_field_id,
      source_column: column,
      source_row: row,
      u: column / Math.max(1, field.grid.columns - 1),
      v: row / Math.max(1, field.grid.rows - 1),
      brightness: clamp(illumination * 0.7 + sample.brightness * 0.3, 0.28, 1),
      relief: terrainSalience,
      tangent_u: relief.tangent[index * 2]!,
      tangent_v: relief.tangent[index * 2 + 1]!,
      length: 0.52 + terrainSalience * 1.72,
      reveal_priority:
        terrainSalience * 0.72 +
        sample.brightness * 0.18 +
        stableSequenceNoise(sequence, revision) * 0.1,
    });
  }
  const samples = [...samplesBySource.values()]
    .sort(
      (left, right) =>
        right.reveal_priority - left.reveal_priority ||
        left.v - right.v ||
        left.u - right.u,
    )
    .slice(0, candidateCount);
  return Object.freeze(samples.map((sample) => Object.freeze(sample)));
}

function landscapeReliefScales(
  field: TerrainFieldSetInput,
): LandscapeReliefField["scales"] {
  const metrics = field.relief_metrics;
  if (!metrics)
    return Object.freeze(
      FALLBACK_RELIEF_RADII.map((radius, index) =>
        Object.freeze({
          id: RELIEF_SCALE_TARGETS[index]!.id,
          target_radius_meters: null,
          support_radius_cells: radius,
          support_radius_meters: null,
          weight: RELIEF_SCALE_TARGETS[index]!.weight,
          supported: true,
          ...reliefScaleOperatorIdentity(
            RELIEF_SCALE_TARGETS[index]!.id,
            radius,
          ),
        }),
      ),
    );
  const representativeSpacing = Math.sqrt(
    metrics.sample_spacing_x_meters * metrics.sample_spacing_y_meters,
  );
  return Object.freeze(
    RELIEF_SCALE_TARGETS.map((target) => {
      const requested = target.radius_meters / representativeSpacing;
      const radius = clamp(
        Math.round(requested),
        1,
        LANDSCAPE_RELIEF_MAXIMUM_SUPPORT_RADIUS_CELLS,
      );
      const supportRadiusMeters = radius * representativeSpacing;
      return Object.freeze({
        id: target.id,
        target_radius_meters: target.radius_meters,
        support_radius_cells: radius,
        support_radius_meters: supportRadiusMeters,
        weight: target.weight,
        supported:
          target.radius_meters >= representativeSpacing &&
          requested <= LANDSCAPE_RELIEF_MAXIMUM_SUPPORT_RADIUS_CELLS &&
          radius * 2 + 1 <= Math.min(field.grid.columns, field.grid.rows),
        ...reliefScaleOperatorIdentity(target.id, radius),
      });
    }),
  );
}

function reliefScaleOperatorIdentity(
  id: LandscapeReliefField["scales"][number]["id"],
  radius: number,
) {
  return {
    channel_id: `${LANDSCAPE_METRIC_GRADIENT_REVISION}:${LANDSCAPE_MDOW_REVISION}:${LANDSCAPE_OPENNESS_REVISION}:${id}:r${radius}`,
    gradient_revision: LANDSCAPE_METRIC_GRADIENT_REVISION,
    illumination_revision: LANDSCAPE_MDOW_REVISION,
    openness_revision: LANDSCAPE_OPENNESS_REVISION,
  } as const;
}

function reliefOperatorContract(): LandscapeReliefField["operators"] {
  return Object.freeze({
    metric_gradient: LANDSCAPE_METRIC_GRADIENT_REVISION,
    mdow: LANDSCAPE_MDOW_REVISION,
    openness: LANDSCAPE_OPENNESS_REVISION,
    ridge_salience: LANDSCAPE_RIDGE_SALIENCE_REVISION,
    tone_mapping: LANDSCAPE_TONE_MAPPING_REVISION,
    lighting_owner: "renderer_neutral_relief_field" as const,
  });
}

function elevationNormal(
  elevation: Float32Array,
  columns: number,
  column: number,
  row: number,
  radius: number,
  spacingX: number,
  spacingY: number,
  verticalScale: number,
): readonly [number, number, number] {
  const left = elevation[row * columns + column - radius]!;
  const right = elevation[row * columns + column + radius]!;
  const top = elevation[(row - radius) * columns + column]!;
  const bottom = elevation[(row + radius) * columns + column]!;
  const derivativeX =
    ((right - left) *
      verticalScale *
      CARTOGRAPHIC_RELIEF_VERTICAL_EXAGGERATION) /
    (2 * radius * spacingX);
  const derivativeY =
    ((bottom - top) *
      verticalScale *
      CARTOGRAPHIC_RELIEF_VERTICAL_EXAGGERATION) /
    (2 * radius * spacingY);
  return normalize3(-derivativeX, -derivativeY, 1);
}

function mdowIllumination(
  normal: readonly [number, number, number],
  slope: number,
): number {
  const key = Math.max(0, dot3(normal, KEY_LIGHT));
  const fill = Math.max(0, dot3(normal, FILL_LIGHT));
  const rim = Math.max(0, dot3(normal, BACK_LIGHT));
  const weighted = 0.24 + key * 0.58 + fill * 0.11 + rim * 0.07;
  const adaptive = smoothstep(0.08, 0.9, slope);
  return clamp(1 + (weighted - 0.78) * (0.52 + adaptive * 0.48), 0.34, 1.18);
}

function directionalOpenness(
  elevation: Float32Array,
  columns: number,
  column: number,
  row: number,
  radius: number,
  spacingX: number,
  spacingY: number,
  verticalScale: number,
): { sky_view_factor: number; openness: number } {
  if (radius === 0) return { sky_view_factor: 1, openness: 0 };
  const distances = [
    ...new Set([
      Math.max(1, Math.round(radius * 0.25)),
      Math.max(1, Math.round(radius * 0.5)),
      radius,
    ]),
  ];
  const center = elevation[row * columns + column]!;
  let skyView = 0;
  let signedOpenness = 0;
  for (const [directionX, directionY] of OPENNESS_DIRECTIONS) {
    let positiveHorizon = 0;
    let negativeHorizon = 0;
    for (const distance of distances) {
      const sampleColumn = column + directionX * distance;
      const sampleRow = row + directionY * distance;
      const horizontalDistance = Math.hypot(
        directionX * distance * spacingX,
        directionY * distance * spacingY,
      );
      const difference =
        (elevation[sampleRow * columns + sampleColumn]! - center) *
        verticalScale;
      positiveHorizon = Math.max(
        positiveHorizon,
        Math.atan2(difference, horizontalDistance),
      );
      negativeHorizon = Math.max(
        negativeHorizon,
        Math.atan2(-difference, horizontalDistance),
      );
    }
    skyView += Math.cos(positiveHorizon) ** 2;
    signedOpenness += (negativeHorizon - positiveHorizon) / (Math.PI / 2);
  }
  return {
    sky_view_factor: clamp(skyView / OPENNESS_DIRECTIONS.length, 0, 1),
    openness: clamp(signedOpenness / OPENNESS_DIRECTIONS.length, -1, 1),
  };
}

function metricCurvature(
  elevation: Float32Array,
  columns: number,
  column: number,
  row: number,
  radius: number,
  elevationSpan: number,
): { profile: number; plan: number } {
  if (radius === 0) return { profile: 0, plan: 0 };
  const center = elevation[row * columns + column]!;
  const left = elevation[row * columns + column - radius]!;
  const right = elevation[row * columns + column + radius]!;
  const top = elevation[(row - radius) * columns + column]!;
  const bottom = elevation[(row + radius) * columns + column]!;
  const threshold = Math.max(elevationSpan * 0.006, 0.000_001);
  return {
    profile: clamp(
      (center - (left + right + top + bottom) / 4) / threshold,
      -1,
      1,
    ),
    plan: clamp((left + right - top - bottom) / 2 / threshold, -1, 1),
  };
}

interface PrefixWindow {
  left: number;
  top: number;
  right: number;
  bottom: number;
  area: number;
}

function prefixSum(
  columns: number,
  rows: number,
  value: (index: number) => number,
): Float64Array {
  const stride = columns + 1;
  const result = new Float64Array(stride * (rows + 1));
  for (let row = 0; row < rows; row += 1) {
    let rowSum = 0;
    for (let column = 0; column < columns; column += 1) {
      rowSum += value(row * columns + column);
      result[(row + 1) * stride + column + 1] =
        result[row * stride + column + 1]! + rowSum;
    }
  }
  return result;
}

function completeWindow(
  support: Float64Array,
  columns: number,
  rows: number,
  column: number,
  row: number,
  radius: number,
): PrefixWindow | null {
  const left = column - radius;
  const top = row - radius;
  const right = column + radius + 1;
  const bottom = row + radius + 1;
  if (left < 0 || top < 0 || right > columns || bottom > rows) return null;
  const window = { left, top, right, bottom, area: (radius * 2 + 1) ** 2 };
  return rectangleSum(support, columns, window) === window.area ? window : null;
}

function rectangleSum(
  prefix: Float64Array,
  columns: number,
  window: PrefixWindow,
): number {
  const stride = columns + 1;
  return (
    prefix[window.bottom * stride + window.right]! -
    prefix[window.top * stride + window.right]! -
    prefix[window.bottom * stride + window.left]! +
    prefix[window.top * stride + window.left]!
  );
}

function verifyTerrainFieldShape(field: TerrainFieldSetInput): void {
  const cells = field.grid.columns * field.grid.rows;
  const metrics = field.relief_metrics;
  if (
    field.field_cells !== cells ||
    field.validity.values.length !== cells ||
    field.elevation.values.length !== cells ||
    field.normal.values.length !== cells * 3 ||
    field.curvature.values.length !== cells ||
    (metrics !== undefined &&
      (metrics.schema !== "rey.terrain-relief-metrics.v1" ||
        !Number.isFinite(metrics.sample_spacing_x_meters) ||
        metrics.sample_spacing_x_meters <= 0 ||
        !Number.isFinite(metrics.sample_spacing_y_meters) ||
        metrics.sample_spacing_y_meters <= 0 ||
        !Number.isFinite(metrics.elevation_range_meters) ||
        metrics.elevation_range_meters <= 0 ||
        (metrics.elevation_value_minimum !== undefined &&
          !Number.isFinite(metrics.elevation_value_minimum)) ||
        (metrics.elevation_value_maximum !== undefined &&
          !Number.isFinite(metrics.elevation_value_maximum)) ||
        (metrics.elevation_value_minimum === undefined) !==
          (metrics.elevation_value_maximum === undefined) ||
        (metrics.elevation_value_minimum !== undefined &&
          metrics.elevation_value_maximum! < metrics.elevation_value_minimum) ||
        !metrics.authority))
  )
    throw new Error("landscape relief input shape is invalid");
}

function verifySampleIndices(
  indices: readonly number[],
  bound: number,
  axis: string,
): void {
  if (
    indices.length === 0 ||
    indices.some(
      (index, sequence) =>
        !Number.isInteger(index) ||
        index < 0 ||
        index >= bound ||
        (sequence > 0 && index <= indices[sequence - 1]!),
    )
  )
    throw new Error(`landscape relief ${axis} samples are invalid`);
}

function sampleReliefComponents(
  values: Float32Array,
  sourceColumns: number,
  columnIndices: readonly number[],
  rowIndices: readonly number[],
  components: number,
): Float32Array {
  const result = new Float32Array(
    columnIndices.length * rowIndices.length * components,
  );
  let output = 0;
  for (const row of rowIndices) {
    for (const column of columnIndices) {
      const sourceOffset = (row * sourceColumns + column) * components;
      for (let component = 0; component < components; component += 1)
        result[output++] = values[sourceOffset + component]!;
    }
  }
  return result;
}

function minimumSupported(values: Float32Array, validity: Uint8Array): number {
  let result = Number.POSITIVE_INFINITY;
  for (let index = 0; index < values.length; index += 1)
    if (validity[index] !== 0) result = Math.min(result, values[index]!);
  return Number.isFinite(result) ? result : 0;
}

function maximumSupported(values: Float32Array, validity: Uint8Array): number {
  let result = Number.NEGATIVE_INFINITY;
  for (let index = 0; index < values.length; index += 1)
    if (validity[index] !== 0) result = Math.max(result, values[index]!);
  return Number.isFinite(result) ? result : 0;
}

function normalize3(
  x: number,
  y: number,
  z: number,
): readonly [number, number, number] {
  const length = Math.hypot(x, y, z);
  return length > 0 ? [x / length, y / length, z / length] : [0, 0, 1];
}

function cartographicLight(
  azimuthDegrees: number,
  altitudeDegrees: number,
): readonly [number, number, number] {
  const azimuth = (azimuthDegrees * Math.PI) / 180;
  const altitude = (altitudeDegrees * Math.PI) / 180;
  const horizontal = Math.cos(altitude);
  return normalize3(
    Math.sin(azimuth) * horizontal,
    -Math.cos(azimuth) * horizontal,
    Math.sin(altitude),
  );
}

function dot3(
  left: readonly [number, number, number],
  right: readonly [number, number, number],
): number {
  return left[0] * right[0] + left[1] * right[1] + left[2] * right[2];
}

function stableSequenceNoise(sequence: number, revision: string): number {
  let value = sequence + 1;
  for (let index = 0; index < revision.length; index += 1)
    value = Math.imul(value ^ revision.charCodeAt(index), 16_777_619);
  value = Math.imul(value ^ (value >>> 16), 0x7feb352d);
  value = Math.imul(value ^ (value >>> 15), 0x846ca68b);
  return ((value ^ (value >>> 16)) >>> 0) / 4_294_967_295;
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(maximum, value));
}

function smoothstep(minimum: number, maximum: number, value: number): number {
  const progress = clamp((value - minimum) / (maximum - minimum), 0, 1);
  return progress * progress * (3 - 2 * progress);
}

function boundsOverlap(
  left: TerrainFieldSetInput["grid"]["bounds"],
  right: TerrainFieldSetInput["grid"]["bounds"],
): boolean {
  return (
    left.x < right.x + right.width &&
    left.x + left.width > right.x &&
    left.y < right.y + right.height &&
    left.y + left.height > right.y
  );
}
