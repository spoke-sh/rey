import { describe, expect, it } from "vitest";
import {
  DEFAULT_LENS_ZOOM,
  EVIDENCE_LENS_ZOOM,
  LANDSCAPE_LENS_ZOOM,
  MAX_LENS_ZOOM,
  MIN_LENS_ZOOM,
  NEIGHBORHOOD_LENS_ZOOM,
  OBJECT_LENS_ZOOM,
  WORLD_ATLAS_MORPH_END_ZOOM,
  WORLD_LENS_ZOOM,
  clampLensZoom,
  draggedGlobeView,
  draggedTerrainOrbit,
  fitScaleForViewport,
  lensRegimeForZoom,
  panForFocusedPoint,
  panForScaleAtPoint,
  panForTerrainTarget,
  pointerWithinRenderedGlobeAtmosphere,
  renderedSceneScale,
  recenterWrappedChartPan,
  smoothZoomStep,
  stepLensZoom,
  wheelZoomDelta,
  worldAtlasMorphProgress,
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
    expect(
      panForScaleAtPoint({ x: 0, y: 0 }, { x: 100, y: -50 }, 1, 2),
    ).toEqual({ x: -100, y: 50 });
    expect(
      panForFocusedPoint(
        { x: 900, y: 650 },
        { width: 1500, height: 1000 },
        0.5,
      ),
    ).toEqual({ x: -75, y: -75 });
    expect(
      renderedSceneScale(true, 0.5, DEFAULT_LENS_ZOOM, "atlas"),
    ).toBeCloseTo(0.5 * (DEFAULT_LENS_ZOOM / WORLD_ATLAS_MORPH_END_ZOOM));
  });

  it("changes rendered scale through formerly clamped World intervals while preserving the pointer anchor", () => {
    const currentScale = renderedSceneScale(false, 1, 0.12, "world");
    const nextScale = renderedSceneScale(false, 1, 0.13, "world");
    const pan = { x: 24, y: -12 };
    const pointer = { x: 320, y: -180 };
    const nextPan = panForScaleAtPoint(pan, pointer, currentScale, nextScale);

    expect(nextScale).toBeGreaterThan(currentScale);
    expect(nextPan).not.toEqual(pan);
    expect((pointer.x - nextPan.x) / nextScale).toBeCloseTo(
      (pointer.x - pan.x) / currentScale,
    );
    expect((pointer.y - nextPan.y) / nextScale).toBeCloseTo(
      (pointer.y - pan.y) / currentScale,
    );
  });

  it("keeps the shared surface scale continuous across the Atlas endpoint", () => {
    expect(renderedSceneScale(false, 1, 0.14, "world")).toBeCloseTo(1.16);
    expect(renderedSceneScale(false, 1, 0.14, "atlas")).toBeCloseTo(1.16);
    expect(renderedSceneScale(false, 1, 0.19, "world")).toBeCloseTo(
      renderedSceneScale(false, 1, 0.19, "atlas"),
    );
    expect(renderedSceneScale(false, 1, 0.24, "world")).toBeCloseTo(1);
    expect(renderedSceneScale(false, 1, 0.24, "atlas")).toBeCloseTo(1);
    expect(renderedSceneScale(false, 1, 0.3, "atlas")).toBeGreaterThan(
      renderedSceneScale(false, 1, 0.29, "atlas"),
    );
  });

  it("normalizes wheel hardware and scales continuous input with the lens", () => {
    expect(wheelZoomDelta(0.1, -100)).toBeCloseTo(0.045);
    expect(wheelZoomDelta(0.1, -1, 1)).toBeCloseTo(0.0072);
    expect(wheelZoomDelta(0.1, -1, 2)).toBeCloseTo(0.045);
    expect(wheelZoomDelta(2, -100)).toBeCloseTo(0.18);
    expect(wheelZoomDelta(2, 100)).toBeCloseTo(-0.18);
    expect(wheelZoomDelta(0.1, Number.NaN)).toBe(0);

    const firstTarget = 0.1 + wheelZoomDelta(0.1, -100);
    const accumulatedTarget = firstTarget + wheelZoomDelta(firstTarget, -100);
    expect(firstTarget).toBeCloseTo(0.145);
    expect(accumulatedTarget).toBeCloseTo(0.19);
  });

  it("eases toward accumulated wheel targets without overshooting", () => {
    const target = 0.145;
    const first = smoothZoomStep(0.1, target, 16);
    expect(first).toBeGreaterThan(0.1);
    expect(first).toBeLessThan(target);

    let current = first;
    for (let frame = 0; frame < 30; frame += 1)
      current = smoothZoomStep(current, target, 16);
    expect(current).toBe(target);
    expect(() => smoothZoomStep(0.1, target, Number.NaN)).toThrow(
      "smooth zoom requires finite camera values",
    );
  });

  it("turns missed animation time into several visible zoom steps", () => {
    const target = 0.24;
    const regularFrame = smoothZoomStep(0.14, target, 20);
    const delayedFrame = smoothZoomStep(0.14, target, 320);

    expect(delayedFrame).toBeCloseTo(regularFrame);
    expect(delayedFrame).toBeGreaterThan(0.14);
    expect(delayedFrame).toBeLessThan(0.18);
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

  it("keeps terrain orbit inside its declared pitch and yaw bounds", () => {
    expect(
      draggedTerrainOrbit(
        { yaw_degrees: 170, pitch_degrees: 35 },
        { x: 100, y: -400 },
      ),
    ).toEqual({ yaw_degrees: -172, pitch_degrees: 72 });
    expect(
      draggedTerrainOrbit(
        { yaw_degrees: -170, pitch_degrees: 35 },
        { x: -100, y: 400 },
      ),
    ).toEqual({ yaw_degrees: 172, pitch_degrees: 22 });
  });

  it("solves the bounded terrain target in camera screen axes", () => {
    expect(
      panForTerrainTarget(
        { x: 900, y: 500 },
        { width: 1500, height: 1000 },
        2,
        { pitch_degrees: 90, yaw_degrees: 0 },
      ),
    ).toEqual({ x: -300, y: -0 });
    const isometric = panForTerrainTarget(
      { x: 900, y: 620 },
      { width: 1500, height: 1000 },
      2,
      { pitch_degrees: 35.26439, yaw_degrees: 45 },
    );
    expect(isometric.x).toBeCloseTo(-42.4264, 3);
    expect(isometric.y).toBeCloseTo(-220.454, 2);
  });

  it("partitions World drag between the rendered atmosphere and surrounding canvas", () => {
    const viewport = { width: 1_600, height: 900 };
    const world = { width: 1_200, height: 720 };
    const pan = { x: 100, y: -40 };

    expect(
      pointerWithinRenderedGlobeAtmosphere(
        { x: 900, y: 410 },
        viewport,
        world,
        1,
        pan,
      ),
    ).toBe(true);
    expect(
      pointerWithinRenderedGlobeAtmosphere(
        { x: 1_222, y: 410 },
        viewport,
        world,
        1,
        pan,
      ),
    ).toBe(false);
    expect(
      pointerWithinRenderedGlobeAtmosphere(
        { x: 900, y: 410 },
        viewport,
        world,
        0,
        pan,
      ),
    ).toBe(false);
  });

  it("exposes the grammar's bounded World-to-Atlas morph band", () => {
    expect(worldAtlasMorphProgress(0.1)).toBe(0);
    expect(worldAtlasMorphProgress(0.14)).toBe(0);
    expect(worldAtlasMorphProgress(0.19)).toBeCloseTo(0.5);
    expect(worldAtlasMorphProgress(0.24)).toBe(1);
    expect(worldAtlasMorphProgress(0.4)).toBe(1);
  });

  it("recenters horizontal chart copies without changing vertical pan", () => {
    expect(recenterWrappedChartPan({ x: 1_250, y: -48 }, 1_200)).toEqual({
      x: 50,
      y: -48,
    });
    expect(recenterWrappedChartPan({ x: -1_250, y: 16 }, 1_200)).toEqual({
      x: -50,
      y: 16,
    });
    expect(recenterWrappedChartPan({ x: 600, y: 0 }, 1_200).x).toBe(-600);
  });
});
