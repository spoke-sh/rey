import type { TopologyNode, TopologyScene } from "../../topology";
import {
  pickSemanticMercator,
  type ProjectionFrame,
  type SemanticMercatorPickCandidate,
} from "../projection/semantic-mercator";

export const EXPLORER_PICKING_REVISION = "rey.explorer.scene-picking@1";

export interface ScenePickingIndex {
  schema: "rey.explorer-picking-index.v1";
  picking_id: string;
  projection_frame: ProjectionFrame | null;
  candidates: readonly SemanticMercatorPickCandidate[];
}

export interface ScenePick {
  focus_id: string;
  x: number;
  y: number;
  semantic_identity?: string;
  semantic_coordinate?: SemanticMercatorPickCandidate["coordinate"];
  inverse_coordinate?: SemanticMercatorPickCandidate["coordinate"];
  chart_wrap_index?: number;
}

export function compileScenePickingIndex(
  scene: TopologyScene,
): ScenePickingIndex {
  const transition = scene.world_atlas_transition;
  const frame = transition
    ? Object.freeze({ ...transition.atlas_frame })
    : null;
  const candidates = Object.freeze(
    transition?.points.map((point) =>
      Object.freeze({
        identity: point.identity,
        focus_id: point.focus_id,
        coordinate: Object.freeze({
          longitude_microdegrees: point.longitude_microdegrees,
          latitude_microdegrees: point.latitude_microdegrees,
        }),
      }),
    ) ?? [],
  );
  return Object.freeze({
    schema: "rey.explorer-picking-index.v1",
    picking_id: [
      EXPLORER_PICKING_REVISION,
      transition?.atlas_revision ?? "atlas:none",
      transition?.projection_revision ?? "projection:none",
      ...candidates.map(
        ({ coordinate, identity }) =>
          `${identity}:${coordinate.longitude_microdegrees},${coordinate.latitude_microdegrees}`,
      ),
    ].join("|"),
    projection_frame: frame,
    candidates,
  });
}

export function pickSceneNode(
  index: ScenePickingIndex,
  node: TopologyNode,
  point: { x: number; y: number },
): ScenePick | null {
  if (
    !node.semantic_identity ||
    !node.semantic_coordinate ||
    !index.projection_frame
  )
    return Object.freeze({ focus_id: node.focus_id, ...point });
  const pick = pickSemanticMercator(
    point,
    index.candidates,
    index.projection_frame,
  );
  if (!pick || pick.identity !== node.semantic_identity) return null;
  return Object.freeze({
    focus_id: pick.focus_id,
    ...point,
    semantic_identity: pick.identity,
    semantic_coordinate: pick.coordinate,
    inverse_coordinate: pick.inverse_coordinate,
    chart_wrap_index: pick.wrap_index,
  });
}
