import { describe, expect, it } from "vitest";
import { OrthographicCamera, Vector3 } from "three/src/Three.WebGPU.js";
import { globeAtlasViewCenter } from "@rey/explorer/globe-projection";
import {
  invertSemanticMercator,
  invertViewAlignedSemanticMercator,
  pickSemanticMercator,
  projectSemanticGlobe,
  projectSemanticMercator,
  projectSemanticMercatorBounds,
  projectViewAlignedSemanticMercator,
  projectWorldAtlasBoundsMorph,
  projectWorldAtlasMorph,
  SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES,
  wrapSemanticLongitude,
} from "./semantic-mercator";

const frame = { x: 70, y: 55, width: 1060, height: 610 };

describe("semantic Mercator projection", () => {
  it("matches a real orbit camera's screen-relative pitch, verified against Three.js's own matrix math", () => {
    // projectSemanticGlobe's pitch/yaw composition mirrors
    // @rey/explorer's globeCameraPose (a real camera tilting about a fixed
    // screen-horizontal axis, applied after content yaw) rather than
    // reproducing it independently by formula alone — this builds an
    // actual Three.js camera at the equivalent pose and confirms the 2D
    // reference projection's x/y agree with what that camera would
    // actually see, for a spread of longitude/latitude/yaw/pitch
    // combinations, not just one hand-checked case.
    const center = { x: 0, y: 0 };
    const radius = 1;
    for (const { longitude, latitude } of [
      { longitude: 30, latitude: 20 },
      { longitude: -70, latitude: -40 },
      { longitude: 170, latitude: 60 },
      { longitude: 0, latitude: 0 },
      { longitude: 120, latitude: -10 },
    ]) {
      const longitudeRadians = (longitude * Math.PI) / 180;
      const latitudeRadians = (latitude * Math.PI) / 180;
      const localX = Math.cos(latitudeRadians) * Math.sin(longitudeRadians);
      const localY = Math.sin(latitudeRadians);
      const localZ = Math.cos(latitudeRadians) * Math.cos(longitudeRadians);
      for (const yawDegrees of [0, 45, -30, 120]) {
        for (const pitchDegrees of [0, -24, 40, -55]) {
          const projected = projectSemanticGlobe(
            {
              longitude_microdegrees: longitude * 1_000_000,
              latitude_microdegrees: latitude * 1_000_000,
            },
            center,
            radius,
            { yaw_degrees: yawDegrees, pitch_degrees: pitchDegrees },
          );

          const yaw = (yawDegrees * Math.PI) / 180;
          const rotatedX = localX * Math.cos(yaw) + localZ * Math.sin(yaw);
          const rotatedZ = -localX * Math.sin(yaw) + localZ * Math.cos(yaw);
          const worldPoint = new Vector3(rotatedX, localY, rotatedZ);

          const pitch = (pitchDegrees * Math.PI) / 180;
          const distance = 6;
          const camera = new OrthographicCamera();
          camera.position.set(
            0,
            distance * Math.sin(pitch),
            distance * Math.cos(pitch),
          );
          camera.rotation.set(-pitch, 0, 0);
          camera.updateMatrixWorld(true);
          const local = worldPoint
            .clone()
            .applyMatrix4(camera.matrixWorldInverse);

          // projectSemanticGlobe negates y deliberately (screen coordinates
          // grow downward; Three.js world coordinates grow upward).
          expect(projected.x).toBeCloseTo(center.x + local.x, 9);
          expect(projected.y).toBeCloseTo(center.y - local.y, 9);
        }
      }
    }
  });

  it("marks a point facing the camera as visible and its antipode as not", () => {
    // At longitude 0, latitude == pitch_degrees is exactly the point whose
    // outward normal points straight at the tilted camera (localY=sin(pitch),
    // localZ=cos(pitch) with localX=0) — its antipode (latitude negated,
    // longitude flipped 180) points straight away from it.
    const pitchDegrees = -24;
    const view = { yaw_degrees: 0, pitch_degrees: pitchDegrees };
    const facingCamera = projectSemanticGlobe(
      {
        longitude_microdegrees: 0,
        latitude_microdegrees: pitchDegrees * 1_000_000,
      },
      { x: 0, y: 0 },
      1,
      view,
    );
    expect(facingCamera.depth).toBeCloseTo(1, 9);
    expect(facingCamera.visible).toBe(true);

    const facingAway = projectSemanticGlobe(
      {
        longitude_microdegrees: 180_000_000,
        latitude_microdegrees: -pitchDegrees * 1_000_000,
      },
      { x: 0, y: 0 },
      1,
      view,
    );
    expect(facingAway.depth).toBeCloseTo(-1, 9);
    expect(facingAway.visible).toBe(false);
  });

  it("wraps chart copies and inverts them to one canonical coordinate", () => {
    expect(wrapSemanticLongitude(181_000_000)).toBe(-179_000_000);
    const source = {
      longitude_microdegrees: -42_000_000,
      latitude_microdegrees: 18_000_000,
    };
    const left = projectSemanticMercator(source, frame);
    const repeated = projectSemanticMercator(source, frame, 1);
    expect(repeated.x - left.x).toBe(frame.width);
    const inverse = invertSemanticMercator(repeated, frame);
    expect(inverse.wrap_index).toBe(1);
    expect(inverse.coordinate.longitude_microdegrees).toBe(-42_000_000);
    expect(inverse.coordinate.latitude_microdegrees).toBeCloseTo(
      18_000_000,
      -1,
    );
  });

  it("clips both polar caps and discloses rather than dropping them", () => {
    const north = projectSemanticMercator(
      {
        longitude_microdegrees: 10_000_000,
        latitude_microdegrees: 87_000_000,
      },
      frame,
    );
    expect(north.latitude_microdegrees).toBe(
      SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES,
    );
    expect(north.polar_disclosure).toBe("north_cap");
    expect(north.y).toBeCloseTo(frame.y, 5);

    const fragments = projectSemanticMercatorBounds(
      "sector:north",
      {
        west_microdegrees: 0,
        south_microdegrees: 60_000_000,
        east_microdegrees: 30_000_000,
        north_microdegrees: 90_000_000,
        crosses_antimeridian: false,
      },
      frame,
    );
    expect(fragments[0]?.polar_disclosures).toEqual(["north_cap"]);
    expect(fragments[0]?.height).toBeGreaterThan(0);
  });

  it("splits antimeridian drawing without splitting semantic identity", () => {
    const fragments = projectSemanticMercatorBounds(
      "county:one",
      {
        west_microdegrees: 179_000_000,
        south_microdegrees: -1_000_000,
        east_microdegrees: -179_000_000,
        north_microdegrees: 1_000_000,
        crosses_antimeridian: true,
      },
      frame,
    );
    expect(fragments).toHaveLength(2);
    expect(new Set(fragments.map(({ identity }) => identity))).toEqual(
      new Set(["county:one"]),
    );
    expect(new Set(fragments.map(({ fragment_id }) => fragment_id)).size).toBe(
      2,
    );
    expect(fragments[0]?.x).toBeGreaterThan(frame.x + frame.width * 0.99);
    expect(fragments[1]?.x).toBe(frame.x);
  });

  it("keeps identity and focus stable across the World-to-Atlas morph", () => {
    const coordinate = {
      longitude_microdegrees: -42_000_000,
      latitude_microdegrees: 18_000_000,
    };
    const worldFrame = { center: { x: 600, y: 360 }, radius: 295.2 };
    const world = projectWorldAtlasMorph(
      "atlas-region:1",
      "regional:scene:1",
      coordinate,
      worldFrame,
      frame,
      { yaw_degrees: 24, pitch_degrees: -8 },
      0,
    );
    const atlas = projectWorldAtlasMorph(
      "atlas-region:1",
      "regional:scene:1",
      coordinate,
      worldFrame,
      frame,
      { yaw_degrees: 24, pitch_degrees: -8 },
      1,
    );
    const middle = projectWorldAtlasMorph(
      "atlas-region:1",
      "regional:scene:1",
      coordinate,
      worldFrame,
      frame,
      { yaw_degrees: 24, pitch_degrees: -8 },
      0.5,
    );
    expect([world, middle, atlas].map(({ identity }) => identity)).toEqual([
      "atlas-region:1",
      "atlas-region:1",
      "atlas-region:1",
    ]);
    expect([world, middle, atlas].map(({ focus_id }) => focus_id)).toEqual([
      "regional:scene:1",
      "regional:scene:1",
      "regional:scene:1",
    ]);
    expect(world).toMatchObject({ x: world.world.x, y: world.world.y });
    expect(atlas).toMatchObject({ x: atlas.atlas.x, y: atlas.atlas.y });
    expect(middle.x).toBeCloseTo((world.x + atlas.x) / 2);
    expect(middle.y).toBeCloseTo((world.y + atlas.y) / 2);
  });

  it("keeps the rotated globe's Atlas-aligned center fixed at the flat endpoint", () => {
    const view = { yaw_degrees: 58, pitch_degrees: -24 };
    const viewCenter = globeAtlasViewCenter(view);
    const coordinate = {
      longitude_microdegrees: Math.round(
        viewCenter.longitude_degrees * 1_000_000,
      ),
      latitude_microdegrees: Math.round(
        viewCenter.latitude_degrees * 1_000_000,
      ),
    };
    const center = projectViewAlignedSemanticMercator(coordinate, frame, view);
    expect(center.x).toBeCloseTo(frame.x + frame.width / 2, 3);
    expect(center.y).toBeCloseTo(frame.y + frame.height / 2, 3);
    expect(
      invertViewAlignedSemanticMercator(center, frame, view).coordinate,
    ).toEqual(coordinate);
    // The flat Atlas chart doesn't auto-straighten pitch here (unaffected by
    // this change — it always uses the raw view center, unlike
    // @rey/explorer's globeAtlasProjectionCenter), so this coordinate stays
    // exactly centered at the flat endpoint regardless of progress-scaling.
    const atlasEndpoint = projectWorldAtlasMorph(
      "region:center",
      "regional:center",
      coordinate,
      { center: { x: 600, y: 360 }, radius: 295.2 },
      frame,
      view,
      1,
    );
    expect(atlasEndpoint.x).toBeCloseTo(600, 3);
    expect(atlasEndpoint.y).toBeCloseTo(360, 3);
  });

  it("keeps the rotated globe's own screen-relative bearing centered at the World endpoint", () => {
    // Mirrors the World-side coordinate that faces a screen-relative-pitch
    // camera (see projectSemanticGlobe's own composition and
    // globeCameraPose in @rey/explorer) — a different coordinate than the
    // flat Atlas chart's own (unrelated, raw-pitch) view center, since the
    // two sides now use genuinely different pitch conventions. Verified
    // directly: this is exactly the same closed-form inversion checked
    // against Three.js's own camera math in the earlier "matches a real
    // orbit camera" test above.
    const view = { yaw_degrees: 58, pitch_degrees: -24 };
    const yaw = (view.yaw_degrees * Math.PI) / 180;
    const pitch = (view.pitch_degrees * Math.PI) / 180;
    const worldViewCenter = {
      longitude_degrees:
        (Math.atan2(
          -Math.cos(pitch) * Math.sin(yaw),
          Math.cos(pitch) * Math.cos(yaw),
        ) *
          180) /
        Math.PI,
      latitude_degrees: (Math.asin(Math.sin(pitch)) * 180) / Math.PI,
    };
    const coordinate = {
      longitude_microdegrees: Math.round(
        worldViewCenter.longitude_degrees * 1_000_000,
      ),
      latitude_microdegrees: Math.round(
        worldViewCenter.latitude_degrees * 1_000_000,
      ),
    };
    const worldEndpoint = projectWorldAtlasMorph(
      "region:center",
      "regional:center",
      coordinate,
      { center: { x: 600, y: 360 }, radius: 295.2 },
      frame,
      view,
      0,
    );
    expect(worldEndpoint.x).toBeCloseTo(600, 3);
    expect(worldEndpoint.y).toBeCloseTo(360, 3);
  });

  it("splits sectors at the view-relative seam instead of twisting them", () => {
    const fragments = projectWorldAtlasBoundsMorph(
      "sector:view-seam",
      {
        west_microdegrees: 115_000_000,
        south_microdegrees: -10_000_000,
        east_microdegrees: 145_000_000,
        north_microdegrees: 10_000_000,
        crosses_antimeridian: false,
      },
      { center: { x: 600, y: 360 }, radius: 295.2 },
      frame,
      { yaw_degrees: 58, pitch_degrees: -24 },
      0.5,
    );
    expect(fragments).toHaveLength(2);
    expect(
      fragments.every(({ identity }) => identity === "sector:view-seam"),
    ).toBe(true);
    for (const fragment of fragments) {
      const atlasWidth = Math.abs(
        fragment.points[1]!.atlas.x - fragment.points[0]!.atlas.x,
      );
      expect(atlasWidth).toBeLessThan(frame.width / 2);
    }
  });

  it("morphs antimeridian fragments through shared semantic identity", () => {
    const fragments = projectWorldAtlasBoundsMorph(
      "sector:wrap",
      {
        west_microdegrees: 179_000_000,
        south_microdegrees: -1_000_000,
        east_microdegrees: -179_000_000,
        north_microdegrees: 1_000_000,
        crosses_antimeridian: true,
      },
      { center: { x: 600, y: 360 }, radius: 295.2 },
      frame,
      { yaw_degrees: 0, pitch_degrees: 0 },
      1,
    );
    expect(fragments).toHaveLength(2);
    expect(fragments.every(({ identity }) => identity === "sector:wrap")).toBe(
      true,
    );
    expect(fragments[0]?.points[1]?.x).toBeCloseTo(frame.x + frame.width);
    expect(fragments[1]?.points[0]?.x).toBeCloseTo(frame.x);
  });

  it("inverse-picks every chart copy back to one retained identity", () => {
    const coordinate = {
      longitude_microdegrees: -42_000_000,
      latitude_microdegrees: 18_000_000,
    };
    const candidates = [
      {
        identity: "atlas-region:1",
        focus_id: "regional:scene:1",
        coordinate,
      },
    ];
    for (const wrapIndex of [-1, 0, 1]) {
      const projected = projectSemanticMercator(coordinate, frame, wrapIndex);
      expect(pickSemanticMercator(projected, candidates, frame)).toMatchObject({
        identity: "atlas-region:1",
        focus_id: "regional:scene:1",
        coordinate,
        inverse_coordinate: coordinate,
        wrap_index: wrapIndex,
        distance: 0,
      });
    }
    expect(
      pickSemanticMercator(
        { x: frame.x + 10, y: frame.y + 10 },
        candidates,
        frame,
      ),
    ).toBeNull();
  });
});
