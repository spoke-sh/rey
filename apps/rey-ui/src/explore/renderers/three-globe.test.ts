import { describe, expect, it } from "vitest";
import type { TopologyGlobe } from "../../topology";
import {
  SEMANTIC_GLOBE_MATERIAL_REVISION,
  createContextGlobeBundle,
} from "./three-globe";

describe("Three.js semantic globe", () => {
  it("materializes admitted regions without changing their semantic identity", () => {
    const globe: TopologyGlobe = {
      schema: "rey.semantic-globe-scene.v1",
      posture: "semantic_atlas",
      globe_id: "atlas:1",
      source_revision: "atlas:1",
      compiler_revision: "compiler:1",
      coordinate_authority: "synthetic semantic sphere",
      clusters: [],
      beacons: [],
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
    const bundle = createContextGlobeBundle(globe, {
      width: 1500,
      height: 1000,
    });

    expect(bundle.material_revision).toBe(SEMANTIC_GLOBE_MATERIAL_REVISION);
    expect(
      bundle.scene.getObjectByName("context-globe-samples:0"),
    ).toBeDefined();
    expect(
      bundle.scene.getObjectByName("context-globe-atmosphere:2"),
    ).toBeDefined();
    expect(
      bundle.scene.getObjectByName("semantic-region:region:1"),
    ).toBeDefined();
    expect(bundle.statistics.triangles).toBeGreaterThan(80_000);
    expect(bundle.statistics.vertices).toBeGreaterThan(14_000);
    bundle.updateGlobeView?.({ yaw_degrees: 24, pitch_degrees: -8 });
    expect(
      bundle.scene.getObjectByName("context-globe:atlas:1")?.rotation.y,
    ).not.toBe(0);
    bundle.dispose();
  });

  it("materializes orientation beacons without claiming an admitted atlas", () => {
    const globe: TopologyGlobe = {
      schema: "rey.explore-orientation-globe.v1",
      posture: "orientation",
      globe_id: "orientation:one",
      source_revision: "working:one",
      compiler_revision: "orientation@1",
      coordinate_authority: "presentation only",
      clusters: [],
      regions: [],
      beacons: [
        {
          id: "workload-beacon:context-anchor-survey",
          focus_id: "beacon:context-anchor-survey",
          workload_id: "context-anchor-survey",
          label: "Survey project context anchors",
          detail: "WORKING",
          source: "sys/context-anchor-survey/workload.yaml",
          source_revision: "blake3:package",
          producer: "codex@gpt-5",
          state: "working",
          mapping_role: "survey",
          next_step: "review and consent",
          longitude_degrees: 14,
          latitude_degrees: 6,
          tone: "attention",
        },
      ],
    };
    const bundle = createContextGlobeBundle(globe, {
      width: 1_200,
      height: 720,
    });
    expect(
      bundle.scene.getObjectByName("workload-beacon:context-anchor-survey"),
    ).toBeDefined();
    expect(bundle.statistics.field_sets).toBe(1);
    expect(bundle.statistics.field_bytes).toBeGreaterThan(200_000);
    bundle.dispose();
  });
});
