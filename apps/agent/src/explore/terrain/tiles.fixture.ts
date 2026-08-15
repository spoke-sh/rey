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
  return Object.freeze({
    ...field,
    field_set_id: "admitted:grid:129x65",
    working_set_id: "admitted:grid:129x65",
    active_band_ids: Object.freeze(["admitted_dem"]),
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
