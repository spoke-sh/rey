import { describe, expect, it } from "vitest";
import { contextGlobeSamples } from "./globe-samples";

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
});
