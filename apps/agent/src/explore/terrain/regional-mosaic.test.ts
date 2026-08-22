import { compileLandscapePatchSet } from "@rey/explorer";
import { describe, expect, it } from "vitest";
import { createFieldGrid } from "../engine/fields";
import type { TerrainFieldSet } from "./compile";
import {
  compileRegionalTerrainMosaic,
  REGIONAL_TERRAIN_MOSAIC_REVISION,
} from "./regional-mosaic";
import { admittedField } from "./tiles.fixture";

describe("regional terrain mosaic", () => {
  it("compiles qualified adjacent patches into one validity-safe field", () => {
    const left = regionalPatch("field:left", 100);
    const spacing = left.grid.bounds.width / (left.grid.columns - 1);
    const right = regionalPatch(
      "field:right",
      left.grid.bounds.x + left.grid.bounds.width,
    );
    copySharedColumn(left, left.grid.columns - 1, right, 0);

    const compiled = compileRegionalTerrainMosaic(
      [
        {
          member_id: "member:left",
          scene_id: "scene:left",
          role: "detail",
          field: left,
        },
        {
          member_id: "member:right",
          scene_id: "scene:right",
          role: "detail",
          field: right,
        },
      ],
      left.field_set_id,
      "composition:adjacent",
      "native_crs84/shared-landscape-frame",
      "qualified-edge-elevation-meters:composition:adjacent",
    );

    expect(spacing).toBeGreaterThan(0);
    expect(compiled.manifest).toMatchObject({
      schema: "rey.landscape-mosaic.v1",
      implementation_revision: REGIONAL_TERRAIN_MOSAIC_REVISION,
      primary_patch_id: left.field_set_id,
      patch_ids: [left.field_set_id, right.field_set_id],
      columns: left.grid.columns + right.grid.columns - 1,
      rows: left.grid.rows,
      shared_vertices: left.grid.rows,
      unsupported_vertices: 0,
      overlap_policy: "qualified_shared_samples_must_match_before_derivation",
      gap_policy: "unsupported_remains_transparent",
    });
    expect(compiled.field.validity.values.every((value) => value === 1)).toBe(
      true,
    );
    expect(compiled.field.landscape_mosaic?.mosaic_id).toBe(
      compiled.manifest.mosaic_id,
    );
    expect(compileLandscapePatchSet([compiled.field])).toMatchObject({
      patch_ids: [left.field_set_id, right.field_set_id],
      overlap_policy: "qualified_shared_samples_must_match_before_derivation",
      gap_policy: "unsupported_remains_transparent",
    });
  });

  it("retains unsupported space between disjoint patches as invalid", () => {
    const left = regionalPatch("field:left", 100);
    const spacing = left.grid.bounds.width / (left.grid.columns - 1);
    const right = regionalPatch(
      "field:right",
      left.grid.bounds.x + left.grid.bounds.width + spacing * 4,
    );
    const compiled = compileRegionalTerrainMosaic(
      [
        {
          member_id: "member:left",
          scene_id: "scene:left",
          role: "detail",
          field: left,
        },
        {
          member_id: "member:right",
          scene_id: "scene:right",
          role: "detail",
          field: right,
        },
      ],
      left.field_set_id,
      "composition:gap",
      "native_crs84/shared-landscape-frame",
      "qualified-edge-elevation-meters:composition:gap",
    );

    expect(compiled.manifest.unsupported_vertices).toBe(3 * left.grid.rows);
    expect(compiled.manifest.omissions).toEqual([
      `${3 * left.grid.rows} mosaic vertices have no admitted terrain source and remain unsupported`,
    ]);
    const gapStart = left.grid.columns;
    for (let row = 0; row < compiled.field.grid.rows; row += 1)
      for (let column = gapStart; column < gapStart + 3; column += 1)
        expect(
          compiled.field.validity.values[
            row * compiled.field.grid.columns + column
          ],
        ).toBe(0);
  });

  it("rejects shared-sample conflicts, area overlap, and incompatible grids", () => {
    const left = regionalPatch("field:left", 100);
    const right = regionalPatch(
      "field:right",
      left.grid.bounds.x + left.grid.bounds.width,
    );
    copySharedColumn(left, left.grid.columns - 1, right, 0);
    right.elevation.values[0] = Math.fround(right.elevation.values[0]! + 0.1);
    expect(() => compilePair(left, right, "conflict")).toThrow(
      "shared elevation conflicts",
    );

    const overlapping = regionalPatch(
      "field:overlap",
      left.grid.bounds.x + left.grid.bounds.width / 2,
    );
    expect(() => compilePair(left, overlapping, "overlap")).toThrow(
      "does not admit overlapping patch areas",
    );

    const incompatible = regionalPatch(
      "field:incompatible",
      left.grid.bounds.x + left.grid.bounds.width,
      left.grid.bounds.width * 0.75,
    );
    expect(() => compilePair(left, incompatible, "incompatible")).toThrow(
      "patch scale is incompatible",
    );
  });
});

function compilePair(
  left: TerrainFieldSet,
  right: TerrainFieldSet,
  revision: string,
) {
  return compileRegionalTerrainMosaic(
    [
      {
        member_id: "member:left",
        scene_id: "scene:left",
        role: "detail",
        field: left,
      },
      {
        member_id: "member:right",
        scene_id: "scene:right",
        role: "detail",
        field: right,
      },
    ],
    left.field_set_id,
    `composition:${revision}`,
    "native_crs84/shared-landscape-frame",
    `qualified-edge-elevation-meters:composition:${revision}`,
  );
}

function regionalPatch(
  fieldSetId: string,
  x: number,
  width = 1300,
): TerrainFieldSet {
  const source = admittedField();
  const grid = createFieldGrid(source.grid.columns, source.grid.rows, {
    x,
    y: source.grid.bounds.y,
    width,
    height: source.grid.bounds.height,
  });
  return {
    ...source,
    field_set_id: fieldSetId,
    source_revision: `source:${fieldSetId}`,
    grid,
    validity: { ...source.validity, grid },
    elevation: { ...source.elevation, grid },
    rainfall: { ...source.rainfall, grid },
    flow_direction: { ...source.flow_direction, grid },
    flow_accumulation: { ...source.flow_accumulation, grid },
    erosion: { ...source.erosion, grid },
    normal: { ...source.normal, grid },
    curvature: { ...source.curvature, grid },
    material: { ...source.material, grid },
  };
}

function copySharedColumn(
  source: TerrainFieldSet,
  sourceColumn: number,
  target: TerrainFieldSet,
  targetColumn: number,
): void {
  for (let row = 0; row < source.grid.rows; row += 1) {
    const sourceIndex = row * source.grid.columns + sourceColumn;
    const targetIndex = row * target.grid.columns + targetColumn;
    target.validity.values[targetIndex] = source.validity.values[sourceIndex]!;
    target.elevation.values[targetIndex] =
      source.elevation.values[sourceIndex]!;
    target.material.occlusion[targetIndex] =
      source.material.occlusion[sourceIndex]!;
    target.material.roughness[targetIndex] =
      source.material.roughness[sourceIndex]!;
    for (let component = 0; component < 3; component += 1)
      target.material.tint[targetIndex * 3 + component] =
        source.material.tint[sourceIndex * 3 + component]!;
  }
}
