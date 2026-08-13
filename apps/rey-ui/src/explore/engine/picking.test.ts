import { describe, expect, it } from "vitest";
import type { TopologyNode, TopologyScene } from "../../topology";
import { projectSemanticMercator } from "../projection/semantic-mercator";
import { compileScenePickingIndex, pickSceneNode } from "./picking";

describe("scene picking index", () => {
  it("binds every chart copy to one immutable semantic identity", () => {
    const frame = { x: 0, y: 0, width: 1200, height: 720 };
    const coordinate = {
      longitude_microdegrees: -42_000_000,
      latitude_microdegrees: 18_000_000,
    };
    const scene = {
      world_atlas_transition: {
        atlas_revision: "atlas:one",
        projection_revision: "projection:one",
        atlas_frame: frame,
        points: [
          {
            identity: "region:one",
            focus_id: "regional:one",
            ...coordinate,
          },
        ],
      },
    } as TopologyScene;
    const node = {
      focus_id: "regional:one",
      semantic_identity: "region:one",
      semantic_coordinate: coordinate,
    } as TopologyNode;
    const index = compileScenePickingIndex(scene);
    for (const wrapIndex of [-1, 0, 1]) {
      const point = projectSemanticMercator(coordinate, frame, wrapIndex);
      expect(pickSceneNode(index, node, point)).toMatchObject({
        focus_id: "regional:one",
        semantic_identity: "region:one",
        chart_wrap_index: wrapIndex,
      });
    }
    expect(Object.isFrozen(index.candidates)).toBe(true);
  });
});
