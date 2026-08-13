import type { WorkloadList } from "../../domain";
import {
  buildTopologyScene,
  type LensRegime,
  type TopologyScene,
} from "../../topology";
import { admittedTopographies } from "../projection/topography-projector";
import { admittedRegionalScenes } from "../projection/regional-scene-projector";
import type { TerrainFieldSet, TerrainProgram } from "../terrain/compile";

export interface SceneSnapshot {
  readonly schema: "rey.reference-scene-snapshot.v1";
  readonly snapshot_id: string;
  readonly source_revisions: readonly string[];
  readonly compiler_revisions: readonly string[];
  readonly regime: LensRegime;
  readonly focus_id: string;
  readonly scene: TopologyScene;
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
  const topographies = admittedTopographies(portfolio);
  const regionalScenes = admittedRegionalScenes(portfolio);
  const sourceRevisions =
    topographies.length > 0 || regionalScenes.length > 0
      ? topographies
          .map(({ projection }) => projection.packet_id)
          .concat(
            regionalScenes.flatMap(({ result, scene }) => [
              result.result_id,
              scene.projection.packet_id,
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
  if (portfolio.semantic_atlas)
    compilerRevisions.push(portfolio.semantic_atlas.compiler.semantic_digest);
  compilerRevisions.sort((left, right) => left.localeCompare(right));
  const snapshotId = [
    "rey.reference-scene-snapshot.v1",
    ...sourceRevisions,
    ...compilerRevisions,
    scene.regime,
    scene.focus_id,
  ].join("|");
  return Object.freeze({
    schema: "rey.reference-scene-snapshot.v1",
    snapshot_id: snapshotId,
    source_revisions: Object.freeze(sourceRevisions),
    compiler_revisions: Object.freeze(compilerRevisions),
    regime: scene.regime,
    focus_id: scene.focus_id,
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
    nodes: freezeRows(scene.nodes),
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
    bearing: Object.freeze({ ...scene.bearing }),
    world: Object.freeze({ ...scene.world }),
    fit_world: Object.freeze({ ...scene.fit_world }),
  });
}
