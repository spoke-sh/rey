import { describe, expect, it } from "vitest";
import {
  DEFAULT_LENS_ZOOM,
  EVIDENCE_LENS_ZOOM,
  LANDSCAPE_LENS_ZOOM,
  MAX_LENS_ZOOM,
  MIN_LENS_ZOOM,
  NEIGHBORHOOD_LENS_ZOOM,
  OBJECT_LENS_ZOOM,
  WORLD_LENS_ZOOM,
  clampLensZoom,
  draggedGlobeView,
  fitScaleForViewport,
  lensRegimeForZoom,
  panForFocusedPoint,
  panForZoomAtPoint,
  renderedSceneScale,
  stepLensZoom,
} from "./camera";

describe("Explorer camera engine", () => {
  it("moves through every semantic regime with hysteresis", () => {
    expect(lensRegimeForZoom(WORLD_LENS_ZOOM)).toBe("world");
    expect(lensRegimeForZoom(DEFAULT_LENS_ZOOM)).toBe("atlas");
    expect(lensRegimeForZoom(LANDSCAPE_LENS_ZOOM)).toBe("landscape");
    expect(lensRegimeForZoom(NEIGHBORHOOD_LENS_ZOOM)).toBe("neighborhoods");
    expect(lensRegimeForZoom(OBJECT_LENS_ZOOM)).toBe("objects");
    expect(lensRegimeForZoom(EVIDENCE_LENS_ZOOM)).toBe("evidence");
    expect(lensRegimeForZoom(0.43, "atlas")).toBe("atlas");
    expect(lensRegimeForZoom(0.48, "atlas")).toBe("landscape");
  });

  it("clamps and steps without skipping a semantic level", () => {
    expect(clampLensZoom(-1)).toBe(MIN_LENS_ZOOM);
    expect(clampLensZoom(20)).toBe(MAX_LENS_ZOOM);
    expect(stepLensZoom(DEFAULT_LENS_ZOOM, -1)).toBe(WORLD_LENS_ZOOM);
    expect(stepLensZoom(DEFAULT_LENS_ZOOM, 1)).toBe(LANDSCAPE_LENS_ZOOM);
    expect(stepLensZoom(EVIDENCE_LENS_ZOOM, 1)).toBe(MAX_LENS_ZOOM);
  });

  it("fits, zooms around the pointer, and focuses in world coordinates", () => {
    expect(
      fitScaleForViewport(
        { width: 1536, height: 1036 },
        { width: 1500, height: 1000 },
      ),
    ).toBe(1);
    expect(panForZoomAtPoint({ x: 0, y: 0 }, { x: 100, y: -50 }, 1, 2)).toEqual(
      { x: -100, y: 50 },
    );
    expect(
      panForFocusedPoint(
        { x: 900, y: 650 },
        { width: 1500, height: 1000 },
        0.5,
      ),
    ).toEqual({ x: -75, y: -75 });
    expect(renderedSceneScale(true, 0.5, DEFAULT_LENS_ZOOM, "atlas")).toBe(0.5);
  });

  it("turns planar drag into bounded presentation-only globe rotation", () => {
    expect(
      draggedGlobeView(
        { yaw_degrees: 4, pitch_degrees: -2 },
        { x: 100, y: -50 },
      ),
    ).toEqual({ yaw_degrees: 26, pitch_degrees: 7 });
    expect(
      draggedGlobeView({ yaw_degrees: 0, pitch_degrees: 58 }, { x: 0, y: -100 })
        .pitch_degrees,
    ).toBe(62);
  });
});
