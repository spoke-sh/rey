import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { TopologyScene } from "../../topology";
import { ReferenceRenderer } from "./reference";

const terrainScene = {
  regime: "world",
  label: "CONTEXT WORLD",
  detail: "one admitted chart",
  focus_id: "cluster:portfolio",
  regions: [
    {
      id: "unexplored",
      label: "UNEXPLORED BEYOND SURVEY",
      detail: "no admitted terrain claim",
      x: 0,
      y: 0,
      width: 1500,
      height: 1000,
      tone: "unknown",
      variant: "map-zone",
    },
  ],
  landforms: [
    {
      id: "charted",
      path: "M100,100L200,100L200,200Z",
      kind: "charted",
      label: "charted",
      detail: "admitted anchor extent",
      tone: "healthy",
    },
  ],
  contours: [
    {
      id: "contour",
      path: "M120,120L180,180",
      level: 4,
      threshold: 0.5,
      anchor_count: 1,
    },
  ],
  natural_features: [
    {
      id: "stream",
      path: "M140,140L160,180",
      kind: "stream",
      label: "PROJECTED HEADWATERS",
      detail: "derived runoff; not a source relationship",
      intensity: 1,
      workload_id: "survey",
    },
  ],
  points: [],
  nodes: [],
  edges: [
    {
      id: "forbidden-source-edge",
      from: "source",
      to: "target",
      kind: "contains",
      label: "must not become geography",
    },
  ],
  omissions: ["source relationships excluded from terrain"],
  bearing: {
    status: "world",
    label: "SURVEY WEATHER",
    detail: "no path has been discovered or built",
    sampled_conditions: 1,
    unresolved_boundaries: 0,
  },
  world: { width: 1500, height: 1000 },
  fit_world: { width: 1500, height: 1000 },
  terrain: true,
  terrain_fields: [],
} satisfies TopologyScene;

describe("reference renderer", () => {
  it("preserves the bounded terrain manifest without source-edge geography", () => {
    const markup = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        scene={terrainScene}
      />,
    );

    expect(markup).toContain('data-renderer="reference"');
    expect(markup).toContain('data-world-geometry="charted"');
    expect(markup).toContain('data-relief-level="4"');
    expect(markup).toContain('data-natural-feature="stream"');
    expect(markup).not.toContain("topology-arrow");
    expect(markup).toContain("not a source relationship");
  });
});
