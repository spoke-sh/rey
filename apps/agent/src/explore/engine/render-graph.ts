import type { TopologyScene } from "../../topology";

export const EXPLORER_RENDER_GRAPH_REVISION = "rey.explorer.render-graph@2";

export const EXPLORER_RENDER_PASS_REVISIONS = Object.freeze({
  validity_background: "rey.render-pass.validity-background@1",
  base_terrain: "rey.render-pass.base-terrain@1",
  height_normals_hillshade: "rey.render-pass.height-normals-hillshade@1",
  ambient_valley_occlusion: "rey.render-pass.ambient-valley-occlusion@1",
  contours: "rey.render-pass.contours@1",
  water_weather_boundary: "rey.render-pass.water-weather-boundary@1",
  features_labels_selection: "rey.render-pass.features-labels-selection@1",
  evidence_accessibility: "rey.render-pass.evidence-accessibility@1",
} satisfies Record<ExplorerRenderPassId, string>);

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
  implementation_revision: string;
  input_revision: string;
  depends_on: readonly ExplorerRenderPassId[];
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
  const selected = graph.passes.filter((pass) => {
    if (!pass.enabled) return false;
    if (pass.id === "contours") return visibility.contours;
    return true;
  });
  const executable: ExplorerRenderPass[] = [];
  const executableIds = new Set<ExplorerRenderPassId>();
  for (const pass of selected) {
    if (pass.depends_on.every((dependency) => executableIds.has(dependency))) {
      executable.push(pass);
      executableIds.add(pass.id);
    }
  }
  return Object.freeze(executable);
}

export function compileExplorerRenderGraph(
  scene: TopologyScene,
): ExplorerRenderGraph {
  const terrain = scene.terrain || scene.terrain_fields.length > 0;
  const fieldRevision = (
    channel: "validity" | "elevation" | "normal" | "curvature" | "material",
  ) =>
    revisionOf(
      scene.terrain_fields.map((field) =>
        [
          field.field_set_id,
          field.source_revision,
          channel === "material"
            ? field.material.implementation_revision
            : field[channel].implementation_revision,
        ].join(":"),
      ),
    );
  const passes = Object.freeze([
    renderPass(
      "validity_background",
      0,
      true,
      "evidence",
      [],
      fieldRevision("validity"),
    ),
    renderPass(
      "base_terrain",
      10,
      terrain,
      "derived",
      ["validity_background"],
      fieldRevision("material"),
    ),
    renderPass(
      "height_normals_hillshade",
      20,
      terrain,
      "derived",
      ["base_terrain"],
      revisionOf([fieldRevision("elevation"), fieldRevision("normal")]),
    ),
    renderPass(
      "ambient_valley_occlusion",
      30,
      terrain,
      "presentation",
      ["height_normals_hillshade"],
      revisionOf([fieldRevision("curvature"), fieldRevision("material")]),
    ),
    renderPass(
      "contours",
      40,
      scene.contours.length > 0,
      "derived",
      ["height_normals_hillshade"],
      revisionOf(
        scene.contours.map(
          ({ id, path, threshold }) => `${id}:${threshold}:${path}`,
        ),
      ),
    ),
    renderPass(
      "water_weather_boundary",
      50,
      scene.natural_features.length > 0 ||
        scene.regions.length > 0 ||
        scene.county_footprint != null,
      "derived",
      terrain ? ["base_terrain"] : ["validity_background"],
      revisionOf([
        ...scene.natural_features.map(
          ({ id, kind, path, detail }) => `${id}:${kind}:${path}:${detail}`,
        ),
        ...scene.regions.map(
          ({ id, fragment_id }) => `${id}:${fragment_id ?? "whole"}`,
        ),
        ...(scene.county_footprint
          ? [
              `${scene.county_footprint.footprint_id}:${scene.county_footprint.source_object_revision}:${scene.county_footprint.path}`,
            ]
          : []),
      ]),
    ),
    renderPass(
      "features_labels_selection",
      60,
      scene.nodes.length > 0 || scene.points.length > 0,
      "interface",
      terrain ? ["base_terrain"] : ["validity_background"],
      revisionOf([
        scene.focus_id,
        ...scene.nodes.map(
          ({ id, x, y, spatial_feature }) =>
            `${id}:${x},${y}:${spatial_feature?.geometry_kind ?? "semantic"}:${spatial_feature?.layer ?? "semantic"}:${spatial_feature?.envelope_path ?? "point"}:${spatial_feature?.authority ?? "interface"}`,
        ),
        ...scene.points.map(
          ({ id, kind, x, y, prominence }) =>
            `${id}:${kind}:${x},${y}:${prominence}`,
        ),
      ]),
    ),
    renderPass(
      "evidence_accessibility",
      70,
      true,
      "interface",
      ["validity_background"],
      revisionOf([
        scene.focus_id,
        ...scene.nodes.map(
          ({ evidence_uri }) => evidence_uri ?? "evidence:none",
        ),
        ...scene.omissions,
      ]),
    ),
  ]);
  return Object.freeze({
    schema: "rey.explorer-render-graph.v1",
    graph_id: [
      EXPLORER_RENDER_GRAPH_REVISION,
      scene.regime,
      ...passes.map(
        (pass) =>
          `${pass.id}:${pass.enabled ? 1 : 0}:${pass.implementation_revision}:${pass.input_revision}:${pass.depends_on.join(",")}`,
      ),
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
  dependsOn: readonly ExplorerRenderPassId[],
  inputRevision: string,
): ExplorerRenderPass {
  return Object.freeze({
    id,
    order,
    enabled,
    authority,
    implementation_revision: EXPLORER_RENDER_PASS_REVISIONS[id],
    input_revision: inputRevision,
    depends_on: Object.freeze([...dependsOn]),
  });
}

function revisionOf(values: readonly string[]): string {
  return values.length === 0
    ? "input:none"
    : [...values].sort((left, right) => left.localeCompare(right)).join("|");
}
