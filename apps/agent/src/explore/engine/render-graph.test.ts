import { describe, expect, it } from "vitest";
import type { TopologyScene } from "../../topology";
import {
  activeExplorerRenderPasses,
  compileExplorerRenderGraph,
} from "./render-graph";

const scene = {
  regime: "landscape",
  terrain: true,
  contours: [{ id: "contour" }],
  natural_features: [],
  regions: [],
  nodes: [{ id: "node" }],
  points: [],
} as unknown as TopologyScene;

describe("Explorer render graph", () => {
  it("retains one ordered authority-bearing pass graph", () => {
    const graph = compileExplorerRenderGraph(scene);
    expect(graph.passes.map(({ id }) => id)).toEqual([
      "validity_background",
      "base_terrain",
      "height_normals_hillshade",
      "ambient_valley_occlusion",
      "contours",
      "water_weather_boundary",
      "features_labels_selection",
      "evidence_accessibility",
    ]);
    expect(graph.passes.map(({ order }) => order)).toEqual([
      0, 10, 20, 30, 40, 50, 60, 70,
    ]);
    expect(graph.passes.find(({ id }) => id === "contours")?.enabled).toBe(
      true,
    );
    expect(
      graph.passes.find(({ id }) => id === "water_weather_boundary")?.enabled,
    ).toBe(false);
    expect(Object.isFrozen(graph.passes)).toBe(true);
  });

  it("changes identity only when pass availability or regime changes", () => {
    const first = compileExplorerRenderGraph(scene);
    expect(
      compileExplorerRenderGraph({ ...scene } as TopologyScene).graph_id,
    ).toBe(first.graph_id);
    expect(
      compileExplorerRenderGraph({
        ...scene,
        terrain: false,
      } as TopologyScene).graph_id,
    ).not.toBe(first.graph_id);
  });

  it("projects transient controls without changing graph identity", () => {
    const graph = compileExplorerRenderGraph(scene);
    const active = activeExplorerRenderPasses(graph, {
      contours: false,
      water: false,
      weather: false,
      probes: false,
    });
    expect(active.map(({ id }) => id)).not.toContain("contours");
    expect(active.map(({ id }) => id)).toContain("base_terrain");
    expect(graph.passes.find(({ id }) => id === "contours")?.enabled).toBe(
      true,
    );
  });
});
