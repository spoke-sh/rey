import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { CanvasFooter, CanvasToolbar } from "./explore";
import type { TopologyScene } from "./topology";

describe("Explorer canvas toolbar", () => {
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
        } as TopologyScene,
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
    const markup = renderToStaticMarkup(
      createElement(CanvasFooter, {
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
    expect(markup).not.toContain("BOUNDED /");
    expect(markup).not.toContain("regional atlas members");
    expect(markup).not.toContain("candidate terrain controls");
  });
});
