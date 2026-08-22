import {
  TERRAIN_VALIDITY_NO_DATA,
  TERRAIN_VALIDITY_UNSUPPORTED,
  TERRAIN_VALIDITY_VALID,
} from "@rey/explorer";
import { describe, expect, it } from "vitest";
import { compileLandscapeHeightHierarchy } from "./height-pyramid";
import { admittedField } from "./tiles.fixture";

describe("landscape height hierarchy", () => {
  it("downsamples validity conservatively without gaining support", () => {
    const field = admittedField();
    const noDataIndex = 32 * field.grid.columns + 64;
    const unsupportedIndex = 10 * field.grid.columns + 10;
    field.validity.values[noDataIndex] = 0;
    field.validity_classification!.values[noDataIndex] =
      TERRAIN_VALIDITY_NO_DATA;
    field.validity.values[unsupportedIndex] = 0;
    field.validity_classification!.values[unsupportedIndex] =
      TERRAIN_VALIDITY_UNSUPPORTED;

    const hierarchy = compileLandscapeHeightHierarchy(field);
    const fine = hierarchy.levels.at(-1)!;
    const parent = hierarchy.levels.at(-2)!;

    expect(hierarchy).toMatchObject({
      complete: true,
      validity_policy: "complete_child_window",
      source_set_encoding: "rey.landscape-source-sets.csr.v1",
      source_patch_ids: [field.field_set_id],
    });
    expect(hierarchy.levels.length).toBeGreaterThan(1);
    expect(fine).toMatchObject({
      columns: field.grid.columns,
      rows: field.grid.rows,
      height_id: expect.stringMatching(/^blake3:/),
      validity_id: expect.stringMatching(/^blake3:/),
      source_contribution_id: expect.stringMatching(/^blake3:/),
    });
    expect(parent.validity_classification[16 * parent.columns + 32]).toBe(
      TERRAIN_VALIDITY_NO_DATA,
    );
    expect(parent.validity_classification[5 * parent.columns + 5]).toBe(
      TERRAIN_VALIDITY_UNSUPPORTED,
    );
    expect(parent.valid_vertices).toBeLessThan(parent.columns * parent.rows);
    expectValidParentsHaveCompleteChildSupport(parent, fine);
  });

  it("retains every contributing patch in a canonical per-sample source set", () => {
    const source = admittedField();
    const cells = source.field_cells;
    const unsupported = 0xffff_ffff;
    const primary = new Uint32Array(cells);
    const secondary = new Uint32Array(cells).fill(unsupported);
    const row = Math.floor(source.grid.rows / 2);
    const column = Math.floor(source.grid.columns / 2);
    const sharedIndex = row * source.grid.columns + column;
    for (let index = 0; index < cells; index += 1)
      primary[index] = index % source.grid.columns < column ? 0 : 1;
    secondary[sharedIndex] = 0;
    const field = {
      ...source,
      landscape_height_sources: Object.freeze({
        schema: "rey.landscape-height-sources.v1" as const,
        patch_ids: Object.freeze(["patch:left", "patch:right"]),
        unsupported_index: unsupported,
        primary_owner_indices: primary,
        secondary_owner_indices: secondary,
      }),
    };

    const first = compileLandscapeHeightHierarchy(field);
    const replay = compileLandscapeHeightHierarchy(field);
    const fine = first.levels.at(-1)!;
    const sharedSources = levelSources(fine, sharedIndex);
    const parent = first.levels.at(-2)!;
    const parentSources = levelSources(
      parent,
      Math.floor(row / 2) * parent.columns + Math.floor(column / 2),
    );

    expect(sharedSources).toEqual([0, 1]);
    expect(parentSources).toEqual([0, 1]);
    expect(first.hierarchy_id).toBe(replay.hierarchy_id);
    expect(first.levels.map(({ level_id }) => level_id)).toEqual(
      replay.levels.map(({ level_id }) => level_id),
    );
  });
});

function expectValidParentsHaveCompleteChildSupport(
  parent: ReturnType<typeof compileLandscapeHeightHierarchy>["levels"][number],
  child: ReturnType<typeof compileLandscapeHeightHierarchy>["levels"][number],
): void {
  for (let row = 0; row < parent.rows; row += 1) {
    for (let column = 0; column < parent.columns; column += 1) {
      const parentIndex = row * parent.columns + column;
      if (
        parent.validity_classification[parentIndex] !== TERRAIN_VALIDITY_VALID
      )
        continue;
      for (
        let childRow = Math.max(0, row * 2 - 1);
        childRow <= Math.min(child.rows - 1, row * 2 + 1);
        childRow += 1
      )
        for (
          let childColumn = Math.max(0, column * 2 - 1);
          childColumn <= Math.min(child.columns - 1, column * 2 + 1);
          childColumn += 1
        )
          expect(
            child.validity_classification[
              childRow * child.columns + childColumn
            ],
          ).toBe(TERRAIN_VALIDITY_VALID);
    }
  }
}

function levelSources(
  level: ReturnType<typeof compileLandscapeHeightHierarchy>["levels"][number],
  index: number,
): number[] {
  return [
    ...level.source_indices.slice(
      level.source_offsets[index],
      level.source_offsets[index + 1],
    ),
  ];
}
