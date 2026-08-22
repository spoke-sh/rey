import { describe, expect, it } from "vitest";
import type { TopologyScene } from "../../topology";
import {
  rendererPreference,
  retainCompatibleTerrainSubmission,
  terrainCompilationSourceKey,
} from "./accelerated-terrain";

function terrainScene(
  revisions: {
    source?: string;
    mosaic?: string;
    composition?: string;
    elevation?: string;
    material?: string;
  } = {},
): TopologyScene {
  return {
    terrain_fields: [
      {
        field_set_id: "terrain:regional-mosaic",
        source_revision: revisions.source ?? "source:1",
        grid: {
          columns: 3,
          rows: 3,
          bounds: { x: 10, y: 20, width: 30, height: 40 },
        },
        elevation: {
          implementation_revision: revisions.elevation ?? "elevation:1",
        },
        material: {
          implementation_revision: revisions.material ?? "material:1",
        },
        landscape_mosaic: {
          mosaic_id: revisions.mosaic ?? "mosaic:1",
          composition_revision: revisions.composition ?? "composition:1",
        },
      },
    ],
    terrain_programs: [],
  } as unknown as TopologyScene;
}

describe("accelerated terrain browser qualification", () => {
  it("keeps backend forcing in the view envelope rather than semantic identity", () => {
    expect(rendererPreference("?renderer=webgpu")).toBe("webgpu");
    expect(rendererPreference("?renderer=webgl2")).toBe("webgl2");
    expect(rendererPreference("?renderer=reference")).toBe("reference");
    expect(rendererPreference("?renderer=unknown")).toBe("auto");
  });

  it("binds Atlas prewarm to exact terrain hierarchy source identity", () => {
    const original = terrainCompilationSourceKey(terrainScene());
    expect(terrainCompilationSourceKey(terrainScene())).toBe(original);
    expect(
      terrainCompilationSourceKey(terrainScene({ source: "source:2" })),
    ).not.toBe(original);
    expect(
      terrainCompilationSourceKey(terrainScene({ mosaic: "mosaic:2" })),
    ).not.toBe(original);
    expect(
      terrainCompilationSourceKey(
        terrainScene({ composition: "composition:2" }),
      ),
    ).not.toBe(original);
    expect(
      terrainCompilationSourceKey(terrainScene({ material: "material:2" })),
    ).not.toBe(original);
  });

  it("retains the last submitted terrain across view jobs but not source changes", () => {
    const sourceKey = terrainCompilationSourceKey(terrainScene());
    const submission = { job_id: "entry-view:1", source_key: sourceKey };
    expect(retainCompatibleTerrainSubmission(submission, sourceKey)).toBe(
      submission,
    );
    expect(
      retainCompatibleTerrainSubmission(
        submission,
        terrainCompilationSourceKey(terrainScene({ source: "source:2" })),
      ),
    ).toBeNull();
  });
});
