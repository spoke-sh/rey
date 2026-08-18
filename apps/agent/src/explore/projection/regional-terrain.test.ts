import { terrainTriangleIndices } from "@rey/explorer";
import { describe, expect, it } from "vitest";
import type { AdmittedRegionalScene } from "../../domain";
import {
  REGIONAL_TERRAIN_SCENE_COMPILER_REVISION,
  compileRegionalTerrainField,
  invertRegionalTerrainPosition,
  projectRegionalTerrainPosition,
  regionalTerrainElevationSummary,
} from "./regional-terrain";
import {
  REGIONAL_TERRAIN_REFINEMENT_REVISION,
  refineRegionalTerrainField,
  regionalTerrainRefinementFactor,
} from "../terrain/refinement";
import {
  deriveRegionalTerrainContours,
  TERRAIN_CONTOUR_COMPILER_REVISION,
} from "../terrain/contours";

function regionalTerrainScene(withHole = true): AdmittedRegionalScene {
  const cells = Array.from({ length: 9 }, (_, index) => {
    const column = index % 3;
    const row = Math.floor(index / 3);
    const valid = !withHole || index !== 4;
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

  it("maps native north-west and south-east into a padded, zoomed-out landscape frame", () => {
    const bounds = regionalTerrainScene().native_bounds;
    // The frame pads the admitted region's own bounds (REGIONAL_TERRAIN_FRAME_PADDING_RATIO)
    // so the data sits inset with a halo of context, rather than filling the frame
    // edge-to-edge — hence these corners land inside {x:96,y:72,w:1008,h:576}, not on it.
    expect(
      projectRegionalTerrainPosition(
        bounds,
        [bounds.west_microdegrees, bounds.north_microdegrees],
        { width: 1200, height: 720 },
      ),
    ).toEqual({ x: 285, y: 180 });
    expect(
      projectRegionalTerrainPosition(
        bounds,
        [bounds.east_microdegrees, bounds.south_microdegrees],
        { width: 1200, height: 720 },
      ),
    ).toEqual({ x: 915, y: 540 });
    // The center of a symmetric pad is unchanged, so the inverse at the frame's
    // own center still recovers the bounds' original center coordinate.
    expect(
      invertRegionalTerrainPosition(
        bounds,
        { x: 600, y: 360 },
        {
          width: 1200,
          height: 720,
        },
      ),
    ).toEqual([-122_500_000, 37_500_000]);
  });

  it("builds a deterministic high-resolution field without expanding admitted support", () => {
    const source = compileRegionalTerrainField(regionalTerrainScene(false), {
      width: 1200,
      height: 720,
    })!;
    const refined = refineRegionalTerrainField(source, 4);
    const replay = refineRegionalTerrainField(source, 4);

    expect(refined.grid).toMatchObject({ columns: 9, rows: 9 });
    expect(refined.field_cells).toBe(81);
    expect(refined.field_set_id).toContain(
      REGIONAL_TERRAIN_REFINEMENT_REVISION,
    );
    expect(refined.active_band_ids).toContain("presentation_microrelief");
    expect(refined.detail_authority).toContain(
      "not observed or authored elevation",
    );
    expect(refined.elevation.values).toEqual(replay.elevation.values);
    expect(refined.validity.values).toEqual(replay.validity.values);

    for (let sourceRow = 0; sourceRow < source.grid.rows; sourceRow += 1) {
      for (
        let sourceColumn = 0;
        sourceColumn < source.grid.columns;
        sourceColumn += 1
      ) {
        const sourceIndex = sourceRow * source.grid.columns + sourceColumn;
        const refinedIndex =
          sourceRow * 4 * refined.grid.columns + sourceColumn * 4;
        expect(refined.validity.values[refinedIndex]).toBe(
          source.validity.values[sourceIndex],
        );
        expect(refined.elevation.values[refinedIndex]).toBeCloseTo(
          source.elevation.values[sourceIndex]!,
          6,
        );
      }
    }
    expect(refined.elevation.values[1]).not.toBeCloseTo(
      (source.elevation.values[0]! * 3 + source.elevation.values[1]!) / 4,
      6,
    );

    const saddleValues = source.elevation.values.slice();
    saddleValues[0] = 0;
    saddleValues[1] = 1;
    saddleValues[source.grid.columns] = 1;
    saddleValues[source.grid.columns + 1] = 0;
    const saddle = refineRegionalTerrainField(
      {
        ...source,
        elevation: { ...source.elevation, values: saddleValues },
      },
      2,
    );
    expect(saddle.elevation.values[saddle.grid.columns + 1]).toBeCloseTo(
      0.5,
      1,
    );

    const withHole = refineRegionalTerrainField(
      compileRegionalTerrainField(regionalTerrainScene(), {
        width: 1200,
        height: 720,
      })!,
      4,
    );
    const center = 4 * withHole.grid.columns + 4;
    expect(withHole.validity.values[center]).toBe(0);
    for (const index of terrainTriangleIndices(withHole))
      expect(withHole.validity.values[index]).toBe(1);
  });

  it("selects refinement from source density instead of a fixed multiplier", () => {
    const source = compileRegionalTerrainField(regionalTerrainScene(false), {
      width: 1200,
      height: 720,
    })!;
    const representative = {
      ...source,
      grid: { ...source.grid, columns: 81, rows: 81 },
    };
    expect(regionalTerrainRefinementFactor(representative)).toBe(4);
    expect(
      regionalTerrainRefinementFactor({
        ...source,
        grid: { ...source.grid, columns: 501, rows: 501 },
      }),
    ).toBe(1);
  });

  it("summarizes high-density elevation without variadic call limits", () => {
    const cells = 251_001;
    const valid = 180_279;
    const validity = new Uint8Array(cells);
    validity.fill(1, 0, valid);
    const elevations = new Array<number>(cells).fill(0);
    elevations.fill(32_000_000, 0, valid);
    elevations[valid - 1] = 1_784_120_000;
    expect(regionalTerrainElevationSummary(validity, elevations)).toEqual({
      valid_count: valid,
      minimum: 32,
      maximum: 1_784.12,
    });
  });

  it("derives scale-aware contours without crossing no-data cells", () => {
    const supported = compileRegionalTerrainField(regionalTerrainScene(false), {
      width: 1200,
      height: 720,
    })!;
    const landscape = deriveRegionalTerrainContours(supported, "landscape");
    const objects = deriveRegionalTerrainContours(supported, "objects");
    expect(landscape.length).toBeGreaterThan(0);
    expect(objects.length).toBeGreaterThan(landscape.length);
    expect(landscape.every(({ path }) => path.length > 0)).toBe(true);
    expect(landscape[0]?.id).toContain(TERRAIN_CONTOUR_COMPILER_REVISION);

    const explicitHole = compileRegionalTerrainField(regionalTerrainScene(), {
      width: 1200,
      height: 720,
    })!;
    expect(deriveRegionalTerrainContours(explicitHole, "landscape")).toEqual(
      [],
    );
  });
});
