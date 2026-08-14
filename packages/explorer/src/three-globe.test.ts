import { describe, expect, it } from "vitest";
import type { ExplorerGlobe } from "./types";
import {
  SEMANTIC_GLOBE_MATERIAL_REVISION,
  compileContextGlobe,
} from "./three-globe";

describe("Three.js semantic globe", () => {
  it("materializes admitted regions without changing their semantic identity", () => {
    const globe: ExplorerGlobe = {
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
    const compiled = compileContextGlobe(globe);

    expect(compiled.material_revision).toBe(SEMANTIC_GLOBE_MATERIAL_REVISION);
    expect(compiled.globe.regions[0]?.id).toBe("region:1");
    expect(compiled.sample_buckets.map(({ id }) => id)).toContain(
      "context-globe-samples:0",
    );
    expect(compiled.pole_patterns.map(({ pole }) => pole)).toEqual([
      "north",
      "south",
    ]);
    expect(compiled.statistics.triangles).toBeGreaterThan(80_000);
    expect(compiled.statistics.vertices).toBeGreaterThan(14_000);
    expect(compiled.statistics.geometry_compilation_ms).toBeGreaterThanOrEqual(
      0,
    );
  });

  it("materializes orientation beacons without claiming an admitted atlas", () => {
    const globe: ExplorerGlobe = {
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
    const compiled = compileContextGlobe(globe);
    expect(compiled.globe.beacons[0]?.workload_id).toBe(
      "context-anchor-survey",
    );
    expect(compiled.statistics.field_sets).toBe(1);
    expect(compiled.statistics.field_bytes).toBeGreaterThan(200_000);
  });
});
