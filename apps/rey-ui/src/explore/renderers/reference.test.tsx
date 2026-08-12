import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { TopologyScene } from "../../topology";
import { ReferenceMapReading, ReferenceRenderer } from "./reference";

const terrainScene = {
  regime: "world",
  label: "CONTEXT WORLD",
  detail: "one admitted chart",
  focus_id: "cluster:portfolio",
  regions: [
    {
      id: "unexplored",
      label: "UNEXPLORED BEYOND SURVEY",
      detail: "no admitted terrain claim",
      x: 0,
      y: 0,
      width: 1500,
      height: 1000,
      tone: "unknown",
      variant: "map-zone",
    },
  ],
  landforms: [
    {
      id: "charted",
      path: "M100,100L200,100L200,200Z",
      kind: "charted",
      label: "charted",
      detail: "admitted anchor extent",
      tone: "healthy",
    },
  ],
  contours: [
    {
      id: "contour",
      path: "M120,120L180,180",
      level: 4,
      threshold: 0.5,
      anchor_count: 1,
    },
  ],
  natural_features: [
    {
      id: "stream",
      path: "M140,140L160,180",
      kind: "stream",
      label: "PROJECTED HEADWATERS",
      detail: "derived runoff; not a source relationship",
      intensity: 1,
      workload_id: "survey",
    },
  ],
  points: [],
  nodes: [],
  edges: [
    {
      id: "forbidden-source-edge",
      from: "source",
      to: "target",
      kind: "contains",
      label: "must not become geography",
    },
  ],
  omissions: ["source relationships excluded from terrain"],
  bearing: {
    status: "world",
    label: "SURVEY WEATHER",
    detail: "no path has been discovered or built",
    sampled_conditions: 1,
    unresolved_boundaries: 0,
  },
  world: { width: 1500, height: 1000 },
  fit_world: { width: 1500, height: 1000 },
  terrain: true,
  terrain_fields: [],
  terrain_programs: [],
  globe: null,
} satisfies TopologyScene;

describe("reference renderer", () => {
  it("preserves the bounded terrain manifest without source-edge geography", () => {
    const markup = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        scene={terrainScene}
      />,
    );

    expect(markup).toContain('data-renderer="reference"');
    expect(markup).toContain('data-world-geometry="charted"');
    expect(markup).toContain('data-relief-level="4"');
    expect(markup).toContain('data-natural-feature="stream"');
    expect(markup).not.toContain("topology-arrow");
    expect(markup).toContain("not a source relationship");
  });

  it("renders the admitted semantic atlas as a spherical World lens", () => {
    const globeScene: TopologyScene = {
      ...terrainScene,
      globe: {
        schema: "rey.semantic-globe-scene.v1",
        posture: "semantic_atlas",
        globe_id: "atlas:1",
        source_revision: "atlas:1",
        compiler_revision: "compiler:1",
        coordinate_authority: "synthetic semantic sphere; not Earth CRS84",
        clusters: [
          {
            id: "cluster:1",
            longitude_degrees: 0,
            latitude_degrees: 0,
            angular_radius_degrees: 8,
            member_count: 1,
            dominant_feature: "file",
          },
        ],
        beacons: [],
        regions: [
          {
            id: "region:1",
            cluster_id: "cluster:1",
            focus_id: "anchor:survey:workspace",
            workload_id: "survey",
            label: "survey",
            detail: "two admitted anchors",
            longitude_degrees: 0,
            latitude_degrees: 0,
            angular_radius_degrees: 5.5,
            tone: "healthy",
          },
        ],
      },
    };
    const markup = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        scene={globeScene}
      />,
    );

    expect(markup).toContain('data-atlas-revision="atlas:1"');
    expect(markup).toContain('data-semantic-region="region:1"');
    expect(markup).toContain("Synthetic semantic longitude and latitude");
    expect(markup).not.toContain('data-world-geometry="charted"');
    expect(markup).not.toContain('data-natural-feature="stream"');
  });

  it("renders a pre-survey workload as a consent beacon rather than terrain", () => {
    const orientationScene: TopologyScene = {
      ...terrainScene,
      label: "PROJECT ORIENTATION",
      focus_id: "beacon:context-anchor-survey",
      globe: {
        schema: "rey.explore-orientation-globe.v1",
        posture: "orientation",
        globe_id: "orientation:working",
        source_revision: "blake3:working",
        compiler_revision: "rey.explore.orientation-globe@1",
        coordinate_authority: "presentation only",
        clusters: [],
        regions: [],
        beacons: [
          {
            id: "workload-beacon:context-anchor-survey",
            focus_id: "beacon:context-anchor-survey",
            workload_id: "context-anchor-survey",
            label: "Survey project context anchors",
            detail: "WORKING / exact file",
            source: "sys/context-anchor-survey/workload.yaml",
            source_revision: "blake3:package",
            producer: "coding harness / codex@gpt-5",
            state: "working",
            mapping_role: "survey",
            next_step: "review the exact file and consent",
            longitude_degrees: 12,
            latitude_degrees: 8,
            tone: "attention",
          },
        ],
      },
      bearing: {
        status: "consent_required",
        label: "SURVEY CONSENT REQUIRED",
        detail: "review the exact file",
        sampled_conditions: 0,
        unresolved_boundaries: 1,
      },
      landforms: [],
      contours: [],
      natural_features: [],
    };
    const markup = renderToStaticMarkup(
      <>
        <ReferenceRenderer
          layers={{ relief: true, water: true, weather: true, probes: true }}
          onFocus={() => undefined}
          scene={orientationScene}
        />
        <ReferenceMapReading scene={orientationScene} />
      </>,
    );

    expect(markup).toContain('data-globe-posture="orientation"');
    expect(markup).toContain('data-workload-beacon="context-anchor-survey"');
    expect(markup).toContain("CONTEXT SURVEY");
    expect(markup).toContain("PROJECTION FABRIC ONLY");
    expect(markup).toContain("FIRST MAPPING STEP");
    expect(markup).toContain("INSPECT EXACT WORKLOAD");
    expect(markup).toContain("REVIEW &amp; CONSENT");
    expect(markup).toContain("NO SURVEY CLAIM");
    expect(markup).not.toContain('data-world-geometry="charted"');
  });
});
