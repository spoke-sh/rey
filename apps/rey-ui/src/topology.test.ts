import { describe, expect, it } from "vitest";
import type {
  ProjectionPacket,
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
  last_run_status: "blocked",
  last_test_result_id: `test:${id}`,
});

const portfolio: WorkloadList = {
  schema: "rey.workload-list.v7",
  catalog: {
    schema: "rey.workload-catalog.v2",
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
    const patchPortfolio: WorkloadList = {
      ...portfolio,
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
      level_id: "overview",
      field_cells: 651,
    });
    expect(atlas.terrain_fields[0]).toMatchObject({
      level_id: "regional",
      field_cells: 2501,
    });
    expect(landscape.terrain_fields[0]?.level_id).toBe("regional");
    expect(neighborhoods.terrain_fields[0]).toMatchObject({
      level_id: "local",
      field_cells: 9801,
    });
    expect(objects.terrain_fields[0]?.level_id).toBe("local");
    expect(evidence.terrain_fields[0]?.level_id).toBe("local");
    expect(atlas.terrain_pyramids[0]).toMatchObject({
      total_cells: 12953,
      total_bytes: 712415,
    });
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
    schema: "rey.projection-packet.v2",
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
        grid_columns: "120",
        grid_rows: "80",
        elevation_scale_ratio: "0.085",
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
    field_pyramid: {
      schema: "rey.terrain-field-pyramid.v1",
      levels: [
        {
          level_id: "overview",
          columns: 31,
          rows: 21,
          cells: 651,
          bytes_per_cell: 55,
          total_bytes: 35805,
          sample_stride: 4,
          regimes: ["world"],
          detail_authority: "coarse anchor resampling",
        },
        {
          level_id: "regional",
          columns: 61,
          rows: 41,
          cells: 2501,
          bytes_per_cell: 55,
          total_bytes: 137555,
          sample_stride: 2,
          regimes: ["atlas", "landscape"],
          detail_authority: "regional anchor resampling",
        },
        {
          level_id: "local",
          columns: 121,
          rows: 81,
          cells: 9801,
          bytes_per_cell: 55,
          total_bytes: 539055,
          sample_stride: 1,
          regimes: ["neighborhoods", "objects", "evidence"],
          detail_authority: "local anchor resampling",
        },
      ],
      total_cells: 12953,
      total_bytes: 712415,
      stable_coordinate_rule: "nested fixture coordinates",
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
      max_field_levels: 3,
      max_layers: 8,
      max_omissions: 1032,
      max_field_cells: 9801,
      max_field_bytes: 627264,
      max_total_field_cells: 12953,
      max_total_field_bytes: 828992,
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
