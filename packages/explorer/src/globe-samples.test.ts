import { describe, expect, it } from "vitest";
import { contextGlobePolePatterns, contextGlobeSamples } from "./globe-samples";

describe("context globe samples", () => {
  it("materializes a dense deterministic spherical presentation field", () => {
    const first = contextGlobeSamples("blake3:orientation", 26_000);
    const replay = contextGlobeSamples("blake3:orientation", 26_000);

    expect(first.length).toBeGreaterThan(14_000);
    expect(first).toEqual(replay);
    expect(first.every((sample) => sample.latitude_degrees >= -90)).toBe(true);
    expect(first.every((sample) => sample.latitude_degrees <= 90)).toBe(true);
    expect(first.every((sample) => sample.longitude_degrees >= -180)).toBe(
      true,
    );
    expect(first.every((sample) => sample.longitude_degrees <= 180)).toBe(true);
  });

  it("only uses admitted region coordinates to emphasize frozen fabric", () => {
    const plain = contextGlobeSamples("atlas:one", 3_000);
    const emphasized = contextGlobeSamples("atlas:one", 3_000, [
      {
        longitude_degrees: 12,
        latitude_degrees: 8,
        angular_radius_degrees: 7,
      },
    ]);

    expect(emphasized.length).toBeGreaterThanOrEqual(plain.length);
    expect(emphasized).not.toEqual(plain);
  });

  it("builds deterministic stipple caps at both exact poles", () => {
    const patterns = contextGlobePolePatterns();

    expect(patterns.map(({ pole }) => pole)).toEqual(["north", "south"]);
    expect(patterns.every(({ samples }) => samples.length === 34)).toBe(true);
    expect(patterns[0]?.samples[0]).toMatchObject({
      latitude_degrees: 90,
      longitude_degrees: 0,
    });
    expect(patterns[1]?.samples[0]).toMatchObject({
      latitude_degrees: -90,
      longitude_degrees: 0,
    });
    expect(
      patterns.every(({ samples }) =>
        samples.every(
          ({ latitude_degrees }) => Math.abs(latitude_degrees) >= 75,
        ),
      ),
    ).toBe(true);
    expect(contextGlobePolePatterns()).toEqual(patterns);
  });
});
