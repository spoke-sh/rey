import {
  finalizeLandscapeHeightPyramid,
  finalizeLandscapePyramidEnvelope,
  finalizeLandscapeReliefPyramid,
  landscapePyramidContentId,
  landscapeReliefFieldByteLength,
  summarizeTerrainFieldValidity,
  verifyLandscapePyramidEnvelope,
  type LandscapePyramidEnvelope,
  type LandscapePyramidLineage,
  type LandscapeReliefField,
} from "@rey/explorer";
import type { TerrainFieldSet } from "./compile";

export const CURRENT_LANDSCAPE_HEIGHT_ENVELOPE_REVISION =
  "rey.terrain.current-height-envelope@1" as const;
export const CURRENT_LANDSCAPE_RELIEF_ENVELOPE_REVISION =
  "rey.terrain.current-relief-envelope@1" as const;

const HEIGHT_OMISSIONS = Object.freeze([
  "only the complete finest field is retained; coarser height levels are not materialized",
  "height source gutters and adjacent-tile border digests are not retained",
]);
const RELIEF_OMISSIONS = Object.freeze([
  "relief is a complete-field prototype rather than a haloed multilevel hierarchy",
  "relief source gutters and adjacent-tile border digests are not retained",
  "slope-adaptive MDOW and sky-view/openness channels are not materialized",
]);

export function compileCurrentLandscapePyramidEnvelope(
  field: TerrainFieldSet,
  relief: LandscapeReliefField,
): LandscapePyramidEnvelope {
  const metrics = field.relief_metrics;
  const reference = field.landscape_reference;
  const summary = field.source_summary;
  if (!metrics || !reference || !summary)
    throw new Error(
      "admitted landscape field is missing metric pyramid source metadata",
    );
  const validity = Object.freeze({
    ...summarizeTerrainFieldValidity(field),
    policy: "conservative_support_only" as const,
  });
  const mosaicId = field.landscape_mosaic?.mosaic_id ?? reference.reference_id;
  const lineage = landscapePyramidLineage(field, reference.reference_id);
  const height = finalizeLandscapeHeightPyramid({
    implementation_revision: CURRENT_LANDSCAPE_HEIGHT_ENVELOPE_REVISION,
    mosaic_id: mosaicId,
    coordinate_reference: reference.coordinate_reference,
    vertical_reference: reference.vertical_reference,
    complete: false,
    omissions: HEIGHT_OMISSIONS,
    levels: [
      {
        level: 0,
        height_id: landscapePyramidContentId("height", [
          field.elevation.values,
        ]),
        implementation_revision: field.elevation.implementation_revision,
        sample_spacing_x_meters: metrics.sample_spacing_x_meters,
        sample_spacing_y_meters: metrics.sample_spacing_y_meters,
        columns: field.grid.columns,
        rows: field.grid.rows,
        bounds: field.grid.bounds,
        validity,
        elevation_minimum_meters: summary.elevation_minimum,
        elevation_maximum_meters: summary.elevation_maximum,
        height_bytes: field.elevation.values.byteLength,
        validity_bytes: field.validity_classification!.values.byteLength,
        source_lineage: lineage,
      },
    ],
  });
  const reliefPyramid = finalizeLandscapeReliefPyramid(
    {
      implementation_revision: CURRENT_LANDSCAPE_RELIEF_ENVELOPE_REVISION,
      mosaic_id: mosaicId,
      source_height_pyramid_id: height.pyramid_id,
      coordinate_reference: reference.coordinate_reference,
      vertical_reference: reference.vertical_reference,
      complete: false,
      omissions: [
        ...RELIEF_OMISSIONS,
        ...relief.scales
          .filter(({ supported }) => !supported)
          .map(
            ({ id, target_radius_meters }) =>
              `${id} relief scale at ${target_radius_meters ?? "unbound"} meters exceeds admitted source support`,
          ),
      ],
      levels: [
        {
          level: 0,
          implementation_revision: relief.implementation_revision,
          source_height_level_id: height.levels[0]!.level_id,
          sample_spacing_x_meters: metrics.sample_spacing_x_meters,
          sample_spacing_y_meters: metrics.sample_spacing_y_meters,
          columns: field.grid.columns,
          rows: field.grid.rows,
          bounds: field.grid.bounds,
          validity,
          channel_ids: [
            `hillshade:${landscapePyramidContentId("hillshade", [relief.hillshade])}`,
            `salience:${landscapePyramidContentId("salience", [relief.salience])}`,
            `tangent:${landscapePyramidContentId("tangent", [relief.tangent])}`,
          ],
          operator_support: relief.scales.map((scale) => ({
            operator_id: `multiscale-hillshade:${scale.id}`,
            implementation_revision: relief.implementation_revision,
            target_radius_meters: scale.target_radius_meters!,
            support_radius_cells: scale.support_radius_cells,
            support_radius_meters: scale.support_radius_meters!,
            gutter_radius_cells: 0,
            supported: false,
            validity_policy: "complete_valid_window" as const,
          })),
          relief_bytes: landscapeReliefFieldByteLength(relief),
          source_lineage: [
            ...lineage,
            {
              kind: "relief-field",
              identity: relief.relief_field_id,
              revision: relief.implementation_revision,
            },
          ],
        },
      ],
    },
    height,
  );
  const envelope = finalizeLandscapePyramidEnvelope(
    field.field_set_id,
    field.source_revision,
    height,
    reliefPyramid,
  );
  verifyLandscapePyramidEnvelope(envelope, field, relief);
  return envelope;
}

function landscapePyramidLineage(
  field: TerrainFieldSet,
  referenceId: string,
): readonly LandscapePyramidLineage[] {
  return Object.freeze([
    {
      kind: "field",
      identity: field.field_set_id,
      revision: field.source_revision,
    },
    {
      kind: "spatial-reference",
      identity: referenceId,
      revision:
        field.landscape_mosaic?.composition_revision ?? field.source_revision,
    },
    ...(field.landscape_mosaic?.patch_ids.map((patchId) => ({
      kind: "source-patch",
      identity: patchId,
      revision: field.landscape_mosaic!.composition_revision,
    })) ?? []),
  ]);
}
