import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import {
  ATLAS_LANDSCAPE_MOVING_TERRAIN_MAXIMUM_LEVEL,
  ATLAS_LANDSCAPE_MOVING_TERRAIN_RESOLUTION_SCALE,
  ATLAS_LANDSCAPE_SETTLED_REFINEMENT_DELAY_MS,
  atlasLandscapeLiveTerrainView,
  atlasTerrainMovingCompilationView,
  atlasTerrainPredictedEntryView,
  atlasTerrainPrewarmDelayMs,
  atlasTerrainPrewarmStatus,
  CanvasFooter,
  CanvasToolbar,
  DEFAULT_EXPLORER_FOOTER_MINIMUM_VISIBLE_MS,
  explorerFooterReducer,
  explorerGeographicCoordinate,
  explorerRegimeNotice,
  initialExplorerFooterState,
  shouldMountTerrainSurface,
} from "./explore";
import type { TopologyScene } from "./topology";
import { atlasLandscapePresentation } from "./explore/projection/atlas-landscape";

describe("Explorer canvas toolbar", () => {
  it("reports the bounded Atlas terrain prewarm lifecycle", () => {
    expect(atlasTerrainPrewarmStatus(false, false, false)).toBe("unavailable");
    expect(atlasTerrainPrewarmStatus(true, false, false)).toBe("scheduled");
    expect(atlasTerrainPrewarmStatus(true, true, false)).toBe("mounted");
    expect(atlasTerrainPrewarmStatus(true, true, true)).toBe("prepared");
    expect(atlasTerrainPrewarmDelayMs("webgpu")).toBe(1_500);
    expect(atlasTerrainPrewarmDelayMs("webgl2")).toBe(600);
  });

  it("prewarms admitted Atlas terrain without presenting it as a globe", () => {
    expect(
      shouldMountTerrainSurface({
        atlas_landscape_transition: null,
        globe: null,
        terrain: false,
        terrain_fields: [{ field_set_id: "terrain:admitted" }],
      } as unknown as TopologyScene),
    ).toBe(false);
    expect(
      shouldMountTerrainSurface(
        {
          atlas_landscape_transition: null,
          globe: null,
          terrain: false,
          terrain_fields: [{ field_set_id: "terrain:admitted" }],
        } as unknown as TopologyScene,
        true,
      ),
    ).toBe(true);
    expect(
      shouldMountTerrainSurface({
        atlas_landscape_transition: null,
        globe: { posture: "regional_scenes" },
        terrain: false,
        terrain_fields: [],
      } as unknown as TopologyScene),
    ).toBe(false);
  });

  it("prewarms the exact predicted Landscape entry camera", () => {
    const scene = {
      world: { width: 1500, height: 1000 },
      terrain_fields: [
        {
          grid: {
            bounds: { x: 180, y: 140, width: 720, height: 520 },
          },
        },
      ],
    } as unknown as TopologyScene;
    const first = atlasTerrainPredictedEntryView(
      scene,
      0.8,
      { width: 1920, height: 1080 },
      { pitch_degrees: 88, yaw_degrees: 0 },
    );
    const replay = atlasTerrainPredictedEntryView(
      scene,
      0.8,
      { width: 1920, height: 1080 },
      { pitch_degrees: 88, yaw_degrees: 0 },
    );
    expect(replay).toEqual(first);
    expect(first).toMatchObject({
      viewport_width: 1920,
      viewport_height: 1080,
      pan_x: 0,
      pan_y: 0,
      yaw_degrees: 0,
    });
    expect(first.rendered_scale).toBeGreaterThan(0.8);
    expect(first.pitch_degrees).toBeLessThan(90);
    expect(
      atlasTerrainMovingCompilationView(
        scene,
        0.8,
        { width: 1920, height: 1080 },
        { pitch_degrees: 88, yaw_degrees: 0 },
      ),
    ).toMatchObject({
      rendered_scale:
        first.rendered_scale * ATLAS_LANDSCAPE_MOVING_TERRAIN_RESOLUTION_SCALE,
      viewport_width: 960,
      viewport_height: 540,
      pitch_degrees: first.pitch_degrees,
      yaw_degrees: first.yaw_degrees,
    });
    expect(ATLAS_LANDSCAPE_MOVING_TERRAIN_MAXIMUM_LEVEL).toBe(6);
    expect(ATLAS_LANDSCAPE_SETTLED_REFINEMENT_DELAY_MS).toBe(300);
  });

  it("presents moving terrain with the live County border camera", () => {
    const scene = {
      regime: "landscape",
      terrain: true,
      county_frame: { frame_id: "county:one" },
      world: { width: 1500, height: 1000 },
    } as unknown as TopologyScene;
    const presentation = atlasLandscapePresentation(
      {
        source_frame: { x: 420, y: 360, width: 180, height: 120 },
        target_frame: { x: 96, y: 72, width: 1008, height: 576 },
      },
      0.48,
      { pitch_degrees: 82, yaw_degrees: 17 },
      scene.world,
    );
    const view = atlasLandscapeLiveTerrainView(
      scene,
      0.72,
      0.5,
      { width: 1720, height: 760 },
      { x: 183, y: -74 },
      presentation,
    );
    expect(view).toMatchObject({
      viewport_width: 1720,
      viewport_height: 760,
      pan_x: 183,
      pan_y: -74,
      pitch_degrees: presentation.pitch_degrees,
      yaw_degrees: presentation.yaw_degrees,
      model_transform: presentation.model_transform,
    });
    expect(view.rendered_scale).toBeGreaterThan(0);
    expect(view.model_transform).not.toEqual({
      scale_x: 1,
      scale_z: 1,
      translate_x: 0,
      translate_z: 0,
      elevation_scale: 1,
    });
  });

  it("keeps view controls without exposing projection layer buttons", () => {
    const markup = renderToStaticMarkup(
      createElement(CanvasToolbar, {
        isFullscreen: false,
        onFit: vi.fn(),
        onFullscreen: vi.fn(),
        onZoomIn: vi.fn(),
        onZoomOut: vi.fn(),
        scene: {
          detail: "Bounded regional evidence",
          globe: null,
          label: "REGIONAL EVIDENCE WORLD",
          regime: "world",
        } as unknown as TopologyScene,
        zoom: 0.1,
      }),
    );

    expect(markup).not.toContain("CONTOURS");
    expect(markup).not.toContain("WATER");
    expect(markup).not.toContain("WEATHER");
    expect(markup).not.toContain("PROBES");
    expect(markup).not.toContain("Bounded regional evidence");
    expect(markup).toContain("Zoom out one semantic level");
    expect(markup).toContain("Zoom in one semantic level");
    expect(markup).toContain("FIT");
    expect(markup).toContain("FULL SCREEN");
  });

  it("keeps interaction guidance without rendering scene omissions", () => {
    const initialState = initialExplorerFooterState();
    const markup = renderToStaticMarkup(
      createElement(CanvasFooter, {
        notice: initialState.notice,
        scene: {
          globe: { posture: "orientation" },
          omissions: [
            "regional atlas members retain exact admitted synthetic placement points; sector membership grants no footprint radius",
            "candidate terrain controls and generated effects cannot become observed height without a separately qualified terrain adapter",
          ],
        } as TopologyScene,
      }),
    );

    expect(markup).toContain("WHEEL / + − TO CHANGE LENS");
    expect(markup).toContain("DRAG TO ORBIT");
    expect(markup).toContain("SELECT TO TRAVERSE");
    expect(markup).toContain('aria-live="polite"');
    expect(markup).toContain('data-visible="true"');
    expect(markup).toContain('data-minimum-visible-ms="5000"');
    expect(markup).toContain('data-notice-phase="visible"');
    expect(markup).toContain('data-notice-tone="guide"');
    expect(markup).not.toContain("BOUNDED /");
    expect(markup).not.toContain("regional atlas members");
    expect(markup).not.toContain("candidate terrain controls");
  });

  it("holds notices for five seconds before queued or automatic dismissal", () => {
    const initial = initialExplorerFooterState(1_000);
    const interacting = explorerFooterReducer(initial, {
      type: "interact",
      occurred_at_ms: 2_000,
    });

    expect(interacting.has_interacted).toBe(true);
    expect(interacting.notice).toMatchObject({
      dismiss_requested: true,
      minimum_visible_ms: 5_000,
      phase: "visible",
      published_at_ms: 1_000,
    });

    const earlyExpiration = explorerFooterReducer(interacting, {
      type: "expire",
      notice_id: interacting.notice!.id,
      occurred_at_ms: 5_999,
    });
    expect(earlyExpiration).toBe(interacting);

    const exiting = explorerFooterReducer(interacting, {
      type: "expire",
      notice_id: interacting.notice!.id,
      occurred_at_ms: 6_000,
    });
    expect(exiting.notice?.phase).toBe("exiting");

    const dismissed = explorerFooterReducer(exiting, {
      type: "clear",
      notice_id: exiting.notice!.id,
    });
    expect(dismissed.notice).toBeNull();

    const updated = explorerFooterReducer(dismissed, {
      type: "publish",
      published_at_ms: 10_000,
      message: "LENS / ATLAS · REGIONAL EVIDENCE ATLAS",
      minimum_visible_ms: DEFAULT_EXPLORER_FOOTER_MINIMUM_VISIBLE_MS,
      tone: "update",
      auto_hide_ms: 5_000,
    });
    expect(updated.notice).toMatchObject({
      id: "explorer-notice:1",
      dismiss_requested: false,
      message: "LENS / ATLAS · REGIONAL EVIDENCE ATLAS",
      minimum_visible_ms: 5_000,
      phase: "visible",
      published_at_ms: 10_000,
      tone: "update",
    });
    expect(
      explorerFooterReducer(updated, {
        type: "expire",
        notice_id: "a-stale-notice",
        occurred_at_ms: 15_000,
      }),
    ).toBe(updated);
    const matureInteraction = explorerFooterReducer(updated, {
      type: "interact",
      occurred_at_ms: 15_000,
    });
    expect(matureInteraction.notice).toMatchObject({
      dismiss_requested: false,
      phase: "exiting",
    });

    const exitingMarkup = renderToStaticMarkup(
      createElement(CanvasFooter, {
        notice: matureInteraction.notice,
        scene: { globe: null } as TopologyScene,
      }),
    );
    expect(exitingMarkup).toContain('data-visible="false"');
    expect(exitingMarkup).toContain('data-notice-phase="exiting"');
    expect(exitingMarkup).toContain("LENS / ATLAS · REGIONAL EVIDENCE ATLAS");
  });

  it("uses the exact regional globe caption when World enters", () => {
    expect(
      explorerRegimeNotice({
        globe: {
          beacons: [],
          posture: "regional_scenes",
          source_revision: "blake3:0123456789abcdef",
        },
        regime: "world",
      } as unknown as TopologyScene),
    ).toBe("REGIONAL WORLD / REV blake3:01234");
  });

  it("renders a quiet footer without retaining hidden copy", () => {
    const markup = renderToStaticMarkup(
      createElement(CanvasFooter, {
        notice: null,
        scene: { globe: null } as TopologyScene,
      }),
    );

    expect(markup).toContain('data-visible="false"');
    expect(markup).toContain('data-notice-phase="quiet"');
    expect(markup).toContain('data-notice-tone="quiet"');
    expect(markup).not.toContain("WHEEL /");
    expect(markup).not.toContain("LENS /");
    expect(markup).not.toContain("RENDERER DEGRADED");
  });

  it("reports bound geographic camera coordinates without relabeling local X/Y", () => {
    expect(
      explorerGeographicCoordinate(
        {
          globe: { posture: "orientation" },
          regime: "world",
          world: { width: 1_200, height: 720 },
        } as TopologyScene,
        { x: 0, y: 0 },
        1,
        { yaw_degrees: 219, pitch_degrees: 12 },
      ),
    ).toEqual({
      authority: "globe_view",
      latitude_degrees: 12,
      longitude_degrees: -141,
    });

    expect(
      explorerGeographicCoordinate(
        {
          county_frame: null,
          globe: null,
          regime: "atlas",
          world: { width: 1_200, height: 720 },
        } as TopologyScene,
        { x: 0, y: 0 },
        1,
        { yaw_degrees: 0, pitch_degrees: 0 },
      ),
    ).toMatchObject({
      authority: "semantic_mercator",
      latitude_degrees: 0,
      longitude_degrees: 0,
    });

    expect(
      explorerGeographicCoordinate(
        {
          county_frame: null,
          globe: null,
          regime: "atlas",
          world: { width: 1_200, height: 720 },
          world_atlas_transition: {
            atlas_frame: { x: 9, y: 5.4, width: 1_182, height: 709.2 },
          },
        } as TopologyScene,
        { x: 591, y: 0 },
        1,
        { yaw_degrees: 0, pitch_degrees: 0 },
      ),
    ).toMatchObject({
      authority: "semantic_mercator",
      longitude_degrees: -180,
    });

    expect(
      explorerGeographicCoordinate(
        {
          county_frame: {
            source_bounds: {
              west_microdegrees: -123_000_000,
              south_microdegrees: 37_000_000,
              east_microdegrees: -122_000_000,
              north_microdegrees: 38_000_000,
              crosses_antimeridian: false,
            },
            source_origin: [-122_500_000, 37_500_000],
            pitch_degrees: 35.26439,
            yaw_degrees: 45,
          },
          globe: null,
          regime: "landscape",
          world: { width: 1_200, height: 720 },
        } as unknown as TopologyScene,
        { x: 0, y: 0 },
        1,
        { yaw_degrees: 0, pitch_degrees: 0 },
      ),
    ).toMatchObject({
      authority: "native_crs84",
      latitude_degrees: 37.5,
      longitude_degrees: -122.5,
    });

    expect(
      explorerGeographicCoordinate(
        {
          county_frame: null,
          globe: null,
          regime: "landscape",
          world: { width: 1_200, height: 720 },
        } as TopologyScene,
        { x: 0, y: 0 },
        1,
        { yaw_degrees: 0, pitch_degrees: 0 },
      ),
    ).toBeNull();
  });
});
