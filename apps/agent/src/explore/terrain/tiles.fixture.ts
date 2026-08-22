import {
  compileTerrainProgram,
  materializeTerrainWorkingSet,
  type TerrainFieldSet,
} from "./compile";
import { proceduralProjection } from "./compile.test-fixture";

export function admittedField(): TerrainFieldSet {
  const program = compileTerrainProgram({
    source_id: "terrain:regional",
    source_revision: "topography:one",
    bounds: { x: 100, y: 80, width: 1300, height: 840 },
    anchors: [
      { id: "west", x: 480, y: 420, prominence: 4 },
      { id: "east", x: 1020, y: 560, prominence: 3 },
    ],
    atmosphere: [],
    unresolved_pressure: 0,
    projection: proceduralProjection,
  });
  const field = materializeTerrainWorkingSet(program, {
    working_set_id: "admitted:grid",
    bounds: program.bounds,
    columns: 129,
    rows: 65,
    detail_authority: "admitted fixture grid",
  });
  field.validity.values.fill(1);
  const validityClassification = createTerrainValidityClassification(
    new Uint8Array(field.field_cells).fill(TERRAIN_VALIDITY_VALID),
    "fixture:admitted-validity-classification@1",
  );
  return Object.freeze({
    ...field,
    field_set_id: "admitted:grid:129x65",
    working_set_id: "admitted:grid:129x65",
    active_band_ids: Object.freeze(["admitted_dem"]),
    source_summary: Object.freeze({
      columns: field.grid.columns,
      rows: field.grid.rows,
      valid_vertices: field.field_cells,
      no_data_vertices: 0,
      unsupported_vertices: 0,
      elevation_minimum: 0,
      elevation_maximum: 1_000,
    }),
    validity_classification: validityClassification,
    landscape_reference: Object.freeze({
      schema: "rey.landscape-spatial-reference.v1" as const,
      reference_id: "fixture:regional-landscape",
      coordinate_reference: "fixture projected metric frame",
      vertical_reference: "fixture elevation meters",
    }),
    relief_metrics: Object.freeze({
      schema: "rey.terrain-relief-metrics.v1" as const,
      sample_spacing_x_meters: 150,
      sample_spacing_y_meters: 150,
      elevation_range_meters: 1_000,
      authority: "fixture metric terrain grid",
    }),
    field_bytes: field.field_bytes + validityClassification.values.byteLength,
  });
}

export function terrainTileView(renderedScale: number) {
  return {
    world_width: 1500,
    world_height: 1000,
    viewport_width: 1500,
    viewport_height: 1000,
    rendered_scale: renderedScale,
    pan_x: 0,
    pan_y: 0,
  };
}
import {
  createTerrainValidityClassification,
  TERRAIN_VALIDITY_VALID,
} from "@rey/explorer";
