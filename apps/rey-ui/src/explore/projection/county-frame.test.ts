import { describe, expect, it } from "vitest";
import type { AdmittedRegionalScene } from "../../domain";
import {
  compileCountyFrame,
  invertCountyScreen,
  nativeBoundsToCountyLocal,
  projectCountyLocal,
} from "./county-frame";

const scene = {
  scene_id: "scene:one",
  native_bounds: {
    west_microdegrees: -123_000_000,
    south_microdegrees: 37_000_000,
    east_microdegrees: -122_000_000,
    north_microdegrees: 38_000_000,
    crosses_antimeridian: false,
  },
  projection: {
    transforms: [
      {
        transform: {
          id: "rey.scene.native-to-county-local",
          revision: 1,
          semantic_digest: "transform:one",
        },
        source_space: "native_crs84",
        target_space: "county_local",
        source_origin: [-122_500_000, 37_500_000],
        target_origin: [0, 0, 0],
        parameters: ["east_north_up_microunits"],
        inverse_policy: "bounded analytic inverse inside admitted envelope",
        distortion: "presentation only",
      },
    ],
    coordinate_bindings: [
      {
        space: "county_local",
        status: "bound",
        dimensions: ["east", "north", "up"],
        units: ["local_microunit", "local_microunit", "local_microunit"],
      },
    ],
  },
} as unknown as AdmittedRegionalScene;

describe("County-local frame", () => {
  it("binds the exact native envelope origin and round-trips its plane", () => {
    const frame = compileCountyFrame(scene);
    const local = nativeBoundsToCountyLocal(frame, {
      west_microdegrees: -122_800_000,
      south_microdegrees: 37_200_000,
      east_microdegrees: -122_200_000,
      north_microdegrees: 37_800_000,
      crosses_antimeridian: false,
    });
    const view = { center: { x: 600, y: 360 }, scale: 0.0005 };
    const screen = projectCountyLocal(frame, local, view);
    const inverted = invertCountyScreen(frame, screen, view);
    expect(inverted.east).toBeCloseTo(local.east);
    expect(inverted.north).toBeCloseTo(local.north);
    expect(inverted.up).toBe(0);
    expect(frame.authority).toContain("do not reconstruct source footprint");
  });

  it("rejects a tangent origin not bound to the scene envelope", () => {
    const tampered = structuredClone(scene);
    const transform = tampered.projection.transforms[0];
    if (!transform) throw new Error("missing fixture transform");
    transform.source_origin = [transform.source_origin[0]! + 1, 37_500_000];
    expect(() => compileCountyFrame(tampered)).toThrow("exact County-local");
  });
});
