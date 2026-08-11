import { describe, expect, it } from "vitest";
import { boundedViewport } from "./renderer";

describe("renderer contracts", () => {
  it("bounds physical viewport work independently of CSS size", () => {
    expect(
      boundedViewport({
        width: 1920.9,
        height: 1080.8,
        device_pixel_ratio: 3,
      }),
    ).toEqual({ width: 1920, height: 1080, device_pixel_ratio: 2 });
    expect(
      boundedViewport({ width: 0, height: -10, device_pixel_ratio: 0.5 }),
    ).toEqual({ width: 1, height: 1, device_pixel_ratio: 1 });
  });
});
