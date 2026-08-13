import { describe, expect, it } from "vitest";
import type {
  ProjectionPacket,
  SemanticAtlas,
  TopographyPatch,
  WorkloadList,
  WorkloadSummary,
} from "./domain";
import {
  DEFAULT_LENS_ZOOM,
  EVIDENCE_LENS_ZOOM,
  LANDSCAPE_LENS_ZOOM,
  MAX_LENS_ZOOM,
  MIN_LENS_ZOOM,
  NEIGHBORHOOD_LENS_ZOOM,
  OBJECT_LENS_ZOOM,
  WORLD_LENS_ZOOM,
  buildTopologyScene,
  clampLensZoom,
  lensRegimeForZoom,
  stepLensZoom,
} from "./topology";

const workload = (id: string): WorkloadSummary => ({
  provenance: null,
  workload: { id, revision: 1, semantic_digest: `digest:${id}` },
  title: id,
  candidate_graph: {
    id: `${id}.graph`,
    revision: 2,
    semantic_digest: `graph:${id}`,
  },
  freshness: "fresh",
  qualification: "failing",
  required: 3,
  passed: 2,
  failed: 1,
  inconclusive: 0,
  evaluated: 3,
  stale: 0,
  optional: 0,
  mining_operations: 2,
  mining_results: 4,
  incomplete_mining_results: 0,
  relation_deltas: 1,
  reasoning_surfaces: 1,
  attention_rows: 1,
  topography_results: 0,
  topography_revision: null,
  topography_coverage: null,
  topography_frontier_rows: 0,
  topography_patch: null,
  topography_projection: null,
  scene_admission_results: 0,
  latest_scene_admission: null,
  last_run_status: "blocked",
  last_test_result_id: `test:${id}`,
});

const portfolio: WorkloadList = {
  schema: "rey.workload-list.v1",
  semantic_atlas: null,
  semantic_atlas_history: [],
  semantic_atlas_deltas: [],
  catalog: {
    schema: "rey.workload-catalog.v1",
    kind: "workspace_packages",
    root: "workloads",
    workload_count: 1,
    admitted_count: 1,
    draft_count: 0,
  },
  workloads: [workload("rey.example")],
  drafts: [],
  attention: {
    schema: "rey.workload-attention.v1",
    attention_id: "attention:1",
    source_snapshot_id: "snapshot:1",
    rows: [
      {
        row_id: "row:1",
        action: "refine",
        subject_kind: "workload",
        subject_id: "rey.example",
        reason: "scenario_delta",
        readiness: "ready",
        evidence_ids: ["evidence:1", "evidence:2"],
        dependency_ids: [],
        priority: 10,
        estimated_cost_units: 2,
      },
    ],
    summary: {
      refine: 1,
      retest: 0,
      create: 0,
      blocked: 0,
      policy_excluded: 0,
      workloads: 1,
      surfaces: 2,
      owned_surfaces: 1,
      unowned_surfaces: 1,
    },
  },
};

describe("context topology lens", () => {
  it("orients a fresh user on exact workload beacons until survey terrain is admitted", () => {
    const generated = {
      workload_id: "context-anchor-survey",
      workload_revision: 1,
      title: "Survey project context anchors",
      source: "sys/context-anchor-survey/workload.yaml",
      source_digest: "blake3:survey-package",
      object_path: "objects/survey-package.yaml",
      bytes: 5_103,
      generation: {
        kind: "coding_harness" as const,
        producer: "codex",
        producer_revision: "gpt-5",
      },
      workload: {
        id: "context-anchor-survey",
        revision: 1,
        semantic_digest: "blake3:survey-workload",
      },
      graph: {
        id: "context-anchor-survey.graph",
        revision: 1,
        semantic_digest: "blake3:survey-graph",
      },
      scenario_suite: {
        id: "context-anchor-survey.scenarios",
        revision: 1,
        semantic_digest: "blake3:survey-scenarios",
      },
    };
    const fresh: WorkloadList = {
      ...portfolio,
      catalog: { ...portfolio.catalog, admitted_count: 0 },
      workloads: [],
      revision: {
        schema: "rey.workload-revision-status.v1",
        state: "working",
        head: null,
        index: null,
        working: {
          schema: "rey.workload-admission-snapshot.v1",
          snapshot_revision: "blake3:fresh-working",
          packages: [generated],
          ignore: null,
        },
        staged: {
          schema: "rey.workload-change-set.v1",
          source_label: "HEAD",
          target_label: "INDEX",
          source_revision: null,
          target_revision: null,
          assessment: "equal",
          inserted: 0,
          deleted: 0,
          modified: 0,
          changes: [],
        },
        unstaged: {
          schema: "rey.workload-change-set.v1",
          source_label: "INDEX",
          target_label: "WORKING",
          source_revision: null,
          target_revision: "blake3:fresh-working",
          assessment: "different",
          inserted: 1,
          deleted: 0,
          modified: 0,
          changes: [
            {
              workload_id: "context-anchor-survey",
              change_kind: "inserted",
              source_revision: null,
              target_revision: "blake3:survey-package",
            },
          ],
        },
        drafts: [],
        commit_ready: false,
        qualification_omissions: [],
        admission_boundary: "human approval required",
      },
    };

    for (const zoom of [
      WORLD_LENS_ZOOM,
      DEFAULT_LENS_ZOOM,
      LANDSCAPE_LENS_ZOOM,
      OBJECT_LENS_ZOOM,
    ]) {
      const scene = buildTopologyScene(fresh, zoom);
      expect(scene.regime).toBe("world");
      expect(scene.nodes).toHaveLength(0);
      expect(scene.globe).toMatchObject({
        schema: "rey.explore-orientation-globe.v1",
        posture: "orientation",
        source_revision: "blake3:fresh-working",
      });
    }
    expect(buildTopologyScene(fresh, WORLD_LENS_ZOOM).globe?.beacons).toEqual([
      expect.objectContaining({
        workload_id: "context-anchor-survey",
        state: "working",
        mapping_role: "survey",
        source: "sys/context-anchor-survey/workload.yaml",
        source_revision: "blake3:survey-package",
      }),
    ]);

    const admittedButUnrun: WorkloadList = {
      ...fresh,
      catalog: { ...fresh.catalog, admitted_count: 1 },
      workloads: [
        {
          ...workload("context-anchor-survey"),
          title: "Survey project context anchors",
          qualification: "qualified",
          failed: 0,
          passed: 3,
        },
      ],
      revision: {
        ...fresh.revision!,
        state: "clean",
        head: {
          schema: "rey.workload-commit.v1",
          commit_id: "blake3:head",
          sequence: 1,
          parent_commit_id: null,
          committed_at_unix: 1,
          message: "Admit survey",
          snapshot: fresh.revision!.working,
          qualification_ids: ["blake3:qualification"],
        },
        index: null,
        staged: fresh.revision!.staged,
        unstaged: fresh.revision!.staged,
      },
    };
    const unrun = buildTopologyScene(admittedButUnrun, OBJECT_LENS_ZOOM);
    expect(unrun.regime).toBe("world");
    expect(unrun.bearing.label).toBe("SURVEY RUN REQUIRED");
    expect(unrun.globe?.beacons).toContainEqual(
      expect.objectContaining({
        workload_id: "context-anchor-survey",
        state: "admitted",
      }),
    );
  });

  it("moves through every semantic regime without a control step skipping one", () => {
    expect(lensRegimeForZoom(WORLD_LENS_ZOOM)).toBe("world");
    expect(lensRegimeForZoom(DEFAULT_LENS_ZOOM)).toBe("atlas");
    expect(lensRegimeForZoom(LANDSCAPE_LENS_ZOOM)).toBe("landscape");
    expect(lensRegimeForZoom(NEIGHBORHOOD_LENS_ZOOM)).toBe("neighborhoods");
    expect(lensRegimeForZoom(OBJECT_LENS_ZOOM)).toBe("objects");
    expect(lensRegimeForZoom(EVIDENCE_LENS_ZOOM)).toBe("evidence");
    expect(stepLensZoom(DEFAULT_LENS_ZOOM, 1)).toBe(LANDSCAPE_LENS_ZOOM);
    expect(stepLensZoom(DEFAULT_LENS_ZOOM, -1)).toBe(WORLD_LENS_ZOOM);
    expect(stepLensZoom(WORLD_LENS_ZOOM, 1)).toBe(DEFAULT_LENS_ZOOM);
    expect(stepLensZoom(LANDSCAPE_LENS_ZOOM, 1)).toBe(NEIGHBORHOOD_LENS_ZOOM);
    expect(stepLensZoom(NEIGHBORHOOD_LENS_ZOOM, 1)).toBe(OBJECT_LENS_ZOOM);
    expect(stepLensZoom(OBJECT_LENS_ZOOM, 1)).toBe(EVIDENCE_LENS_ZOOM);
    expect(stepLensZoom(OBJECT_LENS_ZOOM, -1)).toBe(NEIGHBORHOOD_LENS_ZOOM);
    expect(lensRegimeForZoom(0.43, "atlas")).toBe("atlas");
  });

  it("clamps the optical coordinate to the declared bounded range", () => {
    expect(clampLensZoom(-4)).toBe(MIN_LENS_ZOOM);
    expect(clampLensZoom(8)).toBe(MAX_LENS_ZOOM);
  });

  it("changes object families while retaining exact portfolio identities", () => {
    const landscape = buildTopologyScene(portfolio, LANDSCAPE_LENS_ZOOM);
    const neighborhoods = buildTopologyScene(portfolio, NEIGHBORHOOD_LENS_ZOOM);
    const objects = buildTopologyScene(
      portfolio,
      OBJECT_LENS_ZOOM,
      "workload:rey.example",
    );

    expect(landscape.regime).toBe("landscape");
    expect(landscape.nodes.map((node) => node.family)).toContain("WORKLOADS");
    expect(neighborhoods.regime).toBe("neighborhoods");
    expect(neighborhoods.nodes).toContainEqual(
      expect.objectContaining({
        focus_id: "workload:rey.example",
        label: "rey.example",
      }),
    );
    expect(objects.regime).toBe("objects");
    expect(objects.nodes).toContainEqual(
      expect.objectContaining({
        family: "COMPUTE GRAPH",
        label: "rey.example.graph",
      }),
    );
    expect(objects.nodes).toContainEqual(
      expect.objectContaining({
        family: "WORKLOAD",
        workload_id: "rey.example",
      }),
    );
  });

  it("derives all six levels from admitted patches and preserves unknown region states", () => {
    const retainedAtlas = atlasFor(surveyPatch);
    const patchPortfolio: WorkloadList = {
      ...portfolio,
      semantic_atlas: retainedAtlas,
      semantic_atlas_history: [retainedAtlas],
      semantic_atlas_deltas: [
        {
          schema: "rey.semantic-atlas-delta.v1",
          delta_id: "atlas-delta:1",
          source_revision: "atlas:empty",
          target_revision: retainedAtlas.atlas_revision,
          inserted: 1,
          removed: 0,
          moved: 0,
          interest_changed: 0,
          merged: 0,
          split: 0,
          region_changes: [],
          cluster_changes: [],
        },
      ],
      workloads: [
        {
          ...portfolio.workloads[0]!,
          topography_results: 1,
          topography_revision: "topography:1",
          topography_coverage: surveyPatch.coverage,
          topography_frontier_rows: 1,
          topography_patch: surveyPatch,
          topography_projection: projectionFor(surveyPatch),
        },
      ],
    };
    const world = buildTopologyScene(patchPortfolio, WORLD_LENS_ZOOM);
    const atlas = buildTopologyScene(patchPortfolio, DEFAULT_LENS_ZOOM);
    const landscape = buildTopologyScene(patchPortfolio, LANDSCAPE_LENS_ZOOM);
    const neighborhoods = buildTopologyScene(
      patchPortfolio,
      NEIGHBORHOOD_LENS_ZOOM,
      "topography:rey.example",
    );
    expect(world.bearing.detail).toContain("atlas atlas:empty → atlas:1");
    const objects = buildTopologyScene(
      patchPortfolio,
      OBJECT_LENS_ZOOM,
      "anchor:rey.example:anchor:readme",
    );
    const evidence = buildTopologyScene(
      patchPortfolio,
      EVIDENCE_LENS_ZOOM,
      "anchor:rey.example:anchor:readme",
    );

    expect(atlas.regions.map((region) => region.tone)).toEqual(
      expect.arrayContaining([
        "healthy",
        "unknown",
        "omitted",
        "stale",
        "unsupported",
        "frontier",
      ]),
    );
    expect(atlas.contours).toHaveLength(7);
    expect(atlas.points).toContainEqual(
      expect.objectContaining({
        coordinate_uri: "rey+local://file/README.md?revision=source%3A1",
        kind: "anchor",
      }),
    );
    const atlasReadme = atlas.points.find(
      (point) => point.label === "README.md",
    );
    const landscapeReadme = landscape.points.find(
      (point) => point.label === "README.md",
    );
    const neighborhoodReadme = neighborhoods.points.find(
      (point) => point.label === "README.md",
    );
    expect(landscapeReadme).toMatchObject({
      x: atlasReadme?.x,
      y: atlasReadme?.y,
    });
    expect(neighborhoodReadme).toMatchObject({
      x: atlasReadme?.x,
      y: atlasReadme?.y,
    });
    expect(landscape.world).toEqual(atlas.world);
    expect(neighborhoods.world).toEqual(atlas.world);
    expect(objects.world).toEqual(atlas.world);
    expect(evidence.world).toEqual(atlas.world);
    expect(world.terrain_fields[0]).toMatchObject({
      working_set_id: "reference:world",
      field_cells: 2501,
    });
    expect(world.globe).toMatchObject({
      schema: "rey.semantic-globe-scene.v1",
      posture: "semantic_atlas",
      source_revision: "atlas:1",
    });
    expect(world.globe?.regions[0]).toMatchObject({
      workload_id: "rey.example",
      longitude_degrees: 12.5,
      latitude_degrees: 24.25,
    });
    expect(atlas.globe).toBeNull();
    expect(atlas.terrain_fields[0]).toMatchObject({
      working_set_id: "reference:atlas",
      field_cells: 2501,
    });
    expect(landscape.terrain_fields[0]?.working_set_id).toBe(
      "reference:landscape",
    );
    expect(neighborhoods.terrain_fields[0]?.working_set_id).toBe(
      "reference:neighborhoods",
    );
    expect(objects.terrain_fields[0]?.working_set_id).toBe("reference:objects");
    expect(evidence.terrain_fields[0]?.working_set_id).toBe(
      "reference:evidence",
    );
    expect(atlas.terrain_programs[0]?.projection.terrain_program).toMatchObject(
      {
        schema: "rey.terrain-program.v1",
        working_set: { max_cells: 65025, max_bytes: 3576375 },
      },
    );
    expect(world.terrain_fields[0]?.grid.bounds).toEqual(
      neighborhoods.terrain_fields[0]?.grid.bounds,
    );
    expect(objects.label).toBe("ANCHOR OBJECTS");
    expect(objects.nodes.map((node) => node.family)).toEqual(
      expect.arrayContaining([
        "ADMITTED PATCH",
        "SOURCE REVISION",
        "LOCATOR OUTCOME",
        "DIRECTED PATCH",
      ]),
    );
    expect(evidence.regime).toBe("evidence");
    expect(evidence.detail).toContain("patch patch:1");
    expect(evidence.omissions).toContain(
      "2 candidate limit omitted: bounded fixture",
    );
    expect(atlas.omissions).toContain(
      "relief height is admitted anchor-sample influence, not inferred semantic similarity",
    );
  });

  it("fails closed on an absent or mismatched projection packet", () => {
    const withoutPacket: WorkloadList = {
      ...portfolio,
      workloads: [
        {
          ...portfolio.workloads[0]!,
          topography_results: 1,
          topography_revision: surveyPatch.topography_revision,
          topography_patch: surveyPatch,
          topography_projection: null,
        },
      ],
    };
    const mismatchedPacket: WorkloadList = {
      ...withoutPacket,
      workloads: [
        {
          ...withoutPacket.workloads[0]!,
          topography_projection: {
            ...projectionFor(surveyPatch),
            source_patch_id: "patch:other",
          },
        },
      ],
    };

    expect(buildTopologyScene(withoutPacket, DEFAULT_LENS_ZOOM).terrain).toBe(
      false,
    );
    expect(
      buildTopologyScene(mismatchedPacket, DEFAULT_LENS_ZOOM).terrain,
    ).toBe(false);
  });

  it("uses packet extent, objects, validity, and omissions for the reference scene", () => {
    const projection = projectionFor(surveyPatch);
    const packetDirected: WorkloadList = {
      ...portfolio,
      workloads: [
        {
          ...portfolio.workloads[0]!,
          topography_results: 1,
          topography_revision: surveyPatch.topography_revision,
          topography_patch: surveyPatch,
          topography_projection: {
            ...projection,
            extent: { ...projection.extent, width: 1800 },
            objects: projection.objects.filter(
              (object) => object.source_id !== "anchor:readme",
            ),
            validity: projection.validity.filter(
              (region) => region.state !== "unsupported",
            ),
            omissions: [
              ...projection.omissions,
              {
                kind: "fixture_limit",
                subject: "packet",
                omitted_count: 1,
                reason: "packet-directed fixture",
              },
            ],
          },
        },
      ],
    };
    const atlas = buildTopologyScene(packetDirected, DEFAULT_LENS_ZOOM);

    expect(atlas.world.width).toBe(1800);
    expect(
      atlas.points.filter((point) => point.kind === "anchor"),
    ).toHaveLength(1);
    expect(atlas.regions.map((region) => region.tone)).not.toContain(
      "unsupported",
    );
    expect(atlas.omissions).toContain(
      "1 fixture limit omitted: packet-directed fixture",
    );
  });

  it("derives weather and hydrology without projecting seed edges as paths", () => {
    const patchPortfolio: WorkloadList = {
      ...portfolio,
      workloads: [
        {
          ...portfolio.workloads[0]!,
          topography_results: 1,
          topography_revision: surveyPatch.topography_revision,
          topography_coverage: surveyPatch.coverage,
          topography_frontier_rows: surveyPatch.frontier.length,
          topography_patch: surveyPatch,
          topography_projection: projectionFor(surveyPatch),
        },
      ],
    };
    const world = buildTopologyScene(patchPortfolio, WORLD_LENS_ZOOM);
    const probe = buildTopologyScene(
      patchPortfolio,
      WORLD_LENS_ZOOM,
      "frontier:rey.example:frontier:1",
    );
    const anchorFocus = world.points.find(
      (point) => point.kind === "anchor",
    )!.focus_id;
    const objects = buildTopologyScene(
      patchPortfolio,
      OBJECT_LENS_ZOOM,
      anchorFocus,
    );
    const evidence = buildTopologyScene(
      patchPortfolio,
      EVIDENCE_LENS_ZOOM,
      anchorFocus,
    );

    expect(world.regime).toBe("world");
    expect(world.landforms.map((landform) => landform.kind)).toEqual([
      "charted",
      "horizon",
    ]);
    expect(world.natural_features.map((feature) => feature.kind)).toEqual(
      expect.arrayContaining(["stream", "river", "weather_front"]),
    );
    expect(world.edges).toEqual([]);
    expect(objects.edges).toEqual([]);
    expect(evidence.edges).toEqual([]);
    expect(world.natural_features).not.toContainEqual(
      expect.objectContaining({ id: expect.stringContaining("edge:") }),
    );
    expect(world.points).toContainEqual(
      expect.objectContaining({
        kind: "frontier",
        signal: "VERIFY ABSENCE OR REPAIR REFERENCE",
      }),
    );
    expect(probe.bearing).toMatchObject({
      status: "probe_required",
      sampled_conditions: 2,
      unresolved_boundaries: 1,
    });
    expect(probe.bearing.detail).toContain("supplies no route");
  });

  it("discloses folded evidence instead of implying a complete object view", () => {
    const scene = buildTopologyScene(
      portfolio,
      OBJECT_LENS_ZOOM,
      "attention:row:1",
    );
    expect(scene.omissions).toEqual(["1 evidence references folded"]);
  });

  it("projects admitted generator provenance as an exact agent neighborhood", () => {
    const generated: WorkloadList = {
      ...portfolio,
      workloads: [
        {
          ...portfolio.workloads[0]!,
          provenance: {
            origin: "workspace_package",
            source: "workloads/example/workload.yaml",
            source_digest: "package:1",
            generation: {
              kind: "coding_harness",
              producer: "codex",
              producer_revision: "gpt-5",
            },
            admission: { state: "accepted", scenario_oracle: "frozen" },
          },
        },
      ],
    };

    const neighborhoods = buildTopologyScene(
      generated,
      NEIGHBORHOOD_LENS_ZOOM,
      "cluster:agents",
    );
    const objects = buildTopologyScene(
      generated,
      OBJECT_LENS_ZOOM,
      "agent:coding_harness:codex@gpt-5",
    );

    expect(neighborhoods.nodes).toContainEqual(
      expect.objectContaining({ family: "AGENT", label: "codex" }),
    );
    expect(objects.label).toBe("AGENT OBJECTS");
    expect(objects.nodes).toContainEqual(
      expect.objectContaining({ family: "REVISION", label: "gpt-5" }),
    );
  });

  it("bounds dense anchor relief and expands the atlas for additional admitted scenes", () => {
    const anchors: TopographyPatch["anchors"] = Array.from(
      { length: 70 },
      (_, index) => {
        const source = `source:${index}`;
        const anchorCoordinate = {
          ...coordinate,
          binding_id: `binding:${index}`,
          coordinate: `rey+local://file/docs%2Fanchor-${index}.md?revision=${encodeURIComponent(source)}`,
          source_revision: source,
        };
        return {
          anchor_id: `anchor:${index}`,
          coordinate: anchorCoordinate,
          kind: "file" as const,
          label: `docs/anchor-${index}.md`,
          source_revision: source,
        };
      },
    );
    const edges: TopographyPatch["edges"] = anchors
      .slice(1)
      .map((anchor, index) => ({
        edge_id: `edge:${index}`,
        source_coordinate: anchors[0]!.coordinate.coordinate,
        target_coordinate: anchor.coordinate.coordinate,
        kind: "references" as const,
        locator: anchor.label,
        evidence_revision: anchors[0]!.source_revision,
      }));
    const densePatch: TopographyPatch = { ...surveyPatch, anchors, edges };
    const first = {
      ...workload("rey.example"),
      topography_results: 1,
      topography_revision: densePatch.topography_revision,
      topography_coverage: densePatch.coverage,
      topography_patch: densePatch,
      topography_projection: projectionFor(densePatch),
    };
    const secondPatch: TopographyPatch = {
      ...densePatch,
      patch_id: "patch:2",
      topography_revision: "topography:2",
      workload: identity("rey.second"),
    };
    const second = {
      ...workload("rey.second"),
      topography_results: 1,
      topography_revision: secondPatch.topography_revision,
      topography_coverage: secondPatch.coverage,
      topography_patch: secondPatch,
      topography_projection: projectionFor(secondPatch),
    };

    const atlas = buildTopologyScene(
      { ...portfolio, workloads: [first, second] },
      DEFAULT_LENS_ZOOM,
    );

    expect(atlas.world).toEqual({ width: 3000, height: 1000 });
    expect(
      atlas.points.filter((point) => point.kind === "anchor"),
    ).toHaveLength(128);
    expect(new Set(atlas.points.map((point) => point.id)).size).toBe(
      atlas.points.length,
    );
    expect(atlas.contours).toHaveLength(14);
    expect(atlas.omissions).toEqual(
      expect.arrayContaining([
        "6 anchor POIs folded from rey.example",
        "6 anchor POIs folded from rey.second",
      ]),
    );
  });
});

const identity = (id: string) => ({
  id,
  revision: 1,
  semantic_digest: `${id}:digest`,
});

function atlasFor(patch: TopographyPatch): SemanticAtlas {
  return {
    schema: "rey.semantic-atlas.v1",
    atlas_id: "atlas:1",
    atlas_revision: "atlas:1",
    compiler: identity("rey.semantic-atlas.polar-cluster"),
    coordinate_system: {
      kind: "synthetic_semantic_sphere",
      axes: ["semantic_longitude", "semantic_latitude"],
      unit: "microdegree",
      longitude_range_microdegrees: [-180_000_000, 180_000_000],
      latitude_range_microdegrees: [-90_000_000, 90_000_000],
      wraps_longitude: true,
      authority: "synthetic admitted survey layout; not Earth coordinates",
      earth_crs: null,
    },
    layout_policy: {
      clustering: "fixture clustering",
      placement: "fixture polar placement",
      recluster_trigger: "admitted source revision change",
      zoom_rule: "zoom selects retained level of detail and never reclusters",
      distance_claim: "no semantic distance claim",
    },
    submitted_sources: 1,
    sources: [
      {
        region_id: "region:1",
        workload_id: "rey.example",
        source_patch_id: patch.patch_id,
        source_topography_revision: patch.topography_revision,
        complete: patch.complete,
        workspace_anchors: 1,
        file_anchors: 1,
        document_anchors: 0,
        external_resource_anchors: 0,
        requested_seeds: patch.coverage.requested_seeds,
        surveyed_seeds: patch.coverage.surveyed_seeds,
        candidates: patch.coverage.unique_candidates,
        frontier_rows: patch.frontier.length,
      },
    ],
    clusters: [
      {
        cluster_id: "cluster:1",
        semantic_longitude_microdegrees: 12_500_000,
        semantic_latitude_microdegrees: 24_250_000,
        angular_radius_microdegrees: 8_000_000,
        member_region_ids: ["region:1"],
        dominant_feature: "file",
      },
    ],
    regions: [
      {
        region_id: "region:1",
        cluster_id: "cluster:1",
        workload_id: "rey.example",
        source_patch_id: patch.patch_id,
        source_topography_revision: patch.topography_revision,
        semantic_longitude_microdegrees: 12_500_000,
        semantic_latitude_microdegrees: 24_250_000,
        angular_radius_microdegrees: 5_500_000,
        anchor_count: patch.anchors.length,
        frontier_rows: patch.frontier.length,
        complete: patch.complete,
        dominant_feature: "file",
      },
    ],
    limits: {
      max_regions: 128,
      max_world_clusters: 16,
      max_members_per_cluster: 128,
      max_omissions: 32,
    },
    complete: true,
    omissions: [],
    lineage: [],
  };
}
const coordinate = {
  schema: "rey.coordinate-binding.v1",
  binding_id: "binding:readme",
  profile: "local_standalone" as const,
  provider: identity("fixture-provider"),
  coordinate: "rey+local://file/README.md?revision=source%3A1",
  identity_class: "revision_bound" as const,
  source_revision: "source:1",
  retention: "fixture",
};
const workspaceCoordinate = {
  ...coordinate,
  binding_id: "binding:workspace",
  coordinate: "rey+local://workspace/current?revision=workspace%3A1",
  source_revision: "workspace:1",
};
const surveyPatch: TopographyPatch = {
  schema: "rey.topography-patch.v1",
  patch_id: "patch:1",
  topography_revision: "topography:1",
  prior_topography_revision: "topography:0",
  workload: identity("rey.example"),
  graph: identity("rey.example.graph"),
  scenario: identity("rey.example.scenario"),
  campaign_id: "campaign:1",
  execution_id: "execution:1",
  operation: identity("survey"),
  implementation: identity("survey-implementation"),
  provider: identity("fixture-provider"),
  capability_snapshot_id: "capability:1",
  complete: false,
  seeds: [
    {
      path: "README.md",
      state: "surveyed",
      source_revision: "source:1",
      coordinate,
      candidate_count: 2,
      detail: "surveyed fixture",
    },
  ],
  candidates: [
    {
      candidate_id: "candidate:1",
      seed_coordinate: coordinate.coordinate,
      seed_revision: "source:1",
      raw: "README.md",
      relationship: "references",
      duplicate: false,
    },
  ],
  resolutions: [
    {
      resolution_id: "resolution:1",
      candidate: "README.md",
      status: "resolved",
      coordinate,
      source_revision: "source:1",
      complete: true,
      detail: "resolved fixture",
    },
  ],
  anchors: [
    {
      anchor_id: "anchor:workspace",
      coordinate: workspaceCoordinate,
      kind: "workspace",
      label: "workspace survey boundary",
      source_revision: "workspace:1",
    },
    {
      anchor_id: "anchor:readme",
      coordinate,
      kind: "file",
      label: "README.md",
      source_revision: "source:1",
    },
  ],
  edges: [
    {
      edge_id: "edge:workspace-readme",
      source_coordinate: workspaceCoordinate.coordinate,
      target_coordinate: coordinate.coordinate,
      kind: "contains",
      locator: "README.md",
      evidence_revision: "source:1",
    },
  ],
  regions: [
    "surveyed",
    "unexplored",
    "omitted",
    "stale",
    "unsupported",
    "frontier",
  ].map((state, index) => ({
    region_id: `region:${state}`,
    coordinate: `region:${index}`,
    state: state as TopographyPatch["regions"][number]["state"],
    surveyed_seeds: state === "surveyed" ? 1 : 0,
    candidate_count: 0,
    detail: `${state} fixture`,
  })),
  coverage: {
    requested_seeds: 1,
    surveyed_seeds: 1,
    surveyed_empty_seeds: 0,
    missing_seeds: 0,
    omitted_seeds: 0,
    candidates: 1,
    unique_candidates: 1,
    resolved_candidates: 1,
    unresolved_candidates: 0,
  },
  frontier: [
    {
      row_id: "frontier:1",
      source_coordinate: coordinate.coordinate,
      locator: "missing.md",
      status: "missing",
      reason: "missing fixture",
    },
  ],
  omissions: [
    {
      kind: "candidate_limit",
      subject: "README.md",
      omitted_count: 2,
      reason: "bounded fixture",
    },
  ],
  lineage: [{ kind: "seed", identity: "README.md", revision: "source:1" }],
  delta: {
    delta_id: "delta:1",
    source_revision: "topography:0",
    target_revision: "topography:1",
    inserted: 1,
    deleted: 0,
    modified: 0,
  },
};

function projectionFor(patch: TopographyPatch): ProjectionPacket {
  const orderedAnchors = [...patch.anchors].sort((left, right) => {
    if (left.kind === "workspace") return -1;
    if (right.kind === "workspace") return 1;
    return left.coordinate.coordinate.localeCompare(
      right.coordinate.coordinate,
    );
  });
  const foldedAnchors = Math.max(0, orderedAnchors.length - 64);
  const foldedFrontier = Math.max(0, patch.frontier.length - 6);
  const degradation = [
    ...(foldedAnchors > 0
      ? [
          {
            kind: "anchor_limit",
            omitted_count: foldedAnchors,
            reason: "anchor scene objects exceed the declared projection limit",
          },
        ]
      : []),
    ...(foldedFrontier > 0
      ? [
          {
            kind: "frontier_limit",
            omitted_count: foldedFrontier,
            reason:
              "frontier scene objects exceed the declared projection limit",
          },
        ]
      : []),
  ];
  const contract = (id: string) => identity(id);
  return {
    schema: "rey.projection-packet.v1",
    packet_id: `projection:${patch.patch_id}`,
    source_patch_id: patch.patch_id,
    source_topography_revision: patch.topography_revision,
    projection_basis: {
      contract: contract("rey.projection.anchor-orientation"),
      input_dimensions: ["anchor.coordinate", "region.validity"],
      output_dimensions: ["scene_x", "scene_y", "relative_elevation"],
      parameters: {
        terrain_width: "1500",
        terrain_height: "1000",
        elevation_scale_ratio: "0.085",
        terrain_evaluation: "absolute_coordinate_procedural",
      },
      normalization: "per-chart relative anchor prominence",
      random_seed: null,
      distance_semantics:
        "synthetic orientation distance; not language or semantic distance",
      neighborhood_semantics: "ordered anchor rings",
      distortion: "ring placement distorts source-space distance",
      stable_coordinate_rule: "semantic coordinates remain stable",
    },
    scene_compiler: contract("rey.projection.topography-scene"),
    extent: { width: 1500, height: 1000, unit: "synthetic_scene_unit" },
    terrain_program: {
      schema: "rey.terrain-program.v1",
      evaluator: contract("rey.projection.procedural-terrain"),
      seed: 42,
      bands: [
        {
          band_id: "macro",
          wavelength_scene_units: 420,
          amplitude_microunits: 210000,
          octaves: 2,
          minimum_samples_per_wavelength: 8,
          detail_authority: "derived macro fixture",
        },
        {
          band_id: "meso",
          wavelength_scene_units: 105,
          amplitude_microunits: 72000,
          octaves: 3,
          minimum_samples_per_wavelength: 7,
          detail_authority: "derived meso fixture",
        },
        {
          band_id: "micro",
          wavelength_scene_units: 24,
          amplitude_microunits: 18000,
          octaves: 2,
          minimum_samples_per_wavelength: 6,
          detail_authority: "presentation-only micro fixture",
        },
      ],
      working_set: {
        max_columns: 255,
        max_rows: 255,
        max_cells: 65025,
        bytes_per_cell: 55,
        max_bytes: 3576375,
        target_sample_spacing_pixels: 4,
        overscan_samples: 3,
        recenter_rule: "camera-relative fixture",
      },
      coordinate_rule: "absolute fixture coordinates",
      validity_rule: "fixture support only",
      detail_rule: "camera selects fixture bands without changing evidence",
    },
    objects: [
      ...orderedAnchors.slice(0, 64).map((anchor) => ({
        object_id: `anchor:${anchor.anchor_id}`,
        source_id: anchor.anchor_id,
        kind: "anchor" as const,
        anchor_kind: anchor.kind,
        frontier_status: null,
        coordinate: anchor.coordinate.coordinate,
        label: anchor.label,
        detail: "admitted topography anchor",
        source_revision: anchor.source_revision,
      })),
      ...patch.frontier.slice(0, 6).map((row) => ({
        object_id: `frontier:${row.row_id}`,
        source_id: row.row_id,
        kind: "frontier" as const,
        anchor_kind: null,
        frontier_status: row.status,
        coordinate: null,
        label: row.locator,
        detail: row.reason,
        source_revision: patch.topography_revision,
      })),
    ],
    validity: patch.regions.map((region) => ({
      region_id: region.region_id,
      coordinate: region.coordinate,
      state: region.state,
      detail: region.detail,
      source_revision: patch.topography_revision,
    })),
    field_channels: [
      ["validity", "mask"],
      ["elevation", "scalar"],
      ["rainfall", "scalar"],
      ["flow_direction", "vector"],
      ["flow_accumulation", "scalar"],
      ["erosion", "scalar"],
      ["normal", "vector"],
      ["curvature", "scalar"],
      ["material", "vector"],
    ].map(([id, kind]) => ({
      id: id!,
      kind: kind as "mask" | "scalar" | "vector",
      semantics: `${id} fixture channel`,
      units: "relative",
      normalization: "fixture",
      source_revision: patch.topography_revision,
      implementation: contract(`rey.projection.${id}`),
    })),
    layers: [
      {
        id: "validity",
        authority: "evidence",
        semantics: "survey validity",
        source_revision: patch.topography_revision,
      },
      {
        id: "relief",
        authority: "derived",
        semantics: "anchor-only relative terrain",
        source_revision: patch.topography_revision,
      },
    ],
    excluded_source_relationships: patch.edges.length,
    limits: {
      max_anchor_objects: 64,
      max_frontier_objects: 6,
      max_validity_regions: 256,
      max_field_channels: 12,
      max_terrain_bands: 8,
      max_layers: 8,
      max_omissions: 1032,
      max_working_set_cells: 65025,
      max_working_set_bytes: 3576375,
      max_contours: 7,
      max_natural_features: 96,
      max_labels: 70,
    },
    complete: patch.complete && degradation.length === 0,
    degradation,
    omissions: [
      ...patch.omissions,
      {
        kind: "semantic_boundary",
        subject: "relief",
        omitted_count: 0,
        reason:
          "relief height is admitted anchor-sample influence, not inferred semantic similarity",
      },
      {
        kind: "semantic_boundary",
        subject: "natural_features",
        omitted_count: 0,
        reason:
          "streams, rivers, weather fronts, and erosion are deterministic survey-field projections, not retained paths or source relationships",
      },
      ...degradation.map((item) => ({ ...item, subject: "projection" })),
    ],
    lineage: [
      {
        kind: "source_patch",
        identity: patch.patch_id,
        revision: patch.topography_revision,
      },
    ],
  };
}
