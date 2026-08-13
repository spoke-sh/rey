import { describe, expect, it } from "vitest";
import {
  invertSemanticMercator,
  projectSemanticMercator,
  projectSemanticMercatorBounds,
  projectWorldAtlasMorph,
  SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES,
  wrapSemanticLongitude,
} from "./semantic-mercator";

const frame = { x: 70, y: 55, width: 1060, height: 610 };

describe("semantic Mercator projection", () => {
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
});
