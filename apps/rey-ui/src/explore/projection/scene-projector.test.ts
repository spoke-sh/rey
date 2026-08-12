import { describe, expect, it } from "vitest";
import type { SceneAdmission, WorkloadList } from "../../domain";
import { compileSceneSnapshot } from "../engine/scene";
import { DEFAULT_LENS_ZOOM, buildTopologyScene } from "../../topology";
import { admittedScenes } from "./scene-projector";

const admission = {
  schema: "rey.scene-admission.v1",
  admission_id: "admission:1",
  request_id: "request:1",
  package_id: "package:1",
  status: "admitted",
  admitted: true,
  validation: {
    schema: "rey.scene-validation.v1",
    validator: "rey.scene-admission.validate@1",
    workload: {
      id: "rey.scene-admission",
      revision: 1,
      semantic_digest: "workload:1",
    },
    graph: {
      id: "rey.scene-admission.graph",
      revision: 1,
      semantic_digest: "graph:1",
    },
    scenario_suite: {
      id: "rey.scene-admission.scenarios",
      revision: 1,
      semantic_digest: "scenarios:1",
    },
    evaluator: {
      id: "rey.scenario.utf8-exact",
      revision: 1,
      semantic_digest: "evaluator:1",
    },
    test_result_id: "test:1",
    qualification_id: "qualification:1",
    run_id: "run:1",
    package_id: "package:1",
    snapshot_revision: "snapshot:1",
    source_objects: ["object:1"],
    sources: 1,
    features: 2,
    coordinates: 6,
    complete: true,
    omissions: [],
  },
  projection: {
    schema: "rey.scene-projection.v1",
    projection_id: "projection:1",
    package_id: "package:1",
    snapshot_revision: "snapshot:1",
    project_id: "alter-landscape",
    coordinate_system: {
      kind: "geographic",
      authority: "OGC",
      code: "CRS84",
      axis_order: "longitude_latitude",
    },
    bounds: { west: -2, south: 1, east: 2, north: 3 },
    features: [
      {
        feature_id: "architecture/accounting",
        source_id: "architecture",
        role: "features",
        geometry_kind: "Polygon",
        geometry: {
          type: "Polygon",
          coordinates: [
            [
              [-2, 1],
              [0, 1],
              [0, 3],
              [-2, 1],
            ],
          ],
        },
        label: "Accounting Core",
        detail: "Immutable balanced postings are the highest evidence.",
        category: "architectural_district",
        marker: null,
        feature_revision: "feature:1",
      },
      {
        feature_id: "markers/ledger",
        source_id: "markers",
        role: "markers",
        geometry_kind: "Point",
        geometry: { type: "Point", coordinates: [1, 2] },
        label: "Ledger Evidence",
        detail: "Balances fold over immutable entries.",
        category: "source_of_truth",
        marker: {
          title: "Ledger Evidence",
          category: "source_of_truth",
          symbol: "ledger",
          min_zoom: 0,
          max_zoom: 24,
          collision_priority: 100,
        },
        feature_revision: "feature:2",
      },
    ],
    complete: true,
    omissions: [],
  },
} satisfies SceneAdmission;

const portfolio = {
  schema: "rey.workload-list.v1",
  semantic_atlas: null,
  workloads: [],
  drafts: [],
  scene_admissions: [admission],
  catalog: { schema: "catalog", admitted_count: 0 },
  attention: { attention_id: "attention", rows: [] },
} as unknown as WorkloadList;

describe("editor scene evidence adapter", () => {
  it("rejects a scene whose validation does not bind the projected package", () => {
    expect(admittedScenes(portfolio)).toEqual([admission]);
    expect(
      admittedScenes({
        ...portfolio,
        scene_admissions: [
          {
            ...admission,
            projection: {
              ...admission.projection,
              package_id: "package:other",
            },
          },
        ],
      }),
    ).toEqual([]);
  });

  it("renders admitted native geometry and its authored descriptions", () => {
    const scene = buildTopologyScene(
      portfolio,
      DEFAULT_LENS_ZOOM,
      "scene-feature:markers/ledger",
    );

    expect(scene.landforms).toHaveLength(1);
    expect(scene.landforms[0]?.path).toContain("M54.00,633.00");
    expect(scene.points.map(({ label }) => label)).toEqual([
      "Accounting Core",
      "Ledger Evidence",
    ]);
    expect(scene.label).toBe("Ledger Evidence");
    expect(scene.detail).toContain("immutable entries");
    expect(scene.bearing.label).toBe("VALIDATED EDITOR SCENE");

    const snapshot = compileSceneSnapshot(
      portfolio,
      DEFAULT_LENS_ZOOM,
      "scene-feature:markers/ledger",
    );
    expect(snapshot.source_revisions).toEqual(["admission:1", "projection:1"]);
    expect(snapshot.compiler_revisions).toContain("workload:1");
  });
});
