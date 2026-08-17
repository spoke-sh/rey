import { describe, expect, it } from "vitest";
import {
  atlasLandscapeCompositionScale,
  atlasLandscapeMorphProgress,
  atlasLandscapePresentation,
  projectAtlasLandscapePoint,
} from "./atlas-landscape";

const binding = {
  source_frame: { x: 120, y: 160, width: 180, height: 90 },
  target_frame: { x: 96, y: 72, width: 1008, height: 576 },
};
const orbit = { pitch_degrees: 88, yaw_degrees: 0 };
const world = { width: 1200, height: 720 };

describe("Atlas-to-Landscape projector", () => {
  it("keeps the terrain attached to the Atlas frame at one endpoint", () => {
    const atlas = atlasLandscapePresentation(binding, 0, orbit, world);
    expect(atlas).toMatchObject({
      progress: 0,
      pitch_degrees: 90,
      // yaw is held at orbit.yaw_degrees for the whole swoop (never
      // animated), so it's already this fixture's orbit.yaw_degrees (0)
      // at progress 0, not merely coincidentally starting there.
      yaw_degrees: 0,
      terrain_opacity: 0,
      atlas_opacity: 1,
      composition_scale: 1,
    });
    expect(
      projectAtlasLandscapePoint(
        { x: binding.target_frame.x, y: binding.target_frame.y },
        atlas.model_transform,
      ),
    ).toEqual({ x: binding.source_frame.x, y: binding.source_frame.y });
    expect(
      projectAtlasLandscapePoint(
        {
          x: binding.target_frame.x + binding.target_frame.width,
          y: binding.target_frame.y + binding.target_frame.height,
        },
        atlas.model_transform,
      ),
    ).toEqual({
      x: binding.source_frame.x + binding.source_frame.width,
      y: binding.source_frame.y + binding.source_frame.height,
    });
  });

  it("ends at an identity terrain model and is direction-independent", () => {
    const landscape = atlasLandscapePresentation(binding, 1, orbit, world);
    expect(landscape.model_transform).toEqual({
      scale_x: 1,
      scale_z: 1,
      translate_x: 0,
      translate_z: 0,
      elevation_scale: 1,
    });
    expect(landscape.pitch_degrees).toBeCloseTo(orbit.pitch_degrees);
    expect(landscape.yaw_degrees).toBeCloseTo(orbit.yaw_degrees);
    expect(landscape.atlas_opacity).toBe(0);
    expect(landscape.terrain_opacity).toBe(1);
    expect(landscape.composition_scale).toBeCloseTo(1.38);

    const forward = [0, 0.25, 0.5, 0.75, 1].map((progress) =>
      atlasLandscapePresentation(binding, progress, orbit, world),
    );
    const reverse = [1, 0.75, 0.5, 0.25, 0]
      .map((progress) =>
        atlasLandscapePresentation(binding, progress, orbit, world),
      )
      .reverse();
    expect(reverse).toEqual(forward);
    expect(forward[2]!.terrain_opacity).toBeGreaterThan(0);
    expect(forward[2]!.atlas_opacity).toBeGreaterThan(0);
    expect(forward.map(({ composition_scale }) => composition_scale)).toEqual(
      [...forward]
        .map(({ composition_scale }) => composition_scale)
        .sort((left, right) => left - right),
    );
  });

  it("uses a continuous perceptual curve independent from semantic LOD", () => {
    const samples = [0.34, 0.4, 0.48, 0.58, 0.66].map(
      atlasLandscapeMorphProgress,
    );
    expect(samples[0]).toBe(0);
    expect(samples.at(-1)).toBe(1);
    expect(samples).toEqual([...samples].sort((left, right) => left - right));
  });

  it("exposes composition_scale standalone, exactly matching the full presentation", () => {
    // Zoom-anchoring call sites (explore.tsx's applyZoomAt/focusNode) need
    // this exact value without paying for the rest of the presentation
    // (model transform, pitch/yaw, opacities) at every candidate zoom they
    // evaluate — this locks in that the standalone function and the full
    // presentation never drift apart, which would silently reintroduce the
    // pan-drift bug this split was written to fix.
    for (const progress of [0, 0.12, 0.3, 0.5, 0.75, 1]) {
      const { composition_scale } = atlasLandscapePresentation(
        binding,
        progress,
        orbit,
        world,
      );
      expect(atlasLandscapeCompositionScale(progress)).toBe(composition_scale);
    }
    expect(atlasLandscapeCompositionScale(-1)).toBe(
      atlasLandscapeCompositionScale(0),
    );
    expect(atlasLandscapeCompositionScale(2)).toBe(
      atlasLandscapeCompositionScale(1),
    );
    expect(() => atlasLandscapeCompositionScale(Number.NaN)).toThrow(
      "Atlas-to-Landscape composition scale requires finite progress",
    );
  });

  it("holds yaw constant through the swoop instead of animating it from zero", () => {
    // Regression test for the reported label jitter: sweeping pitch and yaw
    // together during the swoop made off-center screen points move
    // non-monotonically. Only pitch is animated; yaw is pinned at the
    // operator's current orbit.yaw_degrees for the whole transition.
    const yawedOrbit = { pitch_degrees: 82, yaw_degrees: 63 };
    const samples = [0, 0.12, 0.25, 0.4, 0.6, 0.8, 1].map(
      (progress) =>
        atlasLandscapePresentation(binding, progress, yawedOrbit, world)
          .yaw_degrees,
    );
    for (const yawDegrees of samples)
      expect(yawDegrees).toBe(yawedOrbit.yaw_degrees);
  });

  it("keeps an off-center point's screen Y monotonic across the whole swoop", () => {
    // Root-cause regression test for the reported jitter: previously,
    // animating yaw alongside pitch could make an off-pivot point's screen Y
    // reverse direction mid-zoom. With yaw held constant this is
    // mathematically guaranteed monotonic for flat (zero-elevation) content,
    // which covers every label — verified here across several yaw/pitch
    // combinations by sampling the real css_transform this module emits.
    const point = {
      x: binding.target_frame.x + binding.target_frame.width * 0.75,
      y: binding.target_frame.y + binding.target_frame.height * 0.2,
    };
    const orbits = [
      { pitch_degrees: 88, yaw_degrees: 0 },
      { pitch_degrees: 82, yaw_degrees: 37 },
      { pitch_degrees: 70, yaw_degrees: -125 },
      { pitch_degrees: 28, yaw_degrees: 179 },
    ];
    for (const sampleOrbit of orbits) {
      const screenYs: number[] = [];
      for (let step = 0; step <= 40; step += 1) {
        const presentation = atlasLandscapePresentation(
          binding,
          step / 40,
          sampleOrbit,
          world,
        );
        screenYs.push(applyCssMatrix(presentation.css_transform, point).y);
      }
      const deltas = screenYs.slice(1).map((y, index) => y - screenYs[index]!);
      const increasing = deltas.every((delta) => delta >= -1e-6);
      const decreasing = deltas.every((delta) => delta <= 1e-6);
      expect(increasing || decreasing).toBe(true);
    }
  });
});

function applyCssMatrix(
  matrix: string,
  point: { x: number; y: number },
): { x: number; y: number } {
  const values = matrix
    .replace("matrix(", "")
    .replace(")", "")
    .split(",")
    .map((value) => Number.parseFloat(value));
  const [a, b, c, d, e, f] = values as [
    number,
    number,
    number,
    number,
    number,
    number,
  ];
  return {
    x: a * point.x + c * point.y + e,
    y: b * point.x + d * point.y + f,
  };
}
