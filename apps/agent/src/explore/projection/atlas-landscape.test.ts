import { describe, expect, it } from "vitest";
import {
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
});
