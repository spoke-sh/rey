import type { WorkloadList } from "../../domain";
import {
  ATLAS_TERRAIN_PREWARM_REVISION,
  buildTopologyScene,
  type LensRegime,
  type TopologyScene,
} from "../../topology";
import { admittedTopographies } from "../projection/topography-projector";
import { admittedRegionalScenes } from "../projection/regional-scene-projector";
import { SEMANTIC_MERCATOR_PROJECTION_REVISION } from "../projection/semantic-mercator";
import {
  COUNTY_FOOTPRINT_PROJECTION_REVISION,
  COUNTY_FRAME_PROJECTION_REVISION,
} from "../projection/county-frame";
import { REGIONAL_TERRAIN_SCENE_COMPILER_REVISION } from "../projection/regional-terrain";
import { REGIONAL_TERRAIN_MOSAIC_REVISION } from "../terrain/regional-mosaic";
import { ATLAS_LANDSCAPE_PROJECTION_REVISION } from "../projection/atlas-landscape";
import type {
  CountyFrame,
  ProjectedCountyFootprint,
} from "../projection/county-frame";
import { SEMANTIC_LABEL_LAYOUT_REVISION } from "./labels";
import type { TerrainFieldSet, TerrainProgram } from "../terrain/compile";
import { SURVEY_TERRAIN_SCENE_COMPILER_REVISION } from "../projection/survey-terrain";
import { SURVEY_SCENE_LAYOUT_REVISION } from "../projection/survey-scene-layout";
import { SURVEY_SCENE_PROJECTION_REVISION } from "../projection/survey-scene";
import { PORTFOLIO_SCENE_PROJECTION_REVISION } from "../projection/portfolio-scene";
import {
  EXPLORER_PICKING_REVISION,
  compileScenePickingIndex,
  type ScenePickingIndex,
} from "./picking";
import {
  EXPLORER_RENDER_GRAPH_REVISION,
  compileExplorerRenderGraph,
  type ExplorerRenderGraph,
} from "./render-graph";

export interface SceneSnapshot {
  readonly schema: "rey.reference-scene-snapshot.v1";
  readonly snapshot_id: string;
  readonly source_revisions: readonly string[];
  readonly compiler_revisions: readonly string[];
  readonly regime: LensRegime;
  readonly focus_id: string;
  readonly render_graph: ExplorerRenderGraph;
  readonly picking_index: ScenePickingIndex;
  readonly scene: TopologyScene;
}

export interface SceneProjectionResult {
  readonly snapshot: SceneSnapshot;
  readonly retained_last_good: boolean;
  readonly error: Error | null;
}

export class LastGoodSceneCompiler {
  #snapshot: SceneSnapshot | undefined;

  compile(
    portfolio: WorkloadList,
    zoom: number,
    focusId: string,
    retainedRegime?: LensRegime,
  ): SceneProjectionResult {
    try {
      const snapshot = compileSceneSnapshot(
        portfolio,
        zoom,
        focusId,
        retainedRegime,
      );
      this.#snapshot = snapshot;
      return Object.freeze({
        snapshot,
        retained_last_good: false,
        error: null,
      });
    } catch (error) {
      if (!this.#snapshot) throw error;
      return Object.freeze({
        snapshot: this.#snapshot,
        retained_last_good: true,
        error: normalizeSceneCompilationError(error),
      });
    }
  }
}

export function compileSceneSnapshot(
  portfolio: WorkloadList,
  zoom: number,
  focusId: string,
  retainedRegime?: LensRegime,
): SceneSnapshot {
  const scene = freezeTopologyScene(
    buildTopologyScene(portfolio, zoom, focusId, retainedRegime),
  );
  const renderGraph = compileExplorerRenderGraph(scene);
  const pickingIndex = compileScenePickingIndex(scene);
  const topographies = admittedTopographies(portfolio);
  const regionalScenes = admittedRegionalScenes(portfolio);
  const sourceRevisions =
    topographies.length > 0 || regionalScenes.length > 0
      ? topographies
          .map(({ projection }) => projection.packet_id)
          .concat(
            regionalScenes.flatMap(({ county_footprint, result, scene }) => [
              result.result_id,
              scene.projection.packet_id,
              ...(scene.projection.terrain?.grid
                ? [scene.projection.terrain.grid.dataset_id]
                : []),
              ...(county_footprint
                ? [
                    county_footprint.footprint_id,
                    county_footprint.source_object_revision,
                  ]
                : []),
            ]),
          )
          .sort((left, right) => left.localeCompare(right))
      : [
          portfolio.catalog.schema,
          portfolio.attention.attention_id,
          ...(portfolio.revision
            ? [
                portfolio.revision.working.snapshot_revision,
                portfolio.revision.index?.snapshot_revision ?? "index:empty",
                portfolio.revision.head?.commit_id ?? "head:empty",
              ]
            : []),
          ...portfolio.workloads.map(
            ({ workload }) => workload.semantic_digest,
          ),
          ...portfolio.drafts.map(({ source_digest }) => source_digest),
        ].sort((left, right) => left.localeCompare(right));
  if (portfolio.semantic_atlas)
    sourceRevisions.push(portfolio.semantic_atlas.atlas_revision);
  const latestAtlasDelta = portfolio.semantic_atlas_deltas.at(-1);
  const latestRetainedAtlas = portfolio.semantic_atlas_history.at(-1);
  if (
    portfolio.semantic_atlas &&
    latestAtlasDelta &&
    latestRetainedAtlas?.atlas_revision ===
      portfolio.semantic_atlas.atlas_revision &&
    latestAtlasDelta.target_revision === portfolio.semantic_atlas.atlas_revision
  )
    sourceRevisions.push(latestAtlasDelta.delta_id);
  sourceRevisions.sort((left, right) => left.localeCompare(right));
  const compilerRevisions = topographies
    .map(({ projection }) => projection.scene_compiler.semantic_digest)
    .concat(
      regionalScenes.flatMap(({ scene }) => [
        scene.admission.implementation.semantic_digest,
        scene.projection.grammar_id,
      ]),
    )
    .sort((left, right) => left.localeCompare(right));
  if (topographies.length > 0) {
    compilerRevisions.push(SURVEY_SCENE_LAYOUT_REVISION);
    compilerRevisions.push(SURVEY_SCENE_PROJECTION_REVISION);
    compilerRevisions.push(SURVEY_TERRAIN_SCENE_COMPILER_REVISION);
  }
  if (regionalScenes.length > 0) {
    compilerRevisions.push(SEMANTIC_MERCATOR_PROJECTION_REVISION);
    compilerRevisions.push(SEMANTIC_LABEL_LAYOUT_REVISION);
    compilerRevisions.push(COUNTY_FRAME_PROJECTION_REVISION);
  }
  if (regionalScenes.some(({ county_footprint }) => county_footprint))
    compilerRevisions.push(COUNTY_FOOTPRINT_PROJECTION_REVISION);
  if (regionalScenes.some(({ scene }) => scene.projection.terrain?.grid))
    compilerRevisions.push(REGIONAL_TERRAIN_SCENE_COMPILER_REVISION);
  if (scene.terrain_fields.some((field) => field.landscape_mosaic))
    compilerRevisions.push(REGIONAL_TERRAIN_MOSAIC_REVISION);
  if (scene.atlas_landscape_transition)
    compilerRevisions.push(ATLAS_LANDSCAPE_PROJECTION_REVISION);
  else if (scene.regime === "atlas" && scene.terrain_fields.length > 0)
    compilerRevisions.push(ATLAS_TERRAIN_PREWARM_REVISION);
  if (portfolio.semantic_atlas)
    compilerRevisions.push(portfolio.semantic_atlas.compiler.semantic_digest);
  if (
    topographies.length === 0 &&
    regionalScenes.length === 0 &&
    scene.regime !== "world"
  )
    compilerRevisions.push(PORTFOLIO_SCENE_PROJECTION_REVISION);
  compilerRevisions.sort((left, right) => left.localeCompare(right));
  compilerRevisions.push(EXPLORER_RENDER_GRAPH_REVISION);
  compilerRevisions.push(EXPLORER_PICKING_REVISION);
  compilerRevisions.sort((left, right) => left.localeCompare(right));
  const snapshotId = [
    "rey.reference-scene-snapshot.v1",
    ...sourceRevisions,
    ...compilerRevisions,
    scene.regime,
    scene.focus_id,
    renderGraph.graph_id,
    pickingIndex.picking_id,
  ].join("|");
  return Object.freeze({
    schema: "rey.reference-scene-snapshot.v1",
    snapshot_id: snapshotId,
    source_revisions: Object.freeze(sourceRevisions),
    compiler_revisions: Object.freeze(compilerRevisions),
    regime: scene.regime,
    focus_id: scene.focus_id,
    render_graph: renderGraph,
    picking_index: pickingIndex,
    scene,
  });
}

function freezeTopologyScene(scene: TopologyScene): TopologyScene {
  const freezeRows = <T extends object>(rows: T[]): T[] =>
    Object.freeze(rows.map((row) => Object.freeze({ ...row }))) as T[];
  return Object.freeze({
    ...scene,
    regions: freezeRows(scene.regions),
    landforms: freezeRows(scene.landforms),
    contours: freezeRows(scene.contours),
    natural_features: freezeRows(scene.natural_features),
    points: freezeRows(scene.points),
    nodes: Object.freeze(
      scene.nodes.map((node) =>
        Object.freeze({
          ...node,
          semantic_coordinate: node.semantic_coordinate
            ? Object.freeze({ ...node.semantic_coordinate })
            : undefined,
          spatial_feature: node.spatial_feature
            ? Object.freeze({ ...node.spatial_feature })
            : undefined,
        }),
      ),
    ) as TopologyScene["nodes"],
    edges: freezeRows(scene.edges),
    omissions: Object.freeze([...scene.omissions]) as string[],
    terrain_fields: Object.freeze([
      ...scene.terrain_fields,
    ]) as TerrainFieldSet[],
    terrain_programs: Object.freeze([
      ...scene.terrain_programs,
    ]) as TerrainProgram[],
    globe: scene.globe
      ? Object.freeze({
          ...scene.globe,
          clusters: freezeRows(scene.globe.clusters),
          regions: freezeRows(scene.globe.regions),
        })
      : null,
    world_atlas_transition: scene.world_atlas_transition
      ? Object.freeze({
          ...scene.world_atlas_transition,
          atlas_frame: Object.freeze({
            ...scene.world_atlas_transition.atlas_frame,
          }),
          points: freezeRows(scene.world_atlas_transition.points),
          sectors: freezeRows(scene.world_atlas_transition.sectors),
        })
      : null,
    atlas_landscape_transition: scene.atlas_landscape_transition
      ? Object.freeze({
          ...scene.atlas_landscape_transition,
          source_frame: Object.freeze({
            ...scene.atlas_landscape_transition.source_frame,
          }),
          target_frame: Object.freeze({
            ...scene.atlas_landscape_transition.target_frame,
          }),
        })
      : null,
    county_frame: scene.county_frame
      ? Object.freeze({
          ...scene.county_frame,
          source_bounds: Object.freeze({ ...scene.county_frame.source_bounds }),
          source_origin: Object.freeze([
            scene.county_frame.source_origin[0],
            scene.county_frame.source_origin[1],
          ]) as CountyFrame["source_origin"],
          target_origin: Object.freeze([
            scene.county_frame.target_origin[0],
            scene.county_frame.target_origin[1],
            scene.county_frame.target_origin[2],
          ]) as CountyFrame["target_origin"],
        })
      : null,
    county_footprint: scene.county_footprint
      ? Object.freeze({
          ...scene.county_footprint,
          native_bounds: Object.freeze({
            ...scene.county_footprint.native_bounds,
          }),
          rings: Object.freeze(
            scene.county_footprint.rings.map((ring) =>
              Object.freeze(
                ring.map(
                  (position) =>
                    Object.freeze([position[0], position[1]]) as readonly [
                      number,
                      number,
                    ],
                ),
              ),
            ),
          ) as ProjectedCountyFootprint["rings"],
          screen_rings: Object.freeze(
            scene.county_footprint.screen_rings.map((ring) =>
              Object.freeze(
                ring.map((position) => Object.freeze({ ...position })),
              ),
            ),
          ) as ProjectedCountyFootprint["screen_rings"],
        })
      : null,
    bearing: Object.freeze({ ...scene.bearing }),
    world: Object.freeze({ ...scene.world }),
    fit_world: Object.freeze({ ...scene.fit_world }),
  });
}

function normalizeSceneCompilationError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}
