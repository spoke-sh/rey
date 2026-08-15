import { terrainTriangleIndices } from "@rey/explorer";
import { describe, expect, it } from "vitest";
import type { AdmittedRegionalScene } from "../../domain";
import {
  REGIONAL_TERRAIN_SCENE_COMPILER_REVISION,
  compileRegionalTerrainField,
  projectRegionalTerrainPosition,
} from "./regional-terrain";

function regionalTerrainScene(): AdmittedRegionalScene {
  const cells = Array.from({ length: 9 }, (_, index) => {
    const column = index % 3;
    const row = Math.floor(index / 3);
    const valid = index !== 4;
    return {
      cell_id: `cell:${index}`,
      source_object_id: `terrain/cell-${row}-${column}`,
      source_artifact_id: "artifact:terrain",
      source_object_revision: `object:${index}`,
      grid_position: [column, row] as [number, number],
      native_position: [
        -123_000_000 + column * 500_000,
        38_000_000 - row * 500_000,
      ] as [number, number],
      elevation_micrometers: valid ? (100 + index * 17) * 1_000_000 : null,
      material: valid ? "granite" : null,
      validity: valid ? ("valid" as const) : ("no_data" as const),
      authority: valid
        ? "exact admitted Point altitude and material at one valid grid vertex"
        : "explicit source no-data vertex; geometry locates the hole but supplies no height or material",
    };
  });
  return {
    native_bounds: {
      west_microdegrees: -123_000_000,
      south_microdegrees: 37_000_000,
      east_microdegrees: -122_000_000,
      north_microdegrees: 38_000_000,
      crosses_antimeridian: false,
    },
    projection: {
      terrain: {
        schema: "rey.regional-terrain-program.v2",
        program_id: "program:terrain",
        evaluator: {
          id: "rey.regional-terrain.rectilinear-grid",
          revision: 1,
          semantic_digest: "evaluator:terrain",
        },
        samples: [],
        grid: {
          schema: "rey.regional-terrain-grid.v1",
          dataset_id: "dataset:terrain",
          source_dataset_id: "terrain-dem",
          columns: 3,
          rows: 3,
          native_bounds: {
            west_microdegrees: -123_000_000,
            south_microdegrees: 37_000_000,
            east_microdegrees: -122_000_000,
            north_microdegrees: 38_000_000,
            crosses_antimeridian: false,
          },
          cells,
          validity_semantics:
            "row-major source vertices are explicitly valid or no_data; no_data cuts triangle support",
          interpolation:
            "piecewise linear only within triangles whose three admitted source vertices are valid",
          authority:
            "qualified rectilinear height/material grid; validity ends at supported source triangles",
        },
        height_unit: "micrometer",
        interpolation:
          "piecewise linear only within triangles whose three admitted source vertices are valid",
        material_semantics:
          "source-declared bounded material identifier; no inferred physical properties",
        authority:
          "qualified rectilinear height/material grid; validity ends at supported source triangles",
      },
    },
  } as unknown as AdmittedRegionalScene;
}

describe("regional terrain projection", () => {
  it("compiles admitted heights and explicit no-data into one renderer-neutral field", () => {
    const field = compileRegionalTerrainField(regionalTerrainScene(), {
      width: 1200,
      height: 720,
    });
    expect(field).not.toBeNull();
    expect(field).toMatchObject({
      active_band_ids: ["admitted_dem"],
      field_cells: 9,
      source_revision: "dataset:terrain",
    });
    expect(field!.validity.values).toEqual(
      Uint8Array.from([1, 1, 1, 1, 0, 1, 1, 1, 1]),
    );
    expect(field!.elevation.values[4]).toBe(0);
    const indices = terrainTriangleIndices(field!);
    expect(indices.length).toBeGreaterThan(0);
    expect([...indices]).not.toContain(4);
    expect(field!.field_bytes).toBeGreaterThan(0);
    expect(field!.field_set_id).toContain(
      REGIONAL_TERRAIN_SCENE_COMPILER_REVISION,
    );
  });

  it("maps native north-west and south-east to the bounded landscape frame", () => {
    const bounds = regionalTerrainScene().native_bounds;
    expect(
      projectRegionalTerrainPosition(
        bounds,
        [bounds.west_microdegrees, bounds.north_microdegrees],
        { width: 1200, height: 720 },
      ),
    ).toEqual({ x: 96, y: 72 });
    expect(
      projectRegionalTerrainPosition(
        bounds,
        [bounds.east_microdegrees, bounds.south_microdegrees],
        { width: 1200, height: 720 },
      ),
    ).toEqual({ x: 1104, y: 648 });
  });
});
