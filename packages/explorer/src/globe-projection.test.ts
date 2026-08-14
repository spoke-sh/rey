import { describe, expect, it } from "vitest";
import {
  buildProjectedBoundsMeshes,
  buildProjectedGlobeMesh,
  GLOBE_ATLAS_REPEAT_DEPTH_CONNECTION_WEIGHT,
  GLOBE_ATLAS_REPEAT_MAX_DEPTH,
  GLOBE_CAMERA_HALF_HEIGHT,
  globeAtlasRepeatDepthOffset,
  globeAtlasRepeatOpacity,
  globeAtlasRepeatOffset,
  globeAtlasRepeatPeriod,
  globeAtlasRepeatSeamWeight,
  globeAtlasRepeatVisibility,
  globeAtlasWidth,
  globeAtlasViewCenter,
  globeAtmosphereOpacity,
  globeProjectionMorphRemaining,
  globeSurfaceOpacity,
  projectGlobeAtlasRepeatCoordinate,
  projectGlobeCoordinate,
  SEMANTIC_MERCATOR_LATITUDE_CUTOFF_DEGREES,
} from "./globe-projection";
import { GLOBE_RADIUS } from "./three-globe";

const world = { width: 1200, height: 720 };
const view = { yaw_degrees: 0, pitch_degrees: 0 };

describe("declarative globe-to-Mercator projection", () => {
  it("provides one bounded atmosphere contraction curve", () => {
    expect(globeProjectionMorphRemaining(-1)).toBe(1);
    expect(globeProjectionMorphRemaining(0.5)).toBe(0.5);
    expect(globeProjectionMorphRemaining(2)).toBe(0);
    expect(() => globeProjectionMorphRemaining(Number.NaN)).toThrow(
      "globe projection progress must be finite",
    );
  });

  it("fades atmosphere faster than it contracts", () => {
    expect(globeAtmosphereOpacity(0)).toBe(1);
    expect(globeAtmosphereOpacity(0.5)).toBe(0.03125);
    expect(globeAtmosphereOpacity(0.75)).toBeCloseTo(0.0000931323);
    expect(globeAtmosphereOpacity(1)).toBe(0);
  });

  it("removes the visible globe scaffold before repeat fabric is prominent", () => {
    expect(globeSurfaceOpacity(0)).toBe(1);
    expect(globeSurfaceOpacity(0.5)).toBeCloseTo(0.25);
    expect(globeSurfaceOpacity(0.62)).toBe(0);
    expect(globeSurfaceOpacity(0.71)).toBe(0);
    expect(globeSurfaceOpacity(1)).toBe(0);
  });

  it("keeps one coordinate on the sphere until the shared surface unfurls", () => {
    const sphere = projectGlobeCoordinate(0, 0, view, world, 0);
    expect(sphere.position).toEqual([0, 0, GLOBE_RADIUS]);
    expect(sphere.normal).toEqual([0, 0, 1]);

    const atlas = projectGlobeCoordinate(0, 0, view, world, 1);
    expect(atlas.position[0]).toBe(0);
    expect(atlas.position[1]).toBeCloseTo(0);
    expect(atlas.position[2]).toBe(0);
    expect(atlas.normal).toEqual([0, 0, 1]);
    const east = projectGlobeCoordinate(180, 0, view, world, 1);
    expect(east.position[0]).toBeCloseTo(
      GLOBE_CAMERA_HALF_HEIGHT * (world.width / world.height) * 0.985,
    );
    const north = projectGlobeCoordinate(
      0,
      SEMANTIC_MERCATOR_LATITUDE_CUTOFF_DEGREES,
      view,
      world,
      1,
    );
    expect(north.position[1]).toBeCloseTo(GLOBE_CAMERA_HALF_HEIGHT * 0.985, 5);
  });

  it("uses the same projection for the base surface and admitted sectors", () => {
    const [sector] = buildProjectedBoundsMeshes(
      {
        west_degrees: -60,
        south_degrees: 10,
        east_degrees: -30,
        north_degrees: 30,
        crosses_antimeridian: false,
      },
      { ...view, projection_morph_progress: 0.42 },
      world,
      0.42,
    );
    const southwest = projectGlobeCoordinate(
      -60,
      10,
      view,
      world,
      0.42,
      GLOBE_RADIUS * 1.008,
      0.016,
    );
    expect(Array.from(sector!.positions.slice(0, 3))).toEqual(
      southwest.position.map(Math.fround),
    );
    expect(sector!.indices.length).toBe(16 * 10 * 6);
  });

  it("keeps the rotated view center anchored while the globe unfurls", () => {
    const rotatedView = { yaw_degrees: 58, pitch_degrees: -24 };
    const center = globeAtlasViewCenter(rotatedView);
    for (const progress of [0, 0.25, 0.5, 0.75, 1]) {
      const projected = projectGlobeCoordinate(
        center.longitude_degrees,
        center.latitude_degrees,
        rotatedView,
        world,
        progress,
      );
      expect(projected.position[0]).toBeCloseTo(0, 10);
      expect(projected.position[1]).toBeCloseTo(0, 10);
    }
  });

  it("moves the surface seam behind a rotated view", () => {
    const rotatedView = { yaw_degrees: 72, pitch_degrees: -18 };
    const center = globeAtlasViewCenter(rotatedView);
    const mesh = buildProjectedGlobeMesh(rotatedView, world, 1, 12, 8);
    const rowLength = 13 * 3;
    const equatorOffset = 4 * rowLength;
    expect(mesh.positions[equatorOffset]).toBeCloseTo(
      -GLOBE_CAMERA_HALF_HEIGHT * (world.width / world.height) * 0.985,
      5,
    );
    expect(mesh.positions[equatorOffset + 12 * 3]).toBeCloseTo(
      GLOBE_CAMERA_HALF_HEIGHT * (world.width / world.height) * 0.985,
      5,
    );
    expect(center.longitude_degrees).not.toBe(0);
  });

  it("defines one exact horizontal period for repeated Atlas charts", () => {
    expect(globeAtlasWidth(world)).toBeCloseTo(
      GLOBE_CAMERA_HALF_HEIGHT * (world.width / world.height) * 0.985 * 2,
    );
    expect(() => globeAtlasWidth({ width: 0, height: 720 })).toThrow(
      "globe projection requires a finite positive world",
    );
  });

  it("keeps repeated charts attached to the unfurling projection seam", () => {
    expect(globeAtlasRepeatPeriod(world, 0)).toBe(0);
    expect(globeAtlasRepeatPeriod(world, 0.5)).toBeCloseTo(
      globeAtlasWidth(world) / 2,
    );
    expect(globeAtlasRepeatPeriod(world, 1)).toBeCloseTo(
      globeAtlasWidth(world),
    );
    expect(globeAtlasRepeatOffset(world, 0, 1)).toBeCloseTo(
      globeAtlasWidth(world) / 2,
    );
    expect(globeAtlasRepeatOffset(world, 0.5, 1)).toBeCloseTo(
      (globeAtlasWidth(world) * 3) / 4,
    );
    expect(globeAtlasRepeatOffset(world, 1, 1)).toBeCloseTo(
      globeAtlasWidth(world),
    );
    expect(globeAtlasRepeatOffset(world, 0.5, -1)).toBeCloseTo(
      (-globeAtlasWidth(world) * 3) / 4,
    );
    expect(globeAtlasRepeatOffset(world, 0.5, 0)).toBe(0);
    expect(() => globeAtlasRepeatOffset(world, 0.5, 2)).toThrow(
      "globe Atlas repeat index must be -1, 0, or 1",
    );

    const progress = 0.79;
    const mesh = buildProjectedGlobeMesh(view, world, progress, 12, 8);
    const rowLength = 13 * 3;
    for (let row = 0; row <= 8; row += 1) {
      const west = mesh.positions[row * rowLength]!;
      const east = mesh.positions[row * rowLength + 12 * 3]!;
      expect(east - west).toBeCloseTo(
        globeAtlasRepeatPeriod(world, progress),
        5,
      );
    }
  });

  it("keeps repeat interiors planar while bending only their seam joint", () => {
    const rotatedView = { yaw_degrees: 58, pitch_degrees: -24 };
    const progress = 0.71;
    const center = globeAtlasViewCenter(rotatedView);
    for (const wrapIndex of [-1, 1]) {
      const connectedLongitude = center.longitude_degrees + wrapIndex * 180;
      const sourceSeamLongitude = center.longitude_degrees - wrapIndex * 180;
      const repeatedSeam = projectGlobeAtlasRepeatCoordinate(
        sourceSeamLongitude,
        28,
        rotatedView,
        world,
        progress,
        wrapIndex,
      );
      const canonicalSeam = projectGlobeCoordinate(
        connectedLongitude,
        28,
        rotatedView,
        world,
        progress,
      );
      expect(
        repeatedSeam.position[0] +
          globeAtlasRepeatOffset(world, progress, wrapIndex),
      ).toBeCloseTo(canonicalSeam.position[0], 8);
      expect(repeatedSeam.position[1]).toBeCloseTo(
        canonicalSeam.position[1],
        8,
      );
      expect(repeatedSeam.position[2]).toBeCloseTo(
        canonicalSeam.position[2],
        8,
      );

      const interior = projectGlobeAtlasRepeatCoordinate(
        center.longitude_degrees,
        12,
        rotatedView,
        world,
        progress,
        wrapIndex,
      );
      const mercator = projectGlobeCoordinate(
        center.longitude_degrees,
        12,
        rotatedView,
        world,
        1,
      );
      expect(interior.position[0]).toBeCloseTo(mercator.position[0], 8);
      expect(interior.position[1]).toBeCloseTo(mercator.position[1], 8);
      expect(interior.position[2]).toBeLessThan(mercator.position[2]);
      expect(interior.normal).toEqual([0, 0, 1]);
    }
    expect(() =>
      projectGlobeAtlasRepeatCoordinate(0, 0, rotatedView, world, progress, 2),
    ).toThrow("globe Atlas repeat projection requires index -1 or 1");
  });

  it("mirrors a dark-to-light dissolve away from each connected seam", () => {
    expect(globeAtlasRepeatSeamWeight(0, -1)).toBe(0);
    expect(globeAtlasRepeatSeamWeight(0.5, -1)).toBe(0.5);
    expect(globeAtlasRepeatSeamWeight(1, -1)).toBe(1);
    expect(globeAtlasRepeatSeamWeight(0, 1)).toBe(1);
    expect(globeAtlasRepeatSeamWeight(0.5, 1)).toBe(0.5);
    expect(globeAtlasRepeatSeamWeight(1, 1)).toBe(0);
    expect(globeAtlasRepeatSeamWeight(0.25, 0)).toBe(1);
    expect(globeAtlasRepeatSeamWeight(-1, 1)).toBe(1);
    expect(globeAtlasRepeatSeamWeight(2, 1)).toBe(0);
    expect(() => globeAtlasRepeatSeamWeight(Number.NaN, 1)).toThrow(
      "globe Atlas repeat position must be finite",
    );
    expect(() => globeAtlasRepeatSeamWeight(0.5, 0.5)).toThrow(
      "globe Atlas repeat index must be an integer",
    );
    expect(globeAtlasRepeatDepthOffset(0.79, 1)).toBe(0);
    expect(
      globeAtlasRepeatDepthOffset(
        0.79,
        GLOBE_ATLAS_REPEAT_DEPTH_CONNECTION_WEIGHT,
      ),
    ).toBe(0);
    expect(
      globeAtlasRepeatDepthOffset(
        0.79,
        GLOBE_ATLAS_REPEAT_DEPTH_CONNECTION_WEIGHT - 0.01,
      ),
    ).toBeLessThan(0);
    expect(globeAtlasRepeatDepthOffset(0, 0)).toBe(
      -GLOBE_ATLAS_REPEAT_MAX_DEPTH,
    );
    expect(globeAtlasRepeatDepthOffset(0.79, 0)).toBeLessThan(0);
    expect(globeAtlasRepeatDepthOffset(1, 0)).toBe(0);
    expect(() => globeAtlasRepeatDepthOffset(0.79, Number.NaN)).toThrow(
      "globe Atlas repeat seam weight must be finite",
    );
  });

  it("uses one reversible dissolve curve entering and exiting Atlas", () => {
    expect(globeAtlasRepeatOpacity(0.58)).toBe(0);
    expect(globeAtlasRepeatOpacity(0.79)).toBeCloseTo(0.5);
    expect(globeAtlasRepeatOpacity(1)).toBe(1);
    const entering = [0.58, 0.68, 0.79, 0.9, 1].map(globeAtlasRepeatOpacity);
    const exiting = [1, 0.9, 0.79, 0.68, 0.58].map(globeAtlasRepeatOpacity);
    expect(exiting).toEqual([...entering].reverse());
    expect(() => globeAtlasRepeatOpacity(Number.NaN)).toThrow(
      "globe Atlas repeat progress must be finite",
    );
  });

  it("grows the repeat gradient outward from an opaque connected seam", () => {
    expect(globeAtlasRepeatVisibility(0.58, 1)).toBe(0);
    expect(globeAtlasRepeatVisibility(0.79, 1)).toBe(1);
    expect(globeAtlasRepeatVisibility(0.79, 0.75)).toBeCloseTo(0.5);
    expect(globeAtlasRepeatVisibility(0.79, 0.5)).toBeCloseTo(0);
    expect(globeAtlasRepeatVisibility(1, 1)).toBe(1);
    expect(globeAtlasRepeatVisibility(1, 0.5)).toBe(0.5);
    expect(globeAtlasRepeatVisibility(1, 0)).toBe(0);
    expect(() => globeAtlasRepeatVisibility(0.79, Number.NaN)).toThrow(
      "globe Atlas repeat seam weight must be finite",
    );
  });

  it("splits attached sectors at the rotated view seam", () => {
    const rotatedView = { yaw_degrees: 58, pitch_degrees: -24 };
    const fragments = buildProjectedBoundsMeshes(
      {
        west_degrees: 115,
        south_degrees: -10,
        east_degrees: 145,
        north_degrees: 10,
        crosses_antimeridian: false,
      },
      rotatedView,
      world,
      1,
      4,
      2,
    );
    expect(fragments).toHaveLength(2);
    for (const fragment of fragments) {
      const west = fragment.positions[0]!;
      const east = fragment.positions[4 * 3]!;
      expect(Math.abs(east - west)).toBeLessThan(
        GLOBE_CAMERA_HALF_HEIGHT * (world.width / world.height),
      );
    }
  });

  it("builds bounded indexed geometry at both projection endpoints", () => {
    for (const progress of [0, 1]) {
      const mesh = buildProjectedGlobeMesh(view, world, progress, 12, 8);
      expect(mesh.positions.length).toBe(13 * 9 * 3);
      expect(mesh.normals.length).toBe(mesh.positions.length);
      expect(mesh.indices.length).toBe(12 * 8 * 6);
      expect([...mesh.positions].every(Number.isFinite)).toBe(true);
      expect(Math.max(...mesh.indices)).toBeLessThan(13 * 9);
    }
  });
});
