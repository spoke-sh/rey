import { describe, expect, it } from "vitest";
import type { TopologyGlobe } from "../../topology";
import {
  SEMANTIC_GLOBE_MATERIAL_REVISION,
  createSemanticGlobeBundle,
} from "./three-globe";

describe("Three.js semantic globe", () => {
  it("materializes admitted regions without changing their semantic identity", () => {
    const globe: TopologyGlobe = {
      schema: "rey.semantic-globe-scene.v1",
      atlas_id: "atlas:1",
      atlas_revision: "atlas:1",
      compiler_revision: "compiler:1",
      coordinate_authority: "synthetic semantic sphere",
      clusters: [],
      regions: [
        {
          id: "region:1",
          cluster_id: "cluster:1",
          focus_id: "anchor:survey:workspace",
          workload_id: "survey",
          label: "survey",
          detail: "admitted region",
          longitude_degrees: 18,
          latitude_degrees: -24,
          angular_radius_degrees: 5.5,
          tone: "frontier",
        },
      ],
    };
    const bundle = createSemanticGlobeBundle(globe, {
      width: 1500,
      height: 1000,
    });

    expect(bundle.material_revision).toBe(SEMANTIC_GLOBE_MATERIAL_REVISION);
    expect(
      bundle.scene.getObjectByName("semantic-region:region:1"),
    ).toBeDefined();
    expect(bundle.statistics.triangles).toBeGreaterThan(16_000);
    bundle.dispose();
  });
});
