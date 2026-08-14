import { describe, expect, it } from "vitest";
import {
  TERRAIN_PATCH_COMPILER_REVISION,
  TERRAIN_PATCH_HALO_SAMPLES,
  compileTerrainProgram,
  materializeTerrainWorkingSet,
  terrainPatchRequestsForView,
  terrainWorkingSetForView,
} from "./compile";
import { proceduralProjection } from "./compile.test-fixture";
import { PROJECTED_SUPPORT } from "./elevation";
import { TerrainPatchCache } from "./patch-cache";

function program() {
  return compileTerrainProgram({
    source_id: "survey:one",
    source_revision: "topography:one",
    bounds: { x: 100, y: 80, width: 1300, height: 840 },
    anchors: [
      { id: "workspace", x: 750, y: 500, prominence: 4 },
      { id: "document", x: 1010, y: 420, prominence: 2 },
    ],
    atmosphere: [{ x: 1300, y: 240 }],
    unresolved_pressure: 0.4,
    projection: proceduralProjection,
  });
}

describe("procedural terrain compiler", () => {
  it("materializes deterministic bounded working sets without stored terrain levels", () => {
    const firstProgram = program();
    const first = materializeTerrainWorkingSet(firstProgram, {
      working_set_id: "camera:one",
      bounds: firstProgram.bounds,
      columns: 121,
      rows: 81,
      detail_authority: "camera fixture",
    });
    const second = materializeTerrainWorkingSet(program(), {
      working_set_id: "camera:one",
      bounds: firstProgram.bounds,
      columns: 121,
      rows: 81,
      detail_authority: "camera fixture",
    });

    expect(first.active_band_ids).toEqual(["macro", "meso"]);
    expect(first.field_cells).toBe(9801);
    expect(first.field_bytes).toBe(539055);
    expect(Array.from(first.elevation.values)).toEqual(
      Array.from(second.elevation.values),
    );
    expect(Array.from(first.validity.values)).toContain(PROJECTED_SUPPORT);
    expect(Array.from(first.validity.values)).toContain(0);
    expect(first.erosion.maximum).toBeGreaterThan(0);
    expect(first.material.tint.some((value) => value > 0)).toBe(true);

    const supported = first.validity.values.findIndex(
      (value) => value === PROJECTED_SUPPORT,
    );
    const normal = first.normal.values.slice(supported * 3, supported * 3 + 3);
    expect(Math.hypot(...normal)).toBeCloseTo(1, 5);
  });

  it("reveals finer bands as the transient sample spacing tightens", () => {
    const compiled = program();
    const overview = materializeTerrainWorkingSet(compiled, {
      working_set_id: "camera:overview",
      bounds: compiled.bounds,
      columns: 41,
      rows: 27,
      detail_authority: "overview fixture",
    });
    const close = materializeTerrainWorkingSet(compiled, {
      working_set_id: "camera:close",
      bounds: compiled.bounds,
      columns: 255,
      rows: 165,
      detail_authority: "close fixture",
    });
    expect(overview.active_band_ids).toEqual(["macro"]);
    expect(close.active_band_ids).toEqual(["macro", "meso"]);
    expect(close.field_cells).toBeGreaterThan(overview.field_cells);
  });

  it("snaps a bounded working set to the camera envelope", () => {
    const compiled = program();
    const first = terrainWorkingSetForView(compiled, {
      world_width: 1500,
      world_height: 1000,
      viewport_width: 900,
      viewport_height: 600,
      rendered_scale: 1.5,
      pan_x: 0,
      pan_y: 0,
    });
    const subSamplePan = terrainWorkingSetForView(compiled, {
      world_width: 1500,
      world_height: 1000,
      viewport_width: 900,
      viewport_height: 600,
      rendered_scale: 1.5,
      pan_x: 0.5,
      pan_y: 0.5,
    });
    expect(first.bounds).toEqual(subSamplePan.bounds);
    expect(first.columns * first.rows).toBeLessThanOrEqual(
      proceduralProjection.terrain_program.working_set.max_cells,
    );
    expect(first.bounds.width).toBeLessThan(compiled.bounds.width);
  });

  it("compiles bounded halo patches with identical shared terrain channels", () => {
    const compiled = program();
    const requests = terrainPatchRequestsForView(
      compiled,
      {
        world_width: 1500,
        world_height: 1000,
        viewport_width: 900,
        viewport_height: 600,
        rendered_scale: 1.5,
        pan_x: 0,
        pan_y: 0,
      },
      65,
      65,
    );
    expect(requests.length).toBeGreaterThan(1);
    expect(
      requests.reduce(
        (cells, request) => cells + request.columns * request.rows,
        0,
      ),
    ).toBeLessThanOrEqual(
      proceduralProjection.terrain_program.working_set.max_cells,
    );
    expect(
      requests.reduce(
        (bytes, request) =>
          bytes +
          request.columns *
            request.rows *
            proceduralProjection.terrain_program.working_set.bytes_per_cell,
        0,
      ),
    ).toBeLessThanOrEqual(
      proceduralProjection.terrain_program.working_set.max_bytes,
    );
    expect(
      requests.every(
        (request) =>
          request.working_set_id.startsWith(TERRAIN_PATCH_COMPILER_REVISION) &&
          request.render_window?.halo_samples === TERRAIN_PATCH_HALO_SAMPLES,
      ),
    ).toBe(true);

    const fields = requests.map((request) =>
      materializeTerrainWorkingSet(compiled, request),
    );
    const [left, right] = adjacentHorizontalPair(fields);
    for (let row = 0; row < left.grid.rows; row += 1) {
      const leftIndex = row * left.grid.columns + left.grid.columns - 1;
      const rightIndex = row * right.grid.columns;
      expect(right.validity.values[rightIndex]).toBe(
        left.validity.values[leftIndex],
      );
      for (const channel of [
        "elevation",
        "rainfall",
        "flow_accumulation",
        "erosion",
        "curvature",
      ] as const)
        expect(right[channel].values[rightIndex]).toBe(
          left[channel].values[leftIndex],
        );
      for (const [channel, components] of [
        ["flow_direction", 2],
        ["normal", 3],
      ] as const)
        for (let component = 0; component < components; component += 1)
          expect(
            right[channel].values[rightIndex * components + component],
          ).toBe(left[channel].values[leftIndex * components + component]);
      for (let component = 0; component < 3; component += 1)
        expect(right.material.tint[rightIndex * 3 + component]).toBe(
          left.material.tint[leftIndex * 3 + component],
        );
      expect(right.material.occlusion[rightIndex]).toBe(
        left.material.occlusion[leftIndex],
      );
      expect(right.material.roughness[rightIndex]).toBe(
        left.material.roughness[leftIndex],
      );
    }
    const [top, bottom] = adjacentVerticalPair(fields);
    for (let column = 0; column < top.grid.columns; column += 1) {
      const topIndex = (top.grid.rows - 1) * top.grid.columns + column;
      const bottomIndex = column;
      expect(terrainSample(bottom, bottomIndex)).toEqual(
        terrainSample(top, topIndex),
      );
    }
  });

  it("reserves patch halos inside the maximum camera allocation", () => {
    const requests = terrainPatchRequestsForView(
      program(),
      {
        world_width: 1500,
        world_height: 1000,
        viewport_width: 1500,
        viewport_height: 1000,
        rendered_scale: 4,
        pan_x: 0,
        pan_y: 0,
      },
      65,
      65,
    );
    expect(requests.length).toBeGreaterThan(1);
    expect(
      requests.every(
        (request) =>
          request.render_window !== undefined &&
          request.render_window.columns <= 65 &&
          request.render_window.rows <= 65,
      ),
    ).toBe(true);
    expect(
      requests.reduce(
        (cells, request) => cells + request.columns * request.rows,
        0,
      ),
    ).toBeLessThanOrEqual(
      proceduralProjection.terrain_program.working_set.max_cells,
    );
  });

  it("retains exact patch identities inside explicit LRU budgets", () => {
    const compiled = program();
    const requests = terrainPatchRequestsForView(
      compiled,
      {
        world_width: 1500,
        world_height: 1000,
        viewport_width: 900,
        viewport_height: 600,
        rendered_scale: 1.5,
        pan_x: 0,
        pan_y: 0,
      },
      65,
      65,
    );
    const cache = new TerrainPatchCache(
      proceduralProjection.terrain_program.working_set.max_cells,
      proceduralProjection.terrain_program.working_set.max_bytes,
    );
    const first = cache.materialize(compiled, requests[0]!);
    const retained = cache.materialize(compiled, requests[0]!);
    expect(retained).toBe(first);
    expect(cache.stats()).toMatchObject({ hits: 1, misses: 1, evictions: 0 });
  });
});

function adjacentHorizontalPair(
  fields: readonly ReturnType<typeof materializeTerrainWorkingSet>[],
) {
  for (const left of fields)
    for (const right of fields)
      if (
        left !== right &&
        Math.abs(
          left.grid.bounds.x + left.grid.bounds.width - right.grid.bounds.x,
        ) < 0.000_001 &&
        left.grid.bounds.y === right.grid.bounds.y &&
        left.grid.bounds.height === right.grid.bounds.height &&
        left.grid.rows === right.grid.rows
      )
        return [left, right] as const;
  throw new Error("fixture has no horizontally adjacent terrain patches");
}

function adjacentVerticalPair(
  fields: readonly ReturnType<typeof materializeTerrainWorkingSet>[],
) {
  for (const top of fields)
    for (const bottom of fields)
      if (
        top !== bottom &&
        Math.abs(
          top.grid.bounds.y + top.grid.bounds.height - bottom.grid.bounds.y,
        ) < 0.000_001 &&
        top.grid.bounds.x === bottom.grid.bounds.x &&
        top.grid.bounds.width === bottom.grid.bounds.width &&
        top.grid.columns === bottom.grid.columns
      )
        return [top, bottom] as const;
  throw new Error("fixture has no vertically adjacent terrain patches");
}

function terrainSample(
  fields: ReturnType<typeof materializeTerrainWorkingSet>,
  index: number,
) {
  return [
    fields.validity.values[index],
    fields.elevation.values[index],
    fields.rainfall.values[index],
    ...fields.flow_direction.values.slice(index * 2, index * 2 + 2),
    fields.flow_accumulation.values[index],
    fields.erosion.values[index],
    ...fields.normal.values.slice(index * 3, index * 3 + 3),
    fields.curvature.values[index],
    ...fields.material.tint.slice(index * 3, index * 3 + 3),
    fields.material.occlusion[index],
    fields.material.roughness[index],
  ];
}
