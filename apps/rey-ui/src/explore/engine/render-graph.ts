import type { TopologyScene } from "../../topology";

export const EXPLORER_RENDER_GRAPH_REVISION = "rey.explorer.render-graph@1";

export type ExplorerRenderPassId =
  | "validity_background"
  | "base_terrain"
  | "height_normals_hillshade"
  | "ambient_valley_occlusion"
  | "contours"
  | "water_weather_boundary"
  | "features_labels_selection"
  | "evidence_accessibility";

export interface ExplorerRenderPass {
  id: ExplorerRenderPassId;
  order: number;
  enabled: boolean;
  authority: "evidence" | "derived" | "presentation" | "interface";
}

export interface ExplorerRenderGraph {
  schema: "rey.explorer-render-graph.v1";
  graph_id: string;
  compiler_revision: typeof EXPLORER_RENDER_GRAPH_REVISION;
  passes: readonly ExplorerRenderPass[];
}

export interface ExplorerRenderVisibility {
  contours: boolean;
  water: boolean;
  weather: boolean;
  probes: boolean;
}

export function activeExplorerRenderPasses(
  graph: ExplorerRenderGraph,
  visibility: ExplorerRenderVisibility,
): readonly ExplorerRenderPass[] {
  return Object.freeze(
    graph.passes.filter((pass) => {
      if (!pass.enabled) return false;
      if (pass.id === "contours") return visibility.contours;
      return true;
    }),
  );
}

export function compileExplorerRenderGraph(
  scene: TopologyScene,
): ExplorerRenderGraph {
  const terrain = scene.terrain;
  const passes = Object.freeze([
    renderPass("validity_background", 0, true, "evidence"),
    renderPass("base_terrain", 10, terrain, "derived"),
    renderPass("height_normals_hillshade", 20, terrain, "derived"),
    renderPass("ambient_valley_occlusion", 30, terrain, "presentation"),
    renderPass("contours", 40, scene.contours.length > 0, "derived"),
    renderPass(
      "water_weather_boundary",
      50,
      scene.natural_features.length > 0 || scene.regions.length > 0,
      "derived",
    ),
    renderPass(
      "features_labels_selection",
      60,
      scene.nodes.length > 0 || scene.points.length > 0,
      "interface",
    ),
    renderPass("evidence_accessibility", 70, true, "interface"),
  ]);
  return Object.freeze({
    schema: "rey.explorer-render-graph.v1",
    graph_id: [
      EXPLORER_RENDER_GRAPH_REVISION,
      scene.regime,
      ...passes.map((pass) => `${pass.id}:${pass.enabled ? 1 : 0}`),
    ].join("|"),
    compiler_revision: EXPLORER_RENDER_GRAPH_REVISION,
    passes,
  });
}

function renderPass(
  id: ExplorerRenderPassId,
  order: number,
  enabled: boolean,
  authority: ExplorerRenderPass["authority"],
): ExplorerRenderPass {
  return Object.freeze({ id, order, enabled, authority });
}
