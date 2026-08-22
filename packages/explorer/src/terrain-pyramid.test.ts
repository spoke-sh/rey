import { describe, expect, it } from "vitest";
import {
  finalizeLandscapeHeightPyramid,
  finalizeLandscapeReliefPyramid,
  verifyLandscapeHeightPyramid,
  verifyLandscapeReliefPyramid,
  type LandscapeHeightPyramidInput,
  type LandscapeReliefPyramidInput,
} from "./terrain-pyramid";

describe("landscape pyramid contracts", () => {
  it("content-identifies metric height levels and exact parent/child links", () => {
    const input = heightInput();
    const first = finalizeLandscapeHeightPyramid(input);
    const replay = finalizeLandscapeHeightPyramid({
      ...input,
      levels: [...input.levels].reverse(),
    });

    expect(first).toEqual(replay);
    expect(first.schema).toBe("rey.landscape-height-pyramid.v1");
    expect(first.pyramid_id).toMatch(/^blake3:[0-9a-f]{64}$/);
    expect(first.levels.map(({ level }) => level)).toEqual([0, 1]);
    expect(first.levels[0]!.parent_level_id).toBeNull();
    expect(first.levels[0]!.child_level_id).toBe(first.levels[1]!.level_id);
    expect(first.levels[1]!.parent_level_id).toBe(first.levels[0]!.level_id);
    expect(first.levels[1]!.child_level_id).toBeNull();
    expect(first.byte_length).toBe(106);
  });

  it("binds relief levels to exact height levels and guttered operator support", () => {
    const height = finalizeLandscapeHeightPyramid(heightInput());
    const relief = finalizeLandscapeReliefPyramid(reliefInput(height), height);

    expect(relief.schema).toBe("rey.landscape-relief-pyramid.v1");
    expect(relief.source_height_pyramid_id).toBe(height.pyramid_id);
    expect(relief.levels[1]!.source_height_level_id).toBe(
      height.levels[1]!.level_id,
    );
    expect(
      relief.levels[1]!.operator_support.map(({ operator_id }) => operator_id),
    ).toEqual(["mdow", "svf"]);
    expect(relief.levels[1]!.operator_support[0]!.support_id).toMatch(
      /^blake3:[0-9a-f]{64}$/,
    );
    expect(relief.byte_length).toBe(500);
  });

  it("rejects identity, conservative-validity, and insufficient-gutter drift", () => {
    const height = finalizeLandscapeHeightPyramid(heightInput());
    expect(() =>
      verifyLandscapeHeightPyramid({
        ...height,
        pyramid_id: "blake3:tampered",
      }),
    ).toThrow("parent/child identity");

    const invalidHeight = structuredClone(height);
    invalidHeight.levels[0]!.validity.valid_vertices += 1;
    expect(() => verifyLandscapeHeightPyramid(invalidHeight)).toThrow(
      "validity contract",
    );

    const relief = finalizeLandscapeReliefPyramid(reliefInput(height), height);
    const insufficientGutter = structuredClone(relief);
    insufficientGutter.levels[0]!.operator_support[0]!.gutter_radius_cells = 1;
    expect(() =>
      verifyLandscapeReliefPyramid(insufficientGutter, height),
    ).toThrow("operator support");
  });
});

function heightInput(): LandscapeHeightPyramidInput {
  return {
    implementation_revision: "height-compiler@1",
    mosaic_id: "mosaic:fixture",
    coordinate_reference: "fixture local metric frame",
    vertical_reference: "fixture elevation meters",
    complete: true,
    omissions: [],
    levels: [
      {
        level: 0,
        implementation_revision: "height-level@1",
        sample_spacing_x_meters: 200,
        sample_spacing_y_meters: 200,
        columns: 3,
        rows: 2,
        bounds: { x: 0, y: 0, width: 400, height: 200 },
        validity: {
          validity_id: "validity:coarse",
          valid_vertices: 4,
          no_data_vertices: 1,
          unsupported_vertices: 1,
          policy: "conservative_support_only",
        },
        elevation_minimum_meters: 20,
        elevation_maximum_meters: 300,
        height_bytes: 24,
        validity_bytes: 6,
        source_lineage: [
          { kind: "mosaic", identity: "mosaic:fixture", revision: "mosaic@1" },
        ],
      },
      {
        level: 1,
        implementation_revision: "height-level@1",
        sample_spacing_x_meters: 100,
        sample_spacing_y_meters: 100,
        columns: 5,
        rows: 3,
        bounds: { x: 0, y: 0, width: 400, height: 200 },
        validity: {
          validity_id: "validity:fine",
          valid_vertices: 12,
          no_data_vertices: 2,
          unsupported_vertices: 1,
          policy: "conservative_support_only",
        },
        elevation_minimum_meters: 10,
        elevation_maximum_meters: 340,
        height_bytes: 60,
        validity_bytes: 16,
        source_lineage: [
          { kind: "mosaic", identity: "mosaic:fixture", revision: "mosaic@1" },
        ],
      },
    ],
  };
}

function reliefInput(
  height: ReturnType<typeof finalizeLandscapeHeightPyramid>,
): LandscapeReliefPyramidInput {
  return {
    implementation_revision: "relief-compiler@1",
    mosaic_id: height.mosaic_id,
    source_height_pyramid_id: height.pyramid_id,
    coordinate_reference: height.coordinate_reference,
    vertical_reference: height.vertical_reference,
    complete: true,
    omissions: [],
    levels: height.levels.map((level, index) => ({
      level: level.level,
      implementation_revision: "relief-level@1",
      source_height_level_id: level.level_id,
      sample_spacing_x_meters: level.sample_spacing_x_meters,
      sample_spacing_y_meters: level.sample_spacing_y_meters,
      columns: level.columns,
      rows: level.rows,
      bounds: level.bounds,
      validity: level.validity,
      channel_ids: ["svf", "hillshade"],
      operator_support: [
        {
          operator_id: "svf",
          implementation_revision: "svf@1",
          target_radius_meters: 1_600,
          support_radius_cells: 8,
          support_radius_meters: 1_600,
          gutter_radius_cells: 8,
          supported: true,
          validity_policy: "complete_valid_window",
        },
        {
          operator_id: "mdow",
          implementation_revision: "mdow@1",
          target_radius_meters: 800,
          support_radius_cells: 4,
          support_radius_meters: 800,
          gutter_radius_cells: 4,
          supported: true,
          validity_policy: "complete_valid_window",
        },
      ],
      relief_bytes: index === 0 ? 200 : 300,
      source_lineage: level.source_lineage,
    })),
  };
}
