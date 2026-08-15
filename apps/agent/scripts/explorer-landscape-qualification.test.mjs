import { describe, expect, it } from "vitest";
import {
  evaluateLandscapeCapture,
  landscapeWorkload,
  validateLandscapeWorkloadSuite,
} from "./explorer-landscape-qualification.mjs";

const suite = {
  schema: "rey.explorer-landscape-workloads.v1",
  suite_id: "suite:fixture",
  target_viewports: ["1920x1080"],
  workloads: [
    {
      id: "holes",
      purpose: "retain holes",
      requirements: {
        minimum_source_no_data_vertices: 1,
        maximum_no_data_leak_triangles: 0,
        maximum_tile_seam_mismatches: 0,
        required_render_passes: ["validity_background", "base_terrain"],
      },
    },
  ],
};

const capture = {
  stage: "landscape",
  scene_snapshot_id: "scene:fixture",
  source_revisions: ["source:fixture"],
  compilers: "compiler:fixture",
  projection: {
    render_passes: ["validity_background", "base_terrain"],
  },
  renderer: {
    backend: "webgpu",
    render_pass_set_id: "passes:fixture",
    render_pass_kinds: "admitted_boundary,river",
    render_pass_line_count: "2",
    resident_cpu_bytes: "1024",
    resident_cpu_budget_bytes: "2048",
    resident_gpu_bytes: "1024",
    resident_gpu_budget_bytes: "2048",
    source_valid_vertices: "15",
    source_no_data_vertices: "1",
    source_elevation_span: "80",
    terrain_maximum_screen_error_pixels: "1.25",
    terrain_tile_seam_mismatches: "0",
    terrain_no_data_leak_triangles: "0",
  },
  labels: { total: 4 },
  scene_omissions: [],
};

describe("Landscape browser workload qualification", () => {
  it("selects one named workload only at a target viewport", () => {
    expect(validateLandscapeWorkloadSuite(suite)).toBe(suite);
    expect(landscapeWorkload(suite, "holes", "1920x1080").id).toBe("holes");
    expect(() => landscapeWorkload(suite, "holes", "800x600")).toThrow(
      "not a target viewport",
    );
  });

  it("requires exact lineage, bounded resources, and zero no-data leakage", () => {
    const workload = landscapeWorkload(suite, "holes", "1920x1080");
    expect(
      evaluateLandscapeCapture(capture, workload, { loss: "none" }, null),
    ).toMatchObject({
      workload_id: "holes",
      passed: true,
      checks: {
        exact_scene_lineage: true,
        no_data_leakage_respected: true,
        tile_seams_respected: true,
      },
    });
  });

  it("fails when an accelerated triangle leaks into no-data", () => {
    const workload = landscapeWorkload(suite, "holes", "1920x1080");
    const result = evaluateLandscapeCapture(
      {
        ...capture,
        renderer: {
          ...capture.renderer,
          terrain_no_data_leak_triangles: "1",
        },
      },
      workload,
      { loss: "none" },
      null,
    );
    expect(result.passed).toBe(false);
    expect(result.checks.no_data_leakage_respected).toBe(false);
  });
});
