import { planarPresentationSamples } from "@rey/explorer/globe-samples";
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

  it("keeps an off-center point's screen Y close to monotonic across the whole swoop", () => {
    // Root-cause regression test for the reported jitter: previously,
    // animating yaw alongside pitch could make an off-pivot point's screen Y
    // reverse direction mid-zoom. With yaw held constant, projectTerrainCoordinate
    // itself is monotonic in screen Y for a *fixed* point as pitch alone
    // sweeps — but css_transform composes that with model_transform, whose
    // scale/translate are *also* interpolating over the same progress. That
    // composition is what's actually rendered, and composing two
    // independently-monotonic factors doesn't guarantee the product is
    // monotonic — so this is a bounded-tolerance check, not the strict
    // guarantee this used to claim (an earlier version of cssTerrainTransform
    // dropped a pivot term, which happened to be monotonic here by
    // coincidence — it was also wrong by up to half the world's width at
    // every progress value, not just this one). Swept the full reachable
    // pitch/yaw envelope ([28,88] x [-180,180], `draggedTerrainOrbit`'s own
    // clamp) offline; worst observed reversal was ~42px, so 50px leaves
    // headroom without hiding a real regression back toward the old bug's
    // scale (hundreds of px).
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
    const maximumReversalPixels = 50;
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
      let runningMinimum = screenYs[0]!;
      let runningMaximum = screenYs[0]!;
      let worstIncreasingReversal = 0;
      let worstDecreasingReversal = 0;
      for (const y of screenYs) {
        worstIncreasingReversal = Math.max(
          worstIncreasingReversal,
          runningMaximum - y,
        );
        worstDecreasingReversal = Math.max(
          worstDecreasingReversal,
          y - runningMinimum,
        );
        runningMaximum = Math.max(runningMaximum, y);
        runningMinimum = Math.min(runningMinimum, y);
      }
      expect(
        Math.min(worstIncreasingReversal, worstDecreasingReversal),
      ).toBeLessThan(maximumReversalPixels);
    }
  });

  it("lands the terrain stipple's dots exactly on the Atlas sector's own stipple dots at progress 0", () => {
    // reference.tsx's AtlasFeatureLayer seeds the focused sector's stipple
    // with the same terrain field revision the landscape stipple uses, over
    // the sector's own (untransformed) rect. AdmittedTerrainFieldLayer seeds
    // the terrain stipple with that revision over field.grid.bounds
    // (target_frame), which — being inside the transformed wrapper — gets
    // warped by this exact css_transform. For the "stipple resolving into
    // relief" effect to actually read as connected rather than as two
    // coincidentally-similar patterns, those two need to land on the same
    // screen pixels at progress 0, where source_frame and the sector rect
    // are the same rectangle. This locks that in with the real sample
    // fabric and the real transform, not just the hand-derived algebra.
    const seed = "dataset:focused-region";
    const samples = planarPresentationSamples(seed, 12);
    expect(samples.length).toBeGreaterThan(0);
    const presentation = atlasLandscapePresentation(binding, 0, orbit, world);
    for (const sample of samples) {
      const terrainPoint = {
        x: binding.target_frame.x + sample.u * binding.target_frame.width,
        y: binding.target_frame.y + sample.v * binding.target_frame.height,
      };
      const atlasSectorPoint = {
        x: binding.source_frame.x + sample.u * binding.source_frame.width,
        y: binding.source_frame.y + sample.v * binding.source_frame.height,
      };
      const warpedTerrainPoint = applyCssMatrix(
        presentation.css_transform,
        terrainPoint,
      );
      expect(warpedTerrainPoint.x).toBeCloseTo(atlasSectorPoint.x, 6);
      expect(warpedTerrainPoint.y).toBeCloseTo(atlasSectorPoint.y, 6);
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
