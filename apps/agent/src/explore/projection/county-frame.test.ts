import { describe, expect, it } from "vitest";
import type { AdmittedRegionalScene } from "../../domain";
import {
  compileCountyFootprint,
  compileCountyFrame,
  countyLocalToNativePosition,
  invertCountyScreen,
  nativePositionToCountyLocal,
  nativeBoundsToCountyLocal,
  projectCountyFootprint,
  projectCountyLocal,
  regionalBoundsCenter,
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
    limits: { max_native_coordinates: 100 },
    objects: [
      {
        object_id: "boundary",
        source_artifact_id: "artifact:one",
        object_revision: "object:one",
        geometry_kind: "Polygon",
        layer: "boundary",
        native_bounds: {
          west_microdegrees: -123_000_000,
          south_microdegrees: 37_000_000,
          east_microdegrees: -122_000_000,
          north_microdegrees: 38_000_000,
          crosses_antimeridian: false,
        },
      },
    ],
    footprint: {
      footprint_id: "footprint:one",
      source_object_id: "boundary",
      source_artifact_id: "artifact:one",
      source_object_revision: "object:one",
      geometry_kind: "Polygon",
      native_bounds: {
        west_microdegrees: -123_000_000,
        south_microdegrees: 37_000_000,
        east_microdegrees: -122_000_000,
        north_microdegrees: 38_000_000,
        crosses_antimeridian: false,
      },
      rings: [
        [
          [-123_000_000, 37_000_000],
          [-122_000_000, 37_000_000],
          [-122_000_000, 38_000_000],
          [-123_000_000, 38_000_000],
          [-123_000_000, 37_000_000],
        ],
      ],
      coordinate_count: 5,
      authority:
        "exact admitted native boundary polygon; footprint validity ends at its rings",
    },
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
  it("matches Rust integer division for odd microdegree spans", () => {
    expect(
      regionalBoundsCenter({
        west_microdegrees: -122_750_000,
        south_microdegrees: 37_250_000,
        east_microdegrees: -122_250_000,
        north_microdegrees: 37_784_437,
        crosses_antimeridian: false,
      }),
    ).toEqual([-122_500_000, 37_517_218]);
    expect(
      regionalBoundsCenter({
        west_microdegrees: -180_000_000,
        south_microdegrees: -5,
        east_microdegrees: -179_999_999,
        north_microdegrees: 0,
        crosses_antimeridian: false,
      }),
    ).toEqual([-179_999_999, -2]);
  });

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
    const native = [-122_200_000, 37_800_000] as const;
    expect(
      countyLocalToNativePosition(
        frame,
        nativePositionToCountyLocal(frame, native),
      ),
    ).toEqual(native);
  });

  it("rejects a tangent origin not bound to the scene envelope", () => {
    const tampered = structuredClone(scene);
    const transform = tampered.projection.transforms[0];
    if (!transform) throw new Error("missing fixture transform");
    transform.source_origin = [transform.source_origin[0]! + 1, 37_500_000];
    expect(() => compileCountyFrame(tampered)).toThrow("exact County-local");
  });

  it("projects exact footprint rings and rejects envelope substitution", () => {
    const frame = compileCountyFrame(scene);
    const footprint = compileCountyFootprint(scene);
    expect(footprint).not.toBeNull();
    const projected = projectCountyFootprint(frame, footprint!, {
      center: { x: 600, y: 360 },
      scale: 0.0004,
    });
    expect(projected.path).toContain("M");
    expect(projected.path).toContain("Z");
    expect(projected.screen_rings[0]).toHaveLength(5);

    const tampered = structuredClone(scene);
    tampered.projection.footprint!.rings[0]![1]![0] += 1;
    expect(() => compileCountyFootprint(tampered)).toThrow(
      "County footprint is invalid",
    );
  });
});
