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
  world_atlas_transition: null,
  atlas_landscape_transition: null,
  county_frame: null,
  county_footprint: null,
} satisfies TopologyScene;

describe("reference renderer", () => {
  it("subordinates the County support boundary and evidence chrome at Landscape", () => {
    const scene = {
      ...terrainScene,
      regime: "landscape",
      county_frame: {
        schema: "rey.county-frame.v1",
        frame_id: "county-frame:1",
        scene_id: "scene:1",
        source_bounds: {
          west_microdegrees: -123_000_000,
          south_microdegrees: 37_000_000,
          east_microdegrees: -122_000_000,
          north_microdegrees: 38_000_000,
          crosses_antimeridian: false,
        },
        source_origin: [-122_500_000, 37_500_000],
        target_origin: [0, 0, 0],
        transform_id: "county-local",
        transform_revision: 1,
        transform_digest: "transform:1",
        pitch_degrees: 88,
        yaw_degrees: 0,
        authority: "bounded test frame",
      },
      county_footprint: {
        footprint_id: "footprint:1",
        scene_id: "scene:1",
        source_object_id: "boundary/county",
        source_artifact_id: "artifact:boundary",
        source_object_revision: "object:boundary",
        native_bounds: {
          west_microdegrees: -123_000_000,
          south_microdegrees: 37_000_000,
          east_microdegrees: -122_000_000,
          north_microdegrees: 38_000_000,
          crosses_antimeridian: false,
        },
        rings: [[[-123_000_000, 38_000_000]]],
        coordinate_count: 1,
        authority: "exact admitted test footprint",
        path: "M96 72 L1104 72 L1104 648 L96 648 Z",
        screen_rings: [[]],
      },
      nodes: [
        {
          id: "road:one",
          focus_id: "road:one",
          family: "road",
          label: "CONTEXT WAY",
          detail: "exact admitted road",
          x: 320,
          y: 420,
          width: 120,
          tone: "neutral",
          semantic_identity: "rey://object/road:one",
          spatial_feature: {
            geometry_kind: "LineString",
            layer: "road",
            envelope_path: "M200 400 L500 440",
            geometry_path: "M200 400 L500 440",
            geometry_representation: "exact_native",
            authority: "exact admitted road geometry",
            cartographic_label: {
              min_zoom: 3,
              max_zoom: 12,
              collision_priority: 80,
            },
          },
        },
      ],
    } satisfies TopologyScene;
    const markup = renderToStaticMarkup(
      <>
        <ReferenceRenderer
          accelerated
          layers={{ relief: true, water: true, weather: true, probes: true }}
          onFocus={() => undefined}
          scene={scene}
        />
        <ReferenceMapReading scene={scene} />
      </>,
    );

    expect(markup).toContain('data-footprint-visual-weight="subordinate"');
    expect(markup).toContain('data-map-reading="compact"');
    expect(markup).not.toContain("BEARING /");
    expect(markup).not.toContain("VALIDITY / CONTOURS");
    expect(markup).toContain("LOD LANDSCAPE");
    expect(markup).toContain('data-accelerated-geometry="true"');
    expect(markup).toContain("exact admitted road geometry");
    expect(markup).toContain("CONTEXT WAY");

    const fallbackMarkup = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        scene={scene}
      />,
    );
    expect(fallbackMarkup).not.toContain("data-accelerated-geometry");
    expect(fallbackMarkup).toContain('d="M200 400 L500 440"');
  });

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
    expect(markup).toContain("<desc>");
    expect(markup).not.toContain("<title>");
    expect(markup).not.toContain(" title=");
  });

  it("keeps the base landform independent from optional contour overlays", () => {
    const markup = renderToStaticMarkup(
      <ReferenceRenderer
        accelerated
        layers={{ relief: false, water: false, weather: false, probes: false }}
        onFocus={() => undefined}
        scene={terrainScene}
      />,
    );

    expect(markup).toContain('data-renderer="reference-overlays"');
    expect(markup).toContain('data-world-geometry="charted"');
    expect(markup).toContain('data-accelerated-surface="true"');
    expect(markup).not.toContain("data-relief-level");
    expect(markup).not.toContain("data-natural-feature");
  });

  it("renders the admitted semantic atlas with selected-first label collisions", () => {
    const globeScene: TopologyScene = {
      ...terrainScene,
      focus_id: "anchor:survey:workspace",
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
          {
            id: "region:2",
            cluster_id: "cluster:1",
            focus_id: "anchor:survey:other",
            workload_id: "survey",
            label: "overlapping survey",
            detail: "same presentation point",
            longitude_degrees: 0,
            latitude_degrees: 0,
            angular_radius_degrees: 2,
            tone: "neutral",
          },
        ],
      },
    };
    for (const globeView of [
      { yaw_degrees: 0, pitch_degrees: 0 },
      { yaw_degrees: 32, pitch_degrees: -12 },
    ]) {
      const markup = renderToStaticMarkup(
        <ReferenceRenderer
          globeView={globeView}
          layers={{ relief: true, water: true, weather: true, probes: true }}
          onFocus={() => undefined}
          scene={globeScene}
        />,
      );

      expect(markup).toContain('data-atlas-revision="atlas:1"');
      expect(markup).toContain('data-semantic-region="region:1"');
      expect(markup).toContain('data-semantic-region="region:2"');
      expect(markup).toContain('data-label-disposition="selected"');
      expect(markup).toContain('data-label-disposition="collision"');
      expect(markup).toContain('data-globe-pole-pattern="north"');
      expect(markup).toContain('data-globe-pole-pattern="south"');
      expect(markup).toContain('data-globe-pole-sample-count="34"');
      expect(markup).not.toContain("data-globe-pole-label");
      expect(markup).not.toContain(">N</text>");
      expect(markup).not.toContain(">S</text>");
      expect(markup).toContain("Synthetic semantic longitude and latitude");
      expect(markup).toContain(
        'data-globe-caption="" text-anchor="middle" x="750"',
      );
      expect(markup).toContain("SEMANTIC SPHERE / REV atlas:1");
      expect(markup).not.toContain("2 ADMITTED REGIONS");
      expect(markup).not.toContain('data-world-geometry="charted"');
      expect(markup).not.toContain('data-natural-feature="stream"');
      expect(markup).not.toContain("<title>");
      expect(markup).not.toContain(" title=");
    }

    const halfwayMarkup = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={0.5}
        scene={globeScene}
      />,
    );
    expect(halfwayMarkup).toContain('data-globe-atmosphere-remaining="0.5"');
    expect(halfwayMarkup).toContain('data-globe-atmosphere-opacity="0.25"');
    expect(halfwayMarkup).toContain(
      'data-globe-atmosphere-shell-scale="0.707106',
    );
    expect(halfwayMarkup).toContain('opacity="0.25" r="308.');
    expect(halfwayMarkup).toContain(
      'data-globe-sphere="" data-globe-surface-opacity="0.25" fill="url(#rey-semantic-globe-fill)" opacity="0.25" r="400"',
    );

    const acceleratedMarkup = renderToStaticMarkup(
      <ReferenceRenderer
        accelerated
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={0.5}
        scene={globeScene}
      />,
    );
    expect(acceleratedMarkup).not.toContain('data-globe-atmosphere=""');
    expect(acceleratedMarkup).not.toContain("data-globe-atmosphere-remaining=");
    expect(acceleratedMarkup).toContain(
      'data-globe-sphere="" data-globe-surface-opacity="0.25" fill="transparent" opacity="0.25" r="400"',
    );

    // Before this fix, region/beacon markers rendered unconditionally here —
    // the only element in this layer that didn't already suppress itself
    // once the accelerated globe took over (unlike the atmosphere, sphere
    // fill, and stipple checked above). At progress 0 (deep in World,
    // before any morph) that reference copy — projected through a
    // completely separate, non-progress-aware implementation — sat a few
    // pixels from the accelerated marker's own progress-aware position;
    // the instant morph began and this reference copy vanished, the visible
    // marker jumped to where the accelerated one had been the whole time,
    // reading as a marker sliding during the crossing.
    const acceleratedRestMarkup = renderToStaticMarkup(
      <ReferenceRenderer
        accelerated
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        scene={globeScene}
      />,
    );
    expect(acceleratedRestMarkup).not.toContain("data-semantic-region=");
  });

  it("hides its own bordered Atlas sector once acceleration is healthy", () => {
    const atlasScene: TopologyScene = {
      ...terrainScene,
      regime: "atlas",
      terrain: false,
      globe: null,
      regions: [
        {
          id: "region:1",
          fragment_id: "region:1",
          label: "region one",
          detail: "settled Atlas region",
          x: 100,
          y: 100,
          width: 80,
          height: 60,
          tone: "healthy",
        },
      ],
      nodes: [
        {
          id: "node:1",
          focus_id: "regional:1",
          family: "region",
          label: "region one",
          detail: "settled Atlas region",
          x: 140,
          y: 130,
          width: 80,
          tone: "healthy",
          semantic_identity: "region:1",
        },
      ],
      world_atlas_transition: {
        schema: "rey.world-atlas-transition.v1",
        atlas_revision: "atlas:1",
        globe_source_revision: "atlas:1",
        projection_revision: "rey.semantic-mercator-projection@2",
        atlas_frame: { x: 0, y: 0, width: 1500, height: 1000 },
        points: [],
        sectors: [],
        authority: "test fixture",
      },
    };
    const extractAtlasFeatureLayer = (markup: string) => {
      const start = markup.indexOf('data-atlas-feature-layer="atlas:1"');
      expect(start).toBeGreaterThan(-1);
      const end = markup.indexOf("</svg>", start);
      return markup.slice(start, end);
    };

    // Same pattern WorldAtlasTransitionLayer already applies to its own
    // sector paths: the bordered sector rect stays in the deterministic
    // reference overlay for keyboard/screen-reader users when unaccelerated,
    // but must not draw a second, borderless-looking box once the
    // accelerated globe's own bordered GlobeSector is already on screen.
    const referenceMarkup = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={1}
        scene={atlasScene}
      />,
    );
    expect(extractAtlasFeatureLayer(referenceMarkup)).toContain(
      'data-semantic-identity="region:1"',
    );
    expect(extractAtlasFeatureLayer(referenceMarkup)).toContain("<rect");
    expect(extractAtlasFeatureLayer(referenceMarkup)).toContain("<circle");

    const acceleratedMarkup = renderToStaticMarkup(
      <ReferenceRenderer
        accelerated
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={1}
        scene={atlasScene}
      />,
    );
    expect(extractAtlasFeatureLayer(acceleratedMarkup)).not.toContain("<rect");
    // The node label/pick-target layer stays present either way — only the
    // duplicated sector geometry is renderer-gated.
    expect(extractAtlasFeatureLayer(acceleratedMarkup)).toContain(
      'data-focus-id="regional:1"',
    );
    // Before this fix, AtlasFeatureLayer's own node marker (a halo + point
    // circle, projected through the atlas Mercator math) rendered
    // unconditionally alongside the accelerated globe's own GlobeSurfaceMarker
    // for the same node — a second independent projection that need not
    // agree pixel-for-pixel, especially as it and the accelerated marker
    // respond differently to continuous zoom. The <g> pick target and its
    // label survive; only the two duplicated marker <circle>s are removed.
    expect(extractAtlasFeatureLayer(acceleratedMarkup)).not.toContain(
      "<circle",
    );
  });

  it("eases an Atlas node label's white halo in as the morph reaches the flat map", () => {
    const dissolvingScene: TopologyScene = {
      ...terrainScene,
      regime: "world",
      terrain: false,
      globe: null,
      regions: [],
      nodes: [
        {
          id: "node:1",
          focus_id: "regional:1",
          family: "region",
          label: "region one",
          detail: "settled Atlas region",
          x: 140,
          y: 130,
          width: 80,
          tone: "healthy",
          semantic_identity: "region:1",
        },
      ],
      world_atlas_transition: {
        schema: "rey.world-atlas-transition.v1",
        atlas_revision: "atlas:1",
        globe_source_revision: "atlas:1",
        projection_revision: "rey.semantic-mercator-projection@2",
        atlas_frame: { x: 0, y: 0, width: 1500, height: 1000 },
        points: [],
        sectors: [],
        authority: "test fixture",
      },
    };
    // chartWrapIndexes excludes the canonical copy (wrapIndex 0) while
    // still dissolving — only the repeat copies (wrapIndex -1/1) render
    // then, so that's what this fix's ramp is actually visible on. Search
    // for a specific wrap index's own marker rather than the first "region
    // one" text found, since each wrap index gets its own <text>.
    const labelStrokeOpacityForWrapIndex = (
      markup: string,
      wrapIndex: string,
    ) => {
      const wrapStart = markup.indexOf(`data-chart-wrap-index="${wrapIndex}"`);
      expect(wrapStart).toBeGreaterThan(-1);
      const match = markup
        .slice(wrapStart)
        .match(/<text[^>]*stroke-opacity="([^"]+)"[^>]*>region one</);
      expect(match).not.toBeNull();
      return Number(match![1]);
    };

    // AtlasFeatureLayer mounts as soon as repeat copies start dissolving in
    // (well before progress reaches 1) — before this fix, a repeat copy's
    // label's white halo was always at full strength the instant it mounted.
    const justDissolving = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={0.6}
        scene={dissolvingScene}
      />,
    );
    expect(labelStrokeOpacityForWrapIndex(justDissolving, "1")).toBeLessThan(
      0.05,
    );

    const midDissolve = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={0.79}
        scene={dissolvingScene}
      />,
    );
    const midOpacity = labelStrokeOpacityForWrapIndex(midDissolve, "1");
    expect(midOpacity).toBeGreaterThan(0.05);
    expect(midOpacity).toBeLessThan(0.95);

    // The canonical copy (wrapIndex 0) only ever mounts once progress
    // reaches exactly 1 and the regime has switched to "atlas" — it has no
    // earlier moment to ease in from, so it must simply be at full
    // strength the instant it exists (see worldAtlasMorphLabelStrokeOpacity,
    // which stays at full through this same handoff on the World side so
    // the swap itself carries no visible discontinuity).
    const atFlatMap = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={1}
        scene={{ ...dissolvingScene, regime: "atlas" }}
      />,
    );
    expect(labelStrokeOpacityForWrapIndex(atFlatMap, "0")).toBe(1);
  });

  it("hides its own morph-transition marker once acceleration is healthy", () => {
    const morphScene: TopologyScene = {
      ...terrainScene,
      regime: "world",
      terrain: false,
      globe: null,
      world_atlas_transition: {
        schema: "rey.world-atlas-transition.v1",
        atlas_revision: "atlas:1",
        globe_source_revision: "atlas:1",
        projection_revision: "rey.semantic-mercator-projection@2",
        atlas_frame: { x: 0, y: 0, width: 1500, height: 1000 },
        points: [
          {
            identity: "region:1",
            focus_id: "regional:1",
            label: "region one",
            longitude_microdegrees: 10_000_000,
            latitude_microdegrees: 5_000_000,
            tone: "healthy",
          },
        ],
        sectors: [],
        authority: "test fixture",
      },
    };
    const extractMorphLayer = (markup: string) => {
      const start = markup.indexOf('data-atlas-revision="atlas:1"');
      expect(start).toBeGreaterThan(-1);
      const end = markup.indexOf("</svg>", start);
      return markup.slice(start, end);
    };

    // Mid-morph (0 < progress < 1): the accelerated globe already renders
    // this same point through GlobeSurfaceMarker, continuously reprojected
    // with morph progress. Before this fix, WorldAtlasTransitionLayer's own
    // marker circle — projected through a second, independent morph
    // implementation (projectWorldAtlasMorph) — rendered unconditionally
    // alongside it, and the two need not agree as progress/zoom changes
    // continuously, reading as a marker sliding around while zooming.
    const referenceMarkup = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={0.5}
        scene={morphScene}
      />,
    );
    expect(extractMorphLayer(referenceMarkup)).toContain(
      'data-focus-id="regional:1"',
    );
    expect(extractMorphLayer(referenceMarkup)).toContain("<circle");

    const acceleratedMarkup = renderToStaticMarkup(
      <ReferenceRenderer
        accelerated
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={0.5}
        scene={morphScene}
      />,
    );
    expect(extractMorphLayer(acceleratedMarkup)).toContain(
      'data-focus-id="regional:1"',
    );
    expect(extractMorphLayer(acceleratedMarkup)).not.toContain("<circle");
  });

  it("eases a retained regional label's white halo in as the morph starts, staying at full strength through arrival at the flat map", () => {
    const morphScene: TopologyScene = {
      ...terrainScene,
      regime: "world",
      terrain: false,
      globe: null,
      world_atlas_transition: {
        schema: "rey.world-atlas-transition.v1",
        atlas_revision: "atlas:1",
        globe_source_revision: "atlas:1",
        projection_revision: "rey.semantic-mercator-projection@2",
        atlas_frame: { x: 0, y: 0, width: 1500, height: 1000 },
        points: [
          {
            identity: "region:1",
            focus_id: "regional:1",
            label: "region one",
            longitude_microdegrees: 10_000_000,
            latitude_microdegrees: 5_000_000,
            tone: "healthy",
          },
        ],
        sectors: [],
        authority: "test fixture",
      },
    };
    const labelStrokeOpacity = (markup: string) => {
      const match = markup.match(
        /<text[^>]*stroke-opacity="([^"]+)"[^>]*>region one</,
      );
      expect(match).not.toBeNull();
      return Number(match![1]);
    };

    // WorldAtlasTransitionLayer only mounts while 0 < progress < 1, so its
    // label's halo must fade in near that boundary rather than popping at
    // full strength the instant it mounts. It must NOT fade back out
    // approaching progress 1: AtlasFeatureLayer's own canonical label only
    // ever mounts at progress 1 exactly (no earlier moment to ease in
    // from), so fading this one out first would dip to nothing right
    // before that pops in at full strength.
    const nearWorldEndpoint = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={0.02}
        scene={morphScene}
      />,
    );
    expect(labelStrokeOpacity(nearWorldEndpoint)).toBeLessThan(0.15);

    const midMorph = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={0.5}
        scene={morphScene}
      />,
    );
    expect(labelStrokeOpacity(midMorph)).toBe(1);

    const nearAtlasEndpoint = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={0.98}
        scene={morphScene}
      />,
    );
    expect(labelStrokeOpacity(nearAtlasEndpoint)).toBe(1);
  });

  it("grows a retained regional label continuously as the morph approaches the flat map", () => {
    const morphScene: TopologyScene = {
      ...terrainScene,
      regime: "world",
      terrain: false,
      globe: null,
      regions: [],
      nodes: [
        {
          id: "node:1",
          focus_id: "regional:1",
          family: "region",
          label: "region one",
          detail: "settled Atlas region",
          x: 140,
          y: 130,
          width: 80,
          tone: "healthy",
          semantic_identity: "region:1",
        },
      ],
      world_atlas_transition: {
        schema: "rey.world-atlas-transition.v1",
        atlas_revision: "atlas:1",
        globe_source_revision: "atlas:1",
        projection_revision: "rey.semantic-mercator-projection@2",
        atlas_frame: { x: 0, y: 0, width: 1500, height: 1000 },
        points: [
          {
            identity: "region:1",
            focus_id: "regional:1",
            label: "region one",
            longitude_microdegrees: 10_000_000,
            latitude_microdegrees: 5_000_000,
            tone: "healthy",
          },
        ],
        sectors: [],
        authority: "test fixture",
      },
    };
    const labelGrowth = (markup: string) => {
      const match = markup.match(
        /<text[^>]*style="[^"]*--rey-world-atlas-label-growth:([^;"]+)[^"]*"[^>]*>region one</,
      );
      expect(match).not.toBeNull();
      return Number(match![1]);
    };

    const nearWorldEndpoint = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={0.02}
        scene={morphScene}
      />,
    );
    expect(labelGrowth(nearWorldEndpoint)).toBeCloseTo(1, 2);

    const midMorph = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={0.5}
        scene={morphScene}
      />,
    );
    const midGrowth = labelGrowth(midMorph);
    // 1 + 0.3 * smoothstep(0.5) = 1 + 0.3 * 0.5 = 1.15
    expect(midGrowth).toBeCloseTo(1.15, 5);

    const laterMorph = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={0.9}
        scene={morphScene}
      />,
    );
    const laterGrowth = labelGrowth(laterMorph);
    expect(laterGrowth).toBeGreaterThan(midGrowth);

    // The canonical Atlas-side label (atlasFeatureLabel) picks up the exact
    // same growth curve, reaching its ceiling (1.3x) right as the World-side
    // label's own growth also reaches it — the size in view keeps climbing
    // continuously through the handoff instead of jumping to a flat value.
    const atFlatMap = renderToStaticMarkup(
      <ReferenceRenderer
        layers={{ relief: true, water: true, weather: true, probes: true }}
        onFocus={() => undefined}
        projectionMorphProgress={1}
        scene={{ ...morphScene, regime: "atlas" }}
      />,
    );
    expect(labelGrowth(atFlatMap)).toBeCloseTo(1.3, 5);
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
    expect(markup).toContain("REVIEW &amp; CONSENT");
    expect(markup).toContain('href="/workloads/context-anchor-survey"');
    expect(markup).toContain("NO SURVEY CLAIM");
    expect(markup).not.toContain('data-world-geometry="charted"');
    expect(markup).toContain(
      'aria-label="WORKING · sys/context-anchor-survey/workload.yaml · revision blake3:package"',
    );
    expect(markup).not.toContain("<title>");
    expect(markup).not.toContain(" title=");
  });
});
