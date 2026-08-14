import { describe, expect, it } from "vitest";
import {
  buildProjectedBoundsMeshes,
  buildProjectedGlobeMesh,
  GLOBE_CAMERA_HALF_HEIGHT,
  globeAtlasViewCenter,
  globeAtmosphereOpacity,
  globeProjectionMorphRemaining,
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
