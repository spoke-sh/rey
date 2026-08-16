import { describe, expect, it } from "vitest";
import type { TopologyScene } from "../../topology";
import {
  EXPLORER_RENDER_GRAPH_REVISION,
  activeExplorerRenderPasses,
  compileExplorerRenderGraph,
  type ExplorerRenderGraph,
} from "./render-graph";

function sceneFixture(): TopologyScene {
  const field = {
    field_set_id: "terrain:one",
    source_revision: "source:one",
    validity: { implementation_revision: "validity:one" },
    elevation: { implementation_revision: "elevation:one" },
    normal: { implementation_revision: "normal:one" },
    curvature: { implementation_revision: "curvature:one" },
    material: { implementation_revision: "material:one" },
  };
  return {
    regime: "landscape",
    focus_id: "node",
    terrain: true,
    terrain_fields: [field],
    contours: [
      {
        id: "contour",
        path: "M0,0 L10,10",
        threshold: 0.5,
      },
    ],
    natural_features: [],
    regions: [],
    county_footprint: null,
    nodes: [{ id: "node", evidence_uri: "evidence:one" }],
    points: [],
    omissions: [],
  } as unknown as TopologyScene;
}

describe("Explorer render graph", () => {
  it("retains one ordered authority-bearing pass graph", () => {
    const scene = sceneFixture();
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
    expect(
      graph.passes.find(({ id }) => id === "height_normals_hillshade"),
    ).toMatchObject({
      implementation_revision: "rey.render-pass.height-normals-hillshade@1",
      depends_on: ["base_terrain"],
    });
    expect(
      graph.passes.find(({ id }) => id === "height_normals_hillshade")
        ?.input_revision,
    ).toMatch(/^input:presentation-hash64:[0-9a-f]{16}:2:/);
    expect(graph.graph_id.length).toBeLessThan(128);
    expect(Object.isFrozen(graph.passes)).toBe(true);
  });

  it("invalidates identity when pass availability, inputs, or regime change", () => {
    const scene = sceneFixture();
    const first = compileExplorerRenderGraph(scene);
    expect(
      compileExplorerRenderGraph({ ...scene } as TopologyScene).graph_id,
    ).toBe(first.graph_id);
    expect(
      compileExplorerRenderGraph({
        ...scene,
        terrain: false,
        terrain_fields: [],
      } as TopologyScene).graph_id,
    ).not.toBe(first.graph_id);
    expect(
      compileExplorerRenderGraph({
        ...scene,
        contours: [{ ...scene.contours[0]!, path: "M0,0 L12,10" }],
      }).graph_id,
    ).not.toBe(first.graph_id);
    expect(
      compileExplorerRenderGraph({
        ...scene,
        terrain_fields: [
          {
            ...scene.terrain_fields[0]!,
            normal: {
              ...scene.terrain_fields[0]!.normal,
              implementation_revision: "normal:two",
            },
          },
        ],
      }).graph_id,
    ).not.toBe(first.graph_id);
  });

  it("keeps presentation identities bounded when vector paths are large", () => {
    const scene = sceneFixture();
    const path = `M0,0 ${"L10,10 ".repeat(100_000)}`;
    const graph = compileExplorerRenderGraph({
      ...scene,
      contours: [{ ...scene.contours[0]!, path }],
    });
    expect(graph.graph_id.length).toBeLessThan(128);
    expect(
      graph.passes.find(({ id }) => id === "contours")?.input_revision.length,
    ).toBeLessThan(128);
    expect(graph.graph_id).not.toContain(path);
  });

  it("projects transient controls without changing graph identity", () => {
    const scene = sceneFixture();
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

  it("fails closed when an enabled pass loses an executable dependency", () => {
    const graph = compileExplorerRenderGraph(sceneFixture());
    const invalid = {
      ...graph,
      passes: graph.passes.map((pass) =>
        pass.id === "base_terrain" ? { ...pass, enabled: false } : pass,
      ),
    } satisfies ExplorerRenderGraph;
    const active = activeExplorerRenderPasses(invalid, {
      contours: true,
      water: true,
      weather: true,
      probes: true,
    });
    expect(active.map(({ id }) => id)).not.toContain(
      "height_normals_hillshade",
    );
    expect(active.map(({ id }) => id)).not.toContain("contours");
    expect(graph.compiler_revision).toBe(EXPLORER_RENDER_GRAPH_REVISION);
  });
});
