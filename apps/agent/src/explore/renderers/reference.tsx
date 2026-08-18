import { Link } from "@tanstack/react-router";
import { useMemo, type CSSProperties } from "react";
import {
  OBJECT_LENS_ZOOM,
  WORLD_GLOBE_ATMOSPHERE_SCALE,
  WORLD_GLOBE_RADIUS_RATIO,
  type LensRegime,
} from "../engine/camera";
import type { GlobeCameraView } from "../engine/camera";
import {
  layoutSemanticLabels,
  SEMANTIC_LABEL_LAYOUT_REVISION,
  type SemanticLabelPlacement,
} from "../engine/labels";
import { exploreStyles as styles } from "../../stylex/explore.stylex";
import { className as sx } from "../../stylex/shared.stylex";
import type {
  TopologyEdge,
  TopologyNode,
  TopologyPointOfInterest,
  TopologyScene,
  TopologyTone,
} from "../../topology";
import {
  contextGlobePolePatterns,
  contextGlobeSamples,
  planarPresentationSamples,
  type PlanarPresentationSample,
} from "@rey/explorer/globe-samples";
import {
  GLOBE_ATLAS_REPEAT_DISSOLVE_START,
  globeAtlasRepeatOpacity,
  globeAtlasRepeatSeamWeight,
  globeAtlasRepeatVisibility,
  globeAtmosphereOpacity,
  globeAtmosphereShellScale,
  globeProjectionMorphRemaining,
  globeSurfaceOpacity,
} from "@rey/explorer";
import { terrainTriangleIndices } from "@rey/explorer";
import {
  projectSemanticGlobe,
  projectWorldAtlasBoundsMorph,
  projectWorldAtlasMorph,
  semanticMercatorViewOffset,
  type SemanticCoordinate,
} from "../projection/semantic-mercator";
import {
  activeExplorerRenderPasses,
  compileExplorerRenderGraph,
  type ExplorerRenderGraph,
} from "../engine/render-graph";
import {
  compileScenePickingIndex,
  pickSceneNode,
  type ScenePickingIndex,
} from "../engine/picking";
import type { AtlasLandscapePresentation } from "../projection/atlas-landscape";
import { featureVisibleAtLens } from "../engine/cartography";
import {
  materializeTerrainTile,
  projectTerrainTilePyramid,
} from "../terrain/tiles";
import type { TerrainFieldSet } from "../terrain/compile";

export interface FocusableTopologyObject {
  focus_id: string;
  x: number;
  y: number;
  semantic_identity?: string;
  semantic_coordinate?: SemanticCoordinate;
  inverse_coordinate?: SemanticCoordinate;
  chart_wrap_index?: number;
}

export interface ReferenceLayerVisibility {
  relief: boolean;
  water: boolean;
  weather: boolean;
  probes: boolean;
}

export function ReferenceRenderer({
  accelerated = false,
  layers,
  onFocus,
  scene,
  globeView = { yaw_degrees: 0, pitch_degrees: 0 },
  projectionMorphProgress = scene.regime === "world" ? 0 : 1,
  renderGraph,
  pickingIndex,
  atlasLandscapeMorphProgress = scene.terrain ? 1 : 0,
  atlasLandscapePresentation,
}: {
  accelerated?: boolean;
  layers: ReferenceLayerVisibility;
  onFocus: (node: FocusableTopologyObject) => void;
  scene: TopologyScene;
  globeView?: GlobeCameraView;
  projectionMorphProgress?: number;
  renderGraph?: ExplorerRenderGraph;
  pickingIndex?: ScenePickingIndex;
  atlasLandscapeMorphProgress?: number;
  atlasLandscapePresentation?: AtlasLandscapePresentation;
}) {
  const activeRenderGraph = renderGraph ?? compileExplorerRenderGraph(scene);
  const activeRenderPasses = activeExplorerRenderPasses(activeRenderGraph, {
    contours: layers.relief,
    water: layers.water,
    weather: layers.weather,
    probes: layers.probes,
  });
  const globeWorld = scene.regime === "world" && scene.globe !== null;
  const morphActive =
    scene.world_atlas_transition !== null &&
    projectionMorphProgress > 0 &&
    projectionMorphProgress < 1;
  const wrappedAtlas =
    scene.regime === "atlas" &&
    scene.world_atlas_transition !== null &&
    projectionMorphProgress >= 1;
  const atlasRepeatOpacity = globeAtlasRepeatOpacity(projectionMorphProgress);
  const atlasSeamPeriod =
    scene.world.width *
    (1 - globeProjectionMorphRemaining(projectionMorphProgress));
  const atlasRepeatOffset = (scene.world.width + atlasSeamPeriod) / 2;
  const dissolvingAtlasRepeats = morphActive && atlasRepeatOpacity > 0;
  // Once a region is admitted, world_atlas_transition (and its sector/marker
  // geometry) is deliberately retained by the scene builder through the
  // "landscape" regime for exactly this band (topology.ts:421-423) — so the
  // atlas sector can keep rendering, fading via atlas_opacity, instead of
  // hard-unmounting the instant the regime flips. Without this the sector
  // rect (opaque, painted after the terrain layer) fully occludes the
  // terrain fading in underneath it, then vanishes outright at the regime
  // boundary — the "jump" that made the transition feel broken.
  const atlasSectorsRetained =
    scene.regime === "landscape" &&
    scene.atlas_landscape_transition !== null &&
    scene.world_atlas_transition !== null;
  const atlasFeatureLayerActive =
    wrappedAtlas || dissolvingAtlasRepeats || atlasSectorsRetained;
  const atlasSectorOpacity = atlasSectorsRetained
    ? (atlasLandscapePresentation?.atlas_opacity ?? 1)
    : 1;
  const chartWrapIndexes = wrappedAtlas
    ? [-1, 0, 1]
    : dissolvingAtlasRepeats
      ? [-1, 1]
      : [0];
  const activePickingIndex = pickingIndex ?? compileScenePickingIndex(scene);
  const atlasLabelPlacements = new Map(
    (atlasFeatureLayerActive
      ? layoutSemanticLabels(
          chartWrapIndexes.flatMap((wrapIndex) =>
            scene.nodes.map((node) => ({
              fragment_id: `${wrapIndex}:${node.id}`,
              semantic_identity: node.semantic_identity ?? node.id,
              focus_id: node.focus_id,
              x: node.x + wrapIndex * atlasRepeatOffset - node.width / 2,
              y: node.y - 63,
              width: node.width,
              height: 126,
              priority: wrapIndex === 0 ? 100 : 10 - Math.abs(wrapIndex),
              selected: wrapIndex === 0 && node.focus_id === scene.focus_id,
            })),
          ),
          96,
        )
      : []
    ).map((placement) => [placement.fragment_id, placement]),
  );
  return (
    <div
      className={sx(
        styles.projection,
        scene.terrain && styles.terrainProjection,
        scene.county_frame && styles.countyProjection,
        scene.terrain && accelerated && styles.acceleratedTerrainProjection,
        scene.terrain &&
          scene.regime === "world" &&
          styles.worldTerrainProjection,
      )}
      data-lens-regime={scene.regime}
      data-render-graph={activeRenderGraph.graph_id}
      data-render-passes={activeRenderPasses.map(({ id }) => id).join(",")}
      data-renderer={accelerated ? "reference-overlays" : "reference"}
      data-atlas-landscape-progress={atlasLandscapeMorphProgress}
    >
      {!globeWorld &&
        !morphActive &&
        !wrappedAtlas &&
        chartWrapIndexes.flatMap((wrapIndex) =>
          scene.regions.map((region) => (
            <div
              aria-hidden={wrapIndex === 0 ? undefined : true}
              className={sx(
                styles.region,
                region.variant === "map-boundary" && styles.mapBoundary,
                region.variant === "map-zone" && styles.mapZone,
                toneStyle(region.tone, "region"),
                scene.regime === "world" &&
                  region.variant === "map-zone" &&
                  region.tone === "unknown" &&
                  styles.worldUnexploredZone,
              )}
              data-chart-wrap-index={wrappedAtlas ? wrapIndex : undefined}
              data-semantic-identity={wrappedAtlas ? region.id : undefined}
              key={`${wrapIndex}:${region.fragment_id ?? region.id}`}
              style={{
                height: region.height,
                left: region.x + wrapIndex * scene.world.width,
                top: region.y,
                width: region.width,
              }}
            >
              <span>{region.label}</span>
              <small>{region.detail}</small>
            </div>
          )),
        )}
      {/*
        Only content naturally laid out in target_frame (native terrain/
        county pixel space) belongs inside this transform: at progress 0,
        atlasLandscapePresentation's css_transform maps target_frame onto
        source_frame exactly, so this content visually emerges from the
        Atlas sector's own position and grows to its natural size by
        progress 1. AtlasFeatureLayer's sector rect is NOT target_frame
        content — it's already at its correct absolute (source_frame)
        position — so applying this same transform to it would warp it
        away from where it belongs (double-transforming an already-correct
        point) instead of leaving it in place while it fades. That's why it
        renders as a sibling below, outside this wrapper, with its own
        independent opacity fade instead.
      */}
      <div
        className={sx(styles.projection)}
        style={
          scene.terrain && atlasLandscapePresentation
            ? {
                opacity: atlasLandscapePresentation.terrain_opacity,
                transform: atlasLandscapePresentation.css_transform,
                transformOrigin: "0 0",
              }
            : undefined
        }
      >
        <CountyFootprintLayer accelerated={accelerated} scene={scene} />
        {!accelerated && scene.terrain ? (
          <AdmittedTerrainFieldLayer scene={scene} />
        ) : null}
        <CountyFeatureLayer
          accelerated={accelerated}
          onFocus={onFocus}
          scene={scene}
        />
      </div>
      {atlasFeatureLayerActive ? (
        <AtlasFeatureLayer
          accelerated={accelerated}
          globeView={globeView}
          labelPlacements={atlasLabelPlacements}
          landscapeMorphProgress={atlasLandscapeMorphProgress}
          landscapeOpacity={atlasSectorOpacity}
          onFocus={onFocus}
          projectionMorphProgress={projectionMorphProgress}
          scene={scene}
          wrapOffset={atlasRepeatOffset}
          wrapIndexes={chartWrapIndexes}
        />
      ) : null}
      <WorldGeometryLayer
        accelerated={accelerated}
        globeView={globeView}
        onFocus={onFocus}
        projectionMorphProgress={projectionMorphProgress}
        scene={scene}
        suppressSemanticObjects={morphActive}
      />
      {morphActive ? (
        <WorldAtlasTransitionLayer
          accelerated={accelerated}
          globeView={globeView}
          onFocus={onFocus}
          progress={projectionMorphProgress}
          scene={scene}
        />
      ) : null}
      {!globeWorld && !accelerated && layers.relief ? (
        <ReliefLayer scene={scene} />
      ) : null}
      {!globeWorld && (layers.water || layers.weather) ? (
        <NaturalFeatureLayer
          scene={scene}
          showWater={layers.water}
          showWeather={layers.weather}
        />
      ) : null}
      {!scene.terrain && scene.edges.length > 0 ? (
        <EdgeLayer
          edges={scene.edges}
          nodes={scene.nodes}
          points={scene.points}
          terrain={scene.terrain}
          world={scene.world}
        />
      ) : null}
      {!globeWorld &&
        scene.points
          .filter((point) => layers.probes || point.kind !== "frontier")
          .map((point) => (
            <PointOfInterest
              key={point.id}
              onFocus={onFocus}
              point={point}
              regime={scene.regime}
            />
          ))}
      {!globeWorld &&
        !morphActive &&
        !scene.county_frame &&
        !wrappedAtlas &&
        chartWrapIndexes.flatMap((wrapIndex) =>
          scene.nodes.map((node) => (
            <TopologyObject
              chartWrapIndex={wrapIndex}
              counterScale={scene.terrain}
              key={`${wrapIndex}:${node.id}`}
              linkToWorkload={
                (scene.regime === "objects" || scene.regime === "evidence") &&
                !scene.terrain
              }
              node={node}
              onFocus={onFocus}
              pickingIndex={activePickingIndex}
              labelPlacement={atlasLabelPlacements.get(
                `${wrapIndex}:${node.id}`,
              )}
              worldWidth={scene.world.width}
            />
          )),
        )}
    </div>
  );
}

const REGIONAL_TERRAIN_STIPPLE_SAMPLE_COUNT = 2_600;

/**
 * Same deterministic dot-fabric technique the globe's samplePath builds
 * (see SemanticGlobeLayer below): one SVG path with many short subpaths,
 * not one DOM node per dot, so a few thousand stipple points cost a single
 * element. Reuses planarPresentationSamples — the flat-map sibling of the
 * globe's contextGlobeSamples — so the abstraction reads as the same visual
 * language, not a coincidentally similar one.
 */
function regionalTerrainStipplePath(field: TerrainFieldSet): string {
  return planarStipplePathSegments(
    field.source_revision,
    REGIONAL_TERRAIN_STIPPLE_SAMPLE_COUNT,
    field.grid.bounds,
    (u, v) => {
      const column = Math.min(
        field.grid.columns - 1,
        Math.round(u * (field.grid.columns - 1)),
      );
      const row = Math.min(
        field.grid.rows - 1,
        Math.round(v * (field.grid.rows - 1)),
      );
      return field.validity.values[row * field.grid.columns + column] === 1;
    },
  );
}

function planarStipplePathSegments(
  sourceRevision: string,
  candidateCount: number,
  bounds: { x: number; y: number; width: number; height: number },
  includeSample: (u: number, v: number) => boolean = () => true,
): string {
  return stipplePathFromSamples(
    planarPresentationSamples(sourceRevision, candidateCount),
    bounds,
    includeSample,
  );
}

/**
 * Split out from planarStipplePathSegments so a caller whose sample count
 * changes continuously (a progress-driven density ramp, redrawn every
 * animation frame) can generate the point fabric once — the expensive part,
 * one hash plus trig per candidate point — and cheaply re-slice/re-stringify
 * a prefix of it per frame, instead of recomputing the whole fabric from
 * scratch every frame.
 */
function stipplePathFromSamples(
  samples: readonly PlanarPresentationSample[],
  bounds: { x: number; y: number; width: number; height: number },
  includeSample: (u: number, v: number) => boolean = () => true,
): string {
  const segments: string[] = [];
  for (const sample of samples) {
    if (!includeSample(sample.u, sample.v)) continue;
    const x = bounds.x + sample.u * bounds.width;
    const y = bounds.y + sample.v * bounds.height;
    const length = Math.max(0.5, 0.7 + sample.brightness * 0.9);
    segments.push(`M${x.toFixed(1)} ${y.toFixed(1)}h${length.toFixed(1)}`);
  }
  return segments.join("");
}

function AdmittedTerrainFieldLayer({ scene }: { scene: TopologyScene }) {
  const fields = useMemo(
    () =>
      scene.terrain_fields
        .filter((field) => field.active_band_ids.includes("admitted_dem"))
        .flatMap((field) => {
          const pyramid = projectTerrainTilePyramid(field);
          return pyramid.tiles
            .filter(({ level }) => level === 0)
            .map((tile) => materializeTerrainTile(field, tile));
        }),
    [scene.terrain_fields],
  );
  const stipplePath = useMemo(
    () => fields.map(regionalTerrainStipplePath).join(""),
    [fields],
  );
  if (fields.length === 0) return null;
  return (
    <svg
      aria-label={`${fields.length} admitted regional terrain field${fields.length === 1 ? "" : "s"}`}
      className={sx(styles.worldGeometryLayer)}
      data-regional-terrain-reference="rey.reference-regional-terrain@1"
      role="img"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <desc>
        Triangles exist only where three admitted source vertices are valid.
        Explicit no-data vertices remain holes.
      </desc>
      {fields.flatMap((field) => {
        const indices = terrainTriangleIndices(field);
        const point = (index: number) => {
          const column = index % field.grid.columns;
          const row = Math.floor(index / field.grid.columns);
          return {
            x:
              field.grid.bounds.x +
              (column / (field.grid.columns - 1)) * field.grid.bounds.width,
            y:
              field.grid.bounds.y +
              (row / (field.grid.rows - 1)) * field.grid.bounds.height,
          };
        };
        return Array.from({ length: indices.length / 3 }, (_, triangle) => {
          const vertexIndexes = [
            indices[triangle * 3]!,
            indices[triangle * 3 + 1]!,
            indices[triangle * 3 + 2]!,
          ] as const;
          const vertices = vertexIndexes.map(point);
          const tint = [0, 1, 2].map(
            (component) =>
              vertexIndexes.reduce(
                (total, index) =>
                  total + field.material.tint[index * 3 + component]!,
                0,
              ) / 3,
          );
          const fill = `rgb(${tint.map((value) => Math.round(Math.max(0, Math.min(1, value)) * 255)).join(" ")})`;
          return (
            <polygon
              data-field-set-id={field.field_set_id}
              data-terrain-triangle={triangle}
              fill={fill}
              key={`${field.field_set_id}:${triangle}`}
              points={vertices.map(({ x, y }) => `${x},${y}`).join(" ")}
              stroke="rgba(247, 232, 184, 0.14)"
              strokeWidth={0.45}
            />
          );
        });
      })}
      {stipplePath ? (
        <path
          aria-hidden="true"
          className={sx(styles.regionalTerrainStipple)}
          d={stipplePath}
        />
      ) : null}
    </svg>
  );
}

const ATLAS_SECTOR_STIPPLE_BASE_SAMPLE_COUNT = 260;

function AtlasFeatureLayer({
  accelerated,
  globeView,
  labelPlacements,
  landscapeMorphProgress = 0,
  landscapeOpacity = 1,
  onFocus,
  projectionMorphProgress,
  scene,
  wrapOffset,
  wrapIndexes,
}: {
  accelerated: boolean;
  globeView: GlobeCameraView;
  labelPlacements: ReadonlyMap<string, SemanticLabelPlacement>;
  landscapeMorphProgress?: number;
  landscapeOpacity?: number;
  onFocus: (node: FocusableTopologyObject) => void;
  projectionMorphProgress: number;
  scene: TopologyScene;
  wrapOffset: number;
  wrapIndexes: readonly number[];
}) {
  const offset = semanticMercatorViewOffset(
    scene.world_atlas_transition!.atlas_frame,
    globeView,
  );
  const labelStrokeOpacity = atlasFeatureLabelStrokeOpacity(
    projectionMorphProgress,
  );
  const labelStyle = {
    "--rey-world-atlas-label-growth": worldAtlasLabelGrowthScale(
      projectionMorphProgress,
    ),
  } as CSSProperties;
  // The focused sector reuses the exact terrain field revision the
  // landscape stipple (regionalTerrainStipplePath) will seed itself with,
  // so as sample count ramps toward REGIONAL_TERRAIN_STIPPLE_SAMPLE_COUNT the
  // same deterministic dots simply keep appearing (R2 sequence positions
  // depend only on index, not candidate count) — the sector's own stipple
  // visibly resolves into the landscape's, instead of one pattern swapping
  // for an unrelated one at the handoff.
  const focusedTerrainRevision = scene.terrain_fields.find((field) =>
    field.active_band_ids.includes("admitted_dem"),
  )?.source_revision;
  // TopologyRegion (sector rects) carries no focus_id of its own — sectors
  // are synthetic membership partitions, not single regions — so the
  // focused sector is whichever one geometrically contains the selected
  // node's marker position.
  const focusedNode = scene.nodes.find(
    (node) => node.focus_id === scene.focus_id,
  );
  const focusedRegion = scene.regions.find(
    (region) =>
      focusedNode !== undefined &&
      focusedNode.x >= region.x &&
      focusedNode.x <= region.x + region.width &&
      focusedNode.y >= region.y &&
      focusedNode.y <= region.y + region.height,
  );
  // Baseline sectors never change density, so their samples and path
  // strings are generated once (per region set) and reused, not rebuilt on
  // every render this component gets during the zoom.
  const baselineStipplePaths = useMemo(
    () =>
      new Map(
        scene.regions
          .filter((region) => region !== focusedRegion)
          .map((region) => [
            region.id,
            planarStipplePathSegments(
              region.id,
              ATLAS_SECTOR_STIPPLE_BASE_SAMPLE_COUNT,
              region,
            ),
          ]),
      ),
    [scene.regions, focusedRegion],
  );
  // The focused sector's full fabric (the expensive part — one hash plus
  // trig per candidate point) is generated once per terrain revision, since
  // it doesn't change through the morph; only how much of its prefix is
  // revealed does. landscapeMorphProgress updates every animation frame, so
  // rounding it before it drives which prefix is sliced/stringified keeps
  // that (cheap, but not free at ~2,600 segments) work from re-running on
  // every single frame — this was expensive enough to visibly stall the
  // Atlas-to-Landscape morph before this fix.
  const focusedFullSamples = useMemo(
    () =>
      focusedTerrainRevision
        ? planarPresentationSamples(
            focusedTerrainRevision,
            REGIONAL_TERRAIN_STIPPLE_SAMPLE_COUNT,
          )
        : null,
    [focusedTerrainRevision],
  );
  const roundedLandscapeMorphProgress =
    Math.round(landscapeMorphProgress * 50) / 50;
  const focusedStipplePath = useMemo(() => {
    if (!focusedRegion || !focusedFullSamples) return "";
    const sampleCount = Math.round(
      ATLAS_SECTOR_STIPPLE_BASE_SAMPLE_COUNT +
        (REGIONAL_TERRAIN_STIPPLE_SAMPLE_COUNT -
          ATLAS_SECTOR_STIPPLE_BASE_SAMPLE_COUNT) *
          roundedLandscapeMorphProgress,
    );
    return stipplePathFromSamples(
      focusedFullSamples.slice(0, sampleCount),
      focusedRegion,
    );
  }, [focusedFullSamples, focusedRegion, roundedLandscapeMorphProgress]);
  const atlasStipplePath = accelerated
    ? ""
    : [...baselineStipplePaths.values(), focusedStipplePath].join("");
  return (
    <svg
      aria-label={`${scene.nodes.length} admitted regional identities on the semantic Mercator atlas`}
      className={sx(styles.worldGeometryLayer, styles.atlasFeatureLayer)}
      data-atlas-feature-layer={scene.world_atlas_transition?.atlas_revision}
      role="group"
      style={landscapeOpacity < 1 ? { opacity: landscapeOpacity } : undefined}
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <desc>
        Occupied sectors retain synthetic membership only. Markers retain one
        canonical identity through horizontally wrapped presentation copies.
      </desc>
      <g
        data-atlas-view-offset={`${offset.x},${offset.y}`}
        transform={`translate(${offset.x} ${offset.y})`}
      >
        {!accelerated
          ? wrapIndexes.flatMap((wrapIndex) =>
              scene.regions.map((region) => {
                const seamWeight = globeAtlasRepeatSeamWeight(
                  (region.x + region.width / 2) / scene.world.width,
                  wrapIndex,
                );
                const opacity =
                  wrapIndex === 0
                    ? 1
                    : globeAtlasRepeatVisibility(
                        projectionMorphProgress,
                        seamWeight,
                      );
                return (
                  <rect
                    aria-hidden="true"
                    className={sx(styles.atlasSector)}
                    data-chart-wrap-index={wrapIndex}
                    data-repeat-seam-weight={seamWeight.toFixed(3)}
                    data-semantic-identity={region.id}
                    height={region.height}
                    key={`${wrapIndex}:${region.fragment_id ?? region.id}`}
                    opacity={opacity}
                    width={region.width}
                    x={region.x + wrapIndex * wrapOffset}
                    y={region.y}
                  >
                    <desc>{`${region.label} / ${region.detail}`}</desc>
                  </rect>
                );
              }),
            )
          : null}
        {atlasStipplePath ? (
          <path
            aria-hidden="true"
            className={sx(styles.regionalTerrainStipple)}
            d={atlasStipplePath}
          />
        ) : null}
        {wrapIndexes.flatMap((wrapIndex) =>
          scene.nodes.map((node) => {
            const placement = labelPlacements.get(`${wrapIndex}:${node.id}`)!;
            const x = node.x + wrapIndex * wrapOffset;
            const seamWeight = globeAtlasRepeatSeamWeight(
              node.x / scene.world.width,
              wrapIndex,
            );
            const opacity =
              wrapIndex === 0
                ? 1
                : globeAtlasRepeatVisibility(
                    projectionMorphProgress,
                    seamWeight,
                  );
            return (
              <g
                aria-hidden={wrapIndex === 0 ? undefined : true}
                aria-label={`${node.label}: ${node.detail}`}
                className={sx(styles.atlasFeature)}
                data-chart-wrap-index={wrapIndex}
                data-focus-id={node.focus_id}
                data-label-disposition={placement.disposition}
                data-label-layout={SEMANTIC_LABEL_LAYOUT_REVISION}
                data-repeat-seam-weight={seamWeight.toFixed(3)}
                data-semantic-identity={node.semantic_identity ?? node.id}
                key={`${wrapIndex}:${node.id}`}
                onClick={() =>
                  onFocus({
                    focus_id: node.focus_id,
                    x: x + offset.x,
                    y: node.y + offset.y,
                    semantic_identity: node.semantic_identity,
                    semantic_coordinate: node.semantic_coordinate,
                    chart_wrap_index: wrapIndex,
                  })
                }
                onKeyDown={(event) => {
                  if (
                    wrapIndex === 0 &&
                    (event.key === "Enter" || event.key === " ")
                  ) {
                    event.preventDefault();
                    onFocus({
                      focus_id: node.focus_id,
                      x: x + offset.x,
                      y: node.y + offset.y,
                      semantic_identity: node.semantic_identity,
                      semantic_coordinate: node.semantic_coordinate,
                      chart_wrap_index: wrapIndex,
                    });
                  }
                }}
                role="button"
                style={{ opacity }}
                tabIndex={wrapIndex === 0 ? 0 : -1}
              >
                {!accelerated ? (
                  <>
                    <circle
                      className={sx(styles.atlasFeatureHalo)}
                      cx={x}
                      cy={node.y}
                      r={15}
                    />
                    <circle
                      className={sx(styles.atlasFeaturePoint)}
                      cx={x}
                      cy={node.y}
                      r={7}
                    />
                  </>
                ) : null}
                {placement.visible ? (
                  <text
                    className={sx(styles.atlasFeatureLabel)}
                    strokeOpacity={labelStrokeOpacity}
                    style={labelStyle}
                    x={x + 13}
                    y={node.y - 11}
                  >
                    {node.label}
                  </text>
                ) : null}
                <desc>{node.detail}</desc>
              </g>
            );
          }),
        )}
      </g>
    </svg>
  );
}

/**
 * Eases an Atlas node label's white halo in as the morph reaches the flat
 * map, instead of it popping to full strength the instant AtlasFeatureLayer
 * first mounts (as early as GLOBE_ATLAS_REPEAT_DISSOLVE_START, well before
 * progress actually reaches 1). Only its repeat copies (wrapIndex -1/1) are
 * ever actually rendered while this is less than 1 — chartWrapIndexes
 * excludes the canonical wrapIndex-0 copy until progress reaches exactly 1
 * (avoiding a duplicate of WorldAtlasTransitionLayer's own retained label),
 * so this always evaluates to a full 1 the one moment that copy exists.
 */
function atlasFeatureLabelStrokeOpacity(progress: number): number {
  const boundedProgress = Math.max(0, Math.min(1, progress));
  if (boundedProgress <= GLOBE_ATLAS_REPEAT_DISSOLVE_START) return 0;
  return (
    1 -
    globeProjectionMorphRemaining(
      (boundedProgress - GLOBE_ATLAS_REPEAT_DISSOLVE_START) /
        (1 - GLOBE_ATLAS_REPEAT_DISSOLVE_START),
    )
  );
}

function CountyFeatureLayer({
  accelerated,
  onFocus,
  scene,
}: {
  accelerated: boolean;
  onFocus: (node: FocusableTopologyObject) => void;
  scene: TopologyScene;
}) {
  // scene (and scene.nodes with it) is stable across pure pan/zoom renders —
  // buildTopologyScene only reruns on focusId/regime/portfolio changes — so
  // memoizing here (previously recomputed unconditionally on every render,
  // including every pan frame) turns a per-frame filter-plus-label-collision
  // pass over every County feature into a one-time cost per scene, the same
  // fix AtlasFeatureLayer's stipple needed for the same reason: Neighborhoods
  // shows every vector layer (roads, hydrology, districts, POIs — nothing
  // this dense renders at Landscape or above at Atlas), so this was the
  // most expensive per-frame recompute in the whole reference layer.
  const features = useMemo(
    () =>
      scene.county_frame
        ? scene.nodes.filter(
            (node) =>
              node.spatial_feature &&
              node.id !==
                `regional-object:${scene.county_footprint?.source_object_id}` &&
              featureVisibleAtLens(
                node.spatial_feature,
                scene.regime,
                node.focus_id === scene.focus_id,
              ),
          )
        : [],
    [
      scene.county_frame,
      scene.county_footprint,
      scene.focus_id,
      scene.nodes,
      scene.regime,
    ],
  );
  const labelPlacements = useMemo(
    () =>
      new Map(
        layoutSemanticLabels(
          features.flatMap((node) => {
            const label = node.spatial_feature?.cartographic_label;
            if (!label) return [];
            return [
              {
                fragment_id: `county:${node.id}`,
                semantic_identity: node.semantic_identity ?? node.id,
                focus_id: node.focus_id,
                x: node.x + 10,
                y: node.y - 20,
                width: Math.max(44, node.label.length * 6.2),
                height: 18,
                priority: label.collision_priority,
                selected: node.focus_id === scene.focus_id,
              },
            ];
          }),
          32,
        ).map((placement) => [placement.fragment_id, placement]),
      ),
    [features, scene.focus_id],
  );
  if (!scene.county_frame || features.length === 0) return null;
  return (
    <svg
      aria-label={`${features.length} admitted County features`}
      className={sx(styles.worldGeometryLayer, styles.countyFeatureLayer)}
      data-county-feature-layer={scene.county_frame.frame_id}
      role="group"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <desc>
        Exact retained native vector geometry is drawn when available. Legacy
        bounds envelopes remain disclosed and do not reconstruct source geometry
        or terrain between admitted samples.
      </desc>
      {features.map((node) => {
        const feature = node.spatial_feature!;
        const point = feature.geometry_kind.toLowerCase() === "point";
        const linear =
          feature.geometry_kind.toLowerCase().includes("line") ||
          ["highway", "road", "railway", "utility", "connector"].includes(
            feature.layer,
          );
        const selected = node.focus_id === scene.focus_id;
        const labelPlacement = labelPlacements.get(`county:${node.id}`);
        const showLabel = selected || labelPlacement?.visible === true;
        const content = (
          <g
            aria-label={`${node.family}: ${node.label}. ${node.detail}`}
            className={sx(
              styles.countyFeature,
              selected && styles.countyFeatureSelected,
            )}
            data-county-feature={node.id}
            data-feature-geometry={feature.geometry_kind}
            data-feature-geometry-representation={
              feature.geometry_representation
            }
            data-feature-layer={feature.layer}
            data-feature-source-authority={feature.authority}
            onClick={
              scene.regime === "objects" || scene.regime === "evidence"
                ? undefined
                : () => onFocus(node)
            }
            onKeyDown={(event) => {
              if (
                scene.regime !== "objects" &&
                scene.regime !== "evidence" &&
                (event.key === "Enter" || event.key === " ")
              ) {
                event.preventDefault();
                onFocus(node);
              }
            }}
            role={
              scene.regime === "objects" || scene.regime === "evidence"
                ? undefined
                : "button"
            }
            tabIndex={
              scene.regime === "objects" || scene.regime === "evidence"
                ? undefined
                : 0
            }
          >
            {point && feature.layer !== "label" ? (
              <>
                <circle
                  className={sx(
                    styles.countyFeaturePoint,
                    accelerated && styles.countyFeatureAcceleratedGeometry,
                  )}
                  cx={node.x}
                  cy={node.y}
                  data-accelerated-geometry={accelerated || undefined}
                  r={feature.layer === "terrain" ? 11 : 7}
                />
                {feature.layer === "terrain" ? (
                  <circle
                    className={sx(
                      styles.countyTerrainSampleHalo,
                      accelerated && styles.countyFeatureAcceleratedGeometry,
                    )}
                    cx={node.x}
                    cy={node.y}
                    data-accelerated-geometry={accelerated || undefined}
                    r={18}
                  />
                ) : null}
              </>
            ) : !point ? (
              <path
                className={sx(
                  styles.countyFeatureEnvelope,
                  linear && styles.countyFeatureLinear,
                  feature.layer === "hydrology" &&
                    linear &&
                    styles.countyFeatureHydrology,
                  feature.layer === "hydrology" &&
                    !linear &&
                    styles.countyFeatureWaterArea,
                  feature.layer === "highway" && styles.countyFeatureHighway,
                  feature.layer === "road" && styles.countyFeatureRoad,
                  feature.layer === "railway" && styles.countyFeatureRailway,
                  feature.layer === "district" && styles.countyFeatureDistrict,
                  feature.layer === "boundary" && styles.countyFeatureBoundary,
                  feature.layer === "terrain_control" &&
                    styles.countyTerrainControl,
                  accelerated && styles.countyFeatureAcceleratedGeometry,
                )}
                data-accelerated-geometry={accelerated || undefined}
                d={feature.geometry_path}
              />
            ) : null}
            {showLabel ? (
              <text
                className={sx(styles.countyFeatureLabel)}
                data-feature-label-visible="true"
                data-label-disposition={labelPlacement?.disposition}
                data-label-layout={
                  labelPlacement ? SEMANTIC_LABEL_LAYOUT_REVISION : undefined
                }
                x={node.x + 12}
                y={node.y - 12}
              >
                {countyFeatureLabel(node.label)}
              </text>
            ) : null}
            <desc>{`${node.detail} / ${feature.authority}`}</desc>
          </g>
        );
        return node.evidence_uri &&
          (scene.regime === "objects" || scene.regime === "evidence") ? (
          <a
            data-object-evidence={node.evidence_uri}
            href={node.evidence_uri}
            key={node.id}
          >
            {content}
            {selected ? (
              <text
                className={sx(styles.countyFeatureEvidenceLabel)}
                x={node.x + 12}
                y={node.y + 3}
              >
                OPEN EXACT EVIDENCE
              </text>
            ) : null}
          </a>
        ) : (
          <g key={node.id}>{content}</g>
        );
      })}
    </svg>
  );
}

function countyFeatureLabel(label: string) {
  const concise = label.split(/[/:]/).filter(Boolean).at(-1) ?? label;
  return concise.replaceAll("-", " ").toUpperCase();
}

function CountyFootprintLayer({
  accelerated,
  scene,
}: {
  accelerated: boolean;
  scene: TopologyScene;
}) {
  const footprint = scene.county_footprint;
  if (!footprint) return null;
  const subordinate =
    scene.regime === "landscape" || scene.regime === "neighborhoods";
  return (
    <svg
      aria-label={`Exact admitted County footprint ${footprint.footprint_id}`}
      className={sx(styles.worldGeometryLayer, styles.countyFootprintLayer)}
      data-county-footprint={footprint.footprint_id}
      data-source-object={footprint.source_object_id}
      data-source-revision={footprint.source_object_revision}
      data-footprint-visual-weight={subordinate ? "subordinate" : "exact"}
      role="img"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <desc>{`${footprint.source_object_id} / ${footprint.coordinate_count} exact native coordinates / ${footprint.authority}`}</desc>
      <path
        className={sx(
          styles.countyFootprint,
          accelerated && styles.countyFootprintAccelerated,
          subordinate && styles.countyFootprintSubordinate,
        )}
        d={footprint.path}
        fillRule="evenodd"
      />
    </svg>
  );
}

function WorldGeometryLayer({
  accelerated,
  onFocus,
  scene,
  globeView,
  projectionMorphProgress,
  suppressSemanticObjects,
}: {
  accelerated: boolean;
  onFocus: (node: FocusableTopologyObject) => void;
  scene: TopologyScene;
  globeView: GlobeCameraView;
  projectionMorphProgress: number;
  suppressSemanticObjects: boolean;
}) {
  if (scene.regime === "world" && scene.globe)
    return (
      <SemanticGlobeLayer
        accelerated={accelerated}
        globeView={globeView}
        onFocus={onFocus}
        projectionMorphProgress={projectionMorphProgress}
        scene={scene}
        suppressSemanticObjects={suppressSemanticObjects}
      />
    );
  if (scene.landforms.length === 0) return null;
  const meridians = Array.from({ length: 11 }, (_, index) =>
    Math.round((scene.world.width * index) / 10),
  );
  const parallels = Array.from({ length: 7 }, (_, index) =>
    Math.round((scene.world.height * index) / 6),
  );
  return (
    <svg
      aria-label={`${scene.landforms.length} admitted world boundary geometries`}
      className={sx(styles.worldGeometryLayer)}
      role="img"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <desc>
        Charted land is derived from admitted anchor extents. Dashed horizons
        include unresolved frontier probes and do not claim observed terrain.
      </desc>
      <g className={sx(styles.worldGraticule)} aria-hidden="true">
        {meridians.map((x) => (
          <line
            key={`meridian:${x}`}
            x1={x}
            x2={x}
            y1={0}
            y2={scene.world.height}
          />
        ))}
        {parallels.map((y) => (
          <line
            key={`parallel:${y}`}
            x1={0}
            x2={scene.world.width}
            y1={y}
            y2={y}
          />
        ))}
      </g>
      {scene.landforms
        .filter((landform) => landform.kind === "horizon")
        .map((landform) => (
          <path
            className={sx(styles.worldHorizon)}
            d={landform.path}
            data-world-geometry={landform.kind}
            key={landform.id}
          >
            <desc>{`${landform.label} / ${landform.detail}`}</desc>
          </path>
        ))}
      {scene.landforms
        .filter((landform) => landform.kind === "charted")
        .map((landform) => (
          <path
            className={sx(
              styles.chartedLand,
              accelerated && styles.chartedLandAccelerated,
            )}
            data-accelerated-surface={accelerated || undefined}
            d={landform.path}
            data-world-geometry={landform.kind}
            key={landform.id}
          >
            <desc>{`${landform.label} / ${landform.detail}`}</desc>
          </path>
        ))}
    </svg>
  );
}

function SemanticGlobeLayer({
  accelerated,
  onFocus,
  scene,
  globeView,
  projectionMorphProgress,
  suppressSemanticObjects,
}: {
  accelerated: boolean;
  onFocus: (node: FocusableTopologyObject) => void;
  scene: TopologyScene;
  globeView: GlobeCameraView;
  projectionMorphProgress: number;
  suppressSemanticObjects: boolean;
}) {
  const globe = scene.globe!;
  const center = { x: scene.world.width / 2, y: scene.world.height / 2 };
  const radius =
    Math.min(scene.world.width, scene.world.height) * WORLD_GLOBE_RADIUS_RATIO;
  const atmosphereRemaining = globeProjectionMorphRemaining(
    projectionMorphProgress,
  );
  const atmosphereOpacity = globeAtmosphereOpacity(projectionMorphProgress);
  const atmosphereShellScale = globeAtmosphereShellScale(
    projectionMorphProgress,
  );
  const surfaceOpacity = globeSurfaceOpacity(projectionMorphProgress);
  const projectedSamples = accelerated
    ? []
    : contextGlobeSamples(globe.source_revision, 5_200, globe.regions)
        .map((sample) => ({
          sample,
          ...projectGlobe(sample, center, radius, globeView),
        }))
        .filter(({ visible }) => visible);
  const samplePath = projectedSamples
    .map(
      ({ x, y, depth }) =>
        `M${x.toFixed(1)} ${y.toFixed(1)}h${Math.max(0.45, 0.9 + depth * 0.85).toFixed(1)}`,
    )
    .join("");
  const projectedPolePatterns = contextGlobePolePatterns().map((pattern) => {
    const samples = pattern.samples
      .map((sample) => ({
        ...projectGlobe(sample, center, radius, globeView),
      }))
      .filter(({ visible }) => visible);
    return {
      pattern,
      path: samples
        .map(
          ({ x, y, depth }) =>
            `M${x.toFixed(1)} ${y.toFixed(1)}h${Math.max(0.45, 0.9 + depth * 0.85).toFixed(1)}`,
        )
        .join(""),
    };
  });
  const projectedRegions = (suppressSemanticObjects ? [] : globe.regions)
    .map((region) => ({
      region,
      ...projectGlobe(region, center, radius, globeView),
    }))
    .filter(({ visible }) => visible)
    .sort((left, right) => left.depth - right.depth);
  const regionLabels = new Map(
    layoutSemanticLabels(
      projectedRegions.map(({ region, x, y, depth }) => ({
        fragment_id: `world:${region.id}`,
        semantic_identity: region.id,
        focus_id: region.focus_id,
        x: x + 12,
        y: y - 28,
        width: Math.max(64, region.label.length * 7.2),
        height: 22,
        priority: Math.round(depth * 1_000),
        selected: region.focus_id === scene.focus_id,
      })),
      70,
    ).map((placement) => [placement.fragment_id, placement]),
  );
  const projectedClusters = (suppressSemanticObjects ? [] : globe.clusters)
    .map((cluster) => ({
      cluster,
      ...projectGlobe(cluster, center, radius, globeView),
    }))
    .filter(({ visible }) => visible);
  const projectedBeacons = (suppressSemanticObjects ? [] : globe.beacons)
    .map((beacon) => ({
      beacon,
      ...projectGlobe(beacon, center, radius, globeView),
    }))
    .filter(({ visible }) => visible)
    .sort((left, right) => left.depth - right.depth);
  const focus = (focus_id: string, x: number, y: number) =>
    onFocus({ focus_id, x, y });
  return (
    <svg
      aria-label={
        globe.posture === "orientation"
          ? `${globe.beacons.length} exact workload beacons on an unmapped project globe`
          : globe.posture === "semantic_atlas"
            ? `${globe.regions.length} admitted semantic world regions on a synthetic globe`
            : `${globe.regions.length} admitted regional projection points on a synthetic globe`
      }
      className={sx(styles.worldGeometryLayer, styles.semanticGlobeLayer)}
      data-atlas-revision={
        globe.posture === "semantic_atlas" ? globe.source_revision : undefined
      }
      data-globe-posture={globe.posture}
      data-globe-revision={globe.source_revision}
      role="group"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <desc>
        {globe.posture === "orientation"
          ? "This unmapped globe orients exact file-backed workload candidates. Beacon positions are stable presentation geometry, not admitted semantic coordinates or distance claims."
          : globe.posture === "semantic_atlas"
            ? "Synthetic semantic longitude and latitude place admitted survey regions on a spherical world. They are not Earth coordinates, and zoom never reclusters this atlas revision."
            : "Revision-bound synthetic placements and occupied sector membership from the retained atlas. They are not Earth coordinates, native County footprints, or physical-distance claims."}
      </desc>
      <defs>
        <radialGradient id="rey-semantic-globe-fill" cx="34%" cy="26%" r="76%">
          <stop offset="0%" stopColor="#f4f1e4" />
          <stop offset="54%" stopColor="#e3e5da" />
          <stop offset="84%" stopColor="#bec9bb" />
          <stop offset="100%" stopColor="#7c9188" />
        </radialGradient>
        <radialGradient id="rey-semantic-globe-atmosphere">
          <stop offset="72%" stopColor="#dbe3d7" stopOpacity="0" />
          <stop offset="91%" stopColor="#a8bdb3" stopOpacity="0.13" />
          <stop offset="97%" stopColor="#f7edd7" stopOpacity="0.36" />
          <stop offset="100%" stopColor="#6f9188" stopOpacity="0" />
        </radialGradient>
        <clipPath id="rey-semantic-globe-clip">
          <circle cx={center.x} cy={center.y} r={radius} />
        </clipPath>
      </defs>
      {!accelerated && atmosphereRemaining > 0 ? (
        <circle
          aria-hidden="true"
          className={sx(styles.semanticGlobeAtmosphere)}
          cx={center.x}
          cy={center.y}
          data-globe-atmosphere=""
          data-globe-atmosphere-remaining={atmosphereRemaining}
          data-globe-atmosphere-opacity={atmosphereOpacity}
          data-globe-atmosphere-shell-scale={atmosphereShellScale}
          fill="url(#rey-semantic-globe-atmosphere)"
          opacity={atmosphereOpacity}
          r={radius * WORLD_GLOBE_ATMOSPHERE_SCALE * atmosphereShellScale}
        />
      ) : null}
      <circle
        className={sx(styles.semanticGlobeSphere)}
        cx={center.x}
        cy={center.y}
        data-accelerated-surface={accelerated || undefined}
        data-globe-halo-scale={WORLD_GLOBE_ATMOSPHERE_SCALE}
        data-globe-sphere=""
        data-globe-surface-opacity={surfaceOpacity}
        fill={accelerated ? "transparent" : "url(#rey-semantic-globe-fill)"}
        opacity={surfaceOpacity}
        r={radius}
      />
      {samplePath ? (
        <path
          aria-hidden="true"
          className={sx(styles.semanticGlobeSamples)}
          clipPath="url(#rey-semantic-globe-clip)"
          d={samplePath}
        />
      ) : null}
      {projectedPolePatterns.map(({ path, pattern }) => (
        <g
          aria-hidden="true"
          data-globe-pole-pattern={pattern.pole}
          data-globe-pole-sample-count={pattern.samples.length}
          key={pattern.id}
        >
          {!accelerated && path ? (
            <path
              className={sx(
                styles.semanticGlobeSamples,
                styles.semanticGlobePolePattern,
              )}
              clipPath="url(#rey-semantic-globe-clip)"
              d={path}
            />
          ) : null}
        </g>
      ))}
      <g aria-label={`${globe.clusters.length} world clusters`}>
        {projectedClusters.map(({ cluster, x, y, depth }) => (
          <circle
            className={sx(styles.semanticGlobeCluster)}
            cx={x}
            cy={y}
            key={cluster.id}
            r={Math.max(
              18,
              (cluster.angular_radius_degrees / 90) *
                radius *
                (0.72 + depth * 0.28),
            )}
          >
            <desc>{`${cluster.member_count} admitted regions / ${cluster.dominant_feature.replaceAll("_", " ")} structure`}</desc>
          </circle>
        ))}
      </g>
      {!accelerated ? (
        <g aria-label={`${projectedRegions.length} visible semantic regions`}>
          {projectedRegions.map(({ region, x, y, depth }) => {
            const label = regionLabels.get(`world:${region.id}`)!;
            return (
              <g
                aria-label={`${region.label}: ${region.detail}`}
                className={sx(
                  styles.semanticGlobeRegion,
                  toneStyle(region.tone, "node"),
                )}
                data-semantic-region={region.id}
                data-label-disposition={label.disposition}
                data-label-layout={SEMANTIC_LABEL_LAYOUT_REVISION}
                key={region.id}
                onClick={() => focus(region.focus_id, x, y)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    focus(region.focus_id, x, y);
                  }
                }}
                role="button"
                tabIndex={0}
              >
                <circle
                  className={sx(styles.semanticGlobeRegionPoint)}
                  cx={x}
                  cy={y}
                  r={7 + depth * 7}
                />
                {label.visible ? (
                  <>
                    <line
                      className={sx(styles.semanticGlobeBeaconLeader)}
                      x1={x + 6}
                      x2={x + 16}
                      y1={y - 6}
                      y2={y - 15}
                    />
                    <text
                      className={sx(styles.semanticGlobeRegionLabel)}
                      x={x + 14}
                      y={y - 10}
                    >
                      {region.label}
                    </text>
                  </>
                ) : null}
                <desc>{region.detail}</desc>
              </g>
            );
          })}
        </g>
      ) : null}
      {!accelerated ? (
        <g aria-label={`${projectedBeacons.length} visible workload beacons`}>
          {projectedBeacons.map(({ beacon, x, y, depth }) => (
            <g
              aria-label={`${beacon.mapping_role === "survey" ? "Survey" : "Workload"} beacon: ${beacon.label}. ${beacon.next_step}`}
              className={sx(
                styles.semanticGlobeBeacon,
                beacon.mapping_role === "survey" &&
                  styles.semanticGlobeSurveyBeacon,
                toneStyle(beacon.tone, "node"),
              )}
              data-beacon-state={beacon.state}
              data-workload-beacon={beacon.workload_id}
              key={beacon.id}
              onClick={() => focus(beacon.focus_id, x, y)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  focus(beacon.focus_id, x, y);
                }
              }}
              role="button"
              tabIndex={0}
            >
              <circle
                className={sx(styles.semanticGlobeBeaconHalo)}
                cx={x}
                cy={y}
                r={18 + depth * 8}
              />
              <circle
                className={sx(styles.semanticGlobeBeaconPoint)}
                cx={x}
                cy={y}
                r={beacon.mapping_role === "survey" ? 10 : 7}
              />
              <line
                className={sx(styles.semanticGlobeBeaconLeader)}
                x1={x + 6}
                x2={x + 16}
                y1={y - 6}
                y2={y - 15}
              />
              <text
                className={sx(styles.semanticGlobeBeaconLabel)}
                x={x + 17}
                y={y - 13}
              >
                {globeBeaconLabel(beacon.workload_id, beacon.mapping_role)}
              </text>
              <text
                className={sx(styles.semanticGlobeBeaconState)}
                x={x + 17}
                y={y + 3}
              >
                {beacon.state.toUpperCase()} · SELECT TO REVIEW
              </text>
              <desc>{`${beacon.label} / ${beacon.detail} / ${beacon.next_step}`}</desc>
            </g>
          ))}
        </g>
      ) : null}
      <text
        className={sx(styles.semanticGlobeCaption)}
        data-globe-caption=""
        textAnchor="middle"
        x={center.x}
        y={center.y + radius * WORLD_GLOBE_ATMOSPHERE_SCALE + 30}
      >
        {globe.posture === "orientation"
          ? `UNMAPPED PROJECT / ${globe.beacons.length} WORKLOAD BEACONS / NO DISTANCE CLAIM`
          : `${globe.posture === "semantic_atlas" ? "SEMANTIC SPHERE" : "REGIONAL WORLD"} / REV ${globe.source_revision.slice(0, 12)}`}
      </text>
    </svg>
  );
}

function WorldAtlasTransitionLayer({
  accelerated,
  globeView,
  onFocus,
  progress,
  scene,
}: {
  accelerated: boolean;
  globeView: GlobeCameraView;
  onFocus: (node: FocusableTopologyObject) => void;
  progress: number;
  scene: TopologyScene;
}) {
  const transition = scene.world_atlas_transition!;
  const labelStrokeOpacity = worldAtlasMorphLabelStrokeOpacity(progress);
  const labelStyle = {
    "--rey-world-atlas-label-growth": worldAtlasLabelGrowthScale(progress),
  } as CSSProperties;
  const worldFrame = {
    center: { x: scene.world.width / 2, y: scene.world.height / 2 },
    radius: Math.min(scene.world.width, scene.world.height) * 0.41,
  };
  const sectorFragments = transition.sectors.flatMap((sector) =>
    projectWorldAtlasBoundsMorph(
      sector.identity,
      sector,
      worldFrame,
      transition.atlas_frame,
      globeView,
      progress,
    ).map((fragment) => ({ fragment, sector })),
  );
  const points = transition.points.map((point) => ({
    point,
    projected: projectWorldAtlasMorph(
      point.identity,
      point.focus_id,
      {
        longitude_microdegrees: point.longitude_microdegrees,
        latitude_microdegrees: point.latitude_microdegrees,
      },
      worldFrame,
      transition.atlas_frame,
      globeView,
      progress,
    ),
  }));
  const pointLabels = new Map(
    layoutSemanticLabels(
      points.map(({ point, projected }) => ({
        fragment_id: `morph:${point.identity}`,
        semantic_identity: point.identity,
        focus_id: point.focus_id,
        x: projected.x + 11,
        y: projected.y - 28,
        width: Math.max(64, point.label.length * 7.2),
        height: 22,
        priority: Math.round(projected.world.depth * 1_000),
        selected: point.focus_id === scene.focus_id,
      })),
      96,
    ).map((placement) => [placement.fragment_id, placement]),
  );
  return (
    <svg
      aria-label={`${points.length} regional identities morphing from World to Atlas`}
      className={sx(styles.worldGeometryLayer, styles.worldAtlasMorphLayer)}
      data-atlas-revision={transition.atlas_revision}
      data-projection-morph={transition.projection_revision}
      data-projection-morph-progress={progress.toFixed(3)}
      role="group"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <desc>{transition.authority}</desc>
      <g aria-label={`${transition.sectors.length} retained sector identities`}>
        {!accelerated
          ? sectorFragments.map(({ fragment, sector }) => (
              <path
                className={sx(styles.worldAtlasMorphSector)}
                d={`${fragment.points.map(({ x, y }, index) => `${index === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`).join(" ")} Z`}
                data-semantic-identity={fragment.identity}
                data-wrap-fragment={fragment.fragment_id}
                key={fragment.fragment_id}
              >
                <desc>{`${sector.label} / ${fragment.polar_disclosures.join(" + ") || "inside Mercator latitude cutoff"}`}</desc>
              </path>
            ))
          : null}
      </g>
      <g aria-label={`${points.length} retained regional identities`}>
        {points.map(({ point, projected }) => {
          const label = pointLabels.get(`morph:${point.identity}`)!;
          return (
            <g
              aria-label={`${point.label}: retained regional identity`}
              className={sx(
                styles.worldAtlasMorphPoint,
                toneStyle(point.tone, "node"),
              )}
              data-focus-id={point.focus_id}
              data-label-disposition={label.disposition}
              data-label-layout={SEMANTIC_LABEL_LAYOUT_REVISION}
              data-semantic-identity={point.identity}
              key={point.identity}
              onClick={() =>
                onFocus({
                  focus_id: point.focus_id,
                  x: projected.x,
                  y: projected.y,
                })
              }
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  onFocus({
                    focus_id: point.focus_id,
                    x: projected.x,
                    y: projected.y,
                  });
                }
              }}
              role="button"
              tabIndex={0}
            >
              {!accelerated ? (
                <circle
                  className={sx(styles.worldAtlasMorphMarker)}
                  cx={projected.x}
                  cy={projected.y}
                  r={8}
                />
              ) : null}
              {label.visible ? (
                <text
                  className={sx(styles.worldAtlasMorphLabel)}
                  strokeOpacity={labelStrokeOpacity}
                  style={labelStyle}
                  x={projected.x + 13}
                  y={projected.y - 10}
                >
                  {point.label}
                </text>
              ) : null}
            </g>
          );
        })}
      </g>
    </svg>
  );
}

const WORLD_ATLAS_MORPH_LABEL_STROKE_FADE_WINDOW = 0.18;

/**
 * Eases a retained regional label's white halo in as WorldAtlasTransitionLayer
 * mounts (progress just above 0), instead of popping to full strength the
 * instant it does — full strength for the rest of the transition, all the
 * way through arriving at the flat map. It does NOT fade back out
 * approaching progress 1: AtlasFeatureLayer's own canonical (wrapIndex 0)
 * label only ever mounts at progress 1 exactly (chartWrapIndexes excludes
 * it while still dissolving, to avoid duplicating this very label), so it
 * has no earlier moment to ease in from — fading this one out first would
 * dip to nothing right before that label pops in at full strength, reading
 * as worse than not fading at all. Staying at full strength through the
 * handoff means both sides of it agree, so the swap itself is invisible.
 * Reuses globeProjectionMorphRemaining's own smoothstep curve
 * (1 - smoothstep) rather than a new formula, so the fade shares its exact
 * shape with the rest of the morph's presentation.
 */
function worldAtlasMorphLabelStrokeOpacity(progress: number): number {
  const boundedProgress = Math.max(0, Math.min(1, progress));
  return (
    1 -
    globeProjectionMorphRemaining(
      Math.min(1, boundedProgress / WORLD_ATLAS_MORPH_LABEL_STROKE_FADE_WINDOW),
    )
  );
}

const WORLD_ATLAS_LABEL_GROWTH = 0.3;

/**
 * Grows a retained regional label as the World<->Atlas morph approaches the
 * flat map, reading as gaining presence while zooming in rather than
 * staying a fixed pixel size the whole way. Layered on top of, not instead
 * of, the counter-scale that cancels the scene's own zoom curve — this
 * growth is a deliberate design curve, independent of that curve's own
 * (non-monotonic, wobble-shaped) one. Shared by both sides of the
 * World<->Atlas handoff (worldAtlasMorphLabel and atlasFeatureLabel) so the
 * size in view keeps climbing continuously through the swap instead of
 * jumping between two different curves. Reuses globeProjectionMorphRemaining's
 * own smoothstep curve, matching the rest of the morph's presentation.
 */
function worldAtlasLabelGrowthScale(progress: number): number {
  const boundedProgress = Math.max(0, Math.min(1, progress));
  const eased = 1 - globeProjectionMorphRemaining(boundedProgress);
  return 1 + WORLD_ATLAS_LABEL_GROWTH * eased;
}

function globeBeaconLabel(workloadId: string, mappingRole: string) {
  if (mappingRole === "survey") return "CONTEXT SURVEY";
  return workloadId
    .split(/[-_]/)
    .filter(Boolean)
    .slice(0, 3)
    .join(" ")
    .toUpperCase();
}

function projectGlobe(
  coordinate: { longitude_degrees: number; latitude_degrees: number },
  center: { x: number; y: number },
  radius: number,
  view: GlobeCameraView,
) {
  return projectSemanticGlobe(
    {
      longitude_microdegrees:
        coordinate.longitude_degrees * MICRODEGREES_PER_DEGREE,
      latitude_microdegrees:
        coordinate.latitude_degrees * MICRODEGREES_PER_DEGREE,
    },
    center,
    radius,
    view,
  );
}

const MICRODEGREES_PER_DEGREE = 1_000_000;

function NaturalFeatureLayer({
  scene,
  showWater,
  showWeather,
}: {
  scene: TopologyScene;
  showWater: boolean;
  showWeather: boolean;
}) {
  const features = scene.natural_features.filter((feature) =>
    feature.kind === "weather_front" ? showWeather : showWater,
  );
  if (features.length === 0) return null;
  const showLabels =
    scene.regime === "world" ||
    scene.regime === "atlas" ||
    scene.regime === "landscape";
  return (
    <svg
      aria-label={`${features.length} emergent natural features`}
      className={sx(styles.naturalFeatureLayer)}
      role="img"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <desc>
        Weather fronts derive from unresolved admitted survey conditions.
        Streams and rivers derive from rainfall accumulation over anchor-only
        relief and visibly erode that projected field. None are source edges,
        discovered paths, or retained natural facts.
      </desc>
      {features.map((feature) => {
        const pathId = `natural-feature-${feature.id.replaceAll(/[^a-zA-Z0-9_-]/g, "-")}`;
        return (
          <g className={sx(styles.naturalFeatureGroup)} key={feature.id}>
            <path
              className={sx(
                styles.naturalFeature,
                feature.kind === "stream" && styles.streamFeature,
                feature.kind === "river" && styles.riverFeature,
                feature.kind === "weather_front" && styles.weatherFront,
              )}
              d={feature.path}
              data-natural-feature={feature.kind}
              id={pathId}
              style={{
                strokeWidth: `calc(${0.7 + feature.intensity * (feature.kind === "river" ? 0.48 : 0.22)}px * var(--rey-terrain-counter-scale))`,
              }}
            >
              <desc>{`${feature.label} / ${feature.detail}`}</desc>
            </path>
            {showLabels &&
            (feature.kind === "river" || feature.kind === "weather_front") ? (
              <text className={sx(styles.naturalFeatureLabel)}>
                <textPath
                  href={`#${pathId}`}
                  startOffset="50%"
                  textAnchor="middle"
                >
                  {feature.label}
                </textPath>
              </text>
            ) : null}
          </g>
        );
      })}
    </svg>
  );
}

function ReliefLayer({ scene }: { scene: TopologyScene }) {
  if (scene.contours.length === 0) return null;
  return (
    <svg
      aria-label={`${scene.contours.length} validity-bounded terrain contour levels`}
      className={sx(styles.reliefLayer)}
      role="img"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <desc>
        Contours are derived from the same terrain elevation shown by the
        surface and stop at every unsupported cell. They do not assert a source
        relationship or extend geographic validity.
      </desc>
      {scene.contours.map((contour) => (
        <path
          className={sx(
            styles.reliefContour,
            contour.level >= 6 && styles.reliefContourPeak,
            contour.level <= 2 && styles.reliefContourLow,
          )}
          d={contour.path}
          data-anchor-count={contour.anchor_count}
          data-relief-level={contour.level}
          key={contour.id}
          style={{
            strokeDasharray:
              contour.level <= 2
                ? "calc(3px * var(--rey-terrain-counter-scale)) calc(3px * var(--rey-terrain-counter-scale))"
                : undefined,
            strokeWidth: `calc(${contour.level >= 6 ? 1.55 : 1.15}px * var(--rey-terrain-counter-scale))`,
          }}
        />
      ))}
    </svg>
  );
}

function EdgeLayer({
  edges,
  nodes,
  points,
  terrain,
  world,
}: {
  edges: TopologyEdge[];
  nodes: TopologyNode[];
  points: TopologyPointOfInterest[];
  terrain: boolean;
  world: TopologyScene["world"];
}) {
  const byId = new Map(
    [...nodes, ...points].map((candidate) => [candidate.id, candidate]),
  );
  return (
    <svg
      aria-hidden="true"
      className={sx(styles.edgeLayer)}
      viewBox={`0 0 ${world.width} ${world.height}`}
    >
      <defs>
        <marker
          id="topology-arrow"
          markerHeight="7"
          markerWidth="7"
          orient="auto-start-reverse"
          refX="6"
          refY="3.5"
        >
          <path d="M0,0 L7,3.5 L0,7 Z" />
        </marker>
      </defs>
      {edges.map((edge) => {
        const from = byId.get(edge.from);
        const to = byId.get(edge.to);
        if (!from || !to) return null;
        const midpoint = {
          x: (from.x + to.x) / 2,
          y: (from.y + to.y) / 2,
        };
        return (
          <g className={sx(styles.edgeGroup)} key={edge.id}>
            <line
              className={sx(
                styles.edge,
                edge.kind === "directs" && styles.edgeDirects,
                edge.kind === "contains" && styles.edgeContains,
              )}
              markerEnd={terrain ? undefined : "url(#topology-arrow)"}
              style={
                terrain
                  ? {
                      strokeDasharray:
                        edge.kind === "contains"
                          ? "calc(6px * var(--rey-terrain-counter-scale)) calc(5px * var(--rey-terrain-counter-scale))"
                          : undefined,
                      strokeWidth: `calc(${edge.kind === "directs" ? 2.4 : 1.5}px * var(--rey-terrain-counter-scale))`,
                    }
                  : undefined
              }
              x1={from.x}
              x2={to.x}
              y1={from.y}
              y2={to.y}
            />
            <text
              className={sx(styles.edgeLabel)}
              style={
                terrain
                  ? {
                      fontSize: "calc(9px * var(--rey-terrain-counter-scale))",
                      strokeWidth:
                        "calc(5px * var(--rey-terrain-counter-scale))",
                    }
                  : undefined
              }
              textAnchor="middle"
              x={midpoint.x}
              y={midpoint.y - 7}
            >
              {edge.label.toUpperCase()}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

function PointOfInterest({
  onFocus,
  point,
  regime,
}: {
  onFocus: (point: TopologyPointOfInterest) => void;
  point: TopologyPointOfInterest;
  regime: LensRegime;
}) {
  const showLabel =
    regime === "world"
      ? point.kind === "frontier" || point.prominence >= 4
      : regime === "atlas"
        ? point.prominence >= 3
        : regime === "landscape"
          ? point.prominence >= 2
          : true;
  const showDetail =
    regime === "neighborhoods" || regime === "objects" || regime === "evidence";
  return (
    <button
      aria-label={`${point.family} point of interest: ${point.label}`}
      className={sx(
        styles.pointOfInterest,
        point.kind === "frontier" && styles.frontierPoint,
        point.prominence >= 3 && styles.prominentPoint,
      )}
      data-prominence={point.prominence}
      onClick={() => onFocus(point)}
      style={{ left: point.x, top: point.y }}
      type="button"
    >
      <i
        className={sx(
          styles.pointMarker,
          point.kind === "frontier" && styles.frontierMarker,
        )}
        aria-hidden="true"
      />
      {showLabel ? (
        <span className={sx(styles.pointLabel)}>
          <small className={sx(styles.pointFamily)}>{point.family}</small>
          <strong className={sx(styles.pointName)}>{point.label}</strong>
          {showDetail ? (
            <>
              <em className={sx(styles.pointSignal)}>{point.signal}</em>
              <em className={sx(styles.pointDetail)}>{point.detail}</em>
              <em className={sx(styles.pointAction)}>{point.action}</em>
            </>
          ) : null}
        </span>
      ) : null}
    </button>
  );
}

function TopologyObject({
  chartWrapIndex,
  counterScale,
  linkToWorkload,
  labelPlacement,
  node,
  onFocus,
  pickingIndex,
  worldWidth,
}: {
  chartWrapIndex: number;
  counterScale: boolean;
  linkToWorkload: boolean;
  labelPlacement?: SemanticLabelPlacement;
  node: TopologyNode;
  onFocus: (node: FocusableTopologyObject) => void;
  pickingIndex: ScenePickingIndex;
  worldWidth: number;
}) {
  const projectedX = node.x + chartWrapIndex * worldWidth;
  const labelVisible = labelPlacement?.visible ?? true;
  const className = sx(
    styles.topologyObject,
    !labelVisible && styles.topologyObjectCulledLabel,
    toneStyle(node.tone, "node"),
  );
  const style = {
    left: projectedX,
    top: node.y,
    ...(counterScale
      ? {
          transform:
            "translate(-50%, -50%) scale(var(--rey-terrain-counter-scale))",
        }
      : {}),
    width: labelVisible ? node.width : 20,
  } as CSSProperties;
  const select = () => {
    const pick = pickSceneNode(pickingIndex, node, {
      x: projectedX,
      y: node.y,
    });
    if (pick) onFocus(pick);
  };
  const content = (
    <>
      <span className={sx(styles.objectFamily)}>{node.family}</span>
      <strong className={sx(styles.objectLabel)}>{node.label}</strong>
      <small className={sx(styles.objectDetail)}>{node.detail}</small>
      <span className={sx(styles.objectAction)}>
        {node.coordinate_uri
          ? "OPEN COORDINATE ↗"
          : linkToWorkload && node.evidence_uri
            ? "OPEN EXACT EVIDENCE ↗"
            : linkToWorkload && node.workload_id
              ? "OPEN RECORD ↗"
              : "FOCUS / ZOOM →"}
      </span>
    </>
  );
  const renderedContent = labelVisible ? (
    content
  ) : (
    <span aria-hidden="true" className={sx(styles.culledLabelMarker)} />
  );

  if (node.coordinate_uri) {
    return (
      <a
        aria-hidden={chartWrapIndex === 0 ? undefined : true}
        aria-label={`${node.family}: ${node.label}`}
        className={className}
        href={`/explore?coordinate=${encodeURIComponent(node.coordinate_uri)}&scale=${OBJECT_LENS_ZOOM}`}
        style={style}
        tabIndex={chartWrapIndex === 0 ? undefined : -1}
        data-label-disposition={labelPlacement?.disposition}
        data-label-layout={
          labelPlacement ? SEMANTIC_LABEL_LAYOUT_REVISION : undefined
        }
      >
        {renderedContent}
      </a>
    );
  }

  if (linkToWorkload && node.evidence_uri) {
    return (
      <a
        aria-hidden={chartWrapIndex === 0 ? undefined : true}
        aria-label={`${node.family}: ${node.label}`}
        className={className}
        data-object-evidence={node.evidence_uri}
        href={node.evidence_uri}
        style={style}
        tabIndex={chartWrapIndex === 0 ? undefined : -1}
      >
        {renderedContent}
      </a>
    );
  }

  if (linkToWorkload && node.workload_id) {
    return (
      <Link
        aria-hidden={chartWrapIndex === 0 ? undefined : true}
        aria-label={`${node.family}: ${node.label}`}
        className={className}
        params={{ workloadId: node.workload_id }}
        style={style}
        tabIndex={chartWrapIndex === 0 ? undefined : -1}
        data-label-disposition={labelPlacement?.disposition}
        data-label-layout={
          labelPlacement ? SEMANTIC_LABEL_LAYOUT_REVISION : undefined
        }
        to="/workloads/$workloadId"
      >
        {renderedContent}
      </Link>
    );
  }

  return (
    <button
      aria-hidden={chartWrapIndex === 0 ? undefined : true}
      aria-label={`${node.family}: ${node.label}`}
      className={className}
      data-chart-wrap-index={
        node.semantic_identity ? chartWrapIndex : undefined
      }
      data-semantic-identity={node.semantic_identity}
      data-label-disposition={labelPlacement?.disposition}
      data-label-layout={
        labelPlacement ? SEMANTIC_LABEL_LAYOUT_REVISION : undefined
      }
      onClick={select}
      style={style}
      tabIndex={chartWrapIndex === 0 ? undefined : -1}
      type="button"
    >
      {renderedContent}
    </button>
  );
}

export function ReferenceMapReading({ scene }: { scene: TopologyScene }) {
  if (!scene.terrain) return null;
  const orientation = scene.globe?.posture === "orientation";
  const selectedBeacon = orientation
    ? (scene.globe?.beacons.find(
        (beacon) => beacon.focus_id === scene.focus_id,
      ) ??
      scene.globe?.beacons.find((beacon) => beacon.mapping_role === "survey") ??
      scene.globe?.beacons[0])
    : undefined;
  const waterSystems = scene.natural_features.filter(
    (feature) => feature.kind === "stream" || feature.kind === "river",
  );
  const admittedWater = scene.nodes.filter(
    ({ spatial_feature }) => spatial_feature?.layer === "hydrology",
  );
  const admittedWaterAreas = admittedWater.filter(
    ({ spatial_feature }) =>
      spatial_feature?.geometry_kind.toLowerCase() === "polygon",
  );
  const weatherFronts = scene.natural_features.filter(
    (feature) => feature.kind === "weather_front",
  );
  const probes = scene.points.filter((point) => point.kind === "frontier");
  const landscape = scene.regime === "landscape";
  if (orientation) {
    return (
      <aside
        className={sx(styles.mapReading, styles.orientationMapReading)}
        aria-label="Project orientation and survey consent"
      >
        {selectedBeacon ? (
          <div
            className={sx(styles.orientationGuide)}
            data-selected-workload-beacon={selectedBeacon.workload_id}
          >
            <span className={sx(styles.bearingEyebrow)}>
              FIRST MAPPING STEP · {scene.bearing.label}
            </span>
            <strong className={sx(styles.bearingTitle)}>
              {selectedBeacon.label}
            </strong>
            <small className={sx(styles.bearingDetail)}>
              {selectedBeacon.next_step}
            </small>
            <code
              aria-label={`${selectedBeacon.state.toUpperCase()} · ${selectedBeacon.source} · revision ${selectedBeacon.source_revision}`}
            >
              {selectedBeacon.state.toUpperCase()} · {selectedBeacon.source}
            </code>
            <span className={sx(styles.orientationActions)}>
              <a
                className={sx(
                  styles.orientationAction,
                  (selectedBeacon.state === "working" ||
                    selectedBeacon.state === "index") &&
                    styles.consentAction,
                )}
                href={`/workloads/${encodeURIComponent(selectedBeacon.workload_id)}`}
              >
                {selectedBeacon.state === "working" ||
                selectedBeacon.state === "index"
                  ? "REVIEW & CONSENT →"
                  : "INSPECT EXACT WORKLOAD →"}
              </a>
            </span>
          </div>
        ) : null}
        <div className={sx(styles.orientationBoundary)} aria-hidden="true">
          {scene.globe?.beacons.length ?? 0} FILE-BACKED SIGNALS · PROJECTION
          FABRIC ONLY · NO SURVEY CLAIM · NO DISTANCE CLAIM
        </div>
      </aside>
    );
  }
  return (
    <aside
      className={sx(styles.mapReading, landscape && styles.landscapeMapReading)}
      aria-label="Map evidence legend"
      data-map-reading={landscape ? "compact" : "detailed"}
    >
      {!landscape ? (
        <div className={sx(styles.bearingCard)}>
          <span className={sx(styles.bearingEyebrow)}>
            BEARING / {scene.bearing.status.replaceAll("_", " ")}
          </span>
          <strong className={sx(styles.bearingTitle)}>
            {scene.bearing.label}
          </strong>
          <small className={sx(styles.bearingDetail)}>
            {scene.bearing.detail}
          </small>
        </div>
      ) : null}
      {!landscape ? (
        <div className={sx(styles.mapKey)} aria-hidden="true">
          {admittedWater.length > 0 ? (
            <>
              <span>
                <i className={sx(styles.keyContour)} /> VALIDITY / CONTOURS
              </span>
              <span>
                <i className={sx(styles.keyStream)} /> ADMITTED / CHANNEL
              </span>
              <span>
                <i className={sx(styles.keyRiver)} /> ADMITTED / WATER AREA
              </span>
              <span>
                <i className={sx(styles.keyWeather)} /> NO-DATA / BOUNDARY
              </span>
            </>
          ) : (
            <>
              <span>
                <i className={sx(styles.keyContour)} /> ANCHORS / RELIEF
              </span>
              <span>
                <i className={sx(styles.keyStream)} /> RUNOFF / STREAM
              </span>
              <span>
                <i className={sx(styles.keyRiver)} /> ACCUMULATION / RIVER
              </span>
              <span>
                <i className={sx(styles.keyWeather)} /> UNRESOLVED / WEATHER
              </span>
            </>
          )}
        </div>
      ) : null}
      <div className={sx(styles.mapScale)} aria-hidden="true">
        <i className={sx(styles.mapScaleBar)} />
        <span>
          {admittedWater.length > 0 ? (
            <>
              {admittedWater.length} ADMITTED WATER FEATURES ·{" "}
              {admittedWaterAreas.length} WATER AREAS · EXACT PATHS
            </>
          ) : (
            <>
              {waterSystems.length} PROJECTED WATER SYSTEMS ·{" "}
              {weatherFronts.length} WEATHER FRONTS · {probes.length} PROBES ·
              NO PATH CLAIM
            </>
          )}{" "}
          · LOD {scene.regime.toUpperCase()}
        </span>
      </div>
    </aside>
  );
}

function toneStyle(tone: TopologyTone, kind: "node" | "region") {
  if (kind === "region") {
    if (tone === "accent") return styles.regionAccent;
    if (tone === "attention" || tone === "blocked")
      return styles.regionAttention;
    if (tone === "healthy") return styles.regionHealthy;
    if (tone === "unknown") return styles.regionUnknown;
    if (tone === "omitted") return styles.regionOmitted;
    if (tone === "stale") return styles.regionStale;
    if (tone === "unsupported") return styles.regionUnsupported;
    if (tone === "frontier") return styles.regionFrontier;
    return styles.regionNeutral;
  }
  if (tone === "accent") return styles.objectAccent;
  if (tone === "attention") return styles.objectAttention;
  if (tone === "blocked") return styles.objectBlocked;
  if (tone === "healthy") return styles.objectHealthy;
  if (tone === "unknown") return styles.objectUnknown;
  if (tone === "omitted") return styles.objectOmitted;
  if (tone === "stale") return styles.objectStale;
  if (tone === "unsupported") return styles.objectUnsupported;
  if (tone === "frontier") return styles.objectFrontier;
  return styles.objectNeutral;
}
