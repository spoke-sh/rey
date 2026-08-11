import type { WorkloadList } from "../../domain";
import {
  buildTopologyScene,
  type LensRegime,
  type TopologyScene,
} from "../../topology";
import { admittedTopographies } from "../projection/topography-projector";

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
  const sourceRevisions =
    topographies.length > 0
      ? topographies
          .map(({ projection }) => projection.packet_id)
          .sort((left, right) => left.localeCompare(right))
      : [
          portfolio.catalog.schema,
          portfolio.attention.attention_id,
          ...portfolio.workloads.map(
            ({ workload }) => workload.semantic_digest,
          ),
        ].sort((left, right) => left.localeCompare(right));
  const compilerRevisions = topographies
    .map(({ projection }) => projection.scene_compiler.semantic_digest)
    .sort((left, right) => left.localeCompare(right));
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
    bearing: Object.freeze({ ...scene.bearing }),
    world: Object.freeze({ ...scene.world }),
    fit_world: Object.freeze({ ...scene.fit_world }),
  });
}
