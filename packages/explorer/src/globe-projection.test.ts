import { describe, expect, it } from "vitest";
import { Object3D, Vector3 } from "three/src/Three.WebGPU.js";
import {
  buildProjectedBoundsMeshes,
  buildProjectedGlobeMesh,
  GLOBE_ATLAS_REPEAT_DEPTH_CONNECTION_WEIGHT,
  GLOBE_ATLAS_REPEAT_MAX_DEPTH,
  GLOBE_CAMERA_DISTANCE,
  GLOBE_CAMERA_HALF_HEIGHT,
  globeAtlasRepeatDepthOffset,
  globeAtlasRepeatOpacity,
  globeAtlasRepeatOffset,
  globeAtlasRepeatPeriod,
  globeAtlasRepeatSeamWeight,
  globeAtlasRepeatVisibility,
  globeAtlasProjectionCenter,
  globeAtlasWidth,
  globeAtlasViewCenter,
  globeAtmosphereOpacity,
  globeAtmosphereRepeatOpacity,
  globeAtmosphereShellScale,
  globeCameraPose,
  globeProjectionMorphRemaining,
  globeSurfaceOpacity,
  interpolateProjectedGlobeMeshes,
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

  it("uses one bounded atmosphere opacity at each projection posture", () => {
    expect(globeAtmosphereOpacity(0)).toBe(1);
    expect(globeAtmosphereOpacity(0.5)).toBe(0.25);
    expect(globeAtmosphereOpacity(0.75)).toBeCloseTo(0.0244140625);
    expect(globeAtmosphereOpacity(0.975)).toBeLessThan(0.000_004);
    expect(globeAtmosphereOpacity(1)).toBe(0);
  });

  it("uses one shell extent in both traversal directions", () => {
    expect(globeAtmosphereShellScale(0.75)).toBeCloseTo(Math.sqrt(0.15625));
    expect(globeAtmosphereShellScale(0.5)).toBeCloseTo(Math.sqrt(0.5));
    expect(globeAtmosphereShellScale(1)).toBe(0);
    expect(globeAtmosphereShellScale(0)).toBe(1);
  });

  it("fades atmosphere symmetrically through the horizontal-repeat dissolve", () => {
    expect(globeAtmosphereRepeatOpacity(1)).toBe(0);
    expect(globeAtmosphereRepeatOpacity(0.58)).toBe(0);
    const repeatOpacity = globeAtlasRepeatOpacity(0.79);
    expect(globeAtmosphereRepeatOpacity(0.79)).toBeCloseTo(
      repeatOpacity * (1 - repeatOpacity),
    );
    expect(globeAtmosphereRepeatOpacity(0.79)).toBeLessThanOrEqual(0.25);
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

  it("keeps the projected sphere anchored to the live yaw bearing, independent of pitch", () => {
    // Pitch no longer lives in vertex data at all — only yaw recenters the
    // sphere (for seam placement). The level (pitch-0) bearing is what
    // projects to screen center at every progress now, sphere and Atlas
    // alike, since projectGlobeCoordinate's sphereNormal is Ry(yaw)*local
    // with no pitch term.
    const rotatedView = { yaw_degrees: 58, pitch_degrees: -24 };
    const levelCenter = globeAtlasProjectionCenter(rotatedView);
    for (const progress of [0, 1]) {
      const projected = projectGlobeCoordinate(
        levelCenter.longitude_degrees,
        levelCenter.latitude_degrees,
        rotatedView,
        world,
        progress,
      );
      expect(projected.position[0]).toBeCloseTo(0, 10);
      expect(projected.position[1]).toBeCloseTo(0, 10);
    }
  });

  it("orbits the camera through the full pitch at progress 0 and levels it at progress 1", () => {
    const rotatedView = { yaw_degrees: 58, pitch_degrees: -24 };

    const atlasPose = globeCameraPose(rotatedView, 1);
    expect(atlasPose.position[0]).toBe(0);
    expect(atlasPose.position[1]).toBeCloseTo(0, 10);
    expect(atlasPose.position[2]).toBe(GLOBE_CAMERA_DISTANCE);
    expect(atlasPose.rotation[0]).toBeCloseTo(0, 10);
    expect(atlasPose.rotation[1]).toBe(0);
    expect(atlasPose.rotation[2]).toBe(0);

    const spherePose = globeCameraPose(rotatedView, 0);
    const pitch = (rotatedView.pitch_degrees * Math.PI) / 180;
    expect(spherePose.position[0]).toBeCloseTo(0, 10);
    expect(spherePose.position[1]).toBeCloseTo(
      GLOBE_CAMERA_DISTANCE * Math.sin(pitch),
      10,
    );
    expect(spherePose.position[2]).toBeCloseTo(
      GLOBE_CAMERA_DISTANCE * Math.cos(pitch),
      10,
    );
    expect(spherePose.rotation[0]).toBeCloseTo(-pitch, 10);
    expect(spherePose.rotation[1]).toBe(0);
    expect(spherePose.rotation[2]).toBe(0);

    const midPose = globeCameraPose(rotatedView, 0.5);
    const progressEased = 1 - globeProjectionMorphRemaining(0.5);
    const midPitch =
      (rotatedView.pitch_degrees * (1 - progressEased) * Math.PI) / 180;
    expect(midPose.position[1]).toBeCloseTo(
      GLOBE_CAMERA_DISTANCE * Math.sin(midPitch),
      10,
    );
    expect(midPose.rotation[0]).toBeCloseTo(-midPitch, 10);

    expect(() => globeCameraPose(rotatedView, Number.NaN)).toThrow(
      "globe camera pose requires finite orientation",
    );
  });

  it("always faces the globe's own center, verified against Three.js's own rotation math", () => {
    // The camera pose is a closed-form formula, not something derived from
    // Three.js's lookAt — this locks in that the two agree exactly, for
    // every bounded pitch value, the same rigor used to verify the earlier
    // de-roll correction matrix it replaces.
    for (const pitchDegrees of [-62, -30, -5, 0, 5, 30, 62]) {
      const pose = globeCameraPose({ pitch_degrees: pitchDegrees }, 0);
      const object = new Object3D();
      object.position.set(...pose.position);
      object.rotation.set(...pose.rotation);
      object.updateMatrixWorld(true);
      const forward = new Vector3(0, 0, -1).applyQuaternion(object.quaternion);
      const toOrigin = new Vector3(0, 0, 0).sub(object.position).normalize();
      expect(forward.x).toBeCloseTo(toOrigin.x, 12);
      expect(forward.y).toBeCloseTo(toOrigin.y, 12);
      expect(forward.z).toBeCloseTo(toOrigin.z, 12);
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
    // projectGlobeAtlasRepeatCoordinate's own connected-seam computation
    // uses globeAtlasProjectionCenter (fixed at pitch 0), not the live,
    // possibly-pitched view center — the flat Atlas frame it bends toward
    // has no way to represent pitch, so this must be the same reference.
    const center = globeAtlasProjectionCenter(rotatedView);
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

  it("interpolates cached mesh endpoints without reprojecting coordinates", () => {
    const sphere = buildProjectedGlobeMesh(view, world, 0, 12, 8);
    const atlas = buildProjectedGlobeMesh(view, world, 1, 12, 8);
    const interpolated = interpolateProjectedGlobeMeshes(sphere, atlas, 0.5);
    const direct = buildProjectedGlobeMesh(view, world, 0.5, 12, 8);
    for (let index = 0; index < interpolated.positions.length; index += 1)
      expect(interpolated.positions[index]).toBeCloseTo(
        direct.positions[index]!,
        6,
      );
    for (let index = 0; index < interpolated.normals.length; index += 1)
      expect(interpolated.normals[index]).toBeCloseTo(
        direct.normals[index]!,
        6,
      );
    expect(interpolated.indices).toBe(sphere.indices);
    expect(interpolateProjectedGlobeMeshes(sphere, atlas, 0)).toBe(sphere);
    expect(interpolateProjectedGlobeMeshes(sphere, atlas, 1)).toBe(atlas);
    expect(() =>
      interpolateProjectedGlobeMeshes(sphere, atlas, Number.NaN),
    ).toThrow("globe mesh interpolation progress must be finite");
    expect(() =>
      interpolateProjectedGlobeMeshes(
        sphere,
        { ...atlas, positions: new Float32Array(0) },
        0.5,
      ),
    ).toThrow("globe mesh interpolation endpoints must have equal shape");
  });

  it("writes into a retained output buffer instead of allocating fresh arrays", () => {
    const sphere = buildProjectedGlobeMesh(view, world, 0, 12, 8);
    const atlas = buildProjectedGlobeMesh(view, world, 1, 12, 8);
    const output = {
      positions: new Float32Array(sphere.positions.length),
      normals: new Float32Array(sphere.normals.length),
    };
    const first = interpolateProjectedGlobeMeshes(sphere, atlas, 0.5, output);
    expect(first.positions).toBe(output.positions);
    expect(first.normals).toBe(output.normals);
    const direct = buildProjectedGlobeMesh(view, world, 0.5, 12, 8);
    for (let index = 0; index < first.positions.length; index += 1)
      expect(first.positions[index]).toBeCloseTo(direct.positions[index]!, 6);

    // A later frame at a different progress reuses the same buffer identity
    // and overwrites its contents rather than aliasing stale values.
    const rawProgress = 0.2;
    const easedProgress = 1 - globeProjectionMorphRemaining(rawProgress);
    const second = interpolateProjectedGlobeMeshes(
      sphere,
      atlas,
      easedProgress,
      output,
    );
    expect(second.positions).toBe(output.positions);
    const directSecond = buildProjectedGlobeMesh(
      view,
      world,
      rawProgress,
      12,
      8,
    );
    for (let index = 0; index < second.positions.length; index += 1)
      expect(second.positions[index]).toBeCloseTo(
        directSecond.positions[index]!,
        6,
      );

    // A mismatched buffer (wrong length) is ignored rather than corrupting
    // or throwing, so callers can pass a stale buffer safely.
    const mismatched = {
      positions: new Float32Array(1),
      normals: new Float32Array(1),
    };
    const fallback = interpolateProjectedGlobeMeshes(
      sphere,
      atlas,
      0.5,
      mismatched,
    );
    expect(fallback.positions).not.toBe(mismatched.positions);
    expect(fallback.positions.length).toBe(sphere.positions.length);
  });

  it("carries one normalized chart-X scalar per vertex, matching globeAtlasRepeatSeamWeight's expected input", () => {
    const mesh = buildProjectedGlobeMesh(view, world, 0, 12, 8);
    const vertexCount = 13 * 9;
    expect(mesh.normalizedChartX.length).toBe(vertexCount);
    expect([...mesh.normalizedChartX].every(Number.isFinite)).toBe(true);

    // Spans a west-to-east column at a fixed row; normalizedChartX must
    // increase monotonically west to east, matching how projectGlobeCoordinate
    // itself lays out the same grid.
    const row = 4;
    const columns = 12;
    let previous = -Infinity;
    for (let column = 0; column <= columns; column += 1) {
      const value = mesh.normalizedChartX[row * (columns + 1) + column]!;
      expect(value).toBeGreaterThanOrEqual(previous);
      previous = value;
    }

    // Recomputing one representative vertex directly through
    // projectGlobeCoordinate must reproduce the same value the mesh builder
    // stored — this is the exact quantity globeAtlasRepeatSeamWeight and
    // globeAtlasRepeatVisibility consume.
    const sampleColumn = 3;
    const sampleLongitude = -180 + (sampleColumn / columns) * 360;
    const sampleLatitude = -90 + (row / 8) * 180;
    const point = projectGlobeCoordinate(
      sampleLongitude,
      sampleLatitude,
      view,
      world,
      0,
    );
    const expectedChartX =
      point.atlas_position[0] / globeAtlasWidth(world) + 0.5;
    expect(
      mesh.normalizedChartX[row * (columns + 1) + sampleColumn],
    ).toBeCloseTo(expectedChartX, 6);
  });

  it("keeps normalizedChartX identical and progress-independent across sphere and atlas endpoints", () => {
    const sphere = buildProjectedGlobeMesh(view, world, 0, 12, 8);
    const atlas = buildProjectedGlobeMesh(view, world, 1, 12, 8);
    // atlas_position doesn't depend on progress, so the same vertex index
    // must produce bit-identical normalizedChartX at both endpoints.
    expect([...sphere.normalizedChartX]).toEqual([...atlas.normalizedChartX]);

    // interpolateProjectedGlobeMeshes must carry it through unchanged (no
    // per-frame recomputation or scratch-buffer churn needed for a value
    // that never varies with progress).
    const interpolated = interpolateProjectedGlobeMeshes(sphere, atlas, 0.5);
    expect(interpolated.normalizedChartX).toBe(sphere.normalizedChartX);
    expect(
      interpolateProjectedGlobeMeshes(sphere, atlas, 0).normalizedChartX,
    ).toBe(sphere.normalizedChartX);
    expect(
      interpolateProjectedGlobeMeshes(sphere, atlas, 1).normalizedChartX,
    ).toBe(atlas.normalizedChartX);
  });

  it("also carries normalizedChartX on sector patch meshes", () => {
    const bounds = {
      west_degrees: -60,
      south_degrees: 10,
      east_degrees: -30,
      north_degrees: 30,
      crosses_antimeridian: false,
    };
    for (const wrapIndex of [0, 1, -1]) {
      const [mesh] = buildProjectedBoundsMeshes(
        bounds,
        view,
        world,
        0.5,
        16,
        10,
        wrapIndex,
      );
      const vertexCount = 17 * 11;
      expect(mesh!.normalizedChartX.length).toBe(vertexCount);
      expect([...mesh!.normalizedChartX].every(Number.isFinite)).toBe(true);
    }
  });

  it("reproduces globeAtlasRepeatVisibility exactly via the TSL-equivalent formula the atmosphere shader uses", () => {
    // ProjectedAtmosphereLayer (fiber-scenes atmosphere shader) computes a
    // repeated wrap copy's spatial sweep in TSL as:
    //   mul(smoothstep(sub(1, max(repeatOpacity, eps)), 1, seamWeight),
    //       step(eps, repeatOpacity))
    // This locks that formula's algebraic equivalence to
    // globeAtlasRepeatVisibility(progress, seamWeight) so a refactor of
    // either side can be checked against the other.
    const smoothstepFn = (value: number) => value * value * (3 - 2 * value);
    const smoothstepEdges = (edge0: number, edge1: number, x: number) => {
      const t = Math.max(0, Math.min(1, (x - edge0) / (edge1 - edge0)));
      return smoothstepFn(t);
    };
    const tslSpatialSweep = (repeatOpacity: number, seamWeight: number) => {
      const eps = 0.000_001;
      const edge0 = 1 - Math.max(repeatOpacity, eps);
      const smoothed = smoothstepEdges(edge0, 1, seamWeight);
      const gate = repeatOpacity >= eps ? 1 : 0;
      return smoothed * gate;
    };
    for (let progress = 0; progress <= 1; progress += 0.05) {
      const repeatOpacity = globeAtlasRepeatOpacity(progress);
      for (let seamWeight = 0; seamWeight <= 1; seamWeight += 0.05) {
        expect(tslSpatialSweep(repeatOpacity, seamWeight)).toBeCloseTo(
          globeAtlasRepeatVisibility(progress, seamWeight),
          9,
        );
      }
    }
  });

  it("feeds the atmosphere sweep the seam weight's complement, so it peaks at the outer edge instead of the seam", () => {
    // Unlike sectors/samples/markers, the atmosphere glow deliberately
    // inverts which edge is brightest: dim at the connected seam, brightest
    // at the copy's outer edge, as a directional cue toward more content if
    // the operator keeps unfurling. This mirrors globe-surface.tsx's exact
    // two ternaries verbatim — seamWeightRaw is what ProjectedAtmosphereLayer
    // used before this fix; outerEdgeWeightRaw is what it uses now — so a
    // future accidental revert of one but not the other, or a copy/paste of
    // the wrong ternary, changes these numbers and fails this test.
    const seamWeightRaw = (normalizedChartX: number, wrapIndex: number) =>
      wrapIndex < 0 ? normalizedChartX : 1 - normalizedChartX;
    const outerEdgeWeightRaw = (normalizedChartX: number, wrapIndex: number) =>
      wrapIndex < 0 ? 1 - normalizedChartX : normalizedChartX;
    const smoothstepFn = (value: number) => value * value * (3 - 2 * value);
    const smoothstepEdges = (edge0: number, edge1: number, x: number) => {
      const t = Math.max(0, Math.min(1, (x - edge0) / (edge1 - edge0)));
      return smoothstepFn(t);
    };
    const tslSpatialSweep = (repeatOpacity: number, weight: number) => {
      const eps = 0.000_001;
      const edge0 = 1 - Math.max(repeatOpacity, eps);
      return smoothstepEdges(edge0, 1, weight) * (repeatOpacity >= eps ? 1 : 0);
    };

    for (const wrapIndex of [1, -1]) {
      // Exact numeric endpoints anchor the direction unambiguously: for
      // wrapIndex > 0, outerEdgeWeightRaw is 0 at chartX=0 and 1 at chartX=1
      // (increasing); for wrapIndex < 0 it's the mirror image (decreasing).
      // seamWeightRaw always runs the opposite direction from
      // outerEdgeWeightRaw at every chartX, by construction (complements).
      expect(outerEdgeWeightRaw(0, wrapIndex)).toBeCloseTo(
        wrapIndex < 0 ? 1 : 0,
        10,
      );
      expect(outerEdgeWeightRaw(1, wrapIndex)).toBeCloseTo(
        wrapIndex < 0 ? 0 : 1,
        10,
      );

      for (
        let normalizedChartX = 0;
        normalizedChartX <= 1;
        normalizedChartX += 0.05
      ) {
        const seamWeight = seamWeightRaw(normalizedChartX, wrapIndex);
        const outerEdgeWeight = outerEdgeWeightRaw(normalizedChartX, wrapIndex);
        expect(outerEdgeWeight).toBeCloseTo(1 - seamWeight, 10);
      }
    }

    // At a representative mid-dissolve progress, the position that used to
    // be fully bright (seam) is now fully dark, and the position that used
    // to be fully dark (outer edge) is now fully bright — for both wrapIndex
    // signs, using each one's own concrete chartX endpoints rather than a
    // shared literal value.
    const progress = 0.79;
    const repeatOpacity = globeAtlasRepeatOpacity(progress);
    for (const wrapIndex of [1, -1]) {
      const seamChartX = wrapIndex < 0 ? 1 : 0;
      const outerEdgeChartX = wrapIndex < 0 ? 0 : 1;
      const oldBrightnessAtSeam = tslSpatialSweep(
        repeatOpacity,
        seamWeightRaw(seamChartX, wrapIndex),
      );
      const newBrightnessAtSeam = tslSpatialSweep(
        repeatOpacity,
        outerEdgeWeightRaw(seamChartX, wrapIndex),
      );
      const oldBrightnessAtOuterEdge = tslSpatialSweep(
        repeatOpacity,
        seamWeightRaw(outerEdgeChartX, wrapIndex),
      );
      const newBrightnessAtOuterEdge = tslSpatialSweep(
        repeatOpacity,
        outerEdgeWeightRaw(outerEdgeChartX, wrapIndex),
      );
      expect(oldBrightnessAtSeam).toBeCloseTo(1, 6);
      expect(newBrightnessAtSeam).toBeCloseTo(0, 6);
      expect(oldBrightnessAtOuterEdge).toBeCloseTo(0, 6);
      expect(newBrightnessAtOuterEdge).toBeCloseTo(1, 6);
    }
  });
});
