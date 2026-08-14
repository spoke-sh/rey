import { Link } from "@tanstack/react-router";
import type { CSSProperties } from "react";
import {
  OBJECT_LENS_ZOOM,
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
import { contextGlobeSamples } from "@rey/explorer/globe-samples";
import {
  projectSemanticGlobe,
  projectWorldAtlasBoundsMorph,
  projectWorldAtlasMorph,
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
}: {
  accelerated?: boolean;
  layers: ReferenceLayerVisibility;
  onFocus: (node: FocusableTopologyObject) => void;
  scene: TopologyScene;
  globeView?: GlobeCameraView;
  projectionMorphProgress?: number;
  renderGraph?: ExplorerRenderGraph;
  pickingIndex?: ScenePickingIndex;
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
  const chartWrapIndexes = wrappedAtlas ? [-1, 0, 1] : [0];
  const activePickingIndex = pickingIndex ?? compileScenePickingIndex(scene);
  const atlasLabelPlacements = new Map(
    (wrappedAtlas
      ? layoutSemanticLabels(
          chartWrapIndexes.flatMap((wrapIndex) =>
            scene.nodes.map((node) => ({
              fragment_id: `${wrapIndex}:${node.id}`,
              semantic_identity: node.semantic_identity ?? node.id,
              focus_id: node.focus_id,
              x: node.x + wrapIndex * scene.world.width - node.width / 2,
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
        scene.terrain && accelerated && styles.acceleratedTerrainProjection,
        scene.terrain &&
          scene.regime === "world" &&
          styles.worldTerrainProjection,
      )}
      data-lens-regime={scene.regime}
      data-render-graph={activeRenderGraph.graph_id}
      data-render-passes={activeRenderPasses.map(({ id }) => id).join(",")}
      data-renderer={accelerated ? "reference-overlays" : "reference"}
    >
      {!globeWorld &&
        !morphActive &&
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
      <CountyFootprintLayer scene={scene} />
      <WorldGeometryLayer
        accelerated={accelerated}
        globeView={globeView}
        onFocus={onFocus}
        scene={scene}
        suppressSemanticObjects={morphActive}
      />
      {morphActive ? (
        <WorldAtlasTransitionLayer
          globeView={globeView}
          onFocus={onFocus}
          progress={projectionMorphProgress}
          scene={scene}
        />
      ) : null}
      {!globeWorld && layers.relief ? <ReliefLayer scene={scene} /> : null}
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

function CountyFootprintLayer({ scene }: { scene: TopologyScene }) {
  const footprint = scene.county_footprint;
  if (!footprint) return null;
  return (
    <svg
      aria-label={`Exact admitted County footprint ${footprint.footprint_id}`}
      className={sx(styles.worldGeometryLayer, styles.countyFootprintLayer)}
      data-county-footprint={footprint.footprint_id}
      data-source-object={footprint.source_object_id}
      data-source-revision={footprint.source_object_revision}
      role="img"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <title>{`${footprint.source_object_id} / ${footprint.coordinate_count} exact native coordinates / ${footprint.authority}`}</title>
      <path
        className={sx(styles.countyFootprint)}
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
  suppressSemanticObjects,
}: {
  accelerated: boolean;
  onFocus: (node: FocusableTopologyObject) => void;
  scene: TopologyScene;
  globeView: GlobeCameraView;
  suppressSemanticObjects: boolean;
}) {
  if (scene.regime === "world" && scene.globe)
    return (
      <SemanticGlobeLayer
        accelerated={accelerated}
        globeView={globeView}
        onFocus={onFocus}
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
      <title>
        Charted land is derived from admitted anchor extents. Dashed horizons
        include unresolved frontier probes and do not claim observed terrain.
      </title>
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
            <title>{`${landform.label} / ${landform.detail}`}</title>
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
            <title>{`${landform.label} / ${landform.detail}`}</title>
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
  suppressSemanticObjects,
}: {
  accelerated: boolean;
  onFocus: (node: FocusableTopologyObject) => void;
  scene: TopologyScene;
  globeView: GlobeCameraView;
  suppressSemanticObjects: boolean;
}) {
  const globe = scene.globe!;
  const center = { x: scene.world.width / 2, y: scene.world.height / 2 };
  const radius =
    Math.min(scene.world.width, scene.world.height) * WORLD_GLOBE_RADIUS_RATIO;
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
  const projectedBeacons = globe.beacons
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
      <title>
        {globe.posture === "orientation"
          ? "This unmapped globe orients exact file-backed workload candidates. Beacon positions are stable presentation geometry, not admitted semantic coordinates or distance claims."
          : globe.posture === "semantic_atlas"
            ? "Synthetic semantic longitude and latitude place admitted survey regions on a spherical world. They are not Earth coordinates, and zoom never reclusters this atlas revision."
            : "Revision-bound synthetic placements and occupied sector membership from the retained atlas. They are not Earth coordinates, native County footprints, or physical-distance claims."}
      </title>
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
      <circle
        aria-hidden="true"
        className={sx(styles.semanticGlobeAtmosphere)}
        cx={center.x}
        cy={center.y}
        data-globe-atmosphere=""
        fill="url(#rey-semantic-globe-atmosphere)"
        r={radius * 1.09}
      />
      <circle
        className={sx(styles.semanticGlobeSphere)}
        cx={center.x}
        cy={center.y}
        data-accelerated-surface={accelerated || undefined}
        fill={accelerated ? "transparent" : "url(#rey-semantic-globe-fill)"}
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
            <title>{`${cluster.member_count} admitted regions / ${cluster.dominant_feature.replaceAll("_", " ")} structure`}</title>
          </circle>
        ))}
      </g>
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
              <title>{region.detail}</title>
            </g>
          );
        })}
      </g>
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
            <title>{`${beacon.label} / ${beacon.detail} / ${beacon.next_step}`}</title>
          </g>
        ))}
      </g>
      <text
        className={sx(styles.semanticGlobeCaption)}
        x={center.x - radius}
        y={center.y + radius + 34}
      >
        {globe.posture === "orientation"
          ? `UNMAPPED PROJECT / ${globe.beacons.length} WORKLOAD BEACONS / NO DISTANCE CLAIM`
          : `${globe.posture === "semantic_atlas" ? "SEMANTIC SPHERE" : "REGIONAL WORLD"} / ${globe.regions.length} ADMITTED REGIONS / REV ${globe.source_revision.slice(0, 12)}`}
      </text>
    </svg>
  );
}

function WorldAtlasTransitionLayer({
  globeView,
  onFocus,
  progress,
  scene,
}: {
  globeView: GlobeCameraView;
  onFocus: (node: FocusableTopologyObject) => void;
  progress: number;
  scene: TopologyScene;
}) {
  const transition = scene.world_atlas_transition!;
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
      <title>{transition.authority}</title>
      <g aria-label={`${transition.sectors.length} retained sector identities`}>
        {sectorFragments.map(({ fragment, sector }) => (
          <path
            className={sx(styles.worldAtlasMorphSector)}
            d={`${fragment.points.map(({ x, y }, index) => `${index === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`).join(" ")} Z`}
            data-semantic-identity={fragment.identity}
            data-wrap-fragment={fragment.fragment_id}
            key={fragment.fragment_id}
          >
            <title>{`${sector.label} / ${fragment.polar_disclosures.join(" + ") || "inside Mercator latitude cutoff"}`}</title>
          </path>
        ))}
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
              <circle
                className={sx(styles.worldAtlasMorphMarker)}
                cx={projected.x}
                cy={projected.y}
                r={8}
              />
              {label.visible ? (
                <text
                  className={sx(styles.worldAtlasMorphLabel)}
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
      <title>
        Weather fronts derive from unresolved admitted survey conditions.
        Streams and rivers derive from rainfall accumulation over anchor-only
        relief and visibly erode that projected field. None are source edges,
        discovered paths, or retained natural facts.
      </title>
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
              <title>{`${feature.label} / ${feature.detail}`}</title>
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
      aria-label={`${scene.contours.length} anchor-derived terrain contour levels`}
      className={sx(styles.reliefLayer)}
      role="img"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <title>
        Relief height is derived only from admitted anchor samples, then eroded
        by the deterministic runoff projection. It does not assert semantic
        similarity or a discovered path.
      </title>
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
  const weatherFronts = scene.natural_features.filter(
    (feature) => feature.kind === "weather_front",
  );
  const probes = scene.points.filter((point) => point.kind === "frontier");
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
            <code title={selectedBeacon.source_revision}>
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
    <aside className={sx(styles.mapReading)} aria-label="Map evidence legend">
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
      <div className={sx(styles.mapKey)} aria-hidden="true">
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
      </div>
      <div className={sx(styles.mapScale)} aria-hidden="true">
        <i className={sx(styles.mapScaleBar)} />
        <span>
          {waterSystems.length} PROJECTED WATER SYSTEMS · {weatherFronts.length}{" "}
          WEATHER FRONTS · {probes.length} PROBES · NO PATH CLAIM · LOD{" "}
          {scene.regime.toUpperCase()}
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
